//! String map owner loans inside the existing compiled host round.
use super::*;
impl Context<'_, '_> {
    pub(in crate::native_artifact::program_runtime) fn prepare_string_map_region(
        &mut self,
        site: usize,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        if self.window_region.is_some() || self.consumed || args[2] != 0 {
            return Err(invalid("duplicate compiled String map preparation"));
        }
        let descriptor = self.control.descriptor;
        self.control.work.charge(
            super::super::NativeExecutionPhase::Setup,
            super::super::amount(descriptor.metadata().string_map_regions.len(), 128, 256)?,
        )?;
        let region = descriptor.metadata().string_map_regions
            .iter()
            .find(|region| {
                u64::from(region.collection()) == args[1]
                    && matches!(&descriptor.metadata().plan.iterations()[region.iteration() as usize],
                NativeControlLoop::Collection(generator) if generator.header() as usize==site)
            })
            .ok_or_else(|| invalid("compiled String map outside its immutable plan"))?;
        if self.control.current != Some(region.iteration())
            || self.control.progress[region.iteration() as usize * 2] != args[0]
        {
            return Err(invalid("compiled String map source count differs"));
        }
        let mut state = self
            .collections
            .take()
            .ok_or_else(|| invalid("compiled String map lacks state"))?;
        enum Bound {
            Complete,
            Kernel([u64; 5]),
            Declined,
        }
        let result = (|| {
            if let Some(processed) = state.prepare_joined_string_map(self, region, args[0])? {
                state.finish_compiled_string_map(self, region, args[0], processed)?;
                Ok(Bound::Complete)
            } else {
                state
                    .prepare_compiled_string_map(self, region, args[0])
                    .map(|words| words.map_or(Bound::Declined, Bound::Kernel))
            }
        })();
        self.collections = Some(state);
        let words = match result? {
            Bound::Declined => return Ok(Some(0)),
            Bound::Complete => {
                self.control.progress[region.iteration() as usize * 2] = 0;
                self.call_inputs.values_mut()[..2].fill(None);
                return Ok(Some(2));
            }
            Bound::Kernel(words) => words,
        };
        self.window_region = Some((region.collection(), args[0]));
        self.write_window_arguments(words)?;
        Ok(Some(1))
    }
    pub(in crate::native_artifact::program_runtime) fn finish_string_map_region(
        &mut self,
        site: usize,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        let (collection, accepted) = self
            .window_region
            .ok_or_else(|| invalid("compiled String map lost loans"))?;
        if args[1] != u64::from(collection) || args[2] != 0 || args[0] > accepted {
            return Err(invalid("compiled String map result exceeds source"));
        }
        let descriptor = self.control.descriptor;
        let region = descriptor
            .metadata()
            .string_map_regions
            .iter()
            .find(|r| r.collection() == collection)
            .ok_or_else(|| invalid("compiled String map lost its plan"))?;
        let NativeControlLoop::Collection(generator) =
            &descriptor.metadata().plan.iterations()[region.iteration() as usize]
        else {
            return Err(invalid("compiled String map lost generator"));
        };
        if generator.header() as usize != site || self.control.current != Some(region.iteration()) {
            return Err(invalid("compiled String map finished outside entry"));
        }
        let mut state = self
            .collections
            .take()
            .ok_or_else(|| invalid("compiled String map lost state"))?;
        let result = state.finish_compiled_string_map(self, region, accepted, args[0]);
        self.collections = Some(state);
        result?;
        self.control.progress[region.iteration() as usize * 2] = 0;
        self.call_inputs.values_mut()[..2].fill(None);
        self.window_region = None;
        Ok(None)
    }
}
