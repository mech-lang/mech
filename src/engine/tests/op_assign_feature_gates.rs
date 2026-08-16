#[cfg(any(feature = "math_mul_assign", feature = "math_div_assign"))]
use mech_core::{FunctionCatalogBuilder, NoMechExecutionServices};
#[cfg(any(feature = "math_mul_assign", feature = "math_div_assign"))]
use mech_engine::{CompilerPlanningConfig, CompilerPlanningProgram};
#[cfg(any(feature = "math_mul_assign", feature = "math_div_assign"))]
use std::sync::Arc;

#[cfg(any(feature = "math_mul_assign", feature = "math_div_assign"))]
fn unavailable_operation(source: &str) -> String {
    let tree = mech_syntax::parser::parse(source).unwrap();
    let mut builder = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_runtime(&mut builder).unwrap();
    mech_engine::install_intrinsic_source(&mut builder).unwrap();
    let catalog = Arc::new(builder.build().unwrap());
    let mut program =
        CompilerPlanningProgram::with_function_catalog(CompilerPlanningConfig::default(), catalog);
    let mut services = NoMechExecutionServices;
    program
        .plan_tree_with_services(&tree, &mut services)
        .unwrap_err()
        .kind_message()
}

#[cfg(feature = "math_mul_assign")]
#[test]
fn whole_mul_assignment_selects_mul_operation_only() {
    let error = unavailable_operation(
        "~x := 6.0\n\
       y := 3.0\n\
       x *= y\n\
       x",
    );
    assert!(error.contains("math/mul-assign"), "{error}");
    assert!(!error.contains("math/div-assign"), "{error}");
}

#[cfg(feature = "math_div_assign")]
#[test]
fn whole_div_assignment_selects_div_operation_only() {
    let error = unavailable_operation(
        "~x := 6.0\n\
       y := 3.0\n\
       x /= y\n\
       x",
    );
    assert!(error.contains("math/div-assign"), "{error}");
    assert!(!error.contains("math/mul-assign"), "{error}");
}
