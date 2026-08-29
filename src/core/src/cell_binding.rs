//! Schema-aware mutable program cells.
//!
//! A [`ValueCell`] owns identity and schema metadata while its exact typed
//! backing remains private. Canonical snapshots are the only universal value
//! representation exposed by this module.

use crate::{
    FloatWidth, FunctionMatrixElement, FunctionMatrixStoragePattern, FunctionRuntimeType,
    FunctionValueRepresentation, IntegerWidth, MResult, MechError, MechErrorKind, Ref, SchemaBody,
    SchemaId, SchemaKey, SchemaTable, ShapeInstance, SnapshotValueError, Value, ValueData,
    ValueDataDraft, ValueDraft,
};
use core::{any::Any, any::type_name, fmt};

#[cfg(feature = "no_std")]
use core::cell;
#[cfg(not(feature = "no_std"))]
use std::cell;

use crate::LegacyValue;

#[cfg(feature = "matrix")]
use crate::snapshot::SequenceView;
use crate::snapshot::SnapshotValidationContext;

#[cfg(all(feature = "no_std", feature = "string"))]
use alloc::string::ToString;
#[cfg(feature = "no_std")]
use alloc::{boxed::Box, rc::Rc, string::String, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, rc::Rc, string::String, vec::Vec};

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
    }
}

/// An exact typed backing that can safely live behind a canonical value cell.
///
/// This trait is sealed. In particular, the legacy universal value and legacy
/// aggregate containers cannot be used as cell backings.
pub trait CanonicalCellBacking:
    canonical_cell_sealed::Sealed + FunctionRuntimeType + Clone + 'static
{
}

impl<T> CanonicalCellBacking for T where
    T: canonical_cell_sealed::Sealed + FunctionRuntimeType + Clone + 'static
{
}

#[derive(Clone)]
struct CellBinding {
    schema: SchemaId,
    schema_key: SchemaKey,
    shape: ShapeInstance,
    schemas: Rc<SchemaTable>,
    storage: Rc<dyn ErasedCellStorage>,
}

trait ErasedCellStorage {
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
    fn same_storage(&self, other: &dyn ErasedCellStorage) -> bool;
    fn borrow_state(&self) -> CellBorrowState;
}

struct ExactCellStorage<T> {
    reference: Ref<T>,
}

struct LegacyCellStorage {
    reference: Ref<LegacyValue>,
    representation: FunctionValueRepresentation,
}

#[derive(Clone, Copy, Debug)]
enum CellBorrowState {
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

impl ErasedCellStorage for LegacyCellStorage {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn representation(&self, _schema: &SchemaBody) -> FunctionValueRepresentation {
        self.representation
    }

    fn snapshot(
        &self,
        _schema: SchemaId,
        _shape: &ShapeInstance,
        _schemas: &SchemaTable,
    ) -> MResult<Value> {
        Err(backing_mismatch::<LegacyValue>(
            FunctionValueRepresentation::AnyValue,
        ))
    }

    fn replace(&self, _value: &Value) -> MResult<()> {
        Err(backing_mismatch::<LegacyValue>(
            FunctionValueRepresentation::AnyValue,
        ))
    }

    fn preflight_replace(&self) -> MResult<()> {
        self.reference
            .try_borrow_mut()
            .map(|_| ())
            .map_err(|_| borrow_conflict(CellAccess::Replace))
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
    binding: CellBinding,
}

impl ValueCell {
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
        let cell = Self {
            binding: CellBinding {
                schema,
                schema_key,
                shape,
                schemas,
                storage: Rc::new(ExactCellStorage { reference }),
            },
        };
        cell.snapshot()?;
        Ok(cell)
    }

    pub fn from_value(value: Value, schemas: Rc<SchemaTable>) -> Self {
        let schema = value.schema();
        let schema_key = value.schema_key();
        let shape = value.shape().clone();
        debug_assert_eq!(
            schemas.entry(schema).map(|entry| entry.key()),
            Some(schema_key),
            "canonical value must retain its originating schema table"
        );
        Self {
            binding: CellBinding {
                schema,
                schema_key,
                shape,
                schemas,
                storage: Rc::new(ExactCellStorage {
                    reference: Ref::new(value),
                }),
            },
        }
    }

