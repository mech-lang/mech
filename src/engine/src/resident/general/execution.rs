//! Allocation-free candidate execution for the schema-driven resident plan.

use core::ops::Range;
use core::sync::atomic::Ordering;

use mech_core::{
    ApplicationRequirementId, CellSlotId, ChangeDetectionPolicy, ExternalInteraction,
    InstanceEpoch, IntegrityConstraintId, MResult, MechError, MechErrorKind, NodeId,
    OutputConstruction, ProgramRevision, ReactiveInstanceId, ResidentKernelError,
    ResidentKernelInputs, ResidentValueKind, ResidentValueMut, ResidentValueRef, SlotIndex, Value,
};

use super::{
    ActivatedExternalNode, ActivatedNodeIndex, ActivatedTurnStep, F64_STATE_ARENA_BASE,
    F64_STATE_SLOT_BIT, F64ReadTapeEntry, ReactiveInstance, ResidentEffectIntent,
    ResidentExternalPublicationAuthority, ResidentIntegrityMode, ResidentReadLocation,
    ResidentRegion, ResidentStorageClass, StateArena, StateVersion, TypedResidentArena,
};

#[derive(Clone, Copy, Debug)]
pub struct CapturedSignalInput<'a> {
    pub slot: SlotIndex,
    pub value: ResidentValueRef<'a>,
}

#[derive(Clone, Copy, Debug)]
pub struct CapturedValueInput<'a> {
    pub slot: SlotIndex,
    pub value: &'a Value,
}

#[derive(Clone, Copy, Debug)]
pub struct ResidentEffectIntentView<'a> {
    pub artifact_node: NodeId,
    pub requirement: ApplicationRequirementId,
    pub ordinal: u32,
    pub interaction: &'a ExternalInteraction,
    pub payload: ResidentValueRef<'a>,
}

pub struct ResidentEffectIntentIter<'a> {
    instance: &'a ReactiveInstance,
    index: usize,
}

impl<'a> Iterator for ResidentEffectIntentIter<'a> {
    type Item = ResidentEffectIntentView<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let intent = *self.instance.workspace.effect_intents.get(self.index)?;
        self.index += 1;
        let external = self.instance.external_node(intent.ordinal)?;
        Some(ResidentEffectIntentView {
            artifact_node: intent.artifact_node,
            requirement: intent.requirement,
            ordinal: intent.ordinal,
            interaction: &external.interaction,
            payload: self
                .instance
                .workspace
                .effect_payloads
                .read(external.captured_payload),
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self
            .instance
            .workspace
            .effect_intents
            .len()
            .saturating_sub(self.index);
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for ResidentEffectIntentIter<'_> {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResidentTurnSummary {
    pub instance: ReactiveInstanceId,
    pub program_revision: ProgramRevision,
    pub before_epoch: InstanceEpoch,
    pub after_epoch: InstanceEpoch,
    pub state_hash: u64,
    pub touched_slots: u16,
    pub changed_slots: u16,
    pub dirty_nodes: u16,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ResidentStructuralProbe {
    pub candidate_seed_bytes: usize,
    pub candidate_materialized_bytes: usize,
    pub published_buffer_copy_bytes: usize,
    pub publication_store_count: usize,
    pub record_preparation_count: usize,
    pub record_append_count: usize,
    pub commit_runtime_call_count: usize,
    pub legacy_journal_capture_count: usize,
    pub runtime_execution_transaction_construction_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResidentExecutionError {
    ActiveCandidate,
    EpochExhausted,
    InputCount {
        expected: usize,
        actual: usize,
    },
    UnknownInput {
        slot: SlotIndex,
    },
    DuplicateInput {
        slot: SlotIndex,
    },
    InputLayout {
        slot: SlotIndex,
    },
    Kernel {
        node: NodeId,
        error: ResidentKernelError,
    },
    Integrity {
        constraint: IntegrityConstraintId,
    },
    InvalidWrite {
        node: NodeId,
    },
    InvalidOutputMaterialization {
        slot: CellSlotId,
    },
    ExternalSummaryRequired,
    ExternalPublicationUnauthorized,
    EffectIntentCapacity,
    CounterOverflow,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResidentEffectPayloadError {
    pub ordinal: u32,
}

impl MechErrorKind for ResidentEffectPayloadError {
    fn name(&self) -> &str {
        "ResidentEffectPayload"
    }

    fn message(&self) -> String {
        format!(
            "resident effect payload {} is unavailable outside a prepared candidate",
            self.ordinal
        )
    }
}

#[must_use = "prepared resident turns must be published or aborted"]
pub struct PreparedResidentTurn<'a> {
    instance: Option<&'a mut ReactiveInstance>,
    working_epoch: InstanceEpoch,
    summary: ResidentTurnSummary,
    probe: ResidentStructuralProbe,
}

impl PreparedResidentTurn<'_> {
    pub fn summary(&self) -> ResidentTurnSummary {
        self.summary
    }

    pub fn structural_probe(&self) -> ResidentStructuralProbe {
        self.probe
    }

    pub fn effect_intents(&self) -> ResidentEffectIntentIter<'_> {
        ResidentEffectIntentIter {
            instance: self
                .instance
                .as_deref()
                .expect("live prepared resident turn"),
            index: 0,
        }
    }

    pub fn materialize_effect_payload(&self, ordinal: u32) -> MResult<Value> {
        self.instance
            .as_deref()
            .expect("live prepared resident turn")
            .materialize_effect_payload(ordinal)
    }

    /// Publishes an ordinary pure resident turn.
    ///
    /// External plans fail closed: their state may be published only through
    /// [`Self::publish_external`] after the runtime coordinator has completed
    /// provider preparation and receipt/outbox preparation.
    #[inline]
    pub fn publish(mut self) -> Result<ResidentTurnSummary, ResidentExecutionError> {
        if self
            .instance
            .as_deref()
            .expect("live prepared resident turn")
            .plan
            .has_external_steps()
        {
            return Err(ResidentExecutionError::ExternalSummaryRequired);
        }
        Ok(self.publish_inner())
    }

    /// Publishes an externally coordinated resident turn after all
    /// prepublication obligations have succeeded. Safe code cannot implement
    /// the authority trait; the only in-repository implementation is private
    /// to the runtime coordinator.
    #[inline]
    #[doc(hidden)]
    pub fn publish_external<A>(
        mut self,
        _authority: &A,
    ) -> Result<ResidentTurnSummary, ResidentExecutionError>
    where
        A: ResidentExternalPublicationAuthority,
    {
        let instance = self
            .instance
            .as_deref()
            .expect("live prepared resident turn");
        if !instance.plan.has_external_steps() && !instance.plan.has_observation_inputs() {
            return Err(ResidentExecutionError::ExternalPublicationUnauthorized);
        }
        Ok(self.publish_inner())
    }

    fn publish_inner(&mut self) -> ResidentTurnSummary {
        let instance = self.instance.take().expect("live prepared resident turn");
        instance
            .published_epoch
            .store(self.working_epoch.get(), Ordering::Release);
        instance.candidate_active = false;
        instance.candidate_epoch = None;
        self.summary
    }

    pub fn abort(mut self) {
        let instance = self.instance.take().expect("live prepared resident turn");
        instance.abort_candidate(self.working_epoch);
    }
}

impl Drop for PreparedResidentTurn<'_> {
    fn drop(&mut self) {
        if let Some(instance) = self.instance.take() {
            instance.abort_candidate(self.working_epoch);
        }
    }
}

impl ReactiveInstance {
    /// Hash the currently published resident state without beginning a candidate.
    pub fn published_state_hash(&self) -> u64 {
        state_hash(self, self.published_epoch())
    }

    fn external_node(&self, ordinal: u32) -> Option<&ActivatedExternalNode> {
        self.plan.steps.iter().find_map(|step| match step {
            ActivatedTurnStep::External(node) if node.effect_ordinal == ordinal => Some(node),
            _ => None,
        })
    }

    pub fn materialize_effect_payload(&self, ordinal: u32) -> MResult<Value> {
        let _working_epoch = self
            .candidate_epoch
            .ok_or_else(|| MechError::new(ResidentEffectPayloadError { ordinal }, None))?;
        let external = self
            .external_node(ordinal)
            .ok_or_else(|| MechError::new(ResidentEffectPayloadError { ordinal }, None))?;
        let intent = self
            .workspace
            .effect_intents
            .iter()
            .find(|intent| intent.ordinal == ordinal)
            .ok_or_else(|| MechError::new(ResidentEffectPayloadError { ordinal }, None))?;
        let payload = self
            .workspace
            .effect_payloads
            .read(external.captured_payload);
        crate::resident_value_adapter::materialize_resident_value(
            &self.plan.schemas,
            external.payload_schema,
            &external.payload_shape,
            external.payload.region(),
            payload,
        )
    }

    pub fn prepare_turn(
        &mut self,
        inputs: &[CapturedSignalInput<'_>],
    ) -> Result<PreparedResidentTurn<'_>, ResidentExecutionError> {
        if self.candidate_active {
            return Err(ResidentExecutionError::ActiveCandidate);
        }
        let working_epoch = self
            .next_epoch
            .ok_or(ResidentExecutionError::EpochExhausted)?;
        self.next_epoch = working_epoch.checked_next().ok();
        let before_epoch = self.published_epoch();
        if let Err(error) = self.begin_workspace(inputs) {
            self.next_epoch = Some(working_epoch);
            return Err(error);
        }
        self.prepare_installed_turn(before_epoch, working_epoch)
    }

    pub fn prepare_turn_values(
        &mut self,
        inputs: &[CapturedValueInput<'_>],
    ) -> Result<PreparedResidentTurn<'_>, ResidentExecutionError> {
        if self.candidate_active {
            return Err(ResidentExecutionError::ActiveCandidate);
        }
        let working_epoch = self
            .next_epoch
            .ok_or(ResidentExecutionError::EpochExhausted)?;
        self.next_epoch = working_epoch.checked_next().ok();
        let before_epoch = self.published_epoch();
        if let Err(error) = self.begin_value_workspace(inputs) {
            self.next_epoch = Some(working_epoch);
            return Err(error);
        }
        self.prepare_installed_turn(before_epoch, working_epoch)
    }

