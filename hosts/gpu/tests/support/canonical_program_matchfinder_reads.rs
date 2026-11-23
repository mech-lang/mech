//! Exact byte order and whole-window zero defaults in the actual Mech helpers.
use super::super::{artifacts, equal, inputs, limits, reference};
use super::KERNEL;
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::JitProgramArtifact;

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
fn deflate_byte_prefix_and_word_reads_default_zero_for_unreadable_windows() {
    // Pinned optimized reference tests establish that ?? 0 defaults the whole
    // word, including a partial trailing window or a U64::MAX offset.
    for (helper, width) in [("byte", 1), ("load-prefix", 4), ("load-word", 8)] {
        let source = format!("{KERNEL}\n{helper}(input<[u8]>, at<u64>)\n");
        for artifact in artifacts(&source).0 {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let mut output: Vec<Option<mech_core::Value>> = vec![None; artifact.outputs().len()];
            for (length, at) in [
                (32, 0u64),
                (32, 25),
                (32, 7),
                (32, u64::MAX),
                (32, 24),
                (0, 0),
                (3, 0),
                (4, 0),
                (7, 0),
                (8, 0),
                (32, 32),
                (32, u64::MAX - 7),
                (32, 1),
            ] {
                let bytes = (0..length)
                    .map(|i| (i * 37 + 129) as u8)
                    .collect::<Vec<_>>();
                let args = inputs(
                    &artifact,
                    &[
                        (
                            "input",
                            Data::Matrix(bytes.iter().copied().map(Data::U8).collect()),
                        ),
                        ("at", Data::U64(at)),
                    ],
                    &memory,
                );
                let previous = output.clone();
                let previous_data = previous
                    .iter()
                    .map(|value| {
                        value
                            .as_ref()
                            .map(|value| value.canonical_data_draft().unwrap())
                    })
                    .collect::<Vec<_>>();
                let ordinary = reference(&artifact, &args);
                jit.evaluate_into(&args, &mut output, limits(), &memory)
                    .unwrap();
                equal(&artifact, &output, &ordinary);
                let window = usize::try_from(at).ok().and_then(|start| {
                    start
                        .checked_add(width)
                        .and_then(|end| bytes.get(start..end))
                });
                let expected = match width {
                    1 => Data::U64(window.map_or(0, |bytes| u64::from(bytes[0]))),
                    4 => Data::U32(
                        window.map_or(0, |bytes| u32::from_le_bytes(bytes.try_into().unwrap())),
                    ),
                    8 => Data::U64(
                        window.map_or(0, |bytes| u64::from_le_bytes(bytes.try_into().unwrap())),
                    ),
                    _ => unreachable!(),
                };
                assert_eq!(
                    output
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    expected,
                    "{helper}: len={length}, at={at}"
                );
                for (alias, expected) in previous.iter().zip(previous_data) {
                    if let Some(alias) = alias {
                        assert_eq!(alias.canonical_data_draft().unwrap(), expected.unwrap());
                    }
                }
            }
            drop(output);
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}
