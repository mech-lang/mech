//! Allocation-free candidate execution for the schema-driven resident plan.

use core::ops::Range;
use core::sync::atomic::Ordering;

use mech_core::{
    CellSlotId, ChangeDetectionPolicy, InstanceEpoch, IntegrityConstraintId, NodeId,
    OutputConstruction, ProgramRevision, ReactiveInstanceId, ResidentKernelError,
    ResidentKernelInputs, ResidentValueKind, ResidentValueMut, ResidentValueRef, SlotIndex,
};

use super::{
    ActivatedNodeIndex, EAGER_F64_STATE_ARENA_BASE, EAGER_F64_STATE_SLOT_BIT, EagerF64Read,
    ReactiveInstance, ResidentReadLocation, ResidentRegion, ResidentStorageClass, StateArena,
    StateVersion, TypedResidentArena,
};

#[derive(Clone, Copy, Debug)]
pub struct CapturedSignalInput<'a> {
    pub slot: SlotIndex,
    pub value: ResidentValueRef<'a>,
}

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
    CounterOverflow,
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

    #[inline]
    pub fn publish(mut self) -> ResidentTurnSummary {
        let instance = self.instance.take().expect("live prepared resident turn");
        instance
            .published_epoch
            .store(self.working_epoch.get(), Ordering::Release);
        instance.candidate_active = false;
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
        if let Err(error) = self.begin_workspace(inputs, before_epoch) {
            self.next_epoch = Some(working_epoch);
            return Err(error);
        }
        let mut probe = ResidentStructuralProbe {
            publication_store_count: 1,
            record_preparation_count: 1,
            record_append_count: 1,
            ..ResidentStructuralProbe::default()
        };
        if let Err(error) = self.execute_candidate(before_epoch, working_epoch, &mut probe, true) {
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
        Ok(self.prepare_turn(inputs)?.publish())
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
        if self.candidate_active {
            return Err(ResidentExecutionError::ActiveCandidate);
        }
        let working_epoch = self
            .next_epoch
            .ok_or(ResidentExecutionError::EpochExhausted)?;
        self.next_epoch = working_epoch.checked_next().ok();
        let before_epoch = self.published_epoch();
        if let Err(error) = self.begin_workspace(inputs, before_epoch) {
            self.next_epoch = Some(working_epoch);
            return Err(error);
        }
        let mut probe = ResidentStructuralProbe::default();
        if let Err(error) = self.execute_candidate(before_epoch, working_epoch, &mut probe, false) {
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
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| {
                node.write.storage == ResidentStorageClass::State
                    && matches!(
                        node.construction,
                        OutputConstruction::ReadModifyWrite { .. }
                    )
            })
            .filter(|(index, node)| {
                !self.plan.nodes[..*index].iter().any(|earlier| {
                    earlier.write.storage == ResidentStorageClass::State
                        && matches!(
                            earlier.construction,
                            OutputConstruction::ReadModifyWrite { .. }
                        )
                        && earlier.write.slot == node.write.slot
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
        }
    }

    fn abort_candidate(&mut self, working_epoch: InstanceEpoch) {
        self.state.abort(working_epoch);
        self.workspace.initialized_output_bits.fill(0);
        self.workspace.all_outputs_initialized = false;
        self.next_epoch = Some(working_epoch);
        self.candidate_active = false;
    }

    #[doc(hidden)]
    pub fn set_next_epoch_for_test(&mut self, next: u64) {
        assert_ne!(next, 0);
        self.next_epoch = Some(InstanceEpoch::new(next));
    }

    fn begin_workspace(
        &mut self,
        inputs: &[CapturedSignalInput<'_>],
        before_epoch: InstanceEpoch,
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
        self.workspace.touched_slots.clear();
        self.workspace.changed_slots.clear();
        if !self.plan.topology.eager_turn {
            self.workspace.dirty_bits.fill(0);
            self.workspace.executed_bits.fill(0);
            for (target, root) in self
                .workspace
                .dirty_bits
                .iter_mut()
                .zip(&self.plan.topology.turn_root_mask)
            {
                *target = *root;
            }
            or_bits(
                &mut self.workspace.dirty_bits,
                &self.plan.topology.mandatory_candidate_mask,
            );
        }
        if self.plan.eager_f64_reads.is_some() {
            for version in &self.state.versions {
                self.workspace.state_f64_arena_by_slot[version.slot.get() as usize] =
                    EAGER_F64_STATE_ARENA_BASE + select_version(version, before_epoch) as u8;
            }
        }
        Ok(())
    }

    fn execute_candidate(
        &mut self,
        before_epoch: InstanceEpoch,
        working_epoch: InstanceEpoch,
        probe: &mut ResidentStructuralProbe,
        track_summary: bool,
    ) -> Result<(), ResidentExecutionError> {
        if self.plan.topology.eager_turn {
            // Activation proved that every admitted node is downstream of a
            // turn root or mandatory constraint. All declared inputs are
            // installed together, so this dense pure graph executes in its
            // prevalidated linear order without per-node scheduler work.
            if self.plan.eager_f64_reads.is_some() {
                for order in 0..self.plan.topology.linear_node_order.len() {
                    let node_index = self.plan.topology.linear_node_order[order];
                    self.execute_eager_f64_node(
                        node_index,
                        before_epoch,
                        working_epoch,
                        probe,
                        track_summary,
                    )?;
                }
            } else {
                for order in 0..self.plan.topology.linear_node_order.len() {
                    let node_index = self.plan.topology.linear_node_order[order];
                    self.execute_eager_node(node_index, before_epoch, working_epoch, probe)?;
                }
            }
            if track_summary {
                self.workspace.executed_bits.fill(u64::MAX);
                if let Some(last) = self.workspace.executed_bits.last_mut() {
                    let remainder = self.plan.nodes.len() % 64;
                    if remainder != 0 {
                        *last = (1_u64 << remainder) - 1;
                    }
                }
                self.workspace.all_outputs_initialized = true;
            }
        } else if self.plan.topology.word_len() == 1 {
            let mut dirty = self.workspace.dirty_bits[0];
            let mut executed = 0_u64;
            for order in 0..self.plan.topology.linear_node_order.len() {
                let node_index = self.plan.topology.linear_node_order[order];
                let index = node_index.get() as usize;
                let node_bit = 1_u64 << index;
                if dirty & node_bit == 0 {
                    continue;
                }
                let changed = self.execute_node(node_index, before_epoch, working_epoch, probe)?;
                executed |= node_bit;
                if changed {
                    dirty |= self.plan.topology.same_turn_downstream_masks[index][0];
                }
            }
            self.workspace.dirty_bits[0] = dirty;
            self.workspace.executed_bits[0] = executed;
            if !self.workspace.all_outputs_initialized
                && executed.count_ones() as usize == self.plan.nodes.len()
            {
                self.workspace.all_outputs_initialized = true;
            }
        } else {
            for order in 0..self.plan.topology.linear_node_order.len() {
                let node_index = self.plan.topology.linear_node_order[order];
                let index = node_index.get() as usize;
                if !bit_is_set(&self.workspace.dirty_bits, index) {
                    continue;
                }
                let changed = self.execute_node(node_index, before_epoch, working_epoch, probe)?;
                set_bit(&mut self.workspace.executed_bits, index);
                if changed {
                    or_bits(
                        &mut self.workspace.dirty_bits,
                        &self.plan.topology.same_turn_downstream_masks[index],
                    );
                }
            }
            if !self.workspace.all_outputs_initialized
                && count_bits(&self.workspace.executed_bits) == self.plan.nodes.len()
            {
                self.workspace.all_outputs_initialized = true;
            }
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
    fn execute_eager_f64_node(
        &mut self,
        node_index: ActivatedNodeIndex,
        before_epoch: InstanceEpoch,
        working_epoch: InstanceEpoch,
        probe: &mut ResidentStructuralProbe,
        track_summary: bool,
    ) -> Result<(), ResidentExecutionError> {
        let index = node_index.get() as usize;
        let node = &self.plan.nodes[index];
        if !track_summary
            && node.write.storage == ResidentStorageClass::State
            && !node.reads_state
            && matches!(node.construction, OutputConstruction::FullWrite { .. })
        {
            let slot = node.write.slot;
            let candidate = self.state.candidate_buffer(slot, before_epoch);
            let output = self.state.buffers[candidate].write(node.write.region);
            let inputs = EagerNonStateF64NodeInputs {
                locations: &self.plan.eager_f64_reads.as_ref().expect("checked above")
                    [node.reads.start as usize..node.reads.end as usize],
                arenas: [
                    &self.activation.f64s,
                    &self.workspace.input.f64s,
                    &self.workspace.scratch.f64s,
                ],
            };
            node.kernel.execute(&inputs, output).map_err(|error| {
                ResidentExecutionError::Kernel {
                    node: node.artifact_node,
                    error,
                }
            })?;
            self.state.tag(slot, candidate, working_epoch);
            self.workspace.state_f64_arena_by_slot[slot.get() as usize] =
                EAGER_F64_STATE_ARENA_BASE + candidate as u8;
            return Ok(());
        }
        if node.write.storage != ResidentStorageClass::Scratch || !node.scratch_prefix_reads {
            self.execute_eager_node(node_index, before_epoch, working_epoch, probe)?;
            return Ok(());
        }
        let (scratch, output) = self
            .workspace
            .scratch
            .split_scratch_output(node.write.region);
        let inputs = EagerF64NodeInputs {
            locations: &self.plan.eager_f64_reads.as_ref().expect("checked above")
                [node.reads.start as usize..node.reads.end as usize],
            arenas: [
                &self.activation.f64s,
                &self.workspace.input.f64s,
                scratch.f64s,
                &self.state.buffers[0].f64s,
                &self.state.buffers[1].f64s,
            ],
            state_arena_by_slot: &self.workspace.state_f64_arena_by_slot,
        };
        node.kernel
            .execute(&inputs, output)
            .map(|_| ())
            .map_err(|error| ResidentExecutionError::Kernel {
                node: node.artifact_node,
                error,
            })
    }

    #[inline(always)]
    fn execute_eager_node(
        &mut self,
        node_index: ActivatedNodeIndex,
        before_epoch: InstanceEpoch,
        working_epoch: InstanceEpoch,
        probe: &mut ResidentStructuralProbe,
    ) -> Result<(), ResidentExecutionError> {
        let index = node_index.get() as usize;
        let node = &self.plan.nodes[index];
        if node.write.storage != ResidentStorageClass::Scratch {
            self.execute_node(node_index, before_epoch, working_epoch, probe)?;
            return Ok(());
        }
        let execution = if node.scratch_prefix_reads {
            let (scratch, output) = self
                .workspace
                .scratch
                .split_scratch_output(node.write.region);
            let inputs = ScratchNodeInputs {
                locations: &self.plan.reads[node.reads.start as usize..node.reads.end as usize],
                activation: &self.activation,
                input: &self.workspace.input,
                state: &self.state,
                scratch,
                epoch: working_epoch,
            };
            node.kernel.execute(&inputs, output)
        } else {
            let (scratch, output) = self.workspace.scratch.split_output(node.write.region);
            let inputs = GeneralScratchNodeInputs {
                locations: &self.plan.reads[node.reads.start as usize..node.reads.end as usize],
                activation: &self.activation,
                input: &self.workspace.input,
                state: &self.state,
                scratch,
                epoch: working_epoch,
            };
            node.kernel.execute(&inputs, output)
        };
        execution
            .map(|_| ())
            .map_err(|error| ResidentExecutionError::Kernel {
                node: node.artifact_node,
                error,
            })
    }

    #[inline(always)]
    fn execute_node(
        &mut self,
        node_index: ActivatedNodeIndex,
        before_epoch: InstanceEpoch,
        working_epoch: InstanceEpoch,
        probe: &mut ResidentStructuralProbe,
    ) -> Result<bool, ResidentExecutionError> {
        let index = node_index.get() as usize;
        let node = &self.plan.nodes[index];
        match node.write.storage {
            ResidentStorageClass::Scratch => {
                let before_scalar = if node.change_detection == ChangeDetectionPolicy::ExactScalar {
                    scalar_token(self.workspace.scratch.read(node.write.region))
                } else {
                    None
                };
                let kernel_changed = if node.scratch_prefix_reads {
                    let (scratch, output) = self
                        .workspace
                        .scratch
                        .split_scratch_output(node.write.region);
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
                        if self.plan.eager_f64_reads.is_some() {
                            self.workspace.state_f64_arena_by_slot[slot.get() as usize] =
                                EAGER_F64_STATE_ARENA_BASE + candidate as u8;
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
                    let inputs = StateNodeInputsWithoutState {
                        locations: &self.plan.reads
                            [node.reads.start as usize..node.reads.end as usize],
                        activation: &self.activation,
                        input: &self.workspace.input,
                        scratch: &self.workspace.scratch,
                    };
                    node.kernel.execute(&inputs, output).map_err(|error| {
                        ResidentExecutionError::Kernel {
                            node: node.artifact_node,
                            error,
                        }
                    })?
                };
                if matches!(node.construction, OutputConstruction::FullWrite { .. }) {
                    self.state.tag(slot, candidate, working_epoch);
                    if self.plan.eager_f64_reads.is_some() {
                        self.workspace.state_f64_arena_by_slot[slot.get() as usize] =
                            EAGER_F64_STATE_ARENA_BASE + candidate as u8;
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
                    Ok(changed)
                } else {
                    Ok(changed)
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

#[derive(Clone, Copy)]
enum SliceRead<'a, T> {
    Whole(&'a [T]),
    Split {
        before: &'a [T],
        after: &'a [T],
        gap_start: usize,
        gap_end: usize,
    },
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
}

impl<'a> ArenaReadAccess<'a> {
    fn whole(arena: &'a TypedResidentArena) -> Self {
        Self {
            bools: SliceRead::Whole(&arena.bools),
            indexes: SliceRead::Whole(&arena.indexes),
            f64s: SliceRead::Whole(&arena.f64s),
        }
    }

    fn read(self, region: ResidentRegion) -> Option<ResidentValueRef<'a>> {
        let range = region.offset..region.offset + region.len;
        match region.kind {
            ResidentValueKind::Bool => self.bools.get(range).map(ResidentValueRef::Bool),
            ResidentValueKind::Index => self.indexes.get(range).map(ResidentValueRef::Index),
            ResidentValueKind::F64 => self.f64s.get(range).map(ResidentValueRef::F64),
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
                    },
                    ResidentValueMut::F64(output),
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
                    },
                    ResidentValueMut::F64(output),
                )
            }
        }
    }
}

#[derive(Clone, Copy)]
struct ScratchArenaReadAccess<'a> {
    bools: &'a [u8],
    indexes: &'a [u64],
    f64s: &'a [f64],
}

impl<'a> ScratchArenaReadAccess<'a> {
    #[inline(always)]
    fn read(self, region: ResidentRegion) -> Option<ResidentValueRef<'a>> {
        let range = region.offset..region.offset + region.len;
        match region.kind {
            ResidentValueKind::Bool => self.bools.get(range).map(ResidentValueRef::Bool),
            ResidentValueKind::Index => self.indexes.get(range).map(ResidentValueRef::Index),
            ResidentValueKind::F64 => self.f64s.get(range).map(ResidentValueRef::F64),
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

struct ScratchNodeInputs<'a> {
    locations: &'a [ResidentReadLocation],
    activation: &'a TypedResidentArena,
    input: &'a TypedResidentArena,
    state: &'a StateArena,
    scratch: ScratchArenaReadAccess<'a>,
    epoch: InstanceEpoch,
}

struct EagerF64NodeInputs<'a> {
    locations: &'a [EagerF64Read],
    arenas: [&'a [f64]; 5],
    state_arena_by_slot: &'a [u8],
}

impl EagerF64NodeInputs<'_> {
    #[inline(always)]
    fn f64_at(&self, index: usize) -> Option<&[f64]> {
        let location = *self.locations.get(index)?;
        let arena = if location.selector & EAGER_F64_STATE_SLOT_BIT == 0 {
            location.selector as usize
        } else {
            let slot = (location.selector & !EAGER_F64_STATE_SLOT_BIT) as usize;
            *self.state_arena_by_slot.get(slot)? as usize
        };
        self.arenas
            .get(arena)?
            .get(location.start as usize..location.end as usize)
    }
}

impl ResidentKernelInputs for EagerF64NodeInputs<'_> {
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

struct EagerNonStateF64NodeInputs<'a> {
    locations: &'a [EagerF64Read],
    arenas: [&'a [f64]; 3],
}

impl EagerNonStateF64NodeInputs<'_> {
    #[inline(always)]
    fn f64_at(&self, index: usize) -> Option<&[f64]> {
        let location = *self.locations.get(index)?;
        if location.selector & EAGER_F64_STATE_SLOT_BIT != 0 {
            return None;
        }
        self.arenas
            .get(location.selector as usize)?
            .get(location.start as usize..location.end as usize)
    }
}

impl ResidentKernelInputs for EagerNonStateF64NodeInputs<'_> {
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
        _ => false,
    }
}

fn scalar_token(value: ResidentValueRef<'_>) -> Option<u64> {
    match value {
        ResidentValueRef::Bool([value]) => Some(u64::from(*value)),
        ResidentValueRef::Index([value]) => Some(*value),
        ResidentValueRef::F64([value]) => Some(value.to_bits()),
        _ => None,
    }
}

fn region_bytes(region: ResidentRegion) -> usize {
    region.len
        * match region.kind {
            ResidentValueKind::Bool => 1,
            ResidentValueKind::Index | ResidentValueKind::F64 => 8,
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
        }
    }
    hash
}

#[inline(always)]
fn hash_word(hash: u64, word: u64) -> u64 {
    (hash.rotate_left(17) ^ word).wrapping_mul(0xd6e8_feb8_6659_fd93)
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
