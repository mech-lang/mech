//! Scalar choices in the real numeric-tree path, with ordinary-emission ablation.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;

const SOURCE: &str = r#"+> bits
+> sequence
choose(a<u64>, b<u64>) => (out<(u64,u64,u64,u64)>) :=
  low := ((a < b)? | true => a | false => b)
  high := ((a < b)? | true => b | false => a)
  tie := ((a == b)? | true => 18446744073709551615u64 | false => 0u64)
  out := (low, high, tie, bits/xor(low, high)).
choose-step(values<[u64]>, pivot<u64>, at<u64>, output<[u64]>) => (out<[u64]>) :=
  selected := choose(values[at + 1u64], pivot)
  a := sequence/append(output, selected.1)
  b := sequence/append(a, selected.2)
  c := sequence/append(b, selected.3)
  out := sequence/append(c, selected.4).
choose-many(values<[u64]>, pivot<u64>, at<u64>, end<u64>, output<[u64]>) => (out<[u64]>) :=
  more := at < end
  next := (more? | true => choose-step(values, pivot, at, output) | false => output)
  out := (more? | true => choose-many(values, pivot, at + 1u64, end, next) | false => output).
data := words<[u64]>
stop := end<u64>
selected := choose-many(data, pivot<u64>, 0u64, stop, sequence/with-capacity(0u64, stop * 4u64))
(selected, data)
"#;

fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    words: &[u64],
    pivot: u64,
    end: u64,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            (
                "words",
                Data::Matrix(words.iter().copied().map(Data::U64).collect()),
            ),
            ("pivot", Data::U64(pivot)),
            ("end", Data::U64(end)),
        ],
        memory,
    )
}

#[test]
fn scalar_select_source_keeps_reports_errors_aliases_and_retry() {
    source_keeps_reports_errors_aliases_and_retry(OptimizationPass::ScalarSelects);
}

#[test]
fn unsigned_guard_source_keeps_reports_errors_aliases_and_retry() {
    source_keeps_reports_errors_aliases_and_retry(OptimizationPass::UnsignedGuardFacts);
}

fn source_keeps_reports_errors_aliases_and_retry(pass: OptimizationPass) {
    let mut words = vec![0, 1, u64::MAX, 1 << 63, (1 << 63) - 1];
    let mut seed = 0x7542_3918_abcd_0053u64;
    for _ in 0..64 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        words.push(seed);
    }
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let native =
            JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                .unwrap();
        let ordinary = JitProgramArtifact::compile_for_test(&artifact, &catalog, pass).unwrap();
        assert!(
            match pass {
                OptimizationPass::ScalarSelects => native.test_emissions().scalar_selects,
                OptimizationPass::UnsignedGuardFacts => native.test_emissions().unsigned_guards,
                _ => unreachable!(),
            } > 0,
            "real source must actually select the transform"
        );
        assert_eq!(
            match pass {
                OptimizationPass::ScalarSelects => ordinary.test_emissions().scalar_selects,
                OptimizationPass::UnsignedGuardFacts => ordinary.test_emissions().unsigned_guards,
                _ => unreachable!(),
            },
            0,
            "the ablation must disable the transform even when backend code sizes converge"
        );
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut outputs = [
                vec![None; artifact.outputs().len()],
                vec![None; artifact.outputs().len()],
            ];
            for pivot in [0, u64::MAX, 1 << 63, seed] {
                for end in [words.len() as u64, 0, words.len() as u64 + 1, 3] {
                    let args = arguments(&artifact, &memory, &words, pivot, end);
                    let aliases = args.clone();
                    let before = contents(&args);
                    let mut reports = Vec::new();
                    for (jit, output) in [&native, &ordinary].into_iter().zip(&mut outputs) {
                        let prior = output.clone();
                        let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                        let result = if restored {
                            observe::run_restored(jit, &args, output, mode.execution(), &memory).0
                        } else {
                            observe::run(jit, &args, output, mode.execution(), &memory).0
                        };
                        if end > words.len() as u64 {
                            reports.push(format!("{:?}", result.unwrap_err()));
                            assert!(output.iter().zip(prior).all(|(now, old)| {
                                now.as_ref()
                                    .unwrap()
                                    .shares_frozen_storage(old.as_ref().unwrap())
                            }));
                            assert_eq!(
                                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                                retained
                            );
                        } else {
                            reports.push(format!("{:?}", result.unwrap()));
                            let expected = Data::Tuple(
                                vec![
                                    Data::Matrix(
                                        words[..end as usize]
                                            .iter()
                                            .flat_map(|&a| {
                                                [
                                                    a.min(pivot),
                                                    a.max(pivot),
                                                    if a == pivot { u64::MAX } else { 0 },
                                                    a ^ pivot,
                                                ]
                                            })
                                            .map(Data::U64)
                                            .collect(),
                                    ),
                                    Data::Matrix(words.iter().copied().map(Data::U64).collect()),
                                ]
                                .into(),
                            );
                            assert_eq!(
                                output
                                    .last()
                                    .unwrap()
                                    .as_ref()
                                    .unwrap()
                                    .canonical_data_draft()
                                    .unwrap(),
                                expected
                            );
                        }
                    }
                    assert_eq!(reports[0], reports[1]);
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                }
            }
            drop(outputs);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn scalar_select_source_keeps_finite_boundaries_and_atomic_publication() {
    resources::finite_boundaries(SOURCE, |artifact, memory| {
        arguments(artifact, memory, &[0, u64::MAX, 1 << 63, 42], 42, 4)
    });
}