    fn prepare_installed_turn(
        &mut self,
        before_epoch: InstanceEpoch,
        working_epoch: InstanceEpoch,
    ) -> Result<PreparedResidentTurn<'_>, ResidentExecutionError> {
        let mut probe = ResidentStructuralProbe {
            publication_store_count: 1,
            record_preparation_count: 1,
            record_append_count: 1,
            ..ResidentStructuralProbe::default()
        };
        if let Err(error) = self.execute_candidate(before_epoch, working_epoch, &mut probe) {
            self.abort_candidate(working_epoch);
            return Err(error);
        }
        self.finalize_changed_slots(before_epoch, working_epoch);
        let counts = (|| {
            Ok::<_, ResidentExecutionError>((
                u16::try_from(self.workspace.touched_slots.len())
                    .map_err(|_| ResidentExecutionError::CounterOverflow)?,
                u16::try_from(self.workspace.changed_slots.len())
                    .map_err(|_| ResidentExecutionError::CounterOverflow)?,
                u16::try_from(count_bits(&self.workspace.executed_bits))
                    .map_err(|_| ResidentExecutionError::CounterOverflow)?,
            ))
        })();
        let (touched_slots, changed_slots, dirty_nodes) = match counts {
            Ok(counts) => counts,
            Err(error) => {
                self.abort_candidate(working_epoch);
                return Err(error);
            }
        };
        let summary = ResidentTurnSummary {
            instance: self.id,
            program_revision: self.plan.program_revision,
            before_epoch,
            after_epoch: working_epoch,
            state_hash: state_hash(self, working_epoch),
            touched_slots,
            changed_slots,
            dirty_nodes,
        };
        self.candidate_active = true;
        self.candidate_epoch = Some(working_epoch);
        Ok(PreparedResidentTurn {
            instance: Some(self),
            working_epoch,
            summary,
            probe,
        })
    }

