#[cfg(feature = "semantic-compiler")]
use super::super::MechFunctionCompiler;
use super::super::{
    FunctionDefinition, FunctionInvocation, MechFunctionImpl, ReactiveNodeId, ReactiveNodeKind,
    ReactivePlan, ReactivePlanSolveOutcome, ReactiveRegisterCommitOutcome, ReactiveSolveStatus,
    ReactiveTurnOutcome, ReactiveTurnState,
};
use super::support::reg;
#[cfg(feature = "semantic-compiler")]
use crate::{BytecodeCompilerContext, Register};
use crate::{
    FunctionDefine, GenericError, MResult, MechError, ValueCell, hash_str,
    internal_pattern_value_identifier,
};
use std::{cell::RefCell, rc::Rc};

#[cfg(feature = "u64")]
struct FalseInvariantWriter {
    output: crate::ManagedPort<u64>,
}

#[cfg(feature = "u64")]
impl MechFunctionImpl for FalseInvariantWriter {
    fn payload_output_plan_policy(&self) -> crate::PayloadOutputPlanPolicy {
        crate::PayloadOutputPlanPolicy::PublishedInvariant
    }

    fn solve_managed(
        &self,
        frame: &mut crate::KernelMemoryFrame<'_>,
        _services: &mut dyn crate::MechExecutionServices,
    ) -> MResult<ReactiveSolveStatus> {
        frame.with_port_init_writer(&self.output, |writer| writer.write_next(99))?;
        Ok(ReactiveSolveStatus::Changed)
    }

    fn to_string(&self) -> String {
        "false published-invariant writer".into()
    }
}

#[cfg(all(feature = "u64", feature = "semantic-compiler"))]
impl MechFunctionCompiler for FalseInvariantWriter {
    fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

#[cfg(feature = "u64")]
#[test]
fn published_invariant_policy_cannot_obtain_output_write_authority() {
    let output = ValueCell::from_exact(7_u64).unwrap();
    let version = output.published_version();
    let instance = crate::function::test_planned_instance(
        Box::new(FalseInvariantWriter {
            output: crate::ManagedPort::output(output.clone()),
        }),
        FunctionInvocation::nullary(output.clone()),
    );

    let error = instance.solve_result().unwrap_err();
    assert_eq!(error.kind_name(), "MemoryRuntimeError");
    assert!(matches!(
        output.snapshot().unwrap().data(),
        crate::ValueData::U64(7)
    ));
    assert_eq!(output.published_version(), version);
}

#[cfg(feature = "f64")]
struct SchedulerFunction {
    label: &'static str,
    output: crate::ManagedPort<f64>,
    kind: ReactiveNodeKind,
    status: ReactiveSolveStatus,
    count: Rc<RefCell<usize>>,
    log: Rc<RefCell<Vec<&'static str>>>,
    error: bool,
}
#[cfg(feature = "f64")]
impl MechFunctionImpl for SchedulerFunction {
    fn solve_managed(
        &self,
        frame: &mut crate::KernelMemoryFrame<'_>,
        _services: &mut dyn crate::MechExecutionServices,
    ) -> MResult<crate::ReactiveSolveStatus> {
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
            if self.status == ReactiveSolveStatus::Changed {
                frame.with_port_init_writer(&self.output, |writer| writer.write_next(0.0))?;
            }
            Ok(self.status)
        }
    }
    fn reactive_node_kind(&self) -> ReactiveNodeKind {
        self.kind
    }
    fn reactive_output_value_cells(&self) -> Vec<crate::ValueCell> {
        vec![self.output.cell().clone()]
    }
    fn to_string(&self) -> String {
        self.label.into()
    }
}
#[cfg(all(feature = "f64", feature = "semantic-compiler"))]
impl MechFunctionCompiler for SchedulerFunction {
    fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

#[cfg(feature = "f64")]
fn scheduler_node(
    plan: &mut ReactivePlan,
    label: &'static str,
    inputs: &[ValueCell],
    kind: ReactiveNodeKind,
    status: ReactiveSolveStatus,
    log: Rc<RefCell<Vec<&'static str>>>,
    error: bool,
) -> (ReactiveNodeId, ValueCell, Rc<RefCell<usize>>) {
    let output = ValueCell::from_exact(0.0).unwrap();
    let count = Rc::new(RefCell::new(0));
    let function = SchedulerFunction {
        label,
        output: crate::ManagedPort::output(output.clone()),
        kind,
        status,
        count: count.clone(),
        log,
        error,
    };
    (
        plan.register_instance_with_activation(
            crate::function::test_planned_instance(
                Box::new(function),
                FunctionInvocation::variadic(output.clone(), inputs.to_vec().into_boxed_slice()),
            ),
            None,
        )
        .unwrap(),
        output,
        count,
    )
}
#[cfg(feature = "f64")]
fn scheduler_source() -> ValueCell {
    ValueCell::from_exact(0.0).unwrap()
}

struct Comb {
    source: crate::ManagedPort<f64>,
    sink: crate::ManagedPort<f64>,
    add: f64,
    count: Rc<RefCell<usize>>,
    fail: bool,
}

#[cfg(feature = "f64")]
struct FalliblePlanStep {
    label: &'static str,
    output: crate::ManagedPort<f64>,
    next: f64,
    calls: Rc<RefCell<usize>>,
    fail: bool,
}

#[cfg(feature = "f64")]
impl MechFunctionImpl for FalliblePlanStep {
    fn solve_managed(
        &self,
        frame: &mut crate::KernelMemoryFrame<'_>,
        _services: &mut dyn crate::MechExecutionServices,
    ) -> MResult<crate::ReactiveSolveStatus> {
        (|| -> MResult<()> {
            *self.calls.borrow_mut() += 1;
            if self.fail {
                return Err(MechError::new(
                    GenericError {
                        msg: self.label.into(),
                    },
                    None,
                ));
            }
            frame.with_port_init_writer(&self.output, |writer| writer.write_next(self.next))?;
            Ok(())
        })()?;
        Ok(crate::ReactiveSolveStatus::Changed)
    }

