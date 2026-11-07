//! Same-plan last-use candidates for selected packed buffer operations.
//! Runtime exclusivity and the original current-visit checks remain mandatory.
use super::{
    NativeCollectionPrimitive as Primitive, NativeIterationAction as Action,
    NativeMatchAction as Scalar, NativeMatchValue as Value, NativeProgramArtifactPlan as Plan,
    NativeProgramFacts as Facts,
};
#[path = "unsigned_buffer_aliases.rs"]
mod aliases;

#[derive(Clone, Debug)]
pub struct NativeUnsignedBufferCall {
    site: u32,
    source: Option<Value>,
    sublist: Option<Value>,
    owned_source_slots: Box<[Value]>,
}
impl NativeUnsignedBufferCall {
    pub fn site(&self) -> u32 {
        self.site
    }
    pub fn source(&self) -> Option<Value> {
        self.source
    }
    pub fn sublist(&self) -> Option<Value> {
        self.sublist
    }
    pub fn owned_source_slots(&self) -> &[Value] {
        &self.owned_source_slots
    }
}
impl Plan {
    /// A fresh derived cache, not serialized ownership or source-site authority.
    pub fn unsigned_buffer_calls(&self) -> Vec<Option<NativeUnsignedBufferCall>> {
        self.facts().unsigned_buffer_calls()
    }
}
impl Facts<'_> {
    pub fn unsigned_buffer_calls(&self) -> Vec<Option<NativeUnsignedBufferCall>> {
        let facts = self;
        let plan = facts.plan();
        if !plan.blocks().iter().any(|block| matches!(block.action(), Action::LibraryCall { template, .. }
            if matches!(template.primitive(), Primitive::MatrixIndexedReplace | Primitive::SequenceWithCapacity | Primitive::SequenceReserve | Primitive::SequenceAppend | Primitive::SequenceAppendLeBytes | Primitive::SequenceAppendSublist | Primitive::SequenceClear))) {
            return Vec::new();
        }
        plan.blocks()
            .iter()
            .enumerate()
            .map(|(site, block)| {
                let Action::LibraryCall {
                    template, operands, ..
                } = block.action()
                else {
                    return None;
                };
                let source = match template.primitive() {
                    Primitive::SequenceWithCapacity => None,
                    Primitive::MatrixIndexedReplace
                    | Primitive::SequenceReserve
                    | Primitive::SequenceAppend
                    | Primitive::SequenceAppendLeBytes
                    | Primitive::SequenceAppendSublist
                    | Primitive::SequenceClear => Some(*operands.first()?),
                    _ => return None,
                };
                let sublist = (template.primitive() == Primitive::SequenceAppendSublist)
                    .then(|| operands.get(1).copied())
                    .flatten();
                let owned_source_slots = source
                    .filter(|source| can_retire(facts, *source, site as u32))
                    .and_then(|source| aliases::possible(facts, source).ok())
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|value| can_retire(facts, *value, site as u32))
                    .collect();
                Some(NativeUnsignedBufferCall {
                    site: site as u32,
                    source,
                    sublist,
                    owned_source_slots,
                })
            })
            .collect()
    }
}

fn can_retire(facts: &Facts<'_>, value: Value, frontier: u32) -> bool {
    // A completed return/seed/latch is not a future read of the old visit.
    // Preserve exact control order and all opaque ownership frontiers, sharing
    // the same mechanical query as existing private-frame retirement.
    !facts
        .old_value_has_future_reads(value, frontier)
        .unwrap_or(true)
}
