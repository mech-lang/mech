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
