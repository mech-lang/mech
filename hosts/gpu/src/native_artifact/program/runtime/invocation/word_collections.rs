//! Exact fresh collection-initializer loans for a compiler-proved total scalar CFG.
use super::{CollectionExecution, Invocation};
use crate::native_artifact::program::runtime::{NativeProgramExecutionError as Error, invalid};

impl Invocation<'_, '_, '_> {
    pub(in crate::native_artifact::program::runtime) fn prepare_word_collection(
        &mut self,
        site: usize,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        if self.window_region.is_some()
            || self.closed_tail.is_some()
            || !self.state.consumed
            || args[2] != 0
        {
            return Err(invalid("duplicate or malformed word collection loan"));
        }
        let regions = &self.descriptor.metadata().word_collections;
        let id =
            u32::try_from(args[0]).map_err(|_| invalid("word collection identity overflow"))?;
        let at = regions
            .binary_search_by_key(&id, |r| r.collection)
            .map_err(|_| invalid("word collection has no total source proof"))?;
        let region = &regions[at];
        if region.initialize as usize != site
            || self.state.control.previous != Some(region.initialize)
            || self.state.control.current != Some(region.iteration)
        {
            return Err(invalid(
                "word collection outside its completed source initializer",
            ));
        }
        let mut state = self
            .state
            .collections
            .take()
            .ok_or_else(|| invalid("word collection has no active builder"))?;
        let result =
            state.prepare_word_collection(&mut CollectionExecution::new(self), region, args[1]);
        self.state.collections = Some(state);
        let Some(words) = result? else {
            return Ok(Some(0));
        };
        for (index, word) in words.into_iter().enumerate() {
            self.write_borrowed_word(index, word)?;
        }
        self.window_region = Some((region.collection, args[1]));
        Ok(Some(1))
    }

    pub(in crate::native_artifact::program::runtime) fn finish_word_collection(
        &mut self,
        site: usize,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        let (collection, count) = self
            .window_region
            .ok_or_else(|| invalid("word collection finish has no active loans"))?;
        if args != [u64::from(collection), count, 0] || self.closed_tail.is_some() {
            return Err(invalid(
                "word collection finish differs from its active loan",
            ));
        }
        let regions = &self.descriptor.metadata().word_collections;
        let at = regions
            .binary_search_by_key(&collection, |r| r.collection)
            .map_err(|_| invalid("word collection lost its total source proof"))?;
        let region = &regions[at];
        if region.initialize as usize != site
            || self.state.control.previous != Some(region.initialize)
            || self.state.control.current != Some(region.iteration)
            || self.progress[region.iteration as usize * 2..][..2] != [count, 1]
        {
            return Err(invalid(
                "word collection source invocation changed during its loan",
            ));
        }
        let mut state = self
            .state
            .collections
            .take()
            .ok_or_else(|| invalid("word collection lost its active builder"))?;
        let result = state.finish_word_collection(region, count);
        self.state.collections = Some(state);
        result?;
        self.progress[region.iteration as usize * 2] = 0;
        self.window_region = None;
        self.state.consumed = true;
        Ok(None)
    }
}