    pub(crate) fn from_inferred_ref<T>(
        reference: Ref<T>,
        matrix_extents: Option<(usize, usize)>,
    ) -> MResult<Self>
    where
        T: CanonicalCellBacking,
    {
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
        Ok(Self::from_value(value, schemas))
    }

    pub const fn schema(&self) -> SchemaId {
        self.binding.schema
    }

    pub const fn schema_key(&self) -> SchemaKey {
        self.binding.schema_key
    }

    pub const fn shape(&self) -> &ShapeInstance {
        &self.binding.shape
    }

    pub fn representation(&self) -> FunctionValueRepresentation {
        let schema = self
            .binding
            .schemas
            .get(self.binding.schema)
            .expect("value-cell schema remains present");
        self.binding.storage.representation(schema.body())
    }

    pub fn snapshot(&self) -> MResult<Value> {
        self.binding.storage.snapshot(
            self.binding.schema,
            &self.binding.shape,
            &self.binding.schemas,
        )
    }

    pub fn replace(&self, value: &Value) -> MResult<()> {
        if value.schema() != self.binding.schema || value.schema_key() != self.binding.schema_key {
            return Err(MechError::new(
                ValueCellSchemaMismatch {
                    expected: self.binding.schema_key,
                    actual: value.schema_key(),
                },
                None,
            )
            .with_compiler_loc());
        }
        if value.shape() != &self.binding.shape {
            return Err(MechError::new(
                ValueCellShapeMismatch {
                    expected: self
                        .binding
                        .shape
                        .parameter_values()
                        .to_vec()
                        .into_boxed_slice(),
                    actual: value.shape().parameter_values().to_vec().into_boxed_slice(),
                },
                None,
            )
            .with_compiler_loc());
        }
        value
            .validate_against(&self.binding.schemas)
            .map_err(snapshot_failure)?;
        self.binding.storage.replace(value)
    }

    pub(crate) fn preflight_replace(&self) -> MResult<()> {
        self.binding.storage.preflight_replace()
    }

    pub fn same_cell(&self, other: &Self) -> bool {
        self.binding
            .storage
            .same_storage(other.binding.storage.as_ref())
    }

    pub(crate) fn try_ref<T: 'static>(&self) -> MResult<Ref<T>> {
        let exact = self
            .binding
            .storage
            .as_any()
            .downcast_ref::<ExactCellStorage<T>>()
            .map(|storage| storage.reference.clone());
        let compatibility = self
            .binding
            .storage
            .as_any()
            .downcast_ref::<LegacyCellStorage>()
            .and_then(|storage| storage.reference.try_borrow().ok())
            .and_then(|value| {
                value
                    .exact_ref_any()
                    .and_then(|reference| reference.downcast_ref::<Ref<T>>())
                    .cloned()
            });
        exact.or(compatibility).ok_or_else(|| {
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

    #[doc(hidden)]
    pub fn new(value: LegacyValue) -> Self {
        Self::from_legacy_ref(Ref::new(value))
    }

    #[doc(hidden)]
    pub fn from_legacy_ref(reference: Ref<LegacyValue>) -> Self {
        let representation = FunctionValueRepresentation::from_value(&reference.borrow());
        let (schema, shape, schemas) = compatibility_unit_schema();
        let schema_key = schemas
            .entry(schema)
            .expect("compatibility unit schema exists")
            .key();
        Self {
            binding: CellBinding {
                schema,
                schema_key,
                shape,
                schemas,
                storage: Rc::new(LegacyCellStorage {
                    reference,
                    representation,
                }),
            },
        }
    }

    #[doc(hidden)]
    pub fn legacy_ref(&self) -> Ref<LegacyValue> {
        self.legacy_ref_compat()
            .expect("legacy compatibility requires a legacy-backed value cell")
    }

    pub(crate) fn legacy_ref_compat(&self) -> Option<Ref<LegacyValue>> {
        self.binding
            .storage
            .as_any()
            .downcast_ref::<LegacyCellStorage>()
            .map(|storage| storage.reference.clone())
    }

    #[doc(hidden)]
    pub fn borrow(&self) -> cell::Ref<'_, LegacyValue> {
        self.binding
            .storage
            .as_any()
            .downcast_ref::<LegacyCellStorage>()
            .expect("legacy compatibility requires a legacy-backed value cell")
            .reference
            .borrow()
    }

    #[doc(hidden)]
    pub fn borrow_mut(&self) -> cell::RefMut<'_, LegacyValue> {
        self.binding
            .storage
            .as_any()
            .downcast_ref::<LegacyCellStorage>()
            .expect("legacy compatibility requires a legacy-backed value cell")
            .reference
            .borrow_mut()
    }

