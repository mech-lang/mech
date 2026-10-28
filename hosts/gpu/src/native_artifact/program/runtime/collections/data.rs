//! Borrowed sequence reads and typed unsigned collection storage.
use super::{Error, invalid};
use mech_core::{Value, ValueData, snapshot::SequenceView};
use mech_engine::native_artifact::NativeScalarKind;

// The collected word stays in its exact storage width. Finalization transfers
// that allocation to the ordinary canonical Matrix constructor, without an
// intermediate draft array or a per-element repacking pass.
pub(super) enum CollectedValues {
    U8(Vec<u8>),
    U16(Vec<u16>),
    U32(Vec<u32>),
    U64(Vec<u64>),
    Draft(Vec<mech_core::ValueDataDraft>),
}
impl Default for CollectedValues {
    fn default() -> Self {
        Self::Draft(Vec::new())
    }
}
impl CollectedValues {
    pub(super) fn new(body: &mech_core::SchemaBody) -> Self {
        use mech_core::{IntegerWidth as W, SchemaBody as B};
        match body {
            B::Matrix { element, .. } => match element.as_ref() {
                B::UnsignedInteger(W::W8) => Self::U8(Vec::new()),
                B::UnsignedInteger(W::W16) => Self::U16(Vec::new()),
                B::UnsignedInteger(W::W32) => Self::U32(Vec::new()),
                B::UnsignedInteger(W::W64) => Self::U64(Vec::new()),
                _ => Self::default(),
            },
            _ => Self::default(),
        }
    }
    pub(super) fn packed_width(&self) -> Option<u64> {
        match self {
            Self::U8(_) => Some(1),
            Self::U16(_) => Some(2),
            Self::U32(_) => Some(4),
            Self::U64(_) => Some(8),
            Self::Draft(_) => None,
        }
    }
    pub(super) fn len(&self) -> usize {
        match self {
            Self::U8(v) => v.len(),
            Self::U16(v) => v.len(),
            Self::U32(v) => v.len(),
            Self::U64(v) => v.len(),
            Self::Draft(v) => v.len(),
        }
    }
    pub(super) fn capacity(&self) -> usize {
        match self {
            Self::U8(v) => v.capacity(),
            Self::U16(v) => v.capacity(),
            Self::U32(v) => v.capacity(),
            Self::U64(v) => v.capacity(),
            Self::Draft(v) => v.capacity(),
        }
    }
    pub(super) fn element_bytes(&self) -> u64 {
        self.packed_width()
            .unwrap_or(core::mem::size_of::<mech_core::ValueDataDraft>() as u64)
    }
    pub(super) fn backing_bytes(&self, capacity: usize, payload: u64) -> Result<u64, Error> {
        let bytes = super::mul(capacity as u64, self.element_bytes())?;
        if self.packed_width().is_some() {
            Ok(bytes)
        } else {
            super::add(bytes, payload)
        }
    }
    pub(super) fn output_bytes(
        &self,
        payload: u64,
        count: u64,
        shape_parameters: usize,
    ) -> Result<u64, Error> {
        let metadata = if self.packed_width().is_some() {
            super::add(
                (core::mem::size_of::<Value>() + core::mem::size_of::<ValueData>()) as u64,
                super::mul(shape_parameters as u64, 8)?,
            )?
        } else {
            super::add(super::mul(count, 32)?, 256)?
        };
        super::add(payload, metadata)
    }
    pub(super) fn retained_nodes(&self) -> Result<u64, Error> {
        super::add(
            if self.packed_width().is_some() {
                0
            } else {
                self.len() as u64
            },
            2,
        )
    }
    pub(super) fn fill_unsigned(
        &mut self,
        count: usize,
        kind: NativeScalarKind,
        word: u64,
    ) -> Result<(), Error> {
        if self.len() != 0 || self.capacity() != count {
            return Err(invalid("uniform fill outside its admitted capacity"));
        }
        match (self, kind) {
            (Self::U8(values), NativeScalarKind::U8) => values.resize(
                count,
                u8::try_from(word).map_err(|_| invalid("uniform U8 exceeds its kind"))?,
            ),
            (Self::U16(values), NativeScalarKind::U16) => values.resize(
                count,
                u16::try_from(word).map_err(|_| invalid("uniform U16 exceeds its kind"))?,
            ),
            (Self::U32(values), NativeScalarKind::U32) => values.resize(
                count,
                u32::try_from(word).map_err(|_| invalid("uniform U32 exceeds its kind"))?,
            ),
            (Self::U64(values), NativeScalarKind::U64) => values.resize(count, word),
            _ => return Err(invalid("uniform fill differs from its packed kind")),
        }
        Ok(())
    }
    pub(super) fn reserve(&mut self, capacity: usize) -> Result<(), Error> {
        fn grow<T>(values: &mut Vec<T>, capacity: usize) -> Result<(), Error> {
            values
                .try_reserve_exact(
                    capacity
                        .checked_sub(values.len())
                        .ok_or_else(|| invalid("collection capacity"))?,
                )
                .map_err(|_| invalid("collection allocation"))?;
            *values = super::allocation::exact_capacity(
                core::mem::take(values),
                capacity,
                "collection exceeds admitted capacity",
            )?;
            Ok(())
        }
        match self {
            Self::U8(v) => grow(v, capacity),
            Self::U16(v) => grow(v, capacity),
            Self::U32(v) => grow(v, capacity),
            Self::U64(v) => grow(v, capacity),
            Self::Draft(v) => grow(v, capacity),
        }
    }
    pub(super) fn push(&mut self, value: mech_core::ValueDataDraft) -> Result<(), Error> {
        use mech_core::ValueDataDraft as D;
        if self.len() == self.capacity() {
            return Err(invalid("collection push lacks admitted capacity"));
        }
        match (self, value) {
            (Self::U8(v), D::U8(x)) => v.push(x),
            (Self::U16(v), D::U16(x)) => v.push(x),
            (Self::U32(v), D::U32(x)) => v.push(x),
            (Self::U64(v), D::U64(x)) => v.push(x),
            (Self::Draft(v), x) => v.push(x),
            _ => return Err(invalid("collected word kind differs")),
        }
        Ok(())
    }
    pub(super) fn finalize(
        self,
        schema: mech_core::SchemaId,
        shape: mech_core::ShapeInstance,
        schemas: std::sync::Arc<mech_core::SchemaTable>,
    ) -> Result<Value, Error> {
        match self {
            Self::U8(v) => Value::from_u8_matrix(schema, shape, v.into_boxed_slice(), schemas),
            Self::U16(v) => Value::from_u16_matrix(schema, shape, v.into_boxed_slice(), schemas),
            Self::U32(v) => Value::from_u32_matrix(schema, shape, v.into_boxed_slice(), schemas),
            Self::U64(v) => Value::from_u64_matrix(schema, shape, v.into_boxed_slice(), schemas),
            Self::Draft(v) => mech_core::ValueDraft {
                schema,
                shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
                data: mech_core::ValueDataDraft::Matrix(v.into_boxed_slice()),
            }
            .finalize(
                &mech_core::snapshot::SnapshotValidationContext::with_shared_schemas(&schemas),
            ),
        }
        .map_err(Error::Snapshot)
    }
}

