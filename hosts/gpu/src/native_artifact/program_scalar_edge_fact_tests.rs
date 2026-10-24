//! Compare the actual optimized entry with the same packed SSA entry without
//! substitutions: all source work, results, failures and origins must agree.
use super::*;
use crate::native_artifact::program_compiler::packed_loop_codegen::{
    take_edge_fact_emissions, without_edge_facts,
    take_tag_thread_emissions, without_tag_threading,
    take_frame_effect_emissions, without_frame_effects,
    take_lookup_fact_emissions, without_lookup_facts,
    take_outline_emissions, without_loop_outlining,
    take_invariant_emissions, without_loop_invariants,
};
use crate::native_artifact::JitProgramArtifact;
use crate::native_artifact::program_compiler::cfg_cleanup::{take_cfg_cleanup_emissions, without_cfg_cleanup};

#[derive(Clone, Copy)]
enum Pass { EdgeFacts, TagThreading, FrameEffects, LookupFacts, CfgCleanup, LoopOutlining, LoopInvariants, PreservingCalls }
impl Pass {
    fn disabled<T>(self, f: impl FnOnce() -> T) -> T {
        match self {
            Self::EdgeFacts => without_edge_facts(f),
            Self::TagThreading => without_tag_threading(f),
            Self::FrameEffects => without_frame_effects(f),
            Self::LookupFacts => without_lookup_facts(f),
            Self::CfgCleanup => without_cfg_cleanup(f),
            Self::LoopOutlining => without_loop_outlining(f),
            Self::LoopInvariants => without_loop_invariants(f),
            Self::PreservingCalls => crate::native_artifact::program_bound_shim::without_preserving_calls(f),
        }
    }
    fn check(self, edges: (usize, usize), tags: (usize, usize, usize), effects: (usize, usize, usize), selected: bool) {
        let lookups = take_lookup_fact_emissions();
        let cfg = take_cfg_cleanup_emissions();
        let outlined = take_outline_emissions();
        let invariants = take_invariant_emissions();
        let preserving = crate::native_artifact::program_bound_shim::take_emissions();
        match self {
            Self::PreservingCalls if selected && cfg!(target_arch = "aarch64") => assert!(preserving > 0),
            Self::PreservingCalls if !selected => assert_eq!(preserving, 0),
            Self::LoopInvariants if !selected => assert_eq!(invariants, (0, 0, 0)),
            Self::LoopOutlining if selected => assert!(outlined > 0),
            Self::LoopOutlining => assert_eq!(outlined, 0),
            Self::CfgCleanup if selected => assert!(cfg.0 > 0 && cfg.1 > 0, "{cfg:?}"),
            Self::CfgCleanup => assert_eq!(cfg, (0, 0)),
            Self::LookupFacts if selected => assert!(lookups.0 > 0 && lookups.1 >= 3, "{lookups:?}"),
            Self::LookupFacts => assert_eq!(lookups, (0, 0)),
            Self::EdgeFacts if !selected => assert_eq!(edges, (0, 0)),
            Self::TagThreading if selected => assert!(tags.0 > 0 && tags.1 >= 4 && tags.2 >= 2, "{tags:?}"),
            Self::TagThreading => assert_eq!(tags, (0, 0, 0)),
            Self::FrameEffects if selected => assert!(effects.0 > 0
                && effects.1 < 14 * effects.0 && effects.2 < 14 * effects.0, "{effects:?}"),
            Self::FrameEffects => assert_eq!(effects, (0, 0, 0)),
            _ => {}
        }
    }
}

#[path = "program_loop_bounds_tests.rs"]
mod loop_bounds_tests;

type PublicationFrontier = (u64, u64, u64, (u64, u64));
thread_local! {
    static PUBLICATION_FRONTIERS: RefCell<Vec<PublicationFrontier>> = const { RefCell::new(Vec::new()) };
}
unsafe extern "C" fn observe_publication_frontier(raw: *mut c_void, site: u64, op: u64,
    first: u64, second: u64, third: u64, result: *mut u64) -> u64
{
    if op == CLOSED_TAIL_END {
        // SAFETY: this observer receives the same pinned owned context as the
        // maintained helper. Capture before table import/publication, whose
        // independently seeded hash iteration can change heapsort's work.
        let ctx = unsafe { &*raw.cast::<Context<'_, '_>>() };
        PUBLICATION_FRONTIERS.with(|records| records.borrow_mut().push((site, first,
            ctx.control.work.used(), ctx.library.used())));
    }
    // SAFETY: unchanged arguments and original maintained helper contract.
    unsafe { helper(raw, site, op, first, second, third, result) }
}

fn structural_report(report: &NativeProgramExecutionReport) -> [u64; 11] {
    [report.visits, report.transport_bytes, report.hash_table_regions, report.hash_table_updates,
        report.packed_map_regions, report.packed_map_updates, report.packed_ssa_regions,
        report.parallel_regions, report.parallel_batches, report.parallel_tasks, report.parallel_peak_workers]
}

