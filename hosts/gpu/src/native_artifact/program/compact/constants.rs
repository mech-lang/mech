//! Immutable native tables and scalar bits from validated canonical constants.
use super::layout::{Kind, Layout, Type};
use crate::native_artifact::{NativeCompileError, compiler::backend};
use cranelift_module::{DataDescription, DataId, Linkage, Module};
use mech_core::{DimensionExpr, SchemaBody, ValueData};
use mech_engine::native_artifact::{NativeProgramArtifactPlan, NativeScalarValue};
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
pub(super) enum Initial {
    Bits(u64),
    Table(DataId),
}
fn scalar(data: &ValueData) -> Result<u64, NativeCompileError> {
    Ok(match data {
        ValueData::Bool(v) => u64::from(*v),
        ValueData::U8(v) => u64::from(*v),
        ValueData::U16(v) => u64::from(*v),
        ValueData::U32(v) => u64::from(*v),
        ValueData::U64(v) | ValueData::Id(v) | ValueData::Index(v) => *v,
        ValueData::I8(v) => *v as u64,
        ValueData::I16(v) => *v as u64,
        ValueData::I32(v) => *v as u64,
        ValueData::I64(v) => *v as u64,
        ValueData::F64(v) => NativeScalarValue::F64(*v).to_bits(),
        _ => {
            return Err(backend(
                "compact constant requires a supported scalar payload",
            ));
        }
    })
}
// Logical lanes and physical element bytes share one layout. Pointer lanes
// become Object relocations; no process address is serialized into constants.
fn packed(
    ty: &Type,
    values: &[Initial],
    out: &mut [u8],
    base: usize,
    relocations: &mut Vec<(usize, DataId)>,
) -> Result<(), NativeCompileError> {
    match &ty.kind {
        Kind::Scalar { bytes, .. } => {
            let Initial::Bits(value) = values[0] else {
                return Err(backend("native scalar constant contains a pointer"));
            };
            out[..usize::from(*bytes)].copy_from_slice(&value.to_ne_bytes()[..usize::from(*bytes)]);
        }
        Kind::Utf8 => {
            for (lane, value) in values.iter().enumerate() {
                match value {
                    Initial::Bits(value) => {
                        out[lane * 8..lane * 8 + 8].copy_from_slice(&value.to_ne_bytes())
                    }
                    Initial::Table(id) => relocations.push((base + lane * 8, *id)),
                }
            }
        }
        Kind::Product(fields) => {
            for field in fields {
                let at = field
                    .packed_offset
                    .ok_or_else(|| backend("native constant field lacks a physical offset"))?;
                let size = field
                    .ty
                    .packed_bytes
                    .ok_or_else(|| backend("native constant field lacks physical size"))?;
                packed(
                    &field.ty,
                    &values[field.offset..field.offset + field.ty.words],
                    &mut out[at..at + size],
                    base + at,
                    relocations,
                )?;
            }
        }
        _ => {
            return Err(backend(
                "native constant element has no fixed physical representation",
            ));
        }
    }
    Ok(())
}
fn flatten<M: Module>(
    module: &mut M,
    ty: &Type,
    body: &SchemaBody,
    data: &ValueData,
    next: &mut usize,
) -> Result<Vec<Initial>, NativeCompileError> {
    let mut out = vec![Initial::Bits(0); ty.words];
    match &ty.kind {
        Kind::Utf8 => {
            let ValueData::String(text) = data else {
                return Err(backend("native UTF8 constant lost its String payload"));
            };
            let bytes = text.as_bytes();
            if !bytes.is_empty() {
                let id = module
                    .declare_data(
                        &format!("mech_compact_utf8_{}", *next),
                        Linkage::Local,
                        false,
                        false,
                    )
                    .map_err(backend)?;
                *next += 1;
                let mut description = DataDescription::new();
                description.define(bytes.into());
                module.define_data(id, &description).map_err(backend)?;
                out[0] = Initial::Table(id);
            }
            out[1] = Initial::Bits(bytes.len() as u64);
        }
        Kind::Scalar { .. } => out[0] = Initial::Bits(scalar(data)?),
        Kind::Product(fields) => {
            let items = match data {
                ValueData::Tuple(v) => v.as_ref(),
                ValueData::Record(v) => v.fields(),
                _ => return Err(backend("compact constant lost product data")),
            };
            let bodies = match body {
                SchemaBody::Tuple(v) => v.iter().collect::<Vec<_>>(),
                SchemaBody::Record(v) => v.iter().map(|f| &f.schema).collect(),
                _ => return Err(backend("compact constant lost product schema")),
            };
            if fields.len() != items.len() || fields.len() != bodies.len() {
                return Err(backend("compact constant product width differs"));
            }
            for ((field, data), body) in fields.iter().zip(items).zip(bodies) {
                let values = flatten(module, &field.ty, body, data, next)?;
                out[field.offset..field.offset + values.len()].copy_from_slice(&values);
            }
        }
        Kind::Tagged(variants) => {
            let ValueData::Enum(value) = data else {
                return Err(backend("compact constant lost enum payload"));
            };
            let ordinal = value.ordinal() as usize;
            let variant = variants
                .get(ordinal)
                .ok_or_else(|| backend("compact constant enum ordinal differs"))?;
            out[0] = Initial::Bits(ordinal as u64);
            match (&variant.ty, value.payload()) {
                (Some(ty), Some(data)) => {
                    let SchemaBody::Enum { variants, .. } = body else {
                        return Err(backend("compact constant lost enum schema"));
                    };
                    let body = variants[ordinal]
                        .payload
                        .as_ref()
                        .ok_or_else(|| backend("compact constant payload schema missing"))?;
                    let values = flatten(module, ty, body, data, next)?;
                    out[variant.offset..variant.offset + values.len()].copy_from_slice(&values);
                }
                (None, None) => {}
                _ => return Err(backend("compact constant enum payload presence differs")),
            }
        }
        Kind::SortedMap { entry } => {
            let (ValueData::Map(map), SchemaBody::Map { key, value, .. }) = (data, body) else {
                return Err(backend("native Map constant lost its typed entries"));
            };
            let fields = entry.fields()?;
            let stride = entry
                .packed_bytes
                .ok_or_else(|| backend("native Map constant has no entry stride"))?;
            let length = map.entries().len();
            let size = length
                .checked_mul(stride)
                .ok_or_else(|| backend("native Map constant size overflow"))?;
            let mut bytes = vec![0; size];
            let mut relocations = vec![];
            for (i, item) in map.entries().iter().enumerate() {
                let mut values = flatten(module, &fields[0].ty, key, item.key().data(), next)?;
                values.extend(flatten(module, &fields[1].ty, value, item.value(), next)?);
                packed(
                    entry,
                    &values,
                    &mut bytes[i * stride..(i + 1) * stride],
                    i * stride,
                    &mut relocations,
                )?;
            }
            if size != 0 {
                let id = module
                    .declare_data(
                        &format!("mech_compact_map_{}", *next),
                        Linkage::Local,
                        false,
                        false,
                    )
                    .map_err(backend)?;
                *next += 1;
                let mut description = DataDescription::new();
                description.set_align(entry.packed_align as u64);
                description.define(bytes.into());
                for (offset, target) in relocations {
                    let target = module.declare_data_in_data(target, &mut description);
                    description.write_data_addr(u32::try_from(offset).map_err(backend)?, target, 0);
                }
                module.define_data(id, &description).map_err(backend)?;
                out[0] = Initial::Table(id);
            }
            for (at, n) in [(1, length), (2, length), (3, 0), (4, 0)] {
                out[at] = Initial::Bits(n as u64);
            }
        }
        Kind::Array { element } => {
            let ValueData::Matrix(value) = data else {
                return Err(backend("compact constant lost matrix payload"));
            };
            let SchemaBody::Matrix {
                dimensions,
                element: element_body,
            } = body
            else {
                return Err(backend("compact constant lost closed matrix schema"));
            };
            let extents = dimensions
                .iter()
                .map(|d| match d {
                    DimensionExpr::Constant(n) => usize::try_from(*n).map_err(backend),
                    _ => Err(backend("compact constant has unresolved dimensions")),
                })
                .collect::<Result<Vec<_>, _>>()?;
            let elements = value.elements();
            let length = elements.len();
            if extents.len() != 2 || extents[0].checked_mul(extents[1]) != Some(length) {
                return Err(backend(
                    "compact constant shape and initialized payload differ",
                ));
            }
            let stride = element
                .packed_bytes
                .ok_or_else(|| backend("compact constant element has no packed representation"))?;
            let size = length
                .checked_mul(stride)
                .ok_or_else(|| backend("compact constant table size overflow"))?;
            let mut bytes = vec![0; size];
            let mut relocations = Vec::new();
            for i in 0..length {
                let data = elements
                    .value_at(i)
                    .ok_or_else(|| backend("native constant element absent"))?;
                let values = flatten(module, element, element_body, &data, next)?;
                packed(
                    element,
                    &values,
                    &mut bytes[i * stride..(i + 1) * stride],
                    i * stride,
                    &mut relocations,
                )?;
            }
            if size != 0 {
                let id = module
                    .declare_data(
                        &format!("mech_compact_table_{}", *next),
                        Linkage::Local,
                        false,
                        false,
                    )
                    .map_err(backend)?;
                *next += 1;
                let mut description = DataDescription::new();
                description.set_align(element.packed_align as u64);
                description.define(bytes.into());
                for (offset, target) in relocations {
                    let target = module.declare_data_in_data(target, &mut description);
                    description.write_data_addr(u32::try_from(offset).map_err(backend)?, target, 0);
                }
                module.define_data(id, &description).map_err(backend)?;
                out[0] = Initial::Table(id);
            }
            for (at, n) in [(1, length), (2, length), (3, extents[0]), (4, extents[1])] {
                out[at] = Initial::Bits(n as u64)
            }
        }
    }
    Ok(out)
}
pub(super) fn prepare<M: Module>(
    module: &mut M,
    plan: &NativeProgramArtifactPlan,
    layout: &Layout,
) -> Result<BTreeMap<usize, Initial>, NativeCompileError> {
    let mut result = BTreeMap::new();
    let mut next = 0;
    for constant in plan.constants() {
        let value = constant.value().get();
        let closed = plan.values()[value as usize]
            .definition()
            .closed_body(constant.constant().shape())
            .map_err(|e| backend(format!("{e:?}")))?;
        let values = flatten(
            module,
            layout.ty(value),
            &closed,
            constant.constant().data(),
            &mut next,
        )?;
        for (index, word) in values.into_iter().enumerate() {
            result.insert(layout.slot(value) + index, word);
        }
    }
    Ok(result)
}
