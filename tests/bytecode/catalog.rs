use std::sync::Arc;

use mech_core::structures::Matrix as ValueMatrix;
use mech_core::{FunctionCatalogBuilder, MResult, ParsedProgram, RuntimeFunctionId, Value};
use mech_engine::{MechProgram, MechProgramConfig};

const SCALAR_ADD_BYTECODE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/architecture/legacy-bytecode/scalar-add.mecb"
));
const MATRIX_SCALAR_ADD_BYTECODE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/architecture/legacy-bytecode/matrix-scalar-add.mecb"
));
const STRING_CONCAT_BYTECODE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/architecture/legacy-bytecode/string-concat.mecb"
));

fn full_program() -> MechProgram {
    MechProgram::with_function_catalog(MechProgramConfig::default(), mech::stdlib::source_catalog())
}

fn parse(bytecode: &[u8]) -> ParsedProgram {
    ParsedProgram::from_bytes(bytecode).expect("checked-in bytecode artifact must remain valid")
}

fn assert_catalog_factory_owned(program: &MechProgram, name: &str) {
    assert!(
        program
            .function_catalog()
            .runtime_factory(RuntimeFunctionId::from_name(name))
            .is_some(),
        "explicit catalog did not contain {name}",
    );
}

fn assert_f64(value: Value, expected: f64) {
    let Value::F64(actual) = value else {
        panic!("expected f64 {expected}, got {value:?}");
    };
    assert_eq!(*actual.borrow(), expected);
}

#[test]
fn scalar_add_bytecode_uses_catalog_factory() {
    let mut program = full_program();
    assert_catalog_factory_owned(&program, "AddSS<f64>");

    let value = program
        .run_bytecode_program(&parse(SCALAR_ADD_BYTECODE))
        .expect("catalog must reconstruct AddSS<f64>");

    assert_f64(value, 3.0);
}

#[test]
fn scalar_add_bytecode_fails_when_catalog_omits_add() {
    const FACTORY_NAME: &str = "AddSS<f64>";

    let catalog = Arc::new(FunctionCatalogBuilder::new().build().unwrap());
    let runtime_id = RuntimeFunctionId::from_name(FACTORY_NAME);
    let mut program =
        MechProgram::with_function_catalog(MechProgramConfig::default(), Arc::clone(&catalog));

    assert!(
        catalog.runtime_factory(runtime_id).is_none(),
        "empty custom catalog unexpectedly contains {FACTORY_NAME}",
    );

    let error = program
        .run_bytecode_program(&parse(SCALAR_ADD_BYTECODE))
        .expect_err("bytecode must fail when its runtime factory is absent from the catalog");

    assert_eq!(error.kind_name(), "UnknownBinaryFunction");
    assert_eq!(
        error.kind_message(),
        format!("Unknown binary function ID: {}", runtime_id.raw()),
    );
}

#[test]
fn matrix_scalar_add_bytecode_uses_catalog_for_every_static_factory() {
    let mut program = full_program();
    assert_catalog_factory_owned(&program, "AddRDS<f64>");
    assert_catalog_factory_owned(&program, "HorizontalConcatenateRDN<f64>");

    let value = program
        .run_bytecode_program(&parse(MATRIX_SCALAR_ADD_BYTECODE))
        .expect("catalog add and concatenation factories must compose");
    let Value::MatrixF64(matrix) = value else {
        panic!("matrix-scalar addition must return an f64 matrix");
    };

    assert_eq!((matrix.rows(), matrix.cols()), (1, 2));
    assert_eq!(matrix.as_vec(), vec![2.0, 3.0]);
}

#[test]
fn dynamic_matrix_add_bytecode_uses_catalog_factory() -> MResult<()> {
    const FACTORY_NAME: &str = "AddMDMD<f64>";

    let mut source = full_program();
    source.run_string(
        "left := [1 2 3 4 5; 6 7 8 9 10; 11 12 13 14 15; 16 17 18 19 20; 21 22 23 24 25]\n\
         right := [25 24 23 22 21; 20 19 18 17 16; 15 14 13 12 11; 10 9 8 7 6; 5 4 3 2 1]\n\
         left + right",
    )?;
    let parsed = ParsedProgram::from_bytes(&source.compile_bytecode()?)?;

    let mut decoded = full_program();
    assert_catalog_factory_owned(&decoded, FACTORY_NAME);
    decoded.run_bytecode_program(&parsed)?;

    let output = decoded.solve_plan()?.value;
    let Value::MatrixF64(ValueMatrix::DMatrix(matrix)) = output else {
        panic!("dynamic matrix addition must return a dynamic f64 matrix");
    };
    let matrix = matrix.borrow();
    assert_eq!((matrix.nrows(), matrix.ncols()), (5, 5));
    assert!(matrix.iter().all(|value| *value == 26.0));
    Ok(())
}

#[test]
fn non_add_bytecode_uses_the_catalog() {
    const FACTORY_NAME: &str = "ConcatSS<string>";

    let mut program = full_program();
    assert_catalog_factory_owned(&program, FACTORY_NAME);

    let value = program
        .run_bytecode_program(&parse(STRING_CONCAT_BYTECODE))
        .expect("string concatenation bytecode must reconstruct through the catalog");
    let Value::String(value) = value else {
        panic!("string concatenation must return a string");
    };
    assert_eq!(&*value.borrow(), "abc");
}
