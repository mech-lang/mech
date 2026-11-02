//! Canonical output snapshots shared by native runtime contract tests.
use mech_core::Value;
use mech_engine::ProgramArtifact;

pub(super) fn snapshot_outputs(
    artifact: &ProgramArtifact,
    outputs: &[Option<Value>],
) -> Vec<Box<[u8]>> {
    outputs
        .iter()
        .map(|value| {
            value
                .as_ref()
                .unwrap()
                .canonical_snapshot_bytes(artifact.schemas())
                .unwrap()
        })
        .collect()
}