    #[inline]
    pub fn turn(
        &mut self,
        inputs: &[CapturedSignalInput<'_>],
    ) -> Result<ResidentTurnSummary, ResidentExecutionError> {
        self.prepare_turn(inputs)?.publish()
    }

    /// Execute and publish one already-admitted resident candidate without
    /// materializing the optional diagnostic summary. The benchmark-only
    /// kernel lane has no recorder or receipt consumer; complete recorded
    /// turns continue to use `prepare_turn`.
    #[doc(hidden)]
    pub fn turn_without_summary(
        &mut self,
        inputs: &[CapturedSignalInput<'_>],
    ) -> Result<(), ResidentExecutionError> {
        if self.plan.has_external_steps() {
            return Err(ResidentExecutionError::ExternalSummaryRequired);
        }
        if self.candidate_active {
            return Err(ResidentExecutionError::ActiveCandidate);
        }
        let working_epoch = self
            .next_epoch
            .ok_or(ResidentExecutionError::EpochExhausted)?;
        self.next_epoch = working_epoch.checked_next().ok();
        let before_epoch = self.published_epoch();
        if let Err(error) = self.begin_workspace_without_summary(inputs) {
            self.next_epoch = Some(working_epoch);
            return Err(error);
        }
        if let Err(error) = self.execute_candidate_without_summary(before_epoch, working_epoch) {
            self.abort_candidate(working_epoch);
            return Err(error);
        }
        self.published_epoch
            .store(working_epoch.get(), Ordering::Release);
        Ok(())
    }

    pub fn execute_then_abort(
        &mut self,
        inputs: &[CapturedSignalInput<'_>],
    ) -> Result<ResidentTurnSummary, ResidentExecutionError> {
        let prepared = self.prepare_turn(inputs)?;
        let summary = prepared.summary();
        prepared.abort();
        Ok(summary)
    }

    pub fn structural_probe(&self) -> ResidentStructuralProbe {
        let candidate_materialized_bytes = self
            .plan
            .slots
            .iter()
            .filter(|slot| slot.storage == ResidentStorageClass::State)
            .map(|slot| region_bytes(slot.region))
            .sum();
        let candidate_seed_bytes = self
            .plan
            .steps
            .iter()
            .enumerate()
            .filter_map(|(index, step)| match step {
                ActivatedTurnStep::Kernel(node) => Some((index, node)),
                ActivatedTurnStep::External(_) => None,
            })
            .filter(|(_, node)| {
                node.write.storage == ResidentStorageClass::State
                    && matches!(
                        node.construction,
                        OutputConstruction::ReadModifyWrite { .. }
                    )
            })
            .filter(|(index, node)| {
                !self.plan.steps[..*index].iter().any(|earlier| {
                    matches!(
                        earlier,
                        ActivatedTurnStep::Kernel(earlier)
                            if earlier.write.storage == ResidentStorageClass::State
                                && matches!(
                                    earlier.construction,
                                    OutputConstruction::ReadModifyWrite { .. }
                                )
                                && earlier.write.slot == node.write.slot
                    )
                })
            })
            .map(|(_, node)| region_bytes(node.write.region))
            .sum();
        ResidentStructuralProbe {
            candidate_seed_bytes,
            candidate_materialized_bytes,
            published_buffer_copy_bytes: 0,
            publication_store_count: 1,
            record_preparation_count: 1,
            record_append_count: 1,
            commit_runtime_call_count: 0,
            legacy_journal_capture_count: 0,
            runtime_execution_transaction_construction_count: 0,
        }
    }

    fn abort_candidate(&mut self, working_epoch: InstanceEpoch) {
        self.state.abort(working_epoch);
        self.workspace.initialized_output_bits.fill(0);
        self.workspace.all_outputs_initialized = false;
        self.workspace.effect_intents.clear();
        self.next_epoch = Some(working_epoch);
        self.candidate_active = false;
        self.candidate_epoch = None;
    }

    #[doc(hidden)]
    pub fn set_next_epoch_for_test(&mut self, next: u64) {
        assert_ne!(next, 0);
        self.next_epoch = Some(InstanceEpoch::new(next));
    }

    fn begin_workspace(
        &mut self,
        inputs: &[CapturedSignalInput<'_>],
    ) -> Result<(), ResidentExecutionError> {
        self.install_inputs(inputs)?;
        self.refresh_f64_state_arenas(self.published_epoch());
        self.begin_scheduler_workspace();
        Ok(())
    }

    fn begin_value_workspace(
        &mut self,
        inputs: &[CapturedValueInput<'_>],
    ) -> Result<(), ResidentExecutionError> {
        self.install_value_inputs(inputs)?;
        self.refresh_f64_state_arenas(self.published_epoch());
        self.begin_scheduler_workspace();
        Ok(())
    }

    fn begin_workspace_without_summary(
        &mut self,
        inputs: &[CapturedSignalInput<'_>],
    ) -> Result<(), ResidentExecutionError> {
        self.install_inputs(inputs)?;
        self.refresh_f64_state_arenas(self.published_epoch());
        self.workspace.effect_intents.clear();
        self.seed_dirty_bits();
        Ok(())
    }

    fn refresh_f64_state_arenas(&mut self, before_epoch: InstanceEpoch) {
        if self.plan.f64_read_tape.is_none() {
            return;
        }
        for version in &self.state.versions {
            self.workspace.state_f64_arena_by_slot[version.slot.get() as usize] =
                F64_STATE_ARENA_BASE + select_version(version, before_epoch) as u8;
        }
    }

    fn install_inputs(
        &mut self,
        inputs: &[CapturedSignalInput<'_>],
    ) -> Result<(), ResidentExecutionError> {
        if inputs.len() != self.plan.inputs.len() {
            return Err(ResidentExecutionError::InputCount {
                expected: self.plan.inputs.len(),
                actual: inputs.len(),
            });
        }
        if inputs
            .iter()
            .zip(&self.plan.inputs)
            .all(|(input, declared)| input.slot == declared.slot)
        {
            for (input, declared) in inputs.iter().zip(&self.plan.inputs) {
                copy_input(&mut self.workspace.input, declared.region, input.value)
                    .map_err(|_| ResidentExecutionError::InputLayout { slot: input.slot })?;
            }
        } else {
            for (ordinal, input) in inputs.iter().enumerate() {
                if !self
                    .plan
                    .inputs
                    .iter()
                    .any(|declared| declared.slot == input.slot)
                {
                    return Err(ResidentExecutionError::UnknownInput { slot: input.slot });
                }
                if inputs[..ordinal].iter().any(|seen| seen.slot == input.slot) {
                    return Err(ResidentExecutionError::DuplicateInput { slot: input.slot });
                }
            }
            for declared in &self.plan.inputs {
                let Some(input) = inputs.iter().find(|input| input.slot == declared.slot) else {
                    return Err(ResidentExecutionError::UnknownInput {
                        slot: declared.slot,
                    });
                };
                copy_input(&mut self.workspace.input, declared.region, input.value)
                    .map_err(|_| ResidentExecutionError::InputLayout { slot: input.slot })?;
            }
        }
        Ok(())
    }

    fn install_value_inputs(
        &mut self,
        inputs: &[CapturedValueInput<'_>],
    ) -> Result<(), ResidentExecutionError> {
        if inputs.len() != self.plan.inputs.len() {
            return Err(ResidentExecutionError::InputCount {
                expected: self.plan.inputs.len(),
                actual: inputs.len(),
            });
        }
        for (ordinal, input) in inputs.iter().enumerate() {
            let Some(declared) = self
                .plan
                .inputs
                .iter()
                .find(|declared| declared.slot == input.slot)
            else {
                return Err(ResidentExecutionError::UnknownInput { slot: input.slot });
            };
            if inputs[..ordinal].iter().any(|seen| seen.slot == input.slot) {
                return Err(ResidentExecutionError::DuplicateInput { slot: input.slot });
            }
            if input.value.schema() != declared.schema
                || input.value.schema_key() != declared.schema_key
                || input.value.shape() != &declared.shape
            {
                return Err(ResidentExecutionError::InputLayout { slot: input.slot });
            }
            crate::resident_value_adapter::write_value(
                &mut self.workspace.input,
                declared.region,
                input.value,
            )
            .map_err(|_| ResidentExecutionError::InputLayout { slot: input.slot })?;
        }
        Ok(())
    }

    fn begin_scheduler_workspace(&mut self) {
        self.workspace.executed_bits.fill(0);
        self.workspace.touched_slots.clear();
        self.workspace.changed_slots.clear();
        self.workspace.effect_intents.clear();
        self.seed_dirty_bits();
    }

    fn seed_dirty_bits(&mut self) {
        self.workspace.dirty_bits.fill(0);
        for (target, root) in self
            .workspace
            .dirty_bits
            .iter_mut()
            .zip(&self.plan.topology.turn_root_mask)
        {
            *target = *root;
        }
        if self.plan.integrity_mode == ResidentIntegrityMode::Checked {
            or_bits(
                &mut self.workspace.dirty_bits,
                &self.plan.topology.mandatory_candidate_mask,
            );
        }
    }

    fn execute_candidate_without_summary(
        &mut self,
        before_epoch: InstanceEpoch,
        working_epoch: InstanceEpoch,
    ) -> Result<(), ResidentExecutionError> {
        if self.plan.topology.word_len() != 1 {
            let mut probe = ResidentStructuralProbe::default();
            return self.execute_candidate(before_epoch, working_epoch, &mut probe);
        }
        let mut dirty = self.workspace.dirty_bits[0];
        for order in 0..self.plan.topology.single_word_schedule.len() {
            let entry = self.plan.topology.single_word_schedule[order];
            if dirty & entry.node_bit == 0 {
                continue;
            }
            // `turn_without_summary` rejects external plans before candidate
            // execution. Keep the established pure resident hot lane direct:
            // effectful plans continue through `execute_step`, while this
            // benchmark/recorder-free lane pays no external dispatcher tax.
            let changed = self.execute_kernel_without_summary(
                entry.node,
                before_epoch,
                working_epoch,
                entry.downstream != 0,
            )?;
            if changed {
                dirty |= entry.downstream;
            }
        }
        self.workspace.dirty_bits[0] = dirty;
        if !self.workspace.all_outputs_initialized
            && (dirty & self.plan.execution_node_mask[0]).count_ones() as usize
                == self.plan.execution_node_order.len()
        {
            self.workspace.all_outputs_initialized = true;
        }
        self.materialize_outputs(before_epoch, working_epoch, false)?;
        self.validate_constraints(working_epoch)
    }

    fn execute_candidate(
        &mut self,
        before_epoch: InstanceEpoch,
        working_epoch: InstanceEpoch,
        probe: &mut ResidentStructuralProbe,
    ) -> Result<(), ResidentExecutionError> {
        if !self.plan.has_external_steps() {
            return self.execute_pure_candidate(before_epoch, working_epoch, probe);
        }
        if self.plan.topology.word_len() == 1 {
            let mut dirty = self.workspace.dirty_bits[0];
            let mut executed = 0_u64;
            for order in 0..self.plan.topology.single_word_schedule.len() {
                let entry = self.plan.topology.single_word_schedule[order];
                if dirty & entry.node_bit == 0 {
                    continue;
                }
                let changed = self.execute_step(entry.node, before_epoch, working_epoch, probe)?;
                executed |= entry.node_bit;
                if changed {
                    dirty |= entry.downstream;
                }
            }
            self.workspace.dirty_bits[0] = dirty;
            self.workspace.executed_bits[0] = executed;
            if !self.workspace.all_outputs_initialized
                && executed.count_ones() as usize == self.plan.execution_node_order.len()
            {
                self.workspace.all_outputs_initialized = true;
            }
        } else {
            for order in 0..self.plan.execution_node_order.len() {
                let node_index = self.plan.execution_node_order[order];
                let index = node_index.get() as usize;
                if !bit_is_set(&self.workspace.dirty_bits, index) {
                    continue;
                }
                let changed = self.execute_step(node_index, before_epoch, working_epoch, probe)?;
                set_bit(&mut self.workspace.executed_bits, index);
                if changed {
                    or_bits(
                        &mut self.workspace.dirty_bits,
                        &self.plan.topology.same_turn_downstream_masks[index],
                    );
                }
            }
            if !self.workspace.all_outputs_initialized
                && count_bits(&self.workspace.executed_bits) == self.plan.execution_node_order.len()
            {
                self.workspace.all_outputs_initialized = true;
            }
        }
        self.materialize_outputs(before_epoch, working_epoch, true)?;
        self.validate_constraints(working_epoch)
    }

    /// Preserve the D2 recorded-turn lane for plans whose activation proves
    /// that every topological step is a resident kernel. Effectful plans use
    /// `execute_candidate` and its unified kernel/external dispatcher.
    fn execute_pure_candidate(
        &mut self,
        before_epoch: InstanceEpoch,
        working_epoch: InstanceEpoch,
        probe: &mut ResidentStructuralProbe,
    ) -> Result<(), ResidentExecutionError> {
        if self.plan.topology.word_len() == 1 {
            let mut dirty = self.workspace.dirty_bits[0];
            let mut executed = 0_u64;
            for order in 0..self.plan.topology.single_word_schedule.len() {
                let entry = self.plan.topology.single_word_schedule[order];
                if dirty & entry.node_bit == 0 {
                    continue;
                }
                let changed =
                    self.execute_kernel(entry.node, before_epoch, working_epoch, probe)?;
                executed |= entry.node_bit;
                if changed {
                    dirty |= entry.downstream;
                }
            }
            self.workspace.dirty_bits[0] = dirty;
            self.workspace.executed_bits[0] = executed;
            if !self.workspace.all_outputs_initialized
                && executed.count_ones() as usize == self.plan.execution_node_order.len()
            {
                self.workspace.all_outputs_initialized = true;
            }
        } else {
            for order in 0..self.plan.execution_node_order.len() {
                let node_index = self.plan.execution_node_order[order];
                let index = node_index.get() as usize;
                if !bit_is_set(&self.workspace.dirty_bits, index) {
                    continue;
                }
                let changed =
                    self.execute_kernel(node_index, before_epoch, working_epoch, probe)?;
                set_bit(&mut self.workspace.executed_bits, index);
                if changed {
                    or_bits(
                        &mut self.workspace.dirty_bits,
                        &self.plan.topology.same_turn_downstream_masks[index],
                    );
                }
            }
            if !self.workspace.all_outputs_initialized
                && count_bits(&self.workspace.executed_bits) == self.plan.execution_node_order.len()
            {
                self.workspace.all_outputs_initialized = true;
            }
        }
        self.materialize_outputs(before_epoch, working_epoch, true)?;
        self.validate_constraints(working_epoch)
    }

    fn materialize_outputs(
        &mut self,
        before_epoch: InstanceEpoch,
        working_epoch: InstanceEpoch,
        record_summary: bool,
    ) -> Result<(), ResidentExecutionError> {
        for index in 0..self.plan.output_materializations.len() {
            let materialization = self.plan.output_materializations[index];
            let target = materialization.target;
            let target_region = self.plan.slots[target.get() as usize].region;
            match materialization.source {
                ResidentReadLocation::Constant(source) => {
                    let candidate = self.state.candidate_buffer(target, before_epoch);
                    copy_input(
                        &mut self.state.buffers[candidate],
                        target_region,
                        self.activation.read(source),
                    )
                    .map_err(|_| {
                        ResidentExecutionError::InvalidOutputMaterialization { slot: target }
                    })?;
                    self.state.tag(target, candidate, working_epoch);
                }
                ResidentReadLocation::Input(source) => {
                    let candidate = self.state.candidate_buffer(target, before_epoch);
                    copy_input(
                        &mut self.state.buffers[candidate],
                        target_region,
                        self.workspace.input.read(source),
                    )
                    .map_err(|_| {
                        ResidentExecutionError::InvalidOutputMaterialization { slot: target }
                    })?;
                    self.state.tag(target, candidate, working_epoch);
                }
                ResidentReadLocation::Scratch(source) => {
                    let candidate = self.state.candidate_buffer(target, before_epoch);
                    copy_input(
                        &mut self.state.buffers[candidate],
                        target_region,
                        self.workspace.scratch.read(source),
                    )
                    .map_err(|_| {
                        ResidentExecutionError::InvalidOutputMaterialization { slot: target }
                    })?;
                    self.state.tag(target, candidate, working_epoch);
                }
                ResidentReadLocation::State {
                    slot: source_slot,
                    region: source_region,
                } => self.state.materialize_state_slot(
                    target,
                    target_region,
                    source_slot,
                    source_region,
                    before_epoch,
                    working_epoch,
                ),
            }
            if record_summary {
                let slot = SlotIndex::new(target.get());
                self.workspace.touched_slots.push(slot);
                let candidate = self.state.select_buffer(target, working_epoch);
                if !self.state.same_at(target, candidate, before_epoch) {
                    self.workspace.changed_slots.push(slot);
                }
            }
        }
        Ok(())
    }

    fn validate_constraints(
        &self,
        working_epoch: InstanceEpoch,
    ) -> Result<(), ResidentExecutionError> {
        if self.plan.integrity_mode == ResidentIntegrityMode::Unchecked {
            return Ok(());
        }
        for constraint in &self.plan.constraints {
            let Some(ResidentValueRef::Bool(predicate)) =
                self.read_location(constraint.predicate, working_epoch)
            else {
                return Err(ResidentExecutionError::Integrity {
                    constraint: constraint.artifact_id,
                });
            };
            if predicate != [1] {
                return Err(ResidentExecutionError::Integrity {
                    constraint: constraint.artifact_id,
                });
            }
        }
        Ok(())
    }

    #[inline(always)]
    fn execute_kernel_without_summary(
        &mut self,
        node_index: ActivatedNodeIndex,
        before_epoch: InstanceEpoch,
        working_epoch: InstanceEpoch,
        change_is_observed: bool,
    ) -> Result<bool, ResidentExecutionError> {
        let index = node_index.get() as usize;
        let node = if let Some(nodes) = &self.plan.pure_kernel_steps {
            &nodes[index]
        } else {
            let ActivatedTurnStep::Kernel(node) = &self.plan.steps[index] else {
                unreachable!("external steps are staged by the dispatcher")
            };
            node
        };
        if node.write.storage == ResidentStorageClass::Scratch {
            let mut ignored_probe = ResidentStructuralProbe::default();
            return self.execute_kernel(
                node_index,
                before_epoch,
                working_epoch,
                &mut ignored_probe,
            );
        }
        if node.write.storage != ResidentStorageClass::State {
            return Err(ResidentExecutionError::InvalidWrite {
                node: node.artifact_node,
            });
        }
        let slot = node.write.slot;
        let candidate = match node.construction {
            OutputConstruction::FullWrite { .. } => self.state.candidate_buffer(slot, before_epoch),
            OutputConstruction::ReadModifyWrite { .. } => {
                let candidate = self.state.begin_rmw(slot, before_epoch, working_epoch).0;
                if self.plan.f64_read_tape.is_some() {
                    self.workspace.state_f64_arena_by_slot[slot.get() as usize] =
                        F64_STATE_ARENA_BASE + candidate as u8;
                }
                candidate
            }
            _ => {
                return Err(ResidentExecutionError::InvalidWrite {
                    node: node.artifact_node,
                });
            }
        };
        let kernel_changed = if node.reads_state {
            let versions = &self.state.versions;
            let (state, output) = StateReadAccess::split_output(
                &mut self.state.buffers,
                versions,
                &self.state.version_by_slot,
                candidate,
                node.write.region,
            );
            let inputs = StateNodeInputs {
                locations: &self.plan.reads[node.reads.start as usize..node.reads.end as usize],
                activation: &self.activation,
                input: &self.workspace.input,
                state,
                scratch: &self.workspace.scratch,
                epoch: working_epoch,
            };
            node.kernel.execute(&inputs, output)
        } else {
            let output = self.state.buffers[candidate].write(node.write.region);
            if let Some(tape) = &self.plan.f64_read_tape {
                let inputs = F64NonStateNodeInputs {
                    locations: &tape[node.reads.start as usize..node.reads.end as usize],
                    arenas: [
                        &self.activation.f64s,
                        &self.workspace.input.f64s,
                        &self.workspace.scratch.f64s,
                    ],
                };
                node.kernel.execute(&inputs, output)
            } else {
                let inputs = StateNodeInputsWithoutState {
                    locations: &self.plan.reads[node.reads.start as usize..node.reads.end as usize],
                    activation: &self.activation,
                    input: &self.workspace.input,
                    scratch: &self.workspace.scratch,
                };
                node.kernel.execute(&inputs, output)
            }
        }
        .map_err(|error| ResidentExecutionError::Kernel {
            node: node.artifact_node,
            error,
        })?;
        if matches!(node.construction, OutputConstruction::FullWrite { .. }) {
            self.state.tag(slot, candidate, working_epoch);
            if self.plan.f64_read_tape.is_some() {
                self.workspace.state_f64_arena_by_slot[slot.get() as usize] =
                    F64_STATE_ARENA_BASE + candidate as u8;
            }
            return Ok(change_is_observed && !self.state.same_at(slot, candidate, before_epoch));
        }
        Ok(change_is_observed && kernel_changed)
    }

    #[inline(always)]
    fn execute_step(
        &mut self,
        node_index: ActivatedNodeIndex,
        before_epoch: InstanceEpoch,
        working_epoch: InstanceEpoch,
        probe: &mut ResidentStructuralProbe,
    ) -> Result<bool, ResidentExecutionError> {
        if matches!(
            self.plan.steps[node_index.get() as usize],
            ActivatedTurnStep::External(_)
        ) {
            self.stage_external(node_index, working_epoch)?;
            return Ok(false);
        }
        self.execute_kernel(node_index, before_epoch, working_epoch, probe)
    }

    fn stage_external(
        &mut self,
        node_index: ActivatedNodeIndex,
        working_epoch: InstanceEpoch,
    ) -> Result<(), ResidentExecutionError> {
        let ActivatedTurnStep::External(node) = &self.plan.steps[node_index.get() as usize] else {
            unreachable!("kernel steps are executed by the kernel dispatcher")
        };
        let artifact_node = node.artifact_node;
        let requirement = node.requirement;
        let ordinal = node.effect_ordinal;
        let source = node.payload;
        let captured = node.captured_payload;
        if self.workspace.effect_intents.len() == self.workspace.effect_intents.capacity() {
            return Err(ResidentExecutionError::EffectIntentCapacity);
        }
        self.capture_effect_payload(source, captured, working_epoch)
            .map_err(|_| ResidentExecutionError::InvalidWrite {
                node: artifact_node,
            })?;
        self.workspace.effect_intents.push(ResidentEffectIntent {
            artifact_node,
            requirement,
            ordinal,
        });
        Ok(())
    }

    fn capture_effect_payload(
        &mut self,
        source: ResidentReadLocation,
        captured: ResidentRegion,
        working_epoch: InstanceEpoch,
    ) -> Result<(), ()> {
        match source {
            ResidentReadLocation::Constant(region) => copy_input(
                &mut self.workspace.effect_payloads,
                captured,
                self.activation.read(region),
            ),
            ResidentReadLocation::Input(region) => copy_input(
                &mut self.workspace.effect_payloads,
                captured,
                self.workspace.input.read(region),
            ),
            ResidentReadLocation::State { slot, region } => {
                let buffer = self.state.select_buffer(slot, working_epoch);
                copy_input(
                    &mut self.workspace.effect_payloads,
                    captured,
                    self.state.buffers[buffer].read(region),
                )
            }
            ResidentReadLocation::Scratch(region) => copy_input(
                &mut self.workspace.effect_payloads,
                captured,
                self.workspace.scratch.read(region),
            ),
        }
    }

    #[inline(always)]
    fn execute_kernel(
        &mut self,
        node_index: ActivatedNodeIndex,
        before_epoch: InstanceEpoch,
        working_epoch: InstanceEpoch,
        probe: &mut ResidentStructuralProbe,
    ) -> Result<bool, ResidentExecutionError> {
        let index = node_index.get() as usize;
        let node = if let Some(nodes) = &self.plan.pure_kernel_steps {
            &nodes[index]
        } else {
            let ActivatedTurnStep::Kernel(node) = &self.plan.steps[index] else {
                unreachable!("external steps are staged by the dispatcher")
            };
            node
        };
        match node.write.storage {
            ResidentStorageClass::Scratch => {
                let before_scalar = if node.change_detection == ChangeDetectionPolicy::ExactScalar {
                    scalar_token(self.workspace.scratch.read(node.write.region))
                } else {
                    None
                };
                let kernel_changed = if node.scratch_prefix_reads
                    && node.kernel.has_direct_f64_output()
                {
                    let (scratch, output) = self
                        .workspace
                        .scratch
                        .split_f64_scratch_output(node.write.region);
                    if let Some(tape) = &self.plan.f64_read_tape {
                        let inputs = F64NodeInputs {
                            locations: &tape[node.reads.start as usize..node.reads.end as usize],
                            arenas: [
                                &self.activation.f64s,
                                &self.workspace.input.f64s,
                                scratch.f64s,
                                &self.state.buffers[0].f64s,
                                &self.state.buffers[1].f64s,
                            ],
                            state_arena_by_slot: &self.workspace.state_f64_arena_by_slot,
                        };
                        node.kernel.execute_f64_output(&inputs, output)
                    } else {
                        let inputs = ScratchNodeInputs {
                            locations: &self.plan.reads
                                [node.reads.start as usize..node.reads.end as usize],
                            activation: &self.activation,
                            input: &self.workspace.input,
                            state: &self.state,
                            scratch,
                            epoch: working_epoch,
                        };
                        node.kernel.execute_f64_output(&inputs, output)
                    }
                } else if node.scratch_prefix_reads {
                    let (scratch, output) = self
                        .workspace
                        .scratch
                        .split_scratch_output(node.write.region);
                    if let Some(tape) = &self.plan.f64_read_tape {
                        let inputs = F64NodeInputs {
                            locations: &tape[node.reads.start as usize..node.reads.end as usize],
                            arenas: [
                                &self.activation.f64s,
                                &self.workspace.input.f64s,
                                scratch.f64s,
                                &self.state.buffers[0].f64s,
                                &self.state.buffers[1].f64s,
                            ],
                            state_arena_by_slot: &self.workspace.state_f64_arena_by_slot,
                        };
                        node.kernel.execute(&inputs, output)
                    } else {
                        let inputs = ScratchNodeInputs {
                            locations: &self.plan.reads
                                [node.reads.start as usize..node.reads.end as usize],
                            activation: &self.activation,
                            input: &self.workspace.input,
                            state: &self.state,
                            scratch,
                            epoch: working_epoch,
                        };
                        node.kernel.execute(&inputs, output)
                    }
                } else {
                    let (scratch, output) = self.workspace.scratch.split_output(node.write.region);
                    let inputs = GeneralScratchNodeInputs {
                        locations: &self.plan.reads
                            [node.reads.start as usize..node.reads.end as usize],
                        activation: &self.activation,
                        input: &self.workspace.input,
                        state: &self.state,
                        scratch,
                        epoch: working_epoch,
                    };
                    node.kernel.execute(&inputs, output)
                }
                .map_err(|error| ResidentExecutionError::Kernel {
                    node: node.artifact_node,
                    error,
                })?;
                let policy_changed = match node.change_detection {
                    ChangeDetectionPolicy::KernelReported => kernel_changed,
                    ChangeDetectionPolicy::ExactScalar => {
                        before_scalar
                            != scalar_token(self.workspace.scratch.read(node.write.region))
                    }
                    ChangeDetectionPolicy::AlwaysChanged => true,
                    ChangeDetectionPolicy::SemanticHash => unreachable!(
                        "semantic-hash resident outputs are rejected during activation"
                    ),
                };
                if self.workspace.all_outputs_initialized {
                    Ok(policy_changed)
                } else {
                    let initialized = bit_is_set(&self.workspace.initialized_output_bits, index);
                    set_bit(&mut self.workspace.initialized_output_bits, index);
                    Ok(!initialized || policy_changed)
                }
            }
            ResidentStorageClass::State => {
                let slot = node.write.slot;
                let candidate = match node.construction {
                    OutputConstruction::FullWrite { .. } => {
                        self.state.candidate_buffer(slot, before_epoch)
                    }
                    OutputConstruction::ReadModifyWrite { .. } => {
                        let (candidate, seeded) =
                            self.state.begin_rmw(slot, before_epoch, working_epoch);
                        if self.plan.f64_read_tape.is_some() {
                            self.workspace.state_f64_arena_by_slot[slot.get() as usize] =
                                F64_STATE_ARENA_BASE + candidate as u8;
                        }
                        if seeded {
                            probe.candidate_seed_bytes += region_bytes(node.write.region);
                            probe.candidate_materialized_bytes += region_bytes(node.write.region);
                            self.workspace
                                .touched_slots
                                .push(SlotIndex::new(slot.get()));
                        }
                        candidate
                    }
                    _ => {
                        return Err(ResidentExecutionError::InvalidWrite {
                            node: node.artifact_node,
                        });
                    }
                };
                let changed = if node.reads_state {
                    let versions = &self.state.versions;
                    let (state, output) = StateReadAccess::split_output(
                        &mut self.state.buffers,
                        versions,
                        &self.state.version_by_slot,
                        candidate,
                        node.write.region,
                    );
                    let inputs = StateNodeInputs {
                        locations: &self.plan.reads
                            [node.reads.start as usize..node.reads.end as usize],
                        activation: &self.activation,
                        input: &self.workspace.input,
                        state,
                        scratch: &self.workspace.scratch,
                        epoch: working_epoch,
                    };
                    node.kernel.execute(&inputs, output).map_err(|error| {
                        ResidentExecutionError::Kernel {
                            node: node.artifact_node,
                            error,
                        }
                    })?
                } else {
                    let output = self.state.buffers[candidate].write(node.write.region);
                    if let Some(tape) = &self.plan.f64_read_tape {
                        let inputs = F64NonStateNodeInputs {
                            locations: &tape[node.reads.start as usize..node.reads.end as usize],
                            arenas: [
                                &self.activation.f64s,
                                &self.workspace.input.f64s,
                                &self.workspace.scratch.f64s,
                            ],
                        };
                        node.kernel.execute(&inputs, output)
                    } else {
                        let inputs = StateNodeInputsWithoutState {
                            locations: &self.plan.reads
                                [node.reads.start as usize..node.reads.end as usize],
                            activation: &self.activation,
                            input: &self.workspace.input,
                            scratch: &self.workspace.scratch,
                        };
                        node.kernel.execute(&inputs, output)
                    }
                    .map_err(|error| ResidentExecutionError::Kernel {
                        node: node.artifact_node,
                        error,
                    })?
                };
                let policy_changed =
                    if matches!(node.construction, OutputConstruction::FullWrite { .. }) {
                        self.state.tag(slot, candidate, working_epoch);
                        if self.plan.f64_read_tape.is_some() {
                            self.workspace.state_f64_arena_by_slot[slot.get() as usize] =
                                F64_STATE_ARENA_BASE + candidate as u8;
                        }
                        probe.candidate_materialized_bytes += region_bytes(node.write.region);
                        self.workspace
                            .touched_slots
                            .push(SlotIndex::new(slot.get()));
                        let changed = !self.state.same_at(slot, candidate, before_epoch);
                        if changed {
                            self.workspace
                                .changed_slots
                                .push(SlotIndex::new(slot.get()));
                        }
                        changed
                    } else {
                        changed
                    };
                if self.workspace.all_outputs_initialized {
                    Ok(policy_changed)
                } else {
                    let initialized = bit_is_set(&self.workspace.initialized_output_bits, index);
                    set_bit(&mut self.workspace.initialized_output_bits, index);
                    Ok(!initialized || policy_changed)
                }
            }
            _ => Err(ResidentExecutionError::InvalidWrite {
                node: node.artifact_node,
            }),
        }
    }

    fn read_location(
        &self,
        location: ResidentReadLocation,
        epoch: InstanceEpoch,
    ) -> Option<ResidentValueRef<'_>> {
        match location {
            ResidentReadLocation::Constant(region) => Some(self.activation.read(region)),
            ResidentReadLocation::Input(region) => Some(self.workspace.input.read(region)),
            ResidentReadLocation::State { slot, region } => {
                let buffer = self.state.select_buffer(slot, epoch);
                Some(self.state.buffers[buffer].read(region))
            }
            ResidentReadLocation::Scratch(region) => Some(self.workspace.scratch.read(region)),
        }
    }

    fn finalize_changed_slots(&mut self, before: InstanceEpoch, working: InstanceEpoch) {
        for artifact in self.plan.rmw_state_slots.iter().copied() {
            let slot = SlotIndex::new(artifact.get());
            if self.workspace.touched_slots.contains(&slot) {
                let candidate = self.state.select_buffer(artifact, working);
                if !self.state.same_at(artifact, candidate, before) {
                    self.workspace.changed_slots.push(slot);
                }
            }
        }
    }
}

impl StateArena {
    fn read(
        &self,
        slot: CellSlotId,
        region: ResidentRegion,
        epoch: InstanceEpoch,
    ) -> Option<ResidentValueRef<'_>> {
        let version = self.version(slot);
        Some(self.buffers[select_version(version, epoch)].read(region))
    }

    fn read_f64(
        &self,
        slot: CellSlotId,
        region: ResidentRegion,
        epoch: InstanceEpoch,
    ) -> Option<&[f64]> {
        let version = self.version(slot);
        self.buffers[select_version(version, epoch)].read_f64(region)
    }

    fn select_buffer(&self, slot: CellSlotId, epoch: InstanceEpoch) -> usize {
        select_version(self.version(slot), epoch)
    }

    fn candidate_buffer(&self, slot: CellSlotId, before: InstanceEpoch) -> usize {
        1 - self.select_buffer(slot, before)
    }

    fn begin_rmw(
        &mut self,
        slot: CellSlotId,
        before: InstanceEpoch,
        working: InstanceEpoch,
    ) -> (usize, bool) {
        if let Some(candidate) = self
            .version(slot)
            .epochs
            .iter()
            .position(|tag| *tag == Some(working))
        {
            return (candidate, false);
        }
        let published = self.select_buffer(slot, before);
        let candidate = 1 - published;
        let region = self.version(slot).region;
        let [left, right] = &mut self.buffers;
        if candidate == 0 {
            left.copy_region_from(region, right, region);
        } else {
            right.copy_region_from(region, left, region);
        }
        self.version_mut(slot).epochs[candidate] = Some(working);
        (candidate, true)
    }

    fn materialize_state_slot(
        &mut self,
        target_slot: CellSlotId,
        target_region: ResidentRegion,
        source_slot: CellSlotId,
        source_region: ResidentRegion,
        before: InstanceEpoch,
        working: InstanceEpoch,
    ) {
        let source_buffer = self.select_buffer(source_slot, working);
        let target_buffer = self.candidate_buffer(target_slot, before);
        if source_buffer == target_buffer {
            self.buffers[target_buffer].copy_region_within(target_region, source_region);
        } else if target_buffer == 0 {
            let [target, source] = &mut self.buffers;
            target.copy_region_from(target_region, source, source_region);
        } else {
            let [source, target] = &mut self.buffers;
            target.copy_region_from(target_region, source, source_region);
        }
        self.tag(target_slot, target_buffer, working);
    }

    fn tag(&mut self, slot: CellSlotId, buffer: usize, epoch: InstanceEpoch) {
        self.version_mut(slot).epochs[buffer] = Some(epoch);
    }

    fn abort(&mut self, working: InstanceEpoch) {
        for version in &mut self.versions {
            for tag in &mut version.epochs {
                if *tag == Some(working) {
                    *tag = None;
                }
            }
        }
    }

    fn same_at(&self, slot: CellSlotId, candidate: usize, before: InstanceEpoch) -> bool {
        let version = self.version(slot);
        let published = self.select_buffer(slot, before);
        regions_equal(
            &self.buffers[candidate],
            version.region,
            &self.buffers[published],
            version.region,
        )
    }
}

enum SliceRead<'a, T> {
    Whole(&'a [T]),
    Split {
        before: &'a [T],
        after: &'a [T],
        gap_start: usize,
        gap_end: usize,
    },
}

impl<T> Copy for SliceRead<'_, T> {}

impl<T> Clone for SliceRead<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T> SliceRead<'a, T> {
    fn get(self, range: Range<usize>) -> Option<&'a [T]> {
        match self {
            Self::Whole(values) => values.get(range),
            Self::Split {
                before,
                after: _,
                gap_start,
                gap_end: _,
            } if range.end <= gap_start => before.get(range),
            Self::Split {
                before: _,
                after,
                gap_start: _,
                gap_end,
            } if range.start >= gap_end => after.get(range.start - gap_end..range.end - gap_end),
            Self::Split { .. } => None,
        }
    }
}

#[derive(Clone, Copy)]
struct ArenaReadAccess<'a> {
    bools: SliceRead<'a, u8>,
    indexes: SliceRead<'a, u64>,
    f64s: SliceRead<'a, f64>,
    strings: SliceRead<'a, String>,
    snapshots: SliceRead<'a, Option<Value>>,
}

impl<'a> ArenaReadAccess<'a> {
    fn whole(arena: &'a TypedResidentArena) -> Self {
        Self {
            bools: SliceRead::Whole(&arena.bools),
            indexes: SliceRead::Whole(&arena.indexes),
            f64s: SliceRead::Whole(&arena.f64s),
            strings: SliceRead::Whole(&arena.strings),
            snapshots: SliceRead::Whole(&arena.snapshots),
        }
    }

    fn read(self, region: ResidentRegion) -> Option<ResidentValueRef<'a>> {
        let range = region.offset..region.offset + region.len;
        match region.kind {
            ResidentValueKind::Bool => self.bools.get(range).map(ResidentValueRef::Bool),
            ResidentValueKind::Index => self.indexes.get(range).map(ResidentValueRef::Index),
            ResidentValueKind::F64 => self.f64s.get(range).map(ResidentValueRef::F64),
            ResidentValueKind::String => self.strings.get(range).map(ResidentValueRef::String),
            ResidentValueKind::Snapshot => {
                self.snapshots.get(range).map(ResidentValueRef::Snapshot)
            }
        }
    }

    fn read_f64(self, region: ResidentRegion) -> Option<&'a [f64]> {
        (region.kind == ResidentValueKind::F64)
            .then(|| self.f64s.get(region.offset..region.offset + region.len))?
    }
}

impl TypedResidentArena {
    fn split_output(
        &mut self,
        region: ResidentRegion,
    ) -> (ArenaReadAccess<'_>, ResidentValueMut<'_>) {
        let gap_start = region.offset;
        let gap_end = region.offset + region.len;
        match region.kind {
            ResidentValueKind::Bool => {
                let (before, tail) = self.bools.split_at_mut(region.offset);
                let (output, after) = tail.split_at_mut(region.len);
                (
                    ArenaReadAccess {
                        bools: SliceRead::Split {
                            before,
                            after,
                            gap_start,
                            gap_end,
                        },
                        indexes: SliceRead::Whole(&self.indexes),
                        f64s: SliceRead::Whole(&self.f64s),
                        strings: SliceRead::Whole(&self.strings),
                        snapshots: SliceRead::Whole(&self.snapshots),
                    },
                    ResidentValueMut::Bool(output),
                )
            }
            ResidentValueKind::Index => {
                let (before, tail) = self.indexes.split_at_mut(region.offset);
                let (output, after) = tail.split_at_mut(region.len);
                (
                    ArenaReadAccess {
                        bools: SliceRead::Whole(&self.bools),
                        indexes: SliceRead::Split {
                            before,
                            after,
                            gap_start,
                            gap_end,
                        },
                        f64s: SliceRead::Whole(&self.f64s),
                        strings: SliceRead::Whole(&self.strings),
                        snapshots: SliceRead::Whole(&self.snapshots),
                    },
                    ResidentValueMut::Index(output),
                )
            }
            ResidentValueKind::F64 => {
                let (before, tail) = self.f64s.split_at_mut(region.offset);
                let (output, after) = tail.split_at_mut(region.len);
                (
                    ArenaReadAccess {
                        bools: SliceRead::Whole(&self.bools),
                        indexes: SliceRead::Whole(&self.indexes),
                        f64s: SliceRead::Split {
                            before,
                            after,
                            gap_start,
                            gap_end,
                        },
                        strings: SliceRead::Whole(&self.strings),
                        snapshots: SliceRead::Whole(&self.snapshots),
                    },
                    ResidentValueMut::F64(output),
                )
            }
            ResidentValueKind::String => {
                let (before, tail) = self.strings.split_at_mut(region.offset);
                let (output, after) = tail.split_at_mut(region.len);
                (
                    ArenaReadAccess {
                        bools: SliceRead::Whole(&self.bools),
                        indexes: SliceRead::Whole(&self.indexes),
                        f64s: SliceRead::Whole(&self.f64s),
                        strings: SliceRead::Split {
                            before,
                            after,
                            gap_start,
                            gap_end,
                        },
                        snapshots: SliceRead::Whole(&self.snapshots),
                    },
                    ResidentValueMut::String(output),
                )
            }
            ResidentValueKind::Snapshot => {
                let (before, tail) = self.snapshots.split_at_mut(region.offset);
                let (output, after) = tail.split_at_mut(region.len);
                (
                    ArenaReadAccess {
                        bools: SliceRead::Whole(&self.bools),
                        indexes: SliceRead::Whole(&self.indexes),
                        f64s: SliceRead::Whole(&self.f64s),
                        strings: SliceRead::Whole(&self.strings),
                        snapshots: SliceRead::Split {
                            before,
                            after,
                            gap_start,
                            gap_end,
                        },
                    },
                    ResidentValueMut::Snapshot(output),
                )
            }
        }
    }

