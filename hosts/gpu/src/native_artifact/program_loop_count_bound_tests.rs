//! Execute the optional stored-value induction with its real guarded owner.
use super::*;

#[test]
fn count_bounds_metadata_binds_actual_reads_additions_and_guarded_emissions() {
    let source = format!("{}\n#Count(\"abcabc\", 2u64, 6u64, empty)\n", structural_projection_tests::COUNT);
    let artifact = tests::artifact_source(&source);
    let catalog = mech_stdlib::source_catalog();
    take_count_bound_emissions();
    let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
    let emitted = take_count_bound_emissions();
    assert!(emitted > 0, "the fixture must change actual source additions");
    let metadata = object.metadata();
    let values = metadata["compiled_count_value_loops"].as_array().unwrap();
    assert_eq!(values.len(), 1);
    let value = &values[0];
    let mutation = &metadata["compiled_mutation_loops"][0];
    assert_eq!(value["header"], mutation["header"]);
    assert_eq!(value["index_word"], mutation["index_word"]);
    assert_eq!(value["active_tag"], mutation["active_tag"]);
    assert_eq!(value["empty_seed_checked"], true);
    let stride = value["version_stride"].as_u64().unwrap();
    assert_eq!(value["emitted_nodes"], 4 * stride);
    let additions = value["source_add_nodes"].as_array().unwrap();
    assert!(!additions.is_empty());
    for row in additions {
        assert!(row[0].as_u64().unwrap() < stride);
        let site = row[1].as_u64().unwrap() as usize;
        assert!(matches!(object.descriptor().plan.blocks()[site].action(),
            NativeIterationAction::Scalar(mech_engine::native_artifact::NativeMatchAction::Primitive {
                op: mech_engine::native_artifact::NativeScalarOp::U64Add, .. })));
    }
    let reads = value["source_read_nodes"].as_array().unwrap();
    assert!(!reads.is_empty());
    for row in reads {
        let site = row[1].as_u64().unwrap() as usize;
        let NativeIterationAction::LibraryCall { template, .. } = object.descriptor().plan.blocks()[site].action()
            else { panic!("original map read"); };
        assert_eq!(template.primitive(), mech_engine::native_artifact::NativeCollectionPrimitive::MapKeyAccess);
    }
    let removed = value["overflow_checks_removed"].as_array().unwrap();
    assert_eq!(removed.len(), emitted);
    assert_eq!(removed.len(), 3 * additions.len());
    for row in removed {
        let node = row[0].as_u64().unwrap();
        assert!((stride..4 * stride).contains(&node));
        assert!(additions.iter().any(|original| original[0] == node % stride && original[1] == row[1]));
    }
    let reference = without_count_bounds(|| crate::native_artifact::emit_program_object(&artifact, &catalog)).unwrap();
    assert_eq!(take_count_bound_emissions(), 0);
    assert!(reference.metadata()["compiled_count_value_loops"].as_array().unwrap().is_empty());
    assert!(!reference.metadata()["compiled_mutation_loops"].as_array().unwrap().is_empty());
}

#[test]
fn count_bounds_preserve_changed_inputs_full_maps_wide_keys_refills_and_exact_work() {
    changed_input_equivalence(Pass::CountBounds);
}

#[test]
fn count_bounds_preserve_real_source_overflow_and_declined_recurrences() {
    arithmetic_failure_equivalence(Pass::CountBounds);
}

#[test]
fn count_bounds_decline_unbounded_replacements_without_disabling_progress_proofs() {
    let body = structural_projection_tests::COUNT;
    for changed in [body.replace("previous + 1u64", "previous + 2u64"),
        body.replace("previous + 1u64", "18446744073709551615u64")]
    {
        assert_ne!(changed, body);
        let artifact = tests::artifact_source(&format!("{changed}\n#Count(\"aaaa\", 1u64, 4u64, empty)\n"));
        let object = crate::native_artifact::emit_program_object(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert!(object.metadata()["compiled_count_value_loops"].as_array().unwrap().is_empty());
        assert!(!object.metadata()["compiled_mutation_loops"].as_array().unwrap().is_empty(),
            "one insertion per recurrence is insufficient to bound arbitrary values");
    }
}