pub(super) fn output_nodes(schema: &mech_core::SchemaBody, count: u64) -> Result<u64, Error> {
    let mech_core::SchemaBody::Matrix { element, .. } = schema else {
        return Err(invalid("collected output is not a matrix"));
    };
    mech_core::snapshot::prospective_packed_matrix_node_count(element, count)
        .ok_or_else(|| invalid("collected node footprint is not a supported packed matrix"))
}

pub(in crate::native_artifact::program::runtime::collections) enum Element<'a> {
    Scalar(NativeScalarKind, u64),
    // Index keeps its canonical identity; it is not U64 arithmetic transport.
    Index(u64),
    Text(&'a str),
}
impl Element<'_> {
    pub(in crate::native_artifact::program::runtime::collections) fn bytes(&self) -> u64 {
        match self {
            Self::Scalar(NativeScalarKind::Bool | NativeScalarKind::U8, _) => 1,
            Self::Scalar(NativeScalarKind::U16, _) => 2,
            Self::Scalar(NativeScalarKind::U32, _) => 4,
            Self::Scalar(_, _) => 8,
            Self::Index(_) => 8,
            Self::Text(text) => text.len() as u64,
        }
    }
}
pub(in crate::native_artifact::program::runtime::collections) fn count(
    value: &Value,
) -> Result<u64, Error> {
    let count = match value.data() {
        ValueData::Matrix(matrix) => matrix.elements().len(),
        ValueData::Set(set) => set.elements().len(),
        ValueData::String(text) => mech_engine::intrinsics::text_graphemes::count(text),
        _ => return Err(invalid("native generator source is not a sequence")),
    };
    u64::try_from(count).map_err(|_| invalid("generator count exceeds u64"))
}
fn scalar(value: &ValueData) -> Result<Element<'_>, Error> {
    Ok(match value {
        ValueData::Bool(value) => Element::Scalar(NativeScalarKind::Bool, u64::from(*value)),
        ValueData::Index(value) => Element::Index(*value),
        ValueData::U8(value) => Element::Scalar(NativeScalarKind::U8, u64::from(*value)),
        ValueData::U16(value) => Element::Scalar(NativeScalarKind::U16, u64::from(*value)),
        ValueData::U32(value) => Element::Scalar(NativeScalarKind::U32, u64::from(*value)),
        ValueData::U64(value) => Element::Scalar(NativeScalarKind::U64, *value),
        ValueData::I64(value) => Element::Scalar(NativeScalarKind::I64, *value as u64),
        ValueData::F64(value) => Element::Scalar(NativeScalarKind::F64, value.bits()),
        ValueData::String(value) => Element::Text(value),
        _ => return Err(invalid("unsupported native generator element")),
    })
}
pub(in crate::native_artifact::program::runtime::collections) fn element(
    value: &Value,
    ordinal: u64,
    offset: usize,
) -> Result<(Element<'_>, usize), Error> {
    let index =
        usize::try_from(ordinal).map_err(|_| invalid("generator index exceeds address range"))?;
    let item = match value.data() {
        ValueData::Matrix(matrix) => match matrix.elements() {
            SequenceView::Index(values) => Element::Index(
                *values
                    .get(index)
                    .ok_or_else(|| invalid("generator index"))?,
            ),
            SequenceView::Bool(values) => Element::Scalar(
                NativeScalarKind::Bool,
                u64::from(
                    *values
                        .get(index)
                        .ok_or_else(|| invalid("generator index"))?,
                ),
            ),
            SequenceView::U8(values) => Element::Scalar(
                NativeScalarKind::U8,
                u64::from(
                    *values
                        .get(index)
                        .ok_or_else(|| invalid("generator index"))?,
                ),
            ),
            SequenceView::U16(values) => Element::Scalar(
                NativeScalarKind::U16,
                u64::from(
                    *values
                        .get(index)
                        .ok_or_else(|| invalid("generator index"))?,
                ),
            ),
            SequenceView::U32(values) => Element::Scalar(
                NativeScalarKind::U32,
                u64::from(
                    *values
                        .get(index)
                        .ok_or_else(|| invalid("generator index"))?,
                ),
            ),
            SequenceView::U64(values) => Element::Scalar(
                NativeScalarKind::U64,
                *values
                    .get(index)
                    .ok_or_else(|| invalid("generator index"))?,
            ),
            SequenceView::I64(values) => Element::Scalar(
                NativeScalarKind::I64,
                *values
                    .get(index)
                    .ok_or_else(|| invalid("generator index"))? as u64,
            ),
            SequenceView::F64(values) => Element::Scalar(
                NativeScalarKind::F64,
                values
                    .get(index)
                    .ok_or_else(|| invalid("generator index"))?
                    .bits(),
            ),
            SequenceView::String(values) => Element::Text(
                values
                    .get(index)
                    .ok_or_else(|| invalid("generator index"))?,
            ),
            SequenceView::Values(values) => scalar(
                values
                    .get(index)
                    .ok_or_else(|| invalid("generator index"))?,
            )?,
            _ => return Err(invalid("native generator storage kind differs")),
        },
        ValueData::Set(set) => scalar(
            set.elements()
                .get(index)
                .ok_or_else(|| invalid("generator index"))?
                .data(),
        )?,
        ValueData::String(text) => {
            let rest = text
                .get(offset..)
                .ok_or_else(|| invalid("generator UTF8 cursor"))?;
            let item = grapheme::Graphemes::from_usvs(rest)
                .iter()
                .next()
                .ok_or_else(|| invalid("generator grapheme cursor"))?
                .as_str();
            return Ok((
                Element::Text(item),
                offset
                    .checked_add(item.len())
                    .ok_or_else(|| invalid("generator cursor overflow"))?,
            ));
        }
        _ => return Err(invalid("native generator source is not a sequence")),
    };
    Ok((item, offset))
}
