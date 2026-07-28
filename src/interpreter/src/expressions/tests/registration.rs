use super::super::{register_expression_function_batch, register_initialized_expression_function};
#[cfg(feature = "compiler")]
use crate::{CompileCtx, MechFunctionCompiler, Register};
use crate::{
    MResult, MechFunction, MechFunctionImpl, Plan, ReactiveCellId, ReactiveDependencyKind, Ref,
    Value,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

struct IndexedExpressionTestFunction {
    output: Value,
    solve_calls: Arc<AtomicUsize>,
}

impl MechFunctionImpl for IndexedExpressionTestFunction {
    fn solve(&self) {
        self.solve_calls.fetch_add(1, Ordering::SeqCst);
    }

    fn out(&self) -> Value {
        self.output.clone()
    }

    fn to_string(&self) -> String {
        "indexed-expression-test".to_string()
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(self.reactive_output_values())
    }
}

#[cfg(feature = "compiler")]
impl MechFunctionCompiler for IndexedExpressionTestFunction {
    fn compile(&self, _ctx: &mut CompileCtx) -> MResult<Register> {
        Ok(0)
    }
}

fn scalar(value: f64) -> (Value, ReactiveCellId) {
    let reference = Ref::new(value);
    let cell = ReactiveCellId::new(reference.id());
    (Value::F64(reference), cell)
}

fn function(output: Value, calls: Arc<AtomicUsize>) -> Box<dyn MechFunction> {
    Box::new(IndexedExpressionTestFunction {
        output,
        solve_calls: calls,
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
    f1.solve();
    f2.solve();

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
