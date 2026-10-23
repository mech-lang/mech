//! The individual maintained-leaf boundary, not a program dispatcher.
use super::NativeCollectionCall;
use crate::memory_planner::{ProgramMemoryPlan, TurnMemoryFacts, TurnMemoryPlan, plan_turn_memory};
use crate::resident::budget::native_turn_plan_cache::{NativeTurnPlanCache, with_cached_turn_plan};
use crate::resident::budget::{self, NativeLibraryWorkBudget, payload::ResidentPayloadOwner};
use crate::resident::general::{ResidentRegion, live};
use mech_core::{
    CurrentMemoryFootprint, MemoryBudgetViolation, MemoryObjectOwner, MemoryPlanError,
    MemoryRuntimeError, ResidentKernelError, ResidentKernelInputs, ResidentShape,
    ResidentValueKind, ResidentValueMut, ResidentValueRef, ResourceDemand, Value,
};
use std::{rc::Rc, sync::Arc};

#[path = "native_live_measurement.rs"]
mod measurement;

#[cfg(all(test, any(feature = "full_source", feature = "full_compiler")))]
#[path = "native_live_measurement_tests.rs"]
mod measurement_tests;

/// An optional optimization of this already-bound maintained leaf. A capacity
/// refusal is sticky until the binding geometry itself is replaced.
#[derive(Debug, Default)]
pub(super) enum NativePlanCacheState {
    #[default]
    Cold,
    Uncached,
    Cached(Rc<NativeTurnPlanCache>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativeCollectionInvocationError {
    Binding(crate::native_artifact::NativeArtifactLoweringError),
    Kernel(ResidentKernelError),
    Plan(MemoryPlanError),
    Memory(MemoryRuntimeError),
    InvalidInput,
    UnadmittedValue,
}
impl From<ResidentKernelError> for NativeCollectionInvocationError {
    fn from(error: ResidentKernelError) -> Self {
        Self::Kernel(error)
    }
}
impl From<MemoryPlanError> for NativeCollectionInvocationError {
    fn from(error: MemoryPlanError) -> Self {
        Self::Plan(error)
    }
}
impl From<MemoryRuntimeError> for NativeCollectionInvocationError {
    fn from(error: MemoryRuntimeError) -> Self {
        Self::Memory(error)
    }
}
impl std::fmt::Display for NativeCollectionInvocationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "native collection invocation: {self:?}")
    }
}
impl std::error::Error for NativeCollectionInvocationError {}

#[cfg(all(test, any(feature = "full_source", feature = "full_compiler")))]
#[path = "native_data_footprint_tests.rs"]
mod data_footprint_tests;

/// Only constructor-private maintained families reach this envelope, including
/// nested scalar/map state carriers. It conservatively covers planner coexistence, deep copies of
/// the actual port descriptors, shared schema-arena wrappers, borrowed
/// argument tapes and BTree facts; it is not a canonical payload bound.
/// Runtime admission reserves it before any of those objects are constructed.
pub(super) fn metadata_envelope(call: &mech_core::CallMemoryPlan) -> Option<u64> {
    let ports = u64::try_from(call.inputs.len().checked_add(call.outputs.len())?).ok()?;
    let entries = u64::try_from(
        call.allocations
            .len()
            .checked_add(call.deferred_witnesses.len())?
            .checked_add(call.transactions.len())?,
    )
    .ok()?;
    metadata_envelope_for(
        call.inputs
            .iter()
            .chain(&call.outputs)
            .map(|port| port.descriptor.schema()),
        ports,
        entries,
    )
}

pub(super) fn metadata_envelope_for<'a>(
    schemas: impl IntoIterator<Item = &'a mech_core::Schema>,
    ports: u64,
    entries: u64,
) -> Option<u64> {
    descriptor_envelope_for(schemas, ports)?.checked_add(
        ports
            .checked_add(entries)?
            .checked_add(4)?
            .checked_mul(2048)?
            .checked_mul(16)?,
    )
}

