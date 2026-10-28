//! Real buffer fields keep packed storage through the maintained access leaf.
use super::{allocation_probe, artifacts, equal, inputs, limits, reference};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::{
    JitProgramArtifact, NativeIterationExecutionError, NativeProgramExecutionError,
};

#[test]
fn real_word_buffer_fields_preserve_publication_aliases_and_recover_after_failure() {
    let source = "project(table<[u16]>, cache<[u32]>, divisor<u64>) => (out<([u16], [u32], u64)>) :=\n  fields := {table: table, cache: cache}\n  a := fields.table\n  b := fields.cache\n  out := (a, b, 12u64 / divisor).\nproject(table<[u16]>, cache<[u32]>, divisor<u64>)\n";
    for (form, artifact) in ["original", "decoded"].into_iter().zip(artifacts(source).0) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut policy = limits();
        policy.library.max_output_elements = Some(1_500_513);
        policy.library.max_compute_work = Some(10_000_000_000);
        policy.library.max_comparison_work = Some(10_000_000_000);
        policy.execution.max_work = 10_000_000_000;
        let mut output = vec![None; artifact.outputs().len()];
        for (table_len, cache_len, divisor) in [
            (3, 5, 1),
            (262_144, 1_500_513, 2),
            (262_144, 1_500_513, 0),
            (262_144, 1_500_513, 3),
            (0, 0, 4),
        ] {
            let args = inputs(
                &artifact,
                &[
                    (
                        "table",
                        Data::Matrix(vec![Data::U16(0xf123); table_len].into()),
                    ),
                    (
                        "cache",
                        Data::Matrix(vec![Data::U32(0xf0123456); cache_len].into()),
                    ),
                    ("divisor", Data::U64(divisor)),
                ],
                &memory,
            );
            let aliases = args.clone();
            let prior = output.clone();
            let before = memory.used_bytes();
            let start = std::time::Instant::now();
            let (result, allocations, bytes) = allocation_probe::measured_with_bytes(|| {
                jit.evaluate_into(&args, &mut output, policy, &memory)
            });
            println!(
                "{}",
                serde_json::json!({"witness":"packed-word-fields",
                "artifact":form,"table_entries":table_len,"cache_words":cache_len,
                "divisor":divisor,"elapsed_seconds":start.elapsed().as_secs_f64(),
                "allocation_requests":allocations,"requested_allocation_bytes":bytes,
                "result":format!("{result:?}")})
            );
            if divisor == 0 {
                assert!(
                    matches!(
                        result,
                        Err(NativeProgramExecutionError::Control(
                            NativeIterationExecutionError::Arithmetic { .. }
                        ))
                    ),
                    "{result:?}"
                );
                assert_eq!(memory.used_bytes(), before);
                assert!(output.iter().zip(&prior).all(|(value, old)| {
                    value
                        .as_ref()
                        .unwrap()
                        .shares_frozen_storage(old.as_ref().unwrap())
                }));
            } else {
                result.unwrap();
                if cache_len <= 5 {
                    equal(&artifact, &output, &reference(&artifact, &args));
                }
                let mech_core::ValueData::Tuple(fields) =
                    output.last().unwrap().as_ref().unwrap().data()
                else {
                    panic!()
                };
                let [
                    mech_core::ValueData::Matrix(a),
                    mech_core::ValueData::Matrix(b),
                    mech_core::ValueData::U64(value),
                ] = fields.as_ref()
                else {
                    panic!()
                };
                assert_eq!(*value, 12 / divisor);
                let mech_core::snapshot::SequenceView::U16(a) = a.elements() else {
                    panic!()
                };
                let mech_core::snapshot::SequenceView::U32(b) = b.elements() else {
                    panic!()
                };
                assert_eq!((a.len(), b.len()), (table_len, cache_len));
                assert!(a.iter().all(|word| *word == 0xf123));
                assert!(b.iter().all(|word| *word == 0xf0123456));
            }
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(value, alias)| value.shares_frozen_storage(alias))
            );
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}
