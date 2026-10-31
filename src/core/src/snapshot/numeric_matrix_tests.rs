use super::*;
use crate::snapshot::SequenceView;
use crate::{DimensionExpr, IntegerInterval, SchemaDraft, SchemaTableBuilder};

fn schema(
    element: SchemaBody,
    rows: u64,
    columns: u64,
) -> (SchemaId, ShapeInstance, Arc<SchemaTable>) {
    let mut builder = SchemaTableBuilder::new();
    let handle = builder
        .insert(
            SchemaDraft {
                body: SchemaBody::Matrix {
                    element: Box::new(element),
                    dimensions: [
                        DimensionExpr::Constant(rows),
                        DimensionExpr::Constant(columns),
                    ]
                    .into(),
                },
                dimension_parameters: Box::new([]),
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let build = builder.finish().unwrap();
    let id = build.resolve(handle).unwrap();
    let schemas = Arc::new(build.table);
    let shape = schemas
        .get(id)
        .unwrap()
        .instantiate_shape(Box::new([]))
        .unwrap();
    (id, shape, schemas)
}

fn ordinary(
    id: SchemaId,
    cells: Box<[ValueDataDraft]>,
    schemas: &Arc<SchemaTable>,
) -> Result<Value, SnapshotValueError> {
    ValueDraft {
        schema: id,
        shape_values: Box::new([]),
        data: ValueDataDraft::Matrix(cells),
    }
    .finalize(&SnapshotValidationContext::with_shared_schemas(schemas))
}

#[test]
fn numeric_matrix_constructors_transfer_all_thirteen_buffers_with_exact_canonical_values() {
    macro_rules! check {
        ($constructor:ident, $variant:ident, $body:expr, $values:expr) => {{
            let all = $values;
            for count in [0, 1, 4] {
                let values = all[..count].to_vec().into_boxed_slice();
                let expected_cells = values
                    .iter()
                    .copied()
                    .map(ValueDataDraft::$variant)
                    .collect();
                let pointer = values.as_ptr();
                let (id, shape, schemas) = schema(
                    $body,
                    if count == 4 { 2 } else { 1 },
                    if count == 4 { 2 } else { count as u64 },
                );
                let expected = ordinary(id, expected_cells, &schemas).unwrap();
                let actual = Value::$constructor(id, shape, values, Arc::clone(&schemas)).unwrap();
                assert_eq!(actual.schema_key(), expected.schema_key());
                assert_eq!(actual.shape(), expected.shape());
                assert_eq!(
                    actual.canonical_data_draft().unwrap(),
                    expected.canonical_data_draft().unwrap()
                );
                assert_eq!(
                    actual.value_hash(&schemas).unwrap(),
                    expected.value_hash(&schemas).unwrap()
                );
                assert_eq!(
                    actual.retained_footprint(&schemas).unwrap(),
                    expected.retained_footprint(&schemas).unwrap()
                );
                let ValueData::Matrix(matrix) = actual.data() else {
                    panic!("Matrix")
                };
                let SequenceView::$variant(values) = matrix.elements() else {
                    panic!("exact packed kind")
                };
                assert_eq!(
                    values.as_ptr(),
                    pointer,
                    "no payload clone or repacking allocation"
                );
                actual.validate_against(&schemas).unwrap();
            }
        }};
    }
    macro_rules! integer {
        ($constructor:ident, $variant:ident, $kind:ident, $width:ident, $ty:ty) => {
            check!(
                $constructor,
                $variant,
                SchemaBody::$kind(IntegerWidth::$width),
                [<$ty>::MIN, 0, 1, <$ty>::MAX]
            );
        };
    }
    integer!(from_u8_matrix, U8, UnsignedInteger, W8, u8);
    integer!(from_u16_matrix, U16, UnsignedInteger, W16, u16);
    integer!(from_u32_matrix, U32, UnsignedInteger, W32, u32);
    integer!(from_u64_matrix, U64, UnsignedInteger, W64, u64);
    integer!(from_u128_matrix, U128, UnsignedInteger, W128, u128);
    integer!(from_i8_matrix, I8, SignedInteger, W8, i8);
    integer!(from_i16_matrix, I16, SignedInteger, W16, i16);
    integer!(from_i32_matrix, I32, SignedInteger, W32, i32);
    integer!(from_i64_matrix, I64, SignedInteger, W64, i64);
    integer!(from_i128_matrix, I128, SignedInteger, W128, i128);
    check!(
        from_index_matrix,
        Index,
        SchemaBody::Index,
        [1, 2, u64::MAX - 1, u64::MAX]
    );
    check!(
        from_f32_matrix,
        F32,
        SchemaBody::FloatingPoint(FloatWidth::W32),
        [0, 0x8000_0000, 0x7fc0_0001, 0xff80_0000].map(F32Bits::from_bits)
    );
    check!(
        from_f64_matrix,
        F64,
        SchemaBody::FloatingPoint(FloatWidth::W64),
        [
            0,
            0x8000_0000_0000_0000,
            0x7ff8_0000_0000_0001,
            0xfff0_0000_0000_0000
        ]
        .map(F64Bits::from_bits)
    );
}

#[test]
fn numeric_matrix_cardinality_and_index_errors_match_ordinary_finalization() {
    for element in [
        SchemaBody::UnsignedInteger(IntegerWidth::W64),
        SchemaBody::Index,
    ] {
        let (id, shape, schemas) = schema(element.clone(), 2, 2);
        for values in [vec![], vec![1, 0, 3], vec![1, 0, 3, 4], vec![1, 2, 3, 4, 5]] {
            let index = element == SchemaBody::Index;
            if !index && values.len() == 4 {
                continue;
            }
            let expected = ordinary(
                id,
                values
                    .iter()
                    .copied()
                    .map(|n| {
                        if index {
                            ValueDataDraft::Index(n)
                        } else {
                            ValueDataDraft::U64(n)
                        }
                    })
                    .collect(),
                &schemas,
            )
            .unwrap_err();
            let actual = if index {
                Value::from_index_matrix(id, shape.clone(), values.into(), Arc::clone(&schemas))
            } else {
                Value::from_u64_matrix(id, shape.clone(), values.into(), Arc::clone(&schemas))
            }
            .unwrap_err();
            assert_eq!(actual, expected);
        }
    }
}

#[test]
fn typed_numeric_matrix_checks_kinds_constraints_and_parameter_shapes() {
    for element in [
        SchemaBody::UnsignedInteger(IntegerWidth::W8),
        SchemaBody::String,
        SchemaBody::IntegerInterval(IntegerInterval::Unsigned {
            width: IntegerWidth::W64,
            lower: 1,
            upper: 2,
            upper_inclusive: true,
        }),
    ] {
        let (id, shape, schemas) = schema(element, 1, 2);
        assert!(matches!(
            Value::from_u64_matrix(id, shape, [1, 2].into(), Arc::clone(&schemas)),
            Err(SnapshotValueError::SnapshotSchemaDefinitionMismatch { .. })
        ));
    }
    use crate::{
        DimensionLifetime, DimensionParameterDeclaration, DimensionParameterId,
        DimensionParameterOrigin,
    };
    let mut builder = SchemaTableBuilder::new();
    let handle = builder
        .insert(
            SchemaDraft {
                body: SchemaBody::Matrix {
                    element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W64)),
                    dimensions: [
                        DimensionExpr::Constant(1),
                        DimensionExpr::Parameter(DimensionParameterId::new(0)),
                    ]
                    .into(),
                },
                dimension_parameters: [DimensionParameterDeclaration {
                    id: DimensionParameterId::new(0),
                    origin: DimensionParameterOrigin::Explicit,
                    lifetime: DimensionLifetime::Activation,
                    lower_bound: DimensionExpr::Constant(1),
                    upper_bound: Some(DimensionExpr::Constant(8)),
                }]
                .into(),
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let built = builder.finish().unwrap();
    let dynamic_id = built.resolve(handle).unwrap();
    let dynamic_schemas = Arc::new(built.table);
    let dynamic_shape = dynamic_schemas
        .get(dynamic_id)
        .unwrap()
        .instantiate_shape([2].into())
        .unwrap();
    let actual = Value::from_u64_matrix(
        dynamic_id,
        dynamic_shape.clone(),
        [1, 2].into(),
        Arc::clone(&dynamic_schemas),
    )
    .unwrap();
    let expected = ValueDraft {
        schema: dynamic_id,
        shape_values: [2].into(),
        data: ValueDataDraft::Matrix([ValueDataDraft::U64(1), ValueDataDraft::U64(2)].into()),
    }
    .finalize(&SnapshotValidationContext::with_shared_schemas(
        &dynamic_schemas,
    ))
    .unwrap();
    assert_eq!(
        actual.value_hash(&dynamic_schemas).unwrap(),
        expected.value_hash(&dynamic_schemas).unwrap()
    );
    // ShapeInstance carries parameter values, not a schema identity. Fixed
    // 1x2 and 2x1 schemas legitimately share an empty parameter instance.
    // A mismatched parameter COUNT is the invalid substitution to reject.
    let (id, fixed_shape, schemas) = schema(SchemaBody::UnsignedInteger(IntegerWidth::W64), 1, 2);
    assert!(Value::from_u64_matrix(id, dynamic_shape, [1, 2].into(), schemas).is_err());
    assert!(
        Value::from_u64_matrix(dynamic_id, fixed_shape, [1, 2].into(), dynamic_schemas).is_err()
    );
}

#[test]
fn borrowed_numeric_matrix_validation_rechecks_foreign_bounds_and_extent_overflow() {
    use crate::{
        DimensionLifetime, DimensionParameterDeclaration, DimensionParameterId,
        DimensionParameterOrigin,
    };
    fn parameterized(rows: u64, lower: u64, upper: Option<u64>) -> (SchemaId, Arc<SchemaTable>) {
        let mut builder = SchemaTableBuilder::new();
        let handle = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Matrix {
                        element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W64)),
                        dimensions: [
                            DimensionExpr::Constant(rows),
                            DimensionExpr::Parameter(DimensionParameterId::new(0)),
                        ]
                        .into(),
                    },
                    dimension_parameters: [DimensionParameterDeclaration {
                        id: DimensionParameterId::new(0),
                        origin: DimensionParameterOrigin::Explicit,
                        lifetime: DimensionLifetime::Activation,
                        lower_bound: DimensionExpr::Constant(lower),
                        upper_bound: upper.map(DimensionExpr::Constant),
                    }]
                    .into(),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let built = builder.finish().unwrap();
        (built.resolve(handle).unwrap(), Arc::new(built.table))
    }
    let (foreign_id, foreign) = parameterized(1, 0, None);
    for (rows, lower, upper, parameter) in [
        (1, 2, Some(4), 1),
        (1, 2, Some(4), 5),
        (2, 0, None, u64::MAX),
    ] {
        let shape = foreign
            .get(foreign_id)
            .unwrap()
            .instantiate_shape([parameter].into())
            .unwrap();
        let (id, schemas) = parameterized(rows, lower, upper);
        let key = schemas.entry(id).unwrap().key();
        // Shape/bound/overflow errors precede payload cardinality errors.
        assert_eq!(
            Value::from_u64_matrix(id, shape, Box::new([]), schemas).unwrap_err(),
            SnapshotValueError::SnapshotSchemaDefinitionMismatch { key }
        );
    }
}

