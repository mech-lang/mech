//! Full scan witness: stable U32 input views, one U64 append owner, and
//! synchronous scalar children with their current arguments and failure order.
use super::*;

const SOURCE: &str = include_str!("../../../../../tests/fixtures/native/numeric-nested-phase.mec");

fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    steps: u64,
    flag: bool,
    allowed: bool,
    deny_at: u64,
    divisor: u64,
    bias: u64,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            ("values", Data::Matrix([3, 5, 7].map(Data::U32).into())),
            ("nodes", Data::Matrix([Data::U64(9)].into())),
            ("steps", Data::U64(steps)),
            ("flag", Data::Bool(flag)),
            ("allowed", Data::Bool(allowed)),
            ("deny-at", Data::U64(deny_at)),
            ("divisor", Data::U64(divisor)),
            ("bias", Data::U64(bias)),
        ],
        memory,
    )
}

fn expected(steps: u64, mut flag: bool, divisor: u64, bias: u64) -> Data {
    let values = [3u64, 5, 7];
    let sum: u64 = (1..=values.len())
        .map(|end| values[..end].iter().sum::<u64>() / divisor)
        .sum();
    let mut nodes = vec![Data::U64(9)];
    for at in 0..steps {
        nodes.push(Data::U64(sum + at + if flag { 0 } else { 100 } + bias));
        flag = !flag;
    }
    Data::Matrix(nodes.into())
}

#[test]
fn nested_numeric_phase_keeps_aliases_current_arguments_and_failure_recovery() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        for restored in [false, true] {
            let mut published = vec![None; artifact.outputs().len()];
            for (steps, flag, allowed, deny_at, divisor, bias, rejection) in [
                (3, true, true, u64::MAX, 1, 0, None),
                (3, false, true, u64::MAX, 2, 0, None),
                (3, true, false, u64::MAX, 0, 0, Some("Integrity")),
                (3, true, true, 1, 1, 0, Some("Integrity")),
                (3, true, true, u64::MAX, 0, 0, Some("Arithmetic")),
                (
                    3,
                    true,
                    true,
                    u64::MAX,
                    1,
                    u64::MAX - 26,
                    Some("Arithmetic"),
                ),
                (0, false, false, 0, 0, u64::MAX, None),
                (1, false, true, u64::MAX, 1, 0, None),
                (3, true, true, u64::MAX, 1, 0, None),
            ] {
                let args = arguments(
                    &artifact, &memory, steps, flag, allowed, deny_at, divisor, bias,
                );
                let aliases = args.clone();
                let before = contents(&args);
                let prior = published.clone();
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let (result, counts) = if restored {
                    observe::run_restored(&jit, &args, &mut published, mode.execution(), &memory)
                } else {
                    observe::run(&jit, &args, &mut published, mode.execution(), &memory)
                };
                if let Some(rejection) = rejection {
                    let error = format!("{:?}", result.unwrap_err());
                    assert!(error.contains(rejection), "{error}");
                    for (actual, old) in published.iter().zip(&prior) {
                        let (actual, old) = (actual.as_ref().unwrap(), old.as_ref().unwrap());
                        assert!(actual.shares_frozen_storage(old));
                        assert_eq!(
                            actual.canonical_data_draft().unwrap(),
                            old.canonical_data_draft().unwrap()
                        );
                    }
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                } else {
                    result.unwrap();
                    let expected = if steps == 0 {
                        Data::Matrix([Data::U64(9)].into())
                    } else {
                        expected(steps, flag, divisor, bias)
                    };
                    assert_eq!(
                        published
                            .last()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        expected,
                    );
                    println!(
                        "{}",
                        serde_json::json!({"witness":"nested-numeric-phase", "restored":restored,
                        "positions":steps,"reserved_nodes":305257,"selected_pieces":counts.selected,
                        "admissions":counts.admits.values().sum::<u64>(),
                        "helpers":counts.helpers.values().sum::<u64>()})
                    );
                }
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(a, b)| a.shares_frozen_storage(b))
                );
            }
            drop(published);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn nested_numeric_phase_finite_limits_keep_publication_and_retry() {
    resources::finite_boundaries(SOURCE, |artifact, memory| {
        arguments(artifact, memory, 2, true, true, u64::MAX, 1, 0)
    });
}
