#![cfg(feature = "full")]

use mech_core::{
    snapshot::{MapEntryDraft, NamedValueDraft, SnapshotValidationContext, TableColumnDraft},
    *,
};
#[cfg(feature = "matrix1")]
use nalgebra::Matrix1;
#[cfg(feature = "matrix2")]
use nalgebra::Matrix2;
#[cfg(feature = "matrix2x3")]
use nalgebra::Matrix2x3;
#[cfg(feature = "matrix3")]
use nalgebra::Matrix3;
#[cfg(feature = "matrix3x2")]
use nalgebra::Matrix3x2;
#[cfg(feature = "matrix4")]
use nalgebra::Matrix4;
#[cfg(feature = "row_vector2")]
use nalgebra::RowVector2;
#[cfg(feature = "row_vector3")]
use nalgebra::RowVector3;
#[cfg(feature = "row_vector4")]
use nalgebra::RowVector4;
#[cfg(feature = "vector2")]
use nalgebra::Vector2;
#[cfg(feature = "vector3")]
use nalgebra::Vector3;
#[cfg(feature = "vector4")]
use nalgebra::Vector4;
use nalgebra::{DMatrix, DVector, RowDVector};
use std::rc::Rc;

fn canonical_cell(body: SchemaBody, data: ValueDataDraft) -> ValueCell {
    let schema = SchemaDraft {
        dimension_parameters: Box::new([]),
        body,
    }
    .finalize()
    .unwrap();
    let mut builder = SchemaTableBuilder::new();
    let handle = builder.insert(schema).unwrap();
    let build = builder.finish().unwrap();
    let schema = build.resolve(handle).unwrap();
    let (schemas, _) = build.into_parts();
    let schemas = Rc::new(schemas);
    let value = ValueDraft {
        schema,
        shape_values: Box::new([]),
        data,
    }
    .finalize(&SnapshotValidationContext::new(&schemas))
    .unwrap();
    ValueCell::from_value(value, schemas).unwrap()
}

fn resolved(body: SchemaBody) -> (Schema, ShapeInstance) {
    resolved_with(Vec::new(), body, Vec::new())
}

fn resolved_with(
    parameters: Vec<DimensionParameterDeclaration>,
    body: SchemaBody,
    values: Vec<u64>,
) -> (Schema, ShapeInstance) {
    let schema = SchemaDraft {
        dimension_parameters: parameters.into_boxed_slice(),
        body,
    }
    .finalize()
    .unwrap();
    let shape = schema.instantiate_shape(values.into_boxed_slice()).unwrap();
    (schema, shape)
}

fn check_fixture_storage_compatibility(
    schema: &Schema,
    shape: &ShapeInstance,
    storage: &StorageCapabilityDescriptor,
) -> Result<(), StorageCompatibilityError> {
    match check_schema_storage_compatibility(schema, shape, storage) {
        Ok(()) => Ok(()),
        Err(SchemaStorageCompatibilityError::Storage(error)) => Err(error),
        Err(SchemaStorageCompatibilityError::Semantic(error)) => {
            panic!("invalid schema/shape fixture: {error:?}")
        }
    }
}

fn parameter(
    ordinal: u32,
    lifetime: DimensionLifetime,
    upper_bound: Option<u64>,
) -> DimensionParameterDeclaration {
    DimensionParameterDeclaration {
        id: DimensionParameterId::new(ordinal),
        origin: DimensionParameterOrigin::Explicit,
        lifetime,
        lower_bound: DimensionExpr::Constant(0),
        upper_bound: upper_bound.map(DimensionExpr::Constant),
    }
}

fn f64_matrix(dimensions: Vec<DimensionExpr>) -> SchemaBody {
    SchemaBody::Matrix {
        element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
        dimensions: dimensions.into_boxed_slice(),
    }
}

