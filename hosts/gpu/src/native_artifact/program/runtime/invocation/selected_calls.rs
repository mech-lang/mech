//! Dispatch one prepared strategy, then retain the ordinary route if live loans decline it.
use super::{Error, GlobalHelper, Invocation};
use crate::native_artifact::program::prepare::call_routes::LibraryCallRoute;

impl Invocation<'_, '_, '_> {
    pub(super) fn prepared_library_call(
        &mut self,
        site: usize,
        words: &[u64],
    ) -> Result<Option<Option<u64>>, Error> {
        match self.descriptor.metadata().call_routes[site] {
            LibraryCallRoute::Canonical => Ok(None),
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
            LibraryCallRoute::UnsignedMatrixElement => {
                Ok(self.unsigned_matrix_element_call(site, words)?.map(Some))
            }
        }
    }
}
