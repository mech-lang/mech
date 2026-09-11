use mech_core::{
    BuiltinKindPredicate, BuiltinScalarKind, CardinalitySpec, FloatWidth, KindExpr, ResolvedType,
    SchemaBody, builtin_scalar_named_kind,
};

fn scalar_satisfies(kind: BuiltinScalarKind, predicate: BuiltinKindPredicate) -> bool {
    ResolvedType::new(kind.kind_expr(), Box::new([]))
        .unwrap()
        .satisfies(predicate)
}

#[test]
fn existing_builtin_ordinals_are_unchanged() {
    let expected = [
        BuiltinScalarKind::U8,
        BuiltinScalarKind::U16,
        BuiltinScalarKind::U32,
        BuiltinScalarKind::U64,
        BuiltinScalarKind::U128,
        BuiltinScalarKind::I8,
        BuiltinScalarKind::I16,
        BuiltinScalarKind::I32,
        BuiltinScalarKind::I64,
        BuiltinScalarKind::I128,
        BuiltinScalarKind::F32,
        BuiltinScalarKind::F64,
        BuiltinScalarKind::C64,
        BuiltinScalarKind::R64,
        BuiltinScalarKind::String,
        BuiltinScalarKind::Bool,
    ];
    for (ordinal, kind) in expected.into_iter().enumerate() {
        assert_eq!(kind.kind_id().get(), ordinal as u32);
        assert_eq!(BuiltinScalarKind::from_kind_id(kind.kind_id()), Some(kind));
    }
    assert_eq!(BuiltinScalarKind::C32.kind_id().get(), 16);
}

#[test]
fn c32_and_c64_are_distinct() {
    let c32 = BuiltinScalarKind::from_schema_body(&SchemaBody::Complex(FloatWidth::W32)).unwrap();
    let c64 = BuiltinScalarKind::from_schema_body(&SchemaBody::Complex(FloatWidth::W64)).unwrap();
    assert_ne!(c32, c64);
    assert_ne!(c32.kind_expr(), c64.kind_expr());
}

#[test]
fn schema_scalar_round_trip_uses_one_registry() {
    for kind in BuiltinScalarKind::ALL {
        assert_eq!(
            BuiltinScalarKind::from_schema_body(&kind.schema_body()),
            Some(kind)
        );
        let (id, path) =
            builtin_scalar_named_kind(mech_core::hash_str(kind.canonical_name())).unwrap();
        assert_eq!(id, kind.kind_id());
        assert_eq!(path, kind.canonical_path().unwrap());
    }
}

#[test]
fn scalar_predicate_membership_matches_the_closed_table() {
    assert!(scalar_satisfies(
        BuiltinScalarKind::U64,
        BuiltinKindPredicate::Number
    ));
    assert!(scalar_satisfies(
        BuiltinScalarKind::F64,
        BuiltinKindPredicate::Number
    ));
    assert!(!scalar_satisfies(
        BuiltinScalarKind::Bool,
        BuiltinKindPredicate::Number
    ));
    assert!(!scalar_satisfies(
        BuiltinScalarKind::String,
        BuiltinKindPredicate::Number
    ));
    assert!(!scalar_satisfies(
        BuiltinScalarKind::C64,
        BuiltinKindPredicate::Ordered
    ));
    assert!(!scalar_satisfies(
        BuiltinScalarKind::U64,
        BuiltinKindPredicate::Negatable
    ));
    assert!(scalar_satisfies(
        BuiltinScalarKind::I64,
        BuiltinKindPredicate::Negatable
    ));
}

#[test]
fn builtin_kind_expression_is_named_by_the_registry() {
    assert_eq!(
        BuiltinScalarKind::F32.kind_expr(),
        KindExpr::Named(BuiltinScalarKind::F32.kind_id())
    );
}

#[test]
fn number_contains_every_numeric_scalar() {
    for kind in BuiltinScalarKind::ALL {
        assert_eq!(
            scalar_satisfies(kind, BuiltinKindPredicate::Number),
            !matches!(kind, BuiltinScalarKind::String | BuiltinScalarKind::Bool),
            "{kind:?}",
        );
    }
}

