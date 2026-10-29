//! Private one-array-plus-scalar results of numeric leaves.
use super::{allocation_probe, artifacts, inputs};
use mech_core::ValueDataDraft as Data;
use mech_gpu::native_artifact::{JitProgramArtifact, NativeProgramLimitMode, emit_program_object};

pub(super) const GENERIC: &str = r#"put(xs<[u32]>, at<u64>, word<u32>) => (out<[u32]>) :=
  ~next := xs
  next[at] = word
  out := next.
walk(xs<[u32]>, i<u64>, left<u64>) => <([u32],u64)>
  | (xs, i, left) => ((left > 0u64)? | true => walk(put(xs, i + 1u64, xs[i + 1u64] + 1u32), i + 1u64, left - 1u64) | false => (xs, i + 17u64)).
outer(xs<[u32]>, start<u64>, left<u64>, again<u64>) => (out<([u32],u64)>) :=
  (next, at) := walk(xs, start, left)
  out := ((again > 0u64)? | true => outer(next, start + 1u64, left, again - 1u64) | false => (next, at)).
(result, total) := outer(data<[u32]>, seed<u64>, steps<u64>, 1u64)
(result, total + 0u64)
"#;

fn sources() -> [(String, String); 2] {
    [
        ("generic-array-index".into(), GENERIC.into()),
        (
            "pinned-assign-symbols".into(),
            format!(
                "{}\nhuff-assign-lengths(symbols<[u32]>, lens<[u8]>, counts<[u64]>, 2u64, 0u64)\n",
                include_str!("../fixtures/native/deflate-huffman-level12.mec")
            ),
        ),
    ]
}

#[test]
fn numeric_results_observe_generic_and_pinned_assign_transport() {
    for (name, source) in sources() {
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
            let memory = mode.memory_budget();
            let mut out = vec![None; artifact.outputs().len()];
            let mut observations = Vec::new();
            for steps in [0, 1, 8, 64, 256] {
                let generic = name == "generic-array-index";
                let mut counts = vec![Data::U64(0); 17];
                counts[1] = Data::U64(1);
                counts[2] = Data::U64(steps);
                let args = inputs(
                    &artifact,
                    &[
                        ("data", Data::Matrix((0..288).map(Data::U32).collect())),
                        ("steps", Data::U64(steps)),
                        ("seed", Data::U64(3)),
                        (
                            "symbols",
                            Data::Matrix(
                                (0..288).map(|i| Data::U32(2048 + (i * 17) % 288)).collect(),
                            ),
                        ),
                        ("lens", Data::Matrix(vec![Data::U8(7); 288].into())),
                        ("counts", Data::Matrix(counts.into())),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let before = args
                    .iter()
                    .map(|v| v.canonical_data_draft().unwrap())
                    .collect::<Vec<_>>();
                jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                    .unwrap();
                let (run, requests, bytes) = allocation_probe::measured_with_bytes(|| {
                    jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                });
                let report = run.unwrap();
                observations.push((steps, requests, bytes, report.visits));
                let expected = if generic {
                    let mut words = (0..288).collect::<Vec<u32>>();
                    for seed in [3, 4] {
                        for i in seed..seed + steps as usize {
                            words[i] += 1;
                        }
                    }
                    Data::Tuple(
                        vec![
                            Data::Matrix(words.into_iter().map(Data::U32).collect()),
                            Data::U64(steps + 21),
                        ]
                        .into(),
                    )
                } else {
                    let mut lens = vec![Data::U8(7); 288];
                    for i in 0..steps {
                        lens[((i * 17) % 288) as usize] = Data::U8(2);
                    }
                    lens[((steps * 17) % 288) as usize] = Data::U8(1);
                    Data::Matrix(lens.into())
                };
                assert_eq!(
                    out.last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    expected
                );
                assert_eq!(
                    args.iter()
                        .map(|v| v.canonical_data_draft().unwrap())
                        .collect::<Vec<_>>(),
                    before
                );
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(a, b)| a.shares_frozen_storage(b))
                );
                println!(
                    "{}",
                    serde_json::json!({"witness":"mixed-numeric-result","case":name,"form":form,"steps":steps,"requests":requests,"requested_bytes":bytes,"visits":report.visits,"work":report.work,"allocator":"System/probe; excluded from primary timing"})
                );
            }
            assert!(
                observations[1..].iter().all(|r| (r.1, r.2, r.3)
                    == (observations[1].1, observations[1].2, observations[1].3)),
                "per-iteration result transport remains: {observations:?}"
            );
            drop(out);
            assert_eq!(memory.used_bytes(), 0);
            assert_eq!(memory.retained_snapshot_nodes(), 0);
        }
    }
}

#[path = "canonical_program_numeric_result_cases.rs"]
mod cases;

#[path = "canonical_program_numeric_result_ownership.rs"]
mod ownership;

#[test]
fn numeric_results_select_generic_and_unchanged_assign_leaf_under_parent() {
    let mut observed = Vec::new();
    for (name, source) in sources() {
        for (form, artifact) in ["original", "decoded"]
            .into_iter()
            .zip(artifacts(&source).0)
        {
            let metadata = emit_program_object(&artifact, &mech_stdlib::source_catalog())
                .unwrap()
                .metadata();
            let regions = metadata["compiled_numeric_write_loops"].as_array().unwrap();
            let selected = regions.iter().any(|r| r["parent"].is_u64());
            observed.push((name.clone(), form, selected));
            println!(
                "{}",
                serde_json::json!({"witness":"mixed-result-selection","case":name,"form":form,"selected_parented":selected,"regions":metadata["numeric_write_regions"],"candidates":metadata["readonly_numeric_candidates"]})
            );
            for region in regions {
                assert_eq!(region["successful_backedge_helpers"], 0);
                assert!(!region["typed_stores"].as_array().unwrap().is_empty());
            }
        }
    }
    assert!(
        observed.iter().all(|r| r.2),
        "mixed result still uses ordinary frame transport: {observed:?}"
    );
}