/// The unchanged descriptor/schema component, also charged on warm native
/// plan hits. Split ownership, not a replacement price or smaller coefficient.
pub(super) fn descriptor_envelope_for<'a>(
    schemas: impl IntoIterator<Item = &'a mech_core::Schema>,
    ports: u64,
) -> Option<u64> {
    // SchemaTable::clone now shares its immutable entry allocation. However
    // Schema/ResolvedValueDescriptor/BoundCall/plan/template clones still own
    // their port-schema trees. Preserve their full heap/growth/coexistence
    // envelope; do not replace every schema-related allocation with Arc size.
    let descriptors = schemas.into_iter().try_fold(
        mech_core::SchemaTable::clone_metadata_allocation_bound_bytes(),
        |total, schema| total.checked_add(schema.clone_allocation_bound_bytes()?),
    )?;
    let schema_copies = ports.checked_mul(4)?.checked_add(4)?;
    descriptors.checked_mul(schema_copies)?.checked_mul(16)
}

#[cfg(test)]
mod metadata_tests {
    use super::metadata_envelope_for;
    use mech_core::{SchemaBody, SchemaDraft, SchemaTableBuilder};

    fn schema(body: SchemaBody) -> mech_core::Schema {
        SchemaDraft {
            body,
            dimension_parameters: Box::new([]),
        }
        .finalize()
        .unwrap()
    }

    #[test]
    fn binding_envelope_tracks_real_descriptor_clones_not_unrelated_arena_entries() {
        let port = schema(SchemaBody::String);
        let mut builder = SchemaTableBuilder::new();
        builder.insert(port.clone()).unwrap();
        let small = builder.clone().finish().unwrap().table;
        builder
            .insert(schema(SchemaBody::Tuple(
                vec![SchemaBody::Bool; 2_048].into_boxed_slice(),
            )))
            .unwrap();
        let large = builder.finish().unwrap().table;
        assert!(
            large.clone_allocation_bound_bytes().unwrap()
                > small.clone_allocation_bound_bytes().unwrap()
        );
        let small_port = small
            .entries()
            .find(|entry| entry.schema() == &port)
            .unwrap()
            .schema();
        let large_port = large
            .entries()
            .find(|entry| entry.schema() == &port)
            .unwrap()
            .schema();
        assert_eq!(
            metadata_envelope_for([small_port], 1, 19),
            metadata_envelope_for([large_port], 1, 19)
        );
        let wide = schema(SchemaBody::Tuple(
            vec![SchemaBody::Bool; 2_048].into_boxed_slice(),
        ));
        let base = metadata_envelope_for([&port], 1, 19).unwrap();
        let grown = metadata_envelope_for([&wide], 1, 19).unwrap();
        assert!(grown - base >= wide.clone_allocation_bound_bytes().unwrap());
        assert!(metadata_envelope_for([&port], u64::MAX, 19).is_none());
    }
}

struct Inputs<'a>(&'a [Option<Value>]);
impl ResidentKernelInputs for Inputs<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn get(&self, index: usize) -> Option<ResidentValueRef<'_>> {
        self.0
            .get(index)
            .map(|value| ResidentValueRef::Snapshot(core::slice::from_ref(value)))
    }
}

fn measurement_error(error: ResidentKernelError) -> MemoryPlanError {
    match error {
        ResidentKernelError::ResourceLimit {
            dimension,
            required,
            limit,
            ..
        } => MemoryPlanError::TargetLimitExceeded {
            violation: MemoryBudgetViolation {
                owner: MemoryObjectOwner::DirectCallPort {
                    call: 0,
                    direction: mech_core::PortDirection::Input,
                    port: 0,
                },
                dimension,
                required,
                limit,
            },
        },
        _ => MemoryPlanError::DescriptorMismatch,
    }
}

