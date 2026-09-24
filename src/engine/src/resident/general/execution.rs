//! Candidate execution for the schema-driven resident plan.

#[path = "comprehension_execution.rs"]
mod comprehension_execution;

pub(super) use comprehension_execution::StructuralProjectionTable;

fn peak_structural_clone_depth(arms: &[super::ActivatedMatchArm]) -> u64 {
    arms.iter()
        .filter_map(|arm| match &arm.pattern {
            super::ActivatedMatchPattern::Structural { clone_depth, .. } => Some(*clone_depth),
            _ => None,
        })
        .max()
        .unwrap_or(0)
}

pub(super) fn structural_projection_schema_context(
    schemas: &mech_core::SchemaTable,
) -> Result<(mech_core::SchemaTable, StructuralProjectionTable), mech_core::SemanticModelError> {
    comprehension_execution::structural_projection_schema_context(schemas)
}

use crate::resident::budget;
use core::ops::Range;
use core::sync::atomic::Ordering;

use mech_core::snapshot::ValueFootprint;
use mech_core::{
    ApplicationRequirementId, CellSlotId, ChangeDetectionPolicy, ExternalInteraction,
    InstanceEpoch, IntegrityConstraintId, MResult, MechError, MechErrorKind, NodeId,
    OutputConstruction, ProgramRevision, ReactiveInstanceId, ResidentKernelError,
    ResidentKernelInputs, ResidentValueKind, ResidentValueMut, ResidentValueRef, SlotIndex, Value,
};

use super::{
    ActivatedExternalNode, ActivatedNodeIndex, ActivatedTurnStep, F64_STATE_ARENA_BASE,
    F64_STATE_SLOT_BIT, F64ReadTapeEntry, ReactiveInstance, ResidentActivationError,
    ResidentEffectIntent, ResidentExternalPublicationAuthority, ResidentIntegrityMode,
    ResidentReadLocation, ResidentRegion, ResidentStorageClass, ResidentValueBorrow, SlotRole,
    StateArena, StateVersion, TypedResidentArena, output_materialization_depends_on_match,
};

// This is a host-stack safety ceiling, not the language's recursion budget.
// Every call is independently admitted through the cumulative resident work
// and frame-memory budget below. Keeping the emergency ceiling modest ensures
// an adversarial call cannot reach the Rust stack limit before admission can
// report a recoverable turn failure.
const MAX_RESIDENT_RECURSION_DEPTH: usize = 24;

#[derive(Clone, Copy, Debug)]
pub struct CapturedSignalInput<'a> {
    pub slot: SlotIndex,
    pub value: ResidentValueRef<'a>,
}

#[derive(Default)]
struct ControlBlockLiveFootprint {
    retained_prefix: usize,
    footprint: ValueFootprint,
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
    MemoryRuntime {
        error: mech_core::MemoryRuntimeError,
    },
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

    /// Creates an owning export from the prepared candidate before any
    /// publication authority is exercised. Loaders use this to include their
    /// initial return value in the same fail-closed transaction.
    #[doc(hidden)]
    pub fn copied_output(&self, output: usize) -> Result<Value, ResidentActivationError> {
        let instance = self
            .instance
            .as_deref()
            .expect("live prepared resident turn");
        if output >= instance.plan.outputs.len() {
            return Err(ResidentActivationError::UnknownOutput { output });
        }
        if !instance
            .workspace
            .candidate_output_ready
            .get(output)
            .copied()
            .unwrap_or(false)
        {
            return Err(ResidentActivationError::OutputUnavailable { output });
        }
        instance.copied_output_at(output, self.working_epoch)
    }

    #[doc(hidden)]
    pub fn output_borrow(&self, output: usize) -> Option<ResidentValueBorrow<'_>> {
        let instance = self
            .instance
            .as_deref()
            .expect("live prepared resident turn");
        if !instance
            .workspace
            .candidate_output_ready
            .get(output)
            .copied()
            .unwrap_or(false)
        {
            return None;
        }
        instance.output_borrow_at(output, self.working_epoch)
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
        instance.publish_continuation_candidates();
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
    /// Resolve invariant lane footprints during activation, before the first
    /// turn. No kernel executes and no state transition is warmed up here.
    pub(super) fn prepare_fixed_turn_plans(&mut self) -> Result<(), ResidentActivationError> {
        for index in 0..self.plan.steps.len() {
            let ActivatedTurnStep::Kernel(node) = &self.plan.steps[index] else {
                continue;
            };
            let artifact_node = node.artifact_node;
            let call = self
                .plan
                .memory_plan
                .call_for_node(node.memory_node)
                .ok_or(ResidentActivationError::InvalidDependency {
                    node: artifact_node,
                })?;
            if super::live::has_invariant_memory_facts(call) {
                self.with_kernel_turn_plan(
                    ActivatedNodeIndex(index as u32),
                    InstanceEpoch::ZERO,
                    InstanceEpoch::ZERO,
                    |_| Ok(()),
                )
                .map_err(|_| ResidentActivationError::ActivationKernel {
                    node: artifact_node,
                })?;
            }
        }
        Ok(())
    }

    fn with_kernel_turn_plan<T>(
        &mut self,
        node_index: ActivatedNodeIndex,
        before_epoch: InstanceEpoch,
        working_epoch: InstanceEpoch,
        execute: impl FnOnce(&mut Self) -> Result<T, ResidentExecutionError>,
    ) -> Result<T, ResidentExecutionError> {
        self.with_kernel_turn_plan_and_live_demand(
            node_index,
            before_epoch,
            working_epoch,
            0,
            0,
            execute,
        )
    }

