//! Real unsigned word conversions selected from the maintained checked leaf.
use super::{artifacts, equal, inputs, limits, reference, reference_result};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_engine::native_artifact::{
    NativeArtifactLoweringError, NativeIterationAction, NativeMatchAction, NativeScalarOp,
    lower_native_program_artifact,
};
use mech_gpu::native_artifact::{
    JitProgramArtifact, NativeProgramDescriptor, NativeProgramExecutionError,
};

fn maximum(width: u32) -> u64 {
    if width == 64 {
        u64::MAX
    } else {
        (1u64 << width) - 1
    }
}
fn word(width: u32, value: u64) -> Option<Data> {
    Some(match width {
        8 => Data::U8(u8::try_from(value).ok()?),
        16 => Data::U16(u16::try_from(value).ok()?),
        32 => Data::U32(u32::try_from(value).ok()?),
        64 => Data::U64(value),
        _ => unreachable!(),
    })
}

fn expected_cast(from: u32, to: u32) -> NativeScalarOp {
    match (from, to) {
        (8, 16) => NativeScalarOp::U8ToU16,
        (8, 32) => NativeScalarOp::U8ToU32,
        (8, 64) => NativeScalarOp::U8ToU64,
        (16, 8) => NativeScalarOp::U16ToU8,
        (16, 32) => NativeScalarOp::U16ToU32,
        (16, 64) => NativeScalarOp::U16ToU64,
        (32, 8) => NativeScalarOp::U32ToU8,
        (32, 16) => NativeScalarOp::U32ToU16,
        (32, 64) => NativeScalarOp::U32ToU64,
        (64, 8) => NativeScalarOp::U64ToU8,
        (64, 16) => NativeScalarOp::U64ToU16,
        (64, 32) => NativeScalarOp::U64ToU32,
        _ => unreachable!(),
    }
}

