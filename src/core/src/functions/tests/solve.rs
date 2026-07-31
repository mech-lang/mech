#[cfg(feature = "compiler")]
use super::super::MechFunctionCompiler;
use super::super::{
    MechFunctionImpl, ReactiveNodeId, ReactiveNodeKind, ReactivePlan, ReactivePlanSolveOutcome,
    ReactiveRegisterCommitOutcome, ReactiveSolveStatus, ReactiveTurnOutcome, ReactiveTurnState,
};
use super::support::reg;
#[cfg(feature = "compiler")]
use crate::{BytecodeCompilerContext, Register};
use crate::{GenericError, MResult, MechError, Ref, ToValue, Value};
use std::{cell::RefCell, rc::Rc};

#[cfg(feature = "f64")]
struct SchedulerFunction {
    label: &'static str,
    output: Value,
    kind: ReactiveNodeKind,
    status: ReactiveSolveStatus,
    count: Rc<RefCell<usize>>,
    log: Rc<RefCell<Vec<&'static str>>>,
    error: bool,
}
#[cfg(feature = "f64")]
impl MechFunctionImpl for SchedulerFunction {
    fn solve(&self) {}
    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
        *self.count.borrow_mut() += 1;
        self.log.borrow_mut().push(self.label);
        if self.error {
            Err(MechError::new(
                GenericError {
                    msg: self.label.into(),
                },
                None,
            ))
        } else {
            Ok(self.status)
        }
    }
    fn out(&self) -> Value {
        self.output.clone()
    }
    fn reactive_node_kind(&self) -> ReactiveNodeKind {
        self.kind
    }
    fn to_string(&self) -> String {
        self.label.into()
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(self.reactive_output_values())
    }
}
#[cfg(all(feature = "f64", feature = "compiler"))]
impl MechFunctionCompiler for SchedulerFunction {
    fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

#[cfg(feature = "f64")]
fn scheduler_node(
    plan: &mut ReactivePlan,
    label: &'static str,
    inputs: &[Value],
    kind: ReactiveNodeKind,
    status: ReactiveSolveStatus,
    log: Rc<RefCell<Vec<&'static str>>>,
    error: bool,
) -> (ReactiveNodeId, Value, Rc<RefCell<usize>>) {
    let output = Value::F64(Ref::new(0.0));
    let count = Rc::new(RefCell::new(0));
    let function = SchedulerFunction {
        label,
        output: output.clone(),
        kind,
        status,
        count: count.clone(),
        log,
        error,
    };
    (
        plan.register(Box::new(function), inputs).unwrap(),
        output,
        count,
    )
}
#[cfg(feature = "f64")]
fn scheduler_source() -> Value {
    Value::F64(Ref::new(0.0))
}

struct Comb {
    source: Ref<f64>,
    sink: Ref<f64>,
    add: f64,
    count: Rc<RefCell<usize>>,
    fail: bool,
}
impl MechFunctionImpl for Comb {
    fn solve(&self) {}
    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
        *self.count.borrow_mut() += 1;
        if self.fail {
            return Err(MechError::new(
                GenericError {
                    msg: "solve failure".into(),
                },
                None,
            ));
        }
        *self.sink.borrow_mut() = *self.source.borrow() + self.add;
        Ok(ReactiveSolveStatus::Changed)
    }
    fn out(&self) -> Value {
        self.sink.to_value()
    }
    fn to_string(&self) -> String {
        "test combinational".into()
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(self.reactive_output_values())
    }
}
#[cfg(feature = "compiler")]
impl MechFunctionCompiler for Comb {
    fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}
