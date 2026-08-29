use mech_core::{snapshot::*, *};
use std::rc::Rc;

fn table_with(body: SchemaBody) -> (SchemaTable, SchemaId) {
    let schema = SchemaDraft {
        dimension_parameters: Box::new([]),
        body,
    }
    .finalize()
    .unwrap();
    let mut builder = SchemaTableBuilder::new();
    let handle = builder.insert(schema).unwrap();
    let build = builder.finish().unwrap();
    let id = build.resolve(handle).unwrap();
    let (schemas, _) = build.into_parts();
    (schemas, id)
}

fn finalize(body: SchemaBody, data: ValueDataDraft) -> (SchemaTable, Value) {
    let (schemas, schema) = table_with(body);
    let value = ValueDraft {
        schema,
        shape_values: Box::new([]),
        data,
    }
    .finalize(&SnapshotValidationContext::new(&schemas))
    .unwrap();
    (schemas, value)
}

#[test]
fn borrowed_views_cover_nested_canonical_aggregates_without_hidden_cells() {
    let pair = SchemaBody::Tuple(vec![SchemaBody::Bool, SchemaBody::String].into_boxed_slice());
    let set = SchemaBody::Set {
        element: Box::new(SchemaBody::Index),
        cardinality: DimensionExpr::Constant(2).into(),
    };
    let body = SchemaBody::Record(
        vec![
            SchemaField {
                name: "pair".into(),
                schema: pair,
            },
            SchemaField {
                name: "lookup".into(),
                schema: SchemaBody::Map {
                    key: Box::new(SchemaBody::Bool),
                    value: Box::new(set),
                    cardinality: DimensionExpr::Constant(1).into(),
                },
            },
            SchemaField {
                name: "table".into(),
                schema: SchemaBody::Table {
                    columns: vec![SchemaField {
                        name: "present".into(),
                        schema: SchemaBody::Option(Box::new(SchemaBody::Bool)),
                    }]
                    .into_boxed_slice(),
                    rows: DimensionExpr::Constant(2).into(),
                },
            },
        ]
        .into_boxed_slice(),
    );
    let data = ValueDataDraft::Record(
        vec![
            NamedValueDraft {
                name: "table".into(),
                value: ValueDataDraft::Table(
                    vec![TableColumnDraft {
                        name: "present".into(),
                        values: vec![
                            ValueDataDraft::Option(OptionDraft {
                                present: false,
                                value: None,
                            }),
                            ValueDataDraft::Option(OptionDraft {
                                present: true,
                                value: Some(Box::new(ValueDataDraft::Bool(true))),
                            }),
                        ]
                        .into_boxed_slice(),
                    }]
                    .into_boxed_slice(),
                ),
            },
            NamedValueDraft {
                name: "lookup".into(),
                value: ValueDataDraft::Map(
                    vec![MapEntryDraft {
                        items: vec![
                            ValueDataDraft::Bool(true),
                            ValueDataDraft::Set(
                                vec![ValueDataDraft::Index(2), ValueDataDraft::Index(1)]
                                    .into_boxed_slice(),
                            ),
                        ]
                        .into_boxed_slice(),
                    }]
                    .into_boxed_slice(),
                ),
            },
            NamedValueDraft {
                name: "pair".into(),
                value: ValueDataDraft::Tuple(
                    vec![
                        ValueDataDraft::Bool(true),
                        ValueDataDraft::String("canonical".into()),
                    ]
                    .into_boxed_slice(),
                ),
            },
        ]
        .into_boxed_slice(),
    );
    let (schemas, value) = finalize(body, data);
    let fields = value.record_view().unwrap().fields();

    assert!(matches!(&fields[0], ValueData::Tuple(values) if values.len() == 2));
    assert!(matches!(&fields[1], ValueData::Map(map) if map.entries().len() == 1));
    assert!(matches!(&fields[2], ValueData::Table(table) if table.len() == 1));
    let rebuilt = value
        .rebuild_record(
            fields.to_vec().into_boxed_slice(),
            &SnapshotValidationContext::new(&schemas),
        )
        .unwrap();
    assert!(rebuilt.snapshot_eq(&schemas, &value, &schemas).unwrap());
    assert_eq!(
        rebuilt.canonical_payload_bytes(&schemas).unwrap(),
        value.canonical_payload_bytes(&schemas).unwrap()
    );
}

