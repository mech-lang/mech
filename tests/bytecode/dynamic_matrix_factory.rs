use mech_bytecode::CompileCtx;
use mech_core::structures::Matrix as ValueMatrix;
use mech_core::{DecodedInstr, MResult, ParsedProgram, ToMatrix, Value, hash_str};
use mech_program::{MechProgram, MechProgramConfig};

#[test]
fn dynamic_matrix_addition_bytecode_reconstructs_from_full_runtime() -> MResult<()> {
    const FACTORY_NAME: &str = "AddMDMD<f64>";

    let factory_id = hash_str(FACTORY_NAME);
    let arguments = vec![
        Value::MatrixF64(<f64 as ToMatrix>::to_matrixd(
            vec![1.0, 3.0, 2.0, 4.0],
            2,
            2,
        )),
        Value::MatrixF64(<f64 as ToMatrix>::to_matrixd(
            vec![5.0, 7.0, 6.0, 8.0],
            2,
            2,
        )),
    ];

    let mut source = MechProgram::new(MechProgramConfig::default());
    source.load_full_stdlib();
    let add_specializer = {
        let functions = source.interpreter().functions();
        let functions = functions.borrow();
        functions
            .function_compilers
            .get(&hash_str("math/add"))
            .cloned()
            .expect("full source-specializer table must contain math/add")
    };
    let function = add_specializer.compile(&arguments)?;

    let mut context = CompileCtx::new();
    function.compile(&mut context)?;
    let bytecode = context.compile()?;
    let parsed = ParsedProgram::from_bytes(&bytecode)?;
    let operation_ids = parsed
        .instrs
        .iter()
        .filter_map(|instruction| match instruction {
            DecodedInstr::BinOp { fxn_id, .. } => Some(*fxn_id),
            DecodedInstr::ConstLoad { .. } | DecodedInstr::Ret { .. } => None,
            other => panic!("dynamic matrix addition emitted unexpected instruction {other:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(operation_ids, vec![factory_id]);

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
    assert_eq!((matrix.nrows(), matrix.ncols()), (2, 2));
    assert_eq!(matrix[(0, 0)], 6.0);
    assert_eq!(matrix[(0, 1)], 8.0);
    assert_eq!(matrix[(1, 0)], 10.0);
    assert_eq!(matrix[(1, 1)], 12.0);
    Ok(())
}
