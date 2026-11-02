//! Test controls for the optional source-derived count-value induction. The
//! bounded source walk owns its proof; emission also requires the existing
//! empty packed owner and guarded source-index/update-tally relation.
use crate::native_artifact::codegen::options::{CompilationControls, OptimizationPass};

pub(in crate::native_artifact::program::codegen::packed_loop) fn enabled(
    controls: &CompilationControls,
) -> bool {
    controls.enabled(OptimizationPass::CountBounds)
}
