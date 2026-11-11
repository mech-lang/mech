//! Invocation-current views. The admitted carrier owns every borrowed backing;
//! no ordinary helper can run while the same closed-tail loan is outstanding.
use super::{Error, Invocation, invalid};
use crate::native_artifact::program::prepare::readonly_numeric::{Array, ArrayStorage, Operation};
use crate::native_artifact::program::runtime::amount;
use crate::native_artifact::program::runtime::buffer_frames::BufferFrames;
use crate::native_artifact::program::runtime::buffer_frames::Field;
use mech_engine::native_artifact::NativeIterationAction as Action;

// A movable read of an empty immutable capture must still have a valid physical
// address. Source length remains zero and its original selector guard rejects.
static EMPTY_CAPTURE_CELL: u64 = 0;

impl Invocation<'_, '_, '_> {
    /// A descriptor-proved immutable capture stays in its current arena slot
    /// for the whole synchronous loan. Mutable roots retain private lanes.
    pub(in crate::native_artifact::program::runtime) fn numeric_array_owner(
        &self,
        array: &Array,
    ) -> Result<Option<&mech_core::Value>, Error> {
        match array.storage {
            ArrayStorage::Carrier { physical, .. } => Ok(self
                .state
                .buffer_frames
                .numeric_owner(physical, array.schema)),
            ArrayStorage::FrameCapture { physical, .. } => Ok(self
                .state
                .buffer_frames
                .numeric_owner(physical, array.schema)),
            ArrayStorage::Capture { value } => Ok(self
                .state
                .arena
                .native_entry_preview(value, &self.state.control.work)?),
        }
    }

    pub(super) fn project_numeric_array_owner(
        &mut self,
        array: &Array,
    ) -> Result<mech_core::Value, Error> {
        match array.storage {
            ArrayStorage::Carrier { physical, .. }
            | ArrayStorage::FrameCapture { physical, .. } => {
                let Field::Managed(owner) = self.state.buffer_frames.project_numeric_owner(
                    physical,
                    array.schema,
                    &mut self.state.control.work,
                )?
                else {
                    return Err(invalid("numeric view has scalar owner"));
                };
                Ok(owner)
            }
            ArrayStorage::Capture { value } => Ok(self
                .state
                .arena
                .copy_current(value, &mut self.state.control.work)?),
        }
    }

    pub(super) fn begin_numeric_views(&mut self, id: usize) -> Result<bool, Error> {
        #[cfg(all(test, feature = "canonical-native-program"))]
        if super::super::composition_tests::observe::decline_numeric_phase() {
            return Ok(false);
        }
        let descriptor = self.descriptor;
        let metadata = descriptor.metadata();
        let region = &metadata.closed_tails.regions[id];
        let proof = region
            .numeric
            .as_ref()
            .ok_or_else(|| invalid("missing numeric leaf proof"))?;
        // A worker's observed shared remainder is not an exclusive reservation.
        // Whole phases initially borrow only the synchronous invocation account.
        if proof.phase.is_some() && self.worker.is_some() {
            return Ok(false);
        }
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
            let Some(owner) = self.numeric_array_owner(array)? else {
                return Ok(false);
            };
            let Some(retained) = owner.memory_budget_retained_bytes(&memory) else {
                return Ok(false);
            };
            entry_work = entry_work.and_then(|n| n.checked_add(retained.checked_mul(8)?));
        }
        let Some(entry_work) = entry_work else {
            return Ok(false);
        };
        let mut cold_scalar_work = Some(0u64);
        for (_, bindings) in &proof.cold_scalars {
            for binding in bindings {
                if !self
                    .state
                    .buffer_frames
                    .empty_scalar_binding(binding.physical)
                {
                    return Ok(false);
                }
                cold_scalar_work = cold_scalar_work.and_then(|n| {
                    n.checked_add((binding.steps as u64).checked_mul(64)?.checked_add(64)?)
                });
            }
        }
        for (_, bindings) in &proof.cold_frames {
            for &(physical, array) in bindings {
                let root = &proof.arrays[array];
                if Some(physical) != root.physical()
                    && !self.state.buffer_frames.empty_numeric_owner(physical)
                {
                    return Ok(false);
                }
                cold_scalar_work = cold_scalar_work.and_then(|n| n.checked_add(64));
            }
        }
        let Some(entry_work) = cold_scalar_work.and_then(|n| n.checked_add(entry_work)) else {
            return Ok(false);
        };
        let Some(cold_alias_count) = proof
            .cold_aliases
            .iter()
            .try_fold(0usize, |n, (_, aliases)| n.checked_add(aliases.len()))
        else {
            return Ok(false);
        };
        let Some(entry_work) =
            entry_work.checked_add(BufferFrames::cold_publication_work(cold_alias_count)?)
        else {
            return Ok(false);
        };
        let Some((remaining, _)) = self.state.library.unscoped_remaining() else {
            return Ok(false);
        };
        let mut publication_work = if let Some(array) = proof.result.direct_array() {
            self.numeric_array_owner(&proof.arrays[array])?
                .and_then(|v| v.memory_budget_retained_bytes(&memory))
                .and_then(|n| n.checked_mul(4))
                .and_then(|n| n.checked_add(64))
        } else {
            Some(0)
        };
        // Tickets bind empty current slots, exact schemas and this arena
        // generation before entering generated code. They retain no Value and
        // therefore cannot introduce a hidden COW alias during a private loan.
        for (_, aliases) in &proof.cold_aliases {
            for alias in aliases {
                let Some(array) = proof.arrays.get(alias.array) else {
                    return Err(invalid("numeric cold alias has a foreign array"));
                };
                let Some(owner) = self.numeric_array_owner(array)? else {
                    return Err(invalid("numeric cold alias lost its current owner"));
                };
                let Some(work) = self.state.arena.native_alias_put_work(
                    alias.value,
                    owner,
                    &self.state.control.work,
                )?
                else {
                    return Ok(false);
                };
                publication_work = publication_work.and_then(|n| n.checked_add(work));
            }
        }
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
                .numeric_array_owner(array)?
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
            let Some(mut bytes) = scalar_bytes.checked_mul(frame.scalar_nodes() as u64) else {
                return Ok(false);
            };
            for array in frame.arrays.iter().flatten() {
                let bound = self.read_borrowed_word(2 + proof.view(*array))?;
                let Some(total) = bytes.checked_add(bound) else {
                    return Ok(false);
                };
                bytes = total;
            }
            if self
                .limits
                .max_output_elements
                .is_some_and(|n| n < frame.maximum_arity as u64)
                || self
                    .limits
                    .max_output_bytes
                    .is_some_and(|n| n < bytes.max(8))
            {
                return Ok(false);
            }
        }
        // Fill only fixed, already admitted transport. The carrier retains each
        // owner throughout native execution; this temporary clone is setup only.
        for (at, array) in proof.arrays.iter().enumerate() {
            if array.witness == usize::MAX {
                if array.width != 0 || array.writable || array.appends {
                    return Err(invalid("opaque numeric leaf has content effects"));
                }
                for index in 0..4 {
                    self.write_borrowed_word(2 + proof.view(at) + index, 0)?;
                }
                continue;
            }
            let owner = self.project_numeric_array_owner(array)?;
            let call = metadata.calls[array.witness]
                .as_ref()
                .ok_or_else(|| invalid("numeric view lost factory"))?;
            let Some(mut view) =
                call.unsigned_matrix_native_view(&owner, &self.state.library.memory_budget())
            else {
                return Ok(false);
            };
            if array.immutable_capture() && view[1] == 0 {
                view[0] = (&EMPTY_CAPTURE_CELL as *const u64) as u64;
            }
            // A no-op store still has the selected array output contract.
            // Decline, rather than reject at entry: an earlier assertion or
            // unselected store must keep its original source/error ordering.
            if self
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
        for binding in proof.scalar_inputs.iter().chain(&proof.scalar_captures) {
            self.state.control.work.charge(
                crate::native_artifact::NativeExecutionPhase::Setup,
                amount(binding.steps, 64, 0)?,
            )?;
            if !self
                .state
                .buffer_frames
                .current_scalar_binding(binding.physical, binding.kind)
            {
                return Err(invalid("numeric scalar has no current word"));
            }
        }
        if proof.writes() || proof.result.direct_array().is_some() || cold_alias_count != 0 {
            let publication = if let Some(array) = proof.result.direct_array() {
                let owner = self
                    .numeric_array_owner(&proof.arrays[array])?
                    .ok_or_else(|| invalid("numeric result lost owner"))?
                    .clone();
                Some(self.state.arena.prepare_put(
                    region.tail.returned().get(),
                    &owner,
                    &mut self.state.control.work,
                )?)
            } else {
                None
            };
            if !self.state.buffer_frames.prepare_private(
                id,
                proof.arrays.len(),
                proof.arrays.iter().any(|a| a.appends),
                publication,
                cold_alias_count,
                &memory,
            )? {
                return Ok(false);
            }
            for (at, array) in proof.arrays.iter().enumerate().filter(|(_, a)| a.appends) {
                let owner = self
                    .numeric_array_owner(array)?
                    .ok_or_else(|| invalid("append preparation lost current owner"))?;
                let Ok(Some(preparation)) = owner.prepare_private_unsigned_appends(&memory) else {
                    drop(self.state.buffer_frames.finish_private(id)?);
                    return Ok(false);
                };
                *self.state.buffer_frames.append_preparation_mut(id, at)? = Some(preparation);
            }
            for (index, alias) in proof
                .cold_aliases
                .iter()
                .flat_map(|(_, aliases)| aliases)
                .enumerate()
            {
                let owner = self
                    .numeric_array_owner(&proof.arrays[alias.array])?
                    .ok_or_else(|| invalid("numeric cold alias lost its admitted owner"))?
                    .clone();
                let ticket = self.state.arena.prepare_put(
                    alias.value,
                    &owner,
                    &mut self.state.control.work,
                )?;
                self.state
                    .buffer_frames
                    .set_private_alias(id, index, alias.value, ticket)?;
            }
        }
        if let Some(phase) = &proof.phase {
            for debit in &phase.loops {
                self.write_borrowed_word(2 + debit.counter, 0)?;
                if let Some(word) = debit.first_visit {
                    self.write_borrowed_word(2 + word, 0)?;
                }
            }
        }
        if proof.writes() {
            for array in 0..proof.arrays.len() {
                self.write_borrowed_word(2 + proof.private_flag(array), 0)?;
            }
        }
        if !proof.scalar_inputs.is_empty()
            || !proof.scalar_captures.is_empty()
            || region.numeric_tree.is_some()
        {
            let bank = self
                .state
                .buffer_frames
                .scalar_bank_pointer(&metadata.buffer_frames)?;
            self.write_borrowed_word(2, bank)?;
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
        if mode == 0 && matches!(proof.result, crate::native_artifact::program::prepare::readonly_numeric::ResultRecipe::Unreachable) {
            return Err(invalid("native interval reached an ordinary-only finish"));
        }
        if (mode != 0 && mode != 3) || (region.numeric_tree.is_none() && proof.phase.is_some()) {
            // Resumable fuel/bounds/first-write boundaries restore current
            // scalar fields. Terminal mode3 cannot publish or resume a scalar
            // consumer; owner/shape/fuel reconciliation below remains intact.
            for binding in &proof.scalar_inputs {
                let bits = self.read_borrowed_word(2 + binding.word)?;
                self.state.buffer_frames.synchronize_scalar_binding(
                    binding.physical,
                    binding.kind,
                    bits,
                )?;
            }
        }
        if mode == 5 {
            return Ok(());
        }
        if proof.writes()
            || proof.result.direct_array().is_some()
            || proof
                .cold_aliases
                .iter()
                .any(|(_, aliases)| !aliases.is_empty())
        {
            for (at, array) in proof.arrays.iter().enumerate() {
                if let Some(private) = self.state.buffer_frames.private_array_mut(id, at)?.take() {
                    let length = if array.appends {
                        Some(self.read_borrowed_word(2 + proof.view(at) + 1)?)
                    } else {
                        None
                    };
                    let owner = Self::finish_numeric_array(private, length)?;
                    self.state.buffer_frames.synchronize_numeric_owner(
                        array.physical().ok_or_else(|| {
                            invalid("captured array cannot receive a mutable owner")
                        })?,
                        array.schema,
                        owner,
                    )?;
                }
            }
            let publication = self.state.buffer_frames.finish_private(id)?;
            if mode == 0 {
                if let Some(array) = proof.result.direct_array() {
                    let owner = self
                        .numeric_array_owner(&proof.arrays[array])?
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
                // Success-edge generated stores publish the exact scalar
                // result after this checked managed-owner reconciliation.
                for &(physical, array) in &frame.array_bindings {
                    {
                        // Clone only after private cells have frozen. No extra
                        // entry pin can defeat the consuming first-write handoff.
                        let owner = self
                            .numeric_array_owner(&proof.arrays[array])?
                            .ok_or_else(|| invalid("numeric result lost current array owner"))?
                            .clone();
                        self.state.buffer_frames.synchronize_numeric_owner(
                            physical,
                            proof.arrays[array].schema,
                            owner,
                        )?;
                    }
                }
            }
        }
        if matches!(mode, 6 | 7) {
            self.export_numeric_continuation(id, site)?;
        }
        if mode == 2 {
            // A bounds or bulk-count rejection executes the original selected
            // operation at its exact source position. Export only its owner.
            let Some(
                Operation::Read { array, .. }
                | Operation::Write { array, .. }
                | Operation::AppendLeBytes { array, .. },
            ) = proof.action(site)
            else {
                return Err(invalid("numeric rejection outside selected access"));
            };
            let Action::LibraryCall { operands, .. } = metadata.plan.blocks()[site].action() else {
                return Err(invalid("numeric bounds lost call"));
            };
            let owner = self.project_numeric_array_owner(&proof.arrays[*array])?;
            self.state
                .arena
                .put(operands[0].get(), &owner, &mut self.state.control.work)?;
        }
        Ok(())
    }
}
