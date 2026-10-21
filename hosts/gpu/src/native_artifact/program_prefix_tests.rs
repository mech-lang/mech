//! Real emitted scanner continuations over ONE genuine, full captured input.
//! These tests do not claim incremental acquisition or reduced retained input.
use super::*;
use call_cache_tests::artifacts;

fn source() -> &'static str {
    "+> text\n+> sequence\n#Locate(rows<[string]>, count<u64>) => <(u64,u64)>\n  | :Skip(i<u64>)\n  | :Take(first<u64>, i<u64>)\n  | :Ready(first<u64>, last<u64>).\n#Locate(rows, count) -> :Skip(1u64)\n  :Skip(i)\n    | i > count -> :Ready(1u64, 1u64)\n    | rows[i] == \"open\" -> :Take(i + 1u64, i + 1u64)\n    | * -> :Skip(i + 1u64)\n  :Take(first, i)\n    | i > count -> :Ready(first, count + 1u64)\n    | rows[i] == \"close\" -> :Ready(first, i)\n    | * -> :Take(first, i + 1u64)\n  :Ready(first, last) => (first, last).\nrows := text/lines(input<string>)\nbounds := #Locate(rows, sequence/length(rows))\nresult := bounds?\n  | (first, last), first < last => text/join(rows[first..last], \"|\")\n  | * => \"\".\nresult\n"
}

fn limits() -> NativeProgramExecutionLimits {
    NativeProgramExecutionLimits {
        execution: NativeExecutionLimits {
            max_work: 1_000_000_000,
            ..Default::default()
        },
        library: call_cache_tests::policy(),
    }
}

fn input(descriptor: &NativeProgramDescriptor, text: &str, memory: &ManagedMemoryBudget) -> Value {
    assert_eq!(descriptor.plan.inputs().len(), 1);
    let value = ValueDraft {
        schema: descriptor.plan.inputs()[0].ty().schema(),
        shape_values: Box::new([]),
        data: ValueDataDraft::String(text.into()),
    }
    .finalize(&SnapshotValidationContext::with_shared_schemas(
        &descriptor.schemas,
    ))
    .unwrap();
    let mut reservation = memory
        .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
        .unwrap();
    value.into_memory_budget(&mut reservation).unwrap()
}

fn carrier(prepared: &Prepared<'_>) -> Vec<u64> {
    let region = prepared.descriptor.prefix.as_ref().unwrap();
    let (_, width) = prepared
        .descriptor
        .scalar_aggregates
        .slot(region.tail.carrier())
        .unwrap();
    let first = prepared.descriptor.plan.transport_input_count() + region.carrier;
    prepared.storage.values()[first..first + width].to_vec()
}

fn assert_unloaned(prepared: &Prepared<'_>) {
    let region = prepared.descriptor.prefix.as_ref().unwrap();
    let layout = &prepared.descriptor.borrowed_strings.regions[region.borrowed];
    let first = prepared.descriptor.plan.transport_input_count();
    assert!(
        prepared.storage.values()[first + layout.ready..first + layout.end]
            .iter()
            .all(|word| *word == 0)
    );
    if let Some(closed) = prepared.descriptor.closed_tails.sites[region.tail.header() as usize] {
        let words = prepared.descriptor.closed_tails.regions[closed].words;
        assert_eq!(
            &prepared.storage.values()[first + words..first + words + 3],
            &[0; 3]
        );
    }
    let arguments = first
        + prepared.locals
        + prepared.progress
        + prepared.descriptor.plan.transport_output_count()
        + 1;
    assert!(
        prepared.storage.values()[arguments..arguments + prepared.descriptor.max_library_inputs]
            .iter()
            .all(|word| *word == 0)
    );
}

fn output_text(outputs: &[Option<Value>]) -> &str {
    assert_eq!(outputs.len(), 1);
    let ValueData::String(text) = outputs[0].as_ref().unwrap().data() else {
        panic!("String result");
    };
    text
}