    fn primary_output_state_port(&self) -> Option<crate::FunctionStatePort<'_>> {
        Some(crate::FunctionStatePort::from_cell(self.output.cell()))
    }

    fn to_string(&self) -> String {
        self.label.into()
    }
}

#[cfg(all(feature = "f64", feature = "semantic-compiler"))]
impl MechFunctionCompiler for FalliblePlanStep {
    fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

#[cfg(feature = "f64")]
#[test]
fn function_definition_plan_propagates_solve_failure_without_publishing_later_outputs() {
    let definition = FunctionDefinition::new(
        hash_str("fallible-plan"),
        "fallible-plan".into(),
        FunctionDefine {
            name: internal_pattern_value_identifier("fallible-plan"),
            input: Vec::new(),
            output: Vec::new(),
            statements: Vec::new(),
            match_arms: Vec::new(),
        },
    );
    let failed_output = ValueCell::from_exact(7.0_f64).unwrap();
    let later_output = ValueCell::from_exact(11.0_f64).unwrap();
    let failed_calls = Rc::new(RefCell::new(0));
    let later_calls = Rc::new(RefCell::new(0));
    definition
        .plan
        .add_function(crate::function::test_planned_instance(
            Box::new(FalliblePlanStep {
                label: "plan solve failed",
                output: crate::ManagedPort::output(failed_output.clone()),
                next: 8.0,
                calls: failed_calls.clone(),
                fail: true,
            }),
            FunctionInvocation::nullary(failed_output.clone()),
        ))
        .unwrap();
    definition
        .plan
        .add_function(crate::function::test_planned_instance(
            Box::new(FalliblePlanStep {
                label: "later plan step",
                output: crate::ManagedPort::output(later_output.clone()),
                next: 12.0,
                calls: later_calls.clone(),
                fail: false,
            }),
            FunctionInvocation::nullary(later_output.clone()),
        ))
        .unwrap();

    let error = definition.solve_result().unwrap_err();

    assert!(error.full_chain_message().contains("plan solve failed"));
    assert!(
        matches!(failed_output.snapshot().unwrap().data(), crate::ValueData::F64(value) if value.to_f64() == 7.0)
    );
    assert!(
        matches!(later_output.snapshot().unwrap().data(), crate::ValueData::F64(value) if value.to_f64() == 11.0)
    );
    assert_eq!(*failed_calls.borrow(), 1);
    assert_eq!(*later_calls.borrow(), 0);
}
impl MechFunctionImpl for Comb {
    fn solve_managed(
        &self,
        frame: &mut crate::KernelMemoryFrame<'_>,
        _services: &mut dyn crate::MechExecutionServices,
    ) -> MResult<crate::ReactiveSolveStatus> {
        *self.count.borrow_mut() += 1;
        if self.fail {
            return Err(MechError::new(
                GenericError {
                    msg: "solve failure".into(),
                },
                None,
            ));
        }
        frame.with_unary_port_views(&self.source, &self.sink, |source, sink| {
            sink.try_fill_column_major(|index| {
                source
                    .get_column_major(index)
                    .map(|value| value + self.add)
                    .ok_or_else(|| {
                        MechError::new(
                            GenericError {
                                msg: "combinational input and output geometry disagree".into(),
                            },
                            None,
                        )
                    })
            })
        })?;
        Ok(crate::ReactiveSolveStatus::Changed)
    }
    fn primary_output_state_port(&self) -> Option<crate::FunctionStatePort<'_>> {
        Some(crate::FunctionStatePort::from_cell(self.sink.cell()))
    }
    fn to_string(&self) -> String {
        "test combinational".into()
    }
}
#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for Comb {
    fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}
