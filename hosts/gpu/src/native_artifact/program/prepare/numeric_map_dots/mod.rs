//! Pure source reconstruction only. Existing numeric-map loans retain all
//! current owners, geometry, resource admission and publication authority.
use super::*;
use mech_engine::native_artifact::NativeScalarOp;
mod affine;
pub(in crate::native_artifact) use affine::{Expr, Index, Operand};
mod cells;
pub(in crate::native_artifact) use cells::{CellPath, CellProgram, CellValue};
mod batch;
pub(in crate::native_artifact) use batch::{Batch, Predicate};

pub(super) fn constant(plan: &Plan, value: u32) -> Option<u64> {
    plan.constants()
        .iter()
        .find(|c| c.value().get() == value)
        .and_then(|c| match c.constant().data() {
            mech_core::ValueData::U64(n) => Some(*n),
            _ => None,
        })
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) struct Dot {
    pub operands: [Operand; 2],
}
impl Dot {
    pub fn derive(plan: &Plan, region: &Region, outer: Option<u32>) -> Option<Self> {
        if !region.range
            || region.rectangle.is_some()
            || region.reduction.is_none()
            || !region.nested.is_empty()
            || region.body.len() > 256
        {
            return None;
        }
        let latch = plan
            .iterations()
            .get(region.iteration as usize)?
            .as_ref()
            .latch();
        for (index, &site) in region.body.iter().enumerate() {
            let expected = region.body.get(index + 1).copied().unwrap_or(latch);
            if plan.blocks().get(site as usize)?.terminator()
                != Terminator::Scalar(Term::Jump(expected))
            {
                return None;
            }
        }
        let mut builder = affine::Builder::new(plan, region, outer)?;
        let operands = builder.product(region.result, 0)?;
        let mut pushed = false;
        // Do not drop a checked operation merely because the result is unused.
        // Every nontrivial original action must belong to this exact closure.
        for &site in &region.body {
            match plan.blocks()[site as usize].action() {
                Action::Scalar(Scalar::None) | Action::CollectionElement { .. } => {}
                Action::CollectionPush { collection, value }
                    if *collection == region.collection
                        && value.get() == region.result
                        && !pushed =>
                {
                    pushed = true;
                }
                Action::Scalar(Scalar::Copy { .. } | Scalar::Primitive { .. })
                | Action::LibraryCall { .. }
                    if builder.used.contains(&site) => {}
                _ => return None,
            }
        }
        pushed.then_some(Self { operands })
    }
}
#[cfg(test)]
mod tests;
