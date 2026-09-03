//! Schema-aware mutable program cells.
//!
//! A [`ValueCell`] owns identity and schema metadata while its exact typed
//! backing remains private. Canonical snapshots are the only universal value
//! representation exposed by this module.

use crate::{
    CardinalitySpec, DimensionExpr, FloatWidth, FunctionMatrixElement,
    FunctionMatrixRepresentation, FunctionMatrixStoragePattern, FunctionRuntimeType,
    FunctionValueRepresentation, IntegerWidth, MResult, MechError, MechErrorKind, Ref,
    ResolvedType, SchemaBody, SchemaId, SchemaKey, SchemaTable, ShapeInstance, SnapshotValueError,
    TypeConstraintFailure, TypeResolutionError, Value, ValueData, ValueDataDraft, ValueDraft,
};
use core::{any::Any, any::type_name, cell, fmt};

#[cfg(feature = "matrix")]
use crate::snapshot::SequenceView;
use crate::snapshot::SnapshotValidationContext;

#[cfg(all(feature = "no_std", feature = "string"))]
use alloc::string::ToString;
#[cfg(feature = "no_std")]
use alloc::{boxed::Box, rc::Rc, string::String, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, rc::Rc, string::String, vec::Vec};

/// Stable logical identity of a canonical mutable cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CanonicalCellId(u64);

impl CanonicalCellId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

mod canonical_cell_sealed {
    use super::*;

    pub trait Sealed: FunctionRuntimeType + Clone + 'static {
        fn snapshot_bound(
            &self,
            schema: SchemaId,
            shape: &ShapeInstance,
            schemas: &SchemaTable,
        ) -> MResult<Value>;

        fn replace_bound(&mut self, value: &Value) -> MResult<()>;

        fn representation(schema: &SchemaBody) -> FunctionValueRepresentation {
            let _ = schema;
            Self::REPRESENTATION
        }

        fn matrix_extents(&self) -> Option<(usize, usize)> {
            None
        }
    }
}

/// An exact typed backing that can safely live behind a canonical value cell.
///
/// This trait is sealed. Universal values and aggregate containers cannot be
/// used as exact cell backings.
pub trait CanonicalCellBacking:
    canonical_cell_sealed::Sealed + FunctionRuntimeType + Clone + 'static
{
}

impl<T> CanonicalCellBacking for T where
    T: canonical_cell_sealed::Sealed + FunctionRuntimeType + Clone + 'static
{
}

#[derive(Clone)]
pub(crate) struct CellBinding {
    pub(crate) identity: CanonicalCellId,
    pub(crate) schema: SchemaId,
    pub(crate) schema_key: SchemaKey,
    pub(crate) shape: Rc<cell::RefCell<ShapeInstance>>,
    pub(crate) schemas: Rc<SchemaTable>,
    pub(crate) storage: Rc<dyn ErasedCellStorage>,
    /// Planning-time topology for composites assembled from live canonical
    /// cells. The canonical snapshot remains the runtime value authority.
    pub(crate) compiler_children: Option<Rc<[ValueCell]>>,
}

pub(crate) trait ErasedCellStorage {
    fn as_any(&self) -> &dyn Any;
    fn representation(&self, schema: &SchemaBody) -> FunctionValueRepresentation;
    fn snapshot(
        &self,
        schema: SchemaId,
        shape: &ShapeInstance,
        schemas: &SchemaTable,
    ) -> MResult<Value>;
    fn replace(&self, value: &Value) -> MResult<()>;
    fn preflight_replace(&self) -> MResult<()>;
    fn capabilities(&self) -> crate::StorageCapabilityDescriptor;
    fn detached_clone(&self) -> MResult<DetachedCellStorage>;
    fn same_storage(&self, other: &dyn ErasedCellStorage) -> bool;
    fn borrow_state(&self) -> CellBorrowState;
}

pub(crate) struct DetachedCellStorage {
    pub identity: CanonicalCellId,
    pub storage: Rc<dyn ErasedCellStorage>,
}

struct ExactCellStorage<T> {
    reference: Ref<T>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum CellBorrowState {
    Available,
    Borrowed,
}

impl<T: CanonicalCellBacking> ErasedCellStorage for ExactCellStorage<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn representation(&self, schema: &SchemaBody) -> FunctionValueRepresentation {
        T::representation(schema)
    }

    fn snapshot(
        &self,
        schema: SchemaId,
        shape: &ShapeInstance,
        schemas: &SchemaTable,
    ) -> MResult<Value> {
        self.reference
            .try_borrow()
            .map_err(|_| borrow_conflict(CellAccess::Snapshot))?
            .snapshot_bound(schema, shape, schemas)
    }

    fn replace(&self, value: &Value) -> MResult<()> {
        self.reference
            .try_borrow_mut()
            .map_err(|_| borrow_conflict(CellAccess::Replace))?
            .replace_bound(value)
    }

    fn preflight_replace(&self) -> MResult<()> {
        self.reference
            .try_borrow_mut()
            .map(|_| ())
            .map_err(|_| borrow_conflict(CellAccess::Replace))
    }

    fn capabilities(&self) -> crate::StorageCapabilityDescriptor {
        crate::runtime_storage::actual_backing_capabilities(T::REPRESENTATION)
    }

    fn detached_clone(&self) -> MResult<DetachedCellStorage> {
        let value = self
            .reference
            .try_borrow()
            .map_err(|_| borrow_conflict(CellAccess::Snapshot))?
            .clone();
        let reference = Ref::new(value);
        let identity = reference.reactive_cell_id();
        Ok(DetachedCellStorage {
            identity,
            storage: Rc::new(Self { reference }),
        })
    }

    fn same_storage(&self, other: &dyn ErasedCellStorage) -> bool {
        other
            .as_any()
            .downcast_ref::<Self>()
            .is_some_and(|other| self.reference.same_handle(&other.reference))
    }

    fn borrow_state(&self) -> CellBorrowState {
        if self.reference.try_borrow().is_ok() {
            CellBorrowState::Available
        } else {
            CellBorrowState::Borrowed
        }
    }
}

/// An opaque, schema-aware mutable program location.
///
/// Exact backing extraction is intentionally crate-private:
///
/// ```compile_fail
/// use mech_core::ValueCell;
///
/// fn expose(cell: &ValueCell) {
///     let _ = cell.try_ref::<f64>();
/// }
/// ```
#[derive(Clone)]
pub struct ValueCell {
    pub(crate) binding: CellBinding,
}

impl ValueCell {
    /// Resolves this cell through its own canonical schema and current shape.
    /// The pure type system never imports physical cell or storage types.
    pub fn resolved_type(&self) -> MResult<ResolvedType> {
        let schemas = self.schema_table();
        let schema = schemas
            .find_by_key(self.schema_key())
            .and_then(|id| schemas.get(id))
            .ok_or_else(|| {
                MechError::from(TypeResolutionError::incompatible(
                    "source expression",
                    TypeConstraintFailure::InvalidScheme {
                        reason: "the cell schema is absent from its canonical schema table".into(),
                    },
                ))
            })?;
        ResolvedType::from_schema(schema, &self.shape()).map_err(MechError::from)
    }

    /// Constructs the canonical empty-tuple value used as the output of an
    /// effect that does not otherwise return a value.
    pub fn unit() -> Self {
        Self::from_inferred_value_data(
            SchemaBody::Tuple(Vec::new().into_boxed_slice()),
            ValueDataDraft::Tuple(Vec::new().into_boxed_slice()),
        )
        .expect("the canonical unit schema and value are valid")
    }

    /// Constructs a standalone canonical cell from an exact scalar backing.
    pub fn from_exact<T>(value: T) -> MResult<Self>
    where
        T: CanonicalCellBacking,
    {
        let matrix_extents = canonical_cell_sealed::Sealed::matrix_extents(&value);
        Self::from_inferred_ref(Ref::new(value), matrix_extents)
    }

    /// Constructs a standalone canonical matrix cell from an exact backing.
    ///
    /// The supplied logical extents are validated when the first snapshot is
    /// captured; the backing remains private behind the cell binding.
    #[cfg(feature = "matrix")]
    pub fn from_exact_matrix_ref<T>(reference: Ref<T>, rows: usize, columns: usize) -> MResult<Self>
    where
        T: CanonicalCellBacking,
    {
        Self::from_inferred_ref(reference, Some((rows, columns)))
    }

