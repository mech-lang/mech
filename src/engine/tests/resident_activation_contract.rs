#![cfg(feature = "compiler")]

use std::{fs, mem::size_of, path::PathBuf};

use mech_core::{CellSlotId, InstanceEpoch, LayoutGeneration, PlanGeneration, SlotIndex};
use serde_json::Value;

const TRACE_SHA256: &str = "ab901e1d115aa92166dc2a6d45a28732e6a548363b829997aa410ae4c2d77c8b";
const TRAJECTORY_SHA256: &str = "ddca8ab17cb390839d4c77e7cecc5203122f249685f5a28c36fd342cf303a758";
const EKF_SOURCE_SHA256: &str = "a64d72c34434fe240dfac2ce31763d4b1af24e8eb3abc0319c167db50468e1ec";

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
fn ekf_fixture_retains_the_frozen_trace_and_oracle() {
    let resident_ekf = json("tests/fixtures/resident-ekf/ekf-v1.json");

    assert_eq!(
        read("tests/architecture/resident-activation/ekf-source-v1.sha256").trim(),
        EKF_SOURCE_SHA256
    );

    assert_eq!(resident_ekf["episode_length"], 4096);
    assert_eq!(resident_ekf["trace"]["sha256"], TRACE_SHA256);
    assert_eq!(
        resident_ekf["reference"]["quantized_trajectory_sha256"],
        TRAJECTORY_SHA256
    );
    assert_eq!(
        resident_ekf["constants"]["initial_state"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        resident_ekf["constants"]["initial_covariance_column_major"]
            .as_array()
            .unwrap()
            .len(),
        9
    );
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
fn ekf_persistent_candidate_payload_is_exactly_96_bytes() {
    assert_eq!((3 + 9) * size_of::<f64>(), 96);
}

#[test]
fn resident_ekf_control_remains_private_and_unrouted() {
    let public_artifact = read("src/engine/src/artifact/model.rs");
    assert!(!public_artifact.contains("resident::"));
    assert!(!public_artifact.contains("ActivatedPlan"));
    assert!(!public_artifact.contains("ReactiveInstance"));

    let compiler_planning = read("src/engine/src/program/compiler_planning.rs");
    assert!(!compiler_planning.contains("ReactiveInstance::frozen_ekf_batch"));

    let resident_module = read("src/engine/src/resident/mod.rs");
    let resident_control = read("src/engine/src/resident/artifact.rs");
    assert!(resident_module.contains("mod artifact;"));
    assert!(!resident_control.contains("struct ProgramArtifact"));
    assert!(!resident_control.contains("fn frozen_ekf_batch"));
    assert!(resident_control.contains("struct ResidentEkfControlFixture"));
}
