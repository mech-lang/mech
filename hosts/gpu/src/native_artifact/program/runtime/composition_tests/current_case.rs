//! Fresh selected variants justify their own projections, including final visits.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;

const SOURCE: &str = include_str!("../../../../../tests/fixtures/native/numeric-current-case.mec");

fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    start: u64,
    stop: u64,
    seed: u32,
    fault: u64,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            ("values", Data::Matrix([17, 11, 7, 3].map(Data::U32).into())),
            ("start", Data::U64(start)),
            ("stop", Data::U64(stop)),
            ("seed", Data::U32(seed)),
            ("nonce", Data::U64(119)),
            ("fault", Data::U64(fault)),
        ],
        memory,
    )
}

#[test]
fn numeric_current_case_reuses_the_selected_tag_for_payload_projections() {
    for artifact in artifacts(SOURCE) {
        let object = crate::native_artifact::emit_program_object_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        let metadata = object.metadata();
        let numeric = [
            "compiled_readonly_numeric_loops",
            "compiled_numeric_write_loops",
        ]
        .into_iter()
        .flat_map(|name| metadata[name].as_array().unwrap())
        .collect::<Vec<_>>();
        assert!(
            !numeric.is_empty(),
            "the witness must reach numeric lowering"
        );
        let guards = numeric
            .iter()
            .flat_map(|region| {
                region
                    .get("proven_projection_guards")
                    .and_then(|v| v.as_array())
                    .into_iter()
                    .flatten()
            })
            .count();
        assert!(
            guards >= 12,
            "both three-field Try/Done calls need current proofs: {guards}"
        );
    }
}

#[test]
fn numeric_current_case_keeps_fresh_visits_errors_publication_and_retry() {
    let mode = NativeProgramLimitMode::NoLimits;
    let mut expected_error = None;
    for artifact in artifacts(SOURCE) {
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &mech_stdlib::source_catalog(),
            OptimizationPass::ScalarLoops,
        )
        .unwrap();
        let native = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        for (reference, jit) in [(true, &ordinary), (false, &native)] {
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let mut published = vec![None; artifact.outputs().len()];
                for (start, stop, seed, fault) in [
                    (0, 4, u32::MAX, u64::MAX),
                    (0, 4, u32::MAX, 1),
                    (1, 3, 7, u64::MAX),
                    (0, 6, 0, u64::MAX),
                    (0, 4, u32::MAX, 1),
                    (4, 4, 23, 4),
                    (u64::MAX, u64::MAX, 91, u64::MAX),
                    (0, 4, u32::MAX, u64::MAX),
                ] {
                    let args = arguments(&artifact, &memory, start, stop, seed, fault);
                    let aliases = args.clone();
                    let before = contents(&args);
                    let previous = published.clone();
                    let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let (result, counts) = if restored {
                        observe::run_restored(jit, &args, &mut published, mode.execution(), &memory)
                    } else {
                        observe::run(jit, &args, &mut published, mode.execution(), &memory)
                    };
                    if fault == 1 {
                        let error = format!("{:?}", result.unwrap_err());
                        assert!(error.contains("Arithmetic"), "{error}");
                        assert_eq!(error, *expected_error.get_or_insert(error.clone()));
                        super::read_packs::unchanged_publication(&published, &previous);
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            held
                        );
                    } else {
                        result.unwrap();
                        let (mut best, mut item) = (seed, 119);
                        for k in start..stop {
                            let value = [17, 11, 7, 3].get(k as usize).copied().unwrap_or(0);
                            if value < best {
                                best = value;
                                item = k + 10;
                            }
                        }
                        assert_eq!(
                            published
                                .last()
                                .unwrap()
                                .as_ref()
                                .unwrap()
                                .canonical_data_draft()
                                .unwrap(),
                            Data::Tuple(
                                vec![Data::U32(best), Data::U64(item), Data::U64(start.max(stop))]
                                    .into()
                            )
                        );
                        if !reference {
                            assert!(!counts.tail_entries.is_empty());
                        }
                    }
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                }
                drop(published);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}

#[test]
fn numeric_current_case_finite_boundaries_keep_aliases_and_recover() {
    resources::finite_boundaries(SOURCE, |artifact, memory| {
        arguments(artifact, memory, 0, 3, u32::MAX, u64::MAX)
    });
}