fn comb(
    p: &mut ReactivePlan,
    source: Ref<f64>,
    sink: Ref<f64>,
    fail: bool,
) -> (ReactiveNodeId, Rc<RefCell<usize>>) {
    let count = Rc::new(RefCell::new(0));
    let node = p
        .register(
            Box::new(Comb {
                source: source.clone(),
                sink,
                add: 1.,
                count: count.clone(),
                fail,
            }),
            &[source.to_value()],
        )
        .unwrap();
    (node, count)
}
fn chain() -> (
    ReactivePlan,
    Ref<f64>,
    Ref<f64>,
    Ref<f64>,
    Ref<f64>,
    ReactiveNodeId,
    ReactiveNodeId,
    ReactiveNodeId,
    Rc<RefCell<usize>>,
    Rc<RefCell<usize>>,
) {
    let mut p = ReactivePlan::new();
    let input = Ref::new(1.);
    let a = Ref::new(1.);
    let middle = Ref::new(2.);
    let b = Ref::new(2.);
    let final_value = Ref::new(3.);
    let (ra, _, _, ca) = reg(&mut p, input.clone(), a.clone(), false);
    let (mid, _) = comb(&mut p, a.clone(), middle.clone(), false);
    let (rb, _, _, cb) = reg(&mut p, middle.clone(), b.clone(), false);
    let (final_node, _) = comb(&mut p, b.clone(), final_value.clone(), false);
    (p, input, a, middle, b, ra, rb, final_node, ca, cb)
}

#[cfg(feature = "f64")]
#[test]
fn reactive_dirty_scheduler_runs_linear_chain() {
    let mut p = ReactivePlan::new();
    let l = Rc::new(RefCell::new(vec![]));
    let d = scheduler_source();
    let (a, ao, _) = scheduler_node(
        &mut p,
        "A",
        &[d.clone()],
        ReactiveNodeKind::Combinational,
        ReactiveSolveStatus::Changed,
        l.clone(),
        false,
    );
    let (b, bo, _) = scheduler_node(
        &mut p,
        "B",
        &[ao],
        ReactiveNodeKind::Combinational,
        ReactiveSolveStatus::Changed,
        l.clone(),
        false,
    );
    let (c, _, _) = scheduler_node(
        &mut p,
        "C",
        &[bo],
        ReactiveNodeKind::Combinational,
        ReactiveSolveStatus::Changed,
        l.clone(),
        false,
    );
    let o = p.solve_dirty_cells(&d.reactive_root_cell_ids()).unwrap();
    assert_eq!(o.executed_nodes, vec![a, b, c]);
    assert_eq!(o.changed_nodes, vec![a, b, c]);
    assert!(o.unchanged_nodes.is_empty() && o.pending_register_nodes.is_empty());
    assert_eq!(*l.borrow(), vec!["A", "B", "C"]);
}

#[cfg(feature = "f64")]
#[test]
fn reactive_dirty_scheduler_orders_independent_branches_by_plan_index() {
    let mut p = ReactivePlan::new();
    let l = Rc::new(RefCell::new(vec![]));
    let x = scheduler_source();
    let y = scheduler_source();
    let (a, _, _) = scheduler_node(
        &mut p,
        "A",
        &[x.clone()],
        ReactiveNodeKind::Combinational,
        ReactiveSolveStatus::Changed,
        l.clone(),
        false,
    );
    let (b, _, _) = scheduler_node(
        &mut p,
        "B",
        &[y.clone()],
        ReactiveNodeKind::Combinational,
        ReactiveSolveStatus::Changed,
        l.clone(),
        false,
    );
    assert_eq!(
        p.solve_dirty_cells(&[y.reactive_root_cell_ids()[0], x.reactive_root_cell_ids()[0]])
            .unwrap()
            .executed_nodes,
        vec![a, b]
    );
}

