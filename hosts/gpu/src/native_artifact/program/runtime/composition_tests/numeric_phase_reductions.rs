//! Fused source folds inside the existing numeric owner/fuel phase.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;
use crate::native_artifact::program::prepare::readonly_numeric::Operation;
use mech_core::snapshot::F64Bits;

const SOURCE: &str = r#"+> stats
#Fold(initial<[f64]:2,4>, first<u64>, last<u64>, row<u64>, divisor<f64>, rounds<u64>) => <[f64]:2,4>
  | :Step(data<[f64]:2,4>, pass<u64>)
  | :Done(data<[f64]:2,4>).
#Fold(initial, first, last, row, divisor, rounds) -> :Step(initial, 0u64)
  :Step(data, pass)
    | pass < rounds -> {
        total := stats/sum/column([(k <= 4u64? | true => data[row,k] / divisor | false => 0.0) | k <- first..last])[1u64]
        ~next := data
        next[1u64,1u64] = total
      } -> :Step(next, pass + 1u64)
    | * -> :Done(data)
  :Done(data) => data.
data := input<[f64]:2,4>
answer := #Fold(data,first<u64>,last<u64>,row<u64>,divisor<f64>,rounds<u64>)
(answer,data)
"#;

fn folds(
    jit: &JitProgramArtifact,
) -> Vec<(
    usize,
    &crate::native_artifact::program::prepare::numeric_phase::Fold,
)> {
    jit.descriptor()
        .metadata()
        .closed_tails
        .regions
        .iter()
        .flat_map(|r| {
            r.numeric.iter().flat_map(|p| {
                p.actions.iter().filter_map(|(site, a)| {
                    if let Operation::Reduce { fold } = a {
                        Some((*site, fold.as_ref()))
                    } else {
                        None
                    }
                })
            })
        })
        .collect()
}
#[test]
fn numeric_phase_reductions_keep_current_versions_order_bits_cold_errors_and_retry() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
            .unwrap();
        if folds(&jit).is_empty() {
            let m = jit.descriptor().metadata();
            for control in m.plan.iterations() {
                if let mech_engine::native_artifact::NativeControlLoop::Tail(tail) = control {
                    eprintln!(
                        "fold phase {:?}",
                        crate::native_artifact::program::prepare::numeric_prefix::derive(
                            &m.plan.facts(),
                            tail,
                            &m.buffer_frames,
                            &m.scalar_aggregates,
                            &m.calls,
                            &m.visits
                        )
                        .err()
                    );
                }
            }
        }
        assert!(
            !folds(&jit).is_empty(),
            "source fold is composed into a native phase"
        );
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::ScalarLoops,
        )
        .unwrap();
        crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
        let memory = mode.memory_budget();
        let mut fast = vec![None; artifact.outputs().len()];
        let mut cold = fast.clone();
        for restored in [false, true] {
            for (first, last, row, divisor, rounds, expect_error) in [
                (1, 5, 1, 1.0, 1, false),
                (1, 5, 1, 1.0, 3, false),
                (1, 7, 1, 1.0, 2, false), // Larger than the native envelope: original source.
                (1, 5, 2, 1.0, 2, false),
                (1, 5, 0, 0.0, 0, false),
                (1, 1, 1, 1.0, 1, true),
                (3, 1, 1, 1.0, 1, true),
                (0, 1, 1, 1.0, 1, true),
                (1, 5, 3, 1.0, 1, true),
            ] {
                let args = super::numeric_maps::typed_arguments(
                    &artifact,
                    &memory,
                    &[
                        (
                            "input",
                            Data::Matrix(
                                [1.0e16, 1.0, -1.0e16, 3.0, -0.0, -0.0, -0.0, -0.0]
                                    .map(|x| Data::F64(F64Bits::from_f64(x)))
                                    .to_vec()
                                    .into(),
                            ),
                            &[2, 4],
                        ),
                        ("first", Data::U64(first), &[]),
                        ("last", Data::U64(last), &[]),
                        ("row", Data::U64(row), &[]),
                        ("rounds", Data::U64(rounds), &[]),
                        ("divisor", Data::F64(F64Bits::from_f64(divisor)), &[]),
                    ],
                );
                let aliases = args.clone();
                let before = contents(&args);
                let old = contents(&fast.iter().flatten().cloned().collect::<Vec<_>>());
                let (a, counts) = if restored {
                    observe::run_restored(&jit, &args, &mut fast, mode.execution(), &memory)
                } else {
                    observe::run(&jit, &args, &mut fast, mode.execution(), &memory)
                };
                let (b, _) = observe::run(&ordinary, &args, &mut cold, mode.execution(), &memory);
                assert_eq!(
                    a.is_err(),
                    expect_error,
                    "{first}..{last} row {row}: {a:?} {counts:?}"
                );
                assert_eq!(a.is_err(), b.is_err());
                if expect_error {
                    assert_eq!(
                        format!("{:?}", a.as_ref().err()),
                        format!("{:?}", b.as_ref().err()),
                        "exact original error"
                    );
                    assert_eq!(
                        contents(&fast.iter().flatten().cloned().collect::<Vec<_>>()),
                        old
                    );
                }
                assert_eq!(
                    contents(&fast.iter().flatten().cloned().collect::<Vec<_>>()),
                    contents(&cold.iter().flatten().cloned().collect::<Vec<_>>()),
                    "exact bits {first}..{last} row {row}"
                );
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
                if !expect_error && rounds > 0 && last <= 5 {
                    for (prepare, fold) in folds(&jit) {
                        assert_eq!(
                            counts.admits.get(&(prepare as u64)).copied().unwrap_or(0),
                            0
                        );
                        for site in fold.source.region.reads.iter().map(|r| r.site).chain(
                            fold.source
                                .region
                                .reduction
                                .iter()
                                .flat_map(|r| [r.sum, r.select]),
                        ) {
                            assert_eq!(
                                counts
                                    .helpers
                                    .get(&(site as u64, GlobalHelper::Library.raw()))
                                    .copied()
                                    .unwrap_or(0),
                                0,
                                "no per-fold host operation at {site}: {counts:?}"
                            );
                        }
                    }
                }
            }
        }
        fast.fill(None);
        cold.fill(None);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn numeric_phase_reductions_select_the_existing_cholesky_source_without_rewriting() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/polybench/cholesky.mec");
    let source = std::fs::read_to_string(path).unwrap();
    for artifact in artifacts(&source) {
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        if folds(&jit).is_empty() {
            let m = jit.descriptor().metadata();
            for control in m.plan.iterations() {
                if let mech_engine::native_artifact::NativeControlLoop::Tail(tail) = control {
                    eprintln!(
                        "cholesky prefix {:?}",
                        crate::native_artifact::program::prepare::numeric_prefix::derive(
                            &m.plan.facts(),
                            tail,
                            &m.buffer_frames,
                            &m.scalar_aggregates,
                            &m.calls,
                            &m.visits
                        )
                        .err()
                    );
                }
            }
        }
        assert_eq!(
            folds(&jit).len(),
            2,
            "both ordered source reductions share the phase"
        );
    }
}

