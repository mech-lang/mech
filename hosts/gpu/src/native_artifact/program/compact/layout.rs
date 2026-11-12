//! Typed compiler coordinates, not managed Value slots or runtime authority.
use crate::native_artifact::NativeCompileError;
use crate::native_artifact::compiler::backend;
use mech_core::{FloatWidth, IntegerWidth, SchemaBody};
use mech_engine::native_artifact::NativeProgramArtifactPlan;
use std::sync::Arc;

// pointer, initialized length, capacity, rows, columns, concrete owner
pub(super) const ARRAY_WORDS: usize = 6;
pub(super) const ARRAY_OWNER: usize = 5;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum Kind {
    /// Immutable UTF8 bytes borrowed for the complete native invocation.
    /// Constants borrow their Object data; input spans borrow admitted input
    /// storage. Neither a managed String nor an owner token enters this ABI.
    Utf8,
    Scalar {
        bytes: u8,
        signed: bool,
        float: bool,
    },
    Product(Vec<Field>),
    Tagged(Vec<Variant>),
    Array {
        element: Arc<Type>,
    },
    /// Sorted canonical entries and an optional native ordinal hash index.
    /// Map lanes are entries/count/capacity/index/mask/owner. Immutable Object
    /// constants have no index; keys borrow invocation UTF8 storage.
    SortedMap {
        entry: Arc<Type>,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Field {
    pub name: Option<String>,
    pub offset: usize,
    pub packed_offset: Option<usize>,
    pub ty: Arc<Type>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Variant {
    pub name: String,
    pub offset: usize,
    pub ty: Option<Arc<Type>>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Type {
    pub kind: Kind,
    pub words: usize,
    // Array leaves are concrete owner references, never canonical identities.
    // Tagged payloads use disjoint logical lanes; inactive owners are zero.
    pub owners: Vec<usize>,
    pub packed_bytes: Option<usize>,
    pub packed_align: usize,
}
fn align(n: usize, a: usize) -> Result<usize, NativeCompileError> {
    n.checked_add(a - 1)
        .map(|n| n & !(a - 1))
        .ok_or_else(|| backend("compact element layout overflow"))
}
fn integer(width: IntegerWidth, signed: bool) -> Result<Type, NativeCompileError> {
    let bytes = match width {
        IntegerWidth::W8 => 1,
        IntegerWidth::W16 => 2,
        IntegerWidth::W32 => 4,
        IntegerWidth::W64 => 8,
        IntegerWidth::W128 => {
            return Err(backend(
                "compact native computation does not support 128-bit integers",
            ));
        }
    };
    Ok(Type {
        kind: Kind::Scalar {
            bytes,
            signed,
            float: false,
        },
        words: 1,
        owners: vec![],
        packed_bytes: Some(bytes.into()),
        packed_align: bytes.into(),
    })
}
impl Type {
    pub fn derive(body: &SchemaBody) -> Result<Arc<Self>, NativeCompileError> {
        let result = match body {
            SchemaBody::String => Self {
                kind: Kind::Utf8,
                words: 2,
                owners: vec![],
                packed_bytes: Some(16),
                packed_align: 8,
            },
            SchemaBody::Bool => integer(IntegerWidth::W8, false)?,
            SchemaBody::UnsignedInteger(width) => integer(*width, false)?,
            SchemaBody::SignedInteger(width) => integer(*width, true)?,
            SchemaBody::IntegerInterval(interval) => return Self::derive(&interval.base_body()),
            SchemaBody::Id | SchemaBody::Index => integer(IntegerWidth::W64, false)?,
            SchemaBody::FloatingPoint(FloatWidth::W64) => Self {
                kind: Kind::Scalar {
                    bytes: 8,
                    signed: false,
                    float: true,
                },
                words: 1,
                owners: vec![],
                packed_bytes: Some(8),
                packed_align: 8,
            },
            SchemaBody::Tuple(items) => return Self::product(items.iter().map(|b| (None, b))),
            SchemaBody::Record(fields) => {
                return Self::product(fields.iter().map(|f| (Some(f.name.clone()), &f.schema)));
            }
            SchemaBody::Enum { variants, .. } => {
                let mut words = 1usize;
                let mut owners = vec![];
                let mut result = Vec::with_capacity(variants.len());
                for variant in variants {
                    let ty = variant.payload.as_ref().map(Self::derive).transpose()?;
                    let offset = words;
                    if let Some(ty) = &ty {
                        owners.extend(ty.owners.iter().map(|n| offset + n));
                        words = words
                            .checked_add(ty.words)
                            .ok_or_else(|| backend("compact tagged layout overflow"))?;
                    }
                    result.push(Variant {
                        name: variant.name.clone(),
                        offset,
                        ty,
                    });
                }
                Self {
                    kind: Kind::Tagged(result),
                    words,
                    owners,
                    packed_bytes: None,
                    packed_align: 8,
                }
            }
            SchemaBody::Map { key, value, .. } => {
                if key.as_ref() != &SchemaBody::String {
                    return Err(backend("compact native Map requires String keys"));
                }
                let entry = Self::product(
                    [key.as_ref(), value.as_ref()]
                        .into_iter()
                        .map(|b| (None, b)),
                )?;
                if !entry.owners.is_empty() || entry.packed_bytes.is_none() {
                    return Err(backend(
                        "compact native Map requires fixed borrowed/scalar values",
                    ));
                }
                Self {
                    kind: Kind::SortedMap { entry },
                    words: ARRAY_WORDS,
                    owners: vec![ARRAY_OWNER],
                    packed_bytes: None,
                    packed_align: 8,
                }
            }
            SchemaBody::Matrix {
                element,
                dimensions,
            } => {
                if dimensions.len() != 2 {
                    return Err(backend("compact typed views require two matrix dimensions"));
                }
                let element = Self::derive(element)?;
                if !element.owners.is_empty() || element.packed_bytes.is_none() {
                    return Err(backend(
                        "compact matrix element requires a fixed scalar/product layout",
                    ));
                }
                Self {
                    kind: Kind::Array { element },
                    words: ARRAY_WORDS,
                    owners: vec![ARRAY_OWNER],
                    packed_bytes: None,
                    packed_align: 8,
                }
            }
            other => {
                return Err(backend(format!(
                    "compact native computation has unsupported schema: {other:?}"
                )));
            }
        };
        Ok(Arc::new(result))
    }
    fn product<'a>(
        fields: impl Iterator<Item = (Option<String>, &'a SchemaBody)>,
    ) -> Result<Arc<Self>, NativeCompileError> {
        let mut result = vec![];
        let mut owners = vec![];
        let mut words = 0usize;
        let mut packed = Some(0usize);
        let mut alignment = 1;
        for (name, body) in fields {
            let ty = Self::derive(body)?;
            let offset = words;
            words = words
                .checked_add(ty.words)
                .ok_or_else(|| backend("compact product word count overflow"))?;
            owners.extend(ty.owners.iter().map(|n| offset + n));
            alignment = alignment.max(ty.packed_align);
            let packed_offset = packed.map(|n| align(n, ty.packed_align)).transpose()?;
            packed = packed_offset
                .zip(ty.packed_bytes)
                .map(|(a, b)| {
                    a.checked_add(b)
                        .ok_or_else(|| backend("compact product byte size overflow"))
                })
                .transpose()?;
            result.push(Field {
                name,
                offset,
                packed_offset,
                ty,
            });
        }
        let packed_bytes = packed.map(|n| align(n, alignment)).transpose()?;
        Ok(Arc::new(Self {
            kind: Kind::Product(result),
            words,
            owners,
            packed_bytes,
            packed_align: alignment,
        }))
    }
    pub(super) fn leaves(self: &Arc<Self>) -> Vec<(usize, Arc<Type>)> {
        fn visit(ty: &Arc<Type>, base: usize, out: &mut Vec<(usize, Arc<Type>)>) {
            match &ty.kind {
                Kind::Product(fields) => {
                    for field in fields {
                        visit(&field.ty, base + field.offset, out);
                    }
                }
                _ => out.push((base, Arc::clone(ty))),
            }
        }
        let mut out = Vec::new();
        visit(self, 0, &mut out);
        out
    }
    pub(super) fn at_path(&self, path: &[usize]) -> Result<(usize, &Type), NativeCompileError> {
        let mut ty = self;
        let mut offset = 0;
        for &ordinal in path {
            let field = ty
                .fields()?
                .get(ordinal)
                .ok_or_else(|| backend("native scalar path outside typed fields"))?;
            offset += field.offset;
            ty = &field.ty;
        }
        Ok((offset, ty))
    }
    pub fn fields(&self) -> Result<&[Field], NativeCompileError> {
        match &self.kind {
            Kind::Product(fields) => Ok(fields),
            _ => Err(backend("compact operation requires product fields")),
        }
    }
    pub fn element(&self) -> Result<&Arc<Type>, NativeCompileError> {
        match &self.kind {
            Kind::Array { element } => Ok(element),
            _ => Err(backend("compact operation requires an array view")),
        }
    }
    pub fn map_entry(&self) -> Result<&Arc<Type>, NativeCompileError> {
        match &self.kind {
            Kind::SortedMap { entry } => Ok(entry),
            _ => Err(backend("compact operation requires a native Map")),
        }
    }
    pub fn summary(&self) -> serde_json::Value {
        let kind = match &self.kind {
            Kind::Utf8 => {
                serde_json::json!({"utf8":"immutable invocation borrow","lanes":["pointer","byte_length"]})
            }
            Kind::Scalar {
                bytes,
                signed,
                float,
            } => serde_json::json!({"scalar_bytes":bytes,"signed":signed,"float":float}),
            Kind::Product(fields) => {
                serde_json::json!({"product":fields.iter().map(|f|serde_json::json!({"name":f.name,"offset":f.offset,"packed_offset":f.packed_offset,"type":f.ty.summary()})).collect::<Vec<_>>()})
            }
            Kind::Tagged(variants) => {
                serde_json::json!({"tagged":variants.iter().map(|v|serde_json::json!({"name":v.name,"offset":v.offset,"payload":v.ty.as_ref().map(|t|t.summary())})).collect::<Vec<_>>()})
            }
            Kind::Array { element } => serde_json::json!({"array_element":element.summary()}),
            Kind::SortedMap { entry } => {
                serde_json::json!({"sorted_map_entry":entry.summary(),"lanes":["entries","count","entry_capacity","index","index_mask","owner"]})
            }
        };
        serde_json::json!({"kind":kind,"words":self.words,"owners":self.owners,"packed_bytes":self.packed_bytes,"packed_align":self.packed_align})
    }
}

pub(super) struct Layout {
    pub types: Vec<Arc<Type>>,
    pub base: Vec<usize>,
    pub words: usize,
}
impl Layout {
    pub fn derive(plan: &NativeProgramArtifactPlan) -> Result<Self, NativeCompileError> {
        let mut schemas: Vec<(&SchemaBody, Arc<Type>)> = vec![];
        let mut types = Vec::with_capacity(plan.values().len());
        let mut base = vec![];
        let mut words = plan.values().len();
        for (value, ty) in plan.values().iter().enumerate() {
            let body = ty.definition().body();
            let layout = if let Some((_, layout)) = schemas.iter().find(|(s, _)| *s == body) {
                Arc::clone(layout)
            } else {
                let layout = Type::derive(body)?;
                schemas.push((body, Arc::clone(&layout)));
                layout
            };
            if matches!(layout.kind, Kind::Scalar { .. }) {
                base.push(value)
            } else {
                base.push(words);
                words = words
                    .checked_add(layout.words)
                    .ok_or_else(|| backend("compact native coordinates overflow"))?
            }
            types.push(layout);
        }
        // Backend offsets remain representable; this is a real representation
        // bound, not an arbitrary generated-program allowance.
        crate::native_artifact::compiler::offset(words)?;
        Ok(Self { types, base, words })
    }
    pub fn ty(&self, value: u32) -> &Arc<Type> {
        &self.types[value as usize]
    }
    /// Resolve the current canonical array provenance into this native layout.
    /// Captures keep their own typed value coordinates; they are never treated
    /// as carrier fields or reconstructed from managed physical home numbers.
    pub fn array_field(
        &self,
        array: &crate::native_artifact::program::prepare::readonly_numeric::Array,
        carrier: u32,
    ) -> Result<(usize, Arc<Type>), NativeCompileError> {
        let (value, field) = match array.storage {
            crate::native_artifact::program::prepare::readonly_numeric::ArrayStorage::Carrier { field, .. } => (carrier, Some(field)),
            crate::native_artifact::program::prepare::readonly_numeric::ArrayStorage::Capture { value } => (value, None),
            crate::native_artifact::program::prepare::readonly_numeric::ArrayStorage::FrameCapture { value, field, .. } => (value, Some(field)),
        };
        let ty = self.ty(value);
        if let Some(field) = field {
            let leaves = ty.leaves();
            let (offset, ty) = leaves
                .get(field)
                .ok_or_else(|| backend("native array provenance leaves typed fields"))?;
            Ok((self.slot(value) + offset, Arc::clone(ty)))
        } else {
            Ok((self.slot(value), Arc::clone(ty)))
        }
    }
    pub fn slot(&self, value: u32) -> usize {
        self.base[value as usize]
    }
}
impl crate::native_artifact::codegen::common::aggregates::AggregateLayout for Layout {
    fn slot(
        &self,
        value: mech_engine::native_artifact::NativeMatchValue,
    ) -> Option<(usize, usize)> {
        self.base
            .get(value.get() as usize)
            .copied()
            .zip(self.types.get(value.get() as usize).map(|t| t.words))
    }
}