#[test]
fn number_rejects_bool_and_string() {
    assert!(!scalar_satisfies(
        BuiltinScalarKind::Bool,
        BuiltinKindPredicate::Number
    ));
    assert!(!scalar_satisfies(
        BuiltinScalarKind::String,
        BuiltinKindPredicate::Number
    ));
}

#[test]
fn ordered_rejects_complex() {
    assert!(!scalar_satisfies(
        BuiltinScalarKind::C32,
        BuiltinKindPredicate::Ordered
    ));
    assert!(!scalar_satisfies(
        BuiltinScalarKind::C64,
        BuiltinKindPredicate::Ordered
    ));
}

#[test]
fn negatable_rejects_unsigned() {
    for kind in [
        BuiltinScalarKind::U8,
        BuiltinScalarKind::U16,
        BuiltinScalarKind::U32,
        BuiltinScalarKind::U64,
        BuiltinScalarKind::U128,
    ] {
        assert!(!scalar_satisfies(kind, BuiltinKindPredicate::Negatable));
    }
}

#[test]
fn range_endpoint_accepts_index_integer_and_float() {
    assert!(
        ResolvedType::new(KindExpr::Index, Box::new([]))
            .unwrap()
            .satisfies(BuiltinKindPredicate::RangeEndpoint)
    );
    for kind in [
        BuiltinScalarKind::U8,
        BuiltinScalarKind::U64,
        BuiltinScalarKind::I32,
        BuiltinScalarKind::F32,
        BuiltinScalarKind::F64,
    ] {
        assert!(scalar_satisfies(kind, BuiltinKindPredicate::RangeEndpoint));
    }
}

#[test]
fn nested_schema_keyability_becomes_predicate_evidence() {
    let kind = ResolvedType::from_schema_body(
        &SchemaBody::Tuple(
            vec![
                SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W32),
                SchemaBody::Option(Box::new(SchemaBody::Set {
                    element: Box::new(SchemaBody::String),
                    cardinality: CardinalitySpec::Exact(mech_core::DimensionExpr::Constant(2)),
                })),
            ]
            .into_boxed_slice(),
        ),
        &[],
    )
    .unwrap();
    assert!(kind.satisfies(BuiltinKindPredicate::Keyable));
}

#[test]
fn equal_types_intersect_conflicting_predicate_evidence() {
    let left = ResolvedType::from_schema_body(&SchemaBody::String, &[]).unwrap();
    let right = ResolvedType::new(BuiltinScalarKind::String.kind_expr(), Box::new([])).unwrap();
    assert_eq!(left, right);
    for predicate in BuiltinKindPredicate::ALL {
        assert_eq!(
            left.satisfies(predicate),
            right.satisfies(predicate),
            "{predicate:?}"
        );
    }
}

#[test]
fn schema_and_direct_kind_predicates_agree_for_structural_products() {
    let cases = [
        (
            KindExpr::Tuple(Box::new([])),
            SchemaBody::Tuple(Box::new([])),
        ),
        (
            KindExpr::Record(Box::new([])),
            SchemaBody::Record(Box::new([])),
        ),
        (
            KindExpr::Tuple(
                vec![BuiltinScalarKind::String.kind_expr(), KindExpr::Index].into_boxed_slice(),
            ),
            SchemaBody::Tuple(vec![SchemaBody::String, SchemaBody::Index].into_boxed_slice()),
        ),
    ];
    for (kind, schema) in cases {
        let direct = ResolvedType::new(kind, Box::new([])).unwrap();
        let derived = ResolvedType::from_schema_body(&schema, &[]).unwrap();
        assert_eq!(direct, derived);
        for predicate in BuiltinKindPredicate::ALL {
            assert_eq!(
                direct.satisfies(predicate),
                derived.satisfies(predicate),
                "{} {predicate:?}",
                direct.semantic_name(),
            );
        }
        assert!(direct.satisfies(BuiltinKindPredicate::Equatable));
        assert!(direct.satisfies(BuiltinKindPredicate::Keyable));
    }
}