#[test]
fn numeric_phase_reductions_images_rederive_source_bindings_views_and_all_funding() {
    for artifact in artifacts(SOURCE) {
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        assert!(!folds(&jit).is_empty());
        let bytes = jit.descriptor().encode_build_owned_image().unwrap();
        let image: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let regions = image["descriptor"]["closed_tails"]["regions"]
            .as_array()
            .unwrap();
        let (region, action) = regions
            .iter()
            .enumerate()
            .find_map(|(r, region)| {
                region["numeric"]["actions"]
                    .as_array()?
                    .iter()
                    .enumerate()
                    .find_map(|(a, action)| action[1]["Reduce"].is_object().then_some((r, a)))
            })
            .unwrap();
        // Exact build-owned metadata still restores the same descriptor.
        let restored = super::super::tests::restore_admitted_helper_descriptor(&jit);
        assert_eq!(
            restored.typed_ir_sha256(),
            jit.descriptor().typed_ir_sha256()
        );
        let base = format!(
            "/descriptor/closed_tails/regions/{region}/numeric/actions/{action}/1/Reduce/fold"
        );
        for field in [
            "source/entry",
            "source/prepare",
            "source/after",
            "source/first",
            "source/last",
            "source/maximum",
            "source/region/matrices/0/witness",
            "source/region/matrices/0/value",
            "arrays/0",
            "budget/schema_bytes",
            "budget/compute",
            "budget/replay",
            "budget/temporary",
            "budget/nodes",
            "budget/output",
            "budget/elements",
            "budget/cloned",
            "budget/comparison",
            "budget/setup",
            "budget/native_step",
        ] {
            let mut forged = image.clone();
            let path = format!("{base}/{field}");
            let word = forged
                .pointer_mut(&path)
                .unwrap_or_else(|| panic!("missing fold proof field {path}"));
            *word = (word.as_u64().unwrap() + 1).into();
            assert!(
                super::super::tests::rejects_malformed_helper_image(
                    &serde_json::to_vec(&forged).unwrap()
                ),
                "forged {field}"
            );
        }
        let mut forged = image.clone();
        let path = format!("{base}/source/inclusive");
        let word = forged.pointer_mut(&path).unwrap();
        *word = (!word.as_bool().unwrap()).into();
        assert!(super::super::tests::rejects_malformed_helper_image(
            &serde_json::to_vec(&forged).unwrap()
        ));
        let mut forged = image.clone();
        let budget = forged
            .pointer_mut(&format!("{base}/budget"))
            .unwrap()
            .as_object_mut()
            .unwrap();
        budget.remove("replay");
        assert!(super::super::tests::rejects_malformed_helper_image(
            &serde_json::to_vec(&forged).unwrap()
        ));
    }
}

