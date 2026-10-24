//! Execute the same source with and without guarded loop versions. Publication
//! is independently hash-seeded, so exact work is compared before that boundary.
use super::*;
use crate::native_artifact::program_compiler::packed_loop_codegen::{
    take_loop_bounds_emissions, without_loop_bounds,
    take_lookup_fact_emissions, without_lookup_facts,
};

unsafe extern "C" fn observed(raw: *mut c_void, site: u64, op: u64,
    first: u64, second: u64, third: u64, result: *mut u64) -> u64
{
    if op == CLOSED_TAIL_END {
        // SAFETY: same live invocation and same frontier as the existing observer.
        let ctx = unsafe { &*raw.cast::<Context<'_, '_>>() };
        PUBLICATION_FRONTIERS.with(|r| r.borrow_mut().push((site, first, ctx.control.work.used(), ctx.library.used())));
    }
    // SAFETY: the maintained observer delegates all original operations and
    // confines the requested finite-cap test override to its original entry.
    unsafe { observe(raw, site, op, first, second, third, result) }
}

fn parameterized() -> String {
    let body = structural_projection_tests::COUNT
        .replace("initial<{string:u64}>)", "initial<{string:u64}>, start<u64>)")
        .replace("#Count(s, width, n, initial) -> :Next(0u64, initial)",
            "#Count(s, width, n, initial, start) -> :Next(start, initial)");
    assert_ne!(body, structural_projection_tests::COUNT);
    format!("{body}\n#Count(text<string>, width<u64>, n<u64>, empty, start<u64>)\n")
}

#[test]
fn guarded_loop_versions_preserve_actual_bounds_arbitrary_seeds_headers_and_work() {
    parameterized_equivalence(false, false, false, false, false, false, false, false, false);
}

#[test]
fn lookup_facts_preserve_limits_actual_bounds_seeds_refill_headers_and_work() {
    parameterized_equivalence(true, false, false, false, false, false, false, false, false);
}

#[test]
fn cfg_cleanup_preserves_limits_actual_bounds_seeds_refill_headers_and_work() {
    parameterized_equivalence(false, true, false, false, false, false, false, false, false);
}

#[test]
fn loop_outlining_preserves_limits_actual_bounds_seeds_refill_headers_and_work() {
    parameterized_equivalence(false, false, true, false, false, false, false, false, false);
}

#[test]
fn loop_invariants_preserve_first_selection_steady_ranges_cap_transitions_and_refill_frontiers() {
    parameterized_equivalence(false, false, false, true, false, false, false, false, false);
}

#[test]
fn preserving_calls_keep_bounds_arbitrary_seeds_cap_transitions_and_refill_frontiers() {
    parameterized_equivalence(false, false, false, false, true, false, false, false, false);
}

#[test]
fn mutation_facts_keep_unaligned_initial_states_caps_seeds_and_refill_frontiers() {
    parameterized_equivalence(false, false, false, false, false, true, false, false, false);
}

#[test]
fn count_bounds_keep_unaligned_initial_states_caps_seeds_and_refill_frontiers() {
    parameterized_equivalence(false, false, false, false, false, false, true, false, false);
}

#[test]
fn loop_constants_keep_seeds_caps_arbitrary_headers_and_refill_frontiers() {
    parameterized_equivalence(false, false, false, false, false, false, false, true, false);
}

#[test]
fn steady_keeps_finite_caps_arbitrary_headers_seeds_and_refill_frontiers() {
    parameterized_equivalence(false, false, false, false, false, false, false, false, true);
}

