use mech_core::structures::Matrix as ValueMatrix;
use mech_core::{MResult, ParsedProgram, RuntimeFunctionId, Value};
use mech_engine::{MechProgram, MechProgramConfig};

fn full_program() -> MechProgram {
    MechProgram::with_function_catalog(MechProgramConfig::default(), mech::stdlib::source_catalog())
}

fn assert_catalog_factory_owned(program: &MechProgram, name: &str) {
    assert!(
        program
            .function_catalog()
            .runtime_entry(RuntimeFunctionId::from_name(name))
            .is_some(),
        "explicit catalog did not contain {name}",
    );
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
