use super::super::registration::{
    register_expression_function_batch, register_initialized_expression_function,
};
#[cfg(feature = "semantic-compiler")]
use crate::{BytecodeCompilerContext, MechFunctionCompiler, Register};
use crate::{
    InitialSolvePolicy, LegacyValue, MResult, MechError, MechErrorKind, MechFunction,
    MechFunctionImpl, Plan, ReactiveCellId, ReactiveDependencyKind, Ref,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

struct IndexedExpressionTestFunction {
    output: LegacyValue,
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
    output: LegacyValue,
    solve_result_calls: Arc<AtomicUsize>,
}

impl MechFunctionImpl for FailingInitialExpressionFunction {
    fn solve_result(&self) -> MResult<()> {
        self.solve_result_calls.fetch_add(1, Ordering::SeqCst);
        Err(MechError::new(InitialExpressionSolveFailure, None))
    }

    fn out(&self) -> LegacyValue {
        self.output.clone()
    }

    fn to_string(&self) -> String {
        "failing-initial-expression".to_owned()
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
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

    fn out(&self) -> LegacyValue {
        self.output.clone()
    }

    fn initial_solve_policy(&self) -> InitialSolvePolicy {
        self.initial_solve_policy
    }

    fn to_string(&self) -> String {
        "indexed-expression-test".to_string()
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for IndexedExpressionTestFunction {
    fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

fn scalar(value: f64) -> (LegacyValue, ReactiveCellId) {
    let reference = Ref::new(value);
    let cell = ReactiveCellId::new(reference.id());
    (LegacyValue::F64(reference), cell)
}

fn function(output: LegacyValue, calls: Arc<AtomicUsize>) -> Box<dyn MechFunction> {
    Box::new(IndexedExpressionTestFunction {
        output,
        solve_calls: calls,
        initial_solve_policy: InitialSolvePolicy::Solve,
    })
}

fn preserving_function(output: LegacyValue, calls: Arc<AtomicUsize>) -> Box<dyn MechFunction> {
    Box::new(IndexedExpressionTestFunction {
        output,
        solve_calls: calls,
        initial_solve_policy: InitialSolvePolicy::PreserveSpecializedOutput,
    })
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
        function(output, calls.clone()),
        &[first, second, third],
    )
    .unwrap();
    let plan_borrow = plan.borrow();
    let node = plan_borrow.node(0).unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(result.reactive_cell_ids(), vec![out]);
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
        function(output, calls.clone()),
        &[input.clone(), input],
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
        preserving_function(output, calls.clone()),
        &[input],
    )
    .unwrap();

    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(result.reactive_cell_ids(), vec![output_cell]);
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
        Box::new(FailingInitialExpressionFunction {
            output,
            solve_result_calls: solve_result_calls.clone(),
        }),
        &[input],
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
    let f1 = function(mid.clone(), first.clone());
    let f2 = function(final_out, second.clone());
    f1.solve_result().unwrap();
    f2.solve_result().unwrap();

    register_expression_function_batch(&plan, vec![(f1, vec![a, b]), (f2, vec![mid, c])]).unwrap();
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
