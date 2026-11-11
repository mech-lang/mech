//! Checked entry/finalization for a complete emitted immutable lookup region.
use super::*;
use crate::native_artifact::program::protocol::GlobalHelper;

impl Invocation<'_, '_, '_> {
    pub(super) fn numeric_map_helper(
        &mut self,
        site: usize,
        tag: GlobalHelper,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        let metadata = self.descriptor.metadata();
        let region = metadata
            .numeric_maps
            .regions
            .iter()
            .find(|region| region.header as usize == site)
            .ok_or_else(|| invalid("native numeric map outside a selected header"))?;
        if self.state.control.previous != Some(site as u32)
            || self.state.control.current != Some(region.iteration)
            || self.closed_tail.is_some()
            || self.window_region.is_some()
            || self.state.scalar_path.is_some()
        {
            return Err(invalid(
                "native numeric map outside original checked header visit",
            ));
        }
        if tag == GlobalHelper::NumericMapPrepare
            && (self.worker.is_some() || self.has_uninventoried_private_owners())
        {
            return Ok(Some(0));
        }
        if tag == GlobalHelper::NumericMapPrepare
            && self.state.collections.as_ref().is_none_or(|collections| {
                !collections.fresh_numeric_header(
                    region.collection,
                    region.iteration,
                    self.progress,
                )
            })
        {
            return Ok(Some(0));
        }
        let replay_credit = if tag == GlobalHelper::NumericMapPrepare {
            let Some(credit) = self
                .state
                .collections
                .as_ref()
                .unwrap()
                .numeric_replay_credit(self.descriptor, region, self.retirements)?
            else {
                return Ok(Some(0));
            };
            let remaining = self
                .state
                .control
                .work
                .limit()
                .saturating_sub(self.state.control.work.used());
            if remaining < credit
                || self
                    .state
                    .library
                    .unscoped_remaining()
                    .is_none_or(|(remaining, _)| remaining < credit)
            {
                return Ok(Some(0));
            }
            Some(credit)
        } else {
            None
        };
        let prior = if tag == GlobalHelper::NumericMapPrepare {
            {
                let Some(geometry) = self.retained_caller_geometry(site)? else {
                    return Ok(Some(0));
                };
                Some(mech_core::ResourceDemand {
                    turn_peak_bytes: geometry.live_bytes,
                    retained_nodes: geometry.live_nodes,
                    ..Default::default()
                })
            }
        } else {
            None
        };
        let mut collections = self
            .state
            .collections
            .take()
            .ok_or_else(|| invalid("native numeric map has no collection owner"))?;
        let result = (|| {
            let mut ctx = CollectionExecution::new(self);
            match tag {
                GlobalHelper::NumericMapPrepare => {
                    if args != [0; 3] {
                        return Err(invalid("native numeric map entry has payload"));
                    }
                    collections
                        .prepare_numeric_map(
                            &mut ctx,
                            region,
                            prior.unwrap(),
                            replay_credit.unwrap(),
                        )
                        .map(|pointer| Some(pointer.unwrap_or(0)))
                }
                GlobalHelper::NumericMapFinish => {
                    if args[2] != 0 || args[0] == 0 || args[1] > 2 {
                        return Err(invalid("native numeric map finish has malformed status"));
                    }
                    collections
                        .finish_numeric_map(&mut ctx, site, args[0], args[1])
                        .map(|done| Some(u64::from(done)))
                }
                _ => Err(invalid("unknown native numeric map helper")),
            }
        })();
        self.state.collections = Some(collections);
        let result = result?;
        if tag == GlobalHelper::NumericMapFinish && result == Some(1) {
            self.progress[region.iteration as usize * 2] = 0;
        }
        Ok(result)
    }
}
