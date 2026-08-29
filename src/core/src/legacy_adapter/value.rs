use self::LegacySnapshotError::*;
#[cfg(any(feature = "atom", feature = "enum"))]
use super::LegacyNominalResolution;
use super::{LegacySemanticContext, kind_expr_from_legacy, schema_from_legacy_value_kind};
use crate::legacy_value::{LegacyValue, ValueKind};
#[cfg(feature = "complex")]
use crate::snapshot::Complex64Bits;
#[cfg(feature = "enum")]
use crate::snapshot::EnumDraft;
#[cfg(feature = "f32")]
use crate::snapshot::F32Bits;
#[cfg(feature = "f64")]
use crate::snapshot::F64Bits;
#[cfg(feature = "map")]
use crate::snapshot::MapEntryDraft;
#[cfg(feature = "record")]
use crate::snapshot::NamedValueDraft;
#[cfg(all(feature = "matrix", feature = "matrixd"))]
use crate::snapshot::SequenceView;
use crate::snapshot::TableColumnDraft;
use crate::snapshot::{
    OptionDraft, ReifiedType, ReifiedTypeDraft, SnapshotPathSegment, SnapshotValidationContext,
    SnapshotValueError, Value, ValueData, ValueDataDraft, ValueDraft,
};
use crate::{
    DimensionExpr, DimensionParameterDeclaration, DimensionParameterId, DimensionParameterOrigin,
    FloatWidth, IntegerWidth, NominalKey, NominalKind, Schema, SchemaBody, SchemaDraft, SchemaId,
    SchemaKey, SchemaTable, SemanticModelError,
};

#[cfg(all(
    feature = "no_std",
    any(
        feature = "enum",
        feature = "map",
        all(feature = "matrix", feature = "matrixd")
    )
))]
use alloc::vec;
#[cfg(feature = "no_std")]
use alloc::{
    boxed::Box,
    collections::BTreeSet,
    string::{String, ToString},
    vec::Vec,
};
#[cfg(all(
    not(feature = "no_std"),
    any(
        feature = "enum",
        feature = "map",
        all(feature = "matrix", feature = "matrixd")
    )
))]
use std::vec;
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, collections::BTreeSet, string::String, vec::Vec};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LegacyEmptyPolicy {
    Reject,
    ResolveOptionAbsence,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LegacyReferencePolicy {
    Reject,
    SnapshotCurrentValue,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LegacyRepresentation {
    U8,
    U16,
    U32,
    U64,
    U128,
    I8,
    I16,
    I32,
    I64,
    I128,
    F32,
    F64,
    Complex64,
    Rational64,
    String,
    Bool,
    Atom,
    Enum,
    Tuple,
    Record,
    Matrix,
    Table,
    Set,
    Map,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LegacySnapshotError {
    Semantic(SemanticModelError),
    Snapshot(SnapshotValueError),
    LegacyEmptyNotSnapshot,
    InvalidTypedEmptySchema,
    LegacyReferenceCycle,
    LegacyReferenceNotPermitted,
    LegacyBorrowConflict,
    HeterogeneousMatrixUnsupported,
    UnresolvedEmptyMatrixElementSchema,
    LegacySelectionValueRequiresC3,
    LegacyNominalMismatch,
    LegacyTypedSchemaMismatch,
    LegacyIndexOutOfRange,
    LegacyDimensionOutOfRange,
    DynamicSchemaUnavailable {
        key: SchemaKey,
        kind: ValueKind,
    },
    LegacyRationalOutOfRange,
    LegacyRepresentationUnavailable {
        representation: LegacyRepresentation,
    },
    UnsupportedLegacyMaterialization,
}

impl From<SemanticModelError> for LegacySnapshotError {
    fn from(error: SemanticModelError) -> Self {
        Self::Semantic(error)
    }
}

impl From<SnapshotValueError> for LegacySnapshotError {
    fn from(error: SnapshotValueError) -> Self {
        Self::Snapshot(error)
    }
}

#[cfg(feature = "no_std")]
fn borrow_legacy<'a, T>(
    value: &'a crate::Ref<T>,
) -> Result<core::cell::Ref<'a, T>, LegacySnapshotError> {
    value
        .try_borrow()
        .map_err(|_| LegacySnapshotError::LegacyBorrowConflict)
}

#[cfg(not(feature = "no_std"))]
fn borrow_legacy<'a, T>(
    value: &'a crate::Ref<T>,
) -> Result<std::cell::Ref<'a, T>, LegacySnapshotError> {
    value
        .try_borrow()
        .map_err(|_| LegacySnapshotError::LegacyBorrowConflict)
}

#[derive(Clone, Copy)]
struct LegacyTarget<'a> {
    root: &'a Schema,
    body: &'a SchemaBody,
    root_level: bool,
}

impl<'a> LegacyTarget<'a> {
    fn root(schema: &'a Schema) -> Self {
        Self {
            root: schema,
            body: schema.body(),
            root_level: true,
        }
    }

    fn child(&self, body: &'a SchemaBody) -> Self {
        Self {
            root: self.root,
            body,
            root_level: false,
        }
    }

    fn semantic_key(&self) -> Result<SchemaKey, LegacySnapshotError> {
        if self.root_level {
            Ok(self.root.key())
        } else {
            projected_target_schema_key(self.root, self.body)
        }
    }
}

fn projected_target_schema_key(
    root: &Schema,
    body: &SchemaBody,
) -> Result<SchemaKey, LegacySnapshotError> {
    let dimension_parameters = root
        .dimension_parameters()
        .iter()
        .enumerate()
        .map(|(index, parameter)| {
            Ok(DimensionParameterDeclaration {
                id: DimensionParameterId::new(
                    u32::try_from(index).map_err(|_| SemanticModelError::DimensionOverflowV1)?,
                ),
                origin: DimensionParameterOrigin::Explicit,
                lifetime: parameter.lifetime(),
                lower_bound: parameter.lower_bound().clone(),
                upper_bound: parameter.upper_bound().cloned(),
            })
        })
        .collect::<Result<Vec<_>, LegacySnapshotError>>()?
        .into_boxed_slice();
    Ok(SchemaDraft {
        dimension_parameters,
        body: body.clone(),
    }
    .finalize()?
    .key())
}

pub struct LegacySnapshotContext<'a> {
    semantic: &'a mut dyn LegacySemanticContext,
    empty_policy: LegacyEmptyPolicy,
    reference_policy: LegacyReferencePolicy,
    active_addresses: BTreeSet<usize>,
}

impl<'a> LegacySnapshotContext<'a> {
    pub fn new(
        semantic: &'a mut dyn LegacySemanticContext,
        empty_policy: LegacyEmptyPolicy,
        reference_policy: LegacyReferencePolicy,
    ) -> Self {
        Self {
            semantic,
            empty_policy,
            reference_policy,
            active_addresses: BTreeSet::new(),
        }
    }

    pub const fn empty_policy(&self) -> LegacyEmptyPolicy {
        self.empty_policy
    }

    pub const fn reference_policy(&self) -> LegacyReferencePolicy {
        self.reference_policy
    }

    fn with_active<T>(
        &mut self,
        address: usize,
        convert: impl FnOnce(&mut Self) -> Result<T, LegacySnapshotError>,
    ) -> Result<T, LegacySnapshotError> {
        if !self.active_addresses.insert(address) {
            return Err(LegacyReferenceCycle);
        }
        let result = convert(self);
        self.active_addresses.remove(&address);
        result
    }
}

pub trait LegacyMaterializationContext {
    fn resolve_nominal(
        &mut self,
        kind: NominalKind,
        key: NominalKey,
    ) -> Result<(u64, String), LegacySnapshotError>;
}

pub fn snapshot_from_legacy(
    value: &LegacyValue,
    schema: SchemaId,
    shape_values: Box<[u64]>,
    validation: &SnapshotValidationContext<'_>,
    context: &mut LegacySnapshotContext<'_>,
) -> Result<Value, LegacySnapshotError> {
    let target = validation
        .schemas()
        .get(schema)
        .ok_or_else(|| SnapshotValueError::UnknownSnapshotSchema { schema })?;
    let is_typed_empty = matches!(value, LegacyValue::EmptyKind(_));
    #[cfg(feature = "matrix")]
    let is_generic_matrix = matches!(value, LegacyValue::MatrixValue(_));
    #[cfg(not(feature = "matrix"))]
    let is_generic_matrix = false;
    let data = draft_from_legacy(
        value,
        LegacyTarget::root(target),
        &shape_values,
        validation,
        context,
    )?;
    let finalized = ValueDraft {
        schema,
        shape_values,
        data,
    }
    .finalize(validation);
    if let Ok(value) = finalized {
        return Ok(value);
    }

    let error = finalized.err().expect("the successful case returned above");
    if is_typed_empty
        && matches!(
            &error,
            SnapshotValueError::PayloadCardinalityMismatchV1 { .. }
        )
    {
        return Err(InvalidTypedEmptySchema);
    }
    if is_generic_matrix && is_matrix_element_schema_mismatch(&error) {
        return Err(HeterogeneousMatrixUnsupported);
    }
    Err(error.into())
}

