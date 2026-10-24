//! Execute the same original/decoded source with intersecting and threaded
//! key facts. The disabled pass keeps every other compiler optimization.
use super::*;

#[test]
fn lookup_threading_metadata_binds_present_and_absent_versions_to_actual_source_calls() {
    let source = format!("{}\n#Count(\"abcabc\", 2u64, 6u64, empty)\n", structural_projection_tests::COUNT);
    let artifact = tests::artifact_source(&source);
    let catalog = mech_stdlib::source_catalog();
    take_lookup_thread_emissions();
    let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
    let actual = take_lookup_thread_emissions();
    assert_eq!(actual.0, 1);
    let metadata = object.metadata();
    let region = &metadata["compiled_lookup_facts"]["regions"][0];
    let proof = &region["state_threading"];
    let states = proof["states"].as_array().unwrap();
    assert_eq!(states.len(), actual.1);
    assert!(states.len() <= 256);
    assert_eq!(region["source_nodes"].as_u64().unwrap() as usize, states.len());
    let old_nodes = proof["original_tag_nodes"].as_u64().unwrap();
    let mut insert_states = std::collections::BTreeMap::<usize, std::collections::BTreeSet<String>>::new();
    for (node, state) in states.iter().enumerate() {
        assert_eq!(state[0], node);
        assert!(state[1].as_u64().unwrap() < old_nodes);
        let site = state[2].as_u64().unwrap() as usize;
        if let NativeIterationAction::LibraryCall { template, .. } = object.descriptor().plan.blocks()[site].action()
            && template.primitive() == mech_engine::native_artifact::NativeCollectionPrimitive::MapInsert
        { insert_states.entry(site).or_default().insert(state[3].as_str().unwrap().to_owned()); }
    }
    assert!(insert_states.values().any(|facts| facts.contains("Present") && facts.contains("Absent")));
    let reference = without_lookup_threading(|| crate::native_artifact::emit_program_object(&artifact, &catalog)).unwrap();
    assert_eq!(take_lookup_thread_emissions(), (0, 0));
    let metadata = reference.metadata();
    let reference_region = &metadata["compiled_lookup_facts"]["regions"][0];
    assert!(reference_region["state_threading"].is_null());
    assert!(reference_region["substitutions"].as_array().unwrap().iter().any(|s| s["operation"] == 5 && s["fact"] == "Cached"));
}

#[test]
fn lookup_threading_preserves_changed_inputs_complete_maps_growth_and_refills() {
    changed_input_equivalence(Pass::LookupThreading);
}

#[test]
fn lookup_threading_preserves_original_overflows_nonempty_seeds_and_error_sites() {
    arithmetic_failure_equivalence(Pass::LookupThreading);
}
