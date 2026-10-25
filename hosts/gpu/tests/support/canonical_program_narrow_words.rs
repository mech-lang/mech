//! Byte/word buffers keep exact widths while scalar reads/casts stay native.
use super::{artifacts, equal, inputs, limits, reference, reference_result};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::JitProgramArtifact;

fn word(width: u32, value: u64) -> Data {
    match width {
        8 => Data::U8(value.try_into().unwrap()),
        16 => Data::U16(value.try_into().unwrap()),
        32 => Data::U32(value.try_into().unwrap()),
        _ => unreachable!(),
    }
}
fn matrix(width: u32, values: &[u64]) -> Data {
    Data::Matrix(
        values
            .iter()
            .map(|&value| word(width, value))
            .collect::<Vec<_>>()
            .into(),
    )
}

#[test]
fn narrow_comparisons_preserve_unsigned_boundaries_and_buffer_reads() {
    for width in [8, 16, 32] {
        let maximum = (1u64 << width) - 1;
        let high_bit = 1u64 << (width - 1);
        let cases = [
            (0, 0),
            (0, maximum),
            (maximum, 0),
            (maximum, maximum),
            (high_bit - 1, high_bit),
            (high_bit, high_bit - 1),
        ];
        for op in ["==", "!=", "<", "<=", ">", ">="] {
            for buffered in [false, true] {
                let source = if buffered {
                    format!("values := words<[u{width}]>\nvalues[1u64] {op} values[2u64]\n")
                } else {
                    format!("left<u{width}> {op} right<u{width}>\n")
                };
                let (artifacts, last) = artifacts(&source);
                for artifact in artifacts {
                    let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
                    let mut args_and_expected = Vec::new();
                    // Establish resident correctness before attempting native lowering.
                    for (left, right) in cases {
                        let values = if buffered {
                            vec![("words", matrix(width, &[left, right]))]
                        } else {
                            vec![("left", word(width, left)), ("right", word(width, right))]
                        };
                        let args = inputs(&artifact, &values, &memory);
                        let expected = match op {
                            "==" => left == right,
                            "!=" => left != right,
                            "<" => left < right,
                            "<=" => left <= right,
                            ">" => left > right,
                            ">=" => left >= right,
                            _ => unreachable!(),
                        };
                        let ordinary = reference(&artifact, &args);
                        assert_eq!(
                            ordinary[last].canonical_data_draft().unwrap(),
                            Data::Bool(expected)
                        );
                        args_and_expected.push((args, ordinary));
                    }
                    let jit =
                        JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog())
                            .unwrap_or_else(|error| {
                                panic!("u{width} {op}, buffered={buffered}: {error:?}")
                            });
                    let mut output = vec![None; artifact.outputs().len()];
                    for (args, ordinary) in &args_and_expected {
                        jit.evaluate_into(args, &mut output, limits(), &memory)
                            .unwrap();
                        equal(&artifact, &output, ordinary);
                    }
                    drop((output, args_and_expected));
                    assert_eq!(memory.used_bytes(), 0);
                }
            }
        }
    }
}

