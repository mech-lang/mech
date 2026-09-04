#[cfg(feature = "matrix_comprehensions")]
use super::super::ValueMatrixComprehension;
#[cfg(feature = "set_comprehensions")]
use super::super::ValueSetComprehension;
#[cfg(feature = "semantic-compiler")]
use crate::CompileCtx;
use crate::{FunctionInvocation, MechFunctionFactory, SchemaBody, ValueCell, ValueData};
#[cfg(feature = "semantic-compiler")]
use mech_core::{BytecodeInstruction, ParsedProgram, hash_str};

#[cfg(feature = "matrix_comprehensions")]
fn matrix_output(values: &[ValueCell], rows: usize, columns: usize) -> ValueCell {
    if values.is_empty() {
        ValueCell::dynamic_matrix(
            SchemaBody::Tuple(Box::new([])),
            vec![rows as u64, columns as u64].into_boxed_slice(),
            Box::new([]),
        )
        .unwrap()
    } else {
        ValueCell::dynamic_matrix_from_cells(rows, columns, values).unwrap()
    }
}

#[cfg(feature = "matrix_comprehensions")]
fn f64_matrix_contents(cell: &ValueCell) -> Vec<f64> {
    cell.matrix_elements()
        .unwrap()
        .unwrap()
        .iter()
        .map(|element| match element.snapshot().unwrap().data() {
            ValueData::F64(value) => value.to_f64(),
            other => panic!("expected f64 matrix element, found {other:?}"),
        })
        .collect()
}

#[cfg(feature = "matrix_comprehensions")]
#[test]
fn matrix_comprehension_factory_reconstructs_variadic_inputs() {
    let first = ValueCell::from_exact(1.0_f64).unwrap();
    let second = ValueCell::from_exact(2.0_f64).unwrap();
    let output = matrix_output(&[first.clone(), second.clone()], 1, 2);
    let alias = output.clone();
    let function = ValueMatrixComprehension::new_invocation(FunctionInvocation::variadic(
        output,
        vec![first, second].into_boxed_slice(),
    ))
    .unwrap();
    function.solve_result().unwrap();
    assert!(alias.same_cell(&alias.clone()));
    assert_eq!(f64_matrix_contents(&alias), vec![1.0, 2.0]);
}

#[cfg(feature = "matrix_comprehensions")]
#[test]
fn matrix_comprehension_factory_accepts_empty_variadic_encoding() {
    let output = matrix_output(&[], 0, 0);
    let function = ValueMatrixComprehension::new_invocation(FunctionInvocation::variadic(
        output.clone(),
        Box::new([]),
    ))
    .unwrap();

    function.solve_result().unwrap();
    assert_eq!(output.shape().parameter_values(), &[0, 0]);
    assert!(output.matrix_elements().unwrap().unwrap().is_empty());
    assert_eq!(
        function.semantic_operation_contract().unwrap().inputs,
        crate::InputPortLayout::Variadic {
            prefix: Box::new([]),
            repeated: crate::InputPortPolicy {
                access: crate::AccessMode::Read,
                delivery: crate::DeliveryMode::Signal,
            },
            min_repetitions: 0,
        },
    );
}

#[cfg(feature = "set_comprehensions")]
#[test]
fn set_comprehension_factory_preserves_checked_set_output() {
    let output =
        ValueCell::empty_dynamic_set(SchemaBody::UnsignedInteger(crate::IntegerWidth::W8)).unwrap();
    let alias = output.clone();
    let function = ValueSetComprehension::new_invocation(FunctionInvocation::variadic(
        output,
        vec![ValueCell::from_exact(7_u8).unwrap()].into_boxed_slice(),
    ))
    .unwrap();
    function.solve_result().unwrap();
    assert!(alias.same_cell(&alias.clone()));
    assert_eq!(alias.set_element_cells().unwrap().unwrap().len(), 1);
}

