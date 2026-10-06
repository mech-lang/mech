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
        assert!(matches!(
            CompositeSnapshotConstructor::shape_for_children(
                ids[2],
                &[(ids[1], shape(&schemas, ids[1]))],
                &schemas,
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

#[test]
fn composite_shape_inference_remaps_nested_child_arenas_and_checks_shared_witnesses() {
    use crate::{
        DimensionLifetime, DimensionParameterDeclaration, DimensionParameterId,
        DimensionParameterOrigin,
    };
    let p = |index| DimensionExpr::Parameter(DimensionParameterId::new(index));
    let matrix = |dimension| SchemaBody::Matrix {
        element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
        dimensions: vec![dimension, DimensionExpr::Constant(2)].into_boxed_slice(),
    };
    let parameterized = |body, count| {
        SchemaDraft {
            body,
            dimension_parameters: (0..count)
                .map(|index| DimensionParameterDeclaration {
                    id: DimensionParameterId::new(index),
                    origin: DimensionParameterOrigin::Inferred,
                    lifetime: DimensionLifetime::Turn,
                    lower_bound: DimensionExpr::Constant(0),
                    upper_bound: Some(DimensionExpr::Constant(4)),
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        }
        .finalize()
        .unwrap()
    };
    let mut builder = SchemaTableBuilder::new();
    let child = builder.insert(parameterized(matrix(p(0)), 1)).unwrap();
    let nested = builder
        .insert(parameterized(
            SchemaBody::Record(
                vec![SchemaField {
                    name: "selected".into(),
                    schema: matrix(p(0)),
                }]
                .into_boxed_slice(),
            ),
            1,
        ))
        .unwrap();
    let output = builder
        .insert(parameterized(
            SchemaBody::Tuple(
                vec![
                    matrix(DimensionExpr::Add(
                        vec![p(0), DimensionExpr::Constant(1)].into_boxed_slice(),
                    )),
                    SchemaBody::Record(
                        vec![SchemaField {
                            name: "selected".into(),
                            schema: matrix(p(1)),
                        }]
                        .into_boxed_slice(),
                    ),
                ]
                .into_boxed_slice(),
            ),
            2,
        ))
        .unwrap();
    let shared = builder
        .insert(parameterized(
            SchemaBody::Tuple(vec![matrix(p(0)), matrix(p(0))].into_boxed_slice()),
            1,
        ))
        .unwrap();
    let built = builder.finish().unwrap();
    let (child, nested, output, shared) = (
        built.resolve(child).unwrap(),
        built.resolve(nested).unwrap(),
        built.resolve(output).unwrap(),
        built.resolve(shared).unwrap(),
    );
    let schemas = Arc::new(built.into_parts().0);
    let shaped = |id, n| {
        (
            id,
            schemas
                .get(id)
                .unwrap()
                .instantiate_shape(Box::new([n]))
                .unwrap(),
        )
    };
    let children = [shaped(child, 3), shaped(nested, 1)];
    let shape =
        CompositeSnapshotConstructor::shape_for_children(output, &children, &schemas).unwrap();
    assert_eq!(shape.parameter_values(), &[2, 1]);
    CompositeSnapshotConstructor::bind(output, shape, &children, Arc::clone(&schemas)).unwrap();
    assert!(matches!(
        CompositeSnapshotConstructor::shape_for_children(
            shared,
            &[shaped(child, 1), shaped(child, 2)],
            &schemas
        ),
        Err(SnapshotValueError::SnapshotSchemaDefinitionMismatch { .. })
    ));
    assert!(matches!(
        CompositeSnapshotConstructor::shape_for_children(output, &[shaped(child, 3)], &schemas),
        Err(SnapshotValueError::SnapshotSchemaDefinitionMismatch { .. })
    ));
}

#[test]
fn composite_dimension_witnesses_are_independent_of_child_order() {
    use crate::{
        DimensionLifetime, DimensionParameterDeclaration, DimensionParameterId,
        DimensionParameterOrigin,
    };
    let p = |index| DimensionExpr::Parameter(DimensionParameterId::new(index));
    let matrix = |dimension| SchemaBody::Matrix {
        element: Box::new(SchemaBody::Index),
        dimensions: vec![dimension, DimensionExpr::Constant(2)].into_boxed_slice(),
    };
    for offset in [0, 1] {
        for order in [
            vec![0, 1, 2],
            vec![0, 2, 1],
            vec![1, 0, 2],
            vec![1, 2, 0],
            vec![2, 0, 1],
            vec![2, 1, 0],
            vec![0, 1],
            vec![1, 0],
        ] {
            let shifted = |index| {
                DimensionExpr::Add(
                    vec![p(index), DimensionExpr::Constant(offset)].into_boxed_slice(),
                )
            };
            let dimensions = [
                shifted(0),
                DimensionExpr::Add(vec![p(0), p(1)].into_boxed_slice()),
                shifted(1),
            ];
            let counts = [1 + offset, 3, 2 + offset];
            let mut builder = SchemaTableBuilder::new();
            let children = order
                .iter()
                .map(|index| {
                    builder
                        .insert(
                            SchemaDraft {
                                body: matrix(DimensionExpr::Constant(counts[*index])),
                                dimension_parameters: Box::new([]),
                            }
                            .finalize()
                            .unwrap(),
                        )
                        .unwrap()
                })
                .collect::<Vec<_>>();
            let output = builder
                .insert(
                    SchemaDraft {
                        body: SchemaBody::Tuple(
                            order
                                .iter()
                                .map(|index| matrix(dimensions[*index].clone()))
                                .collect::<Vec<_>>()
                                .into_boxed_slice(),
                        ),
                        dimension_parameters: (0..2)
                            .map(|index| DimensionParameterDeclaration {
                                id: DimensionParameterId::new(index),
                                origin: DimensionParameterOrigin::Inferred,
                                lifetime: DimensionLifetime::Turn,
                                lower_bound: DimensionExpr::Constant(0),
                                upper_bound: Some(DimensionExpr::Constant(4)),
                            })
                            .collect::<Vec<_>>()
                            .into_boxed_slice(),
                    }
                    .finalize()
                    .unwrap(),
                )
                .unwrap();
            let built = builder.finish().unwrap();
            let output = built.resolve(output).unwrap();
            let children = children
                .into_iter()
                .map(|child| built.resolve(child).unwrap())
                .collect::<Vec<_>>();
            let schemas = Arc::new(built.into_parts().0);
            let children = children
                .into_iter()
                .map(|child| (child, shape(&schemas, child)))
                .collect::<Vec<_>>();
            let inferred =
                CompositeSnapshotConstructor::shape_for_children(output, &children, &schemas)
                    .unwrap_or_else(|error| panic!("{order:?}: {error:?}"));
            let SchemaBody::Tuple(closed) =
                schemas.get(output).unwrap().closed_body(&inferred).unwrap()
            else {
                panic!()
            };
            assert_eq!(
                closed.as_ref(),
                order
                    .iter()
                    .map(|index| matrix(DimensionExpr::Constant(counts[*index])))
                    .collect::<Vec<_>>()
                    .as_slice()
            );
            CompositeSnapshotConstructor::bind(output, inferred, &children, schemas).unwrap();
        }
    }
}

#[test]
fn shared_extent_solver_preserves_exact_axes_while_solving_compound_axes() {
    use crate::{
        DimensionLifetime, DimensionParameterDeclaration, DimensionParameterId,
        DimensionParameterOrigin,
    };
    let p = |index| DimensionExpr::Parameter(DimensionParameterId::new(index));
    let sum = DimensionExpr::Add(vec![p(0), p(1)].into_boxed_slice());
    for (dimensions, extents) in [
        (vec![p(0), sum.clone(), p(1)], vec![1, 3, 2]),
        (vec![sum.clone(), p(1), p(0)], vec![3, 2, 1]),
        (vec![p(0), sum.clone()], vec![1, 3]),
        (vec![sum, p(0)], vec![3, 1]),
        (
            vec![
                p(0),
                DimensionExpr::Multiply(vec![p(0), p(1)].into_boxed_slice()),
            ],
            vec![2, 6],
        ),
        (
            vec![DimensionExpr::Multiply(vec![p(0), p(0)].into_boxed_slice())],
            vec![9],
        ),
        (
            vec![
                DimensionExpr::Min(vec![p(0), DimensionExpr::Constant(2)].into_boxed_slice()),
                DimensionExpr::Add(vec![p(0), DimensionExpr::Constant(1)].into_boxed_slice()),
            ],
            vec![2, 4],
        ),
        (
            vec![
                DimensionExpr::Max(vec![p(0), DimensionExpr::Constant(2)].into_boxed_slice()),
                DimensionExpr::Add(vec![p(0), DimensionExpr::Constant(1)].into_boxed_slice()),
            ],
            vec![2, 2],
        ),
    ] {
        let schema = SchemaDraft {
            body: SchemaBody::Matrix {
                element: Box::new(SchemaBody::Index),
                dimensions: dimensions.into_boxed_slice(),
            },
            dimension_parameters: (0..2)
                .map(|index| DimensionParameterDeclaration {
                    id: DimensionParameterId::new(index),
                    origin: DimensionParameterOrigin::Inferred,
                    lifetime: DimensionLifetime::Turn,
                    lower_bound: DimensionExpr::Constant(0),
                    upper_bound: Some(DimensionExpr::Constant(8)),
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        }
        .finalize()
        .unwrap();
        let shape = crate::shape_for_resolved_extents(&schema, &extents).unwrap();
        let SchemaBody::Matrix { dimensions, .. } = schema.closed_body(&shape).unwrap() else {
            panic!()
        };
        assert_eq!(
            dimensions.as_ref(),
            extents
                .into_iter()
                .map(DimensionExpr::Constant)
                .collect::<Vec<_>>()
                .as_slice()
        );
    }
}