    #[inline(always)]
    fn split_scratch_output(
        &mut self,
        region: ResidentRegion,
    ) -> (ScratchArenaReadAccess<'_>, ResidentValueMut<'_>) {
        match region.kind {
            ResidentValueKind::Bool => {
                let (before, tail) = self.bools.split_at_mut(region.offset);
                let (output, _) = tail.split_at_mut(region.len);
                (
                    ScratchArenaReadAccess {
                        bools: before,
                        indexes: &self.indexes,
                        f64s: &self.f64s,
                        strings: &self.strings,
                        snapshots: &self.snapshots,
                    },
                    ResidentValueMut::Bool(output),
                )
            }
            ResidentValueKind::Index => {
                let (before, tail) = self.indexes.split_at_mut(region.offset);
                let (output, _) = tail.split_at_mut(region.len);
                (
                    ScratchArenaReadAccess {
                        bools: &self.bools,
                        indexes: before,
                        f64s: &self.f64s,
                        strings: &self.strings,
                        snapshots: &self.snapshots,
                    },
                    ResidentValueMut::Index(output),
                )
            }
            ResidentValueKind::F64 => {
                let (before, tail) = self.f64s.split_at_mut(region.offset);
                let (output, _) = tail.split_at_mut(region.len);
                (
                    ScratchArenaReadAccess {
                        bools: &self.bools,
                        indexes: &self.indexes,
                        f64s: before,
                        strings: &self.strings,
                        snapshots: &self.snapshots,
                    },
                    ResidentValueMut::F64(output),
                )
            }
            ResidentValueKind::String => {
                let (before, tail) = self.strings.split_at_mut(region.offset);
                let (output, _) = tail.split_at_mut(region.len);
                (
                    ScratchArenaReadAccess {
                        bools: &self.bools,
                        indexes: &self.indexes,
                        f64s: &self.f64s,
                        strings: before,
                        snapshots: &self.snapshots,
                    },
                    ResidentValueMut::String(output),
                )
            }
            ResidentValueKind::Snapshot => {
                let (before, tail) = self.snapshots.split_at_mut(region.offset);
                let (output, _) = tail.split_at_mut(region.len);
                (
                    ScratchArenaReadAccess {
                        bools: &self.bools,
                        indexes: &self.indexes,
                        f64s: &self.f64s,
                        strings: &self.strings,
                        snapshots: before,
                    },
                    ResidentValueMut::Snapshot(output),
                )
            }
        }
    }

