#[cfg(feature = "compiler")]
use super::super::MechFunctionCompiler;
use super::super::{
    GuardFunctionSafety, MechFunction, MechFunctionImpl, NativeFunctionCompiler,
    ReactiveDependencyKind, ReactiveDependencyScope, ReactiveNodeId, ReactiveNodeKind,
    ReactivePlan, ReactiveRegisterCommit, reactive_register_sealed,
};
#[cfg(all(feature = "set", feature = "f64"))]
use crate::MechSet;
#[cfg(feature = "compiler")]
use crate::{CompileCtx, Register};
use crate::{GenericError, MResult, MechError, ReactiveCellId, Ref, ToValue, Value, ValueKind};
use std::{cell::RefCell, rc::Rc};

pub(super) struct PureStaticTestCompiler;
impl NativeFunctionCompiler for PureStaticTestCompiler {
    fn compile(&self, _arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
        unreachable!("safety metadata test must not compile the function")
    }

    fn guard_safety(&self) -> GuardFunctionSafety {
        GuardFunctionSafety::PureStatic
    }
}

pub(super) struct TestFunction {
    name: &'static str,
    output: Value,
    dependency_kinds: Option<Vec<ReactiveDependencyKind>>,
    dependency_scopes: Option<Vec<ReactiveDependencyScope>>,
    node_kind: ReactiveNodeKind,
}

impl TestFunction {
    pub(super) fn new(name: &'static str) -> Self {
        Self {
            name,
            output: Value::Empty,
            dependency_kinds: None,
            dependency_scopes: None,
            node_kind: ReactiveNodeKind::Combinational,
        }
    }

    #[cfg(feature = "f64")]
    pub(super) fn with_output(name: &'static str, output: Value) -> Self {
        Self {
            name,
            output,
            dependency_kinds: None,
            dependency_scopes: None,
            node_kind: ReactiveNodeKind::Combinational,
        }
    }

    pub(super) fn with_dependency_kinds(
        mut self,
        dependency_kinds: Option<Vec<ReactiveDependencyKind>>,
    ) -> Self {
        self.dependency_kinds = dependency_kinds;
        self
    }

    pub(super) fn with_dependency_scopes(
        mut self,
        scopes: Option<Vec<ReactiveDependencyScope>>,
    ) -> Self {
        self.dependency_scopes = scopes;
        self
    }

    pub(super) fn with_node_kind(mut self, node_kind: ReactiveNodeKind) -> Self {
        self.node_kind = node_kind;
        self
    }
}

impl MechFunctionImpl for TestFunction {
    fn solve(&self) {}

    fn out(&self) -> Value {
        self.output.clone()
    }

    fn reactive_dependency_kinds(
        &self,
        _argument_count: usize,
    ) -> Option<Vec<ReactiveDependencyKind>> {
        self.dependency_kinds.clone()
    }

    fn reactive_dependency_scopes(
        &self,
        _argument_count: usize,
    ) -> Option<Vec<ReactiveDependencyScope>> {
        self.dependency_scopes.clone()
    }

    fn reactive_node_kind(&self) -> ReactiveNodeKind {
        self.node_kind
    }

    fn to_string(&self) -> String {
        self.name.to_string()
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(self.reactive_output_values())
    }
}

#[cfg(feature = "compiler")]
impl MechFunctionCompiler for TestFunction {
    fn compile(&self, _ctx: &mut CompileCtx) -> MResult<Register> {
        Ok(0)
    }
}

#[cfg(all(feature = "set", feature = "f64"))]
pub(super) fn set_output() -> (Value, ReactiveCellId, ReactiveCellId, ReactiveCellId) {
    let first = Ref::new(1.0);
    let second = Ref::new(2.0);
    let mut members = indexmap::IndexSet::new();
    members.insert(Value::F64(first.clone()));
    members.insert(Value::F64(second.clone()));
    let set = Ref::new(MechSet {
        kind: ValueKind::F64,
        num_elements: 2,
        set: members,
    });

    (
        Value::Set(set.clone()),
        ReactiveCellId::new(set.id()),
        ReactiveCellId::new(first.id()),
        ReactiveCellId::new(second.id()),
    )
}

#[cfg(feature = "f64")]
pub(super) fn scalar(value: f64) -> (Value, ReactiveCellId) {
    let reference = Ref::new(value);
    let cell = ReactiveCellId::new(reference.id());
    (Value::F64(reference), cell)
}

#[cfg(feature = "f64")]
struct Commit {
    sink: Ref<f64>,
    next: f64,
    cells: Vec<ReactiveCellId>,
    count: Rc<RefCell<usize>>,
}
#[cfg(feature = "f64")]
impl reactive_register_sealed::Sealed for Commit {}
#[cfg(feature = "f64")]
impl ReactiveRegisterCommit for Commit {
    fn output_cells(&self) -> &[ReactiveCellId] {
        &self.cells
    }
    fn commit(self: Box<Self>) {
        *self.sink.borrow_mut() = self.next;
        *self.count.borrow_mut() += 1;
    }
}
#[cfg(feature = "f64")]
struct TestRegister {
    source: Ref<f64>,
    sink: Ref<f64>,
    solve: Rc<RefCell<usize>>,
    stage: Rc<RefCell<usize>>,
    commit: Rc<RefCell<usize>>,
    fail: bool,
}
#[cfg(feature = "f64")]
impl MechFunctionImpl for TestRegister {
    fn solve(&self) {
        *self.solve.borrow_mut() += 1;
    }
    fn out(&self) -> Value {
        self.sink.to_value()
    }
    fn reactive_node_kind(&self) -> ReactiveNodeKind {
        ReactiveNodeKind::Register
    }
    fn stage_register(&self) -> MResult<Box<dyn ReactiveRegisterCommit>> {
        *self.stage.borrow_mut() += 1;
        if self.fail {
            return Err(MechError::new(
                GenericError {
                    msg: "stage failure".into(),
                },
                None,
            ));
        }
        Ok(Box::new(Commit {
            sink: self.sink.clone(),
            next: *self.source.borrow(),
            cells: self.reactive_output_cell_ids(),
            count: self.commit.clone(),
        }))
    }
    fn to_string(&self) -> String {
        "test register".into()
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(self.reactive_output_values())
    }
}
#[cfg(all(feature = "compiler", feature = "f64"))]
impl MechFunctionCompiler for TestRegister {
    fn compile(&self, _: &mut CompileCtx) -> MResult<Register> {
        Ok(0)
    }
}
#[cfg(feature = "f64")]
fn counters() -> (Rc<RefCell<usize>>, Rc<RefCell<usize>>, Rc<RefCell<usize>>) {
    (
        Rc::new(RefCell::new(0)),
        Rc::new(RefCell::new(0)),
        Rc::new(RefCell::new(0)),
    )
}
#[cfg(feature = "f64")]
pub(super) fn reg(
    p: &mut ReactivePlan,
    source: Ref<f64>,
    sink: Ref<f64>,
    fail: bool,
) -> (
    ReactiveNodeId,
    Rc<RefCell<usize>>,
    Rc<RefCell<usize>>,
    Rc<RefCell<usize>>,
) {
    let (solve, stage, commit) = counters();
    let node = p
        .register(
            Box::new(TestRegister {
                source: source.clone(),
                sink,
                solve: solve.clone(),
                stage: stage.clone(),
                commit: commit.clone(),
                fail,
            }),
            &[source.to_value()],
        )
        .unwrap();
    (node, solve, stage, commit)
}
