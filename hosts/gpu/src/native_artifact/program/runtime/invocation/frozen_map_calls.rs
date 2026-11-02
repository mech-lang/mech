//! Synchronous invocation orchestration for this owner.
use super::*;
impl Invocation<'_, '_, '_> {
    pub(in crate::native_artifact::program::runtime) fn frozen_native_action(
        &self,
        site: usize,
    ) -> bool {
        let proof = &self.descriptor.metadata().frozen_maps;
        proof
            .copies
            .get(site)
            .copied()
            .flatten()
            .or_else(|| proof.reads.get(site).copied().flatten())
            .and_then(|id| {
                self.state
                    .map_owners
                    .state
                    .as_ref()?
                    .slots
                    .get(proof.regions[id].owner)?
                    .frozen
                    .as_ref()
            })
            .is_some_and(|owner| owner.token != 0)
    }
    pub(in crate::native_artifact::program::runtime) fn frozen_map_helper(
        &mut self,
        site: usize,
        operation: u64,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        let mut state = self
            .state
            .map_owners
            .state
            .take()
            .ok_or_else(|| invalid("frozen-map helper before admitted session"))?;
        let result = match operation {
            FROZEN_MAP_COPY => state.copy_frozen(&mut MapExecution::new(self), site, args),
            FROZEN_MAP_READ => state.read_frozen(&mut MapExecution::new(self), site, args),
            _ => state.access_frozen(&mut MapExecution::new(self), site, operation, args),
        };
        self.state.map_owners.state = Some(state);
        let word = result?;
        self.state.consumed = true;
        Ok(Some(word))
    }
    pub(in crate::native_artifact::program::runtime) fn retire_frozen(
        &mut self,
        iteration: u32,
    ) -> Result<(), Error> {
        let Some(state) = self.state.map_owners.state.as_mut() else {
            return Ok(());
        };
        self.state.control.work.charge(
            Phase::Setup,
            amount(
                state.descriptor.metadata().frozen_maps.regions.len(),
                64,
                64,
            )?,
        )?;
        for proof in &state.descriptor.metadata().frozen_maps.regions {
            if proof.parent == Some(iteration) {
                state.slots[proof.owner].retire_frozen();
            }
        }
        Ok(())
    }
}