    #[inline(always)]
    fn split_f64_scratch_output(
        &mut self,
        region: ResidentRegion,
    ) -> (ScratchArenaReadAccess<'_>, &mut [f64]) {
        debug_assert_eq!(region.kind, ResidentValueKind::F64);
        let (before, tail) = self.f64s.split_at_mut(region.offset);
        let (output, _) = tail.split_at_mut(region.len);
        (
            ScratchArenaReadAccess {
                bools: &self.bools,
                indexes: &self.indexes,
                f64s: before,
                strings: &self.strings,
                snapshots: &self.snapshots,
            },
            output,
        )
    }
}

#[derive(Clone, Copy)]
struct ScratchArenaReadAccess<'a> {
    bools: &'a [u8],
    indexes: &'a [u64],
    f64s: &'a [f64],
    strings: &'a [String],
    snapshots: &'a [Option<Value>],
}

impl<'a> ScratchArenaReadAccess<'a> {
    #[inline(always)]
    fn read(self, region: ResidentRegion) -> Option<ResidentValueRef<'a>> {
        let range = region.offset..region.offset + region.len;
        match region.kind {
            ResidentValueKind::Bool => self.bools.get(range).map(ResidentValueRef::Bool),
            ResidentValueKind::Index => self.indexes.get(range).map(ResidentValueRef::Index),
            ResidentValueKind::F64 => self.f64s.get(range).map(ResidentValueRef::F64),
            ResidentValueKind::String => self.strings.get(range).map(ResidentValueRef::String),
            ResidentValueKind::Snapshot => {
                self.snapshots.get(range).map(ResidentValueRef::Snapshot)
            }
        }
    }

    #[inline(always)]
    fn read_f64(self, region: ResidentRegion) -> Option<&'a [f64]> {
        (region.kind == ResidentValueKind::F64)
            .then(|| self.f64s.get(region.offset..region.offset + region.len))?
    }
}

