//! Frozen selection plus pre-admitted live-geometry binding of ONE leaf.
//! No graph scheduling, native visit permission or alternate leaf semantics.
use super::{
    Error, NativeCollectionCall, NativeProgramLibraryTemplate, bind_current, current_template,
    finite_limits,
    invocation::{NativeCollectionInvocationError as InvokeError, metadata_envelope_for},
};
use crate::{
    ProgramArtifact,
    resident::{budget::NativeLibraryWorkBudget, general::live},
};
use mech_core::{
    CurrentMemoryFootprint, FunctionCatalog, ManagedMemoryBudget, ManagedMemoryReservation,
    MemoryBudgetLimits, MemoryPlanError, ResidentKernelFactory, ResourceDemand, SchemaTable, Value,
};
use std::sync::Arc;

/// Compiler-owned selection/immutable schemas, not a bound runtime call or
/// authority to enter its source site. No artifact/catalog traversal occurs
/// during invoke_current. Its complete temporary binding is admitted first.
#[derive(Debug)]
pub struct NativePreparedLibraryCall {
    template: NativeProgramLibraryTemplate,
    schemas: Arc<SchemaTable>,
    factory: ResidentKernelFactory,
    binding_bytes: u64,
    binding_work: u64,
    output_parameters: usize,
    input_keys: Box<[mech_core::SchemaKey]>,
    output_key: mech_core::SchemaKey,
    lookup_work: u64,
    output_follows_input: bool,
    cache_identity: Arc<()>,
}

/// One owning native invocation's lazy per-site binding slot. This retains
/// neither arguments nor results and grants no permission to visit a site.
/// It is intentionally non-Clone and cannot migrate to another work ledger.
#[derive(Debug, Default)]
pub struct NativePreparedLibraryCallCache {
    binding: Option<Box<CachedBinding>>,
}

#[derive(Debug)]
struct CachedBinding {
    // Retire all bound metadata before releasing its admitted capacity.
    call: Option<NativeCollectionCall>,
    identity: Arc<()>,
    limits: MemoryBudgetLimits,
    work: NativeLibraryWorkBudget,
    _reservation: ManagedMemoryReservation,
}

/// Prepare only the actual current private-witness-selected site. This is
/// compiler/setup allocation; it does not enter a work scope or execute a leaf.
pub fn prepare_native_program_library_call(
    artifact: &ProgramArtifact,
    template: &NativeProgramLibraryTemplate,
    catalog: &FunctionCatalog,
) -> Result<NativePreparedLibraryCall, Error> {
    let template = current_template(artifact, template, catalog)?;
    let node = template.origin().node();
    let key = template.selected_key();
    let factory = catalog
        .resident_factory(&key.module_path, &key.operation_name)
        .ok_or(Error::MissingSemanticWitness { node })?
        .factory();
    let ports = (template.inputs().len() as u64)
        .checked_add(1)
        .ok_or_else(|| super::invalid(node))?;
    // Only sealed shallow text/map/range/selection/conversion/constructor families reach here.
    // Numeric ranges, positional matrix selection and scalar-key map lookup
    // contain at most rank-two scalar matrices or two atomic map children;
    // selector state has fixed metadata size. Shallow map/tuple/record packs
    // add one atomic input port per child, already included in `ports`.
    // Build/NoAdditionalScratch ranges add no implementation scratch entries;
    // CanonicalFinalize String selection/matrix conversion uses the same output entry family.
    // Tests assert the actual NativeHost plan stays within this entry bound.
    // Their deferred AnyValue planning has <=2 allocations/port, <=8 additional
    // output stage/scratch allocations, one transaction and one witness/port.
    // Reserve extra entry slack too. Numeric extents do not create plan
    // entries; actual port schema trees (including tuple/record children and
    // dimension expressions) retain their deep-clone bound inside the common
    // envelope. Only the immutable full schema arena is shared.
    let entries = ports
        .checked_mul(3)
        .and_then(|n| n.checked_add(16))
        .ok_or_else(|| super::invalid(node))?;
    let binding_bytes = metadata_envelope_for(
        template
            .inputs()
            .iter()
            .chain(core::iter::once(template.output())),
        ports,
        entries,
    )
    .ok_or_else(|| super::invalid(node))?;
    // Covers construction/traversal and eventual complete metadata retirement
    // before either occurs, including error paths. Never a per-call refill.
    let binding_work = binding_bytes
        .checked_mul(8)
        .and_then(|n| n.checked_add(256))
        .ok_or_else(|| super::invalid(node))?;
    let output_parameters = template.output().dimension_parameters().len();
    // Schema::key encodes and hashes its whole tree. Retain immutable keys
    // here, not inside an ostensibly constant-work runtime cache lookup.
    let input_keys = template
        .inputs()
        .iter()
        .map(|schema| schema.key())
        .collect();
    let output_key = template.output().key();
    // Lookup reads only borrowed validated shape tapes. Fund their complete
    // declared lengths, including the prior-output/zero-default signature.
    let parameters = template
        .inputs()
        .iter()
        .chain(core::iter::once(template.output()))
        .try_fold(0u64, |total, schema| {
            total.checked_add(u64::try_from(schema.dimension_parameters().len()).ok()?)
        })
        .ok_or_else(|| super::invalid(node))?;
    let lookup_work = ports
        .checked_mul(64)
        .and_then(|n| n.checked_add(parameters.checked_mul(16)?))
        .and_then(|n| n.checked_add(256))
        .ok_or_else(|| super::invalid(node))?;
    let output_follows_input = matches!(
        template.contract(),
        mech_core::ResolvedOperationContract::Declared(contract)
            if matches!(&contract.outputs[0].construction,
                mech_core::OutputConstruction::FullWrite {
                    shape: mech_core::ShapeRule::SameAsInput { .. }
                })
    );
    Ok(NativePreparedLibraryCall {
        template,
        schemas: Arc::new(artifact.schemas().clone()),
        factory,
        binding_bytes,
        binding_work,
        output_parameters,
        input_keys,
        output_key,
        lookup_work,
        output_follows_input,
        cache_identity: Arc::new(()),
    })
}

