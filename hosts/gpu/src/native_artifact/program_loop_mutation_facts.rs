//! Test controls and actual-emission observations for the source-derived
//! single-insertion/index relation. The range proof owns the CFG certificate;
//! the emitter additionally checks the initial runtime tally/index equality.
use crate::native_artifact::compilation::{CompilationControls, OptimizationPass};

pub(super) fn enabled(controls: &CompilationControls) -> bool {
    controls.enabled(OptimizationPass::MutationFacts)
}