#[test]
fn numeric_phase_reductions_preserve_partial_arithmetic_failure_with_finite_replay() {
    let source = SOURCE.replace(
        "data[row,k] / divisor",
        "data[row,k + k / (3u64 - k)] / divisor",
    );
    for artifact in artifacts(&source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
            .unwrap();
        assert!(!folds(&jit).is_empty());
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::ScalarLoops,
        )
        .unwrap();
        let memory = mode.memory_budget();
        let args = super::numeric_maps::typed_arguments(
            &artifact,
            &memory,
            &[
                (
                    "input",
                    Data::Matrix(
                        [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]
                            .map(|x| Data::F64(F64Bits::from_f64(x)))
                            .into(),
                    ),
                    &[2, 4],
                ),
                ("first", Data::U64(1), &[]),
                ("last", Data::U64(4), &[]),
                ("row", Data::U64(1), &[]),
                ("rounds", Data::U64(1), &[]),
                ("divisor", Data::F64(F64Bits::from_f64(1.0)), &[]),
            ],
        );
        let retained = memory.used_bytes();
        let mut arithmetic = 0;
        for restored in [false, true] {
            for dimension in 0..9 {
                for cap in [0, 1, 16, 255, 4095, 65535, 1_000_000, 1_000_000_000_000] {
                    let mut policy = mode.execution();
                    match dimension {
                        0 => policy.library.max_output_elements = Some(cap),
                        1 => policy.library.max_output_bytes = Some(cap),
                        2 => policy.library.max_temporary_bytes = Some(cap),
                        3 => policy.library.max_cloned_bytes = Some(cap),
                        4 => policy.library.max_retained_nodes = Some(cap),
                        5 => policy.library.max_compute_work = Some(cap),
                        6 => policy.library.max_comparison_work = Some(cap),
                        7 => policy.execution.max_work = cap,
                        _ => policy.library.max_scalar_instructions = Some(cap),
                    }
                    let mut cold = vec![None; artifact.outputs().len()];
                    let mut fast = cold.clone();
                    let (a, _) = observe::run(&ordinary, &args, &mut cold, policy, &memory);
                    let (b, counts) = if restored {
                        observe::run_restored(&jit, &args, &mut fast, policy, &memory)
                    } else {
                        observe::run(&jit, &args, &mut fast, policy, &memory)
                    };
                    let a = format!("{:?}", a.unwrap_err());
                    let b = format!("{:?}", b.unwrap_err());
                    if a.contains("Arithmetic") {
                        arithmetic += 1;
                        assert_eq!(
                            a, b,
                            "source error dimension {dimension} cap {cap}: {counts:?}"
                        );
                    }
                    assert!(cold.iter().all(Option::is_none) && fast.iter().all(Option::is_none));
                    drop((cold, fast));
                    assert_eq!(memory.used_bytes(), retained);
                }
            }
        }
        assert!(arithmetic > 0);
        drop(args);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn numeric_phase_reductions_inclusive_endpoints_keep_overflow_and_inactive_reads() {
    let source = SOURCE.replace("first..last", "first..=last");
    for artifact in artifacts(&source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
            .unwrap();
        assert!(!folds(&jit).is_empty());
        assert!(folds(&jit).iter().all(|(_, fold)| fold.source.inclusive));
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::ScalarLoops,
        )
        .unwrap();
        let memory = mode.memory_budget();
        let mut fast = vec![None; artifact.outputs().len()];
        let mut cold = fast.clone();
        for restored in [false, true] {
            for (first, last, row, error) in [
                (1, 4, 1, false),
                (1, 5, 1, false),
                (u64::MAX, u64::MAX, 0, false),
                (0, u64::MAX, 0, true),
                (4, 3, 1, true),
                (1, 4, 0, true),
                (0, 0, 1, true),
                (2, 2, 2, false),
            ] {
                let args = super::numeric_maps::typed_arguments(
                    &artifact,
                    &memory,
                    &[
                        (
                            "input",
                            Data::Matrix(
                                [1.0e16, 1.0, -1.0e16, 3.0, -0.0, -0.0, -0.0, -0.0]
                                    .map(|x| Data::F64(F64Bits::from_f64(x)))
                                    .into(),
                            ),
                            &[2, 4],
                        ),
                        ("first", Data::U64(first), &[]),
                        ("last", Data::U64(last), &[]),
                        ("row", Data::U64(row), &[]),
                        ("rounds", Data::U64(1), &[]),
                        ("divisor", Data::F64(F64Bits::from_f64(1.0)), &[]),
                    ],
                );
                let aliases = args.clone();
                let before = contents(&args);
                let old = contents(&fast.iter().flatten().cloned().collect::<Vec<_>>());
                let (a, counts) = if restored {
                    observe::run_restored(&jit, &args, &mut fast, mode.execution(), &memory)
                } else {
                    observe::run(&jit, &args, &mut fast, mode.execution(), &memory)
                };
                let (b, _) = observe::run(&ordinary, &args, &mut cold, mode.execution(), &memory);
                assert_eq!(a.is_err(), error, "{first}..={last} row {row}: {a:?}");
                assert_eq!(a.is_err(), b.is_err());
                if !error
                    && last
                        .checked_sub(first)
                        .and_then(|n| n.checked_add(1))
                        .is_some_and(|n| n <= 4)
                {
                    for (_, fold) in folds(&jit) {
                        assert_eq!(
                            counts
                                .helpers
                                .get(&(
                                    u64::from(fold.source.region.header),
                                    GlobalHelper::NumericMapPrepare.raw()
                                ))
                                .copied()
                                .unwrap_or(0),
                            0,
                            "inclusive fold stays native, including an inactive invalid read"
                        );
                    }
                }
                if error {
                    assert_eq!(
                        format!("{:?}", a.as_ref().err()),
                        format!("{:?}", b.as_ref().err())
                    );
                    assert_eq!(
                        contents(&fast.iter().flatten().cloned().collect::<Vec<_>>()),
                        old
                    );
                }
                assert_eq!(
                    contents(&fast.iter().flatten().cloned().collect::<Vec<_>>()),
                    contents(&cold.iter().flatten().cloned().collect::<Vec<_>>())
                );
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
            }
        }
        fast.fill(None);
        cold.fill(None);
        assert_eq!(memory.used_bytes(), 0);
    }
}
