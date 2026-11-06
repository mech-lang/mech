//! Select a total unsigned clamp from the original pointwise scalar CFG.
//! Only the element varies; the subtraction threshold is an immutable capture.
use crate::native_artifact::codegen::control::plan::CodePlan;
use crate::native_artifact::program::prepare::word_collections::Region;
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeIterationTerminator as Term,
    NativeMatchAction as Scalar, NativeMatchTerminator as Branch, NativeScalarOp as Op,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Threshold {
    Constant(u64),
    Capture(u32),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Expression {
    Element,
    Uniform(Threshold),
    Difference(Threshold),
    Less(Threshold, bool),
}
#[derive(Clone, Default)]
struct State {
    values: BTreeMap<u32, Expression>,
    below: Option<bool>,
}

impl State {
    fn read(&self, value: mech_engine::native_artifact::NativeMatchValue) -> Option<Expression> {
        self.values.get(&value.get()).copied()
    }
    fn primitive(&mut self, op: Op, inputs: &[Expression]) -> Option<Expression> {
        use Expression as E;
        use Op::*;
        Some(match (op, inputs) {
            (
                U8ToU16 | U8ToU32 | U8ToU64 | U16ToU8 | U16ToU32 | U16ToU64 | U32ToU8 | U32ToU16
                | U32ToU64 | U64ToU8 | U64ToU16 | U64ToU32,
                [value],
            ) => *value,
            (U64Sub | U32Sub, [E::Element, E::Uniform(threshold)]) => E::Difference(*threshold),
            (U64Lt | U32Lt | U16Lt | U8Lt, [E::Element, E::Uniform(threshold)])
            | (U64Gt | U16Gt | U8Gt, [E::Uniform(threshold), E::Element]) => {
                E::Less(*threshold, true)
            }
            (U64Ge | U16Ge | U8Ge, [E::Element, E::Uniform(threshold)])
            | (U64Le | U32Le | U16Le | U8Le, [E::Uniform(threshold), E::Element]) => {
                E::Less(*threshold, false)
            }
            _ => return None,
        })
    }
}

fn threshold(selected: &mut Option<Threshold>, value: Threshold) -> Option<()> {
    if selected.is_some_and(|old| old != value) {
        return None;
    }
    *selected = Some(value);
    Some(())
}

/// This is an instruction-selection proof over an already reconstructed total
/// collection. Checked conversions preserve their numeric value on every
/// reached path; no failing operation or current admission is bypassed.
pub(super) fn select(code: CodePlan<'_>, region: &Region) -> Option<Threshold> {
    if region.input_width != region.output_width {
        return None;
    }
    let sites = region.sites.iter().copied().collect::<BTreeSet<_>>();
    let mut initial = State::default();
    for &capture in &region.captures {
        initial
            .values
            .insert(capture, Expression::Uniform(Threshold::Capture(capture)));
    }
    for i in 0..code.constant_count() {
        let (value, constant) = code.constant(i).ok()?;
        if initial.values.contains_key(&value.get())
            && let Some(constant) = constant
        {
            initial.values.insert(
                value.get(),
                Expression::Uniform(Threshold::Constant(constant)),
            );
        }
    }
    let mut pending = vec![(region.body, initial)];
    let mut selected = None;
    let mut completed = false;
    // One symbolic comparison partitions the element domain into two paths.
    // Revisiting a block in the same partition declines rather than assuming
    // an arbitrary merge or a cyclic execution is the same computation.
    let mut visited = BTreeSet::new();
    while let Some((site, mut state)) = pending.pop() {
        if !sites.contains(&site) || !visited.insert((site, state.below)) {
            return None;
        }
        let block = &code.blocks()[site as usize];
        match block.action() {
            Action::CollectionElement { .. } if site == region.body => {
                state.values.insert(region.element, Expression::Element);
            }
            Action::CollectionPush { value, .. } if site == region.push => {
                match (state.below, state.read(*value)?) {
                    (Some(true), Expression::Uniform(Threshold::Constant(0))) => {}
                    (Some(false), Expression::Difference(value)) => {
                        threshold(&mut selected, value)?;
                    }
                    _ => return None,
                }
                completed = true;
                continue;
            }
            Action::Scalar(Scalar::None) => {}
            Action::Scalar(Scalar::Copy { source, output }) => {
                state.values.insert(output.get(), state.read(*source)?);
            }
            Action::Scalar(Scalar::Primitive {
                op,
                operands,
                output,
            }) => {
                let inputs = operands
                    .iter()
                    .map(|&v| state.read(v))
                    .collect::<Option<Vec<_>>>()?;
                let value = state.primitive(*op, &inputs)?;
                if let Expression::Less(value, _) | Expression::Difference(value) = value {
                    threshold(&mut selected, value)?;
                }
                state.values.insert(output.get(), value);
            }
            _ => return None,
        }
        let (predicate, yes, no) = match block.terminator() {
            Term::Scalar(Branch::Jump(next)) => {
                pending.push((next, state));
                continue;
            }
            Term::Scalar(Branch::Branch { condition, yes, no }) => {
                (state.read(condition)?, yes, no)
            }
            Term::Scalar(Branch::LiteralBranch {
                left,
                right,
                yes,
                no,
            }) => {
                let (left, right) = (state.read(left)?, state.read(right)?);
                let (test, literal) = match (left, right) {
                    (test, Expression::Uniform(Threshold::Constant(value)))
                    | (Expression::Uniform(Threshold::Constant(value)), test)
                        if value <= 1 =>
                    {
                        (test, value != 0)
                    }
                    _ => return None,
                };
                if literal {
                    (test, yes, no)
                } else {
                    (test, no, yes)
                }
            }
            _ => return None,
        };
        let Expression::Less(value, positive) = predicate else {
            return None;
        };
        threshold(&mut selected, value)?;
        if let Some(below) = state.below {
            pending.push((if below == positive { yes } else { no }, state));
        } else {
            let mut other = state.clone();
            state.below = Some(positive);
            other.below = Some(!positive);
            pending.extend([(yes, state), (no, other)]);
        }
    }
    completed.then_some(selected).flatten()
}
