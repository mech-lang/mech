//! Actual emitted single-yield append sinks, not a workload callback.
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
        .filter(|collection| matches!(collection.result(), NativeCollectionResult::TextJoin(_)))
        .count()
}
fn prior(output: &[Option<Value>]) -> Vec<Value> {
    output
        .iter()
        .map(|value| value.as_ref().unwrap().clone())
        .collect()
}

#[test]
fn selected_join_sink_keeps_empty_yields_unicode_separators_and_source_order() {
    let source = "+> text\ntext/join([part | part <- source<[string]>], separator<string>)\n";
    for artifact in artifacts(source).0 {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(
            selected(&jit),
            1,
            "test must execute the optimized strategy"
        );
        let object = emit_program_object(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(
            object.descriptor().typed_ir_sha256(),
            jit.descriptor().typed_ir_sha256()
        );
        assert!(
            jit.descriptor()
                .plan()
                .blocks()
                .iter()
                .any(|b| matches!(b.action(), NativeIterationAction::TextJoinFinish { .. }))
        );
        let mut output = vec![None; artifact.outputs().len()];
        for (parts, separator, oracle) in [
            (vec![], "|", ""),
            (vec![""], "|", ""),
            (vec!["", "", ""], "|", "||"),
            (vec!["", "é", "", "🧑‍🚀", ""], "\0é", "\0éé\0é\0é🧑‍🚀\0é"),
            (vec!["a", "", "b"], "", "ab"),
            (vec!["é", "é", "\r\n", "🇺🇳"], "→", "é→é→\r\n→🇺🇳"),
        ] {
            let args = inputs(
                &artifact,
                &[
                    ("source", strings(&parts)),
                    ("separator", Data::String(separator.into())),
                ],
                &memory,
            );
            let expected = reference(&artifact, &args);
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &expected);
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::String(oracle.into())
            );
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn selected_join_sink_composes_nested_generators_filters_and_captured_separator() {
    for source in [
        "+> text\ntext/join([part | row <- rows<[string]>, part <- row, part != \"x\"], separator<string>)\n",
        "+> text\ntext/join([text/join([part | part <- row, part != \"x\"], separator<string>) | row <- rows<[string]>], \"/\")\n",
    ] {
        for artifact in artifacts(source).0 {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert!(selected(&jit) >= 1);
            for rows in [
                vec![],
                vec!["x"],
                vec!["éxé", "🧑‍🚀\0", "\r\n🇺🇳", ""],
                vec!["xx", "a"],
            ] {
                let args = inputs(
                    &artifact,
                    &[
                        ("rows", strings(&rows)),
                        ("separator", Data::String("|".into())),
                    ],
                    &memory,
                );
                let mut output = vec![None; artifact.outputs().len()];
                jit.evaluate_into(&args, &mut output, limits(), &memory)
                    .unwrap();
                equal(&artifact, &output, &reference(&artifact, &args));
            }
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn selected_join_sink_preserves_late_body_errors_lazy_filters_and_recovery() {
    for source in [
        "+> text\ntext/join([part[1u64] | part <- source<[string]>], \"|\")\n",
        "+> text\ntext/join([part[1u64] | part <- source<[string]>, part != \"skip\"], \"|\")\n",
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
                vec!["é", "skip", "🧑‍🚀"],
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
                        assert_eq!(memory.used_bytes(), before);
                        equal(&artifact, &output, &old);
                    }
                }
            }
            drop(output);
            assert_eq!(memory.used_bytes(), 0);
        }
    }
    // The false filter must still prevent the later invalid selector.
    super::check(
        "+> text\ntext/join([part[1u64] | part <- source<[string]>, part != \"\"], \"|\")\n",
        &[("source", strings(&["", "é", "", "🧑‍🚀"]))],
    );
}

#[test]
fn selected_join_sink_denials_are_atomic_all_dimensions_share_the_same_invocation_and_recover() {
    let source =
        "+> text\ntext/join([part | part <- source<[string]>, part != \"\"], separator<string>)\n";
    for artifact in artifacts(source).0 {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(selected(&jit), 1);
        let text = "é\0é🧑‍🚀".repeat(128);
        let args = inputs(
            &artifact,
            &[
                ("source", strings(&[&text, "", &text, &text])),
                ("separator", Data::String("\r\n".into())),
            ],
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
            assert_eq!(memory.used_bytes(), before, "dimension {dimension}");
            equal(&artifact, &output, &old);
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
        assert_eq!(memory.used_bytes(), before);
        equal(&artifact, &output, &old);
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        equal(&artifact, &output, &expected);
        drop(old);
        // Exact prospective output bound rejects before a later append grows
        // the exposed String. Earlier private pieces never become published.
        let mut denied = limits();
        denied.library.max_output_bytes = Some(
            mech_core::snapshot::prospective_string_value_footprint(0, false, 1, text.len() as u64)
                .unwrap()
                .retained_bytes,
        );
        let old = prior(&output);
        let before = memory.used_bytes();
        assert!(
            jit.evaluate_into(&args, &mut output, denied, &memory)
                .is_err()
        );
        assert_eq!(memory.used_bytes(), before);
        equal(&artifact, &output, &old);
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
fn selected_join_sink_declines_matrix_escape_and_later_fallible_separator() {
    for source in [
        "+> text\nparts := [part | part <- source<string>]\ntext/join(parts, \"\")\nparts\n",
        "+> text\nparts := [part | part <- source<string>]\nseparator := source[1u64]\ntext/join(parts, separator)\n",
        "+> text\nparts := [part | part <- source<string>]\ntext/join(parts, \"\") + text/join(parts, \"-\")\n",
    ] {
        for artifact in artifacts(source).0 {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert_eq!(selected(&jit), 0, "{source}");
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let args = inputs(
                &artifact,
                &[("source", Data::String("é🧑‍🚀".into()))],
                &memory,
            );
            let mut output = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &reference(&artifact, &args));
        }
    }
}

#[test]
fn selected_join_sink_actual_factory_witness_cannot_be_replaced_by_the_same_name_or_pointer() {
    let maintained = mech_stdlib::source_catalog();
    for artifact in artifacts("+> text\ntext/join([part | part <- source<string>], \"\")\n").0 {
        let jit = JitProgramArtifact::compile(&artifact, &maintained).unwrap();
        let sink = jit
            .descriptor()
            .plan()
            .collections()
            .iter()
            .find_map(|c| match c.result() {
                NativeCollectionResult::TextJoin(s) => Some(s),
                _ => None,
            })
            .unwrap();
        let original = maintained
            .resident_factory(&["text".into()], "join")
            .unwrap();
        for altered in [
            original.clone().with_factory(original.factory()),
            original.clone().with_key(original.key().clone()),
            original
                .clone()
                .with_implementation_memory(original.implementation_memory()),
        ] {
            let mut builder = mech_core::FunctionCatalogBuilder::new();
            builder.insert_resident_factory_entry(altered).unwrap();
            let catalog = builder.build().unwrap();
            assert!(matches!(mech_engine::native_artifact::prepare_native_program_library_call(&artifact,sink.template(),&catalog),Err(mech_engine::native_artifact::NativeArtifactLoweringError::MissingSemanticWitness{..})));
        }
    }
}

#[test]
fn selected_join_sink_global_memory_denial_retains_old_publication_and_releases_private_state() {
    for artifact in
        artifacts("+> text\ntext/join([part | part <- source<[string]>], separator<string>)\n").0
    {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(selected(&jit), 1);
        let args = inputs(
            &artifact,
            &[
                ("source", strings(&["é", "", "🧑‍🚀", "\0"])),
                ("separator", Data::String("|".into())),
            ],
            &memory,
        );
        let mut output = vec![None; artifact.outputs().len()];
        let expected = reference(&artifact, &args);
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        equal(&artifact, &output, &expected);
        let before = memory.used_bytes();
        let old = prior(&output);
        let ballast = memory
            .reserve_capacity(memory.limit_bytes() - before - 1)
            .unwrap();
        let with_ballast = memory.used_bytes();
        let error = jit
            .evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap_err();
        assert!(
            matches!(
                error,
                mech_gpu::native_artifact::NativeProgramExecutionError::Base(
                    mech_gpu::native_artifact::NativeExecutionError::Memory { .. }
                )
            ),
            "{error:?}"
        );
        assert_eq!(memory.used_bytes(), with_ballast);
        equal(&artifact, &output, &old);
        drop(ballast);
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
fn selected_join_sink_does_not_skip_a_late_missing_map_key() {
    let source = "+> text\n+> map\nmapping := {\"a\": \"A\", \"é\": \"É\"}\ntext/join([mapping{part} | part <- source<[string]>], \"\")\n";
    for artifact in artifacts(source).0 {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(selected(&jit), 1);
        let mut output = vec![None; artifact.outputs().len()];
        for parts in [vec!["a", "é"], vec!["a", "absent"], vec!["é", "a"]] {
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
                    let error = jit
                        .evaluate_into(&args, &mut output, limits(), &memory)
                        .unwrap_err();
                    let expected_origin=jit.descriptor().plan().blocks().iter().find_map(|block|match block.action(){NativeIterationAction::LibraryCall{template,..}if template.primitive()==mech_engine::native_artifact::NativeCollectionPrimitive::MapKeyAccess=>Some(template.origin()),_=>None}).unwrap();
                    assert!(
                        matches!(error,mech_gpu::native_artifact::NativeProgramExecutionError::Library{origin,..}if origin==expected_origin),
                        "{error:?}"
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

#[test]
fn selected_join_append_rejects_unreserved_capacity_without_repair_or_mutation() {
    let catalog = mech_stdlib::source_catalog();
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
            .expect("actual maintained selected sink");
        let selected = mech_engine::native_artifact::prepare_native_program_library_call(
            &artifact,
            sink.template(),
            &catalog,
        )
        .unwrap();
        for has_previous in [false, true] {
            let mut target = String::with_capacity(8);
            target.push_str("a");
            let capacity = target.capacity();
            let pointer = target.as_ptr();
            let prior = target.clone();
            let part = "x".repeat(capacity + 1);
            let error = selected
                .selected_text_join_append(&mut target, "|é", has_previous, &part)
                .unwrap_err();
            assert!(
                matches!(
                    error,
                    mech_engine::native_artifact::NativeCollectionInvocationError::Kernel(
                        mech_core::ResidentKernelError::InvalidShape
                    )
                ),
                "{error:?}"
            );
            assert_eq!(target, prior);
            assert_eq!(target.capacity(), capacity);
            assert_eq!(target.as_ptr(), pointer);
        }
        let mut target = String::new();
        selected
            .selected_text_join_append(&mut target, "", false, "")
            .unwrap();
        selected
            .selected_text_join_append(&mut target, "", true, "")
            .unwrap();
        assert!(target.is_empty());
        assert_eq!(target.capacity(), 0);
        let mut target = String::with_capacity(64);
        selected
            .selected_text_join_append(&mut target, "|", false, "")
            .unwrap();
        selected
            .selected_text_join_append(&mut target, "|", true, "")
            .unwrap();
        selected
            .selected_text_join_append(&mut target, "|", true, "é🧑‍🚀\0")
            .unwrap();
        assert_eq!(target, "||é🧑‍🚀\0");
    }
}
