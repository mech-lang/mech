//! Authoritative builtin semantic kinds and compiler-provided kind predicates.

use crate::{
    CanonicalNominalPath, FloatWidth, IntegerWidth, KindExpr, KindId, SchemaBody,
    SemanticModelError,
};

#[cfg(feature = "no_std")]
use alloc::{borrow::ToOwned, string::String};
#[cfg(not(feature = "no_std"))]
use std::string::String;

/// Stable semantic identities for every builtin scalar kind.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u16)]
pub enum BuiltinScalarKind {
    U8 = 0,
    U16 = 1,
    U32 = 2,
    U64 = 3,
    U128 = 4,
    I8 = 5,
    I16 = 6,
    I32 = 7,
    I64 = 8,
    I128 = 9,
    F32 = 10,
    F64 = 11,
    C64 = 12,
    R64 = 13,
    String = 14,
    Bool = 15,
    C32 = 16,
}

impl BuiltinScalarKind {
    pub const ALL: [Self; 17] = [
        Self::U8,
        Self::U16,
        Self::U32,
        Self::U64,
        Self::U128,
        Self::I8,
        Self::I16,
        Self::I32,
        Self::I64,
        Self::I128,
        Self::F32,
        Self::F64,
        Self::C64,
        Self::R64,
        Self::String,
        Self::Bool,
        Self::C32,
    ];

    pub const fn kind_id(self) -> KindId {
        KindId::new(self as u16 as u32)
    }

    pub const fn from_kind_id(id: KindId) -> Option<Self> {
        match id.get() {
            0 => Some(Self::U8),
            1 => Some(Self::U16),
            2 => Some(Self::U32),
            3 => Some(Self::U64),
            4 => Some(Self::U128),
            5 => Some(Self::I8),
            6 => Some(Self::I16),
            7 => Some(Self::I32),
            8 => Some(Self::I64),
            9 => Some(Self::I128),
            10 => Some(Self::F32),
            11 => Some(Self::F64),
            12 => Some(Self::C64),
            13 => Some(Self::R64),
            14 => Some(Self::String),
            15 => Some(Self::Bool),
            16 => Some(Self::C32),
            _ => None,
        }
    }

    pub const fn from_schema_body(body: &SchemaBody) -> Option<Self> {
        match body {
            SchemaBody::UnsignedInteger(IntegerWidth::W8) => Some(Self::U8),
            SchemaBody::UnsignedInteger(IntegerWidth::W16) => Some(Self::U16),
            SchemaBody::UnsignedInteger(IntegerWidth::W32) => Some(Self::U32),
            SchemaBody::UnsignedInteger(IntegerWidth::W64) => Some(Self::U64),
            SchemaBody::UnsignedInteger(IntegerWidth::W128) => Some(Self::U128),
            SchemaBody::SignedInteger(IntegerWidth::W8) => Some(Self::I8),
            SchemaBody::SignedInteger(IntegerWidth::W16) => Some(Self::I16),
            SchemaBody::SignedInteger(IntegerWidth::W32) => Some(Self::I32),
            SchemaBody::SignedInteger(IntegerWidth::W64) => Some(Self::I64),
            SchemaBody::SignedInteger(IntegerWidth::W128) => Some(Self::I128),
            SchemaBody::FloatingPoint(FloatWidth::W32) => Some(Self::F32),
            SchemaBody::FloatingPoint(FloatWidth::W64) => Some(Self::F64),
            SchemaBody::Complex(FloatWidth::W32) => Some(Self::C32),
            SchemaBody::Complex(FloatWidth::W64) => Some(Self::C64),
            SchemaBody::Rational64 => Some(Self::R64),
            SchemaBody::String => Some(Self::String),
            SchemaBody::Bool => Some(Self::Bool),
            _ => None,
        }
    }

