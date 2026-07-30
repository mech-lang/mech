#[cfg(feature = "compiler")]
use super::super::MechFunctionCompiler;
use super::super::{
    MechFunctionImpl, ReactiveNodeId, ReactiveNodeKind, ReactivePlan, ReactiveRegisterCommit,
    ReactiveRegisterCommitOutcome, reactive_register_sealed,
};
#[cfg(feature = "compiler")]
use crate::{BytecodeCompilerContext, Register};
use crate::{GenericError, MResult, MechError, ReactiveCellId, Ref, ToValue, Value};
use std::{cell::RefCell, rc::Rc};

struct RegisterStageTestCommit {
    label: &'static str,
    sink: Ref<f64>,
    next: f64,
    output_cells: Vec<ReactiveCellId>,
    commit_count: Rc<RefCell<usize>>,
    commit_log: Rc<RefCell<Vec<&'static str>>>,
    total_commit_count: Rc<RefCell<usize>>,
}
impl reactive_register_sealed::Sealed for RegisterStageTestCommit {}
impl ReactiveRegisterCommit for RegisterStageTestCommit {
    fn output_cells(&self) -> &[ReactiveCellId] {
        &self.output_cells
    }
    fn commit(self: Box<Self>) {
        *self.sink.borrow_mut() = self.next;
        *self.commit_count.borrow_mut() += 1;
        *self.total_commit_count.borrow_mut() += 1;
        self.commit_log.borrow_mut().push(self.label);
    }
}
struct RegisterStageTestFunction {
    label: &'static str,
    sink: Ref<f64>,
    sources: Vec<Ref<f64>>,
    stage_count: Rc<RefCell<usize>>,
    solve_count: Rc<RefCell<usize>>,
    commit_count: Rc<RefCell<usize>>,
    stage_log: Rc<RefCell<Vec<&'static str>>>,
    commit_log: Rc<RefCell<Vec<&'static str>>>,
    total_commit_count: Rc<RefCell<usize>>,
    commit_counts_observed_during_stage: Rc<RefCell<Vec<usize>>>,
    fail_stage: bool,
    mismatch_outputs: Option<Vec<ReactiveCellId>>,
}
impl MechFunctionImpl for RegisterStageTestFunction {
    fn solve(&self) {
        *self.solve_count.borrow_mut() += 1;
    }
    fn out(&self) -> Value {
        self.sink.to_value()
    }
    fn reactive_node_kind(&self) -> ReactiveNodeKind {
        ReactiveNodeKind::Register
    }
    fn stage_register(&self) -> MResult<Box<dyn ReactiveRegisterCommit>> {
        *self.stage_count.borrow_mut() += 1;
        self.stage_log.borrow_mut().push(self.label);
        let total = *self.total_commit_count.borrow();
        self.commit_counts_observed_during_stage
            .borrow_mut()
            .push(total);
        if self.fail_stage {
            return Err(MechError::new(
                GenericError {
                    msg: self.label.to_string(),
                },
                None,
            ));
        }
        let next = self
            .sources
            .iter()
            .map(|source| *source.borrow())
            .sum::<f64>();
        let output_cells = self
            .mismatch_outputs
            .clone()
            .unwrap_or_else(|| self.reactive_output_cell_ids());
        Ok(Box::new(RegisterStageTestCommit {
            label: self.label,
            sink: self.sink.clone(),
            next,
            output_cells,
            commit_count: self.commit_count.clone(),
            commit_log: self.commit_log.clone(),
            total_commit_count: self.total_commit_count.clone(),
        }))
    }
    fn to_string(&self) -> String {
        self.label.to_string()
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(self.reactive_output_values())
    }
}
#[cfg(feature = "compiler")]
impl MechFunctionCompiler for RegisterStageTestFunction {
    fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}
