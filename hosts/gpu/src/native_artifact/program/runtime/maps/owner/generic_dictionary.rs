//! One completed owned dictionary and its exact virtual source generation.
use super::*;
use crate::native_artifact::NativeValueHandle;
use crate::native_artifact::program::runtime::maps::string::frozen::FrozenStringMap;

pub(in crate::native_artifact::program::runtime) struct Dictionary {
    pub map: FrozenStringMap,
    pub handle: NativeValueHandle,
    pub value: u32,
    pub owner: usize,
    pub born: u64,
    pub header: usize,
}
impl Session<'_> {
    pub(in crate::native_artifact::program::runtime) fn dictionary_nodes(
        &self,
        work: &mut NativeWorkMeter,
    ) -> Result<u64, Error> {
        let Some(state) = self.state.as_ref() else {
            return Ok(0);
        };
        if state.dictionaries.is_empty() {
            return Ok(0);
        }
        work.charge(Phase::Setup, amount(state.dictionaries.len(), 64, 64)?)?;
        let mut nodes = amount(
            state.dictionaries.len(),
            (2 * size_of::<Option<Dictionary>>()) as u64,
            0,
        )?;
        for owner in state.dictionaries.iter().flatten() {
            nodes = add(nodes, owner.map.retained_nodes())?;
        }
        Ok(nodes)
    }
}
impl Invocation<'_, '_, '_> {
    pub(in crate::native_artifact::program::runtime) fn take_dictionary(
        &mut self,
        site: usize,
    ) -> Result<Option<Dictionary>, Error> {
        let Some(proof) = self.descriptor.metadata().generic_dictionary.header(site) else {
            return Ok(None);
        };
        let Some(state) = self.state.map_owners.state.as_mut() else {
            return Ok(None);
        };
        let id = self
            .descriptor
            .metadata()
            .generic_dictionary
            .regions
            .iter()
            .position(|r| r.owner == proof.owner)
            .ok_or_else(|| invalid("dictionary bank lost owner"))?;
        let Some(dictionary) = state.dictionaries[id].take() else {
            return Ok(None);
        };
        if dictionary.value != proof.result.get()
            || dictionary.owner != proof.owner
            || dictionary.header != site
            || dictionary.born >= self.state.control.visits
        {
            return Err(invalid("stale or foreign dictionary source"));
        }
        dictionary
            .map
            .check_account(&self.state.library)
            .map_err(storage_error)?;
        self.state
            .arena
            .check_virtual(dictionary.handle, &mut self.state.control.work)?;
        Ok(Some(dictionary))
    }
    pub(in crate::native_artifact::program::runtime) fn install_dictionary(
        &mut self,
        site: usize,
        map: FrozenStringMap,
    ) -> Result<(), Error> {
        let proof = self
            .descriptor
            .metadata()
            .generic_dictionary
            .exit(site)
            .ok_or_else(|| invalid("dictionary exit lost proof"))?;
        let ty = self.descriptor.metadata().types[proof.result.get() as usize];
        if map.schema() != ty.schema {
            return Err(invalid("dictionary output schema differs"));
        }
        let handle = self.state.arena.define_virtual(
            proof.result.get(),
            ty.schema,
            ty.key,
            &mut self.state.control.work,
        )?;
        let state = self
            .state
            .map_owners
            .state
            .as_mut()
            .ok_or_else(|| invalid("dictionary exit lost state"))?;
        let id = self
            .descriptor
            .metadata()
            .generic_dictionary
            .regions
            .iter()
            .position(|r| r.owner == proof.owner)
            .ok_or_else(|| invalid("dictionary exit lost bank"))?;
        if state.dictionaries[id].is_some() {
            return Err(invalid("redefining live native dictionary"));
        }
        state.dictionaries[id] = Some(Dictionary {
            map,
            handle,
            value: proof.result.get(),
            owner: proof.owner,
            born: self.state.control.visits,
            header: proof.lookup_header as usize,
        });
        Ok(())
    }
    pub(in crate::native_artifact::program::runtime) fn materialize_dictionary(
        &mut self,
        site: usize,
        dictionary: Dictionary,
    ) -> Result<(), Error> {
        self.state
            .arena
            .check_virtual(dictionary.handle, &mut self.state.control.work)?;
        dictionary
            .map
            .check_account(&self.state.library)
            .map_err(storage_error)?;
        let geometry = self
            .retained_caller_geometry(site)?
            .ok_or_else(|| invalid("dictionary fallback lacks complete caller inventory"))?;
        // The moved table is outside the registry, so add its real private nodes.
        let geometry = PublicationGeometry {
            live_bytes: geometry.live_bytes,
            live_nodes: add(geometry.live_nodes, dictionary.map.retained_nodes())?,
        };
        let value = dictionary
            .map
            .materialize(geometry)
            .map_err(storage_error)?;
        self.state
            .arena
            .put(dictionary.value, &value, &mut self.state.control.work)?;
        Ok(())
    }
}

#[cfg(test)]
impl Session<'_> {
    pub(in crate::native_artifact::program::runtime) fn has_dictionary(&self, value: u32) -> bool {
        self.state
            .as_ref()
            .is_some_and(|s| s.dictionaries.iter().flatten().any(|d| d.value == value))
    }
}
