//! Static per-site strategy; current owners, shapes and loans remain runtime checks.
use mech_engine::native_artifact::{
    NativeBufferFrames, NativeIterationAction, NativePreparedLibraryCall,
    NativeProgramArtifactPlan, NativeUnsignedBufferCall,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub(in crate::native_artifact) enum LibraryCallRoute {
    Canonical,
    BufferFrame,
    BorrowedMapKeysFrame,
    BorrowedMapKeysLength,
    UnsignedBuffer,
    RecordScalarField,
    UnsignedMatrixLength,
    MatrixLength,
    UnsignedMatrixElement,
    StringMatrixElement,
    UnsignedMatrixReplace,
}

pub(in crate::native_artifact) fn derive(
    plan: &NativeProgramArtifactPlan,
    frames: &NativeBufferFrames,
    buffers: &[Option<NativeUnsignedBufferCall>],
    calls: &[Option<NativePreparedLibraryCall>],
    keys: &super::borrowed_map_keys::Layout,
) -> Box<[LibraryCallRoute]> {
    plan.blocks()
        .iter()
        .enumerate()
        .map(|(site, block)| {
            if !matches!(block.action(), NativeIterationAction::LibraryCall { .. }) {
                return LibraryCallRoute::Canonical;
            }
            if keys.frame(site).is_some() {
                return LibraryCallRoute::BorrowedMapKeysFrame;
            }
            if keys.length(site).is_some() {
                return LibraryCallRoute::BorrowedMapKeysLength;
            }
            // Preserve the established priority when a private frame projection
            // also has the selected record-field scalar strategy.
            if frames.action(site).is_some() {
                return LibraryCallRoute::BufferFrame;
            }
            let Some(call) = calls.get(site).and_then(Option::as_ref) else {
                return LibraryCallRoute::Canonical;
            };
            if buffers.get(site).is_some_and(Option::is_some) {
                if call.supports_unsigned_matrix_replace() {
                    return LibraryCallRoute::UnsignedMatrixReplace;
                }
                if call.supports_unsigned_buffer() {
                    return LibraryCallRoute::UnsignedBuffer;
                }
            }
            if call.record_scalar_field_kind().is_some() {
                LibraryCallRoute::RecordScalarField
            } else if call.supports_unsigned_matrix_length() {
                LibraryCallRoute::UnsignedMatrixLength
            } else if call.supports_matrix_length() {
                LibraryCallRoute::MatrixLength
            } else if call.supports_unsigned_matrix_element() {
                LibraryCallRoute::UnsignedMatrixElement
            } else if call.supports_string_matrix_element() {
                LibraryCallRoute::StringMatrixElement
            } else {
                LibraryCallRoute::Canonical
            }
        })
        .collect()
}

#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "call_routes_tests.rs"]
mod tests;
