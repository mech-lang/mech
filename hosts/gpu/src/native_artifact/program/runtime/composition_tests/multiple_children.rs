//! The reached compressor parent contains a collection plus several ordinary
//! children, including nested private-frame parents. No view may span a child.
use super::*;

pub(super) const SOURCE: &str =
    include_str!("../../../../../tests/fixtures/native/numeric-multiple-children.mec");

fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    steps: u64,
    flag: bool,
    first: bool,
    second: bool,
    divisor: u64,
    empty: bool,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            (
                "words",
                Data::Matrix(if empty { vec![] } else { vec![Data::U64(7)] }.into()),
            ),
            ("steps", Data::U64(steps)),
            ("flag", Data::Bool(flag)),
            ("first-allowed", Data::Bool(first)),
            ("second-allowed", Data::Bool(second)),
            ("divisor", Data::U64(divisor)),
        ],
        memory,
    )
}

fn expected(steps: u64, mut flag: bool, divisor: u64, empty: bool) -> Data {
    let mut word = 7;
    for left in (1..=steps).rev() {
        word = (word + if flag { 2 } else { 5 } + left) / divisor;
        word = (word + 3) / divisor;
        flag = !flag;
    }
    Data::Tuple(
        vec![
            Data::Matrix(if empty { vec![] } else { vec![Data::U64(word)] }.into()),
            Data::Bool(flag),
        ]
        .into(),
    )
}

#[test]
fn multiple_child_boundaries_keep_current_owners_ordered_errors_and_retry() {
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
            for (steps, flag, first, second, divisor, empty, rejection) in [
                (2, true, true, true, 1, false, None),
                (3, false, true, true, 2, false, None),
                (2, false, false, true, 0, false, Some("Integrity")),
                (2, true, true, false, 1, false, Some("Integrity")),
                (2, true, true, false, 0, false, Some("Arithmetic")),
                (0, false, false, false, 0, true, None),
                (1, false, true, true, 1, true, Some("Index")),
                (3, false, true, true, 1, false, None),
            ] {
                let args = arguments(
                    &artifact, &memory, steps, flag, first, second, divisor, empty,
                );
                let aliases = args.clone();
                let before = contents(&args);
                let prior = published.clone();
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let (result, _) = if restored {
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
                    assert_eq!(
                        published
                            .last()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        expected(steps, flag, divisor, empty)
                    );
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

#[test]
fn multiple_child_parent_pieces_are_selected_and_execute() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let plan = jit.descriptor().plan();
        let parent_id = plan
            .iterations()
            .iter()
            .enumerate()
            .find_map(|(id, _)| {
                let children = plan
                    .iterations()
                    .iter()
                    .filter(|child| child.as_ref().parent() == Some(id as u32))
                    .collect::<Vec<_>>();
                (children.len() >= 3
                    && children.iter().any(|c| {
                        matches!(
                            c,
                            mech_engine::native_artifact::NativeControlLoop::Collection(_)
                        )
                    }))
                .then_some(id as u32)
            })
            .expect("witness must retain mixed ordinary child boundaries");
        let parent = jit
            .descriptor()
            .metadata()
            .compositions
            .parents
            .iter()
            .find(|parent| parent.iteration == parent_id)
            .expect("multi-child parent is excluded before its pieces are proved");
        let entries = parent
            .segments
            .iter()
            .filter(|part| part.numeric.is_some())
            .map(|part| u64::from(part.entry))
            .collect::<std::collections::BTreeSet<_>>();
        assert!(!entries.is_empty());
        assert!(parent.children.len() >= 3);
        for piece in &parent.segments {
            assert!(piece.sites.iter().all(|site| {
                parent
                    .children
                    .iter()
                    .all(|child| *site != child.child_initialize && *site != child.child_exit)
            }));
        }
        let nested_entries = jit
            .descriptor()
            .metadata()
            .compositions
            .parents
            .iter()
            .filter(|parent| parent.suspended_child.is_some())
            .flat_map(|parent| &parent.segments)
            .filter(|piece| piece.numeric.is_some())
            .map(|piece| u64::from(piece.entry))
            .collect::<std::collections::BTreeSet<_>>();
        assert!(
            !nested_entries.is_empty(),
            "nested parent must retain independently proved pieces"
        );
        let args = arguments(&artifact, &memory, 3, false, true, true, 1, false);
        let mut outputs = vec![None; artifact.outputs().len()];
        let (result, counts) = observe::run(&jit, &args, &mut outputs, mode.execution(), &memory);
        result.unwrap();
        assert!(
            counts
                .selected_sites
                .iter()
                .any(|(site, &count)| entries.contains(site) && count > 0),
            "{counts:?}"
        );
        assert!(
            counts
                .selected_sites
                .iter()
                .any(|(site, &count)| nested_entries.contains(site) && count > 0),
            "nested native piece must execute with the exact suspended ancestor: {counts:?}"
        );
        for child in &parent.children {
            let calls = if child.initialize_progress == 2 { 4 } else { 3 };
            assert_eq!(
                counts
                    .admits
                    .get(&u64::from(child.child_initialize))
                    .copied()
                    .unwrap_or(0),
                calls
            );
            assert_eq!(
                counts
                    .admits
                    .get(&u64::from(child.child_exit))
                    .copied()
                    .unwrap_or(0),
                calls
            );
        }
        println!(
            "{}",
            serde_json::json!({"witness":"multiple-ordinary-children", "children":parent.children.len(),
            "executed_native_pieces":counts.selected, "selected_sites":counts.selected_sites,
            "logical_admissions":counts.admits.values().sum::<u64>(), "helpers":counts.helpers.values().sum::<u64>()})
        );
        assert_eq!(
            outputs
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            expected(3, false, 1, false)
        );
        drop((args, outputs));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn multiple_child_finite_boundaries_preserve_publication_contents_and_retry() {
    super::resources::finite_boundaries(SOURCE, |artifact, memory| {
        arguments(artifact, memory, 2, false, true, true, 1, false)
    });
}
