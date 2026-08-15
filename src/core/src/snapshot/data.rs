use super::{
    SnapshotValueError,
    sequence::{SequenceStorage, SequenceView},
};
use crate::{
    CanonicalNominalPath, DimensionExpr, DimensionLifetime, DimensionParameterDeclaration,
    DimensionParameterId, DimensionParameterOrigin, KindExpr, KindField, KindId,
    NamedKindPathResolver, NominalKey, SchemaKey, canonical_closed_kind_bytes,
};

#[cfg(feature = "no_std")]
use alloc::{
    borrow::ToOwned,
    boxed::Box,
    collections::{BTreeMap, BTreeSet},
    string::String,
    vec::Vec,
};
#[cfg(not(feature = "no_std"))]
use std::{
    boxed::Box,
    collections::{BTreeMap, BTreeSet},
    string::String,
};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct F32Bits(u32);

impl F32Bits {
    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    pub const fn bits(self) -> u32 {
        self.0
    }

    pub fn from_f32(value: f32) -> Self {
        Self(value.to_bits())
    }

    pub fn to_f32(self) -> f32 {
        f32::from_bits(self.0)
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct F64Bits(u64);

impl F64Bits {
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    pub const fn bits(self) -> u64 {
        self.0
    }

    pub fn from_f64(value: f64) -> Self {
        Self(value.to_bits())
    }

    pub fn to_f64(self) -> f64 {
        f64::from_bits(self.0)
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Complex32Bits {
    real: F32Bits,
    imaginary: F32Bits,
}

impl Complex32Bits {
    pub const fn new(real: F32Bits, imaginary: F32Bits) -> Self {
        Self { real, imaginary }
    }

    pub const fn real(&self) -> F32Bits {
        self.real
    }

    pub const fn imaginary(&self) -> F32Bits {
        self.imaginary
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Complex64Bits {
    real: F64Bits,
    imaginary: F64Bits,
}

impl Complex64Bits {
    pub const fn new(real: F64Bits, imaginary: F64Bits) -> Self {
        Self { real, imaginary }
    }

    pub const fn real(&self) -> F64Bits {
        self.real
    }

    pub const fn imaginary(&self) -> F64Bits {
        self.imaginary
    }
}

#[derive(Clone, Debug)]
pub struct Rational64Value {
    numerator: i64,
    denominator: u64,
}

impl Rational64Value {
    pub fn new(numerator: i64, denominator: u64) -> Result<Self, SnapshotValueError> {
        if denominator == 0
            || gcd(numerator.unsigned_abs(), denominator) != 1
            || (numerator == 0 && denominator != 1)
        {
            return Err(SnapshotValueError::NonCanonicalRationalV1);
        }
        Ok(Self {
            numerator,
            denominator,
        })
    }

    pub const fn numerator(&self) -> i64 {
        self.numerator
    }

    pub const fn denominator(&self) -> u64 {
        self.denominator
    }
}

const fn gcd(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

#[derive(Clone, Debug)]
pub enum ValueData {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    F32(F32Bits),
    F64(F64Bits),
    Complex32(Complex32Bits),
    Complex64(Complex64Bits),
    Rational64(Rational64Value),
    Bool(bool),
    String(Box<str>),
    Id(u64),
    Index(u64),
    Atom,
    Enum(EnumValue),
    Option(Option<Box<ValueData>>),
    Tuple(Box<[ValueData]>),
    Record(RecordValue),
    Matrix(MatrixValue),
    Table(TableValue),
    Set(SetValue),
    Map(MapValue),
    Type(ReifiedType),
}

impl ValueData {
    pub const fn kind(&self) -> super::ValueDataKind {
        use super::ValueDataKind;
        match self {
            Self::U8(_) => ValueDataKind::U8,
            Self::U16(_) => ValueDataKind::U16,
            Self::U32(_) => ValueDataKind::U32,
            Self::U64(_) => ValueDataKind::U64,
            Self::U128(_) => ValueDataKind::U128,
            Self::I8(_) => ValueDataKind::I8,
            Self::I16(_) => ValueDataKind::I16,
            Self::I32(_) => ValueDataKind::I32,
            Self::I64(_) => ValueDataKind::I64,
            Self::I128(_) => ValueDataKind::I128,
            Self::F32(_) => ValueDataKind::F32,
            Self::F64(_) => ValueDataKind::F64,
            Self::Complex32(_) => ValueDataKind::Complex32,
            Self::Complex64(_) => ValueDataKind::Complex64,
            Self::Rational64(_) => ValueDataKind::Rational64,
            Self::Bool(_) => ValueDataKind::Bool,
            Self::String(_) => ValueDataKind::String,
            Self::Id(_) => ValueDataKind::Id,
            Self::Index(_) => ValueDataKind::Index,
            Self::Atom => ValueDataKind::Atom,
            Self::Enum(_) => ValueDataKind::Enum,
            Self::Option(_) => ValueDataKind::Option,
            Self::Tuple(_) => ValueDataKind::Tuple,
            Self::Record(_) => ValueDataKind::Record,
            Self::Matrix(_) => ValueDataKind::Matrix,
            Self::Table(_) => ValueDataKind::Table,
            Self::Set(_) => ValueDataKind::Set,
            Self::Map(_) => ValueDataKind::Map,
            Self::Type(_) => ValueDataKind::ReifiedType,
        }
    }
}

#[derive(Clone, Debug)]
pub struct EnumValue {
    pub(super) ordinal: u32,
    pub(super) payload: Option<Box<ValueData>>,
}

impl EnumValue {
    pub const fn ordinal(&self) -> u32 {
        self.ordinal
    }

    pub fn payload(&self) -> Option<&ValueData> {
        self.payload.as_deref()
    }
}

#[derive(Clone, Debug)]
pub struct RecordValue {
    /// Stored in schema field order.
    pub(super) fields: Box<[ValueData]>,
}

impl RecordValue {
    pub fn fields(&self) -> &[ValueData] {
        &self.fields
    }
}

#[derive(Clone, Debug)]
pub struct MatrixValue {
    pub(super) elements: SequenceStorage,
}

impl MatrixValue {
    pub fn elements(&self) -> SequenceView<'_> {
        self.elements.view()
    }
}

#[derive(Clone, Debug)]
pub struct TableValue {
    /// Stored in schema column order.
    pub(super) columns: Box<[SequenceStorage]>,
}

impl TableValue {
    pub fn len(&self) -> usize {
        self.columns.len()
    }

    pub fn is_empty(&self) -> bool {
        self.columns.is_empty()
    }

    pub fn column(&self, index: usize) -> Option<SequenceView<'_>> {
        self.columns.get(index).map(SequenceStorage::view)
    }
}

#[derive(Clone, Debug)]
pub struct SetValue {
    /// Canonical key order.
    pub(super) elements: Box<[CanonicalKeyValue]>,
}

impl SetValue {
    pub fn elements(&self) -> &[CanonicalKeyValue] {
        &self.elements
    }
}

#[derive(Clone, Debug)]
pub struct MapValue {
    /// Canonical key order.
    pub(super) entries: Box<[MapEntryValue]>,
}

impl MapValue {
    pub fn entries(&self) -> &[MapEntryValue] {
        &self.entries
    }
}

#[derive(Clone, Debug)]
pub struct MapEntryValue {
    pub(super) key: CanonicalKeyValue,
    pub(super) value: ValueData,
}

impl MapEntryValue {
    pub fn key(&self) -> &CanonicalKeyValue {
        &self.key
    }

    pub fn value(&self) -> &ValueData {
        &self.value
    }
}

#[derive(Clone, Debug)]
pub struct CanonicalKeyValue {
    pub(super) data: ValueData,
}

impl CanonicalKeyValue {
    pub fn data(&self) -> &ValueData {
        &self.data
    }
}

#[derive(Clone, Debug)]
pub enum ReifiedType {
    Kind(ReifiedKind),
    Schema(SchemaKey),
}

#[derive(Clone, Debug)]
pub struct ReifiedKind {
    canonical_bytes: Box<[u8]>,
}

struct NoNamedKinds;

impl NamedKindPathResolver for NoNamedKinds {
    fn canonical_path(&self, _id: KindId) -> Option<&CanonicalNominalPath> {
        None
    }
}

fn kind_requires_named_resolver(kind: &KindExpr) -> bool {
    match kind {
        KindExpr::Named(_) => true,
        KindExpr::Matrix { element, .. }
        | KindExpr::Option(element)
        | KindExpr::Set { element, .. }
        | KindExpr::Reference(element)
        | KindExpr::TypeOf(element) => kind_requires_named_resolver(element),
        KindExpr::Tuple(elements) => elements.iter().any(kind_requires_named_resolver),
        KindExpr::Record(fields) => fields
            .iter()
            .any(|field| kind_requires_named_resolver(&field.kind)),
        KindExpr::Table { columns, .. } => columns
            .iter()
            .any(|column| kind_requires_named_resolver(&column.kind)),
        KindExpr::Map { key, value, .. } => {
            kind_requires_named_resolver(key) || kind_requires_named_resolver(value)
        }
        KindExpr::Wildcard
        | KindExpr::Never
        | KindExpr::Hole
        | KindExpr::Parameter(_)
        | KindExpr::Id
        | KindExpr::Index
        | KindExpr::Atom(_)
        | KindExpr::Enum(_) => false,
    }
}

impl ReifiedKind {
    pub fn from_closed_kind(
        kind: &KindExpr,
        dimensions: &[DimensionParameterDeclaration],
        named_kinds: &dyn NamedKindPathResolver,
    ) -> Result<Self, SnapshotValueError> {
        Ok(Self {
            canonical_bytes: canonical_closed_kind_bytes(kind, dimensions, named_kinds)?,
        })
    }

    pub(crate) fn from_closed_kind_with_optional_resolver(
        kind: &KindExpr,
        dimensions: &[DimensionParameterDeclaration],
        named_kinds: Option<&dyn NamedKindPathResolver>,
    ) -> Result<Self, SnapshotValueError> {
        if kind_requires_named_resolver(kind) {
            let resolver = named_kinds.ok_or(SnapshotValueError::MissingNamedKindResolver)?;
            return Self::from_closed_kind(kind, dimensions, resolver);
        }
        Self::from_closed_kind(kind, dimensions, &NoNamedKinds)
    }

    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    /// Reconstructs a reified kind from its bytecode-v1 semantic material.
    ///
    /// The bytes are accepted only when they are a complete, structurally
    /// canonical closed-kind encoding. This route deliberately does not
    /// recreate a legacy `Kind` value.
    pub fn from_canonical_bytes(
        canonical_bytes: impl Into<Box<[u8]>>,
    ) -> Result<Self, SnapshotValueError> {
        let canonical_bytes = canonical_bytes.into();
        let (kind, dimensions, named_kinds) = decode_canonical_reified_kind(&canonical_bytes)?;
        let reencoded = canonical_closed_kind_bytes(&kind, &dimensions, &named_kinds)
            .map_err(|_| SnapshotValueError::InvalidCanonicalReifiedKindV1)?;
        if reencoded.as_ref() != canonical_bytes.as_ref() {
            return invalid_reified_kind();
        }
        Ok(Self { canonical_bytes })
    }
}

const MAX_REIFIED_KIND_BYTES: usize = 16_777_216;
const MAX_REIFIED_KIND_DEPTH: usize = 256;

fn invalid_reified_kind<T>() -> Result<T, SnapshotValueError> {
    Err(SnapshotValueError::InvalidCanonicalReifiedKindV1)
}

struct CanonicalKindReader<'a> {
    bytes: &'a [u8],
    offset: usize,
    depth: usize,
    parameter_count: u32,
}

impl<'a> CanonicalKindReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            offset: 0,
            depth: 0,
            parameter_count: 0,
        }
    }

    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.offset)
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], SnapshotValueError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or(SnapshotValueError::InvalidCanonicalReifiedKindV1)?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(SnapshotValueError::InvalidCanonicalReifiedKindV1)?;
        self.offset = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, SnapshotValueError> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, SnapshotValueError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().map_err(
            |_| SnapshotValueError::InvalidCanonicalReifiedKindV1,
        )?))
    }

    fn u64(&mut self) -> Result<u64, SnapshotValueError> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().map_err(
            |_| SnapshotValueError::InvalidCanonicalReifiedKindV1,
        )?))
    }

    fn node(&mut self) -> Result<CanonicalKindReader<'a>, SnapshotValueError> {
        let depth = self
            .depth
            .checked_add(1)
            .ok_or(SnapshotValueError::InvalidCanonicalReifiedKindV1)?;
        if depth > MAX_REIFIED_KIND_DEPTH {
            return invalid_reified_kind();
        }
        let length = usize::try_from(self.u64()?)
            .map_err(|_| SnapshotValueError::InvalidCanonicalReifiedKindV1)?;
        let mut child = Self::new(self.take(length)?);
        child.depth = depth;
        child.parameter_count = self.parameter_count;
        Ok(child)
    }

    fn finish(self) -> Result<(), SnapshotValueError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            invalid_reified_kind()
        }
    }
}

