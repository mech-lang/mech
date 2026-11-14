//! Mechanical graph query only. Consumers decide which reads and owners qualify.
use super::{NativeProgramFacts, NativeProgramReadRole as Read};
use crate::native_artifact::{
    NativeCollectionResult, NativeCollectionSource, NativeControlLoop as Loop, NativeMatchValue,
    NativeProgramArtifactPlan as Plan,
};
use std::collections::BTreeMap;

// This scratch belongs to one immutable plan borrow. The limit bounds optional
// per-value graph indexes, not program size or ownership eligibility.
const MAX_LIVE_BYTES: usize = 32 * 1024 * 1024;
const FORWARD_VISITS: usize = 128;

#[derive(Default)]
pub(super) struct ReadCache {
    values: BTreeMap<u32, Option<ReadSites>>,
    scratch: TraversalScratch,
    live_bytes: usize,
}

struct ReadSites {
    reads: Box<[(u32, bool)]>,
    definitions: Box<[u32]>,
    live: Option<Box<[u64]>>,
    forward_only: bool,
}

#[derive(Default)]
struct TraversalScratch {
    seen: Vec<u32>,
    epoch: u32,
    pending: Vec<u32>,
    reverse_pending: Vec<u32>,
}

impl TraversalScratch {
    fn begin(&mut self, block_count: usize) {
        self.pending.clear();
        self.seen.resize(block_count, 0);
        if self.epoch == u32::MAX {
            self.seen.fill(0);
            self.epoch = 1;
        } else {
            self.epoch += 1;
        }
    }
}

impl ReadSites {
    fn new(reads: Vec<(u32, bool)>, mut definitions: Vec<u32>) -> Self {
        definitions.sort_unstable();
        definitions.dedup();
        Self {
            reads: reads.into_boxed_slice(),
            definitions: definitions.into_boxed_slice(),
            live: None,
            forward_only: false,
        }
    }

    fn reads(&self, site: u32, terminator: bool) -> bool {
        self.reads.binary_search(&(site, terminator)).is_ok()
    }

    fn writes(&self, site: u32) -> bool {
        self.definitions.binary_search(&site).is_ok()
    }

    fn query<'a>(
        &mut self,
        block_count: usize,
        frontier: u32,
        successors: impl Fn(u32) -> Result<[Option<u32>; 2], &'static str>,
        predecessors: impl Fn(u32) -> Result<&'a [u32], &'static str>,
        scratch: &mut TraversalScratch,
        live_bytes: &mut usize,
        forward_visits: usize,
    ) -> Result<bool, &'static str> {
        if frontier as usize >= block_count {
            return Err("old-value frontier outside control graph");
        }
        if self.reads(frontier, true) {
            return Ok(true);
        }
        if let Some(live) = &self.live {
            return Ok(successors(frontier)?
                .into_iter()
                .flatten()
                .any(|site| live[site as usize / 64] & (1 << (site % 64)) != 0));
        }
        // Cheap queries retain the original ordered walk. An expensive query
        // switches direction: start at reads and stop at fresh definitions,
        // rather than scan an entire later program just to prove absence.
        if let Some(found) = reachable_read_with_scratch(
            block_count,
            frontier,
            &successors,
            |site| self.writes(site),
            |site| self.reads(site, false),
            |site| self.reads(site, true),
            scratch,
            if self.forward_only {
                usize::MAX
            } else {
                forward_visits
            },
        )? {
            return Ok(found);
        }
        let bytes = block_count.div_ceil(64).saturating_mul(8);
        if bytes <= MAX_LIVE_BYTES.saturating_sub(*live_bytes) {
            // Global graph validation can fail on an edge the original query
            // never visits. Such failures resume the ordered walk, preserving
            // its early result and error order. Reverse scratch is independent
            // of the suspended forward worklist.
            if let Ok(live) = predecessors(frontier).and_then(|_| {
                self.reverse_live(block_count, &predecessors, &mut scratch.reverse_pending)
            }) {
                let found = successors(frontier)?
                    .into_iter()
                    .flatten()
                    .any(|site| live[site as usize / 64] & (1 << (site % 64)) != 0);
                *live_bytes += bytes;
                self.live = Some(live);
                return Ok(found);
            }
        }
        self.forward_only = true;
        walk_pending(
            successors,
            |site| self.writes(site),
            |site| self.reads(site, false),
            |site| self.reads(site, true),
            scratch,
            usize::MAX,
        )
        .map(|found| found.expect("unbounded old-value walk"))
    }

    fn reverse_live<'a>(
        &self,
        block_count: usize,
        predecessors: impl Fn(u32) -> Result<&'a [u32], &'static str>,
        pending: &mut Vec<u32>,
    ) -> Result<Box<[u64]>, &'static str> {
        let mut live = vec![0_u64; block_count.div_ceil(64)];
        pending.clear();
        for &(site, terminator) in self.reads.iter() {
            if (site as usize) < block_count && (!terminator || !self.writes(site)) {
                if mark_live(&mut live, site) {
                    pending.push(site);
                }
            }
        }
        while let Some(site) = pending.pop() {
            for &previous in predecessors(site)? {
                // An action reads before it writes; action-read definitions are
                // already seeds above. All other definitions kill the old owner
                // before their terminator and successors.
                if !self.writes(previous) && mark_live(&mut live, previous) {
                    pending.push(previous);
                }
            }
        }
        Ok(live.into_boxed_slice())
    }
}

