use mech_core::matrix::Matrix;
use mech_core::{LegacyValue, Ref};
use mech_engine::{MechProgram, MechProgramConfig};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

const LITERAL: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/architecture/bytecode-v1/literal-f64.mecb"
));
const SCALAR: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/architecture/bytecode-v1/scalar-add-f64.mecb"
));
const MATRIX: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/architecture/bytecode-v1/dynamic-matrix-add-f64.mecb"
));
const STRING: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/architecture/bytecode-v1/string.mecb"
));

fn run(bytecode: &[u8]) -> LegacyValue {
    MechProgram::with_function_catalog(MechProgramConfig::default(), mech_stdlib::runtime_catalog())
        .run_bytecode(bytecode)
        .expect("official bytecode-v1 fixture must execute in WASM")
}

#[wasm_bindgen_test]
fn official_literal_scalar_matrix_and_string_fixtures_execute() {
    assert_eq!(run(LITERAL), LegacyValue::F64(Ref::new(42.0)));
    assert_eq!(run(SCALAR), LegacyValue::F64(Ref::new(3.0)));
    assert_eq!(
        run(MATRIX),
        LegacyValue::MatrixF64(Matrix::from_vec(vec![26.0; 25], 5, 5)),
    );
    assert_eq!(
        run(STRING),
        LegacyValue::String(Ref::new("bytecode-v1".to_owned())),
    );
}