#[cfg(feature = "f64")]
#[test]
fn reactive_dirty_scheduler_skips_unrelated_nodes() {
    let mut p = ReactivePlan::new();
    let l = Rc::new(RefCell::new(vec![]));
    let d = scheduler_source();
    let (_a, _, _) = scheduler_node(
        &mut p,
        "A",
        &[d.clone()],
        ReactiveNodeKind::Combinational,
        ReactiveSolveStatus::Changed,
        l.clone(),
        false,
    );
    let (u, _, uc) = scheduler_node(
        &mut p,
        "U",
        &[],
        ReactiveNodeKind::Combinational,
        ReactiveSolveStatus::Changed,
        l,
        false,
    );
    let o = p.solve_dirty_cells(&d.reactive_root_cell_ids()).unwrap();
    assert_eq!(*uc.borrow(), 0);
    assert!(!o.executed_nodes.contains(&u));
}

#[cfg(feature = "f64")]
#[test]
fn reactive_dirty_scheduler_deduplicates_dirty_cells() {
    let mut p = ReactivePlan::new();
    let l = Rc::new(RefCell::new(vec![]));
    let d = scheduler_source();
    let (_, _, c) = scheduler_node(
        &mut p,
        "A",
        &[d.clone()],
        ReactiveNodeKind::Combinational,
        ReactiveSolveStatus::Changed,
        l,
        false,
    );
    let cell = d.reactive_root_cell_ids()[0];
    p.solve_dirty_cells(&[cell, cell, cell]).unwrap();
    assert_eq!(*c.borrow(), 1);
}

#[cfg(feature = "f64")]
#[test]
fn reactive_dirty_scheduler_executes_fan_in_consumer_once() {
    let mut p = ReactivePlan::new();
    let l = Rc::new(RefCell::new(vec![]));
    let x = scheduler_source();
    let y = scheduler_source();
    let (_, lo, _) = scheduler_node(
        &mut p,
        "L",
        &[x.clone()],
        ReactiveNodeKind::Combinational,
        ReactiveSolveStatus::Changed,
        l.clone(),
        false,
    );
    let (_, ro, _) = scheduler_node(
        &mut p,
        "R",
        &[y.clone()],
        ReactiveNodeKind::Combinational,
        ReactiveSolveStatus::Changed,
        l.clone(),
        false,
    );
    let (_, _, c) = scheduler_node(
        &mut p,
        "J",
        &[lo, ro],
        ReactiveNodeKind::Combinational,
        ReactiveSolveStatus::Changed,
        l,
        false,
    );
    p.solve_dirty_cells(&[x.reactive_root_cell_ids()[0], y.reactive_root_cell_ids()[0]])
        .unwrap();
    assert_eq!(*c.borrow(), 1);
}

#[cfg(feature = "f64")]
#[test]
fn reactive_dirty_scheduler_propagates_changed_outputs() {
    reactive_dirty_scheduler_runs_linear_chain();
}

#[cfg(feature = "f64")]
#[test]
fn reactive_dirty_scheduler_stops_on_unchanged() {
    let mut p = ReactivePlan::new();
    let l = Rc::new(RefCell::new(vec![]));
    let d = scheduler_source();
    let (a, ao, ac) = scheduler_node(
        &mut p,
        "A",
        &[d.clone()],
        ReactiveNodeKind::Combinational,
        ReactiveSolveStatus::Unchanged,
        l.clone(),
        false,
    );
    let (b, _, bc) = scheduler_node(
        &mut p,
        "B",
        &[ao],
        ReactiveNodeKind::Combinational,
        ReactiveSolveStatus::Changed,
        l,
        false,
    );
    let o = p.solve_dirty_cells(&d.reactive_root_cell_ids()).unwrap();
    assert_eq!(*ac.borrow(), 1);
    assert_eq!(*bc.borrow(), 0);
    assert_eq!(o.unchanged_nodes, vec![a]);
    assert!(!o.executed_nodes.contains(&b));
}

