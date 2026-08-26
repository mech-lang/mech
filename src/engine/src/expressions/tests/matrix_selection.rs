use crate::{
    BytecodeCompilerContext, FunctionCatalog, FunctionCatalogBuilder, FunctionExport,
    FunctionExposure, FunctionSpecializer, Interpreter, LegacyValue, MResult, MechFunction,
    MechFunctionCompiler, MechFunctionImpl, Ref, Register,
};
use mech_core::matrix::Matrix as RuntimeMatrix;
use nalgebra::DVector;
use std::sync::Arc;

#[derive(Debug)]
struct InclusiveRangeFunction {
    out: LegacyValue,
}

impl MechFunctionImpl for InclusiveRangeFunction {
    fn solve_result(&self) -> MResult<()> {
        Ok(())
    }

    fn out(&self) -> LegacyValue {
        self.out.clone()
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
    }

    fn to_string(&self) -> String {
        "TestInclusiveRange".to_string()
    }
}

impl MechFunctionCompiler for InclusiveRangeFunction {
    fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

struct InclusiveRangeSpecializer;

impl FunctionSpecializer for InclusiveRangeSpecializer {
    fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
        let [start, terminal] = arguments else {
            panic!("inclusive range test double expects two arguments");
        };
        let start = *start.as_f64()?.borrow() as usize;
        let terminal = *terminal.as_f64()?.borrow() as usize;
        let values = (start..=terminal).map(|value| value as f64).collect();
        Ok(Box::new(InclusiveRangeFunction {
            out: LegacyValue::MatrixF64(RuntimeMatrix::DVector(Ref::new(DVector::from_vec(
                values,
            )))),
        }))
    }
}

fn function_catalog_with_range() -> Arc<FunctionCatalog> {
    let mut builder = FunctionCatalogBuilder::new();
    crate::test_support::catalog::install_intrinsic_runtime(&mut builder).unwrap();
    crate::test_support::catalog::install_intrinsic_source(&mut builder).unwrap();
    let operation = builder
        .insert_specializer("range/inclusive", Arc::new(InclusiveRangeSpecializer))
        .unwrap();
    builder
        .insert_export(FunctionExport {
            operation,
            canonical_name: "range/inclusive".to_string(),
            module: None,
            item: None,
            exposure: FunctionExposure::Prelude,
        })
        .unwrap();
    Arc::new(builder.build().unwrap())
}

fn evaluate_selection(selector: &str) -> (Vec<usize>, Vec<f64>) {
    let source = format!("x := [1.0 2.0 3.0; 4.0 5.0 6.0; 7.0 8.0 9.0]; x{selector}");
    let tree = mech_syntax::parser::parse(&source).unwrap();
    let catalog = if selector.contains("..=") {
        function_catalog_with_range()
    } else {
        crate::test_support::catalog::function_catalog()
    };
    let mut interpreter = Interpreter::with_function_catalog(0, 10_000, catalog);
    let output = interpreter.interpret(&tree).unwrap();
    let output = match output {
        LegacyValue::MutableReference(value) => value.borrow().clone(),
        value => value,
    };
    let shape = output.shape();
    let LegacyValue::MatrixF64(matrix) = output else {
        panic!("expected an f64 matrix selection");
    };
    (shape, matrix.as_vec())
}

#[test]
fn explicit_all_selector_preserves_linear_and_scalar_axis_access() {
    assert_eq!(
        evaluate_selection("[:]"),
        (
            vec![9, 1],
            vec![1.0, 4.0, 7.0, 2.0, 5.0, 8.0, 3.0, 6.0, 9.0]
        )
    );
    assert_eq!(
        evaluate_selection("[:,2]"),
        (vec![3, 1], vec![2.0, 5.0, 8.0])
    );
    assert_eq!(
        evaluate_selection("[2,:]"),
        (vec![1, 3], vec![4.0, 5.0, 6.0])
    );
}

#[test]
fn explicit_all_selector_preserves_range_access_on_both_axes() {
    assert_eq!(
        evaluate_selection("[:,1..=2]"),
        (vec![3, 2], vec![1.0, 4.0, 7.0, 2.0, 5.0, 8.0])
    );
    assert_eq!(
        evaluate_selection("[1..=2,:]"),
        (vec![2, 3], vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0])
    );
}

#[test]
fn explicit_all_selector_preserves_index_vector_access_on_both_axes() {
    assert_eq!(
        evaluate_selection("[:,[1 3]]"),
        (vec![3, 2], vec![1.0, 4.0, 7.0, 3.0, 6.0, 9.0])
    );
    assert_eq!(
        evaluate_selection("[[1 3],:]"),
        (vec![2, 3], vec![1.0, 7.0, 2.0, 8.0, 3.0, 9.0])
    );
}

#[test]
fn explicit_all_selector_preserves_logical_mask_access_on_both_axes() {
    assert_eq!(
        evaluate_selection("[:,[true false true]]"),
        (vec![3, 2], vec![1.0, 4.0, 7.0, 3.0, 6.0, 9.0])
    );
    assert_eq!(
        evaluate_selection("[[true false true],:]"),
        (vec![2, 3], vec![1.0, 7.0, 2.0, 8.0, 3.0, 9.0])
    );
}
