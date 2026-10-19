//! The individual maintained-leaf boundary, not a program dispatcher.
use super::NativeCollectionCall;
use crate::memory_planner::{ProgramMemoryPlan, plan_turn_memory};
use crate::resident::budget::{self, NativeLibraryWorkBudget, payload::ResidentPayloadOwner};
use crate::resident::general::{ResidentRegion, live};
use mech_core::{
    CurrentMemoryFootprint, MemoryBudgetViolation, MemoryObjectOwner, MemoryPlanError,
    MemoryRuntimeError, ResidentKernelError, ResidentKernelInputs, ResidentShape,
    ResidentValueKind, ResidentValueMut, ResidentValueRef, ResourceDemand, Value,
};

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

/// Only the constructor-private shallow maintained families reach this
/// envelope. It conservatively covers planner coexistence, deep copies of
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
    // SchemaTable::clone now shares its immutable entry allocation. However
    // Schema/ResolvedValueDescriptor/BoundCall/plan/template clones still own
    // their port-schema trees. Preserve their full heap/growth/coexistence
    // envelope; do not replace every schema-related allocation with Arc size.
    let descriptors = schemas.into_iter().try_fold(
        mech_core::SchemaTable::clone_metadata_allocation_bound_bytes(),
        |total, schema| total.checked_add(schema.clone_allocation_bound_bytes()?),
    )?;
    let schema_copies = ports.checked_mul(4)?.checked_add(4)?;
    descriptors
        .checked_mul(schema_copies)?
        .checked_add(
            ports
                .checked_add(entries)?
                .checked_add(4)?
                .checked_mul(2048)?,
        )?
        .checked_mul(16)
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
            || self.invoke_scoped(inputs, output, work),
        )?
    }

    fn invoke_scoped(
        &self,
        inputs: &[Option<Value>],
        output: &mut Option<Value>,
        work: &NativeLibraryWorkBudget,
    ) -> Result<bool, NativeCollectionInvocationError> {
        if inputs.len() != self.inputs.len() {
            return Err(NativeCollectionInvocationError::InvalidInput);
        }
        // All traversal, temporary storage and retirement remain under this
        // exact cumulative account, including failed calls and repeated scopes.
        work.charge(self.invocation_metadata_bytes, 0)?;
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
        let mut check = |footprint: CurrentMemoryFootprint, observed: ResourceDemand| {
            budget::observe_control_work(observed).map_err(measurement_error)?;
            let mut observed = observed;
            observed.turn_peak_bytes = observed
                .turn_peak_bytes
                .checked_add(self.invocation_metadata_bytes)
                .ok_or(MemoryPlanError::TargetAddressOverflow)?;
            live::check_measurement_with_limits(footprint, observed, self.memory.target.limits)
        };
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
        let turn = plan_turn_memory(&plan, node, &facts)?;
        if let Some(violation) = turn.budget_violations.first() {
            return Err(budget::resource_budget_error(violation).into());
        }
        drop(arguments);
        let mut candidate = output.clone();
        let result = budget::with_payload_admission(Some(scope.admission()), || {
            budget::with_resident_turn_plan(turn, || {
                self.kernel.execute(
                    &Inputs(inputs),
                    ResidentValueMut::Snapshot(core::slice::from_mut(&mut candidate)),
                )
            })
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
}
