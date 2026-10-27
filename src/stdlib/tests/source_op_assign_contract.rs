#![cfg(all(feature = "full_compiler", not(feature = "no_std")))]

use mech_core::ValueData;
use mech_core::snapshot::SequenceView;

mod support;

use support::evaluate;

#[test]
fn source_map_assignment_rejects_missing_keys() {
    for setup in ["", "counts = map/insert(counts, \"existing\", 7u64)\n"] {
        let source = format!(
            "+> map\n~counts<{{string:u64}}> := map/new()\n{setup}counts{{\"é\"}} = 1u64\ncounts\n"
        );
        assert!(
            matches!(
                evaluate(&source),
                Err(support::CanonicalEvaluationError::Execution(
                    mech_engine::resident::ResidentExecutionError::Kernel {
                        error: mech_core::ResidentKernelError::InvalidInput,
                        ..
                    }
                ))
            ),
            "accepted missing-key assignment or returned another error: {source}"
        );
    }
}

#[test]
fn source_map_insert_adds_keys_and_assignment_updates_existing_numeric_keys() {
    let output = evaluate("+> map\n~labels<{u64:string}> := map/new()\nlabels = map/insert(labels, 7u64, \"α\")\nlabels = map/insert(labels, 9u64, \"β\")\nlabels{7u64} = \"γ\"\nlabels\n").unwrap();
    let ValueData::Map(map) = output.data() else {
        panic!("expected map")
    };
    assert_eq!(map.entries().len(), 2);
    for (entry, (key, value)) in map.entries().iter().zip([(7, "γ"), (9, "β")]) {
        assert!(matches!(entry.key().data(), ValueData::U64(actual) if *actual == key));
        assert!(matches!(entry.value(), ValueData::String(actual) if actual.as_ref() == value));
    }
}

fn matrix_after_indexed_add_assignment(selector: &str, value: &str) -> Vec<f64> {
    let source =
        format!("~x := [1.0 2.0 3.0; 4.0 5.0 6.0; 7.0 8.0 9.0]; x{selector} += {value}; x");
    let output = evaluate(&source).unwrap_or_else(|error| panic!("{selector}: {error:?}"));
    let ValueData::Matrix(matrix) = output.data() else {
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

#[test]
fn source_map_constructor_requires_a_map_result_kind_and_key_value_context() {
    for source in [
        "+> map\nmap/new()\n",
        "+> map\nx<u64> := map/new()\nx\n",
        "+> map\nx<{string:u64}> := map/new(1u64)\nx\n",
    ] {
        assert!(
            evaluate(source).is_err(),
            "accepted invalid constructor: {source}"
        );
    }
    let output = evaluate("+> map\nx<{u64:string}> := map/new()\nx\n").unwrap();
    assert!(matches!(output.data(), ValueData::Map(map) if map.entries().is_empty()));
}

#[test]
fn source_map_assignment_preserves_fixed_literal_cardinality() {
    let output = evaluate("~counts := {\"a\": 1u64}\ncounts{\"a\"} = 3u64\ncounts\n").unwrap();
    assert!(matches!(output.data(), ValueData::Map(map)
        if map.entries().len() == 1 && matches!(map.entries()[0].value(), ValueData::U64(3))));
    for source in [
        "~counts<{string:u64}> := {:}\ncounts{\"a\"} = 1u64\ncounts\n",
        "~counts := {\"a\": 1u64}\ncounts{\"b\"} = 2u64\ncounts\n",
    ] {
        assert!(
            evaluate(source).is_err(),
            "grew a fixed-cardinality literal: {source}"
        );
    }
}