fn is_matrix_element_schema_mismatch(error: &SnapshotValueError) -> bool {
    let path = if let SnapshotValueError::SnapshotDataSchemaMismatch { path, .. } = error {
        path
    } else if let SnapshotValueError::AggregateArityMismatchV1 { path, .. } = error {
        path
    } else if let SnapshotValueError::AggregateFieldMismatchV1 { path } = error {
        path
    } else if let SnapshotValueError::PayloadCardinalityMismatchV1 { path, .. } = error {
        path
    } else if let SnapshotValueError::EnumOrdinalOutOfRangeV1 { path, .. } = error {
        path
    } else if let SnapshotValueError::EnumPayloadMismatchV1 { path } = error {
        path
    } else if let SnapshotValueError::MapEntryArityMismatchV1 { path, .. } = error {
        path
    } else if let SnapshotValueError::InvalidIndexV1 { path, .. } = error {
        path
    } else {
        return false;
    };
    matches!(
        path.segments().first(),
        Some(SnapshotPathSegment::MatrixElement(_))
    )
}

fn draft_from_legacy(
    value: &LegacyValue,
    target: LegacyTarget<'_>,
    shape_values: &[u64],
    validation: &SnapshotValidationContext<'_>,
    context: &mut LegacySnapshotContext<'_>,
) -> Result<ValueDataDraft, LegacySnapshotError> {
    if let LegacyValue::Typed(inner, legacy_kind) = value {
        let typed_schema = schema_from_legacy_value_kind(legacy_kind, context.semantic)?;
        if typed_schema.key() != target.semantic_key()? {
            return Err(LegacyTypedSchemaMismatch);
        }
        if let SchemaBody::Option(element) = target.body {
            return Ok(ValueDataDraft::Option(OptionDraft {
                present: true,
                value: Some(Box::new(draft_from_legacy(
                    inner,
                    target.child(element),
                    shape_values,
                    validation,
                    context,
                )?)),
            }));
        }
        return draft_from_legacy(inner, target, shape_values, validation, context);
    }
    if matches!(value, LegacyValue::Empty) {
        if context.empty_policy == LegacyEmptyPolicy::ResolveOptionAbsence
            && matches!(target.body, SchemaBody::Option(_))
        {
            return Ok(ValueDataDraft::Option(OptionDraft {
                present: false,
                value: None,
            }));
        }
        return Err(LegacyEmptyNotSnapshot);
    }
    if let LegacyValue::EmptyKind(legacy_kind) = value {
        let empty_schema = schema_from_legacy_value_kind(legacy_kind, context.semantic)?;
        if empty_schema.key() != target.semantic_key()? {
            return Err(LegacyTypedSchemaMismatch);
        }
        return empty_draft(target.body, shape_values);
    }
    if let LegacyValue::MutableReference(reference) = value {
        if context.reference_policy == LegacyReferencePolicy::Reject {
            return Err(LegacyReferenceNotPermitted);
        }
        return context.with_active(reference.addr(), |context| {
            let borrowed = borrow_legacy(reference)?;
            draft_from_legacy(&borrowed, target, shape_values, validation, context)
        });
    }
    if matches!(value, LegacyValue::IndexAll) {
        return Err(LegacySelectionValueRequiresC3);
    }

    draft_from_legacy_body(value, target, shape_values, validation, context)
}

