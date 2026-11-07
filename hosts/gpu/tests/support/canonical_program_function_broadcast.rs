//! Unary statement-body functions use the same shape-preserving element lift.
use super::{artifacts, equal, inputs, limits, reference, reference_result};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::JitProgramArtifact;

#[test]
fn statement_and_expression_broadcast_preserve_live_matrix_axes_and_original_capture() {
    for definition in [
        "step(value<u64>) => (out<u64>) :=\n  out := value + bias.",
        "step(value<u64>) => <u64> := value + bias.",
    ] {
        let source = format!(
            "bias := offset<u64>\n{definition}\ncaller(values<[u64]>) => (out<[u64]>) :=\n  bias := 100u64\n  out := step(values).\ncaller(data<[u64]>)\n"
        );
        for artifact in artifacts(&source).0 {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let mut output = vec![None; artifact.outputs().len()];
            for (rows, columns, bias) in [(1, 0, 2), (2, 3, 7), (3, 1, 1), (1, 2, 9)] {
                let mut args = inputs(
                    &artifact,
                    &[
                        ("offset", Data::U64(bias)),
                        (
                            "data",
                            Data::Matrix((0..rows * columns).map(Data::U64).collect()),
                        ),
                    ],
                    &memory,
                );
                let index = artifact
                    .inputs()
                    .iter()
                    .position(|input| {
                        mech_engine::decode_source_input_name(&input.name).unwrap() == "data"
                    })
                    .unwrap();
                let schema = artifact
                    .schemas()
                    .get(artifact.inputs()[index].schema)
                    .unwrap();
                let shape =
                    mech_core::shape_for_resolved_extents(schema, &[rows, columns]).unwrap();
                let value = mech_core::ValueDraft {
                    schema: artifact.inputs()[index].schema,
                    shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
                    data: Data::Matrix((0..rows * columns).map(Data::U64).collect()),
                }
                .finalize(&mech_core::snapshot::SnapshotValidationContext::new(
                    artifact.schemas(),
                ))
                .unwrap();
                let mut claim = memory
                    .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
                    .unwrap();
                args[index] = value.into_memory_budget(&mut claim).unwrap();
                let ordinary = reference(&artifact, &args);
                jit.evaluate_into(&args, &mut output, limits(), &memory)
                    .unwrap();
                equal(&artifact, &output, &ordinary);
                let result = output.last().unwrap().as_ref().unwrap();
                assert_eq!(
                    result.canonical_data_draft().unwrap(),
                    Data::Matrix((0..rows * columns).map(|n| Data::U64(n + bias)).collect())
                );
                let mech_core::SchemaBody::Matrix { dimensions, .. } = result
                    .schemas()
                    .unwrap()
                    .get(result.schema())
                    .unwrap()
                    .closed_body(result.shape())
                    .unwrap()
                else {
                    panic!()
                };
                assert_eq!(
                    dimensions.as_ref(),
                    &[
                        mech_core::DimensionExpr::Constant(rows),
                        mech_core::DimensionExpr::Constant(columns)
                    ]
                );
            }
            drop(output);
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn statement_function_broadcast_discards_partial_narrowed_outputs_and_recovers() {
    for width in [8, 16, 32] {
        let source = format!(
            "narrow(value<u64>) => (out<u{width}>) :=\n  out<u{width}> := value.\nnarrow(values<[u64]>)\n"
        );
        for artifact in artifacts(&source).0 {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let mut output = vec![None; artifact.outputs().len()];
            let maximum = (1u64 << width) - 1;
            for values in [vec![maximum, 0], vec![1, maximum + 1], vec![], vec![3]] {
                let args = inputs(
                    &artifact,
                    &[(
                        "values",
                        Data::Matrix(values.iter().copied().map(Data::U64).collect()),
                    )],
                    &memory,
                );
                if values.contains(&(maximum + 1)) {
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
                        Data::Matrix(
                            values
                                .into_iter()
                                .map(|n| match width {
                                    8 => Data::U8(n as u8),
                                    16 => Data::U16(n as u16),
                                    32 => Data::U32(n as u32),
                                    _ => unreachable!(),
                                })
                                .collect()
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
fn statement_broadcast_keeps_unused_checked_definitions_and_skips_empty_bodies() {
    let source = "checked(value<u64>) => (out<u64>) :=\n  unused := 10u64 / value\n  out := 7u64.\nchecked(values<[u64]>)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for values in [vec![1u64, 2], vec![3, 0], vec![], vec![4]] {
            let args = inputs(
                &artifact,
                &[(
                    "values",
                    Data::Matrix(values.iter().copied().map(Data::U64).collect()),
                )],
                &memory,
            );
            if values.contains(&0) {
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
                assert!(
                    matches!(
                        error,
                        mech_gpu::native_artifact::NativeProgramExecutionError::Control(
                            mech_gpu::native_artifact::NativeIterationExecutionError::Arithmetic { .. }
                        )
                    ),
                    "{error:?}"
                );
                equal(&artifact, &output, &prior);
                assert_eq!(memory.used_bytes(), before);
            } else {
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
                    Data::Matrix(vec![Data::U64(7); values.len()].into())
                );
            }
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}
