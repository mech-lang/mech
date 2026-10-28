//! Static per-site strategy; current owners, shapes and loans remain runtime checks.
use mech_engine::native_artifact::{
    NativeBufferFrames, NativeIterationAction, NativePreparedLibraryCall,
    NativeProgramArtifactPlan, NativeUnsignedBufferCall,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) enum LibraryCallRoute {
    Canonical,
    BufferFrame,
    UnsignedBuffer,
    RecordScalarField,
    UnsignedMatrixLength,
    UnsignedMatrixElement,
    UnsignedMatrixReplace,
}

pub(in crate::native_artifact) fn derive(
    plan: &NativeProgramArtifactPlan,
    frames: &NativeBufferFrames,
    buffers: &[Option<NativeUnsignedBufferCall>],
    calls: &[Option<NativePreparedLibraryCall>],
) -> Box<[LibraryCallRoute]> {
    plan.blocks()
        .iter()
        .enumerate()
        .map(|(site, block)| {
            if !matches!(block.action(), NativeIterationAction::LibraryCall { .. }) {
                return LibraryCallRoute::Canonical;
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
            } else if call.supports_unsigned_matrix_element() {
                LibraryCallRoute::UnsignedMatrixElement
            } else {
                LibraryCallRoute::Canonical
            }
        })
        .collect()
}

#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "call_routes_tests.rs"]
mod tests;
