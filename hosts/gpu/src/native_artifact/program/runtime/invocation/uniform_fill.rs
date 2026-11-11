//! Checked preparation/publication around one emitted uniform construction.
use super::*;
use crate::native_artifact::program::protocol::GlobalHelper;
impl Invocation<'_, '_, '_> {
    pub(super) fn uniform_fill_helper(
        &mut self,
        site: usize,
        tag: GlobalHelper,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        let metadata = self.descriptor.metadata();
        self.state.control.work.charge(
            crate::native_artifact::NativeExecutionPhase::Setup,
            super::amount(metadata.uniform_word_collections.len(), 32, 128)?,
        )?;
        let region = metadata
            .uniform_word_collections
            .iter()
            .find(|region| region.header() as usize == site)
            .ok_or_else(|| invalid("native fill outside proved header"))?;
        if self.state.control.previous != Some(site as u32)
            || self.state.control.current != Some(region.iteration())
        {
            return Err(invalid("native fill outside original admitted header"));
        }
        let prepare = tag == GlobalHelper::UniformFillPrepare;
        if prepare
            && (self.worker.is_some()
                || self.has_uninventoried_private_owners()
                || self.closed_tail.is_some()
                || self.window_region.is_some()
                || self.state.scalar_path.is_some())
        {
            return Ok(Some(0));
        }
        if prepare
            && self.state.collections.as_ref().is_none_or(|state| {
                !state.fresh_uniform_header(region.collection(), region.iteration(), self.progress)
            })
        {
            return Ok(Some(0));
        }
        let prior = if prepare {
            let Some(geometry) = self.retained_caller_geometry(site)? else {
                return Ok(Some(0));
            };
            Some(mech_core::ResourceDemand {
                turn_peak_bytes: geometry.live_bytes,
                retained_nodes: geometry.live_nodes,
                ..Default::default()
            })
        } else {
            None
        };
        let mut state = self
            .state
            .collections
            .take()
            .ok_or_else(|| invalid("native fill has no builder"))?;
        let result = (|| {
            let mut ctx = CollectionExecution::new(self);
            match tag {
                GlobalHelper::UniformFillPrepare => {
                    if args[2] != 0 {
                        return Err(invalid("native fill prepare payload"));
                    }
                    state
                        .prepare_uniform_words(&mut ctx, region, args[0], args[1], prior.unwrap())
                        .map(|pointer| Some(pointer.unwrap_or(0)))
                }
                GlobalHelper::UniformFillFinish => {
                    if args[0] == 0 || args[1] > 1 || args[2] != 0 {
                        return Err(invalid("native fill finish payload"));
                    }
                    state
                        .finish_uniform_words(&mut ctx, site, args[0], args[1])
                        .map(|()| Some(1))
                }
                _ => Err(invalid("unknown native fill helper")),
            }
        })();
        self.state.collections = Some(state);
        let result = result?;
        if !prepare {
            self.progress[region.iteration() as usize * 2] = 0;
        }
        Ok(result)
    }
}
