//! Actual packed-row loans into emitted Mech; no Rust scanner or new I/O route.
//! The genuine full snapshot remains for deferred-admission qualification only.
use super::prefix_tests::{assert_unloaned, limits, source, string_value};
use super::*;
use crate::native_artifact::JitProgramArtifact;
use mech_engine::native_artifact::NativeLibraryWorkBudget;

fn run_windowed(jit: &JitProgramArtifact, text: &str) {
    run_windowed_delivery(jit, text, true);
}

fn run_windowed_delivery(jit: &JitProgramArtifact, text: &str, incremental: bool) -> Vec<Box<[u8]>> {
    let descriptor = jit.descriptor();
    let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
    assert_eq!(descriptor.plan.transport_input_count(), 1);
    let value = string_value(
        descriptor,
        descriptor.plan.transport_input_type(0).schema(),
        text,
        &memory,
    );
    let (inputs, observations) = if descriptor.plan.inputs().len() == 1 {
        (vec![value], vec![])
    } else {
        (vec![], vec![value])
    };
    let mut expected = vec![None; descriptor.plan.transport_output_count()];
    let mut entry = jit.test_generated_entry();
    let mut ordinary = Prepared::new(descriptor, limits(), &memory).unwrap();
    ordinary.activate(&mut entry).unwrap();
    ordinary
        .finish(&mut entry, &inputs, &observations, &mut expected)
        .unwrap();
    let mut actual = vec![None; expected.len()];
    let mut prepared = Prepared::new(descriptor, limits(), &memory).unwrap();
    prepared.activate(&mut entry).unwrap();
    prepared
        .import_turn(&inputs, &observations, &actual)
        .unwrap();
    let library = prepared.state.as_ref().unwrap().library.clone();
    let site = descriptor
        .prefix
        .as_ref()
        .unwrap()
        .frontier
        .read()
        .count()
        .lines()
        .producer() as usize;
    let producer = descriptor.calls[site].as_ref().unwrap();
    let mut owner = producer
        .prepare_text_line_chunks(text.len() as u64, 4, limits().library, &library)
        .unwrap();
    let mut prior_available = 0;
    let mut sealed = None;
    for character in text.chars() {
        let mut bytes = [0; 4];
        owner.push(character.encode_utf8(&mut bytes)).unwrap();
        if !incremental {
            continue;
        }
        let available = owner.available_lines().unwrap();
        let first = if sealed.is_some() {
            available + 1
        } else {
            prior_available + 1
        };
        let window = owner.borrow_window(first, available + 1).unwrap();
        assert!(window.row_count() <= 1, "no full-input pointer table");
        super::super::borrowed::take_window_capture();
        prepared
            .run_prefix_window(&mut entry, available, false, Some(&window))
            .unwrap();
        assert_unloaned(&prepared);
        assert_eq!(
            super::super::borrowed::take_window_capture(),
            Some((
                window.native_rows().0,
                window.row_count() as u64,
                window.first(),
            ))
        );
        assert!(actual.iter().all(Option::is_none));
        drop(window);
        let retained = prepared.retention.unwrap();
        if let Some(end) = retained.end {
            if let Some(prior) = sealed {
                assert_eq!(prior, retained);
            } else {
                owner.retain_range(retained.first, end).unwrap();
                sealed = Some(retained);
            }
        } else {
            owner.retain_from(retained.first).unwrap();
        }
        prior_available = available;
    }
    let facts = owner.finish().unwrap();
    let first = if sealed.is_some() {
        facts.lines() + 1
    } else {
        prior_available + 1
    };
    let window = owner.borrow_window(first, facts.lines() + 1).unwrap();
    prepared
        .run_prefix_window(&mut entry, facts.lines(), true, Some(&window))
        .unwrap();
    let producer_work = window.producer_compute_work();
    drop(window);
    let report = prepared.publish(&mut actual).unwrap();
    assert!(
        report.work >= producer_work,
        "framing/window work belongs to the program account too"
    );
    let mut snapshots = Vec::new();
    for (actual, expected) in actual.iter().zip(&expected) {
        snapshots.push(
            actual
                .as_ref()
                .unwrap()
                .canonical_snapshot_bytes(&descriptor.schemas)
                .unwrap(),
        );
        assert_eq!(
            actual
                .as_ref()
                .unwrap()
                .canonical_snapshot_bytes(&descriptor.schemas)
                .unwrap(),
            expected
                .as_ref()
                .unwrap()
                .canonical_snapshot_bytes(&descriptor.schemas)
                .unwrap(),
        );
    }
    drop((owner, library, actual, expected, inputs, observations));
    assert_eq!(memory.used_bytes(), 0);
    snapshots
}

