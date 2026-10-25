//! Original selected unsigned reads returning words into native scalar storage.
use super::{Error, Invocation, invalid};
use mech_engine::native_artifact::{NativeIterationAction as Action, NativeScalarKind};

impl Invocation<'_, '_, '_> {
    pub(super) fn unsigned_matrix_element_call(
        &mut self,
        site: usize,
        words: &[u64],
    ) -> Result<Option<u64>, Error> {
        let descriptor = self.descriptor;
        let Some(call) = descriptor.metadata().calls[site].as_ref() else {
            return Ok(None);
        };
        if !call.supports_unsigned_matrix_element() {
            return Ok(None);
        }
        let Action::LibraryCall {
            operands, output, ..
        } = descriptor.metadata().plan.blocks()[site].action()
        else {
            return Err(invalid("word read outside its selected call"));
        };
        if operands.len() != 2
            || words.len() != 2
            || words[0] != 0
            || descriptor.metadata().plan.values()[operands[0].get() as usize]
                .scalar_kind()
                .is_some()
            || descriptor.metadata().plan.values()[operands[1].get() as usize].scalar_kind()
                != Some(NativeScalarKind::U64)
            || !descriptor.metadata().plan.values()[output.get() as usize]
                .scalar_kind().is_some_and(|kind| kind.unsigned_maximum().is_some())
        {
            return Err(invalid("word read native transport differs"));
        }
        // Private windows/loans need their own owner census. Keep their existing
        // canonical route until this read strategy has the corresponding proof.
        if self.line_window.is_some()
            || self.window_region.is_some()
            || self.window_hashes.is_some()
            || self.closed_tail.is_some()
            || self.prefix.as_ref().is_some_and(|prefix| !prefix.input.eof)
        {
            return Ok(None);
        }
        let coexistence = self.selected_join_coexistence(None)?;
        let source = self
            .state
            .arena
            .copy_current(operands[0].get(), &mut self.state.control.work)?;
        let word = call
            .unsigned_matrix_element(&source, words[1], self.limits, &self.state.library, coexistence)
            .map_err(|error| Error::Library {
                origin: descriptor.metadata().plan.blocks()[site].origin(),
                error,
            })?;
        // Like a U64 structural observation, this is already a native word.
        // A later canonical use/publication constructs its actual Value there.
        Ok(Some(word))
    }
}