    #[doc(hidden)]
    pub fn try_borrow(&self) -> Result<cell::Ref<'_, LegacyValue>, cell::BorrowError> {
        self.binding
            .storage
            .as_any()
            .downcast_ref::<LegacyCellStorage>()
            .expect("legacy compatibility requires a legacy-backed value cell")
            .reference
            .try_borrow()
    }

    #[doc(hidden)]
    pub fn try_borrow_mut(&self) -> Result<cell::RefMut<'_, LegacyValue>, cell::BorrowMutError> {
        self.binding
            .storage
            .as_any()
            .downcast_ref::<LegacyCellStorage>()
            .expect("legacy compatibility requires a legacy-backed value cell")
            .reference
            .try_borrow_mut()
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

impl MechErrorKind for ValueCellSnapshotFailure {
    fn name(&self) -> &str {
        "ValueCellSnapshotFailure"
    }

    fn message(&self) -> String {
        format!("canonical value cell snapshot failed: {:?}", self.error)
    }
}

fn borrow_conflict(access: CellAccess) -> MechError {
    MechError::new(ValueCellBorrowConflict { access }, None).with_compiler_loc()
}

fn snapshot_failure(error: SnapshotValueError) -> MechError {
    MechError::new(ValueCellSnapshotFailure { error }, None).with_compiler_loc()
}

fn finalize_draft(
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

trait CanonicalScalar: FunctionRuntimeType + Clone + fmt::Debug + PartialEq + 'static {
    fn data_draft(&self) -> ValueDataDraft;
    fn from_data(data: &ValueData) -> Option<Self>;

    #[cfg(feature = "matrix")]
    fn from_sequence(values: SequenceView<'_>, index: usize) -> Option<Self>;
}

macro_rules! scalar_backing {
    ($type:ty, $feature:literal, $draft:ident, $data:ident, $sequence:ident) => {
        #[cfg(feature = $feature)]
        impl CanonicalScalar for $type {
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

scalar_backing!(u8, "u8", U8, U8, U8);
scalar_backing!(u16, "u16", U16, U16, U16);
scalar_backing!(u32, "u32", U32, U32, U32);
scalar_backing!(u64, "u64", U64, U64, U64);
scalar_backing!(u128, "u128", U128, U128, U128);
scalar_backing!(i8, "i8", I8, I8, I8);
scalar_backing!(i16, "i16", I16, I16, I16);
scalar_backing!(i32, "i32", I32, I32, I32);
scalar_backing!(i64, "i64", I64, I64, I64);
scalar_backing!(i128, "i128", I128, I128, I128);
scalar_backing!(bool, "bool", Bool, Bool, Bool);

macro_rules! float_backing {
    ($type:ty, $feature:literal, $draft:ident, $data:ident, $sequence:ident, $bits:ty, $from:ident, $to:ident) => {
        #[cfg(feature = $feature)]
        impl CanonicalScalar for $type {
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
    to_f32
);
float_backing!(
    f64,
    "f64",
    F64,
    F64,
    F64,
    crate::snapshot::F64Bits,
    from_f64,
    to_f64
);

impl CanonicalScalar for usize {
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
impl CanonicalScalar for String {
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
impl CanonicalScalar for crate::C64 {
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
impl CanonicalScalar for crate::R64 {
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
    T: CanonicalScalar,
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
    T: CanonicalScalar,
{
    let ValueData::Matrix(replacement) = value.data() else {
        return Err(backing_mismatch::<T>(T::REPRESENTATION));
    };
    let values = replacement.elements();
    let expected = matrix.rows().saturating_mul(matrix.cols());
    let mut replaced = matrix.clone_matrix();
    for index in 0..expected {
        let element = T::from_sequence(values, index)
            .ok_or_else(|| backing_mismatch::<T>(T::REPRESENTATION))?;
        let row = index / matrix.cols();
        let column = index % matrix.cols();
        replaced.set_element(row, column, element);
    }
    *matrix = replaced;
    Ok(())
}

#[cfg(feature = "matrix")]
trait CanonicalMatrix<T>: Sized {
    fn rows(&self) -> usize;
    fn cols(&self) -> usize;
    fn element(&self, row: usize, column: usize) -> &T;
    fn set_element(&mut self, row: usize, column: usize, value: T);
    fn clone_matrix(&self) -> Self;
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
            T: CanonicalScalar,
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

            fn set_element(&mut self, row: usize, column: usize, value: T) {
                self[(row, column)] = value;
            }

            fn clone_matrix(&self) -> Self {
                self.clone()
            }
        }

        #[cfg(feature = $feature)]
        impl<T> canonical_cell_sealed::Sealed for crate::$type<T>
        where
            T: CanonicalScalar,
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
matrix_backing!(RowDVector, "row_vectord");
#[cfg(feature = "matrix")]
matrix_backing!(Vector2, "vector2");
#[cfg(feature = "matrix")]
matrix_backing!(Vector3, "vector3");
#[cfg(feature = "matrix")]
matrix_backing!(Vector4, "vector4");
#[cfg(feature = "matrix")]
matrix_backing!(DVector, "vectord");
#[cfg(feature = "matrix")]
matrix_backing!(DMatrix, "matrixd");

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

fn compatibility_unit_schema() -> (SchemaId, ShapeInstance, Rc<SchemaTable>) {
    let schema = crate::SchemaDraft {
        dimension_parameters: Vec::new().into_boxed_slice(),
        body: SchemaBody::Tuple(Vec::new().into_boxed_slice()),
    }
    .finalize()
    .expect("compatibility unit schema is valid");
    let shape = schema
        .instantiate_shape(Vec::new().into_boxed_slice())
        .expect("compatibility unit shape is valid");
    let mut builder = crate::SchemaTableBuilder::new();
    let handle = builder
        .insert(schema)
        .expect("compatibility unit schema can be inserted");
    let build = builder
        .finish()
        .expect("compatibility unit schema table is valid");
    let schema = build
        .resolve(handle)
        .expect("compatibility unit schema handle resolves");
    (schema, shape, Rc::new(build.table))
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

        assert!(first.same_cell(&clone));
        assert!(!first.same_cell(&separate));
        assert_eq!(first.snapshot().unwrap(), separate.snapshot().unwrap());
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

    #[cfg(feature = "f64")]
    #[test]
    fn canonical_value_cells_snapshot_and_replace_without_changing_identity() {
        let schema = f64_schema();
        let original = f64_value(&schema, 2.0);
        let cell = ValueCell::from_value(original.clone(), schema.schemas.clone());
        let alias = cell.clone();

        assert_eq!(cell.snapshot().unwrap(), original);
        cell.replace(&f64_value(&schema, 3.0)).unwrap();
        assert!(cell.same_cell(&alias));
        assert!(matches!(
            alias.snapshot().unwrap().data(),
            ValueData::F64(value) if value.to_f64() == 3.0
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