fn comb(
    p: &mut ReactivePlan,
    source: ValueCell,
    sink: ValueCell,
    fail: bool,
) -> (ReactiveNodeId, Rc<RefCell<usize>>) {
    let count = Rc::new(RefCell::new(0));
    let invocation = FunctionInvocation::unary(sink, source);
    let (sink, source) = invocation.expect_unary().unwrap();
    let node = p
        .register_instance_with_activation(
            crate::function::test_planned_instance(
                Box::new(Comb {
                    source: source.try_managed_element::<f64>().unwrap(),
                    sink: sink.try_managed_element::<f64>().unwrap(),
                    add: 1.,
                    count: count.clone(),
                    fail,
                }),
                invocation,
            ),
            None,
        )
        .unwrap();
    (node, count)
}
fn chain() -> (
    ReactivePlan,
    ValueCell,
    ValueCell,
    ValueCell,
    ValueCell,
    ReactiveNodeId,
    ReactiveNodeId,
    ReactiveNodeId,
    Rc<RefCell<usize>>,
    Rc<RefCell<usize>>,
) {
    let mut p = ReactivePlan::new();
    let input = ValueCell::from_exact(1.).unwrap();
    let a = ValueCell::from_exact(1.).unwrap();
    let middle = ValueCell::from_exact(2.).unwrap();
    let b = ValueCell::from_exact(2.).unwrap();
    let final_value = ValueCell::from_exact(3.).unwrap();
    let (ra, _, ca, _) = reg(&mut p, input.clone(), a.clone(), false);
    drop(comb(&mut p, a.clone(), middle.clone(), false));
    let (rb, _, cb, _) = reg(&mut p, middle.clone(), b.clone(), false);
    let (final_node, _) = comb(&mut p, b.clone(), final_value.clone(), false);
    (p, input, a, middle, b, ra, rb, final_node, ca, cb)
}

