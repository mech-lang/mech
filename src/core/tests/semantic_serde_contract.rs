#![cfg(feature = "serde")]

use mech_core::*;
use serde::{Serialize, de::DeserializeOwned};

fn assert_serialize<T: Serialize>(value: &T) {
    serde_json::to_value(value).unwrap();
}

fn assert_deserialize<T: DeserializeOwned>() {}

#[test]
fn open_semantic_syntax_remains_serializable_and_deserializable() {
    assert_deserialize::<SchemaDraft>();
    assert_deserialize::<SchemaBody>();
    assert_deserialize::<DimensionExpr>();
    assert_deserialize::<DimensionParameterDeclaration>();
    assert_deserialize::<KindExpr>();
    assert_deserialize::<KindParameter>();
    assert_deserialize::<InputKindScheme>();
    assert_deserialize::<KindConstraint>();

    let draft = SchemaDraft {
        dimension_parameters: Vec::new().into_boxed_slice(),
        body: SchemaBody::Bool,
    };
    let encoded = serde_json::to_string(&draft).unwrap();
    assert_eq!(
        serde_json::from_str::<SchemaDraft>(&encoded).unwrap(),
        draft
    );
}

#[test]
fn finalized_semantic_values_serialize_only_after_validated_construction() {
    let path = CanonicalNominalPath::new(vec!["fixture".to_owned(), "Choice".to_owned()]).unwrap();
    assert_serialize(&path);

    let schema = SchemaDraft {
        dimension_parameters: vec![DimensionParameterDeclaration {
            id: DimensionParameterId::new(0),
            origin: DimensionParameterOrigin::Explicit,
            lifetime: DimensionLifetime::Turn,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: Some(DimensionExpr::Constant(8)),
        }]
        .into_boxed_slice(),
        body: SchemaBody::Matrix {
            element: Box::new(SchemaBody::Bool),
            dimensions: vec![DimensionExpr::Parameter(DimensionParameterId::new(0))]
                .into_boxed_slice(),
        },
    }
    .finalize()
    .unwrap();
    let shape = schema
        .instantiate_shape(vec![4].into_boxed_slice())
        .unwrap();
    assert_serialize(&schema);
    assert_serialize(&schema.dimension_parameters()[0]);
    assert_serialize(&shape);
    assert!(!schema.canonical_bytes().is_empty());
    assert!(!shape.canonical_bytes().is_empty());

    let scheme = KindScheme::new(
        Vec::new().into_boxed_slice(),
        Vec::new().into_boxed_slice(),
        InputKindScheme::Fixed(vec![KindExpr::Id].into_boxed_slice()),
        vec![KindExpr::Index].into_boxed_slice(),
        Vec::new().into_boxed_slice(),
    )
    .unwrap();
    assert_serialize(&scheme);
}