    fn with_kernel_turn_plan_and_live_demand<T>(
        &mut self,
        node_index: ActivatedNodeIndex,
        before_epoch: InstanceEpoch,
        working_epoch: InstanceEpoch,
        live_bytes: u64,
        live_nodes: u64,
        execute: impl FnOnce(&mut Self) -> Result<T, ResidentExecutionError>,
    ) -> Result<T, ResidentExecutionError> {
        let index = node_index.get() as usize;
        if live_bytes == 0
            && live_nodes == 0
            && let Some(cached) = self.workspace.fixed_turn_plans[index].clone()
        {
            // Reuse the certified base plan, not a materialization permit.
            // The kernel still admits its concrete work and scratch demand.
            return super::super::budget::with_resident_turn_plan(cached, || execute(self));
        }
        let node = self.plan.steps[index]
            .memory_site()
            .expect("executable memory call");
        let artifact_node = node.artifact_node;
        let fail = || ResidentExecutionError::Kernel {
            node: artifact_node,
            error: ResidentKernelError::InvalidShape,
        };
        let output_location = match node.write.storage {
            ResidentStorageClass::State => ResidentReadLocation::State {
                slot: node.write.slot,
                region: node.write.region,
            },
            ResidentStorageClass::Scratch => ResidentReadLocation::Scratch(node.write.region),
            ResidentStorageClass::Input => ResidentReadLocation::Input(node.write.region),
            ResidentStorageClass::Constant => ResidentReadLocation::Constant(node.write.region),
        };
        let current = self
            .read_location(output_location, before_epoch)
            .ok_or_else(fail)?;
        let call = self
            .plan
            .memory_plan
            .call_for_node(node.memory_node)
            .ok_or_else(fail)?;
        let base = match node.construction {
            OutputConstruction::ReadModifyWrite { base_input, .. } => Some(base_input as usize),
            _ => None,
        };
        let region = node.write.region;
        let storage = node.write.storage;
        let candidate = if storage == ResidentStorageClass::State {
            if matches!(
                node.construction,
                OutputConstruction::ReadModifyWrite { .. }
            ) {
                self.state
                    .version(node.write.slot)
                    .epochs
                    .iter()
                    .position(|epoch| *epoch == Some(working_epoch))
                    .unwrap_or_else(|| self.state.candidate_buffer(node.write.slot, before_epoch))
            } else {
                self.state.candidate_buffer(node.write.slot, before_epoch)
            }
        } else {
            0
        };
        let target = match storage {
            ResidentStorageClass::State => &self.state.buffers[candidate],
            ResidentStorageClass::Scratch => &self.workspace.scratch,
            ResidentStorageClass::Input => &self.workspace.input,
            ResidentStorageClass::Constant => &self.activation,
        };
        let scope = target
            .prepare_payload_write(region)
            .and_then(|scope| {
                if scope.is_some() {
                    Ok(scope)
                } else {
                    self.transient_budget
                        .as_ref()
                        .map(|owner| owner.begin(region))
                        .transpose()
                }
            })
            .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?;
        if let Some(scope) = &scope {
            let container = u64::try_from(call.inputs.len())
                .ok()
                .and_then(|inputs| {
                    inputs.checked_mul(core::mem::size_of::<ResidentValueRef<'_>>() as u64)
                })
                .ok_or_else(fail)?;
            scope
                .admit_auxiliary(container)
                .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?;
        }
        let mut reads = self.plan.reads[node.reads.start as usize..node.reads.end as usize].iter();
        let inputs = (0..call.inputs.len())
            .map(|ordinal| {
                let location = if Some(ordinal) == base {
                    node.rmw_base.unwrap_or(output_location)
                } else {
                    *reads.next().ok_or_else(fail)?
                };
                self.read_location(location, working_epoch).ok_or_else(fail)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut facts =
            super::live::facts(call, node.memory_node, &inputs, current, &self.plan.schemas)
                .map_err(|_| fail())?;
        facts.additional_demand.turn_peak_bytes = facts
            .additional_demand
            .turn_peak_bytes
            .checked_add(live_bytes)
            .ok_or_else(fail)?;
        facts.additional_demand.retained_nodes = facts
            .additional_demand
            .retained_nodes
            .checked_add(live_nodes)
            .ok_or_else(fail)?;
        let turn_plan = crate::memory_planner::plan_current_resident_turn(
            &self.plan.memory_plan,
            node.memory_node,
            &facts,
        )
        .map_err(|_| ResidentExecutionError::Kernel {
            node: artifact_node,
            error: ResidentKernelError::InvalidShape,
        })?;
        if !turn_plan.budget_violations.is_empty() {
            return Err(ResidentExecutionError::Kernel {
                node: artifact_node,
                error: ResidentKernelError::InvalidShape,
            });
        }
        let cacheable =
            live_bytes == 0 && live_nodes == 0 && super::live::has_invariant_memory_facts(call);
        let has_canonical_input = inputs
            .iter()
            .any(|input| input.kind() == ResidentValueKind::Snapshot);
        drop(inputs);
        let turn_plan = std::sync::Arc::new(turn_plan);
        if cacheable {
            self.workspace.fixed_turn_plans[index] = Some(turn_plan.clone());
            // The same invariant proof that permits steady-state reuse also
            // proves there is no indirect payload to admit on this first
            // call. In particular, activation's cache preparation executes
            // no kernel and must not manufacture a transient payload scope.
            return super::super::budget::with_resident_turn_plan(turn_plan, || execute(self));
        }
        if let Some(scope) = &scope {
            if region.kind == ResidentValueKind::Snapshot || has_canonical_input {
                let bytes = self
                    .plan
                    .schemas
                    .clone_allocation_bound_bytes()
                    .and_then(|bytes| bytes.checked_mul(2))
                    .ok_or_else(fail)?;
                scope
                    .admit_auxiliary(bytes)
                    .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?;
            }
        }
        let admission = scope.as_ref().map(|scope| scope.admission());
        let result = super::super::budget::with_payload_admission(admission, || {
            super::super::budget::with_resident_turn_plan(turn_plan, || execute(self))
        });
        let admission_error = scope.as_ref().and_then(|scope| scope.last_error());
        let target = match storage {
            ResidentStorageClass::State => &mut self.state.buffers[candidate],
            ResidentStorageClass::Scratch => &mut self.workspace.scratch,
            ResidentStorageClass::Input => &mut self.workspace.input,
            ResidentStorageClass::Constant => &mut self.activation,
        };
        if result.is_err() || admission_error.is_some() {
            target
                .abort_payload_write(region, scope)
                .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?;
        } else {
            target
                .finish_payload_write(region, scope)
                .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?;
        }
        if let Some(error) = admission_error {
            return Err(ResidentExecutionError::MemoryRuntime { error });
        }
        result
    }

    /// Hash the currently published resident state without beginning a candidate.
    pub fn published_state_hash(&self) -> u64 {
        state_hash(self, self.published_epoch())
    }

    /// Recompute selected materialized outputs from the currently published
    /// state without executing state transitions or staging external effects.
    /// Replacement runtimes use this after migrating compatible live cells so
    /// every non-migrated projection describes the installed state epoch.
    pub fn refresh_output_projections(
        &mut self,
        artifact: &crate::ProgramArtifact,
        targets: &std::collections::BTreeSet<CellSlotId>,
    ) -> Result<(), ResidentExecutionError> {
        if targets.is_empty() {
            return Ok(());
        }
        if self.candidate_active {
            return Err(ResidentExecutionError::ActiveCandidate);
        }
        for target in targets {
            if self
                .plan
                .slots
                .get(target.get() as usize)
                .is_none_or(|slot| slot.role != SlotRole::Output)
                || !self
                    .plan
                    .output_materializations
                    .iter()
                    .any(|materialization| materialization.target == *target)
            {
                return Err(ResidentExecutionError::InvalidOutputMaterialization { slot: *target });
            }
        }

        if artifact.revision() != self.plan.program_revision {
            return Err(ResidentExecutionError::InvalidOutputMaterialization {
                slot: *targets.first().expect("nonempty targets checked"),
            });
        }
        let epoch = self.published_epoch();
        // A canonical state writer copies its final candidate into the retained
        // cell. During projection refresh that candidate means the published
        // state value: recomputing it would execute the transition a second time.
        // Derive this relation from the accepted artifact's identity operation,
        // not from source names or an independent evaluation graph.
        let mut published_candidates = std::collections::BTreeMap::new();
        for state in artifact
            .slots()
            .iter()
            .filter(|slot| slot.role == SlotRole::State)
        {
            let crate::ProducerReference::NodeOutput { node, .. } = state.producer else {
                continue;
            };
            let Some(writer) = artifact.nodes()[node.get() as usize].as_operation() else {
                continue;
            };
            if writer.operation.module_path.as_ref() != ["core"]
                || writer.operation.operation_name != "assign"
            {
                continue;
            }
            let [
                crate::BindingDeclaration::Input {
                    source: crate::ArtifactSource::Slot(source),
                    ..
                },
            ] = &artifact.bindings()
                [writer.input_bindings.start as usize..writer.input_bindings.end as usize]
            else {
                continue;
            };
            let source_slot = &artifact.slots()[source.get() as usize];
            let crate::ProducerReference::NodeOutput { node: producer, .. } = source_slot.producer
            else {
                continue;
            };
            if source_slot.role != SlotRole::Derived
                || self.plan.slots[source.get() as usize].storage != ResidentStorageClass::Scratch
            {
                continue;
            }
            if let Some(previous) = published_candidates.insert(producer, state.slot) {
                let previous_region = self.plan.slots[previous.get() as usize].region;
                let region = self.plan.slots[state.slot.get() as usize].region;
                if !rmw_outputs_equal(
                    &self.state.buffers[self.state.published_buffer(previous, epoch)],
                    previous_region,
                    &self.state.buffers[self.state.published_buffer(state.slot, epoch)],
                    region,
                    &self.plan.schemas,
                ) {
                    return Err(ResidentExecutionError::InvalidOutputMaterialization {
                        slot: state.slot,
                    });
                }
            }
        }
        self.refresh_f64_state_arenas(epoch);
        let execution_order = self.plan.execution_node_order.to_vec();
        let mut probe = ResidentStructuralProbe::default();
        for node_index in execution_order {
            let execute = matches!(
                &self.plan.steps[node_index.get() as usize],
                ActivatedTurnStep::Kernel(node)
                    if node.write.storage == ResidentStorageClass::Scratch
            );
            if execute {
                let ActivatedTurnStep::Kernel(node) = &self.plan.steps[node_index.get() as usize]
                else {
                    unreachable!("projection refresh selects only kernels");
                };
                if let Some(source) = published_candidates.get(&node.artifact_node) {
                    let source_region = self.plan.slots[source.get() as usize].region;
                    let source_buffer = self.state.published_buffer(*source, epoch);
                    self.workspace
                        .scratch
                        .copy_region_from(
                            node.write.region,
                            &self.state.buffers[source_buffer],
                            source_region,
                        )
                        .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?;
                } else {
                    self.execute_kernel(node_index, epoch, epoch, &mut probe)?;
                }
            }
        }

        let materializations = self.plan.output_materializations.to_vec();
        let staged = (|| {
            for materialization in &materializations {
                if !targets.contains(&materialization.target) {
                    continue;
                }
                let target = materialization.target;
                let target_region = self.plan.slots[target.get() as usize].region;
                match materialization.source {
                    ResidentReadLocation::Constant(source) => {
                        self.state.stage_projection_from_arena(
                            target,
                            target_region,
                            epoch,
                            &self.activation,
                            source,
                        )
                    }
                    ResidentReadLocation::Input(source)
                    | ResidentReadLocation::LexicalInput(source) => {
                        self.state.stage_projection_from_arena(
                            target,
                            target_region,
                            epoch,
                            &self.workspace.input,
                            source,
                        )
                    }
                    ResidentReadLocation::Scratch(source) => {
                        self.state.stage_projection_from_arena(
                            target,
                            target_region,
                            epoch,
                            &self.workspace.scratch,
                            source,
                        )
                    }
                    ResidentReadLocation::State {
                        slot: source_slot,
                        region: source_region,
                    } => self.state.stage_projection_from_state_slot(
                        target,
                        target_region,
                        source_slot,
                        source_region,
                        epoch,
                    ),
                }
                .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?;
            }
            self.validate_constraints_with_projection_candidates(epoch, Some(targets))
        })();
        staged?;
        for materialization in &materializations {
            let target = materialization.target;
            if targets.contains(&target) {
                let candidate = self.state.candidate_buffer(target, epoch);
                let mut epochs = [None, None];
                epochs[candidate] = Some(epoch);
                self.state.version_mut(target).epochs = epochs;
            }
        }
        Ok(())
    }

    fn external_node(&self, ordinal: u32) -> Option<&ActivatedExternalNode> {
        self.plan.steps.iter().find_map(|step| match step {
            ActivatedTurnStep::External(node) if node.effect_ordinal == ordinal => Some(node),
            _ => None,
        })
    }

    pub fn materialize_effect_payload(&self, ordinal: u32) -> MResult<Value> {
        if self.candidate_epoch.is_none() {
            return Err(MechError::new(ResidentEffectPayloadError { ordinal }, None));
        }
        let external = self
            .external_node(ordinal)
            .ok_or_else(|| MechError::new(ResidentEffectPayloadError { ordinal }, None))?;
        if !self
            .workspace
            .effect_intents
            .iter()
            .any(|intent| intent.ordinal == ordinal)
        {
            return Err(MechError::new(ResidentEffectPayloadError { ordinal }, None));
        }
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
            self.memory_budget().as_ref(),
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
        if !self.plan.has_only_kernel_steps() {
            return self.prepare_turn(inputs)?.publish().map(|_| ());
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
                ActivatedTurnStep::External(_)
                | ActivatedTurnStep::Match(_)
                | ActivatedTurnStep::Recur(_)
                | ActivatedTurnStep::Suspend(_)
                | ActivatedTurnStep::Publish(_)
                | ActivatedTurnStep::Comprehension(_) => None,
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
        // Kernel-level payload admission becomes durable only with the whole
        // turn. Drop every payload written by completed steps before erasing
        // their epoch/execution evidence so a failed turn cannot consume the
        // next turn's exact candidate headroom.
        self.state.abort_payloads(working_epoch);
        for index in 0..self.plan.topology.linear_node_order.len() {
            if !bit_is_set(&self.workspace.executed_bits, index) {
                continue;
            }
            let (scratch, effect) = match &self.plan.steps[index] {
                ActivatedTurnStep::Kernel(node) => (
                    (node.write.storage == ResidentStorageClass::Scratch)
                        .then_some(node.write.region),
                    None,
                ),
                ActivatedTurnStep::External(node) => (None, Some(node.captured_payload)),
                ActivatedTurnStep::Match(node) => (Some(node.write.region), None),
                ActivatedTurnStep::Recur(node) => (Some(node.write.region), None),
                ActivatedTurnStep::Suspend(_) => (None, None),
                ActivatedTurnStep::Publish(publication) => {
                    let region = match &self.plan.steps[publication.target.get() as usize] {
                        ActivatedTurnStep::Match(control) => Some(control.write.region),
                        _ => None,
                    };
                    (region, None)
                }
                ActivatedTurnStep::Comprehension(node) => (Some(node.write.region), None),
            };
            if let Some(region) = scratch {
                self.workspace.scratch.discard_payload_write(region);
            }
            if let Some(region) = effect {
                self.workspace.effect_payloads.discard_payload_write(region);
            }
        }
        for (index, step) in self.plan.steps.iter().enumerate() {
            if bit_is_set(&self.workspace.continuation_publications, index)
                && let ActivatedTurnStep::Match(control) = step
            {
                self.workspace
                    .scratch
                    .discard_payload_write(control.write.region);
            }
        }
        for step in &self.plan.steps {
            if let ActivatedTurnStep::Kernel(node) = step
                && let Some(previous) = node.rmw_previous
            {
                self.workspace.rmw_previous.discard_payload_write(previous);
            }
        }
        self.state.abort(working_epoch);
        self.workspace.initialized_output_bits.fill(0);
        self.workspace.all_outputs_initialized = false;
        self.workspace.effect_intents.clear();
        self.workspace.continuation_candidates.fill(None);
        self.workspace.completed_continuations.fill(0);
        self.workspace.continuation_publications.fill(0);
        self.workspace.continuation_capture_frames.clear();
        self.workspace.active_resume_state = None;
        self.workspace
            .candidate_output_ready
            .clone_from(&self.output_ready);
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
                copy_input(&mut self.workspace.input, declared.region, input.value).map_err(
                    |error| error.at(ResidentExecutionError::InputLayout { slot: input.slot }),
                )?;
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
                copy_input(&mut self.workspace.input, declared.region, input.value).map_err(
                    |error| error.at(ResidentExecutionError::InputLayout { slot: input.slot }),
                )?;
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
            .map_err(|error| match error {
                ResidentActivationError::MemoryRuntime { error } => {
                    ResidentExecutionError::MemoryRuntime { error }
                }
                _ => ResidentExecutionError::InputLayout { slot: input.slot },
            })?;
        }
        Ok(())
    }

    fn begin_scheduler_workspace(&mut self) {
        self.workspace.executed_bits.fill(0);
        self.workspace.touched_slots.clear();
        self.workspace.changed_slots.clear();
        self.workspace.effect_intents.clear();
        self.workspace.continuation_candidates.fill(None);
        self.workspace.completed_continuations.fill(0);
        self.workspace.continuation_publications.fill(0);
        self.workspace.continuation_capture_frames.clear();
        self.workspace.active_resume_state = None;
        self.workspace
            .candidate_output_ready
            .clone_from(&self.output_ready);
        self.seed_dirty_bits();
    }

    fn publish_continuation_candidates(&mut self) {
        for index in 0..self.plan.steps.len() {
            let node = ActivatedNodeIndex(index as u32);
            if bit_is_set(&self.workspace.continuation_publications, index) {
                set_bit(&mut self.published_continuations, index);
            }
            if bit_is_set(&self.workspace.completed_continuations, index) {
                self.continuations[index] = None;
                self.ready_continuations.retain(|ready| *ready != node);
            }
            if let Some(value) = self.workspace.continuation_candidates[index].take() {
                self.continuations[index] = Some(value);
                self.ready_continuations.retain(|ready| *ready != node);
                self.ready_continuations.push_back(node);
            }
        }
        self.output_ready
            .clone_from(&self.workspace.candidate_output_ready);
        self.workspace.completed_continuations.fill(0);
        self.workspace.continuation_publications.fill(0);
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
        let mut executed = 0_u64;
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
            executed |= entry.node_bit;
            self.workspace.executed_bits[0] = executed;
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
        if self.plan.has_only_kernel_steps() {
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
                self.workspace.executed_bits[0] = executed;
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

    /// Preserve the recorded-turn lane for plans whose activation proves
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
                self.workspace.executed_bits[0] = executed;
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

    fn unpublished_continuation(&self, index: usize) -> bool {
        matches!(
            self.plan.steps.get(index),
            Some(ActivatedTurnStep::Match(control))
                if control.continuation
                    && !bit_is_set(&self.workspace.continuation_publications, index)
                    && !bit_is_set(&self.published_continuations, index)
                    && (self.workspace.continuation_candidates[index].is_some()
                        || (self.continuations[index].is_some()
                            && !bit_is_set(&self.workspace.completed_continuations, index)))
        )
    }

    fn materialize_outputs(
        &mut self,
        before_epoch: InstanceEpoch,
        working_epoch: InstanceEpoch,
        record_summary: bool,
    ) -> Result<(), ResidentExecutionError> {
        for index in 0..self.plan.output_materializations.len() {
            let materialization = self.plan.output_materializations[index];
            let suspended = self.plan.steps.iter().enumerate().any(|(step, node)| {
                self.unpublished_continuation(step)
                    && matches!(node, ActivatedTurnStep::Match(control)
                    if output_materialization_depends_on_match(
                        &self.plan,
                        materialization,
                        step,
                        control.write.region,
                    ))
            });
            if suspended {
                continue;
            }
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
                    .map_err(|error| {
                        error.at(ResidentExecutionError::InvalidOutputMaterialization {
                            slot: target,
                        })
                    })?;
                    self.state.tag(target, candidate, working_epoch);
                }
                ResidentReadLocation::Input(source)
                | ResidentReadLocation::LexicalInput(source) => {
                    let candidate = self.state.candidate_buffer(target, before_epoch);
                    let value = if matches!(
                        materialization.source,
                        ResidentReadLocation::LexicalInput(_)
                    ) {
                        lexical_capture(&self.workspace.continuation_capture_frames, source)
                            .map(super::OwnedResidentValue::as_ref)
                            .unwrap_or_else(|| self.workspace.input.read(source))
                    } else {
                        self.workspace.input.read(source)
                    };
                    copy_input(&mut self.state.buffers[candidate], target_region, value).map_err(
                        |error| {
                            error.at(ResidentExecutionError::InvalidOutputMaterialization {
                                slot: target,
                            })
                        },
                    )?;
                    self.state.tag(target, candidate, working_epoch);
                }
                ResidentReadLocation::Scratch(source) => {
                    let candidate = self.state.candidate_buffer(target, before_epoch);
                    copy_input(
                        &mut self.state.buffers[candidate],
                        target_region,
                        self.workspace.scratch.read(source),
                    )
                    .map_err(|error| {
                        error.at(ResidentExecutionError::InvalidOutputMaterialization {
                            slot: target,
                        })
                    })?;
                    self.state.tag(target, candidate, working_epoch);
                }
                ResidentReadLocation::State {
                    slot: source_slot,
                    region: source_region,
                } => self
                    .state
                    .materialize_state_slot(
                        target,
                        target_region,
                        source_slot,
                        source_region,
                        before_epoch,
                        working_epoch,
                    )
                    .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?,
            }
            if record_summary {
                let slot = SlotIndex::new(target.get());
                self.workspace.touched_slots.push(slot);
                let candidate = self.state.select_buffer(target, working_epoch);
                if !self.state.same_at(target, candidate, before_epoch) {
                    self.workspace.changed_slots.push(slot);
                }
            }
            let physical_target = self.plan.slots[target.get() as usize].physical_index;
            for (output, ready) in self
                .plan
                .outputs
                .iter()
                .zip(self.workspace.candidate_output_ready.iter_mut())
            {
                if output.slot == physical_target {
                    *ready = true;
                }
            }
        }
        Ok(())
    }

    fn validate_constraints(
        &self,
        working_epoch: InstanceEpoch,
    ) -> Result<(), ResidentExecutionError> {
        self.validate_constraints_with_projection_candidates(working_epoch, None)
    }

    fn validate_constraints_with_projection_candidates(
        &self,
        working_epoch: InstanceEpoch,
        staged_outputs: Option<&std::collections::BTreeSet<CellSlotId>>,
    ) -> Result<(), ResidentExecutionError> {
        if self.plan.integrity_mode == ResidentIntegrityMode::Unchecked {
            return Ok(());
        }
        for constraint in &self.plan.constraints {
            let unpublished = self.plan.steps.iter().enumerate().any(|(index, step)| {
                if !self.unpublished_continuation(index) {
                    return false;
                }
                let ActivatedTurnStep::Match(control) = step else {
                    return false;
                };
                let source = match constraint.predicate {
                    ResidentReadLocation::State { slot, .. } => self
                        .plan
                        .output_materializations
                        .iter()
                        .find(|materialization| materialization.target == slot)
                        .map_or(constraint.predicate, |materialization| {
                            materialization.source
                        }),
                    source => source,
                };
                super::read_location_depends_on_match(
                    &self.plan,
                    source,
                    index,
                    control.write.region,
                )
            });
            if unpublished {
                continue;
            }
            let predicate = match constraint.predicate {
                ResidentReadLocation::State { slot, region }
                    if staged_outputs.is_some_and(|outputs| outputs.contains(&slot)) =>
                {
                    let buffer = self.state.candidate_buffer(slot, working_epoch);
                    Some(self.state.buffers[buffer].read(region))
                }
                location => self.read_location(location, working_epoch),
            };
            let Some(ResidentValueRef::Bool(predicate)) = predicate else {
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
        self.with_kernel_turn_plan(node_index, before_epoch, working_epoch, |this| {
            this.execute_kernel_without_summary_planned(
                node_index,
                before_epoch,
                working_epoch,
                change_is_observed,
            )
        })
    }

    #[inline(always)]
    fn execute_kernel_without_summary_planned(
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
            return self.execute_kernel_planned(
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
                let candidate = self
                    .state
                    .begin_rmw(slot, before_epoch, working_epoch)
                    .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?
                    .0;
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
                captures: &self.workspace.continuation_capture_frames,
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
                    captures: &self.workspace.continuation_capture_frames,
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
    pub(super) fn execute_activation_control(
        &mut self,
        index: ActivatedNodeIndex,
    ) -> Result<(), ResidentExecutionError> {
        let mut probe = ResidentStructuralProbe::default();
        self.execute_step(index, InstanceEpoch::ZERO, InstanceEpoch::ZERO, &mut probe)?;
        Ok(())
    }

    pub(super) fn execute_step(
        &mut self,
        node_index: ActivatedNodeIndex,
        before_epoch: InstanceEpoch,
        working_epoch: InstanceEpoch,
        probe: &mut ResidentStructuralProbe,
    ) -> Result<bool, ResidentExecutionError> {
        self.execute_step_with_live_demand(node_index, before_epoch, working_epoch, probe, 0, 0)
    }

    pub(super) fn execute_step_with_live_demand(
        &mut self,
        node_index: ActivatedNodeIndex,
        before_epoch: InstanceEpoch,
        working_epoch: InstanceEpoch,
        probe: &mut ResidentStructuralProbe,
        live_bytes: u64,
        live_nodes: u64,
    ) -> Result<bool, ResidentExecutionError> {
        if matches!(
            self.plan.steps[node_index.get() as usize],
            ActivatedTurnStep::Comprehension(_)
        ) {
            if live_bytes == 0 && live_nodes == 0 {
                return self.execute_comprehension(node_index, before_epoch, working_epoch, probe);
            }
            return self.execute_comprehension_with_live_demand(
                node_index,
                before_epoch,
                working_epoch,
                probe,
                live_bytes,
                live_nodes,
            );
        }
        if matches!(
            self.plan.steps[node_index.get() as usize],
            ActivatedTurnStep::Match(_)
        ) {
            let ActivatedTurnStep::Match(matched) = &self.plan.steps[node_index.get() as usize]
            else {
                unreachable!()
            };
            if matched.continuation
                && self.continuations[node_index.get() as usize].is_some()
                && self.ready_continuations.front().copied() != Some(node_index)
            {
                // One drain resumes only the selected continuation. Other
                // ready roots keep their state and lexical captures intact.
                return Ok(false);
            }
            let node = matched.artifact_node;
            let mut facts = crate::memory_planner::TurnMemoryFacts::default();
            facts.additional_demand.turn_peak_bytes = live_bytes;
            facts.additional_demand.retained_nodes = live_nodes;
            let turn_plan = crate::memory_planner::plan_current_resident_turn(
                &self.plan.memory_plan,
                matched.budget_node,
                &facts,
            )
            .map_err(|_| ResidentExecutionError::Kernel {
                node,
                error: ResidentKernelError::InvalidShape,
            })?;
            if !turn_plan.budget_violations.is_empty() {
                return Err(ResidentExecutionError::Kernel {
                    node,
                    error: ResidentKernelError::InvalidShape,
                });
            }
            let resuming = self.ready_continuations.front().copied() == Some(node_index)
                && self.continuations[node_index.get() as usize].is_some();
            if resuming {
                self.workspace
                    .continuation_capture_frames
                    .try_reserve(1)
                    .map_err(|_| ResidentExecutionError::Kernel {
                        node,
                        error: ResidentKernelError::InvalidShape,
                    })?;
                let saved = self.continuations[node_index.get() as usize]
                    .take()
                    .expect("selected continuation remains available");
                self.workspace.active_resume_state = Some(saved.state);
                self.workspace
                    .continuation_capture_frames
                    .push(saved.captures);
            }
            let result = budget::with_resident_turn_plan(turn_plan, || {
                let result = budget::with_control_work_budget(|| {
                    self.execute_match_expression(
                        node_index,
                        before_epoch,
                        working_epoch,
                        probe,
                        live_bytes,
                        live_nodes,
                    )
                });
                let ActivatedTurnStep::Match(control) = &self.plan.steps[node_index.get() as usize]
                else {
                    unreachable!()
                };
                // The selected yield has been copied into its enclosing result.
                // No block-local payload survives success, failure or a later abort.
                for region in &control.locals {
                    self.workspace.scratch.discard_payload_write(*region);
                }
                result
            });
            if resuming {
                let captures = self
                    .workspace
                    .continuation_capture_frames
                    .pop()
                    .expect("selected continuation capture frame remains present");
                let state = self
                    .workspace
                    .active_resume_state
                    .take()
                    .expect("selected continuation state remains present");
                self.continuations[node_index.get() as usize] =
                    Some(super::ResidentContinuation { state, captures });
            }
            return result;
        }
        if let ActivatedTurnStep::Recur(call) = self.plan.steps[node_index.get() as usize] {
            return self.execute_recursive_call(
                node_index,
                call,
                before_epoch,
                working_epoch,
                probe,
                live_bytes,
                live_nodes,
            );
        }
        if let ActivatedTurnStep::Suspend(suspension) = self.plan.steps[node_index.get() as usize] {
            let target = suspension.target.get() as usize;
            let capture_sources = match &self.plan.steps[target] {
                ActivatedTurnStep::Match(control) => control.capture_sources.clone(),
                _ => {
                    return Err(ResidentExecutionError::Kernel {
                        node: suspension.artifact_node,
                        error: ResidentKernelError::InvalidInput,
                    });
                }
            };
            let fail = |error| ResidentExecutionError::Kernel {
                node: suspension.artifact_node,
                error,
            };
            let mut capture_meter = budget::ResidentBudgetMeter::default();
            let (capture_bytes, capture_nodes) = core::iter::once(suspension.argument)
                .chain(
                    capture_sources
                        .iter()
                        .copied()
                        .filter(|source| !matches!(source, ResidentReadLocation::Input(_))),
                )
                .try_fold((0_u64, 0_u64), |(bytes, nodes), source| {
                    let value = self.read_location(source, working_epoch)?;
                    let footprint = resident_frame_value_footprint(
                        value,
                        &self.plan.schemas,
                        &mut capture_meter,
                    )?;
                    Some((
                        bytes.checked_add(footprint.0)?,
                        nodes.checked_add(footprint.1)?,
                    ))
                })
                .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
            let capture_work = capture_meter
                .estimate()
                .compute_work()
                .checked_add(capture_bytes)
                .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
            (|| -> Result<(), ResidentKernelError> {
                budget::PreparedKernel::new(
                    (),
                    budget::resident_cost! {
                        compute_work: capture_work,
                        temporary_bytes: capture_bytes,
                        cloned_bytes: capture_bytes,
                        retained_nodes: capture_nodes,
                        ..budget::KernelCostEstimate::default()
                    },
                )
                .admit_control()?
                .into_plan();
                Ok(())
            })()
            .map_err(fail)?;
            let state = self
                .read_location(suspension.argument, working_epoch)
                .map(owned_resident_value)
                .ok_or_else(|| fail(ResidentKernelError::InvalidInput))?;
            let captures = capture_sources
                .iter()
                .copied()
                .filter(|source| !matches!(source, ResidentReadLocation::Input(_)))
                .map(|source| {
                    self.read_location(source, working_epoch)
                        .map(|value| (source, owned_resident_value(value)))
                        .ok_or(ResidentExecutionError::Kernel {
                            node: suspension.artifact_node,
                            error: ResidentKernelError::InvalidInput,
                        })
                })
                .collect::<Result<Box<[_]>, _>>()?;
            self.workspace.continuation_candidates[target] =
                Some(super::ResidentContinuation { state, captures });
            clear_bit(&mut self.workspace.completed_continuations, target);
            return Ok(false);
        }
        if let ActivatedTurnStep::Publish(publication) = self.plan.steps[node_index.get() as usize]
        {
            let fail = |error| ResidentExecutionError::Kernel {
                node: publication.artifact_node,
                error,
            };
            let mut publication_meter = budget::ResidentBudgetMeter::default();
            let (bytes, nodes) = self
                .read_location(publication.value, working_epoch)
                .and_then(|value| {
                    resident_frame_value_footprint(
                        value,
                        &self.plan.schemas,
                        &mut publication_meter,
                    )
                })
                .ok_or_else(|| fail(ResidentKernelError::InvalidInput))?;
            let cloned_bytes = bytes
                .checked_mul(2)
                .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
            let retained_nodes = nodes
                .checked_mul(2)
                .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
            let publication_work = publication_meter
                .estimate()
                .compute_work()
                .checked_add(cloned_bytes)
                .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
            (|| -> Result<(), ResidentKernelError> {
                budget::PreparedKernel::new(
                    (),
                    budget::resident_cost! {
                        compute_work: publication_work,
                        temporary_bytes: cloned_bytes,
                        cloned_bytes,
                        retained_nodes,
                        ..budget::KernelCostEstimate::default()
                    },
                )
                .admit_control()?
                .into_plan();
                Ok(())
            })()
            .map_err(fail)?;
            let value = self
                .read_location(publication.value, working_epoch)
                .map(owned_resident_value)
                .ok_or_else(|| fail(ResidentKernelError::InvalidInput))?;
            let target = publication.target.get() as usize;
            let write = match &self.plan.steps[target] {
                ActivatedTurnStep::Match(control)
                    if control.write.storage == ResidentStorageClass::Scratch =>
                {
                    control.write
                }
                _ => {
                    return Err(ResidentExecutionError::Kernel {
                        node: publication.artifact_node,
                        error: ResidentKernelError::InvalidInput,
                    });
                }
            };
            let unchanged =
                resident_values_equal(self.workspace.scratch.read(write.region), value.as_ref());
            copy_input(&mut self.workspace.scratch, write.region, value.as_ref()).map_err(
                |error| {
                    error.at(ResidentExecutionError::Kernel {
                        node: publication.artifact_node,
                        error: ResidentKernelError::InvalidOutput,
                    })
                },
            )?;
            set_bit(&mut self.workspace.continuation_publications, target);
            return Ok(!unchanged);
        }
        if matches!(
            self.plan.steps[node_index.get() as usize],
            ActivatedTurnStep::External(_)
        ) {
            self.stage_external(node_index, working_epoch)?;
            return Ok(false);
        }
        self.execute_kernel_with_live_demand(
            node_index,
            before_epoch,
            working_epoch,
            probe,
            live_bytes,
            live_nodes,
        )
    }

    fn execute_match_expression(
        &mut self,
        node_index: ActivatedNodeIndex,
        before_epoch: InstanceEpoch,
        working_epoch: InstanceEpoch,
        probe: &mut ResidentStructuralProbe,
        live_bytes: u64,
        live_nodes: u64,
    ) -> Result<bool, ResidentExecutionError> {
        let resume_state = self.workspace.active_resume_state.take();
        let result = self.execute_match_expression_with_state(
            node_index,
            before_epoch,
            working_epoch,
            probe,
            live_bytes,
            live_nodes,
            resume_state.as_ref(),
        );
        self.workspace.active_resume_state = resume_state;
        result
    }

    fn execute_match_expression_with_state(
        &mut self,
        node_index: ActivatedNodeIndex,
        before_epoch: InstanceEpoch,
        working_epoch: InstanceEpoch,
        probe: &mut ResidentStructuralProbe,
        live_bytes: u64,
        live_nodes: u64,
        resume_state: Option<&super::OwnedResidentValue>,
    ) -> Result<bool, ResidentExecutionError> {
        let index = node_index.get() as usize;
        let ActivatedTurnStep::Match(matched) = &self.plan.steps[index] else {
            unreachable!()
        };
        let node = matched.artifact_node;
        let budget_node = matched.budget_node;
        let write = matched.write;
        let continuation = matched.continuation;
        let arm_count = matched.arms.len();
        let fail = || ResidentExecutionError::Kernel {
            node,
            error: ResidentKernelError::InvalidInput,
        };
        let kernel_fail = |error| ResidentExecutionError::Kernel { node, error };
        let recursive_scrutinee = self
            .workspace
            .recursive_scrutinees
            .iter()
            .rev()
            .find_map(|frame| (frame.target == node_index).then_some(frame.argument));
        let scrutinee_source = recursive_scrutinee.unwrap_or(matched.scrutinee);
        let continuation_scrutinee = if recursive_scrutinee.is_none()
            && matched.continuation
            && self.ready_continuations.front().copied() == Some(node_index)
        {
            resume_state
        } else {
            None
        };
        let scrutinee_schema = matched.scrutinee_schema;
        let scrutinee_shape_values = matched.scrutinee_shape_values.clone();
        let structural_work = matched.arms.iter().try_fold(0u64, |total, arm| {
            let super::ActivatedMatchPattern::Structural { work, .. } = &arm.pattern else {
                return Some(total);
            };
            total.checked_add(*work)
        });
        // Arms are attempted in order and release their descent copies before
        // the next arm. Only the deepest arm's copies can coexist.
        let structural_clone_multiplicity = peak_structural_clone_depth(&matched.arms);
        let structural_dynamic_target_depth = matched
            .arms
            .iter()
            .try_fold(0u64, |depth, arm| -> Result<u64, ResidentKernelError> {
                let super::ActivatedMatchPattern::Structural { pattern, .. } = &arm.pattern else {
                    return Ok(depth);
                };
                Ok(
                    depth.max(comprehension_execution::pattern_dynamic_target_depth(
                        pattern,
                        &self.plan.schemas,
                    )?),
                )
            })
            .map_err(kernel_fail)?;
        let structural_binding_count = matched.arms.iter().try_fold(0u64, |total, arm| {
            let super::ActivatedMatchPattern::Structural { binding_count, .. } = &arm.pattern
            else {
                return Some(total);
            };
            total.checked_add(*binding_count)
        });
        let structural_equality_count = matched.arms.iter().try_fold(0u64, |total, arm| {
            let super::ActivatedMatchPattern::Structural { equality_count, .. } = &arm.pattern
            else {
                return Some(total);
            };
            total.checked_add(*equality_count)
        });
        let structural_snapshot_finalization_count =
            matched.arms.iter().try_fold(0u64, |total, arm| {
                let super::ActivatedMatchPattern::Structural {
                    snapshot_finalization_count,
                    ..
                } = &arm.pattern
                else {
                    return Some(total);
                };
                total.checked_add(*snapshot_finalization_count)
            });
        let mut structural_scrutinee = None;
        // This scope outlives every selected guard and body step. Repeated
        // measurements of managed locals must consume cumulative control work.
        let mut local_meter = budget::ResidentBudgetMeter::default();
        for arm_index in 0..arm_count {
            let ActivatedTurnStep::Match(matched) = &self.plan.steps[index] else {
                unreachable!()
            };
            let arm = &matched.arms[arm_index];
            let pattern = arm.pattern.clone();
            let binding_regions = arm.binding_regions.clone();
            let guard_regions = arm.guard_regions.clone();
            let guard = arm.guard.clone();
            let body = arm.body.clone();
            let pattern_matches = match pattern {
                super::ActivatedMatchPattern::Literal(literal) => {
                    let scrutinee = continuation_scrutinee
                        .as_ref()
                        .map(|value| value.as_ref())
                        .or_else(|| self.read_location(scrutinee_source, working_epoch))
                        .ok_or_else(fail)?;
                    match (
                        scrutinee,
                        self.read_location(literal, working_epoch)
                            .ok_or_else(fail)?,
                    ) {
                        (
                            ResidentValueRef::Bool([left @ (0 | 1)]),
                            ResidentValueRef::Bool([right @ (0 | 1)]),
                        ) => left == right,
                        (ResidentValueRef::Index([left]), ResidentValueRef::Index([right])) => {
                            left == right
                        }
                        (ResidentValueRef::F64([left]), ResidentValueRef::F64([right])) => {
                            left == right
                        }
                        (
                            ResidentValueRef::Snapshot([Some(left)]),
                            ResidentValueRef::Snapshot([Some(right)]),
                        ) => {
                            let left_schema = left
                                .validate_against(&self.plan.schemas)
                                .map_err(|_| fail())?;
                            let right_schema = right
                                .validate_against(&self.plan.schemas)
                                .map_err(|_| fail())?;
                            if !crate::is_control_scalar_schema(left_schema)
                                || left_schema != right_schema
                            {
                                return Err(fail());
                            }
                            left.language_eq(&self.plan.schemas, right, &self.plan.schemas)
                                .map_err(|_| fail())?
                        }
                        _ => return Err(fail()),
                    }
                }
                super::ActivatedMatchPattern::Wildcard | super::ActivatedMatchPattern::Bind => true,
                super::ActivatedMatchPattern::Structural { pattern, .. } => {
                    if structural_scrutinee.is_none() {
                        let work = structural_work
                            .ok_or_else(|| kernel_fail(ResidentKernelError::InvalidShape))?;
                        let clone_multiplicity = structural_clone_multiplicity;
                        let dynamic_target_depth = structural_dynamic_target_depth;
                        let equality_count = structural_equality_count
                            .ok_or_else(|| kernel_fail(ResidentKernelError::InvalidShape))?;
                        let binding_count = structural_binding_count
                            .ok_or_else(|| kernel_fail(ResidentKernelError::InvalidShape))?;
                        let snapshot_finalization_count = structural_snapshot_finalization_count
                            .ok_or_else(|| kernel_fail(ResidentKernelError::InvalidShape))?;
                        let scrutinee = continuation_scrutinee
                            .as_ref()
                            .map(|value| value.as_ref())
                            .or_else(|| self.read_location(scrutinee_source, working_epoch))
                            .ok_or_else(fail)?;
                        let shape_values = match scrutinee {
                            ResidentValueRef::Snapshot([Some(value)]) => {
                                value.shape().parameter_values().to_vec().into_boxed_slice()
                            }
                            _ => scrutinee_shape_values.clone(),
                        };
                        let structural_array = matches!(
                            self.plan
                                .schemas
                                .get(scrutinee_schema)
                                .ok_or_else(fail)?
                                .body(),
                            mech_core::SchemaBody::Matrix { .. }
                        );
                        let canonical_finalization_work =
                            comprehension_execution::admit_pattern_item_materialization(
                                scrutinee,
                                scrutinee_source.region(),
                                scrutinee_schema,
                                structural_array,
                                work,
                                binding_count,
                                equality_count,
                                snapshot_finalization_count,
                                clone_multiplicity,
                                dynamic_target_depth,
                                &self.plan.schemas,
                            )
                            .map_err(kernel_fail)?;
                        let item = comprehension_execution::resident_pattern_item(
                            scrutinee,
                            scrutinee_source.region(),
                            scrutinee_schema,
                            &shape_values,
                            &self.plan.schemas,
                            structural_array,
                        )
                        .ok_or_else(fail)?;
                        structural_scrutinee =
                            Some((item, shape_values, canonical_finalization_work));
                    }
                    let (item, source_shape_values, canonical_finalization_work) =
                        structural_scrutinee.as_ref().ok_or_else(fail)?;
                    let matched = self.match_structural_pattern_item(
                        node,
                        &pattern,
                        item,
                        source_shape_values,
                        *canonical_finalization_work,
                        working_epoch,
                    )?;
                    matched
                }
            };
            if !pattern_matches {
                for region in &binding_regions {
                    self.workspace.scratch.discard_payload_write(*region);
                }
                continue;
            }
            if let Some(guard) = guard {
                let mut guard_live = ControlBlockLiveFootprint::default();
                for step in guard.steps.iter() {
                    self.execute_control_step_with_live_demand(
                        node,
                        &guard,
                        step,
                        &mut guard_live,
                        &mut local_meter,
                        (before_epoch, working_epoch),
                        probe,
                        (live_bytes, live_nodes),
                    )?;
                }
                let guard_matches = match self.read_location(guard.yield_value, working_epoch) {
                    Some(ResidentValueRef::Bool([1])) => true,
                    Some(ResidentValueRef::Bool([0])) => false,
                    _ => return Err(fail()),
                };
                for region in &guard_regions {
                    self.workspace.scratch.discard_payload_write(*region);
                }
                if !guard_matches {
                    for region in &binding_regions {
                        self.workspace.scratch.discard_payload_write(*region);
                    }
                    continue;
                }
            }
            let mut body_live = ControlBlockLiveFootprint::default();
            for step in body.steps.iter() {
                // Branch switches always initialize their selected locals; no
                // sibling output participates in scheduling or initialization.
                self.execute_control_step_with_live_demand(
                    node,
                    &body,
                    step,
                    &mut body_live,
                    &mut local_meter,
                    (before_epoch, working_epoch),
                    probe,
                    (live_bytes, live_nodes),
                )?;
            }
            if continuation && self.workspace.continuation_candidates[index].is_some() {
                let published = bit_is_set(&self.workspace.continuation_publications, index);
                if published {
                    set_bit(&mut self.workspace.initialized_output_bits, index);
                }
                return Ok(published);
            }
            if continuation {
                set_bit(&mut self.workspace.completed_continuations, index);
            }
            let captured_yield = self
                .workspace
                .continuation_capture_frames
                .iter()
                .rev()
                .flat_map(|frame| frame.iter())
                .find_map(|(source, value)| (*source == body.yield_value).then_some(value));
            if let Some(captured) = captured_yield {
                let target = if write.storage == ResidentStorageClass::Constant {
                    &mut self.activation
                } else {
                    &mut self.workspace.scratch
                };
                let unchanged = resident_values_equal(target.read(write.region), captured.as_ref());
                copy_input(target, write.region, captured.as_ref())
                    .map_err(|error| error.at(fail()))?;
                let initialized = bit_is_set(&self.workspace.initialized_output_bits, index);
                set_bit(&mut self.workspace.initialized_output_bits, index);
                return Ok(!initialized || !unchanged);
            }
            // The selected body's bindings and earlier results remain live
            // through publication, including locals the final yield does not
            // use. Replan the wrapper with those payloads before cloning the
            // yield into its output.
            let publication_locals = self
                .resident_local_footprint(
                    body.locals.iter().copied(),
                    &self.plan.schemas,
                    &mut local_meter,
                )
                .map_err(kernel_fail)?;
            let mut facts = crate::memory_planner::TurnMemoryFacts::default();
            facts.additional_demand.turn_peak_bytes = live_bytes
                .checked_add(publication_locals.retained_bytes)
                .ok_or_else(|| kernel_fail(ResidentKernelError::InvalidShape))?;
            facts.additional_demand.retained_nodes = live_nodes
                .checked_add(publication_locals.node_count)
                .ok_or_else(|| kernel_fail(ResidentKernelError::InvalidShape))?;
            let publication_plan = crate::memory_planner::plan_current_resident_turn(
                &self.plan.memory_plan,
                budget_node,
                &facts,
            )
            .map_err(|_| kernel_fail(ResidentKernelError::InvalidShape))?;
            if !publication_plan.budget_violations.is_empty() {
                return Err(kernel_fail(ResidentKernelError::InvalidShape));
            }
            return budget::with_resident_turn_plan(publication_plan, || {
                let converts_to_snapshot = write.region.kind == ResidentValueKind::Snapshot
                    && body.yield_value.region().kind != ResidentValueKind::Snapshot;
                let converted = if converts_to_snapshot {
                    let target = if write.storage == ResidentStorageClass::Constant {
                        &self.activation
                    } else {
                        &self.workspace.scratch
                    };
                    let prior = match target.read(write.region) {
                        ResidentValueRef::Snapshot([Some(value)]) => Some(value),
                        ResidentValueRef::Snapshot([None]) => None,
                        _ => return Err(fail()),
                    };
                    let (prior_snapshot_nodes, prior_footprint_work) =
                        match_conversion_prior_footprint(prior, &self.plan.schemas)
                            .map_err(|error| ResidentExecutionError::Kernel { node, error })?;
                    let scope = target
                        .prepare_payload_write(write.region)
                        .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?;
                    let source = self
                        .read_location(body.yield_value, working_epoch)
                        .ok_or_else(fail)?;
                    let cost =
                        crate::resident::numeric::resident_value_snapshot_materialization_cost(
                            &self.plan.schemas,
                            body.yield_layout.schema_id,
                            &body.yield_layout.shape_instance,
                            source,
                        )
                        .map_err(|error| ResidentExecutionError::Kernel { node, error })?;
                    (|| -> Result<(), ResidentKernelError> {
                        budget::PreparedKernel::new(
                            (),
                            budget::resident_cost! {
                                comparison_work: cost.comparison_work
                                    .checked_add(prior_footprint_work.comparison_work())
                                    .ok_or(ResidentKernelError::InvalidShape)?,
                                compute_work: cost.compute_work
                                    .checked_add(prior_footprint_work.compute_work())
                                    .ok_or(ResidentKernelError::InvalidShape)?,
                                output_elements: cost.output_elements,
                                output_bytes: cost.persistent_bytes,
                                temporary_bytes: cost.temporary_bytes,
                                cloned_bytes: cost.cloned_bytes,
                                retained_nodes: match_conversion_peak_retained_nodes(
                                    prior_snapshot_nodes,
                                    cost.retained_nodes,
                                )?,
                                ..budget::KernelCostEstimate::default()
                            },
                        )
                        .admit_control()?
                        .into_plan();
                        Ok(())
                    })()
                    .map_err(|error| ResidentExecutionError::Kernel { node, error })?;
                    if let Some(scope) = &scope {
                        scope
                            .admit_snapshot_materialization(
                                cost.persistent_bytes,
                                cost.temporary_bytes,
                            )
                            .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?;
                        scope.start();
                    }
                    let converted = budget::with_payload_admission(
                        scope.as_ref().map(|scope| scope.admission()),
                        || {
                            crate::resident::numeric::resident_value_to_snapshot(
                                &self.plan.schemas,
                                body.yield_layout.schema_id,
                                &body.yield_layout.shape_instance,
                                body.yield_layout.shape,
                                source,
                            )
                        },
                    );
                    let value = match converted {
                        Ok(value) => value,
                        Err(error) => {
                            let target = if write.storage == ResidentStorageClass::Constant {
                                &mut self.activation
                            } else {
                                &mut self.workspace.scratch
                            };
                            target
                                .abort_payload_write(write.region, scope)
                                .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?;
                            return Err(ResidentExecutionError::Kernel { node, error });
                        }
                    };
                    if value.schema() != self.plan.slots[write.slot.get() as usize].schema {
                        let target = if write.storage == ResidentStorageClass::Constant {
                            &mut self.activation
                        } else {
                            &mut self.workspace.scratch
                        };
                        target
                            .abort_payload_write(write.region, scope)
                            .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?;
                        return Err(ResidentExecutionError::Kernel {
                            node,
                            error: ResidentKernelError::InvalidOutput,
                        });
                    }
                    Some((value, scope))
                } else {
                    None
                };
                let (unchanged, copied) = if let Some((next, scope)) = converted {
                    let target = if write.storage == ResidentStorageClass::Constant {
                        &mut self.activation
                    } else {
                        &mut self.workspace.scratch
                    };
                    let ResidentValueRef::Snapshot([current]) = target.read(write.region) else {
                        unreachable!("snapshot conversion target was checked")
                    };
                    let unchanged = current.as_ref().map_or(Ok(false), |current| {
                        current.snapshot_eq(&self.plan.schemas, &next, &self.plan.schemas)
                    });
                    let unchanged = match unchanged {
                        Ok(unchanged) => unchanged,
                        Err(_) => {
                            target
                                .abort_payload_write(write.region, scope)
                                .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?;
                            return Err(ResidentExecutionError::Kernel {
                                node,
                                error: ResidentKernelError::InvalidOutput,
                            });
                        }
                    };
                    if let Some(prepared) = scope.as_ref() {
                        if let Err(error) = prepared.admit_value(&next) {
                            target
                                .abort_payload_write(write.region, scope)
                                .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?;
                            return Err(ResidentExecutionError::MemoryRuntime { error });
                        }
                    }
                    let ResidentValueMut::Snapshot([target_value]) = target.write(write.region)
                    else {
                        unreachable!("snapshot conversion target was checked")
                    };
                    *target_value = Some(next);
                    target
                        .finish_payload_write(write.region, scope)
                        .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?;
                    (unchanged, Ok(()))
                } else if write.storage == ResidentStorageClass::Constant {
                    match body.yield_value {
                        ResidentReadLocation::Constant(region) => (
                            regions_equal(&self.activation, write.region, &self.activation, region),
                            self.activation
                                .copy_region_within(write.region, region)
                                .map_err(ResidentCopyError::Memory),
                        ),
                        ResidentReadLocation::Scratch(region) => (
                            regions_equal(
                                &self.activation,
                                write.region,
                                &self.workspace.scratch,
                                region,
                            ),
                            self.activation
                                .copy_region_from(write.region, &self.workspace.scratch, region)
                                .map_err(ResidentCopyError::Memory),
                        ),
                        _ => return Err(fail()),
                    }
                } else {
                    let unchanged = match body.yield_value {
                        ResidentReadLocation::Constant(region) => regions_equal(
                            &self.workspace.scratch,
                            write.region,
                            &self.activation,
                            region,
                        ),
                        ResidentReadLocation::Input(region) => regions_equal(
                            &self.workspace.scratch,
                            write.region,
                            &self.workspace.input,
                            region,
                        ),
                        ResidentReadLocation::LexicalInput(region) => {
                            let value = lexical_capture(
                                &self.workspace.continuation_capture_frames,
                                region,
                            )
                            .map(super::OwnedResidentValue::as_ref)
                            .unwrap_or_else(|| self.workspace.input.read(region));
                            resident_values_equal(self.workspace.scratch.read(write.region), value)
                        }
                        ResidentReadLocation::State { slot, region } => {
                            let buffer = self.state.select_buffer(slot, working_epoch);
                            regions_equal(
                                &self.workspace.scratch,
                                write.region,
                                &self.state.buffers[buffer],
                                region,
                            )
                        }
                        ResidentReadLocation::Scratch(region) => regions_equal(
                            &self.workspace.scratch,
                            write.region,
                            &self.workspace.scratch,
                            region,
                        ),
                    };
                    // Publish the selected value through the same managed copy path used
                    // for ordinary resident state. Composite construction remains in its
                    // existing bound kernel; no sibling branch is evaluated here.
                    let copied = match body.yield_value {
                        ResidentReadLocation::Constant(region) => self
                            .workspace
                            .scratch
                            .copy_region_from(write.region, &self.activation, region)
                            .map_err(ResidentCopyError::Memory),
                        ResidentReadLocation::Input(region) => self
                            .workspace
                            .scratch
                            .copy_region_from(write.region, &self.workspace.input, region)
                            .map_err(ResidentCopyError::Memory),
                        ResidentReadLocation::LexicalInput(region) => {
                            let value = lexical_capture(
                                &self.workspace.continuation_capture_frames,
                                region,
                            )
                            .map(super::OwnedResidentValue::as_ref)
                            .unwrap_or_else(|| self.workspace.input.read(region));
                            copy_input(&mut self.workspace.scratch, write.region, value)
                        }
                        ResidentReadLocation::State { slot, region } => {
                            let buffer = self.state.select_buffer(slot, working_epoch);
                            self.workspace
                                .scratch
                                .copy_region_from(write.region, &self.state.buffers[buffer], region)
                                .map_err(ResidentCopyError::Memory)
                        }
                        ResidentReadLocation::Scratch(region) => self
                            .workspace
                            .scratch
                            .copy_region_within(write.region, region)
                            .map_err(ResidentCopyError::Memory),
                    };
                    (unchanged, copied)
                };
                copied.map_err(|error| error.at(fail()))?;
                let initialized = bit_is_set(&self.workspace.initialized_output_bits, index);
                set_bit(&mut self.workspace.initialized_output_bits, index);
                Ok(!initialized || !unchanged)
            });
        }
        Err(fail())
    }

    fn execute_control_step_with_live_demand(
        &mut self,
        owner: NodeId,
        block: &super::ActivatedControlBlock,
        step: &super::ActivatedControlStep,
        block_live: &mut ControlBlockLiveFootprint,
        meter: &mut budget::ResidentBudgetMeter,
        epochs: (InstanceEpoch, InstanceEpoch),
        probe: &mut ResidentStructuralProbe,
        live: (u64, u64),
    ) -> Result<bool, ResidentExecutionError> {
        let (before_epoch, working_epoch) = epochs;
        let (live_bytes, live_nodes) = live;
        let fail = |error| ResidentExecutionError::Kernel { node: owner, error };
        let retained = usize::try_from(step.retained_local_count)
            .map_err(|_| fail(ResidentKernelError::InvalidShape))?;
        let nested_match = matches!(
            self.plan.steps[step.node.get() as usize],
            ActivatedTurnStep::Match(_)
        );
        // Nested matches count their prior output while executing, but may
        // replace it. Keep that slot out of the persistent prefix and measure
        // its new value only when the following step needs it.
        let stable_end = if nested_match {
            retained
                .checked_sub(1)
                .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?
        } else {
            retained
        };
        let added = block
            .locals
            .get(block_live.retained_prefix..stable_end)
            .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
        block_live.footprint = block_live
            .footprint
            .checked_add(
                self.resident_local_footprint(added.iter().copied(), &self.plan.schemas, meter)
                    .map_err(fail)?,
            )
            .map_err(|_| fail(ResidentKernelError::InvalidShape))?;
        block_live.retained_prefix = stable_end;
        let mut locals = block_live.footprint;
        if nested_match {
            let prior_output = *block
                .locals
                .get(stable_end)
                .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
            locals = locals
                .checked_add(
                    self.resident_local_footprint([prior_output], &self.plan.schemas, meter)
                        .map_err(fail)?,
                )
                .map_err(|_| fail(ResidentKernelError::InvalidShape))?;
        }
        let mut excluded = ValueFootprint::zero();
        for index in step.excluded_locals.iter().copied() {
            let index =
                usize::try_from(index).map_err(|_| fail(ResidentKernelError::InvalidShape))?;
            if index >= retained {
                return Err(fail(ResidentKernelError::InvalidShape));
            }
            let region = *block
                .locals
                .get(index)
                .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
            excluded = excluded
                .checked_add(
                    self.resident_local_footprint([region], &self.plan.schemas, meter)
                        .map_err(fail)?,
                )
                .map_err(|_| fail(ResidentKernelError::InvalidShape))?;
        }
        locals = ValueFootprint {
            encoded_bytes: locals
                .encoded_bytes
                .checked_sub(excluded.encoded_bytes)
                .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?,
            retained_bytes: locals
                .retained_bytes
                .checked_sub(excluded.retained_bytes)
                .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?,
            node_count: locals
                .node_count
                .checked_sub(excluded.node_count)
                .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?,
        };
        let live_bytes = live_bytes
            .checked_add(locals.retained_bytes)
            .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
        let live_nodes = live_nodes
            .checked_add(locals.node_count)
            .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
        self.execute_step_with_live_demand(
            step.node,
            before_epoch,
            working_epoch,
            probe,
            live_bytes,
            live_nodes,
        )
    }

    fn execute_recursive_call(
        &mut self,
        node_index: ActivatedNodeIndex,
        call: super::ActivatedRecursiveCall,
        before_epoch: InstanceEpoch,
        working_epoch: InstanceEpoch,
        probe: &mut ResidentStructuralProbe,
        live_bytes: u64,
        live_nodes: u64,
    ) -> Result<bool, ResidentExecutionError> {
        let fail = |error| ResidentExecutionError::Kernel {
            node: call.artifact_node,
            error,
        };
        let depth = self.workspace.recursive_scrutinees.len();
        if depth >= MAX_RESIDENT_RECURSION_DEPTH
            || call.write.storage != ResidentStorageClass::Scratch
        {
            return Err(fail(ResidentKernelError::InvalidShape));
        }
        let (regions, returned, region_inventory_bytes) = {
            let ActivatedTurnStep::Match(root) = &self.plan.steps[call.target.get() as usize]
            else {
                return Err(fail(ResidentKernelError::InvalidInput));
            };
            let count = root
                .locals
                .len()
                .checked_add(usize::from(
                    root.write.storage == ResidentStorageClass::Scratch,
                ))
                .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
            let region_inventory_bytes = u64::try_from(count)
                .ok()
                .and_then(|count| count.checked_mul(core::mem::size_of::<ResidentRegion>() as u64))
                .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
            (|| -> Result<(), ResidentKernelError> {
                budget::PreparedKernel::new(
                    (),
                    recursive_inventory_cost(live_bytes, live_nodes, region_inventory_bytes)?,
                )
                .admit_control()?
                .into_plan();
                Ok(())
            })()
            .map_err(fail)?;
            let mut regions = Vec::with_capacity(count);
            regions.extend_from_slice(&root.locals);
            if root.write.storage == ResidentStorageClass::Scratch {
                // Recursive arms share the target output slot. Its value from
                // the preceding turn must survive the inner invocation.
                regions.push(root.write.region);
            }
            regions.sort_unstable_by_key(|region| (region.kind as u8, region.offset, region.len));
            regions.dedup();
            (regions, root.write, region_inventory_bytes)
        };

        let mut frame_meter = budget::ResidentBudgetMeter::default();
        let (value_bytes, value_nodes) = regions
            .iter()
            .try_fold((0u64, 0u64), |(bytes, nodes), region| {
                let value = resident_frame_value_footprint(
                    self.workspace.scratch.read(*region),
                    &self.plan.schemas,
                    &mut frame_meter,
                )
                .ok_or(ResidentKernelError::InvalidShape)?;
                Ok::<_, ResidentKernelError>((
                    bytes
                        .checked_add(value.0)
                        .ok_or(ResidentKernelError::InvalidShape)?,
                    nodes
                        .checked_add(value.1)
                        .ok_or(ResidentKernelError::InvalidShape)?,
                ))
            })
            .map_err(fail)?;
        let measured_frame_work = frame_meter.estimate().compute_work();
        let frame_bytes = region_inventory_bytes
            .checked_add(value_bytes)
            .and_then(|bytes| {
                bytes.checked_add((2 * core::mem::size_of::<super::RecursiveFrame>()) as u64)
            })
            .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
        let frame_nodes = value_nodes
            .checked_add(2)
            .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
        let frame_copy_work = frame_bytes
            .checked_mul(2)
            .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
        (|| -> Result<(), ResidentKernelError> {
            let peak_bytes = live_bytes
                .checked_add(frame_bytes)
                .ok_or(ResidentKernelError::InvalidShape)?;
            let peak_nodes = live_nodes
                .checked_add(frame_nodes)
                .ok_or(ResidentKernelError::InvalidShape)?;
            budget::PreparedKernel::new(
                (),
                budget::resident_cost! {
                    compute_work: measured_frame_work
                        .checked_add(frame_copy_work)
                        .ok_or(ResidentKernelError::InvalidShape)?,
                    temporary_bytes: peak_bytes,
                    cloned_bytes: frame_copy_work,
                    retained_nodes: peak_nodes,
                    ..budget::KernelCostEstimate::default()
                },
            )
            .admit_control()?
            .into_plan();
            Ok(())
        })()
        .map_err(fail)?;

        let child_live_bytes = live_bytes
            .checked_add(frame_bytes)
            .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
        let child_live_nodes = live_nodes
            .checked_add(frame_nodes)
            .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
        let budget_node = match &self.plan.steps[call.target.get() as usize] {
            ActivatedTurnStep::Match(target) => target.budget_node,
            _ => unreachable!("recursive target was checked above"),
        };
        let mut facts = crate::memory_planner::TurnMemoryFacts::default();
        facts.additional_demand.turn_peak_bytes = child_live_bytes;
        facts.additional_demand.retained_nodes = child_live_nodes;
        let child_plan = crate::memory_planner::plan_current_resident_turn(
            &self.plan.memory_plan,
            budget_node,
            &facts,
        )
        .map_err(|_| fail(ResidentKernelError::InvalidShape))?;
        if !child_plan.budget_violations.is_empty() {
            return Err(fail(ResidentKernelError::InvalidShape));
        }

        let frame = regions
            .iter()
            .map(|region| {
                (
                    *region,
                    owned_resident_value(self.workspace.scratch.read(*region)),
                )
            })
            .collect::<Vec<_>>();
        let target_index = call.target.get() as usize;
        let target_initialized = bit_is_set(&self.workspace.initialized_output_bits, target_index);
        self.workspace
            .recursive_scrutinees
            .push(super::RecursiveFrame {
                target: call.target,
                argument: call.argument,
            });
        let invoked = budget::with_resident_turn_plan(child_plan, || {
            self.execute_match_expression(
                call.target,
                before_epoch,
                working_epoch,
                probe,
                child_live_bytes,
                child_live_nodes,
            )
        });
        self.workspace.recursive_scrutinees.pop();
        if target_initialized {
            set_bit(&mut self.workspace.initialized_output_bits, target_index);
        } else {
            clear_bit(&mut self.workspace.initialized_output_bits, target_index);
        }

        if self.workspace.continuation_candidates[call.target.get() as usize].is_some() {
            for (region, value) in &frame {
                copy_input(&mut self.workspace.scratch, *region, value.as_ref())
                    .map_err(|error| error.at(fail(ResidentKernelError::InvalidOutput)))?;
            }
            invoked?;
            return Ok(bit_is_set(
                &self.workspace.continuation_publications,
                call.target.get() as usize,
            ));
        }

        let result = invoked.and_then(|changed| {
            let mut child_meter = budget::ResidentBudgetMeter::default();
            let child_locals = match &self.plan.steps[call.target.get() as usize] {
                ActivatedTurnStep::Match(target) => self
                    .resident_local_footprint(
                        target.locals.iter().copied(),
                        &self.plan.schemas,
                        &mut child_meter,
                    )
                    .map_err(fail)?,
                _ => unreachable!("recursive target was checked above"),
            };
            let location = match returned.storage {
                ResidentStorageClass::Constant => ResidentReadLocation::Constant(returned.region),
                ResidentStorageClass::Input => ResidentReadLocation::Input(returned.region),
                ResidentStorageClass::State => ResidentReadLocation::State {
                    slot: returned.slot,
                    region: returned.region,
                },
                ResidentStorageClass::Scratch => ResidentReadLocation::Scratch(returned.region),
            };
            let value = self
                .read_location(location, working_epoch)
                .ok_or_else(|| fail(ResidentKernelError::InvalidOutput))?;
            let mut result_meter = budget::ResidentBudgetMeter::default();
            let (result_bytes, result_nodes) =
                resident_frame_value_footprint(value, &self.plan.schemas, &mut result_meter)
                    .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
            let measured_result_work = result_meter.estimate().compute_work();
            let result_copy_work = result_bytes
                .checked_mul(2)
                .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
            let peak_bytes = live_bytes
                .checked_add(frame_copy_work)
                .and_then(|bytes| bytes.checked_add(child_locals.retained_bytes))
                .and_then(|bytes| bytes.checked_add(result_copy_work))
                .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
            let peak_nodes = live_nodes
                .checked_add(
                    frame_nodes
                        .checked_mul(2)
                        .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?,
                )
                .and_then(|nodes| nodes.checked_add(child_locals.node_count))
                .and_then(|nodes| nodes.checked_add(result_nodes.checked_mul(2)?))
                .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
            (|| -> Result<(), ResidentKernelError> {
                budget::PreparedKernel::new(
                    (),
                    budget::resident_cost! {
                        compute_work: child_meter
                            .estimate()
                            .compute_work()
                            .checked_add(measured_result_work)
                            .and_then(|work| work.checked_add(result_copy_work))
                            .ok_or(ResidentKernelError::InvalidShape)?,
                        temporary_bytes: peak_bytes,
                        cloned_bytes: result_copy_work,
                        retained_nodes: peak_nodes,
                        ..budget::KernelCostEstimate::default()
                    },
                )
                .admit_control()?
                .into_plan();
                Ok(())
            })()
            .map_err(fail)?;
            Ok((changed, owned_resident_value(value)))
        });

        for (region, value) in &frame {
            copy_input(&mut self.workspace.scratch, *region, value.as_ref())
                .map_err(|error| error.at(fail(ResidentKernelError::InvalidOutput)))?;
        }
        let (changed, result) = result?;
        copy_input(
            &mut self.workspace.scratch,
            call.write.region,
            result.as_ref(),
        )
        .map_err(|error| error.at(fail(ResidentKernelError::InvalidOutput)))?;
        set_bit(
            &mut self.workspace.initialized_output_bits,
            node_index.get() as usize,
        );
        Ok(changed)
    }

    fn match_structural_pattern_item(
        &mut self,
        node: NodeId,
        pattern: &crate::CollectionPattern<
            super::ActivatedPatternBinding,
            super::ActivatedPatternValue,
        >,
        item: &comprehension_execution::PatternItem,
        source_shape_values: &[u64],
        canonical_finalization_work: u64,
        working: InstanceEpoch,
    ) -> Result<bool, ResidentExecutionError> {
        let fail = |error| ResidentExecutionError::Kernel { node, error };
        match pattern {
            crate::CollectionPattern::Wildcard => Ok(true),
            crate::CollectionPattern::Bind {
                schema: binding, ..
            } => self.bind_match_pattern_item(
                node,
                *binding,
                item.clone(),
                source_shape_values,
                canonical_finalization_work,
            ),
            crate::CollectionPattern::Equal(peer) => {
                let peer_value = self
                    .read_location(peer.location, working)
                    .ok_or_else(|| fail(ResidentKernelError::InvalidInput))?;
                item.language_equals(
                    peer_value,
                    peer.location.region(),
                    peer.schema,
                    source_shape_values,
                    canonical_finalization_work,
                    &self.plan.schemas,
                    &self.plan.structural_projections,
                )
                .map_err(fail)?
                .ok_or_else(|| fail(ResidentKernelError::InvalidInput))
            }
            crate::CollectionPattern::Enum { ordinal, payload } => {
                let Some((actual, value_payload)) =
                    item.enum_variant(&self.plan.schemas).map_err(fail)?
                else {
                    return Ok(false);
                };
                if actual != *ordinal {
                    return Ok(false);
                }
                match (payload.as_deref(), value_payload.as_ref()) {
                    (None, None) => Ok(true),
                    (Some(pattern), Some(child)) => self.match_structural_pattern_item(
                        node,
                        pattern,
                        child,
                        source_shape_values,
                        canonical_finalization_work,
                        working,
                    ),
                    _ => Ok(false),
                }
            }
            crate::CollectionPattern::Tuple(items) => {
                if item.structural_len(true) != Some(items.len()) {
                    return Ok(false);
                }
                for (index, pattern) in items.iter().enumerate() {
                    let Some(child) =
                        item.child(index, &self.plan.schemas, &self.plan.structural_projections)
                    else {
                        return Ok(false);
                    };
                    if !self.match_structural_pattern_item(
                        node,
                        pattern,
                        &child,
                        source_shape_values,
                        canonical_finalization_work,
                        working,
                    )? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            crate::CollectionPattern::Array {
                prefix,
                rest,
                suffix,
            } => {
                let Some(count) = item.structural_len(false) else {
                    return Ok(false);
                };
                let required = prefix
                    .len()
                    .checked_add(suffix.len())
                    .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
                if count < required || (rest.is_none() && count != required) {
                    return Ok(false);
                }
                for (index, pattern) in prefix.iter().enumerate() {
                    let Some(child) =
                        item.child(index, &self.plan.schemas, &self.plan.structural_projections)
                    else {
                        return Ok(false);
                    };
                    if !self.match_structural_pattern_item(
                        node,
                        pattern,
                        &child,
                        source_shape_values,
                        canonical_finalization_work,
                        working,
                    )? {
                        return Ok(false);
                    }
                }
                if let Some(rest) = rest {
                    let Some(middle) = item.middle(
                        prefix.len(),
                        suffix.len(),
                        &self.plan.schemas,
                        &self.plan.structural_projections,
                    ) else {
                        return Ok(false);
                    };
                    if !self.match_structural_pattern_item(
                        node,
                        rest,
                        &middle,
                        source_shape_values,
                        canonical_finalization_work,
                        working,
                    )? {
                        return Ok(false);
                    }
                }
                for (index, pattern) in suffix.iter().enumerate() {
                    let Some(child) = item.child(
                        count - suffix.len() + index,
                        &self.plan.schemas,
                        &self.plan.structural_projections,
                    ) else {
                        return Ok(false);
                    };
                    if !self.match_structural_pattern_item(
                        node,
                        pattern,
                        &child,
                        source_shape_values,
                        canonical_finalization_work,
                        working,
                    )? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
        }
    }

    fn bind_match_pattern_item(
        &mut self,
        node: NodeId,
        binding: super::ActivatedPatternBinding,
        item: comprehension_execution::PatternItem,
        source_shape_values: &[u64],
        binding_finalization_work: u64,
    ) -> Result<bool, ResidentExecutionError> {
        let fail = |error| ResidentExecutionError::Kernel { node, error };
        fn dense_values<T: Default>(values: Vec<T>, region: ResidentRegion) -> Option<Vec<T>> {
            if values.len() != region.len {
                return None;
            }
            let mut dense = (0..values.len()).map(|_| T::default()).collect::<Vec<_>>();
            for (ordinal, value) in values.into_iter().enumerate() {
                let offset = comprehension_execution::dense_collection_offset(region, ordinal)?;
                *dense.get_mut(offset)? = value;
            }
            Some(dense)
        }
        let Some(comprehension_execution::PatternBindingItem {
            shape_values,
            data,
            schemas: source_schemas,
            schema_index: source_schema_index,
            ..
        }) = item
            .into_binding(
                binding.schema,
                source_shape_values,
                &self.plan.schemas,
                &self.plan.structural_projections,
            )
            .map_err(fail)?
        else {
            return Ok(false);
        };
        match (data, binding.region.kind) {
            (mech_core::ValueDataDraft::Matrix(values), ResidentValueKind::Bool) => {
                let values = values
                    .into_vec()
                    .into_iter()
                    .map(|value| match value {
                        mech_core::ValueDataDraft::Bool(value) => Some(u8::from(value)),
                        _ => None,
                    })
                    .collect::<Option<Vec<_>>>()
                    .and_then(|values| dense_values(values, binding.region))
                    .ok_or_else(|| fail(ResidentKernelError::InvalidOutput))?;
                let ResidentValueMut::Bool(target) = self.workspace.scratch.write(binding.region)
                else {
                    unreachable!("binding kind was validated")
                };
                target.copy_from_slice(&values);
            }
            (mech_core::ValueDataDraft::Matrix(values), ResidentValueKind::Index) => {
                let values = values
                    .into_vec()
                    .into_iter()
                    .map(|value| match value {
                        mech_core::ValueDataDraft::Index(value) => Some(value),
                        _ => None,
                    })
                    .collect::<Option<Vec<_>>>()
                    .and_then(|values| dense_values(values, binding.region))
                    .ok_or_else(|| fail(ResidentKernelError::InvalidOutput))?;
                let ResidentValueMut::Index(target) = self.workspace.scratch.write(binding.region)
                else {
                    unreachable!("binding kind was validated")
                };
                target.copy_from_slice(&values);
            }
            (mech_core::ValueDataDraft::Matrix(values), ResidentValueKind::F64) => {
                let values = values
                    .into_vec()
                    .into_iter()
                    .map(|value| match value {
                        mech_core::ValueDataDraft::F64(value) => Some(value.to_f64()),
                        _ => None,
                    })
                    .collect::<Option<Vec<_>>>()
                    .and_then(|values| dense_values(values, binding.region))
                    .ok_or_else(|| fail(ResidentKernelError::InvalidOutput))?;
                let ResidentValueMut::F64(target) = self.workspace.scratch.write(binding.region)
                else {
                    unreachable!("binding kind was validated")
                };
                target.copy_from_slice(&values);
            }
            (mech_core::ValueDataDraft::Matrix(values), ResidentValueKind::String) => {
                let values = values
                    .into_vec()
                    .into_iter()
                    .map(|value| match value {
                        mech_core::ValueDataDraft::String(value) => Some(value),
                        _ => None,
                    })
                    .collect::<Option<Vec<_>>>()
                    .and_then(|values| dense_values(values, binding.region))
                    .ok_or_else(|| fail(ResidentKernelError::InvalidOutput))?;
                let scope = self
                    .workspace
                    .scratch
                    .prepare_payload_write(binding.region)
                    .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?;
                if let Some(scope) = &scope {
                    scope
                        .admit_copy(ResidentValueRef::String(&values), 0)
                        .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?;
                    scope.start();
                }
                let ResidentValueMut::String(target) = self.workspace.scratch.write(binding.region)
                else {
                    unreachable!("binding kind was validated")
                };
                for (target, value) in target.iter_mut().zip(values) {
                    *target = value;
                }
                self.workspace
                    .scratch
                    .finish_payload_write(binding.region, scope)
                    .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?;
            }
            (mech_core::ValueDataDraft::Bool(value), ResidentValueKind::Bool) => {
                let ResidentValueMut::Bool([target]) = self.workspace.scratch.write(binding.region)
                else {
                    unreachable!("binding kind was validated")
                };
                *target = u8::from(value);
            }
            (mech_core::ValueDataDraft::Index(value), ResidentValueKind::Index) => {
                let ResidentValueMut::Index([target]) =
                    self.workspace.scratch.write(binding.region)
                else {
                    unreachable!("binding kind was validated")
                };
                *target = value;
            }
            (mech_core::ValueDataDraft::F64(value), ResidentValueKind::F64) => {
                let ResidentValueMut::F64([target]) = self.workspace.scratch.write(binding.region)
                else {
                    unreachable!("binding kind was validated")
                };
                *target = value.to_f64();
            }
            (mech_core::ValueDataDraft::String(value), ResidentValueKind::String) => {
                let scope = self
                    .workspace
                    .scratch
                    .prepare_payload_write(binding.region)
                    .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?;
                if let Some(scope) = &scope {
                    scope
                        .admit_copy(ResidentValueRef::String(core::slice::from_ref(&value)), 0)
                        .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?;
                    scope.start();
                }
                let ResidentValueMut::String([target]) =
                    self.workspace.scratch.write(binding.region)
                else {
                    unreachable!("binding kind was validated")
                };
                *target = value;
                self.workspace
                    .scratch
                    .finish_payload_write(binding.region, scope)
                    .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?;
            }
            (data, ResidentValueKind::Snapshot) => {
                let canonical_budget = mech_core::snapshot::SnapshotCanonicalizationBudget::new(
                    binding_finalization_work,
                );
                let next = comprehension_execution::finalize_pattern_binding(
                    binding.schema,
                    &shape_values,
                    data,
                    source_schemas,
                    source_schema_index,
                    &self.plan.schemas,
                    &canonical_budget,
                )
                .map_err(|_| fail(ResidentKernelError::InvalidOutput))?;
                let scope = self
                    .workspace
                    .scratch
                    .prepare_payload_write(binding.region)
                    .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?;
                if let Some(scope) = &scope {
                    scope
                        .admit_value(&next)
                        .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?;
                    scope.start();
                }
                let ResidentValueMut::Snapshot([target]) =
                    self.workspace.scratch.write(binding.region)
                else {
                    unreachable!("binding kind was validated")
                };
                *target = Some(next);
                self.workspace
                    .scratch
                    .finish_payload_write(binding.region, scope)
                    .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?;
            }
            _ => return Err(fail(ResidentKernelError::InvalidOutput)),
        }
        Ok(true)
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
        let suspended = self.plan.steps.iter().enumerate().any(|(index, step)| {
            self.unpublished_continuation(index)
                && matches!(step, ActivatedTurnStep::Match(control)
                if super::read_location_depends_on_match(
                    &self.plan,
                    source,
                    index,
                    control.write.region,
                ))
        });
        if suspended {
            return Ok(());
        }
        if self.workspace.effect_intents.len() == self.workspace.effect_intents.capacity() {
            return Err(ResidentExecutionError::EffectIntentCapacity);
        }
        self.capture_effect_payload(source, captured, working_epoch)
            .map_err(|error| {
                error.at(ResidentExecutionError::InvalidWrite {
                    node: artifact_node,
                })
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
    ) -> Result<(), ResidentCopyError> {
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
            ResidentReadLocation::LexicalInput(region) => {
                let value = lexical_capture(&self.workspace.continuation_capture_frames, region)
                    .map(super::OwnedResidentValue::as_ref)
                    .unwrap_or_else(|| self.workspace.input.read(region));
                copy_input(&mut self.workspace.effect_payloads, captured, value)
            }
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
        self.with_kernel_turn_plan(node_index, before_epoch, working_epoch, |this| {
            this.execute_kernel_planned(node_index, before_epoch, working_epoch, probe)
        })
    }

    #[inline(always)]
    fn execute_kernel_with_live_demand(
        &mut self,
        node_index: ActivatedNodeIndex,
        before_epoch: InstanceEpoch,
        working_epoch: InstanceEpoch,
        probe: &mut ResidentStructuralProbe,
        live_bytes: u64,
        live_nodes: u64,
    ) -> Result<bool, ResidentExecutionError> {
        self.with_kernel_turn_plan_and_live_demand(
            node_index,
            before_epoch,
            working_epoch,
            live_bytes,
            live_nodes,
            |this| this.execute_kernel_planned(node_index, before_epoch, working_epoch, probe),
        )
    }

    #[inline(always)]
    fn execute_kernel_planned(
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
                if let Some(previous) = node.rmw_previous {
                    self.workspace
                        .rmw_previous
                        .copy_region_from(previous, &self.workspace.scratch, node.write.region)
                        .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?;
                }
                if let Some(base) = node.rmw_base {
                    let destination = node.write.region;
                    let result = match base {
                        ResidentReadLocation::Constant(region) => self
                            .workspace
                            .scratch
                            .copy_region_from(destination, &self.activation, region)
                            .map_err(ResidentCopyError::Memory),
                        ResidentReadLocation::Input(region) => self
                            .workspace
                            .scratch
                            .copy_region_from(destination, &self.workspace.input, region)
                            .map_err(ResidentCopyError::Memory),
                        ResidentReadLocation::LexicalInput(region) => {
                            let value = lexical_capture(
                                &self.workspace.continuation_capture_frames,
                                region,
                            )
                            .map(super::OwnedResidentValue::as_ref)
                            .unwrap_or_else(|| self.workspace.input.read(region));
                            copy_input(&mut self.workspace.scratch, destination, value)
                        }
                        ResidentReadLocation::State { slot, region } => {
                            let buffer = self.state.select_buffer(slot, working_epoch);
                            self.workspace
                                .scratch
                                .copy_region_from(destination, &self.state.buffers[buffer], region)
                                .map_err(ResidentCopyError::Memory)
                        }
                        ResidentReadLocation::Scratch(region) => self
                            .workspace
                            .scratch
                            .copy_region_within(destination, region)
                            .map_err(ResidentCopyError::Memory),
                    };
                    result.map_err(|error| {
                        error.at(ResidentExecutionError::InvalidWrite {
                            node: node.artifact_node,
                        })
                    })?;
                }
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
                            captures: &self.workspace.continuation_capture_frames,
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
                            captures: &self.workspace.continuation_capture_frames,
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
                        captures: &self.workspace.continuation_capture_frames,
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
                    ChangeDetectionPolicy::KernelReported => {
                        if let Some(previous) = node.rmw_previous {
                            !rmw_outputs_equal(
                                &self.workspace.rmw_previous,
                                previous,
                                &self.workspace.scratch,
                                node.write.region,
                                &self.plan.schemas,
                            )
                        } else {
                            kernel_changed
                        }
                    }
                    ChangeDetectionPolicy::ExactScalar => {
                        match (
                            before_scalar,
                            scalar_token(self.workspace.scratch.read(node.write.region)),
                        ) {
                            (Some(before), Some(after)) => before != after,
                            // Non-scalar snapshots have no fixed scalar token;
                            // their maintained kernel compares the typed value.
                            _ => kernel_changed,
                        }
                    }
                    ChangeDetectionPolicy::AlwaysChanged => true,
                    ChangeDetectionPolicy::SemanticHash => unreachable!(
                        "semantic-hash resident outputs are rejected during activation"
                    ),
                };
                if let Some(previous) = node.rmw_previous {
                    self.workspace.rmw_previous.discard_payload_write(previous);
                }
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
                        let (candidate, seeded) = self
                            .state
                            .begin_rmw(slot, before_epoch, working_epoch)
                            .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?;
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
                        captures: &self.workspace.continuation_capture_frames,
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
                            captures: &self.workspace.continuation_capture_frames,
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
        if let Some(value) = self
            .workspace
            .continuation_capture_frames
            .iter()
            .rev()
            .flat_map(|frame| frame.iter())
            .find_map(|(source, value)| (*source == location).then_some(value))
        {
            return Some(value.as_ref());
        }
        match location {
            ResidentReadLocation::Constant(region) => Some(self.activation.read(region)),
            ResidentReadLocation::Input(region) | ResidentReadLocation::LexicalInput(region) => {
                Some(self.workspace.input.read(region))
            }
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

fn match_conversion_peak_retained_nodes(
    prior_snapshot_nodes: u64,
    candidate_nodes: u64,
) -> Result<u64, ResidentKernelError> {
    prior_snapshot_nodes
        .checked_add(candidate_nodes)
        .ok_or(ResidentKernelError::InvalidShape)
}

fn recursive_inventory_cost(
    live_bytes: u64,
    live_nodes: u64,
    inventory_bytes: u64,
) -> Result<budget::KernelCostEstimate, ResidentKernelError> {
    Ok(budget::resident_cost! {
        // Inventory copying, sorting, and deduplication run while the
        // enclosing arm's dynamically measured locals remain resident.
        compute_work: inventory_bytes,
        temporary_bytes: live_bytes
            .checked_add(inventory_bytes)
            .ok_or(ResidentKernelError::InvalidShape)?,
        retained_nodes: live_nodes,
        ..budget::KernelCostEstimate::default()
    })
}

fn match_conversion_prior_footprint(
    prior: Option<&Value>,
    schemas: &mech_core::SchemaTable,
) -> Result<(u64, budget::KernelCostEstimate), ResidentKernelError> {
    let mut meter = budget::ResidentBudgetMeter::default();
    let nodes = match prior {
        Some(prior) => {
            budget::measure_canonical_value_footprint(&mut meter, prior, schemas)?.node_count
        }
        None => 0,
    };
    Ok((nodes, meter.estimate()))
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

    fn stage_projection_from_arena(
        &mut self,
        target_slot: CellSlotId,
        target_region: ResidentRegion,
        epoch: InstanceEpoch,
        source: &TypedResidentArena,
        source_region: ResidentRegion,
    ) -> mech_core::MemoryRuntimeResult<()> {
        let target_buffer = self.candidate_buffer(target_slot, epoch);
        self.buffers[target_buffer].copy_region_from(target_region, source, source_region)
    }

    fn stage_projection_from_state_slot(
        &mut self,
        target_slot: CellSlotId,
        target_region: ResidentRegion,
        source_slot: CellSlotId,
        source_region: ResidentRegion,
        epoch: InstanceEpoch,
    ) -> mech_core::MemoryRuntimeResult<()> {
        let target_buffer = self.candidate_buffer(target_slot, epoch);
        let source_buffer = self.published_buffer(source_slot, epoch);
        if source_buffer == target_buffer {
            self.buffers[target_buffer].copy_region_within(target_region, source_region)?;
        } else if target_buffer == 0 {
            let [target, source] = &mut self.buffers;
            target.copy_region_from(target_region, source, source_region)?;
        } else {
            let [source, target] = &mut self.buffers;
            target.copy_region_from(target_region, source, source_region)?;
        }
        Ok(())
    }

    fn candidate_buffer(&self, slot: CellSlotId, before: InstanceEpoch) -> usize {
        1 - self.select_buffer(slot, before)
    }

    fn begin_rmw(
        &mut self,
        slot: CellSlotId,
        before: InstanceEpoch,
        working: InstanceEpoch,
    ) -> mech_core::MemoryRuntimeResult<(usize, bool)> {
        if let Some(candidate) = self
            .version(slot)
            .epochs
            .iter()
            .position(|tag| *tag == Some(working))
        {
            return Ok((candidate, false));
        }
        let published = self.select_buffer(slot, before);
        let candidate = 1 - published;
        let region = self.version(slot).region;
        let [left, right] = &mut self.buffers;
        if candidate == 0 {
            left.copy_region_from(region, right, region)?;
        } else {
            right.copy_region_from(region, left, region)?;
        }
        self.version_mut(slot).epochs[candidate] = Some(working);
        Ok((candidate, true))
    }

    fn materialize_state_slot(
        &mut self,
        target_slot: CellSlotId,
        target_region: ResidentRegion,
        source_slot: CellSlotId,
        source_region: ResidentRegion,
        before: InstanceEpoch,
        working: InstanceEpoch,
    ) -> mech_core::MemoryRuntimeResult<()> {
        let source_buffer = self.select_buffer(source_slot, working);
        let target_buffer = self.candidate_buffer(target_slot, before);
        if source_buffer == target_buffer {
            self.buffers[target_buffer].copy_region_within(target_region, source_region)?;
        } else if target_buffer == 0 {
            let [target, source] = &mut self.buffers;
            target.copy_region_from(target_region, source, source_region)?;
        } else {
            let [source, target] = &mut self.buffers;
            target.copy_region_from(target_region, source, source_region)?;
        }
        self.tag(target_slot, target_buffer, working);
        Ok(())
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

type ResidentCaptureFrames = [Box<[(ResidentReadLocation, super::OwnedResidentValue)]>];

fn lexical_capture<'a>(
    frames: &'a ResidentCaptureFrames,
    region: ResidentRegion,
) -> Option<&'a super::OwnedResidentValue> {
    frames
        .iter()
        .rev()
        .flat_map(|frame| frame.iter())
        .find_map(|(source, value)| {
            (*source == ResidentReadLocation::LexicalInput(region)).then_some(value)
        })
}

fn lexical_capture_f64<'a>(
    frames: &'a ResidentCaptureFrames,
    region: ResidentRegion,
) -> Option<&'a [f64]> {
    let super::OwnedResidentValue::F64(values) = lexical_capture(frames, region)? else {
        return None;
    };
    Some(values)
}

struct ScratchNodeInputs<'a> {
    locations: &'a [ResidentReadLocation],
    activation: &'a TypedResidentArena,
    input: &'a TypedResidentArena,
    captures: &'a ResidentCaptureFrames,
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
            ResidentReadLocation::LexicalInput(region) => {
                lexical_capture_f64(self.captures, region).or_else(|| self.input.read_f64(region))
            }
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
            ResidentReadLocation::LexicalInput(region) => lexical_capture(self.captures, region)
                .map(super::OwnedResidentValue::as_ref)
                .or_else(|| Some(self.input.read(region))),
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
    captures: &'a ResidentCaptureFrames,
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
            ResidentReadLocation::LexicalInput(region) => lexical_capture(self.captures, region)
                .map(super::OwnedResidentValue::as_ref)
                .or_else(|| Some(self.input.read(region))),
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
            ResidentReadLocation::LexicalInput(region) => {
                lexical_capture_f64(self.captures, region).or_else(|| self.input.read_f64(region))
            }
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
    captures: &'a ResidentCaptureFrames,
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
            ResidentReadLocation::LexicalInput(region) => lexical_capture(self.captures, region)
                .map(super::OwnedResidentValue::as_ref)
                .or_else(|| Some(self.input.read(region))),
            ResidentReadLocation::Scratch(region) => Some(self.scratch.read(region)),
            ResidentReadLocation::State { .. } => None,
        }
    }

    fn f64(&self, index: usize) -> Option<&[f64]> {
        match *self.locations.get(index)? {
            ResidentReadLocation::Constant(region) => self.activation.read_f64(region),
            ResidentReadLocation::Input(region) => self.input.read_f64(region),
            ResidentReadLocation::LexicalInput(region) => {
                lexical_capture_f64(self.captures, region).or_else(|| self.input.read_f64(region))
            }
            ResidentReadLocation::Scratch(region) => self.scratch.read_f64(region),
            ResidentReadLocation::State { .. } => None,
        }
    }
}

struct StateNodeInputs<'a> {
    locations: &'a [ResidentReadLocation],
    activation: &'a TypedResidentArena,
    input: &'a TypedResidentArena,
    captures: &'a ResidentCaptureFrames,
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
            ResidentReadLocation::LexicalInput(region) => lexical_capture(self.captures, region)
                .map(super::OwnedResidentValue::as_ref)
                .or_else(|| Some(self.input.read(region))),
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
            ResidentReadLocation::LexicalInput(region) => {
                lexical_capture_f64(self.captures, region).or_else(|| self.input.read_f64(region))
            }
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

enum ResidentCopyError {
    Layout,
    Memory(mech_core::MemoryRuntimeError),
}

impl ResidentCopyError {
    fn at(self, fallback: ResidentExecutionError) -> ResidentExecutionError {
        match self {
            Self::Layout => fallback,
            Self::Memory(error) => ResidentExecutionError::MemoryRuntime { error },
        }
    }
}

fn copy_input(
    arena: &mut TypedResidentArena,
    region: ResidentRegion,
    value: ResidentValueRef<'_>,
) -> Result<(), ResidentCopyError> {
    let scope = arena
        .prepare_payload_write(region)
        .map_err(ResidentCopyError::Memory)?;
    if let Some(scope) = &scope {
        scope
            .admit_copy(value, 0)
            .map_err(ResidentCopyError::Memory)?;
        scope.start();
    }
    let result = copy_input_unchecked(arena, region, value);
    arena
        .finish_payload_write(region, scope)
        .map_err(ResidentCopyError::Memory)?;
    result.map_err(|_| ResidentCopyError::Layout)
}

fn owned_resident_value(value: ResidentValueRef<'_>) -> super::OwnedResidentValue {
    match value {
        ResidentValueRef::Bool(values) => {
            super::OwnedResidentValue::Bool(values.to_vec().into_boxed_slice())
        }
        ResidentValueRef::Index(values) => {
            super::OwnedResidentValue::Index(values.to_vec().into_boxed_slice())
        }
        ResidentValueRef::F64(values) => {
            super::OwnedResidentValue::F64(values.to_vec().into_boxed_slice())
        }
        ResidentValueRef::String(values) => {
            super::OwnedResidentValue::String(values.to_vec().into_boxed_slice())
        }
        ResidentValueRef::Snapshot(values) => {
            super::OwnedResidentValue::Snapshot(values.to_vec().into_boxed_slice())
        }
    }
}

fn resident_frame_value_footprint(
    value: ResidentValueRef<'_>,
    schemas: &mech_core::SchemaTable,
    meter: &mut budget::ResidentBudgetMeter,
) -> Option<(u64, u64)> {
    // The frame Vec owns one entry and one boxed slice per local. Include an
    // allocator allowance per box so many small scalar locals cannot evade
    // the byte ceiling. The entry contains the enum and its slice header.
    const ALLOCATION_OVERHEAD: u64 = 16;
    let entry = u64::try_from(core::mem::size_of::<(
        ResidentRegion,
        super::OwnedResidentValue,
    )>())
    .ok()?
    .checked_add(ALLOCATION_OVERHEAD)?;
    let fixed = |len: usize, element: usize| {
        u64::try_from(len)
            .ok()?
            .checked_mul(u64::try_from(element).ok()?)
    };
    let (payload_bytes, payload_nodes) = match value {
        ResidentValueRef::Bool(values) => (fixed(values.len(), core::mem::size_of::<u8>())?, 0),
        ResidentValueRef::Index(values) => (fixed(values.len(), core::mem::size_of::<u64>())?, 0),
        ResidentValueRef::F64(values) => (fixed(values.len(), core::mem::size_of::<f64>())?, 0),
        ResidentValueRef::String(values) => {
            meter
                .charge_compute_work(u64::try_from(values.len()).ok()?)
                .ok()?;
            values.iter().try_fold(
                (fixed(values.len(), core::mem::size_of::<String>())?, 0u64),
                |(bytes, nodes), value| {
                    Some((
                        bytes
                            .checked_add(u64::try_from(value.len()).ok()?)?
                            .checked_add(ALLOCATION_OVERHEAD)?,
                        nodes.checked_add(1)?,
                    ))
                },
            )?
        }
        ResidentValueRef::Snapshot(values) => {
            meter
                .charge_compute_work(u64::try_from(values.len()).ok()?)
                .ok()?;
            values.iter().try_fold(
                (
                    fixed(values.len(), core::mem::size_of::<Option<Value>>())?,
                    0u64,
                ),
                |(bytes, nodes), value| {
                    let Some(value) = value else {
                        return Some((bytes, nodes));
                    };
                    let footprint =
                        budget::measure_canonical_value_footprint(meter, value, schemas).ok()?;
                    Some((
                        bytes.checked_add(footprint.retained_bytes)?,
                        nodes.checked_add(footprint.node_count)?,
                    ))
                },
            )?
        }
    };
    Some((
        entry.checked_add(payload_bytes)?,
        payload_nodes.checked_add(1)?,
    ))
}

fn copy_input_unchecked(
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
            for (target, source) in target.iter_mut().zip(source) {
                *target = source.clone();
            }
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

fn resident_values_equal(left: ResidentValueRef<'_>, right: ResidentValueRef<'_>) -> bool {
    match (left, right) {
        (ResidentValueRef::Bool(left), ResidentValueRef::Bool(right)) => left == right,
        (ResidentValueRef::Index(left), ResidentValueRef::Index(right)) => left == right,
        (ResidentValueRef::F64(left), ResidentValueRef::F64(right)) => left
            .iter()
            .zip(right)
            .all(|(left, right)| left.to_bits() == right.to_bits()),
        (ResidentValueRef::String(left), ResidentValueRef::String(right)) => left == right,
        (ResidentValueRef::Snapshot(_), ResidentValueRef::Snapshot(_)) => false,
        _ => false,
    }
}

fn rmw_outputs_equal(
    left: &TypedResidentArena,
    left_region: ResidentRegion,
    right: &TypedResidentArena,
    right_region: ResidentRegion,
    schemas: &mech_core::SchemaTable,
) -> bool {
    if let (ResidentValueRef::Snapshot(left), ResidentValueRef::Snapshot(right)) =
        (left.read(left_region), right.read(right_region))
    {
        return left.len() == right.len()
            && left
                .iter()
                .zip(right)
                .all(|(left, right)| match (left, right) {
                    (None, None) => true,
                    (Some(left), Some(right)) => {
                        left.schema() == right.schema()
                            && left.shape() == right.shape()
                            && schemas.get(left.schema()).is_some_and(|schema| {
                                mech_core::snapshot::schema_data_snapshot_eq(
                                    schema.body(),
                                    left.data(),
                                    right.data(),
                                )
                            })
                    }
                    _ => false,
                });
    }
    regions_equal(left, left_region, right, right_region)
}

fn scalar_token(value: ResidentValueRef<'_>) -> Option<(u8, u128, u128)> {
    use mech_core::ValueData;
    let data = match value {
        ResidentValueRef::Bool([value]) => return Some((0, u128::from(*value), 0)),
        ResidentValueRef::Index([value]) => return Some((1, u128::from(*value), 0)),
        ResidentValueRef::F64([value]) => return Some((2, u128::from(value.to_bits()), 0)),
        ResidentValueRef::String([value]) => return Some((3, u128::from(hash_string(value)), 0)),
        ResidentValueRef::Snapshot([Some(value)]) => value.data(),
        _ => return None,
    };
    // Preserve all scalar bits: narrowing i128/u128 or hashing a snapshot
    // would make distinct values indistinguishable to ExactScalar propagation.
    Some(match data {
        ValueData::U8(value) => (4, u128::from(*value), 0),
        ValueData::U16(value) => (5, u128::from(*value), 0),
        ValueData::U32(value) => (6, u128::from(*value), 0),
        ValueData::U64(value) => (7, u128::from(*value), 0),
        ValueData::U128(value) => (8, *value, 0),
        ValueData::I8(value) => (9, *value as u128, 0),
        ValueData::I16(value) => (10, *value as u128, 0),
        ValueData::I32(value) => (11, *value as u128, 0),
        ValueData::I64(value) => (12, *value as u128, 0),
        ValueData::I128(value) => (13, *value as u128, 0),
        ValueData::F32(value) => (14, u128::from(value.bits()), 0),
        ValueData::F64(value) => (15, u128::from(value.bits()), 0),
        ValueData::Complex32(value) => (
            16,
            u128::from(value.real().bits()),
            u128::from(value.imaginary().bits()),
        ),
        ValueData::Complex64(value) => (
            17,
            u128::from(value.real().bits()),
            u128::from(value.imaginary().bits()),
        ),
        ValueData::Rational64(value) => (
            18,
            value.numerator() as u128,
            u128::from(value.denominator()),
        ),
        ValueData::Bool(value) => (19, u128::from(*value), 0),
        ValueData::Id(value) => (20, u128::from(*value), 0),
        ValueData::Index(value) => (21, u128::from(*value), 0),
        _ => return None,
    })
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

fn clear_bit(words: &mut [u64], bit: usize) {
    words[bit / 64] &= !(1_u64 << (bit % 64));
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
    use crate::resident::general::comprehension::ActivatedCollectionStep;
    use crate::resident::general::{ResidentArenaSizes, StateVersion};
    use mech_core::ResidentShape;

    #[test]
    fn recursive_frame_cost_includes_entries_and_snapshot_nodes() {
        let snapshot = mech_core::ValueCell::from_exact(true)
            .unwrap()
            .snapshot()
            .unwrap();
        let schemas = snapshot.schemas().unwrap();
        let mut meter = budget::ResidentBudgetMeter::default();
        let (scalar_bytes, scalar_nodes) =
            resident_frame_value_footprint(ResidentValueRef::Bool(&[1]), &schemas, &mut meter)
                .unwrap();
        assert!(
            scalar_bytes
                > core::mem::size_of::<(ResidentRegion, super::super::OwnedResidentValue)>() as u64
        );
        assert_eq!(scalar_nodes, 1);
        let (snapshot_bytes, snapshot_nodes) = resident_frame_value_footprint(
            ResidentValueRef::Snapshot(&[Some(snapshot.clone())]),
            &schemas,
            &mut meter,
        )
        .unwrap();
        let footprint = snapshot.retained_footprint(&schemas).unwrap();
        assert!(snapshot_bytes > footprint.retained_bytes);
        assert_eq!(snapshot_nodes, footprint.node_count + 1);
        assert!(meter.estimate().compute_work() > 0);
    }

    #[cfg(feature = "source")]
    fn source_instance(source: &str) -> ReactiveInstance {
        source_instance_with_budget(source, None)
    }

    #[cfg(feature = "source")]
    fn source_instance_with_budget(
        source: &str,
        budget: Option<mech_core::ManagedMemoryBudget>,
    ) -> ReactiveInstance {
        use mech_syntax::document::parser::{
            canonical::parse_canonical_phase_2i_rule_for_test, rules,
        };
        use mech_syntax::document::{
            AstNode, DocumentId, ExpressionSyntax, ParseConfig, Revision, SyntaxNode, TextSnapshot,
        };
        fn expression(node: SyntaxNode) -> Option<ExpressionSyntax> {
            ExpressionSyntax::cast(node.clone()).or_else(|| node.children().find_map(expression))
        }
        let parsed = parse_canonical_phase_2i_rule_for_test(
            TextSnapshot::new(DocumentId(822), Revision(1), source).unwrap(),
            rules::EXPRESSION,
            ParseConfig::default(),
        )
        .unwrap();
        assert!(parsed.is_strictly_clean());
        let artifact = crate::CanonicalSourceFrontend
            .compile_expression(&expression(parsed.syntax()).unwrap())
            .unwrap()
            .compile_artifact()
            .unwrap();
        let mut catalog = mech_core::FunctionCatalogBuilder::new();
        crate::install_intrinsic_resident(&mut catalog).unwrap();
        crate::resident::activate_with_options(
            mech_core::ReactiveInstanceId::new(822, 1),
            &artifact,
            &catalog.build().unwrap(),
            &crate::resident::ActivationFacts::default(),
            crate::resident::ResidentActivationOptions {
                memory_budget: budget,
                ..Default::default()
            },
        )
        .unwrap()
    }

    #[cfg(feature = "source")]
    #[test]
    fn control_locals_retain_distinct_certified_turn_call_plans() {
        let instance =
            source_instance("flag<bool> ? | true => signal<f64> + 1 | false => signal<f64> * 2");
        let mut locals = 0;
        for (index, step) in instance.plan.steps.iter().enumerate() {
            let ActivatedTurnStep::Kernel(kernel) = step else {
                continue;
            };
            locals += 1;
            assert_ne!(kernel.memory_node, kernel.artifact_node);
            let turn = instance.workspace.fixed_turn_plans[index].as_ref().unwrap();
            assert_eq!(turn.node, kernel.memory_node);
            let call = turn
                .call
                .as_ref()
                .expect("a local operation cannot use an empty enclosing-node plan");
            assert_eq!(call.inputs.len(), 2);
            assert_eq!(call.outputs.len(), 1);
        }
        assert_eq!(locals, 2);
    }

    #[cfg(feature = "source")]
    #[test]
    fn nested_comprehension_operation_plan_counts_live_enclosing_arena() {
        let mut instance = source_instance("[(x + 1) | x <- signal<[f64]:1,1>]");
        let slot = instance.plan.inputs[0].slot;
        instance
            .turn(&[CapturedSignalInput {
                slot,
                value: ResidentValueRef::F64(&[1.0]),
            }])
            .unwrap();
        let operation = instance
            .plan
            .steps
            .iter()
            .position(|step| matches!(step, ActivatedTurnStep::Kernel(_)))
            .map(|index| ActivatedNodeIndex(index as u32))
            .expect("the comprehension contains one nested arithmetic operation");
        let before = instance.published_epoch();
        let working = before.checked_next().unwrap();
        assert!(matches!(
            instance.with_kernel_turn_plan_and_live_demand(
                operation,
                before,
                working,
                mech_core::RESIDENT_MAX_BYTES,
                1,
                |_| Ok(()),
            ),
            Err(ResidentExecutionError::Kernel {
                error: ResidentKernelError::InvalidShape,
                ..
            })
        ));
    }

    #[cfg(feature = "source")]
    #[test]
    fn nested_parameterized_operation_locals_use_turn_shaped_snapshot_storage() {
        let instance =
            source_instance("[[z | z <- rest] + rest | [head | rest] <- signal<[[f64]:1,3]:1,2>]");
        assert!(instance.plan.steps.iter().any(|step| {
            matches!(
                step,
                ActivatedTurnStep::Kernel(kernel)
                    if kernel.write.region.kind == ResidentValueKind::Snapshot
            )
        }));
    }

    #[cfg(feature = "source")]
    #[test]
    fn nested_comprehension_steps_retain_unconsumed_managed_local_demand() {
        let instance = source_instance(
            "[number + 1 | [head | rest] <- signal<[[f64]:1,3]:1,2>, number <- [1]]",
        );
        let retained = instance.plan.steps.iter().find_map(|step| {
            let ActivatedTurnStep::Comprehension(control) = step else {
                return None;
            };
            control.steps.iter().find_map(|step| {
                let ActivatedCollectionStep::Operation {
                    retained_local_count,
                    excluded_locals,
                    ..
                } = step
                else {
                    return None;
                };
                Some((
                    control
                        .locals
                        .get(..*retained_local_count as usize)
                        .expect("validated retained-local prefix"),
                    excluded_locals.as_ref(),
                ))
            })
        });
        let (retained, excluded) = retained.expect("ordinary operation inside the comprehension");
        assert!(
            retained
                .iter()
                .enumerate()
                .any(|(index, local)| local.kind == ResidentValueKind::Snapshot
                    && !excluded.contains(&(index as u32))),
            "the unconsumed rest binding remains live across the arithmetic step"
        );
    }

    #[cfg(feature = "source")]
    #[test]
    fn nested_comprehension_child_omits_live_captured_rest_input() {
        let instance =
            source_instance("[[z + 1 | z <- rest] | [head | rest] <- signal<[[f64]:1,3]:1,2>]");
        let (capture, child_reads) = instance
            .plan
            .steps
            .iter()
            .find_map(|step| {
                let ActivatedTurnStep::Comprehension(control) = step else {
                    return None;
                };
                let capture = instance.plan.reads
                    [control.reads.start as usize..control.reads.end as usize]
                    .iter()
                    .copied()
                    .find(|location| {
                        matches!(location, ResidentReadLocation::Scratch(region)
                            if region.kind == ResidentValueKind::Snapshot)
                    })?;
                let child = control.steps.iter().find_map(|step| {
                    let ActivatedCollectionStep::Operation { node, .. } = step else {
                        return None;
                    };
                    instance.plan.steps[node.get() as usize].memory_site()
                })?;
                Some((capture, child.reads))
            })
            .expect("nested comprehension captures the outer rest binding");
        assert!(
            !instance.plan.reads[child_reads.start as usize..child_reads.end as usize]
                .contains(&capture),
            "the child call plan cannot account for its wrapper's live capture"
        );
    }

    #[cfg(feature = "source")]
    #[test]
    fn selected_structural_arm_steps_retain_unconsumed_binding_demand() {
        let instance = source_instance("[1 2 3] ? | [head | rest] => head + 1 | * => 0");
        let control = instance.plan.steps.iter().find_map(|step| {
            let ActivatedTurnStep::Match(control) = step else {
                return None;
            };
            control
                .arms
                .iter()
                .find(|arm| !arm.binding_regions.is_empty())
                .map(|arm| (control, arm))
        });
        let (_, arm) = control.expect("structural match arm");
        let (binding_index, binding) = arm
            .binding_regions
            .iter()
            .copied()
            .enumerate()
            .find(|(_, binding)| binding.kind == ResidentValueKind::Snapshot)
            .expect("managed rest binding");
        assert_eq!(binding.kind, ResidentValueKind::Snapshot);
        assert_eq!(arm.body.locals[binding_index], binding);
        assert!(arm.body.steps.iter().all(|step| {
            step.retained_local_count > binding_index as u32
                && !step.excluded_locals.contains(&(binding_index as u32))
        }));
    }

    #[cfg(feature = "source")]
    #[test]
    fn match_publication_counts_unused_managed_binding() {
        let mut instance =
            source_instance("(\"retained\", true) ? | (unused, true) => \"yield\" | * => \"\"");
        let operation = instance
            .plan
            .steps
            .iter()
            .position(|step| matches!(step, ActivatedTurnStep::Match(_)))
            .map(|index| ActivatedNodeIndex(index as u32))
            .expect("match operation");
        let ActivatedTurnStep::Match(control) = &instance.plan.steps[operation.get() as usize]
        else {
            unreachable!()
        };
        let budget_node = control.budget_node;
        assert!(control.arms[0].body.steps.is_empty());
        assert!(
            control.arms[0]
                .body
                .locals
                .iter()
                .any(|local| local.kind == ResidentValueKind::String)
        );
        // Find the largest enclosing demand admitted by the original wrapper.
        // The selected binding pushes final publication beyond that limit.
        let admitted = |bytes| {
            let mut facts = crate::memory_planner::TurnMemoryFacts::default();
            facts.additional_demand.turn_peak_bytes = bytes;
            crate::memory_planner::plan_current_resident_turn(
                &instance.plan.memory_plan,
                budget_node,
                &facts,
            )
            .is_ok_and(|plan| plan.budget_violations.is_empty())
        };
        assert!(admitted(0));
        let mut low = 0;
        let mut high = mech_core::RESIDENT_MAX_BYTES;
        while low < high {
            let middle = low + (high - low + 1) / 2;
            if admitted(middle) {
                low = middle;
            } else {
                high = middle - 1;
            }
        }
        let before = instance.published_epoch();
        let working = before.checked_next().unwrap();
        let result = instance.execute_match_expression(
            operation,
            before,
            working,
            &mut ResidentStructuralProbe::default(),
            low,
            0,
        );
        assert!(matches!(
            result,
            Err(ResidentExecutionError::Kernel {
                error: ResidentKernelError::InvalidShape,
                ..
            })
        ));
    }

    #[cfg(feature = "source")]
    #[test]
    fn nested_match_operations_retain_managed_inputs_and_prior_output() {
        let instance = source_instance("[(item ? | * => item) | item <- signal<[string]:1,1>]");
        let (control, operation) = instance
            .plan
            .steps
            .iter()
            .find_map(|step| {
                let ActivatedTurnStep::Comprehension(control) = step else {
                    return None;
                };
                control.steps.iter().find_map(|operation| {
                    let ActivatedCollectionStep::Operation { node, .. } = operation else {
                        return None;
                    };
                    matches!(
                        instance.plan.steps[node.get() as usize],
                        ActivatedTurnStep::Match(_)
                    )
                    .then_some((control.as_ref(), operation))
                })
            })
            .expect("nested match operation");
        let ActivatedCollectionStep::Operation {
            retained_local_count,
            excluded_locals,
            ..
        } = operation
        else {
            unreachable!()
        };
        assert_eq!(*retained_local_count as usize, control.locals.len());
        assert!(excluded_locals.is_empty());
        assert!(
            control.locals.iter().all(|local| matches!(
                local.kind,
                ResidentValueKind::String | ResidentValueKind::Snapshot
            )),
            "the match scrutinee and previous output are both managed locals"
        );
    }

    #[cfg(feature = "source")]
    #[test]
    fn nested_match_plan_counts_live_enclosing_arena() {
        let mut instance =
            source_instance("[(item ? | 1 => 10 | * => 20) | item <- signal<[f64]:1,1>]");
        let slot = instance.plan.inputs[0].slot;
        instance
            .turn(&[CapturedSignalInput {
                slot,
                value: ResidentValueRef::F64(&[1.0]),
            }])
            .unwrap();
        let operation = instance
            .plan
            .steps
            .iter()
            .position(|step| matches!(step, ActivatedTurnStep::Match(_)))
            .map(|index| ActivatedNodeIndex(index as u32))
            .expect("the comprehension contains one nested match operation");
        let before = instance.published_epoch();
        let working = before.checked_next().unwrap();
        let result = instance.execute_step_with_live_demand(
            operation,
            before,
            working,
            &mut ResidentStructuralProbe::default(),
            mech_core::RESIDENT_MAX_BYTES + 1,
            0,
        );
        assert!(
            matches!(
                result,
                Err(ResidentExecutionError::Kernel {
                    error: ResidentKernelError::InvalidShape,
                    ..
                })
            ),
            "{result:?}"
        );
    }

    #[cfg(feature = "source")]
    #[test]
    fn nested_selected_match_steps_count_live_enclosing_arena() {
        let mut instance = source_instance("true ? | true => signal<f64> + 1 | false => signal");
        let slot = instance.plan.inputs[0].slot;
        instance
            .turn(&[CapturedSignalInput {
                slot,
                value: ResidentValueRef::F64(&[1.0]),
            }])
            .unwrap();
        let operation = instance
            .plan
            .steps
            .iter()
            .position(|step| matches!(step, ActivatedTurnStep::Match(_)))
            .map(|index| ActivatedNodeIndex(index as u32))
            .expect("match operation");
        let before = instance.published_epoch();
        let working = before.checked_next().unwrap();
        let result = instance.execute_match_expression(
            operation,
            before,
            working,
            &mut ResidentStructuralProbe::default(),
            mech_core::RESIDENT_MAX_BYTES,
            0,
        );
        assert!(matches!(
            result,
            Err(ResidentExecutionError::Kernel {
                error: ResidentKernelError::InvalidShape,
                ..
            })
        ));
    }

    #[cfg(feature = "source")]
    #[test]
    fn nested_comprehension_plan_counts_live_enclosing_arena() {
        let mut instance =
            source_instance("[[z | z <- [1 2], z <= item] | item <- signal<[f64]:1,1>]");
        let slot = instance.plan.inputs[0].slot;
        instance
            .turn(&[CapturedSignalInput {
                slot,
                value: ResidentValueRef::F64(&[1.0]),
            }])
            .unwrap();
        let operation = instance
            .plan
            .steps
            .iter()
            .position(|step| matches!(step, ActivatedTurnStep::Comprehension(_)))
            .map(|index| ActivatedNodeIndex(index as u32))
            .expect("the outer comprehension contains one nested comprehension");
        let before = instance.published_epoch();
        let working = before.checked_next().unwrap();
        assert!(matches!(
            instance.execute_step_with_live_demand(
                operation,
                before,
                working,
                &mut ResidentStructuralProbe::default(),
                mech_core::RESIDENT_MAX_BYTES,
                1,
            ),
            Err(ResidentExecutionError::Kernel {
                error: ResidentKernelError::InvalidShape,
                ..
            })
        ));
    }

    #[cfg(feature = "source")]
    #[test]
    fn match_payload_locals_are_released_after_commit_failure_and_abort() {
        for source in [
            "(values<[f64]:1,2>, flag<bool> ? | true => ((signal<f64>, true), 1) | false => ((signal, false), values[index<f64>]))",
            "(values<[f64]:1,2>, flag<bool> ? | true => (signal<f64> ? | * => ((signal, true), 1)) | false => (signal ? | * => ((signal, false), values[index<f64>])))",
        ] {
            for managed in [false, true] {
                let mut instance = source_instance_with_budget(
                    source,
                    managed.then(|| mech_core::ManagedMemoryBudget::new(1024 * 1024)),
                );
                let slots = instance
                    .plan
                    .inputs
                    .iter()
                    .map(|input| input.slot)
                    .collect::<Vec<_>>();
                let inputs = |flag, index| {
                    [
                        CapturedSignalInput {
                            slot: slots[0],
                            value: ResidentValueRef::F64(&[1.0, 2.0]),
                        },
                        CapturedSignalInput {
                            slot: slots[1],
                            value: ResidentValueRef::Bool(flag),
                        },
                        CapturedSignalInput {
                            slot: slots[2],
                            value: ResidentValueRef::F64(&[7.0]),
                        },
                        CapturedSignalInput {
                            slot: slots[3],
                            value: ResidentValueRef::F64(index),
                        },
                    ]
                };
                let released = |instance: &ReactiveInstance| {
                    let mut payloads = 0;
                    for step in &instance.plan.steps {
                        let ActivatedTurnStep::Match(control) = step else {
                            continue;
                        };
                        for region in &control.locals {
                            if let ResidentValueRef::Snapshot(values) =
                                instance.workspace.scratch.read(*region)
                            {
                                payloads += 1;
                                assert!(
                                    values.iter().all(Option::is_none),
                                    "match local retained a payload"
                                );
                            }
                        }
                    }
                    assert!(
                        payloads >= 4,
                        "both arms must construct managed local payloads"
                    );
                };
                instance.turn(&inputs(&[1], &[3.0])).unwrap();
                released(&instance);
                let epoch = instance.published_epoch();
                let output = instance
                    .copied_output(0)
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap();
                assert!(instance.turn(&inputs(&[0], &[3.0])).is_err());
                released(&instance);
                assert_eq!(instance.published_epoch(), epoch);
                instance
                    .prepare_turn(&inputs(&[0], &[2.0]))
                    .unwrap()
                    .abort();
                released(&instance);
                assert_eq!(instance.published_epoch(), epoch);
                assert_eq!(
                    instance
                        .copied_output(0)
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    output
                );
                instance.turn(&inputs(&[0], &[2.0])).unwrap();
                released(&instance);
            }
        }
    }

    #[cfg(feature = "source")]
    #[test]
    fn structural_arm_clone_admission_uses_peak_depth() {
        let instance = source_instance(
            "(\"payload\", true) ? | (text, false) => 0 | (text, true) => 1 | * => 2",
        );
        let arms = instance
            .plan
            .steps
            .iter()
            .find_map(|step| match step {
                ActivatedTurnStep::Match(matched) => Some(matched.arms.as_ref()),
                _ => None,
            })
            .expect("structural match step");
        let depths = arms
            .iter()
            .filter_map(|arm| match &arm.pattern {
                super::super::ActivatedMatchPattern::Structural { clone_depth, .. } => {
                    Some(*clone_depth)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(depths.len(), 2);
        assert!(depths.iter().all(|depth| *depth > 0));
        assert_eq!(
            peak_structural_clone_depth(arms),
            *depths.iter().max().unwrap()
        );
        assert!(peak_structural_clone_depth(arms) < depths.iter().sum());
    }

    #[cfg(feature = "source")]
    #[test]
    fn rejected_structural_arm_releases_its_payload_bindings_before_fallthrough() {
        let mut instance = source_instance(
            "(\"payload\", 2) ? | (first, 0) => first | (second, *) => second | * => \"\"",
        );
        let node_index = instance
            .plan
            .steps
            .iter()
            .position(|step| matches!(step, ActivatedTurnStep::Match(_)))
            .and_then(|index| u32::try_from(index).ok())
            .map(ActivatedNodeIndex)
            .expect("structural match step");
        let (rejected, selected) = {
            let ActivatedTurnStep::Match(control) = &instance.plan.steps[node_index.get() as usize]
            else {
                unreachable!()
            };
            assert_eq!(control.arms[0].binding_regions.len(), 1);
            assert_eq!(control.arms[1].binding_regions.len(), 1);
            (
                control.arms[0].binding_regions[0],
                control.arms[1].binding_regions[0],
            )
        };
        let before = instance.published_epoch();
        let working = before.checked_next().unwrap();
        instance
            .execute_match_expression(
                node_index,
                before,
                working,
                &mut ResidentStructuralProbe::default(),
                0,
                0,
            )
            .unwrap();

        assert!(matches!(
            instance.workspace.scratch.read(rejected),
            ResidentValueRef::String([value]) if value.is_empty()
        ));
        assert!(matches!(
            instance.workspace.scratch.read(selected),
            ResidentValueRef::String([value]) if value == "payload"
        ));
    }

    #[cfg(feature = "source")]
    #[test]
    fn selected_match_calls_share_a_budget_that_resets_each_turn() {
        fn metered(
            kernel: &mech_core::BoundResidentKernel,
            inputs: &dyn ResidentKernelInputs,
            output: ResidentValueMut<'_>,
        ) -> Result<bool, ResidentKernelError> {
            // A cheap executor with a declared workload exercises real control
            // admission without allocating a maximum-sized numerical fixture.
            budget::ResidentBudgetMeter::default().charge_compute_work(kernel.parameters()[0])?;
            let ResidentValueMut::F64(output) = output else {
                unreachable!()
            };
            output[0] = inputs.f64(0).unwrap()[0] + 1.0;
            Ok(true)
        }
        for source in [
            "flag<bool> ? | true => math/neg(signal<f64>) + 1 | false => signal",
            "flag<bool> ? | true => (signal<f64> ? | item => math/neg(item)) + 1 | false => signal",
        ] {
            let mut instance = source_instance(source);
            let slots = instance
                .plan
                .inputs
                .iter()
                .map(|input| input.slot)
                .collect::<Vec<_>>();
            let inputs = |flag| {
                [
                    CapturedSignalInput {
                        slot: slots[0],
                        value: ResidentValueRef::Bool(flag),
                    },
                    CapturedSignalInput {
                        slot: slots[1],
                        value: ResidentValueRef::F64(&[7.0]),
                    },
                ]
            };
            let install = |instance: &mut ReactiveInstance, work| {
                let mut calls = 0;
                for step in &mut instance.plan.steps {
                    if let ActivatedTurnStep::Kernel(kernel) = step {
                        kernel.kernel =
                            mech_core::BoundResidentKernel::new(metered, Box::new([work]));
                        calls += 1;
                    }
                }
                assert_eq!(calls, 2);
            };
            install(&mut instance, mech_core::RESIDENT_MAX_COMPUTE_WORK / 2 + 1);
            instance.turn(&inputs(&[0])).unwrap();
            let epoch = instance.published_epoch();
            assert!(matches!(
                instance.turn(&inputs(&[1])),
                Err(ResidentExecutionError::Kernel {
                    error: ResidentKernelError::InvalidShape,
                    ..
                })
            ));
            assert_eq!(instance.published_epoch(), epoch);
            assert_eq!(
                instance
                    .copied_output(0)
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                mech_core::ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_f64(7.0))
            );
            install(&mut instance, mech_core::RESIDENT_MAX_COMPUTE_WORK / 2);
            for _ in 0..2 {
                instance.turn(&inputs(&[1])).unwrap();
                assert_eq!(
                    instance
                        .copied_output(0)
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    mech_core::ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_f64(9.0))
                );
            }
        }
    }

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
                TypedResidentArena::allocate_projected_sizes(sizes),
                TypedResidentArena::allocate_projected_sizes(sizes),
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
        let (candidate, seeded) = state
            .begin_rmw(
                CellSlotId::new(0),
                InstanceEpoch::ZERO,
                InstanceEpoch::new(1),
            )
            .unwrap();
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
        let (first, seeded_first) = state
            .begin_rmw(CellSlotId::new(0), InstanceEpoch::ZERO, working)
            .unwrap();
        state.buffers[first].f64s[0] += 2.0;
        let (second, seeded_second) = state
            .begin_rmw(CellSlotId::new(0), InstanceEpoch::ZERO, working)
            .unwrap();
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

        let (candidate, _) = state.begin_rmw(slot, InstanceEpoch::ZERO, working).unwrap();
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
        let (first, _) = state
            .begin_rmw(slot0, InstanceEpoch::ZERO, InstanceEpoch::new(1))
            .unwrap();
        state.buffers[first].f64s[0] = 11.0;
        let (second, _) = state
            .begin_rmw(slot1, InstanceEpoch::new(1), InstanceEpoch::new(2))
            .unwrap();
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
        let (candidate, _) = state.begin_rmw(slot, InstanceEpoch::ZERO, working).unwrap();
        assert!(state.same_at(slot, candidate, InstanceEpoch::ZERO));
        state.abort(working);
        assert_eq!(scalar(&state, 0, 0), 10.0);
    }

    #[test]
    fn match_conversion_peak_counts_prior_and_candidate_snapshot_nodes()
    -> Result<(), ResidentKernelError> {
        let prior = 32_770;
        let candidate = 32_770;
        let peak = match_conversion_peak_retained_nodes(prior, candidate).unwrap();
        assert_eq!(peak, 65_540);
        assert_eq!(
            budget::PreparedKernel::new(
                (),
                budget::resident_cost! {
                    retained_nodes: peak,
                    ..budget::KernelCostEstimate::default()
                },
            )
            .admit_control()
            .map(|permit| permit.into_plan()),
            Err(ResidentKernelError::InvalidShape)
        );
        assert_eq!(
            match_conversion_peak_retained_nodes(u64::MAX, 1),
            Err(ResidentKernelError::InvalidShape)
        );
        Ok(())
    }

    #[test]
    fn recursive_inventory_cost_includes_live_local_demand() {
        let cost = recursive_inventory_cost(1_024, 17, 96).unwrap();
        assert_eq!(cost.compute_work(), 96);
        assert_eq!(cost.temporary_bytes(), 1_120);
        assert_eq!(cost.retained_nodes(), 17);
        assert_eq!(
            recursive_inventory_cost(u64::MAX, 0, 1),
            Err(ResidentKernelError::InvalidShape)
        );
    }

    #[cfg(feature = "source")]
    #[test]
    fn match_conversion_prior_footprint_charges_the_complete_borrowed_traversal()
    -> Result<(), ResidentKernelError> {
        let mut instance =
            source_instance("true ? | true => signal<[string]:1,2> | false => [\"\" \"\"]");
        let slot = instance.plan.inputs[0].slot;
        let prior = ["p".repeat(25 * 1024), "q".repeat(25 * 1024)];
        instance
            .turn(&[CapturedSignalInput {
                slot,
                value: ResidentValueRef::String(&prior),
            }])
            .unwrap();
        let output = instance.copied_output(0).unwrap();
        let (nodes, work) =
            match_conversion_prior_footprint(Some(&output), &instance.plan.schemas).unwrap();
        assert!(nodes >= 5);
        assert!(work.comparison_work() >= 50 * 1024);

        let candidate_work = 20 * 1024;
        assert_eq!(
            budget::PreparedKernel::new(
                (),
                budget::resident_cost! {
                    comparison_work: work.comparison_work() + candidate_work,
                    compute_work: work.compute_work() + candidate_work,
                    ..budget::KernelCostEstimate::default()
                },
            )
            .admit_control()
            .map(|permit| permit.into_plan()),
            Err(ResidentKernelError::InvalidShape)
        );
        Ok(())
    }
}
