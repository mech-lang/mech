//! Eligibility must retain source versions, every original visit and failures.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;

const SOURCE: &str =
    include_str!("../../../../../tests/fixtures/native/numeric-boolean-mask-reuse.mec");

fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    offsets: &[u64],
    ids: &[u64],
    start: u64,
    posts: u64,
    capacity: u64,
) -> Vec<Value> {
    let matrix = |values: &[u64]| Data::Matrix(values.iter().copied().map(Data::U64).collect());
    inputs(
        artifact,
        &[
            ("offsets", matrix(offsets)),
            ("ids", matrix(ids)),
            ("start", Data::U64(start)),
            ("posts", Data::U64(posts)),
            ("capacity", Data::U64(capacity)),
        ],
        memory,
    )
}

fn expected(offsets: &[u64], ids: &[u64], start: usize, posts: usize) -> Data {
    let mut masks = Vec::new();
    let mut eligible = true;
    for post in start..posts {
        let row = &ids[offsets[post] as usize..offsets[post + 1] as usize];
        if row.len() > 8 {
            eligible = false;
            break;
        }
        let mut mask = 0u64;
        for &id in row {
            let bit = 1u64.wrapping_shl(id as u32);
            if bit <= mask {
                eligible = false;
                break;
            }
            mask |= bit;
        }
        if !eligible {
            break;
        }
        masks.push(Data::U64(mask));
    }
    Data::Tuple(
        vec![
            Data::Matrix(masks.into()),
            Data::Bool(eligible),
            Data::Matrix(offsets.iter().copied().map(Data::U64).collect()),
            Data::Matrix(ids.iter().copied().map(Data::U64).collect()),
        ]
        .into(),
    )
}

#[test]
fn numeric_boolean_relations_lower_guarded_mask_reuse_and_preserve_each_current_visit() {
    let negated = SOURCE.replace("more && row.2", "!(!more || !row.2)");
    for source in [SOURCE, negated.as_str()] {
        for artifact in artifacts(source) {
            let mode = NativeProgramLimitMode::NoLimits;
            let catalog = mech_stdlib::source_catalog();
            let native =
                JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                    .unwrap();
            let reference = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::ScalarLoops,
            )
            .unwrap();
            let region = native
                .descriptor()
                .metadata()
                .closed_tails
                .regions
                .iter()
                .find(|region| {
                    region
                        .numeric_tree
                        .as_ref()
                        .is_some_and(|tree| tree.nodes.len() >= 2)
                        && region
                            .numeric
                            .as_ref()
                            .is_some_and(|proof| proof.arrays.iter().any(|a| a.appends))
                })
                .expect("the append parent and scalar row child must form one native tree");
            let sites = region.numeric.as_ref().unwrap().sites.clone();
            for restored in [false, true] {
                for (offsets, ids, start, posts) in [
                    (vec![0, 3, 5, 5], vec![0, 1, 4, 2, 5], 0, 3),
                    (vec![0, 3, 5], vec![0, 1, 4, 5, 2], 0, 2),
                    (vec![0, 3, 5], vec![0, 1, 4, 2, 2], 0, 2),
                    (vec![0, 1, 1, 3], vec![63, 0, 63], 0, 3),
                    (vec![0, 9], (0..9).collect(), 0, 1),
                    (vec![0], vec![], 0, 0),
                    (vec![0, 2], vec![2, 4], 1, 1),
                ] {
                    let mut same_work = None;
                    for (optimized, jit) in [(false, &reference), (true, &native)] {
                        let memory = mode.memory_budget();
                        let args =
                            arguments(&artifact, &memory, &offsets, &ids, start, posts, posts);
                        let aliases = args.clone();
                        let before = contents(&args);
                        let mut output = vec![None; artifact.outputs().len()];
                        let (result, calls) = if restored {
                            observe::run_restored(
                                jit,
                                &args,
                                &mut output,
                                mode.execution(),
                                &memory,
                            )
                        } else {
                            observe::run(jit, &args, &mut output, mode.execution(), &memory)
                        };
                        let report = result.unwrap();
                        assert_eq!(report.work, *same_work.get_or_insert(report.work));
                        assert_eq!(
                            output
                                .last()
                                .unwrap()
                                .as_ref()
                                .unwrap()
                                .canonical_data_draft()
                                .unwrap(),
                            expected(&offsets, &ids, start as usize, posts as usize)
                        );
                        assert_eq!(contents(&args), before);
                        assert_eq!(contents(&aliases), before);
                        if optimized {
                            assert!(
                                calls
                                    .helpers
                                    .keys()
                                    .all(|(site, _)| !sites.contains(&(*site as usize))),
                                "{calls:?}"
                            );
                        }
                        drop(output);
                        drop(aliases);
                        drop(args);
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            (0, 0)
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn numeric_boolean_relations_preserve_exact_index_arithmetic_errors_publication_and_retry() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let reference = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::ScalarLoops,
        )
        .unwrap();
        let native =
            JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                .unwrap();
        let mut errors = [None, None];
        for jit in [&reference, &native] {
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let mut output = vec![None; artifact.outputs().len()];
                for (offsets, failure) in [
                    (vec![0, 1, 3], None),
                    (vec![0, 1, 4], Some((0, "Index"))),
                    (vec![0, 2, 1], Some((1, "Arithmetic"))),
                    (vec![0, 1, 3], None),
                ] {
                    let args = arguments(&artifact, &memory, &offsets, &[0, 2, 4], 0, 2, 2);
                    let aliases = args.clone();
                    let before = contents(&args);
                    let previous = output.clone();
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let result = if restored {
                        observe::run_restored(jit, &args, &mut output, mode.execution(), &memory).0
                    } else {
                        observe::run(jit, &args, &mut output, mode.execution(), &memory).0
                    };
                    if let Some((index, kind)) = failure {
                        let error = format!("{:?}", result.unwrap_err());
                        assert!(error.contains(kind), "{error}");
                        assert_eq!(error, *errors[index].get_or_insert(error.clone()));
                        assert!(output.iter().zip(&previous).all(|(new, old)| {
                            new.as_ref()
                                .unwrap()
                                .shares_frozen_storage(old.as_ref().unwrap())
                        }));
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            retained
                        );
                    } else {
                        result.unwrap();
                        assert_eq!(
                            output
                                .last()
                                .unwrap()
                                .as_ref()
                                .unwrap()
                                .canonical_data_draft()
                                .unwrap(),
                            expected(&offsets, &[0, 2, 4], 0, 2)
                        );
                    }
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                }
                drop(output);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}

#[test]
fn numeric_boolean_relations_keep_finite_boundaries_and_retry() {
    resources::finite_boundaries(SOURCE, |artifact, memory| {
        arguments(artifact, memory, &[0, 3, 5, 5], &[0, 1, 4, 2, 5], 0, 3, 3)
    });
}

#[test]
fn numeric_boolean_relations_decline_a_real_discard_of_the_appended_version() {
    let source = SOURCE.replace(
        "(row.2? | true => subset-prepare",
        "((row.2 == false)? | true => subset-prepare",
    );
    assert_ne!(source, SOURCE);
    for artifact in artifacts(&source) {
        let native = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        assert!(
            !native
                .descriptor()
                .metadata()
                .closed_tails
                .regions
                .iter()
                .any(|region| region
                    .numeric_tree
                    .as_ref()
                    .is_some_and(|tree| tree.nodes.len() >= 2)
                    && region
                        .numeric
                        .as_ref()
                        .is_some_and(|proof| proof.arrays.iter().any(|a| a.appends))),
            "an append followed by returning the old buffer is not current-version preserving"
        );
    }
}