#[test]
fn set_and_map_rebuilds_canonicalize_order_and_reject_duplicate_keys() {
    let (set_schemas, set) = finalize(
        SchemaBody::Set {
            element: Box::new(SchemaBody::Bool),
            cardinality: DimensionExpr::Constant(2).into(),
        },
        ValueDataDraft::Set(
            vec![ValueDataDraft::Bool(true), ValueDataDraft::Bool(false)].into_boxed_slice(),
        ),
    );
    let set = set
        .rebuild_set(
            vec![ValueData::Bool(true), ValueData::Bool(false)].into_boxed_slice(),
            &SnapshotValidationContext::new(&set_schemas),
        )
        .unwrap();
    let elements = set.set_view().unwrap().elements();
    assert!(matches!(elements[0].data(), ValueData::Bool(false)));
    assert!(matches!(elements[1].data(), ValueData::Bool(true)));
    assert!(matches!(
        set.rebuild_set(
            vec![ValueData::Bool(true), ValueData::Bool(true)].into_boxed_slice(),
            &SnapshotValidationContext::new(&set_schemas),
        ),
        Err(SnapshotValueError::DuplicateCanonicalKeyV1 { .. })
    ));

    let (map_schemas, map) = finalize(
        SchemaBody::Map {
            key: Box::new(SchemaBody::Bool),
            value: Box::new(SchemaBody::String),
            cardinality: DimensionExpr::Constant(2).into(),
        },
        ValueDataDraft::Map(
            vec![
                MapEntryDraft {
                    items: vec![
                        ValueDataDraft::Bool(true),
                        ValueDataDraft::String("yes".into()),
                    ]
                    .into_boxed_slice(),
                },
                MapEntryDraft {
                    items: vec![
                        ValueDataDraft::Bool(false),
                        ValueDataDraft::String("no".into()),
                    ]
                    .into_boxed_slice(),
                },
            ]
            .into_boxed_slice(),
        ),
    );
    let map = map
        .rebuild_map(
            vec![
                (ValueData::Bool(true), ValueData::String("yes".into())),
                (ValueData::Bool(false), ValueData::String("no".into())),
            ]
            .into_boxed_slice(),
            &SnapshotValidationContext::new(&map_schemas),
        )
        .unwrap();
    assert!(matches!(
        map.map_view().unwrap().entries()[0].key().data(),
        ValueData::Bool(false)
    ));
    assert!(matches!(
        map.rebuild_map(
            vec![
                (ValueData::Bool(false), ValueData::String("a".into())),
                (ValueData::Bool(false), ValueData::String("b".into())),
            ]
            .into_boxed_slice(),
            &SnapshotValidationContext::new(&map_schemas),
        ),
        Err(SnapshotValueError::DuplicateCanonicalKeyV1 { .. })
    ));
}

#[test]
fn dynamic_set_cardinality_preserves_cell_identity_and_enforces_bounds() {
    let (schemas, empty) = finalize(
        SchemaBody::Set {
            element: Box::new(SchemaBody::Index),
            cardinality: CardinalitySpec::Dynamic { upper_bound: None },
        },
        ValueDataDraft::Set(Box::new([])),
    );
    let schemas = Rc::new(schemas);
    let cell = ValueCell::from_value(empty, schemas.clone()).unwrap();
    let alias = cell.clone();

    for values in [vec![3, 1, 2], vec![7], vec![]] {
        let next = cell
            .snapshot()
            .unwrap()
            .rebuild_set(
                values
                    .into_iter()
                    .map(ValueData::Index)
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
                &SnapshotValidationContext::new(&schemas),
            )
            .unwrap();
        cell.replace(&next).unwrap();
        assert!(cell.same_cell(&alias));
    }
    assert_eq!(
        cell.snapshot()
            .unwrap()
            .set_view()
            .unwrap()
            .elements()
            .len(),
        0
    );

    let (bounded_schemas, bounded) = finalize(
        SchemaBody::Set {
            element: Box::new(SchemaBody::Index),
            cardinality: CardinalitySpec::Dynamic {
                upper_bound: Some(DimensionExpr::Constant(2)),
            },
        },
        ValueDataDraft::Set(Box::new([])),
    );
    assert!(matches!(
        bounded.rebuild_set(
            vec![
                ValueData::Index(1),
                ValueData::Index(2),
                ValueData::Index(3)
            ]
            .into_boxed_slice(),
            &SnapshotValidationContext::new(&bounded_schemas),
        ),
        Err(SnapshotValueError::PayloadCardinalityMismatchV1 { .. })
    ));

    let (exact_schemas, exact) = finalize(
        SchemaBody::Set {
            element: Box::new(SchemaBody::Index),
            cardinality: CardinalitySpec::Exact(DimensionExpr::Constant(2)),
        },
        ValueDataDraft::Set(
            vec![ValueDataDraft::Index(1), ValueDataDraft::Index(2)].into_boxed_slice(),
        ),
    );
    assert!(matches!(
        exact.rebuild_set(
            vec![ValueData::Index(1)].into_boxed_slice(),
            &SnapshotValidationContext::new(&exact_schemas),
        ),
        Err(SnapshotValueError::PayloadCardinalityMismatchV1 { .. })
    ));
}

