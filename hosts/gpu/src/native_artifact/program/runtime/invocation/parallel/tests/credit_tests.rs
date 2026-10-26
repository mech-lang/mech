//! Actual emitted counting, all original windows and complete output equality.
use crate::native_artifact::program::runtime::invocation::parallel::tests::*;

#[test]
fn funded_worker_credits_preserve_all_seven_maps_and_original_decoded_source_results() {
    let source = format!(
        r#"{}
+> text
+> sequence
+> number
count(s<string>, w<u64>) => (out<string>) :=
  n := sequence/length(s)
  h := #Count(s, w, n, empty)
  key := s[1u64..=w]
  out := number/format(h{{key}}, 0u64).
text := seed<string>
text/join([count(text, w) | w <- [1u64 2u64 3u64 4u64 6u64 12u64 18u64]], ",")
"#,
        crate::native_artifact::program::runtime::structural_projection_tests::COUNT
    );
    let original = crate::native_artifact::program::runtime::tests::artifact_source(&source);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    struct Restore(Option<usize>);
    impl Drop for Restore {
        fn drop(&mut self) {
            crate::native_artifact::program::runtime::invocation::parallel::TEST_WORKERS
                .with(|v| v.set(self.0));
        }
    }
    let _restore = Restore(
        crate::native_artifact::program::runtime::invocation::parallel::TEST_WORKERS
            .with(|v| v.get()),
    );
    for artifact in [original, decoded] {
        let jit = crate::native_artifact::JitProgramArtifact::compile(
            &artifact,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        for (text, period) in [("a".repeat(4096), 1usize), ("ab:".repeat(1366), 3)] {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let raw = ValueDraft {
                schema: artifact.inputs()[0].schema,
                shape_values: Box::new([]),
                data: ValueDataDraft::String(text.clone()),
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let mut reservation = memory
                .reserve_capacity(raw.memory_budget_admission_bytes(&memory).unwrap())
                .unwrap();
            let args = [raw.into_memory_budget(&mut reservation).unwrap()];
            drop(reservation);
            let baseline = (memory.used_bytes(), memory.retained_nodes());
            let expected = [1usize, 2, 3, 4, 6, 12, 18]
                .map(|width| ((text.len() - width) / period + 1).to_string())
                .join(",");
            let mut snapshots = Vec::new();
            for workers in [1, 7] {
                crate::native_artifact::program::runtime::invocation::parallel::TEST_WORKERS
                    .with(|v| v.set(Some(workers)));
                let mut limits = crate::native_artifact::program::runtime::tests::limits();
                limits.execution.max_work = 1 << 60;
                limits.library.max_compute_work = Some(1 << 56);
                limits.library.max_comparison_work = Some(1 << 56);
                limits.library.max_retained_nodes = Some(1_048_576);
                let mut outputs = vec![None; artifact.outputs().len()];
                let report = jit
                    .evaluate_into(&args, &mut outputs, limits, &memory)
                    .unwrap();
                assert_eq!(report.hash_table_regions, 7);
                assert_eq!(report.hash_table_updates, 7 * text.len() as u64 - 39);
                if workers == 1 {
                    assert_eq!(
                        (report.worker_credit_batches, report.worker_credit_claims),
                        (0, 0)
                    );
                } else {
                    assert_eq!(report.parallel_tasks, 7);
                    assert!(
                        report.worker_credit_batches > 0 && report.worker_credit_claims > 0,
                        "actual emitted refill must select finite credit: {report:?}"
                    );
                }
                let output = outputs.last().unwrap().as_ref().unwrap();
                assert!(matches!(output.data(), ValueData::String(s) if s.as_ref() == expected));
                snapshots.push(output.canonical_snapshot_bytes(artifact.schemas()).unwrap());
                drop(outputs);
                assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
            }
            assert_eq!(snapshots[0], snapshots[1]);
            drop(args);
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
        }
    }
}
