//! Transient canonical-root views, never independently owned Map Values.
use super::*;
use crate::native_artifact::NativeValueHandle;
use crate::native_artifact::program::protocol::GlobalHelper;
use std::mem::size_of;

#[cfg(test)]
thread_local! {
    static OBSERVATION: std::cell::Cell<[u64;4]> = const {std::cell::Cell::new([0;4])};
}
#[cfg(test)]
pub(in crate::native_artifact::program::runtime) fn take_observation() -> [u64; 4] {
    OBSERVATION.with(|c| c.replace([0; 4]))
}
#[cfg(test)]
fn observe(index: usize) {
    OBSERVATION.with(|c| {
        let mut v = c.get();
        v[index] += 1;
        c.set(v);
    })
}

#[derive(Clone, Copy)]
pub(super) struct Published {
    pub(super) owner: usize,
    pub(super) source: NativeValueHandle,
    pub(super) visit: u64,
}
#[derive(Clone, Copy)]
struct Pending {
    region: usize,
    source: Option<NativeValueHandle>,
    visit: u64,
}
#[derive(Default)]
pub(super) struct Lanes {
    pub(super) published: Option<Published>,
    pending: Option<Pending>,
}
// The original retained-owner header already admitted this fixed allowance.
// Neither witness pins a Value, weak control block, payload or allocation.
const _: () = assert!(size_of::<Lanes>() <= 128);

