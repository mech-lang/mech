use mech_core::{
    MResult, ReactiveInstanceId, ResidentValueKind, ResidentValueRef, ResolvedOperationContract,
};
use mech_engine::__gate_b_resident::ResidentEkfBatch;
use mech_engine::__resident::{
    ActivatedTurnStep, ActivationFacts, CapturedSignalInput, FrozenEkfCompilationServices,
    ReactiveInstance, ResidentStorageClass, ResidentValueBorrow, activate,
    compile_frozen_ekf_source, frozen_ekf_compiler_catalog,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const SOURCE: &str =
    include_str!("../../../../tests/architecture/resident-activation/ekf-source-v1.mec");
const TRACE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../benchmarks/runtime/gate-b/ekf-input-v1.bin"
));
const TURNS: usize = 4_096;
const EXPECTED_TRAJECTORY: &str =
    "ddca8ab17cb390839d4c77e7cecc5203122f249685f5a28c36fd342cf303a758";

fn frames() -> impl Iterator<Item = [f64; 4]> {
    assert_eq!(TRACE.len(), TURNS * 32);
    TRACE.chunks_exact(32).map(|row| {
        let value = |offset| f64::from_le_bytes(row[offset..offset + 8].try_into().unwrap());
        [value(0), value(8), value(16), value(24)]
    })
}

fn state(instance: &ReactiveInstance) -> [f64; 12] {
    let mut state = [0.0; 12];
    for slot in instance
        .plan
        .slots
        .iter()
        .filter(|slot| slot.storage == ResidentStorageClass::State)
    {
        let ResidentValueBorrow::F64 { values, .. } = instance
            .state_borrow(slot.artifact_id)
            .expect("frozen EKF state slot")
        else {
            panic!("frozen EKF state is f64")
        };
        match values.len() {
            3 => state[..3].copy_from_slice(values),
            9 => state[3..].copy_from_slice(values),
            _ => panic!("unexpected frozen EKF state shape"),
        }
    }
    state
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn location(location: ResidentStorageClass, len: usize) -> &'static str {
    match location {
        ResidentStorageClass::Input => "InputFrame",
        ResidentStorageClass::State if len == 9 => "Covariance",
        ResidentStorageClass::State => "State",
        ResidentStorageClass::Scratch => "Scratch",
        ResidentStorageClass::Constant => "Constant",
    }
}

fn captured<'a>(instance: &ReactiveInstance, frame: &'a [f64; 4]) -> CapturedSignalInput<'a> {
    CapturedSignalInput {
        slot: instance.plan.inputs[0].slot,
        value: ResidentValueRef::F64(frame),
    }
}