fn assert_exact_scalar(cell: ValueCell, expected: ScalarMemoryKind) {
    let capabilities = cell.storage_capabilities();
    assert_eq!(capabilities.topology, StorageTopology::Scalar(expected));
    assert_eq!(capabilities.extent, StorageExtentCapability::Single);
    assert_eq!(
        capabilities.addressing.positional,
        PositionalAddressingCapability::None
    );
    assert!(capabilities.addressing.whole_value);
    assert!(!capabilities.access.region_mutable);
    assert!(capabilities.access.canonical_snapshot);
    assert_eq!(
        capabilities.accounting,
        StorageAccountingCapability::FixedScalar
    );
    assert!(capabilities.ownership.detachable);
    assert!(capabilities.publication.atomic_replace);
    cell.validate_storage_contract().unwrap();
}

#[test]
fn every_exact_scalar_and_string_backing_reports_its_actual_capabilities() {
    assert_exact_scalar(
        ValueCell::from_exact(1_u8).unwrap(),
        ScalarMemoryKind::Unsigned(IntegerWidth::W8),
    );
    assert_exact_scalar(
        ValueCell::from_exact(1_u16).unwrap(),
        ScalarMemoryKind::Unsigned(IntegerWidth::W16),
    );
    assert_exact_scalar(
        ValueCell::from_exact(1_u32).unwrap(),
        ScalarMemoryKind::Unsigned(IntegerWidth::W32),
    );
    assert_exact_scalar(
        ValueCell::from_exact(1_u64).unwrap(),
        ScalarMemoryKind::Unsigned(IntegerWidth::W64),
    );
    assert_exact_scalar(
        ValueCell::from_exact(1_u128).unwrap(),
        ScalarMemoryKind::Unsigned(IntegerWidth::W128),
    );
    assert_exact_scalar(
        ValueCell::from_exact(1_i8).unwrap(),
        ScalarMemoryKind::Signed(IntegerWidth::W8),
    );
    assert_exact_scalar(
        ValueCell::from_exact(1_i16).unwrap(),
        ScalarMemoryKind::Signed(IntegerWidth::W16),
    );
    assert_exact_scalar(
        ValueCell::from_exact(1_i32).unwrap(),
        ScalarMemoryKind::Signed(IntegerWidth::W32),
    );
    assert_exact_scalar(
        ValueCell::from_exact(1_i64).unwrap(),
        ScalarMemoryKind::Signed(IntegerWidth::W64),
    );
    assert_exact_scalar(
        ValueCell::from_exact(1_i128).unwrap(),
        ScalarMemoryKind::Signed(IntegerWidth::W128),
    );
    assert_exact_scalar(
        ValueCell::from_exact(1_f32).unwrap(),
        ScalarMemoryKind::Floating(FloatWidth::W32),
    );
    assert_exact_scalar(
        ValueCell::from_exact(1_f64).unwrap(),
        ScalarMemoryKind::Floating(FloatWidth::W64),
    );
    assert_exact_scalar(
        ValueCell::from_exact(C64::new(1.0, 2.0)).unwrap(),
        ScalarMemoryKind::Complex(FloatWidth::W64),
    );
    assert_exact_scalar(
        ValueCell::from_exact(R64::new(1, 2)).unwrap(),
        ScalarMemoryKind::Rational64,
    );
    assert_exact_scalar(ValueCell::from_exact(true).unwrap(), ScalarMemoryKind::Bool);
    assert_exact_scalar(
        ValueCell::from_exact(1_usize).unwrap(),
        ScalarMemoryKind::Index,
    );

    let string = ValueCell::from_exact("text".to_owned()).unwrap();
    let capabilities = string.storage_capabilities();
    assert_eq!(
        capabilities.topology,
        StorageTopology::Scalar(ScalarMemoryKind::String)
    );
    assert_eq!(capabilities.extent, StorageExtentCapability::Single);
    assert_eq!(
        capabilities.addressing.positional,
        PositionalAddressingCapability::Rank(1)
    );
    assert!(capabilities.addressing.arbitrary_regions);
    assert!(capabilities.access.region_mutable);
    assert_eq!(
        capabilities.accounting,
        StorageAccountingCapability::CanonicalSnapshot
    );
    string.validate_storage_contract().unwrap();
}

