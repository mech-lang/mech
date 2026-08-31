#[cfg(feature = "semantic-compiler")]
use super::super::MechFunctionCompiler;
use super::super::{
    FunctionInstance, FunctionInvocation, MechFunctionImpl, ReactiveDependencyKind,
    ReactiveDependencyScope, ReactiveNodeId, ReactiveNodeKind, ReactivePlan,
    ReactiveRegisterCommit, reactive_register_sealed,
};
#[cfg(feature = "semantic-compiler")]
use crate::{BytecodeCompilerContext, Register};
use crate::{CanonicalCellId, FunctionStatePort, GenericError, MResult, MechError, Ref, ValueCell};
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
        Self::with_output(name, ValueCell::unit())
    }

    pub(super) fn with_output(name: &'static str, output: ValueCell) -> Self {
        Self {
            name,
            output,
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
        self.name.into()
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for TestFunction {
    fn compile(&self, _context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

pub(super) fn index(value: usize) -> (ValueCell, CanonicalCellId) {
    let value = ValueCell::from_exact(value).unwrap();
    let identity = value.reactive_cell_id();
    (value, identity)
}

#[cfg(feature = "f64")]
pub(super) fn f64_cell(reference: Ref<f64>) -> ValueCell {
    ValueCell::from_inferred_ref(reference, None).unwrap()
}

#[cfg(feature = "f64")]
struct TestRegisterCommit {
    sink: Ref<f64>,
    next: f64,
    cells: Vec<CanonicalCellId>,
    count: Rc<RefCell<usize>>,
}

#[cfg(feature = "f64")]
impl reactive_register_sealed::Sealed for TestRegisterCommit {}

#[cfg(feature = "f64")]
impl ReactiveRegisterCommit for TestRegisterCommit {
    fn output_cells(&self) -> &[CanonicalCellId] {
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

    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_ref(&self.sink))
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
        Ok(Box::new(TestRegisterCommit {
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
pub(super) fn reg(
    plan: &mut ReactivePlan,
    source: Ref<f64>,
    sink: Ref<f64>,
    fail: bool,
) -> (
    ReactiveNodeId,
    Rc<RefCell<usize>>,
    Rc<RefCell<usize>>,
    Rc<RefCell<usize>>,
) {
    let solve = Rc::new(RefCell::new(0));
    let stage = Rc::new(RefCell::new(0));
    let commit = Rc::new(RefCell::new(0));
    let source_cell = f64_cell(source.clone());
    let sink_cell = f64_cell(sink.clone());
    let instance = FunctionInstance::new(
        Box::new(TestRegister {
            source,
            sink,
            solve: solve.clone(),
            stage: stage.clone(),
            commit: commit.clone(),
            fail,
        }),
        FunctionInvocation::unary(sink_cell, source_cell),
    );
    let node = plan
        .register_instance_with_activation(instance, None)
        .unwrap();
    (node, solve, stage, commit)
}
