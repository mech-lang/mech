//! One funded acyclic parent piece. The ordinary child entry/exit is retained;
//! no parent loan, view or pending fuel survives that boundary.
use super::tail::{Region, State};
use super::{Error, Invocation, invalid};
use crate::native_artifact::NativeExecutionPhase;
use crate::native_artifact::program::prepare::composition::frames::Source;
use crate::native_artifact::program::runtime::amount;
use mech_core::Value;

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
            || self.window_region.is_some()
            || self.state.control.previous != Some(site as u32)
            || self.state.control.current != Some(parent.iteration)
            || self.progress[parent.iteration as usize * 2 + 1] != 1
            || self.state.scalar_path.is_some()
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
        let mut entry = amount(frames.sources.len(), 8, 0)
            .ok()
            .and_then(|n| metadata.observation_schema_bytes.checked_mul(n))
            .and_then(|n| n.checked_add(amount(proof.words, 128, 2048).ok()?));
        for source in &frames.sources {
            let Some(owner) = self.segment_source(source)? else {
                return Ok(Some(0));
            };
            let Some(bytes) = owner.memory_budget_retained_bytes(&memory) else {
                return Ok(Some(0));
            };
            entry = entry.and_then(|n| n.checked_add(bytes.checked_mul(8)?));
        }
        let Some(entry) = entry else {
            return Ok(Some(0));
        };
        let Some(required) = entry
            .checked_add(proof.compute)
            .and_then(|n| n.checked_add(proof.retirement_work))
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
            for (ordinal, slot) in recipe.fields.iter().enumerate() {
                if let Some(slot) = slot {
                    let Some((_, bits)) = metadata
                        .buffer_frames
                        .fields_at(value as usize)
                        .and_then(|f| f.get(ordinal))
                        .and_then(|f| self.state.buffer_frames.current_leaf(*f))
                    else {
                        return Ok(Some(0));
                    };
                    self.write_borrowed_word(*slot, bits)?;
                }
            }
        }
        let scalar_bytes = (core::mem::size_of::<Value>()
            + core::mem::size_of::<mech_core::ValueData>()
            + 8) as u64;
        for frame in &frames.frames {
            if frames.inputs.contains(&frame.value) {
                continue;
            }
            let mut bytes = 0u64;
            for source in &frame.arrays {
                let bound = if let Some(source) = source {
                    let Some(owner) = self.segment_source(&frames.sources[*source])? else {
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
            let Some(owner) = self.segment_source(&frames.sources[view.source])? else {
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
        self.closed_tail = Some(State {
            region: Region::Segment(id),
            used: self.state.library.used(),
            remaining: self
                .state
                .library
                .unscoped_remaining()
                .ok_or_else(|| invalid("parent entry changed account scope"))?,
            entry: site as u32,
            partial: false,
        });
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
        let outcome = (|| {
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
                                .ok_or_else(|| invalid("parent export lost its current source"))?
                                .clone();
                            self.state.call_inputs.values_mut()[*source] = Some(owner);
                        }
                    }
                }
            }
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
            if let Some(exit) = export {
                for value in &exit.frames {
                    let recipe = frames
                        .frames
                        .iter()
                        .find(|f| f.value == value.get())
                        .unwrap();
                    for (ordinal, (word, source)) in
                        recipe.fields.iter().zip(&recipe.arrays).enumerate()
                    {
                        if let Some(word) = word {
                            let bits = self.read_borrowed_word(*word)?;
                            self.state.buffer_frames.synchronize_scalar(
                                &metadata.buffer_frames,
                                *value,
                                ordinal,
                                bits,
                            )?;
                        } else if let Some(source) = source {
                            let owner = self.state.call_inputs.values()[*source]
                                .as_ref()
                                .ok_or_else(|| invalid("parent export source pin missing"))?
                                .clone();
                            self.state.buffer_frames.synchronize_managed(
                                &metadata.buffer_frames,
                                *value,
                                ordinal,
                                owner,
                            )?;
                        } else {
                            return Err(invalid("parent export has no scalar or managed source"));
                        }
                    }
                }
            }
            Ok(())
        })();
        self.state.call_inputs.values_mut()[..frames.sources.len()].fill(None);
        outcome?;
        self.state.scalar_path = None;
        self.state.control.previous = Some(site as u32);
        self.state.consumed = args[0] != 2;
        Ok(None)
    }
}
