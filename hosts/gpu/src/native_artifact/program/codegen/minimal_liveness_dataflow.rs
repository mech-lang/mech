//! Pure dataflow over the exact helper emitter's recorded effects. No plan,
//! resource, guard or visit authority is cached here.
use crate::native_artifact::program::packed_liveness::{
    MAX_DENSE_CELLS, dense_live_before_indexed,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(super) struct Effects {
    pub(super) reads: BTreeSet<usize>,
    pub(super) writes: BTreeSet<usize>,
}
impl Effects {
    pub(super) fn used(&mut self, word: usize) {
        if !self.writes.contains(&word) {
            self.reads.insert(word);
        }
    }
}
pub(super) fn required(
    sites: &[usize],
    effects: &BTreeMap<usize, Effects>,
    edges: &BTreeMap<usize, Vec<usize>>,
    exits: &BTreeSet<usize>,
    terminal: &BTreeSet<usize>,
    header: usize,
) -> Option<BTreeSet<usize>> {
    required_with_limit(
        sites,
        effects,
        edges,
        exits,
        terminal,
        header,
        MAX_DENSE_CELLS,
    )
}

fn required_with_limit(
    sites: &[usize],
    effects: &BTreeMap<usize, Effects>,
    edges: &BTreeMap<usize, Vec<usize>>,
    exits: &BTreeSet<usize>,
    terminal: &BTreeSet<usize>,
    header: usize,
    max_cells: usize,
) -> Option<BTreeSet<usize>> {
    // Local ordinals keep memory independent of the largest source word ID.
    let selected = effects.keys().copied().collect::<Vec<_>>();
    if let Ok(start) = selected.binary_search(&header) {
        let next = selected
            .iter()
            .map(|site| {
                edges[site]
                    .iter()
                    .map(|target| selected.binary_search(target).unwrap())
                    .collect()
            })
            .collect::<Vec<_>>();
        let reads = selected
            .iter()
            .map(|site| {
                let effect = &effects[site];
                let mut reads = effect.reads.clone();
                if exits.contains(site) {
                    // Terminal outputs are observed AFTER this site's writes.
                    // Local read-before-write effects still seed independently.
                    reads.extend(terminal.difference(&effect.writes).copied());
                }
                reads
            })
            .collect::<Vec<_>>();
        let kills = selected
            .iter()
            .map(|site| effects[site].writes.iter().copied().collect())
            .collect::<Vec<_>>();
        if let Some(live) = dense_live_before_indexed(&next, &reads, &kills, start, max_cells) {
            return Some(live);
        }
    }
    // Keep the original complete set solver for larger packed products and
    // its missing-header behavior. Effects and selected edges are unchanged.
    let mut live = effects
        .iter()
        .map(|(&s, e)| (s, e.reads.clone()))
        .collect::<BTreeMap<_, _>>();
    loop {
        let mut changed = false;
        for &site in sites.iter().rev() {
            let mut next = effects[&site].reads.clone();
            if exits.contains(&site) {
                next.extend(terminal.difference(&effects[&site].writes).copied());
            }
            for target in &edges[&site] {
                next.extend(live[target].difference(&effects[&site].writes).copied());
            }
            if live[&site] != next {
                live.insert(site, next);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    live.remove(&header)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Independent read-before-write path oracle, not a fixed-point solver.
    fn reference(
        effects: &BTreeMap<usize, Effects>,
        edges: &BTreeMap<usize, Vec<usize>>,
        exits: &BTreeSet<usize>,
        terminal: &BTreeSet<usize>,
        header: usize,
    ) -> Option<BTreeSet<usize>> {
        if !effects.contains_key(&header) {
            return None;
        }
        let values = effects
            .values()
            .flat_map(|e| e.reads.iter())
            .chain(terminal)
            .copied()
            .collect::<BTreeSet<_>>();
        Some(
            values
                .into_iter()
                .filter(|word| {
                    let mut seen = BTreeSet::new();
                    let mut pending = vec![header];
                    while let Some(site) = pending.pop() {
                        if !seen.insert(site) {
                            continue;
                        }
                        let effect = &effects[&site];
                        if effect.reads.contains(word) {
                            return true;
                        }
                        if effect.writes.contains(word) {
                            continue;
                        }
                        if exits.contains(&site) && terminal.contains(word) {
                            return true;
                        }
                        pending.extend(&edges[&site]);
                    }
                    false
                })
                .collect(),
        )
    }

    #[test]
    fn local_reads_survive_writes_but_terminal_outputs_and_fallbacks_keep_their_order() {
        let mut effect = Effects::default();
        effect.used(3);
        effect.writes.extend([3, 4]);
        effect.used(4); // This read follows a write and needs no input.
        let effects = BTreeMap::from([(7, effect)]);
        let edges = BTreeMap::from([(7, vec![])]);
        let terminal = BTreeSet::from([3, 4, 5]);
        assert_eq!(
            required(&[7], &effects, &edges, &BTreeSet::from([7]), &terminal, 7),
            Some(BTreeSet::from([3, 5]))
        );
        // A fallback has its exact exported reads and no implicit result edge.
        assert_eq!(
            required(&[7], &effects, &edges, &BTreeSet::new(), &terminal, 7),
            Some(BTreeSet::from([3]))
        );
    }

    #[test]
    fn packed_and_fallback_match_read_before_write_paths_on_seeded_helper_graphs() {
        for seed in 0..512u64 {
            let mut state = seed + 1;
            let mut random = || {
                state = state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                (state >> 32) as usize
            };
            let word = |value: usize| {
                if value % 5 == 0 {
                    usize::MAX - value % 8
                } else {
                    value % 129
                }
            };
            let count = random() % 17 + 1;
            let mut sites = (0..count).map(|i| i * 131 + 23).collect::<Vec<_>>();
            let mut effects = BTreeMap::new();
            let mut edges = BTreeMap::new();
            let mut exits = BTreeSet::new();
            for &site in &sites {
                let mut effect = Effects::default();
                for _ in 0..random() % 7 {
                    effect.used(word(random()));
                }
                for _ in 0..random() % 7 {
                    effect.writes.insert(word(random()));
                }
                effects.insert(site, effect);
                edges.insert(
                    site,
                    (0..random() % 4).map(|_| sites[random() % count]).collect(),
                );
                if random() % 3 == 0 {
                    exits.insert(site);
                }
            }
            let terminal = (0..random() % 7).map(|_| word(random())).collect();
            sites.rotate_left(seed as usize % count);
            if seed % 2 == 0 {
                sites.reverse();
            }
            if seed % 4 == 0 {
                sites.push(sites[0]);
            }
            for &header in effects.keys() {
                let expected = reference(&effects, &edges, &exits, &terminal, header);
                assert_eq!(
                    required(&sites, &effects, &edges, &exits, &terminal, header),
                    expected,
                    "packed seed={seed} header={header}"
                );
                assert_eq!(
                    required_with_limit(&sites, &effects, &edges, &exits, &terminal, header, 0),
                    expected,
                    "fallback seed={seed} header={header}"
                );
            }
        }
    }

    #[test]
    fn sparse_word_and_site_ids_use_local_ordinals_and_respect_the_cap() {
        let effects = BTreeMap::from([
            (
                0,
                Effects {
                    reads: BTreeSet::new(),
                    writes: BTreeSet::from([usize::MAX - 1]),
                },
            ),
            (
                usize::MAX,
                Effects {
                    reads: BTreeSet::from([usize::MAX]),
                    writes: BTreeSet::new(),
                },
            ),
        ]);
        let edges = BTreeMap::from([(0, vec![usize::MAX]), (usize::MAX, vec![0])]);
        let exits = BTreeSet::from([usize::MAX]);
        let terminal = BTreeSet::from([64]);
        let expected = Some(BTreeSet::from([64, usize::MAX]));
        for limit in [0, 1, 2, MAX_DENSE_CELLS] {
            assert_eq!(
                required_with_limit(
                    &[0, usize::MAX],
                    &effects,
                    &edges,
                    &exits,
                    &terminal,
                    0,
                    limit
                ),
                expected
            );
        }
        let next = vec![vec![1], vec![0]];
        let reads = vec![BTreeSet::new(), BTreeSet::from([64usize, usize::MAX])];
        let kills = vec![vec![usize::MAX - 1], vec![]];
        assert_eq!(dense_live_before_indexed(&next, &reads, &kills, 0, 1), None);
        assert_eq!(
            dense_live_before_indexed(&next, &reads, &kills, 0, 2),
            expected
        );
        assert_eq!(MAX_DENSE_CELLS, 16 * 1024 * 1024);
    }

    #[test]
    fn empty_or_missing_header_retains_the_original_absence_result() {
        let terminal = BTreeSet::from([usize::MAX]);
        assert_eq!(
            required(
                &[],
                &BTreeMap::new(),
                &BTreeMap::new(),
                &BTreeSet::new(),
                &terminal,
                0
            ),
            None
        );
        let effects = BTreeMap::from([(2, Effects::default())]);
        let edges = BTreeMap::from([(2, vec![2])]);
        assert_eq!(
            required(&[2], &effects, &edges, &BTreeSet::from([2]), &terminal, 9),
            None
        );
    }

    #[test]
    fn long_reverse_cycle_has_the_same_header_input_as_the_old_full_sweeps() {
        let count = 200;
        let sites = (0..count).collect::<Vec<_>>();
        let effects = sites
            .iter()
            .map(|&site| {
                (
                    site,
                    Effects {
                        reads: if site == 0 {
                            BTreeSet::from([77])
                        } else {
                            BTreeSet::new()
                        },
                        writes: BTreeSet::new(),
                    },
                )
            })
            .collect();
        let edges = sites
            .iter()
            .map(|&site| (site, vec![(site + count - 1) % count]))
            .collect();
        let expected = Some(BTreeSet::from([77]));
        for limit in [0, MAX_DENSE_CELLS] {
            assert_eq!(
                required_with_limit(
                    &sites,
                    &effects,
                    &edges,
                    &BTreeSet::new(),
                    &BTreeSet::new(),
                    1,
                    limit
                ),
                expected
            );
        }
    }
}