fn assert_matrix(cell: ValueCell, extent: StorageExtentCapability, element: ScalarMemoryKind) {
    let capabilities = cell.storage_capabilities();
    assert_eq!(
        capabilities.topology,
        StorageTopology::DenseSequence {
            element: StorageElementKind::Scalar(element),
        }
    );
    assert_eq!(capabilities.extent, extent);
    assert_eq!(
        capabilities.addressing.positional,
        PositionalAddressingCapability::Rank(2)
    );
    assert!(capabilities.addressing.arbitrary_regions);
    assert!(capabilities.canonicalization.recursive);
    assert!(capabilities.access.region_mutable);
    assert_eq!(
        capabilities.accounting,
        StorageAccountingCapability::CanonicalSnapshot
    );
}

#[test]
fn every_exact_matrix_class_reports_fixed_or_resizable_axes() {
    let f64_kind = ScalarMemoryKind::Floating(FloatWidth::W64);
    #[cfg(any(
        feature = "matrix1",
        feature = "matrix2",
        feature = "matrix3",
        feature = "matrix4",
        feature = "matrix2x3",
        feature = "matrix3x2",
        feature = "row_vector2",
        feature = "row_vector3",
        feature = "row_vector4",
        feature = "vector2",
        feature = "vector3",
        feature = "vector4"
    ))]
    macro_rules! fixed {
        ($value:expr, $rows:expr, $columns:expr) => {{
            let cell = ValueCell::from_exact($value).unwrap();
            assert_matrix(
                cell.clone(),
                StorageExtentCapability::FixedDimensions(vec![$rows, $columns].into_boxed_slice()),
                f64_kind,
            );
            cell.validate_storage_contract().unwrap();
        }};
    }
    #[cfg(feature = "matrix1")]
    fixed!(Matrix1::<f64>::zeros(), 1, 1);
    #[cfg(feature = "matrix2")]
    fixed!(Matrix2::<f64>::zeros(), 2, 2);
    #[cfg(feature = "matrix3")]
    fixed!(Matrix3::<f64>::zeros(), 3, 3);
    #[cfg(feature = "matrix4")]
    fixed!(Matrix4::<f64>::zeros(), 4, 4);
    #[cfg(feature = "matrix2x3")]
    fixed!(Matrix2x3::<f64>::zeros(), 2, 3);
    #[cfg(feature = "matrix3x2")]
    fixed!(Matrix3x2::<f64>::zeros(), 3, 2);
    #[cfg(feature = "row_vector2")]
    fixed!(RowVector2::<f64>::zeros(), 1, 2);
    #[cfg(feature = "row_vector3")]
    fixed!(RowVector3::<f64>::zeros(), 1, 3);
    #[cfg(feature = "row_vector4")]
    fixed!(RowVector4::<f64>::zeros(), 1, 4);
    #[cfg(feature = "vector2")]
    fixed!(Vector2::<f64>::zeros(), 2, 1);
    #[cfg(feature = "vector3")]
    fixed!(Vector3::<f64>::zeros(), 3, 1);
    #[cfg(feature = "vector4")]
    fixed!(Vector4::<f64>::zeros(), 4, 1);

    let row = ValueCell::from_exact(RowDVector::<f64>::zeros(3)).unwrap();
    assert_matrix(
        row.clone(),
        StorageExtentCapability::ResizableDimensions(vec![Some(1), None].into_boxed_slice()),
        f64_kind,
    );

    let column = ValueCell::from_exact(DVector::<f64>::zeros(3)).unwrap();
    assert_matrix(
        column.clone(),
        StorageExtentCapability::ResizableDimensions(vec![None, Some(1)].into_boxed_slice()),
        f64_kind,
    );

    let matrix = ValueCell::from_exact(DMatrix::<f64>::zeros(2, 3)).unwrap();
    assert_matrix(
        matrix.clone(),
        StorageExtentCapability::ResizableDimensions(vec![None, None].into_boxed_slice()),
        f64_kind,
    );
    matrix.validate_storage_contract().unwrap();
}

