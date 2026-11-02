//! One funded acyclic parent piece. The ordinary child entry/exit is retained;
//! no parent loan, view or pending fuel survives that boundary.
use super::tail::{Region, State};
use super::{Error, Invocation, invalid};
use crate::native_artifact::NativeExecutionPhase;
use crate::native_artifact::program::prepare::composition::frames::{
    Layout as FrameLayout, Source,
};
use crate::native_artifact::program::prepare::composition::numeric::Layout as NumericLayout;
use crate::native_artifact::program::prepare::readonly_numeric::Operation;
use crate::native_artifact::program::runtime::amount;
use mech_core::Value;
use mech_engine::native_artifact::NativeIterationAction;

// A sufficient bound only: an overflowing or too-large product must leave the
// per-field and exact-footprint checks available. This carries no admission.
fn frame_capacity_bound(largest: u64, fields: usize, limit: Option<u64>) -> Option<u64> {
    let bound = largest.checked_mul(u64::try_from(fields).ok()?)?;
    (bound.max(8) <= limit.unwrap_or(u64::MAX)).then_some(bound)
}

impl Invocation<'_, '_, '_> {
    fn segment_source(&self, source: &Source) -> Result<Option<&Value>, Error> {
        Ok(match *source {
            Source::Value(value) => self
                .state
                .arena
                .native_entry_preview(value, &self.state.control.work)?,
            Source::Field { value, ordinal } => self
                .descriptor
                .metadata()
                .buffer_frames
                .fields_at(value as usize)
                .and_then(|f| f.get(ordinal))
                .and_then(|f| self.state.buffer_frames.current_leaf(*f))
                .and_then(|(owner, _)| owner),
        })
    }

    pub(super) fn begin_numeric_segment(
        &mut self,
        site: usize,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        let descriptor = self.descriptor;
        let metadata = descriptor.metadata();
        let id = metadata
            .compositions
            .entry(site)
            .ok_or_else(|| invalid("parent native entry lacks a prepared segment"))?;
        let (parent, segment) = metadata
            .compositions
            .region(id)
            .ok_or_else(|| invalid("parent native entry index differs from its layout"))?;
        let frames = segment
            .frame_plan
            .as_ref()
            .ok_or_else(|| invalid("parent native frame proof missing"))?;
        let proof = segment
            .numeric
            .as_ref()
            .ok_or_else(|| invalid("parent numeric proof missing"))?;
        if args != [0; 3]
            || self.closed_tail.is_some()
            || (self.worker.is_none()
                && self.parent_continuation.current()
                    != parent.suspended_child.map(|child| child as usize))
            || self.window_region.is_some()
            || self.state.control.previous != Some(site as u32)
            || self.state.control.current != Some(parent.iteration)
            || self.progress[parent.iteration as usize * 2 + 1] != 1
            || self.state.scalar_path.is_some_and(|start| {
                start as usize != site
                    || metadata
                        .map_admission
                        .group(site)
                        .is_none_or(|group| group.owner.is_some() || group.start() != start)
            })
        {
            return Err(invalid(
                "parent native entry is outside its exact active source visit",
            ));
        }
        if self.worker.is_some() || self.has_uninventoried_private_owners() {
            return Ok(Some(0));
        }
        let Some(remaining) = self.state.library.unscoped_remaining() else {
            return Ok(Some(0));
        };
        let memory = self.state.library.memory_budget();
        let scalar_bytes = (core::mem::size_of::<Value>()
            + core::mem::size_of::<mech_core::ValueData>()
            + 8) as u64;
        let mut largest_source_bytes = scalar_bytes;
        let mut entry = amount(frames.sources.len(), 8, 0)
            .ok()
            .and_then(|n| metadata.observation_schema_bytes.checked_mul(n))
            .and_then(|n| n.checked_add(amount(proof.words, 128, 2048).ok()?));
        // These borrows cover this entry only. Owner tables and source values
        // are immutable throughout preflight; the separate argument word bank
        // may be written below. Large inventories keep the ordinary lookup.
        let mut current_sources: [Option<&Value>; 8] = [None; 8];
        for (at, source) in frames.sources.iter().enumerate() {
            let Some(owner) = self.segment_source(source)? else {
                return Ok(Some(0));
            };
            // Keep current schema-key validation even when a byte limit does
            // not require traversing the payload's retained footprint below.
            if owner.validate_against(&metadata.schemas).is_err() {
                return Ok(Some(0));
            }
            let Some(bytes) = owner.memory_budget_retained_bytes(&memory) else {
                return Ok(Some(0));
            };
            largest_source_bytes = largest_source_bytes.max(bytes);
            entry = entry.and_then(|n| n.checked_add(bytes.checked_mul(8)?));
            if let Some(current) = current_sources.get_mut(at) {
                *current = Some(owner);
            }
        }
        let current_source = |at: usize| {
            if let Some(owner) = current_sources.get(at).copied().flatten() {
                Ok(Some(owner))
            } else {
                self.segment_source(&frames.sources[at])
            }
        };
        let mut alias_work = Some(0u64);
        for alias in &frames.aliases {
            // The constant/unambiguous selector is proved by the original
            // frame analysis, not looked up again at runtime. Its logical
            // result boundary still requires explicit finite call limits.
            // Current admitted capacity is a sufficient output-byte bound;
            // a tight/refused bound declines without spending or rejecting,
            // so the ordinary call checks its exact footprint at its site.
            if alias.call_result
                && ([
                    self.limits.max_output_elements,
                    self.limits.max_output_bytes,
                    self.limits.max_temporary_bytes,
                    self.limits.max_cloned_bytes,
                    self.limits.max_retained_nodes,
                    self.limits.max_comparison_work,
                    self.limits.max_compute_work,
                ]
                .iter()
                .any(Option::is_none)
                    || self.limits.max_output_elements.is_some_and(|n| n < 1)
                    || self
                        .limits
                        .max_output_bytes
                        .is_some_and(|n| n < largest_source_bytes))
            {
                return Ok(Some(0));
            }
            let Some(owner) = current_source(alias.source)? else {
                return Ok(Some(0));
            };
            let Some(work) = self.state.arena.native_alias_put_work(
                alias.value.get(),
                owner,
                &self.state.control.work,
            )?
            else {
                return Ok(Some(0));
            };
            alias_work = alias_work.and_then(|total| total.checked_add(work));
        }
        // Policy96 prices the actual current alias/type/slot preview. The put
        // bound is only checked here; ordinary put spends it at a reached exit.
        entry = entry.and_then(|n| n.checked_add(amount(frames.aliases.len(), 128, 0).ok()?));
        let Some(entry) = entry else {
            return Ok(Some(0));
        };
        let Some(required) = entry
            .checked_add(proof.compute)
            .and_then(|n| n.checked_add(proof.retirement_work))
            .and_then(|n| n.checked_add(alias_work?))
        else {
            return Ok(Some(0));
        };
        if required > remaining.0
            || self
                .state
                .control
                .work
                .limit()
                .checked_sub(self.state.control.work.used())
                .is_none_or(|n| n < required)
        {
            return Ok(Some(0));
        }
        // A stale previous physical definition must be retired by its original
        // path, rather than silently overwritten by a virtual frame.
        for frame in &frames.frames {
            if !frames.inputs.contains(&frame.value)
                && metadata
                    .buffer_frames
                    .fields_at(frame.value as usize)
                    .is_some_and(|f| {
                        f.iter()
                            .any(|f| self.state.buffer_frames.current_leaf(*f).is_some())
                    })
            {
                return Ok(Some(0));
            }
        }
        for (_, values) in &frames.retirements {
            for &value in values {
                if metadata.frame_retirements.get(value as usize).is_none()
                    && !self
                        .state
                        .arena
                        .native_retirement_available(value, &self.state.control.work)?
                {
                    return Ok(Some(0));
                }
            }
        }
        let temporary = metadata
            .observation_schema_bytes
            .checked_mul(frames.sources.len() as u64)
            .and_then(|n| n.checked_add(amount(proof.words, 8, 1024).ok()?));
        let Some(temporary) = temporary else {
            return Ok(Some(0));
        };
        let Ok(_temporary) = memory.reserve_capacity(temporary) else {
            return Ok(Some(0));
        };
        if proof.words > self.argument_capacity
            || frames.sources.len() > self.state.call_inputs.values().len()
        {
            return Err(invalid(
                "parent native transport is smaller than its prepared inventory",
            ));
        }
        for word in 0..proof.words {
            self.write_borrowed_word(word, 0)?;
        }
        for &value in &frames.inputs {
            let recipe = frames
                .frames
                .iter()
                .find(|f| f.value == value)
                .ok_or_else(|| invalid("parent input frame has no scalar lanes"))?;
            // Resolve immutable geometry lazily at the first actual field read,
            // preserving declines for absent or invalid current field values.
            let mut current_fields = None;
            for (ordinal, slot) in recipe.fields.iter().enumerate() {
                if let Some(slot) = slot {
                    current_fields =
                        current_fields.or_else(|| metadata.buffer_frames.fields_at(value as usize));
                    let Some((_, bits)) = current_fields
                        .and_then(|f| f.get(ordinal))
                        .and_then(|f| self.state.buffer_frames.current_leaf(*f))
                    else {
                        return Ok(Some(0));
                    };
                    self.write_borrowed_word(*slot, bits)?;
                }
            }
        }
        for frame in &frames.frames {
            if frames.inputs.contains(&frame.value) {
                continue;
            }
            // Current admitted capacity includes the exact retained footprint
            // and ownership headers. It is a conservative upper bound, never
            // a permission or a reason to reject a tighter valid byte limit.
            // Every managed field comes from a current source checked above;
            // every other field has scalar_bytes. A maximum-times-count proof
            // avoids revisiting fields when it already fits the current cap.
            let mut upper = frame_capacity_bound(
                largest_source_bytes,
                frame.arrays.len(),
                self.limits.max_output_bytes,
            );
            if upper.is_none() {
                upper = Some(0u64);
                for source in &frame.arrays {
                    let bound = if let Some(source) = source {
                        let Some(owner) = current_source(*source)? else {
                            return Ok(Some(0));
                        };
                        owner.memory_budget_retained_bytes(&memory)
                    } else {
                        Some(scalar_bytes)
                    };
                    upper = upper.and_then(|n| n.checked_add(bound?));
                    if upper.is_none() {
                        break;
                    }
                }
            }
            let mut bytes = upper.unwrap_or(0);
            if upper.is_none_or(|n| n.max(8) > self.limits.max_output_bytes.unwrap_or(u64::MAX)) {
                // A refused/overflowing upper bound proves nothing. Preserve
                // the exact original walk, sum, overflow and byte-limit check.
                bytes = 0;
                for source in &frame.arrays {
                    let bound = if let Some(source) = source {
                        let Some(owner) = current_source(*source)? else {
                            return Ok(Some(0));
                        };
                        let Ok(footprint) = owner.retained_footprint(&metadata.schemas) else {
                            return Ok(Some(0));
                        };
                        footprint.retained_bytes
                    } else {
                        scalar_bytes
                    };
                    let Some(next) = bytes.checked_add(bound) else {
                        return Ok(Some(0));
                    };
                    bytes = next;
                }
            }
            if self
                .limits
                .max_output_elements
                .is_some_and(|n| n < frame.fields.len() as u64)
                || self
                    .limits
                    .max_output_bytes
                    .is_some_and(|n| n < bytes.max(8))
            {
                return Ok(Some(0));
            }
        }
        for (array, view) in proof.views.iter().enumerate() {
            let Some(owner) = current_source(view.source)? else {
                return Ok(Some(0));
            };
            let call = metadata.calls[view.witness]
                .as_ref()
                .ok_or_else(|| invalid("parent numeric view lost its selected factory"))?;
            let Some(words) = call.unsigned_matrix_native_view(owner, &memory) else {
                return Ok(Some(0));
            };
            for (at, word) in words.into_iter().enumerate() {
                self.write_borrowed_word(frames.scalar_words + array * 4 + at, word)?;
            }
        }
        self.state
            .control
            .work
            .charge(NativeExecutionPhase::Setup, entry)?;
        self.state
            .library
            .charge(proof.compute, 0)
            .map_err(|error| Error::Collection {
                origin: metadata.plan.blocks()[site].origin(),
                error,
            })?;
        let state = State {
            region: Region::Segment(id),
            used: self.state.library.used(),
            remaining: self
                .state
                .library
                .unscoped_remaining()
                .ok_or_else(|| invalid("parent entry changed account scope"))?,
            entry: site as u32,
            partial: false,
        };
        // Selection proves that any active ordinary scalar group is wholly
        // inside this piece. Its existing debit is retained. Every decline
        // above leaves its marker intact for the ordinary scalar continuation.
        self.state.scalar_path = None;
        self.closed_tail = Some(state);
        Ok(Some(1))
    }

    pub(super) fn end_numeric_segment(
        &mut self,
        site: usize,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        let state = self
            .closed_tail
            .take()
            .ok_or_else(|| invalid("parent native exit without entry"))?;
        let Region::Segment(id) = state.region else {
            return Err(invalid("parent exit received a closed Tail loan"));
        };
        let descriptor = self.descriptor;
        let metadata = descriptor.metadata();
        let (parent, segment) = metadata
            .compositions
            .region(id)
            .ok_or_else(|| invalid("parent native exit lost its exact segment"))?;
        let frames = segment
            .frame_plan
            .as_ref()
            .ok_or_else(|| invalid("parent exit frame proof missing"))?;
        let proof = segment
            .numeric
            .as_ref()
            .ok_or_else(|| invalid("parent exit numeric proof missing"))?;
        if args[2] != 0
            || self.state.control.previous != Some(state.entry)
            || self.state.control.current != Some(parent.iteration)
            || self.progress[parent.iteration as usize * 2 + 1] != 1
            || self.state.library.used() != state.used
            || self.state.library.unscoped_remaining() != Some(state.remaining)
        {
            return Err(invalid(
                "parent native account, scope or entry changed during execution",
            ));
        }
        let export = match args[0] {
            0 => Some(
                frames
                    .exits
                    .iter()
                    .find(|e| e.from as usize == site && u64::from(e.to) == args[1])
                    .ok_or_else(|| invalid("parent native exit is outside its proved frontier"))?,
            ),
            2 | 3 if args[1] == 0 && segment.sites.contains(&(site as u32)) => None,
            _ => {
                return Err(invalid(
                    "parent native exit mode differs from its source boundary",
                ));
            }
        };
        if self.state.call_inputs.values().iter().any(Option::is_some) {
            return Err(invalid(
                "parent native exit overlaps an ordinary input bank",
            ));
        }
        let outcome = if args[0] == 2 {
            self.finish_numeric_bounds_retry(site, frames, proof)
        } else {
            (|| {
                if let Some(exit) = export {
                    // Reuse the already admitted ordinary Value bank. Pin current
                    // sources only until ordered retirement and final field copy.
                    for value in &exit.frames {
                        let recipe = frames
                            .frames
                            .iter()
                            .find(|f| f.value == value.get())
                            .ok_or_else(|| invalid("parent export lacks exact frame words"))?;
                        for source in recipe.arrays.iter().flatten() {
                            if self.state.call_inputs.values()[*source].is_none() {
                                let owner = self
                                    .segment_source(&frames.sources[*source])?
                                    .ok_or_else(|| {
                                        invalid("parent export lost its current source")
                                    })?
                                    .clone();
                                self.state.call_inputs.values_mut()[*source] = Some(owner);
                            }
                        }
                    }
                    for value in &exit.aliases {
                        let alias = frames
                            .aliases
                            .iter()
                            .find(|alias| alias.value == *value)
                            .ok_or_else(|| invalid("parent alias export lacks its exact source"))?;
                        if self.state.call_inputs.values()[alias.source].is_none() {
                            let owner = self
                                .segment_source(&frames.sources[alias.source])?
                                .ok_or_else(|| {
                                    invalid("parent alias export lost its current owner")
                                })?
                                .clone();
                            self.state.call_inputs.values_mut()[alias.source] = Some(owner);
                        }
                    }
                }
                self.retire_reached_segment_values(frames, proof)?;
                if let Some(exit) = export {
                    // Only real escapes become canonical. Internal projections are
                    // immutable source aliases, never owning arena temporaries.
                    for value in &exit.aliases {
                        let alias = frames
                            .aliases
                            .iter()
                            .find(|alias| alias.value == *value)
                            .ok_or_else(|| invalid("parent alias export lost its exact recipe"))?;
                        let owner = self.state.call_inputs.values()[alias.source]
                            .as_ref()
                            .ok_or_else(|| invalid("parent alias export owner pin missing"))?;
                        self.state
                            .arena
                            .put(value.get(), owner, &mut self.state.control.work)?;
                    }
                    for value in &exit.frames {
                        let recipe = frames
                            .frames
                            .iter()
                            .find(|f| f.value == value.get())
                            .unwrap();
                        let mut current_fields = None;
                        for (ordinal, (word, source)) in
                            recipe.fields.iter().zip(&recipe.arrays).enumerate()
                        {
                            if let Some(word) = word {
                                let bits = self.read_borrowed_word(*word)?;
                                current_fields = current_fields
                                    .or_else(|| metadata.buffer_frames.fields(*value));
                                let field = current_fields
                                    .and_then(|fields| fields.get(ordinal))
                                    .ok_or_else(|| {
                                    invalid("numeric reconciliation outside frame")
                                })?;
                                self.state
                                    .buffer_frames
                                    .synchronize_scalar_field(*field, bits)?;
                            } else if let Some(source) = source {
                                let owner = self.state.call_inputs.values()[*source]
                                    .as_ref()
                                    .ok_or_else(|| invalid("parent export source pin missing"))?
                                    .clone();
                                current_fields = current_fields
                                    .or_else(|| metadata.buffer_frames.fields(*value));
                                let field = current_fields
                                    .and_then(|fields| fields.get(ordinal))
                                    .ok_or_else(|| {
                                    invalid("numeric managed reconciliation outside frame")
                                })?;
                                self.state
                                    .buffer_frames
                                    .synchronize_managed_field(*field, owner)?;
                            } else {
                                return Err(invalid(
                                    "parent export has no scalar or managed source",
                                ));
                            }
                        }
                    }
                }
                Ok(())
            })()
        };
        self.state.call_inputs.values_mut()[..frames.sources.len()].fill(None);
        outcome?;
        self.state.scalar_path = None;
        self.state.control.previous = Some(site as u32);
        self.state.consumed = args[0] != 2;
        Ok(None)
    }

    #[inline(always)]
    fn retire_reached_segment_values(
        &mut self,
        frames: &FrameLayout,
        proof: &NumericLayout,
    ) -> Result<(), Error> {
        let descriptor = self.descriptor;
        let metadata = descriptor.metadata();
        let flags = frames.scalar_words + proof.views.len() * 4;
        for (at, (_, values)) in frames.retirements.iter().enumerate() {
            match self.read_borrowed_word(flags + at)? {
                0 => continue,
                1 => {}
                _ => return Err(invalid("parent retirement flag is not Boolean")),
            }
            for &value in values {
                if !self.state.buffer_frames.retire(
                    &metadata.frame_retirements,
                    value,
                    &mut self.state.control.work,
                )? {
                    self.state
                        .arena
                        .retire_current(value, &mut self.state.control.work)?;
                }
            }
        }
        Ok(())
    }

    #[cold]
    #[inline(never)]
    fn finish_numeric_bounds_retry(
        &mut self,
        site: usize,
        frames: &FrameLayout,
        proof: &NumericLayout,
    ) -> Result<(), Error> {
        if !proof
            .actions
            .iter()
            .any(|(at, action)| *at == site && matches!(action, Operation::Read { .. }))
        {
            return Err(invalid("parent bounds retry is outside a proved read"));
        }
        let descriptor = self.descriptor;
        let NativeIterationAction::LibraryCall { operands, .. } =
            descriptor.metadata().plan.blocks()[site].action()
        else {
            return Err(invalid("parent bounds retry lost its original call"));
        };
        let alias = frames
            .aliases
            .iter()
            .find(|alias| alias.value == operands[0]);
        if let Some(alias) = alias {
            // Pin only the original reader's private operand. Entry already
            // funded this possible escape; source retirement keeps its order.
            let owner = self
                .segment_source(&frames.sources[alias.source])?
                .ok_or_else(|| invalid("parent bounds retry lost its current owner"))?
                .clone();
            self.state.call_inputs.values_mut()[alias.source] = Some(owner);
        }
        self.retire_reached_segment_values(frames, proof)?;
        if let Some(alias) = alias {
            let owner = self.state.call_inputs.values()[alias.source]
                .as_ref()
                .ok_or_else(|| invalid("parent bounds retry owner pin missing"))?;
            self.state
                .arena
                .put(alias.value.get(), owner, &mut self.state.control.work)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::frame_capacity_bound;

    #[test]
    fn capacity_proof_cannot_accept_an_excessive_field_sum() {
        // Unequal, repeated and zero-sized fields exercise the conservative
        // proof independently of the actual frame representation.
        for fields in [vec![], vec![0], vec![1, 7, 1], vec![8, 8], vec![9, 1, 0]] {
            let largest = fields.iter().copied().max().unwrap_or(0);
            let exact: u64 = fields.iter().sum();
            for cap in 0..=32 {
                if let Some(bound) = frame_capacity_bound(largest, fields.len(), Some(cap)) {
                    assert!(bound >= exact);
                    assert!(exact.max(8) <= cap);
                }
            }
        }
        // A refused coarse proof must not reject this valid tighter sum.
        assert_eq!(frame_capacity_bound(7, 3, Some(9)), None);
    }

    #[test]
    fn capacity_proof_declines_overflow_and_preserves_the_minimum_footprint() {
        assert_eq!(frame_capacity_bound(u64::MAX, 2, None), None);
        assert_eq!(
            frame_capacity_bound(u64::MAX, 1, Some(u64::MAX)),
            Some(u64::MAX)
        );
        assert_eq!(frame_capacity_bound(100, 0, Some(7)), None);
        assert_eq!(frame_capacity_bound(100, 0, Some(8)), Some(0));
    }
}
