//! Prepay an acyclic native path between observable runtime boundaries.
//! Branches and scalar joins remain generated instructions. A selected map
//! helper may anchor a path, but no other helper or loop header is hidden.
use crate::native_artifact::iteration_abi::{NativeIterationVisit, Progress};
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeMapOwnerWords, NativeMatchAction as Scalar,
    NativeProgramArtifactPlan,
};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Frontier {
    pub site: u32,
    pub previous: u32,
}
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Group {
    pub owner: usize,
    pub sites: Box<[u32]>,
    pub frontier: Box<[Frontier]>,
    pub work: u64,
}
impl Group {
    pub fn start(&self) -> u32 {
        self.sites[0]
    }
    pub fn predecessor(&self, site: u32) -> Option<u32> {
        self.frontier
            .binary_search_by_key(&site, |edge| edge.site)
            .ok()
            .map(|index| self.frontier[index].previous)
    }
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Layout {
    pub groups: Box<[Group]>,
    pub sites: Box<[Option<usize>]>,
}
impl Layout {
    pub fn group(&self, site: usize) -> Option<&Group> {
        self.sites
            .get(site)
            .copied()
            .flatten()
            .map(|id| &self.groups[id])
    }

    pub fn derive(
        plan: &NativeProgramArtifactPlan,
        owners: &NativeMapOwnerWords,
        visits: &[NativeIterationVisit],
    ) -> Result<Self, &'static str> {
        if owners.regions().is_empty() {
            return Ok(Self::default());
        }
        let count = plan.blocks().len();
        if visits.len() != count {
            return Err("map admission visit inventory differs");
        }
        let mut predecessors = vec![Vec::new(); count];
        for (site, visit) in visits.iter().enumerate() {
            for next in visit.successors.into_iter().flatten() {
                predecessors
                    .get_mut(next as usize)
                    .ok_or("map admission successor outside plan")?
                    .push(site);
            }
        }
        let mut selected = vec![None; count];
        let mut roots = vec![None; count];
        let mut known = vec![false; count];
        for (site, block) in plan.blocks().iter().enumerate() {
            let visit = visits[site];
            let owner = owners
                .regions()
                .iter()
                .position(|region| block.iteration() == Some(region.iteration()));
            let Some(owner) = owner else {
                known[site] = true;
                continue;
            };
            let helper = owners.call_region(site) == Some(owner)
                || owners
                    .span_site(site)
                    .is_some_and(|id| owners.spans()[id].region() == owner);
            let native = owners.action_region(site) == Some(owner)
                || matches!(
                    block.action(),
                    Action::Scalar(Scalar::None | Scalar::Primitive { .. })
                )
                || matches!(block.action(), Action::Scalar(Scalar::Copy { output, .. })
                    if plan.values()[output.get() as usize].scalar_kind().is_some());
            if (!native && !helper)
                || visit.complete
                || visit.reject
                || visit.integrity
                || plan.activation_boundary() == Some(site as u32)
                || matches!(visit.progress, Progress::Initialize(_) | Progress::Exit(_))
            {
                known[site] = true;
                continue;
            }
            selected[site] = Some(owner);
            // Headers cut every declared backedge. Helpers cut every effect.
            // A Tail latch can be native: it changes no runtime counter, and
            // its precise retirement remains at the next header boundary.
            if helper
                || matches!(visit.progress, Progress::Header(_))
                || site == plan.entry() as usize
            {
                roots[site] = Some(site);
                known[site] = true;
            }
        }
        loop {
            let mut changed = false;
            for site in 0..count {
                if known[site] || !predecessors[site].iter().all(|&prior| known[prior]) {
                    continue;
                }
                let common = predecessors[site].first().and_then(|&prior| roots[prior]);
                roots[site] = Some(
                    if common.is_some_and(|root| {
                        selected[root] == selected[site]
                            && predecessors[site]
                                .iter()
                                .all(|&prior| roots[prior] == Some(root))
                    }) {
                        common.unwrap()
                    } else {
                        site
                    },
                );
                known[site] = true;
                changed = true;
            }
            if !changed {
                break;
            }
        }
        // Unresolved cycles decline grouping. Every included interior was
        // assigned only AFTER all its predecessors, proving an acyclic body
        // with one entry. A merge from another region starts a new boundary.
        let mut assigned = vec![None; count];
        let mut groups = Vec::new();
        for first in 0..count {
            if roots[first] != Some(first) {
                continue;
            }
            let mut sites = vec![first as u32];
            sites.extend(
                (0..count)
                    .filter(|&site| site != first && roots[site] == Some(first))
                    .map(|site| site as u32),
            );
            if sites.len() < 2 {
                continue;
            }
            let mut frontier = Vec::new();
            let mut work = 128u64;
            for &site in &sites {
                work = work
                    .checked_add(visits[site as usize].work)
                    .and_then(|n| n.checked_add(16))
                    .ok_or("map admission work overflow")?;
                for next in visits[site as usize].successors.into_iter().flatten() {
                    if roots[next as usize] != Some(first) || next as usize == first {
                        frontier.push(Frontier {
                            site: next,
                            previous: site,
                        });
                    }
                }
            }
            frontier.sort_unstable_by_key(|edge| edge.site);
            frontier.dedup_by_key(|edge| edge.site);
            work = work
                .checked_add(
                    (frontier.len() as u64)
                        .checked_mul(16)
                        .ok_or("map frontier work overflow")?,
                )
                .ok_or("map admission work overflow")?;
            let id = groups.len();
            for &site in &sites {
                assigned[site as usize] = Some(id);
            }
            groups.push(Group {
                owner: selected[first].unwrap(),
                sites: sites.into_boxed_slice(),
                frontier: frontier.into_boxed_slice(),
                work,
            });
        }
        Ok(Self {
            groups: groups.into_boxed_slice(),
            sites: assigned.into_boxed_slice(),
        })
    }
}
