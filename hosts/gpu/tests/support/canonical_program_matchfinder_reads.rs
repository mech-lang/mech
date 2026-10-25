//! Byte-order and checked-read failures in the actual Mech word-loader helpers.
use super::super::{artifacts, equal, inputs, limits, reference, reference_result};
use super::KERNEL;
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::{JitProgramArtifact, NativeProgramExecutionError as Error};

#[test]
fn deflate_prefix_and_word_reads_preserve_all_bits_at_unaligned_offsets() {
    let bytes = (0..73).map(|i| (i * 37 + 129) as u8).collect::<Vec<_>>();
    for (helper, width) in [("load-prefix", 4), ("load-word", 8)] {
        let source = format!("{KERNEL}\n{helper}(input<[u8]>, at<u64>)\n");
        for artifact in artifacts(&source).0 {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let mut output = vec![None; artifact.outputs().len()];
            for at in [0, 1, 7, 8, 9, 31, 32, 33, bytes.len() - width] {
                let args = inputs(
                    &artifact,
                    &[
                        (
                            "input",
                            Data::Matrix(bytes.iter().copied().map(Data::U8).collect()),
                        ),
                        ("at", Data::U64(at as u64)),
                    ],
                    &memory,
                );
                let ordinary = reference(&artifact, &args);
                jit.evaluate_into(&args, &mut output, limits(), &memory)
                    .unwrap();
                equal(&artifact, &output, &ordinary);
                let expected = if width == 4 {
                    Data::U32(u32::from_le_bytes(
                        bytes[at..at + width].try_into().unwrap(),
                    ))
                } else {
                    Data::U64(u64::from_le_bytes(
                        bytes[at..at + width].try_into().unwrap(),
                    ))
                };
                assert_eq!(
                    output
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    expected
                );
            }
            drop(output);
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn deflate_word_read_late_bounds_and_offset_overflow_preserve_publication_and_recover() {
    let source = format!("{KERNEL}\nload-word(input<[u8]>, at<u64>)\n");
    for artifact in artifacts(&source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for at in [0u64, 25, 7, u64::MAX, 24] {
            let args = inputs(
                &artifact,
                &[
                    ("input", Data::Matrix((0..32).map(Data::U8).collect())),
                    ("at", Data::U64(at)),
                ],
                &memory,
            );
            if at == 25 || at == u64::MAX {
                assert!(reference_result(&artifact, &args).is_err());
                let prior = output
                    .iter()
                    .map(|value| value.clone().unwrap())
                    .collect::<Vec<_>>();
                let before = memory.used_bytes();
                let error = jit
                    .evaluate_into(&args, &mut output, limits(), &memory)
                    .unwrap_err();
                if at == u64::MAX {
                    assert!(matches!(error, Error::Control(mech_gpu::native_artifact::NativeIterationExecutionError::Arithmetic { .. })), "{error:?}");
                } else {
                    assert!(matches!(error, Error::Library {
                        error: mech_engine::native_artifact::NativeCollectionInvocationError::Kernel(
                            mech_core::ResidentKernelError::IndexOutOfRange { index: 33, upper_bound: 32 }
                        ), ..
                    }), "{error:?}");
                }
                equal(&artifact, &output, &prior);
                assert_eq!(memory.used_bytes(), before);
            } else {
                let ordinary = reference(&artifact, &args);
                jit.evaluate_into(&args, &mut output, limits(), &memory)
                    .unwrap();
                equal(&artifact, &output, &ordinary);
            }
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}
