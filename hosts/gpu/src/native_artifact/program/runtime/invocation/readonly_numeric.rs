//! Invocation-current views. The admitted carrier owns every borrowed backing;
//! no ordinary helper can run while the same closed-tail loan is outstanding.
use super::{Error, Invocation, invalid};
use crate::native_artifact::program::prepare::readonly_numeric::Operation;
use crate::native_artifact::program::runtime::buffer_frames::Field;
use mech_engine::native_artifact::NativeIterationAction as Action;

impl Invocation<'_, '_, '_> {
    pub(super) fn begin_numeric_views(&mut self, region: usize) -> Result<bool, Error> {
        let descriptor = self.descriptor;
        let metadata = descriptor.metadata();
        let region = &metadata.closed_tails.regions[region];
        let proof = region
            .numeric
            .as_ref()
            .ok_or_else(|| invalid("missing numeric leaf proof"))?;
        if self.has_uninventoried_private_owners() {
            return Ok(false);
        }
        // Pure same-account/availability preview precedes any potentially
        // failing optimization work. Unsupported/tight contexts keep the route.
        let memory = self.state.library.memory_budget();
        let mut entry_work = metadata
            .observation_schema_bytes
            .checked_mul(proof.arrays.len() as u64 * 8)
            .and_then(|n| n.checked_add(proof.words as u64 * 128 + 2048));
        for array in &proof.arrays {
            let Some(owner) = self.state.buffer_frames.managed_owner(
                &metadata.buffer_frames,
                region.tail.carrier(),
                array.field,
            ) else {
                return Err(invalid("numeric entry lost current owner"));
            };
            let Some(retained) = owner.memory_budget_retained_bytes(&memory) else {
                return Ok(false);
            };
            entry_work = entry_work.and_then(|n| n.checked_add(retained.checked_mul(8)?));
        }
        let Some(entry_work) = entry_work else {
            return Ok(false);
        };
        let Some((remaining, _)) = self.state.library.unscoped_remaining() else {
            return Ok(false);
        };
        let publication_work = if let Some(array) = proof.result.direct_array() {
            self.state
                .buffer_frames
                .managed_owner(
                    &metadata.buffer_frames,
                    region.tail.carrier(),
                    proof.arrays[array].field,
                )
                .and_then(|v| v.memory_budget_retained_bytes(&memory))
                .and_then(|n| n.checked_mul(4))
                .and_then(|n| n.checked_add(64))
        } else {
            Some(0)
        };
        if publication_work
            .and_then(|n| n.checked_add(entry_work))
            .is_none_or(|n| n > remaining)
        {
            return Ok(false);
        }
        self.state.control.work.charge(
            crate::native_artifact::NativeExecutionPhase::Setup,
            entry_work,
        )?;
        // Temporary validation metadata is separately admitted once. A capacity
        // miss declines optimization and retains the original source boundary.
        let temporary = metadata
            .observation_schema_bytes
            .checked_mul(proof.arrays.len() as u64)
            .and_then(|n| n.checked_add(proof.words as u64 * 8 + 1024));
        let Some(temporary) = temporary else {
            return Ok(false);
        };
        let Ok(_temporary) = memory.reserve_capacity(temporary) else {
            return Ok(false);
        };
        for (at, array) in proof.arrays.iter().enumerate() {
            let owner = self
                .state
                .buffer_frames
                .managed_owner(&metadata.buffer_frames, region.tail.carrier(), array.field)
                .ok_or_else(|| invalid("numeric entry lost current owner"))?;
            let Ok(footprint) = owner.retained_footprint(&metadata.schemas) else {
                return Ok(false);
            };
            self.write_borrowed_word(2 + proof.view(at), footprint.retained_bytes)?;
        }
        // All private packs, including duplicate managed fields in a helper,
        // retain their logical output bounds. A miss declines, never traps.
        let scalar_bytes = (core::mem::size_of::<mech_core::Value>()
            + core::mem::size_of::<mech_core::ValueData>()
            + 8) as u64;
        for frame in &proof.frames {
            let mut bytes = 0u64;
            for array in &frame.arrays {
                let bound = if let Some(array) = array {
                    self.read_borrowed_word(2 + proof.view(*array))?
                } else {
                    scalar_bytes
                };
                let Some(total) = bytes.checked_add(bound) else {
                    return Ok(false);
                };
                bytes = total;
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
                return Ok(false);
            }
        }
        // Fill only fixed, already admitted transport. Borrow current headers
        // while the carrier retains each owner throughout native execution.
        for (at, array) in proof.arrays.iter().enumerate() {
            let Field::Managed(owner) = self.state.buffer_frames.project(
                &metadata.buffer_frames,
                region.tail.carrier(),
                array.field,
                &mut self.state.control.work,
            )?
            else {
                return Err(invalid("numeric view has scalar owner"));
            };
            let call = metadata.calls[array.witness]
                .as_ref()
                .ok_or_else(|| invalid("numeric view lost factory"))?;
            let Some(view) =
                call.unsigned_matrix_native_view(owner, &self.state.library.memory_budget())
            else {
                return Ok(false);
            };
            // A no-op store still has the selected array output contract.
            // Decline, rather than reject at entry: an earlier assertion or
            // unselected store must keep its original source/error ordering.
            if array.writable
                && self
                    .limits
                    .max_output_elements
                    .is_some_and(|limit| view[1] > limit)
            {
                return Ok(false);
            }
            if let Some(slot) = proof.capacity(at) {
                let mech_core::ValueData::Matrix(matrix) = owner.data() else {
                    return Ok(false);
                };
                let Some(capacity) = matrix.unsigned_buffer_capacity() else {
                    return Ok(false);
                };
                if view[2] != 1
                    || self
                        .limits
                        .max_output_elements
                        .is_some_and(|limit| capacity as u64 > limit)
                {
                    return Ok(false);
                }
                self.write_borrowed_word(2 + slot, capacity as u64)?;
            }
            for (index, word) in view.into_iter().enumerate() {
                self.write_borrowed_word(2 + proof.view(at) + index, word)?;
            }
        }
        for (ordinal, slot) in proof
            .fields(region.tail.carrier().get())
            .ok_or_else(|| invalid("numeric scalar frame missing"))?
            .iter()
            .enumerate()
        {
            if let Some(slot) = slot {
                let Field::Scalar { value, .. } = self.state.buffer_frames.project(
                    &metadata.buffer_frames,
                    region.tail.carrier(),
                    ordinal,
                    &mut self.state.control.work,
                )?
                else {
                    return Err(invalid("numeric scalar has managed owner"));
                };
                self.write_borrowed_word(2 + slot, value.to_bits())?;
            }
        }
        if proof.writes() || proof.result.direct_array().is_some() {
            let publication = if let Some(array) = proof.result.direct_array() {
                let owner = self
                    .state
                    .buffer_frames
                    .managed_owner(
                        &metadata.buffer_frames,
                        region.tail.carrier(),
                        proof.arrays[array].field,
                    )
                    .ok_or_else(|| invalid("numeric result lost owner"))?;
                Some(self.state.arena.prepare_put(
                    region.tail.returned().get(),
                    owner,
                    &mut self.state.control.work,
                )?)
            } else {
                None
            };
            let id = metadata.closed_tails.sites[region.tail.header() as usize]
                .ok_or_else(|| invalid("numeric region lost layout"))?;
            if !self.state.buffer_frames.prepare_private(
                id,
                proof.arrays.len(),
                proof.arrays.iter().any(|a| a.appends),
                publication,
                &memory,
            )? {
                return Ok(false);
            }
            for (at, array) in proof.arrays.iter().enumerate().filter(|(_, a)| a.appends) {
                let owner = self
                    .state
                    .buffer_frames
                    .managed_owner(&metadata.buffer_frames, region.tail.carrier(), array.field)
                    .ok_or_else(|| invalid("append preparation lost current owner"))?;
                let Ok(Some(preparation)) = owner.prepare_private_unsigned_appends(&memory) else {
                    drop(self.state.buffer_frames.finish_private(id)?);
                    return Ok(false);
                };
                *self.state.buffer_frames.append_preparation_mut(id, at)? = Some(preparation);
            }
        }
        if proof.writes() {
            for array in 0..proof.arrays.len() {
                self.write_borrowed_word(2 + proof.private_flag(array), 0)?;
            }
        }
        Ok(true)
    }
    /// No allocation or new resource admission is needed to restore current
    /// scalar fields. Managed owners have remained in these exact lanes.
    pub(super) fn synchronize_numeric_carrier(
        &mut self,
        region: usize,
        site: usize,
        mode: u64,
    ) -> Result<(), Error> {
        let descriptor = self.descriptor;
        let metadata = descriptor.metadata();
        let id = region;
        let region = &metadata.closed_tails.regions[id];
        let Some(proof) = &region.numeric else {
            return Ok(());
        };
        for (ordinal, slot) in proof
            .fields(region.tail.carrier().get())
            .ok_or_else(|| invalid("numeric scalar frame missing"))?
            .iter()
            .enumerate()
        {
            if let Some(slot) = slot {
                let bits = self.read_borrowed_word(2 + slot)?;
                self.state.buffer_frames.synchronize_scalar(
                    &metadata.buffer_frames,
                    region.tail.carrier(),
                    ordinal,
                    bits,
                )?;
            }
        }
        if mode == 5 {
            return Ok(());
        }
        if proof.writes() || proof.result.direct_array().is_some() {
            for (at, array) in proof.arrays.iter().enumerate() {
                if let Some(private) = self.state.buffer_frames.private_array_mut(id, at)?.take() {
                    let length = if array.appends {
                        Some(self.read_borrowed_word(2 + proof.view(at) + 1)?)
                    } else {
                        None
                    };
                    let owner = Self::finish_numeric_array(private, length)?;
                    self.state.buffer_frames.synchronize_managed(
                        &metadata.buffer_frames,
                        region.tail.carrier(),
                        array.field,
                        owner,
                    )?;
                }
            }
            let publication = self.state.buffer_frames.finish_private(id)?;
            if mode == 0 {
                if let Some(array) = proof.result.direct_array() {
                    let owner = self
                        .state
                        .buffer_frames
                        .managed_owner(
                            &metadata.buffer_frames,
                            region.tail.carrier(),
                            proof.arrays[array].field,
                        )
                        .ok_or_else(|| invalid("numeric result lost final owner"))?
                        .clone();
                    self.state.arena.put_prepared(
                        publication
                            .ok_or_else(|| invalid("numeric result lost publication admission"))?,
                        owner,
                    )?;
                }
            }
        }
        if mode == 0 {
            if let Some(value) = proof.result.frame() {
                // The returned scalar need not equal a carrier scalar. Its
                // exact terminal Pack/Copy lanes were spilled at the exit.
                let frame = proof
                    .frames
                    .iter()
                    .find(|f| f.value == value)
                    .ok_or_else(|| invalid("numeric result lost exact frame lanes"))?;
                for (ordinal, (slot, array)) in frame.fields.iter().zip(&frame.arrays).enumerate() {
                    if let Some(slot) = slot {
                        let bits = self.read_borrowed_word(2 + slot)?;
                        self.state.buffer_frames.synchronize_scalar(
                            &metadata.buffer_frames,
                            region.tail.returned(),
                            ordinal,
                            bits,
                        )?;
                    } else if let Some(array) = array {
                        // Clone only after private cells have frozen. No extra
                        // entry pin can defeat the consuming first-write handoff.
                        let owner = self
                            .state
                            .buffer_frames
                            .managed_owner(
                                &metadata.buffer_frames,
                                region.tail.carrier(),
                                proof.arrays[*array].field,
                            )
                            .ok_or_else(|| invalid("numeric result lost current array owner"))?
                            .clone();
                        self.state.buffer_frames.synchronize_managed(
                            &metadata.buffer_frames,
                            region.tail.returned(),
                            ordinal,
                            owner,
                        )?;
                    } else {
                        return Err(invalid("numeric result field lacks a native lane"));
                    }
                }
            }
        }
        if mode == 2 {
            // A native bounds miss executes the original selected operation at
            // its exact source position. Materialize only that cold operand.
            let Some(Operation::Read { array, .. } | Operation::Write { array, .. }) =
                proof.action(site)
            else {
                return Err(invalid("numeric bounds exit outside read"));
            };
            let Action::LibraryCall { operands, .. } = metadata.plan.blocks()[site].action() else {
                return Err(invalid("numeric bounds lost call"));
            };
            let Field::Managed(owner) = self.state.buffer_frames.project(
                &metadata.buffer_frames,
                region.tail.carrier(),
                proof.arrays[*array].field,
                &mut self.state.control.work,
            )?
            else {
                return Err(invalid("numeric bounds lost owner"));
            };
            self.state
                .arena
                .put(operands[0].get(), owner, &mut self.state.control.work)?;
        }
        Ok(())
    }
}
