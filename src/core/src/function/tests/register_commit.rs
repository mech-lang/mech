#[cfg(feature = "semantic-compiler")]
use super::super::MechFunctionCompiler;
use super::super::{
    FunctionInstance, FunctionInvocation, MechFunctionImpl, ReactiveNodeId, ReactiveNodeKind,
    ReactivePlan, ReactiveRegisterCommit, ReactiveRegisterCommitOutcome, reactive_register_sealed,
};
#[cfg(feature = "semantic-compiler")]
use crate::{BytecodeCompilerContext, Register};
use crate::{CanonicalCellId, FunctionStatePort, GenericError, MResult, MechError, Ref, ValueCell};
use std::{cell::RefCell, rc::Rc};

struct StagedWrite {
    label: &'static str,
    sink: Ref<f64>,
    next: f64,
    outputs: Vec<CanonicalCellId>,
    commits: Rc<RefCell<Vec<&'static str>>>,
}

impl reactive_register_sealed::Sealed for StagedWrite {}

impl ReactiveRegisterCommit for StagedWrite {
    fn output_cells(&self) -> &[CanonicalCellId] {
        &self.outputs
    }

    fn commit(self: Box<Self>) {
        *self.sink.borrow_mut() = self.next;
        self.commits.borrow_mut().push(self.label);
    }
}

struct RegisterFunction {
    label: &'static str,
    sink: Ref<f64>,
    sources: Vec<Ref<f64>>,
    stages: Rc<RefCell<Vec<(&'static str, usize)>>>,
    commits: Rc<RefCell<Vec<&'static str>>>,
    fail: bool,
    staged_outputs: Option<Vec<CanonicalCellId>>,
}

impl MechFunctionImpl for RegisterFunction {
    fn solve_result(&self) -> MResult<()> {
        panic!("register commits must stage rather than solve directly")
    }

    fn stage_register(&self) -> MResult<Box<dyn ReactiveRegisterCommit>> {
        let committed = self.commits.borrow().len();
        self.stages.borrow_mut().push((self.label, committed));
        if self.fail {
            return Err(MechError::new(
                GenericError {
                    msg: format!("{} failed to stage", self.label),
                },
                None,
            ));
        }
        Ok(Box::new(StagedWrite {
            label: self.label,
            sink: self.sink.clone(),
            next: self.sources.iter().map(|source| *source.borrow()).sum(),
            outputs: self
                .staged_outputs
                .clone()
                .unwrap_or_else(|| self.reactive_output_cell_ids()),
            commits: self.commits.clone(),
        }))
    }