#[derive(Default)]
struct DecodedNamedKinds {
    ids: BTreeMap<Box<[u8]>, KindId>,
    paths: BTreeMap<KindId, CanonicalNominalPath>,
}

impl DecodedNamedKinds {
    fn intern(&mut self, path: CanonicalNominalPath) -> Result<KindId, SnapshotValueError> {
        let canonical = path.canonical_bytes();
        if let Some(id) = self.ids.get(&canonical) {
            return Ok(*id);
        }
        let raw = u32::try_from(self.paths.len())
            .map_err(|_| SnapshotValueError::InvalidCanonicalReifiedKindV1)?;
        let id = KindId::new(raw);
        self.ids.insert(canonical, id);
        self.paths.insert(id, path);
        Ok(id)
    }
}

impl NamedKindPathResolver for DecodedNamedKinds {
    fn canonical_path(&self, id: KindId) -> Option<&CanonicalNominalPath> {
        self.paths.get(&id)
    }
}

fn decode_canonical_reified_kind(
    bytes: &[u8],
) -> Result<
    (
        KindExpr,
        Box<[DimensionParameterDeclaration]>,
        DecodedNamedKinds,
    ),
    SnapshotValueError,
> {
    if bytes.len() > MAX_REIFIED_KIND_BYTES {
        return invalid_reified_kind();
    }
    let mut reader = CanonicalKindReader::new(bytes);
    if reader.u8()? != 1 {
        return invalid_reified_kind();
    }
    reader.parameter_count = reader.u32()?;
    let parameter_count = usize::try_from(reader.parameter_count)
        .map_err(|_| SnapshotValueError::InvalidCanonicalReifiedKindV1)?;
    if parameter_count > reader.remaining() {
        return invalid_reified_kind();
    }

    let mut dimensions = Vec::with_capacity(parameter_count);
    for id in 0..reader.parameter_count {
        let lifetime = match reader.u8()? {
            1 => DimensionLifetime::Activation,
            2 => DimensionLifetime::Turn,
            _ => return invalid_reified_kind(),
        };
        let lower_bound = decode_dimension_child(&mut reader)?;
        let upper_bound = match reader.u8()? {
            0 => None,
            1 => Some(decode_dimension_child(&mut reader)?),
            _ => return invalid_reified_kind(),
        };
        dimensions.push(DimensionParameterDeclaration {
            id: DimensionParameterId::new(id),
            origin: DimensionParameterOrigin::Explicit,
            lifetime,
            lower_bound,
            upper_bound,
        });
    }

    let mut named_kinds = DecodedNamedKinds::default();
    let mut body = reader.node()?;
    let kind = decode_canonical_kind_body(&mut body, &mut named_kinds)?;
    body.finish()?;
    reader.finish()?;
    Ok((kind, dimensions.into_boxed_slice(), named_kinds))
}