#[test]
fn inferred_vector_schemas_preserve_their_invariant_axes() {
    let row = ValueCell::from_exact(RowDVector::<f64>::zeros(3)).unwrap();
    let column = ValueCell::from_exact(DVector::<f64>::zeros(3)).unwrap();
    for cell in [&row, &column] {
        cell.validate_storage_contract().unwrap();
    }
    let row_descriptor = row.resolved_descriptor().unwrap();
    let column_descriptor = column.resolved_descriptor().unwrap();
    let SchemaBody::Matrix {
        dimensions: row_dimensions,
        ..
    } = row_descriptor.schema().body()
    else {
        panic!("row vector must retain a matrix schema")
    };
    let SchemaBody::Matrix {
        dimensions: column_dimensions,
        ..
    } = column_descriptor.schema().body()
    else {
        panic!("column vector must retain a matrix schema")
    };
    assert_eq!(row_dimensions[0], DimensionExpr::Constant(1));
    assert!(matches!(row_dimensions[1], DimensionExpr::Parameter(_)));
    assert!(matches!(column_dimensions[0], DimensionExpr::Parameter(_)));
    assert_eq!(column_dimensions[1], DimensionExpr::Constant(1));
}

#[test]
fn canonical_value_backing_is_universal_for_scalar_and_aggregate_schemas() {
    let cases = vec![
        (SchemaBody::Bool, ValueDataDraft::Bool(true)),
        (
            SchemaBody::Matrix {
                element: Box::new(SchemaBody::Bool),
                dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(2)]
                    .into_boxed_slice(),
            },
            ValueDataDraft::Matrix(
                vec![ValueDataDraft::Bool(true), ValueDataDraft::Bool(false)].into_boxed_slice(),
            ),
        ),
        (
            SchemaBody::Record(
                vec![SchemaField {
                    name: "flag".to_owned(),
                    schema: SchemaBody::Bool,
                }]
                .into_boxed_slice(),
            ),
            ValueDataDraft::Record(
                vec![NamedValueDraft {
                    name: "flag".to_owned(),
                    value: ValueDataDraft::Bool(true),
                }]
                .into_boxed_slice(),
            ),
        ),
        (
            SchemaBody::Table {
                columns: vec![SchemaField {
                    name: "flag".to_owned(),
                    schema: SchemaBody::Bool,
                }]
                .into_boxed_slice(),
                rows: DimensionExpr::Constant(1).into(),
            },
            ValueDataDraft::Table(
                vec![TableColumnDraft {
                    name: "flag".to_owned(),
                    values: vec![ValueDataDraft::Bool(true)].into_boxed_slice(),
                }]
                .into_boxed_slice(),
            ),
        ),
        (
            SchemaBody::Set {
                element: Box::new(SchemaBody::Bool),
                cardinality: DimensionExpr::Constant(1).into(),
            },
            ValueDataDraft::Set(vec![ValueDataDraft::Bool(true)].into_boxed_slice()),
        ),
        (
            SchemaBody::Map {
                key: Box::new(SchemaBody::Bool),
                value: Box::new(SchemaBody::String),
                cardinality: DimensionExpr::Constant(1).into(),
            },
            ValueDataDraft::Map(
                vec![MapEntryDraft {
                    items: vec![
                        ValueDataDraft::Bool(true),
                        ValueDataDraft::String("value".to_owned()),
                    ]
                    .into_boxed_slice(),
                }]
                .into_boxed_slice(),
            ),
        ),
    ];

    for (body, data) in cases {
        let cell = canonical_cell(body, data);
        let capabilities = cell.storage_capabilities();
        assert_eq!(capabilities.topology, StorageTopology::CanonicalValue);
        assert_eq!(capabilities.extent, StorageExtentCapability::Any);
        assert_eq!(
            capabilities.addressing.positional,
            PositionalAddressingCapability::AnyRank
        );
        assert!(capabilities.addressing.named_members);
        assert!(capabilities.addressing.keyed_members);
        assert!(capabilities.addressing.arbitrary_regions);
        assert!(capabilities.canonicalization.self_describing);
        assert!(capabilities.canonicalization.recursive);
        assert!(capabilities.canonicalization.tagged);
        assert!(capabilities.canonicalization.ordered_keys);
        assert!(capabilities.canonicalization.unique_keys);
        assert_eq!(
            capabilities.accounting,
            StorageAccountingCapability::CanonicalSnapshot
        );
        cell.validate_storage_contract().unwrap();
    }
}