fn mark_live(live: &mut [u64], site: u32) -> bool {
    let word = &mut live[site as usize / 64];
    let bit = 1 << (site % 64);
    let fresh = *word & bit == 0;
    *word |= bit;
    fresh
}

impl NativeProgramFacts<'_> {
    /// Exact original visit of a descriptive read. The bool marks a terminator
    /// read after the action. Collection/publication and effect reads retain
    /// independent ownership frontiers and return None. This mechanical mapping
    /// grants neither liveness, ownership nor permission to execute a visit.
    pub fn control_read_site(plan: &Plan, role: Read) -> Option<(u32, bool)> {
        Some(match role {
            Read::Action(site) => (site, false),
            Read::Terminator(site) => (site, true),
            Read::LoopSeed(id) | Read::LoopNext(id) => {
                let seed = matches!(role, Read::LoopSeed(_));
                let site = match plan.iterations().get(id as usize)? {
                    Loop::Tail(t) => {
                        if seed {
                            t.initialize()
                        } else {
                            t.latch()
                        }
                    }
                    Loop::Iteration(t) => {
                        if seed {
                            t.initialize()
                        } else {
                            t.latch()
                        }
                    }
                    Loop::Collection(_) => return None,
                };
                (site, false)
            }
            Read::LoopReturned(id) => (
                match plan.iterations().get(id as usize)? {
                    Loop::Tail(t) => t.finish(),
                    _ => return None,
                },
                true,
            ),
            _ => return None,
        })
    }

    /// Same old-owner query with loop reads placed at their actual visits.
    /// Unknown ownership frontiers remain conservative; no read is discarded.
    pub(crate) fn old_value_has_future_reads(
        &self,
        value: NativeMatchValue,
        frontier: u32,
    ) -> Result<bool, &'static str> {
        let Ok(mut cache) = self.owner_reads.try_borrow_mut() else {
            let Some(reads) = self.old_value_read_sites(value)? else {
                return Ok(true);
            };
            return self.old_value_read_before_redefinition(
                value,
                frontier,
                |site| reads.binary_search(&(site, false)).is_ok(),
                |site| reads.binary_search(&(site, true)).is_ok(),
            );
        };
        let ReadCache {
            values,
            scratch,
            live_bytes,
        } = &mut *cache;
        let entry = match values.entry(value.get()) {
            std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
            std::collections::btree_map::Entry::Vacant(entry) => {
                let reads = self.old_value_read_sites(value)?;
                let sites = match reads {
                    Some(reads) => Some(ReadSites::new(
                        reads,
                        self.definition_sites(value.get())?
                            .iter()
                            .filter_map(|definition| definition.site())
                            .collect(),
                    )),
                    None => None,
                };
                entry.insert(sites)
            }
        };
        match entry {
            None => Ok(true),
            Some(sites) => sites.query(
                self.plan().blocks().len(),
                frontier,
                |site| self.successors(site),
                |site| self.predecessors(site),
                scratch,
                live_bytes,
                FORWARD_VISITS,
            ),
        }
    }

    fn old_value_read_sites(
        &self,
        value: NativeMatchValue,
    ) -> Result<Option<Vec<(u32, bool)>>, &'static str> {
        let mut reads = Vec::new();
        for &role in self.read_roles(value)? {
            if let Read::CollectionSource(iteration) = role {
                let Some(Loop::Collection(generator)) =
                    self.plan().iterations().get(iteration as usize)
                else {
                    return Ok(None);
                };
                let Some(collection) = self
                    .plan()
                    .collections()
                    .get(generator.collection() as usize)
                else {
                    return Ok(None);
                };
                // Specialized consumers retain their independent ownership
                // frontiers. Only the ordinary materialized lifecycle is finite
                // here; this does not broaden private-frame eligibility.
                if !matches!(generator.source(), NativeCollectionSource::Value(_))
                    || !matches!(collection.result(), NativeCollectionResult::Materialized)
                    || collection.string_span_projection().is_some()
                    || collection.string_element_map_projection().is_some()
                {
                    return Ok(None);
                }
                // Keep the arena source through initialization, every active
                // generator boundary, and source-pin release at Exit. A shape-
                // preserving finish reads the arena source AFTER that release.
                // Include finish for all ordinary results conservatively. CFG
                // backedges and fresh definitions still use the same old-owner
                // query below; the runtime's current exclusivity gate remains.
                reads.extend([
                    (generator.initialize(), false),
                    (generator.header(), false),
                    (generator.body(), false),
                    (generator.latch(), false),
                    (generator.exit(), false),
                    (collection.finish(), false),
                ]);
            } else if let Some(site) = Self::control_read_site(self.plan(), role) {
                reads.push(site);
            } else {
                return Ok(None);
            }
        }
        // Repeated graph probes must not scan every descriptive read. The
        // role validation above retains its original order and opaque cases.
        reads.sort_unstable();
        reads.dedup();
        Ok(Some(reads))
    }

    /// An action reads before writing its result; a redefining action kills the
    /// old owner before its terminator. Includes backedges and the frontier's
    /// terminator. Opaque reads and loop/publication roles remain caller policy.
    pub(crate) fn old_value_read_before_redefinition(
        &self,
        value: NativeMatchValue,
        frontier: u32,
        action_reads: impl Fn(u32) -> bool,
        terminator_reads: impl Fn(u32) -> bool,
    ) -> Result<bool, &'static str> {
        let definitions = self.definition_sites(value.get())?;
        reachable_read(
            self.plan().blocks().len(),
            frontier,
            |site| self.successors(site),
            |site| {
                definitions
                    .iter()
                    .any(|definition| definition.site() == Some(site))
            },
            action_reads,
            terminator_reads,
        )
    }
}

