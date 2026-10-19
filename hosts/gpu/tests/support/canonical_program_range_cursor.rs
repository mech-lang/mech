//! Actual generated range cursors: canonical semantics and no Matrix fiction.
use super::{artifacts, equal, inputs, limits, reference, reference_result};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_engine::native_artifact::{
    NativeCollectionSource, NativeControlLoop, NativeIterationAction,
};
use mech_gpu::native_artifact::{
    JitProgramArtifact, NativeProgramExecutionError, emit_program_object,
};

#[test]
fn selected_range_cursor_preserves_terminal_empty_and_overflow_errors_and_recovers() {
    for separator in ["..", "..="] {
        let source = format!("[i | i <- start<u64>{separator}end<u64>]\n");
        let materialized = artifacts(&format!("start<u64>{separator}end<u64>\n")).0;
        for (artifact, ordinary_artifact) in artifacts(&source).0.into_iter().zip(materialized) {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let cursors = jit.descriptor().plan().iterations().iter().filter(|data| matches!(data,
                NativeControlLoop::Collection(generator) if matches!(generator.source(), NativeCollectionSource::U64Range(_)))).count();
            assert_eq!(cursors, 1, "the emitted path must really use a cursor");
            let object = emit_program_object(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert_eq!(
                object.descriptor().typed_ir_sha256(),
                jit.descriptor().typed_ir_sha256()
            );
            let mut output = vec![None; artifact.outputs().len()];
            let ordinary_jit =
                JitProgramArtifact::compile(&ordinary_artifact, &mech_stdlib::source_catalog())
                    .unwrap();
            assert!(
                ordinary_jit
                    .descriptor()
                    .plan()
                    .blocks()
                    .iter()
                    .all(|block| !matches!(
                        block.action(),
                        NativeIterationAction::PrepareU64Range { .. }
                    ))
            );
            for (start, end) in [
                (1, 4),
                (7, 7),
                (9, 2),
                (u64::MAX - 2, u64::MAX),
                (0, u64::MAX),
                (2, 5),
            ] {
                let args = inputs(
                    &artifact,
                    &[("start", Data::U64(start)), ("end", Data::U64(end))],
                    &memory,
                );
                // A huge valid exclusive range is a resource-denial case, not
                // a semantic cardinality error; never ask the reference to allocate it.
                if start == 0 && end == u64::MAX && separator == ".." {
                    continue;
                }
                let expected = reference_result(&artifact, &args);
                match expected {
                    Ok(expected) => {
                        jit.evaluate_into(&args, &mut output, limits(), &memory)
                            .unwrap();
                        equal(&artifact, &output, &expected);
                    }
                    Err(_) => {
                        let prior = output
                            .iter()
                            .map(|value| value.as_ref().unwrap().clone())
                            .collect::<Vec<_>>();
                        let before = memory.used_bytes();
                        let error = jit
                            .evaluate_into(&args, &mut output, limits(), &memory)
                            .unwrap_err();
                        let expected_origin = jit
                            .descriptor()
                            .plan()
                            .iterations()
                            .iter()
                            .find_map(|data| match data {
                                NativeControlLoop::Collection(data) => match data.source() {
                                    NativeCollectionSource::U64Range(cursor) => {
                                        Some(cursor.template().origin())
                                    }
                                    _ => None,
                                },
                                _ => None,
                            })
                            .unwrap();
                        assert!(
                            matches!(error, NativeProgramExecutionError::Library { origin,
                            error: mech_engine::native_artifact::NativeCollectionInvocationError::Kernel(mech_core::ResidentKernelError::InvalidInput)
                        } if origin == expected_origin),
                            "{start}{separator}{end}: {error:?}"
                        );
                        assert_eq!(memory.used_bytes(), before);
                        equal(&artifact, &output, &prior);
                        // Compare the exact maintained error kind with a real
                        // selected range that remains materialized/observable.
                        let ordinary_args = inputs(
                            &ordinary_artifact,
                            &[("start", Data::U64(start)), ("end", Data::U64(end))],
                            &memory,
                        );
                        let mut ordinary_output = vec![None; ordinary_artifact.outputs().len()];
                        let ordinary_error = ordinary_jit
                            .evaluate_into(&ordinary_args, &mut ordinary_output, limits(), &memory)
                            .unwrap_err();
                        assert!(matches!(ordinary_error, NativeProgramExecutionError::Library {
                            error: mech_engine::native_artifact::NativeCollectionInvocationError::Kernel(mech_core::ResidentKernelError::InvalidInput), ..
                        }), "ordinary {start}{separator}{end}: {ordinary_error:?}");
                    }
                }
            }
            drop(output);
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn nested_selected_range_cursors_reset_for_each_outer_visit_and_each_invocation() {
    let source = "[i + j | i <- 1u64..=outer<u64>, j <- 1u64..=i]\n";
    for artifact in artifacts(source).0 {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let cursor_count = jit.descriptor().plan().iterations().iter().filter(|data| matches!(data,
            NativeControlLoop::Collection(generator) if matches!(generator.source(), NativeCollectionSource::U64Range(_)))).count();
        assert_eq!(cursor_count, 2);
        let object = emit_program_object(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(
            object.descriptor().typed_ir_sha256(),
            jit.descriptor().typed_ir_sha256()
        );
        let mut output = vec![None; artifact.outputs().len()];
        for outer in [1u64, 4, 2, 3, 1] {
            let args = inputs(&artifact, &[("outer", Data::U64(outer))], &memory);
            let expected = reference(&artifact, &args);
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &expected);
            let oracle = Data::Matrix(
                (1..=outer)
                    .flat_map(|i| (1..=i).map(move |j| Data::U64(i + j)))
                    .collect(),
            );
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                oracle
            );
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn selected_range_cursor_keeps_every_candidate_filter_order_and_only_admits_real_output() {
    let source = "[i | i <- 0u64..=count<u64>, i < 2u64]\n";
    for artifact in artifacts(source).0 {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert!(
            jit.descriptor()
                .plan()
                .blocks()
                .iter()
                .any(|block| matches!(
                    block.action(),
                    NativeIterationAction::PrepareU64Range { .. }
                ))
        );
        let mut caps = limits();
        caps.library.max_output_elements = Some(2);
        let mut output = vec![None; artifact.outputs().len()];
        let mut small_visits = None;
        for count in [3u64, 1024, 3] {
            let args = inputs(&artifact, &[("count", Data::U64(count))], &memory);
            let expected = reference(&artifact, &args);
            let report = jit
                .evaluate_into(&args, &mut output, caps, &memory)
                .unwrap();
            equal(&artifact, &output, &expected);
            if count == 3 {
                if let Some(before) = small_visits {
                    assert_eq!(report.visits, before);
                }
                small_visits = Some(report.visits);
            } else {
                assert!(
                    report.visits >= small_visits.unwrap() + count - 3,
                    "filters do not license skipping source candidates"
                );
            }
        }
        let args = inputs(&artifact, &[("count", Data::U64(3))], &memory);
        let prior = output
            .iter()
            .map(|value| value.as_ref().unwrap().clone())
            .collect::<Vec<_>>();
        let before = memory.used_bytes();
        let mut denied = caps;
        denied.library.max_compute_work = Some(1);
        assert!(
            jit.evaluate_into(&args, &mut output, denied, &memory)
                .is_err()
        );
        assert_eq!(memory.used_bytes(), before);
        equal(&artifact, &output, &prior);
        let ballast = memory
            .reserve_capacity(memory.limit_bytes() - before)
            .unwrap();
        assert!(
            jit.evaluate_into(&args, &mut output, caps, &memory)
                .is_err()
        );
        equal(&artifact, &output, &prior);
        drop(ballast);
        assert_eq!(memory.used_bytes(), before);
        jit.evaluate_into(&args, &mut output, caps, &memory)
            .unwrap();
        equal(&artifact, &output, &prior);
        drop((args, output, prior));
        assert_eq!(memory.used_bytes(), 0);
    }
    // Keep the fit guard before subtraction and the exact n+1 candidates.
    for (text, width) in [("", 2), ("ABBA", 0), ("ABBA", 5), ("é🧑‍🚀é", 2)] {
        super::check(
            &format!(
                "{}windows(input<string>, width<u64>)\n",
                super::window_qualification::WINDOWS_FUNCTION
            ),
            &[
                ("input", Data::String(text.into())),
                ("width", Data::U64(width)),
            ],
        );
    }
}

#[test]
fn hoisted_range_cursor_executes_before_the_builder_and_reads_each_invocations_endpoints() {
    let source = "collect(start<u64>, end<u64>) => (out<[u64]>) :=\n  range := start..end\n  out := [i | i <- range].\ncollect(start<u64>, end<u64>)\n";
    for artifact in artifacts(source).0 {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let cursors = jit
            .descriptor()
            .plan()
            .iterations()
            .iter()
            .filter_map(|iteration| match iteration {
                NativeControlLoop::Collection(data) => match data.source() {
                    NativeCollectionSource::U64Range(cursor) => Some(cursor),
                    _ => None,
                },
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(cursors.len(), 1);
        assert!(cursors[0].before_collection_begin());
        let mut output = vec![None; artifact.outputs().len()];
        for (start, end) in [(0, 4), (7, 10), (u64::MAX - 2, u64::MAX), (1, 3)] {
            let args = inputs(
                &artifact,
                &[("start", Data::U64(start)), ("end", Data::U64(end))],
                &memory,
            );
            let expected = reference(&artifact, &args);
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &expected);
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn range_cursor_selection_rejects_replaced_factories_before_binding_or_execution() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static BINDS: AtomicUsize = AtomicUsize::new(0);
    fn foreign(
        _: &mech_core::ResidentKernelBindRequest<'_>,
    ) -> Result<mech_core::BoundResidentKernel, mech_core::ResidentKernelBindError> {
        BINDS.fetch_add(1, Ordering::SeqCst);
        Err(mech_core::ResidentKernelBindError::UnsupportedLayout)
    }
    let catalog = mech_stdlib::source_catalog();
    let entry = catalog
        .resident_factory(&["range".into()], "exclusive")
        .unwrap();
    let mut replaced = mech_core::FunctionCatalogBuilder::new();
    replaced
        .insert_resident_factory_entry(entry.clone().with_factory(foreign))
        .unwrap();
    let replaced = replaced.build().unwrap();
    for artifact in artifacts("[i | i <- 0u64..count<u64>]\n").0 {
        assert!(JitProgramArtifact::compile(&artifact, &replaced).is_err());
        assert!(emit_program_object(&artifact, &replaced).is_err());
    }
    assert_eq!(BINDS.load(Ordering::SeqCst), 0);
}
