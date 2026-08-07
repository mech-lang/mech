use std::sync::Arc;

use mech::{
    FunctionSystem, LegacyFunctionBoundaryBuilder, MechProgram, MechProgramConfig,
    default_function_system,
};
use mech_core::{
    FunctionCatalogBuilder, FunctionSpecializer, MResult, MechFunction, NativeFunctionCompiler,
    OperationId, Value, hash_str,
};

const ADD_SOURCE: &str = "result := 1.0 + 2.0\nresult";

struct LegacyCompilerMustNotRun;

impl NativeFunctionCompiler for LegacyCompilerMustNotRun {
    fn compile(&self, _: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
        panic!("catalog-owned named operation reached the legacy compiler")
    }
}

struct UnreachableSpecializer;

impl FunctionSpecializer for UnreachableSpecializer {
    fn specialize(&self, _: &[Value]) -> MResult<Box<dyn MechFunction>> {
        panic!("non-visible catalog operation was specialized")
    }
}

fn remove_legacy_math_add_compiler(program: &MechProgram) {
    let operation = hash_str("math/add");
    let functions = program.interpreter().functions();
    let removed = functions.borrow_mut().function_compilers.remove(&operation);
    assert!(
        removed.is_some(),
        "source proof requires the legacy math/add compiler to be loaded first",
    );
}

fn dereference(value: Value) -> Value {
    match value {
        Value::MutableReference(reference) => dereference(reference.borrow().clone()),
        Value::Typed(value, _) => dereference(*value),
        value => value,
    }
}

fn assert_f64(value: Value, expected: f64) {
    let value = dereference(value);
    let Value::F64(actual) = value else {
        panic!("expected f64 {expected}, got {value:?}");
    };
    assert_eq!(*actual.borrow(), expected);
}

#[test]
fn standard_catalog_source_addition_does_not_use_legacy_compiler() {
    let mut program = MechProgram::new(MechProgramConfig::default());
    assert!(
        program
            .function_system()
            .legacy_boundary()
            .owns_operation(OperationId::from_name("math/add"))
    );
    program.load_full_stdlib();
    remove_legacy_math_add_compiler(&program);

    let result = program
        .run_string(ADD_SOURCE)
        .expect("standard catalog must specialize source math/add");

    assert_f64(result, 3.0);
}

#[test]
fn explicitly_injected_catalog_source_addition_does_not_use_legacy_compiler() {
    let function_system = default_function_system();
    let catalog = Arc::clone(function_system.catalog());
    let legacy_boundary = Arc::clone(function_system.legacy_boundary());
    let mut program =
        MechProgram::with_function_system(MechProgramConfig::default(), function_system);
    assert!(Arc::ptr_eq(program.function_catalog(), &catalog));
    assert!(Arc::ptr_eq(
        program.function_system().legacy_boundary(),
        &legacy_boundary,
    ));

    program.load_full_stdlib();
    remove_legacy_math_add_compiler(&program);
    let result = program
        .run_string(ADD_SOURCE)
        .expect("explicitly injected catalog must specialize source math/add");

    assert_f64(result, 3.0);
}

#[test]
fn empty_catalog_source_addition_reports_named_operation_unavailable() {
    let catalog = Arc::new(FunctionCatalogBuilder::new().build().unwrap());
    let mut program = MechProgram::with_function_catalog(MechProgramConfig::default(), catalog);
    program.load_full_stdlib();
    remove_legacy_math_add_compiler(&program);

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
    program.load_full_stdlib();

    let error = program.run_string(ADD_SOURCE).unwrap_err();
    assert_eq!(error.kind_name(), "FunctionOperationNotVisible");
    assert_eq!(
        error.kind_message(),
        "function operation `math/add` (0x00cc529041cb60c3) is not visible in this program",
    );
}

#[test]
fn named_math_add_uses_catalog_without_legacy_compiler() {
    let mut program = MechProgram::new(MechProgramConfig::default());
    program.load_full_stdlib();
    remove_legacy_math_add_compiler(&program);

    let result = program
        .run_string("math/add(1.0, 2.0)")
        .expect("named math/add must resolve through the catalog");

    assert_f64(result, 3.0);
}

#[test]
fn empty_catalog_and_boundary_allow_named_legacy_fallback() {
    let catalog = Arc::new(FunctionCatalogBuilder::new().build().unwrap());
    let function_system = FunctionSystem::from_catalog(catalog);
    assert!(
        !function_system
            .legacy_boundary()
            .owns_operation(OperationId::from_name("math/add"))
    );
    let mut program =
        MechProgram::with_function_system(MechProgramConfig::default(), function_system);
    program.load_full_stdlib();

    let result = program
        .run_string("math/add(1.0, 2.0)")
        .expect("an unclaimed named operation must remain eligible for legacy fallback");

    assert_f64(result, 3.0);
}

#[test]
fn claimed_named_operation_blocks_legacy_fallback_without_special_cases() {
    const NAME: &str = "test/claimed";

    let catalog = Arc::new(FunctionCatalogBuilder::new().build().unwrap());
    let mut boundary = LegacyFunctionBoundaryBuilder::new();
    boundary.claim_operation(NAME);
    let function_system = FunctionSystem::new(catalog, Arc::new(boundary.build()));
    let mut program =
        MechProgram::with_function_system(MechProgramConfig::default(), function_system);
    program
        .interpreter()
        .functions()
        .borrow_mut()
        .function_compilers
        .insert(hash_str(NAME), Arc::new(LegacyCompilerMustNotRun));

    let error = program.run_string("test/claimed(1.0)").unwrap_err();
    assert_eq!(error.kind_name(), "FunctionOperationUnavailable");
    assert_eq!(
        error.kind_message(),
        format!(
            "function operation `test/claimed` (0x{:016x}) is unavailable in the catalog",
            OperationId::from_name(NAME).raw(),
        ),
    );
}
