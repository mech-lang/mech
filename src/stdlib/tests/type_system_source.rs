#![cfg(all(feature = "full_compiler", not(feature = "no_std")))]

use mech_core::snapshot::SequenceView;
use mech_core::{BuiltinScalarKind, SourceTypeAuthority, ValueData};
use std::collections::BTreeSet;

mod support;

use support::{current_extents, descriptor, evaluate, resolved_type};

fn assert_source_failure(error: &support::CanonicalEvaluationError, expected_message: &str) {
    let error = error
        .source()
        .unwrap_or_else(|| panic!("expected a canonical source error, found {error:?}"));
    assert!(
        error.code.starts_with("source-semantics/"),
        "unexpected canonical error code: {error:?}",
    );
    assert!(
        error.message.contains(expected_message),
        "canonical failure lost {expected_message:?}: {error:?}",
    );
    assert!(
        error.anchor.range.start < error.anchor.range.end,
        "canonical failure lost its exact source range: {error:?}",
    );
}

fn assert_scalar_kind(source: &str, expected: BuiltinScalarKind) {
    let value = evaluate(source).unwrap_or_else(|error| panic!("{source}: {error:?}"));
    assert_eq!(resolved_type(&value).kind(), &expected.kind_expr());
}

#[test]
fn source_numeric_promotions_are_semantically_selected() {
    for (source, expected) in [
        ("1<u8> + 2<u16>", BuiltinScalarKind::U16),
        ("1<u8> + 2<i8>", BuiltinScalarKind::I16),
        ("1<u32> + 2<f32>", BuiltinScalarKind::F64),
    ] {
        assert_scalar_kind(source, expected);
    }

    let error = evaluate("1<i64> + 2<f64>").unwrap_err();
    assert_source_failure(&error, "math/add");
}

#[test]
fn semantic_formula_add_routes_strings_and_numbers() {
    let text = evaluate("\"left\" + \"right\"").unwrap();
    assert!(matches!(
        text.data(),
        ValueData::String(value) if value.as_ref() == "leftright"
    ));
    let number = evaluate("1<u8> + 2<u16>").unwrap();
    assert!(matches!(number.data(), ValueData::U16(3)));
}

#[test]
fn source_explicit_casts_use_checked_conversion_plans() {
    assert_scalar_kind("value<i64> := 3; value<f64>", BuiltinScalarKind::F64);

    let truncated = evaluate("value := 3.9; value<i32>").unwrap();
    assert!(matches!(truncated.data(), ValueData::I32(3)));

    let error = evaluate("value := 2147483648.0; value<i32>").unwrap_err();
    assert_eq!(
        error.source().map(|error| error.code),
        Some("source-semantics/constant-conversion-failed"),
    );
    assert_source_failure(&error, "outside the target range");

    assert_scalar_kind("value := true; value<string>", BuiltinScalarKind::String);
    assert_scalar_kind("value := 42<u64>; value<string>", BuiltinScalarKind::String);
}

#[test]
fn source_matrix_promotion_preserves_shape_and_element_order() {
    let value = evaluate("[1<f32> 2<f32>] + [3<f64> 4<f64>]").unwrap();
    let ValueData::Matrix(matrix) = value.data() else {
        panic!("promoted matrix expression must remain a matrix")
    };
    let SequenceView::F64(elements) = matrix.elements() else {
        panic!("promoted matrix expression must use f64 elements")
    };
    assert_eq!(
        elements
            .iter()
            .map(|value| value.to_f64())
            .collect::<Vec<_>>(),
        vec![4.0, 6.0],
    );
}

#[test]
fn transposed_live_linear_range_participates_in_row_broadcast() {
    let selected = evaluate("truth := [2.0; 3.0; 0.5]; truth[1..=2]'").unwrap();
    let descriptor = descriptor(&selected);
    let mech_core::SchemaBody::Matrix { dimensions, .. } = descriptor.schema().body() else {
        panic!("transposed selection must retain a matrix schema");
    };
    assert_eq!(dimensions[0], mech_core::DimensionExpr::Constant(1));
    assert_eq!(dimensions[1], mech_core::DimensionExpr::Constant(2));

    let output = evaluate(
        "cameras := [1.0 1.0; 5.0 1.0; 5.0 5.0; 1.0 5.0];
         truth := [2.0; 3.0; 0.5]; cameras - truth[1..=2]'",
    )
    .unwrap();
    assert_eq!(current_extents(&output).as_ref(), &[4, 2]);
    let ValueData::Matrix(matrix) = output.data() else {
        panic!("expected f64 camera offsets")
    };
    let SequenceView::F64(elements) = matrix.elements() else {
        panic!("expected f64 camera offsets")
    };
    let values = elements
        .iter()
        .map(|value| value.to_f64())
        .collect::<Vec<_>>();
    assert_eq!(values, [-1.0, -2.0, 3.0, -2.0, 3.0, 2.0, -1.0, 2.0]);
}