impl Invocation<'_, '_, '_> {
    pub(in crate::native_artifact::program::runtime) fn borrowed_record_map_pending(
        &self,
        site: usize,
    ) -> bool {
        self.state
            .map_owners
            .state
            .as_ref()
            .and_then(|state| state.borrowed_records.pending)
            .is_some_and(|pending| {
                pending.source.is_some()
                    && self
                        .descriptor
                        .metadata()
                        .borrowed_record_maps
                        .regions
                        .get(pending.region)
                        .is_some_and(|r| r.read == site)
            })
    }
    #[cfg(test)]
    pub(in crate::native_artifact::program::runtime) fn test_forget_borrowed_record_map_publication(
        &mut self,
    ) {
        if let Some(state) = self.state.map_owners.state.as_mut() {
            state.borrowed_records.published = None;
        }
    }
    pub(in crate::native_artifact::program::runtime) fn borrowed_record_map_helper(
        &mut self,
        site: usize,
        tag: GlobalHelper,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        if self.state.consumed
            || args != [self.arguments as u64, 2, 0]
            || self.argument_capacity < 2
        {
            return Err(invalid("malformed or replayed borrowed Record Map call"));
        }
        // SAFETY: the invocation-owned pointer/count was checked above; the
        // generated caller is suspended and no borrowed lane escapes.
        if unsafe { core::slice::from_raw_parts(self.arguments, 2) } != [0, 0] {
            return Err(invalid("borrowed Record Map requires managed operands"));
        }
        let descriptor = self.descriptor;
        let layout = &descriptor.metadata().borrowed_record_maps;
        let index = match tag {
            GlobalHelper::BorrowedRecordMapProject => layout.projects.get(site),
            GlobalHelper::BorrowedRecordMapRead => layout.reads.get(site),
            _ => return Err(invalid("unknown borrowed Record Map operation")),
        }
        .copied()
        .flatten()
        .ok_or_else(|| invalid("borrowed Record Map lacks exact proof"))?;
        let proof = &layout.regions[index];
        if self.state.control.current != proof.iteration {
            return Err(invalid("borrowed Record Map outside lexical visit"));
        }
        let Some(mut state) = self.state.map_owners.state.take() else {
            // No retained publication exists. Both source calls keep their
            // ordinary selected kernels and their original ordered errors.
            let result = self.helper(site as u64, GlobalHelper::Library.raw(), args)?;
            #[cfg(test)]
            observe(if tag == GlobalHelper::BorrowedRecordMapProject {
                1
            } else {
                3
            });
            return Ok(if tag == GlobalHelper::BorrowedRecordMapProject {
                Some(0)
            } else {
                result
            });
        };
        let decision: Result<Option<NativeValueHandle>, Error> = (|| {
            state.guard(&MapExecution::new(self), site)?;
            self.state.control.work.charge(Phase::Setup, 256)?;
            if tag == GlobalHelper::BorrowedRecordMapProject {
                state.borrowed_records.pending = None;
                let Some(published) = state.borrowed_records.published else {
                    return Ok(None);
                };
                if published.owner != proof.owner
                    || self.state.control.visits <= published.visit
                    || state.active_tables != 0
                    || state.slots.iter().any(|s| s.frozen.is_some())
                    || state.spans.iter().any(Option::is_some)
                {
                    return Ok(None);
                }
                let Some(geometry) = state.geometry(&mut MapExecution::new(self), site)? else {
                    return Ok(None);
                };
                // No owner or payload is retained by this view. Admit the
                // complete current caller and the following scalar before
                // choosing it; a tight optional scope keeps both kernels.
                let output_bytes=match descriptor.metadata().plan.blocks()[proof.read].action() {
                    Action::LibraryCall {template,..} if template.primitive()==mech_engine::native_artifact::NativeCollectionPrimitive::MapContainsKey=>1,
                    _=>8,
                };
                if geometry
                    .live_bytes
                    .checked_add(output_bytes)
                    .zip(self.limits.max_temporary_bytes)
                    .is_none_or(|(need, cap)| need > cap)
                    || geometry
                        .live_nodes
                        .checked_add(1)
                        .zip(self.limits.max_retained_nodes)
                        .is_none_or(|(need, cap)| need > cap)
                {
                    return Ok(None);
                }
                let handle = self
                    .state
                    .arena
                    .current_handle(proof.source, &mut self.state.control.work)?;
                Ok((handle == published.source).then_some(handle))
            } else {
                let pending = state
                    .borrowed_records
                    .pending
                    .take()
                    .ok_or_else(|| invalid("borrowed Record Map read omitted projection"))?;
                if pending.region != index
                    || pending.visit.checked_add(1) != Some(self.state.control.visits)
                {
                    return Err(invalid("borrowed Record Map view escaped adjacent calls"));
                }
                if let Some(expected) = pending.source {
                    let actual = self
                        .state
                        .arena
                        .current_handle(proof.source, &mut self.state.control.work)?;
                    if expected != actual {
                        return Err(invalid("stale or foreign borrowed Record Map root"));
                    }
                }
                Ok(pending.source)
            }
        })();
        // Complete coexistence must see the SAME owner's admitted metadata.
        self.state.map_owners.state = Some(state);
        let source_handle = decision?;
        if source_handle.is_none() {
            let result = self.helper(site as u64, GlobalHelper::Library.raw(), args)?;
            #[cfg(test)]
            observe(if tag == GlobalHelper::BorrowedRecordMapProject {
                1
            } else {
                3
            });
            if tag == GlobalHelper::BorrowedRecordMapProject {
                self.state
                    .map_owners
                    .state
                    .as_mut()
                    .unwrap()
                    .borrowed_records
                    .pending = Some(Pending {
                    region: index,
                    source: None,
                    visit: self.state.control.visits,
                });
                return Ok(Some(0));
            }
            return Ok(result);
        }
        let call = descriptor.metadata().calls[site]
            .as_ref()
            .ok_or_else(|| invalid("borrowed Record Map lost selected factory"))?;
        let mut coexistence = self.selected_join_coexistence(None)?;
        let state = self.state.map_owners.state.as_ref().unwrap();
        coexistence.turn_peak_bytes = coexistence
            .turn_peak_bytes
            .checked_add(state.reservation.capacity_bytes())
            .ok_or_else(|| invalid("borrowed Record Map metadata overflows"))?;
        coexistence.retained_nodes = coexistence
            .retained_nodes
            .checked_add(state._nodes.nodes())
            .ok_or_else(|| invalid("borrowed Record Map nodes overflow"))?;
        let source = self
            .state
            .arena
            .copy_current(proof.source, &mut self.state.control.work)?;
        let Action::LibraryCall { operands, .. } =
            descriptor.metadata().plan.blocks()[site].action()
        else {
            return Err(invalid("borrowed Record Map outside call"));
        };
        let selector = self
            .state
            .arena
            .copy_current(operands[1].get(), &mut self.state.control.work)?;
        let result = if tag == GlobalHelper::BorrowedRecordMapProject {
            call.admit_borrowed_record_map(
                &source,
                &selector,
                proof.ordinal,
                self.limits,
                &self.state.library,
                coexistence,
            )
            .map(|()| Some(1))
        } else {
            call.read_borrowed_record_map(
                &source,
                descriptor.metadata().types[proof.source as usize].schema,
                proof.ordinal,
                &selector,
                self.limits,
                &self.state.library,
                coexistence,
            )
            .map(Some)
        }
        .map_err(|error| Error::Library {
            origin: descriptor.metadata().plan.blocks()[site].origin(),
            error,
        })?;
        let state = self.state.map_owners.state.as_mut().unwrap();
        if tag == GlobalHelper::BorrowedRecordMapProject {
            state.borrowed_records.pending = Some(Pending {
                region: index,
                source: source_handle,
                visit: self.state.control.visits,
            });
        }
        state.last_owned_visit = Some(self.state.control.visits);
        self.state.consumed = true;
        #[cfg(test)]
        observe(if tag == GlobalHelper::BorrowedRecordMapProject {
            0
        } else {
            2
        });
        Ok(result)
    }
}
