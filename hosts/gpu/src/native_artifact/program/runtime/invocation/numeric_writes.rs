//! One cold first-change boundary per array. Original selected replacement owns
//! validation/copy/error ordering. The rest of the recurrence uses native cells.
use super::{Error, Invocation, invalid};
use crate::native_artifact::program::prepare::readonly_numeric::Operation;
use crate::native_artifact::program::runtime::buffer_frames::Field;
use mech_core::snapshot::{UnsignedMatrixOwnershipTransfer, UnsignedMatrixViewMut};
use mech_engine::native_artifact::NativeIterationAction as Action;

impl Invocation<'_, '_, '_> {
    pub(super) fn prepare_numeric_write(
        &mut self,
        site: usize,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        let metadata = self.descriptor.metadata();
        let id = metadata
            .closed_tails
            .sites
            .get(site)
            .copied()
            .flatten()
            .ok_or_else(|| invalid("numeric materialization outside proved region"))?;
        let region = &metadata.closed_tails.regions[id];
        let proof = region
            .numeric
            .as_ref()
            .ok_or_else(|| invalid("numeric materialization lacks proof"))?;
        let Some(Operation::Write { array, .. }) = proof.action(site) else {
            return Err(invalid("numeric materialization outside selected write"));
        };
        if args[2] != 0
            || self
                .state
                .buffer_frames
                .private_array_mut(id, *array)?
                .is_some()
        {
            return Err(invalid("duplicate or malformed numeric materialization"));
        }
        let index = self.read_borrowed_word(2 + proof.words)?;
        let incoming = self.read_borrowed_word(3 + proof.words)?;
        // Close the current loan before any fallible factory/account operation.
        // This traversal has already been prepaid at its native header.
        self.end_closed_tail(site, [args[0], args[1], 5])?;
        let Action::LibraryCall { .. } = metadata.plan.blocks()[site].action() else {
            return Err(invalid("numeric write lost source operation"));
        };
        let call = metadata.calls[site]
            .as_ref()
            .ok_or_else(|| invalid("numeric write lost selected factory"))?;
        let Field::Managed(source) = self.state.buffer_frames.project(
            &metadata.buffer_frames,
            region.tail.carrier(),
            proof.arrays[*array].field,
            &mut self.state.control.work,
        )?
        else {
            return Err(invalid("numeric write lost source owner"));
        };
        let coexistence = self.selected_join_coexistence(None)?;
        let (value, changed) = call
            .unsigned_matrix_replace(
                source,
                index,
                incoming,
                false,
                self.limits,
                &self.state.library,
                coexistence,
            )
            .map_err(|error| Error::Library {
                origin: metadata.plan.blocks()[site].origin(),
                error,
            })?;
        if !changed {
            return Err(invalid("numeric materialization did not change a cell"));
        }
        let memory = self.state.library.memory_budget();
        let metadata_bytes = metadata
            .observation_schema_bytes
            .checked_add(4096)
            .ok_or_else(|| invalid("numeric materialization metadata overflows"))?;
        self.state.control.work.charge(
            crate::native_artifact::NativeExecutionPhase::Setup,
            metadata_bytes
                .checked_mul(16)
                .ok_or_else(|| invalid("numeric materialization work overflows"))?,
        )?;
        let _metadata = memory.reserve_capacity(metadata_bytes)?;
        let shape = value.shape().clone();
        let UnsignedMatrixOwnershipTransfer::Owned(owned) =
            value.try_into_owned_unsigned_matrix(&memory)?
        else {
            return Err(invalid("new numeric replacement is not exclusive"));
        };
        let mut private = owned.into_private_updates(shape).map_err(Error::Snapshot)?;
        let address = match private.elements_mut() {
            UnsignedMatrixViewMut::U8(cells) => cells.as_mut_ptr() as u64,
            UnsignedMatrixViewMut::U16(cells) => cells.as_mut_ptr() as u64,
            UnsignedMatrixViewMut::U32(cells) => cells.as_mut_ptr() as u64,
            UnsignedMatrixViewMut::U64(cells) => cells.as_mut_ptr() as u64,
        };
        // Moving this owner moves only metadata; the allocated cells stay put.
        *self.state.buffer_frames.private_array_mut(id, *array)? = Some(private);
        self.write_borrowed_word(2 + proof.view(*array), address)?;
        self.write_borrowed_word(2 + proof.private_flag(*array), 1)?;
        self.resume_numeric_after_write(id, site)?;
        Ok(None)
    }
}
