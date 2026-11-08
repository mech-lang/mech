//! Synchronous invocation orchestration for this owner.
use super::*;
use crate::native_artifact::program::protocol::GlobalHelper;
impl Invocation<'_, '_, '_> {
    pub(in crate::native_artifact::program::runtime) fn matrix_map_key_call(
        &mut self,
        site: usize,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        if self.state.consumed
            || args != [self.arguments as u64, 2, 0]
            || self.argument_capacity < 2
        {
            return Err(invalid(
                "matrix key helper outside exact retained visit/lane",
            ));
        }
        let metadata = self.descriptor.metadata();
        self.state.control.work.charge(
            Phase::Setup,
            amount(metadata.matrix_map_keys.regions.len(), 64, 0)?,
        )?;
        let (_, proof) = metadata
            .matrix_map_keys
            .producer(site)
            .ok_or_else(|| invalid("matrix key helper outside producer proof"))?;
        if !self.state.map_owners.has_retained_binding(proof.owner) {
            return Err(invalid("matrix key helper has no active retained owner"));
        }
        // SAFETY: exact invocation-owned pointer and capacity checked above;
        // generated execution is suspended during this synchronous callback.
        let words = unsafe { core::slice::from_raw_parts(self.arguments, 2) };
        if words[0] != 0 {
            return Err(invalid("matrix key source transported as scalar"));
        }
        let index = words[1];
        let mut state = self.state.map_owners.state.take().unwrap();
        let result = state.project_matrix_key(&mut MapExecution::new(self), site, index);
        self.state.map_owners.state = Some(state);
        if result? {
            self.state.consumed = true;
            Ok(None)
        } else {
            // Optional table/headroom refusal preserves the ordinary selected
            // String producer at this SAME visit and on the SAME accounts.
            self.helper(site as u64, GlobalHelper::Library.raw(), args)
        }
    }

    pub(in crate::native_artifact::program::runtime) fn retained_constructor_check(
        &mut self,
        site: usize,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        if args[1..] != [0; 2] || self.state.consumed {
            return Err(invalid("malformed or replayed retained constructor check"));
        }
        let mut state = self
            .state
            .map_owners
            .state
            .take()
            .ok_or_else(|| invalid("retained constructor has no owner session"))?;
        let result = state.constructor_statistics(&mut MapExecution::new(self), site, args[0]);
        self.state.map_owners.state = Some(state);
        let (entries, key_bytes) = result?;
        let descriptor = self.descriptor;
        let call = descriptor.metadata().calls[site]
            .as_ref()
            .ok_or_else(|| invalid("retained constructor lost selected factory"))?;
        let coexistence = self.selected_join_coexistence(None)?;
        call.admit_map_owner_constructor(
            entries,
            key_bytes,
            self.limits,
            &self.state.library,
            coexistence,
        )
        .map_err(|error| Error::Library {
            origin: descriptor.metadata().plan.blocks()[site].origin(),
            error,
        })?;
        self.state.consumed = true;
        Ok(None)
    }

    /// Retire private views at the SAME Tail latch/exit boundary as ordinary
    /// immutable locals. A later iteration can never read an earlier key.
    pub(in crate::native_artifact::program::runtime) fn retire_keys(
        &mut self,
        iteration: u32,
    ) -> Result<(), Error> {
        let Some(state) = self.state.map_owners.state.as_mut() else {
            return Ok(());
        };
        let layout = state
            .descriptor
            .metadata()
            .map_owner_words
            .as_ref()
            .unwrap();
        self.state
            .control
            .work
            .charge(Phase::Setup, amount(layout.spans().len(), 64, 64)?)?;
        for (id, proof) in layout.spans().iter().enumerate() {
            if layout.regions()[proof.region()].iteration() == iteration {
                drop(state.spans[id].take());
            }
        }
        Ok(())
    }
    pub(in crate::native_artifact::program::runtime) fn retained_native_span(
        &self,
        site: usize,
    ) -> bool {
        self.descriptor
            .metadata()
            .map_owner_words
            .as_ref()
            .is_some_and(|layout| {
                layout.span_site(site).is_some_and(|id| {
                    self.state
                        .map_owners
                        .has_retained_binding(layout.spans()[id].region())
                })
            })
    }
    pub(in crate::native_artifact::program::runtime) fn begin_retained_map(
        &mut self,
        site: usize,
    ) -> Result<Option<InitialWords>, Error> {
        // Check authority before lazy allocation, including refused-entry replay.
        if !core::ptr::eq(self.state.map_owners.descriptor, self.descriptor)
            || self.state.control.previous != u32::try_from(site).ok()
            || self.state.control.completed
            || self.state.consumed
            || self.state.map_owners.refused_visit == Some(self.state.control.visits)
            || self
                .state
                .map_owners
                .descriptor
                .metadata()
                .map_owner_words
                .as_ref()
                .and_then(|layout| layout.initializer(site))
                .is_none()
        {
            return Err(invalid(
                "retained map session entry outside fresh initializer",
            ));
        }
        if self.state.map_owners.state.is_none() {
            let limit = self
                .limits
                .max_temporary_bytes
                .ok_or_else(|| invalid("retained map session requires finite metadata cap"))?;
            let memory = self
                .state
                .library
                .memory_budget()
                .with_limits(limit, self.limits.max_retained_nodes.unwrap_or(0));
            let Some(state) = State::try_new(
                self.state.map_owners.descriptor,
                &memory,
                &mut self.state.control.work,
                limit,
            )?
            else {
                self.state.map_owners.refused_visit = Some(self.state.control.visits);
                return Ok(None);
            };
            self.state.map_owners.state = Some(state);
        }
        let mut state = self.state.map_owners.state.take().unwrap();
        let result = state.try_begin_retained(&mut MapExecution::new(self), site);
        // Restore on both success and failure. No new ledger or permit is made.
        self.state.map_owners.state = Some(state);
        let result = result?;
        if result.is_none() {
            let metadata = self.descriptor.metadata();
            let owner = metadata
                .map_owner_words
                .as_ref()
                .unwrap()
                .initializer(site)
                .unwrap();
            for proof in &metadata.borrowed_map_keys.regions {
                if metadata
                    .matrix_map_keys
                    .producer(proof.cells[0])
                    .is_some_and(|(_, p)| p.owner == owner)
                {
                    self.materialize_borrowed_keys(proof.output.get())?;
                }
            }
        }
        Ok(result)
    }

    pub(in crate::native_artifact::program::runtime) fn call_retained_map(
        &mut self,
        site: usize,
        words: &[u64],
    ) -> Result<u64, Error> {
        let mut state = self
            .state
            .map_owners
            .state
            .take()
            .ok_or_else(|| invalid("retained map session call before admitted entry"))?;
        let result = state.call(&mut MapExecution::new(self), site, words);
        self.state.map_owners.state = Some(state);
        result
    }

    pub(in crate::native_artifact::program::runtime) fn finish_retained_map(
        &mut self,
        site: usize,
        token: u64,
        words: &[u64],
        dictionary: bool,
    ) -> Result<Publication, Error> {
        let mut state = self
            .state
            .map_owners
            .state
            .take()
            .ok_or_else(|| invalid("retained map session publication before admitted entry"))?;
        let result = if dictionary {
            state.publish_retained_choice(&mut MapExecution::new(self), site, token, words, true)
        } else {
            state.publish_retained(&mut MapExecution::new(self), site, token, words)
        };
        self.state.map_owners.state = Some(state);
        result
    }
}
impl Invocation<'_, '_, '_> {
    pub(in crate::native_artifact::program::runtime) fn retained_native_action(
        &self,
        site: usize,
    ) -> bool {
        self.descriptor
            .metadata()
            .map_owner_words
            .as_ref()
            .and_then(|layout| {
                crate::native_artifact::program::prepare::map_owner::action_region(layout, site)
            })
            .is_some_and(|index| self.state.map_owners.has_retained_binding(index))
    }
    pub(in crate::native_artifact::program::runtime) fn retained_native_exit(
        &self,
        site: usize,
    ) -> bool {
        self.descriptor
            .metadata()
            .map_owner_words
            .as_ref()
            .and_then(|layout| layout.exit(site))
            .is_some_and(|index| self.state.map_owners.has_retained_binding(index))
    }
    pub(in crate::native_artifact::program::runtime) fn has_retained_maps(&self) -> bool {
        self.state
            .map_owners
            .state
            .as_ref()
            .is_some_and(|state| state.active_tables != 0)
    }
    pub(in crate::native_artifact::program::runtime) fn retained_native_call(
        &self,
        site: usize,
    ) -> bool {
        self.descriptor
            .metadata()
            .map_owner_words
            .as_ref()
            .and_then(|layout| {
                crate::native_artifact::program::prepare::map_owner::call_region(layout, site)
            })
            .is_some_and(|index| self.state.map_owners.has_retained_binding(index))
    }
    pub(in crate::native_artifact::program::runtime) fn retained_map_helper(
        &mut self,
        site: usize,
        operation: u64,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        use crate::native_artifact::program::protocol::abi::{
            MAP_OWNER_BEGIN, MAP_OWNER_CALL, MAP_OWNER_END, MAP_OWNER_SPAN, MOVE,
        };
        if self.state.consumed {
            return Err(invalid("duplicate retained map helper"));
        }
        if operation == MAP_OWNER_BEGIN {
            if args != [0; 3] {
                return Err(invalid("malformed retained map entry"));
            }
            if let Some(words) = self.begin_retained_map(site)? {
                let layout = self.descriptor.metadata().map_owner_words.as_ref().unwrap();
                let index = layout.initializer(site).unwrap();
                let count = layout
                    .aggregates()
                    .slot(layout.regions()[index].tail().carrier())
                    .unwrap()
                    .1;
                if words.words().len() != count || count > self.argument_capacity {
                    return Err(invalid("retained map initializer word lane differs"));
                }
                self.state
                    .control
                    .work
                    .charge(Phase::Setup, amount(count, 16, 128)?)?;
                for (index, &word) in words.words().iter().enumerate() {
                    self.write_borrowed_word(index, word)?;
                }
                self.state.consumed = true;
                Ok(Some(1))
            } else {
                self.helper(site as u64, MOVE, [0; 3])?;
                Ok(Some(0))
            }
        } else if operation == MAP_OWNER_SPAN {
            if !self.retained_native_span(site) {
                return Err(invalid("map key projection lacks active owner"));
            }
            let mut state = self.state.map_owners.state.take().unwrap();
            let result = state.project_key(&mut MapExecution::new(self), site, args);
            self.state.map_owners.state = Some(state);
            result?;
            self.state.consumed = true;
            Ok(None)
        } else if operation == MAP_OWNER_CALL {
            let Action::LibraryCall { operands, .. } =
                self.descriptor.metadata().plan.blocks()[site].action()
            else {
                return Err(invalid("retained map helper outside library call"));
            };
            let count = operands.len();
            if args != [self.arguments as u64, count as u64, 0]
                || count > 3
                || count > self.argument_capacity
                || !self.retained_native_call(site)
            {
                return Err(invalid("retained map helper operand lane differs"));
            }
            let mut words = [0; 3];
            // SAFETY: exact admitted pointer and capacity checked above; native
            // execution is suspended and no borrowed lane escapes this copy.
            words[..count]
                .copy_from_slice(unsafe { core::slice::from_raw_parts(self.arguments, count) });
            let result = self.call_retained_map(site, &words[..count])?;
            self.state.consumed = true;
            Ok(Some(result))
        } else if operation == MAP_OWNER_END
            || operation
                == crate::native_artifact::program::protocol::GlobalHelper::MapDictionaryEnd.raw()
        {
            let Action::Scalar(mech_engine::native_artifact::NativeMatchAction::Copy {
                output,
                ..
            }) = self.descriptor.metadata().plan.blocks()[site].action()
            else {
                return Err(invalid("retained map exit lacks canonical destination"));
            };
            let output = *output;
            let layout = self
                .descriptor
                .metadata()
                .map_owner_words
                .as_ref()
                .ok_or_else(|| invalid("retained map exit layout missing"))?;
            let region = layout
                .exit(site)
                .and_then(|id| layout.regions().get(id))
                .ok_or_else(|| invalid("retained map exit proof missing"))?;
            let count = layout
                .aggregates()
                .slot(region.tail().returned())
                .ok_or_else(|| invalid("retained map result word slot missing"))?
                .1;
            let mut words = [0u64; 32];
            if count == 1 && args[1..] == [0; 2] {
                words[0] = args[0];
            } else {
                if count > words.len()
                    || count > self.argument_capacity
                    || args[1] != self.arguments as usize as u64
                    || args[2] != count as u64
                {
                    return Err(invalid("malformed retained map publication words"));
                }
                // SAFETY: the exact invocation-owned argument pointer/count
                // was checked; generated execution remains suspended.
                words[..count]
                    .copy_from_slice(unsafe { core::slice::from_raw_parts(self.arguments, count) });
            }
            let token = match self.finish_retained_map(
                site,
                args[0],
                &words[..count],
                operation
                    == crate::native_artifact::program::protocol::GlobalHelper::MapDictionaryEnd
                        .raw(),
            )? {
                Publication::Canonical(value) => {
                    self.state
                        .arena
                        .put(output.get(), &value, &mut self.state.control.work)?;
                    if let Some(owner) = layout.exit(site)
                        && self
                            .descriptor
                            .metadata()
                            .borrowed_record_maps
                            .regions
                            .iter()
                            .any(|r| r.owner == owner)
                    {
                        let source = self
                            .state
                            .arena
                            .current_handle(output.get(), &mut self.state.control.work)?;
                        self.state
                            .map_owners
                            .state
                            .as_mut()
                            .unwrap()
                            .borrowed_records
                            .published = Some(super::borrowed_records::Published {
                            owner,
                            source,
                            visit: self.state.control.visits,
                        });
                    }
                    0
                }
                Publication::Frozen(token) => token,
                Publication::Dictionary(map) => {
                    self.install_dictionary(site, map)?;
                    0
                }
            };
            self.state.consumed = true;
            Ok(Some(token))
        } else {
            Err(invalid("unknown retained map helper"))
        }
    }
}
