//! Real selected cursors reject impossible public range storage before body work.
use super::super::{artifacts, equal, inputs, limits, reference};
use mech_core::{
    ManagedMemoryBudget, MemoryBudgetDimension, ResidentKernelError, ResidentResourcePhase,
    ValueDataDraft as Data,
};
use mech_engine::native_artifact::{
    NativeCollectionInvocationError as LeafError, NativeCollectionSource, NativeControlLoop,
    NativeIterationAction, NativeMatchAction, NativeMatchOrigin, NativeScalarOp,
};
use mech_gpu::native_artifact::{
    JitProgramArtifact, NativeIterationExecutionError, NativeProgramExecutionError as Error,
};

fn source(separator: &str) -> String {
    format!("[i / divisor<u64> | i <- start<u64>{separator}end<u64>]\n")
}
fn end_for_count(count: u64, inclusive: bool) -> u64 {
    if inclusive { count - 1 } else { count }
}
fn origins(jit: &JitProgramArtifact) -> (NativeMatchOrigin, NativeMatchOrigin) {
    let plan = jit.descriptor().plan();
    let cursors = plan
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
    assert_eq!(
        cursors.len(),
        1,
        "must execute the actual selected range source"
    );
    let body = plan
        .blocks()
        .iter()
        .find(|block| {
            matches!(
                block.action(),
                NativeIterationAction::Scalar(NativeMatchAction::Primitive {
                    op: NativeScalarOp::U64Div,
                    ..
                })
            )
        })
        .expect("actual per-candidate checked division")
        .origin();
    (cursors[0].template().origin(), body)
}
fn layout_limit() -> u64 {
    // Layout::array uses the CURRENT target's isize::MAX and actual Rust draft
    // layout. This formula is target-relative on both32- and64-bit builds.
    (isize::MAX as usize / core::mem::size_of::<Data>() - 2) as u64
}

