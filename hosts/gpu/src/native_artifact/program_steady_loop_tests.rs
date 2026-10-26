//! Actual helper-free native functions, precise source continuations, finite
//! budgets and complete maps, compared with the original compiled source CFG.
use super::*;

#[test]
fn lookup_split_metadata_binds_owned_cache_widths_and_both_function_limits() {
    let artifact = tests::artifact_source(&format!(
        "{}\n#Count(\"abcabc\",2u64,6u64,empty)\n",
        structural_projection_tests::COUNT
    ));
    let catalog = mech_stdlib::source_catalog();
    let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
    let metadata = object.metadata();
    let ordinary = &metadata["compiled_steady_loops"][0];
    let cached = &ordinary["lookup_cache_function"];
    assert_eq!(ordinary["lookup_mode"], "hash");
    assert_eq!(cached["lookup_mode"], "direct");
    assert!(ordinary["candidate_value_loads"].as_u64().unwrap() > 0);
    assert_eq!(cached["candidate_value_loads"], 0);
    let reference = without_candidate_value_loads(|| {
        crate::native_artifact::emit_program_object(&artifact, &catalog)
    })
    .unwrap();
    assert_eq!(
        reference.metadata()["compiled_steady_loops"][0]["candidate_value_loads"],
        0
    );
    assert_eq!(
        ordinary["lookup_cache_selection_word"],
        core::mem::offset_of!(
            crate::native_artifact::program_packed_map::Frame,
            lookup_cache
        ) / 8
    );
    let domain = object
        .descriptor()
        .metadata()
        .closed_tails
        .regions
        .iter()
        .find_map(|r| r.packed.as_ref())
        .unwrap()
        .packing_domain();
    let widths = (1..=crate::native_artifact::program_packed_map::MAX_WIDTH)
        .filter(|&w| {
            crate::native_artifact::program_packed_map::lookup_cache_entries(domain, w).is_some()
        })
        .collect::<Vec<_>>();
    assert_eq!(cached["specialized_widths"], serde_json::json!(widths));
    assert_eq!(cached["native_calls"], 0);
    assert!(cached["lookup_cache_function"].is_null());
    assert!(cached["lookup_cache_selection_word"].is_null());
    for key in [
        "header",
        "active_node",
        "active_tag",
        "source_nodes",
        "entry_words",
        "cursor_range_site",
        "cursor_index_word",
        "prepaid_extent_arguments",
        "fuel_reconstructed_at_exits",
    ] {
        assert_eq!(ordinary[key], cached[key], "{key}");
    }
    for continuation in cached["continuations"].as_array().unwrap() {
        assert!(
            ordinary["continuations"]
                .as_array()
                .unwrap()
                .contains(continuation)
        );
    }
    let parent = &metadata["compiled_outlined_loops"][0];
    let both = ordinary["code_bytes"].as_u64().unwrap() + cached["code_bytes"].as_u64().unwrap();
    assert!(
        metadata["code_bytes"].as_u64().unwrap() > parent["code_bytes"].as_u64().unwrap() + both
    );
    let nested = ordinary["stack_bytes"]
        .as_u64()
        .unwrap()
        .max(cached["stack_bytes"].as_u64().unwrap());
    assert!(
        metadata["stack_bytes"].as_u64().unwrap()
            >= parent["stack_bytes"].as_u64().unwrap() + nested
    );
    let mut limits = crate::native_artifact::NativeCompileLimits::default();
    limits.max_stack_bytes = metadata["stack_bytes"].as_u64().unwrap();
    limits.max_code_bytes = metadata["code_bytes"].as_u64().unwrap();
    assert!(
        crate::native_artifact::emit_program_object_with_limits(&artifact, &catalog, limits)
            .is_ok()
    );
    limits.max_code_bytes -= 1;
    assert!(
        crate::native_artifact::emit_program_object_with_limits(&artifact, &catalog, limits)
            .is_err()
    );
    let dynamic = without_lookup_loop_split(|| {
        crate::native_artifact::emit_program_object(&artifact, &catalog)
    })
    .unwrap();
    let dynamic = &dynamic.metadata()["compiled_steady_loops"][0];
    assert_eq!(dynamic["lookup_mode"], "dynamic");
    assert!(dynamic["lookup_cache_function"].is_null());
}