#[test]
fn canonical_value_accepts_every_semantic_topology() {
    let canonical =
        canonical_cell(SchemaBody::Bool, ValueDataDraft::Bool(true)).storage_capabilities();
    let bodies = vec![
        SchemaBody::Dynamic,
        SchemaBody::Bool,
        SchemaBody::Enum {
            key: NominalKey::from_bytes([1; 32]),
            variants: Box::new([]),
        },
        SchemaBody::Tuple(Box::new([])),
        f64_matrix(vec![DimensionExpr::Constant(1)]),
        SchemaBody::Table {
            columns: Box::new([]),
            rows: DimensionExpr::Constant(0).into(),
        },
        SchemaBody::Set {
            element: Box::new(SchemaBody::Bool),
            cardinality: DimensionExpr::Constant(0).into(),
        },
        SchemaBody::Map {
            key: Box::new(SchemaBody::Bool),
            value: Box::new(SchemaBody::Bool),
            cardinality: DimensionExpr::Constant(0).into(),
        },
        SchemaBody::ReifiedType,
    ];
    for body in bodies {
        let (schema, shape) = resolved(body);
        check_fixture_storage_compatibility(&schema, &shape, &canonical).unwrap();
    }
}

#[test]
fn scalar_and_dense_compatibility_preserve_schema_authority() {
    let scalar = ValueCell::from_exact(1_f64).unwrap().storage_capabilities();
    let (f64_schema, f64_shape) = resolved(SchemaBody::FloatingPoint(FloatWidth::W64));
    check_fixture_storage_compatibility(&f64_schema, &f64_shape, &scalar).unwrap();
    let (bool_schema, bool_shape) = resolved(SchemaBody::Bool);
    assert_eq!(
        check_fixture_storage_compatibility(&bool_schema, &bool_shape, &scalar),
        Err(StorageCompatibilityError::ScalarKindMismatch)
    );
    let (dynamic_schema, dynamic_shape) = resolved(SchemaBody::Dynamic);
    assert_eq!(
        check_fixture_storage_compatibility(&dynamic_schema, &dynamic_shape, &scalar),
        Err(StorageCompatibilityError::TopologyMismatch)
    );

    let mut matrix = ValueCell::from_exact(DMatrix::<f64>::zeros(2, 2))
        .unwrap()
        .storage_capabilities();
    matrix.extent = StorageExtentCapability::FixedDimensions(vec![2, 2].into_boxed_slice());
    let (matching_schema, matching_shape) = resolved(f64_matrix(vec![
        DimensionExpr::Constant(2),
        DimensionExpr::Constant(2),
    ]));
    check_fixture_storage_compatibility(&matching_schema, &matching_shape, &matrix).unwrap();
    let (wrong_element_schema, wrong_element_shape) = resolved(SchemaBody::Matrix {
        element: Box::new(SchemaBody::Bool),
        dimensions: vec![DimensionExpr::Constant(2), DimensionExpr::Constant(2)].into_boxed_slice(),
    });
    assert_eq!(
        check_fixture_storage_compatibility(&wrong_element_schema, &wrong_element_shape, &matrix,),
        Err(StorageCompatibilityError::DenseElementMismatch)
    );
    let (aggregate_schema, aggregate_shape) = resolved(SchemaBody::Matrix {
        element: Box::new(SchemaBody::Tuple(Box::new([]))),
        dimensions: vec![DimensionExpr::Constant(2), DimensionExpr::Constant(2)].into_boxed_slice(),
    });
    assert_eq!(
        check_fixture_storage_compatibility(&aggregate_schema, &aggregate_shape, &matrix),
        Err(StorageCompatibilityError::DenseElementMismatch)
    );
    assert_eq!(
        check_fixture_storage_compatibility(&dynamic_schema, &dynamic_shape, &matrix),
        Err(StorageCompatibilityError::TopologyMismatch)
    );
}

