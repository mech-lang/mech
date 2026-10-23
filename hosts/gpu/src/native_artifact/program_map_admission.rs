//! Admit a straight-line run of native owner words once, before its first
//! instruction. Every original instruction and failure origin is still emitted.
//! No library helper, loop boundary, alternate entry or backedge is hidden.
use super::iteration_abi::{NativeIterationVisit, Progress};
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeMapOwnerWords, NativeMatchAction as Scalar,
    NativeProgramArtifactPlan,
};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Group {
    pub owner: usize,
    pub sites: Box<[u32]>,
    pub work: u64,
}
impl Group {
    pub fn start(&self) -> u32 { self.sites[0] }
    pub fn end(&self) -> u32 { *self.sites.last().unwrap() }
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Layout {
    pub groups: Box<[Group]>,
    pub sites: Box<[Option<usize>]>,
}
impl Layout {
    pub fn group(&self, site: usize) -> Option<&Group> {
        self.sites.get(site).copied().flatten().map(|id| &self.groups[id])
    }

    pub fn derive(
        plan: &NativeProgramArtifactPlan, owners: &NativeMapOwnerWords,
        visits: &[NativeIterationVisit],
    ) -> Result<Self, &'static str> {
        if owners.regions().is_empty() { return Ok(Self::default()); }
        let count = plan.blocks().len();
        if visits.len() != count { return Err("map admission visit inventory differs"); }
        let mut predecessors = vec![0usize; count];
        for visit in visits {
            for next in visit.successors.into_iter().flatten() {
                let incoming = predecessors.get_mut(next as usize)
                    .ok_or("map admission successor outside plan")?;
                *incoming = incoming.checked_add(1).ok_or("map admission predecessors overflow")?;
            }
        }
        let mut selected = vec![None; count];
        for (site, block) in plan.blocks().iter().enumerate() {
            let visit = visits[site];
            if visit.progress != Progress::Normal || visit.complete || visit.reject
                || plan.activation_boundary() == Some(site as u32) {
                continue;
            }
            let Some(owner) = owners.regions().iter().position(|region|
                block.iteration() == Some(region.iteration())) else { continue; };
            // The owning lane makes these structural packs/projections/copies
            // native words. Refused owner entry retains all canonical callbacks.
            let native = owners.action_region(site) == Some(owner)
                || matches!(block.action(), Action::Scalar(Scalar::None | Scalar::Primitive { .. }))
                || matches!(block.action(), Action::Scalar(Scalar::Copy { output, .. })
                    if plan.values()[output.get() as usize].scalar_kind().is_some());
            if native { selected[site] = Some(owner); }
        }
        let mut assigned = vec![None; count];
        let mut groups = Vec::new();
        for first in 0..count {
            let Some(owner) = selected[first] else { continue; };
            if assigned[first].is_some() { continue; }
            let mut sites = vec![first as u32];
            let mut work = visits[first].work;
            let mut site = first;
            while let [Some(next), None] = visits[site].successors {
                let next = next as usize;
                if next == first || predecessors[next] != 1 || selected[next] != Some(owner)
                    || assigned[next].is_some() || next == plan.entry() as usize {
                    break;
                }
                sites.push(next as u32);
                work = work.checked_add(visits[next].work).ok_or("map admission work overflow")?;
                site = next;
            }
            if sites.len() < 2 { continue; }
            // Constant-time entry/end checks and optional exceptional-origin
            // membership are paid before any instruction in this group runs.
            work = work.checked_add((sites.len() as u64).checked_mul(16)
                .and_then(|n| n.checked_add(128)).ok_or("map admission proof cost overflow")?)
                .ok_or("map admission work overflow")?;
            let id = groups.len();
            for &site in &sites { assigned[site as usize] = Some(id); }
            groups.push(Group { owner, sites: sites.into_boxed_slice(), work });
        }
        Ok(Self { groups: groups.into_boxed_slice(), sites: assigned.into_boxed_slice() })
    }
}
