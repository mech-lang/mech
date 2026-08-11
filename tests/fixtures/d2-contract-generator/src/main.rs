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
use sha2::{Digest, Sha256};
use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};

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
    let mut decoded_instance = activate(
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
    let final_state_hash = exact_state_hash(&final_x, &final_v);
    let energy_drift = raw.energy() - initial_energy;
    assert_legacy_trajectory(&catalog, &raw.masses, &trajectory_sha256);
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
    assert_eq!(
        ALLOCATIONS.load(Ordering::SeqCst),
        0,
        "steady-state resident n-body turns allocate nothing"
    );
    println!(
        "d2-nbody initial={initial_state_hash} trajectory={trajectory_sha256} final={final_state_hash} energy_drift={energy_drift:.17e}"
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

fn resident_state(instance: &mech_engine::__resident::ReactiveInstance) -> Vec<u64> {
    instance
        .plan
        .slots
        .iter()
        .filter(|slot| slot.storage == ResidentStorageClass::State)
        .flat_map(|slot| match instance.state_borrow(slot.artifact_id).unwrap() {
            ResidentValueBorrow::F64 { values, .. } => {
                values.iter().map(|value| value.to_bits()).collect::<Vec<_>>()
            }
            _ => panic!("n-body state is f64"),
        })
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
    for slot in instance
        .plan
        .slots
        .iter()
        .filter(|slot| {
            slot.storage == ResidentStorageClass::Constant
                && slot.region.kind == mech_core::ResidentValueKind::F64
                && slot.region.shape.rows == 10
                && slot.region.shape.columns == 1
        })
    {
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
                let squared = core::array::from_fn(|axis| {
                    let delta = self.x[left + axis * 10] - self.x[right + axis * 10];
                    delta.powf(2.0)
                });
                let distance = squared.iter().sum::<f64>();
                pairs[ordinal] = (left, right, squared, 0.01 * distance.powf(-1.5));
                ordinal += 1;
            }
        }
        for (left, right, squared, magnitude) in pairs {
            for axis in 0..3 {
                self.v[left + axis * 10] -= squared[axis] * self.masses[right] * magnitude;
            }
        }
        for (left, right, squared, magnitude) in pairs {
            for axis in 0..3 {
                self.v[right + axis * 10] += squared[axis] * self.masses[left] * magnitude;
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
                    .map(|axis| {
                        (self.x[left + axis * 10] - self.x[right + axis * 10]).powi(2)
                    })
                    .sum::<f64>()
                    .sqrt();
                self.masses[left] * self.masses[right] / distance
            })
            .sum::<f64>();
        kinetic - potential
    }
}

fn assert_legacy_trajectory(
    catalog: &std::sync::Arc<mech_core::FunctionCatalog>,
    masses: &[f64; 10],
    expected_hash: &str,
) {
    let resident_initial = compile_initial_state(catalog);
    let mut legacy =
        MechProgram::with_function_catalog(MechProgramConfig::default(), catalog.clone());
    legacy
        .run_string(SOURCE)
        .expect("ordinary legacy n-body source executes its numerical closure");
    let mut reference = RawNbody {
        x: resident_initial.0,
        v: resident_initial.1,
        masses: *masses,
    };
    reference.advance();
    let mut trajectory = Sha256::new();
    for turn in 0..4_096 {
        if turn != 0 {
            let plan = legacy.interpreter().plan();
            let plan = plan.0.borrow_mut();
            for step in 172..=192 {
                plan[step]
                    .solve_result()
                    .expect("execute one legacy n-body turn-plan step");
            }
            reference.advance();
        }
        let x = initial_legacy_axis(&legacy, "x");
        let v = initial_legacy_axis(&legacy, "v");
        assert_quantized_equal(&x, &reference.x, turn, "legacy x");
        assert_quantized_equal(&v, &reference.v, turn, "legacy v");
        update_quantized(&mut trajectory, &x);
        update_quantized(&mut trajectory, &v);
    }
    assert_eq!(hex(trajectory.finalize()), expected_hash);
}

fn compile_initial_state(
    catalog: &std::sync::Arc<mech_core::FunctionCatalog>,
) -> ([f64; 30], [f64; 30]) {
    let (artifact, _) = compile(SOURCE, catalog.clone());
    let instance = activate(
        mech_core::ReactiveInstanceId::new(99, 0),
        &artifact,
        catalog,
        &ActivationFacts::default(),
    )
    .unwrap();
    let x = artifact.outputs()[0].source;
    let v = artifact
        .slots()
        .iter()
        .find(|slot| slot.role == SlotRole::State && slot.slot != x)
        .unwrap()
        .slot;
    (
        resident_f64_slot(&instance, x).try_into().unwrap(),
        resident_f64_slot(&instance, v).try_into().unwrap(),
    )
}

fn initial_legacy_axis(program: &MechProgram, name: &str) -> [f64; 30] {
    program
        .root_symbol_value(name)
        .unwrap()
        .as_vecf64()
        .expect("legacy n-body state is f64")
        .try_into()
        .unwrap()
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

fn hex(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
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
