//! A terminal error needs settlement state, not every source temporary.
use super::*;

const SOURCE: &str =
    include_str!("../../../../../tests/fixtures/native/numeric-nested-phase-growing.mec");

#[test]
fn numeric_failure_snapshot_omits_interior_scalar_temporaries() {
    for artifact in artifacts(SOURCE) {
        let object = crate::native_artifact::emit_program_object_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        let report = object.metadata();
        let rows = report["compiled_numeric_write_loops"].as_array().unwrap();
        assert!(!rows.is_empty());
        for row in rows {
            let all = row["register_words"].as_u64().unwrap();
            let failure = row["failure_spill_words"].as_u64().unwrap();
            assert!(failure > 0 && failure < all, "{row}");
        }
    }
}