    fn reactive_node_kind(&self) -> ReactiveNodeKind {
        ReactiveNodeKind::Register
    }

    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_ref(&self.sink))
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
    stages: Rc<RefCell<Vec<(&'static str, usize)>>>,
    commits: Rc<RefCell<Vec<&'static str>>>,
    fail: bool,
) -> ReactiveNodeId {
    register_with_staged_outputs(
        plan,
        label,
        output,
        sources,
        stages,
        commits,
        RegisterControls {
            fail,
            staged_outputs: None,
        },
    )
}

struct RegisterControls {
    fail: bool,
    staged_outputs: Option<Vec<CanonicalCellId>>,
}

fn register_with_staged_outputs(
    plan: &mut ReactivePlan,
    label: &'static str,
    output: ValueCell,
    sources: Vec<ValueCell>,
    stages: Rc<RefCell<Vec<(&'static str, usize)>>>,
    commits: Rc<RefCell<Vec<&'static str>>>,
    controls: RegisterControls,
) -> ReactiveNodeId {
    let sink = output.try_ref::<f64>().unwrap();
    let source_refs = sources
        .iter()
        .map(|source| source.try_ref::<f64>().unwrap())
        .collect();
    plan.register_instance_with_activation(
        FunctionInstance::new(
            Box::new(RegisterFunction {
                label,
                sink,
                sources: source_refs,
                stages,
                commits,
                fail: controls.fail,
                staged_outputs: controls.staged_outputs,
            }),
            FunctionInvocation::variadic(output, sources.into_boxed_slice()),
        ),
        None,
    )
    .unwrap()
}

#[test]
fn register_batch_stages_every_write_before_committing_in_plan_order() {
    let mut plan = ReactivePlan::new();
    let stages = Rc::new(RefCell::new(Vec::new()));
    let commits = Rc::new(RefCell::new(Vec::new()));
    let x_cell = ValueCell::from_exact(1.0).unwrap();
    let y_cell = ValueCell::from_exact(2.0).unwrap();
    let x = x_cell.try_ref::<f64>().unwrap();
    let y = y_cell.try_ref::<f64>().unwrap();
    let x_alias = x.clone();
    let y_alias = y.clone();
    let first = register(
        &mut plan,
        "x",
        x_cell.clone(),
        vec![x_cell.clone(), y_cell.clone()],
        stages.clone(),
        commits.clone(),
        false,
    );
    let second = register(
        &mut plan,
        "y",
        y_cell.clone(),
        vec![y_cell, x_cell],
        stages.clone(),
        commits.clone(),
        false,
    );

    let outcome = plan
        .commit_pending_registers(&[second, first, second])
        .unwrap();

    assert_eq!(&*stages.borrow(), &[("x", 0), ("y", 0)]);
    assert_eq!(&*commits.borrow(), &["x", "y"]);
    assert_eq!(outcome.staged_nodes, vec![first, second]);
    assert_eq!(outcome.committed_nodes, vec![first, second]);
    assert_eq!((*x.borrow(), *y.borrow()), (3.0, 3.0));
    assert!(x.same_handle(&x_alias));
    assert!(y.same_handle(&y_alias));
}

#[test]
fn register_batch_is_atomic_when_a_later_write_cannot_stage() {
    let mut plan = ReactivePlan::new();
    let stages = Rc::new(RefCell::new(Vec::new()));
    let commits = Rc::new(RefCell::new(Vec::new()));
    let x_cell = ValueCell::from_exact(1.0).unwrap();
    let y_cell = ValueCell::from_exact(2.0).unwrap();
    let source = ValueCell::from_exact(4.0).unwrap();
    let x = x_cell.try_ref::<f64>().unwrap();
    let y = y_cell.try_ref::<f64>().unwrap();
    let first = register(
        &mut plan,
        "x",
        x_cell,
        vec![source],
        stages.clone(),
        commits.clone(),
        false,
    );
    let second = register(
        &mut plan,
        "y",
        y_cell,
        Vec::new(),
        stages.clone(),
        commits.clone(),
        true,
    );

    let error = plan.commit_pending_registers(&[first, second]).unwrap_err();

    assert!(error.simple_message().contains("y failed to stage"));
    assert_eq!(&*stages.borrow(), &[("x", 0), ("y", 0)]);
    assert!(commits.borrow().is_empty());
    assert_eq!((*x.borrow(), *y.borrow()), (1.0, 2.0));
}

struct RegisterWithoutStaging {
    sink: Ref<f64>,
    solves: Rc<RefCell<usize>>,
}

impl MechFunctionImpl for RegisterWithoutStaging {
    fn solve_result(&self) -> MResult<()> {
        *self.solves.borrow_mut() += 1;
        Ok(())
    }

    fn reactive_node_kind(&self) -> ReactiveNodeKind {
        ReactiveNodeKind::Register
    }

    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_ref(&self.sink))
    }

