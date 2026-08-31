use mech_core::snapshot::SequenceView;
use mech_core::{
    AccessMode, AliasPolicy, ApplicationRequirement, ApplicationRequirementId, BindingId,
    BoundResidentKernel, ChangeDetectionPolicy, ConstantStoreBuilder, DeclaredOperationContract,
    DeliveryMode, DimensionExpr, DimensionLifetime, DimensionParameterDeclaration,
    DimensionParameterId, DimensionParameterOrigin, ExecutionResourceRequest, ExternalInteraction,
    FloatWidth, InputId, LayoutGeneration, NodeId,
    ObservationContract, ObservationReplayPolicy, OperationContractTableBuilder,
    OutputConstruction, OutputId, PlanGeneration, RegionPolicy, ResidentKernelBindError,
    ResidentKernelBindRequest, ResidentKernelError, ResidentKernelInputs, ResidentValueKind,
    ResidentValueMut, ResolvedInputPort, ResolvedOperationContract, ResolvedOutputPort, SchemaBody,
    ResourceDelivery, ResourceIntent, SchemaDraft, SchemaTableBuilder, ShapeRule, ValueData,
    ValueDataDraft, ValueDraft,
};
use mech_engine::__resident::{
    ActivatedKernelNode, ActivatedTurnStep, ActivationFacts, ResidentActivationError,
    ResidentStorageClass, ResidentValueBorrow, StateMigrationMapping, StateMigrationPolicy,
    activate,
};
use mech_engine::{
    ApplicationRequirementTable, ArtifactSource, BindingDeclaration, InitializerReference,
    InputDeclaration, NodeDeclaration, OperationReference, OutputDeclaration, ProducerReference,
    ProgramArtifact, ProgramArtifactDraft, SlotDeclaration, SlotRole,
    decode_program_artifact_bytecode_v1, encode_program_artifact_bytecode_v1,
};
use mech_runtime::RuntimeBuilder;
use sha2::{Digest, Sha256};
use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};

mod gate_d;

struct CountingAllocator;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn kernel_step(step: &ActivatedTurnStep) -> &ActivatedKernelNode {
    let ActivatedTurnStep::Kernel(node) = step else {
        panic!("D2 contains only resident kernel turn steps")
    };
    node
}

const SOURCE: &str =
    include_str!("../../../../tests/architecture/resident-activation/n-body-source-v1.mec");

