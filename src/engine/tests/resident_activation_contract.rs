#![cfg(feature = "compiler")]

use std::{fs, mem::size_of, path::PathBuf};

use mech_core::{CellSlotId, InstanceEpoch, LayoutGeneration, PlanGeneration, SlotIndex};
use serde_json::{Value, json};

const TRACE_SHA256: &str = "ab901e1d115aa92166dc2a6d45a28732e6a548363b829997aa410ae4c2d77c8b";
const TRAJECTORY_SHA256: &str = "ddca8ab17cb390839d4c77e7cecc5203122f249685f5a28c36fd342cf303a758";
const D0_EKF_SOURCE_SHA256: &str =
    "a64d72c34434fe240dfac2ce31763d4b1af24e8eb3abc0319c167db50468e1ec";

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(relative: &str) -> String {
    fs::read_to_string(repository_root().join(relative)).unwrap()
}

fn json(relative: &str) -> Value {
    serde_json::from_str(&read(relative)).unwrap()
}

#[test]
fn ordinary_ekf_resident_source_parses_without_grammar_changes() {
    let source = read("tests/architecture/resident-activation/ekf-source-v1.mec");
    mech_syntax::parser::parse(&source).expect("the frozen ordinary source must parse completely");
    assert!(source.contains(
        "finite-candidate! := ekf/candidate-finite(corrected-state,\n  symmetrized-covariance)"
    ));
    assert!(!source.contains("ekf/state-finite"));
}

#[test]
fn d0_workload_retains_the_frozen_gate_b_trace_and_oracle() {
    let workload = json("tests/architecture/resident-activation/ekf-workload-v1.json");
    let gate_b = json("benchmarks/runtime/gate-b/ekf-v1.json");

    assert_eq!(workload["source"]["sha256"], D0_EKF_SOURCE_SHA256);
    assert_eq!(
        read("tests/architecture/resident-activation/ekf-source-v1.sha256").trim(),
        D0_EKF_SOURCE_SHA256
    );

    assert_eq!(workload["gate_b"]["episode_length"], 4096);
    assert_eq!(workload["gate_b"]["trace_sha256"], TRACE_SHA256);
    assert_eq!(workload["gate_b"]["trajectory_sha256"], TRAJECTORY_SHA256);
    assert_eq!(
        gate_b["episode_length"],
        workload["gate_b"]["episode_length"]
    );
    assert_eq!(
        gate_b["trace"]["sha256"],
        workload["gate_b"]["trace_sha256"]
    );
    assert_eq!(
        gate_b["reference"]["quantized_trajectory_sha256"],
        workload["gate_b"]["trajectory_sha256"]
    );

    assert_eq!(
        workload["state"][0]["initial_payload"],
        gate_b["constants"]["initial_state"]
    );
    assert_eq!(
        workload["state"][1]["initial_payload_column_major"],
        gate_b["constants"]["initial_covariance_column_major"]
    );
    assert_eq!(
        workload["constants"][0]["payload"],
        gate_b["constants"]["dt"]
    );
    assert_eq!(
        workload["constants"][1]["payload"],
        gate_b["constants"]["landmark"]
    );
    assert_eq!(
        workload["constants"][2]["payload"],
        gate_b["constants"]["process_covariance_column_major"]
    );
    assert_eq!(
        workload["constants"][3]["payload"],
        gate_b["constants"]["measurement_covariance_column_major"]
    );

    assert_eq!(workload["operations"][15]["role"], "integrity-predicate");
    assert_eq!(
        workload["operations"][15]["operation"],
        "ekf/candidate-finite"
    );
    assert_eq!(
        workload["operations"][15]["input_schemas"],
        json!(["3x1", "3x3"])
    );
    assert_eq!(workload["operations"][15]["output_schema"], "bool");
    assert_eq!(
        workload["operations"][15]["change_detection"],
        "ExactScalar"
    );
    assert_eq!(
        workload["integrity_constraints"][0]["operation"],
        "integrity/assert"
    );
    assert_eq!(
        workload["integrity_constraints"][0]["predicate_operation_ordinal"],
        15
    );
    assert_eq!(workload["integrity_constraints"][0]["input_count"], 1);
    assert_eq!(workload["integrity_constraints"][0]["output_count"], 0);
}

#[test]
fn resident_activation_identity_domains_remain_distinct() {
    assert_eq!(size_of::<CellSlotId>(), size_of::<u32>());
    assert_eq!(size_of::<SlotIndex>(), size_of::<u32>());
    assert_eq!(size_of::<InstanceEpoch>(), size_of::<u64>());

    assert_eq!(PlanGeneration::ZERO.get(), 0);
    assert_eq!(LayoutGeneration::ZERO.get(), 0);
    assert_eq!(InstanceEpoch::ZERO.get(), 0);

    assert!(InstanceEpoch::new(u64::MAX).checked_next().is_err());
    assert!(PlanGeneration::new(u64::MAX).checked_next().is_err());
    assert!(LayoutGeneration::new(u64::MAX).checked_next().is_err());
}

#[test]
fn d1_ekf_persistent_candidate_storage_is_exactly_96_bytes() {
    let workload = json("tests/architecture/resident-activation/ekf-workload-v1.json");
    let state_elements = workload["state"][0]["initial_payload"]
        .as_array()
        .unwrap()
        .len();
    let covariance_elements = workload["state"][1]["initial_payload_column_major"]
        .as_array()
        .unwrap()
        .len();
    let candidate_bytes = (state_elements + covariance_elements) * size_of::<f64>();

    assert_eq!(state_elements, 3);
    assert_eq!(covariance_elements, 9);
    assert_eq!(candidate_bytes, 96);
    assert_eq!(
        workload["d1_acceptance_targets"]["persistent_candidate_bytes_per_instance"],
        candidate_bytes
    );
}

#[test]
fn gate_b_resident_control_remains_private_and_unrouted() {
    let public_artifact = read("src/engine/src/artifact/model.rs");
    assert!(!public_artifact.contains("resident::"));
    assert!(!public_artifact.contains("ActivatedPlan"));
    assert!(!public_artifact.contains("ReactiveInstance"));

    let normal_program = read("src/engine/src/program/instance.rs");
    assert!(!normal_program.contains("ReactiveInstance::frozen_ekf_batch"));

    let resident_module = read("src/engine/src/resident/mod.rs");
    let resident_artifact = read("src/engine/src/resident/artifact.rs");
    assert!(resident_module.contains("mod artifact;"));
    assert!(resident_artifact.contains("pub(crate) struct ProgramArtifact"));
    assert!(resident_artifact.contains("fn frozen_ekf_batch"));
}
