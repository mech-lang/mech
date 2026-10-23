//! Compare the actual optimized entry with the same packed SSA entry without
//! substitutions: all source work, results, failures and origins must agree.
use super::*;
use crate::native_artifact::program_compiler::packed_loop_codegen::{
    take_edge_fact_emissions, without_edge_facts,
};
use crate::native_artifact::JitProgramArtifact;

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
                    let status = unsafe { entry(frame) };
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
                executions.push((result, pairs, terminal.get().unwrap()));
                drop((outputs, inputs));
                assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
            }
            assert_eq!(executions[0], executions[1], "{text:?}, extent={extent}, n={n}: unchanged source work and failures");
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
