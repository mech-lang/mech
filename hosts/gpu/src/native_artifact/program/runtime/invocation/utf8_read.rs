//! Checked entry/finalization for a complete emitted immutable lookup region.
use super::*;
use crate::native_artifact::program::protocol::GlobalHelper;

impl Invocation<'_, '_, '_> {
    pub(super) fn utf8_read_helper(
        &mut self,
        site: usize,
        tag: GlobalHelper,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        let metadata = self.descriptor.metadata();
        let region = metadata
            .utf8_reads
            .regions
            .iter()
            .find(|region| region.header as usize == site)
            .ok_or_else(|| invalid("native String lookup outside a selected header"))?;
        if self.state.control.previous != Some(site as u32)
            || self.state.control.current != Some(region.iteration)
            || self.closed_tail.is_some()
            || self.window_region.is_some()
            || self.state.scalar_path.is_some()
        {
            return Err(invalid(
                "native String lookup outside original checked header visit",
            ));
        }
        let mut dictionary = if tag == GlobalHelper::Utf8ReadPrepare {
            self.take_dictionary(site)?
        } else {
            None
        };
        if tag == GlobalHelper::Utf8ReadPrepare
            && (self.worker.is_some() || self.has_uninventoried_private_owners())
        {
            if let Some(owner) = dictionary.take() {
                self.materialize_dictionary(site, owner)?;
            }
            return Ok(Some(0));
        }
        if tag == GlobalHelper::Utf8ReadPrepare
            && self.state.collections.as_ref().is_none_or(|collections| {
                !collections.fresh_utf8_header(region.collection, region.iteration, self.progress)
            })
        {
            if let Some(owner) = dictionary.take() {
                self.materialize_dictionary(site, owner)?;
            }
            return Ok(Some(0));
        }
        let prior = if tag == GlobalHelper::Utf8ReadPrepare {
            {
                let Some(geometry) = self.retained_caller_geometry(site)? else {
                    if let Some(owner) = dictionary.take() {
                        self.materialize_dictionary(site, owner)?;
                    }
                    return Ok(Some(0));
                };
                Some(mech_core::ResourceDemand {
                    turn_peak_bytes: geometry.live_bytes,
                    retained_nodes: geometry
                        .live_nodes
                        .checked_add(dictionary.as_ref().map_or(0, |d| d.map.retained_nodes()))
                        .ok_or_else(|| invalid("dictionary caller nodes overflow"))?,
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
            .ok_or_else(|| invalid("native String lookup has no collection owner"))?;
        let result = (|| {
            let mut ctx = CollectionExecution::new(self);
            match tag {
                GlobalHelper::Utf8ReadPrepare => {
                    if args[0] > 1 || args[1..] != [0; 2] {
                        return Err(invalid("native String lookup entry has payload"));
                    }
                    collections
                        .prepare_utf8_read(
                            &mut ctx,
                            region,
                            prior.unwrap(),
                            args[0] == 1,
                            &mut dictionary,
                        )
                        .map(|pointer| Some(pointer.unwrap_or(0)))
                }
                GlobalHelper::Utf8ReadFinish => {
                    if args[2] != 0 || args[0] == 0 || args[1] > 2 {
                        return Err(invalid("native String lookup finish has malformed status"));
                    }
                    collections
                        .finish_utf8_read(&mut ctx, site, args[0], args[1], &mut dictionary)
                        .map(|done| Some(u64::from(done)))
                }
                _ => Err(invalid("unknown native String lookup helper")),
            }
        })();
        self.state.collections = Some(collections);
        let result = result?;
        if let Some(owner) = dictionary.take() {
            if !matches!(result, Some(0)) {
                return Err(invalid("successful lookup failed to consume dictionary"));
            }
            self.materialize_dictionary(site, owner)?;
        }
        if tag == GlobalHelper::Utf8ReadFinish && result == Some(1) {
            self.progress[region.iteration as usize * 2] = 0;
        }
        Ok(result)
    }
}