    fn to_string(&self) -> String {
        "unsupported".into()
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for RegisterWithoutStaging {
    fn compile(&self, _context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

struct CountingCombinational(Rc<RefCell<usize>>);

impl MechFunctionImpl for CountingCombinational {
    fn solve_result(&self) -> MResult<()> {
        *self.0.borrow_mut() += 1;
        Ok(())
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
fn reactive_register_commit_rejects_missing_node_without_staging() {
    let mut plan = ReactivePlan::new();
    let stages = Rc::new(RefCell::new(Vec::new()));
    let commits = Rc::new(RefCell::new(Vec::new()));
    let output = ValueCell::from_exact(1.0).unwrap();
    let sink = output.try_ref::<f64>().unwrap();
    let node = register(
        &mut plan,
        "present",
        output,
        Vec::new(),
        stages.clone(),
        commits.clone(),
        false,
    );

    let error = plan
        .commit_pending_registers(&[node, plan.nodes.len() + 100])
        .unwrap_err();

    assert_eq!(error.kind_name(), "ReactiveRegisterNodeNotFound");
    assert!(stages.borrow().is_empty());
    assert!(commits.borrow().is_empty());
    assert_eq!(*sink.borrow(), 1.0);
}

#[test]
fn reactive_register_commit_rejects_combinational_node_without_staging() {
    let mut plan = ReactivePlan::new();
    let stages = Rc::new(RefCell::new(Vec::new()));
    let commits = Rc::new(RefCell::new(Vec::new()));
    let output = ValueCell::from_exact(1.0).unwrap();
    let register_node = register(
        &mut plan,
        "register",
        output,
        Vec::new(),
        stages.clone(),
        commits.clone(),
        false,
    );
    let solves = Rc::new(RefCell::new(0));
    let combinational = plan.push(Box::new(CountingCombinational(solves.clone())));

    let error = plan
        .commit_pending_registers(&[register_node, combinational])
        .unwrap_err();

    assert_eq!(error.kind_name(), "ReactiveRegisterNodeKind");
    assert!(stages.borrow().is_empty());
    assert!(commits.borrow().is_empty());
    assert_eq!(*solves.borrow(), 0);
}

#[test]
fn reactive_register_commit_rejects_overlapping_outputs_before_staging() {
    let mut plan = ReactivePlan::new();
    let stages = Rc::new(RefCell::new(Vec::new()));
    let commits = Rc::new(RefCell::new(Vec::new()));
    let output = ValueCell::from_exact(1.0).unwrap();
    let sink = output.try_ref::<f64>().unwrap();
    let first = register(
        &mut plan,
        "first",
        output.clone(),
        Vec::new(),
        stages.clone(),
        commits.clone(),
        false,
    );
    let second = register(
        &mut plan,
        "second",
        output,
        Vec::new(),
        stages.clone(),
        commits.clone(),
        false,
    );

    let error = plan.commit_pending_registers(&[first, second]).unwrap_err();

    assert_eq!(error.kind_name(), "ReactiveRegisterOutputConflict");
    assert!(stages.borrow().is_empty());
    assert!(commits.borrow().is_empty());
    assert_eq!(*sink.borrow(), 1.0);
}

#[test]
fn reactive_register_commit_rejects_staged_output_mismatch_without_commit() {
    let mut plan = ReactivePlan::new();
    let stages = Rc::new(RefCell::new(Vec::new()));
    let commits = Rc::new(RefCell::new(Vec::new()));
    let output = ValueCell::from_exact(1.0).unwrap();
    let sink = output.try_ref::<f64>().unwrap();
    let other = ValueCell::from_exact(2.0).unwrap().reactive_cell_id();
    let node = register_with_staged_outputs(
        &mut plan,
        "mismatch",
        output,
        Vec::new(),
        stages.clone(),
        commits.clone(),
        RegisterControls {
            fail: false,
            staged_outputs: Some(vec![other]),
        },
    );

    let error = plan.commit_pending_registers(&[node]).unwrap_err();

    assert_eq!(error.kind_name(), "ReactiveRegisterStagedOutputMismatch");
    assert_eq!(stages.borrow().as_slice(), &[("mismatch", 0)]);
    assert!(commits.borrow().is_empty());
    assert_eq!(*sink.borrow(), 1.0);
}

#[test]
fn reactive_register_commit_returns_ordered_unique_dirty_cells() {
    let mut plan = ReactivePlan::new();
    let stages = Rc::new(RefCell::new(Vec::new()));
    let commits = Rc::new(RefCell::new(Vec::new()));
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
        commits.clone(),
        false,
    );
    let second = register(
        &mut plan,
        "second",
        second_output,
        Vec::new(),
        stages,
        commits,
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
    let commits = Rc::new(RefCell::new(Vec::new()));
    let output = ValueCell::from_exact(1.0).unwrap();
    let output_cell = output.reactive_cell_id();
    let register_node = register(
        &mut plan,
        "register",
        output,
        vec![ValueCell::from_exact(2.0).unwrap()],
        stages,
        commits.clone(),
        false,
    );
    let downstream_solves = Rc::new(RefCell::new(0));
    plan.push(Box::new(CountingCombinational(downstream_solves.clone())));

    let outcome = plan.commit_pending_registers(&[register_node]).unwrap();

    assert_eq!(commits.borrow().as_slice(), &["register"]);
    assert_eq!(outcome.dirty_cells, vec![output_cell]);
    assert_eq!(*downstream_solves.borrow(), 0);
}

#[test]
fn reactive_register_commit_rejects_unsupported_register_staging() {
    let mut plan = ReactivePlan::new();
    let output = ValueCell::from_exact(1.0).unwrap();
    let sink = output.try_ref::<f64>().unwrap();
    let solves = Rc::new(RefCell::new(0));
    let node = plan
        .register_instance_with_activation(
            FunctionInstance::new(
                Box::new(RegisterWithoutStaging {
                    sink: sink.clone(),
                    solves: solves.clone(),
                }),
                FunctionInvocation::variadic(output, Box::new([])),
            ),
            None,
        )
        .unwrap();

    let error = plan.commit_pending_registers(&[node]).unwrap_err();

    assert_eq!(error.kind_name(), "ReactiveRegisterStagingUnsupported");
    assert_eq!((*solves.borrow(), *sink.borrow()), (0, 1.0));
}

#[test]
fn reactive_register_commit_empty_pending_set_is_noop() {
    let mut plan = ReactivePlan::new();
    assert_eq!(
        plan.commit_pending_registers(&[]).unwrap(),
        ReactiveRegisterCommitOutcome::default(),
    );
}
