use super::super::registration::{
    register_expression_function_batch, register_initialized_expression_function,
};
use crate::{
    AccessMode, AliasPolicy, CanonicalCellId, ChangeDetectionPolicy, DeliveryMode, ExecutionTarget,
    ExternalInteraction, FunctionInstance, FunctionInvocation, InitialSolvePolicy, InputPortLayout,
    InputPortPolicy, MResult, MechError, MechErrorKind, MechFunction, MechFunctionImpl,
    OperationContractDeclaration, OutputConstruction, OutputPortPolicy, Plan,
    ReactiveDependencyKind, ResolvedOperationDescriptor, RuntimeFunctionId, ShapeRule,
    SpecializedFunction, ValueCell,
};
#[cfg(feature = "semantic-compiler")]
use crate::{BytecodeCompilerContext, MechFunctionCompiler, Register};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

struct IndexedExpressionTestFunction {
    solve_calls: Arc<AtomicUsize>,
    initial_solve_policy: InitialSolvePolicy,
}

#[derive(Clone, Debug)]
struct InitialExpressionSolveFailure;

impl MechErrorKind for InitialExpressionSolveFailure {
    fn name(&self) -> &str {
        "InitialExpressionSolveFailure"
    }

    fn message(&self) -> String {
        "initial expression solve failed".to_owned()
    }
}

struct FailingInitialExpressionFunction {
    solve_result_calls: Arc<AtomicUsize>,
}

impl MechFunctionImpl for FailingInitialExpressionFunction {
    fn solve_result(&self) -> MResult<()> {
        self.solve_result_calls.fetch_add(1, Ordering::SeqCst);
        Err(MechError::new(InitialExpressionSolveFailure, None))
    }

