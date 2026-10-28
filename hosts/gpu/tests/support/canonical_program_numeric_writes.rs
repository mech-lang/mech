//! Unchanged Huffman counting and an ordinary checked indexed-write recurrence.
use super::{allocation_probe, artifacts, inputs};
use mech_core::{ValueData, ValueDataDraft as Data};
use mech_gpu::native_artifact::{JitProgramArtifact, NativeProgramLimitMode, emit_program_object};

pub(super) const WRITE: &str = "+> sequence\nput(xs<[u32]>, at<u64>, value<u32>) => (out<[u32]>) :=\n  ~next := xs\n  next[at] = value\n  out := next.\nwalk(xs<[u32]>, i<u64>, end<u64>) => (out<[u32]>) :=\n  more := i < end\n  out := (more? | true => walk(put(xs, i + 1u64, xs[i + 1u64] + 1u32), i + 1u64, end) | false => xs).\nwalk(data<[u32]>, 0u64, steps<u64>)\n";

#[test]
fn numeric_write_baseline_observes_transport_in_generic_and_pinned_huffman_loops() {
    let huffman = format!(
        "{}\nhuff-count-symbols(freqs<[u32]>, steps<u64>, counts<[u64]>, 0u64, 255u64)\n",
        include_str!("../fixtures/native/deflate-huffman-level12.mec")
    );
    for (name, source) in [
        ("generic", WRITE),
        ("pinned-huffman-count", huffman.as_str()),
    ] {
        for (form, artifact) in ["original", "decoded"].into_iter().zip(artifacts(source).0) {
            let mode = NativeProgramLimitMode::NoLimits;
            let jit = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            let memory = mode.memory_budget();
            let mut outputs = vec![None; artifact.outputs().len()];
            let mut observations = Vec::new();
            for steps in [0, 1, 8, 64, 256] {
                let args = inputs(
                    &artifact,
                    &[
                        ("data", Data::Matrix((0..256).map(Data::U32).collect())),
                        (
                            "freqs",
                            Data::Matrix((0..256).map(|i| Data::U32(i % 17)).collect()),
                        ),
                        ("counts", Data::Matrix(vec![Data::U64(0); 256].into())),
                        ("steps", Data::U64(steps)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let before = args
                    .iter()
                    .map(|v| v.canonical_data_draft().unwrap())
                    .collect::<Vec<_>>();
                jit.evaluate_into(&args, &mut outputs, mode.execution(), &memory)
                    .unwrap();
                let started = std::time::Instant::now();
                let (result, requests, bytes) = allocation_probe::measured_with_bytes(|| {
                    jit.evaluate_into(&args, &mut outputs, mode.execution(), &memory)
                });
                let seconds = started.elapsed().as_secs_f64();
                let report = result.unwrap();
                observations.push((steps, requests, bytes, report.visits));
                let ValueData::Matrix(matrix) = outputs.last().unwrap().as_ref().unwrap().data()
                else {
                    panic!("matrix result")
                };
                if name == "generic" {
                    let mech_core::snapshot::SequenceView::U32(words) = matrix.elements() else {
                        panic!("U32 result")
                    };
                    assert_eq!(words.len(), 256);
                    for (i, word) in words.iter().enumerate() {
                        assert_eq!(*word, i as u32 + u32::from((i as u64) < steps));
                    }
                } else {
                    let mech_core::snapshot::SequenceView::U64(words) = matrix.elements() else {
                        panic!("U64 result")
                    };
                    let mut expected = vec![0u64; 256];
                    for i in 0..steps {
                        expected[(i % 17) as usize] += 1;
                    }
                    assert_eq!(words, expected);
                }
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(a, b)| a.shares_frozen_storage(b))
                );
                assert_eq!(
                    args.iter()
                        .map(|v| v.canonical_data_draft().unwrap())
                        .collect::<Vec<_>>(),
                    before
                );
                println!(
                    "{}",
                    serde_json::json!({"witness":name,"form":form,"steps":steps,"requests":requests,"requested_bytes":bytes,"visits":report.visits,"work":report.work,"diagnostic_seconds":seconds,"allocator":"System/probe; not primary compression timing"})
                );
            }
            assert!(
                observations[1..].iter().all(|row| (row.1, row.2, row.3)
                    == (observations[1].1, observations[1].2, observations[1].3)),
                "funded private updates must lose per-element allocations and visits: {observations:?}"
            );
            drop(outputs);
            assert_eq!(memory.used_bytes(), 0);
            assert_eq!(memory.retained_snapshot_nodes(), 0);
        }
    }
}

#[test]
fn numeric_write_recurrence_selects_native_frame_and_typed_store_execution() {
    let huffman = format!(
        "{}\nhuff-count-symbols(freqs<[u32]>, steps<u64>, counts<[u64]>, 0u64, 255u64)\n",
        include_str!("../fixtures/native/deflate-huffman-level12.mec")
    );
    for source in [WRITE, huffman.as_str()] {
        for artifact in artifacts(source).0 {
            let object = emit_program_object(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let metadata = object.metadata();
            let loops = metadata["compiled_numeric_write_loops"].as_array().unwrap();
            assert!(
                !loops.is_empty(),
                "indexed-write recurrence still uses ordinary frame/store transactions: {metadata}"
            );
            for region in loops {
                assert_eq!(region["successful_backedge_helpers"], 0);
                assert!(!region["typed_stores"].as_array().unwrap().is_empty());
            }
        }
    }
}

#[path = "canonical_program_numeric_write_cases.rs"]
mod cases;