    /// Constructs a fresh exact backing for a declared runtime output.
    ///
    /// Source specialization uses the runtime factory signature as the
    /// authority for storage representation while the operation supplies the
    /// resolved logical matrix dimensions. No erased universal value or
    /// mutable universal handle is involved in output construction.
    pub fn default_for_representation(
        representation: FunctionValueRepresentation,
        _matrix_dimensions: Option<(usize, usize)>,
    ) -> MResult<Self> {
        macro_rules! scalar {
            ($value:expr) => {
                return Self::from_exact($value)
            };
        }
        match representation {
            #[cfg(feature = "u8")]
            FunctionValueRepresentation::U8 => scalar!(0_u8),
            #[cfg(feature = "u16")]
            FunctionValueRepresentation::U16 => scalar!(0_u16),
            #[cfg(feature = "u32")]
            FunctionValueRepresentation::U32 => scalar!(0_u32),
            #[cfg(feature = "u64")]
            FunctionValueRepresentation::U64 => scalar!(0_u64),
            #[cfg(feature = "u128")]
            FunctionValueRepresentation::U128 => scalar!(0_u128),
            #[cfg(feature = "i8")]
            FunctionValueRepresentation::I8 => scalar!(0_i8),
            #[cfg(feature = "i16")]
            FunctionValueRepresentation::I16 => scalar!(0_i16),
            #[cfg(feature = "i32")]
            FunctionValueRepresentation::I32 => scalar!(0_i32),
            #[cfg(feature = "i64")]
            FunctionValueRepresentation::I64 => scalar!(0_i64),
            #[cfg(feature = "i128")]
            FunctionValueRepresentation::I128 => scalar!(0_i128),
            #[cfg(feature = "f32")]
            FunctionValueRepresentation::F32 => scalar!(0.0_f32),
            #[cfg(feature = "f64")]
            FunctionValueRepresentation::F64 => scalar!(0.0_f64),
            #[cfg(feature = "complex")]
            FunctionValueRepresentation::C64 => scalar!(crate::C64::new(0.0, 0.0)),
            #[cfg(feature = "rational")]
            FunctionValueRepresentation::R64 => scalar!(crate::R64::new(0, 1)),
            #[cfg(feature = "bool")]
            FunctionValueRepresentation::Bool => scalar!(false),
            #[cfg(feature = "string")]
            FunctionValueRepresentation::String => scalar!(String::new()),
            FunctionValueRepresentation::Index => scalar!(1_usize),
            #[cfg(feature = "matrix")]
            FunctionValueRepresentation::Matrix { element, storage } => {
                let dimensions = _matrix_dimensions.ok_or_else(|| {
                    MechError::new(
                        ValueCellOutputConstructionUnsupported {
                            representation,
                            reason: "matrix dimensions were not supplied".into(),
                        },
                        None,
                    )
                    .with_compiler_loc()
                })?;
                macro_rules! matrix_element {
                    ($type:ty, $default:expr) => {
                        return default_matrix_cell::<$type>(storage, dimensions, $default)
                    };
                }
                match element {
                    FunctionMatrixElement::Index => matrix_element!(usize, 1_usize),
                    #[cfg(feature = "bool")]
                    FunctionMatrixElement::Bool => matrix_element!(bool, false),
                    #[cfg(feature = "string")]
                    FunctionMatrixElement::String => matrix_element!(String, String::new()),
                    #[cfg(feature = "u8")]
                    FunctionMatrixElement::U8 => matrix_element!(u8, 0_u8),
                    #[cfg(feature = "u16")]
                    FunctionMatrixElement::U16 => matrix_element!(u16, 0_u16),
                    #[cfg(feature = "u32")]
                    FunctionMatrixElement::U32 => matrix_element!(u32, 0_u32),
                    #[cfg(feature = "u64")]
                    FunctionMatrixElement::U64 => matrix_element!(u64, 0_u64),
                    #[cfg(feature = "u128")]
                    FunctionMatrixElement::U128 => matrix_element!(u128, 0_u128),
                    #[cfg(feature = "i8")]
                    FunctionMatrixElement::I8 => matrix_element!(i8, 0_i8),
                    #[cfg(feature = "i16")]
                    FunctionMatrixElement::I16 => matrix_element!(i16, 0_i16),
                    #[cfg(feature = "i32")]
                    FunctionMatrixElement::I32 => matrix_element!(i32, 0_i32),
                    #[cfg(feature = "i64")]
                    FunctionMatrixElement::I64 => matrix_element!(i64, 0_i64),
                    #[cfg(feature = "i128")]
                    FunctionMatrixElement::I128 => matrix_element!(i128, 0_i128),
                    #[cfg(feature = "f32")]
                    FunctionMatrixElement::F32 => matrix_element!(f32, 0.0_f32),
                    #[cfg(feature = "f64")]
                    FunctionMatrixElement::F64 => matrix_element!(f64, 0.0_f64),
                    #[cfg(feature = "complex")]
                    FunctionMatrixElement::C64 => {
                        matrix_element!(crate::C64, crate::C64::new(0.0, 0.0))
                    }
                    #[cfg(feature = "rational")]
                    FunctionMatrixElement::R64 => {
                        matrix_element!(crate::R64, crate::R64::new(0, 1))
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        Err(MechError::new(
            ValueCellOutputConstructionUnsupported {
                representation,
                reason: "no exact canonical backing exists for this representation".into(),
            },
            None,
        )
        .with_compiler_loc())
    }

    pub fn from_ref<T>(
        reference: Ref<T>,
        schema: SchemaId,
        shape: ShapeInstance,
        schemas: Rc<SchemaTable>,
    ) -> MResult<Self>
    where
        T: CanonicalCellBacking,
    {
        let schema_key = schemas
            .entry(schema)
            .map(|entry| entry.key())
            .ok_or_else(|| {
                snapshot_failure(SnapshotValueError::UnknownSnapshotSchema { schema })
            })?;
        let identity = reference.reactive_cell_id();
        let cell = Self {
            binding: CellBinding {
                identity,
                schema,
                schema_key,
                shape: Rc::new(cell::RefCell::new(shape)),
                schemas,
                storage: Rc::new(ExactCellStorage { reference }),
                compiler_children: None,
            },
        };
        cell.snapshot()?;
        Ok(cell)
    }

    pub fn from_value(value: Value, schemas: Rc<SchemaTable>) -> MResult<Self> {
        let value = rebind_value(value, schemas.as_ref())?;
        Ok(Self::from_bound_value(value, schemas))
    }

    fn from_bound_value(value: Value, schemas: Rc<SchemaTable>) -> Self {
        let schema = value.schema();
        let schema_key = value.schema_key();
        let shape = value.shape().clone();
        debug_assert_eq!(
            schemas.entry(schema).map(|entry| entry.key()),
            Some(schema_key),
            "canonical value must retain its originating schema table"
        );
        let reference = Ref::new(value);
        let identity = reference.reactive_cell_id();
        Self {
            binding: CellBinding {
                identity,
                schema,
                schema_key,
                shape: Rc::new(cell::RefCell::new(shape)),
                schemas,
                storage: Rc::new(ExactCellStorage { reference }),
                compiler_children: None,
            },
        }
    }

    /// Creates a mutable cell from a detached canonical value and the schema
    /// context retained by that value.
    pub fn from_snapshot(value: Value) -> MResult<Self> {
        let schemas = value.schemas().ok_or_else(|| {
            MechError::new(ValueSchemaContextUnavailable, None).with_compiler_loc()
        })?;
        value.validate_against(&schemas).map_err(snapshot_failure)?;
        Self::from_runtime_value(value, Rc::new((*schemas).clone()))
    }

    /// Constructs an empty standalone set whose element schema is closed and
    /// whose current cardinality may change without changing schema identity.
    pub fn empty_dynamic_set(element: SchemaBody) -> MResult<Self> {
        Self::from_inferred_value_data(
            SchemaBody::Set {
                element: Box::new(element),
                cardinality: CardinalitySpec::Dynamic { upper_bound: None },
            },
            ValueDataDraft::Set(Vec::new().into_boxed_slice()),
        )
    }

    /// Constructs an empty table whose row extent can vary without replacing
    /// the table cell.
    pub fn empty_dynamic_table(columns: Box<[crate::SchemaField]>) -> MResult<Self> {
        let data = columns
            .iter()
            .map(|column| crate::snapshot::TableColumnDraft {
                name: column.name.clone(),
                values: Box::new([]),
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        Self::from_inferred_value_data(
            SchemaBody::Table {
                columns,
                rows: CardinalitySpec::Dynamic { upper_bound: None },
            },
            ValueDataDraft::Table(data),
        )
    }

    /// Constructs an empty map whose entry extent can vary without replacing
    /// the map cell.
    pub fn empty_dynamic_map(key: SchemaBody, value: SchemaBody) -> MResult<Self> {
        Self::from_inferred_value_data(
            SchemaBody::Map {
                key: Box::new(key),
                value: Box::new(value),
                cardinality: CardinalitySpec::Dynamic { upper_bound: None },
            },
            ValueDataDraft::Map(Box::new([])),
        )
    }

    /// Constructs a dense matrix whose dimensions are turn-scoped schema
    /// parameters. Subsequent replacements may change those dimensions while
    /// retaining the same schema and cell identity.
    pub fn dynamic_matrix(
        element: SchemaBody,
        dimensions: Box<[u64]>,
        elements: Box<[ValueDataDraft]>,
    ) -> MResult<Self> {
        let (schema, shape, schemas) = dynamic_matrix_schema(element, dimensions)?;
        let value = finalize_draft(
            schema,
            &shape,
            schemas.as_ref(),
            ValueDataDraft::Matrix(elements),
        )?;
        Self::from_runtime_value(value, schemas)
    }

    /// Constructs a turn-varying rank-two matrix with a resizable matrix
    /// backing even when its initial shape happens to be a row or column.
    /// Dynamic kernel outputs use this when either axis may change later.
    #[doc(hidden)]
    pub fn dynamic_rank_matrix(
        element: SchemaBody,
        dimensions: Box<[u64]>,
        elements: Box<[ValueDataDraft]>,
    ) -> MResult<Self> {
        let (schema, shape, schemas) = dynamic_matrix_schema(element, dimensions)?;
        let value = finalize_draft(
            schema,
            &shape,
            schemas.as_ref(),
            ValueDataDraft::Matrix(elements),
        )?;
        #[cfg(all(feature = "matrix", feature = "matrixd"))]
        if let ValueData::Matrix(matrix) = value.data()
            && let Some(cell) =
                dynamic_matrix_cell(matrix.elements(), schema, &shape, schemas.clone(), true)?
        {
            return Ok(cell);
        }
        Ok(Self::from_bound_value(value, schemas))
    }

    /// Constructs a standalone canonical cell from a closed schema body and
    /// matching canonical data draft.
    pub fn from_schema_data(body: SchemaBody, data: ValueDataDraft) -> MResult<Self> {
        Self::from_inferred_value_data(body, data)
    }

    /// Constructs a detached heterogeneous tuple from canonical child cells.
    /// Child schemas are closed before embedding so no table-local schema id
    /// escapes into the new tuple's schema arena.
    pub fn tuple_from_cells(cells: &[Self]) -> MResult<Self> {
        let elements = cells
            .iter()
            .map(Self::closed_schema_body)
            .collect::<MResult<Vec<_>>>()?;
        let body = SchemaBody::Tuple(elements.clone().into_boxed_slice());
        let (schema, shape, schemas) = merged_schema(body, cells.iter())?;
        let values = cells
            .iter()
            .zip(&elements)
            .map(|(cell, expected)| {
                canonical_cell_draft_for_schema(cell, expected, schemas.as_ref())
            })
            .collect::<MResult<Vec<_>>>()?;
        let value = finalize_draft(
            schema,
            &shape,
            schemas.as_ref(),
            ValueDataDraft::Tuple(values.into_boxed_slice()),
        )?;
        let mut tuple = Self::from_runtime_value(value, schemas)?;
        tuple.binding.compiler_children = Some(cells.to_vec().into());
        Ok(tuple)
    }

    /// Rebuilds this tuple from current child-cell values in its existing
    /// merged schema arena.
    pub fn rebuild_tuple_cells(&self, cells: &[Self]) -> MResult<Value> {
        let SchemaBody::Tuple(elements) = self.closed_schema_body()? else {
            return Err(aggregate_rebuild_unsupported(self, "tuple"));
        };
        if elements.len() != cells.len() {
            return Err(aggregate_rebuild_arity(
                self,
                "tuple",
                elements.len(),
                cells.len(),
            ));
        }
        let values = cells
            .iter()
            .zip(&elements)
            .map(|(cell, expected)| {
                canonical_cell_draft_for_schema(cell, expected, self.binding.schemas.as_ref())
            })
            .collect::<MResult<Vec<_>>>()?;
        self.rebuild_data_draft(ValueDataDraft::Tuple(values.into_boxed_slice()))
    }

    /// Constructs a canonical record from named child cells in one schema
    /// arena, including concrete schemas retained below dynamic children.
    pub fn record_from_cells(fields: &[(String, Self)]) -> MResult<Self> {
        let schema_fields = fields
            .iter()
            .map(|(name, cell)| {
                Ok(crate::SchemaField {
                    name: name.clone(),
                    schema: cell.closed_schema_body()?,
                })
            })
            .collect::<MResult<Vec<_>>>()?;
        let body = SchemaBody::Record(schema_fields.clone().into_boxed_slice());
        let (schema, shape, schemas) = merged_schema(body, fields.iter().map(|(_, cell)| cell))?;
        let data = record_cell_fields_draft(fields, &schema_fields, schemas.as_ref())?;
        let value = finalize_draft(schema, &shape, schemas.as_ref(), data)?;
        Self::from_runtime_value(value, schemas)
    }

    /// Rebuilds this record from current named child-cell values in its
    /// existing merged schema arena.
    pub fn rebuild_record_cells(&self, fields: &[(String, Self)]) -> MResult<Value> {
        let SchemaBody::Record(schema_fields) = self.closed_schema_body()? else {
            return Err(aggregate_rebuild_unsupported(self, "record"));
        };
        if schema_fields.len() != fields.len() {
            return Err(aggregate_rebuild_arity(
                self,
                "record",
                schema_fields.len(),
                fields.len(),
            ));
        }
        let data = record_cell_fields_draft(fields, &schema_fields, self.binding.schemas.as_ref())?;
        self.rebuild_data_draft(data)
    }

    /// Constructs a canonical table from source cells while retaining every
    /// concrete schema needed by dynamic (`*`) columns in one schema arena.
    pub fn table_from_cell_columns(
        columns: Box<[(crate::SchemaField, Box<[Self]>)]>,
        rows: CardinalitySpec,
    ) -> MResult<Self> {
        let body = SchemaBody::Table {
            columns: columns
                .iter()
                .map(|(field, _)| field.clone())
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            rows,
        };
        let cells = columns
            .iter()
            .flat_map(|(_, values)| values.iter())
            .collect::<Vec<_>>();
        let (schema, shape, schemas) = merged_schema(body, cells.iter().copied())?;
        let data = table_cell_columns_draft(&columns, schemas.as_ref())?;
        let value = finalize_draft(schema, &shape, schemas.as_ref(), data)?;
        Self::from_runtime_value(value, schemas)
    }

    /// Rebuilds this table with current source-cell values using its existing
    /// merged arena. This preserves both table identity and dynamic children.
    pub fn rebuild_table_cell_columns(&self, columns: &[(String, Box<[Self]>)]) -> MResult<Value> {
        let SchemaBody::Table {
            columns: schema_columns,
            ..
        } = self.closed_schema_body()?
        else {
            return Err(MechError::new(
                ValueCellOutputConstructionUnsupported {
                    representation: self.representation(),
                    reason: "table-cell reconstruction requires a canonical table".into(),
                },
                None,
            )
            .with_compiler_loc());
        };
        if schema_columns.len() != columns.len() {
            return Err(MechError::new(
                ValueCellOutputConstructionUnsupported {
                    representation: self.representation(),
                    reason: format!(
                        "table schema has {} columns but {} columns were supplied",
                        schema_columns.len(),
                        columns.len(),
                    ),
                },
                None,
            )
            .with_compiler_loc());
        }
        let columns = schema_columns
            .into_vec()
            .into_iter()
            .zip(columns)
            .map(|(field, (name, values))| {
                if field.name != *name {
                    return Err(MechError::new(
                        ValueCellOutputConstructionUnsupported {
                            representation: self.representation(),
                            reason: format!(
                                "table schema column {} does not match supplied column {name}",
                                field.name,
                            ),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
                Ok((field, values.clone()))
            })
            .collect::<MResult<Vec<_>>>()?;
        let data = table_cell_columns_draft(&columns, self.binding.schemas.as_ref())?;
        self.rebuild_data_draft(data)
    }

    /// Constructs a row-major dynamic matrix from homogeneous canonical child
    /// cells. The output keeps one cell identity while later turns may change
    /// its dimensions through [`ValueCell::replace`].
    pub fn dynamic_matrix_from_cells(rows: usize, columns: usize, cells: &[Self]) -> MResult<Self> {
        if rows.saturating_mul(columns) != cells.len() {
            return Err(MechError::new(
                ValueCellOutputConstructionUnsupported {
                    representation: FunctionValueRepresentation::AnyValue,
                    reason: format!(
                        "matrix dimensions require {} elements but {} were supplied",
                        rows.saturating_mul(columns),
                        cells.len()
                    ),
                },
                None,
            )
            .with_compiler_loc());
        }
        let Some(first) = cells.first() else {
            return Err(MechError::new(
                ValueCellOutputConstructionUnsupported {
                    representation: FunctionValueRepresentation::AnyValue,
                    reason: "an empty matrix requires an explicit element schema".into(),
                },
                None,
            )
            .with_compiler_loc());
        };
        let element = first.closed_schema_body()?;
        let mut values = Vec::with_capacity(cells.len());
        for cell in cells {
            let candidate = cell.closed_schema_body()?;
            if candidate != element {
                return Err(MechError::new(
                    ValueCellOutputConstructionUnsupported {
                        representation: cell.representation(),
                        reason: "matrix elements must share one canonical schema".into(),
                    },
                    None,
                )
                .with_compiler_loc());
            }
            values.push(canonical_cell_draft(cell)?);
        }
        Self::dynamic_matrix(
            element,
            vec![rows as u64, columns as u64].into_boxed_slice(),
            values.into_boxed_slice(),
        )
    }

    /// Returns this cell's schema with every shape parameter resolved to its
    /// current concrete extent. The returned body is safe to embed in a
    /// standalone derived-output schema.
    pub fn closed_schema_body(&self) -> MResult<SchemaBody> {
        let schema = self
            .binding
            .schemas
            .get(self.binding.schema)
            .expect("value-cell schema remains present");
        close_schema_body(schema.body(), &self.binding.shape.borrow())
    }

    /// Returns detached canonical tuple element cells, or `None` when this is
    /// not a tuple. Child schemas are closed against the parent's current
    /// shape before the cells are constructed.
    pub fn tuple_elements(&self) -> MResult<Option<Vec<Self>>> {
        let SchemaBody::Tuple(schemas) = self.closed_schema_body()? else {
            return Ok(None);
        };
        let draft = self
            .snapshot()?
            .canonical_data_draft()
            .map_err(snapshot_failure)?;
        let ValueDataDraft::Tuple(values) = draft else {
            unreachable!("validated tuple schema retains tuple data")
        };
        child_cells(schemas.into_vec(), values.into_vec()).map(Some)
    }

    /// Returns tuple child cells while retaining identities captured during
    /// canonical tuple assembly. Source destructuring uses this narrow path
    /// to keep reactive topology; ordinary value inspection remains detached.
    #[doc(hidden)]
    pub fn reactive_tuple_elements(&self) -> MResult<Option<Vec<Self>>> {
        let SchemaBody::Tuple(_) = self.closed_schema_body()? else {
            return Ok(None);
        };
        match &self.binding.compiler_children {
            Some(children) => Ok(Some(children.to_vec())),
            None => self.tuple_elements(),
        }
    }

    #[cfg(feature = "semantic-compiler")]
    pub(crate) fn compiler_composite_children(&self) -> Option<&[Self]> {
        self.binding.compiler_children.as_deref()
    }

    /// Returns detached canonical matrix element cells in row-major order, or
    /// `None` when this is not a matrix.
    pub fn matrix_elements(&self) -> MResult<Option<Vec<Self>>> {
        let SchemaBody::Matrix { element, .. } = self.closed_schema_body()? else {
            return Ok(None);
        };
        let draft = self
            .snapshot()?
            .canonical_data_draft()
            .map_err(snapshot_failure)?;
        let ValueDataDraft::Matrix(values) = draft else {
            unreachable!("validated matrix schema retains matrix data")
        };
        values
            .into_vec()
            .into_iter()
            .map(|value| Self::from_schema_data((*element).clone(), value))
            .collect::<MResult<Vec<_>>>()
            .map(Some)
    }

    /// Returns detached canonical set element cells in canonical key order,
    /// or `None` when this is not a set.
    pub fn set_element_cells(&self) -> MResult<Option<Vec<Self>>> {
        let SchemaBody::Set { element, .. } = self.closed_schema_body()? else {
            return Ok(None);
        };
        let draft = self
            .snapshot()?
            .canonical_data_draft()
            .map_err(snapshot_failure)?;
        let ValueDataDraft::Set(values) = draft else {
            unreachable!("validated set schema retains set data")
        };
        values
            .into_vec()
            .into_iter()
            .map(|value| Self::from_schema_data((*element).clone(), value))
            .collect::<MResult<Vec<_>>>()
            .map(Some)
    }

    /// Reconstructs the private exact backing used by typed function ports
    /// while retaining canonical schema and shape metadata. Aggregate values
    /// remain backed by immutable [`Value`] data.
    pub(crate) fn from_runtime_value(value: Value, schemas: Rc<SchemaTable>) -> MResult<Self> {
        let value = rebind_value(value, schemas.as_ref())?;
        let schema = value.schema();
        let shape = value.shape().clone();
        macro_rules! scalar {
            ($value:expr) => {
                return Self::from_ref(Ref::new($value), schema, shape, schemas)
            };
        }
        match value.data() {
            #[cfg(feature = "u8")]
            ValueData::U8(value) => scalar!(*value),
            #[cfg(feature = "u16")]
            ValueData::U16(value) => scalar!(*value),
            #[cfg(feature = "u32")]
            ValueData::U32(value) => scalar!(*value),
            #[cfg(feature = "u64")]
            ValueData::U64(value) => scalar!(*value),
            #[cfg(feature = "u128")]
            ValueData::U128(value) => scalar!(*value),
            #[cfg(feature = "i8")]
            ValueData::I8(value) => scalar!(*value),
            #[cfg(feature = "i16")]
            ValueData::I16(value) => scalar!(*value),
            #[cfg(feature = "i32")]
            ValueData::I32(value) => scalar!(*value),
            #[cfg(feature = "i64")]
            ValueData::I64(value) => scalar!(*value),
            #[cfg(feature = "i128")]
            ValueData::I128(value) => scalar!(*value),
            #[cfg(feature = "f32")]
            ValueData::F32(value) => scalar!(value.to_f32()),
            #[cfg(feature = "f64")]
            ValueData::F64(value) => scalar!(value.to_f64()),
            #[cfg(feature = "complex")]
            ValueData::Complex64(value) => scalar!(crate::C64::new(
                value.real().to_f64(),
                value.imaginary().to_f64(),
            )),
            #[cfg(feature = "rational")]
            ValueData::Rational64(value) => {
                if let Ok(denominator) = i64::try_from(value.denominator()) {
                    scalar!(crate::R64::new(value.numerator(), denominator));
                }
            }
            #[cfg(feature = "bool")]
            ValueData::Bool(value) => scalar!(*value),
            #[cfg(feature = "string")]
            ValueData::String(value) => scalar!(value.to_string()),
            ValueData::Index(value) => {
                if let Ok(value) = usize::try_from(*value) {
                    scalar!(value);
                }
            }
            #[cfg(all(feature = "matrix", feature = "matrixd"))]
            ValueData::Matrix(matrix) => {
                if let Some(cell) =
                    dynamic_matrix_cell(matrix.elements(), schema, &shape, schemas.clone(), false)?
                {
                    return Ok(cell);
                }
            }
            _ => {}
        }
        Ok(Self::from_bound_value(value, schemas))
    }

    pub(crate) fn from_inferred_ref<T>(
        reference: Ref<T>,
        matrix_extents: Option<(usize, usize)>,
    ) -> MResult<Self>
    where
        T: CanonicalCellBacking,
    {
        if let (
            FunctionValueRepresentation::Matrix {
                element,
                storage:
                    FunctionMatrixStoragePattern::Exact(
                        FunctionMatrixRepresentation::RowVectorD
                        | FunctionMatrixRepresentation::VectorD
                        | FunctionMatrixRepresentation::MatrixD,
                    ),
            },
            Some((rows, columns)),
        ) = (T::REPRESENTATION, matrix_extents)
        {
            let element = schema_body_for_matrix_element(element)
                .ok_or_else(|| backing_mismatch::<T>(T::REPRESENTATION))?;
            let (schema, shape, schemas) = dynamic_matrix_schema(
                element,
                vec![rows as u64, columns as u64].into_boxed_slice(),
            )?;
            return Self::from_ref(reference, schema, shape, schemas);
        }
        let body = schema_body_for_representation(T::REPRESENTATION, matrix_extents)
            .ok_or_else(|| backing_mismatch::<T>(T::REPRESENTATION))?;
        let (schema, shape, schemas) = standalone_schema(body)?;
        Self::from_ref(reference, schema, shape, schemas)
    }

    pub(crate) fn from_inferred_value_data(
        body: SchemaBody,
        data: ValueDataDraft,
    ) -> MResult<Self> {
        let (schema, shape, schemas) = standalone_schema(body)?;
        let value = finalize_draft(schema, &shape, &schemas, data)?;
        Self::from_runtime_value(value, schemas)
    }

    pub const fn schema(&self) -> SchemaId {
        self.binding.schema
    }

    pub const fn schema_key(&self) -> SchemaKey {
        self.binding.schema_key
    }

    pub fn shape(&self) -> cell::Ref<'_, ShapeInstance> {
        self.binding.shape.borrow()
    }

    pub(crate) fn schema_table(&self) -> Rc<SchemaTable> {
        self.binding.schemas.clone()
    }

    pub fn representation(&self) -> FunctionValueRepresentation {
        let schema = self
            .binding
            .schemas
            .get(self.binding.schema)
            .expect("value-cell schema remains present");
        self.binding.storage.representation(schema.body())
    }

    pub fn type_memory_contract(&self) -> MResult<crate::TypeMemoryContract> {
        let schema = self
            .binding
            .schemas
            .get(self.binding.schema)
            .ok_or_else(|| {
                snapshot_failure(SnapshotValueError::UnknownSnapshotSchema {
                    schema: self.binding.schema,
                })
            })?;
        Ok(schema.type_memory_contract()?)
    }

    pub fn resolved_type_memory_contract(&self) -> MResult<crate::ResolvedTypeMemoryContract> {
        let schema = self
            .binding
            .schemas
            .get(self.binding.schema)
            .ok_or_else(|| {
                snapshot_failure(SnapshotValueError::UnknownSnapshotSchema {
                    schema: self.binding.schema,
                })
            })?;
        let shape = self
            .binding
            .shape
            .try_borrow()
            .map_err(|_| borrow_conflict(CellAccess::Snapshot))?
            .clone();
        Ok(schema.resolved_type_memory_contract(&shape)?)
    }

    pub fn storage_capabilities(&self) -> crate::StorageCapabilityDescriptor {
        self.binding.storage.capabilities()
    }

    /// Performs the opt-in shadow check without changing construction or execution.
    pub fn validate_storage_contract(&self) -> MResult<()> {
        let schema = self
            .binding
            .schemas
            .get(self.binding.schema)
            .ok_or_else(|| {
                snapshot_failure(SnapshotValueError::UnknownSnapshotSchema {
                    schema: self.binding.schema,
                })
            })?;
        let shape = self
            .binding
            .shape
            .try_borrow()
            .map_err(|_| borrow_conflict(CellAccess::Snapshot))?
            .clone();
        crate::check_schema_storage_compatibility(
            schema,
            &shape,
            &self.binding.storage.capabilities(),
        )
        .map_err(|error| match error {
            crate::SchemaStorageCompatibilityError::Semantic(error) => error.into(),
            crate::SchemaStorageCompatibilityError::Storage(reason) => MechError::new(
                ValueCellStorageContractViolation {
                    schema: self.binding.schema_key,
                    reason,
                },
                None,
            )
            .with_compiler_loc(),
        })
    }

    /// Describes whether this cell's schema permits its resolved extents to
    /// change while the cell identity remains stable.
    pub fn extent_evolution(&self) -> crate::ExtentEvolution {
        self.binding
            .schemas
            .get(self.binding.schema)
            .expect("value-cell schema remains present")
            .extent_evolution()
    }

    pub fn snapshot(&self) -> MResult<Value> {
        let shape = self.binding.shape.borrow().clone();
        self.binding
            .storage
            .snapshot(self.binding.schema, &shape, &self.binding.schemas)
    }

    /// Clones this cell's exact backing into a new, independent mutable cell.
    ///
    /// Schema, shape, and storage representation are retained while physical
    /// cell identity is deliberately fresh. Source specialization uses this
    /// for full-write outputs whose representation mirrors an input.
    pub fn detached_clone(&self) -> MResult<Self> {
        let detached = self.binding.storage.detached_clone()?;
        let shape = self
            .binding
            .shape
            .try_borrow()
            .map_err(|_| borrow_conflict(CellAccess::Snapshot))?
            .clone();
        Ok(Self {
            binding: CellBinding {
                identity: detached.identity,
                schema: self.binding.schema,
                schema_key: self.binding.schema_key,
                shape: Rc::new(cell::RefCell::new(shape)),
                schemas: self.binding.schemas.clone(),
                storage: detached.storage,
                compiler_children: None,
            },
        })
    }

    pub fn replace(&self, value: &Value) -> MResult<()> {
        if value.schema_key() != self.binding.schema_key {
            return Err(MechError::new(
                ValueCellSchemaMismatch {
                    expected: self.binding.schema_key,
                    actual: value.schema_key(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let value = rebind_value(value.clone(), self.binding.schemas.as_ref())?;
        debug_assert_eq!(value.schema(), self.binding.schema);
        let mut shape = self
            .binding
            .shape
            .try_borrow_mut()
            .map_err(|_| borrow_conflict(CellAccess::Replace))?;
        let current_shape = shape.clone();
        let schema = self
            .binding
            .schemas
            .get(self.binding.schema)
            .expect("value-cell schema remains present");
        if !shape_change_allowed(schema, &current_shape, value.shape()) {
            return Err(MechError::new(
                ValueCellShapeMismatch {
                    expected: current_shape.parameter_values().to_vec().into_boxed_slice(),
                    actual: value.shape().parameter_values().to_vec().into_boxed_slice(),
                },
                None,
            )
            .with_compiler_loc());
        }
        value
            .validate_against(&self.binding.schemas)
            .map_err(snapshot_failure)?;
        self.binding.storage.replace(&value)?;
        *shape = value.shape().clone();
        Ok(())
    }

    /// Verifies that the cell can be mutably borrowed for a later atomic
    /// replacement without changing its identity.
    pub fn preflight_replace(&self) -> MResult<()> {
        let _shape = self
            .binding
            .shape
            .try_borrow_mut()
            .map_err(|_| borrow_conflict(CellAccess::Replace))?;
        self.binding.storage.preflight_replace()
    }

    pub fn same_logical_cell(&self, other: &Self) -> bool {
        self.binding.identity == other.binding.identity
    }

    pub fn same_storage(&self, other: &Self) -> bool {
        self.binding
            .storage
            .same_storage(other.binding.storage.as_ref())
    }

    /// Compatibility spelling for physical storage identity.
    ///
    /// New code should choose `same_logical_cell` or `same_storage`
    /// explicitly. This method retains its existing physical-storage meaning.
    pub fn same_cell(&self, other: &Self) -> bool {
        self.same_storage(other)
    }

    pub fn reactive_cell_id(&self) -> CanonicalCellId {
        self.binding.identity
    }

    #[cfg(feature = "functions")]
    pub(crate) fn same_exact_ref<T: 'static>(&self, reference: &Ref<T>) -> bool {
        self.binding
            .storage
            .as_any()
            .downcast_ref::<ExactCellStorage<T>>()
            .is_some_and(|storage| storage.reference.same_handle(reference))
    }

    #[cfg(feature = "semantic-compiler")]
    pub(crate) fn compiler_identity(&self) -> usize {
        self.binding.identity.get() as usize
    }

    /// Compares canonical schema, shape, and payload without considering cell
    /// identity or relying on either cell's local schema ids.
    pub fn snapshot_eq(&self, other: &Self) -> MResult<bool> {
        let left = self.snapshot()?;
        let right = other.snapshot()?;
        left.snapshot_eq(
            self.binding.schemas.as_ref(),
            &right,
            other.binding.schemas.as_ref(),
        )
        .map_err(snapshot_failure)
    }

    /// Compares two canonical key values using the schema-directed ordering
    /// rules used by sets and maps.
    pub fn key_eq(&self, other: &Self) -> MResult<bool> {
        let left = self.snapshot()?;
        let right = other.snapshot()?;
        left.key_cmp(
            self.binding.schemas.as_ref(),
            &right,
            other.binding.schemas.as_ref(),
        )
        .map(|ordering| ordering == core::cmp::Ordering::Equal)
        .map_err(snapshot_failure)
    }

    #[cfg(feature = "functions")]
    pub(crate) fn set_contains(&self, candidate: &Self) -> MResult<bool> {
        let set = self.snapshot()?;
        let candidate_value = candidate.snapshot()?;
        set.set_contains(
            self.binding.schemas.as_ref(),
            &candidate_value,
            candidate.binding.schemas.as_ref(),
        )
        .map_err(snapshot_failure)
    }

    #[cfg(feature = "functions")]
    pub(crate) fn set_elements_after_insert(&self, candidate: &Self) -> MResult<Box<[ValueData]>> {
        let set = self.snapshot()?;
        let candidate_value = candidate.snapshot()?;
        set.set_elements_after_insert(
            self.binding.schemas.as_ref(),
            &candidate_value,
            candidate.binding.schemas.as_ref(),
        )
        .map_err(snapshot_failure)
    }

    #[cfg(feature = "functions")]
    pub(crate) fn set_elements(&self) -> MResult<Box<[ValueData]>> {
        let snapshot = self.snapshot()?;
        let Some(set) = snapshot.set_view() else {
            return Err(backing_mismatch::<Value>(self.representation()));
        };
        Ok(set
            .elements()
            .iter()
            .map(|value| value.data().clone())
            .collect::<Vec<_>>()
            .into_boxed_slice())
    }

    #[cfg(feature = "functions")]
    pub(crate) fn set_element_drafts(&self) -> MResult<Box<[ValueDataDraft]>> {
        self.snapshot()?
            .set_element_drafts(self.binding.schemas.as_ref())
            .map_err(snapshot_failure)
    }

    #[cfg(feature = "functions")]
    pub(crate) fn set_elements_after_remove(&self, candidate: &Self) -> MResult<Box<[ValueData]>> {
        let set = self.snapshot()?;
        let candidate_value = candidate.snapshot()?;
        set.set_elements_after_remove(
            self.binding.schemas.as_ref(),
            &candidate_value,
            candidate.binding.schemas.as_ref(),
        )
        .map_err(snapshot_failure)
    }

    #[cfg(feature = "functions")]
    pub(crate) fn set_union_elements(&self, other: &Self) -> MResult<Box<[ValueData]>> {
        self.set_binary_elements(other, Value::set_union_elements)
    }

    #[cfg(feature = "functions")]
    pub(crate) fn set_intersection_elements(&self, other: &Self) -> MResult<Box<[ValueData]>> {
        self.set_binary_elements(other, Value::set_intersection_elements)
    }

    #[cfg(feature = "functions")]
    pub(crate) fn set_difference_elements(&self, other: &Self) -> MResult<Box<[ValueData]>> {
        self.set_binary_elements(other, Value::set_difference_elements)
    }

    #[cfg(feature = "functions")]
    pub(crate) fn set_symmetric_difference_elements(
        &self,
        other: &Self,
    ) -> MResult<Box<[ValueData]>> {
        self.set_binary_elements(other, Value::set_symmetric_difference_elements)
    }

    #[cfg(feature = "functions")]
    pub(crate) fn set_relation(
        &self,
        other: &Self,
        relation: crate::SetValueRelation,
    ) -> MResult<bool> {
        let left = self.snapshot()?;
        let right = other.snapshot()?;
        left.set_relation(
            self.binding.schemas.as_ref(),
            &right,
            other.binding.schemas.as_ref(),
            relation,
        )
        .map_err(snapshot_failure)
    }

    #[cfg(feature = "functions")]
    fn set_binary_elements(
        &self,
        other: &Self,
        operation: fn(
            &Value,
            &SchemaTable,
            &Value,
            &SchemaTable,
        ) -> Result<Box<[ValueData]>, SnapshotValueError>,
    ) -> MResult<Box<[ValueData]>> {
        let left = self.snapshot()?;
        let right = other.snapshot()?;
        operation(
            &left,
            self.binding.schemas.as_ref(),
            &right,
            other.binding.schemas.as_ref(),
        )
        .map_err(snapshot_failure)
    }

    #[cfg(feature = "functions")]
    pub(crate) fn rebuild_set(&self, elements: Box<[ValueData]>) -> MResult<Value> {
        let template = self.snapshot()?;
        template
            .rebuild_set(
                elements,
                &SnapshotValidationContext::new(self.binding.schemas.as_ref()),
            )
            .map_err(snapshot_failure)
    }

    #[cfg(feature = "functions")]
    pub(crate) fn rebuild_set_drafts(&self, elements: Box<[ValueDataDraft]>) -> MResult<Value> {
        let template = self.snapshot()?;
        template
            .rebuild_set_drafts(
                elements,
                &SnapshotValidationContext::new(self.binding.schemas.as_ref()),
            )
            .map_err(snapshot_failure)
    }

    /// Rebuilds this matrix's canonical value for new resolved dimensions.
    /// Dynamic dimensions may change; fixed dimensions remain enforced.
    pub fn rebuild_matrix_drafts(
        &self,
        dimensions: Box<[u64]>,
        elements: Box<[ValueDataDraft]>,
    ) -> MResult<Value> {
        let schema = self
            .binding
            .schemas
            .get(self.binding.schema)
            .expect("value-cell schema remains present");
        let SchemaBody::Matrix {
            dimensions: declared_dimensions,
            ..
        } = schema.body()
        else {
            return Err(backing_mismatch::<Value>(self.representation()));
        };
        if declared_dimensions.len() != dimensions.len() {
            return Err(MechError::new(
                ValueCellShapeMismatch {
                    expected: declared_dimensions
                        .iter()
                        .filter_map(|dimension| {
                            self.binding
                                .shape
                                .borrow()
                                .resolve_dimension(dimension)
                                .ok()
                        })
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                    actual: dimensions,
                },
                None,
            )
            .with_compiler_loc());
        }

        let mut shape_values = self.binding.shape.borrow().parameter_values().to_vec();
        for (declared, actual) in declared_dimensions.iter().zip(dimensions.iter().copied()) {
            match declared {
                DimensionExpr::Constant(expected) if *expected == actual => {}
                DimensionExpr::Parameter(parameter) => {
                    let Some(value) = shape_values.get_mut(parameter.get() as usize) else {
                        return Err(MechError::new(
                            ValueCellShapeMismatch {
                                expected: self
                                    .binding
                                    .shape
                                    .borrow()
                                    .parameter_values()
                                    .to_vec()
                                    .into_boxed_slice(),
                                actual: dimensions.clone(),
                            },
                            None,
                        )
                        .with_compiler_loc());
                    };
                    *value = actual;
                }
                _ => {
                    let expected = declared_dimensions
                        .iter()
                        .map(|dimension| {
                            self.binding
                                .shape
                                .borrow()
                                .resolve_dimension(dimension)
                                .unwrap_or(u64::MAX)
                        })
                        .collect::<Vec<_>>()
                        .into_boxed_slice();
                    return Err(MechError::new(
                        ValueCellShapeMismatch {
                            expected,
                            actual: dimensions,
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
            }
        }
        ValueDraft {
            schema: self.binding.schema,
            shape_values: shape_values.into_boxed_slice(),
            data: ValueDataDraft::Matrix(elements),
        }
        .finalize(&SnapshotValidationContext::new(
            self.binding.schemas.as_ref(),
        ))
        .map_err(snapshot_failure)
    }

    /// Rebuilds canonical data against this cell's schema while retaining the
    /// cell's schema identity. Dynamic set, map, and table extents are
    /// validated by their declared cardinality policy.
    pub fn rebuild_data_draft(&self, data: ValueDataDraft) -> MResult<Value> {
        finalize_draft(
            self.binding.schema,
            &self.binding.shape.borrow(),
            self.binding.schemas.as_ref(),
            data,
        )
    }

    #[cfg(feature = "functions")]
    pub(crate) fn try_ref<T: 'static>(&self) -> MResult<Ref<T>> {
        let exact = self
            .binding
            .storage
            .as_any()
            .downcast_ref::<ExactCellStorage<T>>()
            .map(|storage| storage.reference.clone());
        exact.ok_or_else(|| {
            MechError::new(
                ValueCellBackingMismatch {
                    expected: type_name::<T>().into(),
                    representation: self.representation(),
                },
                None,
            )
            .with_compiler_loc()
        })
    }
}

fn canonical_cell_draft(cell: &ValueCell) -> MResult<ValueDataDraft> {
    cell.snapshot()?
        .canonical_data_draft()
        .map_err(snapshot_failure)
}

fn aggregate_rebuild_unsupported(cell: &ValueCell, expected: &'static str) -> MechError {
    MechError::new(
        ValueCellOutputConstructionUnsupported {
            representation: cell.representation(),
            reason: format!("{expected}-cell reconstruction requires a canonical {expected}"),
        },
        None,
    )
    .with_compiler_loc()
}

fn aggregate_rebuild_arity(
    cell: &ValueCell,
    aggregate: &'static str,
    expected: usize,
    actual: usize,
) -> MechError {
    MechError::new(
        ValueCellOutputConstructionUnsupported {
            representation: cell.representation(),
            reason: format!(
                "{aggregate} schema has {expected} children but {actual} children were supplied"
            ),
        },
        None,
    )
    .with_compiler_loc()
}

fn merged_schema<'a>(
    body: SchemaBody,
    cells: impl IntoIterator<Item = &'a ValueCell>,
) -> MResult<(SchemaId, ShapeInstance, Rc<SchemaTable>)> {
    let schema = crate::SchemaDraft {
        dimension_parameters: Vec::new().into_boxed_slice(),
        body,
    }
    .finalize()
    .map_err(|error| snapshot_failure(error.into()))?;
    let shape = schema
        .instantiate_shape(Vec::new().into_boxed_slice())
        .map_err(|error| snapshot_failure(error.into()))?;
    let mut builder = crate::SchemaTableBuilder::new();
    let handle = builder.insert(schema)?;
    for cell in cells {
        for entry in cell.binding.schemas.entries() {
            builder.insert(entry.schema().clone())?;
        }
    }
    let build = builder.finish()?;
    let schema = build.resolve(handle)?;
    Ok((schema, shape, Rc::new(build.table)))
}

fn table_cell_columns_draft(
    columns: &[(crate::SchemaField, Box<[ValueCell]>)],
    schemas: &SchemaTable,
) -> MResult<ValueDataDraft> {
    columns
        .iter()
        .map(|(field, values)| {
            Ok(crate::snapshot::TableColumnDraft {
                name: field.name.clone(),
                values: values
                    .iter()
                    .map(|cell| canonical_cell_draft_for_schema(cell, &field.schema, schemas))
                    .collect::<MResult<Vec<_>>>()?
                    .into_boxed_slice(),
            })
        })
        .collect::<MResult<Vec<_>>>()
        .map(|columns| ValueDataDraft::Table(columns.into_boxed_slice()))
}

fn record_cell_fields_draft(
    fields: &[(String, ValueCell)],
    schema_fields: &[crate::SchemaField],
    schemas: &SchemaTable,
) -> MResult<ValueDataDraft> {
    fields
        .iter()
        .zip(schema_fields)
        .map(|((name, cell), field)| {
            if *name != field.name {
                return Err(MechError::new(
                    ValueCellOutputConstructionUnsupported {
                        representation: FunctionValueRepresentation::Record,
                        reason: format!(
                            "record schema field {} does not match supplied field {name}",
                            field.name,
                        ),
                    },
                    None,
                )
                .with_compiler_loc());
            }
            Ok(crate::snapshot::NamedValueDraft {
                name: name.clone(),
                value: canonical_cell_draft_for_schema(cell, &field.schema, schemas)?,
            })
        })
        .collect::<MResult<Vec<_>>>()
        .map(|fields| ValueDataDraft::Record(fields.into_boxed_slice()))
}

fn canonical_cell_draft_for_schema(
    cell: &ValueCell,
    expected: &SchemaBody,
    schemas: &SchemaTable,
) -> MResult<ValueDataDraft> {
    let actual = cell.closed_schema_body()?;
    if matches!(expected, SchemaBody::Dynamic) {
        let snapshot = cell.snapshot()?;
        let concrete = match snapshot.data() {
            ValueData::Dynamic(value) => {
                let Some(value) = value.value() else {
                    return Ok(ValueDataDraft::Dynamic(None));
                };
                value
            }
            _ => &snapshot,
        };
        let schema = schemas.find_by_key(concrete.schema_key()).ok_or_else(|| {
            MechError::new(
                ValueCellOutputConstructionUnsupported {
                    representation: cell.representation(),
                    reason: format!(
                        "dynamic child schema {:?} is absent from the aggregate schema arena",
                        concrete.schema_key(),
                    ),
                },
                None,
            )
            .with_compiler_loc()
        })?;
        let shape = concrete.shape().clone();
        let concrete = concrete
            .rebind(schema, &shape, schemas)
            .map_err(snapshot_failure)?;
        let data = concrete.canonical_data_draft().map_err(snapshot_failure)?;
        return Ok(ValueDataDraft::Dynamic(Some(Box::new(ValueDraft {
            schema,
            shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
            data,
        }))));
    }
    if actual != *expected {
        return Err(MechError::new(
            ValueCellOutputConstructionUnsupported {
                representation: cell.representation(),
                reason: format!("aggregate child expected schema {expected:?}, found {actual:?}",),
            },
            None,
        )
        .with_compiler_loc());
    }
    let snapshot = cell.snapshot()?;
    let schema = schemas.find_by_key(snapshot.schema_key()).ok_or_else(|| {
        MechError::new(
            ValueCellOutputConstructionUnsupported {
                representation: cell.representation(),
                reason: format!(
                    "aggregate child schema {:?} is absent from the aggregate schema arena",
                    snapshot.schema_key(),
                ),
            },
            None,
        )
        .with_compiler_loc()
    })?;
    let shape = snapshot.shape().clone();
    snapshot
        .rebind(schema, &shape, schemas)
        .map_err(snapshot_failure)?
        .canonical_data_draft()
        .map_err(snapshot_failure)
}

fn child_cells(schemas: Vec<SchemaBody>, values: Vec<ValueDataDraft>) -> MResult<Vec<ValueCell>> {
    schemas
        .into_iter()
        .zip(values)
        .map(|(schema, value)| ValueCell::from_schema_data(schema, value))
        .collect()
}

#[cfg(feature = "matrix")]
fn default_matrix_cell<T>(
    storage: FunctionMatrixStoragePattern,
    dimensions: (usize, usize),
    default: T,
) -> MResult<ValueCell>
where
    T: CanonicalMatrixElementBacking,
{
    let (rows, columns) = dimensions;
    #[allow(
        unused_macros,
        reason = "exact matrix constructors are feature-selected below"
    )]
    macro_rules! exact {
        ($matrix:expr, $expected_rows:expr, $expected_columns:expr) => {{
            if (rows, columns) != ($expected_rows, $expected_columns) {
                return Err(MechError::new(
                    ValueCellOutputConstructionUnsupported {
                        representation: FunctionValueRepresentation::Matrix {
                            element: crate::matrix_element_for_representation(T::REPRESENTATION),
                            storage,
                        },
                        reason: format!(
                            "declared storage is {}x{}, resolved output is {}x{}",
                            $expected_rows, $expected_columns, rows, columns,
                        ),
                    },
                    None,
                )
                .with_compiler_loc());
            }
            return ValueCell::from_exact_matrix_ref(Ref::new($matrix), rows, columns);
        }};
    }
    match storage {
        #[cfg(feature = "matrix1")]
        FunctionMatrixStoragePattern::Exact(FunctionMatrixRepresentation::Matrix1) => {
            exact!(crate::Matrix1::from_element(default), 1, 1)
        }
        #[cfg(feature = "matrix2")]
        FunctionMatrixStoragePattern::Exact(FunctionMatrixRepresentation::Matrix2) => {
            exact!(crate::Matrix2::from_element(default), 2, 2)
        }
        #[cfg(feature = "matrix3")]
        FunctionMatrixStoragePattern::Exact(FunctionMatrixRepresentation::Matrix3) => {
            exact!(crate::Matrix3::from_element(default), 3, 3)
        }
        #[cfg(feature = "matrix4")]
        FunctionMatrixStoragePattern::Exact(FunctionMatrixRepresentation::Matrix4) => {
            exact!(crate::Matrix4::from_element(default), 4, 4)
        }
        #[cfg(feature = "matrix2x3")]
        FunctionMatrixStoragePattern::Exact(FunctionMatrixRepresentation::Matrix2x3) => {
            exact!(crate::Matrix2x3::from_element(default), 2, 3)
        }
        #[cfg(feature = "matrix3x2")]
        FunctionMatrixStoragePattern::Exact(FunctionMatrixRepresentation::Matrix3x2) => {
            exact!(crate::Matrix3x2::from_element(default), 3, 2)
        }
        #[cfg(feature = "row_vector2")]
        FunctionMatrixStoragePattern::Exact(FunctionMatrixRepresentation::RowVector2) => {
            exact!(crate::RowVector2::from_element(default), 1, 2)
        }
        #[cfg(feature = "row_vector3")]
        FunctionMatrixStoragePattern::Exact(FunctionMatrixRepresentation::RowVector3) => {
            exact!(crate::RowVector3::from_element(default), 1, 3)
        }
        #[cfg(feature = "row_vector4")]
        FunctionMatrixStoragePattern::Exact(FunctionMatrixRepresentation::RowVector4) => {
            exact!(crate::RowVector4::from_element(default), 1, 4)
        }
        #[cfg(feature = "vector2")]
        FunctionMatrixStoragePattern::Exact(FunctionMatrixRepresentation::Vector2) => {
            exact!(crate::Vector2::from_element(default), 2, 1)
        }
        #[cfg(feature = "vector3")]
        FunctionMatrixStoragePattern::Exact(FunctionMatrixRepresentation::Vector3) => {
            exact!(crate::Vector3::from_element(default), 3, 1)
        }
        #[cfg(feature = "vector4")]
        FunctionMatrixStoragePattern::Exact(FunctionMatrixRepresentation::Vector4) => {
            exact!(crate::Vector4::from_element(default), 4, 1)
        }
        #[cfg(feature = "row_vectord")]
        FunctionMatrixStoragePattern::Exact(FunctionMatrixRepresentation::RowVectorD) => {
            if rows != 1 {
                return Err(MechError::new(
                    ValueCellOutputConstructionUnsupported {
                        representation: FunctionValueRepresentation::Matrix {
                            element: crate::matrix_element_for_representation(T::REPRESENTATION),
                            storage,
                        },
                        reason: format!("row-vector output requires one row, found {rows}"),
                    },
                    None,
                )
                .with_compiler_loc());
            }
            ValueCell::from_exact_matrix_ref(
                Ref::new(crate::RowDVector::from_element(columns, default)),
                rows,
                columns,
            )
        }
        #[cfg(feature = "vectord")]
        FunctionMatrixStoragePattern::Exact(FunctionMatrixRepresentation::VectorD) => {
            if columns != 1 {
                return Err(MechError::new(
                    ValueCellOutputConstructionUnsupported {
                        representation: FunctionValueRepresentation::Matrix {
                            element: crate::matrix_element_for_representation(T::REPRESENTATION),
                            storage,
                        },
                        reason: format!(
                            "column-vector output requires one column, found {columns}"
                        ),
                    },
                    None,
                )
                .with_compiler_loc());
            }
            ValueCell::from_exact_matrix_ref(
                Ref::new(crate::DVector::from_element(rows, default)),
                rows,
                columns,
            )
        }
        #[cfg(feature = "matrixd")]
        FunctionMatrixStoragePattern::Exact(FunctionMatrixRepresentation::MatrixD) => {
            ValueCell::from_exact_matrix_ref(
                Ref::new(crate::DMatrix::from_element(rows, columns, default)),
                rows,
                columns,
            )
        }
        _ => Err(MechError::new(
            ValueCellOutputConstructionUnsupported {
                representation: FunctionValueRepresentation::Matrix {
                    element: crate::matrix_element_for_representation(T::REPRESENTATION),
                    storage,
                },
                reason: "output factory requires an exact matrix storage representation".into(),
            },
            None,
        )
        .with_compiler_loc()),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValueCellOutputConstructionUnsupported {
    pub representation: FunctionValueRepresentation,
    pub reason: String,
}

impl MechErrorKind for ValueCellOutputConstructionUnsupported {
    fn name(&self) -> &str {
        "ValueCellOutputConstructionUnsupported"
    }

    fn message(&self) -> String {
        format!(
            "cannot construct canonical output backing for {:?}: {}",
            self.representation, self.reason,
        )
    }
}

fn shape_change_allowed(
    schema: &crate::Schema,
    current: &ShapeInstance,
    next: &ShapeInstance,
) -> bool {
    let current = current.parameter_values();
    let next = next.parameter_values();
    current.len() == next.len()
        && schema
            .dimension_parameters()
            .iter()
            .zip(current.iter().zip(next))
            .all(|(parameter, (current, next))| {
                parameter.lifetime() == crate::DimensionLifetime::Turn || current == next
            })
}

#[cfg(all(feature = "matrix", feature = "matrixd"))]
fn dynamic_matrix_cell(
    elements: SequenceView<'_>,
    schema: SchemaId,
    shape: &ShapeInstance,
    schemas: Rc<SchemaTable>,
    preserve_dynamic_rank: bool,
) -> MResult<Option<ValueCell>> {
    let _ = preserve_dynamic_rank;
    let Some(entry) = schemas.entry(schema) else {
        return Ok(None);
    };
    let SchemaBody::Matrix { dimensions, .. } = entry.schema().body() else {
        return Ok(None);
    };
    let [rows, columns] = dimensions.as_ref() else {
        return Ok(None);
    };
    let (Ok(rows), Ok(columns)) = (
        shape
            .resolve_dimension(rows)
            .ok()
            .and_then(|value| usize::try_from(value).ok())
            .ok_or(()),
        shape
            .resolve_dimension(columns)
            .ok()
            .and_then(|value| usize::try_from(value).ok())
            .ok_or(()),
    ) else {
        return Ok(None);
    };
    macro_rules! matrix {
        ($values:expr) => {{
            #[cfg(feature = "row_vectord")]
            if !preserve_dynamic_rank && rows == 1 {
                let backing = crate::RowDVector::from_row_slice($values);
                return ValueCell::from_ref(Ref::new(backing), schema, shape.clone(), schemas)
                    .map(Some);
            }
            #[cfg(feature = "vectord")]
            if !preserve_dynamic_rank && columns == 1 {
                let backing = crate::DVector::from_column_slice($values);
                return ValueCell::from_ref(Ref::new(backing), schema, shape.clone(), schemas)
                    .map(Some);
            }
            let backing = crate::DMatrix::from_row_slice(rows, columns, $values);
            return ValueCell::from_ref(Ref::new(backing), schema, shape.clone(), schemas)
                .map(Some);
        }};
    }
    match elements {
        #[cfg(feature = "u8")]
        SequenceView::U8(values) => matrix!(values),
        #[cfg(feature = "u16")]
        SequenceView::U16(values) => matrix!(values),
        #[cfg(feature = "u32")]
        SequenceView::U32(values) => matrix!(values),
        #[cfg(feature = "u64")]
        SequenceView::U64(values) => matrix!(values),
        #[cfg(feature = "u128")]
        SequenceView::U128(values) => matrix!(values),
        #[cfg(feature = "i8")]
        SequenceView::I8(values) => matrix!(values),
        #[cfg(feature = "i16")]
        SequenceView::I16(values) => matrix!(values),
        #[cfg(feature = "i32")]
        SequenceView::I32(values) => matrix!(values),
        #[cfg(feature = "i64")]
        SequenceView::I64(values) => matrix!(values),
        #[cfg(feature = "i128")]
        SequenceView::I128(values) => matrix!(values),
        #[cfg(feature = "f32")]
        SequenceView::F32(values) => {
            let values = values
                .iter()
                .map(|value| value.to_f32())
                .collect::<Vec<_>>();
            matrix!(&values)
        }
        #[cfg(feature = "f64")]
        SequenceView::F64(values) => {
            let values = values
                .iter()
                .map(|value| value.to_f64())
                .collect::<Vec<_>>();
            matrix!(&values)
        }
        #[cfg(feature = "complex")]
        SequenceView::Complex64(values) => {
            let values = values
                .iter()
                .map(|value| crate::C64::new(value.real().to_f64(), value.imaginary().to_f64()))
                .collect::<Vec<_>>();
            matrix!(&values)
        }
        #[cfg(feature = "rational")]
        SequenceView::Rational64(values) => {
            let Some(values) = values
                .iter()
                .map(|value| {
                    i64::try_from(value.denominator())
                        .ok()
                        .map(|denominator| crate::R64::new(value.numerator(), denominator))
                })
                .collect::<Option<Vec<_>>>()
            else {
                return Ok(None);
            };
            matrix!(&values)
        }
        #[cfg(feature = "bool")]
        SequenceView::Bool(values) => matrix!(values),
        #[cfg(feature = "string")]
        SequenceView::String(values) => {
            let values = values
                .iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>();
            matrix!(&values)
        }
        SequenceView::Index(values) => {
            let Some(values) = values
                .iter()
                .map(|value| usize::try_from(*value).ok())
                .collect::<Option<Vec<_>>>()
            else {
                return Ok(None);
            };
            matrix!(&values)
        }
        _ => Ok(None),
    }
}

impl fmt::Debug for ValueCell {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ValueCell")
            .field("schema_key", &self.binding.schema_key)
            .field("shape", &self.binding.shape)
            .field("representation", &self.representation())
            .field("borrow_state", &self.binding.storage.borrow_state())
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CellAccess {
    Snapshot,
    Replace,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueCellBorrowConflict {
    pub access: CellAccess,
}

impl MechErrorKind for ValueCellBorrowConflict {
    fn name(&self) -> &str {
        "ValueCellBorrowConflict"
    }

    fn message(&self) -> String {
        format!(
            "canonical value cell is already borrowed during {:?}",
            self.access
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueCellSchemaMismatch {
    pub expected: SchemaKey,
    pub actual: SchemaKey,
}

impl MechErrorKind for ValueCellSchemaMismatch {
    fn name(&self) -> &str {
        "ValueCellSchemaMismatch"
    }

    fn message(&self) -> String {
        "replacement value has a different canonical schema".into()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueCellShapeMismatch {
    pub expected: Box<[u64]>,
    pub actual: Box<[u64]>,
}

impl MechErrorKind for ValueCellShapeMismatch {
    fn name(&self) -> &str {
        "ValueCellShapeMismatch"
    }

    fn message(&self) -> String {
        format!(
            "replacement value has shape {:?}, expected {:?}",
            self.actual, self.expected
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueCellBackingMismatch {
    pub expected: String,
    pub representation: FunctionValueRepresentation,
}

impl MechErrorKind for ValueCellBackingMismatch {
    fn name(&self) -> &str {
        "ValueCellBackingMismatch"
    }

    fn message(&self) -> String {
        format!(
            "canonical value cell representation {:?} does not use exact backing {}",
            self.representation, self.expected
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ValueCellSnapshotFailure {
    pub error: SnapshotValueError,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueCellStorageContractViolation {
    pub schema: SchemaKey,
    pub reason: crate::StorageCompatibilityError,
}

impl MechErrorKind for ValueCellStorageContractViolation {
    fn name(&self) -> &str {
        "ValueCellStorageContractViolation"
    }

    fn message(&self) -> String {
        format!(
            "value-cell storage does not satisfy schema {:?}: {}",
            self.schema, self.reason
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueSchemaContextUnavailable;

impl MechErrorKind for ValueSchemaContextUnavailable {
    fn name(&self) -> &str {
        "ValueSchemaContextUnavailable"
    }

    fn message(&self) -> String {
        "canonical value does not retain the schema table required to create a value cell".into()
    }
}

impl MechErrorKind for ValueCellSnapshotFailure {
    fn name(&self) -> &str {
        "ValueCellSnapshotFailure"
    }

    fn message(&self) -> String {
        format!("canonical value cell snapshot failed: {:?}", self.error)
    }
}

pub(crate) fn borrow_conflict(access: CellAccess) -> MechError {
    MechError::new(ValueCellBorrowConflict { access }, None).with_compiler_loc()
}

fn snapshot_failure(error: SnapshotValueError) -> MechError {
    MechError::new(ValueCellSnapshotFailure { error }, None).with_compiler_loc()
}

pub(crate) fn finalize_draft(
    schema: SchemaId,
    shape: &ShapeInstance,
    schemas: &SchemaTable,
    data: ValueDataDraft,
) -> MResult<Value> {
    ValueDraft {
        schema,
        shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
        data,
    }
    .finalize(&SnapshotValidationContext::new(schemas))
    .map_err(snapshot_failure)
}

#[doc(hidden)]
pub trait CanonicalMatrixElementBacking:
    canonical_matrix_element_sealed::Sealed
    + FunctionRuntimeType
    + Clone
    + fmt::Debug
    + PartialEq
    + 'static
{
    fn data_draft(&self) -> ValueDataDraft;
    fn from_data(data: &ValueData) -> Option<Self>;

    #[cfg(feature = "matrix")]
    fn from_sequence(values: SequenceView<'_>, index: usize) -> Option<Self>;
}

mod canonical_matrix_element_sealed {
    pub trait Sealed: Sized {}
}

macro_rules! scalar_backing {
    ($type:ty, $feature:literal, $draft:ident, $data:ident, $sequence:ident, $legacy:ident, $matrix_legacy:ident) => {
        #[cfg(feature = $feature)]
        impl CanonicalMatrixElementBacking for $type {
            fn data_draft(&self) -> ValueDataDraft {
                ValueDataDraft::$draft(*self)
            }

            fn from_data(data: &ValueData) -> Option<Self> {
                match data {
                    ValueData::$data(value) => Some(*value),
                    _ => None,
                }
            }

            #[cfg(feature = "matrix")]
            fn from_sequence(values: SequenceView<'_>, index: usize) -> Option<Self> {
                match values {
                    SequenceView::$sequence(values) => values.get(index).copied(),
                    _ => None,
                }
            }
        }

        #[cfg(feature = $feature)]
        impl canonical_matrix_element_sealed::Sealed for $type {}

        #[cfg(feature = $feature)]
        impl canonical_cell_sealed::Sealed for $type {
            fn snapshot_bound(
                &self,
                schema: SchemaId,
                shape: &ShapeInstance,
                schemas: &SchemaTable,
            ) -> MResult<Value> {
                finalize_draft(schema, shape, schemas, self.data_draft())
            }

            fn replace_bound(&mut self, value: &Value) -> MResult<()> {
                let replacement = Self::from_data(value.data())
                    .ok_or_else(|| backing_mismatch::<Self>(Self::REPRESENTATION))?;
                *self = replacement;
                Ok(())
            }
        }
    };
}

scalar_backing!(u8, "u8", U8, U8, U8, U8, MatrixU8);
scalar_backing!(u16, "u16", U16, U16, U16, U16, MatrixU16);
scalar_backing!(u32, "u32", U32, U32, U32, U32, MatrixU32);
scalar_backing!(u64, "u64", U64, U64, U64, U64, MatrixU64);
scalar_backing!(u128, "u128", U128, U128, U128, U128, MatrixU128);
scalar_backing!(i8, "i8", I8, I8, I8, I8, MatrixI8);
scalar_backing!(i16, "i16", I16, I16, I16, I16, MatrixI16);
scalar_backing!(i32, "i32", I32, I32, I32, I32, MatrixI32);
scalar_backing!(i64, "i64", I64, I64, I64, I64, MatrixI64);
scalar_backing!(i128, "i128", I128, I128, I128, I128, MatrixI128);
scalar_backing!(bool, "bool", Bool, Bool, Bool, Bool, MatrixBool);

macro_rules! float_backing {
    ($type:ty, $feature:literal, $draft:ident, $data:ident, $sequence:ident, $bits:ty, $from:ident, $to:ident, $legacy:ident, $matrix_legacy:ident) => {
        #[cfg(feature = $feature)]
        impl CanonicalMatrixElementBacking for $type {
            fn data_draft(&self) -> ValueDataDraft {
                ValueDataDraft::$draft(<$bits>::$from(*self))
            }

            fn from_data(data: &ValueData) -> Option<Self> {
                match data {
                    ValueData::$data(value) => Some(value.$to()),
                    _ => None,
                }
            }

            #[cfg(feature = "matrix")]
            fn from_sequence(values: SequenceView<'_>, index: usize) -> Option<Self> {
                match values {
                    SequenceView::$sequence(values) => values.get(index).map(|value| value.$to()),
                    _ => None,
                }
            }
        }

        #[cfg(feature = $feature)]
        impl canonical_matrix_element_sealed::Sealed for $type {}

        #[cfg(feature = $feature)]
        impl canonical_cell_sealed::Sealed for $type {
            fn snapshot_bound(
                &self,
                schema: SchemaId,
                shape: &ShapeInstance,
                schemas: &SchemaTable,
            ) -> MResult<Value> {
                finalize_draft(schema, shape, schemas, self.data_draft())
            }

            fn replace_bound(&mut self, value: &Value) -> MResult<()> {
                let replacement = Self::from_data(value.data())
                    .ok_or_else(|| backing_mismatch::<Self>(Self::REPRESENTATION))?;
                *self = replacement;
                Ok(())
            }
        }
    };
}

float_backing!(
    f32,
    "f32",
    F32,
    F32,
    F32,
    crate::snapshot::F32Bits,
    from_f32,
    to_f32,
    F32,
    MatrixF32
);
float_backing!(
    f64,
    "f64",
    F64,
    F64,
    F64,
    crate::snapshot::F64Bits,
    from_f64,
    to_f64,
    F64,
    MatrixF64
);

impl CanonicalMatrixElementBacking for usize {
    fn data_draft(&self) -> ValueDataDraft {
        ValueDataDraft::Index(*self as u64)
    }

    fn from_data(data: &ValueData) -> Option<Self> {
        match data {
            ValueData::Index(value) => usize::try_from(*value).ok(),
            _ => None,
        }
    }

    #[cfg(feature = "matrix")]
    fn from_sequence(values: SequenceView<'_>, index: usize) -> Option<Self> {
        match values {
            SequenceView::Index(values) => {
                values.get(index).and_then(|value| (*value).try_into().ok())
            }
            _ => None,
        }
    }
}

impl canonical_matrix_element_sealed::Sealed for usize {}

impl canonical_cell_sealed::Sealed for usize {
    fn snapshot_bound(
        &self,
        schema: SchemaId,
        shape: &ShapeInstance,
        schemas: &SchemaTable,
    ) -> MResult<Value> {
        finalize_draft(schema, shape, schemas, self.data_draft())
    }

    fn replace_bound(&mut self, value: &Value) -> MResult<()> {
        *self = Self::from_data(value.data())
            .ok_or_else(|| backing_mismatch::<Self>(Self::REPRESENTATION))?;
        Ok(())
    }
}

#[cfg(feature = "string")]
impl CanonicalMatrixElementBacking for String {
    fn data_draft(&self) -> ValueDataDraft {
        ValueDataDraft::String(self.clone())
    }

    fn from_data(data: &ValueData) -> Option<Self> {
        match data {
            ValueData::String(value) => Some(value.to_string()),
            _ => None,
        }
    }

    #[cfg(feature = "matrix")]
    fn from_sequence(values: SequenceView<'_>, index: usize) -> Option<Self> {
        match values {
            SequenceView::String(values) => values.get(index).map(|value| value.to_string()),
            _ => None,
        }
    }
}

#[cfg(feature = "string")]
impl canonical_matrix_element_sealed::Sealed for String {}

#[cfg(feature = "string")]
impl canonical_cell_sealed::Sealed for String {
    fn snapshot_bound(
        &self,
        schema: SchemaId,
        shape: &ShapeInstance,
        schemas: &SchemaTable,
    ) -> MResult<Value> {
        finalize_draft(schema, shape, schemas, self.data_draft())
    }

    fn replace_bound(&mut self, value: &Value) -> MResult<()> {
        *self = Self::from_data(value.data())
            .ok_or_else(|| backing_mismatch::<Self>(Self::REPRESENTATION))?;
        Ok(())
    }
}

#[cfg(feature = "complex")]
impl CanonicalMatrixElementBacking for crate::C64 {
    fn data_draft(&self) -> ValueDataDraft {
        ValueDataDraft::Complex64(crate::snapshot::Complex64Bits::new(
            crate::snapshot::F64Bits::from_f64(self.0.re),
            crate::snapshot::F64Bits::from_f64(self.0.im),
        ))
    }

    fn from_data(data: &ValueData) -> Option<Self> {
        match data {
            ValueData::Complex64(value) => Some(crate::C64::new(
                value.real().to_f64(),
                value.imaginary().to_f64(),
            )),
            _ => None,
        }
    }

    #[cfg(feature = "matrix")]
    fn from_sequence(values: SequenceView<'_>, index: usize) -> Option<Self> {
        match values {
            SequenceView::Complex64(values) => values
                .get(index)
                .map(|value| crate::C64::new(value.real().to_f64(), value.imaginary().to_f64())),
            _ => None,
        }
    }
}

#[cfg(feature = "complex")]
impl canonical_matrix_element_sealed::Sealed for crate::C64 {}

#[cfg(feature = "complex")]
impl canonical_cell_sealed::Sealed for crate::C64 {
    fn snapshot_bound(
        &self,
        schema: SchemaId,
        shape: &ShapeInstance,
        schemas: &SchemaTable,
    ) -> MResult<Value> {
        finalize_draft(schema, shape, schemas, self.data_draft())
    }

    fn replace_bound(&mut self, value: &Value) -> MResult<()> {
        *self = Self::from_data(value.data())
            .ok_or_else(|| backing_mismatch::<Self>(Self::REPRESENTATION))?;
        Ok(())
    }
}

#[cfg(feature = "rational")]
impl CanonicalMatrixElementBacking for crate::R64 {
    fn data_draft(&self) -> ValueDataDraft {
        ValueDataDraft::Rational64 {
            numerator: *self.numer(),
            denominator: *self.denom() as u64,
        }
    }

    fn from_data(data: &ValueData) -> Option<Self> {
        match data {
            ValueData::Rational64(value) => i64::try_from(value.denominator())
                .ok()
                .map(|denominator| crate::R64::new(value.numerator(), denominator)),
            _ => None,
        }
    }

    #[cfg(feature = "matrix")]
    fn from_sequence(values: SequenceView<'_>, index: usize) -> Option<Self> {
        match values {
            SequenceView::Rational64(values) => values.get(index).and_then(|value| {
                i64::try_from(value.denominator())
                    .ok()
                    .map(|denominator| crate::R64::new(value.numerator(), denominator))
            }),
            _ => None,
        }
    }
}

#[cfg(feature = "rational")]
impl canonical_matrix_element_sealed::Sealed for crate::R64 {}

#[cfg(feature = "rational")]
impl canonical_cell_sealed::Sealed for crate::R64 {
    fn snapshot_bound(
        &self,
        schema: SchemaId,
        shape: &ShapeInstance,
        schemas: &SchemaTable,
    ) -> MResult<Value> {
        finalize_draft(schema, shape, schemas, self.data_draft())
    }

    fn replace_bound(&mut self, value: &Value) -> MResult<()> {
        *self = Self::from_data(value.data())
            .ok_or_else(|| backing_mismatch::<Self>(Self::REPRESENTATION))?;
        Ok(())
    }
}

#[cfg(feature = "matrix")]
fn matrix_snapshot<T>(
    matrix: &impl CanonicalMatrix<T>,
    schema: SchemaId,
    shape: &ShapeInstance,
    schemas: &SchemaTable,
) -> MResult<Value>
where
    T: CanonicalMatrixElementBacking,
{
    validate_matrix_shape(matrix, schema, shape, schemas)?;
    let mut elements = Vec::with_capacity(matrix.rows().saturating_mul(matrix.cols()));
    for row in 0..matrix.rows() {
        for column in 0..matrix.cols() {
            elements.push(matrix.element(row, column).data_draft());
        }
    }
    finalize_draft(
        schema,
        shape,
        schemas,
        ValueDataDraft::Matrix(elements.into_boxed_slice()),
    )
}

#[cfg(feature = "matrix")]
fn matrix_replace<T>(matrix: &mut impl CanonicalMatrix<T>, value: &Value) -> MResult<()>
where
    T: CanonicalMatrixElementBacking,
{
    let ValueData::Matrix(replacement) = value.data() else {
        return Err(backing_mismatch::<T>(T::REPRESENTATION));
    };
    let schemas = value
        .schemas()
        .ok_or_else(|| MechError::new(ValueSchemaContextUnavailable, None).with_compiler_loc())?;
    let Some(SchemaBody::Matrix { dimensions, .. }) =
        schemas.get(value.schema()).map(|schema| schema.body())
    else {
        return Err(backing_mismatch::<T>(T::REPRESENTATION));
    };
    let [rows, columns] = dimensions.as_ref() else {
        return Err(backing_mismatch::<T>(T::REPRESENTATION));
    };
    let rows = usize::try_from(
        value
            .shape()
            .resolve_dimension(rows)
            .map_err(|error| snapshot_failure(error.into()))?,
    )
    .map_err(|_| backing_mismatch::<T>(T::REPRESENTATION))?;
    let columns = usize::try_from(
        value
            .shape()
            .resolve_dimension(columns)
            .map_err(|error| snapshot_failure(error.into()))?,
    )
    .map_err(|_| backing_mismatch::<T>(T::REPRESENTATION))?;
    let expected = rows.saturating_mul(columns);
    let values = replacement.elements();
    let mut elements = Vec::with_capacity(expected);
    for index in 0..expected {
        elements.push(
            T::from_sequence(values, index)
                .ok_or_else(|| backing_mismatch::<T>(T::REPRESENTATION))?,
        );
    }
    matrix
        .replace_elements(rows, columns, elements)
        .ok_or_else(|| {
            MechError::new(
                ValueCellShapeMismatch {
                    expected: vec![matrix.rows() as u64, matrix.cols() as u64].into_boxed_slice(),
                    actual: vec![rows as u64, columns as u64].into_boxed_slice(),
                },
                None,
            )
            .with_compiler_loc()
        })
}

#[cfg(feature = "matrix")]
trait CanonicalMatrix<T>: Sized {
    fn rows(&self) -> usize;
    fn cols(&self) -> usize;
    fn element(&self, row: usize, column: usize) -> &T;
    fn replace_elements(&mut self, rows: usize, columns: usize, elements: Vec<T>) -> Option<()>;
}

#[cfg(feature = "matrix")]
fn validate_matrix_shape<T>(
    matrix: &impl CanonicalMatrix<T>,
    schema: SchemaId,
    shape: &ShapeInstance,
    schemas: &SchemaTable,
) -> MResult<()> {
    let Some(SchemaBody::Matrix { dimensions, .. }) =
        schemas.get(schema).map(|schema| schema.body())
    else {
        return Err(backing_mismatch::<T>(FunctionValueRepresentation::AnyValue));
    };
    if dimensions.len() != 2
        || shape.resolve_dimension(&dimensions[0]).ok() != Some(matrix.rows() as u64)
        || shape.resolve_dimension(&dimensions[1]).ok() != Some(matrix.cols() as u64)
    {
        return Err(MechError::new(
            ValueCellShapeMismatch {
                expected: shape.parameter_values().to_vec().into_boxed_slice(),
                actual: vec![matrix.rows() as u64, matrix.cols() as u64].into_boxed_slice(),
            },
            None,
        )
        .with_compiler_loc());
    }
    Ok(())
}

#[cfg(feature = "matrix")]
macro_rules! matrix_backing {
    ($type:ident, $feature:literal) => {
        #[cfg(feature = $feature)]
        impl<T> CanonicalMatrix<T> for crate::$type<T>
        where
            T: CanonicalMatrixElementBacking,
        {
            fn rows(&self) -> usize {
                self.nrows()
            }

            fn cols(&self) -> usize {
                self.ncols()
            }

            fn element(&self, row: usize, column: usize) -> &T {
                &self[(row, column)]
            }

            fn replace_elements(
                &mut self,
                rows: usize,
                columns: usize,
                elements: Vec<T>,
            ) -> Option<()> {
                if rows != self.nrows() || columns != self.ncols() {
                    return None;
                }
                let mut replaced = self.clone();
                for (index, element) in elements.into_iter().enumerate() {
                    replaced[(index / columns, index % columns)] = element;
                }
                *self = replaced;
                Some(())
            }
        }

        #[cfg(feature = $feature)]
        impl<T> canonical_cell_sealed::Sealed for crate::$type<T>
        where
            T: CanonicalMatrixElementBacking,
            crate::$type<T>: FunctionRuntimeType,
        {
            fn snapshot_bound(
                &self,
                schema: SchemaId,
                shape: &ShapeInstance,
                schemas: &SchemaTable,
            ) -> MResult<Value> {
                matrix_snapshot(self, schema, shape, schemas)
            }

            fn replace_bound(&mut self, value: &Value) -> MResult<()> {
                matrix_replace(self, value)
            }

            fn matrix_extents(&self) -> Option<(usize, usize)> {
                Some((self.nrows(), self.ncols()))
            }
        }
    };
}

#[cfg(feature = "matrix")]
matrix_backing!(Matrix1, "matrix1");
#[cfg(feature = "matrix")]
matrix_backing!(Matrix2, "matrix2");
#[cfg(feature = "matrix")]
matrix_backing!(Matrix3, "matrix3");
#[cfg(feature = "matrix")]
matrix_backing!(Matrix4, "matrix4");
#[cfg(feature = "matrix")]
matrix_backing!(Matrix2x3, "matrix2x3");
#[cfg(feature = "matrix")]
matrix_backing!(Matrix3x2, "matrix3x2");
#[cfg(feature = "matrix")]
matrix_backing!(RowVector2, "row_vector2");
#[cfg(feature = "matrix")]
matrix_backing!(RowVector3, "row_vector3");
#[cfg(feature = "matrix")]
matrix_backing!(RowVector4, "row_vector4");
#[cfg(feature = "matrix")]
matrix_backing!(Vector2, "vector2");
#[cfg(feature = "matrix")]
matrix_backing!(Vector3, "vector3");
#[cfg(feature = "matrix")]
matrix_backing!(Vector4, "vector4");
#[cfg(feature = "matrix")]
macro_rules! dynamic_matrix_storage {
    ($type:ident, $feature:literal) => {
        #[cfg(feature = $feature)]
        impl<T> canonical_cell_sealed::Sealed for crate::$type<T>
        where
            T: CanonicalMatrixElementBacking,
            crate::$type<T>: FunctionRuntimeType,
        {
            fn snapshot_bound(
                &self,
                schema: SchemaId,
                shape: &ShapeInstance,
                schemas: &SchemaTable,
            ) -> MResult<Value> {
                matrix_snapshot(self, schema, shape, schemas)
            }

            fn replace_bound(&mut self, value: &Value) -> MResult<()> {
                matrix_replace(self, value)
            }

            fn matrix_extents(&self) -> Option<(usize, usize)> {
                Some((self.nrows(), self.ncols()))
            }
        }
    };
}

#[cfg(feature = "row_vectord")]
impl<T: CanonicalMatrixElementBacking> CanonicalMatrix<T> for crate::RowDVector<T> {
    fn rows(&self) -> usize {
        self.nrows()
    }

    fn cols(&self) -> usize {
        self.ncols()
    }

    fn element(&self, row: usize, column: usize) -> &T {
        &self[(row, column)]
    }

    fn replace_elements(&mut self, rows: usize, columns: usize, elements: Vec<T>) -> Option<()> {
        if rows != 1 {
            return None;
        }
        *self = crate::RowDVector::from_row_slice(&elements[..columns]);
        Some(())
    }
}

#[cfg(feature = "vectord")]
impl<T: CanonicalMatrixElementBacking> CanonicalMatrix<T> for crate::DVector<T> {
    fn rows(&self) -> usize {
        self.nrows()
    }

    fn cols(&self) -> usize {
        self.ncols()
    }

    fn element(&self, row: usize, column: usize) -> &T {
        &self[(row, column)]
    }

    fn replace_elements(&mut self, rows: usize, columns: usize, elements: Vec<T>) -> Option<()> {
        if columns != 1 || rows != elements.len() {
            return None;
        }
        *self = crate::DVector::from_vec(elements);
        Some(())
    }
}

#[cfg(feature = "matrixd")]
impl<T: CanonicalMatrixElementBacking> CanonicalMatrix<T> for crate::DMatrix<T> {
    fn rows(&self) -> usize {
        self.nrows()
    }

    fn cols(&self) -> usize {
        self.ncols()
    }

    fn element(&self, row: usize, column: usize) -> &T {
        &self[(row, column)]
    }

    fn replace_elements(&mut self, rows: usize, columns: usize, elements: Vec<T>) -> Option<()> {
        *self = crate::DMatrix::from_row_slice(rows, columns, &elements);
        Some(())
    }
}

#[cfg(feature = "matrix")]
dynamic_matrix_storage!(RowDVector, "row_vectord");
#[cfg(feature = "matrix")]
dynamic_matrix_storage!(DVector, "vectord");
#[cfg(feature = "matrix")]
dynamic_matrix_storage!(DMatrix, "matrixd");

impl canonical_cell_sealed::Sealed for Value {
    fn snapshot_bound(
        &self,
        schema: SchemaId,
        shape: &ShapeInstance,
        schemas: &SchemaTable,
    ) -> MResult<Value> {
        self.validate_against(schemas).map_err(snapshot_failure)?;
        if self.schema() != schema || self.shape() != shape {
            return Err(MechError::new(
                ValueCellShapeMismatch {
                    expected: shape.parameter_values().to_vec().into_boxed_slice(),
                    actual: self.shape().parameter_values().to_vec().into_boxed_slice(),
                },
                None,
            )
            .with_compiler_loc());
        }
        Ok(self.clone())
    }

    fn replace_bound(&mut self, value: &Value) -> MResult<()> {
        *self = value.clone();
        Ok(())
    }

    fn representation(schema: &SchemaBody) -> FunctionValueRepresentation {
        representation_for_schema(schema)
    }
}

fn representation_for_schema(schema: &SchemaBody) -> FunctionValueRepresentation {
    match schema {
        SchemaBody::Dynamic => FunctionValueRepresentation::AnyValue,
        SchemaBody::UnsignedInteger(IntegerWidth::W8) => FunctionValueRepresentation::U8,
        SchemaBody::UnsignedInteger(IntegerWidth::W16) => FunctionValueRepresentation::U16,
        SchemaBody::UnsignedInteger(IntegerWidth::W32) => FunctionValueRepresentation::U32,
        SchemaBody::UnsignedInteger(IntegerWidth::W64) => FunctionValueRepresentation::U64,
        SchemaBody::UnsignedInteger(IntegerWidth::W128) => FunctionValueRepresentation::U128,
        SchemaBody::SignedInteger(IntegerWidth::W8) => FunctionValueRepresentation::I8,
        SchemaBody::SignedInteger(IntegerWidth::W16) => FunctionValueRepresentation::I16,
        SchemaBody::SignedInteger(IntegerWidth::W32) => FunctionValueRepresentation::I32,
        SchemaBody::SignedInteger(IntegerWidth::W64) => FunctionValueRepresentation::I64,
        SchemaBody::SignedInteger(IntegerWidth::W128) => FunctionValueRepresentation::I128,
        SchemaBody::FloatingPoint(FloatWidth::W32) => FunctionValueRepresentation::F32,
        SchemaBody::FloatingPoint(FloatWidth::W64) => FunctionValueRepresentation::F64,
        SchemaBody::Complex(FloatWidth::W64) => FunctionValueRepresentation::C64,
        SchemaBody::Rational64 => FunctionValueRepresentation::R64,
        SchemaBody::String => FunctionValueRepresentation::String,
        SchemaBody::Bool => FunctionValueRepresentation::Bool,
        SchemaBody::Id => FunctionValueRepresentation::Id,
        SchemaBody::Index => FunctionValueRepresentation::Index,
        SchemaBody::Matrix { element, .. } => FunctionValueRepresentation::Matrix {
            element: matrix_element_for_schema(element),
            storage: FunctionMatrixStoragePattern::AnyStorage,
        },
        SchemaBody::Atom(_) => FunctionValueRepresentation::Atom,
        SchemaBody::Enum { .. } => FunctionValueRepresentation::Enum,
        SchemaBody::Record(_) => FunctionValueRepresentation::Record,
        SchemaBody::Map { .. } => FunctionValueRepresentation::Map,
        SchemaBody::Set { .. } => FunctionValueRepresentation::Set,
        SchemaBody::Table { .. } => FunctionValueRepresentation::Table,
        SchemaBody::Tuple(_) => FunctionValueRepresentation::Tuple,
        SchemaBody::ReifiedType => FunctionValueRepresentation::Kind,
        SchemaBody::Option(_) | SchemaBody::Complex(FloatWidth::W32) => {
            FunctionValueRepresentation::AnyValue
        }
    }
}

fn matrix_element_for_schema(schema: &SchemaBody) -> FunctionMatrixElement {
    match representation_for_schema(schema) {
        FunctionValueRepresentation::U8 => FunctionMatrixElement::U8,
        FunctionValueRepresentation::U16 => FunctionMatrixElement::U16,
        FunctionValueRepresentation::U32 => FunctionMatrixElement::U32,
        FunctionValueRepresentation::U64 => FunctionMatrixElement::U64,
        FunctionValueRepresentation::U128 => FunctionMatrixElement::U128,
        FunctionValueRepresentation::I8 => FunctionMatrixElement::I8,
        FunctionValueRepresentation::I16 => FunctionMatrixElement::I16,
        FunctionValueRepresentation::I32 => FunctionMatrixElement::I32,
        FunctionValueRepresentation::I64 => FunctionMatrixElement::I64,
        FunctionValueRepresentation::I128 => FunctionMatrixElement::I128,
        FunctionValueRepresentation::F32 => FunctionMatrixElement::F32,
        FunctionValueRepresentation::F64 => FunctionMatrixElement::F64,
        FunctionValueRepresentation::C64 => FunctionMatrixElement::C64,
        FunctionValueRepresentation::R64 => FunctionMatrixElement::R64,
        FunctionValueRepresentation::String => FunctionMatrixElement::String,
        FunctionValueRepresentation::Bool => FunctionMatrixElement::Bool,
        FunctionValueRepresentation::Index => FunctionMatrixElement::Index,
        _ => FunctionMatrixElement::Value,
    }
}

fn backing_mismatch<T>(representation: FunctionValueRepresentation) -> MechError {
    MechError::new(
        ValueCellBackingMismatch {
            expected: type_name::<T>().into(),
            representation,
        },
        None,
    )
    .with_compiler_loc()
}

fn schema_body_for_representation(
    representation: FunctionValueRepresentation,
    matrix_extents: Option<(usize, usize)>,
) -> Option<SchemaBody> {
    Some(match representation {
        FunctionValueRepresentation::U8 => SchemaBody::UnsignedInteger(IntegerWidth::W8),
        FunctionValueRepresentation::U16 => SchemaBody::UnsignedInteger(IntegerWidth::W16),
        FunctionValueRepresentation::U32 => SchemaBody::UnsignedInteger(IntegerWidth::W32),
        FunctionValueRepresentation::U64 => SchemaBody::UnsignedInteger(IntegerWidth::W64),
        FunctionValueRepresentation::U128 => SchemaBody::UnsignedInteger(IntegerWidth::W128),
        FunctionValueRepresentation::I8 => SchemaBody::SignedInteger(IntegerWidth::W8),
        FunctionValueRepresentation::I16 => SchemaBody::SignedInteger(IntegerWidth::W16),
        FunctionValueRepresentation::I32 => SchemaBody::SignedInteger(IntegerWidth::W32),
        FunctionValueRepresentation::I64 => SchemaBody::SignedInteger(IntegerWidth::W64),
        FunctionValueRepresentation::I128 => SchemaBody::SignedInteger(IntegerWidth::W128),
        FunctionValueRepresentation::F32 => SchemaBody::FloatingPoint(FloatWidth::W32),
        FunctionValueRepresentation::F64 => SchemaBody::FloatingPoint(FloatWidth::W64),
        FunctionValueRepresentation::C64 => SchemaBody::Complex(FloatWidth::W64),
        FunctionValueRepresentation::R64 => SchemaBody::Rational64,
        FunctionValueRepresentation::String => SchemaBody::String,
        FunctionValueRepresentation::Bool => SchemaBody::Bool,
        FunctionValueRepresentation::Id => SchemaBody::Id,
        FunctionValueRepresentation::Index => SchemaBody::Index,
        FunctionValueRepresentation::Matrix { element, .. } => {
            let (rows, columns) = matrix_extents?;
            SchemaBody::Matrix {
                element: Box::new(schema_body_for_matrix_element(element)?),
                dimensions: vec![
                    crate::DimensionExpr::Constant(rows as u64),
                    crate::DimensionExpr::Constant(columns as u64),
                ]
                .into_boxed_slice(),
            }
        }
        _ => return None,
    })
}

fn schema_body_for_matrix_element(element: FunctionMatrixElement) -> Option<SchemaBody> {
    Some(match element {
        FunctionMatrixElement::U8 => SchemaBody::UnsignedInteger(IntegerWidth::W8),
        FunctionMatrixElement::U16 => SchemaBody::UnsignedInteger(IntegerWidth::W16),
        FunctionMatrixElement::U32 => SchemaBody::UnsignedInteger(IntegerWidth::W32),
        FunctionMatrixElement::U64 => SchemaBody::UnsignedInteger(IntegerWidth::W64),
        FunctionMatrixElement::U128 => SchemaBody::UnsignedInteger(IntegerWidth::W128),
        FunctionMatrixElement::I8 => SchemaBody::SignedInteger(IntegerWidth::W8),
        FunctionMatrixElement::I16 => SchemaBody::SignedInteger(IntegerWidth::W16),
        FunctionMatrixElement::I32 => SchemaBody::SignedInteger(IntegerWidth::W32),
        FunctionMatrixElement::I64 => SchemaBody::SignedInteger(IntegerWidth::W64),
        FunctionMatrixElement::I128 => SchemaBody::SignedInteger(IntegerWidth::W128),
        FunctionMatrixElement::F32 => SchemaBody::FloatingPoint(FloatWidth::W32),
        FunctionMatrixElement::F64 => SchemaBody::FloatingPoint(FloatWidth::W64),
        FunctionMatrixElement::C64 => SchemaBody::Complex(FloatWidth::W64),
        FunctionMatrixElement::R64 => SchemaBody::Rational64,
        FunctionMatrixElement::String => SchemaBody::String,
        FunctionMatrixElement::Bool => SchemaBody::Bool,
        FunctionMatrixElement::Index => SchemaBody::Index,
        FunctionMatrixElement::Value => return None,
    })
}

pub(crate) fn close_schema_body(body: &SchemaBody, shape: &ShapeInstance) -> MResult<SchemaBody> {
    fn dimension(expression: &DimensionExpr, shape: &ShapeInstance) -> MResult<DimensionExpr> {
        shape
            .resolve_dimension(expression)
            .map(DimensionExpr::Constant)
            .map_err(|error| snapshot_failure(error.into()))
    }

    fn close_extent(value: &CardinalitySpec, shape: &ShapeInstance) -> MResult<CardinalitySpec> {
        Ok(match value {
            CardinalitySpec::Exact(value) => CardinalitySpec::Exact(dimension(value, shape)?),
            CardinalitySpec::Dynamic { upper_bound } => CardinalitySpec::Dynamic {
                upper_bound: upper_bound
                    .as_ref()
                    .map(|value| dimension(value, shape))
                    .transpose()?,
            },
        })
    }

    Ok(match body {
        SchemaBody::Dynamic => SchemaBody::Dynamic,
        SchemaBody::Bool => SchemaBody::Bool,
        SchemaBody::UnsignedInteger(width) => SchemaBody::UnsignedInteger(*width),
        SchemaBody::SignedInteger(width) => SchemaBody::SignedInteger(*width),
        SchemaBody::FloatingPoint(width) => SchemaBody::FloatingPoint(*width),
        SchemaBody::Complex(width) => SchemaBody::Complex(*width),
        SchemaBody::Rational64 => SchemaBody::Rational64,
        SchemaBody::String => SchemaBody::String,
        SchemaBody::Id => SchemaBody::Id,
        SchemaBody::Index => SchemaBody::Index,
        SchemaBody::Atom(key) => SchemaBody::Atom(*key),
        SchemaBody::Enum { key, variants } => SchemaBody::Enum {
            key: *key,
            variants: variants
                .iter()
                .map(|variant| {
                    Ok(crate::EnumVariantSchema {
                        name: variant.name.clone(),
                        payload: variant
                            .payload
                            .as_ref()
                            .map(|payload| close_schema_body(payload, shape))
                            .transpose()?,
                    })
                })
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
        },
        SchemaBody::Option(element) => {
            SchemaBody::Option(Box::new(close_schema_body(element, shape)?))
        }
        SchemaBody::Tuple(elements) => SchemaBody::Tuple(
            elements
                .iter()
                .map(|element| close_schema_body(element, shape))
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
        ),
        SchemaBody::Record(fields) => SchemaBody::Record(
            fields
                .iter()
                .map(|field| {
                    Ok(crate::SchemaField {
                        name: field.name.clone(),
                        schema: close_schema_body(&field.schema, shape)?,
                    })
                })
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
        ),
        SchemaBody::Matrix {
            element,
            dimensions,
        } => SchemaBody::Matrix {
            element: Box::new(close_schema_body(element, shape)?),
            dimensions: dimensions
                .iter()
                .map(|value| dimension(value, shape))
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
        },
        SchemaBody::Table { columns, rows } => SchemaBody::Table {
            columns: columns
                .iter()
                .map(|field| {
                    Ok(crate::SchemaField {
                        name: field.name.clone(),
                        schema: close_schema_body(&field.schema, shape)?,
                    })
                })
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
            rows: close_extent(rows, shape)?,
        },
        SchemaBody::Set {
            element,
            cardinality: value,
        } => SchemaBody::Set {
            element: Box::new(close_schema_body(element, shape)?),
            cardinality: close_extent(value, shape)?,
        },
        SchemaBody::Map {
            key,
            value,
            cardinality,
        } => SchemaBody::Map {
            key: Box::new(close_schema_body(key, shape)?),
            value: Box::new(close_schema_body(value, shape)?),
            cardinality: close_extent(cardinality, shape)?,
        },
        SchemaBody::ReifiedType => SchemaBody::ReifiedType,
    })
}

fn rebind_value(value: Value, schemas: &SchemaTable) -> MResult<Value> {
    let schema = schemas.find_by_key(value.schema_key()).ok_or_else(|| {
        snapshot_failure(SnapshotValueError::SnapshotSchemaTableMismatch {
            schema: value.schema(),
            expected: value.schema_key(),
            actual: schemas.entry(value.schema()).map(|entry| entry.key()),
        })
    })?;
    value
        .rebind(schema, value.shape(), schemas)
        .map_err(snapshot_failure)
}

fn dynamic_matrix_schema(
    element: SchemaBody,
    dimensions: Box<[u64]>,
) -> MResult<(SchemaId, ShapeInstance, Rc<SchemaTable>)> {
    let declarations = dimensions
        .iter()
        .enumerate()
        .map(|(index, _)| crate::DimensionParameterDeclaration {
            id: crate::DimensionParameterId::new(index as u32),
            origin: crate::DimensionParameterOrigin::Inferred,
            lifetime: crate::DimensionLifetime::Turn,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: None,
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let body = SchemaBody::Matrix {
        element: Box::new(element),
        dimensions: (0..dimensions.len())
            .map(|index| DimensionExpr::Parameter(crate::DimensionParameterId::new(index as u32)))
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    };
    let schema = crate::SchemaDraft {
        dimension_parameters: declarations,
        body,
    }
    .finalize()
    .map_err(|error| snapshot_failure(error.into()))?;
    let shape = schema
        .instantiate_shape(dimensions)
        .map_err(|error| snapshot_failure(error.into()))?;
    let mut builder = crate::SchemaTableBuilder::new();
    let handle = builder
        .insert(schema)
        .map_err(|error| snapshot_failure(error.into()))?;
    let build = builder
        .finish()
        .map_err(|error| snapshot_failure(error.into()))?;
    let schema = build
        .resolve(handle)
        .map_err(|error| snapshot_failure(error.into()))?;
    Ok((schema, shape, Rc::new(build.table)))
}

fn standalone_schema(body: SchemaBody) -> MResult<(SchemaId, ShapeInstance, Rc<SchemaTable>)> {
    let schema = crate::SchemaDraft {
        dimension_parameters: Vec::new().into_boxed_slice(),
        body,
    }
    .finalize()
    .map_err(|error| snapshot_failure(error.into()))?;
    let shape = schema
        .instantiate_shape(Vec::new().into_boxed_slice())
        .map_err(|error| snapshot_failure(error.into()))?;
    let mut builder = crate::SchemaTableBuilder::new();
    let handle = builder
        .insert(schema)
        .map_err(|error| snapshot_failure(error.into()))?;
    let build = builder
        .finish()
        .map_err(|error| snapshot_failure(error.into()))?;
    let schema = build
        .resolve(handle)
        .map_err(|error| snapshot_failure(error.into()))?;
    Ok((schema, shape, Rc::new(build.table)))
}

#[cfg(all(test, any(feature = "f64", feature = "u8", feature = "string")))]
mod tests {
    use super::*;
    #[cfg(any(feature = "f64", feature = "u8"))]
    use crate::DimensionExpr;
    #[cfg(all(feature = "f64", feature = "matrix"))]
    use crate::{DimensionLifetime, DimensionParameterId, DimensionParameterOrigin};
    use crate::{DimensionParameterDeclaration, SchemaDraft, SchemaTableBuilder};

    struct TestSchema {
        id: SchemaId,
        shape: ShapeInstance,
        schemas: Rc<SchemaTable>,
    }

    fn test_schema(
        body: SchemaBody,
        dimensions: Box<[DimensionParameterDeclaration]>,
        shape_values: &[u64],
    ) -> TestSchema {
        let schema = SchemaDraft {
            dimension_parameters: dimensions,
            body,
        }
        .finalize()
        .unwrap();
        let shape = schema
            .instantiate_shape(shape_values.to_vec().into_boxed_slice())
            .unwrap();
        let mut builder = SchemaTableBuilder::new();
        let handle = builder.insert(schema).unwrap();
        let build = builder.finish().unwrap();
        let id = build.resolve(handle).unwrap();
        TestSchema {
            id,
            shape,
            schemas: Rc::new(build.table),
        }
    }

    #[cfg(feature = "f64")]
    fn f64_schema() -> TestSchema {
        test_schema(
            SchemaBody::FloatingPoint(FloatWidth::W64),
            Vec::new().into_boxed_slice(),
            &[],
        )
    }

    #[cfg(feature = "f64")]
    fn f64_value(schema: &TestSchema, value: f64) -> Value {
        finalize_draft(
            schema.id,
            &schema.shape,
            &schema.schemas,
            ValueDataDraft::F64(crate::snapshot::F64Bits::from_f64(value)),
        )
        .unwrap()
    }

    #[cfg(all(feature = "f64", feature = "matrix"))]
    fn matrix_schema(rows: u64, columns: u64) -> TestSchema {
        let dimensions = [
            DimensionParameterDeclaration {
                id: DimensionParameterId::new(0),
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Activation,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: None,
            },
            DimensionParameterDeclaration {
                id: DimensionParameterId::new(1),
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Activation,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: None,
            },
        ];
        test_schema(
            SchemaBody::Matrix {
                element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                dimensions: vec![
                    DimensionExpr::Parameter(DimensionParameterId::new(0)),
                    DimensionExpr::Parameter(DimensionParameterId::new(1)),
                ]
                .into_boxed_slice(),
            },
            dimensions.into(),
            &[rows, columns],
        )
    }

    #[cfg(feature = "f64")]
    #[test]
    fn cloned_and_separate_cells_have_explicit_identity_and_structural_value_equality() {
        let schema = f64_schema();
        let first = ValueCell::from_ref(
            Ref::new(7.0_f64),
            schema.id,
            schema.shape.clone(),
            schema.schemas.clone(),
        )
        .unwrap();
        let clone = first.clone();
        let separate =
            ValueCell::from_ref(Ref::new(7.0_f64), schema.id, schema.shape, schema.schemas)
                .unwrap();
        let detached = first.detached_clone().unwrap();

        assert!(first.same_logical_cell(&clone));
        assert!(first.same_storage(&clone));
        assert!(first.same_cell(&clone));
        assert!(!first.same_logical_cell(&separate));
        assert!(!first.same_storage(&separate));
        assert!(!first.same_cell(&separate));
        assert!(first.snapshot_eq(&separate).unwrap());
        assert!(!first.same_logical_cell(&detached));
        assert!(!first.same_storage(&detached));
        assert!(first.snapshot_eq(&detached).unwrap());

        let same_identity_detached_storage = ValueCell {
            binding: CellBinding {
                identity: first.binding.identity,
                ..detached.binding.clone()
            },
        };
        assert!(first.same_logical_cell(&same_identity_detached_storage));
        assert!(!first.same_storage(&same_identity_detached_storage));

        let different_identity_shared_storage = ValueCell {
            binding: CellBinding {
                identity: detached.binding.identity,
                ..first.binding.clone()
            },
        };
        assert!(!first.same_logical_cell(&different_identity_shared_storage));
        assert!(first.same_storage(&different_identity_shared_storage));

        for other in [
            &clone,
            &separate,
            &detached,
            &same_identity_detached_storage,
            &different_identity_shared_storage,
        ] {
            assert_eq!(first.same_cell(other), first.same_storage(other));
        }
    }

    fn assert_replace_borrow_conflict_preserves_value<T>(reference: Ref<T>, schema: TestSchema)
    where
        T: CanonicalCellBacking,
    {
        let cell = ValueCell::from_ref(
            reference.clone(),
            schema.id,
            schema.shape.clone(),
            schema.schemas.clone(),
        )
        .unwrap();
        let before = cell.detached_clone().unwrap();
        let replacement = before.snapshot().unwrap();
        let held = reference.borrow_mut();
        assert!(cell.replace(&replacement).is_err());
        drop(held);
        assert!(cell.snapshot_eq(&before).unwrap());
        assert!(
            cell.storage_capabilities()
                .publication
                .preserves_previous_on_failure
        );
    }

    #[test]
    fn declared_atomic_publication_preserves_representative_backings_on_borrow_conflict() {
        #[cfg(feature = "f64")]
        {
            let scalar_schema = f64_schema();
            assert_replace_borrow_conflict_preserves_value(Ref::new(1.25_f64), scalar_schema);

            let canonical_schema = f64_schema();
            let canonical = f64_value(&canonical_schema, 1.25);
            assert_replace_borrow_conflict_preserves_value(Ref::new(canonical), canonical_schema);
        }

        #[cfg(feature = "string")]
        assert_replace_borrow_conflict_preserves_value(
            Ref::new("before".to_owned()),
            test_schema(SchemaBody::String, Box::new([]), &[]),
        );

        #[cfg(all(feature = "f64", feature = "matrixd"))]
        assert_replace_borrow_conflict_preserves_value(
            Ref::new(crate::DMatrix::<f64>::zeros(2, 3)),
            matrix_schema(2, 3),
        );

        #[cfg(all(feature = "f64", feature = "matrix2"))]
        assert_replace_borrow_conflict_preserves_value(
            Ref::new(crate::Matrix2::<f64>::zeros()),
            matrix_schema(2, 2),
        );
    }

    #[cfg(feature = "f64")]
    #[test]
    fn exact_scalar_snapshot_and_replacement_preserve_the_original_ref() {
        let schema = f64_schema();
        let reference = Ref::new(1.25_f64);
        let alias = reference.clone();
        let cell = ValueCell::from_ref(
            reference,
            schema.id,
            schema.shape.clone(),
            schema.schemas.clone(),
        )
        .unwrap();

        assert!(matches!(
            cell.snapshot().unwrap().data(),
            ValueData::F64(value) if value.to_f64() == 1.25
        ));
        cell.replace(&f64_value(&schema, 9.5)).unwrap();
        assert_eq!(*alias.borrow(), 9.5);
        assert!(cell.try_ref::<f64>().unwrap().same_handle(&alias));
    }

    #[cfg(all(feature = "f64", feature = "matrix2", feature = "matrixd"))]
    #[test]
    fn fixed_and_dynamic_matrix_snapshots_are_row_major_and_keep_exact_handles() {
        let fixed_schema = matrix_schema(2, 2);
        let fixed = Ref::new(crate::Matrix2::new(1.0, 2.0, 3.0, 4.0));
        let fixed_cell = ValueCell::from_ref(
            fixed.clone(),
            fixed_schema.id,
            fixed_schema.shape,
            fixed_schema.schemas,
        )
        .unwrap();
        let ValueData::Matrix(fixed_value) = fixed_cell.snapshot().unwrap().data().clone() else {
            panic!("fixed matrix snapshot")
        };
        assert!(matches!(
            fixed_value.elements(),
            SequenceView::F64(values)
                if values.iter().map(|value| value.to_f64()).collect::<Vec<_>>()
                    == vec![1.0, 2.0, 3.0, 4.0]
        ));
        assert!(
            fixed_cell
                .try_ref::<crate::Matrix2<f64>>()
                .unwrap()
                .same_handle(&fixed)
        );

        let dynamic_schema = matrix_schema(2, 3);
        let dynamic = Ref::new(crate::DMatrix::from_row_slice(
            2,
            3,
            &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
        ));
        let dynamic_cell = ValueCell::from_ref(
            dynamic.clone(),
            dynamic_schema.id,
            dynamic_schema.shape,
            dynamic_schema.schemas,
        )
        .unwrap();
        assert!(matches!(
            dynamic_cell.snapshot().unwrap().data(),
            ValueData::Matrix(matrix)
                if matches!(matrix.elements(), SequenceView::F64(values) if values.len() == 6)
        ));
        assert!(
            dynamic_cell
                .try_ref::<crate::DMatrix<f64>>()
                .unwrap()
                .same_handle(&dynamic)
        );
    }

    #[cfg(all(feature = "complex", feature = "rational", feature = "matrixd"))]
    #[test]
    fn canonical_complex_and_rational_matrices_recover_exact_dynamic_backings() {
        let complex = ValueCell::dynamic_matrix(
            SchemaBody::Complex(crate::FloatWidth::W64),
            vec![2, 2].into_boxed_slice(),
            (1..=4)
                .map(|value| {
                    ValueDataDraft::Complex64(crate::snapshot::Complex64Bits::new(
                        crate::snapshot::F64Bits::from_f64(f64::from(value)),
                        crate::snapshot::F64Bits::from_f64(0.0),
                    ))
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        )
        .unwrap();
        assert!(
            complex.try_ref::<crate::DMatrix<crate::C64>>().is_ok(),
            "canonical c64 matrices must retain an exact planning/runtime backing",
        );

        let rational = ValueCell::dynamic_matrix(
            SchemaBody::Rational64,
            vec![2, 2].into_boxed_slice(),
            (1..=4)
                .map(|numerator| ValueDataDraft::Rational64 {
                    numerator,
                    denominator: 1,
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        )
        .unwrap();
        assert!(
            rational.try_ref::<crate::DMatrix<crate::R64>>().is_ok(),
            "canonical r64 matrices must retain an exact planning/runtime backing",
        );
    }

    #[cfg(all(feature = "f64", feature = "matrix2", feature = "matrixd"))]
    #[test]
    fn declared_output_representations_construct_exact_canonical_backings() {
        let scalar =
            ValueCell::default_for_representation(FunctionValueRepresentation::F64, None).unwrap();
        assert_eq!(*scalar.try_ref::<f64>().unwrap().borrow(), 0.0);

        let fixed = ValueCell::default_for_representation(
            <crate::Matrix2<f64> as FunctionRuntimeType>::REPRESENTATION,
            Some((2, 2)),
        )
        .unwrap();
        assert_eq!(
            *fixed.try_ref::<crate::Matrix2<f64>>().unwrap().borrow(),
            crate::Matrix2::zeros(),
        );

        let dynamic = ValueCell::default_for_representation(
            <crate::DMatrix<f64> as FunctionRuntimeType>::REPRESENTATION,
            Some((2, 3)),
        )
        .unwrap();
        assert_eq!(
            dynamic
                .try_ref::<crate::DMatrix<f64>>()
                .unwrap()
                .borrow()
                .shape(),
            (2, 3),
        );

        let error = ValueCell::default_for_representation(
            <crate::Matrix2<f64> as FunctionRuntimeType>::REPRESENTATION,
            Some((2, 3)),
        )
        .unwrap_err();
        assert_eq!(error.kind_name(), "ValueCellOutputConstructionUnsupported");
    }

    #[cfg(all(feature = "f64", feature = "functions", feature = "vectord"))]
    #[test]
    fn exact_matrix_values_infer_their_canonical_extents() {
        let cell = ValueCell::from_exact(crate::DVector::from_vec(vec![1.0_f64, 2.0, 3.0]))
            .expect("a dynamic vector provides its own matrix extents");

        assert_eq!(
            cell.representation(),
            <crate::DVector<f64> as FunctionRuntimeType>::REPRESENTATION,
        );
        assert_eq!(
            cell.try_ref::<crate::DVector<f64>>()
                .unwrap()
                .borrow()
                .as_slice(),
            &[1.0, 2.0, 3.0],
        );
        assert_eq!(cell.snapshot().unwrap().shape().parameter_values(), &[3, 1]);
    }

    #[cfg(feature = "f64")]
    #[test]
    fn canonical_value_cells_snapshot_and_replace_without_changing_identity() {
        let schema = f64_schema();
        let original = f64_value(&schema, 2.0);
        let cell = ValueCell::from_value(original.clone(), schema.schemas.clone()).unwrap();
        let alias = cell.clone();
        let expected = ValueCell::from_value(original, schema.schemas.clone()).unwrap();

        assert!(cell.snapshot_eq(&expected).unwrap());
        cell.replace(&f64_value(&schema, 3.0)).unwrap();
        assert!(cell.same_cell(&alias));
        assert!(matches!(
            alias.snapshot().unwrap().data(),
            ValueData::F64(value) if value.to_f64() == 3.0
        ));
    }

    #[cfg(feature = "f64")]
    #[test]
    fn value_cell_key_equality_uses_canonical_float_keys() {
        let left = ValueCell::from_schema_data(
            SchemaBody::FloatingPoint(FloatWidth::W64),
            ValueDataDraft::F64(crate::snapshot::F64Bits::from_bits(0x7ff0_0000_0000_0001)),
        )
        .unwrap();
        let right = ValueCell::from_schema_data(
            SchemaBody::FloatingPoint(FloatWidth::W64),
            ValueDataDraft::F64(crate::snapshot::F64Bits::from_bits(0xfff8_0000_0000_0042)),
        )
        .unwrap();
        assert!(!left.snapshot_eq(&right).unwrap());
        assert!(left.key_eq(&right).unwrap());

        let negative_zero = ValueCell::from_schema_data(
            SchemaBody::FloatingPoint(FloatWidth::W64),
            ValueDataDraft::F64(crate::snapshot::F64Bits::from_f64(-0.0)),
        )
        .unwrap();
        let positive_zero = ValueCell::from_schema_data(
            SchemaBody::FloatingPoint(FloatWidth::W64),
            ValueDataDraft::F64(crate::snapshot::F64Bits::from_f64(0.0)),
        )
        .unwrap();
        assert!(negative_zero.key_eq(&positive_zero).unwrap());
    }

    #[cfg(feature = "string")]
    #[test]
    fn canonical_values_rebind_by_schema_definition_across_reordered_tables() {
        let source = test_schema(SchemaBody::String, Vec::new().into_boxed_slice(), &[]);
        let value = finalize_draft(
            source.id,
            &source.shape,
            &source.schemas,
            ValueDataDraft::String("shared".into()),
        )
        .unwrap();
        let replacement = finalize_draft(
            source.id,
            &source.shape,
            &source.schemas,
            ValueDataDraft::String("replacement".into()),
        )
        .unwrap();

        let mut target = SchemaTableBuilder::new();
        target
            .insert(
                SchemaDraft {
                    dimension_parameters: Vec::new().into_boxed_slice(),
                    body: SchemaBody::Bool,
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        target
            .insert(
                SchemaDraft {
                    dimension_parameters: Vec::new().into_boxed_slice(),
                    body: SchemaBody::String,
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let target = Rc::new(target.finish().unwrap().table);
        let target_schema = target.find_by_key(value.schema_key()).unwrap();
        assert_ne!(source.id, target_schema);

        let cell = ValueCell::from_value(value, target.clone()).unwrap();
        assert_eq!(cell.schema(), target_schema);
        assert!(matches!(
            cell.snapshot().unwrap().data(),
            ValueData::String(value) if value.as_ref() == "shared"
        ));

        cell.replace(&replacement).unwrap();
        assert_eq!(cell.schema(), target_schema);
        assert!(matches!(
            cell.snapshot().unwrap().data(),
            ValueData::String(value) if value.as_ref() == "replacement"
        ));
    }

    #[cfg(all(feature = "f64", feature = "string"))]
    #[test]
    fn dynamic_table_and_record_children_rebind_colliding_local_schema_ids() {
        let text = ValueCell::from_exact("none".to_owned()).unwrap();
        let number = ValueCell::from_exact(4.0_f64).unwrap();
        assert_eq!(text.schema(), number.schema());
        assert_ne!(text.schema_key(), number.schema_key());

        let columns = vec![(
            crate::SchemaField {
                name: "value".into(),
                schema: SchemaBody::Dynamic,
            },
            vec![text.clone(), number.clone()].into_boxed_slice(),
        )]
        .into_boxed_slice();
        let table = ValueCell::table_from_cell_columns(
            columns,
            CardinalitySpec::Exact(DimensionExpr::Constant(2)),
        )
        .unwrap();
        let record = ValueCell::record_from_cells(&[("table".into(), table.clone())]).unwrap();

        let ValueData::Record(record) = record.snapshot().unwrap().data().clone() else {
            panic!("record data")
        };
        let ValueData::Table(table_value) = &record.fields()[0] else {
            panic!("table field")
        };
        let SequenceView::Values(values) = table_value.column(0).unwrap() else {
            panic!("dynamic table column")
        };
        assert!(matches!(
            values[0],
            ValueData::Dynamic(ref value)
                if matches!(value.value().unwrap().data(), ValueData::String(text) if text.as_ref() == "none")
        ));
        assert!(matches!(
            values[1],
            ValueData::Dynamic(ref value)
                if matches!(value.value().unwrap().data(), ValueData::F64(number) if number.to_f64() == 4.0)
        ));

        *text.try_ref::<String>().unwrap().borrow_mut() = "changed".into();
        let replacement = table
            .rebuild_table_cell_columns(&[("value".into(), vec![text, number].into_boxed_slice())])
            .unwrap();
        let alias = table.clone();
        table.replace(&replacement).unwrap();
        assert!(table.same_cell(&alias));
        assert!(matches!(
            table.snapshot().unwrap().data(),
            ValueData::Table(value)
                if matches!(value.column(0), Some(SequenceView::Values(values))
                    if matches!(&values[0], ValueData::Dynamic(value)
                        if matches!(value.value().unwrap().data(), ValueData::String(text) if text.as_ref() == "changed")))
        ));

        let reversed = ValueCell::table_from_cell_columns(
            vec![(
                crate::SchemaField {
                    name: "value".into(),
                    schema: SchemaBody::Dynamic,
                },
                vec![
                    ValueCell::from_exact(4.0_f64).unwrap(),
                    ValueCell::from_exact("none".to_owned()).unwrap(),
                ]
                .into_boxed_slice(),
            )]
            .into_boxed_slice(),
            CardinalitySpec::Exact(DimensionExpr::Constant(2)),
        )
        .unwrap();
        assert!(reversed.snapshot().is_ok());
    }

    #[cfg(all(feature = "f64", feature = "matrixd"))]
    #[test]
    fn concrete_matrix_shapes_rebind_between_dynamic_and_fixed_schema_definitions() {
        let source = ValueCell::default_for_representation(
            <crate::DMatrix<f64> as FunctionRuntimeType>::REPRESENTATION,
            Some((4, 1)),
        )
        .unwrap()
        .snapshot()
        .unwrap();
        let target = test_schema(
            SchemaBody::Matrix {
                element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                dimensions: vec![DimensionExpr::Constant(4), DimensionExpr::Constant(1)]
                    .into_boxed_slice(),
            },
            Vec::new().into_boxed_slice(),
            &[],
        );

        let rebound = source
            .rebind(target.id, &target.shape, &target.schemas)
            .unwrap();
        assert_eq!(rebound.schema(), target.id);
        assert_eq!(
            rebound.schema_key(),
            target.schemas.entry(target.id).unwrap().key()
        );
        assert!(matches!(
            rebound.data(),
            ValueData::Matrix(matrix) if matrix.elements().len() == 4
        ));
    }

    #[cfg(all(feature = "u8", feature = "string"))]
    #[test]
    fn dynamic_table_and_map_extents_preserve_cell_identity_across_turns() {
        use crate::snapshot::{MapEntryDraft, TableColumnDraft};

        let table = ValueCell::empty_dynamic_table(
            vec![crate::SchemaField {
                name: "value".into(),
                schema: SchemaBody::UnsignedInteger(IntegerWidth::W8),
            }]
            .into_boxed_slice(),
        )
        .unwrap();
        let table_alias = table.clone();
        for values in [vec![1_u8, 2, 3], vec![9], vec![]] {
            let replacement = ValueCell::from_schema_data(
                table.closed_schema_body().unwrap(),
                ValueDataDraft::Table(
                    vec![TableColumnDraft {
                        name: "value".into(),
                        values: values
                            .into_iter()
                            .map(ValueDataDraft::U8)
                            .collect::<Vec<_>>()
                            .into_boxed_slice(),
                    }]
                    .into_boxed_slice(),
                ),
            )
            .unwrap()
            .snapshot()
            .unwrap();
            table.replace(&replacement).unwrap();
            assert!(table.same_cell(&table_alias));
        }

        let map = ValueCell::empty_dynamic_map(
            SchemaBody::UnsignedInteger(IntegerWidth::W8),
            SchemaBody::String,
        )
        .unwrap();
        let map_alias = map.clone();
        for entries in [vec![(1_u8, "one"), (2, "two")], vec![(2, "two")], vec![]] {
            let replacement = ValueCell::from_schema_data(
                map.closed_schema_body().unwrap(),
                ValueDataDraft::Map(
                    entries
                        .into_iter()
                        .map(|(key, value)| MapEntryDraft {
                            items: vec![
                                ValueDataDraft::U8(key),
                                ValueDataDraft::String(value.into()),
                            ]
                            .into_boxed_slice(),
                        })
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                ),
            )
            .unwrap()
            .snapshot()
            .unwrap();
            map.replace(&replacement).unwrap();
            assert!(map.same_cell(&map_alias));
        }
    }

    #[cfg(feature = "f64")]
    #[test]
    fn turn_scoped_matrix_extents_change_without_replacing_the_cell() {
        let cell = ValueCell::dynamic_matrix(
            SchemaBody::FloatingPoint(FloatWidth::W64),
            vec![0, 0].into_boxed_slice(),
            Box::new([]),
        )
        .unwrap();
        let alias = cell.clone();

        for (dimensions, values) in [
            (vec![1, 3], vec![1.0, 2.0, 3.0]),
            (vec![2, 1], vec![4.0, 5.0]),
            (vec![0, 0], vec![]),
        ] {
            let next = cell
                .rebuild_matrix_drafts(
                    dimensions.clone().into_boxed_slice(),
                    values
                        .into_iter()
                        .map(|value| ValueDataDraft::F64(crate::snapshot::F64Bits::from_f64(value)))
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                )
                .unwrap();
            cell.replace(&next).unwrap();
            assert!(cell.same_cell(&alias));
            assert_eq!(cell.shape().parameter_values(), dimensions.as_slice());
        }
    }

    #[cfg(feature = "f64")]
    #[test]
    fn detached_dynamic_cells_have_independent_payload_and_shape_state() {
        let source = ValueCell::dynamic_matrix(
            SchemaBody::FloatingPoint(FloatWidth::W64),
            vec![1, 1].into_boxed_slice(),
            vec![ValueDataDraft::F64(crate::snapshot::F64Bits::from_f64(1.0))].into_boxed_slice(),
        )
        .unwrap();
        let before = source.detached_clone().unwrap();
        let detached = source.detached_clone().unwrap();
        let replacement = detached
            .rebuild_matrix_drafts(
                vec![1, 2].into_boxed_slice(),
                vec![
                    ValueDataDraft::F64(crate::snapshot::F64Bits::from_f64(2.0)),
                    ValueDataDraft::F64(crate::snapshot::F64Bits::from_f64(3.0)),
                ]
                .into_boxed_slice(),
            )
            .unwrap();

        detached.replace(&replacement).unwrap();

        assert_eq!(source.shape().parameter_values(), &[1, 1]);
        assert_eq!(detached.shape().parameter_values(), &[1, 2]);
        assert!(source.snapshot_eq(&before).unwrap());
        assert!(!source.same_cell(&detached));
    }

    #[cfg(feature = "f64")]
    #[test]
    fn staged_dynamic_replacement_changes_shape_only_when_committed() {
        use crate::ReactiveRegisterCommit;

        let sink = ValueCell::dynamic_matrix(
            SchemaBody::FloatingPoint(FloatWidth::W64),
            vec![1, 1].into_boxed_slice(),
            vec![ValueDataDraft::F64(crate::snapshot::F64Bits::from_f64(1.0))].into_boxed_slice(),
        )
        .unwrap();
        let alias = sink.clone();
        let before = sink.detached_clone().unwrap();
        let replacement = sink
            .rebuild_matrix_drafts(
                vec![1, 2].into_boxed_slice(),
                vec![
                    ValueDataDraft::F64(crate::snapshot::F64Bits::from_f64(2.0)),
                    ValueDataDraft::F64(crate::snapshot::F64Bits::from_f64(3.0)),
                ]
                .into_boxed_slice(),
            )
            .unwrap();
        let expected = sink.detached_clone().unwrap();
        expected.replace(&replacement).unwrap();

        let abandoned =
            crate::ReactiveValueCellWrite::new(sink.clone(), replacement.clone()).unwrap();
        assert_eq!(sink.shape().parameter_values(), &[1, 1]);
        assert!(sink.snapshot_eq(&before).unwrap());
        drop(abandoned);
        assert_eq!(sink.shape().parameter_values(), &[1, 1]);
        assert!(sink.snapshot_eq(&before).unwrap());

        let committed =
            crate::ReactiveValueCellWrite::new(sink.clone(), replacement.clone()).unwrap();
        assert_eq!(sink.shape().parameter_values(), &[1, 1]);
        assert!(sink.snapshot_eq(&before).unwrap());
        Box::new(committed).commit();

        assert!(sink.same_cell(&alias));
        assert_eq!(sink.shape().parameter_values(), &[1, 2]);
        assert!(sink.snapshot_eq(&expected).unwrap());
    }

    #[cfg(feature = "f64")]
    #[test]
    fn held_shape_borrows_make_direct_and_staged_replacement_atomic() {
        let cell = ValueCell::dynamic_matrix(
            SchemaBody::FloatingPoint(FloatWidth::W64),
            vec![1, 1].into_boxed_slice(),
            vec![ValueDataDraft::F64(crate::snapshot::F64Bits::from_f64(1.0))].into_boxed_slice(),
        )
        .unwrap();
        let replacement = cell
            .rebuild_matrix_drafts(
                vec![1, 2].into_boxed_slice(),
                vec![
                    ValueDataDraft::F64(crate::snapshot::F64Bits::from_f64(2.0)),
                    ValueDataDraft::F64(crate::snapshot::F64Bits::from_f64(3.0)),
                ]
                .into_boxed_slice(),
            )
            .unwrap();
        let before = cell.detached_clone().unwrap();

        let shape = cell.shape();
        let direct = cell.replace(&replacement).unwrap_err();
        assert_eq!(direct.kind_name(), "ValueCellBorrowConflict");
        assert_eq!(shape.parameter_values(), &[1, 1]);
        assert!(cell.snapshot_eq(&before).unwrap());
        drop(shape);

        let shape = cell.shape();
        let staged = match crate::ReactiveValueCellWrite::new(cell.clone(), replacement) {
            Ok(_) => panic!("staging must reject a held shape borrow"),
            Err(error) => error,
        };
        assert_eq!(staged.kind_name(), "ValueCellBorrowConflict");
        assert_eq!(shape.parameter_values(), &[1, 1]);
        assert!(cell.snapshot_eq(&before).unwrap());
    }

    #[cfg(all(feature = "f64", feature = "matrixd"))]
    #[test]
    fn exact_dynamic_matrix_backing_resizes_without_replacing_its_handle() {
        let backing = Ref::new(crate::DMatrix::<f64>::zeros(0, 0));
        let alias = backing.clone();
        let cell = ValueCell::from_inferred_ref(backing, Some((0, 0))).unwrap();
        let schema_key = cell.schema_key();

        for (dimensions, values) in [
            (vec![2, 2], vec![1.0, 2.0, 3.0, 4.0]),
            (vec![1, 3], vec![5.0, 6.0, 7.0]),
            (vec![0, 0], vec![]),
        ] {
            let next = cell
                .rebuild_matrix_drafts(
                    dimensions.clone().into_boxed_slice(),
                    values
                        .iter()
                        .copied()
                        .map(|value| ValueDataDraft::F64(crate::snapshot::F64Bits::from_f64(value)))
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                )
                .unwrap();
            cell.replace(&next).unwrap();

            assert_eq!(cell.schema_key(), schema_key);
            assert_eq!(cell.shape().parameter_values(), dimensions.as_slice());
            assert!(
                cell.try_ref::<crate::DMatrix<f64>>()
                    .unwrap()
                    .same_handle(&alias)
            );
            assert_eq!(
                *alias.borrow(),
                crate::DMatrix::from_row_slice(
                    dimensions[0] as usize,
                    dimensions[1] as usize,
                    &values,
                )
            );
        }
    }

    #[cfg(feature = "u8")]
    #[test]
    fn exact_and_bounded_dynamic_collection_extents_are_enforced() {
        let values = |count: u8| {
            (0..count)
                .map(ValueDataDraft::U8)
                .collect::<Vec<_>>()
                .into_boxed_slice()
        };
        let exact = ValueCell::from_schema_data(
            SchemaBody::Set {
                element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
                cardinality: CardinalitySpec::Exact(DimensionExpr::Constant(2)),
            },
            ValueDataDraft::Set(values(1)),
        )
        .unwrap_err();
        assert!(exact.kind_message().contains("Cardinality"), "{exact:?}");

        let bounded = ValueCell::from_schema_data(
            SchemaBody::Set {
                element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
                cardinality: CardinalitySpec::Dynamic {
                    upper_bound: Some(DimensionExpr::Constant(2)),
                },
            },
            ValueDataDraft::Set(values(3)),
        )
        .unwrap_err();
        assert!(
            bounded.kind_message().contains("Cardinality"),
            "{bounded:?}"
        );
    }

    #[cfg(feature = "u8")]
    #[test]
    fn exact_and_dynamic_collection_schemas_are_semantically_distinct() {
        let element = Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8));
        let exact = ValueCell::from_schema_data(
            SchemaBody::Set {
                element: element.clone(),
                cardinality: CardinalitySpec::Exact(DimensionExpr::Constant(0)),
            },
            ValueDataDraft::Set(Box::new([])),
        )
        .unwrap();
        let dynamic = ValueCell::from_schema_data(
            SchemaBody::Set {
                element,
                cardinality: CardinalitySpec::Dynamic { upper_bound: None },
            },
            ValueDataDraft::Set(Box::new([])),
        )
        .unwrap();

        assert_ne!(exact.schema_key(), dynamic.schema_key());
        assert!(!exact.snapshot_eq(&dynamic).unwrap());
    }

    #[cfg(feature = "u8")]
    #[test]
    fn exact_collection_values_rebind_into_compatible_dynamic_extent_schemas() {
        let exact = ValueCell::from_schema_data(
            SchemaBody::Set {
                element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
                cardinality: CardinalitySpec::Exact(DimensionExpr::Constant(2)),
            },
            ValueDataDraft::Set(
                vec![ValueDataDraft::U8(1), ValueDataDraft::U8(2)].into_boxed_slice(),
            ),
        )
        .unwrap()
        .snapshot()
        .unwrap();
        let target = test_schema(
            SchemaBody::Set {
                element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
                cardinality: CardinalitySpec::Dynamic {
                    upper_bound: Some(DimensionExpr::Constant(3)),
                },
            },
            Vec::new().into_boxed_slice(),
            &[],
        );

        let rebound = exact
            .rebind(target.id, &target.shape, &target.schemas)
            .unwrap();
        assert_eq!(rebound.schema(), target.id);
        assert_eq!(
            rebound.schema_key(),
            target.schemas.entry(target.id).unwrap().key()
        );
        assert!(matches!(
            rebound.data(),
            ValueData::Set(elements) if elements.elements().len() == 2
        ));

        let exact_map = ValueCell::from_schema_data(
            SchemaBody::Map {
                key: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
                value: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
                cardinality: CardinalitySpec::Exact(DimensionExpr::Constant(1)),
            },
            ValueDataDraft::Map(
                vec![crate::snapshot::MapEntryDraft {
                    items: vec![ValueDataDraft::U8(1), ValueDataDraft::U8(2)].into_boxed_slice(),
                }]
                .into_boxed_slice(),
            ),
        )
        .unwrap()
        .snapshot()
        .unwrap();
        let map_target = test_schema(
            SchemaBody::Map {
                key: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
                value: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
                cardinality: CardinalitySpec::Dynamic {
                    upper_bound: Some(DimensionExpr::Constant(2)),
                },
            },
            Vec::new().into_boxed_slice(),
            &[],
        );
        let rebound_map = exact_map
            .rebind(map_target.id, &map_target.shape, &map_target.schemas)
            .unwrap();
        assert!(matches!(
            rebound_map.data(),
            ValueData::Map(entries) if entries.entries().len() == 1
        ));
    }

    #[cfg(feature = "f64")]
    #[test]
    fn replacement_checks_schema_and_shape_before_mutating() {
        let scalar_schema = f64_schema();
        let scalar = Ref::new(1.0_f64);
        let cell = ValueCell::from_ref(
            scalar.clone(),
            scalar_schema.id,
            scalar_schema.shape.clone(),
            scalar_schema.schemas.clone(),
        )
        .unwrap();
        let index_schema = test_schema(SchemaBody::Index, Vec::new().into_boxed_slice(), &[]);
        let index = finalize_draft(
            index_schema.id,
            &index_schema.shape,
            &index_schema.schemas,
            ValueDataDraft::Index(2),
        )
        .unwrap();
        assert!(
            cell.replace(&index)
                .unwrap_err()
                .kind_as::<ValueCellSchemaMismatch>()
                .is_some()
        );
        assert_eq!(*scalar.borrow(), 1.0);
    }

    #[cfg(all(feature = "f64", feature = "matrixd"))]
    #[test]
    fn replacement_rejects_a_different_shape_without_resizing_the_backing() {
        let two_by_two = matrix_schema(2, 2);
        let backing = Ref::new(crate::DMatrix::from_element(2, 2, 1.0));
        let cell = ValueCell::from_ref(
            backing.clone(),
            two_by_two.id,
            two_by_two.shape,
            two_by_two.schemas.clone(),
        )
        .unwrap();
        let replacement = ValueDraft {
            schema: two_by_two.id,
            shape_values: vec![1, 4].into_boxed_slice(),
            data: ValueDataDraft::Matrix(
                vec![ValueDataDraft::F64(crate::snapshot::F64Bits::from_f64(2.0)); 4]
                    .into_boxed_slice(),
            ),
        }
        .finalize(&SnapshotValidationContext::new(&two_by_two.schemas))
        .unwrap();

        assert!(
            cell.replace(&replacement)
                .unwrap_err()
                .kind_as::<ValueCellShapeMismatch>()
                .is_some()
        );
        assert_eq!(backing.borrow().shape(), (2, 2));
    }

    #[cfg(feature = "f64")]
    #[test]
    fn borrow_conflicts_are_structured_and_debug_never_exposes_payload_or_address() {
        let schema = f64_schema();
        let backing = Ref::new(12345.625_f64);
        let cell =
            ValueCell::from_ref(backing.clone(), schema.id, schema.shape, schema.schemas).unwrap();
        let available = format!("{cell:?}");
        assert!(!available.contains("12345.625"));
        assert!(!available.contains("0x"));

        let _borrow = backing.borrow_mut();
        let error = cell.snapshot().unwrap_err();
        assert_eq!(
            error.kind_as::<ValueCellBorrowConflict>().unwrap().access,
            CellAccess::Snapshot
        );
        let borrowed = format!("{cell:?}");
        assert!(borrowed.contains("Borrowed"));
        assert!(!borrowed.contains("12345.625"));
        assert!(!borrowed.contains("0x"));
    }
}
