use mech_core::{MResult, RuntimeFunctionId, SchemaBody, snapshot::SequenceView};
use mech_runtime::{ResidentDurabilityPolicy, RuntimeBuilder};

fn assert_catalog_factory_owned(name: &str) {
    assert!(
        mech::stdlib::source_catalog()
            .runtime_entry(RuntimeFunctionId::from_name(name))
            .is_some(),
        "explicit catalog did not contain {name}",
    );
}

#[test]
fn dynamic_matrix_add_bytecode_uses_catalog_factory() -> MResult<()> {
    const FACTORY_NAME: &str = "AddMDMD<f64>";

    let source = "left := [1 2 3 4 5; 6 7 8 9 10; 11 12 13 14 15; 16 17 18 19 20; 21 22 23 24 25]\n\
         right := [25 24 23 22 21; 20 19 18 17 16; 15 14 13 12 11; 10 9 8 7 6; 5 4 3 2 1]\n\
         left + right";
    let bytecode = RuntimeBuilder::new()
        .function_catalog(mech::stdlib::source_catalog())
        .build_compiler()?
        .compile_source(source)?
        .into_parts()
        .1;
    assert_catalog_factory_owned(FACTORY_NAME);
    let mut runtime = RuntimeBuilder::new()
        .function_catalog(mech::stdlib::source_catalog())
        .build()?;
    let output = runtime
        .load_bytecode_program(&bytecode, ResidentDurabilityPolicy::Volatile)?
        .initial_value
        .into_value();
    let matrix = output
        .matrix_view()
        .expect("dynamic matrix addition must return a canonical matrix");
    let SequenceView::F64(values) = matrix.elements() else {
        panic!("dynamic matrix addition must return an f64 matrix");
    };
    let schemas = output
        .schemas()
        .expect("canonical matrix retains its schema arena");
    let SchemaBody::Matrix { dimensions, .. } = schemas
        .get(output.schema())
        .expect("canonical matrix schema exists")
        .body()
    else {
        panic!("expected canonical matrix schema");
    };
    assert_eq!(
        (
            output.shape().resolve_dimension(&dimensions[0]),
            output.shape().resolve_dimension(&dimensions[1]),
        ),
        (Ok(5), Ok(5)),
    );
    assert_eq!(values.len(), 25);
    assert!(values.iter().all(|value| value.to_f64() == 26.0));
    Ok(())
}