fn main() {
    if std::env::args().any(|argument| argument == "--gate-d-benchmark") {
        gate_d::run();
        return;
    }
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
    assert_rmw_region(
        &artifact,
        position,
        position_writers[0],
        RegionPolicy::WholeValue,
    );
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
            .any(|source| source_reads_state_after(
                &artifact,
                *source,
                velocity,
                velocity_writers[1]
            ))
    );

    let mut activation_nodes = BTreeSet::new();
    loop {
        let before = activation_nodes.len();
        for node in artifact.nodes() {
            let activation_only = node_inputs(&artifact, node)
                .iter()
                .all(|source| match source {
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
        let ResolvedOperationContract::Declared(contract) = artifact
            .contracts()
            .get(node.contract)
            .expect("node contract")
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
    assert!(
        artifact
            .contracts()
            .iter()
            .all(|contract| matches!(contract, ResolvedOperationContract::Declared(_))),
        "every n-body operation must carry a declared contract",
    );

    let mut source_instance = activate(
        mech_core::ReactiveInstanceId::new(0, 0),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .expect("source n-body artifact must activate generically");
    let mut decoded_instance = activate(
        mech_core::ReactiveInstanceId::new(1, 0),
        &decoded,
        &catalog,
        &ActivationFacts::default(),
    )
    .expect("decoded n-body artifact must activate generically");
    assert_eq!(
        source_instance.plan.program_revision,
        decoded_instance.plan.program_revision
    );
    assert_eq!(source_instance.plan.slots, decoded_instance.plan.slots);
    assert_eq!(
        source_instance.plan.activation_nodes,
        decoded_instance.plan.activation_nodes
    );
    assert_eq!(
        source_instance.plan.topology.word_len(),
        source_instance.plan.steps.len().div_ceil(64),
    );
    assert!(source_instance.plan.activation_nodes.len() > 32);
    assert_reordered_artifact_execution(&artifact, &catalog);
    assert_production_dirty_propagation(&artifact);
    assert_explicit_state_migration(&catalog);
    assert_activation_fact_reconfiguration(&catalog);
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
    let copied = source_instance
        .copied_output(0)
        .expect("copied positions snapshot");
    assert_eq!(copied.schema(), artifact.outputs()[0].schema);
    let probe = source_instance.structural_probe();
    assert_eq!(source_instance.state.candidate_bytes(), 480);
    assert_eq!(source_instance.state.dual_payload_bytes(), 960);
    assert_eq!(probe.candidate_seed_bytes, 480);
    assert_eq!(probe.candidate_materialized_bytes, 480);
    assert_eq!(probe.published_buffer_copy_bytes, 0);
    assert_eq!(probe.publication_store_count, 1);
    let initial_x = resident_f64_slot(&source_instance, position);
    let initial_v = resident_f64_slot(&source_instance, velocity);
    let masses = resident_masses(&source_instance);
    let mut raw = RawNbody {
        x: initial_x.clone().try_into().unwrap(),
        v: initial_v.clone().try_into().unwrap(),
        masses,
    };
    let initial_state_hash = exact_state_hash(&initial_x, &initial_v);
    let initial_energy = raw.energy();
    let mut trajectory = Sha256::new();
    for turn in 0..4_096 {
        let source_summary = source_instance.turn(&[]).expect("source n-body turn");
        let decoded_summary = decoded_instance.turn(&[]).expect("decoded n-body turn");
        raw.advance();
        assert_eq!(source_summary.state_hash, decoded_summary.state_hash);
        assert_eq!(source_summary.touched_slots, 2);
        assert_eq!(source_summary.after_epoch, decoded_summary.after_epoch);
        assert_eq!(
            resident_state(&source_instance),
            resident_state(&decoded_instance),
            "turn {turn}"
        );
        let resident_x = resident_f64_slot(&source_instance, position);
        let resident_v = resident_f64_slot(&source_instance, velocity);
        assert_quantized_equal(&resident_x, &raw.x, turn, "x");
        assert_quantized_equal(&resident_v, &raw.v, turn, "v");
        update_quantized(&mut trajectory, &resident_x);
        update_quantized(&mut trajectory, &resident_v);
    }
    let trajectory_sha256 = hex(trajectory.finalize());
    let final_x = resident_f64_slot(&source_instance, position);
    let final_v = resident_f64_slot(&source_instance, velocity);
    // The resident/raw/legacy equivalence contract is intentionally frozen at
    // the same 1e-10 quantization as the trajectory. Exact floating-point bits
    // may differ across supported CPU architectures because LLVM can contract
    // arithmetic differently while preserving that semantic trajectory.
    let final_state_hash = quantized_state_hash(&final_x, &final_v);
    let energy_drift = raw.energy() - initial_energy;
    assert!(
        energy_drift.abs() <= 1.0e-3,
        "the 4,096-turn signed-force trajectory exceeds the frozen absolute energy-drift bound"
    );
    let mut allocation_instance = activate(
        mech_core::ReactiveInstanceId::new(2, 0),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .unwrap();
    ALLOCATIONS.store(0, Ordering::SeqCst);
    for _ in 0..4_096 {
        allocation_instance.turn(&[]).unwrap();
    }
    let steady_state_allocations = ALLOCATIONS.load(Ordering::SeqCst);
    assert_eq!(
        steady_state_allocations,
        0,
        "steady-state resident n-body turns allocate nothing"
    );
    println!(
        "D2_PROJECTION platform={}-{} revision={} artifact_nodes={} activation_nodes={} turn_nodes={} slots={} state_slots=2 velocity_writers={} position_writers={} legacy_opaque=0 unclassified=0 candidate_bytes={} candidate_seed_bytes={} candidate_materialized_bytes={} dual_state_bytes={} publication_stores={} steady_state_allocations={} turns=4096 initial={} trajectory={} final={} energy_drift={energy_drift:.17e} source_bytecode_exact=true raw_exact=true legacy_exact=true stable_topological=true dirty_propagation=true",
        std::env::consts::ARCH,
        std::env::consts::OS,
        hex(artifact.revision().as_bytes()),
        artifact.nodes().len(),
        activation_nodes.len(),
        source_instance.plan.steps.len(),
        artifact.slots().len(),
        velocity_writers.len(),
        position_writers.len(),
        source_instance.state.candidate_bytes(),
        probe.candidate_seed_bytes,
        probe.candidate_materialized_bytes,
        source_instance.state.dual_payload_bytes(),
        probe.publication_store_count,
        steady_state_allocations,
        initial_state_hash,
        trajectory_sha256,
        final_state_hash,
    );

    source_instance
        .reactivate(
            &artifact,
            &catalog,
            &ActivationFacts::default(),
            StateMigrationPolicy::PreserveCompatibleRejectIncompatible,
        )
        .expect("same revision reactivation is a no-op");
    assert_eq!(source_instance.plan.plan_generation, PlanGeneration::ZERO);
    assert_eq!(
        source_instance.plan.layout_generation,
        LayoutGeneration::ZERO
    );

    let same_layout_source = SOURCE.replacen("Δt := 0.01", "Δt := 0.02", 1);
    let (same_layout, _) = compile(&same_layout_source, catalog.clone());
    assert_ne!(artifact.revision(), same_layout.revision());
    let explicit_state_map = [
        StateMigrationMapping {
            source: position,
            target: position,
        },
        StateMigrationMapping {
            source: velocity,
            target: velocity,
        },
    ];
    source_instance
        .reactivate_with_state_map(
            &same_layout,
            &catalog,
            &ActivationFacts::default(),
            StateMigrationPolicy::PreserveCompatibleRejectIncompatible,
            &explicit_state_map,
        )
        .expect("compatible state migrates across a same-layout revision");
    assert_eq!(source_instance.plan.plan_generation, PlanGeneration::new(1));
    assert_eq!(
        source_instance.plan.layout_generation,
        LayoutGeneration::ZERO
    );
    assert_eq!(source_instance.output_borrow(0).unwrap().len(), 30);

    let changed_layout_source = SOURCE.replacen("1..=10", "1..=9", 1).replacen(
        "planets := [☉ ☿ ♀ ♁ ♂ ♃ ♄ ♅ ♆ ♇]'",
        "planets := [☉ ☿ ♀ ♁ ♂ ♃ ♄ ♅ ♆]'",
        1,
    );
    let (changed_layout, _) = compile(&changed_layout_source, catalog.clone());
    let before_revision = source_instance.plan.program_revision;
    assert!(matches!(
        source_instance.reactivate_with_state_map(
            &changed_layout,
            &catalog,
            &ActivationFacts::default(),
            StateMigrationPolicy::PreserveCompatibleRejectIncompatible,
            &explicit_state_map,
        ),
        Err(ResidentActivationError::IncompatibleState { .. })
    ));
    assert_eq!(source_instance.plan.program_revision, before_revision);
    source_instance
        .reactivate_with_state_map(
            &changed_layout,
            &catalog,
            &ActivationFacts::default(),
            StateMigrationPolicy::PreserveCompatibleResetIncompatible,
            &explicit_state_map,
        )
        .expect("explicit reset admits an incompatible state shape");
    assert_eq!(source_instance.plan.plan_generation, PlanGeneration::new(2));
    assert_eq!(
        source_instance.plan.layout_generation,
        LayoutGeneration::new(1)
    );
    assert_eq!(source_instance.output_borrow(0).unwrap().len(), 27);
    let ValueData::Matrix(copied_matrix) = copied.data() else {
        panic!("copied output remains an owned matrix after reactivation")
    };
    let SequenceView::F64(copied_values) = copied_matrix.elements() else {
        panic!("copied output remains an owned f64 matrix after reactivation")
    };
    assert_eq!(copied_values.len(), 30);
}

fn assert_reordered_artifact_execution(
    artifact: &ProgramArtifact,
    catalog: &std::sync::Arc<mech_core::FunctionCatalog>,
) {
    let canonical = activate(
        mech_core::ReactiveInstanceId::new(90, 0),
        artifact,
        catalog,
        &ActivationFacts::default(),
    )
    .expect("activate canonical n-body artifact");
    let activation = canonical
        .plan
        .activation_nodes
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let turn = canonical
        .plan
        .steps
        .iter()
        .map(ActivatedTurnStep::artifact_node)
        .collect::<BTreeSet<_>>();
    let adjacent_edge = |nodes: &BTreeSet<NodeId>| {
        artifact.nodes().iter().find_map(|child| {
            if !nodes.contains(&child.node) {
                return None;
            }
            node_inputs(artifact, child).into_iter().find_map(|source| {
                let ArtifactSource::Slot(slot) = source else {
                    return None;
                };
                let ProducerReference::NodeOutput { node: parent, .. } =
                    artifact.slots()[slot.get() as usize].producer
                else {
                    return None;
                };
                (nodes.contains(&parent) && parent.get() + 1 == child.node.get())
                    .then_some((parent, child.node))
            })
        })
    };
    let activation_edge = adjacent_edge(&activation).expect("adjacent activation dependency");
    let turn_edge = adjacent_edge(&turn).expect("adjacent turn dependency");
    assert!(
        [activation_edge.0, activation_edge.1]
            .iter()
            .all(|node| !turn.contains(node))
    );
    let mut order = (0..artifact.nodes().len()).collect::<Vec<_>>();
    order.swap(
        activation_edge.0.get() as usize,
        activation_edge.1.get() as usize,
    );
    order.swap(turn_edge.0.get() as usize, turn_edge.1.get() as usize);
    let reordered = reorder_artifact(artifact, &order);
    let bytes = encode_program_artifact_bytecode_v1(&reordered)
        .expect("encode physically reordered bytecode-v1 artifact");
    let decoded = decode_program_artifact_bytecode_v1(&bytes)
        .expect("decode physically reordered bytecode-v1 artifact");
    let mut expected = activate(
        mech_core::ReactiveInstanceId::new(91, 0),
        artifact,
        catalog,
        &ActivationFacts::default(),
    )
    .expect("activate canonical comparison artifact");
    let mut source = activate(
        mech_core::ReactiveInstanceId::new(92, 0),
        &reordered,
        catalog,
        &ActivationFacts::default(),
    )
    .expect("activate physically reordered source artifact");
    let mut bytecode = activate(
        mech_core::ReactiveInstanceId::new(93, 0),
        &decoded,
        catalog,
        &ActivationFacts::default(),
    )
    .expect("activate physically reordered decoded artifact");
    expected.turn(&[]).expect("canonical turn");
    source.turn(&[]).expect("reordered source turn");
    bytecode.turn(&[]).expect("reordered bytecode turn");
    assert_eq!(resident_state(&source), resident_state(&expected));
    assert_eq!(resident_state(&bytecode), resident_state(&expected));
    assert!(
        source
            .plan
            .activation_nodes
            .windows(2)
            .any(|nodes| nodes[0].get() > nodes[1].get()),
        "activation executes in dependency order rather than physical node order"
    );
    assert!(
        source
            .plan
            .topology
            .linear_node_order
            .windows(2)
            .any(|nodes| {
                let left = kernel_step(&source.plan.steps[nodes[0].get() as usize]).artifact_node;
                let right = kernel_step(&source.plan.steps[nodes[1].get() as usize]).artifact_node;
                left.get() > right.get()
            }),
        "turn execution order differs from physical node order"
    );
}

fn assert_explicit_state_migration(
    catalog: &std::sync::Arc<mech_core::FunctionCatalog>,
) {
    const BASE: &str = "~a := [1.0, 2.0; 1.5, 2.5]\n~b := [3.0, 4.0; 3.5, 4.5]\na += [1.0, 1.0; 1.0, 1.0]\nb += [1.0, 1.0; 1.0, 1.0]\nout := a\n";
    const SWAPPED: &str = "~b := [3.0, 4.0; 3.5, 4.5]\n~a := [1.0, 2.0; 1.5, 2.5]\na += [1.0, 1.0; 1.0, 1.0]\nb += [1.0, 1.0; 1.0, 1.0]\nout := a\n";
    const INSERTED: &str = "~c := [5.0, 6.0; 5.5, 6.5]\n~a := [1.0, 2.0; 1.5, 2.5]\n~b := [3.0, 4.0; 3.5, 4.5]\nc += [1.0, 1.0; 1.0, 1.0]\na += [1.0, 1.0; 1.0, 1.0]\nb += [1.0, 1.0; 1.0, 1.0]\nout := a\n";
    const DELETED: &str = "~a := [1.0, 2.0; 1.5, 2.5]\na += [1.0, 1.0; 1.0, 1.0]\nout := a\n";

    let (base, _) = compile(BASE, catalog.clone());
    let (swapped, _) = compile(SWAPPED, catalog.clone());
    let (inserted, _) = compile(INSERTED, catalog.clone());
    let (deleted, _) = compile(DELETED, catalog.clone());
    assert_eq!(state_slots(&base).len(), 2);
    assert_eq!(state_slots(&swapped).len(), 2);
    assert_eq!(state_slots(&inserted).len(), 3);
    assert_eq!(state_slots(&deleted).len(), 1);
    assert_bind_rejected(
        &wrong_rmw_base_artifact(&deleted),
        catalog,
        &ActivationFacts::default(),
        ResidentKernelBindError::UnsupportedContract,
    );

    let base_a = state_slot_with_initial(&base, 1.0);
    let base_b = state_slot_with_initial(&base, 3.0);
    let swapped_a = state_slot_with_initial(&swapped, 1.0);
    let swapped_b = state_slot_with_initial(&swapped, 3.0);
    let inserted_a = state_slot_with_initial(&inserted, 1.0);
    let inserted_b = state_slot_with_initial(&inserted, 3.0);
    let inserted_c = state_slot_with_initial(&inserted, 5.0);
    let deleted_a = state_slot_with_initial(&deleted, 1.0);

    let mut instance = activate(
        mech_core::ReactiveInstanceId::new(94, 0),
        &base,
        catalog,
        &ActivationFacts::default(),
    )
    .expect("activate two-state migration fixture");
    instance.turn(&[]).expect("advance migration source state");
    let preserved_a = resident_f64_slot(&instance, base_a);
    let preserved_b = resident_f64_slot(&instance, base_b);
    instance
        .reactivate_with_state_map(
            &swapped,
            catalog,
            &ActivationFacts::default(),
            StateMigrationPolicy::PreserveCompatibleRejectIncompatible,
            &[
                StateMigrationMapping {
                    source: base_a,
                    target: swapped_a,
                },
                StateMigrationMapping {
                    source: base_b,
                    target: swapped_b,
                },
            ],
        )
        .expect("explicit logical mapping survives same-shaped state reordering");
    assert_eq!(resident_f64_slot(&instance, swapped_a), preserved_a);
    assert_eq!(resident_f64_slot(&instance, swapped_b), preserved_b);

    let before_revision = instance.plan.program_revision;
    let before_epoch = instance.published_epoch();
    let before_state = resident_state(&instance);
    assert!(matches!(
        instance.reactivate_with_state_map(
            &inserted,
            catalog,
            &ActivationFacts::default(),
            StateMigrationPolicy::PreserveCompatibleRejectIncompatible,
            &[
                StateMigrationMapping {
                    source: swapped_a,
                    target: inserted_a,
                },
                StateMigrationMapping {
                    source: swapped_b,
                    target: inserted_b,
                },
            ],
        ),
        Err(ResidentActivationError::IncompatibleState { slot }) if slot == inserted_c
    ));
    assert_eq!(instance.plan.program_revision, before_revision);
    assert_eq!(instance.published_epoch(), before_epoch);
    assert_eq!(resident_state(&instance), before_state);
    instance
        .reactivate_with_state_map(
            &inserted,
            catalog,
            &ActivationFacts::default(),
            StateMigrationPolicy::PreserveCompatibleResetIncompatible,
            &[
                StateMigrationMapping {
                    source: swapped_a,
                    target: inserted_a,
                },
                StateMigrationMapping {
                    source: swapped_b,
                    target: inserted_b,
                },
            ],
        )
        .expect("explicit reset initializes a newly inserted state");
    assert_eq!(resident_f64_slot(&instance, inserted_a), preserved_a);
    assert_eq!(resident_f64_slot(&instance, inserted_b), preserved_b);
    assert_eq!(
        resident_f64_slot(&instance, inserted_c),
        vec![5.0, 5.5, 6.0, 6.5]
    );

    instance
        .reactivate_with_state_map(
            &deleted,
            catalog,
            &ActivationFacts::default(),
            StateMigrationPolicy::PreserveCompatibleRejectIncompatible,
            &[StateMigrationMapping {
                source: inserted_a,
                target: deleted_a,
            }],
        )
        .expect("deleting an unmapped old state preserves the remaining logical state");
    assert_eq!(resident_f64_slot(&instance, deleted_a), preserved_a);
    assert_eq!(state_slots(&deleted).len(), 1);
}

fn wrong_rmw_base_artifact(artifact: &ProgramArtifact) -> ProgramArtifact {
    assert_eq!(artifact.nodes().len(), 1);
    let node = &artifact.nodes()[0];
    let ResolvedOperationContract::Declared(mut contract) = artifact
        .contracts()
        .get(node.contract)
        .unwrap()
        .clone()
    else {
        unreachable!()
    };
    contract.outputs[0].construction = OutputConstruction::ReadModifyWrite {
        base_input: 1,
        regions: RegionPolicy::WholeValue,
    };
    contract.outputs[0].alias = AliasPolicy::MayAlias { input: 1 };
    let mut contracts = OperationContractTableBuilder::new();
    let handle = contracts
        .insert(ResolvedOperationContract::Declared(contract))
        .unwrap();
    let contracts = contracts.finish().unwrap();
    let contract = contracts.resolve(handle).unwrap();
    let contracts = contracts.into_parts().0;
    let mut bindings = artifact.bindings().to_vec();
    bindings.swap(0, 1);
    for (ordinal, binding) in bindings[..2].iter_mut().enumerate() {
        let BindingDeclaration::Input {
            id,
            port_ordinal,
            ..
        } = binding
        else {
            unreachable!()
        };
        *id = BindingId::new(ordinal as u32);
        *port_ordinal = ordinal as u16;
    }
    let BindingDeclaration::Output { id, .. } = &mut bindings[2] else {
        unreachable!()
    };
    *id = BindingId::new(2);
    let mut nodes = artifact.nodes().to_vec();
    nodes[0].contract = contract;
    ProgramArtifactDraft {
        schemas: artifact.schemas().clone(),
        constants: artifact.constants().clone(),
        contracts,
        requirements: artifact.requirements().clone(),
        inputs: artifact.inputs().to_vec().into_boxed_slice(),
        slots: artifact.slots().to_vec().into_boxed_slice(),
        nodes: nodes.into_boxed_slice(),
        bindings: bindings.into_boxed_slice(),
        outputs: artifact.outputs().to_vec().into_boxed_slice(),
        constraints: artifact.constraints().to_vec().into_boxed_slice(),
    compute_regions: artifact.compute_regions().to_vec().into_boxed_slice(),
    }
    .finalize()
    .expect("alternate RMW base remains structurally valid")
}

fn assert_activation_fact_reconfiguration(
    catalog: &std::sync::Arc<mech_core::FunctionCatalog>,
) {
    use mech_core::snapshot::{F64Bits, SnapshotValidationContext};
    use mech_core::{CellSlotId, ConstantId};

    let fixed_schema = SchemaDraft {
        dimension_parameters: Box::new([]),
        body: SchemaBody::Matrix {
            element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
            dimensions: vec![DimensionExpr::Constant(2), DimensionExpr::Constant(2)]
                .into_boxed_slice(),
        },
    }
    .finalize()
    .unwrap();
    let dynamic_schema = SchemaDraft {
        dimension_parameters: vec![DimensionParameterDeclaration {
            id: DimensionParameterId::new(0),
            origin: DimensionParameterOrigin::Explicit,
            lifetime: DimensionLifetime::Activation,
            lower_bound: DimensionExpr::Constant(1),
            upper_bound: Some(DimensionExpr::Constant(8)),
        }]
        .into_boxed_slice(),
        body: SchemaBody::Matrix {
            element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
            dimensions: vec![
                DimensionExpr::Parameter(DimensionParameterId::new(0)),
                DimensionExpr::Constant(1),
            ]
            .into_boxed_slice(),
        },
    }
    .finalize()
    .unwrap();
    let mut schema_builder = SchemaTableBuilder::new();
    let fixed_handle = schema_builder.insert(fixed_schema).unwrap();
    let dynamic_handle = schema_builder.insert(dynamic_schema).unwrap();
    let bool_handle = schema_builder
        .insert(
            SchemaDraft {
                dimension_parameters: Box::new([]),
                body: SchemaBody::Bool,
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let schema_build = schema_builder.finish().unwrap();
    let fixed = schema_build.resolve(fixed_handle).unwrap();
    let dynamic = schema_build.resolve(dynamic_handle).unwrap();
    let bool_ = schema_build.resolve(bool_handle).unwrap();
    let (schemas, _) = schema_build.into_parts();

    let fixed_value = ValueDraft {
        schema: fixed,
        shape_values: Box::new([]),
        data: ValueDataDraft::Matrix(
            [1.0, 2.0, 3.0, 4.0]
                .into_iter()
                .map(|value| ValueDataDraft::F64(F64Bits::from_f64(value)))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
    }
    .finalize(&SnapshotValidationContext::new(&schemas))
    .unwrap();
    let mut constants_builder = ConstantStoreBuilder::new(&schemas);
    let constant_handle = constants_builder.insert(fixed_value).unwrap();
    let constants_build = constants_builder.finish().unwrap();
    let constant = constants_build.resolve(constant_handle).unwrap();
    let (constants, _) = constants_build.into_parts();

    let contract = ResolvedOperationContract::Declared(DeclaredOperationContract {
        inputs: vec![ResolvedInputPort {
            schema: fixed,
            access: AccessMode::Read,
            delivery: DeliveryMode::Signal,
        }]
        .into_boxed_slice(),
        outputs: vec![ResolvedOutputPort {
            schema: fixed,
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::SameAsInput { input: 0 },
            },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    });
    let mut contract_builder = OperationContractTableBuilder::new();
    let contract_handle = contract_builder.insert(contract).unwrap();
    let contract_build = contract_builder.finish().unwrap();
    let contract = contract_build.resolve(contract_handle).unwrap();
    let (contracts, _) = contract_build.into_parts();

    let input_slot = CellSlotId::new(0);
    let state_slot = CellSlotId::new(1);
    let artifact = ProgramArtifactDraft {
        schemas,
        constants,
        contracts,
        requirements: ApplicationRequirementTable::empty(),
        inputs: vec![InputDeclaration {
            input: InputId::new(0),
            name: "deployment-vector".to_owned(),
            slot: input_slot,
            schema: dynamic,
        }]
        .into_boxed_slice(),
        slots: vec![
            SlotDeclaration {
                slot: input_slot,
                schema: dynamic,
                role: SlotRole::Input,
                producer: ProducerReference::Input(InputId::new(0)),
                initializer: None,
            },
            SlotDeclaration {
                slot: state_slot,
                schema: fixed,
                role: SlotRole::State,
                producer: ProducerReference::NodeOutput {
                    node: NodeId::new(0),
                    output_ordinal: 0,
                },
                initializer: Some(InitializerReference::Constant(constant)),
            },
        ]
        .into_boxed_slice(),
        nodes: vec![NodeDeclaration {
            node: NodeId::new(0),
            operation: OperationReference {
                module_path: vec!["core".to_owned()].into_boxed_slice(),
                operation_name: "assign".to_owned(),
            },
            contract,
            requirement: None,
            input_bindings: 0..1,
            output_bindings: 1..2,
        }]
        .into_boxed_slice(),
        bindings: vec![
            BindingDeclaration::Input {
                id: BindingId::new(0),
                node: NodeId::new(0),
                port_ordinal: 0,
                source: ArtifactSource::Constant(ConstantId::new(0)),
            },
            BindingDeclaration::Output {
                id: BindingId::new(1),
                node: NodeId::new(0),
                port_ordinal: 0,
                target: state_slot,
            },
        ]
        .into_boxed_slice(),
        outputs: vec![OutputDeclaration {
            output: OutputId::new(0),
            name: "state".to_owned(),
            interactive_binding: None,
            source: state_slot,
            schema: fixed,
        }]
        .into_boxed_slice(),
        constraints: Box::new([]),
    compute_regions: Box::new([]),
    }
    .finalize()
    .expect("build activation-dimension resident artifact");

    let dynamic_schema = artifact.schemas().entry(dynamic).unwrap().schema();
    let facts = |rows| {
        let mut facts = ActivationFacts::default();
        facts.slot_shapes.insert(
            input_slot,
            dynamic_schema
                .instantiate_shape(vec![rows].into_boxed_slice())
                .unwrap(),
        );
        facts
    };
    let mut instance = activate(
        mech_core::ReactiveInstanceId::new(95, 0),
        &artifact,
        catalog,
        &facts(2),
    )
    .expect("activate a constant-driven FullWrite state root");
    assert_eq!(instance.plan.topology.turn_root_nodes.len(), 1);
    assert_eq!(
        kernel_step(&instance.plan.steps[0]).write.storage,
        ResidentStorageClass::State
    );
    instance
        .reactivate(
            &artifact,
            catalog,
            &facts(5),
            StateMigrationPolicy::PreserveCompatibleRejectIncompatible,
        )
        .expect("same artifact with different activation facts rebuilds its layout");
    assert_eq!(instance.plan.plan_generation, PlanGeneration::new(1));
    assert_eq!(
        instance.plan.layout_generation,
        LayoutGeneration::new(1)
    );
    assert_eq!(instance.plan.inputs[0].region.shape.rows, 5);

    let mut invalid = ActivationFacts::default();
    let other_shape = SchemaDraft {
        dimension_parameters: vec![DimensionParameterDeclaration {
            id: DimensionParameterId::new(0),
            origin: DimensionParameterOrigin::Explicit,
            lifetime: DimensionLifetime::Activation,
            lower_bound: DimensionExpr::Constant(1),
            upper_bound: Some(DimensionExpr::Constant(16)),
        }]
        .into_boxed_slice(),
        body: SchemaBody::Matrix {
            element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
            dimensions: vec![
                DimensionExpr::Parameter(DimensionParameterId::new(0)),
                DimensionExpr::Constant(1),
            ]
            .into_boxed_slice(),
        },
    }
    .finalize()
    .unwrap()
    .instantiate_shape(vec![12].into_boxed_slice())
    .unwrap();
    invalid.slot_shapes.insert(input_slot, other_shape);
    assert!(matches!(
        activate(
            mech_core::ReactiveInstanceId::new(96, 0),
            &artifact,
            catalog,
            &invalid,
        ),
        Err(ResidentActivationError::UnresolvedShape { slot }) if slot == input_slot
    ));

    let bytes = encode_program_artifact_bytecode_v1(&artifact).unwrap();
    let decoded = decode_program_artifact_bytecode_v1(&bytes).unwrap();
    let decoded_instance = activate(
        mech_core::ReactiveInstanceId::new(97, 0),
        &decoded,
        catalog,
        &facts(3),
    )
    .expect("decoded constant-driven FullWrite state root activates");
    assert_eq!(decoded_instance.plan.topology.turn_root_nodes.len(), 1);

    let ResolvedOperationContract::Declared(mut observation_contract) = artifact
        .contracts()
        .get(artifact.nodes()[0].contract)
        .unwrap()
        .clone()
    else {
        unreachable!()
    };
    observation_contract.interaction = ExternalInteraction::Observation(ObservationContract {
        replay: ObservationReplayPolicy::CaptureAsInputFact,
    });
    let observation_state = replace_single_contract(
        &artifact,
        ResolvedOperationContract::Declared(observation_contract),
    );
    assert!(matches!(
        activate(
            mech_core::ReactiveInstanceId::new(98, 0),
            &observation_state,
            catalog,
            &facts(2),
        ),
        Err(ResidentActivationError::InvalidNodeOutput { node }) if node == NodeId::new(0)
    ));

    let ResolvedOperationContract::Declared(assign_contract) = artifact
        .contracts()
        .get(artifact.nodes()[0].contract)
        .unwrap()
        .clone()
    else {
        unreachable!()
    };
    let mut wrong_construction = assign_contract.clone();
    wrong_construction.outputs[0].construction = OutputConstruction::FullWrite {
        shape: ShapeRule::Declared,
    };
    assert_bind_rejected(
        &replace_single_contract(
            &artifact,
            ResolvedOperationContract::Declared(wrong_construction),
        ),
        catalog,
        &facts(2),
        ResidentKernelBindError::UnsupportedContract,
    );
    let mut wrong_change = assign_contract.clone();
    wrong_change.outputs[0].change_detection = ChangeDetectionPolicy::AlwaysChanged;
    assert_bind_rejected(
        &replace_single_contract(
            &artifact,
            ResolvedOperationContract::Declared(wrong_change),
        ),
        catalog,
        &facts(2),
        ResidentKernelBindError::UnsupportedContract,
    );

    let wrong_dimensions = wrong_dimension_artifact(&artifact, dynamic);
    assert_bind_rejected(
        &wrong_dimensions,
        catalog,
        &facts(3),
        ResidentKernelBindError::UnsupportedLayout,
    );
    let bool_assignment = canonical_bool_assign_artifact(&artifact, bool_);
    let decoded_bool_assignment = decode_program_artifact_bytecode_v1(
        &encode_program_artifact_bytecode_v1(&bool_assignment).unwrap(),
    )
    .unwrap();
    for (id, candidate) in [&bool_assignment, &decoded_bool_assignment]
        .into_iter()
        .enumerate()
    {
        activate(
            mech_core::ReactiveInstanceId::new(96 + id as u32, 0),
            candidate,
            catalog,
            &ActivationFacts::default(),
        )
        .expect("canonical core/assign supports the declared Bool schema");
    }
    let observation_bytes = encode_program_artifact_bytecode_v1(&observation_state).unwrap();
    let observation_decoded =
        decode_program_artifact_bytecode_v1(&observation_bytes).unwrap();
    assert!(matches!(
        activate(
            mech_core::ReactiveInstanceId::new(99, 0),
            &observation_decoded,
            catalog,
            &facts(2),
        ),
        Err(ResidentActivationError::InvalidNodeOutput { node }) if node == NodeId::new(0)
    ));

    let zero_input = zero_input_state_artifact(&artifact);
    let mut wrong_arity_node = zero_input.nodes()[0].clone();
    wrong_arity_node.operation = OperationReference {
        module_path: vec!["core".to_owned()].into_boxed_slice(),
        operation_name: "assign".to_owned(),
    };
    let wrong_arity = ProgramArtifactDraft {
        schemas: zero_input.schemas().clone(),
        constants: zero_input.constants().clone(),
        contracts: zero_input.contracts().clone(),
        requirements: zero_input.requirements().clone(),
        inputs: Box::new([]),
        slots: zero_input.slots().to_vec().into_boxed_slice(),
        nodes: vec![wrong_arity_node].into_boxed_slice(),
        bindings: zero_input.bindings().to_vec().into_boxed_slice(),
        outputs: zero_input.outputs().to_vec().into_boxed_slice(),
        constraints: Box::new([]),
    compute_regions: Box::new([]),
    }
    .finalize()
    .unwrap();
    assert_bind_rejected(
        &wrong_arity,
        catalog,
        &ActivationFacts::default(),
        ResidentKernelBindError::UnsupportedContract,
    );
    let mut builder = mech_core::FunctionCatalogBuilder::new();
    builder
        .insert_resident_factory(["test"], "zero-input-state", bind_zero_input_state)
        .unwrap();
    let zero_catalog = builder.build().unwrap();
    for (id, candidate) in [
        zero_input.clone(),
        decode_program_artifact_bytecode_v1(
            &encode_program_artifact_bytecode_v1(&zero_input).unwrap(),
        )
        .unwrap(),
    ]
    .iter()
    .enumerate()
    {
        let mut instance = activate(
            mech_core::ReactiveInstanceId::new(100 + id as u32, 0),
            candidate,
            &zero_catalog,
            &ActivationFacts::default(),
        )
        .expect("zero-input state transition activates as a turn root");
        assert_eq!(instance.plan.topology.turn_root_nodes.len(), 1);
        instance.turn(&[]).expect("zero-input state transition runs");
        assert_eq!(
            resident_f64_slot(&instance, candidate.outputs()[0].source),
            vec![9.0; 4]
        );
    }
}

fn replace_single_contract(
    artifact: &ProgramArtifact,
    contract: ResolvedOperationContract,
) -> ProgramArtifact {
    let external = matches!(
        contract,
        ResolvedOperationContract::Declared(DeclaredOperationContract {
            interaction: ExternalInteraction::Observation(_)
                | ExternalInteraction::Effect(_)
                | ExternalInteraction::TransactionalExternal(_),
            ..
        })
    );
    let mut builder = OperationContractTableBuilder::new();
    let handle = builder.insert(contract).unwrap();
    let build = builder.finish().unwrap();
    let contract = build.resolve(handle).unwrap();
    let (contracts, _) = build.into_parts();
    let mut nodes = artifact.nodes().to_vec();
    assert_eq!(nodes.len(), 1);
    nodes[0].contract = contract;
    let requirements = if external {
        nodes[0].requirement = Some(ApplicationRequirementId::new(0));
        ApplicationRequirementTable::from_canonical_entries(vec![ApplicationRequirement::Resource(
            ExecutionResourceRequest {
                base_uri: "gate-d2://contract/probe".to_owned(),
                path: "value".to_owned(),
                context_name: "probe".to_owned(),
                operation: "read".to_owned(),
                intent: ResourceIntent::Read,
                delivery: ResourceDelivery::Live,
            },
        )])
        .unwrap()
    } else {
        artifact.requirements().clone()
    };
    ProgramArtifactDraft {
        schemas: artifact.schemas().clone(),
        constants: artifact.constants().clone(),
        contracts,
        requirements,
        inputs: artifact.inputs().to_vec().into_boxed_slice(),
        slots: artifact.slots().to_vec().into_boxed_slice(),
        nodes: nodes.into_boxed_slice(),
        bindings: artifact.bindings().to_vec().into_boxed_slice(),
        outputs: artifact.outputs().to_vec().into_boxed_slice(),
        constraints: artifact.constraints().to_vec().into_boxed_slice(),
    compute_regions: artifact.compute_regions().to_vec().into_boxed_slice(),
    }
    .finalize()
    .expect("replacement contract remains a structurally valid artifact")
}

fn assert_bind_rejected(
    artifact: &ProgramArtifact,
    catalog: &mech_core::FunctionCatalog,
    facts: &ActivationFacts,
    expected: ResidentKernelBindError,
) {
    let decoded = decode_program_artifact_bytecode_v1(
        &encode_program_artifact_bytecode_v1(artifact).unwrap(),
    )
    .unwrap();
    for (id, candidate) in [artifact, &decoded].into_iter().enumerate() {
        assert!(matches!(
            activate(
                mech_core::ReactiveInstanceId::new(110 + id as u32, 0),
                candidate,
                catalog,
                facts,
            ),
            Err(ResidentActivationError::KernelBind { error, .. }) if error == expected
        ));
    }
}

fn wrong_dimension_artifact(
    template: &ProgramArtifact,
    dynamic: mech_core::SchemaId,
) -> ProgramArtifact {
    use mech_core::snapshot::{F64Bits, SnapshotValidationContext};
    use mech_core::{CellSlotId, ConstantId};

    let value = ValueDraft {
        schema: dynamic,
        shape_values: vec![2].into_boxed_slice(),
        data: ValueDataDraft::Matrix(
            [1.0, 2.0]
                .into_iter()
                .map(|value| ValueDataDraft::F64(F64Bits::from_f64(value)))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
    }
    .finalize(&SnapshotValidationContext::new(template.schemas()))
    .unwrap();
    let mut constants = ConstantStoreBuilder::new(template.schemas());
    constants.insert(value).unwrap();
    let constants = constants.finish().unwrap().into_parts().0;
    let contract = ResolvedOperationContract::Declared(DeclaredOperationContract {
        inputs: vec![
            ResolvedInputPort {
                schema: dynamic,
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            },
            ResolvedInputPort {
                schema: dynamic,
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            },
        ]
        .into_boxed_slice(),
        outputs: vec![ResolvedOutputPort {
            schema: dynamic,
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    });
    let mut contracts = OperationContractTableBuilder::new();
    let contract_handle = contracts.insert(contract).unwrap();
    let contracts = contracts.finish().unwrap();
    let contract = contracts.resolve(contract_handle).unwrap();
    let contracts = contracts.into_parts().0;
    let input = CellSlotId::new(0);
    let state = CellSlotId::new(1);
    ProgramArtifactDraft {
        schemas: template.schemas().clone(),
        constants,
        contracts,
        requirements: template.requirements().clone(),
        inputs: vec![InputDeclaration {
            input: InputId::new(0),
            name: "dynamic".to_owned(),
            slot: input,
            schema: dynamic,
        }]
        .into_boxed_slice(),
        slots: vec![
            SlotDeclaration {
                slot: input,
                schema: dynamic,
                role: SlotRole::Input,
                producer: ProducerReference::Input(InputId::new(0)),
                initializer: None,
            },
            SlotDeclaration {
                slot: state,
                schema: dynamic,
                role: SlotRole::State,
                producer: ProducerReference::NodeOutput {
                    node: NodeId::new(0),
                    output_ordinal: 0,
                },
                initializer: Some(InitializerReference::Constant(ConstantId::new(0))),
            },
        ]
        .into_boxed_slice(),
        nodes: vec![NodeDeclaration {
            node: NodeId::new(0),
            operation: OperationReference {
                module_path: vec!["math".to_owned()].into_boxed_slice(),
                operation_name: "sub".to_owned(),
            },
            contract,
            requirement: None,
            input_bindings: 0..2,
            output_bindings: 2..3,
        }]
        .into_boxed_slice(),
        bindings: vec![
            BindingDeclaration::Input {
                id: BindingId::new(0),
                node: NodeId::new(0),
                port_ordinal: 0,
                source: ArtifactSource::Slot(input),
            },
            BindingDeclaration::Input {
                id: BindingId::new(1),
                node: NodeId::new(0),
                port_ordinal: 1,
                source: ArtifactSource::Constant(ConstantId::new(0)),
            },
            BindingDeclaration::Output {
                id: BindingId::new(2),
                node: NodeId::new(0),
                port_ordinal: 0,
                target: state,
            },
        ]
        .into_boxed_slice(),
        outputs: vec![OutputDeclaration {
            output: OutputId::new(0),
            name: "state".to_owned(),
            interactive_binding: None,
            source: state,
            schema: dynamic,
        }]
        .into_boxed_slice(),
        constraints: Box::new([]),
    compute_regions: Box::new([]),
    }
    .finalize()
    .expect("mismatched activation shapes remain structurally valid")
}

fn canonical_bool_assign_artifact(
    template: &ProgramArtifact,
    bool_: mech_core::SchemaId,
) -> ProgramArtifact {
    use mech_core::snapshot::SnapshotValidationContext;
    use mech_core::{CellSlotId, ConstantId};

    let value = ValueDraft {
        schema: bool_,
        shape_values: Box::new([]),
        data: ValueDataDraft::Bool(false),
    }
    .finalize(&SnapshotValidationContext::new(template.schemas()))
    .unwrap();
    let mut constants = ConstantStoreBuilder::new(template.schemas());
    constants.insert(value).unwrap();
    let constants = constants.finish().unwrap().into_parts().0;
    let contract = ResolvedOperationContract::Declared(DeclaredOperationContract {
        inputs: vec![ResolvedInputPort {
            schema: bool_,
            access: AccessMode::Read,
            delivery: DeliveryMode::Signal,
        }]
        .into_boxed_slice(),
        outputs: vec![ResolvedOutputPort {
            schema: bool_,
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::SameAsInput { input: 0 },
            },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    });
    let mut contracts = OperationContractTableBuilder::new();
    let handle = contracts.insert(contract).unwrap();
    let contracts = contracts.finish().unwrap();
    let contract = contracts.resolve(handle).unwrap();
    let contracts = contracts.into_parts().0;
    let state = CellSlotId::new(0);
    ProgramArtifactDraft {
        schemas: template.schemas().clone(),
        constants,
        contracts,
        requirements: template.requirements().clone(),
        inputs: Box::new([]),
        slots: vec![SlotDeclaration {
            slot: state,
            schema: bool_,
            role: SlotRole::State,
            producer: ProducerReference::NodeOutput {
                node: NodeId::new(0),
                output_ordinal: 0,
            },
            initializer: Some(InitializerReference::Constant(ConstantId::new(0))),
        }]
        .into_boxed_slice(),
        nodes: vec![NodeDeclaration {
            node: NodeId::new(0),
            operation: OperationReference {
                module_path: vec!["core".to_owned()].into_boxed_slice(),
                operation_name: "assign".to_owned(),
            },
            contract,
            requirement: None,
            input_bindings: 0..1,
            output_bindings: 1..2,
        }]
        .into_boxed_slice(),
        bindings: vec![
            BindingDeclaration::Input {
                id: BindingId::new(0),
                node: NodeId::new(0),
                port_ordinal: 0,
                source: ArtifactSource::Constant(ConstantId::new(0)),
            },
            BindingDeclaration::Output {
                id: BindingId::new(1),
                node: NodeId::new(0),
                port_ordinal: 0,
                target: state,
            },
        ]
        .into_boxed_slice(),
        outputs: vec![OutputDeclaration {
            output: OutputId::new(0),
            name: "state".to_owned(),
            interactive_binding: None,
            source: state,
            schema: bool_,
        }]
        .into_boxed_slice(),
        constraints: Box::new([]),
    compute_regions: Box::new([]),
    }
    .finalize()
    .expect("wrong-kind operation artifact remains structurally valid")
}

fn zero_input_state_artifact(artifact: &ProgramArtifact) -> ProgramArtifact {
    let output_schema = artifact.outputs()[0].schema;
    let contract = ResolvedOperationContract::Declared(DeclaredOperationContract {
        inputs: Box::new([]),
        outputs: vec![ResolvedOutputPort {
            schema: output_schema,
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    });
    let mut builder = OperationContractTableBuilder::new();
    let handle = builder.insert(contract).unwrap();
    let build = builder.finish().unwrap();
    let contract = build.resolve(handle).unwrap();
    let (contracts, _) = build.into_parts();
    let original_state_slot = artifact.outputs()[0].source;
    let state_slot = mech_core::CellSlotId::new(0);
    let mut state = artifact.slots()[original_state_slot.get() as usize].clone();
    state.slot = state_slot;
    state.producer = ProducerReference::NodeOutput {
        node: NodeId::new(0),
        output_ordinal: 0,
    };
    let mut output = artifact.outputs()[0].clone();
    output.source = state_slot;
    ProgramArtifactDraft {
        schemas: artifact.schemas().clone(),
        constants: artifact.constants().clone(),
        contracts,
        requirements: artifact.requirements().clone(),
        inputs: Box::new([]),
        slots: vec![state].into_boxed_slice(),
        nodes: vec![NodeDeclaration {
            node: NodeId::new(0),
            operation: OperationReference {
                module_path: vec!["test".to_owned()].into_boxed_slice(),
                operation_name: "zero-input-state".to_owned(),
            },
            contract,
            requirement: None,
            input_bindings: 0..0,
            output_bindings: 0..1,
        }]
        .into_boxed_slice(),
        bindings: vec![BindingDeclaration::Output {
            id: BindingId::new(0),
            node: NodeId::new(0),
            port_ordinal: 0,
            target: state_slot,
        }]
        .into_boxed_slice(),
        outputs: vec![output].into_boxed_slice(),
        constraints: artifact.constraints().to_vec().into_boxed_slice(),
    compute_regions: artifact.compute_regions().to_vec().into_boxed_slice(),
    }
    .finalize()
    .expect("zero-input state artifact is structurally valid")
}

fn bind_zero_input_state(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let ResolvedOperationContract::Declared(contract) = request.contract else {
        return Err(ResidentKernelBindError::UnsupportedContract);
    };
    if !request.inputs.is_empty()
        || !contract.inputs.is_empty()
        || contract.interaction != ExternalInteraction::Pure
        || contract.outputs.len() != 1
        || request.output.kind != ResidentValueKind::F64
    {
        return Err(ResidentKernelBindError::UnsupportedContract);
    }
    Ok(BoundResidentKernel::new(
        execute_zero_input_state,
        Box::new([]),
    ))
}

fn execute_zero_input_state(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if !inputs.is_empty() {
        return Err(ResidentKernelError::InvalidInput);
    }
    let ResidentValueMut::F64(output) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    output.fill(9.0);
    Ok(true)
}

fn assert_production_dirty_propagation(template: &ProgramArtifact) {
    let mut builder = mech_core::FunctionCatalogBuilder::new();
    builder
        .insert_resident_factory(["test"], "reported-copy", bind_reported_copy)
        .unwrap();
    builder
        .insert_resident_factory(["test"], "always-copy", bind_always_copy)
        .unwrap();
    builder
        .insert_resident_factory(["test"], "state-copy", bind_reported_copy)
        .unwrap();
    let catalog = builder.build().unwrap();

    for (policy, root_operation, expected_second) in [
        (ChangeDetectionPolicy::KernelReported, "reported-copy", 1),
        (ChangeDetectionPolicy::AlwaysChanged, "always-copy", 2),
    ] {
        let source = dirty_propagation_artifact(template, policy, root_operation);
        let decoded = decode_program_artifact_bytecode_v1(
            &encode_program_artifact_bytecode_v1(&source).unwrap(),
        )
        .unwrap();
        for (route, artifact) in [("source", &source), ("bytecode", &decoded)] {
            let mut instance = activate(
                mech_core::ReactiveInstanceId::new(
                    120 + u32::from(policy == ChangeDetectionPolicy::AlwaysChanged),
                    u32::from(route == "bytecode"),
                ),
                artifact,
                &catalog,
                &ActivationFacts::default(),
            )
            .unwrap_or_else(|error| panic!("{route} dirty fixture activation failed: {error:?}"));
            let first = instance.turn(&[]).expect("first dirty fixture turn");
            let second = instance.turn(&[]).expect("second dirty fixture turn");
            assert_eq!(first.dirty_nodes, 2, "{route} first turn initializes the chain");
            assert_eq!(
                second.dirty_nodes, expected_second,
                "{route} second turn follows the declared change policy"
            );
        }
    }
}

fn dirty_propagation_artifact(
    template: &ProgramArtifact,
    root_policy: ChangeDetectionPolicy,
    root_operation: &str,
) -> ProgramArtifact {
    let original_state = template.outputs()[0].source;
    let schema = template.outputs()[0].schema;
    let mut contracts = OperationContractTableBuilder::new();
    let root_contract = contracts
        .insert(copy_contract(schema, root_policy))
        .unwrap();
    let state_contract = contracts
        .insert(copy_contract(schema, ChangeDetectionPolicy::KernelReported))
        .unwrap();
    let build = contracts.finish().unwrap();
    let root_contract = build.resolve(root_contract).unwrap();
    let state_contract = build.resolve(state_contract).unwrap();
    let (contracts, _) = build.into_parts();
    let scratch = mech_core::CellSlotId::new(0);
    let state = mech_core::CellSlotId::new(1);
    let mut state_declaration = template.slots()[original_state.get() as usize].clone();
    state_declaration.slot = state;
    state_declaration.producer = ProducerReference::NodeOutput {
        node: NodeId::new(1),
        output_ordinal: 0,
    };
    let mut output = template.outputs()[0].clone();
    output.source = state;
    ProgramArtifactDraft {
        schemas: template.schemas().clone(),
        constants: template.constants().clone(),
        contracts,
        requirements: template.requirements().clone(),
        inputs: Box::new([]),
        slots: vec![
            SlotDeclaration {
                slot: scratch,
                schema,
                role: SlotRole::Derived,
                producer: ProducerReference::NodeOutput {
                    node: NodeId::new(0),
                    output_ordinal: 0,
                },
                initializer: None,
            },
            state_declaration,
        ]
        .into_boxed_slice(),
        nodes: vec![
            NodeDeclaration {
                node: NodeId::new(0),
                operation: OperationReference {
                    module_path: vec!["test".to_owned()].into_boxed_slice(),
                    operation_name: root_operation.to_owned(),
                },
                contract: root_contract,
                requirement: None,
                input_bindings: 0..1,
                output_bindings: 1..2,
            },
            NodeDeclaration {
                node: NodeId::new(1),
                operation: OperationReference {
                    module_path: vec!["test".to_owned()].into_boxed_slice(),
                    operation_name: "state-copy".to_owned(),
                },
                contract: state_contract,
                requirement: None,
                input_bindings: 2..3,
                output_bindings: 3..4,
            },
        ]
        .into_boxed_slice(),
        bindings: vec![
            BindingDeclaration::Input {
                id: BindingId::new(0),
                node: NodeId::new(0),
                port_ordinal: 0,
                source: ArtifactSource::Slot(state),
            },
            BindingDeclaration::Output {
                id: BindingId::new(1),
                node: NodeId::new(0),
                port_ordinal: 0,
                target: scratch,
            },
            BindingDeclaration::Input {
                id: BindingId::new(2),
                node: NodeId::new(1),
                port_ordinal: 0,
                source: ArtifactSource::Slot(scratch),
            },
            BindingDeclaration::Output {
                id: BindingId::new(3),
                node: NodeId::new(1),
                port_ordinal: 0,
                target: state,
            },
        ]
        .into_boxed_slice(),
        outputs: vec![output].into_boxed_slice(),
        constraints: Box::new([]),
    compute_regions: Box::new([]),
    }
    .finalize()
    .expect("dirty-propagation artifact is structurally valid")
}

fn copy_contract(
    schema: mech_core::SchemaId,
    change_detection: ChangeDetectionPolicy,
) -> ResolvedOperationContract {
    ResolvedOperationContract::Declared(DeclaredOperationContract {
        inputs: vec![ResolvedInputPort {
            schema,
            access: AccessMode::Read,
            delivery: DeliveryMode::Signal,
        }]
        .into_boxed_slice(),
        outputs: vec![ResolvedOutputPort {
            schema,
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::SameAsInput { input: 0 },
            },
            alias: AliasPolicy::NoAlias,
            change_detection,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    })
}

fn bind_reported_copy(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_test_copy(request, ChangeDetectionPolicy::KernelReported)
}

fn bind_always_copy(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_test_copy(request, ChangeDetectionPolicy::AlwaysChanged)
}

fn bind_test_copy(
    request: &ResidentKernelBindRequest<'_>,
    expected_change: ChangeDetectionPolicy,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let ResolvedOperationContract::Declared(contract) = request.contract else {
        return Err(ResidentKernelBindError::UnsupportedContract);
    };
    if request.inputs.len() != 1
        || contract.inputs.len() != 1
        || contract.outputs.len() != 1
        || contract.outputs[0].change_detection != expected_change
        || request.inputs[0].kind != ResidentValueKind::F64
        || request.output.kind != ResidentValueKind::F64
        || request.inputs[0].shape != request.output.shape
    {
        return Err(ResidentKernelBindError::UnsupportedContract);
    }
    Ok(BoundResidentKernel::new(execute_test_copy, Box::new([])))
}

fn execute_test_copy(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let Some(mech_core::ResidentValueRef::F64(input)) = inputs.get(0) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let ResidentValueMut::F64(output) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    if input.len() != output.len() {
        return Err(ResidentKernelError::InvalidShape);
    }
    let changed = input
        .iter()
        .zip(output.iter())
        .any(|(left, right)| left.to_bits() != right.to_bits());
    output.copy_from_slice(input);
    Ok(changed)
}

fn state_slots(artifact: &ProgramArtifact) -> Vec<mech_core::CellSlotId> {
    artifact
        .slots()
        .iter()
        .filter(|slot| slot.role == SlotRole::State)
        .map(|slot| slot.slot)
        .collect()
}

fn state_slot_with_initial(artifact: &ProgramArtifact, first: f64) -> mech_core::CellSlotId {
    artifact
        .slots()
        .iter()
        .find_map(|slot| {
            let mech_engine::InitializerReference::Constant(constant) = slot.initializer?;
            let value = artifact.constants().get(constant)?;
            let ValueData::Matrix(matrix) = value.data() else {
                return None;
            };
            let SequenceView::F64(values) = matrix.elements() else {
                return None;
            };
            (slot.role == SlotRole::State
                && values.first().is_some_and(|value| value.to_f64() == first))
            .then_some(slot.slot)
        })
        .expect("logical state is identified by its frozen initializer")
}

fn reorder_artifact(artifact: &ProgramArtifact, order: &[usize]) -> ProgramArtifact {
    assert_eq!(order.len(), artifact.nodes().len());
    let mut old_to_new = vec![NodeId::new(0); order.len()];
    for (new, old) in order.iter().copied().enumerate() {
        old_to_new[old] = NodeId::new(new as u32);
    }
    let mut bindings = Vec::with_capacity(artifact.bindings().len());
    let mut nodes = Vec::with_capacity(order.len());
    for (new, old) in order.iter().copied().enumerate() {
        let original = &artifact.nodes()[old];
        let node = NodeId::new(new as u32);
        let input_start = bindings.len() as u32;
        for binding in &artifact.bindings()
            [original.input_bindings.start as usize..original.input_bindings.end as usize]
        {
            let BindingDeclaration::Input {
                port_ordinal,
                source,
                ..
            } = binding
            else {
                panic!("input binding range contains only inputs")
            };
            bindings.push(BindingDeclaration::Input {
                id: BindingId::new(bindings.len() as u32),
                node,
                port_ordinal: *port_ordinal,
                source: *source,
            });
        }
        let input_end = bindings.len() as u32;
        let output_start = input_end;
        for binding in &artifact.bindings()
            [original.output_bindings.start as usize..original.output_bindings.end as usize]
        {
            let BindingDeclaration::Output {
                port_ordinal,
                target,
                ..
            } = binding
            else {
                panic!("output binding range contains only outputs")
            };
            bindings.push(BindingDeclaration::Output {
                id: BindingId::new(bindings.len() as u32),
                node,
                port_ordinal: *port_ordinal,
                target: *target,
            });
        }
        nodes.push(mech_engine::NodeDeclaration {
            node,
            operation: original.operation.clone(),
            contract: original.contract,
            requirement: original.requirement,
            input_bindings: input_start..input_end,
            output_bindings: output_start..bindings.len() as u32,
        });
    }
    let slots = artifact
        .slots()
        .iter()
        .cloned()
        .map(|mut slot| {
            if let ProducerReference::NodeOutput {
                node,
                output_ordinal,
            } = slot.producer
            {
                slot.producer = ProducerReference::NodeOutput {
                    node: old_to_new[node.get() as usize],
                    output_ordinal,
                };
            }
            slot
        })
        .collect::<Vec<_>>();
    ProgramArtifactDraft {
        schemas: artifact.schemas().clone(),
        constants: artifact.constants().clone(),
        contracts: artifact.contracts().clone(),
        requirements: artifact.requirements().clone(),
        inputs: artifact.inputs().to_vec().into_boxed_slice(),
        slots: slots.into_boxed_slice(),
        nodes: nodes.into_boxed_slice(),
        bindings: bindings.into_boxed_slice(),
        outputs: artifact.outputs().to_vec().into_boxed_slice(),
        constraints: artifact.constraints().to_vec().into_boxed_slice(),
    compute_regions: artifact.compute_regions().to_vec().into_boxed_slice(),
    }
    .finalize()
    .expect("physically reordered acyclic artifact remains valid")
}

fn resident_state(instance: &mech_engine::__resident::ReactiveInstance) -> Vec<u64> {
    instance
        .plan
        .slots
        .iter()
        .filter(|slot| slot.storage == ResidentStorageClass::State)
        .flat_map(
            |slot| match instance.state_borrow(slot.artifact_id).unwrap() {
                ResidentValueBorrow::F64 { values, .. } => values
                    .iter()
                    .map(|value| value.to_bits())
                    .collect::<Vec<_>>(),
                _ => panic!("n-body state is f64"),
            },
        )
        .collect()
}

fn resident_f64_slot(
    instance: &mech_engine::__resident::ReactiveInstance,
    slot: mech_core::CellSlotId,
) -> Vec<f64> {
    let ResidentValueBorrow::F64 { values, .. } = instance.state_borrow(slot).unwrap() else {
        panic!("n-body state is f64")
    };
    values.to_vec()
}

fn resident_masses(instance: &mech_engine::__resident::ReactiveInstance) -> [f64; 10] {
    for slot in instance.plan.slots.iter().filter(|slot| {
        slot.storage == ResidentStorageClass::Constant
            && slot.region.kind == mech_core::ResidentValueKind::F64
            && slot.region.shape.rows == 10
            && slot.region.shape.columns == 1
    }) {
        let values = &instance.activation.f64_storage()
            [slot.region.offset..slot.region.offset + slot.region.len];
        if values.first().is_some_and(|value| *value > 30.0)
            && values.iter().all(|value| value.is_finite() && *value > 0.0)
        {
            return values.try_into().unwrap();
        }
    }
    panic!("mechanically compiled mass vector is present in activation storage")
}

#[derive(Clone)]
struct RawNbody {
    x: [f64; 30],
    v: [f64; 30],
    masses: [f64; 10],
}

impl RawNbody {
    fn advance(&mut self) {
        let mut pairs = [(0usize, 0usize, [0.0; 3], 0.0); 45];
        let mut ordinal = 0;
        for left in 0..10 {
            for right in left + 1..10 {
                let delta = core::array::from_fn(|axis| {
                    self.x[left + axis * 10] - self.x[right + axis * 10]
                });
                let distance_squared = delta.iter().map(|value| value.powf(2.0)).sum::<f64>();
                pairs[ordinal] = (left, right, delta, 0.01 * distance_squared.powf(-1.5));
                ordinal += 1;
            }
        }
        for (left, right, delta, magnitude) in pairs {
            for axis in 0..3 {
                self.v[left + axis * 10] -= delta[axis] * self.masses[right] * magnitude;
            }
        }
        for (left, right, delta, magnitude) in pairs {
            for axis in 0..3 {
                self.v[right + axis * 10] += delta[axis] * self.masses[left] * magnitude;
            }
        }
        for index in 0..30 {
            self.x[index] += self.v[index] * 0.01;
        }
    }

    fn energy(&self) -> f64 {
        let kinetic = (0..10)
            .map(|body| {
                0.5 * self.masses[body]
                    * (0..3)
                        .map(|axis| self.v[body + axis * 10].powi(2))
                        .sum::<f64>()
            })
            .sum::<f64>();
        let potential = (0..10)
            .flat_map(|left| (left + 1..10).map(move |right| (left, right)))
            .map(|(left, right)| {
                let distance = (0..3)
                    .map(|axis| (self.x[left + axis * 10] - self.x[right + axis * 10]).powi(2))
                    .sum::<f64>()
                    .sqrt();
                self.masses[left] * self.masses[right] / distance
            })
            .sum::<f64>();
        kinetic - potential
    }
}

fn assert_quantized_equal(left: &[f64], right: &[f64], turn: usize, lane: &str) {
    assert_eq!(left.len(), right.len());
    for (index, (left, right)) in left.iter().zip(right).enumerate() {
        assert_eq!(
            quantize(*left),
            quantize(*right),
            "{lane} differs at turn {turn}, element {index}: {left:?} != {right:?}"
        );
    }
}

fn quantize(value: f64) -> i64 {
    (value / 1.0e-10).round() as i64
}

fn update_quantized(hash: &mut Sha256, values: &[f64]) {
    for value in values {
        hash.update(quantize(*value).to_le_bytes());
    }
}

fn exact_state_hash(x: &[f64], v: &[f64]) -> String {
    let mut hash = Sha256::new();
    for value in x.iter().chain(v) {
        hash.update(value.to_bits().to_le_bytes());
    }
    hex(hash.finalize())
}

fn quantized_state_hash(x: &[f64], v: &[f64]) -> String {
    let mut hash = Sha256::new();
    update_quantized(&mut hash, x);
    update_quantized(&mut hash, v);
    hex(hash.finalize())
}

fn hex(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn compile(
    source: &str,
    catalog: std::sync::Arc<mech_core::FunctionCatalog>,
) -> (ProgramArtifact, Vec<u8>) {
    RuntimeBuilder::new()
        .function_catalog(catalog)
        .build_compiler()
        .expect("source compiler construction failed")
        .compile_source(source)
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
    let ResolvedOperationContract::Declared(contract) = artifact
        .contracts()
        .get(node.contract)
        .expect("writer contract")
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
