//! Current Boolean relations for native ownership proofs. A relation expires
//! whenever its output or any input home is defined again. Truths already
//! copied into independent Boolean homes keep their own current-value lifetime.
use super::flow::Flow;
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeIterationTerminator as Term,
    NativeMatchAction as Scalar, NativeMatchTerminator as Branch,
    NativeProgramArtifactPlan as Plan, NativeScalarKind, NativeScalarOp as Op,
    NativeStructuralObservation as Observation, NativeStructuralStep as Step,
};
use std::collections::{BTreeMap, VecDeque};
use std::rc::Rc;

#[derive(Clone, Debug, Eq, PartialEq)]
enum Expression {
    Copy(u32),
    Not(u32),
    And(u32, u32),
    Or(u32, u32),
    Project { source: u32, path: Vec<(u8, u32)> },
}
impl Expression {
    fn reads(&self, value: u32) -> bool {
        match self {
            Self::Copy(a) | Self::Not(a) => *a == value,
            Self::And(a, b) | Self::Or(a, b) => *a == value || *b == value,
            Self::Project { source, .. } => *source == value,
        }
    }
}
#[derive(Clone, Default, Debug, Eq, PartialEq)]
pub(super) struct State {
    known: Rc<BTreeMap<u32, bool>>,
    expressions: Rc<BTreeMap<u32, Expression>>,
    dirty: bool,
}
impl State {
    pub fn is_empty(&self) -> bool {
        self.known.is_empty()
    }
    pub fn merge(&mut self, other: &Self) -> bool {
        fn intersect<T: Clone + PartialEq>(
            local: &mut Rc<BTreeMap<u32, T>>,
            incoming: &Rc<BTreeMap<u32, T>>,
        ) -> bool {
            if Rc::ptr_eq(local, incoming)
                || local
                    .iter()
                    .all(|(key, value)| incoming.get(key) == Some(value))
            {
                return false;
            }
            Rc::make_mut(local).retain(|key, value| incoming.get(key) == Some(value));
            true
        }
        let changed = intersect(&mut self.known, &other.known)
            | intersect(&mut self.expressions, &other.expressions);
        self.dirty |= changed;
        changed
    }
    fn record(&mut self, value: u32, truth: bool) {
        if self.known.get(&value) != Some(&truth) {
            Rc::make_mut(&mut self.known).insert(value, truth);
            self.dirty = true;
        }
    }
    fn close(&mut self) {
        if !self.dirty {
            return;
        }
        loop {
            let mut inferred = Vec::new();
            for (&out, e) in self.expressions.iter() {
                let truth = self.known.get(&out).copied();
                match *e {
                    Expression::Copy(a) | Expression::Not(a) => {
                        let invert = matches!(e, Expression::Not(_));
                        if let Some(t) = truth {
                            inferred.push((a, t ^ invert));
                        }
                        if let Some(&t) = self.known.get(&a) {
                            inferred.push((out, t ^ invert));
                        }
                    }
                    Expression::And(a, b) | Expression::Or(a, b) => {
                        let conjunction = matches!(e, Expression::And(..));
                        let left = self.known.get(&a).copied();
                        let right = self.known.get(&b).copied();
                        if truth == Some(conjunction) {
                            inferred.extend([(a, conjunction), (b, conjunction)]);
                        } else if truth == Some(!conjunction) {
                            if left == Some(conjunction) {
                                inferred.push((b, !conjunction));
                            }
                            if right == Some(conjunction) {
                                inferred.push((a, !conjunction));
                            }
                        }
                        if left == Some(!conjunction) || right == Some(!conjunction) {
                            inferred.push((out, !conjunction));
                        } else if left == Some(conjunction) && right == Some(conjunction) {
                            inferred.push((out, conjunction));
                        }
                    }
                    Expression::Project { .. } => {
                        if let Some(t) = truth {
                            for (&peer, expression) in self.expressions.iter() {
                                if expression == e {
                                    inferred.push((peer, t));
                                }
                            }
                        }
                    }
                }
            }
            let before = self.known.len();
            for (value, truth) in inferred {
                if !self.known.contains_key(&value) {
                    Rc::make_mut(&mut self.known).insert(value, truth);
                }
            }
            if self.known.len() == before {
                self.dirty = false;
                break;
            }
        }
    }
    fn define(&mut self, defs: &[u32], expression: Option<&(u32, Expression)>) {
        for value in defs {
            if self.known.contains_key(value) {
                Rc::make_mut(&mut self.known).remove(value);
                self.dirty = true;
            }
        }
        let stale =
            |out: &u32, e: &Expression| defs.contains(out) || defs.iter().any(|&v| e.reads(v));
        if !defs.is_empty() && self.expressions.iter().any(|(out, e)| stale(out, e)) {
            Rc::make_mut(&mut self.expressions).retain(|out, e| !stale(out, e));
            self.dirty = true;
        }
        if let Some((output, e)) = expression {
            // An in-place definition reads the preceding version. It cannot
            // establish an equation about the new input home without a phi.
            if !defs.iter().any(|&v| e.reads(v)) && self.expressions.get(output) != Some(e) {
                Rc::make_mut(&mut self.expressions).insert(*output, e.clone());
                self.dirty = true;
            }
        }
        self.close();
    }
}

