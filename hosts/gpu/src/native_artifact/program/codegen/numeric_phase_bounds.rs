//! Guard current phase geometry/length floors independently of backing pointers.
use super::{numeric_loop_windows::Version, traversal_facts::Proof as Facts};
use crate::native_artifact::codegen::control::plan::CodePlan;
use crate::native_artifact::program::prepare::readonly_numeric::Proof;
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeIterationTerminator as Term,
    NativeMatchAction as Scalar, NativeMatchTerminator as Branch, NativeMatchValue,
    NativeScalarOp as Op,
};
use std::collections::BTreeMap;

pub(super) fn derive(
    code: CodePlan<'_>,
    proof: &Proof,
    versions: &[Version],
    facts: &Facts,
    terminal_fallbacks: bool,
) -> Vec<(usize, u64)> {
    // Existing numeric membership fixes each view's identity; its only
    // length-changing action here is successful append. A private handoff is
    // permitted only when the exact caller contract proves it returns from
    // the phase before executing the unsupported operation. Re-entry checks
    // the new current view; no length or pointer fact crosses that return.
    if proof.actions.iter().any(|(_, operation)| {
        matches!(
            operation,
            crate::native_artifact::program::prepare::readonly_numeric::Operation::Fallback
        ) && !terminal_fallbacks
            || matches!(operation,
        crate::native_artifact::program::prepare::readonly_numeric::Operation::AppendLeBytes { .. })
    }) {
        return Vec::new();
    }
    let mut floors = BTreeMap::<usize, u64>::new();
    for version in versions {
        if version.reduction.is_none() || version.windows.is_empty() {
            continue;
        }
        let required = version
            .windows
            .iter()
            .map(|window| {
                let upper = *facts.query_upper.get(&(version.initialize, window.count))?;
                if upper > u32::MAX as u64 {
                    return None;
                }
                Some((window.array, upper.checked_add(window.adjustment())?))
            })
            .collect::<Option<Vec<_>>>();
        let Some(required) = required else {
            continue;
        };
        for (array, minimum) in required {
            floors
                .entry(array)
                .and_modify(|old| *old = (*old).max(minimum))
                .or_insert(minimum);
        }
    }
    if std::env::var_os("MECH_PHASE_ASSERTION_FLOORS").is_some() {
        assertion_floors(code, proof, &mut floors);
    }
    floors.into_iter().collect()
}

// These are guarded entry requirements, not source invariants: a short current
// view declines the native phase and resumes the original lexical checks.
// Membership above permits only identity-preserving updates and growing append.
fn assertion_floors(code: CodePlan<'_>, proof: &Proof, floors: &mut BTreeMap<usize, u64>) {
    let CodePlan::Program(program) = code else {
        return;
    };
    let facts = program.plan().facts();
    let constants = (0..code.constant_count())
        .filter_map(|i| {
            code.constant(i)
                .ok()
                .and_then(|(v, bits)| bits.map(|n| (v.get(), n)))
        })
        .collect::<BTreeMap<_, _>>();
    let length = |value: NativeMatchValue| {
        let [definition] = facts.definition_sites(value.get()).ok()? else {
            return None;
        };
        match proof.action(definition.site()? as usize)? {
            super::super::prepare::readonly_numeric::Operation::Length { array, output }
                if *output == value.get() =>
            {
                Some(*array)
            }
            _ => None,
        }
    };
    for &site in &proof.sites {
        let Term::Scalar(Branch::Branch { condition, no, .. }) = code.blocks()[site].terminator()
        else {
            continue;
        };
        // Do not look through effectful blocks or general control flow. The
        // canonical assertion rejection block is empty and fails integrity.
        let Some(rejected) = code.blocks().get(no as usize) else {
            continue;
        };
        if !matches!(rejected.action(), Action::Scalar(Scalar::None))
            || !matches!(
                rejected.terminator(),
                Term::Scalar(Branch::IntegrityFailure)
            )
        {
            continue;
        }
        let Ok([definition]) = facts.definition_sites(condition.get()) else {
            continue;
        };
        let Some(at) = definition.site() else {
            continue;
        };
        let Action::Scalar(Scalar::Primitive {
            op,
            operands,
            output,
            ..
        }) = code.blocks()[at as usize].action()
        else {
            continue;
        };
        if *output != condition {
            continue;
        }
        let [left, right] = operands.as_ref() else {
            continue;
        };
        let required = length(*left)
            .zip(constants.get(&right.get()).copied())
            .and_then(|(array, n)| minimum(*op, false, n).map(|minimum| (array, minimum)))
            .or_else(|| {
                length(*right)
                    .zip(constants.get(&left.get()).copied())
                    .and_then(|(array, n)| minimum(*op, true, n).map(|minimum| (array, minimum)))
            });
        if let Some((array, minimum)) = required {
            floors
                .entry(array)
                .and_modify(|old| *old = (*old).max(minimum))
                .or_insert(minimum);
        }
    }
}