struct Fixture {
    node: ReactiveNodeId,
    sink: Ref<f64>,
    stage: Rc<RefCell<usize>>,
    solve: Rc<RefCell<usize>>,
    commit: Rc<RefCell<usize>>,
}
fn add(
    plan: &mut ReactivePlan,
    label: &'static str,
    sink: Ref<f64>,
    sources: Vec<Ref<f64>>,
    stage_log: Rc<RefCell<Vec<&'static str>>>,
    commit_log: Rc<RefCell<Vec<&'static str>>>,
    total: Rc<RefCell<usize>>,
    fail: bool,
    mismatch: Option<Vec<ReactiveCellId>>,
) -> Fixture {
    let stage = Rc::new(RefCell::new(0));
    let solve = Rc::new(RefCell::new(0));
    let commit = Rc::new(RefCell::new(0));
    let observed = Rc::new(RefCell::new(vec![]));
    let node = plan
        .register(
            Box::new(RegisterStageTestFunction {
                label,
                sink: sink.clone(),
                sources,
                stage_count: stage.clone(),
                solve_count: solve.clone(),
                commit_count: commit.clone(),
                stage_log,
                commit_log,
                total_commit_count: total,
                commit_counts_observed_during_stage: observed,
                fail_stage: fail,
                mismatch_outputs: mismatch,
            }),
            &[],
        )
        .unwrap();
    Fixture {
        node,
        sink,
        stage,
        solve,
        commit,
    }
}
fn shared() -> (
    Rc<RefCell<Vec<&'static str>>>,
    Rc<RefCell<Vec<&'static str>>>,
    Rc<RefCell<usize>>,
) {
    (
        Rc::new(RefCell::new(vec![])),
        Rc::new(RefCell::new(vec![])),
        Rc::new(RefCell::new(0)),
    )
}