#[cfg(feature = "set_comprehensions")]
#[test]
fn set_comprehension_factory_accepts_empty_variadic_encoding() {
    let output =
        ValueCell::empty_dynamic_set(SchemaBody::FloatingPoint(crate::FloatWidth::W64)).unwrap();
    let function = ValueSetComprehension::new_invocation(FunctionInvocation::variadic(
        output.clone(),
        Box::new([]),
    ))
    .unwrap();
    function.solve_result().unwrap();
    assert!(output.set_element_cells().unwrap().unwrap().is_empty());
}

#[cfg(feature = "set_comprehensions")]
#[test]
fn set_comprehension_factory_rejects_non_set_output() {
    let result = ValueSetComprehension::new_invocation(FunctionInvocation::variadic(
        ValueCell::unit(),
        Box::new([]),
    ));
    let error = match result {
        Ok(_) => panic!("non-set bytecode output should be rejected"),
        Err(error) => error,
    };
    assert_eq!(error.kind_name(), "SetComprehensionOutputKindMismatch");
}

#[cfg(all(feature = "set_comprehensions", feature = "semantic-compiler"))]
#[test]
fn set_comprehension_bytecode_encodes_ordered_child_registers() {
    let first = ValueCell::from_exact(1_u8).unwrap();
    let second = ValueCell::from_exact(2_u8).unwrap();
    let output =
        ValueCell::empty_dynamic_set(SchemaBody::UnsignedInteger(crate::IntegerWidth::W8)).unwrap();
    let function = ValueSetComprehension::new_invocation(FunctionInvocation::variadic(
        output,
        vec![first, second].into_boxed_slice(),
    ))
    .unwrap();
    let mut context = CompileCtx::new();
    let output = function.compile(&mut context).unwrap();
    let parsed = ParsedProgram::from_bytes(&context.finish(output).unwrap()).unwrap();
    assert!(parsed.instructions.iter().any(|instruction| matches!(
        instruction,
        BytecodeInstruction::RuntimeVariadic { function, arguments, .. }
            if *function == hash_str("set/comprehension")
                && arguments.len() == 2
                && arguments[0] != arguments[1]
    )));
}

#[cfg(all(feature = "matrix_comprehensions", feature = "semantic-compiler"))]
#[test]
fn matrix_comprehension_bytecode_reuses_repeated_child_registers() {
    let repeated = ValueCell::from_exact(3.0_f64).unwrap();
    let output = matrix_output(&[repeated.clone(), repeated.clone()], 1, 2);
    let function = ValueMatrixComprehension::new_invocation(FunctionInvocation::variadic(
        output,
        vec![repeated.clone(), repeated].into_boxed_slice(),
    ))
    .unwrap();
    let mut context = CompileCtx::new();
    let output = function.compile(&mut context).unwrap();
    let parsed = ParsedProgram::from_bytes(&context.finish(output).unwrap()).unwrap();
    assert!(parsed.instructions.iter().any(|instruction| matches!(
        instruction,
        BytecodeInstruction::RuntimeVariadic { function, arguments, .. }
            if *function == hash_str("matrix/comprehension")
                && arguments.len() == 2
                && arguments[0] == arguments[1]
    )));
}

#[cfg(all(feature = "matrix_comprehensions", feature = "semantic-compiler"))]
#[test]
fn empty_matrix_comprehension_uses_nullary_semantic_construction() {
    let output_cell = matrix_output(&[], 0, 0);
    let function = ValueMatrixComprehension::new_invocation(FunctionInvocation::variadic(
        output_cell,
        Box::new([]),
    ))
    .unwrap();
    let mut context = CompileCtx::new();
    let output = function.compile(&mut context).unwrap();
    let compiled = context.finish_program(output).unwrap();

    assert!(!compiled.matrix_literals.contains_key(&output));
    assert!(
        compiled
            .program
            .instructions
            .iter()
            .any(|instruction| matches!(
                instruction,
                BytecodeInstruction::RuntimeNullary { function, dst }
                    if *function == hash_str("matrix/comprehension")
                        && *dst == output
            ))
    );
}