#[test]
fn narrow_comparison_assertions_keep_error_order_publication_and_retry() {
    for width in [8, 16, 32] {
        let source = format!(
            "check(left<u{width}>, right<u{width}>, divisor<u64>) => (out<u64>) :=\n  ordered! := left <= right\n  out := 12u64 / divisor.\ncheck(left<u{width}>, right<u{width}>, divisor<u64>)\n"
        );
        let (artifacts, last) = artifacts(&source);
        for artifact in artifacts {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let mut output: Vec<Option<mech_core::Value>> = vec![None; artifact.outputs().len()];
            let maximum = (1u64 << width) - 1;
            for (left, right, divisor) in [(maximum, maximum, 3), (maximum, 0, 0), (0, maximum, 2)]
            {
                let args = inputs(
                    &artifact,
                    &[
                        ("left", word(width, left)),
                        ("right", word(width, right)),
                        ("divisor", Data::U64(divisor)),
                    ],
                    &memory,
                );
                if left > right {
                    assert!(reference_result(&artifact, &args).unwrap_err().contains("LexicalIntegrity"));
                    let previous = output
                        .iter()
                        .map(|value| value.clone().unwrap())
                        .collect::<Vec<_>>();
                    let retained = memory.used_bytes();
                    let error = jit
                        .evaluate_into(&args, &mut output, limits(), &memory)
                        .unwrap_err();
                    assert!(matches!(error,
                        mech_gpu::native_artifact::NativeProgramExecutionError::Control(
                            mech_gpu::native_artifact::NativeIterationExecutionError::Integrity { .. }
                        )), "{error:?}");
                    equal(&artifact, &output, &previous);
                    assert_eq!(memory.used_bytes(), retained);
                } else {
                    let ordinary = reference(&artifact, &args);
                    jit.evaluate_into(&args, &mut output, limits(), &memory)
                        .unwrap();
                    equal(&artifact, &output, &ordinary);
                    assert_eq!(
                        output[last]
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        Data::U64(12 / divisor)
                    );
                }
            }
            drop(output);
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn narrow_reads_and_widening_execute_with_zero_snapshot_clones() {
    for width in [8, 16, 32] {
        for source in [
            format!("buffer := values<[u{width}]>\nresult<u64> := buffer[position<u64>]\nresult\n"),
            format!(
                "read(xs<[u{width}]>, at<u64>) => (out<u64>) :=\n  out<u64> := xs[at].\nread(values<[u{width}]>, position<u64>)\n"
            ),
        ] {
            let (artifacts, last) = artifacts(&source);
            for artifact in artifacts {
                let jit =
                    JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
                let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
                let mut output = vec![None; artifact.outputs().len()];
                let maximum = (1u64 << width) - 1;
                let values = [0, maximum, 1, maximum / 2 + 1];
                let mut policy = limits();
                policy.library.max_cloned_bytes = Some(0);
                policy.library.max_output_bytes = Some(u64::from(width / 8));
                for position in [2, 4, 1, 3, 2] {
                    let args = inputs(
                        &artifact,
                        &[
                            ("values", matrix(width, &values)),
                            ("position", Data::U64(position)),
                        ],
                        &memory,
                    );
                    let expected = reference(&artifact, &args);
                    jit.evaluate_into(&args, &mut output, policy, &memory)
                        .unwrap();
                    equal(&artifact, &output, &expected);
                    assert_eq!(
                        output[last]
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        Data::U64(values[position as usize - 1])
                    );
                }
                drop(output);
                assert_eq!(memory.used_bytes(), 0);
            }
        }
    }
}

#[test]
fn narrow_generator_yields_and_checked_late_cast_errors_preserve_publication() {
    for width in [8, 16, 32] {
        let source = format!(
            "increment(value<u{width}>) => (out<u{width}>) :=\n  wide<u64> := value\n  out<u{width}> := wide + 1u64.\n[increment(item) | item <- values<[u{width}]>]\n"
        );
        let (artifacts, last) = artifacts(&source);
        for artifact in artifacts {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let mut output = vec![None; artifact.outputs().len()];
            let maximum = (1u64 << width) - 1;
            for values in [
                vec![0, 1, 2],
                vec![1, maximum],
                vec![3],
                vec![],
                vec![maximum - 1],
            ] {
                let args = inputs(&artifact, &[("values", matrix(width, &values))], &memory);
                if values.contains(&maximum) {
                    assert!(
                        reference_result(&artifact, &args)
                            .unwrap_err()
                            .contains("Arithmetic")
                    );
                    let prior = output
                        .iter()
                        .map(|value| value.clone().unwrap())
                        .collect::<Vec<_>>();
                    let before = memory.used_bytes();
                    let error = jit
                        .evaluate_into(&args, &mut output, limits(), &memory)
                        .unwrap_err();
                    assert!(matches!(error, mech_gpu::native_artifact::NativeProgramExecutionError::Control(
                        mech_gpu::native_artifact::NativeIterationExecutionError::Arithmetic { .. }
                    )), "{error:?}");
                    equal(&artifact, &output, &prior);
                    assert_eq!(memory.used_bytes(), before);
                } else {
                    let expected = reference(&artifact, &args);
                    jit.evaluate_into(&args, &mut output, limits(), &memory)
                        .unwrap();
                    equal(&artifact, &output, &expected);
                    assert_eq!(
                        output[last]
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        matrix(
                            width,
                            &values.iter().map(|value| value + 1).collect::<Vec<_>>()
                        )
                    );
                }
            }
            drop(output);
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn narrow_selectors_keep_the_ordinary_indexed_access_fallback() {
    for width in [8, 16, 32] {
        let source = format!("buffer := values<[u{width}]>\nbuffer[position<u{width}>]\n");
        let (artifacts, last) = artifacts(&source);
        for artifact in artifacts {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let args = inputs(
                &artifact,
                &[
                    ("values", matrix(width, &[17, 23])),
                    ("position", word(width, 2)),
                ],
                &memory,
            );
            let mut output = vec![None; artifact.outputs().len()];
            let expected = reference(&artifact, &args);
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &expected);
            assert_eq!(
                output[last]
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                word(width, 23)
            );
        }
    }
}
