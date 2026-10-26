//! Synchronous invocation orchestration for this owner.
use super::*;
impl<'state, 'plan, 'a> Invocation<'state, 'a, 'plan> {
    pub(in crate::native_artifact::program::runtime) fn begin_closed_map(
        &mut self,
        region: usize,
    ) -> Result<Option<[u64; 4]>, Error> {
        let Some(mut state) = self.state.map_owners.state.take() else {
            return Ok(None);
        };
        let result = state.try_begin_closed(&mut MapExecution::new(self), region);
        self.state.map_owners.state = Some(state);
        result
    }
    pub(in crate::native_artifact::program::runtime) fn reconcile_closed(
        &mut self,
        region: usize,
        remaining: u64,
        mode: u64,
    ) -> Result<u64, Error> {
        if !matches!(mode, 0 | 2 | 3) {
            return Err(invalid("closed map cannot suspend"));
        }
        let index = self.descriptor.metadata().closed_tails.regions[region]
            .map
            .unwrap();
        let state = self
            .state
            .map_owners
            .state
            .as_mut()
            .ok_or_else(|| invalid("closed map session lost"))?;
        let owner = state.slots[index]
            .take_closed()
            .ok_or_else(|| invalid("closed map owner lost"))?;
        let Owner {
            mut binding,
            packed,
            packed_ssa_entered,
            guarded_loop_entered,
            guarded_calls_entered,
            funded,
            mut updates,
            error,
            ..
        } = *owner;
        if let Some(error) = error {
            state.slots[index].retain(binding);
            return Err(error);
        }
        let traversals = funded
            .checked_sub(remaining)
            .ok_or_else(|| invalid("closed map body fuel was renewed"))?;
        let is_packed = packed.is_some();
        if let Some(packed) = packed {
            if packed.remaining() != remaining || packed.funded() != funded {
                return Err(invalid("packed map fuel differs from generated body"));
            }
            updates = packed.updates();
            if mode == 0 {
                let frozen_proof = state
                    .descriptor
                    .metadata()
                    .frozen_maps
                    .regions
                    .iter()
                    .find(|proof| proof.owner == index && proof.closed == region);
                #[cfg(test)]
                let frozen_proof =
                    if crate::native_artifact::program::runtime::maps::owner::frozen::TEST_DISABLE
                        .with(std::cell::Cell::get)
                    {
                        None
                    } else {
                        frozen_proof
                    };
                if let Some(proof) = frozen_proof {
                    if state.slots[index].frozen.is_some() {
                        return Err(invalid("overlapping frozen map"));
                    }
                    let map = packed.freeze().map_err(storage_error)?;
                    if map.updates() != updates || map.traversals() != traversals {
                        return Err(invalid("frozen map completion differs"));
                    }
                    state.slots[index].stage_frozen(
                        crate::native_artifact::program::runtime::maps::owner::frozen::FrozenSlot {
                            map,
                            token: 0,
                            parent: proof.parent,
                            born_visit: self.state.control.visits,
                        },
                    );
                } else {
                    let (map, written, consumed) = packed.finish().map_err(storage_error)?;
                    if written != updates || consumed != traversals {
                        return Err(invalid("packed map completion differs"));
                    }
                    binding.install_native_map(map).map_err(storage_error)?;
                }
            }
        }
        state.slots[index].retain(binding);
        self.state.stats.hash_table_updates = add(self.state.stats.hash_table_updates, updates)?;
        if is_packed {
            self.state.stats.packed_map_updates =
                add(self.state.stats.packed_map_updates, updates)?;
        }
        if packed_ssa_entered {
            self.state.stats.packed_ssa_regions = add(self.state.stats.packed_ssa_regions, 1)?;
        }
        if guarded_loop_entered {
            self.state.stats.guarded_loop_regions = add(self.state.stats.guarded_loop_regions, 1)?;
        }
        if guarded_calls_entered {
            self.state.stats.guarded_call_regions = add(self.state.stats.guarded_call_regions, 1)?;
        }
        Ok(traversals)
    }
}