impl NativePreparedLibraryCall {
    pub fn template(&self) -> &NativeProgramLibraryTemplate {
        &self.template
    }
    pub const fn binding_metadata_bytes(&self) -> u64 {
        self.binding_bytes
    }
    pub const fn binding_work(&self) -> u64 {
        self.binding_work
    }

    fn validate_values(
        &self,
        inputs: &[Option<Value>],
        output: &Option<Value>,
        memory: &ManagedMemoryBudget,
    ) -> Result<(), InvokeError> {
        let mech_core::ResolvedOperationContract::Declared(contract) = self.template.contract()
        else {
            return Err(InvokeError::InvalidInput);
        };
        for ((value, key), port) in inputs.iter().zip(&self.input_keys).zip(&contract.inputs) {
            let value = value.as_ref().ok_or(InvokeError::InvalidInput)?;
            if value.schema() != port.schema || value.schema_key() != *key {
                return Err(InvokeError::InvalidInput);
            }
            if value.memory_budget_retained_bytes(memory).is_none() {
                return Err(InvokeError::UnadmittedValue);
            }
        }
        if let Some(value) = output.as_ref() {
            if value.schema() != contract.outputs[0].schema || value.schema_key() != self.output_key
            {
                return Err(InvokeError::InvalidInput);
            }
            if value.memory_budget_retained_bytes(memory).is_none() {
                return Err(InvokeError::UnadmittedValue);
            }
        }
        Ok(())
    }

