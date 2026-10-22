//! Immutable dispatch of the same plan's complete-use line-matrix proofs.
use mech_engine::native_artifact::{NativeProgramArtifactPlan, NativeTextLineMatrixRegion};

#[derive(Clone, Copy, Debug, serde::Serialize)]
pub(super) enum Site {
    Produce(usize),
    Length(usize),
}

#[derive(Default, Debug, serde::Serialize)]
pub(super) struct Layout {
    pub regions: Vec<NativeTextLineMatrixRegion>,
    pub sites: Vec<Option<Site>>,
    pub matrices: Vec<Option<usize>>,
}
impl Layout {
    pub fn derive(plan: &NativeProgramArtifactPlan) -> Result<Self, &'static str> {
        let mut layout = Self {
            regions: plan.text_line_matrix_regions(),
            sites: vec![None; plan.blocks().len()],
            matrices: vec![None; plan.values().len()],
        };
        for (ordinal, region) in layout.regions.iter().enumerate() {
            if layout.matrices[region.matrix().get() as usize]
                .replace(ordinal)
                .is_some()
                || layout.sites[region.producer() as usize]
                    .replace(Site::Produce(ordinal))
                    .is_some()
            {
                return Err("overlapping line matrix producers");
            }
            for &site in region.lengths() {
                if layout.sites[site as usize]
                    .replace(Site::Length(ordinal))
                    .is_some()
                {
                    return Err("overlapping line matrix lengths");
                }
            }
        }
        Ok(layout)
    }
}