fn parameterized_equivalence(compare_lookup_facts: bool, compare_cfg_cleanup: bool, compare_outlining: bool, compare_invariants: bool, compare_preserving: bool, compare_mutation: bool, compare_counts: bool, compare_constants: bool, compare_steady: bool) {
    let original = tests::artifact_source(&parameterized());
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap()).unwrap();
    for artifact in [original, decoded] {
        let catalog = mech_stdlib::source_catalog();
        take_loop_bounds_emissions();
        take_lookup_fact_emissions();
        take_outline_emissions();
        take_mutation_emissions();
        take_count_bound_emissions();
        take_constant_emissions();
        if compare_preserving { crate::native_artifact::program_bound_shim::take_emissions(); }
        take_steady_emissions();
        let optimized = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        if compare_steady { assert!(take_steady_emissions()>0); }
        let mutation_optimized = take_mutation_emissions().0 > 0;
        let constant_emitted = take_constant_emissions();
        if compare_constants { assert!(constant_emitted > 0); }
        let count_emitted = take_count_bound_emissions();
        if compare_counts { assert!(count_emitted > 0, "fixture must actually remove source count checks"); }
        if compare_mutation { assert!(mutation_optimized, "fixture must select the actual mutation proof"); }
        if compare_preserving && cfg!(target_arch = "aarch64") {
            assert!(crate::native_artifact::program_bound_shim::take_emissions() > 0);
        }
        assert_eq!(take_outline_emissions(), 1);
        let emitted = take_loop_bounds_emissions();
        assert_eq!(emitted.0, 1, "must emit the guarded graph, not only its fallback");
        assert!(emitted.1 > 0);
        let lookup_emissions = take_lookup_fact_emissions();
        assert!(lookup_emissions.0 > 0 && lookup_emissions.1 >= 4);
        take_outline_emissions();
        let baseline = if compare_steady {
            without_steady_loops(|| JitProgramArtifact::compile(&artifact, &catalog))
        } else if compare_constants {
            without_loop_constants(|| JitProgramArtifact::compile(&artifact, &catalog))
        } else if compare_counts {
            without_count_bounds(|| JitProgramArtifact::compile(&artifact, &catalog))
        } else if compare_mutation {
            without_mutation_facts(|| JitProgramArtifact::compile(&artifact, &catalog))
        } else if compare_preserving {
            crate::native_artifact::program_bound_shim::without_preserving_calls(|| JitProgramArtifact::compile(&artifact, &catalog))
        } else if compare_invariants {
            without_loop_invariants(|| JitProgramArtifact::compile(&artifact, &catalog))
        } else if compare_outlining {
            without_loop_outlining(|| JitProgramArtifact::compile(&artifact, &catalog))
        } else if compare_cfg_cleanup {
            without_cfg_cleanup(|| JitProgramArtifact::compile(&artifact, &catalog))
        } else if compare_lookup_facts {
            without_lookup_facts(|| JitProgramArtifact::compile(&artifact, &catalog))
        } else { without_loop_bounds(|| JitProgramArtifact::compile(&artifact, &catalog)) }.unwrap();
        let mutation_baseline = take_mutation_emissions().0 > 0;
        if compare_steady {
            assert_eq!(take_steady_emissions(),0);
            assert_eq!(take_loop_bounds_emissions(),emitted);
        } else if compare_constants {
            assert_eq!(take_constant_emissions(), 0);
            assert_eq!(take_loop_bounds_emissions(), emitted);
        } else if compare_counts {
            assert_eq!(take_count_bound_emissions(), 0);
            assert_eq!(take_loop_bounds_emissions(), emitted);
        } else if compare_mutation {
            assert!(!mutation_baseline);
            assert_eq!(take_loop_bounds_emissions(), emitted);
        } else if compare_preserving {
            assert_eq!(take_loop_bounds_emissions(), emitted);
        } else if compare_invariants {
            // The candidate emits three guarded copies (checked calls,
            // allowed calls, steady rolling); the reference emits only one.
            assert_eq!(emitted.1 % 3, 0);
            assert_eq!(take_loop_bounds_emissions(), (emitted.0, emitted.1 / 3));
        } else if compare_outlining {
            assert_eq!(take_outline_emissions(), 0);
            assert_eq!(take_loop_bounds_emissions(), emitted);
        } else if compare_lookup_facts {
            assert_eq!(take_lookup_fact_emissions(), (0, 0));
            assert_eq!(take_loop_bounds_emissions(), emitted);
        } else if compare_cfg_cleanup {
            assert_eq!(take_loop_bounds_emissions(), emitted);
        } else { assert_eq!(take_loop_bounds_emissions(), (0, 0)); }
        let long = "a".repeat(1024);
        let cases = [
            ("abcabc", 1u64, 6u64, 0u64, Mode::Native),
            ("abcabc", 2, 5, 0, Mode::Native), // n may be smaller than actual length.
            ("abcabc", 2, 6, 2, Mode::Native),
            ("abcabc", 2, 6, 4, Mode::Native), // Last productive index.
            ("abcabc", 2, 6, 5, Mode::Native), // Exactly trip_end: stop then Done.
            ("abcabc", 2, 6, u64::MAX, Mode::Native),
            ("abcdefghijklmnopqr", 18, 18, 0, Mode::Native),
            ("abc", 0, 3, 0, Mode::Native),
            ("abc", 0, u64::MAX, u64::MAX, Mode::TightCall), // No future call may fail early.
            ("abc", 4, 3, 0, Mode::Native),
            ("abc", 2, 0, 0, Mode::Native),
            ("", 1, 0, 0, Mode::Native),
            ("abc", 2, 4, 0, Mode::Native), // Decline; original selection failure.
            ("abc", 2, 4, 4, Mode::Native), // Decline but original stop succeeds.
            ("aa", 1, 2, 0, Mode::TightCall),
            ("aa", 1, 2, 0, Mode::TightWrite),
            ("aaaaaaaaaaaaaaaaaaa", 18, 19, 0, Mode::WriteTransition),
            ("abcdefghijklmnopabcdefghijklmnop", 1, 32, 0, Mode::LimitedInsert),
            ("aa", 1, 2, 0, Mode::OneShort),
        ];
        let refill_cases = [
            (long.as_str(), 1, 1022, 0, Mode::Native),
            (long.as_str(), 1, 1023, 0, Mode::Native), // With full loans, refill on final Done header.
            (long.as_str(), 1, 1024, 0, Mode::Native),
        ];
        let all_width_text = "abcdefghijklmnopqr".repeat(4);
        let all_width_cases = (1..=crate::native_artifact::program_packed_map::MAX_WIDTH)
            .filter(|_| compare_steady)
            .map(|width| (all_width_text.as_str(), width, all_width_text.len() as u64, 0, Mode::Native));
        for ((text, width, n, start, mode), full_refill_budget, expected_default_denial) in cases.into_iter()
            .map(|case| (case, false, false))
            .chain(refill_cases.iter().copied().map(|case| (case, false, true)))
            .chain(refill_cases.iter().copied().map(|case| (case, true, false)))
            .chain(all_width_cases.map(|case| (case, true, false)))
        {
            let mut executions = Vec::new();
            for (version, jit) in [&optimized, &baseline].into_iter().enumerate() {
                MODE.with(|m| m.set(mode)); OBS.with(|r| *r.borrow_mut() = Observation::default());
                PUBLICATION_FRONTIERS.with(|r| r.borrow_mut().clear());
                let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
                let inputs = artifact.inputs().iter().map(|input| {
                    let name = mech_engine::decode_source_input_name(&input.name).unwrap();
                    let data = match name.as_str() {
                        "text" => ValueDataDraft::String(text.into()), "width" => ValueDataDraft::U64(width),
                        "n" => ValueDataDraft::U64(n), "start" => ValueDataDraft::U64(start),
                        _ => panic!("unexpected input {name}"),
                    };
                    let value = ValueDraft { schema: input.schema, shape_values: Box::new([]), data }
                        .finalize(&SnapshotValidationContext::new(artifact.schemas())).unwrap();
                    let mut reservation = memory.reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap()).unwrap();
                    value.into_memory_budget(&mut reservation).unwrap()
                }).collect::<Vec<_>>();
                let mut outputs = vec![None]; let terminal = Cell::new(None); let entry = jit.test_checked_entry();
                let mut limits = tests::limits();
                if full_refill_budget {
                    // The default comparison cap is 65,536: these loops need
                    // over 337,000 units and cannot reach a 1,024-header loan.
                    // Keep those original-cap failures above, then separately
                    // fund complete loans with explicit finite TEST limits.
                    limits.execution.max_work = 1 << 40;
                    limits.library.max_compute_work = Some(1 << 36);
                    limits.library.max_comparison_work = Some(1 << 34);
                }
                let mut outcome = execute(jit.descriptor(), |frame| {
                    // SAFETY: inspect only the synchronous owned entry/context.
                    let old = unsafe { (*frame).helper }; unsafe { (*frame).helper = observed; }
                    let status = unsafe { entry(frame) }; unsafe { (*frame).helper = old; }
                    let ctx = unsafe { &*(*frame).base.context.cast::<Context<'_, '_>>() };
                    terminal.set(Some((status, (status != STATUS_OK).then(|| unsafe { (*frame).base.error_node }),
                        ctx.control.work.used(), ctx.library.used())));
                    status
                }, &inputs, &mut outputs, limits, &memory).map_err(|e| format!("{e:?}"));
                let pairs = outputs[0].as_ref().map(|value| {
                    let ValueData::Map(map) = value.data() else { panic!("map"); };
                    map.entries().iter().map(|e| match (e.key().data(), e.value()) {
                        (ValueData::String(k), ValueData::U64(v)) => (k.to_string(), *v), _ => panic!("String/U64 map"),
                    }).collect::<std::collections::BTreeMap<_, _>>()
                });
                let mut obs = OBS.with(|r| core::mem::take(&mut *r.borrow_mut()));
                assert!(obs.errors.is_empty(), "{obs:?}");
                let mutation_selected = [mutation_optimized, mutation_baseline][version];
                let selected = u64::from((version == 0 || compare_lookup_facts || compare_cfg_cleanup || compare_outlining || compare_invariants || compare_preserving || compare_mutation || compare_counts || compare_constants || compare_steady)
                    && (!mutation_selected || start == 0) && obs.packed == 1 && width > 0 && width <= n && n <= text.len() as u64);
                assert_eq!(obs.bound[14], selected, "actual selector, version={version}, width={width}, n={n}");
                if let Ok(report) = &mut outcome {
                    assert_eq!(report.guarded_loop_regions, selected);
                    // Compare every existing report field unchanged after
                    // independently checking the new optimization receipt.
                    assert!(report.guarded_call_regions <= report.guarded_loop_regions);
                    report.guarded_call_regions = 0;
                    report.guarded_loop_regions = 0;
                }
                if mode == Mode::WriteTransition {
                    assert!(outcome.is_err());
                    assert_eq!((obs.bound[7], obs.bound[12]), (1, 1), "one non-growing insertion then original write-cap denial");
                    if compare_invariants { assert_eq!(obs.guarded_calls, u64::from(version == 0)); }
                }
                obs.bound[14] = 0;
                let frontiers = PUBLICATION_FRONTIERS.with(|r| core::mem::take(&mut *r.borrow_mut()));
                if width == 0 { assert_eq!(pairs, Some(std::collections::BTreeMap::new()), "a nonexecuted call must not fail early"); }
                if mode == Mode::Native && n <= text.len() as u64 && outcome.is_ok() {
                    let mut expected = std::collections::BTreeMap::new();
                    if width > 0 && width <= n && start <= n - width {
                        for i in start..=n-width {
                            *expected.entry(text[i as usize..(i+width) as usize].to_owned()).or_insert(0u64) += 1;
                        }
                    }
                    assert_eq!(pairs.as_ref(), Some(&expected));
                    if width > 0 && !text.is_empty() {
                        let report = outcome.as_ref().unwrap();
                        assert_eq!((report.packed_map_regions, report.packed_ssa_regions), (1, 1));
                        assert_eq!(report.packed_map_updates, expected.values().sum::<u64>());
                    }
                }
                executions.push((outcome, pairs, terminal.get().unwrap(), frontiers, obs.bound));
                drop((outputs, inputs)); assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
            }
            let (a,b) = (&executions[0], &executions[1]);
            assert_eq!(a.1, b.1, "complete map, width={width}, n={n}, start={start}");
            assert_eq!((a.2.0,a.2.1), (b.2.0,b.2.1), "first source status/origin");
            assert_eq!(a.3, b.3, "exact source work and final fuel");
            assert_eq!(a.4, b.4, "same source helper/debit/refill frontiers");
            match (&a.0, &b.0) {
                (Ok(a),Ok(b)) => assert_eq!(structural_report(a), structural_report(b)),
                _ => assert_eq!(a.0,b.0),
            }
            if a.0.is_err() || a.1.as_ref().is_some_and(|m| m.len() <= 1) {
                assert_eq!(a.0,b.0); assert_eq!(a.2,b.2, "deterministic total work");
            }
            // Check both variants before requiring success so an unexpectedly
            // tight fixture budget reports their actual matching error/work.
            if expected_default_denial {
                let error = a.0.as_ref().expect_err("original finite comparison cap must refuse this long loop");
                assert!(error.contains("dimension: ComparisonWork"), "{error}");
                let limit = tests::limits().library.max_comparison_work.unwrap();
                assert!(error.contains(&format!("limit: {limit}")), "{error}");
            } else if mode == Mode::Native && n <= text.len() as u64 {
                assert!(a.0.is_ok(), "valid oracle case failed: width={width}, n={n}, start={start}; optimized={:?}; checked={:?}; frontier={:?}", a.0, b.0, a.2);
            }
        }
    }
    MODE.with(|m| m.set(Mode::Native));
}

