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
use std::{cell::RefCell, rc::Rc};

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
