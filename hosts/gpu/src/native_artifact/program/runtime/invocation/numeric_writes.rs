//! One cold first-change boundary per array. Original selected operation owns
//! validation/copy/error ordering. The rest of the recurrence uses native cells.
use super::{Error, Invocation, invalid};
use crate::native_artifact::program::prepare::readonly_numeric::Operation;
use mech_core::snapshot::UnsignedMatrixViewMut;
use mech_engine::native_artifact::NativeIterationAction as Action;

impl Invocation<'_, '_, '_> {
    pub(super) fn prepare_numeric_write(
        &mut self,
        site: usize,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        let metadata = self.descriptor.metadata();
        // A selected complete tree and its ordinary fallback leaves have
        // overlapping source sites. Only the current exclusive loan identifies
        // which independently proved owner bank is live at this exact store.
        let Some(super::tail::State {
            region: super::tail::Region::Tail(id),
            ..
        }) = self.closed_tail.as_ref()
        else {
            return Err(invalid("numeric materialization has no current Tail loan"));
        };
        let id = *id;
        let region = &metadata.closed_tails.regions[id];
        let proof = region
            .numeric
            .as_ref()
            .ok_or_else(|| invalid("numeric materialization lacks proof"))?;
        let Some(operation @ (Operation::Write { array, .. } | Operation::Append { array, .. })) =
            proof.action(site)
        else {
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
        self.state
            .control
            .work
            .charge(crate::native_artifact::NativeExecutionPhase::Setup, 64)?;
        let source = self.state.buffer_frames.take_numeric_owner(
            proof.arrays[*array]
                .physical()
                .ok_or_else(|| invalid("captured array cannot receive a mutable owner"))?,
            proof.arrays[*array].schema,
        )?;
        let coexistence = self.selected_join_coexistence(None)?;
        let result = if matches!(operation, Operation::Append { .. }) {
            call.unsigned_matrix_append_private(
                source,
                incoming,
                true,
                self.limits,
                &self.state.library,
                coexistence,
            )
        } else {
            call.unsigned_matrix_replace_private(
                source,
                index,
                incoming,
                true,
                self.limits,
                &self.state.library,
                coexistence,
            )
        };
        let mut private = result.map_err(|error| Error::Library {
            origin: metadata.plan.blocks()[site].origin(),
            error,
        })?;
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
        if proof.arrays[*array].appends {
            let preparation = self
                .state
                .buffer_frames
                .append_preparation_mut(id, *array)?
                .take()
                .ok_or_else(|| invalid("numeric append lost its admitted preparation"))?;
            private = private
                .with_append_preparation(preparation)
                .map_err(Error::Snapshot)?;
        }
        let address = if let Some(capacity) = proof.capacity(*array) {
            let [address, length, cells] = private
                .reserved_native_view()
                .ok_or_else(|| invalid("numeric append lacks a reserved Vec view"))?;
            self.write_borrowed_word(2 + proof.view(*array) + 1, length)?;
            self.write_borrowed_word(2 + proof.view(*array) + 2, 1)?;
            self.write_borrowed_word(2 + proof.view(*array) + 3, length)?;
            self.write_borrowed_word(2 + capacity, cells)?;
            address
        } else {
            match private.elements_mut() {
                UnsignedMatrixViewMut::U8(cells) => cells.as_mut_ptr() as u64,
                UnsignedMatrixViewMut::U16(cells) => cells.as_mut_ptr() as u64,
                UnsignedMatrixViewMut::U32(cells) => cells.as_mut_ptr() as u64,
                UnsignedMatrixViewMut::U64(cells) => cells.as_mut_ptr() as u64,
            }
        };
        // Moving this owner moves only metadata; the allocated cells stay put.
        *self.state.buffer_frames.private_array_mut(id, *array)? = Some(private);
        self.write_borrowed_word(2 + proof.view(*array), address)?;
        self.write_borrowed_word(2 + proof.private_flag(*array), 1)?;
        self.resume_numeric_after_write(id, site)?;
        Ok(None)
    }

    pub(super) fn finish_numeric_array(
        mut private: mech_core::snapshot::PrivateUnsignedMatrix,
        length: Option<u64>,
    ) -> Result<mech_core::Value, Error> {
        if let Some(length) = length {
            let length =
                usize::try_from(length).map_err(|_| invalid("numeric append length overflows"))?;
            // SAFETY: this same-plan region's sealed emitter admits capacity
            // before a traversal and advances the live length only after its
            // typed store. No borrow survives the closed-tail reconciliation.
            // Core additionally rejects shrinking or exceeding actual capacity.
            unsafe { private.assume_initialized_prefix(length) }.map_err(Error::Snapshot)?;
        }
        Ok(private.finish())
    }
}
