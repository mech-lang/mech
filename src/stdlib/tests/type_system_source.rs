#![cfg(all(feature = "full_compiler", not(feature = "no_std")))]

use mech_core::snapshot::SequenceView;
use mech_core::{
    BuiltinScalarKind, MResult, NoMechExecutionServices, SourceTypeAuthority, ValueCell, ValueData,
};
use mech_engine::program::{CompilerPlanningConfig, CompilerPlanningProgram};
use std::collections::BTreeSet;

fn evaluate(source: &str) -> MResult<ValueCell> {
    let tree = mech_syntax::parser::parse(source).expect("R3 source fixture must parse");
    CompilerPlanningProgram::with_function_catalog(
        CompilerPlanningConfig::default(),
        mech_stdlib::source_catalog(),
    )
    .plan_tree_with_services(&tree, &mut NoMechExecutionServices)?
    .ok_or_else(|| {
        mech_core::MechError::new(
            mech_core::GenericError {
                msg: "R3 source fixture produced no value".into(),
            },
            None,
        )
    })
}

fn assert_scalar_kind(source: &str, expected: BuiltinScalarKind) {
    let value = evaluate(source).unwrap_or_else(|error| panic!("{source}: {error:?}"));
    assert_eq!(value.resolved_type().unwrap().kind(), &expected.kind_expr());
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
    assert_eq!(error.kind_name(), "TypeIncompatibility");
    assert!(error.kind_message().contains("math/add"));
    assert!(
        !error.tokens.is_empty(),
        "source tokens must survive typing"
    );
}

#[test]
fn semantic_formula_add_routes_strings_and_numbers() {
    let text = evaluate("\"left\" + \"right\"").unwrap();
    assert!(matches!(
        text.snapshot().unwrap().data(),
        ValueData::String(value) if value.as_ref() == "leftright"
    ));
    let number = evaluate("1<u8> + 2<u16>").unwrap();
    assert!(matches!(
        number.snapshot().unwrap().data(),
        ValueData::U16(3)
    ));
}

#[test]
fn source_explicit_casts_use_checked_conversion_plans() {
    assert_scalar_kind("value<i64> := 3; value<f64>", BuiltinScalarKind::F64);

    let truncated = evaluate("value := 3.9; value<i32>").unwrap();
    assert!(matches!(
        truncated.snapshot().unwrap().data(),
        ValueData::I32(3)
    ));

    let error = evaluate("value := 2147483648.0; value<i32>").unwrap_err();
    assert_eq!(error.kind_name(), "ConversionOutOfRange");

    assert_scalar_kind("value := true; value<string>", BuiltinScalarKind::String);
    assert_scalar_kind("value := 42<u64>; value<string>", BuiltinScalarKind::String);
}

#[test]
fn source_matrix_promotion_preserves_shape_and_element_order() {
    let value = evaluate("[1<f32> 2<f32>] + [3<f64> 4<f64>]").unwrap();
    let snapshot = value.snapshot().unwrap();
    let ValueData::Matrix(matrix) = snapshot.data() else {
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
fn source_matrix_dimensions_are_checked_and_preserved() {
    let transposed = evaluate("[1.0 2.0 3.0; 4.0 5.0 6.0]'").unwrap();
    assert_eq!(
        transposed.current_top_level_extents().unwrap().as_ref(),
        &[3, 2],
    );

    let product = evaluate("[1.0 2.0; 3.0 4.0] ** [5.0; 6.0]").unwrap();
    assert_eq!(
        product.current_top_level_extents().unwrap().as_ref(),
        &[2, 1]
    );
    let snapshot = product.snapshot().unwrap();
    let ValueData::Matrix(matrix) = snapshot.data() else {
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
    assert!(
        mismatch.kind_message().contains("math/add"),
        "dimension failure must retain the semantic operation: {mismatch:?}",
    );

    let product_mismatch = evaluate("[1.0 2.0] ** [1.0; 2.0; 3.0]").unwrap_err();
    assert_eq!(product_mismatch.kind_name(), "TypeIncompatibility");
    assert!(
        product_mismatch.kind_message().contains("matrix/matmul"),
        "matrix-product dimensions must fail in semantic resolution: {product_mismatch:?}",
    );

    let rectangular_solve = evaluate("[1.0 2.0 3.0; 4.0 5.0 6.0] \\ [1.0; 2.0]").unwrap_err();
    assert!(
        rectangular_solve.display_message().contains("matrix/solve"),
        "solve failure must retain the semantic operation: {rectangular_solve:?}",
    );
}

#[test]
fn source_variadic_templates_cover_large_concat_and_exact_sets() {
    let row = format!("[{}]", vec!["1.0"; 40].join(" "));
    let concatenated = evaluate(&row).unwrap();
    assert_eq!(
        concatenated.current_top_level_extents().unwrap().as_ref(),
        &[1, 40],
    );

    let set = evaluate("{1<u8>, 2<u8>, 3<u8>}").unwrap();
    assert!(matches!(
        set.resolved_type().unwrap().kind(),
        mech_core::KindExpr::Set {
            cardinality: mech_core::DimensionExpr::Constant(3),
            ..
        }
    ));
}

#[test]
fn strict_equality_never_inserts_a_conversion() {
    let error = evaluate("1<u8> === 1<u16>").unwrap_err();
    assert_eq!(error.kind_name(), "TypeIncompatibility");
    assert!(error.kind_message().contains("compare/seq"));

    let ordinary = evaluate("1<u8> == 1<u16>").unwrap();
    assert!(matches!(
        ordinary.snapshot().unwrap().data(),
        ValueData::Bool(true)
    ));

    let shape_error = evaluate("[1.0] === [1.0 2.0]").unwrap_err();
    assert_eq!(shape_error.kind_name(), "TypeIncompatibility");
    assert!(shape_error.kind_message().contains("compare/seq"));
}

#[test]
fn ordering_and_keyability_are_semantic_constraints() {
    let complex = evaluate("(1 + 2i) < (2 + 3i)").unwrap_err();
    assert_eq!(complex.kind_name(), "TypeIncompatibility");
    assert!(complex.kind_message().contains("Ordered"));

    let member = evaluate("2<u8> ∈ {1<u8>, 2<u8>}").unwrap();
    assert!(matches!(
        member.snapshot().unwrap().data(),
        ValueData::Bool(true)
    ));
}

#[test]
fn user_function_boundaries_use_type_system_conversions() {
    let value =
        evaluate("```mech\nwiden(x<f64>) = y<f64> :=\n  y := x.\nwiden(7<i32>)\n```").unwrap();
    assert!(matches!(
        value.snapshot().unwrap().data(),
        ValueData::F64(value) if value.to_f64() == 7.0
    ));

    let output =
        evaluate("```mech\nwiden-output(x<i32>) = y<f64> :=\n  y := x.\nwiden-output(9<i32>)\n```")
            .unwrap();
    assert!(matches!(
        output.snapshot().unwrap().data(),
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
        assert!(
            !error.tokens.is_empty(),
            "semantic failure lost its source range for {source}: {error:?}",
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
        value
            .resolved_type()
            .unwrap_or_else(|error| panic!("{source}: {error:?}"));
    }
}