#[cfg(feature = "f64")]
#[test]
fn reactive_dirty_scheduler_ignores_sampled_consumers() {
    let mut p = ReactivePlan::new();
    let l = Rc::new(RefCell::new(vec![]));
    let d = scheduler_source();
    let (n, _, c) = scheduler_node(
        &mut p,
        "R",
        &[],
        ReactiveNodeKind::Register,
        ReactiveSolveStatus::Changed,
        l,
        false,
    );
    p.sampled_consumers
        .entry(d.reactive_root_cell_ids()[0])
        .or_default()
        .push(n);
    let o = p.solve_dirty_cells(&d.reactive_root_cell_ids()).unwrap();
    assert_eq!(*c.borrow(), 0);
    assert!(!o.pending_register_nodes.contains(&n));
}

#[cfg(feature = "f64")]
#[test]
fn reactive_dirty_scheduler_reports_register_pending_without_execution() {
    let mut p = ReactivePlan::new();
    let l = Rc::new(RefCell::new(vec![]));
    let d = scheduler_source();
    let (r, _, c) = scheduler_node(
        &mut p,
        "R",
        &[d.clone()],
        ReactiveNodeKind::Register,
        ReactiveSolveStatus::Changed,
        l,
        false,
    );
    let o = p.solve_dirty_cells(&d.reactive_root_cell_ids()).unwrap();
    assert_eq!(o.pending_register_nodes, vec![r]);
    assert_eq!(*c.borrow(), 0);
    assert!(o.executed_nodes.is_empty());
}

#[cfg(feature = "f64")]
#[test]
fn reactive_dirty_scheduler_stops_at_register_boundary() {
    let mut p = ReactivePlan::new();
    let l = Rc::new(RefCell::new(vec![]));
    let d = scheduler_source();
    let (r, ro, rc) = scheduler_node(
        &mut p,
        "R",
        &[d.clone()],
        ReactiveNodeKind::Register,
        ReactiveSolveStatus::Changed,
        l.clone(),
        false,
    );
    let (_, _, dc) = scheduler_node(
        &mut p,
        "D",
        &[ro],
        ReactiveNodeKind::Combinational,
        ReactiveSolveStatus::Changed,
        l,
        false,
    );
    let o = p.solve_dirty_cells(&d.reactive_root_cell_ids()).unwrap();
    assert_eq!(o.pending_register_nodes, vec![r]);
    assert_eq!(*rc.borrow(), 0);
    assert_eq!(*dc.borrow(), 0);
}

#[cfg(feature = "f64")]
#[test]
fn reactive_dirty_scheduler_dirty_register_output_runs_downstream_only() {
    let mut p = ReactivePlan::new();
    let l = Rc::new(RefCell::new(vec![]));
    let d = scheduler_source();
    let (r, ro, rc) = scheduler_node(
        &mut p,
        "R",
        &[d.clone()],
        ReactiveNodeKind::Register,
        ReactiveSolveStatus::Changed,
        l.clone(),
        false,
    );
    let (_, _, dc) = scheduler_node(
        &mut p,
        "D",
        &[ro.clone()],
        ReactiveNodeKind::Combinational,
        ReactiveSolveStatus::Changed,
        l,
        false,
    );
    let cell = ro.reactive_root_cell_ids()[0];
    let o = p.solve_dirty_cells(&[cell]).unwrap();
    assert!(!o.pending_register_nodes.contains(&r));
    assert_eq!(*rc.borrow(), 0);
    assert_eq!(*dc.borrow(), 1);
}

#[cfg(feature = "f64")]
#[test]
fn reactive_dirty_scheduler_stops_on_error() {
    let mut p = ReactivePlan::new();
    let l = Rc::new(RefCell::new(vec![]));
    let d = scheduler_source();
    let (_, ao, ac) = scheduler_node(
        &mut p,
        "A",
        &[d.clone()],
        ReactiveNodeKind::Combinational,
        ReactiveSolveStatus::Changed,
        l.clone(),
        true,
    );
    let (_, _, bc) = scheduler_node(
        &mut p,
        "B",
        &[ao],
        ReactiveNodeKind::Combinational,
        ReactiveSolveStatus::Changed,
        l,
        false,
    );
    let e = p
        .solve_dirty_cells(&d.reactive_root_cell_ids())
        .unwrap_err();
    assert!(e.kind_message().contains("A"));
    assert_eq!(*ac.borrow(), 1);
    assert_eq!(*bc.borrow(), 0);
}

