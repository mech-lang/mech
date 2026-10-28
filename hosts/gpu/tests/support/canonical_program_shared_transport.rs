//! Ordinary managed copies share one immutable owner and its prepaid cleanup.
use super::{allocation_probe, artifacts, inputs, limits};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::{
    JitProgramArtifact, NativeIterationExecutionError, NativeProgramExecutionError,
};

#[test]
fn repeated_buffer_returns_share_storage_keep_finite_work_and_recover() {
    let source = "allowed := permit<bool>\nrelay(value<[u32]>) => (out<[u32]>) :=\n  permitted! := allowed\n  out := value.\npayload<[u32]> |64> relay\n";
    for (form, artifact) in ["original", "decoded"].into_iter().zip(artifacts(source).0) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (count, word, permitted) in [
            (31, 17u32, true),
            (1_500_513, 0xf0123456, true),
            (1_500_513, 0x87654321, false),
            (1_500_513, 0x87654321, true),
        ] {
            let args = inputs(
                &artifact,
                &[
                    ("payload", Data::Matrix(vec![Data::U32(word); count].into())),
                    ("permit", Data::Bool(permitted)),
                ],
                &memory,
            );
            let aliases = args.clone();
            let prior = output.clone();
            let before = memory.used_bytes();
            let start = std::time::Instant::now();
            let (result, allocations, bytes) = allocation_probe::measured_with_bytes(|| {
                jit.evaluate_into(&args, &mut output, limits(), &memory)
            });
            println!(
                "{}",
                serde_json::json!({"witness":"repeated-buffer-returns", "artifact":form,
                    "words":count, "permitted":permitted,
                    "elapsed_seconds":start.elapsed().as_secs_f64(),
                    "allocation_requests":allocations, "requested_allocation_bytes":bytes,
                    "result":format!("{result:?}")})
            );
            if permitted {
                result.unwrap();
                let selected = output.last().unwrap().as_ref().unwrap();
                let source = args
                    .iter()
                    .find(|value| matches!(value.data(), mech_core::ValueData::Matrix(_)))
                    .unwrap();
                assert!(selected.shares_frozen_storage(source));
                if count == 1_500_513 {
                    assert!(
                        bytes < count * 4,
                        "{allocations} allocations, {bytes} bytes"
                    );
                }
            } else {
                assert!(matches!(
                    result,
                    Err(NativeProgramExecutionError::Control(
                        NativeIterationExecutionError::Integrity { .. }
                    ))
                ));
                assert_eq!(memory.used_bytes(), before);
                assert!(output.iter().zip(&prior).all(|(value, old)| {
                    value
                        .as_ref()
                        .unwrap()
                        .shares_frozen_storage(old.as_ref().unwrap())
                }));
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