#[test]
fn exact_and_dynamic_set_schemas_are_explicitly_not_strictly_equal() {
    let (exact_schemas, exact) = finalize(
        SchemaBody::Set {
            element: Box::new(SchemaBody::Index),
            cardinality: CardinalitySpec::Exact(DimensionExpr::Constant(1)),
        },
        ValueDataDraft::Set(vec![ValueDataDraft::Index(1)].into_boxed_slice()),
    );
    let (dynamic_schemas, dynamic) = finalize(
        SchemaBody::Set {
            element: Box::new(SchemaBody::Index),
            cardinality: CardinalitySpec::Dynamic { upper_bound: None },
        },
        ValueDataDraft::Set(vec![ValueDataDraft::Index(1)].into_boxed_slice()),
    );

    assert!(
        !exact
            .snapshot_eq(&exact_schemas, &dynamic, &dynamic_schemas)
            .unwrap()
    );
    assert!(
        !exact
            .language_eq(&exact_schemas, &dynamic, &dynamic_schemas)
            .unwrap()
    );
}

#[test]
fn option_enum_and_heterogeneous_matrix_rebuilds_validate_children() {
    let (option_schemas, option) = finalize(
        SchemaBody::Option(Box::new(SchemaBody::Bool)),
        ValueDataDraft::Option(OptionDraft {
            present: true,
            value: Some(Box::new(ValueDataDraft::Bool(true))),
        }),
    );
    let absent = option
        .rebuild_option(None, &SnapshotValidationContext::new(&option_schemas))
        .unwrap();
    assert!(matches!(absent.option_view(), Some(None)));

    let (enum_schemas, enumeration) = finalize(
        SchemaBody::Enum {
            key: NominalKey::from_bytes([7; 32]),
            variants: vec![
                EnumVariantSchema {
                    name: "Empty".into(),
                    payload: None,
                },
                EnumVariantSchema {
                    name: "Flag".into(),
                    payload: Some(SchemaBody::Bool),
                },
            ]
            .into_boxed_slice(),
        },
        ValueDataDraft::Enum(EnumDraft {
            ordinal: 0,
            payload: None,
        }),
    );
    let enumeration = enumeration
        .rebuild_enum(
            1,
            Some(ValueData::Bool(true)),
            &SnapshotValidationContext::new(&enum_schemas),
        )
        .unwrap();
    assert_eq!(enumeration.enum_view().unwrap().ordinal(), 1);
    assert!(matches!(
        enumeration.rebuild_enum(
            1,
            Some(ValueData::String("wrong".into())),
            &SnapshotValidationContext::new(&enum_schemas),
        ),
        Err(SnapshotValueError::SnapshotDataSchemaMismatch { .. })
    ));

    let (matrix_schemas, matrix) = finalize(
        SchemaBody::Matrix {
            element: Box::new(SchemaBody::Tuple(
                vec![SchemaBody::Bool, SchemaBody::String].into_boxed_slice(),
            )),
            dimensions: vec![DimensionExpr::Constant(2)].into_boxed_slice(),
        },
        ValueDataDraft::Matrix(
            vec![
                ValueDataDraft::Tuple(
                    vec![
                        ValueDataDraft::Bool(true),
                        ValueDataDraft::String("a".into()),
                    ]
                    .into_boxed_slice(),
                ),
                ValueDataDraft::Tuple(
                    vec![
                        ValueDataDraft::Bool(false),
                        ValueDataDraft::String("b".into()),
                    ]
                    .into_boxed_slice(),
                ),
            ]
            .into_boxed_slice(),
        ),
    );
    let replacement = vec![
        ValueData::Tuple(
            vec![ValueData::Bool(false), ValueData::String("x".into())].into_boxed_slice(),
        ),
        ValueData::Tuple(
            vec![ValueData::Bool(true), ValueData::String("y".into())].into_boxed_slice(),
        ),
    ]
    .into_boxed_slice();
    let matrix = matrix
        .rebuild_matrix(
            replacement,
            &SnapshotValidationContext::new(&matrix_schemas),
        )
        .unwrap();
    assert!(matches!(
        matrix.matrix_view().unwrap().elements(),
        SequenceView::Values(values) if values.len() == 2
    ));
}
