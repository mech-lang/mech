use mech_core::*;

fn boxed(values: impl IntoIterator<Item = DimensionExpr>) -> Box<[DimensionExpr]> {
    values.into_iter().collect::<Vec<_>>().into_boxed_slice()
}

#[test]
fn arithmetic_normalization_matches_the_v1_reference_rules() {
    let parameter = DimensionParameterId::new(0);
    let normalized = normalize_dimension(
        &DimensionExpr::Add(boxed([
            DimensionExpr::Constant(0),
            DimensionExpr::Add(boxed([
                DimensionExpr::Constant(2),
                DimensionExpr::Parameter(parameter),
            ])),
            DimensionExpr::Constant(3),
        ])),
        1,
    )
    .unwrap();
    assert_eq!(
        normalized,
        DimensionExpr::Add(boxed([
            DimensionExpr::Constant(5),
            DimensionExpr::Parameter(parameter),
        ]))
    );

    assert_eq!(
        normalize_dimension(
            &DimensionExpr::Multiply(boxed([
                DimensionExpr::Constant(u64::MAX),
                DimensionExpr::Constant(0),
            ])),
            0,
        )
        .unwrap(),
        DimensionExpr::Constant(0)
    );
    assert!(matches!(
        normalize_dimension(
            &DimensionExpr::Add(boxed([
                DimensionExpr::Constant(u64::MAX),
                DimensionExpr::Constant(1),
            ])),
            0,
        ),
        Err(SemanticModelError::DimensionOverflowV1)
    ));
}

#[test]
fn min_max_flatten_sort_deduplicate_and_reject_empty() {
    let expression = DimensionExpr::Min(boxed([
        DimensionExpr::Constant(7),
        DimensionExpr::Min(boxed([
            DimensionExpr::Constant(2),
            DimensionExpr::Constant(7),
        ])),
    ]));
    assert_eq!(
        normalize_dimension(&expression, 0).unwrap(),
        DimensionExpr::Min(boxed([
            DimensionExpr::Constant(2),
            DimensionExpr::Constant(7),
        ]))
    );
    assert!(matches!(
        normalize_dimension(&DimensionExpr::Max(boxed([])), 0),
        Err(SemanticModelError::EmptyMinMaxV1 {
            operator: DimensionOperator::Max
        })
    ));
}

#[test]
fn dimension_builder_is_dense_and_compile_time_parameters_are_rejected() {
    let mut environment = DimensionEnvironmentBuilder::new();
    assert_eq!(
        environment
            .declare(
                DimensionParameterOrigin::Explicit,
                DimensionLifetime::Activation,
                DimensionExpr::Constant(1),
                None,
            )
            .unwrap()
            .get(),
        0
    );
    assert_eq!(
        environment
            .declare(
                DimensionParameterOrigin::Inferred,
                DimensionLifetime::Turn,
                DimensionExpr::Constant(0),
                Some(DimensionExpr::Constant(9)),
            )
            .unwrap()
            .get(),
        1
    );
    assert_eq!(environment.declarations().len(), 2);

    let mut environment = DimensionEnvironmentBuilder::new();
    assert!(matches!(
        environment.declare(
            DimensionParameterOrigin::Explicit,
            DimensionLifetime::CompileTime,
            DimensionExpr::Constant(1),
            None,
        ),
        Err(SemanticModelError::CompileTimeDimensionParameterV1)
    ));
    assert!(environment.declarations().is_empty());
}

struct NoNamedKinds;

impl NamedKindPathResolver for NoNamedKinds {
    fn canonical_path(&self, _id: KindId) -> Option<&CanonicalNominalPath> {
        None
    }
}

#[test]
fn closed_kind_bytes_use_the_frozen_envelope_tags_and_framing() {
    let kind = KindExpr::Matrix {
        element: Box::new(KindExpr::Id),
        dimensions: boxed([DimensionExpr::Add(boxed([
            DimensionExpr::Constant(3),
            DimensionExpr::Constant(4),
        ]))]),
    };
    let bytes = canonical_closed_kind_bytes(&kind, &[], &NoNamedKinds).unwrap();
    assert_eq!(
        bytes.as_ref(),
        &[
            0x01, 0x00, 0x00, 0x00, 0x00, // envelope and parameter count
            0x1f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // body frame
            0x09, // Matrix
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, // Id node
            0x01, 0x00, 0x00, 0x00, // one dimension
            0x09, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // dimension node
            0x01, 0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]
    );
}

#[test]
fn closed_kind_encoding_rejects_holes_parameters_and_unknown_names() {
    assert!(matches!(
        canonical_closed_kind_bytes(&KindExpr::Hole, &[], &NoNamedKinds),
        Err(SemanticModelError::UnresolvedKindHole)
    ));
    assert!(matches!(
        canonical_closed_kind_bytes(
            &KindExpr::Parameter(KindParameterId::new(4)),
            &[],
            &NoNamedKinds
        ),
        Err(SemanticModelError::KindParameterNotClosed { .. })
    ));
    assert!(matches!(
        canonical_closed_kind_bytes(&KindExpr::Named(KindId::new(3)), &[], &NoNamedKinds),
        Err(SemanticModelError::UnknownNamedKind { .. })
    ));
}

#[test]
fn closed_kind_encoding_rejects_duplicate_names_and_preserves_field_order() {
    let field = |name: &str, kind| KindField {
        name: name.to_owned(),
        kind,
    };
    let duplicate = KindExpr::Option(Box::new(KindExpr::Record(
        vec![field("x", KindExpr::Id), field("x", KindExpr::Index)].into_boxed_slice(),
    )));
    assert!(matches!(
        canonical_closed_kind_bytes(&duplicate, &[], &NoNamedKinds),
        Err(SemanticModelError::DuplicateKindName {
            category: KindNameCategory::RecordField,
            ..
        })
    ));

    let first = KindExpr::Record(
        vec![field("a", KindExpr::Id), field("b", KindExpr::Index)].into_boxed_slice(),
    );
    let reversed = KindExpr::Record(
        vec![field("b", KindExpr::Index), field("a", KindExpr::Id)].into_boxed_slice(),
    );
    assert_ne!(
        canonical_closed_kind_bytes(&first, &[], &NoNamedKinds).unwrap(),
        canonical_closed_kind_bytes(&reversed, &[], &NoNamedKinds).unwrap(),
    );
}

#[test]
fn closed_kind_normalizes_before_parameter_reachability() {
    let parameter = DimensionParameterId::new(0);
    let declarations = vec![DimensionParameterDeclaration {
        id: parameter,
        origin: DimensionParameterOrigin::Explicit,
        lifetime: DimensionLifetime::Turn,
        lower_bound: DimensionExpr::Constant(0),
        upper_bound: None,
    }];
    let reduced = KindExpr::Matrix {
        element: Box::new(KindExpr::Id),
        dimensions: boxed([DimensionExpr::Multiply(boxed([
            DimensionExpr::Constant(0),
            DimensionExpr::Parameter(parameter),
        ]))]),
    };
    let constant = KindExpr::Matrix {
        element: Box::new(KindExpr::Id),
        dimensions: boxed([DimensionExpr::Constant(0)]),
    };
    assert_eq!(
        canonical_closed_kind_bytes(&reduced, &declarations, &NoNamedKinds).unwrap(),
        canonical_closed_kind_bytes(&constant, &[], &NoNamedKinds).unwrap()
    );
}
