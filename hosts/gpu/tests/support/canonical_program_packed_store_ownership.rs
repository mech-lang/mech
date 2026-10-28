//! Copy complexity through the recursive packed-argument carrier, not just a leaf.
use super::{allocation_probe, artifacts, inputs};
use mech_core::{ManagedMemoryBudget, ValueData, ValueDataDraft as Data};
use mech_gpu::native_artifact::{JitProgramArtifact, NativeProgramLimitMode};

#[test]
fn recursive_unsigned_updates_allocate_only_the_necessary_payload() {
    let mut comparisons = Vec::new();
    for (width, real) in [(16, 262_144usize), (32, 1_500_513)] {
        for private in [false, true] {
            let entry = if private {
                format!(
                    "run(count<u64>, steps<u64>) => (out<[u{width}]>) :=\n  words := [0u{width} | i <- 0u64..=count, i < count]\n  out := walk(words, 1u64, steps).\nrun(count<u64>, steps<u64>)\n"
                )
            } else {
                format!("walk(input<[u{width}]>, 1u64, steps<u64>)\n")
            };
            let source = format!(
                "put(words<[u{width}]>, at<u64>) => (out<[u{width}]>) :=\n  ~next := words\n  next[at] = 7u{width}\n  out := next.\nwalk(words<[u{width}]>, at<u64>, left<u64>) => <[u{width}]>\n  | (words, at, left) => ((left > 0u64)? | true => walk(put(words, at), at + 1u64, left - 1u64) | false => words).\n{entry}"
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
                let mut measurements = Vec::new();
                for count in [288, real] {
                    for steps in [0, 1, 8] {
                        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
                        let named = if private {
                            vec![
                                ("count", Data::U64(count as u64)),
                                ("steps", Data::U64(steps)),
                            ]
                        } else {
                            let zero = if width == 16 {
                                Data::U16(0)
                            } else {
                                Data::U32(0)
                            };
                            vec![
                                ("input", Data::Matrix(vec![zero; count].into())),
                                ("steps", Data::U64(steps)),
                            ]
                        };
                        let args = inputs(&artifact, &named, &memory);
                        let aliases = args.clone();
                        let mut output = vec![None; artifact.outputs().len()];
                        jit.evaluate_into(&args, &mut output, mode.execution(), &memory)
                            .unwrap();
                        output.fill(None);
                        let started = std::time::Instant::now();
                        let (result, requests, bytes) =
                            allocation_probe::measured_with_bytes(|| {
                                jit.evaluate_into(&args, &mut output, mode.execution(), &memory)
                            });
                        let elapsed = started.elapsed().as_secs_f64();
                        let report = result.unwrap();
                        let ValueData::Matrix(matrix) =
                            output.last().unwrap().as_ref().unwrap().data()
                        else {
                            panic!("packed output")
                        };
                        macro_rules! check {
                            ($words:expr) => {{
                                assert_eq!($words.len(), count);
                                assert!($words.iter().enumerate().all(
                                    |(i, &word)| word == if i < steps as usize { 7 } else { 0 }
                                ));
                            }};
                        }
                        match matrix.elements() {
                            mech_core::snapshot::SequenceView::U16(words) => check!(words),
                            mech_core::snapshot::SequenceView::U32(words) => check!(words),
                            _ => panic!("packed kind"),
                        }
                        assert!(
                            args.iter()
                                .zip(&aliases)
                                .all(|(a, b)| a.shares_frozen_storage(b))
                        );
                        for value in &aliases {
                            if let ValueData::Matrix(matrix) = value.data() {
                                match matrix.elements() {
                                    mech_core::snapshot::SequenceView::U16(words) => {
                                        assert!(words.iter().all(|&v| v == 0))
                                    }
                                    mech_core::snapshot::SequenceView::U32(words) => {
                                        assert!(words.iter().all(|&v| v == 0))
                                    }
                                    _ => panic!("input kind"),
                                }
                            }
                        }
                        println!(
                            "{}",
                            serde_json::json!({"witness":"recursive-scalar-store","form":form,"width":width,"private":private,"count":count,"steps":steps,"requests":requests,"requested_bytes":bytes,"elapsed_seconds":elapsed,"visits":report.visits,"work":report.work})
                        );
                        measurements.push((count, steps, bytes));
                        drop((output, args, aliases));
                        assert_eq!(memory.used_bytes(), 0);
                        assert_eq!(memory.retained_snapshot_nodes(), 0);
                    }
                }
                for steps in [0, 1, 8] {
                    let bytes = |count| {
                        measurements
                            .iter()
                            .find(|(n, s, _)| *n == count && *s == steps)
                            .unwrap()
                            .2
                    };
                    let expected = usize::from(private || steps > 0) * (real - 288) * (width / 8);
                    comparisons.push((
                        width,
                        private,
                        form,
                        steps,
                        bytes(real) - bytes(288),
                        expected,
                    ));
                }
            }
        }
    }
    for (width, private, form, steps, actual, expected) in comparisons {
        assert_eq!(
            actual, expected,
            "u{width}, private={private}, {form}, stores={steps}: payload allocation must not scale with store count"
        );
    }
}