#[test]
fn public_compatibility_boundary_derives_the_contract_from_its_schema() {
    let scalar = ValueCell::from_exact(1_f64).unwrap().storage_capabilities();
    let (f64_schema, f64_shape) = resolved(SchemaBody::FloatingPoint(FloatWidth::W64));
    check_schema_storage_compatibility(&f64_schema, &f64_shape, &scalar).unwrap();

    let (bool_schema, bool_shape) = resolved(SchemaBody::Bool);
    assert_eq!(
        check_schema_storage_compatibility(&bool_schema, &bool_shape, &scalar),
        Err(SchemaStorageCompatibilityError::Storage(
            StorageCompatibilityError::ScalarKindMismatch,
        ))
    );

    let mut transposed = ValueCell::from_exact(DMatrix::<f64>::zeros(3, 2))
        .unwrap()
        .storage_capabilities();
    transposed.extent = StorageExtentCapability::FixedDimensions(vec![3, 2].into_boxed_slice());
    let (matrix_schema, matrix_shape) = resolved(f64_matrix(vec![
        DimensionExpr::Constant(2),
        DimensionExpr::Constant(3),
    ]));
    assert_eq!(
        check_schema_storage_compatibility(&matrix_schema, &matrix_shape, &transposed),
        Err(SchemaStorageCompatibilityError::Storage(
            StorageCompatibilityError::AxisMismatch,
        ))
    );

    let parameter_id = DimensionParameterId::new(0);
    let (_, foreign_shape) = resolved_with(
        vec![parameter(0, DimensionLifetime::Activation, Some(4))],
        f64_matrix(vec![
            DimensionExpr::Parameter(parameter_id),
            DimensionExpr::Constant(1),
        ]),
        vec![2],
    );
    assert_eq!(
        check_schema_storage_compatibility(&f64_schema, &foreign_shape, &scalar),
        Err(SchemaStorageCompatibilityError::Semantic(
            SemanticModelError::ShapeParameterCountMismatchV1 {
                expected: 0,
                actual: 1,
            },
        ))
    );

    let canonical = canonical_cell(SchemaBody::Bool, ValueDataDraft::Bool(true));
    check_schema_storage_compatibility(
        &matrix_schema,
        &matrix_shape,
        &canonical.storage_capabilities(),
    )
    .unwrap();
}