fn decode_canonical_dimension(
    reader: &mut CanonicalKindReader<'_>,
) -> Result<DimensionExpr, SnapshotValueError> {
    Ok(match reader.u8()? {
        1 => DimensionExpr::Constant(reader.u64()?),
        2 => {
            let id = reader.u32()?;
            if id >= reader.parameter_count {
                return invalid_reified_kind();
            }
            DimensionExpr::Parameter(DimensionParameterId::new(id))
        }
        tag @ 3..=6 => {
            let count = bounded_count(reader)?;
            let mut children = Vec::with_capacity(count as usize);
            for _ in 0..count {
                children.push(decode_dimension_child(reader)?);
            }
            let children = children.into_boxed_slice();
            match tag {
                3 => DimensionExpr::Add(children),
                4 => DimensionExpr::Multiply(children),
                5 => DimensionExpr::Min(children),
                6 => DimensionExpr::Max(children),
                _ => unreachable!(),
            }
        }
        _ => return invalid_reified_kind(),
    })
}

fn decode_canonical_kind_body(
    reader: &mut CanonicalKindReader<'_>,
    named_kinds: &mut DecodedNamedKinds,
) -> Result<KindExpr, SnapshotValueError> {
    Ok(match reader.u8()? {
        1 => KindExpr::Wildcard,
        2 => KindExpr::Never,
        4 => KindExpr::Named(named_kinds.intern(decode_canonical_nominal_path(reader)?)?),
        5 => KindExpr::Id,
        6 => KindExpr::Index,
        7 => KindExpr::Atom(decode_nominal_key(reader)?),
        8 => KindExpr::Enum(decode_nominal_key(reader)?),
        9 => {
            let element = Box::new(decode_kind_child(reader, named_kinds)?);
            let count = bounded_count(reader)?;
            let mut dimensions = Vec::with_capacity(count as usize);
            for _ in 0..count {
                dimensions.push(decode_dimension_child(reader)?);
            }
            KindExpr::Matrix {
                element,
                dimensions: dimensions.into_boxed_slice(),
            }
        }
        10 => KindExpr::Option(Box::new(decode_kind_child(reader, named_kinds)?)),
        11 => {
            let count = bounded_count(reader)?;
            let mut elements = Vec::with_capacity(count as usize);
            for _ in 0..count {
                elements.push(decode_kind_child(reader, named_kinds)?);
            }
            KindExpr::Tuple(elements.into_boxed_slice())
        }
        12 => KindExpr::Record(decode_kind_fields(reader, named_kinds)?),
        13 => KindExpr::Table {
            columns: decode_kind_fields(reader, named_kinds)?,
            rows: decode_dimension_child(reader)?,
        },
        14 => KindExpr::Set {
            element: Box::new(decode_kind_child(reader, named_kinds)?),
            cardinality: decode_dimension_child(reader)?,
        },
        15 => KindExpr::Map {
            key: Box::new(decode_kind_child(reader, named_kinds)?),
            value: Box::new(decode_kind_child(reader, named_kinds)?),
            cardinality: decode_dimension_child(reader)?,
        },
        16 => KindExpr::Reference(Box::new(decode_kind_child(reader, named_kinds)?)),
        17 => KindExpr::TypeOf(Box::new(decode_kind_child(reader, named_kinds)?)),
        _ => return invalid_reified_kind(),
    })
}

