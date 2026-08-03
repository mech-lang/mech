//! Inline RuntimeType keys used by enum payloads.
//!
//! Keeping this forwarding boundary inside the constant codec ensures enums
//! use precisely the same ID-independent canonical representation as runtime
//! type finalization.

use crate::{MResult, program::bytecode::RuntimeType};

pub(crate) fn encode(ty: &RuntimeType) -> MResult<Vec<u8>> {
    super::super::types::canonical_runtime_type_key(ty)
}

pub(crate) fn decode(bytes: &[u8]) -> MResult<RuntimeType> {
    super::super::types::decode_canonical_runtime_type_key(bytes)
}
