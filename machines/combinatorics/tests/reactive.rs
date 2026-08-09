#![cfg(all(
    feature = "compiler",
    feature = "source",
    feature = "n_choose_k",
    feature = "f64",
    feature = "matrixd"
))]

use std::sync::Arc;

use mech_core::{
    BytecodeCompilerContext, FunctionCatalog, FunctionCatalogBuilder, MResult, MechError,
    MechErrorKind, MechFunctionCompiler, MechFunctionImpl, Ref, Register, LegacyValue, hash_str,
    structures::matrix::Matrix,
};
use mech_engine::{MechProgram, MechProgramConfig, ProgramInputId, ProgramInputUpdate};
use nalgebra::DMatrix;

fn owner_source_catalog() -> Arc<FunctionCatalog> {
    let mut builder = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_runtime(&mut builder).unwrap();
    mech_engine::install_intrinsic_source(&mut builder).unwrap();
    mech_combinatorics::install_runtime(&mut builder).unwrap();
    mech_combinatorics::install_source(&mut builder).unwrap();
    Arc::new(builder.build().unwrap())
}

fn unwrap_value(value: LegacyValue) -> LegacyValue {
    match value {
        LegacyValue::MutableReference(reference) => unwrap_value(reference.borrow().clone()),
        LegacyValue::Typed(value, _) => unwrap_value(*value),
        value => value,
    }
}

fn symbol_value(program: &MechProgram, name: &str) -> LegacyValue {
    let symbols = program.interpreter().symbols();
    let value = symbols
        .borrow()
        .get(hash_str(name))
        .unwrap_or_else(|| panic!("missing symbol `{name}`"));
    value.borrow().clone()
}

fn f64_value(value: LegacyValue) -> f64 {
    let LegacyValue::F64(value) = unwrap_value(value) else {
        panic!("expected f64 value");
    };
    *value.borrow()
}

fn dynamic_f64_matrix(value: LegacyValue) -> Ref<DMatrix<f64>> {
    let LegacyValue::MatrixF64(Matrix::DMatrix(matrix)) = unwrap_value(value) else {
        panic!("expected a dynamic f64 matrix");
    };
    matrix
}

fn ensure_input(program: &mut MechProgram, name: &str, value: LegacyValue) -> ProgramInputId {
    program
        .ensure_input(program.interpreter().id, hash_str(name), name, value)
        .unwrap()
}

fn update_k(program: &mut MechProgram, input: ProgramInputId, value: f64) -> MResult<()> {
    program
        .update_inputs_and_advance_turn(&[ProgramInputUpdate {
            input,
            value: LegacyValue::F64(Ref::new(value)),
        }])
        .map(|_| ())
}

fn assert_matrix(matrix: &Ref<DMatrix<f64>>, rows: usize, cols: usize, expected: &[f64]) {
    let matrix = matrix.borrow();
    assert_eq!((matrix.nrows(), matrix.ncols()), (rows, cols));
    assert_eq!(matrix.as_slice(), expected);
}

fn reactive_program() -> (MechProgram, ProgramInputId) {
    let mut program = MechProgram::with_function_catalog(
        MechProgramConfig::default(),
        owner_source_catalog(),
    );
    ensure_input(
        &mut program,
        "n",
        LegacyValue::MatrixF64(Matrix::DMatrix(Ref::new(DMatrix::from_vec(
            1,
            3,
            vec![1.0, 2.0, 3.0],
        )))),
    );
    let k = ensure_input(&mut program, "k", LegacyValue::F64(Ref::new(1.0)));
    program
        .run_string(
            "+> combinatorics/n-choose-k\n\
             result := n-choose-k(n, k)\n\
             result",
        )
        .unwrap();
    (program, k)
}

#[test]
fn invalid_initial_n_choose_k_selection_is_reported() {
    for invalid in [0.0, 4.0] {
        let mut program = MechProgram::with_function_catalog(
            MechProgramConfig::default(),
            owner_source_catalog(),
        );
        ensure_input(
            &mut program,
            "n",
            LegacyValue::MatrixF64(Matrix::DMatrix(Ref::new(DMatrix::from_vec(
                1,
                3,
                vec![1.0, 2.0, 3.0],
            )))),
        );
        ensure_input(&mut program, "k", LegacyValue::F64(Ref::new(invalid)));

        let error = program
            .run_string(
                "+> combinatorics/n-choose-k\n\
                 result := n-choose-k(n, k)\n\
                 result",
            )
            .unwrap_err();

        assert_eq!(error.kind_name(), "NChooseKMatrixSelectionInvalid");
    }
}