#[test]
fn guarded_loop_metadata_records_emission_and_declines_changed_source_obligations() {
    let source = structural_projection_tests::COUNT;
    for (body, accepted) in [
        (source.replace("Next", "Advance").replace("Done", "Finish"), true),
        (source.replace("index >= n", "index == n"), false),
        (source.replace("index >= n", "n - index == 0u64"), false),
        (source.replace("    | width > n - index -> :Done(counts)\n", ""), false),
        (source.replace(":Next(index + 1u64, increment", ":Next(index + 2u64, increment"), false),
        (source.replace("index >= n -> :Done(counts)", "index >= n -> :Done(initial)"), false),
        (source.replace("(index + width)]", "(index + index + 1u64)]"), false),
    ] {
        let artifact = tests::artifact_source(&format!("{body}\n#Count(\"abcabc\", 2u64, 6u64, empty)\n"));
        let catalog = mech_stdlib::source_catalog(); take_loop_bounds_emissions();
        let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
        let emitted = take_loop_bounds_emissions(); let metadata = object.metadata();
        let receipt = &metadata["compiled_guarded_loops"];
        assert_eq!(receipt["complete"], true);
        let regions = receipt["regions"].as_array().unwrap();
        assert_eq!(regions.len(), emitted.0); assert_eq!(!regions.is_empty(), accepted, "{body}");
        if let Some(region) = regions.first() {
            assert_eq!(region["checked_fallback"], true);
            assert_eq!(region["stop_guards"].as_array().unwrap().len(), 3);
            assert!(region["emitted_nodes"].as_u64().unwrap() <= 1024);
            assert!((1..=1024).contains(&region["fixed_template_instructions"].as_u64().unwrap()));
            assert_eq!(region["certified_range_versions"].as_array().unwrap().len(), emitted.1);
            let nodes = region["emitted_nodes"].as_u64().unwrap();
            let header = region["header"].as_u64().unwrap();
            let invariant = metadata["compiled_loop_invariants"].as_array().unwrap().iter()
                .find(|r| r["header"] == header);
            let first_guarded = invariant.map_or(nodes / 2, |r| {
                let stride = r["version_stride"].as_u64().unwrap();
                assert_eq!(nodes, 4 * stride);
                stride
            });
            for version in region["certified_range_versions"].as_array().unwrap() {
                assert!((first_guarded..nodes).contains(&version.as_u64().unwrap()), "only guarded versions may omit range bounds");
            }
            let descriptor = object.descriptor();
            let header = region["header"].as_u64().unwrap() as usize;
            let region_id = descriptor.closed_tails.sites[header].unwrap();
            for site in region["stop_guards"].as_array().unwrap() {
                let site = site.as_u64().unwrap() as usize;
                assert_eq!(descriptor.closed_tails.sites[site], Some(region_id));
                assert!(matches!(descriptor.plan.blocks()[site].terminator(),
                    NativeIterationTerminator::Scalar(NativeMatchTerminator::Branch { .. })));
            }
            for latch in region["recurrence_latches"].as_array().unwrap() {
                assert_eq!(*latch, descriptor.closed_tails.regions[region_id].tail.latch());
            }
        }
    }
}

