//! Compatibility-only fixed-width and UTF-8 scalar decoding for bytecode-v1 constants.

use crate::{LegacyValue, MResult, Ref};

#[cfg(any(feature = "string", feature = "variable_define"))]
use super::super::owned_utf8;
use super::{RuntimeType, invalid};

macro_rules! fixed {
    ($feature:literal, $variant:ident, $primitive:ty, $width:expr, $bytes:expr) => {{
        #[cfg(feature = $feature)]
        {
            let raw: [u8; $width] = $bytes.try_into().map_err(|_| {
                invalid::<()>(concat!(
                    stringify!($variant),
                    " constant has an invalid byte length"
                ))
                .unwrap_err()
            })?;
            Ok(LegacyValue::$variant(Ref::new(
                <$primitive>::from_le_bytes(raw),
            )))
        }
        #[cfg(not(feature = $feature))]
        {
            invalid(concat!(
                stringify!($variant),
                " constants are unavailable in this runtime"
            ))
        }
    }};
}

macro_rules! float {
    ($feature:literal, $variant:ident, $primitive:ty, $bits:ty, $width:expr, $bytes:expr) => {{
        #[cfg(feature = $feature)]
        {
            let raw: [u8; $width] = $bytes.try_into().map_err(|_| {
                invalid::<()>(concat!(
                    stringify!($variant),
                    " constant has an invalid byte length"
                ))
                .unwrap_err()
            })?;
            Ok(LegacyValue::$variant(Ref::new(<$primitive>::from_bits(
                <$bits>::from_le_bytes(raw),
            ))))
        }
        #[cfg(not(feature = $feature))]
        {
            invalid(concat!(
                stringify!($variant),
                " constants are unavailable in this runtime"
            ))
        }
    }};
}

/// Decode a scalar type, returning `None` for composite runtime types.
pub(super) fn decode(ty: &RuntimeType, bytes: &[u8]) -> Option<MResult<LegacyValue>> {
    if !matches!(
        ty,
        RuntimeType::Empty
            | RuntimeType::Bool
            | RuntimeType::String
            | RuntimeType::U8
            | RuntimeType::U16
            | RuntimeType::U32
            | RuntimeType::U64
            | RuntimeType::U128
            | RuntimeType::I8
            | RuntimeType::I16
            | RuntimeType::I32
            | RuntimeType::I64
            | RuntimeType::I128
            | RuntimeType::F32
            | RuntimeType::F64
            | RuntimeType::Id
            | RuntimeType::Index
            | RuntimeType::C64
            | RuntimeType::R64
    ) {
        return None;
    }

    Some((|| -> MResult<LegacyValue> {
        match ty {
            RuntimeType::Empty => {
                if bytes.is_empty() {
                    Ok(LegacyValue::Empty)
                } else {
                    invalid("Empty constant must have zero payload bytes")
                }
            }
            RuntimeType::Bool => {
                #[cfg(any(feature = "bool", feature = "variable_define"))]
                {
                    match bytes {
                        [0] => Ok(LegacyValue::Bool(Ref::new(false))),
                        [1] => Ok(LegacyValue::Bool(Ref::new(true))),
                        _ => invalid("Bool constant must be exactly 0x00 or 0x01"),
                    }
                }
                #[cfg(not(any(feature = "bool", feature = "variable_define")))]
                {
                    invalid("Bool constants are unavailable in this runtime")
                }
            }
            RuntimeType::String => {
                #[cfg(any(feature = "string", feature = "variable_define"))]
                {
                    let value = owned_utf8(bytes, "String constant")
                        .map_err(|_| invalid::<()>("invalid UTF-8 String constant").unwrap_err())?;
                    Ok(LegacyValue::String(Ref::new(value)))
                }
                #[cfg(not(any(feature = "string", feature = "variable_define")))]
                {
                    invalid("String constants are unavailable in this runtime")
                }
            }
            RuntimeType::U8 => fixed!("u8", U8, u8, 1, bytes),
            RuntimeType::U16 => fixed!("u16", U16, u16, 2, bytes),
            RuntimeType::U32 => fixed!("u32", U32, u32, 4, bytes),
            RuntimeType::U64 => fixed!("u64", U64, u64, 8, bytes),
            RuntimeType::U128 => fixed!("u128", U128, u128, 16, bytes),
            RuntimeType::I8 => fixed!("i8", I8, i8, 1, bytes),
            RuntimeType::I16 => fixed!("i16", I16, i16, 2, bytes),
            RuntimeType::I32 => fixed!("i32", I32, i32, 4, bytes),
            RuntimeType::I64 => fixed!("i64", I64, i64, 8, bytes),
            RuntimeType::I128 => fixed!("i128", I128, i128, 16, bytes),
            RuntimeType::F32 => float!("f32", F32, f32, u32, 4, bytes),
            RuntimeType::F64 => float!("f64", F64, f64, u64, 8, bytes),
            RuntimeType::Id => {
                let raw: [u8; 8] = bytes.try_into().map_err(|_| {
                    invalid::<()>("Id constant must contain eight bytes").unwrap_err()
                })?;
                Ok(LegacyValue::Id(u64::from_le_bytes(raw)))
            }
            RuntimeType::Index => {
                let raw: [u8; 8] = bytes.try_into().map_err(|_| {
                    invalid::<()>("Index constant must contain eight bytes").unwrap_err()
                })?;
                let value = usize::try_from(u64::from_le_bytes(raw))
                    .map_err(|_| invalid::<()>("Index constant exceeds usize").unwrap_err())?;
                Ok(LegacyValue::Index(Ref::new(value)))
            }
            RuntimeType::C64 => {
                #[cfg(feature = "complex")]
                {
                    let raw: [u8; 16] = bytes.try_into().map_err(|_| {
                        invalid::<()>("C64 constant must contain sixteen bytes").unwrap_err()
                    })?;
                    Ok(LegacyValue::C64(Ref::new(crate::C64::new(
                        f64::from_bits(u64::from_le_bytes(raw[..8].try_into().unwrap())),
                        f64::from_bits(u64::from_le_bytes(raw[8..].try_into().unwrap())),
                    ))))
                }
                #[cfg(not(feature = "complex"))]
                {
                    invalid("C64 constants are unavailable in this runtime")
                }
            }
            RuntimeType::R64 => {
                #[cfg(feature = "rational")]
                {
                    let raw: [u8; 16] = bytes.try_into().map_err(|_| {
                        invalid::<()>("R64 constant must contain sixteen bytes").unwrap_err()
                    })?;
                    let numerator = i64::from_le_bytes(raw[..8].try_into().unwrap());
                    let denominator = i64::from_le_bytes(raw[8..].try_into().unwrap());
                    if denominator <= 0 {
                        return invalid("R64 constant denominator must be positive and nonzero");
                    }
                    let value = crate::R64::new(numerator, denominator);
                    if *value.numer() != numerator || *value.denom() != denominator {
                        return invalid("R64 constant is not reduced");
                    }
                    Ok(LegacyValue::R64(Ref::new(value)))
                }
                #[cfg(not(feature = "rational"))]
                {
                    invalid("R64 constants are unavailable in this runtime")
                }
            }
            _ => unreachable!("non-scalar runtime type passed the scalar decoder"),
        }
    })())
}