#[test]
fn disjoint_private_state_preserves_results_across_callbacks_and_window_resumes() {
    use crate::native_artifact::compiler::memory_regions::without_disjoint_state;
    for artifact in call_cache_tests::artifacts(source()) {
        let catalog = mech_stdlib::source_catalog();
        let optimized = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let ordinary =
            without_disjoint_state(|| JitProgramArtifact::compile(&artifact, &catalog)).unwrap();
        let descriptor = optimized.descriptor();
        assert!(!descriptor.closed_tails.regions.is_empty());
        for (text, expected) in [
            ("skip\nopen\nA\nB\nclose\nafter\n", "A|B"),
            (
                "skip\r\nopen\r\ne\u{301}\r\n猫\r\nclose\r\nafter\r",
                "e\u{301}|猫",
            ),
            ("", ""),
            ("missing\n", ""),
            ("open\nA", "A"),
        ] {
            let expected = ValueDraft {
                schema: descriptor.plan.transport_output_type(0).schema(),
                shape_values: Box::new([]),
                data: ValueDataDraft::String(expected.into()),
            }
            .finalize(&SnapshotValidationContext::with_shared_schemas(
                &descriptor.schemas,
            ))
            .unwrap()
            .canonical_snapshot_bytes(&descriptor.schemas)
            .unwrap();
            for incremental in [false, true] {
                assert_eq!(
                    run_windowed_delivery(&ordinary, text, incremental),
                    [expected.clone()]
                );
                assert_eq!(
                    run_windowed_delivery(&optimized, text, incremental),
                    [expected.clone()]
                );
            }
        }
    }
}

#[test]
fn disjoint_private_state_preserves_first_failure_and_finite_work_accounting() {
    use crate::native_artifact::compiler::memory_regions::without_disjoint_state;
    for artifact in call_cache_tests::artifacts(source()) {
        let catalog = mech_stdlib::source_catalog();
        let optimized = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let ordinary =
            without_disjoint_state(|| JitProgramArtifact::compile(&artifact, &catalog)).unwrap();
        let mut failed = 0;
        let mut succeeded = 0;
        for max_work in [0, 256, 4096, 65536, 1_000_000_000] {
            let mut outcomes = Vec::new();
            for jit in [&ordinary, &optimized] {
                let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
                let descriptor = jit.descriptor();
                let inputs = [string_value(
                    descriptor,
                    descriptor.plan.transport_input_type(0).schema(),
                    "skip\nopen\nA\nB\nclose\nafter\n",
                    &memory,
                )];
                let mut outputs = vec![None; descriptor.plan.transport_output_count()];
                let mut limit = limits();
                limit.execution.max_work = max_work;
                let outcome = jit.evaluate_into(&inputs, &mut outputs, limit, &memory);
                if outcome.is_err() {
                    assert!(outputs.iter().all(Option::is_none));
                    failed += 1;
                } else {
                    succeeded += 1;
                }
                // Both the exact error origin and required/allowed meter
                // values are observable. Successful reports must match too.
                outcomes.push(format!("{outcome:?}"));
                drop((inputs, outputs));
                assert_eq!(memory.used_bytes(), 0);
            }
            assert_eq!(outcomes[0], outcomes[1], "max_work={max_work}");
        }
        assert!(failed > 0 && succeeded > 0);
    }
}

#[test]
fn first_eof_window_runs_source_before_installing_final_count() {
    for artifact in call_cache_tests::artifacts(source()) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        for text in ["", "open\nA\nclose\n", "open\n猫\nclose\n"] {
            run_windowed_delivery(&jit, text, false);
        }
    }
}

#[test]
fn completed_utf8_cells_feed_emitted_windows_and_drop_every_loan_before_discard() {
    for artifact in call_cache_tests::artifacts(source()) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        for text in [
            "skip\nopen\nA\nB\nclose\nafter\n",
            "skip\r\nopen\r\ne\u{301}\r\n猫\r\nclose\r\nafter\r",
            "",
            "missing\n",
            "open\nA",
        ] {
            run_windowed(&jit, text);
        }
    }
}

#[test]
fn forward_jumps_discard_only_completed_input_without_rejecting_valid_carriers() {
    let jumping = source().replace("i + 1u64", "i + 2u64");
    for artifact in call_cache_tests::artifacts(&jumping) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert!(jit.descriptor().prefix.is_some());
        run_windowed(
            &jit,
            "skip\nignored\nopen\nignored\nA\nignored\nclose\nafter\n",
        );
    }
}

#[test]
#[cfg(feature = "canonical-native-program-host")]
fn actual_full_knucleotide_original_and_decoded_execute_from_bounded_row_windows() {
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
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        run_windowed(
            &jit,
            ">ONE\nAAA\n>TWO\nCCCC\n>THREE\nGGTATTTTAATTTATAGT\nGGTAC\n>FOUR\nT\n",
        );
    }
}

