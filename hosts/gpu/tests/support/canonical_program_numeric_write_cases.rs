use super::super::{allocation_probe, artifacts, equal, inputs, reference_result};
use super::WRITE;
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::{JitProgramArtifact, NativeProgramLimitMode, emit_program_object};

fn matrix(values: impl IntoIterator<Item = Data>) -> Data {
    Data::Matrix(values.into_iter().collect())
}
fn result(out: &[Option<mech_core::Value>]) -> Data {
    out.last()
        .unwrap()
        .as_ref()
        .unwrap()
        .canonical_data_draft()
        .unwrap()
}
fn selected(source: &str, expected: bool) {
    let artifact = artifacts(source).0.into_iter().next().unwrap();
    let metadata = emit_program_object(&artifact, &mech_stdlib::source_catalog())
        .unwrap()
        .metadata();
    let regions = metadata["compiled_numeric_write_loops"].as_array().unwrap();
    assert_eq!(!regions.is_empty(), expected, "{metadata}");
    for region in regions {
        assert_eq!(region["successful_backedge_helpers"], 0);
        assert!(!region["typed_stores"].as_array().unwrap().is_empty());
        assert!(
            !region["first_change_boundaries"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
}

#[test]
fn numeric_writes_all_unsigned_widths_rectangular_geometry_and_empty_visits() {
    use mech_core::snapshot::{SnapshotValidationContext, ValueDraft};
    for width in [8, 16, 32, 64] {
        // Narrow arithmetic is a separate selected-factory capability. This
        // qualification isolates the fixed-width store with a typed constant.
        let incoming: u64 = (1u64 << (width - 1)) | 1;
        let source = WRITE
            .replace("xs[i + 1u64] + 1u32", &format!("{incoming}u32"))
            .replace("u32", &format!("u{width}"));
        selected(&source, true);
        for artifact in artifacts(&source).0 {
            let mode = NativeProgramLimitMode::NoLimits;
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let mut out = vec![None; artifact.outputs().len()];
            for (rows, columns, steps) in [
                (2, 3, 4),
                (3, 2, 4),
                (1, 6, 0),
                (6, 1, 6),
                (0, 3, 0),
                (3, 0, 0),
                (2, 3, 2),
            ] {
                let count = rows * columns;
                let word = |i: u64| match width {
                    8 => Data::U8(128 + i as u8),
                    16 => Data::U16(32768 + i as u16),
                    32 => Data::U32(0x8000_0000 + i as u32),
                    64 => Data::U64(0x8000_0000_0000_0000 + i),
                    _ => unreachable!(),
                };
                let mut args = inputs(
                    &artifact,
                    &[
                        ("data", matrix((0..count).map(word))),
                        ("steps", Data::U64(steps)),
                    ],
                    &memory,
                );
                let at = artifact
                    .inputs()
                    .iter()
                    .position(|p| mech_engine::decode_source_input_name(&p.name).unwrap() == "data")
                    .unwrap();
                let schema = artifact.inputs()[at].schema;
                let shape = mech_core::shape_for_resolved_extents(
                    artifact.schemas().get(schema).unwrap(),
                    &[rows, columns],
                )
                .unwrap();
                let value = ValueDraft {
                    schema,
                    shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
                    data: args[at].canonical_data_draft().unwrap(),
                }
                .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                .unwrap();
                let mut admission = memory
                    .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
                    .unwrap();
                args[at] = value.into_memory_budget(&mut admission).unwrap();
                drop(admission);
                let before = args[at].canonical_data_draft().unwrap();
                let expected = reference_result(&artifact, &args).unwrap();
                jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                    .unwrap();
                equal(&artifact, &out, &expected);
                assert_eq!(args[at].canonical_data_draft().unwrap(), before);
                if steps == 0 {
                    assert!(
                        out.last()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .shares_frozen_storage(&args[at])
                    );
                }
                // Independent physical result: only the first `steps` source
                // column-major indices change, never contiguous row prefixes.
                let wanted = matrix((0..count).map(|i| {
                    let logical = (i % columns) * rows + i / columns;
                    if logical < steps { word(1) } else { word(i) }
                }));
                assert_eq!(result(&out), wanted);
            }
            drop(out);
            assert_eq!(memory.used_bytes(), 0);
            assert_eq!(memory.retained_snapshot_nodes(), 0);
        }
    }
}

#[test]
fn numeric_writes_refuse_old_reads_old_returns_and_discarded_updates() {
    let base = "put(xs<[u32]>, value<u32>) => (out<[u32]>) :=\n  ~next := xs\n  next[1u64] = value\n  out := next.\n";
    for body in [
        "walk(xs<[u32]>, left<u64>) => (out<u64>) :=\n  next := put(xs, 99u32)\n  old<u64> := xs[1u64]\n  out := ((left > 0u64)? | true => walk(next, left - 1u64) | false => old).\nwalk(data<[u32]>, 2u64)\n",
        "walk(xs<[u32]>, left<u64>) => (out<[u32]>) :=\n  next := put(xs, 99u32)\n  out := ((left > 0u64)? | true => walk(next, left - 1u64) | false => xs).\nwalk(data<[u32]>, 2u64)\n",
        "walk(xs<[u32]>, left<u64>) => (out<[u32]>) :=\n  next := put(xs, 99u32)\n  out := ((left > 0u64)? | true => walk(xs, left - 1u64) | false => next).\nwalk(data<[u32]>, 2u64)\n",
    ] {
        let source = format!("{base}{body}");
        selected(&source, false);
        for artifact in artifacts(&source).0 {
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let args = inputs(&artifact, &[("data", matrix([Data::U32(7)]))], &memory);
            let expected = reference_result(&artifact, &args).unwrap();
            let mut out = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                .unwrap();
            equal(&artifact, &out, &expected);
            assert_eq!(
                args[0].canonical_data_draft().unwrap(),
                matrix([Data::U32(7)])
            );
            drop((args, out, expected));
            assert_eq!(memory.used_bytes(), 0);
            assert_eq!(memory.retained_snapshot_nodes(), 0);
        }
    }
}

#[test]
fn numeric_writes_noop_changes_preserve_the_original_backing() {
    let source = WRITE.replace("xs[i + 1u64] + 1u32", "xs[i + 1u64]");
    selected(&source, true);
    for artifact in artifacts(&source).0 {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        for steps in [0, 1, 8, 64] {
            let args = inputs(
                &artifact,
                &[
                    ("data", matrix((0..64).map(Data::U32))),
                    ("steps", Data::U64(steps)),
                ],
                &memory,
            );
            let at = artifact
                .inputs()
                .iter()
                .position(|p| mech_engine::decode_source_input_name(&p.name).unwrap() == "data")
                .unwrap();
            let mut out = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                .unwrap();
            assert!(
                out.last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .shares_frozen_storage(&args[at])
            );
        }
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
}

#[test]
fn numeric_writes_real_buffers_allocate_once_and_not_per_store() {
    for (width, count) in [(16, 262144), (32, 1500513)] {
        let source = WRITE
            .replace("xs[i + 1u64] + 1u32", "1u32")
            .replace("u32", &format!("u{width}"));
        for artifact in artifacts(&source).0 {
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let mut observations = Vec::new();
            for steps in [1, 8] {
                let zero = if width == 16 {
                    Data::U16(0)
                } else {
                    Data::U32(0)
                };
                let args = inputs(
                    &artifact,
                    &[
                        ("data", Data::Matrix(vec![zero; count].into())),
                        ("steps", Data::U64(steps)),
                    ],
                    &memory,
                );
                let mut out = vec![None; artifact.outputs().len()];
                // Warm lazy immutable metadata before comparing loop slopes.
                jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                    .unwrap();
                let (run, requests, bytes) = allocation_probe::measured_with_bytes(|| {
                    jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                });
                let report = run.unwrap();
                observations.push((requests, bytes, report.visits));
                let mut wanted = vec![
                    if width == 16 {
                        Data::U16(0)
                    } else {
                        Data::U32(0)
                    };
                    count
                ];
                wanted[..steps as usize].fill(if width == 16 {
                    Data::U16(1)
                } else {
                    Data::U32(1)
                });
                assert_eq!(result(&out), Data::Matrix(wanted.into()));
                println!(
                    "{}",
                    serde_json::json!({"witness":"private-real-buffer","width":width,"count":count,"steps":steps,"requests":requests,"requested_bytes":bytes,"visits":report.visits,"work":report.work})
                );
            }
            assert_eq!(observations[0], observations[1]);
            assert_eq!(memory.used_bytes(), 0);
            assert_eq!(memory.retained_snapshot_nodes(), 0);
        }
    }
}

#[path = "canonical_program_numeric_write_recovery.rs"]
mod recovery;

#[path = "canonical_program_numeric_write_paths.rs"]
mod paths;

#[test]
fn numeric_static_vector_writes_preserve_current_shapes_and_first_change_aliases() {
    for (rows, columns) in [(1u64, 6u64), (6, 1), (2, 3)] {
        let source = format!(
            "<words> := <[u32]:{rows},{columns}>\n{}",
            WRITE.replace("<[u32]>", "<words>")
        );
        selected(&source, true);
        for artifact in artifacts(&source).0 {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            let mut out = vec![None; artifact.outputs().len()];
            for steps in [4, 7, 6] {
                let args = inputs(
                    &artifact,
                    &[
                        ("data", matrix((0..6).map(Data::U32))),
                        ("steps", Data::U64(steps)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let before = args
                    .iter()
                    .map(|v| v.canonical_data_draft().unwrap())
                    .collect::<Vec<_>>();
                let prior = out.clone();
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let expected = reference_result(&artifact, &args);
                let actual = jit.evaluate_into(&args, &mut out, mode.execution(), &memory);
                if steps <= 6 {
                    actual.unwrap();
                    equal(&artifact, &out, &expected.unwrap());
                    let wanted = matrix((0u64..6).map(|physical| {
                        let logical = (physical % columns) * rows + physical / columns;
                        Data::U32(physical as u32 + u32::from(logical < steps))
                    }));
                    assert_eq!(result(&out), wanted);
                } else {
                    assert!(expected.is_err());
                    assert!(actual.is_err());
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                    assert!(out.iter().zip(&prior).all(|(now, old)| {
                        now.as_ref()
                            .unwrap()
                            .shares_frozen_storage(old.as_ref().unwrap())
                    }));
                }
                assert_eq!(
                    args.iter()
                        .map(|v| v.canonical_data_draft().unwrap())
                        .collect::<Vec<_>>(),
                    before
                );
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(now, old)| now.shares_frozen_storage(old))
                );
            }
            drop(out);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
