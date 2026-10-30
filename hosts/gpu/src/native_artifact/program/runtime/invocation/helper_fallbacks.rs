//! Canonical materialization stays outside the frequently selected helper path.
use super::*;

impl Invocation<'_, '_, '_> {
    #[inline(never)]
    pub(super) fn canonical_structural_observation(
        &mut self,
        site: usize,
    ) -> Result<Option<u64>, Error> {
        let descriptor = self.descriptor;
        let NativeIterationAction::Structural {
            source,
            path,
            observation,
            ..
        } = descriptor.metadata().plan.blocks()[site].action()
        else {
            return Err(invalid(
                "structural helper outside its exact canonical site",
            ));
        };
        use mech_engine::native_artifact::{
            NativeStructuralObservation as Observation, NativeStructuralStep as Step,
        };
        let work = (path.len() as u64)
            .checked_mul(32)
            .and_then(|n| n.checked_add(64))
            .ok_or_else(|| invalid("structural observation work overflow"))?;
        self.state.control.work.charge(
            NativeExecutionPhase::Node(descriptor.metadata().plan.blocks()[site].origin().node()),
            work,
        )?;
        let root = self
            .state
            .arena
            .copy_current(source.get(), &mut self.state.control.work)?;
        if *observation == Observation::CanonicalValue {
            let NativeIterationAction::Structural { output, .. } =
                descriptor.metadata().plan.blocks()[site].action()
            else {
                unreachable!()
            };
            let prior = if self
                .state
                .arena
                .current_retained_bytes(output.get(), &mut self.state.control.work)?
                .is_some()
            {
                Some(
                    self.state
                        .arena
                        .copy_current(output.get(), &mut self.state.control.work)?,
                )
            } else {
                None
            };
            let result = mech_engine::native_artifact::native_project_structural_value(
                &descriptor.metadata().plan,
                site,
                &descriptor.metadata().schemas,
                &root,
                prior.as_ref(),
                self.limits,
                &self.state.library,
            )
            .map_err(|error| NativeProgramExecutionError::Library {
                origin: descriptor.metadata().plan.blocks()[site].origin(),
                error,
            })?;
            self.state
                .arena
                .put(output.get(), &result, &mut self.state.control.work)?;
            self.state.consumed = true;
            return descriptor.metadata().plan.values()[output.get() as usize]
                .scalar_kind()
                .map(|kind| bits(&result, kind))
                .transpose();
        }
        let mut data = root.data();
        for step in path {
            data = match (step, data) {
                (Step::EnumPayload { ordinal }, ValueData::Enum(value))
                    if value.ordinal() == *ordinal =>
                {
                    value
                        .payload()
                        .ok_or_else(|| invalid("structural enum has no payload"))?
                }
                (Step::TupleItem { ordinal }, ValueData::Tuple(values)) => values
                    .get(*ordinal as usize)
                    .ok_or_else(|| invalid("structural tuple index"))?,
                _ => {
                    return Err(invalid(
                        "structural observation path differs from its admitted schema",
                    ));
                }
            };
        }
        let word = match (observation, data) {
            (Observation::U64, ValueData::U64(value)) => *value,
            (Observation::EnumOrdinal { ordinal }, ValueData::Enum(value)) => {
                u64::from(value.ordinal() == *ordinal)
            }
            (Observation::StringEqual { peer }, ValueData::String(left)) => {
                let peer = self
                    .state
                    .arena
                    .copy_current(peer.get(), &mut self.state.control.work)?;
                let ValueData::String(right) = peer.data() else {
                    return Err(invalid("structural String peer has another kind"));
                };
                let work = u64::try_from(left.len().max(right.len()))
                    .map_err(|_| invalid("structural String comparison work overflow"))?;
                // Both finite cumulative compute and comparison accounts
                // admit the complete worst-case raw-byte equality first.
                self.state.library.charge(work, work).map_err(|error| {
                    NativeProgramExecutionError::Collection {
                        origin: descriptor.metadata().plan.blocks()[site].origin(),
                        error,
                    }
                })?;
                u64::from(left == right)
            }
            _ => {
                return Err(invalid(
                    "structural result differs from its exact native kind",
                ));
            }
        };
        self.state.consumed = true;
        Ok(Some(word))
    }

    #[inline(never)]
    pub(super) fn canonical_library_call(
        &mut self,
        site: usize,
        operands: &[NativeMatchValue],
        output: NativeMatchValue,
        words: &[u64],
    ) -> Result<Option<u64>, Error> {
        let descriptor = self.descriptor;
        for (index, &operand) in operands.iter().enumerate() {
            let value = if descriptor.metadata().plan.values()[operand.get() as usize]
                .scalar_kind()
                .is_some()
            {
                self.scalar(operand, words[index])?
            } else {
                if words[index] != 0 {
                    return Err(invalid("managed operand has a native payload"));
                }
                self.state
                    .arena
                    .copy_current(operand.get(), &mut self.state.control.work)?
            };
            self.state.call_inputs.values_mut()[index] = Some(value);
        }
        let call = descriptor.metadata().calls[site]
            .as_ref()
            .ok_or_else(|| invalid("missing exact prepared selection"))?;
        let mut result = None;
        let outcome = self.state.call_caches.invoke(
            site,
            call,
            &self.state.call_inputs.values()[..operands.len()],
            &mut result,
            self.limits,
            &self.state.library,
        );
        // Every scalar/imported owner prepaid retirement before insertion.
        // Clear both successful and failed calls without retaining an old
        // call's owners while the next site is being constructed.
        self.state.call_inputs.values_mut()[..operands.len()].fill(None);
        outcome.map_err(|error| NativeProgramExecutionError::Library {
            origin: descriptor.metadata().plan.blocks()[site].origin(),
            error,
        })?;
        let result = result.ok_or_else(|| invalid("library omitted output"))?;
        self.state
            .arena
            .put(output.get(), &result, &mut self.state.control.work)?;
        self.state.consumed = true;
        descriptor.metadata().plan.values()[output.get() as usize]
            .scalar_kind()
            .map(|kind| bits(&result, kind))
            .transpose()
    }

    #[inline(never)]
    pub(super) fn collection_helper(
        &mut self,
        site: usize,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        let descriptor = self.descriptor;
        let mut state = self
            .state
            .collections
            .take()
            .ok_or_else(|| invalid("collection helper without state"))?;
        let uniform_iteration = match descriptor.metadata().plan.blocks()[site].terminator() {
            NativeIterationTerminator::Header { iteration, .. } => Some(iteration),
            _ => None,
        };
        let result = if uniform_iteration.is_some() {
            state
                .fill_uniform_words(&mut CollectionExecution::new(self), site, args[0], args[1])
                .map(|filled| Some(u64::from(filled)))
        } else {
            state.helper(
                &mut CollectionExecution::new(self),
                site,
                [args[0], args[1]],
            )
        };
        self.state.collections = Some(state);
        let result = result?;
        if let Some(iteration) = uniform_iteration
            && result == Some(1)
        {
            // The proved pure region constructed its complete result. The
            // next edge remains this same header's ordinary exit; do not
            // fabricate source visits for operations no longer executed.
            self.progress[iteration as usize * 2] = 0;
        }
        self.state.consumed = true;
        Ok(result)
    }

    #[inline(never)]
    pub(super) fn typed_output_helper(
        &mut self,
        site: usize,
        tag: GlobalHelper,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        let descriptor = self.descriptor;
        if tag != GlobalHelper::decode(OUTPUT_BASE + self.state.output_cursor as u64)
            || (self.action_required(site) && !self.state.consumed)
            || descriptor.metadata().plan.blocks()[site].terminator()
                != NativeIterationTerminator::Scalar(NativeMatchTerminator::Complete)
        {
            return Err(invalid("typed output outside exact completion order"));
        }
        if self.state.output_cursor >= descriptor.metadata().plan.transport_output_count() {
            return Err(invalid("extra typed output"));
        }
        let out = descriptor
            .metadata()
            .plan
            .transport_output_value(self.state.output_cursor);
        let ty = descriptor
            .metadata()
            .plan
            .transport_output_type(self.state.output_cursor);
        let value = if ty.scalar_kind().is_some() {
            self.scalar(out, args[0])?
        } else {
            self.state
                .arena
                .copy_current(out.get(), &mut self.state.control.work)?
        };
        let target = descriptor
            .metadata()
            .plan
            .values()
            .len()
            .checked_add(self.state.output_cursor)
            .and_then(|n| u32::try_from(n).ok())
            .ok_or_else(|| invalid("typed output slot overflow"))?;
        self.state
            .arena
            .put(target, &value, &mut self.state.control.work)?;
        self.state.output_cursor += 1;
        Ok(None)
    }
}
