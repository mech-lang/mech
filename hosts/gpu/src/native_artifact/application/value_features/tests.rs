use crate::native_artifact::application::value_features::*;

#[test]
fn nested_payloads_supply_their_own_gates() {
    let body = SchemaBody::Tuple(
        vec![
            SchemaBody::UnsignedInteger(IntegerWidth::W128),
            SchemaBody::Option(Box::new(SchemaBody::Complex(FloatWidth::W32))),
            SchemaBody::Rational64,
        ]
        .into_boxed_slice(),
    );
    assert_eq!(
        from_bodies(std::iter::once(&body)).features,
        BTreeSet::from(["complex", "f32", "r64", "tuple", "u128"])
    );
}

#[test]
fn signed_cells_are_selected_only_by_signed_schema_requirements() {
    let unsigned = SchemaBody::UnsignedInteger(IntegerWidth::W64);
    assert_eq!(
        from_bodies(std::iter::once(&unsigned)).features,
        BTreeSet::from(["u64"]),
    );
    let signed = SchemaBody::Option(Box::new(SchemaBody::SignedInteger(IntegerWidth::W64)));
    assert_eq!(
        from_bodies(std::iter::once(&signed)).features,
        BTreeSet::from(["i64"]),
    );
}

#[test]
fn dynamic_nested_payloads_keep_complete_conversion_support() {
    let body = SchemaBody::Option(Box::new(SchemaBody::Dynamic));
    assert!(from_bodies(std::iter::once(&body)).open_types);
    assert!(from_bodies(std::iter::once(&SchemaBody::ReifiedType)).open_types);
}