#[test]
fn emitted_scanner_preserves_carrier_and_one_ledger_across_header_pauses() {
    for artifact in artifacts(source()) {
        let jit = super::super::super::JitProgramArtifact::compile(
            &artifact,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        let descriptor = jit.descriptor();
        assert!(descriptor.prefix.is_some());
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let inputs = [input(
            descriptor,
            "skip\nopen\nA\nB\nclose\nafter\n",
            &memory,
        )];
        let mut outputs = vec![None; descriptor.plan.transport_output_count()];
        let mut entry = jit.test_generated_entry();
        let mut prepared = Prepared::new(descriptor, limits(), &memory).unwrap();
        prepared.activate(&mut entry).unwrap();
        prepared.import_turn(&inputs, &[], &outputs).unwrap();
        let mut work = prepared.state.as_ref().unwrap().work.used();
        let mut comparison = 0;
        let mut states = Vec::new();
        for available in 0..=6 {
            prepared.run_prefix(&mut entry, available, false).unwrap();
            assert!(prepared.paused.is_some());
            assert_unloaned(&prepared);
            assert!(outputs.iter().all(Option::is_none));
            let state = prepared.state.as_ref().unwrap();
            assert!(state.work.used() > work);
            assert!(state.library.used().1 >= comparison);
            assert!(!state.completed);
            work = state.work.used();
            comparison = state.library.used().1;
            states.push(carrier(&prepared));
        }
        assert_ne!(
            states[0], states[1],
            "the body must progress, not replay its seed"
        );
        assert_eq!(
            states[5], states[6],
            "a returned range awaits EOF without replay"
        );
        prepared.run_prefix(&mut entry, 6, true).unwrap();
        let report = prepared.publish(&mut outputs).unwrap();
        assert!(report.work > work);
        assert!(
            matches!(outputs[0].as_ref().unwrap().data(), ValueData::String(text) if text.as_ref() == "A|B")
        );
        let mut ordinary = vec![None; outputs.len()];
        jit.evaluate_into(&inputs, &mut ordinary, limits(), &memory)
            .unwrap();
        assert_eq!(output_text(&outputs), output_text(&ordinary));
        drop((outputs, ordinary, inputs));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn empty_missing_marker_and_unicode_fallback_resume_to_exact_native_result() {
    for artifact in artifacts(source()) {
        let jit = super::super::super::JitProgramArtifact::compile(
            &artifact,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        let descriptor = jit.descriptor();
        for (text, count) in [
            ("", 0),
            ("unmatched\n", 1),
            ("open\r\ne\u{301}\r\n猫\r\nclose\r\n", 4),
            ("open\nA", 2),
        ] {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let inputs = [input(descriptor, text, &memory)];
            let mut outputs = vec![None; descriptor.plan.transport_output_count()];
            let mut entry = jit.test_generated_entry();
            let mut prepared = Prepared::new(descriptor, limits(), &memory).unwrap();
            prepared.activate(&mut entry).unwrap();
            prepared.import_turn(&inputs, &[], &outputs).unwrap();
            for available in 0..=count {
                prepared.run_prefix(&mut entry, available, false).unwrap();
                assert_unloaned(&prepared);
            }
            prepared.run_prefix(&mut entry, count, true).unwrap();
            prepared.publish(&mut outputs).unwrap();
            let mut ordinary = vec![None; outputs.len()];
            jit.evaluate_into(&inputs, &mut ordinary, limits(), &memory)
                .unwrap();
            assert_eq!(output_text(&outputs), output_text(&ordinary), "{text:?}");
            drop((outputs, ordinary, inputs));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn continuation_rejects_regression_false_extent_and_forged_pause_atomically() {
    for artifact in artifacts(source()) {
        let jit = super::super::super::JitProgramArtifact::compile(
            &artifact,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        let descriptor = jit.descriptor();
        for kind in 0..4 {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let inputs = [input(descriptor, "open\nA\nclose\n", &memory)];
            let outputs = vec![None; descriptor.plan.transport_output_count()];
            let mut entry = jit.test_generated_entry();
            let mut prepared = Prepared::new(descriptor, limits(), &memory).unwrap();
            prepared.activate(&mut entry).unwrap();
            prepared.import_turn(&inputs, &[], &outputs).unwrap();
            match kind {
                0 => {
                    prepared.run_prefix(&mut entry, 1, false).unwrap();
                    assert!(prepared.run_prefix(&mut entry, 0, false).is_err());
                }
                1 => assert!(prepared.run_prefix(&mut entry, 4, false).is_err()),
                2 => assert!(prepared.run_prefix(&mut entry, 2, true).is_err()),
                _ => assert!(
                    prepared
                        .run_prefix(&mut |_| STATUS_INPUT_PENDING, 0, false)
                        .is_err()
                ),
            }
            assert!(outputs.iter().all(Option::is_none));
            drop((prepared, inputs, outputs));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn returned_checkpoint_rejects_uncleared_lanes_and_mutated_immutable_facts() {
    for artifact in artifacts(source()) {
        let jit = super::super::super::JitProgramArtifact::compile(
            &artifact,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        let descriptor = jit.descriptor();
        for field in 0..7 {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let inputs = [input(descriptor, "open\nA\nclose\n", &memory)];
            let outputs = vec![None; descriptor.plan.transport_output_count()];
            let mut native = jit.test_generated_entry();
            let mut entry = |frame| {
                let status = native(frame);
                if status == STATUS_INPUT_PENDING {
                    super::super::test_corrupt_prefix_checkpoint(frame, field);
                }
                status
            };
            let mut prepared = Prepared::new(descriptor, limits(), &memory).unwrap();
            prepared.activate(&mut entry).unwrap();
            prepared.import_turn(&inputs, &[], &outputs).unwrap();
            assert!(
                prepared.run_prefix(&mut entry, 0, false).is_err(),
                "field {field}"
            );
            assert!(outputs.iter().all(Option::is_none));
            drop((prepared, inputs, outputs));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
#[cfg(feature = "canonical-native-program-host")]
fn real_full_source_selects_generic_header_continuation_without_enabling_capture() {
    let mut compiler = mech_runtime::RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .host_factory(Box::new(mech_terminal::CliHostFactory::new().unwrap()))
        .unwrap()
        .host_instance(mech_runtime::HostInstanceConfig {
            name: "cli".into(),
            provider: "cli".into(),
            settings: mech_runtime::ConfigValue::Map(Default::default()),
        })
        .build_compiler()
        .unwrap();
    let product = compiler
        .compile_canonical_source(include_str!(
            "../../tests/fixtures/native/knucleotide-idiomatic.mec"
        ))
        .unwrap();
    let original = product.artifact().clone();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(product.bytecode()).unwrap();
    for artifact in [original, decoded] {
        let jit = super::super::super::JitProgramArtifact::compile(
            &artifact,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        let region = jit
            .descriptor()
            .prefix
            .as_ref()
            .expect("ordinary scanner proof");
        assert_eq!(region.frontier.states().len(), 3);
        assert_eq!(region.frontier.read().count().guards().len(), 2);
    }
}

#[test]
fn comparison_allowance_is_not_renewed_by_a_prefix_resume() {
    for artifact in artifacts(source()) {
        let jit = super::super::super::JitProgramArtifact::compile(
            &artifact,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        let descriptor = jit.descriptor();
        let region = descriptor.prefix.as_ref().unwrap();
        let closed = descriptor.closed_tails.sites[region.tail.header() as usize].unwrap();
        let allowance = descriptor.closed_tails.regions[closed].comparison;
        assert!(allowance > 0);
        let mut limits = limits();
        limits.library.max_comparison_work = Some(allowance);
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let inputs = [input(descriptor, "open\nA\nclose\n", &memory)];
        let outputs = vec![None; descriptor.plan.transport_output_count()];
        let mut entry = jit.test_generated_entry();
        let mut prepared = Prepared::new(descriptor, limits, &memory).unwrap();
        prepared.activate(&mut entry).unwrap();
        prepared.import_turn(&inputs, &[], &outputs).unwrap();
        prepared.run_prefix(&mut entry, 1, false).unwrap();
        assert_eq!(prepared.state.as_ref().unwrap().library.used().1, allowance);
        assert!(prepared.run_prefix(&mut entry, 2, false).is_err());
        assert!(outputs.iter().all(Option::is_none));
        drop((prepared, inputs, outputs));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn unproved_backward_scan_and_completion_bypass_have_no_continuation() {
    let backward = source().replace(":Skip(i + 1u64)", ":Skip(1u64)");
    let bypass = source().replace("bounds := #Locate(rows, sequence/length(rows))", "bounds := flag<bool>?\n  | true => #Locate(rows, sequence/length(rows))\n  | false => (1u64, 1u64).\n");
    for source in [backward, bypass] {
        for artifact in artifacts(&source) {
            let jit = super::super::super::JitProgramArtifact::compile(
                &artifact,
                &mech_stdlib::source_catalog(),
            )
            .unwrap();
            assert!(jit.descriptor().prefix.is_none(), "{source}");
        }
    }
}