#[test]
fn foreign_producer_or_work_ledger_and_false_extent_fail_before_native_entry() {
    for artifact in call_cache_tests::artifacts(source()) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let other_jit =
            JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        for mode in 0..7 {
            let descriptor = jit.descriptor();
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let inputs = [string_value(
                descriptor,
                descriptor.plan.inputs()[0].ty().schema(),
                "open\nA\nclose\n",
                &memory,
            )];
            let outputs = vec![None; descriptor.plan.transport_output_count()];
            let mut entry = jit.test_generated_entry();
            let mut prepared = Prepared::new(descriptor, limits(), &memory).unwrap();
            prepared.activate(&mut entry).unwrap();
            prepared.import_turn(&inputs, &[], &outputs).unwrap();
            let library = if mode == 1 || mode == 5 {
                NativeLibraryWorkBudget::new(1 << 40, 1 << 32, &memory).unwrap()
            } else {
                prepared.state.as_ref().unwrap().library.clone()
            };
            let source_descriptor = if mode == 0 || mode == 4 {
                other_jit.descriptor()
            } else {
                descriptor
            };
            let site = source_descriptor
                .prefix
                .as_ref()
                .unwrap()
                .frontier
                .read()
                .count()
                .lines()
                .producer() as usize;
            let mut owner = source_descriptor.calls[site]
                .as_ref()
                .unwrap()
                .prepare_text_line_chunks(128, 128, limits().library, &library)
                .unwrap();
            owner.push("open\nA\nclose\n").unwrap();
            if mode >= 4 {
                owner.finish().unwrap();
            }
            let window = owner.borrow_window(1, 4).unwrap();
            let before = library.used();
            let mut calls = 0;
            let result = prepared.run_prefix_window(
                &mut |frame| {
                    calls += 1;
                    entry(frame)
                },
                if mode == 2 || mode == 6 { 2 } else { 3 },
                mode >= 3,
                Some(&window),
            );
            assert!(result.is_err(), "mode {mode}");
            assert_eq!(calls, 0);
            assert_eq!(
                library.used(),
                before,
                "invalid window must not debit or refill work"
            );
            assert!(outputs.iter().all(Option::is_none));
            drop(window);
            drop((prepared, owner, library, inputs, outputs));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn framing_work_reaches_the_global_work_limit_before_any_generated_prefix_instruction() {
    for artifact in call_cache_tests::artifacts(source()) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let descriptor = jit.descriptor();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let text = "open\nA\nclose\n".to_owned() + &"x".repeat(4096);
        let inputs = [string_value(
            descriptor,
            descriptor.plan.inputs()[0].ty().schema(),
            &text,
            &memory,
        )];
        let outputs = vec![None; descriptor.plan.transport_output_count()];
        let site = descriptor
            .prefix
            .as_ref()
            .unwrap()
            .frontier
            .read()
            .count()
            .lines()
            .producer() as usize;
        let producer = descriptor.calls[site].as_ref().unwrap();
        let mut entry = jit.test_generated_entry();
        let mut probe = Prepared::new(descriptor, limits(), &memory).unwrap();
        probe.activate(&mut entry).unwrap();
        probe.import_turn(&inputs, &[], &outputs).unwrap();
        let baseline = probe.state.as_ref().unwrap().work.used();
        let ledger = probe.state.as_ref().unwrap().library.clone();
        let mut owner = producer
            .prepare_text_line_chunks(text.len() as u64, text.len(), limits().library, &ledger)
            .unwrap();
        owner.push(&text).unwrap();
        let window = owner.borrow_window(1, 4).unwrap();
        producer.bind_text_line_window(&window, &ledger).unwrap();
        let required = baseline + window.producer_compute_work();
        assert_eq!(
            probe.state.as_ref().unwrap().work.used(),
            required,
            "the program meter must already include every storage debit exactly once"
        );
        drop(window);
        drop((owner, ledger, probe));
        let mut limited = limits();
        limited.execution.max_work = required - 1;
        let mut prepared = Prepared::new(descriptor, limited, &memory).unwrap();
        prepared.activate(&mut entry).unwrap();
        prepared.import_turn(&inputs, &[], &outputs).unwrap();
        assert_eq!(prepared.state.as_ref().unwrap().work.used(), baseline);
        let ledger = prepared.state.as_ref().unwrap().library.clone();
        let mut owner = producer
            .prepare_text_line_chunks(text.len() as u64, text.len(), limited.library, &ledger)
            .unwrap();
        owner.push(&text).unwrap();
        let window = owner.borrow_window(1, 4).unwrap();
        let mut calls = 0;
        let result = prepared.run_prefix_window(
            &mut |frame| {
                calls += 1;
                entry(frame)
            },
            3,
            false,
            Some(&window),
        );
        assert!(
            matches!(
                &result,
                Err(NativeProgramExecutionError::Library {
                    error: NativeCollectionInvocationError::Kernel(
                        mech_core::ResidentKernelError::ResourceLimit {
                            dimension: mech_core::MemoryBudgetDimension::ComputeWork,
                            ..
                        }
                    ),
                    ..
                })
            ),
            "{result:?}; expected global minimum {required}"
        );
        assert_eq!(calls, 0);
        assert!(outputs.iter().all(Option::is_none));
        drop(window);
        drop((prepared, owner, ledger, inputs, outputs));
        assert_eq!(memory.used_bytes(), 0);
    }
}