fn minimum(op: Op, reversed: bool, literal: u64) -> Option<u64> {
    match (op, reversed) {
        (Op::U64Ge, false) | (Op::U64Le, true) | (Op::U64Eq, _) => Some(literal),
        (Op::U64Gt, false) | (Op::U64Lt, true) => literal.checked_add(1),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn static_index_ranges_keep_wrap_zero_and_upper_edges_conservative() {
        assert_eq!(
            scalar_range(Op::U64WrappingAdd, &[(0, 15), (1, 1)]),
            Some((1, 16))
        );
        assert_eq!(
            scalar_range(Op::U64WrappingAdd, &[(0, u64::MAX), (1, 1)]),
            None
        );
        assert_eq!(scalar_range(Op::U64WrappingSub, &[(0, 15), (1, 1)]), None);
        assert_eq!(
            scalar_range(Op::U64WrappingSub, &[(1, 16), (1, 1)]),
            Some((0, 15))
        );
        assert_eq!(
            scalar_range(Op::U64WrappingMul, &[(0, 65535), (2, 2)]),
            Some((0, 131070))
        );
        assert_eq!(
            scalar_range(Op::U64WrappingMul, &[(0, u64::MAX), (2, 2)]),
            Some((0, u64::MAX))
        );
    }
    #[test]
    fn assertion_floor_requires_unsigned_lower_bound_and_checked_strictness() {
        for (op, reversed) in [
            (Op::U64Ge, false),
            (Op::U64Le, true),
            (Op::U64Eq, false),
            (Op::U64Eq, true),
        ] {
            assert_eq!(minimum(op, reversed, 16), Some(16));
            assert_eq!(minimum(op, reversed, u64::MAX), Some(u64::MAX));
        }
        for (op, reversed) in [(Op::U64Gt, false), (Op::U64Lt, true)] {
            assert_eq!(minimum(op, reversed, 15), Some(16));
            assert_eq!(minimum(op, reversed, u64::MAX), None);
        }
        for (op, reversed) in [
            (Op::U64Lt, false),
            (Op::U64Le, false),
            (Op::U64Ge, true),
            (Op::U64Gt, true),
            (Op::U64Ne, false),
        ] {
            assert_eq!(minimum(op, reversed, 16), None);
        }
    }
}

// Pure scalar ranges need no selected-path facts. A masked index has the same
// bounds on every visit, including continuations after a synchronous child.
// Follow only unique scalar definitions and literal constants. Ambiguous,
// structural, cyclic and opaque definitions remain the full unsigned range.
pub(super) struct Indices<'a> {
    code: CodePlan<'a>,
    constants: BTreeMap<u32, u64>,
    ranges: BTreeMap<u32, (u64, u64)>,
}
impl<'a> Indices<'a> {
    pub(super) fn new(code: CodePlan<'a>) -> Self {
        Self {
            code,
            ranges: BTreeMap::new(),
            constants: (0..code.constant_count())
                .filter_map(|i| {
                    code.constant(i)
                        .ok()
                        .and_then(|(v, n)| n.map(|n| (v.get(), n)))
                })
                .collect(),
        }
    }
    fn definition(&self, value: u32) -> Option<&'a Scalar> {
        let CodePlan::Program(program) = self.code else {
            return None;
        };
        let facts = program.plan().facts();
        let [definition] = facts.definition_sites(value).ok()? else {
            return None;
        };
        match self.code.blocks()[definition.site()? as usize].action() {
            Action::Scalar(action) => Some(action),
            _ => None,
        }
    }
    pub(super) fn proves(&mut self, index: u32, minimum: u64) -> bool {
        use std::collections::BTreeSet;
        let mut pending = vec![(index, false)];
        let mut active = BTreeSet::new();
        let mut cyclic = BTreeSet::new();
        while let Some((value, finish)) = pending.pop() {
            if !finish {
                if self.ranges.contains_key(&value) {
                    continue;
                }
                if let Some(&constant) = self.constants.get(&value) {
                    self.ranges.insert(value, (constant, constant));
                    continue;
                }
                if !active.insert(value) {
                    cyclic.extend(active.iter().copied());
                    self.ranges.insert(value, (0, u64::MAX));
                    continue;
                }
                pending.push((value, true));
                match self.definition(value) {
                    Some(Scalar::Copy { source, .. }) => pending.push((source.get(), false)),
                    Some(Scalar::Primitive { op, operands, .. }) if supported(*op) => {
                        pending.extend(operands.iter().rev().map(|v| (v.get(), false)));
                    }
                    _ => {}
                }
                continue;
            }
            if cyclic.contains(&value) {
                active.remove(&value);
                self.ranges.insert(value, (0, u64::MAX));
                continue;
            }
            let range = match self.definition(value) {
                Some(Scalar::Copy { source, .. }) => self.ranges.get(&source.get()).copied(),
                Some(Scalar::Primitive { op, operands, .. }) if supported(*op) => {
                    let args = operands
                        .iter()
                        .map(|v| self.ranges.get(&v.get()).copied())
                        .collect::<Option<Vec<_>>>();
                    args.and_then(|args| scalar_range(*op, &args))
                }
                _ => None,
            }
            .unwrap_or((0, u64::MAX));
            active.remove(&value);
            self.ranges.insert(value, range);
        }
        self.ranges
            .get(&index)
            .is_some_and(|&(low, high)| low >= 1 && high <= minimum)
    }
}
fn supported(op: Op) -> bool {
    matches!(
        op,
        Op::U64BitAnd | Op::U64WrappingAdd | Op::U64WrappingSub | Op::U64WrappingMul
    )
}
fn scalar_range(op: Op, args: &[(u64, u64)]) -> Option<(u64, u64)> {
    let &[a, b] = args else { return None };
    match op {
        Op::U64BitAnd => Some((0, a.1.min(b.1))),
        Op::U64WrappingAdd => Some((a.0.checked_add(b.0)?, a.1.checked_add(b.1)?)),
        Op::U64WrappingSub if a.0 >= b.1 => Some((a.0 - b.1, a.1 - b.0)),
        Op::U64WrappingMul => {
            use crate::native_artifact::program::prepare::word_collections::bounds::Range;
            Some((0, Range(a.0, a.1).wrapping_mul(Range(b.0, b.1)).1))
        }
        _ => None,
    }
}
