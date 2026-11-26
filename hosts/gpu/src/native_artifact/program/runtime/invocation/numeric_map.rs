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
    pub(super) fn finish_numeric_reduction(
        &mut self,
        site: usize,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        let descriptor = self.descriptor;
        let region = descriptor
            .metadata()
            .numeric_maps
            .at_reduction_finish(site)
            .ok_or_else(|| invalid("numeric reduction outside its selected finish"))?;
        if args != [0; 3]
            || self.state.consumed
            || self.state.control.previous != Some(site as u32)
            || !matches!(descriptor.metadata().plan.blocks()[site].action(),mech_engine::native_artifact::NativeIterationAction::CollectionFinish{collection} if *collection==region.collection)
        {
            return Err(invalid("numeric reduction finish lost original admission"));
        }
        let mut state = self
            .state
            .collections
            .take()
            .ok_or_else(|| invalid("numeric reduction has no collection state"))?;
        let result = state.finish_numeric_reduction(&mut CollectionExecution::new(self), site);
        self.state.collections = Some(state);
        let Some((word, header)) = result? else {
            return Ok(Some(0));
        };
        if header != region.header as usize {
            return Err(invalid("numeric reduction source header differs"));
        }
        self.state.consumed = true;
        let proof = region.reduction.as_ref().unwrap();
        // These are the exact original pure continuation visits, with normal
        // admission, retirement and required-action accounting. The private
        // proof supplies the already computed operation result; it grants no
        // generic runtime arithmetic or arbitrary source skipping.
        for &next in &proof.continuation {
            self.visit(u64::from(next))?;
            self.state.consumed = true;
        }
        self.write_borrowed_word(0, word)?;
        Ok(Some(1))
    }
    pub(super) fn finish_numeric_transpose(
        &mut self,
        site: usize,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        let descriptor = self.descriptor;
        let region = descriptor
            .metadata()
            .numeric_maps
            .at_transpose_finish(site)
            .ok_or_else(|| invalid("numeric transpose outside selected finish"))?;
        let projection = region.transpose.as_ref().unwrap();
        if args != [0; 3]
            || self.state.consumed
            || self.state.control.previous != Some(site as u32)
            || !matches!(descriptor.metadata().plan.blocks()[site].action(),mech_engine::native_artifact::NativeIterationAction::CollectionFinish{collection} if *collection==region.collection)
        {
            return Err(invalid("numeric transpose lost original finish admission"));
        }
        if self.closed_tail.is_some()
            || self.window_region.is_some()
            || self.state.scalar_path.is_some()
            || self.worker.is_some()
            || self.has_uninventoried_private_owners()
        {
            return Ok(Some(0));
        }
        let Some(geometry) = self.retained_caller_geometry(site)? else {
            return Ok(Some(0));
        };
        let prior = mech_core::ResourceDemand {
            turn_peak_bytes: geometry.live_bytes,
            retained_nodes: geometry.live_nodes,
            ..Default::default()
        };
        let mut collections = self
            .state
            .collections
            .take()
            .ok_or_else(|| invalid("numeric transpose lacks builder state"))?;
        let result = collections.finish_numeric_transpose(
            &mut CollectionExecution::new(self),
            region,
            prior,
        );
        self.state.collections = Some(collections);
        let Some(value) = result? else {
            return Ok(Some(0));
        };
        self.state.consumed = true;
        // The tracked result coexists with every exact original source visit.
        // Publish only AFTER the original transpose admission/retirement, so
        // a repeated loop visit cannot clear a prematurely installed owner.
        for &next in &projection.continuation {
            self.visit(u64::from(next))?;
            self.state.consumed = true;
        }
        self.state
            .arena
            .put(projection.output, &value, &mut self.state.control.work)?;
        self.write_borrowed_word(0, 0)?;
        Ok(Some(1))
    }
}
