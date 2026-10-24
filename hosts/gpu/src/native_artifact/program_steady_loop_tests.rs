//! Actual helper-free native functions, precise source continuations, finite
//! budgets and complete maps, compared with the original compiled source CFG.
use super::*;

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
    assert_eq!(pure["specialized_widths"], serde_json::json!(
        (1..=crate::native_artifact::program_packed_map::MAX_WIDTH).collect::<Vec<_>>()
    ));
    let mut node_sites = std::collections::BTreeMap::new();
    let closed = &object.descriptor().closed_tails;
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