#[test]
fn unsigned_word_conversions_preserve_boundaries_failures_and_image_metadata() {
    for from in [8, 16, 32, 64] {
        for to in [8, 16, 32, 64] {
            if from == to {
                continue;
            }
            for source in [
                format!("converted<u{to}> := input<u{from}>\nconverted\n"),
                format!(
                    "convert-word(value<u{from}>) => (out<u{to}>) :=\n  converted<u{to}> := value\n  out := converted.\nconvert-word(input<u{from}>)\n"
                ),
            ] {
                for artifact in artifacts(&source).0 {
                    let catalog = mech_stdlib::source_catalog();
                    let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
                    assert!(jit.descriptor().plan().blocks().iter().any(|block| matches!(
                        block.action(), NativeIterationAction::Scalar(NativeMatchAction::Primitive { op, .. })
                        if *op == expected_cast(from, to)
                    )));
                    let encoded = jit.descriptor().encode_build_owned_image().unwrap();
                    // SAFETY: this checked fixture's unchanged image is inspected
                    // only; it is never paired with or used to execute code.
                    let restored = unsafe {
                        NativeProgramDescriptor::decode_build_owned_image(&encoded, &catalog)
                    }
                    .unwrap();
                    assert_eq!(restored.encode_build_owned_image().unwrap(), encoded);
                    let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
                    let mut output = vec![None; artifact.outputs().len()];
                    let boundary = maximum(from).min(maximum(to));
                    let mut values = vec![0, maximum(from), 1, boundary, boundary - 1, 0];
                    if from > to {
                        values.insert(2, maximum(to) + 1);
                    }
                    for value in values {
                        let args =
                            inputs(&artifact, &[("input", word(from, value).unwrap())], &memory);
                        let prior = output.clone();
                        if let Some(expected) = word(to, value) {
                            let ordinary = reference(&artifact, &args);
                            jit.evaluate_into(&args, &mut output, limits(), &memory)
                                .unwrap();
                            equal(&artifact, &output, &ordinary);
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
                        } else {
                            assert!(reference_result(&artifact, &args).is_err());
                            let before = memory.used_bytes();
                            assert!(matches!(
                                jit.evaluate_into(&args, &mut output, limits(), &memory),
                                Err(NativeProgramExecutionError::Control(
                                    mech_gpu::native_artifact::NativeIterationExecutionError::Arithmetic { .. }
                                ))
                            ));
                            for (next, prior) in output.iter().zip(&prior) {
                                assert!(
                                    next.as_ref()
                                        .unwrap()
                                        .snapshot_eq(
                                            artifact.schemas(),
                                            prior.as_ref().unwrap(),
                                            artifact.schemas()
                                        )
                                        .unwrap()
                                );
                            }
                            assert_eq!(memory.used_bytes(), before);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn unsigned_conversion_does_not_replace_existing_direct_scalar_casts() {
    for (source, expected) in [
        (
            "converted<i64> := input<u64>\nconverted\n",
            NativeScalarOp::U64ToI64,
        ),
        (
            "converted<u64> := input<i64>\nconverted\n",
            NativeScalarOp::I64ToU64,
        ),
        (
            "converted<f64> := input<u64>\nconverted\n",
            NativeScalarOp::U64ToF64,
        ),
        (
            "converted<f64> := input<i64>\nconverted\n",
            NativeScalarOp::I64ToF64,
        ),
        (
            "converted<u64> := input<f64>\nconverted\n",
            NativeScalarOp::F64ToU64,
        ),
        (
            "converted<i64> := input<f64>\nconverted\n",
            NativeScalarOp::F64ToI64,
        ),
    ] {
        for artifact in artifacts(source).0 {
            let plan =
                lower_native_program_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert!(plan.blocks().iter().any(|block| matches!(block.action(),
                NativeIterationAction::Scalar(NativeMatchAction::Primitive { op, .. })
                if *op == expected)));
        }
    }
}

#[test]
fn unsigned_byte_words_feed_native_bitwise_operations_and_bounds_recover() {
    let mut source = String::from("+> bits\nread-word(bytes<[u8]>, at<u64>) => (out<u64>) :=\n");
    for index in 0..8 {
        source.push_str(&format!("  b{index}<u64> := bytes[at + {index}u64]\n"));
    }
    let mut result = String::from("b0");
    for index in 1..8 {
        result = format!(
            "bits/or({result}, bits/shift-left-wrap(b{index}, {}u64))",
            index * 8
        );
    }
    source.push_str(&format!(
        "  out := {result}.\nread-word(input<[u8]>, position<u64>)\n"
    ));
    let bytes = [0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef, 0xfe, 0xdc];
    for artifact in artifacts(&source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for position in [1u64, 3, 0, 4, 2, 1] {
            let args = inputs(
                &artifact,
                &[
                    (
                        "input",
                        Data::Matrix(bytes.iter().copied().map(Data::U8).collect()),
                    ),
                    ("position", Data::U64(position)),
                ],
                &memory,
            );
            let prior = output.clone();
            if (1..=3).contains(&position) {
                jit.evaluate_into(&args, &mut output, limits(), &memory)
                    .unwrap();
                equal(&artifact, &output, &reference(&artifact, &args));
                let start = position as usize - 1;
                let expected = u64::from_le_bytes(bytes[start..start + 8].try_into().unwrap());
                assert_eq!(
                    output
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::U64(expected)
                );
            } else {
                assert!(reference_result(&artifact, &args).is_err());
                let before = memory.used_bytes();
                assert!(
                    jit.evaluate_into(&args, &mut output, limits(), &memory)
                        .is_err()
                );
                for (next, prior) in output.iter().zip(&prior) {
                    assert!(
                        next.as_ref()
                            .unwrap()
                            .snapshot_eq(
                                artifact.schemas(),
                                prior.as_ref().unwrap(),
                                artifact.schemas()
                            )
                            .unwrap()
                    );
                }
                assert_eq!(memory.used_bytes(), before);
            }
        }
    }
}

#[test]
fn a_replacement_conversion_factory_cannot_obtain_the_builtin_witness() {
    let mut builder = mech_core::FunctionCatalogBuilder::new();
    builder
        .insert_resident_factory(
            ["convert"],
            "kind",
            mech_core::ImplementationMemoryClass::CanonicalFinalize,
            |_| panic!("native metadata validation must not execute a replacement factory"),
        )
        .unwrap();
    let catalog = builder.build().unwrap();
    for artifact in artifacts("converted<u64> := input<u8>\nconverted\n").0 {
        assert!(matches!(
            lower_native_program_artifact(&artifact, &catalog),
            Err(NativeArtifactLoweringError::MissingSemanticWitness { .. })
        ));
    }
}
