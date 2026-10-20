//! Actual original/decoded emitted projection with the ordinary resident oracle.
//! Object emission proves identity only; it is not a linked-execution claim.
use super::{artifacts, equal, inputs, limits, reference, reference_result};
use mech_core::{
    ManagedMemoryBudget, ResidentKernelError, ValueDataDraft as Data, snapshot::MapEntryDraft,
};
use mech_engine::native_artifact::{NativeCollectionInvocationError, NativeIterationAction};
use mech_gpu::native_artifact::{
    JitProgramArtifact, NativeProgramExecutionError, emit_program_object,
};

const SOURCE: &str = "+> text\n+> map\ntext/join([(entries<{string:string}>){piece} | piece <- source<string>], separator<string>)\n";

#[test]
fn compiled_byte_map_has_constant_visits_and_keeps_missing_key_atomic() {
    for artifact in artifacts(SOURCE).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(jit.descriptor().plan().string_map_regions().len(), 1);
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        let mut short_visits = None;
        for (text, entries, separator) in [
            ("ababa", vec![("a", "A"), ("b", "B")], ""),
            (
                "ababababababababababababa",
                vec![("a", "A"), ("b", "B")],
                "",
            ),
            ("aa?", vec![("a", "A")], ""),
            ("", vec![], ""),
            ("a\0a", vec![("a", "\0"), ("\0", "a")], ""),
            ("éé", vec![("é", "A")], ""),
            ("\r\nA", vec![("\r\n", "L"), ("A", "a")], ""),
            ("aa", vec![("a", "é")], ""),
            ("aa", vec![("a", "")], ""),
            ("aa", vec![("a", "A")], "|"),
            ("\u{7f}\u{7f}", vec![("\u{7f}", "\r")], ""),
        ] {
            let args = inputs(
                &artifact,
                &[
                    ("source", Data::String(text.into())),
                    ("entries", dictionary(&entries)),
                    ("separator", Data::String(separator.into())),
                ],
                &memory,
            );
            let expected = reference_result(&artifact, &args);
            let previous = output.clone();
            let before = memory.used_bytes();
            let actual = jit.evaluate_into(&args, &mut output, limits(), &memory);
            match expected {
                Ok(expected) => {
                    let report = actual.unwrap();
                    equal(&artifact, &output, &expected);
                    if text == "ababa" {
                        short_visits = Some(report.visits);
                    }
                    if text == "ababababababababababababa" {
                        assert_eq!(Some(report.visits), short_visits);
                    }
                }
                Err(expected) => {
                    assert!(expected.contains("InvalidInput"), "{expected}");
                    let error = actual.unwrap_err();
                    let projection = jit
                        .descriptor()
                        .plan()
                        .collections()
                        .iter()
                        .find_map(|c| c.string_element_map_projection())
                        .unwrap();
                    assert!(
                        matches!(error, NativeProgramExecutionError::Library { origin,
                        error:NativeCollectionInvocationError::Kernel(ResidentKernelError::InvalidInput) }
                        if origin==projection.selection_template().origin()),
                        "{error:?}"
                    );
                    for (current, old) in output.iter().zip(&previous) {
                        assert!(
                            current
                                .as_ref()
                                .unwrap()
                                .shares_frozen_storage(old.as_ref().unwrap())
                        );
                    }
                    assert_eq!(
                        memory.used_bytes(),
                        before,
                        "failed native map must release scratch/loans"
                    );
                }
            }
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn compiled_byte_map_finite_work_denial_preserves_outputs_and_recovers() {
    for artifact in artifacts(SOURCE).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let args = inputs(
            &artifact,
            &[
                ("source", Data::String("ab".repeat(1000))),
                ("entries", dictionary(&[("a", "A"), ("b", "B")])),
                ("separator", Data::String("".into())),
            ],
            &memory,
        );
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        let previous = output.clone();
        let before = memory.used_bytes();
        let mut denied = limits();
        denied.library.max_compute_work = Some(200_000);
        let error = jit
            .evaluate_into(&args, &mut output, denied, &memory)
            .unwrap_err();
        assert!(format!("{error:?}").contains("ComputeWork"), "{error:?}");
        for (current, old) in output.iter().zip(&previous) {
            assert!(
                current
                    .as_ref()
                    .unwrap()
                    .shares_frozen_storage(old.as_ref().unwrap())
            );
        }
        assert_eq!(memory.used_bytes(), before);
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
    }
}
fn dictionary(entries: &[(&str, &str)]) -> Data {
    Data::Map(
        entries
            .iter()
            .map(|(key, value)| MapEntryDraft {
                items: vec![Data::String((*key).into()), Data::String((*value).into())]
                    .into_boxed_slice(),
            })
            .collect(),
    )
}
fn selected(jit: &JitProgramArtifact) -> usize {
    jit.descriptor()
        .plan()
        .collections()
        .iter()
        .filter(|collection| collection.string_element_map_projection().is_some())
        .count()
}

#[test]
fn borrowed_string_map_entry_is_generic_unicode_empty_lazy_late_error_and_recovers() {
    for artifact in artifacts(SOURCE).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(selected(&jit), 1);
        assert_eq!(
            jit.descriptor()
                .plan()
                .blocks()
                .iter()
                .filter(|block| matches!(
                    block.action(),
                    NativeIterationAction::ProjectStringMapEntry { .. }
                ))
                .count(),
            1
        );
        let object = emit_program_object(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(
            object.descriptor().typed_ir_sha256(),
            jit.descriptor().typed_ir_sha256()
        );
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let entries = dictionary(&[
            ("", "unused"),
            ("é", "precomposed"),
            ("é", "combining"),
            ("🧑‍🚀", "🚀"),
            ("\r\n", "line"),
            ("\0", "zero"),
            ("x", ""),
        ]);
        let mut output = vec![None; artifact.outputs().len()];
        for (text, map, separator) in [
            ("éé🧑‍🚀\r\n\0x", entries.clone(), "→"),
            ("", dictionary(&[]), "é"),
            ("é?", entries.clone(), "|"),
            ("🧑‍🚀éé", entries.clone(), "\0"),
        ] {
            let args = inputs(
                &artifact,
                &[
                    ("source", Data::String(text.into())),
                    ("entries", map),
                    ("separator", Data::String(separator.into())),
                ],
                &memory,
            );
            match reference_result(&artifact, &args) {
                Ok(expected) => {
                    jit.evaluate_into(&args, &mut output, limits(), &memory)
                        .unwrap();
                    equal(&artifact, &output, &expected);
                }
                Err(error) => {
                    assert!(error.contains("InvalidInput"), "{error}");
                    let prior = output
                        .iter()
                        .map(|value| value.as_ref().unwrap().clone())
                        .collect::<Vec<_>>();
                    let baseline = memory.used_bytes();
                    let error = jit
                        .evaluate_into(&args, &mut output, limits(), &memory)
                        .unwrap_err();
                    let selected = jit
                        .descriptor()
                        .plan()
                        .collections()
                        .iter()
                        .find_map(|collection| collection.string_element_map_projection())
                        .unwrap();
                    assert!(
                        matches!(error, NativeProgramExecutionError::Library { origin,
                        error: NativeCollectionInvocationError::Kernel(ResidentKernelError::InvalidInput) }
                        if origin == selected.selection_template().origin()),
                        "{error:?}"
                    );
                    equal(&artifact, &output, &prior);
                    assert_eq!(memory.used_bytes(), baseline);
                }
            }
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn borrowed_string_map_entry_does_not_fuse_observed_key_or_intervening_work() {
    for source in [
        "+> text\n+> map\ntext/join([(entries<{string:string}>){piece} + piece | piece <- source<string>], separator<string>)\n",
        "+> text\n+> map\ntext/join([(entries<{string:string}>){piece} + \"extra\" | piece <- source<string>], separator<string>)\n",
        "+> text\n+> map\ntext/join([(entries<{string:string}>){piece} | piece <- source<string>, piece != \"\"], separator<string>)\n",
    ] {
        for artifact in artifacts(source).0 {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert_eq!(selected(&jit), 0, "{source}");
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let args = inputs(
                &artifact,
                &[
                    ("source", Data::String("é🧑‍🚀".into())),
                    ("entries", dictionary(&[("é", "first"), ("🧑‍🚀", "last")])),
                    ("separator", Data::String("|".into())),
                ],
                &memory,
            );
            let mut output = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &reference(&artifact, &args));
            drop((args, output));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn borrowed_string_map_entry_comparison_cap_is_cumulative_exact_and_one_short_atomic() {
    for artifact in artifacts(SOURCE).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(selected(&jit), 1);
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let args = inputs(
            &artifact,
            &[
                ("source", Data::String("azaz".into())),
                ("entries", dictionary(&[("a", "A"), ("m", "M"), ("z", "Z")])),
                ("separator", Data::String("|".into())),
            ],
            &memory,
        );
        let mut output = vec![None; artifact.outputs().len()];
        let baseline = jit
            .evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        let exact = baseline.comparison_work;
        assert!(exact > 0);
        let expected = reference(&artifact, &args);
        let used = memory.used_bytes();
        let mut policy = limits();
        policy.library.max_comparison_work = Some(exact);
        jit.evaluate_into(&args, &mut output, policy, &memory)
            .unwrap();
        equal(&artifact, &output, &expected);
        assert_eq!(memory.used_bytes(), used);
        policy.library.max_comparison_work = Some(exact - 1);
        let error = jit
            .evaluate_into(&args, &mut output, policy, &memory)
            .unwrap_err();
        let selected = jit
            .descriptor()
            .plan()
            .collections()
            .iter()
            .find_map(|collection| collection.string_element_map_projection())
            .unwrap();
        assert!(
            matches!(error, NativeProgramExecutionError::Library { origin,
            error: NativeCollectionInvocationError::Kernel(ResidentKernelError::ResourceLimit {
                dimension: mech_core::MemoryBudgetDimension::ComparisonWork, required, limit, .. }) }
            if origin == selected.selection_template().origin() && required == exact && limit == exact - 1),
            "{error:?}"
        );
        equal(&artifact, &output, &expected);
        assert_eq!(memory.used_bytes(), used);
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        equal(&artifact, &output, &expected);
        assert_eq!(memory.used_bytes(), used);
        drop((args, output));
        assert_eq!(memory.used_bytes(), 0);
    }
}
