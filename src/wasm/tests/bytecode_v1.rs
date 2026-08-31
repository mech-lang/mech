use mech_core::{Value, ValueData, snapshot::SequenceView};
use mech_runtime::{ResidentDurabilityPolicy, RuntimeBuilder};
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

fn run(bytecode: &[u8]) -> Value {
    let mut runtime = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::runtime_catalog())
        .build()
        .expect("official bytecode-v1 fixture runtime must build in WASM");
    runtime
        .load_bytecode_program(bytecode, ResidentDurabilityPolicy::Volatile)
        .expect("official bytecode-v1 fixture must execute residently in WASM")
        .initial_value
        .into_value()
}

#[wasm_bindgen_test]
fn official_literal_scalar_matrix_and_string_fixtures_execute() {
    assert!(matches!(run(LITERAL).data(), ValueData::F64(value) if value.to_f64() == 42.0));
    assert!(matches!(run(SCALAR).data(), ValueData::F64(value) if value.to_f64() == 3.0));
    let matrix = run(MATRIX);
    let ValueData::Matrix(matrix) = matrix.data() else {
        panic!("matrix fixture must return a canonical matrix")
    };
    assert!(
        matches!(matrix.elements(), SequenceView::F64(values) if values.len() == 25 && values.iter().all(|value| value.to_f64() == 26.0))
    );
    assert!(
        matches!(run(STRING).data(), ValueData::String(value) if value.as_ref() == "bytecode-v1")
    );
}