fn reachable_read_with_scratch(
    block_count: usize,
    frontier: u32,
    successors: impl Fn(u32) -> Result<[Option<u32>; 2], &'static str>,
    writes: impl Fn(u32) -> bool,
    action_reads: impl Fn(u32) -> bool,
    terminator_reads: impl Fn(u32) -> bool,
    scratch: &mut TraversalScratch,
    max_visits: usize,
) -> Result<Option<bool>, &'static str> {
    if frontier as usize >= block_count {
        return Err("old-value frontier outside control graph");
    }
    if terminator_reads(frontier) {
        return Ok(Some(true));
    }
    scratch.begin(block_count);
    scratch
        .pending
        .extend(successors(frontier)?.into_iter().flatten());
    walk_pending(
        successors,
        writes,
        action_reads,
        terminator_reads,
        scratch,
        max_visits,
    )
}

fn walk_pending(
    successors: impl Fn(u32) -> Result<[Option<u32>; 2], &'static str>,
    writes: impl Fn(u32) -> bool,
    action_reads: impl Fn(u32) -> bool,
    terminator_reads: impl Fn(u32) -> bool,
    scratch: &mut TraversalScratch,
    max_visits: usize,
) -> Result<Option<bool>, &'static str> {
    let mut visited = 0;
    while !scratch.pending.is_empty() {
        if visited == max_visits {
            return Ok(None);
        }
        let site = scratch.pending.pop().unwrap();
        let seen = scratch
            .seen
            .get_mut(site as usize)
            .ok_or("old-value edge outside control graph")?;
        if core::mem::replace(seen, scratch.epoch) == scratch.epoch {
            continue;
        }
        visited += 1;
        if action_reads(site) {
            return Ok(Some(true));
        }
        if writes(site) {
            continue;
        }
        if terminator_reads(site) {
            return Ok(Some(true));
        }
        scratch
            .pending
            .extend(successors(site)?.into_iter().flatten());
    }
    Ok(Some(false))
}

