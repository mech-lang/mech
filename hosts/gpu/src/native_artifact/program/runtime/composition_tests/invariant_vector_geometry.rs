//! General schemas must decline unit-axis identities and keep exact errors/retry.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;
const SOURCE: &str = r#"+> sequence
shape-store(data<[u64]>, at<u64>) => (out<[u64]>) :=
  ~next := data
  next[at + 1u64] = data[at + 1u64] + 1u64
  out := next.
shape-bump(data<[u64]>, at<u64>, stop<u64>) => (out<[u64]>) :=
  more := at < stop
  out := (more? | true => shape-bump(shape-store(data, at), at + 1u64, stop) | false => data).
original := data<[u64]>
result := shape-bump(original, 0u64, stop<u64>)
(result, original)
"#;
#[test]
fn vector_geometry_keeps_mutation_errors_aliases_finite_prefixes_and_retry() {
    let mode = NativeProgramLimitMode::NoLimits;
    let catalog = mech_stdlib::source_catalog();
    for artifact in artifacts(SOURCE) {
        let object = crate::native_artifact::emit_program_object_with_limits(
            &artifact,
            &catalog,
            mode.compilation(),
        )
        .unwrap();
        let m = object.metadata();
        // This input's schema admits general two-dimensional matrices. A
        // row-shaped test value does not authorize a static unit-axis fact.
        let rows = [
            "compiled_numeric_write_loops",
            "compiled_readonly_numeric_loops",
        ]
        .into_iter()
        .flat_map(|k| m[k].as_array().unwrap())
        .collect::<Vec<_>>();
        assert!(!rows.is_empty(), "the native read/write loop must execute");
        assert!(
            rows.iter()
                .all(|r| r["canonical_vector_views"].as_array().unwrap().is_empty())
        );
        let pair = [
            JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                .unwrap(),
            JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::InvariantVectorGeometry,
            )
            .unwrap(),
        ];
        for (values, stop) in [
            (vec![], 0),
            (vec![], 1),
            (vec![3, 4, 5], 3),
            (vec![3, 4, 5], 4),
            (vec![u64::MAX], 1),
        ] {
            let memory = mode.memory_budget();
            let args = inputs(
                &artifact,
                &[
                    (
                        "data",
                        Data::Matrix(values.into_iter().map(Data::U64).collect()),
                    ),
                    ("stop", Data::U64(stop)),
                ],
                &memory,
            );
            let before = contents(&args);
            for budget in [None, Some(0), Some(1), Some(10_000)] {
                let mut expected = None;
                for jit in &pair {
                    for _ in 0..2 {
                        let mut policy = mode.execution();
                        if let Some(work) = budget {
                            policy.execution.max_work = work;
                        }
                        let mut output = vec![None; artifact.outputs().len()];
                        let result = jit.evaluate_into(&args, &mut output, policy, &memory);
                        let observed = (
                            format!("{result:?}"),
                            output
                                .iter()
                                .map(|v| v.as_ref().map(|v| v.canonical_data_draft().unwrap()))
                                .collect::<Vec<_>>(),
                        );
                        if let Some(prior) = &expected {
                            assert_eq!(&observed, prior);
                        } else {
                            expected = Some(observed);
                        }
                        assert_eq!(contents(&args), before);
                        drop(output);
                    }
                }
            }
            drop(args);
            assert_eq!(memory.used_bytes(), 0);
            assert_eq!(memory.retained_snapshot_nodes(), 0);
        }
    }
}
