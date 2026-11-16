use super::super::{artifacts, inputs};
use super::{WEIGHTED, witness_limits};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::{JitProgramArtifact, emit_program_object};

#[test]
fn readonly_numeric_real_buffers_initialize_in_mech_without_an_iteration_slope() {
    use mech_gpu::native_artifact::NativeProgramLimitMode;
    let body = WEIGHTED
        .rsplit_once("\nweighted(")
        .unwrap()
        .0
        .replace("u8", "u16");
    let source = format!(
        "{body}\ndata := [3u32 | i <- 0u64..=1500513u64, i < 1500513u64]\ncoefficients := [1u16 | i <- 0u64..=262144u64, i < 262144u64]\n(data, coefficients, weighted(data, coefficients, 0u64, steps<u64>, 17u64))\n"
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
        assert_eq!(
            jit.descriptor()
                .plan()
                .facts()
                .uniform_unsigned_collections()
                .len(),
            2
        );
        let memory = mode.memory_budget();
        let mut rows = Vec::new();
        for steps in [0, 1, 8] {
            let args = inputs(&artifact, &[("steps", Data::U64(steps))], &memory);
            let mut output = vec![None; artifact.outputs().len()];
            // Prepared factory metadata is initialized on the first call.
            // Each warmup still constructs and releases both real arrays.
            jit.evaluate_into(&args, &mut output, mode.execution(), &memory)
                .unwrap();
            drop(output);
            let mut output = vec![None; artifact.outputs().len()];
            let start = std::time::Instant::now();
            let (result, requests, bytes) =
                super::super::allocation_probe::measured_with_bytes(|| {
                    jit.evaluate_into(&args, &mut output, mode.execution(), &memory)
                });
            let report = result.unwrap();
            let elapsed = start.elapsed().as_secs_f64();
            let mech_core::ValueData::Tuple(fields) =
                output.last().unwrap().as_ref().unwrap().data()
            else {
                panic!("expected real buffers and reduction")
            };
            assert!(
                matches!(fields[2], mech_core::ValueData::U64(value) if value == 17 + 3 * steps)
            );
            let mut packed = Vec::new();
            for value in &fields[..2] {
                if let mech_core::ValueData::Matrix(matrix) = value {
                    use mech_core::snapshot::SequenceView;
                    match matrix.elements() {
                        SequenceView::U16(words) => {
                            assert_eq!(words.len(), 262144);
                            assert!(words.iter().all(|word| *word == 1));
                            packed.push(16);
                        }
                        SequenceView::U32(words) => {
                            assert_eq!(words.len(), 1500513);
                            assert!(words.iter().all(|word| *word == 3));
                            packed.push(32);
                        }
                        _ => panic!("unexpected real buffer representation"),
                    }
                }
            }
            packed.sort_unstable();
            assert_eq!(packed, [16, 32]);
            println!(
                "{}",
                serde_json::json!({"witness":"readonly-real-buffers","form":form,"u16_words":262144,"u32_words":1500513,"steps":steps,"requests":requests,"requested_bytes":bytes,"visits":report.visits,"work":report.work,"diagnostic_seconds":elapsed,"policy":"NoLimits diagnostic; input construction and native compilation excluded"})
            );
            rows.push((requests, bytes, report.visits));
            drop((args, output));
            assert_eq!(memory.used_bytes(), 0);
            assert_eq!(memory.retained_snapshot_nodes(), 0);
        }
        assert!(
            rows.iter().all(|row| *row == rows[0]),
            "real buffer native loop has an allocation/transition slope: {rows:?}"
        );
    }
}