#[test]
fn rolling_path_elementwise_update_preserves_live_shape_relations() {
    let output = evaluate(
        "samples := 1..=4;
         path := (samples' * 0.0) ** [1.0 1.0] + [10.0 20.0];
         advanced := [path[2..=4,:]; 30.0 40.0];
         path + 1.0 * (advanced - path)",
    )
    .unwrap();
    assert_eq!(current_extents(&output).as_ref(), &[4, 2]);
    let ValueData::Matrix(matrix) = output.data() else {
        panic!("expected f64 path points")
    };
    let SequenceView::F64(elements) = matrix.elements() else {
        panic!("expected f64 path points")
    };
    let values = elements
        .iter()
        .map(|value| value.to_f64())
        .collect::<Vec<_>>();
    assert_eq!(values, [10.0, 20.0, 10.0, 20.0, 10.0, 20.0, 30.0, 40.0]);
    assert!(evaluate("[1.0 2.0; 3.0 4.0] - [1.0 2.0 3.0; 4.0 5.0 6.0]").is_err());
}

#[test]
fn source_matrix_dimensions_are_checked_and_preserved() {
    let transposed = evaluate("[1.0 2.0 3.0; 4.0 5.0 6.0]'").unwrap();
    assert_eq!(current_extents(&transposed).as_ref(), &[3, 2],);

    let product = evaluate("[1.0 2.0; 3.0 4.0] ** [5.0; 6.0]").unwrap();
    assert_eq!(current_extents(&product).as_ref(), &[2, 1]);
    let ValueData::Matrix(matrix) = product.data() else {
        panic!("matrix product must remain a matrix")
    };
    let SequenceView::F64(elements) = matrix.elements() else {
        panic!("matrix product must preserve f64 elements")
    };
    assert_eq!(
        elements
            .iter()
            .map(|value| value.to_f64())
            .collect::<Vec<_>>(),
        vec![17.0, 39.0],
    );

    let mismatch = evaluate("[1.0 2.0] + [1.0; 2.0]").unwrap_err();
    assert_source_failure(&mismatch, "math/add");

    let product_mismatch = evaluate("[1.0 2.0] ** [1.0; 2.0; 3.0]").unwrap_err();
    assert_source_failure(&product_mismatch, "matrix/matmul");

    let rectangular_solve = evaluate("[1.0 2.0 3.0; 4.0 5.0 6.0] \\ [1.0; 2.0]").unwrap_err();
    assert_source_failure(&rectangular_solve, "matrix/solve");
}

#[test]
fn source_variadic_templates_cover_large_concat_and_exact_sets() {
    let row = format!("[{}]", vec!["1.0"; 40].join(" "));
    let concatenated = evaluate(&row).unwrap();
    assert_eq!(current_extents(&concatenated).as_ref(), &[1, 40],);

    let set = evaluate("{1<u8>, 2<u8>, 3<u8>}").unwrap();
    assert!(matches!(
        resolved_type(&set).kind(),
        mech_core::KindExpr::Set {
            cardinality: mech_core::DimensionExpr::Constant(3),
            ..
        }
    ));
}

#[test]
fn strict_equality_never_inserts_a_conversion() {
    let error = evaluate("1<u8> === 1<u16>").unwrap_err();
    assert_source_failure(&error, "compare/seq");

    let ordinary = evaluate("1<u8> == 1<u16>").unwrap();
    assert!(matches!(ordinary.data(), ValueData::Bool(true)));

    let shape_error = evaluate("[1.0] === [1.0 2.0]").unwrap_err();
    assert_source_failure(&shape_error, "compare/seq");
}

#[test]
fn ordering_and_keyability_are_semantic_constraints() {
    let complex = evaluate("(1 + 2i) < (2 + 3i)").unwrap_err();
    assert_source_failure(&complex, "Ordered");

    let member = evaluate("2<u8> ∈ {1<u8>, 2<u8>}").unwrap();
    assert!(matches!(member.data(), ValueData::Bool(true)));
}

#[test]
fn user_function_boundaries_use_type_system_conversions() {
    let value =
        evaluate("```mech\nwiden(x<f64>) = y<f64> :=\n  y := x.\nwiden(7<i32>)\n```").unwrap();
    assert!(matches!(
        value.data(),
        ValueData::F64(value) if value.to_f64() == 7.0
    ));

    let output =
        evaluate("```mech\nwiden-output(x<i32>) = y<f64> :=\n  y := x.\nwiden-output(9<i32>)\n```")
            .unwrap();
    assert!(matches!(
        output.data(),
        ValueData::F64(value) if value.to_f64() == 9.0
    ));
}

#[test]
fn semantic_source_failures_retain_source_ranges() {
    for source in [
        "1<i64> + 2<f64>",
        "1<u8> === 1<u16>",
        "(1 + 2i) < (2 + 3i)",
        "[1.0 2.0] + [1.0; 2.0]",
    ] {
        let error = evaluate(source).expect_err("fixture must be rejected semantically");
        let error = error
            .source()
            .unwrap_or_else(|| panic!("{source}: expected canonical source failure"));
        assert!(
            error.anchor.range.start < error.anchor.range.end,
            "{source}: {error:?}"
        );
    }
}

#[test]
fn selected_source_catalog_is_completely_scheme_authoritative() {
    let catalog = mech_stdlib::source_catalog();
    let mut previous_name = None;
    for entry in catalog.all_specializers() {
        let SourceTypeAuthority::Schemes(declaration) = &entry.type_authority else {
            panic!(
                "named operation {} is syntax-directed",
                entry.operation.canonical_name
            )
        };
        assert!(
            !declaration.overloads.is_empty() || declaration.template.is_some(),
            "{} has neither semantic overloads nor an arity template",
            entry.operation.canonical_name,
        );
        let ids = declaration
            .overloads
            .iter()
            .map(|overload| overload.id)
            .collect::<BTreeSet<_>>();
        assert_eq!(
            ids.len(),
            declaration.overloads.len(),
            "{} repeats an overload ID",
            entry.operation.canonical_name,
        );
        if let Some(previous) = previous_name.replace(entry.operation.canonical_name.as_ref()) {
            assert_ne!(
                previous,
                entry.operation.canonical_name.as_ref(),
                "catalog names must be unique"
            );
        }
    }

    let intrinsic_operations = catalog
        .intrinsic_specializer_entries()
        .map(|entry| {
            assert_eq!(
                entry.type_authority,
                SourceTypeAuthority::SyntaxDirectedIntrinsic,
            );
            entry.operation.id
        })
        .collect::<BTreeSet<_>>();
    assert!(
        catalog
            .all_exports()
            .all(|export| !intrinsic_operations.contains(&export.operation))
    );
}

#[test]
fn representative_named_outputs_match_their_resolved_calls_exactly() {
    for source in [
        "1<u8>..=3<u8>",
        "set/insert({1<u8>, 2<u8>}, 3<u8>)",
        "set/remove({1<u8>, 2<u8>}, 1<u8>)",
        "set/union({1<u8>, 2<u8>}, {2<u8>, 3<u8>})",
        "set/intersection({1<u8>, 2<u8>}, {2<u8>, 3<u8>})",
        "set/powerset({1<u8>, 2<u8>})",
        "[1<u8> 2<u8> 3<u8>]",
        "[1<u8>; 2<u8>; 3<u8>]",
        "+> stats\nstats/sum/row([1<u64> 2<u64>; 3<u64> 4<u64>])",
        "+> combinatorics\ncombinatorics/n-choose-k([4<u64> 5<u64>], 2<u64>)",
    ] {
        let value = evaluate(source).unwrap_or_else(|error| panic!("{source}: {error:?}"));
        let _ = resolved_type(&value);
    }
}

fn assert_bool_matrix(source: &str, expected: &[bool]) {
    let value = evaluate(source).unwrap_or_else(|error| panic!("{source}: {error:?}"));
    let ValueData::Matrix(matrix) = value.data() else {
        panic!("{source}: expected an elementwise Boolean matrix")
    };
    let SequenceView::Bool(elements) = matrix.elements() else {
        panic!("{source}: expected Boolean elements")
    };
    assert_eq!(elements, expected, "{source}");
}

#[test]
fn source_same_type_matrix_comparisons_remain_elementwise() {
    for (lhs, rhs) in [
        ("[1.0 2.0; 3.0 4.0]", "[1.0 0.0; 3.0 0.0]"),
        ("[true false; false true]", "[true true; false false]"),
        ("[\"a\" \"b\"; \"c\" \"d\"]", "[\"a\" \"x\"; \"c\" \"y\"]"),
    ] {
        assert_bool_matrix(&format!("{lhs} == {rhs}"), &[true, false, true, false]);
        assert_bool_matrix(&format!("{lhs} != {rhs}"), &[false, true, false, true]);
    }
}

#[test]
fn source_boolean_broadcasts_cover_each_operation_and_operand_order() {
    let matrix = "[true false true; false true false]";
    for (other, and, or, xor) in [
        (
            "true",
            vec![true, false, true, false, true, false],
            vec![true; 6],
            vec![false, true, false, true, false, true],
        ),
        (
            "[true; false]",
            vec![true, false, true, false, false, false],
            vec![true, true, true, false, true, false],
            vec![false, true, false, false, true, false],
        ),
        (
            "[false true false]",
            vec![false, false, false, false, true, false],
            vec![true, true, true, false, true, false],
            vec![true, true, true, false, false, false],
        ),
    ] {
        for (operator, expected) in [("&&", and), ("||", or), ("xor", xor)] {
            for (lhs, rhs) in [(matrix, other), (other, matrix)] {
                let source = if operator == "xor" {
                    format!("logic/xor({lhs}, {rhs})")
                } else {
                    format!("{lhs} {operator} {rhs}")
                };
                assert_bool_matrix(&source, &expected);
            }
        }
    }
}

#[test]
fn source_rational_power_preserves_the_integral_exponent() {
    for (source, numerator, denominator) in [("3/2 ^ 2<i32>", 9, 4), ("3/2 ^ -2<i32>", 4, 9)] {
        let value = evaluate(source).unwrap_or_else(|error| panic!("{source}: {error:?}"));
        assert!(
            matches!(value.data(), ValueData::Rational64(value)
            if value.numerator() == numerator && value.denominator() == denominator),
            "{source}"
        );
    }
}
