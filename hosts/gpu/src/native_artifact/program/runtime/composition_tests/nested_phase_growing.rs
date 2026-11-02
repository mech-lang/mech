//! Current contents, not just common provenance, must cross native children.
use super::*;
const SOURCE: &str =
    include_str!("../../../../../tests/fixtures/native/numeric-nested-phase-growing.mec");
#[test]
fn nested_numeric_phase_children_read_growing_contents_and_length() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let phases = &jit.descriptor().metadata().closed_tails.regions;
        assert_eq!(phases.len(), 1);
        assert_eq!(
            phases[0]
                .numeric
                .as_ref()
                .unwrap()
                .phase
                .as_ref()
                .unwrap()
                .loops
                .len(),
            3
        );
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut published = vec![None; artifact.outputs().len()];
            for (steps, allowed, deny_at, divisor, rejection) in [
                (3, true, u64::MAX, 1, None),
                (3, true, 1, 1, Some("Integrity")),
                (3, false, u64::MAX, 0, Some("Integrity")),
                (3, true, u64::MAX, 0, Some("Arithmetic")),
                (0, false, 0, 0, None),
                (3, true, u64::MAX, 1, None),
            ] {
                let args = inputs(
                    &artifact,
                    &[
                        ("nodes", Data::Matrix([Data::U64(9)].into())),
                        ("steps", Data::U64(steps)),
                        ("allowed", Data::Bool(allowed)),
                        ("deny-at", Data::U64(deny_at)),
                        ("divisor", Data::U64(divisor)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let contents_before = contents(&aliases);
                let previous = published.clone();
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let (result, counts) = if restored {
                    observe::run_restored(&jit, &args, &mut published, mode.execution(), &memory)
                } else {
                    observe::run(&jit, &args, &mut published, mode.execution(), &memory)
                };
                if let Some(rejection) = rejection {
                    let error = format!("{:?}", result.unwrap_err());
                    assert!(error.contains(rejection), "{error}");
                    for (new, old) in published.iter().zip(&previous) {
                        assert!(
                            new.as_ref()
                                .unwrap()
                                .shares_frozen_storage(old.as_ref().unwrap())
                        );
                    }
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                } else {
                    result.unwrap();
                    assert_eq!(counts.phases, 1);
                    assert_eq!(
                        counts.helpers.values().sum::<u64>(),
                        if steps == 0 { 7 } else { 8 }
                    );
                    let expected = if steps == 0 {
                        vec![9]
                    } else {
                        vec![9, 9, 28, 75]
                    };
                    assert_eq!(
                        published
                            .last()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        Data::Matrix(
                            expected
                                .into_iter()
                                .map(Data::U64)
                                .collect::<Vec<_>>()
                                .into()
                        )
                    );
                    println!(
                        "{}",
                        serde_json::json!({"witness":"growing-numeric-phase","restored":restored,"positions":steps,"admissions":counts.admits.values().sum::<u64>(),"helpers":counts.helpers.values().sum::<u64>(),"selected_pieces":counts.selected})
                    );
                }
                assert_eq!(contents(&args), contents_before);
                assert_eq!(contents(&aliases), contents_before);
            }
            drop(published);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