struct StateReadAccess<'a> {
    buffers: [ArenaReadAccess<'a>; 2],
    versions: &'a [StateVersion],
    version_by_slot: &'a [Option<usize>],
}

impl<'a> StateReadAccess<'a> {
    #[cfg(test)]
    fn whole(state: &'a StateArena) -> Self {
        Self {
            buffers: [
                ArenaReadAccess::whole(&state.buffers[0]),
                ArenaReadAccess::whole(&state.buffers[1]),
            ],
            versions: &state.versions,
            version_by_slot: &state.version_by_slot,
        }
    }

    fn split_output(
        buffers: &'a mut [TypedResidentArena; 2],
        versions: &'a [StateVersion],
        version_by_slot: &'a [Option<usize>],
        candidate: usize,
        region: ResidentRegion,
    ) -> (Self, ResidentValueMut<'a>) {
        let [left, right] = buffers;
        if candidate == 0 {
            let (left, output) = left.split_output(region);
            (
                Self {
                    buffers: [left, ArenaReadAccess::whole(right)],
                    versions,
                    version_by_slot,
                },
                output,
            )
        } else {
            let (right, output) = right.split_output(region);
            (
                Self {
                    buffers: [ArenaReadAccess::whole(left), right],
                    versions,
                    version_by_slot,
                },
                output,
            )
        }
    }