fn bounded_count(reader: &mut CanonicalKindReader<'_>) -> Result<u32, SnapshotValueError> {
    let count = reader.u32()?;
    if usize::try_from(count).ok().is_none()
        || usize::try_from(count).unwrap() > reader.remaining() / 8
    {
        return invalid_reified_kind();
    }
    Ok(count)
}

fn decode_kind_child(
    reader: &mut CanonicalKindReader<'_>,
    named_kinds: &mut DecodedNamedKinds,
) -> Result<KindExpr, SnapshotValueError> {
    let mut child = reader.node()?;
    let kind = decode_canonical_kind_body(&mut child, named_kinds)?;
    child.finish()?;
    Ok(kind)
}

fn decode_dimension_child(
    reader: &mut CanonicalKindReader<'_>,
) -> Result<DimensionExpr, SnapshotValueError> {
    let mut child = reader.node()?;
    let dimension = decode_canonical_dimension(&mut child)?;
    child.finish()?;
    Ok(dimension)
}

fn decode_kind_fields(
    reader: &mut CanonicalKindReader<'_>,
    named_kinds: &mut DecodedNamedKinds,
) -> Result<Box<[KindField]>, SnapshotValueError> {
    let count = bounded_count(reader)?;
    let mut names = BTreeSet::new();
    let mut fields = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let length = usize::try_from(reader.u64()?)
            .map_err(|_| SnapshotValueError::InvalidCanonicalReifiedKindV1)?;
        let name = core::str::from_utf8(reader.take(length)?)
            .map_err(|_| SnapshotValueError::InvalidCanonicalReifiedKindV1)?;
        if name.is_empty() || !names.insert(name.to_owned()) {
            return invalid_reified_kind();
        }
        fields.push(KindField {
            name: name.to_owned(),
            kind: decode_kind_child(reader, named_kinds)?,
        });
    }
    Ok(fields.into_boxed_slice())
}

