#![cfg(all(feature = "full_compiler", not(feature = "no_std")))]

use mech_core::snapshot::SequenceView;
use mech_core::{NoMechExecutionServices, ValueData};
use mech_engine::program::{CompilerPlanningConfig, CompilerPlanningProgram};

fn matrix_after_indexed_add_assignment(selector: &str, value: &str) -> Vec<f64> {
    let source =
        format!("~x := [1.0 2.0 3.0; 4.0 5.0 6.0; 7.0 8.0 9.0]; x{selector} += {value}; x");
    let tree = mech_syntax::parser::parse(&source).unwrap();
    let mut program = CompilerPlanningProgram::with_function_catalog(
        CompilerPlanningConfig::default(),
        mech_stdlib::source_catalog(),
    );
    let output = program
        .plan_tree_with_services(&tree, &mut NoMechExecutionServices)
        .unwrap_or_else(|error| panic!("{selector}: {error:?}"))
        .expect("indexed assignment source returns its matrix output");
    let snapshot = output.snapshot().unwrap();
    let ValueData::Matrix(matrix) = snapshot.data() else {
        panic!("expected an f64 matrix add-assignment result");
    };
    let SequenceView::F64(elements) = matrix.elements() else {
        panic!("expected exact f64 matrix storage");
    };
    elements.iter().map(|element| element.to_f64()).collect()
}

#[test]
fn source_indexed_all_selector_preserves_applicable_add_assignment_layouts() {
    for (selector, value, expected) in [
        (
            "[2,:]",
            "10.0",
            vec![1.0, 2.0, 3.0, 14.0, 15.0, 16.0, 7.0, 8.0, 9.0],
        ),
        (
            "[1..=2,:]",
            "10.0",
            vec![11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 7.0, 8.0, 9.0],
        ),
        (
            "[[1 3],:]",
            "10.0",
            vec![11.0, 12.0, 13.0, 4.0, 5.0, 6.0, 17.0, 18.0, 19.0],
        ),
        (
            "[[true false true],:]",
            "[10.0 10.0 10.0; 10.0 10.0 10.0; 10.0 10.0 10.0]",
            vec![11.0, 12.0, 13.0, 4.0, 5.0, 6.0, 17.0, 18.0, 19.0],
        ),
    ] {
        assert_eq!(
            matrix_after_indexed_add_assignment(selector, value),
            expected,
            "{selector}",
        );
    }
}