#[cfg(feature = "f64")]
#[test]
fn reactive_dirty_scheduler_empty_dirty_set_is_noop() {
    let mut p = ReactivePlan::new();
    let l = Rc::new(RefCell::new(vec![]));
    let d = scheduler_source();
    let (_, _, c) = scheduler_node(
        &mut p,
        "A",
        &[d],
        ReactiveNodeKind::Combinational,
        ReactiveSolveStatus::Changed,
        l,
        false,
    );
    assert_eq!(
        p.solve_dirty_cells(&[]).unwrap(),
        ReactivePlanSolveOutcome::default()
    );
    assert_eq!(*c.borrow(), 0);
}

#[test]
fn reactive_turn_propagates_register_outputs_after_commit() {
    let mut p = ReactivePlan::new();
    let input = Ref::new(1.);
    let a = Ref::new(1.);
    let out = Ref::new(2.);
    let (r, solve, stage, commit) = reg(&mut p, input.clone(), a.clone(), false);
    let (d, down) = comb(&mut p, a.clone(), out.clone(), false);
    *input.borrow_mut() = 10.;
    let mut s = ReactiveTurnState::default();
    let o = p
        .advance_reactive_turn(&mut s, &input.to_value().reactive_root_cell_ids())
        .unwrap();
    assert_eq!(o.before_commit.pending_register_nodes, vec![r]);
    assert_eq!(o.register_commit.staged_nodes, vec![r]);
    assert_eq!(o.register_commit.committed_nodes, vec![r]);
    assert!(o.after_commit.executed_nodes.contains(&d));
    assert_eq!(
        (
            *a.borrow(),
            *out.borrow(),
            *solve.borrow(),
            *stage.borrow(),
            *commit.borrow(),
            *down.borrow()
        ),
        (10., 11., 0, 1, 1, 1)
    );
    assert!(s.pending_register_nodes.is_empty());
}

#[test]
fn reactive_turn_defers_post_commit_registers_until_next_turn() {
    let (mut p, input, a, middle, b, ra, rb, _, ca, cb) = chain();
    let final_value = p
        .nodes
        .last()
        .unwrap()
        .function
        .out()
        .as_f64()
        .unwrap()
        .clone();
    *input.borrow_mut() = 10.;
    let mut s = ReactiveTurnState::default();
    let first = p
        .advance_reactive_turn(&mut s, &input.to_value().reactive_root_cell_ids())
        .unwrap();
    assert_eq!(first.register_commit.committed_nodes, vec![ra]);
    assert_eq!(first.after_commit.pending_register_nodes, vec![rb]);
    assert_eq!(s.pending_register_nodes, vec![rb]);
    assert_eq!(
        (
            *a.borrow(),
            *middle.borrow(),
            *b.borrow(),
            *final_value.borrow(),
            *cb.borrow()
        ),
        (10., 11., 2., 3., 0)
    );
    let second = p.advance_reactive_turn(&mut s, &[]).unwrap();
    assert_eq!(second.register_commit.committed_nodes, vec![rb]);
    assert_eq!((*ca.borrow(), *cb.borrow()), (1, 1));
    assert!(!s.has_pending_registers());
}

#[test]
fn reactive_turn_commits_each_register_layer_at_most_once() {
    let (mut p, input, _, _, _, ra, rb, _, ca, cb) = chain();
    *input.borrow_mut() = 10.;
    let mut s = ReactiveTurnState::default();
    p.advance_reactive_turn(&mut s, &input.to_value().reactive_root_cell_ids())
        .unwrap();
    assert_eq!((*ca.borrow(), *cb.borrow()), (1, 0));
    p.advance_reactive_turn(&mut s, &[]).unwrap();
    assert_eq!((*ca.borrow(), *cb.borrow()), (1, 1));
    assert_ne!(ra, rb);
}

