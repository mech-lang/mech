use mech_core::structures::Matrix as ValueMatrix;
use mech_core::{
    BytecodeInstruction, LegacyValue, MResult, ParsedProgram, RuntimeFunctionId, hash_str,
};
use mech_runtime::{ResidentDurabilityPolicy, RuntimeBuilder};

#[test]
fn dynamic_matrix_addition_bytecode_reconstructs_from_full_runtime() -> MResult<()> {
    const FACTORY_NAME: &str = "AddMDMD<f64>";

    let factory_id = hash_str(FACTORY_NAME);
    let source = "left := [1 2 3 4 5; 6 7 8 9 10; 11 12 13 14 15; 16 17 18 19 20; 21 22 23 24 25]\n\
         right := [25 24 23 22 21; 20 19 18 17 16; 15 14 13 12 11; 10 9 8 7 6; 5 4 3 2 1]\n\
         left + right";
    let bytecode = RuntimeBuilder::new()
        .function_catalog(mech::stdlib::source_catalog())
        .build_compiler()?
        .compile_source(source)?
        .into_parts()
        .1;
    let parsed = ParsedProgram::from_bytes(&bytecode)?;
    let operation_ids = parsed
        .instructions
        .iter()
        .filter_map(|instruction| match instruction {
            BytecodeInstruction::RuntimeBinary { function, .. } => Some(*function),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(operation_ids.last(), Some(&factory_id));
    assert_eq!(
        operation_ids
            .iter()
            .filter(|operation_id| **operation_id == factory_id)
            .count(),
        1,
    );

    let catalog = mech::stdlib::source_catalog();

    let runtime_id = RuntimeFunctionId::from_raw(factory_id);
    let catalog_entry = catalog
        .runtime_entry(runtime_id)
        .expect("standard catalog must contain the decoded runtime factory");
    assert_eq!(catalog_entry.name, FACTORY_NAME);

    let mut runtime = RuntimeBuilder::new().function_catalog(catalog).build()?;
    let decoded_output = runtime
        .load_bytecode_program(&bytecode, ResidentDurabilityPolicy::Volatile)?
        .initial_value
        .into_value();

    let LegacyValue::MatrixF64(ValueMatrix::DMatrix(matrix)) = decoded_output else {
        panic!("dynamic matrix addition must return a dynamic f64 matrix");
    };
    let matrix = matrix.borrow();
    assert_eq!((matrix.nrows(), matrix.ncols()), (5, 5));
    assert!(matrix.iter().all(|value| *value == 26.0));
    Ok(())
}
