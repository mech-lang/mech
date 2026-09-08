#[cfg(feature = "semantic-compiler")]
use super::super::MechFunctionCompiler;
use super::super::{
    FunctionInstance, FunctionInvocation, MechFunctionImpl, ReactiveDependencyKind,
    ReactiveDependencyScope, ReactiveNodeId, ReactiveNodeKind, ReactivePlan,
};
#[cfg(feature = "semantic-compiler")]
use crate::{BytecodeCompilerContext, Register};
use crate::{CanonicalCellId, FunctionStatePort, GenericError, MResult, MechError, ValueCell};
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
    pub(super) fn into_instance(self) -> FunctionInstance {
        let output = self.output.clone();
        crate::function::test_planned_instance(Box::new(self), FunctionInvocation::nullary(output))
    }

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
    fn solve_managed(
        &self,
        _frame: &mut crate::KernelMemoryFrame<'_>,
        _services: &mut dyn crate::MechExecutionServices,
    ) -> MResult<crate::ReactiveSolveStatus> {
        (|| -> MResult<()> { Ok(()) })()?;
        Ok(crate::ReactiveSolveStatus::Changed)
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
struct TestRegister {
    source: crate::ManagedPort<f64>,
    sink: crate::ManagedPort<f64>,
    stage: Rc<RefCell<usize>>,
    fail: bool,
}

#[cfg(feature = "f64")]
impl MechFunctionImpl for TestRegister {
    fn solve_managed(
        &self,
        frame: &mut crate::KernelMemoryFrame<'_>,
        _services: &mut dyn crate::MechExecutionServices,
    ) -> MResult<crate::ReactiveSolveStatus> {
        *self.stage.borrow_mut() += 1;
        if self.fail {
            return Err(MechError::new(
                GenericError {
                    msg: "stage failure".into(),
                },
                None,
            ));
        }
        frame.with_unary_port_views(&self.source, &self.sink, |source, sink| {
            sink.try_fill_column_major(|index| {
                source.get_column_major(index).ok_or_else(|| {
                    MechError::new(
                        GenericError {
                            msg: "register input and output geometry disagree".into(),
                        },
                        None,
                    )
                })
            })
        })?;
        Ok(crate::ReactiveSolveStatus::Changed)
    }

    fn reactive_node_kind(&self) -> ReactiveNodeKind {
        ReactiveNodeKind::Register
    }

    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_cell(self.sink.cell()))
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
    source: ValueCell,
    sink: ValueCell,
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
    let invocation = FunctionInvocation::unary(sink, source);
    let (sink, source) = invocation.expect_unary().unwrap();
    let instance = crate::function::test_planned_instance(
        Box::new(TestRegister {
            source: source.try_managed_element::<f64>().unwrap(),
            sink: sink.try_managed_element::<f64>().unwrap(),
            stage: stage.clone(),
            fail,
        }),
        invocation,
    );
    let node = plan
        .register_instance_with_activation(instance, None)
        .unwrap();
    (node, solve, stage, commit)
}
