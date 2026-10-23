//! Shared descriptor/codegen decisions for conditional retained-map transport.
//! These identify sites; the invocation owner still grants execution authority.
use mech_engine::native_artifact::{
    NativeMapOwnerWords as Layout, NativeScalarAggregateAction as Action,
};
pub(super) fn action_region(layout: &Layout, site: usize) -> Option<usize> {
    layout.action_region(site)
}
pub(super) fn call_region(layout: &Layout, site: usize) -> Option<usize> {
    layout.call_region(site)
}
/// Fund constant-time runtime owner checks plus the bounded emitted word work.
/// Costs cover both the selected branch and its canonical fallback.
pub(super) fn extra_work(layout: &Layout, site: usize) -> Result<u64, &'static str> {
    if layout.regions().is_empty() { return Ok(0); }
    let mut words = 0usize;
    let mut guards = 0usize;
    if let Some(index) = layout.initializer(site) {
        words = layout.aggregates().slot(layout.regions()[index].tail().carrier())
            .ok_or("map owner initial slot missing")?.1;
    } else if layout.call_region(site).is_some() || layout.exit(site).is_some()
        || layout.span_site(site).is_some() {
        words = 4;
    } else if layout.action_region(site).is_some() {
        let action = layout.aggregates().action(site).ok_or("map owner action missing")?;
        let output = match action {
            Action::Pack { output, .. } | Action::Copy { output, .. } => *output,
            Action::Project { output, guards: checks, .. }
                | Action::Observe { output, guards: checks, .. } => { guards = checks.len(); *output },
        };
        words = layout.aggregates().slot(output).map_or(1, |slot| slot.1);
    }
    (words as u64).checked_mul(64)
        .and_then(|n| n.checked_add((guards as u64).checked_mul(64)?))
        .and_then(|n| n.checked_add(256)).ok_or("map owner visit work overflow")
}
pub(super) fn local_words(layout: &Layout) -> Result<usize, &'static str> {
    layout.frame_words().checked_add(layout.regions().len()).ok_or("map owner flag lane overflow")
}
pub(super) fn initial_words(layout: &Layout) -> usize {
    layout.regions().iter().map(|region|
        layout.aggregates().slot(region.tail().carrier()).expect("proved carrier slot").1)
        .max().unwrap_or(0)
}
