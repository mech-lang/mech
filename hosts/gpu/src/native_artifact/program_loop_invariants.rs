//! Guard private call flags and peel adjacency only inside the existing
//! source-derived window-loop certificate. Unknown shapes keep checked code.
use super::*;
use crate::native_artifact::compilation::{CompilationControls, OptimizationPass};

pub(in crate::native_artifact) fn enabled(controls: &CompilationControls) -> bool {
    controls.enabled(OptimizationPass::LoopInvariants)
}

pub(super) struct Proof {
    pub steady_latches: BTreeSet<usize>,
}
pub(super) fn derive(
    controls: &CompilationControls,
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
    if !enabled(controls) || graph.nodes.len() > 256 || slots.len() > 256 {
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
