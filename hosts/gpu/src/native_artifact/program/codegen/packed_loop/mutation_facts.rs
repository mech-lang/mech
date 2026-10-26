//! Test controls and actual-emission observations for the source-derived
//! single-insertion/index relation. The range proof owns the CFG certificate;
//! the emitter additionally checks the initial runtime tally/index equality.
use crate::native_artifact::codegen::options::{CompilationControls, OptimizationPass};

pub(in crate::native_artifact::program::codegen::packed_loop) fn enabled(
    controls: &CompilationControls,
) -> bool {
    controls.enabled(OptimizationPass::MutationFacts)
}
