//! A larger parent piece may follow its already funded ordinary scalar prefix.
use super::*;

fn overlap(jit: &JitProgramArtifact) -> (u32, Vec<u32>) {
    let metadata = jit.descriptor().metadata();
    let pieces = metadata
        .compositions
        .parents
        .iter()
        .flat_map(|parent| &parent.segments)
        .filter(|piece| {
            piece.numeric.is_some()
                && piece
                    .frame_plan
                    .as_ref()
                    .is_some_and(|frame| frame.aliases.iter().any(|alias| alias.call_result))
                && metadata
                    .map_admission
                    .group(piece.entry as usize)
                    .is_some_and(|group| {
                        group.owner.is_none()
                            && group.start() == piece.entry
                            && group.sites.iter().all(|site| piece.sites.contains(site))
                    })
        })
        .collect::<Vec<_>>();
    assert_eq!(pieces.len(), 1, "fixture must isolate the real overlap");
    let piece = pieces[0];
    assert!(
        metadata.compositions.entry(piece.entry as usize).is_some(),
        "a fully contained scalar prefix must not exclude the proved piece"
    );
    let group = metadata.map_admission.group(piece.entry as usize).unwrap();
    for other in metadata
        .compositions
        .parents
        .iter()
        .flat_map(|p| &p.segments)
    {
        if metadata
            .map_admission
            .group(other.entry as usize)
            .is_some_and(|group| {
                group.owner.is_some()
                    || group.start() != other.entry
                    || !group.sites.iter().all(|site| other.sites.contains(site))
            })
        {
            assert!(metadata.compositions.entry(other.entry as usize).is_none());
        }
    }
    (piece.entry, group.sites.to_vec())
}

fn check_output(output: &[Option<Value>], length: usize, steps: u64) {
    let mut expected = vec![Data::U8(97); length];
    if length > 0 {
        expected[0] = Data::U8(97 + (length as u8 + 1) * steps as u8);
    }
    assert_eq!(
        output
            .last()
            .unwrap()
            .as_ref()
            .unwrap()
            .canonical_data_draft()
            .unwrap(),
        Data::Matrix(expected.into())
    );
}

#[test]
fn contained_scalar_prefix_selects_current_parent_piece_on_each_visit() {
    for artifact in artifacts(bounds_retry::SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let (entry, _) = overlap(&jit);
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut output = vec![None; artifact.outputs().len()];
            for (length, steps) in [(3, 2), (5, 0), (7, 4), (2, 3)] {
                let args = bounds_retry::arguments(&artifact, &memory, length, steps);
                let aliases = args.clone();
                let before = contents(&args);
                let (result, counts) = if restored {
                    observe::run_restored(&jit, &args, &mut output, mode.execution(), &memory)
                } else {
                    observe::run(&jit, &args, &mut output, mode.execution(), &memory)
                };
                result.unwrap();
                assert_eq!(
                    counts.selected_sites.get(&u64::from(entry)).copied(),
                    Some(steps + 1),
                    "each recursive visit must reacquire current owners: {counts:?}"
                );
                check_output(&output, length, steps);
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

#[test]
fn declined_parent_piece_restores_paid_scalar_prefix_and_recovers() {
    for artifact in artifacts(bounds_retry::SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let (entry, scalar_sites) = overlap(&jit);
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let args = bounds_retry::arguments(&artifact, &memory, 3, 2);
            let before = contents(&args);
            let aliases = args.clone();
            let mut output = vec![None; artifact.outputs().len()];
            for decline in [false, true, false] {
                let mut policy = mode.execution();
                if decline {
                    // Ordinary exact results fit. The conservative whole-piece
                    // owner bound must decline without invalidating scalar fuel.
                    policy.library.max_output_bytes = Some(1024);
                }
                let (result, counts) = if restored {
                    observe::run_restored(&jit, &args, &mut output, policy, &memory)
                } else {
                    observe::run(&jit, &args, &mut output, policy, &memory)
                };
                result.unwrap();
                let selected = counts
                    .selected_sites
                    .get(&u64::from(entry))
                    .copied()
                    .unwrap_or(0);
                assert_eq!(selected, if decline { 0 } else { 3 }, "{counts:?}");
                assert_eq!(
                    counts
                        .helpers
                        .iter()
                        .filter(|((site, operation), _)| {
                            *site == u64::from(entry)
                                && matches!(
                                    GlobalHelper::decode(*operation),
                                    GlobalHelper::NumericSegmentBegin
                                        | GlobalHelper::AdmitNumericSegmentBegin
                                )
                        })
                        .map(|(_, count)| count)
                        .sum::<u64>(),
                    3,
                    "the decline must exercise the actual native preview: {counts:?}"
                );
                for &site in &scalar_sites[1..] {
                    assert!(
                        !counts.admits.contains_key(&u64::from(site)),
                        "a paid scalar interior must not be admitted again: {counts:?}"
                    );
                }
                check_output(&output, 3, 2);
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
            }
            drop((output, args, aliases));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