    pub fn schema_body(self) -> SchemaBody {
        match self {
            Self::U8 => SchemaBody::UnsignedInteger(IntegerWidth::W8),
            Self::U16 => SchemaBody::UnsignedInteger(IntegerWidth::W16),
            Self::U32 => SchemaBody::UnsignedInteger(IntegerWidth::W32),
            Self::U64 => SchemaBody::UnsignedInteger(IntegerWidth::W64),
            Self::U128 => SchemaBody::UnsignedInteger(IntegerWidth::W128),
            Self::I8 => SchemaBody::SignedInteger(IntegerWidth::W8),
            Self::I16 => SchemaBody::SignedInteger(IntegerWidth::W16),
            Self::I32 => SchemaBody::SignedInteger(IntegerWidth::W32),
            Self::I64 => SchemaBody::SignedInteger(IntegerWidth::W64),
            Self::I128 => SchemaBody::SignedInteger(IntegerWidth::W128),
            Self::F32 => SchemaBody::FloatingPoint(FloatWidth::W32),
            Self::F64 => SchemaBody::FloatingPoint(FloatWidth::W64),
            Self::C32 => SchemaBody::Complex(FloatWidth::W32),
            Self::C64 => SchemaBody::Complex(FloatWidth::W64),
            Self::R64 => SchemaBody::Rational64,
            Self::String => SchemaBody::String,
            Self::Bool => SchemaBody::Bool,
        }
    }

    pub const fn canonical_name(self) -> &'static str {
        match self {
            Self::U8 => "u8",
            Self::U16 => "u16",
            Self::U32 => "u32",
            Self::U64 => "u64",
            Self::U128 => "u128",
            Self::I8 => "i8",
            Self::I16 => "i16",
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::I128 => "i128",
            Self::F32 => "f32",
            Self::F64 => "f64",
            Self::C32 => "c32",
            Self::C64 => "c64",
            Self::R64 => "r64",
            Self::String => "string",
            Self::Bool => "bool",
        }
    }

    pub fn canonical_path(self) -> Result<CanonicalNominalPath, SemanticModelError> {
        CanonicalNominalPath::new([
            "mech".to_owned(),
            "builtin".to_owned(),
            "scalar".to_owned(),
            self.canonical_name().to_owned(),
        ])
    }

    pub const fn kind_expr(self) -> KindExpr {
        KindExpr::Named(self.kind_id())
    }

    pub(crate) const fn satisfies(self, predicate: BuiltinKindPredicate) -> bool {
        use BuiltinKindPredicate as C;
        use BuiltinScalarKind as K;
        match predicate {
            C::Number => !matches!(self, K::String | K::Bool),
            C::Real => !matches!(self, K::C32 | K::C64 | K::String | K::Bool),
            C::Integer => matches!(
                self,
                K::U8
                    | K::U16
                    | K::U32
                    | K::U64
                    | K::U128
                    | K::I8
                    | K::I16
                    | K::I32
                    | K::I64
                    | K::I128
            ),
            C::FloatingPoint => matches!(self, K::F32 | K::F64),
            C::Ordered => !matches!(self, K::C32 | K::C64 | K::Bool),
            C::Negatable => matches!(
                self,
                K::I8
                    | K::I16
                    | K::I32
                    | K::I64
                    | K::I128
                    | K::F32
                    | K::F64
                    | K::C32
                    | K::C64
                    | K::R64
            ),
            C::RangeEndpoint => !matches!(self, K::C32 | K::C64 | K::R64 | K::String | K::Bool),
            C::Equatable => true,
            C::Keyable => !matches!(self, K::C32 | K::C64),
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u16)]
pub enum BuiltinKindPredicate {
    Number,
    Real,
    Integer,
    FloatingPoint,
    Ordered,
    Negatable,
    RangeEndpoint,
    Equatable,
    Keyable,
}

impl BuiltinKindPredicate {
    pub const ALL: [Self; 9] = [
        Self::Number,
        Self::Real,
        Self::Integer,
        Self::FloatingPoint,
        Self::Ordered,
        Self::Negatable,
        Self::RangeEndpoint,
        Self::Equatable,
        Self::Keyable,
    ];

