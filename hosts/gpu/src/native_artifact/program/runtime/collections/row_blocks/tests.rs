//! Original/decoded/restored source execution against the maintained scan.
use super::*;
use crate::native_artifact::{JitProgramArtifact, NativeProgramLimitMode};

struct Generic(bool);
impl Generic {
    fn set(value: bool) -> Self {
        Self(FORCE_GENERIC_PAYLOAD.with(|flag| flag.replace(value)))
    }
}
impl Drop for Generic {
    fn drop(&mut self) {
        FORCE_GENERIC_PAYLOAD.with(|flag| flag.set(self.0));
    }
}
fn artifacts(source: &str) -> [mech_engine::ProgramArtifact; 2] {
    let original = crate::native_artifact::program::runtime::tests::artifact_source(source);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    [original, decoded]
}
fn inputs(
    artifact: &mech_engine::ProgramArtifact,
    memory: &ManagedMemoryBudget,
    count: u64,
    row: Option<(u64, &[ValueDataDraft])>,
    denominator: u64,
) -> Vec<Value> {
    artifact
        .inputs()
        .iter()
        .map(|port| {
            let name = mech_engine::decode_source_input_name(&port.name).unwrap();
            let value = if name == "divisor" {
                ValueDraft {
                    schema: port.schema,
                    shape_values: Box::new([]),
                    data: ValueDataDraft::U64(denominator),
                }
                .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                .unwrap()
            } else {
                let (rows, columns, values) = match name.as_str() {
                    "items" => (
                        1,
                        count,
                        (0..count).map(ValueDataDraft::U64).collect::<Vec<_>>(),
                    ),
                    "row" => {
                        let (rows, values) = row.unwrap();
                        (
                            rows,
                            if rows == 0 {
                                0
                            } else {
                                values.len() as u64 / rows
                            },
                            values.to_vec(),
                        )
                    }
                    _ => panic!("unexpected input {name}"),
                };
                let schema = artifact.schemas().get(port.schema).unwrap();
                ValueDraft {
                    schema: port.schema,
                    shape_values: mech_core::shape_for_resolved_extents(schema, &[rows, columns])
                        .unwrap()
                        .parameter_values()
                        .into(),
                    data: ValueDataDraft::Matrix(values.into_boxed_slice()),
                }
                .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                .unwrap()
            };
            let mut reservation = memory
                .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
                .unwrap();
            value.into_memory_budget(&mut reservation).unwrap()
        })
        .collect()
}
fn descriptors(jit: &JitProgramArtifact) -> [NativeProgramDescriptor; 1] {
    [crate::native_artifact::program::runtime::tests::restore_admitted_helper_descriptor(jit)]
}
fn same_report(
    mut left: crate::native_artifact::program::runtime::NativeProgramExecutionReport,
    mut right: crate::native_artifact::program::runtime::NativeProgramExecutionReport,
) {
    // Scheduling overlap is not a source/resource price. All work, visits,
    // transport, credit claims and native-region fields must stay exact.
    left.parallel_peak_workers = 0;
    right.parallel_peak_workers = 0;
    assert_eq!(left, right);
}

#[test]
fn packed_row_payload_keeps_all_scalar_kinds_unicode_empty_shapes_and_exact_prices() {
    let mode = NativeProgramLimitMode::NoLimits;
    for (literal, packed) in [
        ("[true false]", true),
        ("[0u8 255u8]", true),
        ("[0u16 65535u16]", true),
        ("[0u32 4294967295u32]", true),
        ("[0u64 18446744073709551615u64]", true),
        ("[-2i64 7i64]", true),
        ("[0.0 -2.5]", true),
        ("[\"é\" \"界🛰\"]", false),
    ] {
        let source = format!("+> sequence\nchoices := items<[u64]>\n[{literal} | * <- choices]\n");
        for artifact in artifacts(&source) {
            let jit = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            for image in descriptors(&jit) {
                for descriptor in [jit.descriptor(), &image] {
                    for count in [0, 1, 3] {
                        let mut outcomes = Vec::new();
                        for generic in [true, false] {
                            let _strategy = Generic::set(generic);
                            PACKED_PAYLOADS.with(|n| n.set(0));
                            let memory = mode.memory_budget();
                            let args = inputs(&artifact, &memory, count, None, 1);
                            let aliases = args.clone();
                            let mut output = vec![None; artifact.outputs().len()];
                            let report = crate::native_artifact::program::runtime::execute(
                                descriptor,
                                jit.test_generated_entry(),
                                &args,
                                &mut output,
                                mode.execution(),
                                &memory,
                            )
                            .unwrap();
                            let value = output
                                .last()
                                .unwrap()
                                .as_ref()
                                .unwrap()
                                .canonical_data_draft()
                                .unwrap();
                            let used = (memory.used_bytes(), memory.retained_snapshot_nodes());
                            assert!(
                                args.iter()
                                    .zip(&aliases)
                                    .all(|(a, b)| a.shares_frozen_storage(b))
                            );
                            assert_eq!(
                                PACKED_PAYLOADS.with(core::cell::Cell::get) > 0,
                                !generic && packed && count > 0,
                                "{source}, count={count}"
                            );
                            outcomes.push((report, value, used));
                            drop((args, aliases, output));
                            assert_eq!(
                                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                                (0, 0)
                            );
                        }
                        same_report(outcomes[0].0, outcomes[1].0);
                        assert_eq!(outcomes[0].1, outcomes[1].1);
                        assert_eq!(outcomes[0].2, outcomes[1].2);
                    }
                }
            }
        }
    }
}

