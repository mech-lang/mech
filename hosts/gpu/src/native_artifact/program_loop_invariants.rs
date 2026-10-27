//! Guard private call flags and peel adjacency only inside the existing
//! source-derived window-loop certificate. Unknown shapes keep checked code.
use super::*;

#[cfg(test)]
thread_local! {
    static DISABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static EMISSIONS: std::cell::Cell<(usize, usize, usize)> = const { std::cell::Cell::new((0, 0, 0)) };
}
pub(in crate::native_artifact) fn enabled() -> bool {
    #[cfg(test)]
    {
        !DISABLED.with(std::cell::Cell::get)
    }
    #[cfg(not(test))]
    {
        true
    }
}
#[cfg(test)]
pub(in crate::native_artifact) fn without_loop_invariants<T>(f: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            DISABLED.with(|v| v.set(self.0));
        }
    }
    let _restore = Restore(DISABLED.with(|v| v.replace(true)));
    f()
}
#[cfg(test)]
pub(in crate::native_artifact) fn take_invariant_emissions() -> (usize, usize, usize) {
    EMISSIONS.with(|v| v.replace((0, 0, 0)))
}

pub(super) struct Proof {
    pub steady_latches: BTreeSet<usize>,
}
pub(super) fn derive(
    code: CodePlan<'_>,
    closed: &Layout,
    id: usize,
    owner: &NativeMapOwnerWords,
    graph: &tag_threading::Graph,
    bounds: &loop_bounds::Proof,
    slots: &BTreeSet<usize>,
) -> Option<Proof> {
    // Four versions fit the existing 1024-node bound. These setup/refresh
    // bounds also leave space in the unchanged fixed work envelope.
    if !enabled() || graph.nodes.len() > 256 || slots.len() > 256 {
        return None;
    }
    let region = &closed.regions[id];
    let mut insertions = 0;
    let mut steady_latches = BTreeSet::new();
    for (index, node) in graph.nodes.iter().enumerate() {
        if owner.call_region(node.site) == region.map
            && let Action::LibraryCall { template, .. } = code.blocks()[node.site].action()
            && template.primitive() == P::MapInsert
        {
            // Refresh dispatch duplicates only a total source jump after the
            // successful operation, never a source action or runtime branch.
            if !matches!(node.edge, tag_threading::Edge::Jump(_)) {
                return None;
            }
            insertions += 1;
        }
        if bounds.recurrence_nodes.contains(&index)
            && let tag_threading::Edge::Jump(tag_threading::Target::Node(next)) = node.edge
            && graph.nodes[next].site == region.tail.header() as usize
            && graph.nodes[next].header_tag == Some(bounds.active_tag)
        {
            steady_latches.insert(index);
        }
    }
    if insertions == 0 || insertions > 4 || steady_latches.is_empty() {
        return None;
    }
    // The existing bound proof visits every productive path and requires one
    // exact selected range before every active recurrence, with index+1 and
    // invariant width. The owner admits only range and contains/get/insert;
    // insertion/refill do not change selected key words or previous_start.
    // Therefore only those completed recurrence edges may enter steady code.
    Some(Proof { steady_latches })
}

pub(super) fn calls_allowed(b: &mut FunctionBuilder<'_>, frame: &FrameState) -> Value {
    let mut allowed = b.ins().iconst(types::I8, 1);
    for field in [
        core::mem::offset_of!(Frame, range_call_allowed),
        core::mem::offset_of!(Frame, read_call_allowed),
        core::mem::offset_of!(Frame, write_call_allowed),
    ] {
        let word = frame.read(b, field);
        let nonzero = b.ins().icmp_imm(IntCC::NotEqual, word, 0);
        allowed = b.ins().band(allowed, nonzero);
    }
    allowed
}
pub(super) fn note(calls: usize, ranges: usize) {
    #[cfg(test)]
    EMISSIONS.with(|v| {
        let old = v.get();
        v.set((old.0 + 1, old.1 + calls, old.2 + ranges));
    });
    #[cfg(not(test))]
    let _ = (calls, ranges);
}