    // Called only after the enclosing binding reservation and work debit.
    // Keep the SAME live shape resolver on both first cached binding and
    // parameterized calls; empty shape tapes do not waive consistency checks.
    fn bind_values(
        &self,
        inputs: &[Option<Value>],
        output: &Option<Value>,
        limits: MemoryBudgetLimits,
    ) -> Result<NativeCollectionCall, InvokeError> {
        let mech_core::ResolvedOperationContract::Declared(contract) = self.template.contract()
        else {
            return Err(InvokeError::InvalidInput);
        };
        let parameters = inputs
            .iter()
            .map(|value| value.as_ref().unwrap().shape().parameter_values())
            .collect::<Vec<_>>();
        let current_shape;
        let empty_shape;
        let output_parameters = if let mech_core::OutputConstruction::FullWrite {
            shape: mech_core::ShapeRule::SameAsInput { input },
        } = &contract.outputs[0].construction
        {
            let index = usize::from(*input);
            let source = inputs
                .get(index)
                .and_then(Option::as_ref)
                .ok_or(InvokeError::InvalidInput)?;
            let schema = self
                .template
                .inputs()
                .get(index)
                .ok_or(InvokeError::InvalidInput)?;
            let dimensions = match schema.body() {
                mech_core::SchemaBody::Matrix { dimensions, .. } => dimensions.as_ref(),
                _ => &[],
            };
            let extents = dimensions
                .iter()
                .map(|dimension| source.shape().resolve_dimension(dimension))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| InvokeError::InvalidInput)?;
            // SameAsInput describes the CURRENT logical extents, not
            // the parameter numbering of another schema or a stale
            // published output. Reuse the canonical shape resolver.
            current_shape = mech_core::shape_for_resolved_extents(self.template.output(), &extents)
                .map_err(|_| InvokeError::InvalidInput)?;
            current_shape.parameter_values()
        } else if let Some(value) = output.as_ref() {
            value.shape().parameter_values()
        } else {
            empty_shape = vec![0; self.output_parameters];
            &empty_shape
        };
        bind_current(
            &self.schemas,
            self.template.clone(),
            self.factory,
            &parameters,
            output_parameters,
            limits,
        )
        .map_err(InvokeError::Binding)
    }

    fn geometry_matches(
        &self,
        call: &NativeCollectionCall,
        inputs: &[Option<Value>],
        output: &Option<Value>,
    ) -> bool {
        if inputs.len() != call.inputs().len()
            || !inputs
                .iter()
                .zip(call.inputs())
                .zip(self.template.inputs())
                .all(|((value, bound), schema)| {
                    let Some(value) = value.as_ref() else {
                        return false;
                    };
                    let actual = value.shape().parameter_values();
                    actual.len() == schema.dimension_parameters().len()
                        && actual == bound.shape_instance.parameter_values()
                })
        {
            return false;
        }
        if self.output_follows_input {
            // The SAME bind_values verified the input->output extent mapping
            // on the first bind. Identical input tapes and immutable schemas
            // therefore imply identical derived output geometry, even when the
            // old published output has a different tape or parameter numbering.
            return true;
        }
        let bound = call.output().shape_instance.parameter_values();
        if let Some(value) = output.as_ref() {
            let actual = value.shape().parameter_values();
            actual.len() == self.output_parameters && actual == bound
        } else {
            // bind_values uses the declared-length zero tape for absent old
            // output. Compare it without allocating a placeholder Vec.
            bound.len() == self.output_parameters && bound.iter().all(|value| *value == 0)
        }
    }

    /// Reuse only the exact current binding geometry at this site within one
    /// owning native invocation. Every hit measures fresh Values; a shape miss
    /// retires the old prepaid binding before constructing its replacement
    /// through the SAME live binder. No argument or result is retained.
    pub fn invoke_current_cached(
        &self,
        cache: &mut NativePreparedLibraryCallCache,
        inputs: &[Option<Value>],
        output: &mut Option<Value>,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
    ) -> Result<bool, InvokeError> {
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        if inputs.len() != self.template.inputs().len() {
            return Err(InvokeError::InvalidInput);
        }
        work.with_call_limits(
            limits.max_compute_work.unwrap(),
            limits.max_comparison_work.unwrap(),
            || {
                work.charge(self.lookup_work, 0)?;
                let memory = work.memory_budget();
                if let Some(binding) = cache.binding.as_ref() {
                    if !Arc::ptr_eq(&binding.identity, &self.cache_identity)
                        || binding.limits != limits
                        || !binding.work.shares_account(work)
                        || binding._reservation.budget() != memory
                    {
                        return Err(InvokeError::InvalidInput);
                    }
                }
                self.validate_values(inputs, output, &memory)?;
                let hit = cache
                    .binding
                    .as_ref()
                    .and_then(|binding| binding.call.as_ref())
                    .is_some_and(|call| self.geometry_matches(call, inputs, output));
                if !hit {
                    let owner_bytes = core::mem::size_of::<CachedBinding>() as u64;
                    let bytes = self
                        .binding_bytes
                        .checked_add(owner_bytes)
                        .ok_or(InvokeError::InvalidInput)?;
                    let setup_work = if cache.binding.is_none() {
                        self.binding_work
                            .checked_add(
                                owner_bytes
                                    .checked_mul(8)
                                    .and_then(|n| n.checked_add(256))
                                    .ok_or(InvokeError::InvalidInput)?,
                            )
                            .ok_or(InvokeError::InvalidInput)?
                    } else {
                        self.binding_work
                    };
                    // Preserve the full original construction+retirement work
                    // envelope. Only an actual geometry hit avoids rebuilding.
                    work.charge(setup_work, 0)?;
                    live::check_measurement_with_limits(
                        CurrentMemoryFootprint::default(),
                        ResourceDemand {
                            turn_peak_bytes: bytes,
                            ..ResourceDemand::default()
                        },
                        limits,
                    )?;
                    let mut nested = limits;
                    nested.max_temporary_bytes = Some(
                        limits
                            .max_temporary_bytes
                            .unwrap()
                            .checked_sub(bytes)
                            .ok_or(MemoryPlanError::TargetAddressOverflow)?,
                    );
                    if let Some(binding) = cache.binding.as_mut() {
                        // Lookup, identities, input validation and the complete
                        // replacement work/temporary checks already succeeded.
                        // Retire the old prepaid call BEFORE any new allocation.
                        // Keep a funded guard if replacement admission fails.
                        drop(binding.call.take());
                        binding._reservation.resize_capacity(owner_bytes)?;
                        binding._reservation.resize_capacity(bytes)?;
                        match self.bind_values(inputs, output, nested) {
                            Ok(call) => binding.call = Some(call),
                            Err(error) => {
                                binding._reservation.resize_capacity(owner_bytes)?;
                                return Err(error);
                            }
                        }
                    } else {
                        let reservation = memory.reserve_capacity(bytes)?;
                        let call = self.bind_values(inputs, output, nested)?;
                        cache.binding = Some(Box::new(CachedBinding {
                            call: Some(call),
                            identity: self.cache_identity.clone(),
                            limits,
                            work: work.clone(),
                            _reservation: reservation,
                        }));
                    }
                }
                // The same maintained planner, current payload facts, bounded
                // kernel, admission and atomic publication run on EVERY visit.
                cache
                    .binding
                    .as_ref()
                    .unwrap()
                    .call
                    .as_ref()
                    .ok_or(InvokeError::InvalidInput)?
                    .invoke(inputs, output, work)
            },
        )?
    }

    /// Invoke the one selected leaf at the actual current input/prior-output
    /// geometry. The generated caller must separately prove its current visit.
    /// All binding metadata is charged/reserved BEFORE vectors, schema clones,
    /// plans or the kernel are built. The SAME enclosing call scope covers
    /// binding and execution, so the inner invoke cannot renew its allowance.
    pub fn invoke_current(
        &self,
        inputs: &[Option<Value>],
        output: &mut Option<Value>,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
    ) -> Result<bool, InvokeError> {
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        if inputs.len() != self.template.inputs().len() {
            return Err(InvokeError::InvalidInput);
        }
        work.with_call_limits(
            limits.max_compute_work.unwrap(),
            limits.max_comparison_work.unwrap(),
            || {
                work.charge(self.binding_work, 0)?;
                let memory = work.memory_budget();
                self.validate_values(inputs, output, &memory)?;
                live::check_measurement_with_limits(
                    CurrentMemoryFootprint::default(),
                    ResourceDemand {
                        turn_peak_bytes: self.binding_bytes,
                        ..ResourceDemand::default()
                    },
                    limits,
                )?;
                let _binding = memory.reserve_capacity(self.binding_bytes)?;
                // This record remains live during invocation. Its reservation is
                // therefore subtracted from the nested temporary allowance; a
                // large invocation account cannot lift the original call policy.
                let mut nested = limits;
                nested.max_temporary_bytes = Some(
                    limits
                        .max_temporary_bytes
                        .unwrap()
                        .checked_sub(self.binding_bytes)
                        .ok_or(MemoryPlanError::TargetAddressOverflow)?,
                );
                let call = self.bind_values(inputs, output, nested)?;
                call.invoke(inputs, output, work)
            },
        )?
    }
}