// The measured worker returns a row produced by sequence operations. General
// multirow horizontal concatenation also permutes cells and is a separate,
// currently unadmitted native family; it is not a one-row error fixture.
// Maintained sequence/append itself requires exactly one row when visited.
const DYNAMIC: &str = "+> sequence\nchoices := items<[u64]>\n[sequence/append(row<[u64]>, 12u64 / divisor<u64>) | * <- choices]\n";

#[test]
fn packed_row_payload_preserves_large_borrowed_rows_shapes_unvisited_errors_and_aliases() {
    let mode = NativeProgramLimitMode::NoLimits;
    for artifact in artifacts(DYNAMIC) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        for image in descriptors(&jit) {
            for descriptor in [jit.descriptor(), &image] {
                for (rows, columns, count, denominator) in [
                    (1, 0, 3, 2),
                    (1, 1, 1, 2),
                    (1, 4097, 4, 2),
                    (2, 2, 0, 0),
                    (2, 2, 3, 2),
                    (0, 0, 0, 0),
                    (0, 0, 2, 2),
                    (1, 2, 3, 0),
                ] {
                    let values: Vec<_> = (0..rows * columns)
                        .map(|i| ValueDataDraft::U64(u64::MAX - i))
                        .collect();
                    let mut outcomes = Vec::new();
                    for generic in [true, false] {
                        let _strategy = Generic::set(generic);
                        let memory = mode.memory_budget();
                        let args = inputs(
                            &artifact,
                            &memory,
                            count,
                            Some((rows, &values)),
                            denominator,
                        );
                        let before: Vec<_> = args
                            .iter()
                            .map(|x| x.canonical_data_draft().unwrap())
                            .collect();
                        let aliases = args.clone();
                        let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
                        let mut output = vec![None; artifact.outputs().len()];
                        let result = crate::native_artifact::program::runtime::execute(
                            descriptor,
                            jit.test_generated_entry(),
                            &args,
                            &mut output,
                            mode.execution(),
                            &memory,
                        );
                        let outcome = match result {
                            Ok(report) => {
                                assert!(
                                    (denominator != 0 && rows == 1) || count == 0,
                                    "visited append requires one row and a nonzero divisor"
                                );
                                let data = output
                                    .last()
                                    .unwrap()
                                    .as_ref()
                                    .unwrap()
                                    .canonical_data_draft()
                                    .unwrap();
                                assert_eq!(
                                    data,
                                    ValueDataDraft::Matrix(
                                        (0..count)
                                            .flat_map(|_| values.iter().cloned().chain(
                                                core::iter::once(ValueDataDraft::U64(
                                                    12 / denominator
                                                ))
                                            ))
                                            .collect()
                                    )
                                );
                                Ok((
                                    report,
                                    data,
                                    memory.used_bytes(),
                                    memory.retained_snapshot_nodes(),
                                ))
                            }
                            Err(error) => {
                                assert!(
                                    count > 0 && (denominator == 0 || rows != 1),
                                    "rows={rows} columns={columns} count={count} divisor={denominator}: {error:?}"
                                );
                                assert!(output.iter().all(Option::is_none));
                                assert_eq!(
                                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                                    held
                                );
                                Err(format!("{error:?}"))
                            }
                        };
                        outcomes.push(outcome);
                        assert!(
                            args.iter()
                                .zip(&aliases)
                                .all(|(a, b)| a.shares_frozen_storage(b))
                        );
                        assert_eq!(
                            args.iter()
                                .map(|x| x.canonical_data_draft().unwrap())
                                .collect::<Vec<_>>(),
                            before
                        );
                        drop((args, aliases, output));
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            (0, 0)
                        );
                    }
                    match (&outcomes[0], &outcomes[1]) {
                        (Ok(a), Ok(b)) => {
                            same_report(a.0, b.0);
                            assert_eq!((&a.1, a.2, a.3), (&b.1, b.2, b.3));
                        }
                        (Err(a), Err(b)) => assert_eq!(a, b),
                        _ => panic!("packed strategy changed a source outcome"),
                    }
                }
            }
        }
    }
}

