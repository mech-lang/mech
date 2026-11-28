//! Exact original-header admission and paired synchronous generator completion.
use super::*;
use crate::native_artifact::program::protocol::GlobalHelper;
impl Invocation<'_, '_, '_> {
    pub(super) fn numeric_generator_helper(
        &mut self,
        site: usize,
        tag: GlobalHelper,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        let selected = self
            .descriptor
            .metadata()
            .numeric_generators
            .at_header(site)
            .ok_or_else(|| invalid("generator outside a selected original header"))?;
        let region = &selected.region;
        if self.state.control.previous != Some(site as u32)
            || self.state.control.current != Some(region.iteration)
            || self.closed_tail.is_some()
            || self.window_region.is_some()
            || self.state.scalar_path.is_some()
        {
            return Err(invalid("generator lost its original checked header visit"));
        }
        let prior = if tag == GlobalHelper::NumericGeneratorPrepare {
            if args != [0; 3] {
                return Err(invalid("generator entry has payload"));
            }
            if self.worker.is_some()
                || self.has_uninventoried_private_owners()
                || self.state.collections.as_ref().is_none_or(|c| {
                    !c.fresh_numeric_header(region.collection, region.iteration, self.progress)
                })
            {
                return Ok(Some(0));
            }
            // Prove complete ordinary replay before any fallible caller query.
            let b = &selected.budget;
            let credit = b
                .replay
                .checked_add(b.setup)
                .and_then(|n| n.checked_add(b.native))
                .and_then(|n| n.checked_add(b.compute))
                .ok_or_else(|| invalid("generator replay credit overflow"))?;
            if self
                .state
                .control
                .work
                .limit()
                .saturating_sub(self.state.control.work.used())
                < credit
                || self
                    .state
                    .library
                    .unscoped_remaining()
                    .is_none_or(|(w, c)| w < credit || c < b.comparison.saturating_mul(2))
            {
                return Ok(Some(0));
            }
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
            .ok_or_else(|| invalid("generator has no original collection owner"))?;
        let result = (|| {
            let mut ctx = CollectionExecution::new(self);
            match tag {
                GlobalHelper::NumericGeneratorPrepare => state
                    .prepare_numeric_generator(&mut ctx, selected, prior.unwrap())
                    .map(|p| Some(p.unwrap_or(0))),
                GlobalHelper::NumericGeneratorFinish => {
                    if args[0] == 0 || args[1] > 2 || args[2] != 0 {
                        return Err(invalid("generator finish status is malformed"));
                    }
                    state
                        .finish_numeric_generator(&mut ctx, selected, args[0], args[1])
                        .map(|done| Some(u64::from(done)))
                }
                _ => Err(invalid("unknown generator helper")),
            }
        })();
        self.state.collections = Some(state);
        let result = result?;
        if tag == GlobalHelper::NumericGeneratorFinish && result == Some(1) {
            self.progress[region.iteration as usize * 2] = 0;
        }
        Ok(result)
    }
}
