use super::*;

#[test]
fn numeric_writes_two_roots_and_two_versions_in_one_visit() {
    let source = "walk(a<[u32]>, b<[u16]>, left<u64>) => (out<u64>) :=\n  x := a[1u64] + 1u32\n  ~first := a\n  first[1u64] = x\n  ~second := first\n  second[1u64] = first[1u64] + 1u32\n  ~next-b := b\n  next-b[1u64] = 32769u16\n  word<u64> := second[1u64]\n  small<u64> := next-b[1u64]\n  out := ((left > 0u64)? | true => walk(second, next-b, left - 1u64) | false => word + small).\nwalk(data<[u32]>, small<[u16]>, steps<u64>)\n";
    selected(source, true);
    for artifact in artifacts(source).0 {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let mut out = vec![None; artifact.outputs().len()];
        for steps in [0, 1, 8, 64] {
            let args = inputs(
                &artifact,
                &[
                    ("data", matrix([Data::U32(10)])),
                    ("small", matrix([Data::U16(0)])),
                    ("steps", Data::U64(steps)),
                ],
                &memory,
            );
            let before = args
                .iter()
                .map(|v| v.canonical_data_draft().unwrap())
                .collect::<Vec<_>>();
            jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                .unwrap();
            equal(
                &artifact,
                &out,
                &reference_result(&artifact, &args).unwrap(),
            );
            assert_eq!(result(&out), Data::U64(32779 + 2 * (steps + 1)));
            assert_eq!(
                args.iter()
                    .map(|v| v.canonical_data_draft().unwrap())
                    .collect::<Vec<_>>(),
                before
            );
        }
        drop(out);
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
}

#[test]
fn numeric_writes_conditional_first_change_and_forwarded_current_versions() {
    let source = "put(xs<[u32]>) => (out<[u32]>) :=\n  value := xs[1u64] + 1u32\n  ~next := xs\n  next[1u64] = value\n  out := next.\nwalk(xs<[u32]>, left<u64>, flip<bool>) => (out<[u32]>) :=\n  changed := (flip? | true => put(xs) | false => xs)\n  out := ((left > 0u64)? | true => walk(changed, left - 1u64, flip == false) | false => changed).\nwalk(data<[u32]>, steps<u64>, flip<bool>)\n";
    selected(source, true);
    for artifact in artifacts(source).0 {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let mut out = vec![None; artifact.outputs().len()];
        for flip in [false, true] {
            for steps in [0, 1, 8, 63] {
                let args = inputs(
                    &artifact,
                    &[
                        ("data", matrix([Data::U32(10)])),
                        ("steps", Data::U64(steps)),
                        ("flip", Data::Bool(flip)),
                    ],
                    &memory,
                );
                jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                    .unwrap();
                equal(
                    &artifact,
                    &out,
                    &reference_result(&artifact, &args).unwrap(),
                );
                let changes = if flip {
                    (steps + 2) / 2
                } else {
                    (steps + 1) / 2
                };
                assert_eq!(result(&out), matrix([Data::U32(10 + changes as u32)]));
            }
        }
        drop(out);
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
}

#[test]
fn numeric_writes_unchanged_values_still_observe_selected_output_element_limits() {
    // Huffman setters also reach unchanged writes. Their selected output bound
    // remains observable even when no buffer copy or changed cell is necessary.
    let source = WRITE.replace("xs[i + 1u64] + 1u32", "xs[i + 1u64]");
    for artifact in artifacts(&source).0 {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let mut out = vec![None; artifact.outputs().len()];
        for steps in [0, 1, 0] {
            let args = inputs(
                &artifact,
                &[
                    ("data", matrix((0..8).map(Data::U32))),
                    ("steps", Data::U64(steps)),
                ],
                &memory,
            );
            let prior = out.clone();
            let before = (memory.used_bytes(), memory.retained_snapshot_nodes());
            let mut limits = mode.execution();
            limits.library.max_output_elements = Some(4);
            let run = jit.evaluate_into(&args, &mut out, limits, &memory);
            if steps == 0 {
                run.unwrap();
            } else {
                let error = format!("{:?}", run.unwrap_err());
                assert!(error.contains("OutputElements"), "{error}");
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    before
                );
                assert!(out.iter().zip(&prior).all(|(a, b)| {
                    a.as_ref()
                        .unwrap()
                        .shares_frozen_storage(b.as_ref().unwrap())
                }));
            }
        }
        drop(out);
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
}