impl NativeCollectionCall {
    /// Invoke exactly this maintained leaf with already-admitted canonical
    /// values. The generated program's current-visit/CFG check is the caller's
    /// separate responsibility; this API never evaluates or schedules a graph.
    /// The old result is replaced only after complete payload ownership transfer.
    pub fn invoke(
        &self,
        inputs: &[Option<Value>],
        output: &mut Option<Value>,
        work: &NativeLibraryWorkBudget,
    ) -> Result<bool, NativeCollectionInvocationError> {
        work.with_call_limits(
            self.memory
                .target
                .limits
                .max_compute_work
                .ok_or(NativeCollectionInvocationError::InvalidInput)?,
            self.memory
                .target
                .limits
                .max_comparison_work
                .ok_or(NativeCollectionInvocationError::InvalidInput)?,
            || self.invoke_scoped(inputs, output, work, None),
        )?
    }

    pub(super) fn invoke_cached(
        &self,
        cache: &mut NativePlanCacheState,
        inputs: &[Option<Value>],
        output: &mut Option<Value>,
        work: &NativeLibraryWorkBudget,
    ) -> Result<bool, NativeCollectionInvocationError> {
        work.with_call_limits(
            self.memory
                .target
                .limits
                .max_compute_work
                .ok_or(NativeCollectionInvocationError::InvalidInput)?,
            self.memory
                .target
                .limits
                .max_comparison_work
                .ok_or(NativeCollectionInvocationError::InvalidInput)?,
            || {
                if let NativePlanCacheState::Cached(current) = cache {
                    if !current.validate(self.memory.target.limits, work) {
                        return Err(NativeCollectionInvocationError::InvalidInput);
                    }
                }
                if matches!(cache, NativePlanCacheState::Cold) {
                    // Only fixed metadata is inspected by this one-time
                    // optional-capacity decision. It never observes payload.
                    let guard_bytes = (core::mem::size_of::<NativePlanCacheState>()
                        + core::mem::size_of::<ResourceDemand>()
                        + core::mem::size_of::<mech_core::MemoryBudgetLimits>()
                        + core::mem::size_of::<mech_core::ManagedMemoryReservation>()
                        + core::mem::size_of::<mech_core::ManagedNodeReservation>()
                        + 3 * core::mem::size_of::<u64>())
                        as u64;
                    work.charge(
                        guard_bytes
                            .checked_mul(8)
                            .and_then(|n| n.checked_add(256))
                            .ok_or(ResidentKernelError::InvalidShape)?,
                        0,
                    )?;
                    let retained = NativeTurnPlanCache::retained_bytes(
                        &self.memory,
                        self.invocation_metadata_bytes,
                    )
                    .ok_or(ResidentKernelError::InvalidShape)?;
                    let finite = match retained.checked_add(self.invocation_metadata_bytes) {
                        Some(peak) => match live::check_measurement_with_limits(
                            CurrentMemoryFootprint::default(),
                            ResourceDemand {
                                turn_peak_bytes: peak,
                                retained_nodes: NativeTurnPlanCache::retained_data_nodes(
                                    &self.memory,
                                )
                                .ok_or(ResidentKernelError::InvalidShape)?,
                                ..ResourceDemand::default()
                            },
                            self.memory.target.limits,
                        ) {
                            Ok(()) => true,
                            Err(MemoryPlanError::TargetLimitExceeded { violation })
                                if matches!(
                                    violation.dimension,
                                    mech_core::MemoryBudgetDimension::TemporaryBytes
                                        | mech_core::MemoryBudgetDimension::RetainedNodes
                                ) =>
                            {
                                false
                            }
                            Err(error) => return Err(error.into()),
                        },
                        None => false,
                    };
                    let temporary = self
                        .memory
                        .target
                        .limits
                        .max_temporary_bytes
                        .ok_or(NativeCollectionInvocationError::InvalidInput)?;
                    // A reservation of C alone can fit yet starve the mandatory
                    // current scope. Probe full C + the unchanged finite nested
                    // temporary allowance + the two concrete scope Rc owners.
                    // No code executes while these probe bytes are held.
                    let probe_bytes = retained.checked_add(temporary).and_then(|n| {
                        n.checked_add(budget::payload::native_scope_metadata_bytes())
                    });
                    let mut probe = if finite {
                        match probe_bytes {
                            Some(bytes) => match work.memory_budget().reserve_capacity(bytes) {
                                Ok(reservation) => Some(reservation),
                                Err(MemoryRuntimeError::BudgetExceeded { .. }) => None,
                                Err(error) => return Err(error.into()),
                            },
                            None => None,
                        }
                    } else {
                        None
                    };
                    // This is an optional metadata cache. Its fixed owner and
                    // weak-data witnesses must fit the same shared node account
                    // before allocation; a denied cache may use the ordinary path.
                    let nodes = if probe.is_some() {
                        let count = 1u64.checked_add(NativeTurnPlanCache::retained_data_nodes(
                            &self.memory).ok_or(ResidentKernelError::InvalidShape)?)
                            .ok_or(ResidentKernelError::InvalidShape)?;
                        match work.memory_budget().reserve_nodes(count) {
                            Ok(nodes) => Some(nodes),
                            Err(MemoryRuntimeError::BudgetExceeded { .. }) => None,
                            Err(error) => return Err(error.into()),
                        }
                    } else { None };
                    if let (Some(headroom), Some(nodes)) = (probe.as_mut(), nodes) {
                        let reservation = headroom.split_capacity(retained)?;
                        let owner = NativeTurnPlanCache::construction_owner_bytes(&self.memory)
                            .ok_or(ResidentKernelError::InvalidShape)?;
                        // Charge the complete unchanged construction/retirement
                        // envelope only when actually attempting cache creation.
                        work.charge(
                            self.invocation_metadata_bytes
                                .checked_add(
                                    owner
                                        .checked_mul(8)
                                        .and_then(|n| n.checked_add(256))
                                        .ok_or(ResidentKernelError::InvalidShape)?,
                                )
                                .ok_or(ResidentKernelError::InvalidShape)?,
                            0,
                        )?;
                        let descriptor_work = descriptor_envelope_for(
                            self.memory
                                .inputs
                                .iter()
                                .chain(&self.memory.outputs)
                                .map(|p| p.descriptor.schema()),
                            (self.memory.inputs.len() + self.memory.outputs.len()) as u64,
                        )
                        .ok_or(ResidentKernelError::InvalidShape)?;
                        *cache = NativePlanCacheState::Cached(NativeTurnPlanCache::new(
                            &self.memory,
                            self.invocation_metadata_bytes,
                            descriptor_work,
                            reservation,
                            nodes,
                            work,
                        )?);
                    } else {
                        *cache = NativePlanCacheState::Uncached;
                    }
                    // Unused headroom is not stored or credited to a payload.
                    drop(probe);
                }
                let selected = match cache {
                    NativePlanCacheState::Cached(current) => Some(current.clone()),
                    NativePlanCacheState::Uncached => None,
                    NativePlanCacheState::Cold => {
                        return Err(NativeCollectionInvocationError::InvalidInput);
                    }
                };
                // Same maintained planner/kernel, current facts, finite guards,
                // cumulative ledger and atomic publication in both modes.
                // No kernel or permit failure is intercepted or retried here.
                self.invoke_scoped(inputs, output, work, selected)
            },
        )?
    }