#[test]
fn reactive_turn_combines_carried_and_new_registers() {
    let mut p = ReactivePlan::new();
    let input = Ref::new(1.);
    let (a, _, sa, ca) = reg(&mut p, input.clone(), Ref::new(0.), false);
    let (b, _, sb, cb) = reg(&mut p, input.clone(), Ref::new(0.), false);
    let mut s = ReactiveTurnState {
        pending_register_nodes: vec![b],
    };
    let o = p
        .advance_reactive_turn(&mut s, &input.to_value().reactive_root_cell_ids())
        .unwrap();
    assert_eq!(o.register_commit.staged_nodes, vec![a, b]);
    assert_eq!(o.register_commit.committed_nodes, vec![a, b]);
    assert_eq!(
        (*sa.borrow(), *sb.borrow(), *ca.borrow(), *cb.borrow()),
        (1, 1, 1, 1)
    );
}

#[test]
fn reactive_turn_combinational_only_has_empty_commit() {
    let mut p = ReactivePlan::new();
    let input = Ref::new(1.);
    let a = Ref::new(2.);
    let b = Ref::new(3.);
    let (na, _) = comb(&mut p, input.clone(), a.clone(), false);
    let (nb, _) = comb(&mut p, a.clone(), b.clone(), false);
    *input.borrow_mut() = 10.;
    let mut s = ReactiveTurnState::default();
    let o = p
        .advance_reactive_turn(&mut s, &input.to_value().reactive_root_cell_ids())
        .unwrap();
    assert_eq!(o.before_commit.executed_nodes, vec![na, nb]);
    assert_eq!(o.register_commit, ReactiveRegisterCommitOutcome::default());
    assert_eq!(o.after_commit, ReactivePlanSolveOutcome::default());
    assert_eq!(*b.borrow(), 12.);
}

#[test]
fn reactive_turn_empty_is_noop() {
    let mut p = ReactivePlan::new();
    let mut s = ReactiveTurnState::default();
    assert_eq!(
        p.advance_reactive_turn(&mut s, &[]).unwrap(),
        ReactiveTurnOutcome::default()
    );
    assert_eq!(s, ReactiveTurnState::default());
}

#[test]
fn reactive_turn_commit_failure_skips_post_commit_propagation() {
    let mut p = ReactivePlan::new();
    let input = Ref::new(1.);
    let sink = Ref::new(1.);
    let (r, solve, stage, commit) = reg(&mut p, input.clone(), sink.clone(), true);
    let (_, down) = comb(&mut p, sink.clone(), Ref::new(2.), false);
    let mut s = ReactiveTurnState::default();
    let e = p
        .advance_reactive_turn(&mut s, &input.to_value().reactive_root_cell_ids())
        .unwrap_err();
    assert!(e.kind_message().contains("stage failure"));
    assert_eq!(
        (
            *solve.borrow(),
            *stage.borrow(),
            *commit.borrow(),
            *down.borrow(),
            *sink.borrow()
        ),
        (0, 1, 0, 0, 1.)
    );
    assert_eq!(s.pending_register_nodes, vec![r]);
}

#[test]
fn reactive_turn_post_commit_failure_does_not_requeue_committed_registers() {
    let mut p = ReactivePlan::new();
    let input = Ref::new(1.);
    let sink = Ref::new(1.);
    let (_, _, _, commit) = reg(&mut p, input.clone(), sink.clone(), false);
    let (_, down) = comb(&mut p, sink.clone(), Ref::new(2.), true);
    *input.borrow_mut() = 10.;
    let mut s = ReactiveTurnState::default();
    assert!(
        p.advance_reactive_turn(&mut s, &input.to_value().reactive_root_cell_ids())
            .is_err()
    );
    assert_eq!(
        (*sink.borrow(), *commit.borrow(), *down.borrow()),
        (10., 1, 1)
    );
    assert!(s.pending_register_nodes.is_empty());
}

