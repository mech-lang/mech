//! A private mixed result must not pin its input before the first changed store.
use super::*;
use mech_core::ValueData;

#[test]
fn mixed_results_keep_one_initial_payload_for_real_private_buffers() {
    for (width, real) in [(16, 262144usize), (32, 1500513usize)] {
        let source = format!(
            "put(xs<[u{width}]>, at<u64>) => (out<[u{width}]>) :=\n  ~next := xs\n  next[at] = 7u{width}\n  out := next.\nwalk(xs<[u{width}]>, at<u64>, left<u64>) => <([u{width}],u64)>\n  | (xs, at, left) => ((left > 0u64)? | true => walk(put(xs, at), at + 1u64, left - 1u64) | false => (xs, at + 17u64)).\nrun(count<u64>, steps<u64>) => (out<[u{width}]>) :=\n  words := [0u{width} | i <- 0u64..=count, i < count]\n  (next, at) := walk(words, 1u64, steps)\n  current! := at == steps + 18u64\n  out := next.\nrun(count<u64>, steps<u64>)\n"
        );
        for (form, artifact) in ["original", "decoded"]
            .into_iter()
            .zip(artifacts(&source).0)
        {
            let mode = NativeProgramLimitMode::NoLimits;
            let jit = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            let metadata = emit_program_object(&artifact, &mech_stdlib::source_catalog())
                .unwrap()
                .metadata();
            assert!(
                metadata["numeric_write_regions"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|r| r["proof"]["result"]["Frame"].is_object())
            );
            let mut rows = Vec::new();
            for count in [288, real] {
                for steps in [0, 1, 8] {
                    let memory = mode.memory_budget();
                    let args = inputs(
                        &artifact,
                        &[
                            ("count", Data::U64(count as u64)),
                            ("steps", Data::U64(steps)),
                        ],
                        &memory,
                    );
                    let mut out = vec![None; artifact.outputs().len()];
                    jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                        .unwrap();
                    out.fill(None);
                    let (run, requests, bytes) = allocation_probe::measured_with_bytes(|| {
                        jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                    });
                    let report = run.unwrap();
                    let ValueData::Matrix(matrix) = out.last().unwrap().as_ref().unwrap().data()
                    else {
                        panic!("packed result");
                    };
                    macro_rules! check {
                        ($words:expr) => {{
                            assert_eq!($words.len(), count);
                            assert!(
                                $words
                                    .iter()
                                    .enumerate()
                                    .all(|(i, &v)| v == if i < steps as usize { 7 } else { 0 })
                            );
                        }};
                    }
                    match matrix.elements() {
                        mech_core::snapshot::SequenceView::U16(v) => check!(v),
                        mech_core::snapshot::SequenceView::U32(v) => check!(v),
                        _ => panic!("unsigned kind"),
                    }
                    rows.push((count, steps, requests, bytes, report.visits));
                    println!(
                        "{}",
                        serde_json::json!({"witness":"private-mixed-result-payload","width":width,"form":form,"count":count,"steps":steps,"requests":requests,"requested_bytes":bytes,"visits":report.visits})
                    );
                    drop((args, out));
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        (0, 0)
                    );
                }
            }
            for steps in [0, 1, 8] {
                let small = rows.iter().find(|r| r.0 == 288 && r.1 == steps).unwrap();
                let large = rows.iter().find(|r| r.0 == real && r.1 == steps).unwrap();
                assert_eq!(
                    large.3 - small.3,
                    (real - 288) * (width / 8),
                    "one private payload; no result pin may add a copy: {rows:?}"
                );
            }
            for count in [288, real] {
                let one = rows.iter().find(|r| r.0 == count && r.1 == 1).unwrap();
                let eight = rows.iter().find(|r| r.0 == count && r.1 == 8).unwrap();
                assert_eq!(
                    (one.2, one.3, one.4),
                    (eight.2, eight.3, eight.4),
                    "funded native updates have no per-iteration transactions"
                );
            }
        }
    }
}