#[test]
fn reactive_n_choose_k_reuses_output_and_updates_dependents() {
    let (mut program, k) = reactive_program();
    let output = dynamic_f64_matrix(symbol_value(&program, "result"));
    let output_id = output.id();
    let dependent = Ref::new(13.0);
    let solves = Ref::new(0_usize);
    register_matrix_dependent(&program, &output, dependent.clone(), solves.clone(), false);

    assert_matrix(&output, 1, 3, &[1.0, 2.0, 3.0]);
    assert_eq!(*dependent.borrow(), 13.0);

    update_k(&mut program, k, 2.0).unwrap();
    assert_eq!(output.id(), output_id);
    assert_matrix(&output, 2, 3, &[1.0, 2.0, 1.0, 3.0, 2.0, 3.0]);
    assert_eq!(*dependent.borrow(), 23.0);
    assert_eq!(*solves.borrow(), 1);

    update_k(&mut program, k, 3.0).unwrap();
    assert_eq!(output.id(), output_id);
    assert_matrix(&output, 3, 1, &[1.0, 2.0, 3.0]);
    assert_eq!(*dependent.borrow(), 31.0);
    assert_eq!(*solves.borrow(), 2);
}

#[test]
fn invalid_reactive_n_choose_k_update_rolls_back_the_input_and_output() {
    for invalid in [0.0, 4.0] {
        let (mut program, k) = reactive_program();
        let output = dynamic_f64_matrix(symbol_value(&program, "result"));
        let dependent = Ref::new(13.0);
        register_matrix_dependent(
            &program,
            &output,
            dependent.clone(),
            Ref::new(0_usize),
            false,
        );

        let error = update_k(&mut program, k, invalid).unwrap_err();

        assert_eq!(error.kind_name(), "NChooseKMatrixSelectionInvalid");
        assert_eq!(f64_value(symbol_value(&program, "k")), 1.0);
        assert_matrix(&output, 1, 3, &[1.0, 2.0, 3.0]);
        assert_eq!(*dependent.borrow(), 13.0);
    }
}

#[derive(Clone, Debug)]
struct DeliberateDependentFailure;

impl MechErrorKind for DeliberateDependentFailure {
    fn name(&self) -> &str {
        "DeliberateDependentFailure"
    }

    fn message(&self) -> String {
        "deliberate downstream reactive failure".to_owned()
    }
}

#[derive(Debug)]
struct MatrixDependent {
    input: Ref<DMatrix<f64>>,
    out: Ref<f64>,
    solves: Ref<usize>,
    fail: bool,
}

impl MechFunctionImpl for MatrixDependent {
    fn solve_result(&self) -> MResult<()> {
        let input = self.input.borrow();
        *self.out.borrow_mut() = (input.nrows() * 10 + input.ncols()) as f64;
        *self.solves.borrow_mut() += 1;
        if self.fail {
            Err(MechError::new(DeliberateDependentFailure, None))
        } else {
            Ok(())
        }
    }

    fn out(&self) -> LegacyValue {
        LegacyValue::F64(self.out.clone())
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
    }

    fn to_string(&self) -> String {
        "failing matrix dependent".to_owned()
    }
}

impl MechFunctionCompiler for MatrixDependent {
    fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

fn register_matrix_dependent(
    program: &MechProgram,
    input: &Ref<DMatrix<f64>>,
    out: Ref<f64>,
    solves: Ref<usize>,
    fail: bool,
) {
    program
        .interpreter()
        .plan()
        .register_function(
            Box::new(MatrixDependent {
                input: input.clone(),
                out,
                solves,
                fail,
            }),
            &[LegacyValue::MatrixF64(Matrix::DMatrix(input.clone()))],
        )
        .unwrap();
}

#[test]
fn downstream_failure_rolls_back_n_choose_k_and_all_dependents() {
    let (mut program, k) = reactive_program();
    let output = dynamic_f64_matrix(symbol_value(&program, "result"));
    let failing_output = Ref::new(13.0);
    register_matrix_dependent(
        &program,
        &output,
        failing_output.clone(),
        Ref::new(0_usize),
        true,
    );

    let error = update_k(&mut program, k, 2.0).unwrap_err();

    assert_eq!(error.kind_name(), "DeliberateDependentFailure");
    assert_eq!(f64_value(symbol_value(&program, "k")), 1.0);
    assert_matrix(&output, 1, 3, &[1.0, 2.0, 3.0]);
    assert_eq!(*failing_output.borrow(), 13.0);
}

#[test]
fn n_choose_k_bytecode_reconstructs_a_dynamic_matrix_output() -> MResult<()> {
    let mut source = MechProgram::with_function_catalog(
        MechProgramConfig::default(),
        owner_source_catalog(),
    );
    source.run_string(
        "+> combinatorics/n-choose-k\n\
         result := n-choose-k([1.0 2.0 3.0], 2.0)\n\
         result",
    )?;
    let bytecode = source.compile_bytecode()?;

    let mut decoded = MechProgram::with_function_catalog(
        MechProgramConfig::default(),
        owner_source_catalog(),
    );
    decoded.run_bytecode(&bytecode)?;

    let output = dynamic_f64_matrix(decoded.solve_plan()?.value);
    assert_matrix(&output, 2, 3, &[1.0, 2.0, 1.0, 3.0, 2.0, 3.0]);
    Ok(())
}
