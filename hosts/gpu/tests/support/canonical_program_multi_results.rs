//! Current owners for every field of a private numeric tuple return.
use super::{allocation_probe, artifacts, equal, inputs, reference_result};
use mech_core::{Value, ValueDataDraft as Data};
use mech_gpu::native_artifact::{JitProgramArtifact, NativeProgramLimitMode, emit_program_object};

const SOURCE: &str = r#"put16(xs<[u16]>, at<u64>, word<u16>) => (out<[u16]>) :=
  ~next := xs
  next[at] = word
  out := next.
put32(xs<[u32]>, at<u64>, word<u32>) => (out<[u32]>) :=
  ~next := xs
  next[at] = word
  out := next.
put8(xs<[u8]>, at<u64>, word<u8>) => (out<[u8]>) :=
  ~next := xs
  next[at] = word
  out := next.
quotient(xs<[u32]>, at<u64>, divisor<u64>) => (out<u32>) :=
  wide<u64> := xs[at]
  out<u32> := (wide + 1u64) / divisor.
walk(a<[u16]>, b<[u32]>, c<[u8]>, i<u64>, left<u64>, permit<bool>, divisor<u64>) => (out<([u16],u64,[u32],[u8])>) :=
  more := left > 0u64
  next-a := (more? | true => put16(a, i + 1u64, 8u16) | false => a)
  allowed! := permit == true
  next-b := (more? | true => put32(b, i + 1u64, quotient(b, i + 1u64, divisor)) | false => b)
  next-c := (more? | true => put8(c, i + 1u64, 18u8) | false => c)
  out := (more? | true => walk(next-a, next-b, next-c, i + 1u64, left - 1u64, permit, divisor) | false => (a, i + 17u64, b, c)).
(a,i,b,c) := walk(first<[u16]>, second<[u32]>, third<[u8]>, 0u64, steps<u64>, permit<bool>, divisor<u64>)
(a, i + 0u64, b, c)
"#;

fn matrix(words: impl IntoIterator<Item = Data>) -> Data {
    Data::Matrix(words.into_iter().collect())
}
fn result(out: &[Option<Value>]) -> Data {
    out.last()
        .unwrap()
        .as_ref()
        .unwrap()
        .canonical_data_draft()
        .unwrap()
}
fn unchanged(out: &[Option<Value>], before: &[Option<Value>]) {
    assert!(out.iter().zip(before).all(|(a, b)| match (a, b) {
        (Some(a), Some(b)) => a.shares_frozen_storage(b),
        (None, None) => true,
        _ => false,
    }));
}
fn selected(artifact: &mech_engine::ProgramArtifact, count: usize) {
    let metadata = emit_program_object(artifact, &mech_stdlib::source_catalog())
        .unwrap()
        .metadata();
    let candidates = metadata["numeric_write_regions"].as_array().unwrap();
    assert!(
        candidates.iter().any(|r| {
            r["proof"]["result"]["Frame"]["arrays"]
                .as_array()
                .is_some_and(|a| a.len() == count)
        }),
        "missing {count}-array return proof: {metadata}"
    );
    for region in candidates {
        assert_eq!(region["successful_backedge_helpers"], 0);
    }
}

