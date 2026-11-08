//! Exercise the actual emitted optimization and its independent checked path.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;

const SOURCE: &str = r#"walk(a<[u64]>, i<u64>, end<u64>) => (out<[u64]>) :=
  old := a[i + 1u64]
  incoming := old + 1u64
  ~next := a
  next[i + 1u64] = incoming
  out := ((i + 1u64 < end)? | true => walk(next, i + 1u64, end) | false => next).
walk(values<[u64]>, 0u64, count<u64>)
"#;

const APPEND_SOURCE: &str = r#"+> sequence
<state> := <([u64],[u64])>
walk(a<[u64]>, keys<[u64]>, i<u64>, end<u64>) => (out<state>) :=
  next-keys := sequence/append(keys, i)
  old := a[1u64]
  incoming := old + 1u64
  ~next := a
  next[1u64] = incoming
  out := ((i + 1u64 < end)? | true => walk(next, next-keys, i + 1u64, end) | false => (next, next-keys)).
built := walk(values<[u64]>, sequence/with-capacity(0u64, 4u64), 0u64, count<u64>)
counts := built.1
keys := built.2
counts[1u64] + sequence/length(keys)
"#;

fn counts(artifact: &ProgramArtifact) -> (usize, usize) {
    let object = crate::native_artifact::emit_program_object_with_limits(
        artifact,
        &mech_stdlib::source_catalog(),
        NativeProgramLimitMode::NoLimits.compilation(),
    )
    .unwrap();
    let metadata = object.metadata();
    [
        "compiled_numeric_write_loops",
        "compiled_readonly_numeric_loops",
    ]
    .into_iter()
    .flat_map(|key| metadata[key].as_array().unwrap())
    .fold((0, 0), |(proved, stores), region| {
        (
            proved
                + region["proven_checked_increment_writes"]
                    .as_array()
                    .unwrap()
                    .len(),
            stores + region["typed_stores"].as_array().unwrap().len(),
        )
    })
}

#[test]
fn checked_increment_writes_preserve_aliases_overflow_bounds_publication_and_retry() {
    let mode = NativeProgramLimitMode::NoLimits;
    for artifact in [SOURCE, APPEND_SOURCE].into_iter().flat_map(artifacts) {
        assert!(
            counts(&artifact).0 > 0,
            "fixture must select the actual optimization"
        );
        let optimized = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let checked = JitProgramArtifact::compile_for_test(
            &artifact,
            &mech_stdlib::source_catalog(),
            OptimizationPass::CheckedIncrementWrites,
        )
        .unwrap();
        let mut outcomes = Vec::new();
        for jit in [&optimized, &checked] {
            let memory = mode.memory_budget();
            let mut output = vec![None; artifact.outputs().len()];
            let mut actual = Vec::new();
            for (values, count) in [
                (vec![1, 2, 3], 3),
                (vec![1, u64::MAX, 3], 3),
                (vec![u64::MAX, 2, 3], 3),
                (vec![4, 5, 6], 4),
                (vec![7, 8, 9], 3),
            ] {
                let args = inputs(
                    &artifact,
                    &[
                        (
                            "values",
                            Data::Matrix(values.into_iter().map(Data::U64).collect()),
                        ),
                        ("count", Data::U64(count)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let before = contents(&args);
                let result = jit.evaluate_into(&args, &mut output, mode.execution(), &memory);
                assert_eq!(contents(&aliases), before);
                actual.push((
                    result
                        .as_ref()
                        .map(|r| (r.work, r.comparison_work))
                        .map_err(|e| format!("{e:?}")),
                    contents(&output.iter().flatten().cloned().collect::<Vec<_>>()),
                ));
            }
            drop(output);
            assert_eq!(memory.used_bytes(), 0);
            assert_eq!(memory.retained_snapshot_nodes(), 0);
            outcomes.push(actual);
        }
        assert_eq!(outcomes[0], outcomes[1]);
    }
    for source in [SOURCE.replace("incoming := old + 1u64", "incoming := 0u64"),
                   SOURCE.replace("next[i + 1u64] = incoming", "next[i + 2u64] = incoming"),
                   SOURCE.replace("  incoming := old + 1u64\n  ~next := a",
                       "  ~staged := a\n  staged[i + 1u64] = 0u64\n  incoming := old + 1u64\n  ~next := staged")] {
        for artifact in artifacts(&source) {
            let (proved, stores) = counts(&artifact);
            assert!(stores > 0, "negative fixture must retain native writes");
            assert_eq!(proved, 0, "unchanged or different-cell stores keep their equality/bounds checks");
        }
    }
}
