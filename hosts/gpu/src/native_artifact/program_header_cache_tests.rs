//! Execute cache invalidation at every proved header, including terminal ones.
use super::*;

#[test]
fn header_cache_metadata_binds_actual_headers_words_and_emissions() {
    let source = format!("{}\n#Count(\"abcabc\", 2u64, 6u64, empty)\n", structural_projection_tests::COUNT);
    let artifact = tests::artifact_source(&source);
    let catalog = mech_stdlib::source_catalog();
    take_header_cache_emissions();
    let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
    let emitted = take_header_cache_emissions();
    assert!(emitted > 0);
    let metadata = object.metadata();
    let regions = metadata["compiled_header_cache_loops"].as_array().unwrap();
    assert_eq!(regions.len(), 1);
    let region = &regions[0];
    assert_eq!(region["cleared_words"], serde_json::json!([7, 8]));
    let header = region["header"].as_u64().unwrap() as usize;
    assert!(matches!(object.descriptor().plan.blocks()[header].terminator(),
        mech_engine::native_artifact::NativeIterationTerminator::TailHeader { .. }));
    let stride = region["version_stride"].as_u64().unwrap();
    let nodes = region["emitted_nodes"].as_u64().unwrap();
    let resets = region["resets"].as_array().unwrap();
    assert_eq!(resets.len(), emitted);
    let tags = &metadata["compiled_tag_threading"]["regions"][0];
    assert_eq!(tags["header"], region["header"]);
    assert_eq!(tags["header_versions"].as_u64().unwrap(), emitted as u64);
    assert_eq!(nodes, 4 * stride);
    for reset in resets {
        assert!(reset[0].as_u64().unwrap() < nodes);
        assert_eq!(reset[1].as_u64().unwrap(), header as u64);
    }
    let reference = without_header_cache_resets(|| crate::native_artifact::emit_program_object(&artifact, &catalog)).unwrap();
    assert_eq!(take_header_cache_emissions(), 0);
    assert!(reference.metadata()["compiled_header_cache_loops"].as_array().unwrap().is_empty());
    assert!(!reference.metadata()["compiled_lookup_facts"]["regions"].as_array().unwrap().is_empty());
}

#[test]
fn header_cache_resets_require_the_lookup_header_invalidation_proof() {
    let source = format!("{}\n#Count(\"aaaa\", 1u64, 4u64, empty)\n", structural_projection_tests::COUNT);
    let artifact = tests::artifact_source(&source);
    take_header_cache_emissions();
    let object = without_lookup_facts(|| crate::native_artifact::emit_program_object(&artifact,
        &mech_stdlib::source_catalog())).unwrap();
    assert_eq!(take_header_cache_emissions(), 0);
    assert!(object.metadata()["compiled_header_cache_loops"].as_array().unwrap().is_empty());
}

#[test]
fn header_cache_resets_preserve_changed_inputs_full_maps_wide_keys_and_work() {
    changed_input_equivalence(Pass::HeaderCache);
}

#[test]
fn header_cache_resets_preserve_original_overflow_and_nonempty_seed_failures() {
    arithmetic_failure_equivalence(Pass::HeaderCache);
}