#[test]
fn readonly_numeric_pinned_true_costs_preserve_short_guards_and_subtraction_order() {
    let header = concat!(
        include_str!("../fixtures/native/deflate-huffman-level12.mec"),
        "\n",
        include_str!("../fixtures/native/deflate-header.mec"),
        "\n"
    );
    for extra in [false, true] {
        let call = if extra {
            "optimal-true-cost-extra(freqs<[u32]>, lens<[u8]>, extras<[u8]>, base<u64>, sym<u64>, end<u64>, cost<u64>)\n"
        } else {
            "optimal-true-cost-literals(freqs<[u32]>, lens<[u8]>, sym<u64>, cost<u64>)\n"
        };
        for artifact in artifacts(&format!("{header}{call}")).0 {
            let catalog = mech_stdlib::source_catalog();
            let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let metadata = emit_program_object(&artifact, &catalog).unwrap().metadata();
            assert!(
                !metadata["compiled_readonly_numeric_loops"]
                    .as_array()
                    .unwrap()
                    .is_empty(),
                "{metadata}"
            );
            println!(
                "{}",
                serde_json::json!({"witness":if extra {"pinned-true-cost-extra"} else {"pinned-true-cost-literals"}, "regions":metadata["readonly_numeric_regions"], "emissions":metadata["compiled_readonly_numeric_loops"]})
            );
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let mut output = vec![None; artifact.outputs().len()];
            for (base, end, cost, empty, expected) in [
                (0, 1, 5, false, Some(if extra { 25 } else { 17 })),
                (1, 0, 5, true, Some(5)),
                (1, 1, 5, true, if extra { None } else { Some(5) }),
                (0, 1, u64::MAX, false, None),
                (0, 1, 0, false, Some(if extra { 20 } else { 12 })),
            ] {
                let args = inputs(
                    &artifact,
                    &[
                        (
                            "freqs",
                            Data::Matrix(if empty { vec![] } else { vec![Data::U32(4)] }.into()),
                        ),
                        (
                            "lens",
                            Data::Matrix(if empty { vec![] } else { vec![Data::U8(3)] }.into()),
                        ),
                        (
                            "extras",
                            Data::Matrix(if empty { vec![] } else { vec![Data::U8(2)] }.into()),
                        ),
                        ("sym", Data::U64(0)),
                        ("base", Data::U64(base)),
                        ("end", Data::U64(end)),
                        ("cost", Data::U64(cost)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let prior = output.clone();
                let before = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let value = jit.evaluate_into(&args, &mut output, witness_limits(), &memory);
                if let Some(expected) = expected {
                    value.unwrap();
                    assert_eq!(
                        output
                            .last()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        Data::U64(expected)
                    );
                } else {
                    let error = format!("{:?}", value.unwrap_err());
                    assert!(error.contains("Arithmetic"), "{error}");
                    assert_eq!(
                        before,
                        (memory.used_bytes(), memory.retained_snapshot_nodes())
                    );
                    assert!(output.iter().zip(&prior).all(|(a, b)| {
                        a.as_ref()
                            .unwrap()
                            .shares_frozen_storage(b.as_ref().unwrap())
                    }));
                }
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(a, b)| a.shares_frozen_storage(b))
                );
            }
            drop(output);
            assert_eq!(memory.used_bytes(), 0);
            assert_eq!(memory.retained_snapshot_nodes(), 0);
        }
    }
}

#[test]
fn readonly_numeric_finite_work_preserves_publication_cleanup_and_retry() {
    // The Boolean result distinguishes the current reduction from a stale seed.
    let source = "walk(words<[u32]>, left<u64>, total<u64>, expected<u64>) => (out<bool>) :=\n  more := left > 0u64\n  value<u64> := (more? | true => words[1u64] | false => 0u32)\n  out := (more? | true => walk(words, left - 1u64, total + value, expected) | false => total == expected).\nwalk(words<[u32]>, steps<u64>, 0u64, expected<u64>)\n";
    for artifact in artifacts(source).0 {
        assert_eq!(artifact.outputs().len(), 1);
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let metadata = emit_program_object(&artifact, &catalog).unwrap().metadata();
        let cost = metadata["readonly_numeric_regions"][0]["compute_per_traversal"]
            .as_u64()
            .unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None];
        let mut baseline = None;
        for steps in [0, 1, 8, 64] {
            let args = inputs(
                &artifact,
                &[
                    ("words", Data::Matrix(vec![Data::U32(3)].into())),
                    ("steps", Data::U64(steps)),
                    ("expected", Data::U64(steps * 3)),
                ],
                &memory,
            );
            jit.evaluate_into(&args, &mut output, witness_limits(), &memory)
                .unwrap();
            let report = jit
                .evaluate_into(&args, &mut output, witness_limits(), &memory)
                .unwrap();
            let base = *baseline.get_or_insert(report.work);
            assert_eq!(report.work, base + steps * cost);
            let mut exact = witness_limits();
            exact.execution.max_work = report.work;
            jit.evaluate_into(&args, &mut output, exact, &memory)
                .unwrap();
            let prior = output.clone();
            let before = (memory.used_bytes(), memory.retained_snapshot_nodes());
            let mut denied = exact;
            denied.execution.max_work -= 1;
            assert!(
                jit.evaluate_into(&args, &mut output, denied, &memory)
                    .is_err()
            );
            assert_eq!(
                before,
                (memory.used_bytes(), memory.retained_snapshot_nodes())
            );
            assert!(
                output[0]
                    .as_ref()
                    .unwrap()
                    .shares_frozen_storage(prior[0].as_ref().unwrap())
            );
            jit.evaluate_into(&args, &mut output, witness_limits(), &memory)
                .unwrap();
            assert_eq!(
                output[0].as_ref().unwrap().canonical_data_draft().unwrap(),
                Data::Bool(true)
            );
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
}
