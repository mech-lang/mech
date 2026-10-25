//! Execute the source-derived progress substitution and its declined cases.
use super::*;

#[test]
fn mutation_facts_metadata_binds_actual_headers_insertions_and_nonempty_probes() {
    let source = format!(
        "{}\n#Count(\"abcabc\", 2u64, 6u64, empty)\n",
        structural_projection_tests::COUNT
    );
    let artifact = tests::artifact_source(&source);
    let catalog = mech_stdlib::source_catalog();
    take_mutation_emissions();
    let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
    let emitted = take_mutation_emissions();
    assert!(
        emitted.0 > 0 && emitted.1 > 0 && emitted.2 > 0,
        "{emitted:?}"
    );
    let metadata = object.metadata();
    let regions = metadata["compiled_mutation_loops"].as_array().unwrap();
    assert_eq!(regions.len(), 1);
    let proof = &regions[0];
    let bound = &metadata["compiled_guarded_loops"]["regions"][0];
    assert_eq!(proof["header"], bound["header"]);
    assert_eq!(proof["index_word"], bound["index_word"]);
    assert_eq!(proof["active_tag"], bound["active_tag"]);
    assert_eq!(proof["alignment_guard"], true);
    assert_eq!(
        proof["counter_headers"].as_array().unwrap().len(),
        emitted.0
    );
    assert_eq!(
        proof["counter_checks_removed"].as_array().unwrap().len(),
        emitted.1
    );
    assert_eq!(
        proof["nonempty_probes"].as_array().unwrap().len(),
        emitted.2
    );
    assert!(!proof["source_insert_nodes"].as_array().unwrap().is_empty());
    let stride = proof["version_stride"].as_u64().unwrap();
    for pair in proof["counter_checks_removed"].as_array().unwrap() {
        assert!(pair[0].as_u64().unwrap() >= stride);
        let site = pair[1].as_u64().unwrap() as usize;
        let NativeIterationAction::LibraryCall { template, .. } =
            object.descriptor().plan.blocks()[site].action()
        else {
            panic!("original insertion site");
        };
        assert_eq!(
            template.primitive(),
            mech_engine::native_artifact::NativeCollectionPrimitive::MapInsert
        );
    }
    for pair in proof["nonempty_probes"].as_array().unwrap() {
        assert_eq!(
            pair[0].as_u64().unwrap() / stride,
            3,
            "only the completed steady recurrence"
        );
    }
    let reference =
        without_mutation_facts(|| crate::native_artifact::emit_program_object(&artifact, &catalog))
            .unwrap();
    assert_eq!(take_mutation_emissions(), (0, 0, 0));
    assert!(
        reference.metadata()["compiled_mutation_loops"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn mutation_facts_preserve_changed_inputs_full_maps_growth_refills_and_exact_work() {
    changed_input_equivalence(Pass::MutationFacts);
}

#[test]
fn mutation_facts_preserve_arithmetic_failures_and_declined_recurrences() {
    arithmetic_failure_equivalence(Pass::MutationFacts);
}

#[test]
fn mutation_facts_decline_skipped_double_insertions_and_other_state_entries() {
    let body = structural_projection_tests::COUNT;
    let skipped = body.replace("  out := map/insert(h, key, previous + 1u64).",
        "  out := ((previous == 0u64)?\n    | true => map/insert(h, key, previous + 1u64)\n    | false => h\n  ).");
    let doubled = body.replace("  out := map/insert(h, key, previous + 1u64).",
        "  once := map/insert(h, key, previous + 1u64)\n  out := map/insert(once, key, previous + 2u64).");
    let entered = body.replace("  | :Next(index<u64>, counts<{string:u64}>)",
        "  | :Start(counts<{string:u64}>)\n  | :Next(index<u64>, counts<{string:u64}>)")
        .replace("#Count(s, width, n, initial) -> :Next(0u64, initial)",
            "#Count(s, width, n, initial) -> :Start(initial)\n  :Start(counts)\n    | * -> :Next(1u64, counts)");
    for (name, source, expected, updates) in [
        ("skipped", skipped, [("ab", 1), ("bc", 1), ("ca", 1)], 3u64),
        ("doubled", doubled, [("ab", 4), ("bc", 4), ("ca", 2)], 10),
        ("other-state", entered, [("ab", 1), ("bc", 2), ("ca", 1)], 4),
    ] {
        eprintln!("mutation negative fixture: {name}");
        assert_ne!(source, body);
        let artifact = tests::artifact_source(&format!(
            "{source}\n#Count(\"abcabc\", 2u64, 6u64, empty)\n"
        ));
        let catalog = mech_stdlib::source_catalog();
        let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
        assert!(
            object.metadata()["compiled_mutation_loops"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        let optimized = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let reference =
            without_mutation_facts(|| JitProgramArtifact::compile(&artifact, &catalog)).unwrap();
        let mut executions = Vec::new();
        for jit in [&optimized, &reference] {
            PUBLICATION_FRONTIERS.with(|records| records.borrow_mut().clear());
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let mut outputs = vec![None];
            let entry = jit.test_checked_entry();
            let report = execute(
                jit.descriptor(),
                |frame| {
                    // SAFETY: the checked entry and all inputs belong to this
                    // synchronous invocation; no pointer escapes it.
                    let old = unsafe { (*frame).helper };
                    unsafe {
                        (*frame).helper = observe_publication_frontier;
                    }
                    let status = unsafe { entry(frame) };
                    unsafe {
                        (*frame).helper = old;
                    }
                    status
                },
                &[],
                &mut outputs,
                tests::limits(),
                &memory,
            )
            .unwrap();
            let ValueData::Map(map) = outputs[0].as_ref().unwrap().data() else {
                panic!("map");
            };
            let pairs = map
                .entries()
                .iter()
                .map(|entry| match (entry.key().data(), entry.value()) {
                    (ValueData::String(key), ValueData::U64(value)) => (key.to_string(), *value),
                    _ => panic!("String/U64 map"),
                })
                .collect::<std::collections::BTreeMap<_, _>>();
            assert_eq!(
                pairs,
                expected
                    .map(|(key, value)| (key.to_owned(), value))
                    .into_iter()
                    .collect()
            );
            assert_eq!(
                report.packed_map_regions, 1,
                "negative fixtures must still exercise packed storage"
            );
            assert_eq!(
                report.packed_map_updates, updates,
                "actual source insertion count differs from window count"
            );
            // Publication sorts an independently hash-seeded canonical map.
            // Compare exact source work/fuel at its entrance, then retain all
            // structural report fields and every published key/count.
            let frontier =
                PUBLICATION_FRONTIERS.with(|records| core::mem::take(&mut *records.borrow_mut()));
            assert_eq!(frontier.len(), 1);
            executions.push((structural_report(&report), pairs, frontier));
            drop(outputs);
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
        }
        assert_eq!(executions[0], executions[1]);
    }
}
