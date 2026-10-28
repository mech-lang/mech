//! Immutable site associations derived only from the canonical complete-use proof.
use mech_engine::native_artifact::{NativeProgramArtifactPlan, NativeSelectedTextJoin};
pub(in crate::native_artifact) mod mapped;

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub(in crate::native_artifact) enum Site {
    Range(usize),
    Select(usize),
    Join(usize),
}

#[derive(Debug, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Layout {
    pub regions: Box<[NativeSelectedTextJoin]>,
    pub sites: Box<[Option<Site>]>,
    pub maps: Box<[Option<mapped::Map>]>,
}

impl Layout {
    /// Both emitted native fuel and runtime visits use this SAME boundary cost.
    /// A selected join elsewhere in the program must not make a closed Tail's
    /// generated envelope disagree with the descriptor that reconciles it.
    pub(in crate::native_artifact) fn fund_visit_checks(
        &self,
        visits: &mut [crate::native_artifact::iteration_abi::NativeIterationVisit],
    ) -> Result<(), &'static str> {
        if visits.len() != self.sites.len() {
            return Err("selected join visit inventory mismatch");
        }
        if !self.regions.is_empty() {
            for visit in visits {
                visit.work = visit
                    .work
                    .checked_add(64)
                    .ok_or("selected join visit work overflow")?;
            }
        }
        Ok(())
    }

    pub(in crate::native_artifact) fn derive(
        plan: &NativeProgramArtifactPlan,
        string_maps: &[mech_engine::native_artifact::NativeStringMapRegion],
    ) -> Result<Self, String> {
        let regions = plan.selected_text_joins().into_boxed_slice();
        let mut sites = vec![None; plan.blocks().len()].into_boxed_slice();
        for (index, region) in regions.iter().enumerate() {
            for (site, role) in region
                .range()
                .map(|range| (range.producer(), Site::Range(index)))
                .into_iter()
                .chain([
                    (region.selection(), Site::Select(index)),
                    (region.consumer(), Site::Join(index)),
                ])
            {
                let entry = sites
                    .get_mut(site as usize)
                    .ok_or("selected join site outside plan")?;
                if entry.replace(role).is_some() {
                    return Err("selected joins share an original operation".into());
                }
            }
        }
        let maps = regions
            .iter()
            .map(|join| mapped::derive(plan, join, &string_maps))
            .collect();
        Ok(Self {
            regions,
            sites,
            maps,
        })
    }
}

#[cfg(all(test, feature = "canonical-native-program"))]
mod tests;