#[test]
fn borrowed_numeric_matrix_validation_resolves_original_dimension_expressions() {
    use crate::{
        DimensionLifetime, DimensionParameterDeclaration, DimensionParameterId,
        DimensionParameterOrigin,
    };
    let mut builder = SchemaTableBuilder::new();
    let handle = builder
        .insert(
            SchemaDraft {
                body: SchemaBody::Matrix {
                    element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W64)),
                    dimensions: [
                        DimensionExpr::Constant(2),
                        DimensionExpr::Add(
                            [
                                DimensionExpr::Parameter(DimensionParameterId::new(0)),
                                DimensionExpr::Constant(1),
                            ]
                            .into(),
                        ),
                    ]
                    .into(),
                },
                dimension_parameters: [DimensionParameterDeclaration {
                    id: DimensionParameterId::new(0),
                    origin: DimensionParameterOrigin::Explicit,
                    lifetime: DimensionLifetime::Activation,
                    lower_bound: DimensionExpr::Constant(0),
                    upper_bound: Some(DimensionExpr::Constant(8)),
                }]
                .into(),
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let built = builder.finish().unwrap();
    let id = built.resolve(handle).unwrap();
    let schemas = Arc::new(built.table);
    for parameter in [0, 2, 8] {
        let shape = schemas
            .get(id)
            .unwrap()
            .instantiate_shape([parameter].into())
            .unwrap();
        let count = (2 * (parameter + 1)) as usize;
        let expected = ValueDraft {
            schema: id,
            shape_values: [parameter].into(),
            data: ValueDataDraft::Matrix(vec![ValueDataDraft::U64(7); count].into()),
        }
        .finalize(&SnapshotValidationContext::with_shared_schemas(&schemas))
        .unwrap();
        let actual =
            Value::from_u64_matrix(id, shape, vec![7; count].into(), Arc::clone(&schemas)).unwrap();
        assert_eq!(actual.shape(), expected.shape());
        assert_eq!(
            actual.canonical_data_draft().unwrap(),
            expected.canonical_data_draft().unwrap()
        );
        assert_eq!(
            actual.value_hash(&schemas).unwrap(),
            expected.value_hash(&schemas).unwrap()
        );
        actual.validate_against(&schemas).unwrap();
    }
}
