//! Compare the actual owned caller/callee binding against rematerialization.
use crate::native_artifact::program::runtime::maps::closed_tests::edge_facts::*;

#[test]
fn loop_constants_metadata_binds_stack_hash_values_source_endpoint_and_actual_loads() {
    let source = format!(
        "{}\n#Count(\"abcabc\", 2u64, 6u64, empty)\n",
        crate::native_artifact::program::runtime::structural_projection_tests::COUNT
    );
    let artifact = crate::native_artifact::program::runtime::tests::artifact_source(&source);
    let catalog = mech_stdlib::source_catalog();
    let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
    let emitted = object.test_emissions().loop_constants;
    assert!(emitted > 0);
    let metadata = object.metadata();
    let callers = &metadata["compiled_loop_constants"];
    assert_eq!(callers["complete"], true);
    assert_eq!(callers["callers"].as_array().unwrap().len(), emitted);
    let caller = &callers["callers"][0];
    let callee = &metadata["compiled_outlined_loops"][0];
    let guarded = &metadata["compiled_guarded_loops"]["regions"][0];
    assert_eq!(caller["header"], callee["header"]);
    assert_eq!(caller["header"], guarded["header"]);
    assert_eq!(caller["stack_bytes"], 24);
    assert_eq!(callee["constant_packet_bytes"], 24);
    assert_eq!(
        caller["hash_values"],
        serde_json::json!([0xbf58476d1ce4e5b9u64, 0x94d049bb133111ebu64])
    );
    let endpoint = serde_json::json!([guarded["width_word"], guarded["limit_word"]]);
    assert_eq!(caller["endpoint_words"], endpoint);
    assert_eq!(callee["constant_endpoint_words"], endpoint);
    assert!(caller["template_instructions"].as_u64().unwrap() <= 64);
    assert!(callee["emitted_hash_constant_loads"].as_u64().unwrap() > 0);
    assert_eq!(callee["emitted_endpoint_loads"], 1);
    let reference = crate::native_artifact::program::compiler::emit_program_object_for_test(
        &artifact,
        &catalog,
        crate::native_artifact::codegen::options::OptimizationPass::LoopConstants,
    )
    .unwrap();
    assert_eq!(reference.test_emissions().loop_constants, 0);
    let metadata = reference.metadata();
    assert!(
        metadata["compiled_loop_constants"]["callers"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    for callee in metadata["compiled_outlined_loops"].as_array().unwrap() {
        assert_eq!(callee["constant_packet_bytes"], 0);
        assert_eq!(callee["emitted_hash_constant_loads"], 0);
        assert_eq!(callee["emitted_endpoint_loads"], 0);
    }
}

#[test]
fn loop_constants_preserve_inline_fallback_without_an_outlined_private_abi() {
    let source = format!(
        "{}\n#Count(\"aaaa\", 1u64, 4u64, empty)\n",
        crate::native_artifact::program::runtime::structural_projection_tests::COUNT
    );
    let artifact = crate::native_artifact::program::runtime::tests::artifact_source(&source);
    let object = crate::native_artifact::program::compiler::emit_program_object_for_test(
        &artifact,
        &mech_stdlib::source_catalog(),
        crate::native_artifact::codegen::options::OptimizationPass::LoopOutlining,
    )
    .unwrap();
    assert_eq!(object.test_emissions().loop_constants, 0);
    assert!(
        object.metadata()["compiled_loop_constants"]["callers"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn loop_constants_preserve_changed_inputs_full_maps_wide_keys_and_exact_work() {
    changed_input_equivalence(Pass::LoopConstants);
}

#[test]
fn loop_constants_preserve_original_overflow_and_nonempty_seed_failures() {
    arithmetic_failure_equivalence(Pass::LoopConstants);
}
