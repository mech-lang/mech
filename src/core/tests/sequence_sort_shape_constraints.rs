//! Sort requires equal live element counts, not equal rigid input namespaces.
use mech_core::*;

fn scalar(kind: BuiltinScalarKind) -> ResolvedType {
    ResolvedType::new(kind.kind_expr(), Box::new([])).unwrap()
}
fn fixed_matrix(element: KindExpr, rows: u64, columns: u64) -> ResolvedType {
    ResolvedType::new(
        KindExpr::Matrix {
            element: Box::new(element),
            dimensions: Box::new([
                DimensionExpr::Constant(rows),
                DimensionExpr::Constant(columns),
            ]),
        },
        Box::new([]),
    )
    .unwrap()
}
fn live_matrix(element: BuiltinScalarKind, rows: u64, upper: u64) -> ResolvedType {
    ResolvedType::new(
        KindExpr::Matrix {
            element: Box::new(element.kind_expr()),
            dimensions: Box::new([
                DimensionExpr::Constant(rows),
                DimensionExpr::Parameter(DimensionParameterId::new(0)),
            ]),
        },
        Box::new([DimensionParameterDeclaration {
            id: DimensionParameterId::new(0),
            origin: DimensionParameterOrigin::Inferred,
            lifetime: DimensionLifetime::Turn,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: Some(DimensionExpr::Constant(upper)),
        }]),
    )
    .unwrap()
}
fn resolve(
    primary: ResolvedType,
    secondary: ResolvedType,
) -> Result<ResolvedOverload, TypeResolutionError> {
    let schemes = maintained_source_schemes("sequence/sort-indices")
        .unwrap()
        .unwrap();
    let candidates = schemes
        .iter()
        .enumerate()
        .map(|(id, scheme)| TypeOverloadCandidate {
            id: id as u64,
            scheme,
        })
        .collect::<Vec<_>>();
    resolve_type_overloads(
        TypeConstraintOrigin::new("sequence/sort-indices", None),
        &candidates,
        &[
            primary,
            scalar(BuiltinScalarKind::String),
            secondary,
            scalar(BuiltinScalarKind::String),
        ],
        None,
    )
}

#[test]
fn distinct_live_element_kinds_keep_input_dimensions_and_identity_conversions() {
    // Equal current products are a runtime obligation; neither the element
    // kinds nor independently declared bounds prove rigid axis equality.
    let primary = live_matrix(BuiltinScalarKind::U64, 1, 12);
    let secondary = live_matrix(BuiltinScalarKind::String, 2, 8);
    let original = [
        primary.clone(),
        scalar(BuiltinScalarKind::String),
        secondary.clone(),
        scalar(BuiltinScalarKind::String),
    ];
    let result = resolve(primary, secondary).unwrap();
    assert_eq!(result.outputs.len(), 1);
    let KindExpr::Matrix {
        element,
        dimensions,
    } = result.outputs[0].kind()
    else {
        panic!("expected Index row")
    };
    assert_eq!(element.as_ref(), &KindExpr::Index);
    assert_eq!(dimensions[0], DimensionExpr::Constant(1));
    assert!(matches!(dimensions[1], DimensionExpr::Parameter(_)));
    assert_eq!(result.conversions.len(), original.len());
    for (conversion, input) in result.conversions.iter().zip(&original) {
        assert_eq!(&conversion.source, input);
        assert_eq!(&conversion.target, input);
        assert_eq!(conversion.step, ConversionStep::Identity);
        assert_eq!(conversion.cost, 0);
    }
}

#[test]
fn fixed_counts_are_checked_by_product_and_result_remains_one_based_index_row() {
    for (a, b, n) in [
        ((2, 3), (3, 2), 6),
        ((1, 0), (0, 7), 0),
        ((0, 5), (2, 0), 0),
    ] {
        let result = resolve(
            fixed_matrix(BuiltinScalarKind::U64.kind_expr(), a.0, a.1),
            fixed_matrix(BuiltinScalarKind::String.kind_expr(), b.0, b.1),
        )
        .unwrap();
        assert_eq!(
            result.outputs.as_ref(),
            &[fixed_matrix(KindExpr::Index, 1, n)]
        );
    }
    for (a, b) in [((1, 2), (1, 3)), ((2, 3), (2, 2)), ((0, 3), (1, 1))] {
        assert!(
            resolve(
                fixed_matrix(BuiltinScalarKind::U64.kind_expr(), a.0, a.1),
                fixed_matrix(BuiltinScalarKind::String.kind_expr(), b.0, b.1),
            )
            .is_err()
        );
    }
}
