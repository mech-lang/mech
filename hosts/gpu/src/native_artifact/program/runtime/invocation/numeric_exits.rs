//! Publish only the exact live internal values at a proved cold source edge.
//! Private cells have already frozen; no native view survives ordinary code.
use super::{Error, Invocation, invalid};

impl Invocation<'_, '_, '_> {
    pub(super) fn export_numeric_continuation(
        &mut self,
        region: usize,
        site: usize,
    ) -> Result<(), Error> {
        let metadata = self.descriptor.metadata();
        let region_id = region;
        let region = &metadata.closed_tails.regions[region];
        let proof = region
            .numeric
            .as_ref()
            .ok_or_else(|| invalid("cold exit lacks numeric state"))?;
        let exit = proof
            .exits
            .iter()
            .find(|exit| exit.to == site)
            .ok_or_else(|| invalid("cold exit lacks an exact continuation recipe"))?;
        for &value in &exit.frames {
            let frame = proof
                .frames
                .iter()
                .find(|frame| frame.value == value)
                .ok_or_else(|| invalid("cold frame lacks its current lanes"))?;
            if let Some(variant) = &proof.tagged
                && variant.contains(value)
            {
                if variant.prefix.is_some() {
                    self.export_numeric_prefix(region_id, value)?;
                    continue;
                }
                let (_, bindings) = variant
                    .scalars
                    .iter()
                    .find(|(v, _)| *v == value)
                    .ok_or_else(|| invalid("numeric enum lost physical scalar bindings"))?;
                for binding in bindings {
                    self.state.control.work.charge(
                        crate::native_artifact::NativeExecutionPhase::Setup,
                        super::super::amount(binding.steps + 2, 64, 64)?,
                    )?;
                    let bits = self.read_borrowed_word(2 + binding.word)?;
                    self.state.buffer_frames.synchronize_scalar_binding(
                        binding.physical,
                        binding.kind,
                        bits,
                    )?;
                }
                for &(physical, array) in &frame.array_bindings {
                    let owner = self
                        .numeric_array_owner(&proof.arrays[array])?
                        .ok_or_else(|| invalid("numeric enum lost current array"))?
                        .clone();
                    self.state
                        .control
                        .work
                        .charge(crate::native_artifact::NativeExecutionPhase::Setup, 256)?;
                    self.state.buffer_frames.synchronize_numeric_owner(
                        physical,
                        proof.arrays[array].schema,
                        owner,
                    )?;
                }
                self.state.buffer_frames.synchronize_numeric_tag(
                    &metadata.buffer_frames,
                    value,
                    variant.ordinal,
                    &mut self.state.control.work,
                )?;
                continue;
            }
            for (ordinal, (word, array)) in frame.fields.iter().zip(&frame.arrays).enumerate() {
                if let Some(word) = word {
                    let bits = self.read_borrowed_word(2 + word)?;
                    self.state.buffer_frames.synchronize_scalar_at(
                        &metadata.buffer_frames,
                        value,
                        ordinal,
                        bits,
                    )?;
                } else if let Some(array) = array {
                    let owner = self
                        .numeric_array_owner(&proof.arrays[*array])?
                        .ok_or_else(|| invalid("cold frame lost its current buffer owner"))?
                        .clone();
                    self.state.buffer_frames.synchronize_managed_at(
                        &metadata.buffer_frames,
                        value,
                        ordinal,
                        owner,
                    )?;
                } else {
                    return Err(invalid("cold frame has an unrepresented field"));
                }
            }
        }
        for &(value, array) in &exit.aliases {
            let owner = self
                .numeric_array_owner(&proof.arrays[array])?
                .ok_or_else(|| invalid("cold alias lost its current buffer owner"))?
                .clone();
            self.state
                .arena
                .put(value, &owner, &mut self.state.control.work)?;
        }
        // The ordinary continuation owns its exact live exports now. Keeping a
        // dead carrier pin would force otherwise exclusive suffix updates to
        // copy. This releases only that proved dead visit; live exported aliases
        // and Core's final consuming-update check keep their normal authority.
        if exit.retire_carrier
            && !self.state.buffer_frames.retire(
                &metadata.frame_retirements,
                region.tail.carrier().get(),
                &mut self.state.control.work,
            )?
        {
            return Err(invalid("cold carrier lacks its retirement recipe"));
        }
        Ok(())
    }
}
