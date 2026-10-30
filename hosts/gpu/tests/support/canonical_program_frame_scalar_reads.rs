//! Ordinary parent frames expose current scalar words around native children.
use super::super::{artifacts, equal, inputs, reference_result};
use mech_core::ValueDataDraft as Data;
use mech_gpu::native_artifact::{JitProgramArtifact, NativeProgramLimitMode, emit_program_object};

pub(super) const SOURCE: &str = r#"inner(xs<[u32]>, left<u64>, total<u64>) => (out<u64>) :=
  word<u64> := xs[1u64]
  out := ((left > 0u64)? | true => inner(xs, left - 1u64, total + word) | false => total).
outer(xs<[u32]>, left<u64>, total<u64>, flag<bool>) => (out<(u64,bool)>) :=
  next := inner(xs, 1u64, total)
  out := ((left > 0u64)? | true => outer(xs, left - 1u64, next, flag == false) | false => (total, flag)).
(total, flag) := outer(data<[u32]>, steps<u64>, seed<u64>, flag<bool>)
(total + 0u64, flag == true)
"#;

#[test]
fn ordinary_frame_scalar_reads_execute_without_projection_callbacks() {
    for artifact in artifacts(SOURCE).0 {
        let metadata = emit_program_object(&artifact, &mech_stdlib::source_catalog())
            .unwrap()
            .metadata();
        assert!(
            metadata["compiled_frame_scalar_reads"]
                .as_array()
                .is_some_and(|r| !r.is_empty()),
            "ordinary parent-frame scalar projections still use callbacks"
        );
    }
}

#[test]
fn ordinary_frame_scalar_reads_keep_fresh_recursive_and_repeated_arguments() {
    for artifact in artifacts(SOURCE).0 {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let mut out = vec![None; artifact.outputs().len()];
        let mut rows = Vec::new();
        for (steps, flag, seed, word) in [
            (0, true, 17, 7),
            (1, false, 19, 11),
            (3, true, 29, 13),
            (8, false, 31, 17),
            (1, true, 37, 19),
        ] {
            let args = inputs(
                &artifact,
                &[
                    ("data", Data::Matrix(vec![Data::U32(word)].into())),
                    ("steps", Data::U64(steps)),
                    ("seed", Data::U64(seed)),
                    ("flag", Data::Bool(flag)),
                ],
                &memory,
            );
            let aliases = args.clone();
            let report = jit
                .evaluate_into(&args, &mut out, mode.execution(), &memory)
                .unwrap();
            equal(
                &artifact,
                &out,
                &reference_result(&artifact, &args).unwrap(),
            );
            assert_eq!(
                out.last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Tuple(
                    vec![
                        Data::U64(seed + steps * u64::from(word)),
                        Data::Bool(flag ^ (steps % 2 == 1))
                    ]
                    .into()
                )
            );
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(a, b)| a.shares_frozen_storage(b))
            );
            rows.push((steps, report.visits, report.work));
        }
        println!(
            "{}",
            serde_json::json!({"witness":"ordinary-frame-scalar-reads","observations":rows})
        );
        drop(out);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