    fn to_string(&self) -> String {
        "failing-initial-expression".to_owned()
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for FailingInitialExpressionFunction {
    fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        panic!("failing expression test function must not be compiled")
    }
}

impl MechFunctionImpl for IndexedExpressionTestFunction {
    fn solve_result(&self) -> MResult<()> {
        self.solve_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn initial_solve_policy(&self) -> InitialSolvePolicy {
        self.initial_solve_policy
    }

    fn to_string(&self) -> String {
        "indexed-expression-test".to_string()
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for IndexedExpressionTestFunction {
    fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

fn scalar(value: f64) -> (ValueCell, CanonicalCellId) {
    let value = ValueCell::from_exact(value).unwrap();
    let cell = value.reactive_cell_id();
    (value, cell)
}

fn function(calls: Arc<AtomicUsize>) -> Box<dyn MechFunction> {
    Box::new(IndexedExpressionTestFunction {
        solve_calls: calls,
        initial_solve_policy: InitialSolvePolicy::Solve,
    })
}

fn preserving_function(calls: Arc<AtomicUsize>) -> Box<dyn MechFunction> {
    Box::new(IndexedExpressionTestFunction {
        solve_calls: calls,
        initial_solve_policy: InitialSolvePolicy::PreserveSpecializedOutput,
    })
}

fn specialized(
    implementation: Box<dyn MechFunction>,
    output: ValueCell,
    inputs: Vec<ValueCell>,
) -> SpecializedFunction {
    let input_count = inputs.len();
    SpecializedFunction::syntax_directed(
        FunctionInstance::new(
            implementation,
            FunctionInvocation::variadic(output, inputs.into_boxed_slice()),
        ),
        ResolvedOperationDescriptor::from_name(
            "test/indexed-expression",
            OperationContractDeclaration {
                inputs: InputPortLayout::Fixed(
                    vec![
                        InputPortPolicy {
                            access: AccessMode::Read,
                            delivery: DeliveryMode::Signal,
                        };
                        input_count
                    ]
                    .into_boxed_slice(),
                ),
                outputs: vec![OutputPortPolicy {
                    access: AccessMode::Write,
                    delivery: DeliveryMode::Signal,
                    construction: OutputConstruction::FullWrite {
                        shape: ShapeRule::Declared,
                    },
                    alias: AliasPolicy::NoAlias,
                    change_detection: ChangeDetectionPolicy::KernelReported,
                }]
                .into_boxed_slice(),
                interaction: ExternalInteraction::Pure,
            },
        )
        .unwrap(),
        RuntimeFunctionId::from_name("IndexedExpressionTestFunction"),
        ExecutionTarget::DirectRuntime,
        mech_core::ImplementationMemoryClass::NoAdditionalScratch,
    )
    .unwrap()
}

#[test]
fn indexed_expression_registration_records_dependencies() {
    let plan = Plan::new();
    let (first, a) = scalar(1.0);
    let (second, b) = scalar(2.0);
    let (third, c) = scalar(3.0);
    let (output, out) = scalar(4.0);
    let calls = Arc::new(AtomicUsize::new(0));
    let result = register_initialized_expression_function(
        &plan,
        specialized(function(calls.clone()), output, vec![first, second, third]),
    )
    .unwrap();
    let plan_borrow = plan.borrow();
    let node = plan_borrow.node(0).unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(result.reactive_cell_id(), out);
    assert_eq!(
        node.inputs.iter().map(|d| d.cell).collect::<Vec<_>>(),
        vec![a, b, c]
    );
    assert!(
        node.inputs
            .iter()
            .all(|d| d.kind == ReactiveDependencyKind::Reactive)
    );
    for cell in [a, b, c] {
        assert_eq!(plan_borrow.reactive_consumers_for(cell), &[0]);
        assert!(plan_borrow.sampled_consumers_for(cell).is_empty());
    }
    assert!(node.outputs.contains(&out));
    assert!(!node.inputs.iter().any(|d| d.cell == out));
}

#[test]
fn indexed_expression_registration_deduplicates_aliases() {
    let plan = Plan::new();
    let (input, cell) = scalar(1.0);
    let (output, _) = scalar(2.0);
    let calls = Arc::new(AtomicUsize::new(0));
    register_initialized_expression_function(
        &plan,
        specialized(function(calls.clone()), output, vec![input.clone(), input]),
    )
    .unwrap();
    let plan = plan.borrow();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(plan.node(0).unwrap().inputs.len(), 1);
    assert_eq!(plan.reactive_consumers_for(cell), &[0]);
}

#[test]
fn indexed_expression_registration_preserves_planned_output_when_requested() {
    let plan = Plan::new();
    let (input, _) = scalar(1.0);
    let (output, output_cell) = scalar(2.0);
    let calls = Arc::new(AtomicUsize::new(0));

    let result = register_initialized_expression_function(
        &plan,
        specialized(preserving_function(calls.clone()), output, vec![input]),
    )
    .unwrap();

    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(result.reactive_cell_id(), output_cell);
    assert_eq!(plan.len(), 1);
}

#[test]
fn indexed_expression_registration_propagates_initial_solve_errors() {
    let plan = Plan::new();
    let (input, _) = scalar(1.0);
    let (output, _) = scalar(2.0);
    let solve_result_calls = Arc::new(AtomicUsize::new(0));

    let error = register_initialized_expression_function(
        &plan,
        specialized(
            Box::new(FailingInitialExpressionFunction {
                solve_result_calls: solve_result_calls.clone(),
            }),
            output,
            vec![input],
        ),
    )
    .unwrap_err();

    assert_eq!(error.kind_name(), "InitialExpressionSolveFailure");
    assert_eq!(solve_result_calls.load(Ordering::SeqCst), 1);
    assert_eq!(plan.len(), 0);
}

#[test]
fn binary_term_batch_registration_preserves_order_and_edges() {
    let plan = Plan::new();
    let (a, ac) = scalar(1.0);
    let (b, bc) = scalar(2.0);
    let (c, cc) = scalar(3.0);
    let (mid, mc) = scalar(4.0);
    let (final_out, fc) = scalar(5.0);
    let first = Arc::new(AtomicUsize::new(0));
    let second = Arc::new(AtomicUsize::new(0));
    let f1 = function(first.clone());
    let f2 = function(second.clone());
    f1.solve_result().unwrap();
    f2.solve_result().unwrap();

    register_expression_function_batch(
        &plan,
        vec![
            specialized(f1, mid.clone(), vec![a, b]),
            specialized(f2, final_out, vec![mid, c]),
        ],
    )
    .unwrap();
    let plan = plan.borrow();
    assert_eq!(plan.len(), 2);
    assert_eq!(
        plan.node(0)
            .unwrap()
            .inputs
            .iter()
            .map(|d| d.cell)
            .collect::<Vec<_>>(),
        vec![ac, bc]
    );
    assert_eq!(
        plan.node(1)
            .unwrap()
            .inputs
            .iter()
            .map(|d| d.cell)
            .collect::<Vec<_>>(),
        vec![mc, cc]
    );
    assert!(plan.node(1).unwrap().outputs.contains(&fc));
    assert_eq!(plan.reactive_consumers_for(mc), &[1]);
    assert_eq!(first.load(Ordering::SeqCst), 1);
    assert_eq!(second.load(Ordering::SeqCst), 1);
}