struct RegisterWithoutStaging {
    sink: Ref<f64>,
    solves: Rc<RefCell<usize>>,
}
impl MechFunctionImpl for RegisterWithoutStaging {
    fn solve(&self) {
        *self.solves.borrow_mut() += 1;
    }
    fn out(&self) -> Value {
        self.sink.to_value()
    }
    fn reactive_node_kind(&self) -> ReactiveNodeKind {
        ReactiveNodeKind::Register
    }
    fn to_string(&self) -> String {
        "unsupported".into()
    }
    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(self.reactive_output_values())
    }
}
#[cfg(feature = "compiler")]
impl MechFunctionCompiler for RegisterWithoutStaging {
    fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

#[test]
fn reactive_register_commit_stages_all_before_any_commit() {
    let mut p = ReactivePlan::new();
    let (xl, cl, t) = shared();
    let x = Ref::new(1.);
    let y = Ref::new(2.);
    let a = add(
        &mut p,
        "X",
        x.clone(),
        vec![x.clone(), y.clone()],
        xl.clone(),
        cl.clone(),
        t.clone(),
        false,
        None,
    );
    let b = add(
        &mut p,
        "Y",
        y.clone(),
        vec![y.clone(), x.clone()],
        xl.clone(),
        cl.clone(),
        t,
        false,
        None,
    );
    let o = p.commit_pending_registers(&[b.node, a.node]).unwrap();
    assert_eq!((*x.borrow(), *y.borrow()), (3., 3.));
    assert_eq!(o.staged_nodes, vec![a.node, b.node]);
    assert_eq!(*xl.borrow(), vec!["X", "Y"]);
    assert_eq!(*cl.borrow(), vec!["X", "Y"]);
    assert_eq!(
        (
            *a.solve.borrow(),
            *b.solve.borrow(),
            *a.stage.borrow(),
            *b.stage.borrow(),
            *a.commit.borrow(),
            *b.commit.borrow()
        ),
        (0, 0, 1, 1, 1, 1)
    );
}

#[test]
fn reactive_register_commit_deduplicates_and_orders_pending_nodes() {
    let mut p = ReactivePlan::new();
    let (l, c, t) = shared();
    let a = add(
        &mut p,
        "A",
        Ref::new(0.),
        vec![],
        l.clone(),
        c.clone(),
        t.clone(),
        false,
        None,
    );
    let b = add(
        &mut p,
        "B",
        Ref::new(0.),
        vec![],
        l.clone(),
        c.clone(),
        t.clone(),
        false,
        None,
    );
    let d = add(
        &mut p,
        "C",
        Ref::new(0.),
        vec![],
        l.clone(),
        c.clone(),
        t,
        false,
        None,
    );
    let o = p
        .commit_pending_registers(&[d.node, a.node, b.node, a.node, d.node, b.node])
        .unwrap();
    assert_eq!(o.staged_nodes, vec![a.node, b.node, d.node]);
    assert_eq!(*l.borrow(), vec!["A", "B", "C"]);
    assert_eq!(*c.borrow(), vec!["A", "B", "C"]);
    for f in [&a, &b, &d] {
        assert_eq!(
            (*f.stage.borrow(), *f.commit.borrow(), *f.solve.borrow()),
            (1, 1, 0)
        );
    }
}

#[test]
fn reactive_register_commit_is_atomic_on_stage_error() {
    let mut p = ReactivePlan::new();
    let (l, c, t) = shared();
    let a = add(
        &mut p,
        "A",
        Ref::new(1.),
        vec![Ref::new(4.)],
        l.clone(),
        c.clone(),
        t.clone(),
        false,
        None,
    );
    let b = add(
        &mut p,
        "B",
        Ref::new(2.),
        vec![],
        l,
        c,
        t.clone(),
        true,
        None,
    );
    let e = p.commit_pending_registers(&[a.node, b.node]).unwrap_err();
    assert!(e.kind_message().contains("B"));
    assert_eq!(
        (
            *a.sink.borrow(),
            *b.sink.borrow(),
            *a.commit.borrow(),
            *b.commit.borrow(),
            *t.borrow()
        ),
        (1., 2., 0, 0, 0)
    );
}

#[test]
fn reactive_register_commit_rejects_missing_node_without_staging() {
    let mut p = ReactivePlan::new();
    let (l, c, t) = shared();
    let a = add(&mut p, "A", Ref::new(1.), vec![], l, c, t, false, None);
    let missing = p.nodes.len() + 100;
    let e = p.commit_pending_registers(&[a.node, missing]).unwrap_err();
    assert_eq!(e.kind_name(), "ReactiveRegisterNodeNotFound");
    assert_eq!(
        (
            *a.stage.borrow(),
            *a.commit.borrow(),
            *a.solve.borrow(),
            *a.sink.borrow()
        ),
        (0, 0, 0, 1.)
    );
}

#[test]
fn reactive_register_commit_rejects_combinational_node_without_staging() {
    let mut p = ReactivePlan::new();
    let (l, c, t) = shared();
    let a = add(&mut p, "A", Ref::new(1.), vec![], l, c, t, false, None);
    struct Combinational;
    impl MechFunctionImpl for Combinational {
        fn solve(&self) {}
        fn out(&self) -> Value {
            Value::Empty
        }
        fn to_string(&self) -> String {
            "C".into()
        }
        fn transaction_state_values(&self) -> MResult<Vec<Value>> {
            Ok(self.reactive_output_values())
        }
    }
    #[cfg(feature = "compiler")]
    impl MechFunctionCompiler for Combinational {
        fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            Ok(0)
        }
    }
    let combinational = p.push(Box::new(Combinational));
    let e = p
        .commit_pending_registers(&[a.node, combinational])
        .unwrap_err();
    assert_eq!(e.kind_name(), "ReactiveRegisterNodeKind");
    assert_eq!(
        (*a.stage.borrow(), *a.commit.borrow(), *a.solve.borrow()),
        (0, 0, 0)
    );
}

#[test]
fn reactive_register_commit_rejects_overlapping_outputs_before_staging() {
    let mut p = ReactivePlan::new();
    let (l, c, t) = shared();
    let sink = Ref::new(1.);
    let a = add(
        &mut p,
        "A",
        sink.clone(),
        vec![],
        l.clone(),
        c.clone(),
        t.clone(),
        false,
        None,
    );
    let b = add(&mut p, "B", sink.clone(), vec![], l, c, t, false, None);
    let e = p.commit_pending_registers(&[a.node, b.node]).unwrap_err();
    assert_eq!(e.kind_name(), "ReactiveRegisterOutputConflict");
    assert_eq!(
        (
            *a.stage.borrow(),
            *b.stage.borrow(),
            *a.commit.borrow(),
            *b.commit.borrow(),
            *sink.borrow()
        ),
        (0, 0, 0, 0, 1.)
    );
}

