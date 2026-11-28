//! Existing selected range sources also feed matrix-block builders.
use super::{artifacts, equal, inputs, limits, reference, reference_result};
use mech_core::{ManagedMemoryBudget, ResidentKernelError, ValueDataDraft as Data};
use mech_engine::native_artifact::{
    NativeCollectionInvocationError, NativeCollectionSource, NativeControlLoop,
    NativeU64RangeCursor,
};
use mech_gpu::native_artifact::{
    JitProgramArtifact, NativeExecutionError, NativeProgramDescriptor,
    NativeProgramExecutionError as Error, emit_program_object,
};

fn block_cursor(jit: &JitProgramArtifact) -> &NativeU64RangeCursor {
    let plan = jit.descriptor().plan();
    plan.iterations()
        .iter()
        .find_map(|data| {
            let NativeControlLoop::Collection(data) = data else {
                return None;
            };
            if !matches!(
                plan.collections()[data.collection() as usize].kind(),
                mech_engine::ComprehensionKind::MatrixHorizontalBlocks { .. }
            ) {
                return None;
            }
            match data.source() {
                NativeCollectionSource::U64Range(cursor) => Some(cursor),
                _ => None,
            }
        })
        .expect("the block generator must actually select a U64 cursor")
}

#[test]
fn block_range_cursors_preserve_endpoints_errors_finite_limits_and_recovery() {
    for separator in ["..", "..="] {
        for hoisted in [false, true] {
            let source = if hoisted {
                format!(
                    "collect(start<u64>, end<u64>) => (out<[u64]>) :=\n  range := start{separator}end\n  out := [[j - i + 1u64 | i <- 1u64..=2u64]' | j <- range].\ncollect(start<u64>, end<u64>)\n"
                )
            } else {
                format!(
                    "[[j - i + 1u64 | i <- 1u64..=2u64]' | j <- start<u64>{separator}end<u64>]\n"
                )
            };
            for artifact in artifacts(&source).0 {
                let catalog = mech_stdlib::source_catalog();
                let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
                let cursor = block_cursor(&jit);
                assert_eq!(cursor.before_collection_begin(), hoisted);
                let origin = cursor.template().origin();
                let object = emit_program_object(&artifact, &catalog).unwrap();
                assert_eq!(
                    object.descriptor().typed_ir_sha256(),
                    jit.descriptor().typed_ir_sha256()
                );
                let image = jit.descriptor().encode_build_owned_image().unwrap();
                // SAFETY: this checked JIT build owns these unchanged bytes;
                // restored metadata is compared, never executed in this test.
                let restored =
                    unsafe { NativeProgramDescriptor::decode_build_owned_image(&image, &catalog) }
                        .unwrap();
                assert_eq!(restored.encode_build_owned_image().unwrap(), image);
                let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
                let mut output = vec![None; artifact.outputs().len()];
                for (start, end) in [(2, 5), (8, 10), (u64::MAX - 1, u64::MAX), (2, 5)] {
                    let args = inputs(
                        &artifact,
                        &[("start", Data::U64(start)), ("end", Data::U64(end))],
                        &memory,
                    );
                    let before = args
                        .iter()
                        .map(|v| v.canonical_data_draft().unwrap())
                        .collect::<Vec<_>>();
                    jit.evaluate_into(&args, &mut output, limits(), &memory)
                        .unwrap();
                    equal(&artifact, &output, &reference(&artifact, &args));
                    assert_eq!(
                        args.iter()
                            .map(|v| v.canonical_data_draft().unwrap())
                            .collect::<Vec<_>>(),
                        before
                    );
                }
                let prior = output
                    .iter()
                    .map(|v| v.as_ref().unwrap().clone())
                    .collect::<Vec<_>>();
                let invalid = if separator == "..=" {
                    vec![(9, 2), (0, u64::MAX)]
                } else {
                    vec![(9, 2), (7, 7)]
                };
                for (start, end) in invalid {
                    let args = inputs(
                        &artifact,
                        &[("start", Data::U64(start)), ("end", Data::U64(end))],
                        &memory,
                    );
                    assert!(reference_result(&artifact, &args).is_err());
                    let retained = memory.used_bytes();
                    let error = jit
                        .evaluate_into(&args, &mut output, limits(), &memory)
                        .unwrap_err();
                    assert!(
                        matches!(error, Error::Library { origin: actual,
                        error: NativeCollectionInvocationError::Kernel(ResidentKernelError::InvalidInput)
                    } if actual == origin),
                        "{source}: {error:?}"
                    );
                    assert_eq!(memory.used_bytes(), retained);
                    equal(&artifact, &output, &prior);
                }
                let good = inputs(
                    &artifact,
                    &[("start", Data::U64(2)), ("end", Data::U64(5))],
                    &memory,
                );
                for dimension in 0..3 {
                    let mut denied = limits();
                    match dimension {
                        0 => denied.library.max_compute_work = Some(1),
                        1 => denied.library.max_output_elements = Some(1),
                        _ => denied.execution.max_work = 1,
                    }
                    let retained = memory.used_bytes();
                    assert!(
                        jit.evaluate_into(&good, &mut output, denied, &memory)
                            .is_err()
                    );
                    assert_eq!(memory.used_bytes(), retained);
                    equal(&artifact, &output, &prior);
                }
                jit.evaluate_into(&good, &mut output, limits(), &memory)
                    .unwrap();
                equal(&artifact, &output, &reference(&artifact, &good));
                drop((good, output, prior));
                assert_eq!(memory.used_bytes(), 0);
            }
        }
    }
}

#[test]
fn block_range_cursors_retain_dependent_row_shape_errors_and_recover() {
    let source = "[[i | i <- 1u64..=j]' | j <- 1u64..=count<u64>]\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        block_cursor(&jit);
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        let good = inputs(&artifact, &[("count", Data::U64(1))], &memory);
        jit.evaluate_into(&good, &mut output, limits(), &memory)
            .unwrap();
        equal(&artifact, &output, &reference(&artifact, &good));
        let prior = output
            .iter()
            .map(|v| v.as_ref().unwrap().clone())
            .collect::<Vec<_>>();
        for count in [2, 3, 2] {
            let args = inputs(&artifact, &[("count", Data::U64(count))], &memory);
            assert!(reference_result(&artifact, &args).is_err());
            let retained = memory.used_bytes();
            let error = jit
                .evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap_err();
            assert!(
                matches!(
                    error,
                    Error::Base(NativeExecutionError::Descriptor {
                        reason: "native matrix block row counts differ"
                    })
                ),
                "{error:?}"
            );
            assert_eq!(memory.used_bytes(), retained);
            equal(&artifact, &output, &prior);
        }
        jit.evaluate_into(&good, &mut output, limits(), &memory)
            .unwrap();
        equal(&artifact, &output, &prior);
        drop((good, output, prior));
        assert_eq!(memory.used_bytes(), 0);
    }
}
