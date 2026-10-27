//! Capacity-bearing sequence calls remain ordinary selected library visits.
use super::{Error, Invocation, invalid};
use mech_engine::native_artifact::{NativeIterationAction as Action, NativeScalarKind as K};

impl Invocation<'_, '_, '_> {
    pub(super) fn unsigned_buffer_call(
        &mut self,
        site: usize,
        words: &[u64],
    ) -> Result<bool, Error> {
        let descriptor = self.descriptor;
        let Some(proof) = descriptor
            .metadata()
            .unsigned_buffer_calls
            .get(site)
            .and_then(Option::as_ref)
        else {
            return Ok(false);
        };
        let Some(call) = descriptor.metadata().calls[site].as_ref() else {
            return Err(invalid("buffer call lost its selected factory"));
        };
        if !call.supports_unsigned_buffer() || proof.site() as usize != site {
            return Err(invalid("buffer call differs from selected site"));
        }
        let Action::LibraryCall {
            operands, output, ..
        } = descriptor.metadata().plan.blocks()[site].action()
        else {
            return Err(invalid("buffer helper outside library call"));
        };
        if words.len() != operands.len()
            || descriptor.metadata().plan.values()[output.get() as usize]
                .scalar_kind()
                .is_some()
        {
            return Err(invalid("buffer output transport differs"));
        }
        for (index, operand) in operands.iter().enumerate() {
            let kind = descriptor.metadata().plan.values()[operand.get() as usize].scalar_kind();
            if Some(*operand) == proof.source() {
                if kind.is_some() || words[index] != 0 {
                    return Err(invalid("buffer source is not a managed owner"));
                }
            } else if !matches!(kind, Some(K::U8 | K::U16 | K::U32 | K::U64)) {
                return Err(invalid("buffer scalar operand kind differs"));
            }
        }
        if self.line_window.is_some()
            || self.window_region.is_some()
            || self.window_hashes.is_some()
            || self.closed_tail.is_some()
            || self.prefix.as_ref().is_some_and(|prefix| !prefix.input.eof)
        {
            return Ok(false);
        }
        let coexistence = self.selected_join_coexistence(None)?;
        // Fixed table/header checks and optional alias retirement share the
        // invocation's existing finite native ledger. No arena-wide sweep runs.
        let checks = (proof.owned_source_slots().len() as u64)
            .checked_mul(64)
            .and_then(|n| n.checked_add(512))
            .ok_or_else(|| invalid("buffer ownership work overflow"))?;
        self.state
            .control
            .work
            .charge(crate::native_artifact::NativeExecutionPhase::Setup, checks)?;
        let source = proof
            .source()
            .map(|source| {
                self.state
                    .arena
                    .copy_current(source.get(), &mut self.state.control.work)
            })
            .transpose()?;
        if let Some(source) = source.as_ref() {
            for slot in proof.owned_source_slots() {
                self.state.arena.retire_current_matching(
                    slot.get(),
                    source,
                    &mut self.state.control.work,
                )?;
            }
        }
        let result = call
            .unsigned_buffer(
                source,
                words.get(1).copied().unwrap_or(0),
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
            .put(output.get(), &result, &mut self.state.control.work)?;
        Ok(true)
    }
}
