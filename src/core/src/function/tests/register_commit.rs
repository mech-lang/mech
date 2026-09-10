#[cfg(feature = "semantic-compiler")]
use super::super::MechFunctionCompiler;
use super::super::{
    FunctionInvocation, MechFunctionImpl, ReactiveNodeId, ReactiveNodeKind, ReactivePlan,
    ReactiveRegisterCommitOutcome,
};
#[cfg(feature = "semantic-compiler")]
use crate::{BytecodeCompilerContext, Register};
use crate::{
    FunctionStatePort, GenericError, MResult, ManagedPort, MechError, ReactiveSolveStatus,
    ValueCell, ValueData,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

struct RegisterFunction {
    label: &'static str,
    output: ManagedPort<f64>,
    sources: Vec<ManagedPort<f64>>,
    stages: Rc<RefCell<Vec<&'static str>>>,
    fail: bool,
}

impl MechFunctionImpl for RegisterFunction {
    fn solve_managed(
        &self,
        frame: &mut crate::KernelMemoryFrame<'_>,
        _services: &mut dyn crate::MechExecutionServices,
    ) -> MResult<ReactiveSolveStatus> {
        self.stages.borrow_mut().push(self.label);
        if self.fail {
            return Err(MechError::new(
                GenericError {
                    msg: format!("{} failed to stage", self.label),
                },
                None,
            ));
        }
        let mut next = 0.0;
        for source in &self.sources {
            next += frame.with_port_slice(source, |values| values[0])?;
        }
        frame.with_port_init_writer(&self.output, |writer| writer.write_next(next))?;
        Ok(ReactiveSolveStatus::Changed)
    }

    fn reactive_node_kind(&self) -> ReactiveNodeKind {
        ReactiveNodeKind::Register
    }

    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_cell(self.output.cell()))
    }

    fn to_string(&self) -> String {
        self.label.into()
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for RegisterFunction {
    fn compile(&self, _context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

fn register(
    plan: &mut ReactivePlan,
    label: &'static str,
    output: ValueCell,
    sources: Vec<ValueCell>,
    stages: Rc<RefCell<Vec<&'static str>>>,
    fail: bool,
) -> ReactiveNodeId {
    let invocation = FunctionInvocation::variadic(output, sources.into_boxed_slice());
    let output = invocation.output().try_managed_element::<f64>().unwrap();
    let sources = invocation
        .inputs()
        .map(|source| source.try_managed_element::<f64>().unwrap())
        .collect();
    plan.register_instance_with_activation(
        crate::function::test_planned_instance(
            Box::new(RegisterFunction {
                label,
                output,
                sources,
                stages,
                fail,
            }),
            invocation,
        ),
        None,
    )
    .unwrap()
}

fn f64_value(cell: &ValueCell) -> f64 {
    let value = cell.snapshot().unwrap();
    let ValueData::F64(value) = value.data() else {
        panic!("expected f64 cell")
    };
    value.to_f64()
}

struct ColdReplanRegister {
    output: ManagedPort<f64>,
    source: ManagedPort<f64>,
    fail: Rc<Cell<bool>>,
}

impl MechFunctionImpl for ColdReplanRegister {
    fn solve_managed(
        &self,
        frame: &mut crate::KernelMemoryFrame<'_>,
        _services: &mut dyn crate::MechExecutionServices,
    ) -> MResult<ReactiveSolveStatus> {
        if self.fail.get() {
            return Err(MechError::new(
                GenericError {
                    msg: "cold-replan register failed after preparation".into(),
                },
                None,
            ));
        }
        frame.with_unary_port_views(&self.source, &self.output, |source, output| {
            let mut next = 0.0;
            for index in 0..source.len() {
                next += source.get_column_major(index).ok_or_else(|| {
                    MechError::new(
                        GenericError {
                            msg: "cold-replan source geometry is incomplete".into(),
                        },
                        None,
                    )
                })?;
            }
            output.try_fill_column_major(|_| Ok(next))
        })?;
        Ok(ReactiveSolveStatus::Changed)
    }

    fn reactive_node_kind(&self) -> ReactiveNodeKind {
        ReactiveNodeKind::Register
    }

    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_cell(self.output.cell()))
    }

    fn to_string(&self) -> String {
        "cold-replan register".into()
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for ColdReplanRegister {
    fn compile(&self, _context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

fn cold_replan_register(
    plan: &mut ReactivePlan,
    output: ValueCell,
    source: ValueCell,
    fail: Rc<Cell<bool>>,
) -> ReactiveNodeId {
    let invocation = FunctionInvocation::unary(output, source);
    let (output, source) = invocation.expect_unary().unwrap();
    plan.register_instance_with_activation(
        crate::function::test_planned_instance(
            Box::new(ColdReplanRegister {
                output: output.try_managed_element::<f64>().unwrap(),
                source: source.try_managed_element::<f64>().unwrap(),
                fail,
            }),
            invocation,
        ),
        None,
    )
    .unwrap()
}

fn replace_f64_matrix(cell: &ValueCell, rows: usize, columns: usize, value: f64) {
    let replacement = ValueCell::from_exact(nalgebra::DMatrix::from_element(rows, columns, value))
        .unwrap()
        .snapshot()
        .unwrap();
    cell.replace(&replacement).unwrap();
}

#[test]
fn failed_cold_replan_registers_reclaim_first_and_distinct_later_domains() {
    let mut first_plan = ReactivePlan::new();
    let first_output = ValueCell::from_exact(1.0_f64).unwrap();
    let first_source =
        ValueCell::from_exact(nalgebra::DMatrix::from_element(1, 1, 2.0_f64)).unwrap();
    let first_fail = Rc::new(Cell::new(true));
    let first = cold_replan_register(
        &mut first_plan,
        first_output.clone(),
        first_source.clone(),
        first_fail.clone(),
    );
    let first_domain = first_output.memory_domain().unwrap();
    let first_ledger = first_domain.ledger();
    let first_metadata = first_domain.metadata_observation();
    let first_revision = first_plan[first]
        .instance()
        .unwrap()
        .managed_plan_revision();
    let first_version = first_output.published_version();
    replace_f64_matrix(&first_source, 2, 3, 2.0);

    for _ in 0..8 {
        let error = first_plan.commit_pending_registers(&[first]).unwrap_err();
        assert!(
            error
                .simple_message()
                .contains("cold-replan register failed after preparation")
        );
        assert_eq!(first_domain.ledger(), first_ledger);
        assert_eq!(first_domain.metadata_observation(), first_metadata);
        assert_eq!(f64_value(&first_output), 1.0);
        assert_eq!(first_output.published_version(), first_version);
        assert_eq!(
            first_plan[first]
                .instance()
                .unwrap()
                .managed_plan_revision(),
            first_revision
        );
    }
    first_fail.set(false);
    first_plan.commit_pending_registers(&[first]).unwrap();
    assert_eq!(f64_value(&first_output), 12.0);
    assert_ne!(
        first_plan[first]
            .instance()
            .unwrap()
            .managed_plan_revision(),
        first_revision
    );

    let mut batch_plan = ReactivePlan::new();
    let earlier_output = ValueCell::from_exact(3.0_f64).unwrap();
    let failing_output = ValueCell::from_exact(4.0_f64).unwrap();
    let earlier_source =
        ValueCell::from_exact(nalgebra::DMatrix::from_element(1, 1, 5.0_f64)).unwrap();
    let failing_source =
        ValueCell::from_exact(nalgebra::DMatrix::from_element(1, 1, 6.0_f64)).unwrap();
    let earlier_fail = Rc::new(Cell::new(false));
    let later_fail = Rc::new(Cell::new(true));
    let earlier = cold_replan_register(
        &mut batch_plan,
        earlier_output.clone(),
        earlier_source.clone(),
        earlier_fail,
    );
    let later = cold_replan_register(
        &mut batch_plan,
        failing_output.clone(),
        failing_source.clone(),
        later_fail.clone(),
    );
    let earlier_domain = earlier_output.memory_domain().unwrap();
    let later_domain = failing_output.memory_domain().unwrap();
    assert_ne!(earlier_domain.id(), later_domain.id());
    let earlier_ledger = earlier_domain.ledger();
    let later_ledger = later_domain.ledger();
    let earlier_metadata = earlier_domain.metadata_observation();
    let later_metadata = later_domain.metadata_observation();
    let earlier_revision = batch_plan[earlier]
        .instance()
        .unwrap()
        .managed_plan_revision();
    let later_revision = batch_plan[later]
        .instance()
        .unwrap()
        .managed_plan_revision();
    let earlier_version = earlier_output.published_version();
    let later_version = failing_output.published_version();
    replace_f64_matrix(&earlier_source, 2, 2, 5.0);
    replace_f64_matrix(&failing_source, 3, 2, 6.0);

    for _ in 0..8 {
        let error = batch_plan
            .commit_pending_registers(&[earlier, later])
            .unwrap_err();
        assert!(
            error
                .simple_message()
                .contains("cold-replan register failed after preparation")
        );
        assert_eq!(earlier_domain.ledger(), earlier_ledger);
        assert_eq!(later_domain.ledger(), later_ledger);
        assert_eq!(earlier_domain.metadata_observation(), earlier_metadata);
        assert_eq!(later_domain.metadata_observation(), later_metadata);
        assert_eq!(
            (f64_value(&earlier_output), f64_value(&failing_output)),
            (3.0, 4.0)
        );
        assert_eq!(earlier_output.published_version(), earlier_version);
        assert_eq!(failing_output.published_version(), later_version);
        assert_eq!(
            batch_plan[earlier]
                .instance()
                .unwrap()
                .managed_plan_revision(),
            earlier_revision
        );
        assert_eq!(
            batch_plan[later]
                .instance()
                .unwrap()
                .managed_plan_revision(),
            later_revision
        );
    }
    later_fail.set(false);
    batch_plan
        .commit_pending_registers(&[earlier, later])
        .unwrap();
    assert_eq!(
        (f64_value(&earlier_output), f64_value(&failing_output)),
        (20.0, 36.0)
    );
}

#[test]
fn register_batch_stages_every_write_before_committing_in_plan_order() {
    let mut plan = ReactivePlan::new();
    let stages = Rc::new(RefCell::new(Vec::new()));
    let x = ValueCell::from_exact(1.0).unwrap();
    let y = ValueCell::from_exact(2.0).unwrap();
    let x_alias = x.clone();
    let y_alias = y.clone();
    let first = register(
        &mut plan,
        "x",
        x.clone(),
        vec![x.clone(), y.clone()],
        stages.clone(),
        false,
    );
    let second = register(
        &mut plan,
        "y",
        y.clone(),
        vec![y.clone(), x.clone()],
        stages.clone(),
        false,
    );

    let outcome = plan
        .commit_pending_registers(&[second, first, second])
        .unwrap();

    assert_eq!(&*stages.borrow(), &["x", "y"]);
    assert_eq!(outcome.staged_nodes, vec![first, second]);
    assert_eq!(outcome.committed_nodes, vec![first, second]);
    // Both kernels read old published state. Sequential publication would
    // incorrectly make y observe x=3 and produce 5.
    assert_eq!((f64_value(&x), f64_value(&y)), (3.0, 3.0));
    assert!(x.same_logical_cell(&x_alias));
    assert!(y.same_logical_cell(&y_alias));
}

#[test]
fn register_batch_is_atomic_when_a_later_kernel_cannot_stage() {
    let mut plan = ReactivePlan::new();
    let stages = Rc::new(RefCell::new(Vec::new()));
    let x = ValueCell::from_exact(1.0).unwrap();
    let y = ValueCell::from_exact(2.0).unwrap();
    let source = ValueCell::from_exact(4.0).unwrap();
    let x_version = x.published_version();
    let y_version = y.published_version();
    let first = register(
        &mut plan,
        "x",
        x.clone(),
        vec![source],
        stages.clone(),
        false,
    );
    let second = register(&mut plan, "y", y.clone(), Vec::new(), stages.clone(), true);

    let error = plan.commit_pending_registers(&[first, second]).unwrap_err();

    assert!(error.simple_message().contains("y failed to stage"));
    assert_eq!(&*stages.borrow(), &["x", "y"]);
    assert_eq!((f64_value(&x), f64_value(&y)), (1.0, 2.0));
    assert_eq!(x.published_version(), x_version);
    assert_eq!(y.published_version(), y_version);
}

struct CountingCombinational(Rc<RefCell<usize>>);

impl MechFunctionImpl for CountingCombinational {
    fn solve_managed(
        &self,
        _frame: &mut crate::KernelMemoryFrame<'_>,
        _services: &mut dyn crate::MechExecutionServices,
    ) -> MResult<ReactiveSolveStatus> {
        *self.0.borrow_mut() += 1;
        Ok(ReactiveSolveStatus::Changed)
    }

    fn to_string(&self) -> String {
        "combinational".into()
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for CountingCombinational {
    fn compile(&self, _context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

#[test]
fn reactive_register_commit_rejects_missing_node_before_staging() {
    let mut plan = ReactivePlan::new();
    let stages = Rc::new(RefCell::new(Vec::new()));
    let output = ValueCell::from_exact(1.0).unwrap();
    let node = register(
        &mut plan,
        "present",
        output.clone(),
        Vec::new(),
        stages.clone(),
        false,
    );

    let error = plan
        .commit_pending_registers(&[node, plan.nodes.len() + 100])
        .unwrap_err();

    assert_eq!(error.kind_name(), "ReactiveRegisterNodeNotFound");
    assert!(stages.borrow().is_empty());
    assert_eq!(f64_value(&output), 1.0);
}

#[test]
fn reactive_register_commit_rejects_combinational_node_before_staging() {
    let mut plan = ReactivePlan::new();
    let stages = Rc::new(RefCell::new(Vec::new()));
    let register_node = register(
        &mut plan,
        "register",
        ValueCell::from_exact(1.0).unwrap(),
        Vec::new(),
        stages.clone(),
        false,
    );
    let solves = Rc::new(RefCell::new(0));
    let combinational = plan
        .push(crate::function::test_planned_instance(
            Box::new(CountingCombinational(solves.clone())),
            FunctionInvocation::nullary(ValueCell::unit()),
        ))
        .unwrap();

    let error = plan
        .commit_pending_registers(&[register_node, combinational])
        .unwrap_err();

    assert_eq!(error.kind_name(), "ReactiveRegisterNodeKind");
    assert!(stages.borrow().is_empty());
    assert_eq!(*solves.borrow(), 0);
}

#[test]
fn reactive_register_commit_rejects_overlapping_outputs_before_staging() {
    let mut plan = ReactivePlan::new();
    let stages = Rc::new(RefCell::new(Vec::new()));
    let output = ValueCell::from_exact(1.0).unwrap();
    let first = register(
        &mut plan,
        "first",
        output.clone(),
        Vec::new(),
        stages.clone(),
        false,
    );
    let second = register(
        &mut plan,
        "second",
        output.clone(),
        Vec::new(),
        stages.clone(),
        false,
    );

    let error = plan.commit_pending_registers(&[first, second]).unwrap_err();

    assert_eq!(error.kind_name(), "ReactiveRegisterOutputConflict");
    assert!(stages.borrow().is_empty());
    assert_eq!(f64_value(&output), 1.0);
}

#[test]
fn reactive_register_commit_returns_ordered_unique_dirty_cells() {
    let mut plan = ReactivePlan::new();
    let stages = Rc::new(RefCell::new(Vec::new()));
    let first_output = ValueCell::from_exact(1.0).unwrap();
    let second_output = ValueCell::from_exact(2.0).unwrap();
    let first_cell = first_output.reactive_cell_id();
    let second_cell = second_output.reactive_cell_id();
    let first = register(
        &mut plan,
        "first",
        first_output,
        Vec::new(),
        stages.clone(),
        false,
    );
    let second = register(
        &mut plan,
        "second",
        second_output,
        Vec::new(),
        stages,
        false,
    );

    let outcome = plan
        .commit_pending_registers(&[second, first, second])
        .unwrap();

    assert_eq!(outcome.dirty_cells, vec![first_cell, second_cell]);
    assert_eq!(outcome.committed_nodes, vec![first, second]);
}

#[test]
fn reactive_register_commit_does_not_execute_downstream_nodes() {
    let mut plan = ReactivePlan::new();
    let stages = Rc::new(RefCell::new(Vec::new()));
    let output = ValueCell::from_exact(1.0).unwrap();
    let output_cell = output.reactive_cell_id();
    let register_node = register(
        &mut plan,
        "register",
        output,
        vec![ValueCell::from_exact(2.0).unwrap()],
        stages,
        false,
    );
    let downstream_solves = Rc::new(RefCell::new(0));
    plan.push(crate::function::test_planned_instance(
        Box::new(CountingCombinational(downstream_solves.clone())),
        FunctionInvocation::nullary(ValueCell::unit()),
    ))
    .unwrap();

    let outcome = plan.commit_pending_registers(&[register_node]).unwrap();

    assert_eq!(outcome.dirty_cells, vec![output_cell]);
    assert_eq!(*downstream_solves.borrow(), 0);
}

struct RegisterWithoutManagedWrite {
    output: ManagedPort<f64>,
    solves: Rc<RefCell<usize>>,
}

impl MechFunctionImpl for RegisterWithoutManagedWrite {
    fn solve_managed(
        &self,
        _frame: &mut crate::KernelMemoryFrame<'_>,
        _services: &mut dyn crate::MechExecutionServices,
    ) -> MResult<ReactiveSolveStatus> {
        *self.solves.borrow_mut() += 1;
        Ok(ReactiveSolveStatus::Changed)
    }

    fn reactive_node_kind(&self) -> ReactiveNodeKind {
        ReactiveNodeKind::Register
    }

    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_cell(self.output.cell()))
    }

    fn to_string(&self) -> String {
        "missing managed write".into()
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for RegisterWithoutManagedWrite {
    fn compile(&self, _context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

#[test]
fn reactive_register_commit_rejects_uninitialized_stage() {
    let mut plan = ReactivePlan::new();
    let output = ValueCell::from_exact(1.0).unwrap();
    let invocation = FunctionInvocation::nullary(output.clone());
    let port = invocation.output().try_managed_element::<f64>().unwrap();
    let solves = Rc::new(RefCell::new(0));
    let node = plan
        .register_instance_with_activation(
            crate::function::test_planned_instance(
                Box::new(RegisterWithoutManagedWrite {
                    output: port,
                    solves: solves.clone(),
                }),
                invocation,
            ),
            None,
        )
        .unwrap();

    let error = plan.commit_pending_registers(&[node]).unwrap_err();

    assert_eq!(error.kind_name(), "MemoryRuntimeError");
    assert!(error.kind_message().contains("UninitializedAccess"));
    assert_eq!((*solves.borrow(), f64_value(&output)), (1, 1.0));
}

#[test]
fn reactive_register_commit_empty_pending_set_is_noop() {
    let mut plan = ReactivePlan::new();
    assert_eq!(
        plan.commit_pending_registers(&[]).unwrap(),
        ReactiveRegisterCommitOutcome::default(),
    );
}