#[test]
fn lookup_split_preserves_changed_inputs_complete_maps_and_exact_work() {
    changed_input_equivalence(Pass::LookupSplit);
}

#[test]
fn lookup_split_preserves_overflows_nonempty_seeds_and_original_failure_sites() {
    arithmetic_failure_equivalence(Pass::LookupSplit);
}

#[test]
fn candidate_values_preserve_full_maps_growth_refills_and_exact_source_work() {
    changed_input_equivalence(Pass::CandidateValues);
}

#[test]
fn candidate_values_preserve_overflow_nonempty_seed_and_failure_origins() {
    arithmetic_failure_equivalence(Pass::CandidateValues);
}

#[test]
fn paired_candidate_values_preserve_compact_producer_maps_refills_and_work() {
    changed_input_equivalence(Pass::PairedCandidates);
}

#[test]
fn steady_metadata_binds_original_nodes_live_continuations_and_native_limits() {
    let source = format!(
        "{}\n#Count(\"abcabc\",2u64,6u64,empty)\n",
        structural_projection_tests::COUNT
    );
    let artifact = tests::artifact_source(&source);
    let catalog = mech_stdlib::source_catalog();
    take_steady_emissions();
    let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
    assert_eq!(take_steady_emissions(), 1);
    let metadata = object.metadata();
    let loops = metadata["compiled_steady_loops"].as_array().unwrap();
    assert_eq!(loops.len(), 1);
    let pure = &loops[0];
    let parent = &metadata["compiled_outlined_loops"][0];
    let bound = &metadata["compiled_guarded_loops"]["regions"][0];
    assert_eq!(pure["header"], parent["header"]);
    assert_eq!(pure["header"], bound["header"]);
    assert_eq!(pure["native_calls"], 0);
    assert_eq!(pure["cleanup"]["pre_codegen_data_passes"], 1);
    assert!(
        pure["cleanup"]["constant_branches_removed"]
            .as_u64()
            .unwrap()
            > 0,
        "the data/control phase must remove actual scalar-constant branches"
    );
    assert!(pure["caller_instructions"].as_u64().unwrap() > 0);
    assert!(pure["code_bytes"].as_u64().unwrap() > 0);
    assert!(pure["symbol"].as_str().unwrap().ends_with("_funded"));
    assert_eq!(pure["active_tag"], bound["active_tag"]);
    let peeled = pure["peeled_probe_nodes"].as_array().unwrap();
    assert!(
        !peeled.is_empty(),
        "fresh source memberships must emit the direct entry path"
    );
    assert_eq!(
        peeled.len(),
        peeled
            .iter()
            .map(|n| n.as_u64().unwrap())
            .collect::<std::collections::BTreeSet<_>>()
            .len()
    );
    assert_eq!(
        pure["specialized_widths"],
        serde_json::json!(
            (1..=crate::native_artifact::program_packed_map::MAX_WIDTH).collect::<Vec<_>>()
        )
    );
    let checks = pure["fuel_check_edges"].as_array().unwrap();
    assert_eq!(
        pure["header_snapshot_edges"], pure["fuel_check_edges"],
        "every emitted incoming header edge needs its own exact cold snapshot"
    );
    assert!(
        checks.iter().any(serde_json::Value::is_null),
        "initial entry retains the fuel check"
    );
    assert!(
        checks.iter().any(serde_json::Value::is_u64),
        "source backedges retain the fuel check"
    );
    assert_eq!(
        pure["prepaid_stop_edges"],
        serde_json::json!(
            checks
                .iter()
                .filter_map(serde_json::Value::as_u64)
                .collect::<Vec<_>>()
        )
    );
    assert_eq!(pure["fuel_reconstructed_at_exits"], true);
    assert_eq!(pure["prepaid_extent_arguments"], 2);
    assert_eq!(pure["cursor_range_site"], bound["range_site"]);
    assert_eq!(pure["cursor_index_word"], bound["index_word"]);
    assert_eq!(pure["cursor_recurrence_edges"], pure["prepaid_stop_edges"]);
    assert!(pure["maximum_fuel_check_instructions"].as_u64().unwrap() > 0);
    assert_eq!(pure["entry_stop_index_word"], bound["index_word"]);
    assert_eq!(
        pure["entry_stop_endpoint_words"],
        serde_json::json!([
            bound["width_word"].as_u64().unwrap(),
            bound["limit_word"].as_u64().unwrap()
        ])
    );
    let mut node_sites = std::collections::BTreeMap::new();
    let closed = &object.descriptor().metadata().closed_tails;
    let region = closed
        .regions
        .iter()
        .position(|r| r.tail.header() as u64 == pure["header"].as_u64().unwrap())
        .unwrap();
    for row in pure["source_nodes"].as_array().unwrap() {
        let node = row[0].as_u64().unwrap();
        let site = row[1].as_u64().unwrap() as usize;
        assert!(node_sites.insert(node, site).is_none());
        assert_eq!(closed.sites[site], Some(region));
    }
    assert!(!pure["continuations"].as_array().unwrap().is_empty());
    for row in pure["continuations"].as_array().unwrap() {
        let node = row[0].as_u64().unwrap() - 1;
        if let Some(site) = row[1].as_u64() {
            assert_eq!(node_sites[&node], site as usize);
        }
        let live = row[2].as_array().unwrap();
        assert_eq!(
            live.len(),
            live.iter()
                .map(|word| word.as_u64().unwrap())
                .collect::<std::collections::BTreeSet<_>>()
                .len()
        );
    }
    assert!(bound["fixed_template_instructions"].as_u64().unwrap() <= 1024);
    assert!(pure["maximum_source_instructions"].as_u64().unwrap() <= 512);
    let baseline =
        without_steady_loops(|| crate::native_artifact::emit_program_object(&artifact, &catalog))
            .unwrap();
    assert!(
        baseline.metadata()["compiled_steady_loops"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(take_steady_emissions(), 0);
    let mut limits = crate::native_artifact::NativeCompileLimits::default();
    limits.max_stack_bytes = metadata["stack_bytes"].as_u64().unwrap() - 1;
    assert!(
        crate::native_artifact::emit_program_object_with_limits(&artifact, &catalog, limits)
            .is_err()
    );
    limits = Default::default();
    limits.max_code_bytes = metadata["code_bytes"].as_u64().unwrap() - 1;
    assert!(
        crate::native_artifact::emit_program_object_with_limits(&artifact, &catalog, limits)
            .is_err()
    );
}

#[test]
fn steady_changed_inputs_complete_maps_growth_refills_and_source_bytecode_agree() {
    changed_input_equivalence(Pass::SteadyLoops);
}

#[test]
fn steady_retains_source_overflows_and_nonempty_seed_errors() {
    arithmetic_failure_equivalence(Pass::SteadyLoops);
}

#[test]
fn steady_declines_changed_recurrences_unbounded_counts_and_missing_outlining() {
    let body = structural_projection_tests::COUNT;
    let catalog = mech_stdlib::source_catalog();
    for changed in [
        body.replace("previous + 1u64", "previous + 2u64"),
        body.replace(
            ":Next(index + 1u64, increment",
            ":Next(index + 2u64, increment",
        ),
    ] {
        assert_ne!(changed, body);
        let artifact =
            tests::artifact_source(&format!("{changed}\n#Count(\"aaaa\",1u64,4u64,empty)\n"));
        let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
        assert!(
            object.metadata()["compiled_steady_loops"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
    let artifact = tests::artifact_source(&format!("{body}\n#Count(\"aaaa\",1u64,4u64,empty)\n"));
    let inline =
        without_loop_outlining(|| crate::native_artifact::emit_program_object(&artifact, &catalog))
            .unwrap();
    assert!(
        inline.metadata()["compiled_steady_loops"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}
