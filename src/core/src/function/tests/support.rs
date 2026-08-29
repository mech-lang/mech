#[cfg(feature = "semantic-compiler")]
use super::super::MechFunctionCompiler;
use super::super::{
    MechFunctionImpl, ReactiveDependencyKind, ReactiveDependencyScope, ReactiveNodeId,
    ReactiveNodeKind, ReactivePlan, ReactiveRegisterCommit, reactive_register_sealed,
};
#[cfg(all(feature = "set", feature = "f64"))]
use crate::MechSet;
#[cfg(feature = "semantic-compiler")]
use crate::{BytecodeCompilerContext, Register};
use crate::{
    GenericError, LegacyReactivePlanRegistration, LegacyValue, MResult, MechError, ReactiveCellId,
    Ref, ToValue, ValueCell, ValueKind,
};
use std::{cell::RefCell, rc::Rc};

pub(super) struct TestFunction {
    name: &'static str,
    output: ValueCell,
    dependency_kinds: Option<Vec<ReactiveDependencyKind>>,
    dependency_scopes: Option<Vec<ReactiveDependencyScope>>,
    node_kind: ReactiveNodeKind,
    description_calls: Option<Rc<RefCell<usize>>>,
}

impl TestFunction {
    pub(super) fn new(name: &'static str) -> Self {
        Self {
            name,
            output: ValueCell::unit(),
            dependency_kinds: None,
            dependency_scopes: None,
            node_kind: ReactiveNodeKind::Combinational,
            description_calls: None,
        }
    }

    #[cfg(feature = "f64")]
    pub(super) fn with_output(name: &'static str, output: LegacyValue) -> Self {
        Self {
            name,
            output: crate::value_cell_from_legacy_function_value(output),
            dependency_kinds: None,
            dependency_scopes: None,
            node_kind: ReactiveNodeKind::Combinational,
            description_calls: None,
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

    pub(super) fn with_description_counter(mut self, calls: Rc<RefCell<usize>>) -> Self {
        self.description_calls = Some(calls);
        self
    }
}

impl MechFunctionImpl for TestFunction {
    fn solve_result(&self) -> MResult<()> {
        Ok(())
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

    fn reactive_output_value_cells(&self) -> Vec<ValueCell> {
        vec![self.output.clone()]
    }

    fn to_string(&self) -> String {
        if let Some(calls) = &self.description_calls {
            *calls.borrow_mut() += 1;
        }
        self.name.to_string()
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for TestFunction {
    fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

#[cfg(all(feature = "set", feature = "f64"))]
pub(super) fn set_output() -> (LegacyValue, ReactiveCellId, ReactiveCellId, ReactiveCellId) {
    let first = Ref::new(1.0);
    let second = Ref::new(2.0);
    let mut members = indexmap::IndexSet::new();
    members.insert(LegacyValue::F64(first.clone()));
    members.insert(LegacyValue::F64(second.clone()));
    let set = Ref::new(MechSet {
        kind: ValueKind::F64,
        max_elements: Some(2),
        num_elements: 2,
        set: members,
    });

    (
        LegacyValue::Set(set.clone()),
        ReactiveCellId::new(set.id()),
        ReactiveCellId::new(first.id()),
        ReactiveCellId::new(second.id()),
    )
}

#[cfg(feature = "f64")]
pub(super) fn scalar(value: f64) -> (LegacyValue, ReactiveCellId) {
    let reference = Ref::new(value);
    let cell = ReactiveCellId::new(reference.id());
    (LegacyValue::F64(reference), cell)
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
    fn solve_result(&self) -> MResult<()> {
        *self.solve.borrow_mut() += 1;
        Ok(())
    }
    fn reactive_node_kind(&self) -> ReactiveNodeKind {
        ReactiveNodeKind::Register
    }
    fn primary_output_state_port(&self) -> Option<crate::FunctionStatePort<'_>> {
        Some(crate::FunctionStatePort::from_ref(&self.sink))
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
}
#[cfg(all(feature = "semantic-compiler", feature = "f64"))]
impl MechFunctionCompiler for TestRegister {
    fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
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
