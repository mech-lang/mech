//! Row refusal, append after a private replacement, and output-limit recovery.
use super::*;

const SOURCE: &str = r#"+> sequence
+> bits
emit-one(bytes<[u8]>, word<u64>, count<u8>, i<u64>) => (out<[u8]>) :=
  ~changed := bytes
  changed[1u64] = 99u8
  out := sequence/append-le-bytes(changed, bits/wrapping-add(word, i), count).
walk(bytes<[u8]>, word<u64>, count<u8>, i<u64>, end<u64>) => (out<[u8]>) :=
  more := i < end
  next := (more? | true => emit-one(bytes, word, count, i) | false => bytes)
  out := (more? | true => walk(next, word, count, i + 1u64, end) | false => bytes).
seed := bytes<[u8]>
walk(seed, word<u64>, count<u8>, 0u64, steps<u64>)
"#;

#[test]
fn bulk_byte_append_preserves_geometry_private_replacement_and_output_limits() {
    use crate::native_artifact::codegen::options::OptimizationPass;
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::ScalarLoops,
        )
        .unwrap();
        let native =
            JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                .unwrap();
        let mut reference = std::collections::BTreeMap::new();
        for (is_reference, jit) in [(true, &ordinary), (false, &native)] {
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let mut published = vec![None; artifact.outputs().len()];
                for (case, column, count, steps, limit) in [
                    ("column-initial", true, 255, 0, None),
                    ("initial", false, 2, 3, None),
                    ("column-zero", true, 0, 1, None),
                    ("column-positive", true, 1, 1, None),
                    ("column-invalid", true, 255, 1, None),
                    ("unselected-column", true, 255, 0, None),
                    ("invalid", false, 255, 1, None),
                    ("growth", false, 8, 16, None),
                    ("zero-after-write", false, 0, 4, None),
                    ("output-elements", false, 8, 4, Some((true, 4))),
                    ("output-bytes", false, 8, 4, Some((false, 0))),
                    ("finite-retry", false, 1, 4, Some((true, 512))),
                    ("retry", false, 3, 4, None),
                ] {
                    let mut args = inputs(
                        &artifact,
                        &[
                            ("bytes", Data::Matrix([Data::U8(7), Data::U8(8)].into())),
                            ("word", Data::U64(0x0102_0304_0506_0708)),
                            ("count", Data::U8(count)),
                            ("steps", Data::U64(steps)),
                        ],
                        &memory,
                    );
                    let at = artifact
                        .inputs()
                        .iter()
                        .position(|port| {
                            mech_engine::decode_source_input_name(&port.name).unwrap() == "bytes"
                        })
                        .unwrap();
                    let schema = artifact.inputs()[at].schema;
                    let extents = if column { [2, 1] } else { [1, 2] };
                    let shape = mech_core::shape_for_resolved_extents(
                        artifact.schemas().get(schema).unwrap(),
                        &extents,
                    )
                    .unwrap();
                    let mut cells = Vec::with_capacity(128);
                    cells.extend([7u8, 8u8]);
                    let owner = Value::from_unsigned_matrix_buffer(
                        schema,
                        shape,
                        mech_core::snapshot::UnsignedMatrixBuffer::U8(cells),
                        std::sync::Arc::new(artifact.schemas().clone()),
                    )
                    .unwrap();
                    let mut reservation = memory
                        .reserve_capacity(owner.memory_budget_admission_bytes(&memory).unwrap())
                        .unwrap();
                    args[at] = owner.into_memory_budget(&mut reservation).unwrap();
                    drop(reservation);
                    let aliases = args.clone();
                    let before = contents(&args);
                    let previous = published.clone();
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let mut policy = mode.execution();
                    if let Some((elements, allowance)) = limit {
                        if elements {
                            policy.library.max_output_elements = Some(allowance);
                        } else {
                            policy.library.max_output_bytes = Some(allowance);
                        }
                    }
                    let (result, observed) = if restored {
                        observe::run_restored(jit, &args, &mut published, policy, &memory)
                    } else {
                        observe::run(jit, &args, &mut published, policy, &memory)
                    };
                    let actual = result
                        .map(|_| contents(&published.iter().flatten().cloned().collect::<Vec<_>>()))
                        .map_err(|error| format!("{error:?}"));
                    if is_reference {
                        assert_eq!(
                            actual,
                            reference
                                .entry(case)
                                .or_insert_with(|| actual.clone())
                                .clone()
                        );
                    } else {
                        assert_eq!(actual, reference[case], "{case} restored={restored}");
                    }
                    if actual.is_err() {
                        for (current, old) in published.iter().zip(&previous) {
                            assert!(
                                current
                                    .as_ref()
                                    .unwrap()
                                    .shares_frozen_storage(old.as_ref().unwrap())
                            );
                        }
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            retained
                        );
                    } else if !column {
                        let mut bytes = vec![Data::U8(7), Data::U8(8)];
                        for i in 0..steps {
                            bytes[0] = Data::U8(99);
                            bytes.extend(
                                0x0102_0304_0506_0708u64.wrapping_add(i).to_le_bytes()
                                    [..usize::from(count)]
                                    .iter()
                                    .copied()
                                    .map(Data::U8),
                            );
                        }
                        assert_eq!(
                            actual.as_ref().unwrap().last().unwrap(),
                            &Data::Matrix(bytes.into())
                        );
                        if !is_reference && case == "initial" {
                            assert!(
                                !observed.first_change_backings.is_empty(),
                                "replacement must enter private cells before bulk append"
                            );
                        }
                    }
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                }
                drop(published);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}