#[test]
fn selected_range_layout_boundary_precedes_body_errors_and_matches_materialized_range() {
    let largest = layout_limit();
    for (separator, inclusive) in [("..", false), ("..=", true)] {
        let ordinary = artifacts(&format!("start<u64>{separator}end<u64>\n")).0;
        for (artifact, ordinary) in artifacts(&source(separator)).0.into_iter().zip(ordinary) {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let catalog = mech_stdlib::source_catalog();
            let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let (range_origin, body_origin) = origins(&jit);
            assert_ne!(range_origin, body_origin);
            let small_end = if inclusive { 3 } else { 4 };
            let good = inputs(
                &artifact,
                &[
                    ("start", Data::U64(1)),
                    ("end", Data::U64(small_end)),
                    ("divisor", Data::U64(1)),
                ],
                &memory,
            );
            let mut output = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&good, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &reference(&artifact, &good));
            let prior = output
                .iter()
                .map(|v| v.as_ref().unwrap().clone())
                .collect::<Vec<_>>();
            // Accepted exact/lower Layout boundaries reach the first real body
            // error, so no enormous loop or Matrix allocation is performed.
            // One beyond must fail at the ORIGINAL selected range first.
            for count in [largest - 1, largest, largest + 1] {
                let args = inputs(
                    &artifact,
                    &[
                        ("start", Data::U64(0)),
                        ("end", Data::U64(end_for_count(count, inclusive))),
                        ("divisor", Data::U64(0)),
                    ],
                    &memory,
                );
                let before = memory.used_bytes();
                let error = jit
                    .evaluate_into(&args, &mut output, limits(), &memory)
                    .unwrap_err();
                if count <= largest {
                    assert!(
                        matches!(error, Error::Control(
                        NativeIterationExecutionError::Arithmetic { origin }
                    ) if origin == body_origin),
                        "{count}{separator}: {error:?}"
                    );
                } else {
                    assert!(
                        matches!(error, Error::Library { origin,
                        error: LeafError::Kernel(ResidentKernelError::InvalidShape)
                    } if origin == range_origin),
                        "{count}{separator}: {error:?}"
                    );
                }
                assert_eq!(memory.used_bytes(), before);
                equal(&artifact, &output, &prior);
            }
            // Empty/reversed or overflowing cardinalities remain InvalidInput
            // before representation and before the same pending division.
            for (start, end) in [(9, 2), if inclusive { (0, u64::MAX) } else { (7, 7) }] {
                let args = inputs(
                    &artifact,
                    &[
                        ("start", Data::U64(start)),
                        ("end", Data::U64(end)),
                        ("divisor", Data::U64(0)),
                    ],
                    &memory,
                );
                let before = memory.used_bytes();
                let error = jit
                    .evaluate_into(&args, &mut output, limits(), &memory)
                    .unwrap_err();
                assert!(
                    matches!(error, Error::Library { origin,
                    error: LeafError::Kernel(ResidentKernelError::InvalidInput)
                } if origin == range_origin),
                    "{start}{separator}{end}: {error:?}"
                );
                assert_eq!(memory.used_bytes(), before);
                equal(&artifact, &output, &prior);
            }
            // SAME maintained ordinary range, still materialized/observable.
            // Its Layout rejection precedes payload allocation and quotas.
            let ordinary_jit = JitProgramArtifact::compile(&ordinary, &catalog).unwrap();
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
            let args = inputs(
                &ordinary,
                &[
                    ("start", Data::U64(0)),
                    ("end", Data::U64(end_for_count(largest + 1, inclusive))),
                ],
                &memory,
            );
            let mut ordinary_output = vec![None; ordinary.outputs().len()];
            let before = memory.used_bytes();
            let error = ordinary_jit
                .evaluate_into(&args, &mut ordinary_output, limits(), &memory)
                .unwrap_err();
            assert!(
                matches!(
                    error,
                    Error::Library {
                        error: LeafError::Kernel(ResidentKernelError::InvalidShape),
                        ..
                    }
                ),
                "materialized {separator}: {error:?}"
            );
            assert!(ordinary_output.iter().all(Option::is_none));
            assert_eq!(memory.used_bytes(), before);
            jit.evaluate_into(&good, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &prior);
            drop((args, ordinary_output, good, output, prior));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn selected_range_layout_check_does_not_move_admission_or_waive_real_output_quotas() {
    for artifact in artifacts(&source("..")).0 {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let (range_origin, _) = origins(&jit);
        let good = inputs(
            &artifact,
            &[
                ("start", Data::U64(1)),
                ("end", Data::U64(4)),
                ("divisor", Data::U64(1)),
            ],
            &memory,
        );
        let mut output = vec![None; artifact.outputs().len()];
        let mut caps = limits();
        caps.library.max_output_elements = Some(3);
        jit.evaluate_into(&good, &mut output, caps, &memory)
            .unwrap();
        let prior = output
            .iter()
            .map(|v| v.as_ref().unwrap().clone())
            .collect::<Vec<_>>();
        let huge = inputs(
            &artifact,
            &[
                ("start", Data::U64(0)),
                ("end", Data::U64(layout_limit() + 1)),
                ("divisor", Data::U64(0)),
            ],
            &memory,
        );
        let before = memory.used_bytes();
        let mut denied = caps;
        denied.library.max_compute_work = Some(1);
        let error = jit
            .evaluate_into(&huge, &mut output, denied, &memory)
            .unwrap_err();
        assert!(
            matches!(error, Error::Collection { origin,
            error: ResidentKernelError::ResourceLimit { phase: ResidentResourcePhase::Admission,
                dimension: MemoryBudgetDimension::ComputeWork, limit: 1, .. }
        } if origin == range_origin),
            "{error:?}"
        );
        assert_eq!(memory.used_bytes(), before);
        equal(&artifact, &output, &prior);
        let four = inputs(
            &artifact,
            &[
                ("start", Data::U64(1)),
                ("end", Data::U64(5)),
                ("divisor", Data::U64(1)),
            ],
            &memory,
        );
        let before = memory.used_bytes();
        let error = jit
            .evaluate_into(&four, &mut output, caps, &memory)
            .unwrap_err();
        assert!(
            matches!(
                error,
                Error::Collection {
                    error: ResidentKernelError::ResourceLimit {
                        dimension: MemoryBudgetDimension::OutputElements,
                        required: 4,
                        limit: 3,
                        ..
                    },
                    ..
                }
            ),
            "{error:?}"
        );
        assert_eq!(memory.used_bytes(), before);
        equal(&artifact, &output, &prior);
        jit.evaluate_into(&good, &mut output, caps, &memory)
            .unwrap();
        equal(&artifact, &output, &prior);
        drop((four, huge, good, output, prior));
        assert_eq!(memory.used_bytes(), 0);
    }
}
