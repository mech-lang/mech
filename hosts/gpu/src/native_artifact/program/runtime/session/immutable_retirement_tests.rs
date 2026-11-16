//! Immutable lexical table storage stays in the exact held descriptor. These
//! tests exercise real original/decoded/restored scalar and numeric-loop entries.
use super::*;
use crate::native_artifact::JitProgramArtifact;

// Plain numeric append tail: no String/map/captured-input scanner fixture.
const NUMERIC_SOURCE: &str = r#"+> sequence
retirement-count(at<u64>, end<u64>, buffer<[u64]>) => (out<[u64]>) :=
  more := at < end
  next := (more? | true => sequence/append(buffer, at) | false => buffer)
  out := (more? | true => retirement-count(at + 1u64, end, next) | false => buffer).
end := seed<u64>
retirement-count(0u64, end, sequence/with-capacity(0u64, end))
"#;

fn limits() -> NativeProgramExecutionLimits {
    NativeProgramExecutionLimits {
        execution: NativeExecutionLimits {
            max_work: 1_000_000_000,
            ..Default::default()
        },
        library: call_cache_tests::policy(),
    }
}

fn assert_descriptor_retirements(frame: *mut Frame, descriptor: &NativeProgramDescriptor) {
    // SAFETY: the caller wraps this Prepared owner's still-live synchronous
    // generated entry. Read the private context before returning to native code;
    // no reference obtained here survives that call or escapes the callback.
    let frame = unsafe { &*frame };
    let context = unsafe { &*frame.base.context.cast::<Invocation<'_, '_, '_>>() };
    let expected = descriptor.metadata().retirements.words();
    assert_eq!(context.retirements.as_ptr(), expected.as_ptr());
    assert_eq!(context.retirements.len(), expected.len());
    assert_eq!(context.retirements, expected);
}

#[test]
fn immutable_retirement_table_borrow_survives_restoration_and_full_numeric_execution() {
    for (source, scalar) in [("seed<u64> + 1u64\n", true), (NUMERIC_SOURCE, false)] {
        let mut expected_work = None;
        for artifact in call_cache_tests::artifacts(source) {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            for restored in [false, true] {
                let restored_descriptor = restored.then(|| {
                    crate::native_artifact::program::runtime::tests::restore_admitted_helper_descriptor(&jit)
                });
                let descriptor = restored_descriptor.as_ref().unwrap_or(jit.descriptor());
                let geometry = Prepared::setup_geometry(descriptor).unwrap();
                let retired = descriptor.metadata().retirements.words().len();
                assert_eq!(retired == 0, scalar);
                let memory = ManagedMemoryBudget::new(256 << 20);
                let mut prepared = Prepared::new(descriptor, limits(), &memory).unwrap();
                assert_eq!(prepared.storage.values().len(), geometry.words - retired);
                if scalar {
                    assert_eq!(
                        prepared.storage.values().len(),
                        geometry.words,
                        "zero-table scalar transport retains its exact physical extent"
                    );
                }
                let value = ValueDraft {
                    schema: descriptor.metadata().plan.inputs()[0].ty().schema(),
                    shape_values: Box::new([]),
                    data: ValueDataDraft::U64(if scalar { 41 } else { 7 }),
                }
                .finalize(&SnapshotValidationContext::with_shared_schemas(
                    &descriptor.metadata().schemas,
                ))
                .unwrap()
                .admit_into_memory_budget(&memory)
                .unwrap();
                let inputs = [value];
                let aliases = inputs.clone();
                let before = inputs[0]
                    .canonical_snapshot_bytes(&descriptor.metadata().schemas)
                    .unwrap();
                let mut outputs = vec![None; descriptor.metadata().plan.transport_output_count()];
                let mut native = jit.test_generated_entry();
                let mut calls = 0;
                let mut entry = |frame| {
                    assert_descriptor_retirements(frame, descriptor);
                    calls += 1;
                    native(frame)
                };
                prepared.activate(&mut entry).unwrap();
                let report = prepared
                    .finish(&mut entry, &inputs, &[], &mut outputs)
                    .unwrap();
                assert_eq!(calls, 2, "activation and one complete turn");
                if scalar {
                    assert!(matches!(
                        outputs[0].as_ref().unwrap().data(),
                        ValueData::U64(42)
                    ));
                } else {
                    let ValueData::Matrix(matrix) = outputs[0].as_ref().unwrap().data() else {
                        panic!("numeric loop must return a packed matrix");
                    };
                    assert!(
                        matches!(matrix.elements(), mech_core::snapshot::SequenceView::U64(words)
                        if words == &[0, 1, 2, 3, 4, 5, 6])
                    );
                }
                let work = (report.work, report.comparison_work);
                if let Some(expected) = expected_work {
                    assert_eq!(work, expected);
                } else {
                    expected_work = Some(work);
                }
                assert_eq!(
                    inputs[0]
                        .canonical_snapshot_bytes(&descriptor.metadata().schemas)
                        .unwrap(),
                    before
                );
                assert_eq!(
                    aliases[0]
                        .canonical_snapshot_bytes(&descriptor.metadata().schemas)
                        .unwrap(),
                    before
                );
                drop((outputs, inputs, aliases));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}

#[test]
fn immutable_retirement_storage_retains_finite_setup_transport_and_work_denials() {
    for artifact in call_cache_tests::artifacts(NUMERIC_SOURCE) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        for restored in [false, true] {
            let restored_descriptor = restored.then(|| {
                crate::native_artifact::program::runtime::tests::restore_admitted_helper_descriptor(
                    &jit,
                )
            });
            let descriptor = restored_descriptor.as_ref().unwrap_or(jit.descriptor());
            let geometry = Prepared::setup_geometry(descriptor).unwrap();
            let retired = descriptor.metadata().retirements.words().len();
            assert!(retired > 0);
            let memory = ManagedMemoryBudget::new(256 << 20);
            let prepared = Prepared::new(descriptor, limits(), &memory).unwrap();
            let setup_work = prepared.state.as_ref().unwrap().control.work.used();
            assert!(
                setup_work >= geometry.logical_words as u64 * 4 + 256 + retired as u64 * 16 + 128
            );
            drop(prepared);
            assert_eq!(memory.used_bytes(), 0);
            let mut transport_short = limits();
            transport_short.execution.max_transport_bytes = geometry.bytes - 1;
            assert!(
                matches!(Prepared::new(descriptor, transport_short, &memory),
                Err(NativeProgramExecutionError::Base(NativeExecutionError::TransportLimit {required, limit}))
                    if required == geometry.bytes && limit == geometry.bytes - 1)
            );
            assert_eq!(memory.used_bytes(), 0);
            let mut work_short = limits();
            work_short.execution.max_work = setup_work - 1;
            assert!(Prepared::new(descriptor, work_short, &memory).is_err());
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
            let retry = Prepared::new(descriptor, limits(), &memory).unwrap();
            assert_eq!(
                retry.state.as_ref().unwrap().control.work.used(),
                setup_work
            );
            drop(retry);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
