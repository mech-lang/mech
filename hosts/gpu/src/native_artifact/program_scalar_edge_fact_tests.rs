//! Compare the actual optimized entry with the same packed SSA entry without
//! substitutions: all source work, results, failures and origins must agree.
use super::*;
use crate::native_artifact::program_compiler::packed_loop_codegen::{
    take_edge_fact_emissions, without_edge_facts,
};
use crate::native_artifact::JitProgramArtifact;

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
        let optimized = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let emitted = take_edge_fact_emissions();
        assert!(emitted.0 >= 8 && emitted.1 >= 4, "actual emitted guard/arithmetic substitutions: {emitted:?}");
        let unchanged = without_edge_facts(|| JitProgramArtifact::compile(&artifact, &catalog)).unwrap();
        assert_eq!(take_edge_fact_emissions(), (0, 0));
        for (text, extent, n) in [
            ("abcabc", 1u64, 6u64), ("abcabc", 2, 6), ("abcabc", 3, 5),
            ("zzzz", 2, 4), ("abcdefgh", 7, 8), ("a", 1, 1),
            ("abc", 2, 0), ("abc", 4, 3), ("", 1, 0),
            // Bounds must still use the actual selected source, not n.
            ("abc", 2, 4),
        ] {
            let mut executions = Vec::new();
            for jit in [&optimized, &unchanged] {
                PUBLICATION_FRONTIERS.with(|records| records.borrow_mut().clear());
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
                }, &inputs, &mut outputs, tests::limits(), &memory);
                let result = outcome.map_err(|error| format!("{error:?}"));
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
        let optimized = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        assert!(take_edge_fact_emissions().0 >= 6, "must actually remove dominated field guards");
        let unchanged = without_edge_facts(|| JitProgramArtifact::compile(&artifact, &catalog)).unwrap();
        assert_eq!(take_edge_fact_emissions(), (0, 0));
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