#[test]
fn multi_array_results_restore_all_current_owners_and_recover_after_late_failure() {
    for (form, artifact) in ["original", "decoded"].into_iter().zip(artifacts(SOURCE).0) {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let mut out = vec![None; artifact.outputs().len()];
        let mut rows = Vec::new();
        for (steps, permit, divisor, short) in [
            (0, true, 1, false),
            (1, true, 1, false),
            (64, true, 1, false),
            (1, false, 0, false),
            (1, true, 0, false),
            (1, true, 1, true),
            (2, true, 1, false),
            (0, true, 0, false),
        ] {
            let args = inputs(
                &artifact,
                &[
                    ("first", matrix(vec![Data::U16(7); 288])),
                    (
                        "second",
                        matrix(vec![Data::U32(11); if short { 0 } else { 288 }]),
                    ),
                    ("third", matrix(vec![Data::U8(17); 288])),
                    ("steps", Data::U64(steps)),
                    ("permit", Data::Bool(permit)),
                    ("divisor", Data::U64(divisor)),
                ],
                &memory,
            );
            let aliases = args.clone();
            let before = args
                .iter()
                .map(|a| a.canonical_data_draft().unwrap())
                .collect::<Vec<_>>();
            let prior = out.clone();
            let used = (memory.used_bytes(), memory.retained_snapshot_nodes());
            let (run, requests, bytes) = allocation_probe::measured_with_bytes(|| {
                jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
            });
            if permit && (steps == 0 || divisor != 0 && !short) {
                let report = run.unwrap();
                let updated =
                    |old: u64, new: u64| (0..288).map(move |i| if i < steps { new } else { old });
                assert_eq!(
                    result(&out),
                    Data::Tuple(
                        vec![
                            matrix(updated(7, 8).map(|v| Data::U16(v as u16))),
                            Data::U64(steps + 17),
                            matrix(updated(11, 12).map(|v| Data::U32(v as u32))),
                            matrix(updated(17, 18).map(|v| Data::U8(v as u8)))
                        ]
                        .into()
                    )
                );
                // The resident oracle has a finite comparison budget; keep its
                // semantic witnesses short, and measure the funded native slope separately.
                if steps < 64 {
                    equal(
                        &artifact,
                        &out,
                        &reference_result(&artifact, &args).unwrap(),
                    );
                }
                rows.push((steps, requests, bytes, report.visits));
                println!(
                    "{}",
                    serde_json::json!({"witness":"multi-array-current-return","form":form,"steps":steps,"requests":requests,"requested_bytes":bytes,"visits":report.visits,"work":report.work})
                );
            } else {
                let error = format!("{:?}", run.unwrap_err());
                assert!(
                    if !permit {
                        error.contains("Integrity")
                    } else if divisor == 0 {
                        error.contains("Arithmetic")
                    } else {
                        error.contains("Kernel")
                    },
                    "{error}"
                );
                assert!(reference_result(&artifact, &args).is_err());
                unchanged(&out, &prior);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    used
                );
            }
            assert_eq!(
                args.iter()
                    .map(|a| a.canonical_data_draft().unwrap())
                    .collect::<Vec<_>>(),
                before
            );
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(a, b)| a.shares_frozen_storage(b))
            );
        }
        // Selection is checked after correctness so the parent run retains
        // useful ordinary-route observations before the expected red assertion.
        selected(&artifact, 3);
        let one = rows.iter().find(|r| r.0 == 1).unwrap();
        let many = rows.iter().find(|r| r.0 == 64).unwrap();
        assert_eq!(
            (one.1, one.2, one.3),
            (many.1, many.2, many.3),
            "per-step result transport remains: {rows:?}"
        );
        drop(out);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn multi_array_results_select_pinned_huffman_placement_and_keep_branch_versions() {
    let source = format!(
        "{}\n(counted,lengths,packed) := huff-place-symbols(freqs<[u32]>, 4u64, counts<[u64]>, lens<[u8]>, words<[u32]>, 0u64, 3u64)\n(counted,lengths,packed)\n",
        include_str!("../fixtures/native/deflate-huffman-level12.mec")
    );
    for artifact in artifacts(&source).0 {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let args = inputs(
            &artifact,
            &[
                ("freqs", matrix([0, 2, 0, 1].map(Data::U32))),
                ("counts", matrix([0, 0, 1, 2].map(Data::U64))),
                ("lens", matrix([7; 4].map(Data::U8))),
                ("words", matrix([99; 4].map(Data::U32))),
            ],
            &memory,
        );
        let mut out = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
            .unwrap();
        assert_eq!(
            result(&out),
            Data::Tuple(
                vec![
                    matrix([0, 1, 2, 2].map(Data::U64)),
                    matrix([0, 7, 0, 7].map(Data::U8)),
                    matrix([1027, 2049, 99, 99].map(Data::U32))
                ]
                .into()
            )
        );
        equal(
            &artifact,
            &out,
            &reference_result(&artifact, &args).unwrap(),
        );
        selected(&artifact, 3);
        drop((args, out));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn multi_array_results_check_freshness_of_each_returned_field() {
    // Only the second returned root is stale. Returning a root of the right
    // kind is insufficient after an earlier write invalidated that version.
    let source = r#"put(xs<[u32]>) => (out<[u32]>) :=
  ~next := xs
  next[1u64] = 9u32
  out := next.
walk(a<[u32]>, b<[u32]>, left<u64>) => (out<([u32],[u32])>) :=
  next-a := put(a)
  next-b := put(b)
  out := ((left > 0u64)? | true => walk(next-a,next-b,left - 1u64) | false => (next-a,b)).
(a,b) := walk(first<[u32]>,second<[u32]>,0u64)
(a,b)
"#;
    for artifact in artifacts(source).0 {
        let metadata = emit_program_object(&artifact, &mech_stdlib::source_catalog())
            .unwrap()
            .metadata();
        assert!(
            metadata["numeric_write_regions"]
                .as_array()
                .unwrap()
                .is_empty(),
            "{metadata}"
        );
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let args = inputs(
            &artifact,
            &[
                ("first", matrix([Data::U32(3)])),
                ("second", matrix([Data::U32(5)])),
            ],
            &memory,
        );
        let mut out = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
            .unwrap();
        assert_eq!(
            result(&out),
            Data::Tuple(vec![matrix([Data::U32(9)]), matrix([Data::U32(5)])].into())
        );
        equal(
            &artifact,
            &out,
            &reference_result(&artifact, &args).unwrap(),
        );
        drop((args, out));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