#[test]
fn loop_invariants_metadata_binds_actual_calls_ranges_latches_and_refreshes() {
    let source = format!("{}\n#Count(\"abcabc\", 2u64, 6u64, empty)\n", structural_projection_tests::COUNT);
    let artifact = tests::artifact_source(&source);
    let catalog = mech_stdlib::source_catalog();
    take_invariant_emissions();
    let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
    let emitted = take_invariant_emissions();
    assert!(emitted.0 == 1 && emitted.1 >= 8 && emitted.2 > 0, "{emitted:?}");
    let metadata = object.metadata();
    let regions = metadata["compiled_loop_invariants"].as_array().unwrap();
    assert_eq!(regions.len(), 1);
    let r = &regions[0];
    let stride = r["version_stride"].as_u64().unwrap();
    assert!(stride > 0 && stride <= 256);
    assert_eq!(r["emitted_nodes"], 4 * stride);
    assert!(r["fixed_template_work"].as_u64().unwrap() <= 1024);
    let bounds = &metadata["compiled_guarded_loops"]["regions"][0];
    for record in r["call_substitutions"].as_array().unwrap() {
        let node = record[0].as_u64().unwrap();
        let site = record[1].as_u64().unwrap() as usize;
        assert!((2 * stride..4 * stride).contains(&node));
        if record[2] == 10 { assert_eq!(record[1], bounds["range_site"]); }
        else {
            let NativeIterationAction::LibraryCall { template, .. } = object.descriptor().plan.blocks()[site].action() else { panic!("original source map call"); };
            let op = match template.primitive() {
                mech_engine::native_artifact::NativeCollectionPrimitive::MapContainsKey => 3,
                mech_engine::native_artifact::NativeCollectionPrimitive::MapKeyAccess => 4,
                mech_engine::native_artifact::NativeCollectionPrimitive::MapInsert => 5,
                _ => panic!("unrelated source operation"),
            };
            assert_eq!(record[2], op);
        }
    }
    for record in r["rolling_substitutions"].as_array().unwrap() {
        assert!((3 * stride..4 * stride).contains(&record[0].as_u64().unwrap()));
        assert_eq!(record[1], bounds["range_site"]);
    }
    assert!(!r["steady_latches"].as_array().unwrap().is_empty());
    for record in r["steady_latches"].as_array().unwrap() {
        assert!(bounds["recurrence_latches"].as_array().unwrap().contains(&record[1]));
    }
    assert_eq!(r["refreshes"].as_array().unwrap().len(), 3);
    let old = without_loop_invariants(|| crate::native_artifact::emit_program_object(&artifact, &catalog)).unwrap();
    assert!(old.metadata()["compiled_loop_invariants"].as_array().unwrap().is_empty());
    assert_eq!(take_invariant_emissions(), (0, 0, 0));
    for changed in [source.replace("index >= n", "index == n"),
        source.replace(":Next(index + 1u64, increment", ":Next(index + 2u64, increment")]
    {
        let artifact = tests::artifact_source(&changed);
        let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
        assert!(object.metadata()["compiled_loop_invariants"].as_array().unwrap().is_empty());
    }
}

#[test]
fn loop_outlining_metadata_and_total_stack_code_admission_include_owned_callees() {
    let artifact = tests::artifact_source(&format!("{}\n#Count(\"abcabc\", 2u64, 6u64, empty)\n",
        structural_projection_tests::COUNT));
    let catalog = mech_stdlib::source_catalog();
    take_outline_emissions();
    let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
    assert_eq!(take_outline_emissions(), 1);
    let metadata = object.metadata();
    let regions = metadata["compiled_outlined_loops"].as_array().unwrap();
    assert_eq!(regions.len(), 1);
    let region = &regions[0];
    let header = region["header"].as_u64().unwrap() as usize;
    assert!(object.descriptor().closed_tails.regions.iter().any(|r|
        r.packed.is_some() && r.tail.header() as usize == header));
    assert!(region["symbol"].as_str().unwrap().ends_with(&format!("_packed_loop_{header}")));
    assert_eq!(region["caller_template_bound"], 64);
    assert_eq!(region["cleanup"]["enabled"], true);
    let stack = metadata["stack_bytes"].as_u64().unwrap();
    let bytes = metadata["code_bytes"].as_u64().unwrap();
    assert!(stack >= region["stack_bytes"].as_u64().unwrap());
    assert!(bytes > region["code_bytes"].as_u64().unwrap());
    let mut limits = crate::native_artifact::NativeCompileLimits::default();
    limits.max_stack_bytes = stack;
    limits.max_code_bytes = bytes;
    assert!(crate::native_artifact::emit_program_object_with_limits(&artifact, &catalog, limits).is_ok());
    limits.max_stack_bytes = stack - 1;
    assert!(crate::native_artifact::emit_program_object_with_limits(&artifact, &catalog, limits).is_err());
    limits.max_stack_bytes = stack;
    limits.max_code_bytes = bytes - 1;
    assert!(crate::native_artifact::emit_program_object_with_limits(&artifact, &catalog, limits).is_err());
    let inline = without_loop_outlining(|| crate::native_artifact::emit_program_object(&artifact, &catalog)).unwrap();
    assert!(inline.metadata()["compiled_outlined_loops"].as_array().unwrap().is_empty());
}