    const fn bit(self) -> u16 {
        1u16 << self as u16
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct BuiltinKindPredicateSet(u16);

impl BuiltinKindPredicateSet {
    pub(crate) const fn empty() -> Self {
        Self(0)
    }

    pub(crate) fn insert(&mut self, predicate: BuiltinKindPredicate) {
        self.0 |= predicate.bit();
    }

    pub(crate) const fn contains(self, predicate: BuiltinKindPredicate) -> bool {
        self.0 & predicate.bit() != 0
    }

    pub(crate) const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }
}

pub fn builtin_scalar_from_name_hash(scalar_id: u64) -> Option<BuiltinScalarKind> {
    BuiltinScalarKind::ALL
        .into_iter()
        .find(|kind| crate::hash_str(kind.canonical_name()) == scalar_id)
}

pub fn builtin_scalar_name(kind: &KindExpr) -> Option<&'static str> {
    match kind {
        KindExpr::Named(id) => {
            BuiltinScalarKind::from_kind_id(*id).map(BuiltinScalarKind::canonical_name)
        }
        _ => None,
    }
}

pub fn builtin_scalar_schema_name(body: &SchemaBody) -> Option<String> {
    BuiltinScalarKind::from_schema_body(body).map(|kind| kind.canonical_name().into())
}

pub(crate) fn intrinsic_kind_satisfies_predicate(
    kind: &KindExpr,
    predicate: BuiltinKindPredicate,
) -> bool {
    match kind {
        KindExpr::Named(id) => {
            BuiltinScalarKind::from_kind_id(*id).is_some_and(|kind| kind.satisfies(predicate))
        }
        KindExpr::Index => matches!(
            predicate,
            BuiltinKindPredicate::Ordered
                | BuiltinKindPredicate::RangeEndpoint
                | BuiltinKindPredicate::Equatable
                | BuiltinKindPredicate::Keyable
        ),
        KindExpr::Id | KindExpr::Atom(_) => matches!(
            predicate,
            BuiltinKindPredicate::Equatable | BuiltinKindPredicate::Keyable
        ),
        KindExpr::TypeOf(_) => matches!(predicate, BuiltinKindPredicate::Equatable),
        _ => false,
    }
}

/// Computes the closed compiler-defined predicates for one kind after its
/// children have been classified. Empty structural products use ordinary
/// vacuous `all` semantics, so `()` and `{}` are both equatable and keyable.
pub(crate) fn intrinsic_kind_predicates(
    kind: &KindExpr,
    children: &[BuiltinKindPredicateSet],
) -> BuiltinKindPredicateSet {
    let mut predicates = BuiltinKindPredicateSet::empty();
    for predicate in BuiltinKindPredicate::ALL {
        if intrinsic_kind_satisfies_predicate(kind, predicate) {
            predicates.insert(predicate);
        }
    }
    if matches!(
        kind,
        KindExpr::Option(_)
            | KindExpr::Matrix { .. }
            | KindExpr::Set { .. }
            | KindExpr::Tuple(_)
            | KindExpr::Record(_)
            | KindExpr::Table { .. }
            | KindExpr::Map { .. }
    ) && children
        .iter()
        .all(|predicates| predicates.contains(BuiltinKindPredicate::Equatable))
    {
        predicates.insert(BuiltinKindPredicate::Equatable);
    }
    if matches!(
        kind,
        KindExpr::Option(_)
            | KindExpr::Matrix { .. }
            | KindExpr::Set { .. }
            | KindExpr::Tuple(_)
            | KindExpr::Record(_)
    ) && children
        .iter()
        .all(|predicates| predicates.contains(BuiltinKindPredicate::Keyable))
    {
        predicates.insert(BuiltinKindPredicate::Keyable);
    }
    predicates
}
