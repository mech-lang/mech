#![cfg(feature = "dynamic-modules")]

extern crate mech_core;

#[path = "support/intrinsic_runner.rs"]
mod intrinsic_runner;

use mech_core::snapshot::SequenceView;

fn run(source: &str) -> bool {
    intrinsic_runner::run(source).is_ok()
}

#[cfg(feature = "dynamic-modules")]
fn run_matrix_n_choose_k(source: &str, expected: Vec<f64>) {
    let result = intrinsic_runner::run(source).unwrap();
    let matrix = result.matrix_view().expect("expected matrix result");
    let SequenceView::F64(actual) = matrix.elements() else {
        panic!("expected f64 matrix result");
    };
    assert_eq!(actual.len(), 2);
    assert_eq!(
        actual
            .iter()
            .map(|value| value.to_f64())
            .collect::<Vec<_>>(),
        expected
    );
}

#[cfg(feature = "dynamic-modules")]
#[test]
fn dynamic_combinatorics_item_import_works() {
    assert!(run(
        "+> combinatorics/n-choose-k\nx := n-choose-k(10.0, 2.0)"
    ));
}

#[cfg(feature = "dynamic-modules")]
#[test]
fn dynamic_combinatorics_module_import_works() {
    assert!(run(
        "+> combinatorics\nx := combinatorics/n-choose-k(10.0, 2.0)"
    ));
}

#[cfg(feature = "dynamic-modules")]
#[test]
fn dynamic_combinatorics_glob_import_works() {
    assert!(run("+> combinatorics/*\nx := n-choose-k(10.0, 2.0)"));
}

#[cfg(feature = "dynamic-modules")]
#[test]
fn dynamic_combinatorics_matrix_scalar_broadcast_works() {
    run_matrix_n_choose_k(
        "+> combinatorics/n-choose-k\nx := n-choose-k([10.0 20.0], 2.0)\nx",
        vec![45.0, 190.0],
    );
}

#[cfg(feature = "dynamic-modules")]
#[test]
fn dynamic_combinatorics_scalar_matrix_broadcast_works() {
    run_matrix_n_choose_k(
        "+> combinatorics/n-choose-k\nx := n-choose-k(10.0, [2.0 3.0])\nx",
        vec![45.0, 120.0],
    );
}

#[cfg(feature = "dynamic-modules")]
#[test]
fn dynamic_combinatorics_matrix_matrix_broadcast_works() {
    run_matrix_n_choose_k(
        "+> combinatorics/n-choose-k\nx := n-choose-k([10.0 20.0], [2.0 3.0])\nx",
        vec![45.0, 1140.0],
    );
}

#[cfg(feature = "dynamic-modules")]
#[test]
fn dynamic_combinatorics_module_import_matrix_broadcast_works() {
    run_matrix_n_choose_k(
        "+> combinatorics\nx := combinatorics/n-choose-k([10.0 20.0], 2.0)\nx",
        vec![45.0, 190.0],
    );
}

#[cfg(feature = "dynamic-modules")]
#[test]
fn dynamic_combinatorics_glob_import_matrix_broadcast_works() {
    run_matrix_n_choose_k(
        "+> combinatorics/*\nx := n-choose-k([10.0 20.0], 2.0)\nx",
        vec![45.0, 190.0],
    );
}

#[cfg(feature = "dynamic-modules")]
#[test]
fn dynamic_combinatorics_matrix_matrix_shape_mismatch_errors() {
    let result = intrinsic_runner::run(
        "+> combinatorics/n-choose-k\nx := n-choose-k([10.0 20.0], [2.0 3.0 4.0])\nx",
    );

    assert!(result.is_err());
}

#[cfg(feature = "dynamic-modules")]
#[test]
fn dynamic_combinatorics_matrix_matrix_same_cells_different_shape_errors() {
    let result = intrinsic_runner::run(
        "+> combinatorics/n-choose-k\nx := n-choose-k([10.0 20.0 30.0 40.0], [2.0 3.0; 4.0 5.0])\nx",
    );

    assert!(result.is_err());
}
