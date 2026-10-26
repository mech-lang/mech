//! Test controls for the optional source-derived count-value induction. The
//! bounded source walk owns its proof; emission also requires the existing
//! empty packed owner and guarded source-index/update-tally relation.
use crate::native_artifact::compilation::{CompilationControls, OptimizationPass};

pub(super) fn enabled(controls: &CompilationControls) -> bool {
    controls.enabled(OptimizationPass::CountBounds)
}
