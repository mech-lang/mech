//! Ordinary recursive numeric bodies remain local to each mapped worker.
use super::*;
use crate::native_artifact::program::runtime::invocation::parallel::TEST_WORKERS;

const NUMERIC: &str = r#"+> sequence
many(data<[u64]>, at<u64>, end<u64>, output<[u64]>) => (out<[u64]>) :=
  more := at < end
  next := (more? | true => sequence/append(output, data[at + 1u64] + 1u64) | false => output)
  out := (more? | true => many(data, at + 1u64, end, next) | false => output).
piece(tables<([u64],(u64,[u8]))>, chunk<u64>, count<u64>) => (out<[u64]>) :=
  start := count * chunk / 8u64
  end := count * (chunk + 1u64) / 8u64
  data := tables.1
  out := many(data, start, end, sequence/with-capacity(0u64, end - start)).
data := values<[u64]>
tables := (data, (1u64, [3u8 5u8]))
chunks := [0u64 1u64 2u64 3u64 4u64 5u64 6u64 7u64]
count := size<u64>
out<[u64]> := [piece(tables, chunk, count) | chunk <- chunks]
out
"#;

fn numeric_args(
    artifact: &mech_engine::ProgramArtifact,
    memory: &ManagedMemoryBudget,
    data: &[u64],
    count: u64,
) -> Vec<Value> {
    artifact
        .inputs()
        .iter()
        .map(|port| {
            let name = mech_engine::decode_source_input_name(&port.name).unwrap();
            let (shape_values, data) = match name.as_str() {
                "values" => (
                    mech_core::shape_for_resolved_extents(
                        artifact.schemas().get(port.schema).unwrap(),
                        &[1, data.len() as u64],
                    )
                    .unwrap()
                    .parameter_values()
                    .into(),
                    ValueDataDraft::Matrix(data.iter().copied().map(ValueDataDraft::U64).collect()),
                ),
                "size" => (Box::new([]) as Box<[u64]>, ValueDataDraft::U64(count)),
                _ => panic!("unexpected argument {name}"),
            };
            let value = ValueDraft {
                schema: port.schema,
                shape_values,
                data,
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let mut reservation = memory
                .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
                .unwrap();
            value.into_memory_budget(&mut reservation).unwrap()
        })
        .collect()
}

#[test]
fn numeric_workers_keep_uneven_partitions_native_calls_errors_aliases_and_restore() {
    let original = crate::native_artifact::program::runtime::tests::artifact_source(NUMERIC);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    for artifact in [original, decoded] {
        let mode = crate::native_artifact::NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let jit = crate::native_artifact::JitProgramArtifact::compile_with_limits(
            &artifact,
            &catalog,
            mode.compilation(),
        )
        .unwrap();
        assert_eq!(jit.descriptor().metadata().parallel.regions.len(), 1);
        let object = crate::native_artifact::emit_program_object_with_limits(
            &artifact,
            &catalog,
            mode.compilation(),
        )
        .unwrap();
        assert!(
            !object.metadata()["compiled_numeric_functions"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        let image = jit.descriptor().encode_build_owned_image().unwrap();
        // SAFETY: the retained JIT entry and this unchanged image have one build owner.
        let restored =
            unsafe { NativeProgramDescriptor::decode_build_owned_image(&image, &catalog) }.unwrap();
        for descriptor in [jit.descriptor(), &restored] {
            assert!(
                descriptor
                    .metadata()
                    .worker_value_prefixes
                    .iter()
                    .all(|&prefix| prefix < descriptor.metadata().types.len()),
                "the existing original/decoded/restored numeric error/alias/retry contract must exercise physical worker prefixes"
            );
            let memory = mode.memory_budget();
            let mut outputs = vec![None; artifact.outputs().len()];
            for (data, count, accepts) in [
                (vec![], 0, true),
                (vec![2], 1, true),
                ((0..19).collect::<Vec<_>>(), 19, true),
                ((0..19).collect::<Vec<_>>(), 8, true),
                (vec![2, 3, 5], 4, false),
                (vec![2, u64::MAX, 5], 3, false),
                (vec![13, 17, 19], 3, true),
            ] {
                let args = numeric_args(&artifact, &memory, &data, count);
                let aliases = args.clone();
                let before = args
                    .iter()
                    .map(|v| v.canonical_data_draft().unwrap())
                    .collect::<Vec<_>>();
                let mut serial = None;
                for workers in [1, 8] {
                    let prior = outputs.clone();
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let old = TEST_WORKERS.with(|setting| setting.replace(Some(workers)));
                    let result = execute(
                        descriptor,
                        |frame| unsafe { jit.test_checked_entry()(frame) },
                        &args,
                        &mut outputs,
                        mode.execution(),
                        &memory,
                    );
                    TEST_WORKERS.with(|setting| setting.set(old));
                    assert_eq!(
                        result.is_ok(),
                        accepts,
                        "{result:?} workers={workers} count={count}"
                    );
                    if let Ok(report) = result {
                        assert_eq!(report.parallel_tasks, if workers == 8 { 8 } else { 0 });
                        if workers == 8 {
                            assert!(report.parallel_numeric_regions >= 8);
                            // Empty partitions may take the checked zero-fuel
                            // terminal edge without executing a numeric body.
                            assert!(
                                count == 0 || report.parallel_numeric_work > 0,
                                "count={count}"
                            );
                        } else {
                            assert_eq!(
                                (
                                    report.parallel_numeric_regions,
                                    report.parallel_numeric_work
                                ),
                                (0, 0)
                            );
                        }
                        let value = outputs
                            .last()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap();
                        assert_eq!(
                            value,
                            ValueDataDraft::Matrix(
                                data[..count as usize]
                                    .iter()
                                    .map(|x| ValueDataDraft::U64(x + 1))
                                    .collect()
                            )
                        );
                        if let Some(serial) = &serial {
                            assert_eq!(&value, serial);
                        }
                        serial = Some(value);
                    } else {
                        assert_eq!(
                            outputs
                                .iter()
                                .map(|v| v.as_ref().map(|v| v.canonical_data_draft().unwrap()))
                                .collect::<Vec<_>>(),
                            prior
                                .iter()
                                .map(|v| v.as_ref().map(|v| v.canonical_data_draft().unwrap()))
                                .collect::<Vec<_>>()
                        );
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            retained
                        );
                    }
                    assert_eq!(
                        aliases
                            .iter()
                            .map(|v| v.canonical_data_draft().unwrap())
                            .collect::<Vec<_>>(),
                        before
                    );
                }
                drop((args, aliases));
            }
            drop(outputs);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn native_row_blocks_preserve_scalar_kinds_empty_shapes_and_atomic_output_limits() {
    let mode = crate::native_artifact::NativeProgramLimitMode::NoLimits;
    let f = |x| ValueDataDraft::F64(F64Bits::from_f64(x));
    for (literal, elements) in [
        (
            "[true false]",
            vec![ValueDataDraft::Bool(true), ValueDataDraft::Bool(false)],
        ),
        (
            "[2u8 7u8]",
            vec![ValueDataDraft::U8(2), ValueDataDraft::U8(7)],
        ),
        (
            "[2u16 513u16]",
            vec![ValueDataDraft::U16(2), ValueDataDraft::U16(513)],
        ),
        (
            "[2u32 70000u32]",
            vec![ValueDataDraft::U32(2), ValueDataDraft::U32(70000)],
        ),
        (
            "[-2i64 7i64]",
            vec![ValueDataDraft::I64(-2), ValueDataDraft::I64(7)],
        ),
        ("[1.25 -2.5]", vec![f(1.25), f(-2.5)]),
        (
            "[\"é\" \"界🛰\"]",
            vec![
                ValueDataDraft::String("é".into()),
                ValueDataDraft::String("界🛰".into()),
            ],
        ),
    ] {
        let source = format!("+> sequence\nchoices := items<[u64]>\n[{literal} | * <- choices]\n");
        let original = crate::native_artifact::program::runtime::tests::artifact_source(&source);
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
        )
        .unwrap();
        for artifact in [original, decoded] {
            let jit = crate::native_artifact::JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            let memory = mode.memory_budget();
            let mut outputs = vec![None; artifact.outputs().len()];
            for count in [0, 1, 3] {
                let value = ValueDraft {
                    schema: artifact.inputs()[0].schema,
                    shape_values: mech_core::shape_for_resolved_extents(
                        artifact.schemas().get(artifact.inputs()[0].schema).unwrap(),
                        &[1, count],
                    )
                    .unwrap()
                    .parameter_values()
                    .into(),
                    data: ValueDataDraft::Matrix((0..count).map(ValueDataDraft::U64).collect()),
                }
                .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                .unwrap();
                let mut admission = memory
                    .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
                    .unwrap();
                let args = [value.into_memory_budget(&mut admission).unwrap()];
                drop(admission);
                jit.evaluate_into(&args, &mut outputs, mode.execution(), &memory)
                    .unwrap_or_else(|error| panic!("{source} count={count}: {error:?}"));
                let expected =
                    ValueDataDraft::Matrix((0..count).flat_map(|_| elements.clone()).collect());
                assert_eq!(
                    outputs
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    expected,
                    "{source}"
                );
                if count != 0 {
                    let prior = outputs.clone();
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let mut limits = mode.execution();
                    limits.library.max_output_elements = Some(2 * count - 1);
                    assert!(
                        jit.evaluate_into(&args, &mut outputs, limits, &memory)
                            .is_err()
                    );
                    for (got, old) in outputs.iter().zip(&prior) {
                        assert_eq!(
                            got.as_ref().unwrap().canonical_data_draft().unwrap(),
                            old.as_ref().unwrap().canonical_data_draft().unwrap()
                        );
                    }
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                }
            }
            drop(outputs);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
