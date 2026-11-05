//! A current child after an opaque parent boundary needs its own native entry.
use super::*;
use mech_engine::native_artifact::NativeControlLoop;
#[path = "resumed_child_failure.rs"]
mod failure;
#[path = "resumed_child_image.rs"]
mod image;

const SOURCE: &str =
    include_str!("../../../../../tests/fixtures/native/phase-descendant-pieces.mec");

#[test]
fn ordinary_suffix_reenters_its_complete_numeric_child() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let native = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let metadata = native.descriptor().metadata();
        let (child, primary) = metadata
            .plan
            .iterations()
            .iter()
            .enumerate()
            .find_map(|(iteration, control)| {
                let NativeControlLoop::Tail(tail) = control else {
                    return None;
                };
                tail.parent()?;
                let owner = metadata.closed_tails.sites[tail.header() as usize]?;
                let outer = &metadata.closed_tails.regions[owner];
                if outer.iteration == iteration as u32
                    || !outer.numeric.as_ref().is_some_and(|proof| {
                        !proof.exits.is_empty()
                            && proof.sites.binary_search(&(tail.header() as usize)).is_ok()
                    })
                {
                    return None;
                }
                let proof = crate::native_artifact::program::prepare::readonly_numeric::derive(
                    &metadata.plan,
                    tail,
                    &metadata.buffer_frames,
                    &metadata.scalar_aggregates,
                    &metadata.calls,
                    &metadata.visits,
                )
                .ok()?;
                (proof.exits.is_empty() && proof.prepaid_sites.is_empty()).then_some((tail, owner))
            })
            .expect("a complete child inside a partial primary region");
        let secondary = metadata.closed_tails.initializers[child.initialize() as usize]
            .expect("ordinary suffix needs a native child entry");
        assert_ne!(secondary, primary);
        let proof = metadata.closed_tails.regions[secondary]
            .numeric
            .as_ref()
            .unwrap();
        assert!(proof.exits.is_empty() && proof.prepaid_sites.is_empty());
        assert!(
            proof
                .sites
                .iter()
                .all(|site| metadata.closed_tails.sites[*site] == Some(primary))
        );
        assert!(
            metadata
                .compositions
                .boundary(child.initialize() as usize)
                .is_some_and(|(_, entry)| entry),
            "the witness needs a tracked child return"
        );
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let args = super::phase_descendants::arguments(&artifact, &memory, 1, 2, 0);
            let aliases = args.clone();
            let before = contents(&args);
            let mut published = vec![None; artifact.outputs().len()];
            let (result, counts) = if restored {
                observe::run_restored(&native, &args, &mut published, mode.execution(), &memory)
            } else {
                observe::run(&native, &args, &mut published, mode.execution(), &memory)
            };
            result.unwrap();
            println!(
                "secondary entries={:?} boundaries={:?}",
                counts.tail_entries, counts.tail_boundaries
            );
            assert!(
                counts
                    .tail_entries
                    .iter()
                    .any(|(_, id, route)| *id == secondary && route.is_some()),
                "{counts:?}"
            );
            assert!(
                counts
                    .tail_entries
                    .iter()
                    .any(|(site, id, route)| *id == primary
                        && *site == metadata.closed_tails.regions[primary].tail.header() as u64
                        && route.is_none()),
                "the outer header must freshly import after shared shadow use: {counts:?}"
            );
            assert!(
                counts.completed_parent_children.contains(&(
                    metadata.closed_tails.regions[secondary].iteration,
                    child.parent().unwrap()
                )),
                "consume the original pending child return: {counts:?}"
            );
            assert!(
                counts
                    .tail_boundaries
                    .iter()
                    .any(|row| row.2 == 6 && row.4.is_some()),
                "{counts:?}"
            );
            for row in counts
                .tail_boundaries
                .iter()
                .filter(|row| row.3.is_some() && row.2 != 6 && row.2 != 7)
            {
                assert_eq!(
                    row.3, row.4,
                    "secondary boundary preserves outer cursor: {row:?}"
                );
            }
            assert_eq!(contents(&args), before);
            assert_eq!(contents(&aliases), before);
            let mut nodes = [129u8, 129, 7];
            for _ in 0..2 {
                for at in 0..2 {
                    nodes[0] = nodes[0].wrapping_add(1);
                    if at == 0 {
                        nodes = nodes.map(|n| n.saturating_sub(128));
                    }
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
                    vec![
                        Data::Matrix(nodes.map(Data::U8).into()),
                        Data::Matrix([Data::U32(2), Data::U32(0)].into()),
                        Data::U64(2)
                    ]
                    .into()
                )
            );
            drop((published, aliases, args));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
