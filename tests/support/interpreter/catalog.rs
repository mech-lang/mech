use std::sync::Arc;

use mech::{MechProgram, MechProgramConfig};
use mech_core::{
    FunctionCatalogBuilder, FunctionSpecializer, LegacyValue, MResult, MechFunction, OperationId,
    hash_str,
};

const ADD_SOURCE: &str = "result := 1.0 + 2.0\nresult";

fn standard_program() -> MechProgram {
    MechProgram::with_function_catalog(MechProgramConfig::default(), mech::stdlib::source_catalog())
}

struct UnreachableSpecializer;

impl FunctionSpecializer for UnreachableSpecializer {
    fn specialize(&self, _: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
        panic!("non-visible catalog operation was specialized")
    }
}

fn assert_math_add_is_catalog_owned(program: &MechProgram) {
    assert!(
        program
            .function_catalog()
            .specializer(OperationId::from_name("math/add"))
            .is_some(),
        "standard catalog must contain math/add",
    );
}

fn dereference(value: LegacyValue) -> LegacyValue {
    match value {
        LegacyValue::MutableReference(reference) => dereference(reference.borrow().clone()),
        LegacyValue::Typed(value, _) => dereference(*value),
        value => value,
    }
}

fn assert_f64(value: LegacyValue, expected: f64) {
    let value = dereference(value);
    let LegacyValue::F64(actual) = value else {
        panic!("expected f64 {expected}, got {value:?}");
    };
    assert_eq!(*actual.borrow(), expected);
}

#[test]
fn standard_catalog_source_addition_uses_catalog_specializer() {
    let mut program = standard_program();
    assert_math_add_is_catalog_owned(&program);

    let result = program
        .run_string(ADD_SOURCE)
        .expect("standard catalog must specialize source math/add");

    assert_f64(result, 3.0);
}

#[test]
fn explicitly_injected_catalog_source_addition_uses_catalog_specializer() {
    let catalog = mech::stdlib::source_catalog();
    let mut program =
        MechProgram::with_function_catalog(MechProgramConfig::default(), Arc::clone(&catalog));
    assert!(Arc::ptr_eq(program.function_catalog(), &catalog));

    assert_math_add_is_catalog_owned(&program);
    let result = program
        .run_string(ADD_SOURCE)
        .expect("explicitly injected catalog must specialize source math/add");

    assert_f64(result, 3.0);
}

#[test]
fn empty_catalog_source_addition_reports_named_operation_unavailable() {
    let catalog = Arc::new(FunctionCatalogBuilder::new().build().unwrap());
    let mut program = MechProgram::with_function_catalog(MechProgramConfig::default(), catalog);
    let error = program.run_string(ADD_SOURCE).unwrap_err();
    assert_eq!(error.kind_name(), "FunctionOperationUnavailable");
    assert_eq!(
        error.kind_message(),
        "function operation `math/add` (0x00cc529041cb60c3) is unavailable in the catalog",
    );
}

#[test]
fn non_visible_catalog_operation_reports_its_name_and_id() {
    let mut builder = FunctionCatalogBuilder::new();
    builder
        .insert_specializer("math/add", Arc::new(UnreachableSpecializer))
        .unwrap();
    let catalog = Arc::new(builder.build().unwrap());
    let mut program = MechProgram::with_function_catalog(MechProgramConfig::default(), catalog);

    let error = program.run_string(ADD_SOURCE).unwrap_err();
    assert_eq!(error.kind_name(), "FunctionOperationNotVisible");
    assert_eq!(
        error.kind_message(),
        "function operation `math/add` (0x00cc529041cb60c3) is not visible in this program",
    );
}

#[test]
fn named_math_add_uses_catalog() {
    let mut program = standard_program();
    assert_math_add_is_catalog_owned(&program);

    let result = program
        .run_string("math/add(1.0, 2.0)")
        .expect("named math/add must resolve through the catalog");

    assert_f64(result, 3.0);
}

#[test]
fn user_definition_shadows_named_catalog_binding_but_not_the_add_operator() {
    let mut program = standard_program();
    let result = program
        .run_string(
            r#"math/add(left<f64>, right<f64>) => <f64>
  | * => 40.0.
named := math/add(1.0, 2.0)
result := named + 2.0
result"#,
        )
        .expect("user named-call precedence must not affect syntax operators");

    assert_f64(result, 42.0);
}

#[test]
fn missing_named_function_returns_the_structured_resolver_error() {
    const NAME: &str = "test/missing";
    let catalog = Arc::new(FunctionCatalogBuilder::new().build().unwrap());
    let mut program = MechProgram::with_function_catalog(MechProgramConfig::default(), catalog);

    let error = program.run_string("test/missing(1.0)").unwrap_err();

    assert_eq!(error.kind_name(), "MissingFunction");
    assert_eq!(
        error.kind_message(),
        format!("Function with id {} not found", hash_str(NAME)),
    );
}

#[test]
fn unbound_catalog_specializer_cannot_rescue_a_named_call() {
    const NAME: &str = "test/unbound";

    let mut builder = FunctionCatalogBuilder::new();
    builder
        .insert_specializer(NAME, Arc::new(UnreachableSpecializer))
        .unwrap();
    let catalog = Arc::new(builder.build().unwrap());
    let mut program = MechProgram::with_function_catalog(MechProgramConfig::default(), catalog);

    let error = program.run_string("test/unbound(1.0)").unwrap_err();

    assert_eq!(error.kind_name(), "MissingFunction");
    assert_eq!(
        error.kind_message(),
        format!("Function with id {} not found", hash_str(NAME)),
    );
}
