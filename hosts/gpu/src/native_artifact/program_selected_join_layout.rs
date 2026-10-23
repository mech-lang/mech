//! Immutable site associations derived only from the canonical complete-use proof.
use mech_engine::native_artifact::{NativeProgramArtifactPlan, NativeSelectedTextJoin};

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub(super) enum Site {
    Range(usize),
    Select(usize),
    Join(usize),
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Layout {
    #[serde(deserialize_with = "super::program_descriptor::image::proof")]
    pub regions: Box<[NativeSelectedTextJoin]>,
    pub sites: Box<[Option<Site>]>,
}

impl Layout {
    /// Both emitted native fuel and runtime visits use this SAME boundary cost.
    /// A selected join elsewhere in the program must not make a closed Tail's
    /// generated envelope disagree with the descriptor that reconciles it.
    pub(super) fn fund_visit_checks(
        &self,
        visits: &mut [super::iteration_abi::NativeIterationVisit],
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

    pub(super) fn derive(plan: &NativeProgramArtifactPlan) -> Result<Self, String> {
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
        Ok(Self { regions, sites })
    }
}

#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "program_selected_join_layout_tests.rs"]
mod tests;