#[test]
fn guarded_loop_count_overflow_stays_at_the_source_replacement() {
    let body = structural_projection_tests::COUNT.replace("previous + 1u64", "previous + 18446744073709551615u64");
    let artifact = tests::artifact_source(&format!("{body}\n#Count(\"aa\", 1u64, 2u64, empty)\n"));
    let catalog = mech_stdlib::source_catalog(); take_loop_bounds_emissions();
    let optimized = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
    assert_eq!(take_loop_bounds_emissions().0, 1);
    let baseline = without_loop_bounds(|| JitProgramArtifact::compile(&artifact, &catalog)).unwrap();
    assert_eq!(take_loop_bounds_emissions(), (0,0));
    let mut results = Vec::new();
    for (version, jit) in [&optimized, &baseline].into_iter().enumerate() {
        MODE.with(|m| m.set(Mode::Native)); OBS.with(|r| *r.borrow_mut() = Observation::default());
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024); let mut outputs = vec![None];
        let terminal = Cell::new(None); let entry = jit.test_checked_entry();
        let error = execute(jit.descriptor(), |frame| {
            // SAFETY: same pinned synchronous entry and maintained observer.
            let old = unsafe { (*frame).helper }; unsafe { (*frame).helper = observed; }
            let status = unsafe { entry(frame) }; unsafe { (*frame).helper = old; }
            let ctx = unsafe { &*(*frame).base.context.cast::<Context<'_, '_>>() };
            terminal.set(Some((status, unsafe { (*frame).base.error_node }, ctx.control.work.used(), ctx.library.used())));
            status
        }, &[], &mut outputs, tests::limits(), &memory).unwrap_err();
        let receipt = terminal.get().unwrap(); assert_eq!(receipt.0, STATUS_ARITHMETIC);
        let origin = jit.descriptor().plan.blocks()[receipt.1 as usize].origin();
        assert!(matches!(&error, NativeProgramExecutionError::Control(
            NativeIterationExecutionError::Arithmetic { origin: actual }) if *actual == origin));
        let mut obs = OBS.with(|r| core::mem::take(&mut *r.borrow_mut()));
        assert_eq!((obs.packed, obs.bound[7]), (1,1), "first source update ran; second replacement overflows");
        assert_eq!(obs.bound[14], u64::from(version == 0));
        obs.bound[14] = 0;
        results.push((format!("{error:?}"), receipt, obs.bound));
        drop(outputs); assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0,0));
    }
    assert_eq!(results[0], results[1]);
}