#[test]
fn preserving_calls_metadata_and_total_stack_code_include_the_nested_bridge() {
    let artifact = tests::artifact_source(&format!("{}\n#Count(\"abcabc\", 2u64, 6u64, empty)\n",
        structural_projection_tests::COUNT));
    let catalog = mech_stdlib::source_catalog();
    let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
    let metadata = object.metadata();
    let shim = &metadata["compiled_bound_call_shim"];
    if !cfg!(target_arch = "aarch64") { assert!(shim.is_null()); return; }
    assert!(shim["symbol"].as_str().unwrap().ends_with("_preserving_bound_call"));
    assert_eq!(shim["template_bound"], 128);
    assert!(shim["code_bytes"].as_u64().unwrap() / 4 <= 128);
    assert!(shim["stack_bytes"].as_u64().unwrap() >= 16);
    let regions = metadata["compiled_preserving_call_regions"].as_array().unwrap();
    assert_eq!(regions.len(), 1);
    let loops = metadata["compiled_outlined_loops"].as_array().unwrap();
    assert_eq!(regions[0]["header"], loops[0]["header"]);
    assert!(regions[0]["emitted_calls"].as_u64().unwrap() > 0);
    assert!(regions[0]["maximum_source_template_work"].as_u64().unwrap() <= 512);
    assert_eq!(regions[0]["extra_entry_work"], 128);
    let stack = metadata["stack_bytes"].as_u64().unwrap();
    let code = metadata["code_bytes"].as_u64().unwrap();
    assert!(stack >= loops[0]["stack_bytes"].as_u64().unwrap() + shim["stack_bytes"].as_u64().unwrap() + 16);
    assert!(code > loops[0]["code_bytes"].as_u64().unwrap() + shim["code_bytes"].as_u64().unwrap());
    let mut limits = crate::native_artifact::NativeCompileLimits::default();
    limits.max_stack_bytes = stack; limits.max_code_bytes = code;
    assert!(crate::native_artifact::emit_program_object_with_limits(&artifact, &catalog, limits).is_ok());
    limits.max_stack_bytes -= 1;
    assert!(crate::native_artifact::emit_program_object_with_limits(&artifact, &catalog, limits).is_err());
    limits.max_stack_bytes = stack; limits.max_code_bytes -= 1;
    assert!(crate::native_artifact::emit_program_object_with_limits(&artifact, &catalog, limits).is_err());
    let ordinary = crate::native_artifact::program_bound_shim::without_preserving_calls(||
        crate::native_artifact::emit_program_object(&artifact, &catalog)).unwrap();
    assert!(ordinary.metadata()["compiled_bound_call_shim"].is_null());
    assert!(ordinary.metadata()["compiled_preserving_call_regions"].as_array().unwrap().is_empty());
}
#[test]
fn preserving_calls_keep_changed_inputs_growth_refills_and_exact_work() {
    changed_input_equivalence(Pass::PreservingCalls);
}
#[test]
fn preserving_calls_keep_arithmetic_failures_and_declined_proofs_at_source_sites() {
    arithmetic_failure_equivalence(Pass::PreservingCalls);
}

#[test]
fn lookup_facts_metadata_records_only_actual_current_key_substitutions() {
    let artifact = tests::artifact_source(&format!("{}\n#Count(\"abcabc\", 2u64, 6u64, empty)\n",
        structural_projection_tests::COUNT));
    let catalog = mech_stdlib::source_catalog();
    take_lookup_fact_emissions();
    let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
    let actual = take_lookup_fact_emissions();
    let metadata = object.metadata();
    let receipt = &metadata["compiled_lookup_facts"];
    assert_eq!(receipt["complete"], true);
    let regions = receipt["regions"].as_array().unwrap();
    assert_eq!(regions.len(), 1);
    assert_eq!(actual.0, 1);
    let region = &regions[0];
    let nodes = region["emitted_nodes"].as_u64().unwrap();
    assert!(nodes <= 2048);
    let substitutions = region["substitutions"].as_array().unwrap();
    assert_eq!(substitutions.len(), actual.1);
    let mut facts = std::collections::BTreeSet::new();
    for record in substitutions {
        assert!(record["node"].as_u64().unwrap() < nodes);
        let site = record["site"].as_u64().unwrap() as usize;
        let NativeIterationAction::LibraryCall { template, .. } = object.descriptor().plan.blocks()[site].action() else { panic!("source call"); };
        let operation = match template.primitive() {
            mech_engine::native_artifact::NativeCollectionPrimitive::MapContainsKey => 3,
            mech_engine::native_artifact::NativeCollectionPrimitive::MapKeyAccess => 4,
            mech_engine::native_artifact::NativeCollectionPrimitive::MapInsert => 5,
            _ => panic!("unrelated operation"),
        };
        assert_eq!(record["operation"], operation);
        facts.insert(record["fact"].as_str().unwrap());
    }
    assert!(facts.contains("Fresh") && facts.contains("Present") && facts.contains("Cached"), "{facts:?}");
    let baseline = without_lookup_facts(|| crate::native_artifact::emit_program_object(&artifact, &catalog)).unwrap();
    assert_eq!(take_lookup_fact_emissions(), (0, 0));
    let receipt = baseline.metadata();
    assert_eq!(receipt["compiled_lookup_facts"]["complete"], true);
    assert!(receipt["compiled_lookup_facts"]["regions"][0]["substitutions"].as_array().unwrap().is_empty());
}

