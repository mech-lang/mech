//! Canonical source/decoded proof fixture for compiled whole-window regions.
use super::artifacts;
use mech_engine::native_artifact::NativeCollectionResult;
use mech_gpu::native_artifact::JitProgramArtifact;

const QUERY: &str = include_str!("../fixtures/native/window-query.mec");

#[test]
fn compiled_region_uses_constant_visits_and_preserves_all_string_semantics() {
    use super::{equal, inputs, limits, reference};
    use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
    for artifact in artifacts(QUERY).0 {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let mut outputs = vec![None; artifact.outputs().len()];
        let mut short_visits = None;
        for (text, width, query) in [
            ("AAAAA", 2, "AA"),
            ("AAAAAAAAAAAAAAAAAAAA", 2, "AA"),
            ("", 1, "A"),
            ("AAA", 0, ""),
            ("AAA", u64::MAX, "A"),
            ("a\0a\0a", 2, "a\0"),
            ("\rA\n\r", 1, "\r"),
            ("ééé", 2, "éé"),
            ("ééé", 2, "éé"),
            ("A\r\nA\r\n", 2, "A\r\n"),
            ("🧑‍🚀🧑‍🚀", 1, "🧑‍🚀"),
            ("AAAAA", 2, "é"),
            ("AAAAA", 2, "not present"),
        ] {
            let args = inputs(
                &artifact,
                &[
                    ("input", Data::String(text.into())),
                    ("width", Data::U64(width)),
                    ("query", Data::String(query.into())),
                ],
                &memory,
            );
            let report = jit
                .evaluate_into(&args, &mut outputs, limits(), &memory)
                .unwrap();
            equal(&artifact, &outputs, &reference(&artifact, &args));
            if text == "AAAAA" && query == "AA" {
                short_visits = Some(report.visits);
            }
            if text == "AAAAAAAAAAAAAAAAAAAA" {
                assert_eq!(
                    Some(report.visits),
                    short_visits,
                    "native loop must not callback per window"
                );
            }
        }
    }
}

#[test]
fn whole_window_comparison_admission_denies_without_publishing_partial_outputs() {
    use super::{inputs, limits};
    use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
    for artifact in artifacts(QUERY).0 {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let args = inputs(
            &artifact,
            &[
                ("input", Data::String("AAAAAAAAAA".into())),
                ("width", Data::U64(2)),
                ("query", Data::String("AA".into())),
            ],
            &memory,
        );
        let mut outputs = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut outputs, limits(), &memory)
            .unwrap();
        let previous = outputs.clone();
        let mut denied = limits();
        denied.library.max_comparison_work = Some(1);
        let error = jit
            .evaluate_into(&args, &mut outputs, denied, &memory)
            .unwrap_err();
        assert!(format!("{error:?}").contains("ComparisonWork"), "{error:?}");
        for (actual, previous) in outputs.iter().zip(&previous) {
            assert!(
                actual
                    .as_ref()
                    .unwrap()
                    .shares_frozen_storage(previous.as_ref().unwrap())
            );
        }
        jit.evaluate_into(&args, &mut outputs, limits(), &memory)
            .unwrap();
    }
}

#[test]
fn exclusive_inner_selector_keeps_its_shorter_window_and_empty_range_semantics() {
    use super::{equal, inputs, limits, reference_result};
    use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
    let source = QUERY.replace("(i + 1u64)..=(i + width)", "(i + 1u64)..(i + width)");
    for artifact in artifacts(&source).0 {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert!(jit.descriptor().plan().window_query_regions().is_empty());
        let mut outputs = vec![None; artifact.outputs().len()];
        for width in [2, 1] {
            let args = inputs(
                &artifact,
                &[
                    ("input", Data::String("AAAAA".into())),
                    ("width", Data::U64(width)),
                    ("query", Data::String("A".into())),
                ],
                &memory,
            );
            let expected = reference_result(&artifact, &args);
            let previous = outputs.clone();
            let result = jit.evaluate_into(&args, &mut outputs, limits(), &memory);
            match expected {
                Ok(expected) => {
                    result.unwrap();
                    equal(&artifact, &outputs, &expected);
                    assert!(matches!(
                        outputs.last().unwrap().as_ref().unwrap().data(),
                        mech_core::ValueData::U64(4)
                    ));
                }
                Err(_) => {
                    assert!(result.is_err());
                    for (actual, previous) in outputs.iter().zip(&previous) {
                        assert!(
                            actual
                                .as_ref()
                                .unwrap()
                                .shares_frozen_storage(previous.as_ref().unwrap())
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn closed_window_query_region_is_proved_from_canonical_operations_not_names() {
    for source in [QUERY.to_owned(), QUERY.replace("lookup", "arbitrary-name")] {
        for artifact in artifacts(&source).0 {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let regions = jit.descriptor().plan().window_query_regions();
            assert_eq!(regions.len(), 1);
            assert_eq!(regions[0].sites().len(), 14);
        }
    }
}

#[test]
fn window_region_declines_changed_iteration_bounds_guards_and_yield() {
    for source in [
        QUERY.replace("0u64..=n", "1u64..=n"),
        QUERY.replace("0u64..=n", "0u64..n"),
        QUERY.replace("width > 0u64", "width > 1u64"),
        QUERY.replace("width <= n - i", "width < n - i"),
        QUERY.replace("i + 1u64", "i + 2u64"),
        QUERY.replace("(i + 1u64)..=(i + width)", "(i + 1u64)..(i + width)"),
        QUERY.replace("width <= n - i", "width <= n - i, i != 2u64"),
    ] {
        for artifact in artifacts(&source).0 {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert!(
                jit.descriptor().plan().window_query_regions().is_empty(),
                "{source}"
            );
        }
    }
}

#[test]
#[ignore = "manual canonical window-region compiler diagnostic; not a benchmark"]
fn inspect_complete_window_query_compiler_region() {
    for (variant, artifact) in artifacts(QUERY).0.iter().enumerate() {
        let jit = JitProgramArtifact::compile(artifact, &mech_stdlib::source_catalog()).unwrap();
        let plan = jit.descriptor().plan();
        let collections = plan
            .collections()
            .iter()
            .filter(|collection| {
                collection.string_span_projection().is_some()
                    && matches!(
                        collection.result(),
                        NativeCollectionResult::GuardedHistogramQuery(_)
                    )
            })
            .collect::<Vec<_>>();
        assert_eq!(collections.len(), 1);
        eprintln!("WINDOW_REGION variant={variant} entry={}", plan.entry());
        eprintln!("COLLECTION {:#?}", collections[0]);
        eprintln!("ITERATIONS {:#?}", plan.iterations());
        for constant in plan.constants() {
            eprintln!("CONSTANT {constant:?}");
        }
        for (index, block) in plan.blocks().iter().enumerate() {
            eprintln!(
                "BLOCK {index} iteration={:?} action={:?} terminator={:?}",
                block.iteration(),
                block.action(),
                block.terminator()
            );
        }
    }
}
