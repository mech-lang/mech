//! Correlation of repeated current-visit Boolean branches in numeric leaves.
use super::{allocation_probe, artifacts, inputs};
use mech_core::ValueDataDraft as Data;
use mech_gpu::native_artifact::{JitProgramArtifact, NativeProgramLimitMode, emit_program_object};

const PREFIX: &str = "put(xs<[u64]>, at<u64>, value<u64>) => (out<[u64]>) :=\n  ~next := xs\n  next[at] = value\n  out := next.\nstep(xs<[u64]>, used<u64>, i<u64>) => (out<([u64],u64)>) :=\n  old := xs[i + 1u64]\n  next := put(xs, i + 1u64, used)\n  out := (next, used + old).\nscan(xs<[u64]>, n<u64>, used<u64>, i<u64>) => (out<([u64],u64)>) :=\n  more := i < n\n  next := (more? | true => step(xs, used, i) | false => (xs, used))\n  (ys, total) := next\n  out := (more? | true => scan(ys, n, total, i + 1u64) | false => (xs, used)).\n(result, total) := scan(data<[u64]>, steps<u64>, seed<u64>, 1u64)\n(result, total + 0u64)\n";

fn sources() -> [(String, String); 2] {
    [
        ("generic-prefix".into(), PREFIX.into()),
        (
            "pinned-cumulative".into(),
            format!(
                "{}\n(result, total) := huff-cumulative(data<[u64]>, steps<u64>, seed<u64>, 1u64)\n(result, total + 0u64)\n",
                include_str!("../fixtures/native/deflate-huffman-level12.mec")
            ),
        ),
    ]
}

#[test]
fn predicate_versions_observe_generic_and_pinned_cumulative() {
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
            for (steps, length) in [
                (0, 288),
                (1, 288),
                (2, 288),
                (8, 288),
                (19, 19),
                (32, 32),
                (64, 64),
                (288, 288),
            ] {
                let original = (0..length).map(|i| i % 7 + 1).collect::<Vec<_>>();
                let args = inputs(
                    &artifact,
                    &[
                        (
                            "data",
                            Data::Matrix(original.iter().copied().map(Data::U64).collect()),
                        ),
                        ("steps", Data::U64(steps)),
                        ("seed", Data::U64(5)),
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
                let started = std::time::Instant::now();
                let (run, requests, bytes) = allocation_probe::measured_with_bytes(|| {
                    jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                });
                let report = run.unwrap();
                let mut expected = original;
                let mut used = 5;
                for i in 1..steps {
                    let old = expected[i as usize];
                    expected[i as usize] = used;
                    used += old;
                }
                assert_eq!(
                    out.last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::Tuple(
                        vec![
                            Data::Matrix(expected.into_iter().map(Data::U64).collect()),
                            Data::U64(used)
                        ]
                        .into()
                    )
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
                    serde_json::json!({"witness":"current-predicate-versions","case":name,"form":form,"end":steps,"length":length,"seconds":started.elapsed().as_secs_f64(),"requests":requests,"requested_bytes":bytes,"visits":report.visits,"work":report.work,"allocator":"System/probe; excluded from primary timing"})
                );
            }
            drop(out);
            assert_eq!(memory.used_bytes(), 0);
            assert_eq!(memory.retained_snapshot_nodes(), 0);
        }
    }
}

#[test]
fn predicate_versions_select_generic_and_unchanged_cumulative_leaf() {
    let mut missing = Vec::new();
    for (name, source) in sources() {
        for (form, artifact) in ["original", "decoded"]
            .into_iter()
            .zip(artifacts(&source).0)
        {
            let metadata = emit_program_object(&artifact, &mech_stdlib::source_catalog())
                .unwrap()
                .metadata();
            let regions = metadata["compiled_numeric_write_loops"].as_array().unwrap();
            println!(
                "{}",
                serde_json::json!({"witness":"predicate-selection", "case":name, "form":form,
                "regions":regions, "candidates":metadata["readonly_numeric_candidates"]})
            );
            if regions.is_empty() {
                missing.push(format!("{name}/{form}"));
            }
            for region in regions {
                assert_eq!(region["successful_backedge_helpers"], 0);
                assert!(!region["typed_stores"].as_array().unwrap().is_empty());
            }
        }
    }
    assert!(
        missing.is_empty(),
        "correlated current-version proof missing: {missing:?}"
    );
}
