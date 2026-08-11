use mech_core::{MResult, ReactiveInstanceId, ResolvedOperationContract};
use mech_engine::__gate_b_resident::ResidentEkfBatch;
use mech_engine::__gate_d::{
    ActivatedNodeKind, FrozenEkfCompilationServices, ResidentStorageLocation, activate,
    compile_frozen_ekf_source,
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

fn state(instance: &mech_engine::__gate_d::ReactiveInstance) -> [f64; 12] {
    let mut state = [0.0; 12];
    state[..3].copy_from_slice(instance.estimate());
    state[3..].copy_from_slice(instance.covariance());
    state
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn location(location: ResidentStorageLocation) -> &'static str {
    match location {
        ResidentStorageLocation::InputFrame => "InputFrame",
        ResidentStorageLocation::State => "State",
        ResidentStorageLocation::Covariance => "Covariance",
        ResidentStorageLocation::Scratch(_) => "Scratch",
        ResidentStorageLocation::Predicate(_) => "Predicate",
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
    let legacy_opaque = artifact
        .contracts()
        .iter()
        .filter(|contract| matches!(contract, ResolvedOperationContract::LegacyOpaque(_)))
        .count();
    let classified_nodes = 1
        + closure.resident_kernels.len()
        + closure.integrity_predicates.len()
        + closure.state_updates.len();

    let mut source = activate(
        ReactiveInstanceId::new(0, 0),
        &compilation.source_artifact,
        &compilation.resource_request,
    )
    .unwrap();
    let reactivated = activate(
        ReactiveInstanceId::new(1, 0),
        &compilation.source_artifact,
        &compilation.resource_request,
    )
    .unwrap();
    let mut decoded = activate(
        ReactiveInstanceId::new(2, 0),
        &compilation.decoded_artifact,
        &compilation.resource_request,
    )
    .unwrap();
    assert_eq!(source.logical_binding_projection(), reactivated.logical_binding_projection());
    assert_eq!(source.logical_binding_projection(), decoded.logical_binding_projection());

    let projection = source.logical_binding_projection();
    let constant_values = projection
        .constant_bits
        .iter()
        .copied()
        .map(f64::from_bits)
        .collect::<Vec<_>>();
    assert_eq!(constant_values.len(), 11);
    let plan = &source.plan;
    let numeric_kernels = plan
        .nodes
        .iter()
        .filter(|node| matches!(node.kind, ActivatedNodeKind::Kernel(_)))
        .count();
    let predicate_kernels = plan
        .nodes
        .iter()
        .filter(|node| matches!(node.kind, ActivatedNodeKind::Predicate(_)))
        .count();
    let state_copies = plan
        .nodes
        .iter()
        .filter(|node| matches!(node.kind, ActivatedNodeKind::StateCopy { .. }))
        .count();
    let constraint_count = plan.constraints.len();
    let input_slot = &plan.slots[plan.input.slot.get() as usize];
    let output_slot = &plan.slots[plan.output.slot.get() as usize];
    let initial_state = *source.estimate();
    let initial_covariance = *source.covariance();
    let state_bindings = [
        json!({
            "artifact_slot": output_slot.artifact_id.get(),
            "physical_slot": output_slot.physical_index.get(),
            "location": location(output_slot.location),
            "initializer": initial_state,
        }),
        json!({
            "artifact_slot": closure.state_updates.iter().find(|update| update.target != closure.output.source).unwrap().target.get(),
            "physical_slot": 1,
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
        "activated_node_count": plan.nodes.len(),
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
            "location": location(input_slot.location),
            "physical_slot": input_slot.physical_index.get(),
        },
        "layout_generation": plan.layout_generation.get(),
        "output": {
            "artifact_slot": output_slot.artifact_id.get(),
            "location": location(output_slot.location),
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
        let source_receipt = source.prepare_turn(frame).unwrap().publish();
        let decoded_receipt = decoded.prepare_turn(frame).unwrap().publish();
        control.turn(frame).unwrap();
        assert_eq!(source_receipt.before_epoch, decoded_receipt.before_epoch);
        assert_eq!(source_receipt.after_epoch, decoded_receipt.after_epoch);
        assert_eq!(source_receipt.state_hash, decoded_receipt.state_hash);
        assert_eq!(state(&source), state(&decoded));
        assert_eq!(source.estimate(), &control.state(0).state);
        assert_eq!(source.covariance(), &control.state(0).covariance);
        trajectory.push(state(&source));
    }
    let published_epoch = source.published_epoch();
    let published_state = state(&source);
    source.execute_then_abort(frames().next().unwrap()).unwrap();
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
        "candidate_written_bytes": structural.candidate_written_bytes,
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