fn decode_canonical_nominal_path(
    reader: &mut CanonicalKindReader<'_>,
) -> Result<CanonicalNominalPath, SnapshotValueError> {
    let count = bounded_count(reader)?;
    if count == 0 {
        return invalid_reified_kind();
    }
    let mut segments = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let length = usize::try_from(reader.u64()?)
            .map_err(|_| SnapshotValueError::InvalidCanonicalReifiedKindV1)?;
        let segment = core::str::from_utf8(reader.take(length)?)
            .map_err(|_| SnapshotValueError::InvalidCanonicalReifiedKindV1)?;
        segments.push(segment.to_owned());
    }
    CanonicalNominalPath::new(segments.into_boxed_slice())
        .map_err(|_| SnapshotValueError::InvalidCanonicalReifiedKindV1)
}

fn decode_nominal_key(
    reader: &mut CanonicalKindReader<'_>,
) -> Result<NominalKey, SnapshotValueError> {
    let bytes = reader
        .take(32)?
        .try_into()
        .map_err(|_| SnapshotValueError::InvalidCanonicalReifiedKindV1)?;
    Ok(NominalKey::from_bytes(bytes))
}

#[cfg(test)]
mod reified_kind_wire_tests {
    use super::*;

    fn push_node(bytes: &mut Vec<u8>, node: &[u8]) {
        bytes.extend_from_slice(&(node.len() as u64).to_le_bytes());
        bytes.extend_from_slice(node);
    }