#[test]
fn reactive_turn_post_commit_failure_preserves_deferred_registers() {
    let mut p = ReactivePlan::new();
    let input = Ref::new(1.);
    let a = Ref::new(1.);
    let middle = Ref::new(2.);
    let b = Ref::new(2.);
    let (a_register, _, _, a_commits) = reg(&mut p, input.clone(), a.clone(), false);
    let (_, middle_solves) = comb(&mut p, a.clone(), middle.clone(), false);
    let (b_register, _, _, b_commits) = reg(&mut p, middle.clone(), b.clone(), false);
    let (_, error_solves) = comb(&mut p, middle.clone(), Ref::new(0.), true);

    *input.borrow_mut() = 10.;
    let mut state = ReactiveTurnState::default();
    let error = p
        .advance_reactive_turn(&mut state, &input.to_value().reactive_root_cell_ids())
        .unwrap_err();

    assert!(error.kind_message().contains("solve failure"));
    assert_eq!((*a.borrow(), *middle.borrow(), *b.borrow()), (10., 11., 2.));
    assert_eq!((*a_commits.borrow(), *b_commits.borrow()), (1, 0));
    assert_eq!((*middle_solves.borrow(), *error_solves.borrow()), (1, 1));
    assert_eq!(state.pending_register_nodes, vec![b_register]);
    assert!(!state.pending_register_nodes.contains(&a_register));

    let retry = p.advance_reactive_turn(&mut state, &[]).unwrap();
    assert_eq!(retry.register_commit.committed_nodes, vec![b_register]);
    assert_eq!((*a_commits.borrow(), *b_commits.borrow()), (1, 1));
    assert_eq!(*b.borrow(), 11.);
    assert!(state.pending_register_nodes.is_empty());
}

#[test]
fn reactive_turn_reuses_existing_plan() {
    let mut p = ReactivePlan::new();
    let input = Ref::new(1.);
    let sink = Ref::new(1.);
    reg(&mut p, input.clone(), sink.clone(), false);
    comb(&mut p, sink.clone(), Ref::new(2.), false);
    let len = p.len();
    let ids = p.nodes.iter().map(|n| n.id).collect::<Vec<_>>();
    let outputs = p
        .nodes
        .iter()
        .map(|n| n.outputs.clone())
        .collect::<Vec<_>>();
    let mut s = ReactiveTurnState::default();
    for value in [10., 20.] {
        *input.borrow_mut() = value;
        p.advance_reactive_turn(&mut s, &input.to_value().reactive_root_cell_ids())
            .unwrap();
        assert_eq!(p.len(), len);
        assert_eq!(p.nodes.iter().map(|n| n.id).collect::<Vec<_>>(), ids);
        assert_eq!(
            p.nodes
                .iter()
                .map(|n| n.outputs.clone())
                .collect::<Vec<_>>(),
            outputs
        );
    }
}

#[test]
fn reactive_turn_pre_commit_failure_preserves_carried_registers() {
    let mut p = ReactivePlan::new();
    let input = Ref::new(1.);
    let (carried, solve, stage, commit) = reg(&mut p, Ref::new(2.), Ref::new(3.), false);
    comb(&mut p, input.clone(), Ref::new(0.), true);
    let mut state = ReactiveTurnState {
        pending_register_nodes: vec![carried],
    };
    let error = p
        .advance_reactive_turn(&mut state, &input.to_value().reactive_root_cell_ids())
        .unwrap_err();
    assert!(error.kind_message().contains("solve failure"));
    assert_eq!(
        (*solve.borrow(), *stage.borrow(), *commit.borrow()),
        (0, 0, 0)
    );
    assert_eq!(state.pending_register_nodes, vec![carried]);
}