#[test]
fn reactive_register_commit_rejects_staged_output_mismatch_without_commit() {
    let mut p = ReactivePlan::new();
    let (l, c, t) = shared();
    let sink = Ref::new(1.);
    let other = Ref::new(2.).to_value().reactive_root_cell_ids();
    let a = add(
        &mut p,
        "A",
        sink.clone(),
        vec![],
        l,
        c,
        t,
        false,
        Some(other),
    );
    let e = p.commit_pending_registers(&[a.node]).unwrap_err();
    assert_eq!(e.kind_name(), "ReactiveRegisterStagedOutputMismatch");
    assert_eq!(
        (
            *a.stage.borrow(),
            *a.commit.borrow(),
            *a.solve.borrow(),
            *sink.borrow()
        ),
        (1, 0, 0, 1.)
    );
}

#[test]
fn reactive_register_commit_returns_ordered_unique_dirty_cells() {
    let mut p = ReactivePlan::new();
    let (l, c, t) = shared();
    let a = add(
        &mut p,
        "A",
        Ref::new(1.),
        vec![],
        l.clone(),
        c.clone(),
        t.clone(),
        false,
        None,
    );
    let b = add(&mut p, "B", Ref::new(2.), vec![], l, c, t, false, None);
    let cells = vec![p.nodes[a.node].outputs[0], p.nodes[b.node].outputs[0]];
    let o = p
        .commit_pending_registers(&[b.node, a.node, b.node])
        .unwrap();
    assert_eq!(o.dirty_cells, cells);
    assert_eq!(o.committed_nodes, vec![a.node, b.node]);
}

#[test]
fn reactive_register_commit_does_not_execute_downstream_nodes() {
    let mut p = ReactivePlan::new();
    let (l, c, t) = shared();
    let a = add(
        &mut p,
        "A",
        Ref::new(1.),
        vec![Ref::new(2.)],
        l,
        c,
        t,
        false,
        None,
    );
    let downstream = Rc::new(RefCell::new(0));
    struct C(Rc<RefCell<usize>>);
    impl MechFunctionImpl for C {
        fn solve(&self) {
            *self.0.borrow_mut() += 1;
        }
        fn out(&self) -> Value {
            Value::Empty
        }
        fn to_string(&self) -> String {
            "C".into()
        }
        fn transaction_state_values(&self) -> MResult<Vec<Value>> {
            Ok(self.reactive_output_values())
        }
    }
    #[cfg(feature = "compiler")]
    impl MechFunctionCompiler for C {
        fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            Ok(0)
        }
    }
    p.push(Box::new(C(downstream.clone())));
    let o = p.commit_pending_registers(&[a.node]).unwrap();
    assert_eq!(*a.commit.borrow(), 1);
    assert!(o.dirty_cells.contains(&p.nodes[a.node].outputs[0]));
    assert_eq!(*downstream.borrow(), 0);
}

#[test]
fn reactive_register_commit_rejects_unsupported_register_staging() {
    let mut p = ReactivePlan::new();
    let sink = Ref::new(1.);
    let solves = Rc::new(RefCell::new(0));
    let n = p
        .register(
            Box::new(RegisterWithoutStaging {
                sink: sink.clone(),
                solves: solves.clone(),
            }),
            &[],
        )
        .unwrap();
    let e = p.commit_pending_registers(&[n]).unwrap_err();
    assert_eq!(e.kind_name(), "ReactiveRegisterStagingUnsupported");
    assert_eq!((*solves.borrow(), *sink.borrow()), (0, 1.));
}

#[test]
fn reactive_register_commit_empty_pending_set_is_noop() {
    let mut p = ReactivePlan::new();
    let (l, c, t) = shared();
    let a = add(&mut p, "A", Ref::new(1.), vec![], l, c, t, false, None);
    assert_eq!(
        p.commit_pending_registers(&[]).unwrap(),
        ReactiveRegisterCommitOutcome::default()
    );
    assert_eq!(
        (*a.stage.borrow(), *a.solve.borrow(), *a.commit.borrow()),
        (0, 0, 0)
    );
}
