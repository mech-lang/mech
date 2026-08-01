use mech_core::structures::Matrix as ValueMatrix;
use mech_core::{DecodedInstr, MResult, ParsedProgram, Value, hash_str};
use mech_program::{MechProgram, MechProgramConfig};

#[test]
fn dynamic_matrix_addition_bytecode_reconstructs_from_full_runtime() -> MResult<()> {
    const FACTORY_NAME: &str = "AddMDMD<f64>";

    let factory_id = hash_str(FACTORY_NAME);
    let mut source = MechProgram::new(MechProgramConfig::default());
    source.load_full_stdlib();
    source.run_string(
        "left := [1 2 3 4 5; 6 7 8 9 10; 11 12 13 14 15; 16 17 18 19 20; 21 22 23 24 25]\n\
         right := [25 24 23 22 21; 20 19 18 17 16; 15 14 13 12 11; 10 9 8 7 6; 5 4 3 2 1]\n\
         left + right",
    )?;

    let bytecode = source.compile_bytecode()?;
    let parsed = ParsedProgram::from_bytes(&bytecode)?;
    let operation_ids = parsed
        .instrs
        .iter()
        .filter_map(|instruction| match instruction {
            DecodedInstr::BinOp { fxn_id, .. } => Some(*fxn_id),
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

    let mut decoded = MechProgram::new(MechProgramConfig::default());
    decoded.load_full_stdlib();
    decoded
        .run_bytecode_program(&parsed)
        .unwrap_or_else(|error| {
            panic!("fresh full runtime failed to reconstruct {FACTORY_NAME}: {error:?}")
        });

    let functions = decoded.interpreter().functions();
    let functions = functions.borrow();
    assert!(
        functions.functions.contains_key(&factory_id),
        "full runtime factory table must contain {FACTORY_NAME}",
    );
    assert_eq!(
        functions.dictionary.borrow().get(&factory_id).cloned(),
        Some(FACTORY_NAME.to_string()),
    );
    drop(functions);

    let decoded_output = decoded.solve_plan()?.value;

    let Value::MatrixF64(ValueMatrix::DMatrix(matrix)) = decoded_output else {
        panic!("dynamic matrix addition must return a dynamic f64 matrix");
    };
    let matrix = matrix.borrow();
    assert_eq!((matrix.nrows(), matrix.ncols()), (5, 5));
    assert!(matrix.iter().all(|value| *value == 26.0));
    Ok(())
}