    fn read(
        &self,
        slot: CellSlotId,
        region: ResidentRegion,
        epoch: InstanceEpoch,
    ) -> Option<ResidentValueRef<'a>> {
        let version = &self.versions[*self.version_by_slot.get(slot.get() as usize)?.as_ref()?];
        self.buffers[select_version(version, epoch)].read(region)
    }

    fn read_f64(
        &self,
        slot: CellSlotId,
        region: ResidentRegion,
        epoch: InstanceEpoch,
    ) -> Option<&'a [f64]> {
        let version = &self.versions[*self.version_by_slot.get(slot.get() as usize)?.as_ref()?];
        self.buffers[select_version(version, epoch)].read_f64(region)
    }
}

struct F64NodeInputs<'a> {
    locations: &'a [F64ReadTapeEntry],
    arenas: [&'a [f64]; 5],
    state_arena_by_slot: &'a [u8],
}

impl F64NodeInputs<'_> {
    #[inline(always)]
    fn f64_at(&self, index: usize) -> Option<&[f64]> {
        let location = *self.locations.get(index)?;
        let arena = if location.selector & F64_STATE_SLOT_BIT == 0 {
            location.selector as usize
        } else {
            let slot = (location.selector & !F64_STATE_SLOT_BIT) as usize;
            *self.state_arena_by_slot.get(slot)? as usize
        };
        self.arenas
            .get(arena)?
            .get(location.start as usize..location.end as usize)
    }
}

impl ResidentKernelInputs for F64NodeInputs<'_> {
    #[inline(always)]
    fn len(&self) -> usize {
        self.locations.len()
    }

    #[inline(always)]
    fn get(&self, index: usize) -> Option<ResidentValueRef<'_>> {
        self.f64_at(index).map(ResidentValueRef::F64)
    }

    #[inline(always)]
    fn f64(&self, index: usize) -> Option<&[f64]> {
        self.f64_at(index)
    }
}

struct F64NonStateNodeInputs<'a> {
    locations: &'a [F64ReadTapeEntry],
    arenas: [&'a [f64]; 3],
}

impl F64NonStateNodeInputs<'_> {
    #[inline(always)]
    fn f64_at(&self, index: usize) -> Option<&[f64]> {
        let location = *self.locations.get(index)?;
        if location.selector & F64_STATE_SLOT_BIT != 0 {
            return None;
        }
        self.arenas
            .get(location.selector as usize)?
            .get(location.start as usize..location.end as usize)
    }
}

impl ResidentKernelInputs for F64NonStateNodeInputs<'_> {
    #[inline(always)]
    fn len(&self) -> usize {
        self.locations.len()
    }

    #[inline(always)]
    fn get(&self, index: usize) -> Option<ResidentValueRef<'_>> {
        self.f64_at(index).map(ResidentValueRef::F64)
    }

    #[inline(always)]
    fn f64(&self, index: usize) -> Option<&[f64]> {
        self.f64_at(index)
    }
}

struct ScratchNodeInputs<'a> {
    locations: &'a [ResidentReadLocation],
    activation: &'a TypedResidentArena,
    input: &'a TypedResidentArena,
    state: &'a StateArena,
    scratch: ScratchArenaReadAccess<'a>,
    epoch: InstanceEpoch,
}

impl ScratchNodeInputs<'_> {
    #[inline(always)]
    fn f64_at(&self, index: usize) -> Option<&[f64]> {
        match *self.locations.get(index)? {
            ResidentReadLocation::Constant(region) => self.activation.read_f64(region),
            ResidentReadLocation::Input(region) => self.input.read_f64(region),
            ResidentReadLocation::State { slot, region } => {
                self.state.read_f64(slot, region, self.epoch)
            }
            ResidentReadLocation::Scratch(region) => self.scratch.read_f64(region),
        }
    }
}

impl ResidentKernelInputs for ScratchNodeInputs<'_> {
    fn len(&self) -> usize {
        self.locations.len()
    }

    fn get(&self, index: usize) -> Option<ResidentValueRef<'_>> {
        match *self.locations.get(index)? {
            ResidentReadLocation::Constant(region) => Some(self.activation.read(region)),
            ResidentReadLocation::Input(region) => Some(self.input.read(region)),
            ResidentReadLocation::State { slot, region } => {
                self.state.read(slot, region, self.epoch)
            }
            ResidentReadLocation::Scratch(region) => self.scratch.read(region),
        }
    }

    fn f64(&self, index: usize) -> Option<&[f64]> {
        self.f64_at(index)
    }

    fn f64_1(&self) -> Option<[&[f64]; 1]> {
        Some([self.f64_at(0)?])
    }

    fn f64_2(&self) -> Option<[&[f64]; 2]> {
        Some([self.f64_at(0)?, self.f64_at(1)?])
    }

    fn f64_3(&self) -> Option<[&[f64]; 3]> {
        Some([self.f64_at(0)?, self.f64_at(1)?, self.f64_at(2)?])
    }

    fn f64_4(&self) -> Option<[&[f64]; 4]> {
        Some([
            self.f64_at(0)?,
            self.f64_at(1)?,
            self.f64_at(2)?,
            self.f64_at(3)?,
        ])
    }
}

struct GeneralScratchNodeInputs<'a> {
    locations: &'a [ResidentReadLocation],
    activation: &'a TypedResidentArena,
    input: &'a TypedResidentArena,
    state: &'a StateArena,
    scratch: ArenaReadAccess<'a>,
    epoch: InstanceEpoch,
}

impl ResidentKernelInputs for GeneralScratchNodeInputs<'_> {
    fn len(&self) -> usize {
        self.locations.len()
    }

    fn get(&self, index: usize) -> Option<ResidentValueRef<'_>> {
        match *self.locations.get(index)? {
            ResidentReadLocation::Constant(region) => Some(self.activation.read(region)),
            ResidentReadLocation::Input(region) => Some(self.input.read(region)),
            ResidentReadLocation::State { slot, region } => {
                self.state.read(slot, region, self.epoch)
            }
            ResidentReadLocation::Scratch(region) => self.scratch.read(region),
        }
    }

    fn f64(&self, index: usize) -> Option<&[f64]> {
        match *self.locations.get(index)? {
            ResidentReadLocation::Constant(region) => self.activation.read_f64(region),
            ResidentReadLocation::Input(region) => self.input.read_f64(region),
            ResidentReadLocation::State { slot, region } => {
                self.state.read_f64(slot, region, self.epoch)
            }
            ResidentReadLocation::Scratch(region) => self.scratch.read_f64(region),
        }
    }
}

struct StateNodeInputsWithoutState<'a> {
    locations: &'a [ResidentReadLocation],
    activation: &'a TypedResidentArena,
    input: &'a TypedResidentArena,
    scratch: &'a TypedResidentArena,
}

impl ResidentKernelInputs for StateNodeInputsWithoutState<'_> {
    fn len(&self) -> usize {
        self.locations.len()
    }

    fn get(&self, index: usize) -> Option<ResidentValueRef<'_>> {
        match *self.locations.get(index)? {
            ResidentReadLocation::Constant(region) => Some(self.activation.read(region)),
            ResidentReadLocation::Input(region) => Some(self.input.read(region)),
            ResidentReadLocation::Scratch(region) => Some(self.scratch.read(region)),
            ResidentReadLocation::State { .. } => None,
        }
    }

    fn f64(&self, index: usize) -> Option<&[f64]> {
        match *self.locations.get(index)? {
            ResidentReadLocation::Constant(region) => self.activation.read_f64(region),
            ResidentReadLocation::Input(region) => self.input.read_f64(region),
            ResidentReadLocation::Scratch(region) => self.scratch.read_f64(region),
            ResidentReadLocation::State { .. } => None,
        }
    }
}

struct StateNodeInputs<'a> {
    locations: &'a [ResidentReadLocation],
    activation: &'a TypedResidentArena,
    input: &'a TypedResidentArena,
    state: StateReadAccess<'a>,
    scratch: &'a TypedResidentArena,
    epoch: InstanceEpoch,
}

impl ResidentKernelInputs for StateNodeInputs<'_> {
    fn len(&self) -> usize {
        self.locations.len()
    }

    fn get(&self, index: usize) -> Option<ResidentValueRef<'_>> {
        match *self.locations.get(index)? {
            ResidentReadLocation::Constant(region) => Some(self.activation.read(region)),
            ResidentReadLocation::Input(region) => Some(self.input.read(region)),
            ResidentReadLocation::State { slot, region } => {
                self.state.read(slot, region, self.epoch)
            }
            ResidentReadLocation::Scratch(region) => Some(self.scratch.read(region)),
        }
    }

    fn f64(&self, index: usize) -> Option<&[f64]> {
        match *self.locations.get(index)? {
            ResidentReadLocation::Constant(region) => self.activation.read_f64(region),
            ResidentReadLocation::Input(region) => self.input.read_f64(region),
            ResidentReadLocation::State { slot, region } => {
                self.state.read_f64(slot, region, self.epoch)
            }
            ResidentReadLocation::Scratch(region) => self.scratch.read_f64(region),
        }
    }
}

#[inline(always)]
fn select_version(version: &StateVersion, epoch: InstanceEpoch) -> usize {
    let [left, right] = version.epochs;
    if left == Some(epoch) {
        return 0;
    }
    if right == Some(epoch) {
        return 1;
    }
    match (left, right) {
        (Some(left), Some(right)) if left <= epoch || right <= epoch => {
            usize::from(right <= epoch && (left > epoch || right > left))
        }
        (Some(left), None) if left <= epoch => 0,
        (None, Some(right)) if right <= epoch => 1,
        _ => panic!("resident state retains a version at or before the selected epoch"),
    }
}

