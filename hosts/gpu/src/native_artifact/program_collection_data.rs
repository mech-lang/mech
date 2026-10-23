//! Borrowed canonical sequence reads. No whole sequence is unpacked or cloned.
use super::{Error, invalid};
use mech_core::{Value, ValueData, snapshot::SequenceView};
use mech_engine::native_artifact::NativeScalarKind;

pub(super) fn output_nodes(schema: &mech_core::SchemaBody, count: u64) -> Result<u64, Error> {
    let mech_core::SchemaBody::Matrix { element, .. } = schema else {
        return Err(invalid("collected output is not a matrix"));
    };
    mech_core::snapshot::prospective_packed_matrix_node_count(element, count)
        .ok_or_else(|| invalid("collected node footprint is not a supported packed matrix"))
}

pub(super) enum Element<'a> {
    Scalar(NativeScalarKind, u64),
    // Index keeps its canonical identity; it is not U64 arithmetic transport.
    Index(u64),
    Text(&'a str),
}
impl Element<'_> {
    pub(super) fn bytes(&self) -> u64 {
        match self {
            Self::Scalar(NativeScalarKind::Bool, _) => 1,
            Self::Scalar(_, _) => 8,
            Self::Index(_) => 8,
            Self::Text(text) => text.len() as u64,
        }
    }
}
pub(super) fn count(value: &Value) -> Result<u64, Error> {
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
        ValueData::U64(value) => Element::Scalar(NativeScalarKind::U64, *value),
        ValueData::I64(value) => Element::Scalar(NativeScalarKind::I64, *value as u64),
        ValueData::F64(value) => Element::Scalar(NativeScalarKind::F64, value.bits()),
        ValueData::String(value) => Element::Text(value),
        _ => return Err(invalid("unsupported native generator element")),
    })
}
pub(super) fn element(
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