fn read_f64(cell: &ValueCell) -> f64 {
    let value = cell.snapshot().unwrap();
    let crate::ValueData::F64(value) = value.data() else {
        panic!("expected f64 cell")
    };
    value.to_f64()
}

fn write_f64(cell: &ValueCell, value: f64) {
    cell.replace(&ValueCell::from_exact(value).unwrap().snapshot().unwrap())
        .unwrap();
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
    let o = p.solve_dirty_cells(&[d.reactive_cell_id()]).unwrap();
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
        p.solve_dirty_cells(&[y.reactive_cell_id(), x.reactive_cell_id()])
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
    let o = p.solve_dirty_cells(&[d.reactive_cell_id()]).unwrap();
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
    let cell = d.reactive_cell_id();
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
    p.solve_dirty_cells(&[x.reactive_cell_id(), y.reactive_cell_id()])
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
        &[ao.clone()],
        ReactiveNodeKind::Combinational,
        ReactiveSolveStatus::Changed,
        l,
        false,
    );
    let version = ao.published_version();
    let before = ao.snapshot().unwrap();
    let allocations = ao.memory_domain().unwrap().ledger();
    let o = p.solve_dirty_cells(&[d.reactive_cell_id()]).unwrap();
    assert_eq!(*ac.borrow(), 1);
    assert_eq!(*bc.borrow(), 0);
    assert_eq!(o.unchanged_nodes, vec![a]);
    assert!(!o.executed_nodes.contains(&b));
    assert_eq!(ao.published_version(), version);
    assert_eq!(ao.memory_domain().unwrap().ledger(), allocations);
    let after = ao.snapshot().unwrap();
    assert!(
        before
            .snapshot_eq(
                before.schemas().as_ref().unwrap().as_ref(),
                &after,
                after.schemas().as_ref().unwrap().as_ref(),
            )
            .unwrap()
    );
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
        .entry(d.reactive_cell_id())
        .or_default()
        .push(n);
    let o = p.solve_dirty_cells(&[d.reactive_cell_id()]).unwrap();
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
    let o = p.solve_dirty_cells(&[d.reactive_cell_id()]).unwrap();
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
    let o = p.solve_dirty_cells(&[d.reactive_cell_id()]).unwrap();
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
    let cell = ro.reactive_cell_id();
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
    let e = p.solve_dirty_cells(&[d.reactive_cell_id()]).unwrap_err();
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
    let input = ValueCell::from_exact(1.).unwrap();
    let a = ValueCell::from_exact(1.).unwrap();
    let out = ValueCell::from_exact(2.).unwrap();
    let (r, solve, stage, _) = reg(&mut p, input.clone(), a.clone(), false);
    let (d, down) = comb(&mut p, a.clone(), out.clone(), false);
    write_f64(&input, 10.);
    let mut s = ReactiveTurnState::default();
    let o = p
        .advance_reactive_turn(&mut s, &[input.reactive_cell_id()])
        .unwrap();
    assert_eq!(o.before_commit.pending_register_nodes, vec![r]);
    assert_eq!(o.register_commit.staged_nodes, vec![r]);
    assert_eq!(o.register_commit.committed_nodes, vec![r]);
    assert!(o.after_commit.executed_nodes.contains(&d));
    assert_eq!(
        (
            read_f64(&a),
            read_f64(&out),
            *solve.borrow(),
            *stage.borrow(),
            *down.borrow()
        ),
        (10., 11., 0, 1, 1)
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
        .instance()
        .unwrap()
        .output()
        .clone();
    write_f64(&input, 10.);
    let mut s = ReactiveTurnState::default();
    let first = p
        .advance_reactive_turn(&mut s, &[input.reactive_cell_id()])
        .unwrap();
    assert_eq!(first.register_commit.committed_nodes, vec![ra]);
    assert_eq!(first.after_commit.pending_register_nodes, vec![rb]);
    assert_eq!(s.pending_register_nodes, vec![rb]);
    assert_eq!(
        (
            read_f64(&a),
            read_f64(&middle),
            read_f64(&b),
            read_f64(&final_value),
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
    write_f64(&input, 10.);
    let mut s = ReactiveTurnState::default();
    p.advance_reactive_turn(&mut s, &[input.reactive_cell_id()])
        .unwrap();
    assert_eq!((*ca.borrow(), *cb.borrow()), (1, 0));
    p.advance_reactive_turn(&mut s, &[]).unwrap();
    assert_eq!((*ca.borrow(), *cb.borrow()), (1, 1));
    assert_ne!(ra, rb);
}

#[test]
fn reactive_turn_combines_carried_and_new_registers() {
    let mut p = ReactivePlan::new();
    let input = ValueCell::from_exact(1.).unwrap();
    let (a, _, sa, _) = reg(
        &mut p,
        input.clone(),
        ValueCell::from_exact(0.).unwrap(),
        false,
    );
    let (b, _, sb, _) = reg(
        &mut p,
        input.clone(),
        ValueCell::from_exact(0.).unwrap(),
        false,
    );
    let mut s = ReactiveTurnState {
        pending_register_nodes: vec![b],
    };
    let o = p
        .advance_reactive_turn(&mut s, &[input.reactive_cell_id()])
        .unwrap();
    assert_eq!(o.register_commit.staged_nodes, vec![a, b]);
    assert_eq!(o.register_commit.committed_nodes, vec![a, b]);
    assert_eq!((*sa.borrow(), *sb.borrow()), (1, 1));
}

#[test]
fn reactive_turn_combinational_only_has_empty_commit() {
    let mut p = ReactivePlan::new();
    let input = ValueCell::from_exact(1.).unwrap();
    let a = ValueCell::from_exact(2.).unwrap();
    let b = ValueCell::from_exact(3.).unwrap();
    let (na, _) = comb(&mut p, input.clone(), a.clone(), false);
    let (nb, _) = comb(&mut p, a.clone(), b.clone(), false);
    write_f64(&input, 10.);
    let mut s = ReactiveTurnState::default();
    let o = p
        .advance_reactive_turn(&mut s, &[input.reactive_cell_id()])
        .unwrap();
    assert_eq!(o.before_commit.executed_nodes, vec![na, nb]);
    assert_eq!(o.register_commit, ReactiveRegisterCommitOutcome::default());
    assert_eq!(o.after_commit, ReactivePlanSolveOutcome::default());
    assert_eq!(read_f64(&b), 12.);
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
    let input = ValueCell::from_exact(1.).unwrap();
    let sink = ValueCell::from_exact(1.).unwrap();
    let (r, solve, stage, _) = reg(&mut p, input.clone(), sink.clone(), true);
    let (_, down) = comb(
        &mut p,
        sink.clone(),
        ValueCell::from_exact(2.).unwrap(),
        false,
    );
    let mut s = ReactiveTurnState::default();
    let e = p
        .advance_reactive_turn(&mut s, &[input.reactive_cell_id()])
        .unwrap_err();
    assert!(e.kind_message().contains("stage failure"));
    assert_eq!(
        (
            *solve.borrow(),
            *stage.borrow(),
            *down.borrow(),
            read_f64(&sink)
        ),
        (0, 1, 0, 1.)
    );
    assert_eq!(s.pending_register_nodes, vec![r]);
}

#[test]
fn reactive_turn_post_commit_failure_does_not_requeue_committed_registers() {
    let mut p = ReactivePlan::new();
    let input = ValueCell::from_exact(1.).unwrap();
    let sink = ValueCell::from_exact(1.).unwrap();
    reg(&mut p, input.clone(), sink.clone(), false);
    let (_, down) = comb(
        &mut p,
        sink.clone(),
        ValueCell::from_exact(2.).unwrap(),
        true,
    );
    write_f64(&input, 10.);
    let mut s = ReactiveTurnState::default();
    assert!(
        p.advance_reactive_turn(&mut s, &[input.reactive_cell_id()])
            .is_err()
    );
    assert_eq!((read_f64(&sink), *down.borrow()), (10., 1));
    assert!(s.pending_register_nodes.is_empty());
}

#[test]
fn reactive_turn_post_commit_failure_preserves_deferred_registers() {
    let mut p = ReactivePlan::new();
    let input = ValueCell::from_exact(1.).unwrap();
    let a = ValueCell::from_exact(1.).unwrap();
    let middle = ValueCell::from_exact(2.).unwrap();
    let b = ValueCell::from_exact(2.).unwrap();
    let (a_register, _, a_stages, _) = reg(&mut p, input.clone(), a.clone(), false);
    let (_, middle_solves) = comb(&mut p, a.clone(), middle.clone(), false);
    let (b_register, _, b_stages, _) = reg(&mut p, middle.clone(), b.clone(), false);
    let (_, error_solves) = comb(
        &mut p,
        middle.clone(),
        ValueCell::from_exact(0.).unwrap(),
        true,
    );

    write_f64(&input, 10.);
    let mut state = ReactiveTurnState::default();
    let error = p
        .advance_reactive_turn(&mut state, &[input.reactive_cell_id()])
        .unwrap_err();

    assert!(error.kind_message().contains("solve failure"));
    assert_eq!(
        (read_f64(&a), read_f64(&middle), read_f64(&b)),
        (10., 11., 2.)
    );
    assert_eq!((*a_stages.borrow(), *b_stages.borrow()), (1, 0));
    assert_eq!((*middle_solves.borrow(), *error_solves.borrow()), (1, 1));
    assert_eq!(state.pending_register_nodes, vec![b_register]);
    assert!(!state.pending_register_nodes.contains(&a_register));

    let retry = p.advance_reactive_turn(&mut state, &[]).unwrap();
    assert_eq!(retry.register_commit.committed_nodes, vec![b_register]);
    assert_eq!((*a_stages.borrow(), *b_stages.borrow()), (1, 1));
    assert_eq!(read_f64(&b), 11.);
    assert!(state.pending_register_nodes.is_empty());
}

#[test]
fn reactive_turn_reuses_existing_plan() {
    let mut p = ReactivePlan::new();
    let input = ValueCell::from_exact(1.).unwrap();
    let sink = ValueCell::from_exact(1.).unwrap();
    reg(&mut p, input.clone(), sink.clone(), false);
    comb(
        &mut p,
        sink.clone(),
        ValueCell::from_exact(2.).unwrap(),
        false,
    );
    let len = p.len();
    let ids = p.nodes.iter().map(|n| n.id).collect::<Vec<_>>();
    let outputs = p
        .nodes
        .iter()
        .map(|n| n.outputs.clone())
        .collect::<Vec<_>>();
    let mut s = ReactiveTurnState::default();
    for value in [10., 20.] {
        write_f64(&input, value);
        p.advance_reactive_turn(&mut s, &[input.reactive_cell_id()])
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
    let input = ValueCell::from_exact(1.).unwrap();
    let (carried, solve, stage, commit) = reg(
        &mut p,
        ValueCell::from_exact(2.).unwrap(),
        ValueCell::from_exact(3.).unwrap(),
        false,
    );
    comb(
        &mut p,
        input.clone(),
        ValueCell::from_exact(0.).unwrap(),
        true,
    );
    let mut state = ReactiveTurnState {
        pending_register_nodes: vec![carried],
    };
    let error = p
        .advance_reactive_turn(&mut state, &[input.reactive_cell_id()])
        .unwrap_err();
    assert!(error.kind_message().contains("solve failure"));
    assert_eq!(
        (*solve.borrow(), *stage.borrow(), *commit.borrow()),
        (0, 0, 0)
    );
    assert_eq!(state.pending_register_nodes, vec![carried]);
}
