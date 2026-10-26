//! Same-plan selected query associations shared by preparation and runtime lookup.
use mech_engine::native_artifact::{
    NativeIterationAction, NativePreparedLibraryCall, NativeProgramArtifactPlan,
};
pub(in crate::native_artifact) fn query_step(
    plan: &NativeProgramArtifactPlan,
    collection: u32,
    ordinal: u32,
) -> Result<&mech_engine::native_artifact::NativeHistogramQueryStep, &'static str> {
    let Some(mech_engine::native_artifact::NativeCollectionResult::GuardedHistogramQuery(sink)) =
        plan.collections()
            .get(collection as usize)
            .map(|c| c.result())
    else {
        return Err("histogram query stage lacks selected proof");
    };
    sink.steps()
        .get(ordinal as usize)
        .ok_or("histogram query step outside proof")
}
/// Compiler-only full immutable association proof. This runs for JIT, Object
/// preparation and linked reconstruction, after canonical current-visit proof
/// and SAME actual artifact/catalog/factory preparation of every selected call.
pub(in crate::native_artifact) fn validate_query_stages(
    plan: &NativeProgramArtifactPlan,
    calls: &[Option<NativePreparedLibraryCall>],
) -> Result<(), &'static str> {
    if calls.len() != plan.blocks().len() {
        return Err("histogram query call lane length differs");
    }
    for (site, block) in plan.blocks().iter().enumerate() {
        let NativeIterationAction::HistogramQueryStep {
            collection,
            step: ordinal,
        } = block.action()
        else {
            continue;
        };
        let step = query_step(plan, *collection, *ordinal)?;
        if step.site() as usize != site {
            return Err("histogram query original site differs");
        }
        match (step.template(), calls[site].as_ref()) {
            (Some(template), Some(selected)) if selected.template() == template => {}
            (None, None) => {}
            _ => return Err("histogram query selected template differs"),
        }
    }
    Ok(())
}

#[cfg(all(test, feature = "canonical-native-program"))]
mod tests;
