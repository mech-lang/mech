//! Same-plan last-use candidates for selected packed buffer operations.
//! Runtime exclusivity and the original current-visit checks remain mandatory.
use super::fusion_semantics::Read;
use super::{
    NativeCollectionPrimitive as Primitive, NativeIterationAction as Action,
    NativeMatchAction as Scalar, NativeMatchValue as Value, NativeProgramArtifactPlan as Plan,
    NativeProgramFacts as Facts,
};

#[derive(Clone, Debug)]
pub struct NativeUnsignedBufferCall {
    site: u32,
    source: Option<Value>,
    owned_source_slots: Box<[Value]>,
}
impl NativeUnsignedBufferCall {
    pub fn site(&self) -> u32 {
        self.site
    }
    pub fn source(&self) -> Option<Value> {
        self.source
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
            if matches!(template.primitive(), Primitive::SequenceWithCapacity | Primitive::SequenceReserve | Primitive::SequenceAppend | Primitive::SequenceAppendLeBytes | Primitive::SequenceClear))) {
            return Vec::new();
        }
        let Ok(actions) = facts.actions() else {
            return vec![None; plan.blocks().len()];
        };
        let Ok(reads) = facts.fusion_reads() else {
            return vec![None; plan.blocks().len()];
        };
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
                    Primitive::SequenceReserve
                    | Primitive::SequenceAppend
                    | Primitive::SequenceAppendLeBytes
                    | Primitive::SequenceClear => Some(*operands.first()?),
                    _ => return None,
                };
                let mut eligible = vec![false; plan.values().len()];
                if let Some(source) = source {
                    if can_retire(facts, reads, source, site as u32) {
                        eligible[source.get() as usize] = true;
                    }
                }
                loop {
                    let mut changed = false;
                    for action in actions.iter() {
                        if let Action::Scalar(Scalar::Copy { source, output }) = action {
                            if eligible[output.get() as usize]
                                && !eligible[source.get() as usize]
                                && can_retire(facts, reads, *source, site as u32)
                            {
                                eligible[source.get() as usize] = true;
                                changed = true;
                            }
                        }
                    }
                    if !changed {
                        break;
                    }
                }
                let owned_source_slots = eligible
                    .into_iter()
                    .enumerate()
                    .filter_map(|(slot, eligible)| eligible.then(|| Value(slot as u32)))
                    .collect();
                Some(NativeUnsignedBufferCall {
                    site: site as u32,
                    source,
                    owned_source_slots,
                })
            })
            .collect()
    }
}

fn can_retire(facts: &Facts<'_>, reads: &[Vec<Read>], value: Value, frontier: u32) -> bool {
    let Some(reads) = reads.get(value.get() as usize) else {
        return false;
    };
    // Collection capture, loop seed/next/return, publication and effects have
    // independent ownership frontiers. Stay immutable unless those consumers
    // establish a dedicated proof; never silently discard such a read.
    if reads.iter().any(|read| {
        matches!(read, Read::Other) || matches!(read, Read::Terminator(site) if *site == frontier)
    }) {
        return false;
    }
    let Ok(definitions) = facts.definition_sites(value.get()) else {
        return false;
    };
    let mut seen = vec![false; facts.plan().blocks().len()];
    let Ok(next) = facts.successors(frontier) else {
        return false;
    };
    let mut pending = next.into_iter().flatten().collect::<Vec<_>>();
    while let Some(site) = pending.pop() {
        let Some(seen) = seen.get_mut(site as usize) else {
            return false;
        };
        if core::mem::replace(seen, true) {
            continue;
        }
        // An action reads its operands before replacing its result. A later
        // definition then kills this OLD owner before any terminator/future
        // read, including fresh visits reached through loop back edges.
        if reads.contains(&Read::Action(site)) {
            return false;
        }
        if definitions
            .iter()
            .any(|definition| definition.site() == Some(site))
        {
            continue;
        }
        if reads.contains(&Read::Terminator(site)) {
            return false;
        }
        let Ok(next) = facts.successors(site) else {
            return false;
        };
        pending.extend(next.into_iter().flatten());
    }
    true
}