fn copy_input(
    arena: &mut TypedResidentArena,
    region: ResidentRegion,
    value: ResidentValueRef<'_>,
) -> Result<(), ()> {
    match (arena.write(region), value) {
        (ResidentValueMut::Bool(target), ResidentValueRef::Bool(source))
            if target.len() == source.len() =>
        {
            if source.iter().any(|value| *value > 1) {
                return Err(());
            }
            target.copy_from_slice(source);
        }
        (ResidentValueMut::Index(target), ResidentValueRef::Index(source))
            if target.len() == source.len() =>
        {
            target.copy_from_slice(source);
        }
        (ResidentValueMut::F64(target), ResidentValueRef::F64(source))
            if target.len() == source.len() =>
        {
            target.copy_from_slice(source);
        }
        (ResidentValueMut::String(target), ResidentValueRef::String(source))
            if target.len() == source.len() =>
        {
            target.clone_from_slice(source);
        }
        (ResidentValueMut::Snapshot(target), ResidentValueRef::Snapshot(source))
            if target.len() == source.len() =>
        {
            target.clone_from_slice(source);
        }
        _ => return Err(()),
    }
    Ok(())
}

fn regions_equal(
    left: &TypedResidentArena,
    left_region: ResidentRegion,
    right: &TypedResidentArena,
    right_region: ResidentRegion,
) -> bool {
    match (left.read(left_region), right.read(right_region)) {
        (ResidentValueRef::Bool(left), ResidentValueRef::Bool(right)) => left == right,
        (ResidentValueRef::Index(left), ResidentValueRef::Index(right)) => left == right,
        (ResidentValueRef::F64(left), ResidentValueRef::F64(right)) => left
            .iter()
            .zip(right)
            .all(|(left, right)| left.to_bits() == right.to_bits()),
        (ResidentValueRef::String(left), ResidentValueRef::String(right)) => left == right,
        // Composite producers currently declare AlwaysChanged. Keep this
        // conservative until a schema-aware arena comparison is introduced.
        (ResidentValueRef::Snapshot(_), ResidentValueRef::Snapshot(_)) => false,
        _ => false,
    }
}

fn scalar_token(value: ResidentValueRef<'_>) -> Option<u64> {
    match value {
        ResidentValueRef::Bool([value]) => Some(u64::from(*value)),
        ResidentValueRef::Index([value]) => Some(*value),
        ResidentValueRef::F64([value]) => Some(value.to_bits()),
        ResidentValueRef::String([value]) => Some(hash_string(value)),
        ResidentValueRef::Snapshot(_) => None,
        _ => None,
    }
}

fn region_bytes(region: ResidentRegion) -> usize {
    region.len
        * match region.kind {
            ResidentValueKind::Bool => 1,
            ResidentValueKind::Index | ResidentValueKind::F64 => 8,
            ResidentValueKind::String => core::mem::size_of::<String>(),
            ResidentValueKind::Snapshot => core::mem::size_of::<Option<Value>>(),
        }
}

fn bit_is_set(words: &[u64], bit: usize) -> bool {
    words[bit / 64] & (1_u64 << (bit % 64)) != 0
}

fn set_bit(words: &mut [u64], bit: usize) {
    words[bit / 64] |= 1_u64 << (bit % 64);
}

fn or_bits(target: &mut [u64], source: &[u64]) {
    for (target, source) in target.iter_mut().zip(source) {
        *target |= *source;
    }
}

fn count_bits(words: &[u64]) -> usize {
    words.iter().map(|word| word.count_ones() as usize).sum()
}

fn state_hash(instance: &ReactiveInstance, epoch: InstanceEpoch) -> u64 {
    // Activation folds program, slot, schema, and shape identity into this
    // immutable seed. A turn folds only the selected payload bits.
    let mut hash = instance.plan.state_hash_seed;
    for artifact_slot in &instance.plan.state_slots {
        let slot = &instance.plan.slots[artifact_slot.get() as usize];
        let buffer = instance.state.select_buffer(slot.artifact_id, epoch);
        match instance.state.buffers[buffer].read(slot.region) {
            ResidentValueRef::Bool(values) => {
                for value in values {
                    hash = hash_word(hash, u64::from(*value));
                }
            }
            ResidentValueRef::Index(values) => {
                for value in values {
                    hash = hash_word(hash, *value);
                }
            }
            ResidentValueRef::F64(values) => {
                for value in values {
                    hash = hash_word(hash, value.to_bits());
                }
            }
            ResidentValueRef::String(values) => {
                for value in values {
                    hash = hash_word(hash, hash_string(value));
                }
            }
            ResidentValueRef::Snapshot(values) => {
                for value in values {
                    let token = value.as_ref().map(Value::resident_token).unwrap_or(0);
                    hash = hash_word(hash, token);
                }
            }
        }
    }
    hash
}

#[inline(always)]
fn hash_word(hash: u64, word: u64) -> u64 {
    (hash.rotate_left(17) ^ word).wrapping_mul(0xd6e8_feb8_6659_fd93)
}

fn hash_string(value: &str) -> u64 {
    value
        .as_bytes()
        .iter()
        .fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resident::general::{ResidentArenaSizes, StateVersion};
    use mech_core::ResidentShape;

    fn state_arena() -> StateArena {
        let sizes = ResidentArenaSizes {
            f64s: 2,
            ..ResidentArenaSizes::default()
        };
        let region = |offset| ResidentRegion {
            kind: ResidentValueKind::F64,
            offset,
            len: 1,
            shape: ResidentShape {
                rows: 1,
                columns: 1,
            },
        };
        let mut state = StateArena {
            buffers: [
                TypedResidentArena::allocate(sizes),
                TypedResidentArena::allocate(sizes),
            ],
            versions: vec![
                StateVersion {
                    slot: CellSlotId::new(0),
                    region: region(0),
                    epochs: [Some(InstanceEpoch::ZERO), None],
                },
                StateVersion {
                    slot: CellSlotId::new(1),
                    region: region(1),
                    epochs: [Some(InstanceEpoch::ZERO), None],
                },
            ]
            .into_boxed_slice(),
            version_by_slot: vec![Some(0), Some(1)].into_boxed_slice(),
        };
        state.buffers[0].f64s.copy_from_slice(&[10.0, 20.0]);
        state
    }

    fn scalar(state: &StateArena, slot: u32, epoch: u64) -> f64 {
        let ResidentValueRef::F64(value) =
            state.read_published(CellSlotId::new(slot), InstanceEpoch::new(epoch))
        else {
            panic!("test state is f64")
        };
        value[0]
    }

    #[test]
    fn sparse_epoch_selection_retains_untouched_state() {
        let mut state = state_arena();
        let (candidate, seeded) = state.begin_rmw(
            CellSlotId::new(0),
            InstanceEpoch::ZERO,
            InstanceEpoch::new(1),
        );
        assert!(seeded);
        state.buffers[candidate].f64s[0] = 11.0;

        assert_eq!(scalar(&state, 0, 1), 11.0);
        assert_eq!(scalar(&state, 1, 1), 20.0);
        assert_eq!(
            state.epochs(CellSlotId::new(1)),
            [Some(InstanceEpoch::ZERO), None]
        );
    }

    #[test]
    fn repeated_rmw_reuses_one_seeded_candidate() {
        let mut state = state_arena();
        let working = InstanceEpoch::new(1);
        let (first, seeded_first) =
            state.begin_rmw(CellSlotId::new(0), InstanceEpoch::ZERO, working);
        state.buffers[first].f64s[0] += 2.0;
        let (second, seeded_second) =
            state.begin_rmw(CellSlotId::new(0), InstanceEpoch::ZERO, working);
        state.buffers[second].f64s[0] += 3.0;

        assert!(seeded_first);
        assert!(!seeded_second);
        assert_eq!(first, second);
        assert_eq!(scalar(&state, 0, 1), 15.0);
    }

    #[test]
    fn full_write_candidate_is_not_seeded_from_publication() {
        let mut state = state_arena();
        state.buffers[1].f64s[0] = 99.0;
        let candidate = state.candidate_buffer(CellSlotId::new(0), InstanceEpoch::ZERO);
        assert_eq!(state.buffers[candidate].f64s[0], 99.0);
        state.buffers[candidate].f64s[0] = 12.0;
        state.tag(CellSlotId::new(0), candidate, InstanceEpoch::new(1));
        assert_eq!(scalar(&state, 0, 1), 12.0);
    }

    #[test]
    fn reads_switch_from_published_base_to_same_turn_candidate() {
        let mut state = state_arena();
        let slot = CellSlotId::new(0);
        let region = state.version(slot).region;
        let working = InstanceEpoch::new(1);
        let before = StateReadAccess::whole(&state)
            .read(slot, region, working)
            .unwrap();
        assert!(matches!(before, ResidentValueRef::F64(values) if values == [10.0]));

        let (candidate, _) = state.begin_rmw(slot, InstanceEpoch::ZERO, working);
        state.buffers[candidate].f64s[0] = 42.0;
        let after = StateReadAccess::whole(&state)
            .read(slot, region, working)
            .unwrap();
        assert!(matches!(after, ResidentValueRef::F64(values) if values == [42.0]));
    }

    #[test]
    fn abort_clears_only_the_working_epoch_tags() {
        let mut state = state_arena();
        let slot0 = CellSlotId::new(0);
        let slot1 = CellSlotId::new(1);
        let (first, _) = state.begin_rmw(slot0, InstanceEpoch::ZERO, InstanceEpoch::new(1));
        state.buffers[first].f64s[0] = 11.0;
        let (second, _) = state.begin_rmw(slot1, InstanceEpoch::new(1), InstanceEpoch::new(2));
        state.buffers[second].f64s[1] = 21.0;

        state.abort(InstanceEpoch::new(2));

        assert!(state.epochs(slot0).contains(&Some(InstanceEpoch::new(1))));
        assert!(!state.epochs(slot1).contains(&Some(InstanceEpoch::new(2))));
        assert_eq!(scalar(&state, 0, 2), 11.0);
        assert_eq!(scalar(&state, 1, 2), 20.0);
    }

    #[test]
    fn unchanged_rmw_candidate_is_detected_without_restoration() {
        let mut state = state_arena();
        let slot = CellSlotId::new(0);
        let working = InstanceEpoch::new(1);
        let (candidate, _) = state.begin_rmw(slot, InstanceEpoch::ZERO, working);
        assert!(state.same_at(slot, candidate, InstanceEpoch::ZERO));
        state.abort(working);
        assert_eq!(scalar(&state, 0, 0), 10.0);
    }
}
