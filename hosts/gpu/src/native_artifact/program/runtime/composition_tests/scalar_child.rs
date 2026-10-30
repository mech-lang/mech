use super::*;

pub(super) const SOURCE: &str =
    include_str!("../../../../../tests/fixtures/native/numeric-scalar-child.mec");

pub(super) fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    steps: u64,
    repeats: u64,
    flag: bool,
    allowed: bool,
    divisor: u64,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            ("words", Data::Matrix([Data::U64(7)].into())),
            ("steps", Data::U64(steps)),
            ("start", Data::U64(2)),
            ("flag", Data::Bool(flag)),
            ("repeats", Data::U64(repeats)),
            ("allowed", Data::Bool(allowed)),
            ("divisor", Data::U64(divisor)),
        ],
        memory,
    )
}

#[test]
fn ordinary_scalar_child_keeps_current_boolean_words_errors_and_retry() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let parents = &jit.descriptor().metadata().compositions.parents;
        assert_eq!(parents.len(), 1);
        let parent = &parents[0];
        assert!(parent.scalar_child && parent.returned_arrays.is_empty());
        let mut published: Vec<Option<Value>> = vec![None; artifact.outputs().len()];
        for (steps, repeats, flag, allowed, divisor, expected, rejection) in [
            (3, 2, true, true, 1, Some(111), None),
            (2, 1, false, true, 2, Some(2), None),
            (3, 2, false, false, 0, None, Some("Integrity")),
            (3, 2, false, true, 0, None, Some("Arithmetic")),
            (0, 2, false, false, 0, Some(2), None),
            (3, 2, false, true, 1, Some(211), None),
        ] {
            let args = arguments(&artifact, &memory, steps, repeats, flag, allowed, divisor);
            let aliases = args.clone();
            let before = contents(&args);
            let prior = published.clone();
            let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
            let (result, counts) =
                observe::run(&jit, &args, &mut published, mode.execution(), &memory);
            if let Some(expected) = expected {
                result.unwrap();
                assert_eq!(
                    published
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::U64(expected),
                );
                assert_eq!(
                    counts
                        .admits
                        .get(&u64::from(parent.child_initialize))
                        .copied()
                        .unwrap_or(0),
                    steps,
                    "child must initialize once per reached call: {counts:?}",
                );
                assert_eq!(
                    counts
                        .admits
                        .get(&u64::from(parent.child_exit))
                        .copied()
                        .unwrap_or(0),
                    steps,
                );
            } else {
                let error = format!("{:?}", result.unwrap_err());
                assert!(error.contains(rejection.unwrap()), "{error}");
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
            }
            assert!(
                counts.selected > 0,
                "real native pieces must execute: {counts:?}"
            );
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
