use mech_core::{
    AliasPolicy, LayoutGeneration, OutputConstruction, PlanGeneration, RegionPolicy,
    ResolvedOperationContract, ValueData,
};
use mech_core::snapshot::SequenceView;
use mech_engine::{
    ArtifactSource, BindingDeclaration, MechProgram, MechProgramConfig, ProducerReference,
    ProgramArtifact, SlotRole, decode_program_artifact_bytecode_v1,
};
use mech_engine::__resident::{
    ActivationFacts, ResidentActivationError, ResidentStorageClass, ResidentValueBorrow,
    StateMigrationPolicy, activate,
};
use std::collections::BTreeSet;

const SOURCE: &str =
    include_str!("../../../../tests/architecture/resident-activation/n-body-source-v1.mec");

fn main() {
    let catalog = mech_stdlib::source_catalog();
    let (artifact, bytecode) = compile(SOURCE, catalog.clone());
    let decoded = decode_program_artifact_bytecode_v1(&bytecode)
        .expect("n-body bytecode v1 must decode into a ProgramArtifact");

    assert_eq!(artifact.revision(), decoded.revision());
    assert_eq!(
        artifact
            .slots()
            .iter()
            .filter(|slot| slot.role == SlotRole::State)
            .count(),
        2,
    );
    let position = artifact.outputs().first().expect("positions output").source;
    let velocity = artifact
        .slots()
        .iter()
        .find(|slot| slot.role == SlotRole::State && slot.slot != position)
        .expect("velocity state")
        .slot;
    let position_writers = state_writers(&artifact, position);
    let velocity_writers = state_writers(&artifact, velocity);
    assert_eq!(position_writers.len(), 1);
    assert_eq!(velocity_writers.len(), 2);
    assert_rmw_region(&artifact, position, position_writers[0], RegionPolicy::WholeValue);
    for writer in &velocity_writers {
        assert_rmw_region(
            &artifact,
            velocity,
            *writer,
            RegionPolicy::IndexedAxis { axis: 0 },
        );
    }
    assert!(velocity_writers[0].get() < velocity_writers[1].get());
    assert_eq!(
        artifact.slots()[velocity.get() as usize].producer,
        ProducerReference::NodeOutput {
            node: velocity_writers[1],
            output_ordinal: 0,
        }
    );
    assert_eq!(
        artifact.slots()[position.get() as usize].producer,
        ProducerReference::NodeOutput {
            node: position_writers[0],
            output_ordinal: 0,
        }
    );
    let x_writer = &artifact.nodes()[position_writers[0].get() as usize];
    assert!(
        node_inputs(&artifact, x_writer)
            .iter()
            .any(|source| source_reads_state_after(&artifact, *source, velocity, velocity_writers[1]))
    );

    let mut activation_nodes = BTreeSet::new();
    loop {
        let before = activation_nodes.len();
        for node in artifact.nodes() {
            let activation_only = node_inputs(&artifact, node).iter().all(|source| match source {
                ArtifactSource::Constant(_) => true,
                ArtifactSource::Slot(slot) => {
                    let declaration = &artifact.slots()[slot.get() as usize];
                    declaration.role != SlotRole::State
                        && matches!(
                            declaration.producer,
                            ProducerReference::NodeOutput { node, .. }
                                if activation_nodes.contains(&node)
                        )
                }
            });
            if activation_only {
                activation_nodes.insert(node.node);
            }
        }
        if activation_nodes.len() == before {
            break;
        }
    }
    for node in artifact.nodes() {
        let ResolvedOperationContract::Declared(contract) =
            artifact.contracts().get(node.contract).expect("node contract")
        else {
            continue;
        };
        if contract
            .outputs
            .iter()
            .any(|output| matches!(output.construction, OutputConstruction::Build { .. }))
        {
            assert!(
                activation_nodes.contains(&node.node),
                "Build node reached the resident turn graph: {:?} {:?} slots={:?}",
                node.operation,
                node_inputs(&artifact, node),
                node_inputs(&artifact, node)
                    .iter()
                    .filter_map(|source| match source {
                        ArtifactSource::Slot(slot) => Some(&artifact.slots()[slot.get() as usize]),
                        ArtifactSource::Constant(_) => None,
                    })
                    .collect::<Vec<_>>(),
            );
        }
    }
    assert!(!activation_nodes.is_empty());
    assert!(activation_nodes.len() < artifact.nodes().len());
    let opaque = artifact
        .nodes()
        .iter()
        .filter(|node| {
            matches!(
                artifact.contracts().get(node.contract),
                Some(ResolvedOperationContract::LegacyOpaque(_))
            )
        })
        .map(|node| {
            format!(
                "{}/{}",
                node.operation.module_path.join("/"),
                node.operation.operation_name
            )
        })
        .collect::<BTreeSet<_>>();
    assert!(opaque.is_empty(), "opaque n-body operations: {opaque:#?}");

    let mut source_instance = activate(
        mech_core::ReactiveInstanceId::new(0, 0),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .expect("source n-body artifact must activate generically");
    let decoded_instance = activate(
        mech_core::ReactiveInstanceId::new(1, 0),
        &decoded,
        &catalog,
        &ActivationFacts::default(),
    )
    .expect("decoded n-body artifact must activate generically");
    assert_eq!(source_instance.plan.program_revision, decoded_instance.plan.program_revision);
    assert_eq!(source_instance.plan.slots, decoded_instance.plan.slots);
    assert_eq!(source_instance.plan.activation_nodes, decoded_instance.plan.activation_nodes);
    assert_eq!(
        source_instance.plan.topology.word_len(),
        source_instance.plan.nodes.len().div_ceil(64),
    );
    assert!(source_instance.plan.activation_nodes.len() > 32);
    assert_eq!(
        source_instance
            .plan
            .slots
            .iter()
            .filter(|slot| slot.storage == ResidentStorageClass::State)
            .count(),
        2,
    );
    let Some(ResidentValueBorrow::F64 { values, shape }) = source_instance.output_borrow(0) else {
        panic!("positions must be a synchronous f64 resident output")
    };
    assert_eq!((shape.rows, shape.columns, values.len()), (10, 3, 30));
    let copied = source_instance.copied_output(0).expect("copied positions snapshot");
    assert_eq!(copied.schema(), artifact.outputs()[0].schema);

    source_instance
        .reactivate(
            &artifact,
            &catalog,
            &ActivationFacts::default(),
            StateMigrationPolicy::PreserveCompatibleRejectIncompatible,
        )
        .expect("same revision reactivation is a no-op");
    assert_eq!(source_instance.plan.plan_generation, PlanGeneration::ZERO);
    assert_eq!(source_instance.plan.layout_generation, LayoutGeneration::ZERO);

    let same_layout_source = SOURCE.replacen("Δt := 0.01", "Δt := 0.02", 1);
    let (same_layout, _) = compile(&same_layout_source, catalog.clone());
    assert_ne!(artifact.revision(), same_layout.revision());
    source_instance
        .reactivate(
            &same_layout,
            &catalog,
            &ActivationFacts::default(),
            StateMigrationPolicy::PreserveCompatibleRejectIncompatible,
        )
        .expect("compatible state migrates across a same-layout revision");
    assert_eq!(source_instance.plan.plan_generation, PlanGeneration::new(1));
    assert_eq!(source_instance.plan.layout_generation, LayoutGeneration::ZERO);
    assert_eq!(source_instance.output_borrow(0).unwrap().len(), 30);

    let changed_layout_source = SOURCE
        .replacen("1..=10", "1..=9", 1)
        .replacen(
            "planets := [☉ ☿ ♀ ♁ ♂ ♃ ♄ ♅ ♆ ♇]'",
            "planets := [☉ ☿ ♀ ♁ ♂ ♃ ♄ ♅ ♆]'",
            1,
        );
    let (changed_layout, _) = compile(&changed_layout_source, catalog.clone());
    let before_revision = source_instance.plan.program_revision;
    assert!(matches!(
        source_instance.reactivate(
            &changed_layout,
            &catalog,
            &ActivationFacts::default(),
            StateMigrationPolicy::PreserveCompatibleRejectIncompatible,
        ),
        Err(ResidentActivationError::IncompatibleState { .. })
    ));
    assert_eq!(source_instance.plan.program_revision, before_revision);
    source_instance
        .reactivate(
            &changed_layout,
            &catalog,
            &ActivationFacts::default(),
            StateMigrationPolicy::PreserveCompatibleResetIncompatible,
        )
        .expect("explicit reset admits an incompatible state shape");
    assert_eq!(source_instance.plan.plan_generation, PlanGeneration::new(2));
    assert_eq!(source_instance.plan.layout_generation, LayoutGeneration::new(1));
    assert_eq!(source_instance.output_borrow(0).unwrap().len(), 27);
    let ValueData::Matrix(copied_matrix) = copied.data() else {
        panic!("copied output remains an owned matrix after reactivation")
    };
    let SequenceView::F64(copied_values) = copied_matrix.elements() else {
        panic!("copied output remains an owned f64 matrix after reactivation")
    };
    assert_eq!(copied_values.len(), 30);
}

fn compile(source: &str, catalog: std::sync::Arc<mech_core::FunctionCatalog>) -> (ProgramArtifact, Vec<u8>) {
    let mut program = MechProgram::with_function_catalog(MechProgramConfig::default(), catalog);
    program.run_string(source).expect("source must execute");
    program
        .compile_program_product()
        .expect("source must compile into a ProgramArtifact")
        .into_parts()
}

fn node_inputs<'a>(
    artifact: &'a ProgramArtifact,
    node: &mech_engine::NodeDeclaration,
) -> Vec<ArtifactSource> {
    artifact.bindings()[node.input_bindings.start as usize..node.input_bindings.end as usize]
        .iter()
        .map(|binding| match binding {
            BindingDeclaration::Input { source, .. } => *source,
            BindingDeclaration::Output { .. } => unreachable!("input range contains output"),
        })
        .collect()
}

