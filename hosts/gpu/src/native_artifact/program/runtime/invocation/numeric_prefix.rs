//! Checked transport between a private common-prefix bank and the original FSM.
//! The original active variant retains every owner throughout the native loan.
use super::{Error, Invocation, invalid};
use crate::native_artifact::program::prepare::readonly_numeric::{Array, ArrayStorage};
use mech_engine::native_artifact::NativeBufferFrameLane as Lane;

impl Invocation<'_, '_, '_> {
    pub(super) fn numeric_array_physical(&self, array: &Array) -> Result<Option<usize>, Error> {
        match array.storage {
            ArrayStorage::Carrier { physical, .. }
            | ArrayStorage::FrameCapture { physical, .. } => Ok(Some(physical)),
            ArrayStorage::Capture { .. } | ArrayStorage::PhaseLocal { .. } => Ok(None),
            ArrayStorage::TaggedCarrier { value, field } => {
                let layout = &self.descriptor.metadata().buffer_frames;
                let [root] = layout
                    .fields_at(value as usize)
                    .ok_or_else(|| invalid("numeric FSM root missing"))?
                else {
                    return Err(invalid("numeric FSM root is not one enum"));
                };
                let tag = self.state.buffer_frames.current_tag(*root)?;
                let variant = layout
                    .tagged_variant(*root, tag as u32)
                    .ok_or_else(|| invalid("numeric FSM variant missing"))?;
                let target = layout
                    .children(variant)
                    .unwrap_or(core::slice::from_ref(&variant))
                    .get(field)
                    .ok_or_else(|| invalid("numeric FSM managed field missing"))?;
                let Lane::Managed { at } = target.lane() else {
                    return Err(invalid("numeric FSM root is not managed"));
                };
                Ok(Some(at))
            }
        }
    }

    /// Immutable captures retain the existing prohibition on mutable owner
    /// publication, even though reads can resolve their physical frame homes.
    pub(super) fn numeric_array_mutable_home(&self, array: &Array) -> Result<Option<usize>, Error> {
        if matches!(
            array.storage,
            ArrayStorage::Carrier { .. } | ArrayStorage::TaggedCarrier { .. }
        ) {
            self.numeric_array_physical(array)
        } else {
            Ok(None)
        }
    }

    pub(super) fn validate_numeric_prefix(&mut self, id: usize) -> Result<(), Error> {
        let metadata = self.descriptor.metadata();
        let proof = metadata.closed_tails.regions[id]
            .numeric
            .as_ref()
            .ok_or_else(|| invalid("numeric prefix lost proof"))?;
        let Some(variant) = &proof.tagged else {
            return Ok(());
        };
        let Some(prefix) = &variant.prefix else {
            return Ok(());
        };
        let [root] = metadata
            .buffer_frames
            .fields_at(variant.carrier as usize)
            .ok_or_else(|| invalid("numeric prefix carrier missing"))?
        else {
            return Err(invalid("numeric prefix carrier is not one enum"));
        };
        let ordinal = self.state.buffer_frames.current_tag(*root)?;
        let physical = prefix
            .physical(variant.carrier, ordinal as u32)
            .ok_or_else(|| invalid("numeric prefix entry variant missing"))?;
        // Importer branches on this exact current tag and reads only its
        // validated scalar homes. Absent scalar suffixes are native zeros.
        for binding in &physical.scalars {
            if !self
                .state
                .buffer_frames
                .current_scalar_binding(binding.physical, binding.kind)
            {
                return Err(invalid("numeric prefix scalar is not current"));
            }
        }
        Ok(())
    }

    pub(super) fn export_numeric_prefix(&mut self, id: usize, value: u32) -> Result<(), Error> {
        let metadata = self.descriptor.metadata();
        let proof = metadata.closed_tails.regions[id]
            .numeric
            .as_ref()
            .ok_or_else(|| invalid("numeric prefix export lost proof"))?;
        let prefix = proof
            .tagged
            .as_ref()
            .and_then(|v| v.prefix.as_ref())
            .ok_or_else(|| invalid("numeric prefix export lacks tag proof"))?;
        let tag = prefix
            .tag(value)
            .ok_or_else(|| invalid("numeric prefix export tag missing"))?;
        let ordinal = u32::try_from(self.read_borrowed_word(2 + tag)?)
            .map_err(|_| invalid("numeric prefix export tag overflows"))?;
        let physical = prefix
            .physical(value, ordinal)
            .ok_or_else(|| invalid("numeric prefix export variant missing"))?;
        for binding in &physical.scalars {
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
        for &(target, array) in &physical.arrays {
            // Resolve every source while its ORIGINAL entry tag is current.
            // All managed fields occur in every variant of this proved prefix.
            let owner = self
                .numeric_array_owner(&proof.arrays[array])?
                .ok_or_else(|| invalid("numeric prefix export lost current owner"))?
                .clone();
            self.state
                .control
                .work
                .charge(crate::native_artifact::NativeExecutionPhase::Setup, 256)?;
            self.state.buffer_frames.synchronize_numeric_owner(
                target,
                proof.arrays[array].schema,
                owner,
            )?;
        }
        // Clearing inactive owners can occur only after every dirty owner has
        // frozen and every active payload home is complete. Publish tag last.
        self.state.buffer_frames.synchronize_numeric_tag(
            &metadata.buffer_frames,
            value,
            ordinal,
            &mut self.state.control.work,
        )?;
        Ok(())
    }
}