#[test]
fn object_metadata_records_actual_frame_sync_masks_and_disabled_absence() {
    let artifact = tests::artifact_source(&format!("{}\n#Count(\"abcabc\", 2u64, 6u64, empty)\n",
        structural_projection_tests::COUNT));
    let catalog = mech_stdlib::source_catalog();
    take_frame_effect_emissions();
    let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
    let actual = take_frame_effect_emissions();
    let metadata = object.metadata();
    let receipt = &metadata["compiled_frame_effects"];
    assert_eq!(receipt["complete"], true);
    let regions = receipt["regions"].as_array().unwrap();
    assert_eq!(regions.len(), 1);
    let tail = object.descriptor().closed_tails.regions.iter().find(|r| r.packed.is_some()).unwrap();
    assert_eq!(regions[0]["header"], tail.tail.header());
    let operations = regions[0]["operations"].as_array().unwrap();
    assert!(operations.len() <= 14);
    let mut totals = (0, 0, 0);
    let mut seen = std::collections::BTreeSet::new();
    for operation in operations {
        let op = operation["operation"].as_u64().unwrap();
        assert!(seen.insert(op));
        let count = operation["emitted_calls"].as_u64().unwrap() as usize;
        assert!(count > 0);
        let (stores, loads): (&[u64], &[u64]) = match op {
            0 => (&[3, 8], &[3, 7, 8]),
            7 => (&[3, 4, 5, 6, 7, 8], &[0, 1, 2, 4, 7, 15, 16, 17, 18]),
            6 | 8 | 9 => (&[], &[]),
            10..=12 => (&[3, 8], &[]),
            _ => panic!("unproved operation cannot claim narrow synchronization"),
        };
        assert_eq!(operation["stored_words"], serde_json::json!(stores));
        assert_eq!(operation["reloaded_words"], serde_json::json!(loads));
        totals.0 += count;
        totals.1 += count * stores.len();
        totals.2 += count * loads.len();
    }
    assert!(seen.contains(&0) && seen.contains(&7), "actual refill and insertion templates");
    assert_eq!(totals, actual);
    assert!(actual.1 < actual.0 * 14 && actual.2 < actual.0 * 14);
    let plain = without_frame_effects(|| crate::native_artifact::emit_program_object(&artifact, &catalog)).unwrap();
    assert_eq!(take_frame_effect_emissions(), (0, 0, 0));
    let metadata = plain.metadata();
    let regions = metadata["compiled_frame_effects"]["regions"].as_array().unwrap();
    assert_eq!(metadata["compiled_frame_effects"]["complete"], true);
    assert_eq!(regions.len(), 1);
    assert!(regions[0]["operations"].as_array().unwrap().is_empty());
    assert!(!metadata["compiled_tag_threading"]["regions"].as_array().unwrap().is_empty());
}

#[test]
fn object_metadata_attests_tag_threading_and_original_source_frontiers() {
    let body = structural_projection_tests::COUNT.replace("Next", "Advance").replace("Done", "Finish");
    let artifact = tests::artifact_source(&format!("{body}\n#Count(\"abcabc\", 2u64, 6u64, empty)\n"));
    let catalog = mech_stdlib::source_catalog();
    take_tag_thread_emissions();
    let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
    let emitted = take_tag_thread_emissions();
    let metadata = object.metadata();
    let receipt = &metadata["compiled_tag_threading"];
    assert_eq!(receipt["complete"], true);
    let regions = receipt["regions"].as_array().unwrap();
    assert_eq!(regions.len(), 1);
    let record = &regions[0];
    let descriptor = object.descriptor();
    let tail = descriptor.closed_tails.regions.iter().find(|r| r.packed.is_some()).unwrap();
    let owner = descriptor.map_owner_words.as_ref().unwrap();
    assert_eq!(record["header"], tail.tail.header());
    assert_eq!(record["carrier_word"], owner.aggregates().slot(tail.tail.carrier()).unwrap().0);
    assert_eq!(record["variants"].as_array().unwrap().len(), 2);
    assert_eq!(record["product_nodes"], emitted.0);
    assert!(emitted.0 <= 1024 && emitted.1 >= 4 && emitted.2 >= 2, "{emitted:?}");
    assert_eq!(record["folded_branches"].as_array().unwrap().len(), emitted.1);
    assert_eq!(record["known_latches"].as_array().unwrap().len(), emitted.2);
    assert!(record["header_versions"].as_u64().unwrap() >= 3,
        "initial, recurring and terminal headers retain their own fuel traversal");
    for site in record["folded_branches"].as_array().unwrap() {
        let site = site.as_u64().unwrap() as usize;
        assert!(matches!(descriptor.plan.blocks()[site].terminator(), NativeIterationTerminator::Scalar(
            NativeMatchTerminator::Branch { .. } | NativeMatchTerminator::LiteralBranch { .. })));
        assert_eq!(descriptor.closed_tails.sites[site], descriptor.closed_tails.sites[tail.tail.header() as usize]);
    }
    for site in record["known_latches"].as_array().unwrap() { assert_eq!(*site, tail.tail.latch()); }
    let plain = without_tag_threading(|| crate::native_artifact::emit_program_object(&artifact, &catalog)).unwrap();
    assert_eq!(take_tag_thread_emissions(), (0, 0, 0));
    let plain = plain.metadata();
    assert_eq!(plain["compiled_tag_threading"]["complete"], true);
    assert!(plain["compiled_tag_threading"]["regions"].as_array().unwrap().is_empty());
    assert!(!plain["compiled_source_edge_facts"]["regions"].as_array().unwrap().is_empty(),
        "disabling tag threading must retain the preceding SSA/edge-fact optimization");
}