    fn noncanonical_matrix_dimension(operator: u8, identity: u64) -> Vec<u8> {
        let mut dimension = vec![operator];
        dimension.extend_from_slice(&2_u32.to_le_bytes());
        push_node(&mut dimension, &[2, 0, 0, 0, 0]);
        let mut constant = vec![1];
        constant.extend_from_slice(&identity.to_le_bytes());
        push_node(&mut dimension, &constant);

        let mut body = vec![9];
        push_node(&mut body, &[5]);
        body.extend_from_slice(&1_u32.to_le_bytes());
        push_node(&mut body, &dimension);

        let mut bytes = vec![1];
        bytes.extend_from_slice(&1_u32.to_le_bytes());
        bytes.push(1);
        let mut lower = vec![1];
        lower.extend_from_slice(&0_u64.to_le_bytes());
        push_node(&mut bytes, &lower);
        bytes.push(0);
        push_node(&mut bytes, &body);
        bytes
    }

    #[test]
    fn canonical_kind_bytes_reconstruct_without_a_legacy_kind() {
        let kind = KindExpr::Tuple(vec![KindExpr::Id, KindExpr::Index].into_boxed_slice());
        let original =
            ReifiedKind::from_closed_kind_with_optional_resolver(&kind, &[], None).unwrap();
        let reconstructed =
            ReifiedKind::from_canonical_bytes(original.canonical_bytes().to_vec()).unwrap();
        assert_eq!(reconstructed.canonical_bytes(), original.canonical_bytes());
    }

    #[test]
    fn canonical_parameterized_kind_bytes_reconstruct_exactly() {
        let dimensions = [DimensionParameterDeclaration {
            id: DimensionParameterId::new(0),
            origin: DimensionParameterOrigin::Inferred,
            lifetime: DimensionLifetime::Activation,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: Some(DimensionExpr::Constant(16)),
        }];
        let kind = KindExpr::Matrix {
            element: Box::new(KindExpr::Id),
            dimensions: vec![DimensionExpr::Parameter(DimensionParameterId::new(0))]
                .into_boxed_slice(),
        };
        let original =
            ReifiedKind::from_closed_kind_with_optional_resolver(&kind, &dimensions, None).unwrap();
        let reconstructed =
            ReifiedKind::from_canonical_bytes(original.canonical_bytes().to_vec()).unwrap();
        assert_eq!(reconstructed.canonical_bytes(), original.canonical_bytes());
    }

    #[test]
    fn malformed_canonical_kind_bytes_are_rejected() {
        for bytes in [Vec::new(), vec![2], vec![1, 0, 0, 0, 0, 1]] {
            assert!(matches!(
                ReifiedKind::from_canonical_bytes(bytes),
                Err(SnapshotValueError::InvalidCanonicalReifiedKindV1)
            ));
        }
    }

    #[test]
    fn nominal_path_segment_count_is_bounded_before_allocation() {
        let mut named = vec![4];
        named.extend_from_slice(&u32::MAX.to_le_bytes());
        let mut bytes = vec![1];
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        push_node(&mut bytes, &named);

        assert!(matches!(
            ReifiedKind::from_canonical_bytes(bytes),
            Err(SnapshotValueError::InvalidCanonicalReifiedKindV1)
        ));
    }

    #[test]
    fn structurally_valid_noncanonical_dimensions_are_rejected() {
        for bytes in [
            noncanonical_matrix_dimension(3, 0),
            noncanonical_matrix_dimension(4, 1),
        ] {
            assert!(matches!(
                ReifiedKind::from_canonical_bytes(bytes),
                Err(SnapshotValueError::InvalidCanonicalReifiedKindV1)
            ));
        }
    }
}