pub(super) struct Guards {
    known: Vec<State>,
    branches: Vec<Option<(u32, usize, usize)>>,
    expressions: Vec<Option<(u32, Expression)>>,
}
impl Guards {
    pub fn derive(plan: &Plan, flow: &Flow) -> Self {
        let facts = plan.facts();
        let branches = plan
            .blocks()
            .iter()
            .enumerate()
            .map(|(site, block)| {
                let operands = match block.terminator() {
                    Term::Scalar(Branch::Branch { condition, .. }) => vec![condition],
                    Term::Scalar(Branch::LiteralBranch { left, right, .. }) => vec![left, right],
                    _ => vec![],
                };
                operands
                    .into_iter()
                    .filter(|v| !plan.constants().iter().any(|c| c.value() == *v))
                    .find_map(|v| {
                        let (no, yes) = facts.boolean_branch_edges(site as u32, v)?;
                        Some((v.get(), no as usize, yes as usize))
                    })
            })
            .collect();
        let boolean = |v: mech_engine::native_artifact::NativeMatchValue| {
            plan.values()[v.get() as usize].scalar_kind() == Some(NativeScalarKind::Bool)
        };
        let expressions = plan
            .blocks()
            .iter()
            .map(|block| {
                Some(match block.action() {
                    Action::Scalar(Scalar::Copy { source, output }) if boolean(*output) => {
                        (output.get(), Expression::Copy(source.get()))
                    }
                    Action::Scalar(Scalar::Primitive {
                        op,
                        operands,
                        output,
                    }) if boolean(*output) => {
                        let expression = match (*op, operands.as_ref()) {
                            (Op::BoolNot, [a]) => Expression::Not(a.get()),
                            (Op::BoolAnd, [a, b]) => Expression::And(a.get(), b.get()),
                            (Op::BoolOr, [a, b]) => Expression::Or(a.get(), b.get()),
                            _ => return None,
                        };
                        (output.get(), expression)
                    }
                    Action::Structural {
                        source,
                        path,
                        observation: Observation::U64 | Observation::CanonicalValue,
                        output,
                    } if boolean(*output) => (
                        output.get(),
                        Expression::Project {
                            source: source.get(),
                            path: path
                                .iter()
                                .map(|step| match *step {
                                    Step::TupleItem { ordinal } => (0, ordinal),
                                    Step::EnumPayload { ordinal } => (1, ordinal),
                                })
                                .collect(),
                        },
                    ),
                    _ => return None,
                })
            })
            .collect();
        let mut result = Self {
            known: vec![State::default(); plan.blocks().len()],
            branches,
            expressions,
        };
        let mut initial = State::default();
        for constant in plan.constants() {
            if let mech_core::ValueData::Bool(t) = constant.constant().data() {
                initial.record(constant.value().get(), *t);
            }
        }
        let mut states = vec![None::<State>; plan.blocks().len()];
        states[flow.entry] = Some(initial);
        let mut queued = vec![false; states.len()];
        let mut pending = VecDeque::from([flow.entry]);
        queued[flow.entry] = true;
        while let Some(site) = pending.pop_front() {
            queued[site] = false;
            let mut after = states[site].as_ref().unwrap().clone();
            result.advance(flow, site, &mut after);
            for &next in &flow.successors[site] {
                let edge = result.edge(site, next, &after);
                let changed = if let Some(existing) = &mut states[next] {
                    existing.merge(&edge)
                } else {
                    states[next] = Some(edge);
                    true
                };
                if changed && !queued[next] {
                    queued[next] = true;
                    pending.push_back(next);
                }
            }
        }
        result.known = states.into_iter().map(Option::unwrap_or_default).collect();
        result
    }
    pub fn known(&self, site: usize) -> &State {
        &self.known[site]
    }
    pub fn advance(&self, flow: &Flow, site: usize, state: &mut State) {
        state.define(&flow.defs[site], self.expressions[site].as_ref());
    }
    pub fn edge(&self, site: usize, next: usize, state: &State) -> State {
        let mut edge = state.clone();
        if let Some((value, no, yes)) = self.branches[site] {
            if next == no {
                edge.record(value, false);
            }
            if next == yes {
                edge.record(value, true);
            }
        }
        edge.close();
        edge
    }
    pub fn successors(&self, flow: &Flow, site: usize, state: &State) -> Vec<usize> {
        if let Some((value, no, yes)) = self.branches[site] {
            if let Some(&truth) = state.known.get(&value) {
                return vec![if truth { yes } else { no }];
            }
        }
        flow.successors[site].clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn projection(source: u32, ordinal: u32) -> Expression {
        Expression::Project {
            source,
            path: vec![(0, ordinal)],
        }
    }
    #[test]
    fn conjunction_truth_reaches_repeated_current_tuple_projections() {
        let mut s = State::default();
        s.define(&[2], Some(&(2, projection(1, 1))));
        s.define(&[4], Some(&(4, Expression::And(2, 3))));
        s.record(4, true);
        s.close();
        assert_eq!(s.known.get(&2), Some(&true));
        assert_eq!(s.known.get(&3), Some(&true));
        s.define(&[5], Some(&(5, projection(1, 1))));
        assert_eq!(s.known.get(&5), Some(&true));
        s.define(&[6], Some(&(6, projection(1, 0))));
        assert!(!s.known.contains_key(&6));
    }
    #[test]
    fn redefining_a_tuple_keeps_old_boolean_truth_but_expires_its_relation() {
        let mut s = State::default();
        s.define(&[2], Some(&(2, projection(1, 1))));
        s.record(2, true);
        s.close();
        s.define(&[1], None);
        s.define(&[3], Some(&(3, projection(1, 1))));
        assert_eq!(s.known.get(&2), Some(&true));
        assert!(!s.known.contains_key(&3));
    }
    #[test]
    fn redefined_copy_operands_and_outputs_do_not_share_old_versions() {
        let mut s = State::default();
        s.define(&[2], Some(&(2, Expression::Copy(1))));
        s.define(&[1], None);
        s.record(2, true);
        s.close();
        assert!(!s.known.contains_key(&1));
        s.define(&[2], None);
        assert!(!s.known.contains_key(&2));
        s.define(&[1], Some(&(1, Expression::Not(1))));
        s.record(1, true);
        s.close();
        assert!(!s.expressions.contains_key(&1));
    }
    #[test]
    fn false_and_true_or_preserve_both_operand_possibilities() {
        for (e, t) in [(Expression::And(1, 2), false), (Expression::Or(1, 2), true)] {
            let mut s = State::default();
            s.define(&[3], Some(&(3, e)));
            s.record(3, t);
            s.close();
            assert!(!s.known.contains_key(&1));
            assert!(!s.known.contains_key(&2));
            s.record(1, !t);
            s.close();
            assert_eq!(s.known.get(&2), Some(&t));
        }
    }
    #[test]
    fn false_or_true_and_and_negated_copies_follow_boolean_semantics() {
        for (e, t) in [(Expression::And(1, 2), true), (Expression::Or(1, 2), false)] {
            let mut s = State::default();
            s.define(&[3], Some(&(3, e)));
            s.define(&[4], Some(&(4, Expression::Not(3))));
            s.define(&[5], Some(&(5, Expression::Copy(4))));
            s.record(5, !t);
            s.close();
            assert_eq!(s.known.get(&1), Some(&t));
            assert_eq!(s.known.get(&2), Some(&t));
        }
    }
    #[test]
    fn joins_keep_only_relations_true_on_every_incoming_path() {
        let mut left = State::default();
        left.define(&[2], Some(&(2, projection(1, 1))));
        left.record(2, true);
        let mut right = left.clone();
        right.define(&[1], None);
        assert!(left.merge(&right));
        left.define(&[3], Some(&(3, projection(1, 1))));
        assert!(!left.known.contains_key(&3));
        right.record(2, false);
        assert!(left.merge(&right));
        assert!(!left.known.contains_key(&2));
    }

    #[test]
    fn sibling_paths_close_with_their_own_truths_and_rejoin_on_common_facts() {
        let mut entry = State::default();
        entry.record(4, true);
        entry.define(&[2], Some(&(2, projection(1, 1))));
        entry.define(&[3], Some(&(3, Expression::Not(2))));

        let mut left = entry.clone();
        let mut right = entry.clone();
        left.record(3, true);
        left.close();
        right.record(3, false);
        right.close();
        assert_eq!(left.known.get(&2), Some(&false));
        assert_eq!(right.known.get(&2), Some(&true));
        assert_eq!(entry.known.get(&2), None);
        assert_eq!(entry.known.get(&3), None);

        assert!(!left.merge(&left.clone()));
        assert!(left.merge(&right));
        left.define(&[], None);
        assert_eq!(left.known.get(&2), None);
        assert_eq!(left.known.get(&3), None);
        assert_eq!(left.known.get(&4), Some(&true));

        left.define(&[1], None);
        left.define(&[5], Some(&(5, projection(1, 1))));
        left.record(3, true);
        left.close();
        assert_eq!(left.known.get(&2), Some(&false));
        assert_eq!(left.known.get(&5), None);
        assert_eq!(right.known.get(&2), Some(&true));
    }
}
