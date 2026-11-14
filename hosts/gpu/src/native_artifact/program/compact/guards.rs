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
use std::collections::VecDeque;
#[path = "guard_state.rs"]
mod state;
use state::Expression;
pub(super) use state::State;

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
                initial.assume(constant.value().get(), *t);
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
                edge.assume(value, false);
            }
            if next == yes {
                edge.assume(value, true);
            }
        }
        edge.close();
        edge
    }
    pub fn successors(&self, flow: &Flow, site: usize, state: &State) -> Vec<usize> {
        if let Some((value, no, yes)) = self.branches[site] {
            if let Some(truth) = state.truth(value) {
                return vec![if truth { yes } else { no }];
            }
        }
        flow.successors[site].clone()
    }
}
