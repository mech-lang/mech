//! Explicit bytecode-v1 compatibility projection for legacy-only tests and facades.

use crate::{EncodedConstant, LegacyValue, MResult, RuntimeType, ValueKind};

#[cfg(feature = "no_std")]
use alloc::vec::Vec;
#[cfg(not(feature = "no_std"))]
use std::vec::Vec;

pub fn legacy_values_from_encoded_bytecode_constants(
    constants: &[EncodedConstant],
) -> MResult<Vec<LegacyValue>> {
    crate::program::bytecode::constants::decode_encoded_legacy_constants_for_adapter(constants)
}

/// Projects one bytecode-v1 runtime type into the legacy compiler kind model.
///
/// Canonical bytecode execution does not use this projection; it remains only
/// for the legacy compiler sidecar while that metadata is retired.
pub fn legacy_value_kind_from_runtime_type(ty: &RuntimeType) -> MResult<ValueKind> {
    crate::program::bytecode::constants::value_kind_from_runtime_type(ty)
}