fn reachable_read(
    block_count: usize,
    frontier: u32,
    successors: impl Fn(u32) -> Result<[Option<u32>; 2], &'static str>,
    writes: impl Fn(u32) -> bool,
    action_reads: impl Fn(u32) -> bool,
    terminator_reads: impl Fn(u32) -> bool,
) -> Result<bool, &'static str> {
    if frontier as usize >= block_count {
        return Err("old-value frontier outside control graph");
    }
    if terminator_reads(frontier) {
        return Ok(true);
    }
    let mut seen = vec![false; block_count];
    let mut pending = successors(frontier)?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    while let Some(site) = pending.pop() {
        let seen = seen
            .get_mut(site as usize)
            .ok_or("old-value edge outside control graph")?;
        if core::mem::replace(seen, true) {
            continue;
        }
        if action_reads(site) {
            return Ok(true);
        }
        if writes(site) {
            continue;
        }
        if terminator_reads(site) {
            return Ok(true);
        }
        pending.extend(successors(site)?.into_iter().flatten());
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_LIVE_BYTES, ReadSites, TraversalScratch, reachable_read, reachable_read_with_scratch,
    };

    fn indexed_query(
        graph: &[[Option<u32>; 2]],
        frontier: u32,
        sites: &mut ReadSites,
        scratch: &mut TraversalScratch,
        bytes: &mut usize,
        limit: usize,
    ) -> Result<bool, &'static str> {
        let mut predecessors = vec![Vec::new(); graph.len()];
        let mut valid = true;
        for (site, next) in graph.iter().enumerate() {
            for &next in next.iter().flatten() {
                if let Some(row) = predecessors.get_mut(next as usize) {
                    row.push(site as u32);
                } else {
                    valid = false;
                }
            }
        }
        sites.query(
            graph.len(),
            frontier,
            |site| graph.get(site as usize).copied().ok_or("missing edge"),
            |site| {
                if !valid {
                    return Err("invalid whole graph");
                }
                predecessors
                    .get(site as usize)
                    .map(Vec::as_slice)
                    .ok_or("missing predecessor")
            },
            scratch,
            bytes,
            limit,
        )
    }

    fn equivalent_queries(
        graph: &[[Option<u32>; 2]],
        writes: Vec<u32>,
        mut reads: Vec<(u32, bool)>,
    ) {
        reads.sort_unstable();
        reads.dedup();
        for limit in [0, 1, 128, usize::MAX] {
            let mut sites = ReadSites::new(reads.clone(), writes.clone());
            let mut scratch = TraversalScratch::default();
            let mut bytes = 0;
            for pass in 0..2 {
                for frontier in 0..graph.len() as u32 + 2 {
                    let expected = reachable_read(
                        graph.len(),
                        frontier,
                        |site| graph.get(site as usize).copied().ok_or("missing edge"),
                        |site| writes.contains(&site),
                        |site| reads.contains(&(site, false)),
                        |site| reads.contains(&(site, true)),
                    );
                    assert_eq!(
                        indexed_query(graph, frontier, &mut sites, &mut scratch, &mut bytes, limit,),
                        expected,
                        "graph={graph:?} writes={writes:?} reads={reads:?} frontier={frontier} pass={pass} limit={limit}"
                    );
                }
            }
        }
    }

    #[test]
    fn suspended_walk_resumes_exact_pending_edges_when_index_is_unavailable() {
        for malformed in [false, true] {
            let graph = (0..260)
                .map(|site| {
                    [
                        if site == 259 && !malformed {
                            None
                        } else {
                            Some(site + 1)
                        },
                        None,
                    ]
                })
                .collect::<Vec<_>>();
            for read in [0, 129, 259, 261] {
                for write in [0, 130, 259, 261] {
                    let mut sites = ReadSites::new(vec![(read, false)], vec![write]);
                    let expected = reachable_read(
                        graph.len(),
                        0,
                        |site| graph.get(site as usize).copied().ok_or("missing edge"),
                        |site| site == write,
                        |site| site == read,
                        |_| false,
                    );
                    let mut bytes = MAX_LIVE_BYTES;
                    assert_eq!(
                        indexed_query(
                            &graph,
                            0,
                            &mut sites,
                            &mut TraversalScratch::default(),
                            &mut bytes,
                            128
                        ),
                        expected
                    );
                }
            }
        }
    }

    #[test]
    fn reverse_index_matches_every_two_block_read_write_and_edge_combination() {
        let edge = |n: u32| match n % 3 {
            0 => None,
            n => Some(n - 1),
        };
        for edges in 0..81 {
            let graph = [
                [edge(edges), edge(edges / 3)],
                [edge(edges / 9), edge(edges / 27)],
            ];
            for masks in 0..64 {
                let writes = (0..2).filter(|n| masks & (1 << n) != 0).collect();
                let reads = (0..4)
                    .filter(|n| masks & (1 << (n + 2)) != 0)
                    .map(|n| (n % 2, n >= 2))
                    .collect();
                equivalent_queries(&graph, writes, reads);
            }
        }
    }

    #[test]
    fn adaptive_index_preserves_loops_kills_and_malformed_graph_errors() {
        for seed in 0..256 {
            let graph = (0..12)
                .map(|site| {
                    [
                        Some((site + seed + 1) % 12),
                        (site % 3 == 0).then_some((site * 3 + seed) % 13),
                    ]
                })
                .collect::<Vec<_>>();
            let writes = (0..14).filter(|site| (site + seed) % 5 == 0).collect();
            let reads = (0..30).map(|n| ((n * 7 + seed) % 14, n % 3 == 0)).collect();
            equivalent_queries(&graph, writes, reads);
        }
        // An unrelated malformed edge must not turn an early success or a
        // killed path into an error when global reverse indexing is attempted.
        let graph = [[Some(1), None], [None, Some(3)]];
        equivalent_queries(&graph, vec![], vec![(1, false)]);
        equivalent_queries(&graph, vec![1], vec![]);
        equivalent_queries(&[], vec![], vec![(0, true)]);
    }

    #[test]
    fn scratch_reuse_and_epoch_wrap_preserve_ordered_errors() {
        let graph = [[Some(1), Some(2)], [Some(3), None], [Some(0), None]];
        let mut scratch = TraversalScratch::default();
        for epoch in [0, 1, u32::MAX - 1, u32::MAX] {
            scratch.epoch = epoch;
            for read in 0..4 {
                let expected = reachable_read(
                    graph.len(),
                    0,
                    |site| graph.get(site as usize).copied().ok_or("missing edge"),
                    |site| site == 1,
                    |site| site == read,
                    |_| false,
                );
                assert_eq!(
                    reachable_read_with_scratch(
                        graph.len(),
                        0,
                        |site| graph.get(site as usize).copied().ok_or("missing edge"),
                        |site| site == 1,
                        |site| site == read,
                        |_| false,
                        &mut scratch,
                        usize::MAX,
                    )
                    .map(|result| result.unwrap()),
                    expected
                );
            }
        }
    }

    #[test]
    fn exhausted_index_budget_keeps_exact_forward_answers() {
        let graph = [[Some(1), None], [Some(0), None]];
        let mut sites = ReadSites::new(vec![(0, false)], vec![]);
        let mut bytes = MAX_LIVE_BYTES;
        assert_eq!(
            indexed_query(
                &graph,
                0,
                &mut sites,
                &mut TraversalScratch::default(),
                &mut bytes,
                0,
            ),
            Ok(true)
        );
        assert!(sites.live.is_none());
        assert!(sites.forward_only);
        assert_eq!(bytes, MAX_LIVE_BYTES);
    }

    fn query(
        graph: &[[Option<u32>; 2]],
        writes: &[u32],
        action: &[u32],
        term: &[u32],
    ) -> Result<bool, &'static str> {
        reachable_read(
            graph.len(),
            0,
            |site| graph.get(site as usize).copied().ok_or("missing edge"),
            |site| writes.contains(&site),
            |site| action.contains(&site),
            |site| term.contains(&site),
        )
    }

    #[test]
    fn old_value_query_preserves_action_write_terminator_order() {
        let graph = [
            [Some(1), None],
            [Some(2), None],
            [Some(3), None],
            [None, None],
        ];
        assert_eq!(query(&graph, &[2], &[2], &[]), Ok(true));
        assert_eq!(query(&graph, &[2], &[], &[2]), Ok(false));
        assert_eq!(query(&graph, &[2], &[3], &[]), Ok(false));
        assert_eq!(query(&graph, &[2], &[], &[1]), Ok(true));
        assert_eq!(query(&graph, &[2], &[], &[0]), Ok(true));
        // The consuming action itself has already read; only future reads count.
        assert_eq!(query(&graph, &[2], &[0], &[]), Ok(false));
    }

    #[test]
    fn old_value_query_handles_backedges_branches_and_fresh_definitions() {
        let mut graph = [
            [Some(1), None],
            [Some(2), Some(3)],
            [Some(0), None],
            [None, None],
        ];
        assert_eq!(query(&graph, &[], &[0], &[]), Ok(true));
        assert_eq!(query(&graph, &[2], &[0], &[]), Ok(false));
        assert_eq!(query(&graph, &[2], &[3], &[]), Ok(true));
        graph[3] = [Some(3), None];
        assert_eq!(query(&graph, &[2], &[], &[]), Ok(false));
        graph[3] = [Some(4), None];
        assert!(query(&graph, &[], &[], &[]).is_err());
    }
    #[test]
    fn indexed_read_membership_preserves_paths_and_graph_errors() {
        for seed in 0..256_u32 {
            let graph = (0..12)
                .map(|site| {
                    [
                        Some((site + seed + 1) % 12),
                        (site % 3 == 0).then_some((site * 3 + seed) % 13),
                    ]
                })
                .collect::<Vec<_>>();
            let reads = (0..30)
                .map(|n| ((n * 7 + seed) % 14, n % 3 == 0))
                .collect::<Vec<_>>();
            let mut indexed = reads.clone();
            indexed.sort_unstable();
            indexed.dedup();
            for frontier in 0..14 {
                let query = |indexed_membership: bool| {
                    reachable_read(
                        graph.len(),
                        frontier,
                        |site| graph.get(site as usize).copied().ok_or("missing edge"),
                        |site| (site + seed) % 5 == 0,
                        |site| {
                            if indexed_membership {
                                indexed.binary_search(&(site, false)).is_ok()
                            } else {
                                reads.contains(&(site, false))
                            }
                        },
                        |site| {
                            if indexed_membership {
                                indexed.binary_search(&(site, true)).is_ok()
                            } else {
                                reads.contains(&(site, true))
                            }
                        },
                    )
                };
                assert_eq!(query(true), query(false));
            }
        }
    }
}
