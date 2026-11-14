//! Finite boundary work from this phase's admitted source word inventory.
//! The sixteen instructions per word cover entry, repeated cold synchronization,
//! and exit templates; the emitter independently checks the concrete instruction
//! count. This does not reserve storage for every value in the whole program.
use super::*;
use mech_engine::native_artifact::{
    NativeAggregateWord as Word, NativeScalarAggregateAction as Aggregate,
};
pub(in crate::native_artifact) fn boundary_work(
    plan: &Plan,
    aggregates: &NativeScalarAggregates,
    forest: &Forest,
    private_words: usize,
) -> Result<u64, &'static str> {
    let mut scalar = BTreeSet::new();
    let mut packed = BTreeSet::new();
    let mut remember = |value: mech_engine::native_artifact::NativeMatchValue| {
        if plan.values()[value.get() as usize].scalar_kind().is_some() {
            scalar.insert(value.get());
        }
        if let Some((start, count)) = aggregates.slot(value) {
            packed.extend(start..start + count);
        }
    };
    for &site in &forest.sites {
        match plan.blocks()[site].action() {
            Action::Scalar(Scalar::Primitive {
                operands, output, ..
            })
            | Action::LibraryCall {
                operands, output, ..
            } => {
                for &value in operands.iter() {
                    remember(value);
                }
                remember(*output);
            }
            Action::Scalar(Scalar::Copy { source, output })
            | Action::Structural { source, output, .. } => {
                remember(*source);
                remember(*output);
            }
            _ => {}
        }
        if let Some(action) = aggregates.action(site) {
            match action {
                Aggregate::Pack { output, words } => {
                    remember(*output);
                    for word in words.iter() {
                        match word {
                            Word::Scalar(v) | Word::Field(v, _) => remember(*v),
                            _ => {}
                        }
                    }
                }
                Aggregate::Copy { source, output }
                | Aggregate::Project { source, output, .. }
                | Aggregate::Observe { source, output, .. } => {
                    remember(*source);
                    remember(*output);
                }
            }
        }
        match plan.blocks()[site].terminator() {
            Term::Scalar(Branch::Branch { condition, .. }) => remember(condition),
            Term::Scalar(Branch::LiteralBranch { left, right, .. }) => {
                remember(left);
                remember(right);
            }
            _ => {}
        }
    }
    let words = scalar
        .len()
        .checked_add(packed.len())
        .and_then(|n| n.checked_add(private_words))
        .and_then(|n| n.checked_add(3))
        .ok_or("phase boundary word inventory overflows")?;
    (words as u64)
        .checked_mul(16)
        .and_then(|n| n.checked_add(super::super::tail::FIXED_OVERHEAD))
        .ok_or("phase boundary work overflows")
}
