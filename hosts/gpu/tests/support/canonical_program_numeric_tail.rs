//! Ordinary immutable arrays and scalar state; no compression-specific source.
use super::{allocation_probe, artifacts, equal, inputs, limits, reference};

fn witness_limits() -> mech_gpu::native_artifact::NativeProgramExecutionLimits {
    let mut policy = limits();
    // New diagnostic slope witness, declared independently of the unchanged
    // component profiles. The 256-element ordinary path costs billions of units.
    policy.execution.max_work = 1_000_000_000_000;
    policy.library.max_compute_work = Some(1_000_000_000_000);
    policy
}
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::JitProgramArtifact;

pub(super) const WEIGHTED: &str = "+> sequence\nget-word(values<[u32]>, at<u64>) => <u32>\n  | (values, at) => ((at < sequence/length(values))? | true => values[at + 1u64] | false => 0u32).\nget-byte(values<[u8]>, at<u64>) => <u8>\n  | (values, at) => ((at < sequence/length(values))? | true => values[at + 1u64] | false => 0u8).\nweighted(data<[u32]>, coefficients<[u8]>, i<u64>, end<u64>, total<u64>) => (out<u64>) :=\n  more := i < end\n  word<u64> := (more? | true => get-word(data, i) | false => 0u32)\n  byte<u64> := (more? | true => get-byte(coefficients, i) | false => 0u8)\n  next := total + word * byte\n  out := (more? | true => weighted(data, coefficients, i + 1u64, end, next) | false => total).\nweighted(data<[u32]>, coefficients<[u8]>, 0u64, steps<u64>, seed<u64>)\n";

#[test]
fn readonly_numeric_reduction_observes_real_allocation_and_transition_slopes() {
    for (form, artifact) in ["original", "decoded"]
        .into_iter()
        .zip(artifacts(WEIGHTED).0)
    {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        for steps in [0, 1, 8, 64, 256] {
            let args = inputs(
                &artifact,
                &[
                    (
                        "data",
                        Data::Matrix((0..256).map(|i| Data::U32(0x8000_0000 + i)).collect()),
                    ),
                    (
                        "coefficients",
                        Data::Matrix((0..256).map(|i| Data::U8((i % 3 + 1) as u8)).collect()),
                    ),
                    ("steps", Data::U64(steps)),
                    ("seed", Data::U64(17)),
                ],
                &memory,
            );
            let aliases = args.clone();
            let expected = (steps <= 8).then(|| reference(&artifact, &args));
            let mut output = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut output, witness_limits(), &memory)
                .unwrap();
            let (result, requests, bytes) = allocation_probe::measured_with_bytes(|| {
                jit.evaluate_into(&args, &mut output, witness_limits(), &memory)
            });
            let report = result.unwrap();
            if let Some(expected) = expected {
                equal(&artifact, &output, &expected);
            }
            let total = 17
                + (0..steps)
                    .map(|i| (0x8000_0000 + i) * (i % 3 + 1))
                    .sum::<u64>();
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::U64(total)
            );
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(a, b)| a.shares_frozen_storage(b))
            );
            println!(
                "{}",
                serde_json::json!({"witness":"readonly-weighted","artifact":form,"steps":steps,"requests":requests,"requested_bytes":bytes,"report":format!("{report:?}")})
            );
        }
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
}

#[test]
fn readonly_numeric_reduction_selects_a_native_leaf_region() {
    let artifact = artifacts(WEIGHTED).0.into_iter().next().unwrap();
    let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
    let metadata =
        mech_gpu::native_artifact::emit_program_object(&artifact, &mech_stdlib::source_catalog())
            .unwrap()
            .metadata();
    assert!(!jit.descriptor().plan().blocks().is_empty());
    assert!(
        metadata["readonly_numeric_regions"]
            .as_array()
            .is_some_and(|regions| !regions.is_empty()),
        "mixed carrier and array operations must run in a native leaf: {metadata}"
    );
}
