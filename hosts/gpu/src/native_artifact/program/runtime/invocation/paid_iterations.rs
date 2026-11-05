//! The segment-level negative must preserve the exact per-source paid predicate.
use super::NumericPrepaid;
use crate::native_artifact::program::prepare::ProgramMetadata;
use crate::native_artifact::{JitProgramArtifact, NativeProgramLimitMode};
use std::collections::BTreeSet;

#[test]
fn native_paid_iteration_guard_preserves_exact_source_overlap() {
    let sources = [
        include_str!("../../../../../tests/fixtures/native/phase-descendant-pieces.mec"),
        include_str!("../../../../../tests/fixtures/native/numeric-nested-rebase.mec"),
        include_str!("../../../../../tests/fixtures/native/numeric-collector-rebase.mec"),
    ];
    // Default inventories, explicit interval inventories, paid/unpaid segments,
    // positive ancestry with exact-inventory holes, above-root negatives,
    // other-branch negatives, and the absence of a source iteration.
    let mut coverage = [0; 8];
    for source in sources {
        let original = super::super::tests::artifact_source(source);
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
        )
        .unwrap();
        for artifact in [original, decoded] {
            let native = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                NativeProgramLimitMode::NoLimits.compilation(),
            )
            .unwrap();
            let restored = super::super::tests::restore_admitted_helper_descriptor(&native);
            for descriptor in [native.descriptor(), &restored] {
                check(descriptor.metadata(), &mut coverage);
            }
        }
    }
    assert!(coverage.iter().all(|&count| count > 0), "{coverage:?}");
    println!("paid-iteration coverage={coverage:?}");
}

fn check(metadata: &ProgramMetadata, coverage: &mut [usize; 8]) {
    for (id, region) in metadata.closed_tails.regions.iter().enumerate() {
        let Some(proof) = &region.numeric else {
            continue;
        };
        let inventory = if proof.prepaid_sites.is_empty() {
            coverage[0] += 1;
            &proof.sites
        } else {
            coverage[1] += 1;
            &proof.prepaid_sites
        };
        for deepest in 0..metadata.plan.iterations().len() as u32 {
            // Independently enumerate the whole ancestry, then clip it at this
            // root. In particular, ancestors above the root are never paid.
            let ancestors = std::iter::successors(Some(deepest), |&at| {
                metadata.plan.iterations()[at as usize].as_ref().parent()
            })
            .collect::<Vec<_>>();
            let Some(root_at) = ancestors.iter().position(|&at| at == region.iteration) else {
                continue;
            };
            let paid = ancestors[..=root_at]
                .iter()
                .copied()
                .collect::<BTreeSet<_>>();
            let route = NumericPrepaid {
                region: id,
                deepest,
            };
            for wanted in
                std::iter::once(None).chain((0..metadata.plan.iterations().len() as u32).map(Some))
            {
                let possible = route.may_include_iteration(wanted);
                assert!(
                    possible || !wanted.is_some_and(|at| paid.contains(&at)),
                    "guard excluded paid ancestry: region={id} deepest={deepest} wanted={wanted:?} paid={paid:?}"
                );
                assert_eq!(possible, wanted.is_some_and(|at| at <= deepest));
                if let Some(wanted) = wanted {
                    if ancestors[root_at + 1..].contains(&wanted) {
                        coverage[5] += 1;
                    } else if !ancestors.contains(&wanted) {
                        coverage[6] += 1;
                    }
                } else {
                    coverage[7] += 1;
                }
            }
            // The independent oracle uses exact inventory membership, including
            // holes, rather than granting credit from ancestry alone.
            let source_paid = |site: usize| {
                inventory.contains(&site)
                    && metadata.plan.blocks()[site]
                        .iteration()
                        .is_some_and(|at| paid.contains(&at))
            };
            for (site, block) in metadata.plan.blocks().iter().enumerate() {
                assert!(route.may_include_iteration(block.iteration()) || !source_paid(site));
                if block.iteration().is_some_and(|at| paid.contains(&at))
                    && !inventory.contains(&site)
                {
                    coverage[4] += 1;
                }
            }
            for parent in &metadata.compositions.parents {
                for segment in &parent.segments {
                    assert!(!segment.sites.is_empty());
                    assert!(segment.sites.iter().all(|&site| {
                        metadata.plan.blocks()[site as usize].iteration() == Some(parent.iteration)
                    }));
                    let old_overlap = segment.sites.iter().any(|&site| source_paid(site as usize));
                    let possible = route.may_include_iteration(Some(parent.iteration));
                    let guarded_overlap =
                        possible && segment.sites.iter().any(|&site| source_paid(site as usize));
                    assert_eq!(guarded_overlap, old_overlap);
                    coverage[if old_overlap { 2 } else { 3 }] += 1;
                }
            }
        }
    }
}
