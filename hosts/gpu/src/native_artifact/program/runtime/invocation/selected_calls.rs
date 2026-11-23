//! Dispatch one prepared strategy, then retain the ordinary route if live loans decline it.
use super::{Error, GlobalHelper, Invocation};
use crate::native_artifact::program::prepare::{call_routes::LibraryCallRoute, line_matrix};

impl Invocation<'_, '_, '_> {
    pub(super) fn prepared_library_call(
        &mut self,
        site: usize,
        words: &[u64],
    ) -> Result<Option<Option<u64>>, Error> {
        let metadata = self.descriptor.metadata();
        match metadata.call_routes[site] {
            LibraryCallRoute::Canonical => Ok(None),
            LibraryCallRoute::BorrowedMapKeysLength => {
                Ok(self.borrowed_keys_length_call(site, words)?.map(Some))
            }
            LibraryCallRoute::BorrowedMapKeysFrame => {
                self.buffer_frame_action(site, GlobalHelper::Library, words)
            }
            LibraryCallRoute::BufferFrame => {
                self.buffer_frame_action(site, GlobalHelper::Library, words)
            }
            LibraryCallRoute::UnsignedBuffer => {
                Ok(self.unsigned_buffer_call(site, words)?.then_some(None))
            }
            LibraryCallRoute::RecordScalarField => {
                Ok(self.record_scalar_field_call(site, words)?.map(Some))
            }
            LibraryCallRoute::UnsignedMatrixLength => {
                Ok(self.unsigned_matrix_length_call(site, words)?.map(Some))
            }
            // The line-matrix route owns this consumer, including its deferred
            // capture and ordinary canonical fallback. A private view has no
            // canonical arena owner for the generic length route to read.
            LibraryCallRoute::MatrixLength
                if matches!(
                    metadata.line_matrices.sites[site],
                    Some(line_matrix::Site::Length(_))
                ) =>
            {
                Ok(None)
            }
            LibraryCallRoute::MatrixLength => Ok(self.matrix_length_call(site, words)?.map(Some)),
            LibraryCallRoute::UnsignedMatrixElement => {
                Ok(self.unsigned_matrix_element_call(site, words)?.map(Some))
            }
            LibraryCallRoute::StringMatrixElement => Ok(self
                .string_matrix_element_call(site, words)?
                .then_some(None)),
            LibraryCallRoute::UnsignedMatrixReplace => Ok(self
                .unsigned_matrix_replace_call(site, words)?
                .then_some(None)),
        }
    }
}