fn state_writers(
    artifact: &ProgramArtifact,
    target: mech_core::CellSlotId,
) -> Vec<mech_core::NodeId> {
    artifact
        .bindings()
        .iter()
        .filter_map(|binding| match binding {
            BindingDeclaration::Output {
                node,
                target: found,
                ..
            } if *found == target => Some(*node),
            _ => None,
        })
        .collect()
}

fn assert_rmw_region(
    artifact: &ProgramArtifact,
    target: mech_core::CellSlotId,
    writer: mech_core::NodeId,
    expected_region: RegionPolicy,
) {
    let node = &artifact.nodes()[writer.get() as usize];
    let ResolvedOperationContract::Declared(contract) =
        artifact.contracts().get(node.contract).expect("writer contract")
    else {
        panic!("state writer is opaque")
    };
    let output = &contract.outputs[0];
    let OutputConstruction::ReadModifyWrite {
        base_input,
        regions,
    } = output.construction
    else {
        panic!("state writer is not RMW")
    };
    assert_eq!(regions, expected_region);
    assert_eq!(output.alias, AliasPolicy::MayAlias { input: base_input });
    assert!(matches!(
        node_inputs(artifact, node)[base_input as usize],
        ArtifactSource::Slot(slot) if slot == target
    ));
}

fn source_reads_state_after(
    artifact: &ProgramArtifact,
    source: ArtifactSource,
    state: mech_core::CellSlotId,
    predecessor: mech_core::NodeId,
) -> bool {
    let ArtifactSource::Slot(slot) = source else {
        return false;
    };
    if slot == state {
        return true;
    }
    if artifact.slots()[slot.get() as usize].role == SlotRole::State {
        return false;
    }
    let ProducerReference::NodeOutput { node, .. } = artifact.slots()[slot.get() as usize].producer
    else {
        return false;
    };
    node.get() > predecessor.get()
        && node_inputs(artifact, &artifact.nodes()[node.get() as usize])
            .iter()
            .any(|source| source_reads_state_after(artifact, *source, state, predecessor))
}
