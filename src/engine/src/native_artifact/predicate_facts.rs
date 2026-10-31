//! Mechanical Bool/copy facts shared by current-visit proof consumers.
//! These describe this immutable plan; they do not establish availability,
//! permit skipping a check, or carry a truth value across loop traversals.
use super::{
    Action, NativeProgramFacts, NativeProgramProducerRole as Role, Scalar, Term, Terminator, Value,
};
use crate::native_artifact::NativeScalarKind;
use mech_core::ValueData;

impl NativeProgramFacts<'_> {
    /// One actual identity Copy definition, excluding phi alternatives, loop
    /// carriers, inputs, and virtual producers. Availability is caller policy.
    pub fn unique_identity_copy(&self, value: Value) -> Option<Value> {
        let [definition] = self.definition_sites(value.get()).ok()? else {
            return None;
        };
        if definition.role() != Role::Scalar {
            return None;
        }
        match self
            .plan()
            .blocks()
            .get(definition.site()? as usize)?
            .action()
        {
            Action::Scalar(Scalar::Copy { source, output }) if *output == value => Some(*source),
            _ => None,
        }
    }

    /// (false successor, true successor) for this exact Bool operand. Literal
    /// equality preserves its polarity and accepts the constant on either side.
    /// No relation is inferred between distinct computed predicates.
    pub fn boolean_branch_edges(&self, site: u32, predicate: Value) -> Option<(u32, u32)> {
        let plan = self.plan();
        if plan.values().get(predicate.get() as usize)?.scalar_kind()
            != Some(NativeScalarKind::Bool)
        {
            return None;
        }
        let literal = |value| {
            plan.constants().iter().find_map(|constant| {
                if constant.value() == value
                    && let ValueData::Bool(value) = constant.constant().data()
                {
                    Some(*value)
                } else {
                    None
                }
            })
        };
        let (yes, no, truth) = match plan.blocks().get(site as usize)?.terminator() {
            Terminator::Scalar(Term::Branch { condition, yes, no }) if condition == predicate => {
                (yes, no, true)
            }
            Terminator::Scalar(Term::LiteralBranch {
                left,
                right,
                yes,
                no,
            }) => {
                let truth = if left == predicate {
                    literal(right)?
                } else if right == predicate {
                    literal(left)?
                } else {
                    return None;
                };
                (yes, no, truth)
            }
            _ => return None,
        };
        // Excluding one arm must never exclude the same successor's other arm.
        (yes != no).then_some(if truth { (no, yes) } else { (yes, no) })
    }
}
