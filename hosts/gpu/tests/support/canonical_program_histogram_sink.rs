//! Actual emitted generic histogram sinks, compared with the resident owner.
use super::{artifacts, equal, inputs, limits, reference, reference_result};
use mech_core::{ManagedMemoryBudget, Value, ValueDataDraft as Data};
use mech_engine::native_artifact::{NativeCollectionResult, NativeIterationAction};
use mech_gpu::native_artifact::{JitProgramArtifact, emit_program_object};

fn strings(values: &[&str]) -> Data {
    Data::Matrix(
        values
            .iter()
            .map(|text| Data::String((*text).into()))
            .collect(),
    )
}
fn selected(jit: &JitProgramArtifact) -> usize {
    jit.descriptor()
        .plan()
        .collections()
        .iter()
        .filter(|collection| matches!(collection.result(), NativeCollectionResult::Histogram(_)))
        .count()
}
fn prior(output: &[Option<Value>]) -> Vec<Value> {
    output
        .iter()
        .map(|value| value.as_ref().unwrap().clone())
        .collect()
}

#[test]
fn selected_histogram_sink_preserves_raw_keys_empty_unicode_nul_and_counts() {
    for artifact in artifacts("+> map\nmap/histogram([part | part <- source<[string]>])\n").0 {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(selected(&jit), 1, "actual full histogram strategy");
        assert!(
            jit.descriptor()
                .plan()
                .blocks()
                .iter()
                .any(|block| matches!(
                    block.action(),
                    NativeIterationAction::HistogramFinish { .. }
                ))
        );
        let object = emit_program_object(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(
            object.descriptor().typed_ir_sha256(),
            jit.descriptor().typed_ir_sha256()
        );
        let mut output = vec![None; artifact.outputs().len()];
        for parts in [
            vec![],
            vec![""],
            vec!["", "", ""],
            vec!["é", "é", "\0", "", "é", "\0x", "\0", "🧑‍🚀", "é"],
            vec!["ba", "b", "a\0", "a", "aa", "a", "b", "\r\n", "🇺🇳"],
        ] {
            let args = inputs(&artifact, &[("source", strings(&parts))], &memory);
            let expected = reference(&artifact, &args);
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &expected);
            let Data::Map(entries) = output
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap()
            else {
                panic!("genuine canonical Map");
            };
            let mut ordinary = std::collections::BTreeMap::<&str, u64>::new();
            for part in &parts {
                *ordinary.entry(part).or_default() += 1;
            }
            assert_eq!(entries.len(), ordinary.len());
            for (entry, (key, count)) in entries.iter().zip(ordinary) {
                assert_eq!(
                    entry.items.as_ref(),
                    &[Data::String(key.into()), Data::U64(count)]
                );
            }
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn selected_histogram_sink_keeps_nested_candidates_filters_and_late_error_order() {
    for artifact in artifacts(
        "+> map\nmap/histogram([part | row <- rows<[string]>, part <- row, part != \"x\"])\n",
    )
    .0
    {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(selected(&jit), 1);
        for rows in [vec![], vec!["x"], vec!["xx", "é\0é", "🧑‍🚀", "", "éé"]] {
            let args = inputs(&artifact, &[("rows", strings(&rows))], &memory);
            let mut output = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &reference(&artifact, &args));
        }
        assert_eq!(memory.used_bytes(), 0);
    }
    for source in [
        "+> map\nmap/histogram([part[1u64] | part <- source<[string]>])\n",
        "+> map\nmap/histogram([part[1u64] | part <- source<[string]>, part != \"skip\"])\n",
        "+> map\nmap/histogram([part[1u64] | part <- source<[string]>, part != \"\"])\n",
    ] {
        for artifact in artifacts(source).0 {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert_eq!(selected(&jit), 1);
            let mut output = vec![None; artifact.outputs().len()];
            for parts in [
                vec!["good", "ok"],
                vec!["good", ""],
                vec!["é", "skip", "🧑‍🚀"],
                vec!["", "good"],
            ] {
                let args = inputs(&artifact, &[("source", strings(&parts))], &memory);
                match reference_result(&artifact, &args) {
                    Ok(expected) => {
                        jit.evaluate_into(&args, &mut output, limits(), &memory)
                            .unwrap();
                        equal(&artifact, &output, &expected);
                    }
                    Err(_) => {
                        let old = prior(&output);
                        let before = memory.used_bytes();
                        assert!(
                            jit.evaluate_into(&args, &mut output, limits(), &memory)
                                .is_err()
                        );
                        equal(&artifact, &output, &old);
                        assert_eq!(memory.used_bytes(), before);
                    }
                }
            }
            drop(output);
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn selected_histogram_sink_denies_all_dimensions_atomically_and_recovers_same_jit() {
    let source = "+> map\nmap/histogram([part | part <- source<[string]>, part != \"skip\"])\n";
    for artifact in artifacts(source).0 {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(selected(&jit), 1);
        let a = "é\0é🧑‍🚀".repeat(64);
        let b = "\0zé".repeat(128);
        let args = inputs(
            &artifact,
            &[("source", strings(&[&a, "skip", &b, &a, "", &b]))],
            &memory,
        );
        let expected = reference(&artifact, &args);
        let mut output = vec![None; artifact.outputs().len()];
        let accepted = jit
            .evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        equal(&artifact, &output, &expected);
        for dimension in 0..7 {
            let mut denied = limits();
            match dimension {
                0 => denied.library.max_output_elements = Some(0),
                1 => denied.library.max_output_bytes = Some(0),
                2 => denied.library.max_temporary_bytes = Some(0),
                3 => denied.library.max_cloned_bytes = Some(0),
                4 => denied.library.max_retained_nodes = Some(0),
                5 => denied.library.max_compute_work = Some(0),
                6 => denied.library.max_comparison_work = Some(0),
                _ => unreachable!(),
            }
            let old = prior(&output);
            let before = memory.used_bytes();
            assert!(
                jit.evaluate_into(&args, &mut output, denied, &memory)
                    .is_err(),
                "dimension {dimension}"
            );
            equal(&artifact, &output, &old);
            assert_eq!(memory.used_bytes(), before, "dimension {dimension}");
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &expected);
        }
        let mut denied = limits();
        denied.execution.max_work = accepted.work - 1;
        let old = prior(&output);
        let before = memory.used_bytes();
        assert!(
            jit.evaluate_into(&args, &mut output, denied, &memory)
                .is_err()
        );
        equal(&artifact, &output, &old);
        assert_eq!(memory.used_bytes(), before);
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        equal(&artifact, &output, &expected);
        drop(old);
        drop(output);
        drop(args);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn selected_histogram_helpers_use_actual_factory_raw_order_footprint_and_checked_count() {
    let catalog = mech_stdlib::source_catalog();
    for artifact in artifacts("+> map\nmap/histogram([part | part <- source<[string]>])\n").0 {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let sink = jit
            .descriptor()
            .plan()
            .collections()
            .iter()
            .find_map(|collection| match collection.result() {
                NativeCollectionResult::Histogram(sink) => Some(sink),
                _ => None,
            })
            .expect("actual maintained selected sink");
        let call = mech_engine::native_artifact::prepare_native_program_library_call(
            &artifact,
            sink.template(),
            &catalog,
        )
        .unwrap();
        for (left, right) in [
            ("", "\0"),
            ("\0", "\0x"),
            ("é", "é"),
            ("🧑‍🚀", "🇺🇳"),
            ("é", "é"),
        ] {
            assert_eq!(
                call.selected_histogram_compare(left, right).unwrap(),
                left.as_bytes().cmp(right.as_bytes())
            );
        }
        assert_eq!(call.selected_histogram_increment(0).unwrap(), 1);
        assert_eq!(
            call.selected_histogram_increment(u64::MAX - 1).unwrap(),
            u64::MAX
        );
        assert!(call.selected_histogram_increment(u64::MAX).is_err());
        for count in [0, 1, 5] {
            let footprint = call.selected_histogram_footprint(count, 19).unwrap();
            assert_eq!(footprint.node_count, count * 2 + 2);
            assert_eq!(footprint.encoded_bytes, count * 16 + 19 + 8);
            assert_eq!(
                call.selected_histogram_finalization_work(count, 19)
                    .unwrap(),
                3 * 19 + 8 * count + 8
            );
            assert_eq!(
                call.selected_histogram_draft_bytes(count).unwrap(),
                count
                    * (core::mem::size_of::<mech_core::snapshot::MapEntryDraft>()
                        + 2 * core::mem::size_of::<Data>()
                        + core::mem::size_of::<String>()) as u64
            );
        }
        assert!(
            call.selected_histogram_footprint(u64::MAX, u64::MAX)
                .is_err()
        );
        assert!(call.selected_histogram_draft_bytes(u64::MAX).is_err());
        assert!(call.selected_histogram_original_scratch_bytes(0).unwrap() > 0);
        assert!(
            call.selected_histogram_original_scratch_bytes(u64::MAX)
                .is_err()
        );
        assert!(
            call.selected_histogram_finalization_work(u64::MAX, u64::MAX)
                .is_err()
        );
    }
    for artifact in artifacts("+> text\ntext/join([part | part <- source<string>], \"\")\n").0 {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let sink = jit
            .descriptor()
            .plan()
            .collections()
            .iter()
            .find_map(|collection| match collection.result() {
                NativeCollectionResult::TextJoin(sink) => Some(sink),
                _ => None,
            })
            .unwrap();
        let call = mech_engine::native_artifact::prepare_native_program_library_call(
            &artifact,
            sink.template(),
            &catalog,
        )
        .unwrap();
        assert!(call.selected_histogram_compare("a", "b").is_err());
        assert!(call.selected_histogram_increment(0).is_err());
        assert!(call.selected_histogram_footprint(0, 0).is_err());
        assert!(call.selected_histogram_draft_bytes(0).is_err());
        assert!(call.selected_histogram_original_scratch_bytes(0).is_err());
        assert!(call.selected_histogram_finalization_work(0, 0).is_err());
    }
}
