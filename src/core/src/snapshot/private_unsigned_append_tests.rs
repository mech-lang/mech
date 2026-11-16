use super::*;
use crate::{
    DimensionLifetime, DimensionParameterDeclaration, DimensionParameterOrigin, SchemaDraft,
    SchemaTableBuilder,
};

fn row(width: IntegerWidth) -> (SchemaId, Arc<SchemaTable>) {
    let mut builder = SchemaTableBuilder::new();
    let handle = builder
        .insert(
            SchemaDraft {
                body: SchemaBody::Matrix {
                    element: Box::new(SchemaBody::UnsignedInteger(width)),
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
                    lower_bound: DimensionExpr::Constant(0),
                    upper_bound: None,
                }]
                .into(),
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let build = builder.finish().unwrap();
    (build.resolve(handle).unwrap(), Arc::new(build.table))
}

#[test]
fn native_reserved_views_initialize_spare_cells_then_publish_all_widths_and_empty_prefixes() {
    macro_rules! check {
        ($kind:ident,$width:ident,$ty:ty,$word:expr) => {{
            for initial in [0usize, 1] {
                let (id, schemas) = row(IntegerWidth::$width);
                let shape = schemas
                    .get(id)
                    .unwrap()
                    .instantiate_shape([initial as u64].into())
                    .unwrap();
                let mut words = Vec::<$ty>::with_capacity(8);
                if initial == 1 {
                    words.push(7);
                }
                let pointer = words.as_mut_ptr();
                let value = Value::from_unsigned_matrix_buffer(
                    id,
                    shape.clone(),
                    UnsignedMatrixBuffer::$kind(words),
                    schemas.clone(),
                )
                .unwrap();
                let budget = crate::ManagedMemoryBudget::new(1_000_000);
                let mut reserve = budget
                    .reserve_capacity(value.memory_budget_admission_bytes(&budget).unwrap())
                    .unwrap();
                let value = value.into_memory_budget(&mut reserve).unwrap();
                drop(reserve);
                let prep = value
                    .prepare_private_unsigned_appends(&budget)
                    .unwrap()
                    .unwrap();
                let crate::snapshot::UnsignedMatrixOwnershipTransfer::Owned(owned) =
                    value.try_into_owned_unsigned_matrix(&budget).unwrap()
                else {
                    panic!()
                };
                let mut private = owned.into_private_appends(shape, prep).unwrap();
                // SAFETY: invalid geometry is explicitly rejected without
                // accessing spare cells or requiring an initialization claim.
                unsafe {
                    assert!(private.assume_initialized_prefix(9).is_err());
                    if initial > 0 {
                        assert!(private.assume_initialized_prefix(0).is_err());
                    }
                }
                let [address, length, capacity] = private.reserved_native_view().unwrap();
                assert_eq!(address, pointer as u64);
                assert_eq!(length, initial as u64);
                assert_eq!(capacity, 8);
                let pointer = core::ptr::with_exposed_provenance_mut::<$ty>(address as usize);
                // SAFETY: this synchronous producer owns the actual Vec view;
                // two spare cells fit in capacity and are initialized before
                // length is committed. No slice borrow is formed during writes.
                unsafe {
                    pointer.add(initial).write($word);
                    pointer.add(initial + 1).write(0);
                    private.assume_initialized_prefix(initial + 2).unwrap();
                }
                let frozen = private.finish();
                frozen.validate_against(&schemas).unwrap();
                let ValueData::Matrix(matrix) = frozen.data() else {
                    panic!()
                };
                let SequenceView::$kind(cells) = matrix.elements() else {
                    panic!()
                };
                assert_eq!(cells.as_ptr(), pointer);
                assert_eq!(cells.len(), initial + 2);
                if initial == 1 {
                    assert_eq!(cells[0], 7);
                }
                assert_eq!(cells[initial], $word);
                assert_eq!(cells[initial + 1], 0);
                assert_eq!(frozen.shape().parameter_values(), &[(initial + 2) as u64]);
                drop(frozen);
                assert_eq!((budget.used_bytes(), budget.retained_nodes()), (0, 0));
            }
        }};
    }
    check!(U8, W8, u8, 255);
    check!(U16, W16, u16, 65535);
    check!(U32, W32, u32, u32::MAX);
    check!(U64, W64, u64, u64::MAX);
}

#[test]
fn shared_replacement_shape_finishes_unselected_private_append() {
    shared_replacement_shape_case(false);
}

#[test]
fn shared_replacement_shape_finishes_selected_private_append() {
    shared_replacement_shape_case(true);
}

fn shared_replacement_shape_case(append: bool) {
    macro_rules! check {
        ($kind:ident, $width:ident, $ty:ty) => {{
            for abort in [false, true] {
                let (id, schemas) = row(IntegerWidth::$width);
                let shape = schemas
                    .get(id)
                    .unwrap()
                    .instantiate_shape([2u64].into())
                    .unwrap();
                let mut initial = Vec::<$ty>::with_capacity(8);
                initial.extend_from_slice(&[7, 8]);
                let source = Value::from_unsigned_matrix_buffer(
                    id,
                    shape,
                    UnsignedMatrixBuffer::$kind(initial),
                    schemas.clone(),
                )
                .unwrap();
                let budget = crate::ManagedMemoryBudget::new(1_000_000);
                let mut admission = budget
                    .reserve_capacity(source.memory_budget_admission_bytes(&budget).unwrap())
                    .unwrap();
                let source = source.into_memory_budget(&mut admission).unwrap();
                drop(admission);
                let alias = source.clone();
                let original = source.canonical_data_draft().unwrap();
                let baseline = (budget.used_bytes(), budget.retained_nodes());
                let preparation = source
                    .prepare_private_unsigned_appends(&budget)
                    .unwrap()
                    .unwrap();
                let constructor = source
                    .unsigned_matrix_replacement_constructor(schemas.clone())
                    .unwrap();
                let mut cells = Vec::<$ty>::with_capacity(8);
                cells.extend_from_slice(&[9, 8]);
                let value = constructor
                    .finish(UnsignedMatrixBuffer::$kind(cells))
                    .unwrap();
                assert!(Arc::ptr_eq(&source.shape, &value.shape));
                let mut admission = budget
                    .reserve_capacity(value.memory_budget_admission_bytes(&budget).unwrap())
                    .unwrap();
                let private = value.into_private_unsigned_matrix(&mut admission).unwrap();
                drop(admission);
                let mut private = private.with_append_preparation(preparation).unwrap();
                if append {
                    assert!(private.append_in_capacity(11));
                }
                if abort {
                    drop(private);
                    assert_eq!((budget.used_bytes(), budget.retained_nodes()), baseline);
                } else {
                    let frozen = private.finish();
                    frozen.validate_against(&schemas).unwrap();
                    assert!(!Arc::ptr_eq(&source.shape, &frozen.shape));
                    assert!(!source.shares_frozen_storage(&frozen));
                    assert_eq!(
                        frozen.shape().parameter_values(),
                        &[if append { 3 } else { 2 }]
                    );
                    let expected = if append { vec![9, 8, 11] } else { vec![9, 8] };
                    assert_eq!(
                        frozen.canonical_data_draft().unwrap(),
                        crate::ValueDataDraft::Matrix(
                            expected
                                .into_iter()
                                .map(crate::ValueDataDraft::$kind)
                                .collect()
                        )
                    );
                    drop(frozen);
                    assert_eq!((budget.used_bytes(), budget.retained_nodes()), baseline);
                }
                assert_eq!(source.shape().parameter_values(), &[2]);
                assert_eq!(alias.shape().parameter_values(), &[2]);
                assert_eq!(source.canonical_data_draft().unwrap(), original);
                assert_eq!(alias.canonical_data_draft().unwrap(), original);
                source.validate_against(&schemas).unwrap();
                alias.validate_against(&schemas).unwrap();
                drop((source, alias));
                assert_eq!((budget.used_bytes(), budget.retained_nodes()), (0, 0));
            }
        }};
    }
    check!(U8, W8, u8);
    check!(U16, W16, u16);
    check!(U32, W32, u32);
    check!(U64, W64, u64);
}

#[test]
fn append_scratch_uses_current_length_when_preparation_came_from_another_value() {
    let mut builder = SchemaTableBuilder::new();
    let schema = SchemaDraft {
        body: SchemaBody::Matrix {
            element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W64)),
            dimensions: [
                DimensionExpr::Parameter(DimensionParameterId::new(0)),
                DimensionExpr::Parameter(DimensionParameterId::new(1)),
            ]
            .into(),
        },
        dimension_parameters: (0..2)
            .map(|i| DimensionParameterDeclaration {
                id: DimensionParameterId::new(i),
                origin: DimensionParameterOrigin::Explicit,
                lifetime: DimensionLifetime::Activation,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: None,
            })
            .collect(),
    }
    .finalize()
    .unwrap();
    let handle = builder.insert(schema).unwrap();
    let built = builder.finish().unwrap();
    let id = built.resolve(handle).unwrap();
    let schemas = Arc::new(built.table);
    let budget = crate::ManagedMemoryBudget::new(1_000_000);
    let make = |length| {
        let shape = schemas
            .get(id)
            .unwrap()
            .instantiate_shape([1, length].into())
            .unwrap();
        let mut words = Vec::with_capacity(8);
        words.extend(7..7 + length);
        Value::from_unsigned_matrix_buffer(
            id,
            shape,
            UnsignedMatrixBuffer::U64(words),
            schemas.clone(),
        )
        .unwrap()
    };
    let source = make(2);
    let mut admission = budget
        .reserve_capacity(source.memory_budget_admission_bytes(&budget).unwrap())
        .unwrap();
    let source = source.into_memory_budget(&mut admission).unwrap();
    drop(admission);
    let preparation = source
        .prepare_private_unsigned_appends(&budget)
        .unwrap()
        .unwrap();
    let current = make(3);
    let mut admission = budget
        .reserve_capacity(current.memory_budget_admission_bytes(&budget).unwrap())
        .unwrap();
    let mut private = current
        .into_private_unsigned_matrix(&mut admission)
        .unwrap()
        .with_append_preparation(preparation)
        .unwrap();
    drop(admission);
    assert!(private.append_in_capacity(11));
    let frozen = private.finish();
    frozen.validate_against(&schemas).unwrap();
    assert_eq!(frozen.shape().parameter_values(), &[1, 4]);
    assert_eq!(source.shape().parameter_values(), &[1, 2]);
    source.validate_against(&schemas).unwrap();
    drop((frozen, source));
    assert_eq!((budget.used_bytes(), budget.retained_nodes()), (0, 0));
}