#[test]
fn fixed_and_resizable_axes_enforce_rank_value_and_evolution() {
    let mut fixed = ValueCell::from_exact(DMatrix::<f64>::zeros(2, 2))
        .unwrap()
        .storage_capabilities();
    fixed.extent = StorageExtentCapability::FixedDimensions(vec![2, 2].into_boxed_slice());
    let (rank_schema, rank_shape) = resolved(f64_matrix(vec![DimensionExpr::Constant(2)]));
    assert_eq!(
        check_fixture_storage_compatibility(&rank_schema, &rank_shape, &fixed),
        Err(StorageCompatibilityError::RankMismatch)
    );
    let (axis_schema, axis_shape) = resolved(f64_matrix(vec![
        DimensionExpr::Constant(2),
        DimensionExpr::Constant(3),
    ]));
    assert_eq!(
        check_fixture_storage_compatibility(&axis_schema, &axis_shape, &fixed),
        Err(StorageCompatibilityError::AxisMismatch)
    );

    let id = DimensionParameterId::new(0);
    let (turn_schema, turn_shape) = resolved_with(
        vec![parameter(0, DimensionLifetime::Turn, Some(4))],
        f64_matrix(vec![
            DimensionExpr::Parameter(id),
            DimensionExpr::Constant(2),
        ]),
        vec![2],
    );
    assert_eq!(
        check_fixture_storage_compatibility(&turn_schema, &turn_shape, &fixed),
        Err(StorageCompatibilityError::DynamicAxisUnsupported)
    );
    let (activation_schema, activation_shape) = resolved_with(
        vec![parameter(0, DimensionLifetime::Activation, Some(4))],
        f64_matrix(vec![
            DimensionExpr::Parameter(id),
            DimensionExpr::Constant(2),
        ]),
        vec![2],
    );
    check_fixture_storage_compatibility(&activation_schema, &activation_shape, &fixed).unwrap();

    let row = ValueCell::from_exact(RowDVector::<f64>::zeros(3))
        .unwrap()
        .storage_capabilities();
    let (row_schema, row_shape) = resolved_with(
        vec![parameter(0, DimensionLifetime::Turn, None)],
        f64_matrix(vec![
            DimensionExpr::Parameter(id),
            DimensionExpr::Constant(3),
        ]),
        vec![1],
    );
    assert_eq!(
        check_fixture_storage_compatibility(&row_schema, &row_shape, &row),
        Err(StorageCompatibilityError::DynamicAxisUnsupported)
    );

    let column = ValueCell::from_exact(DVector::<f64>::zeros(3))
        .unwrap()
        .storage_capabilities();
    let (column_schema, column_shape) = resolved_with(
        vec![parameter(0, DimensionLifetime::Turn, None)],
        f64_matrix(vec![
            DimensionExpr::Constant(3),
            DimensionExpr::Parameter(id),
        ]),
        vec![1],
    );
    assert_eq!(
        check_fixture_storage_compatibility(&column_schema, &column_shape, &column),
        Err(StorageCompatibilityError::DynamicAxisUnsupported)
    );

    let dynamic = ValueCell::from_exact(DMatrix::<f64>::zeros(2, 3))
        .unwrap()
        .storage_capabilities();
    let second = DimensionParameterId::new(1);
    let (dynamic_schema, dynamic_shape) = resolved_with(
        vec![
            parameter(0, DimensionLifetime::Turn, Some(8)),
            parameter(1, DimensionLifetime::Turn, None),
        ],
        f64_matrix(vec![
            DimensionExpr::Parameter(id),
            DimensionExpr::Parameter(second),
        ]),
        vec![2, 3],
    );
    check_fixture_storage_compatibility(&dynamic_schema, &dynamic_shape, &dynamic).unwrap();
}

#[test]
fn addressing_canonicalization_and_accounting_fail_independently() {
    let canonical =
        canonical_cell(SchemaBody::Bool, ValueDataDraft::Bool(true)).storage_capabilities();

    let (tuple_schema, tuple_shape) = resolved(SchemaBody::Tuple(Box::new([])));
    let mut no_position = canonical.clone();
    no_position.addressing.positional = PositionalAddressingCapability::None;
    assert_eq!(
        check_fixture_storage_compatibility(&tuple_schema, &tuple_shape, &no_position),
        Err(StorageCompatibilityError::PositionalAddressingUnsupported)
    );

    let (record_schema, record_shape) = resolved(SchemaBody::Record(Box::new([])));
    let mut no_names = canonical.clone();
    no_names.addressing.named_members = false;
    assert_eq!(
        check_fixture_storage_compatibility(&record_schema, &record_shape, &no_names),
        Err(StorageCompatibilityError::NamedAddressingUnsupported)
    );

    let (set_schema, set_shape) = resolved(SchemaBody::Set {
        element: Box::new(SchemaBody::Bool),
        cardinality: DimensionExpr::Constant(0).into(),
    });
    let mut no_keys = canonical.clone();
    no_keys.addressing.keyed_members = false;
    assert_eq!(
        check_fixture_storage_compatibility(&set_schema, &set_shape, &no_keys),
        Err(StorageCompatibilityError::KeyedAddressingUnsupported)
    );
    let mut no_recursive = canonical.clone();
    no_recursive.canonicalization.recursive = false;
    assert_eq!(
        check_fixture_storage_compatibility(&set_schema, &set_shape, &no_recursive),
        Err(StorageCompatibilityError::CanonicalizationUnsupported)
    );

    let string = ValueCell::from_exact("value".to_owned()).unwrap();
    let mut fixed_accounting = string.storage_capabilities();
    fixed_accounting.accounting = StorageAccountingCapability::FixedScalar;
    let (string_schema, string_shape) = resolved(SchemaBody::String);
    assert_eq!(
        check_fixture_storage_compatibility(&string_schema, &string_shape, &fixed_accounting,),
        Err(StorageCompatibilityError::AccountingUnsupported)
    );
    check_fixture_storage_compatibility(&set_schema, &set_shape, &canonical).unwrap();
}