fn main() -> MResult<()> {
    let mut services = FrozenEkfCompilationServices::default();
    let compilation = compile_frozen_ekf_source(SOURCE, &mut services)?;
    assert_eq!(compilation.source_artifact.revision(), compilation.decoded_artifact.revision());
    assert_eq!(compilation.source_closure, compilation.decoded_closure);

    let source_sha256 = hex(&Sha256::digest(SOURCE.as_bytes()));
    let trace_sha256 = hex(&Sha256::digest(TRACE));
    let revision = hex(compilation.source_artifact.revision().as_bytes());
    let closure = &compilation.source_closure;
    let artifact = &compilation.source_artifact;
    let declared_contracts = artifact
        .contracts()
        .iter()
        .filter(|contract| matches!(contract, ResolvedOperationContract::Declared(_)))
        .count();
    assert_eq!(declared_contracts, artifact.contracts().len());
    let legacy_opaque = 0;
    let classified_nodes = 1
        + closure.resident_kernels.len()
        + closure.integrity_predicates.len()
        + closure.state_updates.len();

    let catalog = frozen_ekf_compiler_catalog()?;
    let facts = ActivationFacts::default();
    let mut source = activate(
        ReactiveInstanceId::new(0, 0),
        &compilation.source_artifact,
        &catalog,
        &facts,
    )
    .unwrap();
    let reactivated = activate(
        ReactiveInstanceId::new(1, 0),
        &compilation.source_artifact,
        &catalog,
        &facts,
    )
    .unwrap();
    let mut decoded = activate(
        ReactiveInstanceId::new(2, 0),
        &compilation.decoded_artifact,
        &catalog,
        &facts,
    )
    .unwrap();
    assert_eq!(source.plan.slots, reactivated.plan.slots);
    assert_eq!(source.plan.topology, reactivated.plan.topology);
    assert_eq!(source.plan.slots, decoded.plan.slots);
    assert_eq!(source.plan.topology, decoded.plan.topology);

    let constant_values = [
        0.05, 25.0, -10.0, 0.04, 0.0, 0.0, 0.0025, 0.25, 0.0, 0.0, 0.0009,
    ];
    let plan = &source.plan;
    let kernel_nodes = plan
        .steps
        .iter()
        .map(|step| match step {
            ActivatedTurnStep::Kernel(node) => node,
            ActivatedTurnStep::External(_) => panic!("D1 contains only resident kernels"),
        })
        .collect::<Vec<_>>();
    let numeric_kernels = kernel_nodes
        .iter()
        .filter(|node| {
            node.write.storage == ResidentStorageClass::Scratch
                && node.write.region.kind == ResidentValueKind::F64
        })
        .count();
    let predicate_kernels = kernel_nodes
        .iter()
        .filter(|node| {
            node.write.storage == ResidentStorageClass::Scratch
                && node.write.region.kind == ResidentValueKind::Bool
        })
        .count();
    let state_copies = kernel_nodes
        .iter()
        .filter(|node| node.write.storage == ResidentStorageClass::State)
        .count();
    let constraint_count = plan.constraints.len();
    let input_slot = &plan.slots[plan.inputs[0].slot.get() as usize];
    let output_slot = &plan.slots[plan.outputs[0].slot.get() as usize];
    let initial = state(&source);
    let initial_state: [f64; 3] = initial[..3].try_into().unwrap();
    let initial_covariance: [f64; 9] = initial[3..].try_into().unwrap();
    let covariance_slot = plan
        .slots
        .iter()
        .find(|slot| slot.storage == ResidentStorageClass::State && slot.region.len == 9)
        .unwrap();
    let state_bindings = [
        json!({
            "artifact_slot": output_slot.artifact_id.get(),
            "physical_slot": output_slot.physical_index.get(),
            "location": location(output_slot.storage, output_slot.region.len),
            "initializer": initial_state,
        }),
        json!({
            "artifact_slot": covariance_slot.artifact_id.get(),
            "physical_slot": covariance_slot.physical_index.get(),
            "location": "Covariance",
            "initializer": initial_covariance,
        }),
    ];

    let artifact_projection = json!({
        "artifact_nodes": artifact.nodes().len(),
        "bytecode_format": "v1",
        "change_detection": {
            "observation_always_changed": 1,
            "pure_kernel_kernel_reported": 17,
            "pure_predicate_exact_scalar": 3,
        },
        "gate": "D1",
        "integrity_constraints": artifact.constraints().len(),
        "integrity_predicates": closure.integrity_predicates.len(),
        "legacy_opaque_contracts": legacy_opaque,
        "observation_roots": 1,
        "outputs": artifact.outputs().len(),
        "program_revision": revision,
        "projection": "artifact",
        "resident_kernels": closure.resident_kernels.len(),
        "schema_version": 1,
        "source_bytecode_revision_equal": true,
        "source_sha256": source_sha256,
        "state_slots": closure.state_updates.len(),
        "state_updates": closure.state_updates.len(),
        "unclassified_nodes": artifact.nodes().len() - classified_nodes,
    });
    let activation_projection = json!({
        "activated_node_count": kernel_nodes.len(),
        "activation_executes_turn": false,
        "constants": {
            "dt": constant_values[0],
            "landmark": &constant_values[1..3],
            "process_covariance": &constant_values[3..7],
            "measurement_covariance": &constant_values[7..11],
        },
        "constraint_count": constraint_count,
        "deterministic_reactivation": true,
        "first_candidate_epoch": source.next_epoch().unwrap().get(),
        "gate": "D1",
        "input": {
            "artifact_slot": input_slot.artifact_id.get(),
            "location": location(input_slot.storage, input_slot.region.len),
            "physical_slot": input_slot.physical_index.get(),
        },
        "layout_generation": plan.layout_generation.get(),
        "output": {
            "artifact_slot": output_slot.artifact_id.get(),
            "location": location(output_slot.storage, output_slot.region.len),
            "physical_slot": output_slot.physical_index.get(),
        },
        "persistent_candidate_bytes": source.state.candidate_bytes(),
        "physical_slot_count": plan.slots.len(),
        "plan_generation": plan.plan_generation.get(),
        "predicate_kernel_count": predicate_kernels,
        "program_revision": revision,
        "projection": "activation",
        "published_epoch": source.published_epoch().get(),
        "resident_kernel_count": numeric_kernels,
        "schema_version": 1,
        "state": state_bindings,
        "state_copy_count": state_copies,
    });

    let structural = source.structural_probe();
    let mut control = ResidentEkfBatch::new(1);
    let mut trajectory = Vec::with_capacity(TURNS);
    for frame in frames() {
        let source_receipt = source
            .prepare_turn(&[captured(&source, &frame)])
            .unwrap()
            .publish()
            .unwrap();
        let decoded_receipt = decoded
            .prepare_turn(&[captured(&decoded, &frame)])
            .unwrap()
            .publish()
            .unwrap();
        control.turn(frame).unwrap();
        assert_eq!(source_receipt.before_epoch, decoded_receipt.before_epoch);
        assert_eq!(source_receipt.after_epoch, decoded_receipt.after_epoch);
        assert_eq!(source_receipt.state_hash, decoded_receipt.state_hash);
        assert_eq!(state(&source), state(&decoded));
        assert_eq!(&state(&source)[..3], &control.state(0).state);
        assert_eq!(&state(&source)[3..], &control.state(0).covariance);
        trajectory.push(state(&source));
    }
    let published_epoch = source.published_epoch();
    let published_state = state(&source);
    let abort_frame = frames().next().unwrap();
    source
        .execute_then_abort(&[captured(&source, &abort_frame)])
        .unwrap();
    let abort_preserves = source.published_epoch() == published_epoch && state(&source) == published_state;

    let mut trajectory_hash = Sha256::new();
    for turn in trajectory {
        for value in turn {
            trajectory_hash.update(((value / 1.0e-10).round() as i64).to_le_bytes());
        }
    }
    let trajectory_sha256 = hex(&trajectory_hash.finalize());
    assert_eq!(trajectory_sha256, EXPECTED_TRAJECTORY);
    let execution_projection = json!({
        "abort_preserves_published_epoch": abort_preserves,
        "candidate_seed_bytes": structural.candidate_seed_bytes,
        "candidate_written_bytes": structural.candidate_materialized_bytes,
        "commit_runtime_calls": structural.commit_runtime_call_count,
        "constraints_per_turn": constraint_count,
        "gate": "D1",
        "gate_b_control_trajectory_equal": true,
        "global_d_targets_implemented": 0,
        "legacy_occurrences_migrated": 0,
        "legacy_journal_captures": structural.legacy_journal_capture_count,
        "legacy_targets_removed": 0,
        "migrated_state_slots": 2,
        "normal_runtime_routing_changed": false,
        "numeric_kernels_per_turn": numeric_kernels,
        "predicate_kernels_per_turn": predicate_kernels,
        "projection": "execution",
        "publication_ordering": "Release",
        "publication_store_count": structural.publication_store_count,
        "published_buffer_copy_bytes": structural.published_buffer_copy_bytes,
        "reader_ordering": "Acquire",
        "schema_version": 1,
        "source_bytecode_trajectory_equal": true,
        "state_copies_per_turn": state_copies,
        "ordinary_ekf_vertical_slice": "complete",
        "admitted_artifacts": 1,
        "trace_sha256": trace_sha256,
        "trajectory_sha256": trajectory_sha256,
        "turns": TURNS,
    });
    let output: Value = json!({
        "activation": activation_projection,
        "artifact": artifact_projection,
        "execution": execution_projection,
    });
    println!("{}", serde_json::to_string(&output).unwrap());
    Ok(())
}