#[test]
fn tag_threading_accepts_reordered_variants_and_an_initial_terminal_state() {
    let body = structural_projection_tests::COUNT.replace(
        "  | :Next(index<u64>, counts<{string:u64}>)\n  | :Done(counts<{string:u64}>).",
        "  | :Done(counts<{string:u64}>)\n  | :Next(index<u64>, counts<{string:u64}>).");
    assert_ne!(body, structural_projection_tests::COUNT);
    for terminal in [false, true] {
        let body = if terminal { body.replace("-> :Next(0u64, initial)", "-> :Done(initial)") } else { body.clone() };
        let original = tests::artifact_source(&format!("{body}\n#Count(\"aaaa\", 2u64, 4u64, empty)\n"));
        let artifact = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap()).unwrap();
        let catalog = mech_stdlib::source_catalog();
        take_tag_thread_emissions();
        let threaded = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let actual = take_tag_thread_emissions();
        assert!(actual.0 > 0 && actual.1 >= 4 && actual.2 >= 2, "{actual:?}");
        let plain = without_tag_threading(|| JitProgramArtifact::compile(&artifact, &catalog)).unwrap();
        assert_eq!(take_tag_thread_emissions(), (0, 0, 0));
        let mut executions = Vec::new();
        for (version, jit) in [&threaded, &plain].into_iter().enumerate() {
            PUBLICATION_FRONTIERS.with(|records| records.borrow_mut().clear());
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let mut outputs = vec![None];
            let entry = jit.test_checked_entry();
            let mut report = execute(jit.descriptor(), |frame| {
                // SAFETY: same synchronous owning entry/observer as above.
                let old = unsafe { (*frame).helper };
                unsafe { (*frame).helper = observe_publication_frontier; }
                let status = unsafe { entry(frame) };
                unsafe { (*frame).helper = old; }
                status
            }, &[], &mut outputs, tests::limits(), &memory).unwrap();
            assert_eq!((report.packed_map_regions, report.packed_ssa_regions, report.packed_map_updates),
                (1, 1, if terminal { 0 } else { 3 }));
            assert_eq!(report.guarded_loop_regions, u64::from(version == 0));
            report.guarded_loop_regions = 0;
                    report.guarded_call_regions = 0;
            let ValueData::Map(map) = outputs[0].as_ref().unwrap().data() else { panic!("map"); };
            let pairs = map.entries().iter().map(|entry| match (entry.key().data(), entry.value()) {
                (ValueData::String(key), ValueData::U64(value)) => (key.to_string(), *value),
                _ => panic!("String/U64 map"),
            }).collect::<Vec<_>>();
            assert_eq!(pairs, if terminal { Vec::new() } else { vec![("aa".to_owned(), 3)] });
            let frontiers = PUBLICATION_FRONTIERS.with(|records| core::mem::take(&mut *records.borrow_mut()));
            assert_eq!(frontiers.len(), 1);
            executions.push((report, pairs, frontiers));
            drop(outputs);
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
        }
        assert_eq!(executions[0], executions[1], "terminal traversal, full accounting, and output are unchanged");
    }
}

#[test]
fn object_metadata_records_actual_source_edge_substitutions_and_disabled_absence() {
    let artifact = tests::artifact_source(&format!("{}\n#Count(\"abcabc\", 2u64, 6u64, empty)\n",
        structural_projection_tests::COUNT));
    let catalog = mech_stdlib::source_catalog();
    take_edge_fact_emissions();
    let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
    let emitted = take_edge_fact_emissions();
    let metadata = object.metadata();
    let receipt = &metadata["compiled_source_edge_facts"];
    assert_eq!(receipt["complete"], true);
    let regions = receipt["regions"].as_array().unwrap();
    assert_eq!(regions.len(), 1);
    let descriptor = object.descriptor();
    let closed = descriptor.closed_tails.regions.iter().find(|r| r.packed.is_some()).unwrap();
    assert_eq!(regions[0]["header"], closed.tail.header());
    let guards = regions[0]["guards"].as_array().unwrap();
    let arithmetic = regions[0]["arithmetic"].as_array().unwrap();
    assert_eq!((guards.len(), arithmetic.len()), emitted);
    assert!(emitted.0 >= 8 && emitted.1 >= 4, "actual object substitutions: {emitted:?}");
    let owner = descriptor.map_owner_words.as_ref().unwrap();
    for record in guards {
        let site = record["site"].as_u64().unwrap() as usize;
        let action = if owner.action_region(site) == closed.map {
            owner.aggregates().action(site)
        } else { descriptor.scalar_aggregates.action(site) }.unwrap();
        use mech_engine::native_artifact::NativeScalarAggregateAction as Aggregate;
        let (source, guards) = match action {
            Aggregate::Observe { source, guards, .. } | Aggregate::Project { source, guards, .. } => (source, guards),
            _ => panic!("reported source guard site"),
        };
        assert_eq!(record["source"], serde_json::to_value(source).unwrap());
        assert!(guards.iter().any(|&(offset, ordinal)| record["offset"] == offset && record["ordinal"] == ordinal));
    }
    for record in arithmetic {
        let site = record["site"].as_u64().unwrap() as usize;
        let NativeIterationAction::Scalar(NativeMatchAction::Primitive { op, operands, .. }) =
            descriptor.plan.blocks()[site].action() else { panic!("reported arithmetic source site"); };
        assert_eq!(record["op"], serde_json::to_value(op).unwrap());
        assert_eq!(record["operands"], serde_json::to_value(operands).unwrap());
    }
    let disabled = without_edge_facts(|| crate::native_artifact::emit_program_object(&artifact, &catalog)).unwrap();
    assert_eq!(take_edge_fact_emissions(), (0, 0));
    let metadata = disabled.metadata();
    let receipt = &metadata["compiled_source_edge_facts"];
    assert_eq!(receipt["complete"], true);
    let regions = receipt["regions"].as_array().unwrap();
    assert_eq!(regions.len(), 1, "SSA remains selected when only edge substitutions are disabled");
    assert_eq!(regions[0]["header"], closed.tail.header());
    assert!(regions[0]["guards"].as_array().unwrap().is_empty());
    assert!(regions[0]["arithmetic"].as_array().unwrap().is_empty());
}