fn empty_draft(
    target: &SchemaBody,
    shape_values: &[u64],
) -> Result<ValueDataDraft, LegacySnapshotError> {
    if matches!(target, SchemaBody::Dynamic) {
        return Ok(ValueDataDraft::Dynamic(None));
    }
    if matches!(target, SchemaBody::Option(_)) {
        return Ok(ValueDataDraft::Option(OptionDraft {
            present: false,
            value: None,
        }));
    }
    if let SchemaBody::Matrix { dimensions, .. } = target
        && resolved_product(dimensions, shape_values)? == 0
    {
        return Ok(ValueDataDraft::Matrix(Box::new([])));
    }
    if let SchemaBody::Tuple(elements) = target
        && elements.is_empty()
    {
        return Ok(ValueDataDraft::Tuple(Box::new([])));
    }
    if let SchemaBody::Record(fields) = target
        && fields.is_empty()
    {
        return Ok(ValueDataDraft::Record(Box::new([])));
    }
    if let SchemaBody::Table { columns, rows } = target
        && evaluate_dimension(rows, shape_values)? == 0
    {
        return Ok(ValueDataDraft::Table(
            columns
                .iter()
                .map(|column| TableColumnDraft {
                    name: column.name.clone(),
                    values: Box::new([]),
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ));
    }
    if let SchemaBody::Set { cardinality, .. } = target
        && evaluate_dimension(cardinality, shape_values)? == 0
    {
        return Ok(ValueDataDraft::Set(Box::new([])));
    }
    if let SchemaBody::Map { cardinality, .. } = target
        && evaluate_dimension(cardinality, shape_values)? == 0
    {
        return Ok(ValueDataDraft::Map(Box::new([])));
    }
    Err(InvalidTypedEmptySchema)
}

fn draft_from_legacy_body(
    value: &LegacyValue,
    target: LegacyTarget<'_>,
    shape_values: &[u64],
    validation: &SnapshotValidationContext<'_>,
    context: &mut LegacySnapshotContext<'_>,
) -> Result<ValueDataDraft, LegacySnapshotError> {
    if matches!(target.body, SchemaBody::Dynamic) {
        if matches!(value, LegacyValue::EmptyKind(ValueKind::Any)) {
            return Ok(ValueDataDraft::Dynamic(None));
        }
        let concrete_kind = value.kind();
        let concrete_schema = schema_from_legacy_value_kind(&concrete_kind, context.semantic)?;
        let concrete_key = concrete_schema.key();
        let concrete_id =
            validation
                .schemas()
                .find_by_key(concrete_key)
                .ok_or(DynamicSchemaUnavailable {
                    key: concrete_key,
                    kind: concrete_kind,
                })?;
        let concrete = validation
            .schemas()
            .get(concrete_id)
            .expect("resolved dynamic schema remains present");
        let data = draft_from_legacy_body(
            value,
            LegacyTarget::root(concrete),
            &[],
            validation,
            context,
        )?;
        return Ok(ValueDataDraft::Dynamic(Some(Box::new(ValueDraft {
            schema: concrete_id,
            shape_values: Box::new([]),
            data,
        }))));
    }
    if let LegacyValue::Typed(inner, legacy_kind) = value {
        let typed_schema = schema_from_legacy_value_kind(legacy_kind, context.semantic)?;
        if typed_schema.key() != target.semantic_key()? {
            return Err(LegacyTypedSchemaMismatch);
        }
        if let SchemaBody::Option(element) = target.body {
            return Ok(ValueDataDraft::Option(OptionDraft {
                present: true,
                value: Some(Box::new(draft_from_legacy_body(
                    inner,
                    target.child(element),
                    shape_values,
                    validation,
                    context,
                )?)),
            }));
        }
        return draft_from_legacy_body(inner, target, shape_values, validation, context);
    }
    if matches!(value, LegacyValue::Empty) {
        if context.empty_policy == LegacyEmptyPolicy::ResolveOptionAbsence
            && matches!(target.body, SchemaBody::Option(_))
        {
            return Ok(ValueDataDraft::Option(OptionDraft {
                present: false,
                value: None,
            }));
        }
        return Err(LegacyEmptyNotSnapshot);
    }
    if let LegacyValue::EmptyKind(legacy_kind) = value {
        let empty_schema = schema_from_legacy_value_kind(legacy_kind, context.semantic)?;
        if empty_schema.key() != target.semantic_key()? {
            return Err(LegacyTypedSchemaMismatch);
        }
        return empty_draft(target.body, shape_values);
    }
    if let LegacyValue::MutableReference(reference) = value {
        if context.reference_policy == LegacyReferencePolicy::Reject {
            return Err(LegacyReferenceNotPermitted);
        }
        return context.with_active(reference.addr(), |context| {
            let borrowed = borrow_legacy(reference)?;
            draft_from_legacy_body(&borrowed, target, shape_values, validation, context)
        });
    }
    if matches!(value, LegacyValue::IndexAll) {
        return Err(LegacySelectionValueRequiresC3);
    }

    if let SchemaBody::Option(element) = target.body {
        return Ok(ValueDataDraft::Option(OptionDraft {
            present: true,
            value: Some(Box::new(draft_from_legacy_body(
                value,
                target.child(element),
                shape_values,
                validation,
                context,
            )?)),
        }));
    }

    let scalar = match value {
        #[cfg(feature = "u8")]
        LegacyValue::U8(value) => Some(ValueDataDraft::U8(*borrow_legacy(value)?)),
        #[cfg(feature = "u16")]
        LegacyValue::U16(value) => Some(ValueDataDraft::U16(*borrow_legacy(value)?)),
        #[cfg(feature = "u32")]
        LegacyValue::U32(value) => Some(ValueDataDraft::U32(*borrow_legacy(value)?)),
        #[cfg(feature = "u64")]
        LegacyValue::U64(value) => Some(ValueDataDraft::U64(*borrow_legacy(value)?)),
        #[cfg(feature = "u128")]
        LegacyValue::U128(value) => Some(ValueDataDraft::U128(*borrow_legacy(value)?)),
        #[cfg(feature = "i8")]
        LegacyValue::I8(value) => Some(ValueDataDraft::I8(*borrow_legacy(value)?)),
        #[cfg(feature = "i16")]
        LegacyValue::I16(value) => Some(ValueDataDraft::I16(*borrow_legacy(value)?)),
        #[cfg(feature = "i32")]
        LegacyValue::I32(value) => Some(ValueDataDraft::I32(*borrow_legacy(value)?)),
        #[cfg(feature = "i64")]
        LegacyValue::I64(value) => Some(ValueDataDraft::I64(*borrow_legacy(value)?)),
        #[cfg(feature = "i128")]
        LegacyValue::I128(value) => Some(ValueDataDraft::I128(*borrow_legacy(value)?)),
        #[cfg(feature = "f32")]
        LegacyValue::F32(value) => Some(ValueDataDraft::F32(F32Bits::from_f32(*borrow_legacy(
            value,
        )?))),
        #[cfg(feature = "f64")]
        LegacyValue::F64(value) => Some(ValueDataDraft::F64(F64Bits::from_f64(*borrow_legacy(
            value,
        )?))),
        #[cfg(feature = "complex")]
        LegacyValue::C64(value) => {
            let value = borrow_legacy(value)?;
            Some(ValueDataDraft::Complex64(Complex64Bits::new(
                F64Bits::from_f64(value.0.re),
                F64Bits::from_f64(value.0.im),
            )))
        }
        #[cfg(feature = "rational")]
        LegacyValue::R64(value) => {
            let value = borrow_legacy(value)?;
            Some(ValueDataDraft::Rational64 {
                numerator: *value.numer(),
                denominator: u64::try_from(*value.denom()).map_err(|_| LegacyRationalOutOfRange)?,
            })
        }
        #[cfg(any(feature = "string", feature = "variable_define"))]
        LegacyValue::String(value) => Some(ValueDataDraft::String(borrow_legacy(value)?.clone())),
        #[cfg(any(feature = "bool", feature = "variable_define"))]
        LegacyValue::Bool(value) => Some(ValueDataDraft::Bool(*borrow_legacy(value)?)),
        LegacyValue::Id(value) => Some(ValueDataDraft::Id(*value)),
        LegacyValue::Index(value) => Some(ValueDataDraft::Index(
            u64::try_from(*borrow_legacy(value)?).map_err(|_| LegacyIndexOutOfRange)?,
        )),
        #[cfg(feature = "atom")]
        LegacyValue::Atom(_) => None,
        #[cfg(feature = "matrix")]
        LegacyValue::MatrixIndex(_) | LegacyValue::MatrixValue(_) => None,
        LegacyValue::MutableReference(_)
        | LegacyValue::Typed(_, _)
        | LegacyValue::Kind(_)
        | LegacyValue::IndexAll
        | LegacyValue::EmptyKind(_)
        | LegacyValue::Empty => None,
        #[cfg(all(feature = "matrix", feature = "bool"))]
        LegacyValue::MatrixBool(_) => None,
        #[cfg(all(feature = "matrix", feature = "u8"))]
        LegacyValue::MatrixU8(_) => None,
        #[cfg(all(feature = "matrix", feature = "u16"))]
        LegacyValue::MatrixU16(_) => None,
        #[cfg(all(feature = "matrix", feature = "u32"))]
        LegacyValue::MatrixU32(_) => None,
        #[cfg(all(feature = "matrix", feature = "u64"))]
        LegacyValue::MatrixU64(_) => None,
        #[cfg(all(feature = "matrix", feature = "u128"))]
        LegacyValue::MatrixU128(_) => None,
        #[cfg(all(feature = "matrix", feature = "i8"))]
        LegacyValue::MatrixI8(_) => None,
        #[cfg(all(feature = "matrix", feature = "i16"))]
        LegacyValue::MatrixI16(_) => None,
        #[cfg(all(feature = "matrix", feature = "i32"))]
        LegacyValue::MatrixI32(_) => None,
        #[cfg(all(feature = "matrix", feature = "i64"))]
        LegacyValue::MatrixI64(_) => None,
        #[cfg(all(feature = "matrix", feature = "i128"))]
        LegacyValue::MatrixI128(_) => None,
        #[cfg(all(feature = "matrix", feature = "f32"))]
        LegacyValue::MatrixF32(_) => None,
        #[cfg(all(feature = "matrix", feature = "f64"))]
        LegacyValue::MatrixF64(_) => None,
        #[cfg(all(feature = "matrix", feature = "string"))]
        LegacyValue::MatrixString(_) => None,
        #[cfg(all(feature = "matrix", feature = "rational"))]
        LegacyValue::MatrixR64(_) => None,
        #[cfg(all(feature = "matrix", feature = "complex"))]
        LegacyValue::MatrixC64(_) => None,
        #[cfg(feature = "set")]
        LegacyValue::Set(_) => None,
        #[cfg(feature = "map")]
        LegacyValue::Map(_) => None,
        #[cfg(feature = "record")]
        LegacyValue::Record(_) => None,
        #[cfg(feature = "table")]
        LegacyValue::Table(_) => None,
        #[cfg(feature = "tuple")]
        LegacyValue::Tuple(_) => None,
        #[cfg(feature = "enum")]
        LegacyValue::Enum(_) => None,
    };
    if let Some(scalar) = scalar {
        return Ok(scalar);
    }

    #[cfg(feature = "atom")]
    if let (LegacyValue::Atom(atom), SchemaBody::Atom(expected)) = (value, target.body) {
        let atom = borrow_legacy(atom)?;
        let legacy_name = {
            let names = borrow_legacy(&atom.0.1)?;
            names.get(&atom.id()).cloned()
        }
        .unwrap_or_else(|| atom.name());
        let LegacyNominalResolution::Atom { key } =
            context
                .semantic
                .resolve_nominal(NominalKind::Atom, atom.id(), &legacy_name)?
        else {
            return Err(LegacyNominalMismatch);
        };
        if &key != expected {
            return Err(LegacyNominalMismatch);
        }
        return Ok(ValueDataDraft::Atom);
    }

    #[cfg(feature = "enum")]
    if let (LegacyValue::Enum(value), SchemaBody::Enum { key, variants }) = (value, target.body) {
        return context.with_active(value.addr(), |context| {
            let value = borrow_legacy(value)?;
            let enum_name = {
                let names = borrow_legacy(&value.names)?;
                names.get(&value.id).cloned()
            }
            .unwrap_or_else(|| value.id.to_string());
            let LegacyNominalResolution::Enum { key: actual, .. } = context
                .semantic
                .resolve_nominal(NominalKind::Enum, value.id, &enum_name)?
            else {
                return Err(LegacyNominalMismatch);
            };
            if &actual != key || value.variants.len() != 1 {
                return Err(LegacyNominalMismatch);
            }
            let (variant_id, payload) = &value.variants[0];
            let variant_name = borrow_legacy(&value.names)?
                .get(variant_id)
                .cloned()
                .ok_or(LegacyNominalMismatch)?;
            let ordinal = variants
                .iter()
                .position(|variant| variant.name == variant_name)
                .ok_or(LegacyNominalMismatch)?;
            let target_payload = variants[ordinal].payload.as_ref();
            let payload = match (target_payload, payload) {
                (Some(schema), Some(payload)) => Some(Box::new(draft_from_legacy_body(
                    payload,
                    target.child(schema),
                    shape_values,
                    validation,
                    context,
                )?)),
                (None, None) => None,
                (Some(_), None) | (None, Some(_)) => return Err(LegacyNominalMismatch),
            };
            Ok(ValueDataDraft::Enum(EnumDraft {
                ordinal: u32::try_from(ordinal).map_err(|_| LegacyNominalMismatch)?,
                payload,
            }))
        });
    }

    #[cfg(feature = "tuple")]
    if let (LegacyValue::Tuple(value), SchemaBody::Tuple(elements)) = (value, target.body) {
        return context.with_active(value.addr(), |context| {
            let value = borrow_legacy(value)?;
            if value.elements.len() != elements.len() {
                return Err(SnapshotValueError::AggregateArityMismatchV1 {
                    path: crate::snapshot::SnapshotPath::root(),
                    expected: elements.len() as u64,
                    actual: value.elements.len() as u64,
                }
                .into());
            }
            Ok(ValueDataDraft::Tuple(
                elements
                    .iter()
                    .zip(value.elements.iter())
                    .map(|(schema, value)| {
                        draft_from_legacy_body(
                            value,
                            target.child(schema),
                            shape_values,
                            validation,
                            context,
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .into_boxed_slice(),
            ))
        });
    }

    #[cfg(feature = "record")]
    if let (LegacyValue::Record(value), SchemaBody::Record(fields)) = (value, target.body) {
        return context.with_active(value.addr(), |context| {
            let value = borrow_legacy(value)?;
            if value.data.len() != fields.len() {
                return Err(SnapshotValueError::AggregateFieldMismatchV1 {
                    path: crate::snapshot::SnapshotPath::root(),
                }
                .into());
            }
            let mut drafts = Vec::with_capacity(fields.len());
            for field in fields {
                let id = value
                    .field_names
                    .iter()
                    .find_map(|(id, name)| (name == &field.name).then_some(id))
                    .ok_or_else(|| SnapshotValueError::AggregateFieldMismatchV1 {
                        path: crate::snapshot::SnapshotPath::root(),
                    })?;
                let legacy = value.data.get(id).ok_or_else(|| {
                    SnapshotValueError::AggregateFieldMismatchV1 {
                        path: crate::snapshot::SnapshotPath::root(),
                    }
                })?;
                drafts.push(NamedValueDraft {
                    name: field.name.clone(),
                    value: draft_from_legacy_body(
                        legacy,
                        target.child(&field.schema),
                        shape_values,
                        validation,
                        context,
                    )?,
                });
            }
            Ok(ValueDataDraft::Record(drafts.into_boxed_slice()))
        });
    }

    #[cfg(feature = "matrix")]
    if let SchemaBody::Matrix { element, .. } = target.body {
        if let Some(values) = typed_matrix_draft(value) {
            return values;
        }
        #[cfg(feature = "matrix")]
        if let LegacyValue::MatrixValue(matrix) = value {
            return context.with_active(matrix.addr(), |context| {
                let mut drafts = Vec::with_capacity(matrix.rows().saturating_mul(matrix.cols()));
                for row in 1..=matrix.rows() {
                    for column in 1..=matrix.cols() {
                        let legacy = matrix.index2d(row, column);
                        drafts.push(draft_from_legacy_body(
                            &legacy,
                            target.child(element),
                            shape_values,
                            validation,
                            context,
                        )?);
                    }
                }
                if !drafts.is_empty() && drafts.len() != matrix.rows() * matrix.cols() {
                    return Err(HeterogeneousMatrixUnsupported);
                }
                Ok(ValueDataDraft::Matrix(drafts.into_boxed_slice()))
            });
        }
    }

    #[cfg(feature = "table")]
    if let (LegacyValue::Table(value), SchemaBody::Table { columns, .. }) = (value, target.body) {
        return context.with_active(value.addr(), |context| {
            let value = borrow_legacy(value)?;
            if value.data.len() != columns.len() {
                return Err(SnapshotValueError::AggregateFieldMismatchV1 {
                    path: crate::snapshot::SnapshotPath::root(),
                }
                .into());
            }
            let mut drafts = Vec::with_capacity(columns.len());
            for column in columns {
                let id = value
                    .col_names
                    .iter()
                    .find_map(|(id, name)| (name == &column.name).then_some(id))
                    .ok_or_else(|| SnapshotValueError::AggregateFieldMismatchV1 {
                        path: crate::snapshot::SnapshotPath::root(),
                    })?;
                let (_, values) = value.data.get(id).ok_or_else(|| {
                    SnapshotValueError::AggregateFieldMismatchV1 {
                        path: crate::snapshot::SnapshotPath::root(),
                    }
                })?;
                let mut column_values = Vec::with_capacity(value.rows);
                for row in 1..=value.rows {
                    column_values.push(draft_from_legacy_body(
                        &values.index2d(row, 1),
                        target.child(&column.schema),
                        shape_values,
                        validation,
                        context,
                    )?);
                }
                drafts.push(TableColumnDraft {
                    name: column.name.clone(),
                    values: column_values.into_boxed_slice(),
                });
            }
            Ok(ValueDataDraft::Table(drafts.into_boxed_slice()))
        });
    }

    #[cfg(feature = "set")]
    if let (LegacyValue::Set(value), SchemaBody::Set { element, .. }) = (value, target.body) {
        return context.with_active(value.addr(), |context| {
            let value = borrow_legacy(value)?;
            Ok(ValueDataDraft::Set(
                value
                    .set
                    .iter()
                    .map(|value| {
                        draft_from_legacy_body(
                            value,
                            target.child(element),
                            shape_values,
                            validation,
                            context,
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .into_boxed_slice(),
            ))
        });
    }

    #[cfg(feature = "map")]
    if let (
        LegacyValue::Map(value),
        SchemaBody::Map {
            key,
            value: value_schema,
            ..
        },
    ) = (value, target.body)
    {
        return context.with_active(value.addr(), |context| {
            let value = borrow_legacy(value)?;
            Ok(ValueDataDraft::Map(
                value
                    .map
                    .iter()
                    .map(|(legacy_key, legacy_value)| {
                        Ok(MapEntryDraft {
                            items: vec![
                                draft_from_legacy_body(
                                    legacy_key,
                                    target.child(key),
                                    shape_values,
                                    validation,
                                    context,
                                )?,
                                draft_from_legacy_body(
                                    legacy_value,
                                    target.child(value_schema),
                                    shape_values,
                                    validation,
                                    context,
                                )?,
                            ]
                            .into_boxed_slice(),
                        })
                    })
                    .collect::<Result<Vec<_>, LegacySnapshotError>>()?
                    .into_boxed_slice(),
            ))
        });
    }

    if let (LegacyValue::Kind(kind), SchemaBody::ReifiedType) = (value, target.body) {
        return reified_draft_from_legacy(kind, context);
    }

    Err(UnsupportedLegacyMaterialization)
}

#[cfg(feature = "matrix")]
fn logical_matrix_values<T: Clone + core::fmt::Debug + PartialEq + 'static>(
    matrix: &crate::matrix::Matrix<T>,
    mut convert: impl FnMut(T) -> Result<ValueDataDraft, LegacySnapshotError>,
) -> Result<ValueDataDraft, LegacySnapshotError> {
    let mut values = Vec::with_capacity(matrix.rows().saturating_mul(matrix.cols()));
    for row in 1..=matrix.rows() {
        for column in 1..=matrix.cols() {
            values.push(convert(matrix.index2d(row, column))?);
        }
    }
    Ok(ValueDataDraft::Matrix(values.into_boxed_slice()))
}

#[cfg(feature = "matrix")]
fn typed_matrix_draft(value: &LegacyValue) -> Option<Result<ValueDataDraft, LegacySnapshotError>> {
    macro_rules! matrix {
        ($variant:ident, $convert:expr) => {
            #[cfg(feature = "matrix")]
            if let LegacyValue::$variant(value) = value {
                return Some(logical_matrix_values(value, $convert));
            }
        };
    }
    matrix!(MatrixIndex, |value| Ok(ValueDataDraft::Index(
        u64::try_from(value).map_err(|_| LegacyIndexOutOfRange)?
    )));
    #[cfg(feature = "bool")]
    matrix!(MatrixBool, |value| Ok(ValueDataDraft::Bool(value)));
    #[cfg(feature = "u8")]
    matrix!(MatrixU8, |value| Ok(ValueDataDraft::U8(value)));
    #[cfg(feature = "u16")]
    matrix!(MatrixU16, |value| Ok(ValueDataDraft::U16(value)));
    #[cfg(feature = "u32")]
    matrix!(MatrixU32, |value| Ok(ValueDataDraft::U32(value)));
    #[cfg(feature = "u64")]
    matrix!(MatrixU64, |value| Ok(ValueDataDraft::U64(value)));
    #[cfg(feature = "u128")]
    matrix!(MatrixU128, |value| Ok(ValueDataDraft::U128(value)));
    #[cfg(feature = "i8")]
    matrix!(MatrixI8, |value| Ok(ValueDataDraft::I8(value)));
    #[cfg(feature = "i16")]
    matrix!(MatrixI16, |value| Ok(ValueDataDraft::I16(value)));
    #[cfg(feature = "i32")]
    matrix!(MatrixI32, |value| Ok(ValueDataDraft::I32(value)));
    #[cfg(feature = "i64")]
    matrix!(MatrixI64, |value| Ok(ValueDataDraft::I64(value)));
    #[cfg(feature = "i128")]
    matrix!(MatrixI128, |value| Ok(ValueDataDraft::I128(value)));
    #[cfg(feature = "f32")]
    matrix!(MatrixF32, |value| Ok(ValueDataDraft::F32(
        F32Bits::from_f32(value)
    )));
    #[cfg(feature = "f64")]
    matrix!(MatrixF64, |value| Ok(ValueDataDraft::F64(
        F64Bits::from_f64(value)
    )));
    #[cfg(feature = "string")]
    matrix!(MatrixString, |value| Ok(ValueDataDraft::String(value)));
    #[cfg(feature = "rational")]
    matrix!(MatrixR64, |value: crate::R64| {
        Ok(ValueDataDraft::Rational64 {
            numerator: *value.numer(),
            denominator: u64::try_from(*value.denom()).map_err(|_| LegacyRationalOutOfRange)?,
        })
    });
    #[cfg(feature = "complex")]
    matrix!(MatrixC64, |value: crate::C64| {
        Ok(ValueDataDraft::Complex64(Complex64Bits::new(
            F64Bits::from_f64(value.0.re),
            F64Bits::from_f64(value.0.im),
        )))
    });
    None
}

fn reified_draft_from_legacy(
    kind: &ValueKind,
    context: &mut LegacySnapshotContext<'_>,
) -> Result<ValueDataDraft, LegacySnapshotError> {
    let reified = match kind {
        ValueKind::Empty => return Err(SemanticModelError::UnresolvedKindHole.into()),
        ValueKind::Any | ValueKind::None | ValueKind::Reference(_) | ValueKind::Kind(_) => {
            let resolution = kind_expr_from_legacy(&kind_from_value_kind(kind), context.semantic)?;
            ReifiedTypeDraft::Kind {
                kind: resolution.kind,
                dimension_parameters: resolution.dimension_parameters,
            }
        }
        ValueKind::U8
        | ValueKind::U16
        | ValueKind::U32
        | ValueKind::U64
        | ValueKind::U128
        | ValueKind::I8
        | ValueKind::I16
        | ValueKind::I32
        | ValueKind::I64
        | ValueKind::I128
        | ValueKind::F32
        | ValueKind::F64
        | ValueKind::C64
        | ValueKind::R64
        | ValueKind::String
        | ValueKind::Bool
        | ValueKind::Id
        | ValueKind::Index
        | ValueKind::Matrix(_, _)
        | ValueKind::Enum(_, _)
        | ValueKind::Record(_)
        | ValueKind::Map(_, _)
        | ValueKind::Atom(_, _)
        | ValueKind::Table(_, _)
        | ValueKind::Tuple(_)
        | ValueKind::Set(_, _)
        | ValueKind::Option(_) => {
            ReifiedTypeDraft::Schema(schema_from_legacy_value_kind(kind, context.semantic)?.key())
        }
    };
    Ok(ValueDataDraft::Type(reified))
}

fn kind_from_value_kind(kind: &ValueKind) -> crate::kind::Kind {
    use crate::kind::Kind;
    match kind {
        ValueKind::Any => Kind::Any,
        ValueKind::None => Kind::None,
        ValueKind::Empty => Kind::Empty,
        ValueKind::Id => Kind::Id,
        ValueKind::Index => Kind::Index,
        ValueKind::Atom(id, name) => Kind::Atom(*id, name.clone()),
        ValueKind::Enum(id, name) => Kind::Enum(*id, name.clone()),
        ValueKind::Matrix(element, dimensions) => {
            Kind::Matrix(Box::new(kind_from_value_kind(element)), dimensions.clone())
        }
        ValueKind::Option(element) => Kind::Option(Box::new(kind_from_value_kind(element))),
        ValueKind::Tuple(elements) => {
            Kind::Tuple(elements.iter().map(kind_from_value_kind).collect())
        }
        ValueKind::Record(fields) => Kind::Record(
            fields
                .iter()
                .map(|(name, kind)| (name.clone(), kind_from_value_kind(kind)))
                .collect(),
        ),
        ValueKind::Table(columns, rows) => Kind::Table(
            columns
                .iter()
                .map(|(name, kind)| (name.clone(), kind_from_value_kind(kind)))
                .collect(),
            *rows,
        ),
        ValueKind::Set(element, cardinality) => {
            Kind::Set(Box::new(kind_from_value_kind(element)), *cardinality)
        }
        ValueKind::Map(key, value) => Kind::Map(
            Box::new(kind_from_value_kind(key)),
            Box::new(kind_from_value_kind(value)),
        ),
        ValueKind::Reference(element) => Kind::Reference(Box::new(kind_from_value_kind(element))),
        ValueKind::Kind(element) => Kind::Kind(Box::new(kind_from_value_kind(element))),
        ValueKind::U8
        | ValueKind::U16
        | ValueKind::U32
        | ValueKind::U64
        | ValueKind::U128
        | ValueKind::I8
        | ValueKind::I16
        | ValueKind::I32
        | ValueKind::I64
        | ValueKind::I128
        | ValueKind::F32
        | ValueKind::F64
        | ValueKind::C64
        | ValueKind::R64
        | ValueKind::String
        | ValueKind::Bool => Kind::Scalar(crate::hash_str(&kind.to_string())),
    }
}

pub fn legacy_from_snapshot(
    value: &Value,
    schemas: &SchemaTable,
    context: &mut dyn LegacyMaterializationContext,
) -> Result<LegacyValue, LegacySnapshotError> {
    let schema = value.validate_against(schemas)?;
    legacy_from_data(
        schema.body(),
        value.data(),
        value.shape().parameter_values(),
        schemas,
        context,
    )
}

fn legacy_from_data(
    schema: &SchemaBody,
    data: &ValueData,
    shape_values: &[u64],
    schemas: &SchemaTable,
    context: &mut dyn LegacyMaterializationContext,
) -> Result<LegacyValue, LegacySnapshotError> {
    if matches!(schema, SchemaBody::Dynamic) {
        let ValueData::Dynamic(value) = data else {
            unreachable!("validated dynamic snapshot changed representation")
        };
        let value = value.value().ok_or(UnsupportedLegacyMaterialization)?;
        return legacy_from_snapshot(value, schemas, context);
    }
    macro_rules! scalar_ref {
        ($feature:literal, $schema:pat, $data:pat => $value:expr, $variant:ident, $repr:ident) => {
            if matches!(schema, $schema) {
                #[cfg(feature = $feature)]
                {
                    let $data = data else {
                        unreachable!("validated snapshot data changed representation")
                    };
                    return Ok(LegacyValue::$variant(crate::Ref::new($value)));
                }
                #[cfg(not(feature = $feature))]
                {
                    return Err(LegacyRepresentationUnavailable {
                        representation: LegacyRepresentation::$repr,
                    });
                }
            }
        };
    }
    scalar_ref!("u8", SchemaBody::UnsignedInteger(IntegerWidth::W8), ValueData::U8(value) => *value, U8, U8);
    scalar_ref!("u16", SchemaBody::UnsignedInteger(IntegerWidth::W16), ValueData::U16(value) => *value, U16, U16);
    scalar_ref!("u32", SchemaBody::UnsignedInteger(IntegerWidth::W32), ValueData::U32(value) => *value, U32, U32);
    scalar_ref!("u64", SchemaBody::UnsignedInteger(IntegerWidth::W64), ValueData::U64(value) => *value, U64, U64);
    scalar_ref!("u128", SchemaBody::UnsignedInteger(IntegerWidth::W128), ValueData::U128(value) => *value, U128, U128);
    scalar_ref!("i8", SchemaBody::SignedInteger(IntegerWidth::W8), ValueData::I8(value) => *value, I8, I8);
    scalar_ref!("i16", SchemaBody::SignedInteger(IntegerWidth::W16), ValueData::I16(value) => *value, I16, I16);
    scalar_ref!("i32", SchemaBody::SignedInteger(IntegerWidth::W32), ValueData::I32(value) => *value, I32, I32);
    scalar_ref!("i64", SchemaBody::SignedInteger(IntegerWidth::W64), ValueData::I64(value) => *value, I64, I64);
    scalar_ref!("i128", SchemaBody::SignedInteger(IntegerWidth::W128), ValueData::I128(value) => *value, I128, I128);
    scalar_ref!("f32", SchemaBody::FloatingPoint(FloatWidth::W32), ValueData::F32(value) => value.to_f32(), F32, F32);
    scalar_ref!("f64", SchemaBody::FloatingPoint(FloatWidth::W64), ValueData::F64(value) => value.to_f64(), F64, F64);

    match (schema, data) {
        (SchemaBody::Complex(FloatWidth::W32), ValueData::Complex32(_)) => {
            return Err(UnsupportedLegacyMaterialization);
        }
        (SchemaBody::Complex(FloatWidth::W64), ValueData::Complex64(..)) => {
            #[cfg(feature = "complex")]
            {
                let ValueData::Complex64(value) = data else {
                    unreachable!("the outer pattern already validated complex data")
                };
                return Ok(LegacyValue::C64(crate::Ref::new(crate::C64::new(
                    value.real().to_f64(),
                    value.imaginary().to_f64(),
                ))));
            }
            #[cfg(not(feature = "complex"))]
            return Err(LegacyRepresentationUnavailable {
                representation: LegacyRepresentation::Complex64,
            });
        }
        (SchemaBody::Rational64, ValueData::Rational64(..)) => {
            #[cfg(feature = "rational")]
            {
                let ValueData::Rational64(value) = data else {
                    unreachable!("the outer pattern already validated rational data")
                };
                return Ok(LegacyValue::R64(crate::Ref::new(crate::R64::new(
                    value.numerator(),
                    i64::try_from(value.denominator()).map_err(|_| LegacyRationalOutOfRange)?,
                ))));
            }
            #[cfg(not(feature = "rational"))]
            return Err(LegacyRepresentationUnavailable {
                representation: LegacyRepresentation::Rational64,
            });
        }
        (SchemaBody::String, ValueData::String(..)) => {
            #[cfg(any(feature = "string", feature = "variable_define"))]
            {
                let ValueData::String(value) = data else {
                    unreachable!("the outer pattern already validated string data")
                };
                return Ok(LegacyValue::String(crate::Ref::new(value.to_string())));
            }
            #[cfg(not(any(feature = "string", feature = "variable_define")))]
            return Err(LegacyRepresentationUnavailable {
                representation: LegacyRepresentation::String,
            });
        }
        (SchemaBody::Bool, ValueData::Bool(..)) => {
            #[cfg(any(feature = "bool", feature = "variable_define"))]
            {
                let ValueData::Bool(value) = data else {
                    unreachable!("the outer pattern already validated bool data")
                };
                return Ok(LegacyValue::Bool(crate::Ref::new(*value)));
            }
            #[cfg(not(any(feature = "bool", feature = "variable_define")))]
            return Err(LegacyRepresentationUnavailable {
                representation: LegacyRepresentation::Bool,
            });
        }
        (SchemaBody::Id, ValueData::Id(value)) => return Ok(LegacyValue::Id(*value)),
        (SchemaBody::Index, ValueData::Index(value)) => {
            return Ok(LegacyValue::Index(crate::Ref::new(
                usize::try_from(*value).map_err(|_| LegacyIndexOutOfRange)?,
            )));
        }
        (SchemaBody::Atom(..), ValueData::Atom) => {
            #[cfg(feature = "atom")]
            {
                let SchemaBody::Atom(key) = schema else {
                    unreachable!("the outer pattern already validated atom data")
                };
                let (id, name) = context.resolve_nominal(NominalKind::Atom, *key)?;
                let names = crate::Ref::new(crate::Dictionary::new());
                names.borrow_mut().insert(id, name);
                return Ok(LegacyValue::Atom(crate::Ref::new(crate::MechAtom((
                    id, names,
                )))));
            }
            #[cfg(not(feature = "atom"))]
            return Err(LegacyRepresentationUnavailable {
                representation: LegacyRepresentation::Atom,
            });
        }
        (SchemaBody::Enum { .. }, ValueData::Enum(..)) => {
            #[cfg(feature = "enum")]
            {
                let (SchemaBody::Enum { key, variants }, ValueData::Enum(value)) = (schema, data)
                else {
                    unreachable!("the outer pattern already validated enum data")
                };
                let (enum_id, enum_name) = context.resolve_nominal(NominalKind::Enum, *key)?;
                let variant = &variants[value.ordinal() as usize];
                let variant_id = crate::hash_str(&variant.name);
                let payload = match (variant.payload.as_ref(), value.payload()) {
                    (Some(schema), Some(value)) => Some(legacy_from_data(
                        schema,
                        value,
                        shape_values,
                        schemas,
                        context,
                    )?),
                    (None, None) => None,
                    (Some(_), None) | (None, Some(_)) => {
                        unreachable!("validated enum changed payload cardinality")
                    }
                };
                let names = crate::Ref::new(crate::Dictionary::new());
                {
                    let mut names = names.borrow_mut();
                    names.insert(enum_id, enum_name);
                    names.insert(variant_id, variant.name.clone());
                }
                return Ok(LegacyValue::Enum(crate::Ref::new(crate::MechEnum {
                    id: enum_id,
                    variants: vec![(variant_id, payload)],
                    names,
                })));
            }
            #[cfg(not(feature = "enum"))]
            return Err(LegacyRepresentationUnavailable {
                representation: LegacyRepresentation::Enum,
            });
        }
        (SchemaBody::Option(element), ValueData::Option(value)) => {
            let option_kind = value_kind_from_schema(schema, shape_values, context)?;
            return match value {
                None => Ok(LegacyValue::EmptyKind(option_kind)),
                Some(value) => Ok(LegacyValue::Typed(
                    Box::new(legacy_from_data(
                        element,
                        value,
                        shape_values,
                        schemas,
                        context,
                    )?),
                    option_kind,
                )),
            };
        }
        (SchemaBody::Tuple(..), ValueData::Tuple(..)) => {
            #[cfg(feature = "tuple")]
            {
                let (SchemaBody::Tuple(elements), ValueData::Tuple(values)) = (schema, data) else {
                    unreachable!("the outer pattern already validated tuple data")
                };
                let values = elements
                    .iter()
                    .zip(values.iter())
                    .map(|(schema, value)| {
                        legacy_from_data(schema, value, shape_values, schemas, context)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                return Ok(LegacyValue::Tuple(crate::Ref::new(
                    crate::MechTuple::from_vec(values),
                )));
            }
            #[cfg(not(feature = "tuple"))]
            return Err(LegacyRepresentationUnavailable {
                representation: LegacyRepresentation::Tuple,
            });
        }
        (SchemaBody::Record(..), ValueData::Record(..)) => {
            #[cfg(feature = "record")]
            {
                let (SchemaBody::Record(fields), ValueData::Record(values)) = (schema, data) else {
                    unreachable!("the outer pattern already validated record data")
                };
                let mut legacy_fields = Vec::with_capacity(fields.len());
                let mut kinds = Vec::with_capacity(fields.len());
                for (field, value) in fields.iter().zip(values.fields()) {
                    let id = crate::hash_str(&field.name);
                    let legacy =
                        legacy_from_data(&field.schema, value, shape_values, schemas, context)?;
                    kinds.push(value_kind_from_schema(
                        &field.schema,
                        shape_values,
                        context,
                    )?);
                    legacy_fields.push((id, field.name.clone(), legacy));
                }
                return Ok(LegacyValue::Record(crate::Ref::new(
                    crate::MechRecord::from_parts(fields.len(), kinds, legacy_fields),
                )));
            }
            #[cfg(not(feature = "record"))]
            return Err(LegacyRepresentationUnavailable {
                representation: LegacyRepresentation::Record,
            });
        }
        (SchemaBody::Matrix { .. }, ValueData::Matrix(..)) => {
            #[cfg(all(feature = "matrix", feature = "matrixd"))]
            let (
                SchemaBody::Matrix {
                    element,
                    dimensions,
                },
                ValueData::Matrix(value),
            ) = (schema, data)
            else {
                unreachable!("the outer pattern already validated matrix data")
            };
            #[cfg(all(feature = "matrix", feature = "matrixd"))]
            return legacy_matrix_from_snapshot(
                element,
                dimensions,
                value.elements(),
                shape_values,
                schemas,
                context,
            );
            #[cfg(not(all(feature = "matrix", feature = "matrixd")))]
            return Err(LegacyRepresentationUnavailable {
                representation: LegacyRepresentation::Matrix,
            });
        }
        (SchemaBody::Table { .. }, ValueData::Table(..)) => {
            #[cfg(all(feature = "table", feature = "matrix", feature = "matrixd"))]
            let (SchemaBody::Table { columns, rows }, ValueData::Table(value)) = (schema, data)
            else {
                unreachable!("the outer pattern already validated table data")
            };
            #[cfg(all(feature = "table", feature = "matrix", feature = "matrixd"))]
            return legacy_table_from_snapshot(
                columns,
                rows,
                value,
                shape_values,
                schemas,
                context,
            );
            #[cfg(not(all(feature = "table", feature = "matrix", feature = "matrixd")))]
            return Err(LegacyRepresentationUnavailable {
                representation: LegacyRepresentation::Table,
            });
        }
        (SchemaBody::Set { .. }, ValueData::Set(..)) => {
            #[cfg(feature = "set")]
            {
                let (
                    SchemaBody::Set {
                        element,
                        cardinality,
                    },
                    ValueData::Set(value),
                ) = (schema, data)
                else {
                    unreachable!("the outer pattern already validated set data")
                };
                let kind = value_kind_from_schema(element, shape_values, context)?;
                let max_elements = Some(checked_usize(evaluate_dimension(
                    cardinality,
                    shape_values,
                )?)?);
                let mut set = indexmap::set::IndexSet::with_capacity(value.elements().len());
                for element_value in value.elements() {
                    set.insert(legacy_from_data(
                        element,
                        element_value.data(),
                        shape_values,
                        schemas,
                        context,
                    )?);
                }
                return Ok(LegacyValue::Set(crate::Ref::new(crate::MechSet {
                    kind,
                    max_elements,
                    num_elements: set.len(),
                    set,
                })));
            }
            #[cfg(not(feature = "set"))]
            return Err(LegacyRepresentationUnavailable {
                representation: LegacyRepresentation::Set,
            });
        }
        (SchemaBody::Map { .. }, ValueData::Map(..)) => {
            #[cfg(feature = "map")]
            {
                let (
                    SchemaBody::Map {
                        key,
                        value: value_schema,
                        ..
                    },
                    ValueData::Map(value),
                ) = (schema, data)
                else {
                    unreachable!("the outer pattern already validated map data")
                };
                let key_kind = value_kind_from_schema(key, shape_values, context)?;
                let value_kind = value_kind_from_schema(value_schema, shape_values, context)?;
                let entries = value
                    .entries()
                    .iter()
                    .map(|entry| {
                        Ok((
                            legacy_from_data(
                                key,
                                entry.key().data(),
                                shape_values,
                                schemas,
                                context,
                            )?,
                            legacy_from_data(
                                value_schema,
                                entry.value(),
                                shape_values,
                                schemas,
                                context,
                            )?,
                        ))
                    })
                    .collect::<Result<Vec<_>, LegacySnapshotError>>()?;
                return Ok(LegacyValue::Map(crate::Ref::new(
                    crate::MechMap::from_typed_vec(
                        key_kind,
                        value_kind,
                        value.entries().len(),
                        entries,
                    ),
                )));
            }
            #[cfg(not(feature = "map"))]
            return Err(LegacyRepresentationUnavailable {
                representation: LegacyRepresentation::Map,
            });
        }
        (SchemaBody::ReifiedType, ValueData::Type(ReifiedType::Schema(key))) => {
            let schema = schemas
                .find_by_key(*key)
                .and_then(|id| schemas.get(id))
                .ok_or(UnsupportedLegacyMaterialization)?;
            return Ok(LegacyValue::Kind(value_kind_from_schema(
                schema.body(),
                &[],
                context,
            )?));
        }
        (SchemaBody::ReifiedType, ValueData::Type(ReifiedType::Kind(_))) => {
            // Snapshot-to-legacy materialization is intentionally partial. ReifiedKind keeps
            // only permanent canonical bytes; the transitional bridge does not reverse-decode
            // them or retain a second legacy-shaped representation.
            return Err(UnsupportedLegacyMaterialization);
        }
        (SchemaBody::Dynamic, _)
        | (SchemaBody::Bool, _)
        | (SchemaBody::UnsignedInteger(_), _)
        | (SchemaBody::SignedInteger(_), _)
        | (SchemaBody::FloatingPoint(_), _)
        | (SchemaBody::Complex(_), _)
        | (SchemaBody::Rational64, _)
        | (SchemaBody::String, _)
        | (SchemaBody::Id, _)
        | (SchemaBody::Index, _)
        | (SchemaBody::Atom(_), _)
        | (SchemaBody::Enum { .. }, _)
        | (SchemaBody::Option(_), _)
        | (SchemaBody::Tuple(_), _)
        | (SchemaBody::Record(_), _)
        | (SchemaBody::Matrix { .. }, _)
        | (SchemaBody::Table { .. }, _)
        | (SchemaBody::Set { .. }, _)
        | (SchemaBody::Map { .. }, _)
        | (SchemaBody::ReifiedType, _) => {
            unreachable!("validated snapshot data changed representation")
        }
    }
}

#[cfg(all(feature = "matrix", feature = "matrixd"))]
fn legacy_matrix_from_snapshot(
    element: &SchemaBody,
    dimensions: &[DimensionExpr],
    sequence: SequenceView<'_>,
    shape_values: &[u64],
    schemas: &SchemaTable,
    context: &mut dyn LegacyMaterializationContext,
) -> Result<LegacyValue, LegacySnapshotError> {
    if dimensions.len() != 2 {
        return Err(UnsupportedLegacyMaterialization);
    }
    #[cfg(not(all(feature = "matrix", feature = "matrixd")))]
    return Err(LegacyRepresentationUnavailable {
        representation: LegacyRepresentation::Matrix,
    });
    #[cfg(all(feature = "matrix", feature = "matrixd"))]
    {
        let rows = checked_usize(evaluate_dimension(&dimensions[0], shape_values)?)?;
        let columns = checked_usize(evaluate_dimension(&dimensions[1], shape_values)?)?;
        let snapshot_values = unpack_sequence(sequence)?;
        let mut row_major = Vec::with_capacity(snapshot_values.len());
        for value in &snapshot_values {
            row_major.push(legacy_from_data(
                element,
                value,
                shape_values,
                schemas,
                context,
            )?);
        }
        let column_major = column_major(row_major, rows, columns);
        let matrix = <LegacyValue as crate::ToMatrix>::to_matrixd(column_major, rows, columns);
        Ok(LegacyValue::MatrixValue(matrix))
    }
}

#[cfg(all(feature = "table", feature = "matrix", feature = "matrixd"))]
fn legacy_table_from_snapshot(
    columns: &[crate::SchemaField],
    rows: &DimensionExpr,
    value: &crate::snapshot::TableValue,
    shape_values: &[u64],
    schemas: &SchemaTable,
    context: &mut dyn LegacyMaterializationContext,
) -> Result<LegacyValue, LegacySnapshotError> {
    #[cfg(not(all(feature = "table", feature = "matrix", feature = "matrixd")))]
    return Err(LegacyRepresentationUnavailable {
        representation: LegacyRepresentation::Table,
    });
    #[cfg(all(feature = "table", feature = "matrix", feature = "matrixd"))]
    {
        let rows = checked_usize(evaluate_dimension(rows, shape_values)?)?;
        let mut legacy_columns = Vec::with_capacity(columns.len());
        let mut names = Vec::with_capacity(columns.len());
        for (index, column) in columns.iter().enumerate() {
            let mut legacy_values = Vec::with_capacity(rows);
            for item in unpack_sequence(value.column(index).expect("validated table column"))? {
                legacy_values.push(legacy_from_data(
                    &column.schema,
                    &item,
                    shape_values,
                    schemas,
                    context,
                )?);
            }
            let id = crate::hash_str(&column.name);
            let matrix = <LegacyValue as crate::ToMatrix>::to_matrixd(legacy_values, rows, 1);
            legacy_columns.push((
                id,
                value_kind_from_schema(&column.schema, shape_values, context)?,
                matrix,
            ));
            names.push((id, column.name.clone()));
        }
        Ok(LegacyValue::Table(crate::Ref::new(
            crate::MechTable::from_parts(rows, columns.len(), legacy_columns, names),
        )))
    }
}

#[cfg(all(feature = "matrix", feature = "matrixd"))]
fn unpack_sequence(sequence: SequenceView<'_>) -> Result<Vec<ValueData>, LegacySnapshotError> {
    macro_rules! values {
        ($items:expr, $variant:ident) => {
            $items
                .iter()
                .cloned()
                .map(ValueData::$variant)
                .collect::<Vec<_>>()
        };
    }
    Ok(match sequence {
        SequenceView::U8(items) => values!(items, U8),
        SequenceView::U16(items) => values!(items, U16),
        SequenceView::U32(items) => values!(items, U32),
        SequenceView::U64(items) => values!(items, U64),
        SequenceView::U128(items) => values!(items, U128),
        SequenceView::I8(items) => values!(items, I8),
        SequenceView::I16(items) => values!(items, I16),
        SequenceView::I32(items) => values!(items, I32),
        SequenceView::I64(items) => values!(items, I64),
        SequenceView::I128(items) => values!(items, I128),
        SequenceView::F32(items) => values!(items, F32),
        SequenceView::F64(items) => values!(items, F64),
        SequenceView::Complex32(items) => values!(items, Complex32),
        SequenceView::Complex64(items) => values!(items, Complex64),
        SequenceView::Rational64(items) => values!(items, Rational64),
        SequenceView::Bool(items) => values!(items, Bool),
        SequenceView::String(items) => items.iter().cloned().map(ValueData::String).collect(),
        SequenceView::Id(items) => values!(items, Id),
        SequenceView::Index(items) => values!(items, Index),
        SequenceView::Unit(count) => {
            vec![ValueData::Atom; usize::try_from(count).map_err(|_| LegacyDimensionOutOfRange)?]
        }
        SequenceView::Values(items) => items.to_vec(),
    })
}

#[cfg(all(feature = "matrix", feature = "matrixd"))]
fn column_major(values: Vec<LegacyValue>, rows: usize, columns: usize) -> Vec<LegacyValue> {
    let mut result = Vec::with_capacity(values.len());
    for column in 0..columns {
        for row in 0..rows {
            result.push(values[row * columns + column].clone());
        }
    }
    result
}

fn value_kind_from_schema(
    schema: &SchemaBody,
    shape_values: &[u64],
    context: &mut dyn LegacyMaterializationContext,
) -> Result<ValueKind, LegacySnapshotError> {
    Ok(match schema {
        SchemaBody::Dynamic => ValueKind::Any,
        SchemaBody::Bool => ValueKind::Bool,
        SchemaBody::UnsignedInteger(IntegerWidth::W8) => ValueKind::U8,
        SchemaBody::UnsignedInteger(IntegerWidth::W16) => ValueKind::U16,
        SchemaBody::UnsignedInteger(IntegerWidth::W32) => ValueKind::U32,
        SchemaBody::UnsignedInteger(IntegerWidth::W64) => ValueKind::U64,
        SchemaBody::UnsignedInteger(IntegerWidth::W128) => ValueKind::U128,
        SchemaBody::SignedInteger(IntegerWidth::W8) => ValueKind::I8,
        SchemaBody::SignedInteger(IntegerWidth::W16) => ValueKind::I16,
        SchemaBody::SignedInteger(IntegerWidth::W32) => ValueKind::I32,
        SchemaBody::SignedInteger(IntegerWidth::W64) => ValueKind::I64,
        SchemaBody::SignedInteger(IntegerWidth::W128) => ValueKind::I128,
        SchemaBody::FloatingPoint(FloatWidth::W32) => ValueKind::F32,
        SchemaBody::FloatingPoint(FloatWidth::W64) => ValueKind::F64,
        SchemaBody::Complex(FloatWidth::W64) => ValueKind::C64,
        SchemaBody::Complex(FloatWidth::W32) => return Err(UnsupportedLegacyMaterialization),
        SchemaBody::Rational64 => ValueKind::R64,
        SchemaBody::String => ValueKind::String,
        SchemaBody::Id => ValueKind::Id,
        SchemaBody::Index => ValueKind::Index,
        SchemaBody::Atom(key) => {
            let (id, name) = context.resolve_nominal(NominalKind::Atom, *key)?;
            ValueKind::Atom(id, name)
        }
        SchemaBody::Enum { key, .. } => {
            let (id, name) = context.resolve_nominal(NominalKind::Enum, *key)?;
            ValueKind::Enum(id, name)
        }
        SchemaBody::Option(element) => ValueKind::Option(Box::new(value_kind_from_schema(
            element,
            shape_values,
            context,
        )?)),
        SchemaBody::Tuple(elements) => ValueKind::Tuple(
            elements
                .iter()
                .map(|element| value_kind_from_schema(element, shape_values, context))
                .collect::<Result<Vec<_>, _>>()?,
        ),
        SchemaBody::Record(fields) => ValueKind::Record(
            fields
                .iter()
                .map(|field| {
                    Ok((
                        field.name.clone(),
                        value_kind_from_schema(&field.schema, shape_values, context)?,
                    ))
                })
                .collect::<Result<Vec<_>, LegacySnapshotError>>()?,
        ),
        SchemaBody::Matrix {
            element,
            dimensions,
        } => ValueKind::Matrix(
            Box::new(value_kind_from_schema(element, shape_values, context)?),
            dimensions
                .iter()
                .map(|dimension| checked_usize(evaluate_dimension(dimension, shape_values)?))
                .collect::<Result<Vec<_>, _>>()?,
        ),
        SchemaBody::Table { columns, rows } => ValueKind::Table(
            columns
                .iter()
                .map(|column| {
                    Ok((
                        column.name.clone(),
                        value_kind_from_schema(&column.schema, shape_values, context)?,
                    ))
                })
                .collect::<Result<Vec<_>, LegacySnapshotError>>()?,
            checked_usize(evaluate_dimension(rows, shape_values)?)?,
        ),
        SchemaBody::Set {
            element,
            cardinality,
        } => ValueKind::Set(
            Box::new(value_kind_from_schema(element, shape_values, context)?),
            Some(checked_usize(evaluate_dimension(
                cardinality,
                shape_values,
            )?)?),
        ),
        SchemaBody::Map { key, value, .. } => ValueKind::Map(
            Box::new(value_kind_from_schema(key, shape_values, context)?),
            Box::new(value_kind_from_schema(value, shape_values, context)?),
        ),
        SchemaBody::ReifiedType => return Err(UnsupportedLegacyMaterialization),
    })
}

fn checked_usize(value: u64) -> Result<usize, LegacySnapshotError> {
    usize::try_from(value).map_err(|_| LegacyDimensionOutOfRange)
}

fn resolved_product(
    dimensions: &[DimensionExpr],
    shape_values: &[u64],
) -> Result<u64, LegacySnapshotError> {
    let mut product = 1_u64;
    for dimension in dimensions {
        product = product
            .checked_mul(evaluate_dimension(dimension, shape_values)?)
            .ok_or(SemanticModelError::DimensionOverflowV1)?;
    }
    Ok(product)
}

fn evaluate_dimension(
    expression: &DimensionExpr,
    values: &[u64],
) -> Result<u64, LegacySnapshotError> {
    let operands = |items: &[DimensionExpr]| {
        items
            .iter()
            .map(|item| evaluate_dimension(item, values))
            .collect::<Result<Vec<_>, _>>()
    };
    Ok(match expression {
        DimensionExpr::Hole => return Err(SemanticModelError::UnresolvedDimensionHole.into()),
        DimensionExpr::Constant(value) => *value,
        DimensionExpr::Parameter(id) => values
            .get(id.get() as usize)
            .copied()
            .ok_or(SemanticModelError::UnknownDimensionParameterV1 { id: *id })?,
        DimensionExpr::Add(items) => {
            operands(items)?
                .into_iter()
                .try_fold(0_u64, |left, right| {
                    left.checked_add(right)
                        .ok_or(SemanticModelError::DimensionOverflowV1)
                })?
        }
        DimensionExpr::Multiply(items) => {
            operands(items)?
                .into_iter()
                .try_fold(1_u64, |left, right| {
                    left.checked_mul(right)
                        .ok_or(SemanticModelError::DimensionOverflowV1)
                })?
        }
        DimensionExpr::Min(items) => {
            operands(items)?
                .into_iter()
                .min()
                .ok_or(SemanticModelError::EmptyMinMaxV1 {
                    operator: crate::DimensionOperator::Min,
                })?
        }
        DimensionExpr::Max(items) => {
            operands(items)?
                .into_iter()
                .max()
                .ok_or(SemanticModelError::EmptyMinMaxV1 {
                    operator: crate::DimensionOperator::Max,
                })?
        }
    })
}