#[test]
fn packed_row_payload_retains_finite_denials_atomic_publication_and_recovery() {
    let mode = NativeProgramLimitMode::NoLimits;
    let values: Vec<_> = (0..257).map(ValueDataDraft::U64).collect();
    for artifact in artifacts(DYNAMIC) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        for image in descriptors(&jit) {
            for descriptor in [jit.descriptor(), &image] {
                let mut results = Vec::new();
                for generic in [true, false] {
                    let _strategy = Generic::set(generic);
                    let memory = mode.memory_budget();
                    let args = inputs(&artifact, &memory, 3, Some((1, &values)), 2);
                    let aliases = args.clone();
                    let mut output = vec![None; artifact.outputs().len()];
                    let first = crate::native_artifact::program::runtime::execute(
                        descriptor,
                        jit.test_generated_entry(),
                        &args,
                        &mut output,
                        mode.execution(),
                        &memory,
                    )
                    .unwrap();
                    let mut outcomes = Vec::new();
                    for denial in 0..15 {
                        let prior = output.clone();
                        let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
                        let mut limits = mode.execution();
                        match denial {
                            0 => limits.execution.max_work = 0,
                            1 => limits.library.max_output_elements = Some(0),
                            2 => limits.library.max_output_bytes = Some(0),
                            3 => limits.library.max_temporary_bytes = Some(0),
                            4 => limits.library.max_cloned_bytes = Some(0),
                            5 => limits.library.max_retained_nodes = Some(0),
                            6 => limits.library.max_compute_work = Some(0),
                            7 => limits.library.max_comparison_work = Some(0),
                            8 => limits.library.max_output_elements = None,
                            9 => limits.library.max_output_bytes = None,
                            10 => limits.library.max_temporary_bytes = None,
                            11 => limits.library.max_cloned_bytes = None,
                            12 => limits.library.max_retained_nodes = None,
                            13 => limits.library.max_compute_work = None,
                            _ => limits.library.max_comparison_work = None,
                        }
                        let result = crate::native_artifact::program::runtime::execute(
                            descriptor,
                            jit.test_generated_entry(),
                            &args,
                            &mut output,
                            limits,
                            &memory,
                        );
                        let normalized = result
                            .as_ref()
                            .map(|r| {
                                let mut r = *r;
                                r.parallel_peak_workers = 0;
                                r
                            })
                            .map_err(|e| format!("{e:?}"));
                        outcomes.push(format!("{normalized:?}"));
                        if result.is_err() {
                            assert!(output.iter().zip(&prior).all(|(a, b)| {
                                a.as_ref()
                                    .unwrap()
                                    .shares_frozen_storage(b.as_ref().unwrap())
                            }));
                            assert_eq!(
                                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                                held
                            );
                        }
                        let retry = crate::native_artifact::program::runtime::execute(
                            descriptor,
                            jit.test_generated_entry(),
                            &args,
                            &mut output,
                            mode.execution(),
                            &memory,
                        )
                        .unwrap();
                        assert_eq!(
                            output
                                .last()
                                .unwrap()
                                .as_ref()
                                .unwrap()
                                .canonical_data_draft()
                                .unwrap(),
                            ValueDataDraft::Matrix(
                                (0..3)
                                    .flat_map(|_| values
                                        .iter()
                                        .cloned()
                                        .chain(core::iter::once(ValueDataDraft::U64(6))))
                                    .collect()
                            )
                        );
                        assert!(retry.work >= first.work);
                    }
                    assert!(outcomes[0].starts_with("Err("));
                    assert!(outcomes[1].starts_with("Err("));
                    assert!(outcomes[2].starts_with("Err("));
                    assert!(
                        args.iter()
                            .zip(&aliases)
                            .all(|(a, b)| a.shares_frozen_storage(b))
                    );
                    results.push(outcomes);
                    drop((args, aliases, output));
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        (0, 0)
                    );
                }
                assert_eq!(results[0], results[1]);
            }
        }
    }
}
