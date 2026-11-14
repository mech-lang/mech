//! One original selected scalar store with native operands and current owners.
use super::{Error, Invocation, invalid};
use mech_engine::native_artifact::{NativeIterationAction as Action, NativeScalarKind};

impl Invocation<'_, '_, '_> {
    pub(super) fn unsigned_matrix_replace_call(
        &mut self,
        site: usize,
        words: &[u64],
    ) -> Result<bool, Error> {
        let descriptor = self.descriptor;
        let proof = descriptor
            .metadata()
            .unsigned_buffer_calls
            .get(site)
            .and_then(Option::as_ref)
            .ok_or_else(|| invalid("unsigned store lost its current-site facts"))?;
        let call = descriptor.metadata().calls[site]
            .as_ref()
            .ok_or_else(|| invalid("unsigned store lost its selected factory"))?;
        let Action::LibraryCall {
            operands, output, ..
        } = descriptor.metadata().plan.blocks()[site].action()
        else {
            return Err(invalid("unsigned store outside library action"));
        };
        let kinds = |index: usize| {
            descriptor.metadata().plan.values()[operands[index].get() as usize].scalar_kind()
        };
        if !call.supports_unsigned_matrix_replace()
            || proof.site() as usize != site
            || operands.len() != 3
            || words.len() != 3
            || words[0] != 0
            || proof.source() != Some(operands[0])
            || proof.sublist().is_some()
            || kinds(0).is_some()
            || kinds(2) != Some(NativeScalarKind::U64)
            || !kinds(1).is_some_and(|kind| {
                kind.unsigned_maximum().is_some() && kind.accepts_bits(words[1])
            })
            || descriptor.metadata().plan.values()[output.get() as usize]
                .scalar_kind()
                .is_some()
        {
            return Err(invalid("unsigned store operand transport differs"));
        }
        if self.has_uninventoried_private_owners() {
            return Ok(false);
        }
        let coexistence = self.selected_join_coexistence(None)?;
        self.state.control.work.charge(
            crate::native_artifact::NativeExecutionPhase::Setup,
            (proof.owned_source_slots().len() as u64)
                .checked_mul(64)
                .and_then(|n| n.checked_add(512))
                .ok_or_else(|| invalid("unsigned store ownership work overflow"))?,
        )?;
        let source = self
            .state
            .arena
            .copy_current(operands[0].get(), &mut self.state.control.work)?;
        // Retire only proved dead slots with this visit's actual frozen identity.
        // External aliases and private frames remain real owners until retired.
        for slot in proof.owned_source_slots() {
            self.state.arena.retire_current_matching(
                slot.get(),
                &source,
                &mut self.state.control.work,
            )?;
        }
        let (value, _changed) = call
            .unsigned_matrix_replace(
                source,
                words[2],
                words[1],
                !proof.owned_source_slots().is_empty(),
                self.limits,
                &self.state.library,
                coexistence,
            )
            .map_err(|error| Error::Library {
                origin: descriptor.metadata().plan.blocks()[site].origin(),
                error,
            })?;
        self.state
            .arena
            .put(output.get(), &value, &mut self.state.control.work)?;
        Ok(true)
    }
}
