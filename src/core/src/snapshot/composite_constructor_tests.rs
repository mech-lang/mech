use super::*;
use crate::{DimensionExpr, FloatWidth, SchemaBody, SchemaDraft, SchemaField, SchemaTableBuilder};
use std::sync::Arc;

fn arena(bodies: Vec<SchemaBody>) -> (Arc<crate::SchemaTable>, Vec<crate::SchemaId>) {
    let mut builder = SchemaTableBuilder::new();
    let handles = bodies
        .into_iter()
        .map(|body| {
            builder
                .insert(
                    SchemaDraft {
                        body,
                        dimension_parameters: Box::new([]),
                    }
                    .finalize()
                    .unwrap(),
                )
                .unwrap()
        })
        .collect::<Vec<_>>();
    let built = builder.finish().unwrap();
    let ids = handles
        .into_iter()
        .map(|handle| built.resolve(handle).unwrap())
        .collect();
    (Arc::new(built.into_parts().0), ids)
}
fn shape(schemas: &crate::SchemaTable, id: crate::SchemaId) -> crate::ShapeInstance {
    schemas
        .get(id)
        .unwrap()
        .instantiate_shape(Box::new([]))
        .unwrap()
}
fn value(schemas: &Arc<crate::SchemaTable>, id: crate::SchemaId, data: ValueDataDraft) -> Value {
    ValueDraft {
        schema: id,
        shape_values: Box::new([]),
        data,
    }
    .finalize(&SnapshotValidationContext::with_shared_schemas(schemas))
    .unwrap()
}

#[test]
fn composite_constructor_rejects_scalar_nested_nominal_and_dimension_substitution() {
    let f = SchemaBody::FloatingPoint(FloatWidth::W64);
    let atom = |name: &str| {
        SchemaBody::Atom(crate::NominalKey::from_path(
            crate::NominalKind::Atom,
            &crate::CanonicalNominalPath::new(vec![name.to_owned()]).unwrap(),
        ))
    };
    let matrix = |n| SchemaBody::Matrix {
        element: Box::new(f.clone()),
        dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(n)].into_boxed_slice(),
    };
    let cases = vec![
        (f.clone(), SchemaBody::Bool, ValueDataDraft::Bool(true)),
        (
            SchemaBody::Tuple(vec![f.clone()].into_boxed_slice()),
            SchemaBody::Tuple(vec![SchemaBody::Bool].into_boxed_slice()),
            ValueDataDraft::Tuple(vec![ValueDataDraft::Bool(true)].into_boxed_slice()),
        ),
        (atom("left"), atom("right"), ValueDataDraft::Atom),
        (
            matrix(1),
            matrix(2),
            ValueDataDraft::Matrix(
                vec![
                    ValueDataDraft::F64(F64Bits::from_f64(1.0)),
                    ValueDataDraft::F64(F64Bits::from_f64(2.0)),
                ]
                .into_boxed_slice(),
            ),
        ),
    ];
    for (expected, actual, data) in cases {
        let (schemas, ids) = arena(vec![
            expected.clone(),
            actual,
            SchemaBody::Tuple(vec![expected].into_boxed_slice()),
        ]);
        assert!(matches!(
            CompositeSnapshotConstructor::bind(
                ids[2],
                shape(&schemas, ids[2]),
                &[(ids[1], shape(&schemas, ids[1]))],
                Arc::clone(&schemas)
            ),
            Err(SnapshotValueError::SnapshotSchemaDefinitionMismatch { .. })
        ));
        let constructor = CompositeSnapshotConstructor::bind(
            ids[2],
            shape(&schemas, ids[2]),
            &[(ids[0], shape(&schemas, ids[0]))],
            Arc::clone(&schemas),
        )
        .unwrap();
        assert!(matches!(
            constructor.construct(vec![value(&schemas, ids[1], data)].into_boxed_slice(), None),
            Err(SnapshotValueError::SnapshotSchemaDefinitionMismatch { .. })
        ));
        assert!(matches!(
            constructor.construct(Box::new([]), None),
            Err(SnapshotValueError::AggregateArityMismatchV1 {
                expected: 1,
                actual: 0,
                ..
            })
        ));
    }
}

#[test]
fn composite_constructor_uses_canonical_map_order_and_duplicate_identity() {
    let f = SchemaBody::FloatingPoint(FloatWidth::W64);
    let (schemas, ids) = arena(vec![
        f.clone(),
        SchemaBody::Map {
            key: Box::new(f.clone()),
            value: Box::new(f),
            cardinality: crate::CardinalitySpec::Exact(DimensionExpr::Constant(2)),
        },
    ]);
    let constructor = CompositeSnapshotConstructor::bind(
        ids[1],
        shape(&schemas, ids[1]),
        &vec![(ids[0], shape(&schemas, ids[0])); 4],
        Arc::clone(&schemas),
    )
    .unwrap();
    let children = |values: [f64; 4]| {
        values
            .into_iter()
            .map(|v| value(&schemas, ids[0], ValueDataDraft::F64(F64Bits::from_f64(v))))
            .collect::<Vec<_>>()
            .into_boxed_slice()
    };
    let output = constructor
        .construct(children([2.0, 20.0, 1.0, 10.0]), None)
        .unwrap();
    let ValueData::Map(map) = output.data() else {
        panic!()
    };
    assert!(matches!(map.entries()[0].key().data(),ValueData::F64(v) if v.to_f64()==1.0));
    assert!(matches!(map.entries()[0].value(),ValueData::F64(v) if v.to_f64()==10.0));
    assert!(matches!(
        constructor.construct(children([-0.0, 10.0, 0.0, 20.0]), None),
        Err(SnapshotValueError::DuplicateCanonicalKeyV1 { .. })
    ));
}

#[test]
fn bound_record_labels_do_not_create_draft_storage_during_construction() {
    let mut costs = Vec::new();
    for label in ["x".to_owned(), "x".repeat(65_536)] {
        let (schemas, ids) = arena(vec![
            SchemaBody::Bool,
            SchemaBody::Record(
                vec![SchemaField {
                    name: label,
                    schema: SchemaBody::Bool,
                }]
                .into_boxed_slice(),
            ),
        ]);
        let constructor = CompositeSnapshotConstructor::bind(
            ids[1],
            shape(&schemas, ids[1]),
            &[(ids[0], shape(&schemas, ids[0]))],
            Arc::clone(&schemas),
        )
        .unwrap();
        costs.push(constructor.allocation_containers().unwrap());
        let result = constructor
            .construct(
                vec![value(&schemas, ids[0], ValueDataDraft::Bool(true))].into_boxed_slice(),
                Some(&SnapshotCanonicalizationBudget::new(0)),
            )
            .unwrap();
        let ValueData::Record(record) = result.data() else {
            panic!()
        };
        assert!(matches!(record.fields(), [ValueData::Bool(true)]));
    }
    assert_eq!(costs[0], costs[1]);
    assert!(costs[0].0 < 4096);
}
