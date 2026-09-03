use mech_core::{
    BuiltinScalarKind, BuiltinTypeClass, FloatWidth, KindExpr, SchemaBody,
    builtin_scalar_named_kind,
};

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
fn scalar_class_membership_matches_the_closed_table() {
    assert!(BuiltinScalarKind::U64.satisfies(BuiltinTypeClass::Number));
    assert!(BuiltinScalarKind::F64.satisfies(BuiltinTypeClass::Number));
    assert!(!BuiltinScalarKind::Bool.satisfies(BuiltinTypeClass::Number));
    assert!(!BuiltinScalarKind::String.satisfies(BuiltinTypeClass::Number));
    assert!(!BuiltinScalarKind::C64.satisfies(BuiltinTypeClass::Ordered));
    assert!(!BuiltinScalarKind::U64.satisfies(BuiltinTypeClass::Negatable));
    assert!(BuiltinScalarKind::I64.satisfies(BuiltinTypeClass::Negatable));
}

#[test]
fn builtin_kind_expression_is_named_by_the_registry() {
    assert_eq!(
        BuiltinScalarKind::F32.kind_expr(),
        KindExpr::Named(BuiltinScalarKind::F32.kind_id())
    );
}
