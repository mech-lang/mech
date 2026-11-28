//! Ordinary transpose induction retains the old native loop as an ablation.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;
use mech_core::snapshot::F64Bits;

fn data(bits: &[u64]) -> Data {
    Data::Matrix(
        bits.iter()
            .map(|&b| Data::F64(F64Bits::from_bits(b)))
            .collect(),
    )
}
fn outputs(values: &[Option<Value>]) -> Vec<Data> {
    values
        .iter()
        .map(|v| v.as_ref().unwrap().canonical_data_draft().unwrap())
        .collect()
}
fn run(
    jit: &JitProgramArtifact,
    args: &[Value],
    out: &mut [Option<Value>],
    policy: NativeProgramExecutionLimits,
    memory: &ManagedMemoryBudget,
    restored: bool,
) -> (
    Result<NativeProgramExecutionReport, NativeProgramExecutionError>,
    observe::Counts,
) {
    if restored {
        observe::run_restored(jit, args, out, policy, memory)
    } else {
        observe::run(jit, args, out, policy, memory)
    }
}

#[test]
fn numeric_transpose_induction_original_decoded_restored_bits_oracle_roundtrip_and_aliases() {
    let words = [
        0,
        1u64 << 63,
        1,
        u64::MAX,
        0x7ff8_0000_0000_0042,
        0x7ff0_0000_0000_0031,
        0xfff8_0000_0000_0064,
        f64::INFINITY.to_bits(),
        f64::NEG_INFINITY.to_bits(),
        (-7.5f64).to_bits(),
    ];
    // The ordinary resident binding stores each axis as u32. Exercise its
    // largest representable empty extent; the isolated admitted loop also
    // checks u64::MAX with null banks.
    let huge = u32::MAX as u64;
    for [rows, columns] in [
        [0, 0],
        [0, 7],
        [7, 0],
        [1, 1],
        [1, 9],
        [9, 1],
        [2, 3],
        [3, 2],
        [7, 9],
        [64, 64],
        [0, huge],
        [huge, 0],
    ] {
        for fixed in [false, true] {
            let axes = if fixed {
                format!(":{rows},{columns}")
            } else {
                String::new()
            };
            let source =
                format!("data := input<[f64]{axes}>\nanswer := data'\n(answer,answer',data)\n");
            for artifact in artifacts(&source) {
                let catalog = mech_stdlib::source_catalog();
                let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
                let flat = JitProgramArtifact::compile_for_test(
                    &artifact,
                    &catalog,
                    OptimizationPass::TransposeInduction,
                )
                .unwrap();
                crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
                let mode = NativeProgramLimitMode::NoLimits;
                let memory = mode.memory_budget();
                let bits = (0..rows * columns)
                    .map(|i| words[i as usize % words.len()])
                    .collect::<Vec<_>>();
                let input = data(&bits);
                let args = numeric_maps::typed_arguments(
                    &artifact,
                    &memory,
                    &[("input", input.clone(), &[rows, columns])],
                );
                let aliases = args.clone();
                let mut transposed = Vec::new();
                // Empty axes must not be iterated, even when one is enormous.
                if rows * columns != 0 {
                    for column in 0..columns {
                        for row in 0..rows {
                            transposed.push(bits[(row * columns + column) as usize]);
                        }
                    }
                }
                let oracle =
                    Data::Tuple(vec![data(&transposed), input.clone(), input.clone()].into());
                let baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
                for restored in [false, true] {
                    let mut actual = vec![None; artifact.outputs().len()];
                    let mut expected = actual.clone();
                    let (a, counts) = run(
                        &native,
                        &args,
                        &mut actual,
                        mode.execution(),
                        &memory,
                        restored,
                    );
                    let (b, old_counts) = run(
                        &flat,
                        &args,
                        &mut expected,
                        mode.execution(),
                        &memory,
                        restored,
                    );
                    assert_eq!(
                        a.as_ref().map_err(|e| format!("{e:?}")),
                        b.as_ref().map_err(|e| format!("{e:?}")),
                        "{rows}x{columns}, fixed{fixed}, restored{restored}"
                    );
                    a.unwrap_or_else(|e| {
                        panic!("{rows}x{columns}, fixed{fixed}, restored{restored}: {e:?}")
                    });
                    assert_eq!(counts.helpers, old_counts.helpers);
                    assert!(counts.helpers.iter().any(|((_, op), &n)| *op
                        == GlobalHelper::NumericBinaryFinish.raw()
                        && n > 0));
                    assert_eq!(
                        outputs(&actual),
                        vec![oracle.clone()],
                        "{rows}x{columns}, fixed{fixed}"
                    );
                    assert_eq!(outputs(&actual), outputs(&expected));
                    assert_eq!(contents(&args), vec![input.clone()]);
                    assert_eq!(contents(&aliases), vec![input.clone()]);
                    assert!(aliases[0].shares_frozen_storage(&args[0]));
                    drop((actual, expected));
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        baseline
                    );
                }
                drop((aliases, args));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}

#[test]
fn numeric_transpose_induction_finite_missing_caps_keep_atomic_outputs_fees_and_retry() {
    for artifact in artifacts("input<[f64]>'\n") {
        let catalog = mech_stdlib::source_catalog();
        let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let flat = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::TransposeInduction,
        )
        .unwrap();
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let args = numeric_maps::typed_arguments(
            &artifact,
            &memory,
            &[("input", data(&[1, 2, 3, 4, 5, 6]), &[2, 3])],
        );
        let prior_args = numeric_maps::typed_arguments(
            &artifact,
            &memory,
            &[("input", data(&[7, 8, 9, 10, 11, 12]), &[2, 3])],
        );
        let mut prior = vec![None; artifact.outputs().len()];
        run(
            &flat,
            &prior_args,
            &mut prior,
            mode.execution(),
            &memory,
            false,
        )
        .0
        .unwrap();
        drop(prior_args);
        let baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
        for restored in [false, true] {
            for dimension in 0..12 {
                for cap in [None, Some(0), Some(1), Some(4096), Some(u64::MAX)] {
                    let mut policy = mode.execution();
                    match dimension {
                        0 => policy.library.max_output_elements = cap,
                        1 => policy.library.max_output_bytes = cap,
                        2 => policy.library.max_temporary_bytes = cap,
                        3 => policy.library.max_cloned_bytes = cap,
                        4 => policy.library.max_retained_nodes = cap,
                        5 => policy.library.max_compute_work = cap,
                        6 => policy.library.max_comparison_work = cap,
                        7 => policy.library.max_scalar_instructions = cap,
                        8 => policy.library.max_transfer_bytes = cap,
                        9 => policy.library.max_storage_buffer_bytes = cap,
                        10 => {
                            policy.library.max_storage_bindings =
                                cap.map(|n| n.min(u32::MAX as u64) as u32)
                        }
                        _ => policy.execution.max_work = cap.unwrap_or(u64::MAX),
                    }
                    let mut actual = prior.clone();
                    let mut expected = prior.clone();
                    let (a, _) = run(&native, &args, &mut actual, policy, &memory, restored);
                    let (b, _) = run(&flat, &args, &mut expected, policy, &memory, restored);
                    assert_eq!(a.is_ok(), b.is_ok(), "{dimension} {cap:?}: {a:?}/{b:?}");
                    if let (Ok(a), Ok(b)) = (&a, &b) {
                        assert_eq!(a, b);
                    }
                    assert_eq!(outputs(&actual), outputs(&expected));
                    if a.is_err() {
                        for (a, p) in actual.iter().zip(&prior) {
                            assert!(
                                a.as_ref()
                                    .unwrap()
                                    .shares_frozen_storage(p.as_ref().unwrap())
                            );
                        }
                    }
                    drop((actual, expected));
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        baseline
                    );
                }
            }
            let mut retry = prior.clone();
            run(
                &native,
                &args,
                &mut retry,
                mode.execution(),
                &memory,
                restored,
            )
            .0
            .unwrap();
            assert_ne!(outputs(&retry), outputs(&prior));
            drop(retry);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                baseline
            );
        }
        drop((args, prior));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