    fn invoke_scoped(
        &self,
        inputs: &[Option<Value>],
        output: &mut Option<Value>,
        work: &NativeLibraryWorkBudget,
        cache: Option<Rc<NativeTurnPlanCache>>,
    ) -> Result<bool, NativeCollectionInvocationError> {
        if inputs.len() != self.inputs.len() {
            return Err(NativeCollectionInvocationError::InvalidInput);
        }
        // All traversal, temporary storage and retirement remain under this
        // exact cumulative account, including failed calls and repeated scopes.
        let warm = cache.as_ref().is_some_and(|cache| cache.has_base());
        if warm {
            cache.as_ref().unwrap().charge_probe()?;
        } else {
            work.charge(self.invocation_metadata_bytes, 0)?;
        }
        let memory = work.memory_budget();
        for (input, layout) in inputs.iter().zip(&self.inputs) {
            let value = input
                .as_ref()
                .ok_or(NativeCollectionInvocationError::InvalidInput)?;
            if value.schema() != layout.schema_id
                || value.schema_key() != layout.schema_key
                || value.shape().parameter_values() != layout.shape_instance.parameter_values()
            {
                return Err(NativeCollectionInvocationError::InvalidInput);
            }
            if value.memory_budget_retained_bytes(&memory).is_none() {
                return Err(NativeCollectionInvocationError::UnadmittedValue);
            }
        }
        if let Some(value) = output.as_ref() {
            // The bound output shape describes this invocation's result.
            // A prior full-write publication may have different dimensions;
            // its actual retained footprint is measured separately below.
            let shape_follows_current_input = matches!(
                self.template.contract(),
                mech_core::ResolvedOperationContract::Declared(contract)
                    if matches!(&contract.outputs[0].construction,
                        mech_core::OutputConstruction::FullWrite {
                            shape: mech_core::ShapeRule::SameAsInput { .. }
                        })
            );
            if value.schema() != self.output.schema_id
                || value.schema_key() != self.output.schema_key
                || (!shape_follows_current_input
                    && value.shape().parameter_values()
                        != self.output.shape_instance.parameter_values())
            {
                return Err(NativeCollectionInvocationError::InvalidInput);
            }
            let bytes = value
                .memory_budget_retained_bytes(&memory)
                .ok_or(NativeCollectionInvocationError::UnadmittedValue)?;
            work.charge(
                bytes
                    .checked_mul(4)
                    .and_then(|n| n.checked_add(32))
                    .ok_or(ResidentKernelError::InvalidShape)?,
                0,
            )?;
        }
        // Reject a call's metadata envelope before constructing even its
        // private payload owner, argument tape or planner maps. A generous
        // invocation memory account cannot lift this call's temporary cap.
        live::check_measurement_with_limits(
            CurrentMemoryFootprint::default(),
            ResourceDemand {
                turn_peak_bytes: self.invocation_metadata_bytes,
                ..ResourceDemand::default()
            },
            self.memory.target.limits,
        )?;
        let owner = ResidentPayloadOwner::new(
            memory.clone(),
            memory.reserve_capacity(0)?,
            memory.reserve_capacity(0)?,
            0,
            1,
        )?;
        let scope = owner.begin(ResidentRegion {
            kind: ResidentValueKind::Snapshot,
            offset: 0,
            len: 1,
            shape: ResidentShape::SCALAR,
        })?;
        // Before cloning plans, constructing fact maps or argument tapes.
        scope.admit_auxiliary(self.invocation_metadata_bytes)?;
        let arguments = inputs
            .iter()
            .map(|value| ResidentValueRef::Snapshot(core::slice::from_ref(value)))
            .collect::<Vec<_>>();
        // Pending DATA summaries are invisible to kernels until every fresh
        // input, old publication, selector region and current budget succeeds.
        let measurements = cache
            .as_ref()
            .map(|cache| cache.begin_data_measurements())
            .transpose()?;
        let mut check = measurement::NativeMeasurementCheck::new(self, cache.as_deref(), &memory);
        let turn = if let Some(cache) = &cache {
            cache.with_facts(
                |facts| -> Result<Arc<TurnMemoryPlan>, NativeCollectionInvocationError> {
                    live::facts_with_indexed_into(
                        &self.memory,
                        self.template.origin().node(),
                        &arguments,
                        ResidentValueRef::Snapshot(core::slice::from_ref(output)),
                        &self.schemas,
                        facts,
                        |ordinal, value, descriptor, schemas, observed| {
                            if let Some(ordinal) = ordinal {
                                let layout = self
                                    .inputs
                                    .get(ordinal)
                                    .ok_or(MemoryPlanError::DescriptorArityMismatch)?;
                                live::footprint_with_data_check(
                                    value,
                                    descriptor,
                                    observed,
                                    &mut check,
                                    measurements.as_ref().unwrap(),
                                    ordinal,
                                    layout.schema_key,
                                )
                            } else {
                                live::footprint_with_check(
                                    value, descriptor, schemas, observed, &mut check,
                                )
                            }
                        },
                    )?;
                    let indices_closed = !crate::resident::text_index::has_index_scope()
                        && crate::resident::text_index::live_demand() == (0, 0);
                    if indices_closed && let Some(plan) = cache.matching_base(facts) {
                        return Ok(plan);
                    }
                    // A warm miss pays the unchanged complete planner envelope
                    // BEFORE old plans retire or replacement allocation begins.
                    if warm {
                        work.charge(self.invocation_metadata_bytes, 0)?;
                    }
                    cache.retire();
                    let plan = Arc::new(self.plan_current(facts)?);
                    if indices_closed {
                        cache.remember_base(plan.clone());
                    }
                    Ok(plan)
                },
            )?
        } else {
            let facts = live::facts_with(
                &self.memory,
                self.template.origin().node(),
                &arguments,
                ResidentValueRef::Snapshot(core::slice::from_ref(output)),
                &self.schemas,
                |value, descriptor, schemas, observed| {
                    live::footprint_with_check(value, descriptor, schemas, observed, &mut check)
                },
            )?;
            Arc::new(self.plan_current(&facts)?)
        };
        if let Some(violation) = turn.budget_violations.first() {
            return Err(budget::resource_budget_error(violation).into());
        }
        // Consume the entire optional transaction before the cache moves into
        // execution, including the empty case. No extra cache owner is needed.
        measurements
            .into_iter()
            .for_each(|transaction| transaction.commit());
        drop(arguments);
        let mut candidate = output.clone();
        let run = || {
            self.kernel.execute(
                &Inputs(inputs),
                ResidentValueMut::Snapshot(core::slice::from_mut(&mut candidate)),
            )
        };
        let result = budget::with_payload_admission(Some(scope.admission()), || {
            if let Some(cache) = cache {
                with_cached_turn_plan(turn, cache, run)
            } else {
                budget::with_resident_turn_plan(turn, run)
            }
        });
        let admission_error = scope.last_error();
        if result.is_err() || admission_error.is_some() {
            scope.abort(ResidentValueMut::Snapshot(core::slice::from_mut(
                &mut candidate,
            )))?;
            if let Some(error) = admission_error {
                return Err(error.into());
            }
            return Err(result.unwrap_err().into());
        }
        let changed = result?;
        scope.finish(ResidentValueMut::Snapshot(core::slice::from_mut(
            &mut candidate,
        )))?;
        let value = candidate
            .as_ref()
            .ok_or(ResidentKernelError::IncompleteOutput)?;
        if value.schema() != self.output.schema_id
            || value.schema_key() != self.output.schema_key
            || value.memory_budget_retained_bytes(&memory).is_none()
        {
            return Err(ResidentKernelError::InvalidOutput.into());
        }
        *output = candidate;
        Ok(changed)
    }

    fn plan_current(
        &self,
        facts: &TurnMemoryFacts,
    ) -> Result<TurnMemoryPlan, NativeCollectionInvocationError> {
        // Compose the actual NativeHost call and its actual allocation family,
        // rather than fabricating a detached test plan or changing a CPU label.
        let node = self.template.origin().node();
        let mut call = self.memory.clone();
        for allocation in &mut call.allocations {
            if let MemoryObjectOwner::NodeScratch { node: owner, .. } = &mut allocation.owner {
                *owner = node;
            }
        }
        let plan = ProgramMemoryPlan {
            call_nodes: Box::new([node]),
            allocations: call.allocations.clone(),
            budget_limits: call.target.limits,
            calls: Box::new([call]),
            ..ProgramMemoryPlan::default()
        };
        Ok(plan_turn_memory(&plan, node, facts)?)
    }
}
