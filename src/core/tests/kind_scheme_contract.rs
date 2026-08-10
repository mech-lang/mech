use mech_core::*;

fn empty_fixed() -> InputKindScheme {
    InputKindScheme::Fixed(Vec::new().into_boxed_slice())
}

#[test]
fn empty_and_variadic_schemes_are_structurally_valid() {
    let empty = KindScheme::new(
        Vec::new().into_boxed_slice(),
        Vec::new().into_boxed_slice(),
        empty_fixed(),
        Vec::new().into_boxed_slice(),
        Vec::new().into_boxed_slice(),
    )
    .unwrap();
    assert!(empty.outputs().is_empty());

    KindScheme::new(
        vec![KindParameter {
            id: KindParameterId::new(0),
            upper_bound: Some(KindExpr::Wildcard),
        }]
        .into_boxed_slice(),
        Vec::new().into_boxed_slice(),
        InputKindScheme::Variadic {
            prefix: vec![KindExpr::Id].into_boxed_slice(),
            repeated: KindExpr::Parameter(KindParameterId::new(0)),
            min_repetitions: 0,
        },
        vec![KindExpr::Never].into_boxed_slice(),
        Vec::new().into_boxed_slice(),
    )
    .unwrap();
}

#[test]
fn kind_parameters_are_dense_unique_and_only_reference_earlier_bounds() {
    assert!(matches!(
        KindScheme::new(
            vec![
                KindParameter {
                    id: KindParameterId::new(0),
                    upper_bound: None,
                },
                KindParameter {
                    id: KindParameterId::new(0),
                    upper_bound: None,
                },
            ]
            .into_boxed_slice(),
            Vec::new().into_boxed_slice(),
            empty_fixed(),
            Vec::new().into_boxed_slice(),
            Vec::new().into_boxed_slice(),
        ),
        Err(SemanticModelError::DuplicateKindParameter { .. })
    ));

    assert!(matches!(
        KindScheme::new(
            vec![KindParameter {
                id: KindParameterId::new(0),
                upper_bound: Some(KindExpr::Parameter(KindParameterId::new(0))),
            }]
            .into_boxed_slice(),
            Vec::new().into_boxed_slice(),
            empty_fixed(),
            Vec::new().into_boxed_slice(),
            Vec::new().into_boxed_slice(),
        ),
        Err(SemanticModelError::ForwardKindParameterReference { .. })
    ));
}

#[test]
fn holes_and_unknown_parameters_are_rejected_everywhere() {
    assert!(matches!(
        KindScheme::new(
            Vec::new().into_boxed_slice(),
            Vec::new().into_boxed_slice(),
            InputKindScheme::Fixed(vec![KindExpr::Hole].into_boxed_slice()),
            Vec::new().into_boxed_slice(),
            Vec::new().into_boxed_slice(),
        ),
        Err(SemanticModelError::UnresolvedKindHole)
    ));
    assert!(matches!(
        KindScheme::new(
            Vec::new().into_boxed_slice(),
            Vec::new().into_boxed_slice(),
            empty_fixed(),
            vec![KindExpr::Parameter(KindParameterId::new(0))].into_boxed_slice(),
            Vec::new().into_boxed_slice(),
        ),
        Err(SemanticModelError::UnknownKindParameter { .. })
    ));
}

#[test]
fn dimension_bounds_are_dense_acyclic_and_backward_only() {
    let parameters = vec![
        DimensionParameterDeclaration {
            id: DimensionParameterId::new(0),
            origin: DimensionParameterOrigin::Explicit,
            lifetime: DimensionLifetime::Activation,
            lower_bound: DimensionExpr::Parameter(DimensionParameterId::new(1)),
            upper_bound: None,
        },
        DimensionParameterDeclaration {
            id: DimensionParameterId::new(1),
            origin: DimensionParameterOrigin::Explicit,
            lifetime: DimensionLifetime::Activation,
            lower_bound: DimensionExpr::Parameter(DimensionParameterId::new(0)),
            upper_bound: None,
        },
    ];
    assert!(matches!(
        KindScheme::new(
            Vec::new().into_boxed_slice(),
            parameters.into_boxed_slice(),
            empty_fixed(),
            Vec::new().into_boxed_slice(),
            Vec::new().into_boxed_slice(),
        ),
        Err(SemanticModelError::CyclicDimensionParameterBoundsV1)
    ));
}

#[test]
fn kind_schemes_reject_duplicate_names_recursively() {
    let duplicate_record = KindExpr::Record(
        vec![
            KindField {
                name: "x".to_owned(),
                kind: KindExpr::Id,
            },
            KindField {
                name: "x".to_owned(),
                kind: KindExpr::Index,
            },
        ]
        .into_boxed_slice(),
    );
    assert!(matches!(
        KindScheme::new(
            Vec::new().into_boxed_slice(),
            Vec::new().into_boxed_slice(),
            InputKindScheme::Fixed(vec![duplicate_record].into_boxed_slice()),
            Vec::new().into_boxed_slice(),
            Vec::new().into_boxed_slice(),
        ),
        Err(SemanticModelError::DuplicateKindName {
            category: KindNameCategory::RecordField,
            ref name,
        }) if name == "x"
    ));

    let nested_duplicate_table = KindExpr::Option(Box::new(KindExpr::Table {
        columns: vec![
            KindField {
                name: "value".to_owned(),
                kind: KindExpr::Id,
            },
            KindField {
                name: "value".to_owned(),
                kind: KindExpr::Index,
            },
        ]
        .into_boxed_slice(),
        rows: DimensionExpr::Constant(0),
    }));
    assert!(matches!(
        KindScheme::new(
            Vec::new().into_boxed_slice(),
            Vec::new().into_boxed_slice(),
            empty_fixed(),
            vec![nested_duplicate_table].into_boxed_slice(),
            Vec::new().into_boxed_slice(),
        ),
        Err(SemanticModelError::DuplicateKindName {
            category: KindNameCategory::TableColumn,
            ..
        })
    ));
}