#[test]
fn current_edge_facts_preserve_changed_inputs_source_names_and_exact_work() {
    changed_input_equivalence(Pass::EdgeFacts);
}
#[test]
fn tag_threading_preserves_changed_inputs_source_names_and_exact_terminal_work() {
    changed_input_equivalence(Pass::TagThreading);
}
#[test]
fn frame_effects_preserve_changed_inputs_growth_refills_and_exact_work() {
    changed_input_equivalence(Pass::FrameEffects);
}
#[test]
fn lookup_facts_preserve_changed_inputs_growth_refills_and_exact_work() {
    changed_input_equivalence(Pass::LookupFacts);
}
#[test]
fn cfg_cleanup_preserves_changed_inputs_growth_refills_and_exact_work() {
    changed_input_equivalence(Pass::CfgCleanup);
}
#[test]
fn loop_outlining_preserves_changed_inputs_growth_refills_and_exact_work() {
    changed_input_equivalence(Pass::LoopOutlining);
}
#[test]
fn loop_invariants_preserve_changed_inputs_growth_refills_and_exact_work() {
    changed_input_equivalence(Pass::LoopInvariants);
}
fn changed_input_equivalence(pass: Pass) {
    // These are ordinary typed parameters, not constants captured from the
    // fixture or assertions that n equals this particular String's length.
    let body = structural_projection_tests::COUNT
        .replace("Count", "Accumulate").replace("index", "cursor")
        .replace("width", "extent").replace("counts", "buckets");
    let source = format!("{body}\n#Accumulate(text<string>, extent<u64>, n<u64>, empty)\n");
    let original = tests::artifact_source(&source);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap()).unwrap();
    for artifact in [original, decoded] {
        let catalog = mech_stdlib::source_catalog();
        take_edge_fact_emissions();
        take_tag_thread_emissions();
        take_frame_effect_emissions();
        let optimized = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let emitted = take_edge_fact_emissions();
        assert!(emitted.0 >= 8 && emitted.1 >= 4, "actual emitted guard/arithmetic substitutions: {emitted:?}");
        let selected = take_tag_thread_emissions();
        pass.check(emitted, selected, take_frame_effect_emissions(), true);
        let unchanged = pass.disabled(|| JitProgramArtifact::compile(&artifact, &catalog)).unwrap();
        let unchanged_facts = take_edge_fact_emissions();
        let unchanged_tags = take_tag_thread_emissions();
        pass.check(unchanged_facts, unchanged_tags, take_frame_effect_emissions(), false);
        let long_text = (0..4099).map(|index| char::from(b'a' + ((index * 7 + index / 26) % 26) as u8))
            .collect::<String>();
        let mut cases = vec![
            ("abcabc", 1u64, 6u64), ("abcabc", 2, 6), ("abcabc", 3, 5),
            ("zzzz", 2, 4), ("abcdefgh", 7, 8), ("a", 1, 1),
            ("abc", 2, 0), ("abc", 4, 3), ("", 1, 0),
            // Bounds must still use the actual selected source, not n.
            ("abc", 2, 4),
        ];
        if matches!(pass, Pass::FrameEffects | Pass::LookupFacts | Pass::CfgCleanup | Pass::LoopOutlining | Pass::LoopInvariants) {
            cases.push((long_text.as_str(), 3, long_text.len() as u64));
            // Unknown ASCII domain at width18 requires both key words. They
            // must survive refills although refill never reads either word.
            cases.push((long_text.as_str(), 18, long_text.len() as u64));
        }
        for (text, extent, n) in cases {
            let mut executions = Vec::new();
            for (version, jit) in [&optimized, &unchanged].into_iter().enumerate() {
                PUBLICATION_FRONTIERS.with(|records| records.borrow_mut().clear());
                OBS.with(|records| *records.borrow_mut() = Observation::default());
                let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
                let inputs = artifact.inputs().iter().map(|input| {
                    let name = mech_engine::decode_source_input_name(&input.name).unwrap();
                    let data = match name.as_str() {
                        "text" => ValueDataDraft::String(text.into()),
                        "extent" => ValueDataDraft::U64(extent),
                        "n" => ValueDataDraft::U64(n),
                        _ => panic!("unexpected input {name}"),
                    };
                    let value = ValueDraft { schema: input.schema, shape_values: Box::new([]), data }
                        .finalize(&SnapshotValidationContext::new(artifact.schemas())).unwrap();
                    let mut reservation = memory.reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap()).unwrap();
                    value.into_memory_budget(&mut reservation).unwrap()
                }).collect::<Vec<_>>();
                let mut outputs = vec![None];
                let terminal = Cell::new(None);
                let entry = jit.test_checked_entry();
                let mut limits = tests::limits();
                if text.len() > 4096 {
                    limits.execution.max_work = 1 << 40;
                    limits.library.max_compute_work = Some(1 << 36);
                    limits.library.max_comparison_work = Some(1 << 34);
                }
                let outcome = execute(jit.descriptor(), |frame| {
                    // SAFETY: this is the owning, synchronous checked entry;
                    // the session's frame and context are live until return.
                    let old = unsafe { (*frame).helper };
                    unsafe { (*frame).helper = observe_publication_frontier; }
                    let status = unsafe { entry(frame) };
                    unsafe { (*frame).helper = old; }
                    let ctx = unsafe { &*(*frame).base.context.cast::<Context<'_, '_>>() };
                    terminal.set(Some((status, (status != STATUS_OK).then(|| unsafe { (*frame).base.error_node }),
                        ctx.control.work.used(), ctx.library.used())));
                    status
                }, &inputs, &mut outputs, limits, &memory);
                if text.len() > 4096 {
                    let actual = outcome.as_ref().expect("growth/refill equivalence must execute successfully");
                    assert_eq!((actual.packed_ssa_regions, actual.packed_map_updates), (1, n - extent + 1));
                    OBS.with(|records| {
                        let records = records.borrow();
                        let minimum_refills = (n - extent + 3 + 1023) / 1024;
                        assert!(records.bound[0] >= minimum_refills && records.bound[7] > 56, "{records:?}");
                        assert_eq!(&records.bound[1..7], &[0; 6]);
                    });
                }
                let mut result = outcome.map_err(|error| format!("{error:?}"));
                if let Ok(report) = &mut result {
                    let selected = u64::from((version == 0 || !matches!(pass, Pass::TagThreading)) && report.packed_ssa_regions == 1
                        && extent > 0 && extent <= n && n <= text.len() as u64);
                    assert_eq!(report.guarded_loop_regions, selected);
                    report.guarded_loop_regions = 0;
                    report.guarded_call_regions = 0;
                }
                let pairs = outputs[0].as_ref().map(|value| {
                    let ValueData::Map(map) = value.data() else { panic!("map"); };
                    map.entries().iter().map(|entry| match (entry.key().data(), entry.value()) {
                        (ValueData::String(key), ValueData::U64(value)) => (key.to_string(), *value),
                        _ => panic!("String/U64 map"),
                    }).collect::<std::collections::BTreeMap<_, _>>()
                });
                if n <= text.len() as u64 {
                    let mut expected = std::collections::BTreeMap::new();
                    if extent <= n {
                        for window in text.as_bytes()[..n as usize].windows(extent as usize) {
                            *expected.entry(String::from_utf8(window.to_vec()).unwrap()).or_insert(0u64) += 1;
                        }
                    }
                    assert_eq!(pairs.as_ref(), Some(&expected));
                    let report = result.as_ref().unwrap();
                    if !text.is_empty() { assert_eq!((report.packed_map_regions, report.packed_ssa_regions), (1, 1)); }
                    let updates = if extent <= n { n - extent + 1 } else { 0 };
                    assert_eq!(report.packed_map_updates, updates);
                } else {
                    assert!(result.is_err(), "actual source bound must still fail");
                    assert!(pairs.is_none());
                }
                let frontiers = PUBLICATION_FRONTIERS.with(|records| core::mem::take(&mut *records.borrow_mut()));
                if result.is_ok() && !text.is_empty() { assert_eq!(frontiers.len(), 1); }
                executions.push((result, pairs, terminal.get().unwrap(), frontiers));
                drop((outputs, inputs));
                assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
            }
            let (left, right) = (&executions[0], &executions[1]);
            assert_eq!(left.1, right.1, "canonical result");
            assert_eq!((left.2.0, left.2.1), (right.2.0, right.2.1), "source status/origin");
            assert_eq!(left.3, right.3, "{text:?}, extent={extent}, n={n}: exact pre-publication work and remaining fuel");
            match (&left.0, &right.0) {
                (Ok(a), Ok(b)) => assert_eq!(structural_report(a), structural_report(b)),
                _ => assert_eq!(left.0, right.0, "identical source failure"),
            }
            // HashMap iteration is independently seeded for the publication
            // bridge. A multi-key heapsort can take a different number of
            // charged sift steps; do not attribute those to generated code.
            // Empty/one-key publication and pre-publication failures supply
            // deterministic end-to-end work witnesses as well.
            if left.0.is_err() || left.1.as_ref().is_some_and(|map| map.len() <= 1) {
                assert_eq!(left.0, right.0, "deterministic full report");
                assert_eq!(left.2, right.2, "deterministic total native/library spend");
            }
        }
    }
}