fn assert_replacement_laws(target: ValueCell, replacement: ValueCell, invalid: ValueCell) {
    target.replace(&replacement.snapshot().unwrap()).unwrap();
    assert!(target.snapshot_eq(&replacement).unwrap());
    let before_failure = target.detached_clone().unwrap();
    assert!(target.replace(&invalid.snapshot().unwrap()).is_err());
    assert!(target.snapshot_eq(&before_failure).unwrap());
    assert!(target.storage_capabilities().ownership.detachable);
    let detached = target.detached_clone().unwrap();
    assert!(!target.same_logical_cell(&detached));
    assert!(!target.same_storage(&detached));
    assert!(target.snapshot_eq(&detached).unwrap());
}

#[test]
fn truthful_descriptors_match_replacement_and_detachment_behavior() {
    let invalid_string = ValueCell::from_exact("invalid".to_owned()).unwrap();
    let invalid_bool = ValueCell::from_exact(false).unwrap();
    assert_replacement_laws(
        ValueCell::from_exact(1_f64).unwrap(),
        ValueCell::from_exact(2_f64).unwrap(),
        invalid_string.clone(),
    );
    assert_replacement_laws(
        ValueCell::from_exact("one".to_owned()).unwrap(),
        ValueCell::from_exact("two".to_owned()).unwrap(),
        invalid_bool.clone(),
    );
    assert_replacement_laws(
        ValueCell::from_exact(DMatrix::<f64>::zeros(2, 3)).unwrap(),
        ValueCell::from_exact(DMatrix::<f64>::from_element(2, 3, 2.0)).unwrap(),
        invalid_bool.clone(),
    );

    let record_body = SchemaBody::Record(
        vec![SchemaField {
            name: "flag".to_owned(),
            schema: SchemaBody::Bool,
        }]
        .into_boxed_slice(),
    );
    let record = |value| {
        canonical_cell(
            record_body.clone(),
            ValueDataDraft::Record(
                vec![NamedValueDraft {
                    name: "flag".to_owned(),
                    value: ValueDataDraft::Bool(value),
                }]
                .into_boxed_slice(),
            ),
        )
    };
    assert_replacement_laws(record(false), record(true), invalid_bool);
}

#[cfg(feature = "matrix2")]
#[test]
fn fixed_matrix_descriptor_matches_replacement_and_detachment_behavior() {
    assert_replacement_laws(
        ValueCell::from_exact(Matrix2::<f64>::zeros()).unwrap(),
        ValueCell::from_exact(Matrix2::<f64>::from_element(2.0)).unwrap(),
        ValueCell::from_exact(Matrix2::<bool>::from_element(false)).unwrap(),
    );
}

#[test]
fn storage_capability_types_have_no_serde_surface() {
    let source = include_str!("../src/memory_contract/storage_capability.rs");
    assert!(!source.contains("Serialize"));
    assert!(!source.contains("Deserialize"));
}