#[test]
fn current_edge_facts_keep_unproved_arithmetic_failures_at_original_sites() {
    arithmetic_failure_equivalence(Pass::EdgeFacts);
}
#[test]
fn tag_threading_keeps_later_iteration_arithmetic_failures_at_original_sites() {
    arithmetic_failure_equivalence(Pass::TagThreading);
}
#[test]
fn frame_effects_keep_arithmetic_failures_and_full_error_spills() {
    arithmetic_failure_equivalence(Pass::FrameEffects);
}
#[test]
fn lookup_facts_keep_arithmetic_failures_at_original_source_sites() {
    arithmetic_failure_equivalence(Pass::LookupFacts);
}
#[test]
fn cfg_cleanup_keeps_arithmetic_failures_at_original_source_sites() {
    arithmetic_failure_equivalence(Pass::CfgCleanup);
}
#[test]
fn loop_outlining_keeps_arithmetic_failures_at_original_source_sites() {
    arithmetic_failure_equivalence(Pass::LoopOutlining);
}
#[test]
fn loop_invariants_keep_arithmetic_failures_and_declined_proofs_at_source_sites() {
    arithmetic_failure_equivalence(Pass::LoopInvariants);
}
fn arithmetic_failure_equivalence(pass: Pass) {
    // The valid first iteration must not establish facts for a later visit.
    // Source mutations intentionally remove or weaken individual premises.
    let source = structural_projection_tests::COUNT;
    let cases = [
        (source.replace("previous + 1u64", "previous + 18446744073709551615u64"), "\"aa\"", 1u64, 2u64),
        (source.replace("index >= n", "index == n")
            .replace(":Next(0u64, initial)", ":Next(3u64, initial)"), "\"abc\"", 1, 2),
        (source.replace("    | width > n - index -> :Done(counts)\n", "")
            .replace(":Next(0u64, initial)", ":Next(18446744073709551614u64, initial)"),
            "\"abc\"", 2, u64::MAX),
        // i<n only proves i+1, not this changed recurrence step i+2.
        (source.replace(":Next(index + 1u64, increment", ":Next(index + 2u64, increment")
            .replace(":Next(0u64, initial)", ":Next(18446744073709551614u64, initial)"),
            "\"abc\"", 1, u64::MAX),
    ];
    for (body, text, width, n) in cases {
        let artifact = tests::artifact_source(&format!("{body}\n#Count({text}, {width}u64, {n}u64, empty)\n"));
        let catalog = mech_stdlib::source_catalog();
        take_edge_fact_emissions();
        take_tag_thread_emissions();
        take_frame_effect_emissions();
        let optimized = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let emitted = take_edge_fact_emissions();
        assert!(emitted.0 >= 6, "must actually remove dominated field guards");
        let selected = take_tag_thread_emissions();
        pass.check(emitted, selected, take_frame_effect_emissions(), true);
        let unchanged = pass.disabled(|| JitProgramArtifact::compile(&artifact, &catalog)).unwrap();
        let unchanged_facts = take_edge_fact_emissions();
        let unchanged_tags = take_tag_thread_emissions();
        pass.check(unchanged_facts, unchanged_tags, take_frame_effect_emissions(), false);
        let mut executions = Vec::new();
        for jit in [&optimized, &unchanged] {
            MODE.with(|mode| mode.set(Mode::Native));
            OBS.with(|observed| *observed.borrow_mut() = Observation::default());
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let entry = jit.test_checked_entry();
            let mut outputs = vec![None];
            let terminal = Cell::new(None);
            let outcome = execute(jit.descriptor(), |frame| {
                // SAFETY: observer and error inspection belong to this same
                // pinned checked invocation and delegate the original calls.
                let old = unsafe { (*frame).helper };
                unsafe { (*frame).helper = observe; }
                let status = unsafe { entry(frame) };
                unsafe { (*frame).helper = old; }
                let ctx = unsafe { &*(*frame).base.context.cast::<Context<'_, '_>>() };
                terminal.set((status == STATUS_ARITHMETIC).then(|| (
                    unsafe { (*frame).base.error_node }, ctx.control.work.used(), ctx.library.used())));
                status
            }, &[], &mut outputs, tests::limits(), &memory);
            let error = outcome.unwrap_err();
            let (site, work, library_work) = terminal.get().expect("original checked source arithmetic must fail");
            let origin = jit.descriptor().plan.blocks()[site as usize].origin();
            assert!(matches!(&error, NativeProgramExecutionError::Control(
                NativeIterationExecutionError::Arithmetic { origin: actual }) if *actual == origin));
            let observed = OBS.with(|v| core::mem::take(&mut *v.borrow_mut()));
            assert!(observed.errors.is_empty(), "{observed:?}");
            assert_eq!((observed.entries, observed.packed, observed.bound[13]), (1, 1, 1));
            assert!(outputs.iter().all(Option::is_none));
            executions.push((site, work, library_work, format!("{error:?}")));
            drop(outputs);
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
        }
        assert_eq!(executions[0], executions[1], "unchanged first arithmetic failure and exact spending");
    }
}
