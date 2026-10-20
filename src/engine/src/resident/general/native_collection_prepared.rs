//! Frozen selection plus pre-admitted live-geometry binding of ONE leaf.
//! No graph scheduling, native visit permission or alternate leaf semantics.
use super::{
    Error, NativeCollectionCall, NativeProgramLibraryTemplate, bind_current, current_template,
    finite_limits,
    invocation::{
        NativeCollectionInvocationError as InvokeError, NativePlanCacheState, metadata_envelope_for,
    },
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

#[path = "native_collection_histogram.rs"]
mod histogram;
#[path = "native_collection_string_map_entry.rs"]
mod string_map_entry;
#[path = "native_collection_string_span.rs"]
mod string_span;
pub use string_span::NativeSelectedStringSpanSelector;
#[cfg(feature = "pure_text")]
#[path = "native_collection_selected_join.rs"]
mod selected_join;
#[cfg(feature = "pure_text")]
pub use selected_join::NativeSelectedStringMatrix;

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
    binding_nodes: u64,
    output_parameters: usize,
    input_keys: Box<[mech_core::SchemaKey]>,
    output_key: mech_core::SchemaKey,
    lookup_work: u64,
    output_follows_input: bool,
    cache_identity: Arc<()>,
    span_selector_metadata_bytes: u64,
    span_selector_work: u64,
    #[cfg(feature = "pure_text")]
    selection_shape_bounds: (u64, u64),
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
    plans: NativePlanCacheState,
    call: Option<NativeCollectionCall>,
    identity: Arc<()>,
    limits: MemoryBudgetLimits,
    work: NativeLibraryWorkBudget,
    projection_count: Option<u64>,
    projection_index: crate::resident::text_index::snapshot::Slot,
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
    // The common envelope covers the original shallow selected families.
    // Recursive convert/set additionally funds its SAME semantic plan trees
    // with the shared schema-depth/evidence bound below, before bind_values.
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
    let common_binding_bytes = metadata_envelope_for(
        template
            .inputs()
            .iter()
            .chain(core::iter::once(template.output())),
        ports,
        entries,
    )
    .ok_or_else(|| super::invalid(node))?;
    #[cfg(feature = "convert")]
    let (conversion_bytes, binding_nodes, conversion_work) = if matches!(
        template.primitive(),
        crate::resident::native_collection_semantics::NativeCollectionPrimitive::MatrixToSetConversion
    ) {
        let bound =
            mech_core::conversion_schema_staging_bound(&template.inputs()[0], template.output())
                .ok_or_else(|| super::invalid(node))?;
        (bound.plan_bytes, bound.plan_nodes, bound.plan_work)
    } else {
        (0, 0, 0)
    };
    #[cfg(not(feature = "convert"))]
    let (conversion_bytes, binding_nodes, conversion_work) = (0, 0, 0);
    let binding_bytes = common_binding_bytes
        .checked_add(conversion_bytes)
        .ok_or_else(|| super::invalid(node))?;
    // Covers construction/traversal and eventual complete metadata retirement
    // before either occurs, including error paths. Never a per-call refill.
    let binding_work = binding_bytes
        .checked_mul(8)
        .and_then(|n| n.checked_add(256))
        .and_then(|n| n.checked_add(conversion_work))
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
    let (span_selector_metadata_bytes, span_selector_work) =
        string_span::selector_geometry_bounds(&template).ok_or_else(|| super::invalid(node))?;
    #[cfg(feature = "pure_text")]
    let selection_shape_bounds = if template.primitive()
        == crate::native_artifact::NativeCollectionPrimitive::MatrixRangeAccess
    {
        string_span::selector_schema_geometry_bounds(template.output())
            .ok_or_else(|| super::invalid(node))?
    } else {
        (0, 0)
    };
    Ok(NativePreparedLibraryCall {
        template,
        schemas: Arc::new(artifact.schemas().clone()),
        factory,
        binding_bytes,
        binding_work,
        binding_nodes,
        output_parameters,
        input_keys,
        output_key,
        lookup_work,
        output_follows_input,
        cache_identity: Arc::new(()),
        span_selector_metadata_bytes,
        span_selector_work,
        #[cfg(feature = "pure_text")]
        selection_shape_bounds,
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

    /// Constant-size selected range semantics for a certified native source
    /// strategy, NOT a kernel invocation or a materialized Matrix. The caller
    /// prepays binding_work and reserves binding_metadata_bytes on the SAME
    /// invocation account before this schema/shape traversal or allocation.
    #[cfg(any(feature = "pure_text", feature = "map_ops"))]
    pub fn selected_u64_range_cardinality(&self, start: u64, end: u64) -> Result<u64, InvokeError> {
        use crate::native_artifact::NativeCollectionPrimitive as Primitive;
        use mech_core::{IntegerWidth, SchemaBody, snapshot::ValueDataDraft};
        let inclusive = match self.template.primitive() {
            Primitive::RangeInclusive => true,
            Primitive::RangeExclusive => false,
            _ => return Err(InvokeError::InvalidInput),
        };
        let scalar = SchemaBody::UnsignedInteger(IntegerWidth::W64);
        if self.template.inputs().len() != 2
            || self
                .template
                .inputs()
                .iter()
                .any(|ty| !ty.dimension_parameters().is_empty() || ty.body() != &scalar)
        {
            return Err(InvokeError::InvalidInput);
        }
        let count = crate::resident::numeric::canonical_range_draft_cardinality(
            &[ValueDataDraft::U64(start), ValueDataDraft::U64(end)],
            inclusive,
            false,
        )
        .filter(|count| *count != 0)
        .ok_or(InvokeError::Kernel(
            mech_core::ResidentKernelError::InvalidInput,
        ))?;
        let count = u64::try_from(count)
            .map_err(|_| InvokeError::Kernel(mech_core::ResidentKernelError::InvalidShape))?;
        // Exactly the ordinary range's shape resolver, including parameter
        // bounds and fixed cardinality constraints. Never silently retag it.
        mech_core::shape_for_resolved_extents(self.template.output(), &[1, count])
            .map_err(|_| InvokeError::Kernel(mech_core::ResidentKernelError::InvalidShape))?;
        // Preserve the ordinary materializing range's physical Layout bound
        // even when the certified source strategy never constructs its Matrix.
        crate::resident::numeric::canonical_u64_range_selector_representation(count)?;
        Ok(count)
    }

    /// Maintained TextJoin mechanics after a constructor-private composed
    /// strategy has prepaid the append/growth. Never invokes a fake Matrix.
    pub fn selected_text_join_append(
        &self,
        target: &mut String,
        separator: &str,
        has_previous: bool,
        value: &str,
    ) -> Result<(), InvokeError> {
        #[cfg(feature = "pure_text")]
        if self.template.primitive() == crate::native_artifact::NativeCollectionPrimitive::TextJoin
        {
            // The native sink already prepaid fixed per-append metadata work.
            // Fail before either push if its promised capacity is absent; this
            // public selected helper must never allocate as an implicit repair.
            let bytes = target
                .len()
                .checked_add(if has_previous { separator.len() } else { 0 })
                .and_then(|n| n.checked_add(value.len()))
                .ok_or(InvokeError::Kernel(
                    mech_core::ResidentKernelError::InvalidShape,
                ))?;
            if bytes > target.capacity() {
                return Err(InvokeError::Kernel(
                    mech_core::ResidentKernelError::InvalidShape,
                ));
            }
            crate::resident::text_ops::append_joined(target, separator, has_previous, value);
            return Ok(());
        }
        let _ = (target, separator, has_previous, value);
        Err(InvokeError::InvalidInput)
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
                    if binding.projection_count.is_some()
                        || !Arc::ptr_eq(&binding.identity, &self.cache_identity)
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
                            retained_nodes: self.binding_nodes,
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
                    nested.max_retained_nodes = Some(
                        limits
                            .max_retained_nodes
                            .unwrap()
                            .checked_sub(self.binding_nodes)
                            .ok_or(MemoryPlanError::TargetAddressOverflow)?,
                    );
                    if let Some(binding) = cache.binding.as_mut() {
                        // Lookup, identities, input validation and the complete
                        // replacement work/temporary checks already succeeded.
                        // Retire the old prepaid call BEFORE any new allocation.
                        // Keep a funded guard if replacement admission fails.
                        binding.plans = NativePlanCacheState::default();
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
                            plans: NativePlanCacheState::default(),
                            call: Some(call),
                            identity: self.cache_identity.clone(),
                            limits,
                            work: work.clone(),
                            projection_count: None,
                            projection_index: Default::default(),
                            _reservation: reservation,
                        }));
                    }
                }
                // The same maintained planner, current payload facts, bounded
                // kernel, admission and atomic publication run on EVERY visit.
                let binding = cache.binding.as_mut().unwrap();
                binding
                    .call
                    .as_ref()
                    .ok_or(InvokeError::InvalidInput)?
                    .invoke_cached(&mut binding.plans, inputs, output, work)
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
                        retained_nodes: self.binding_nodes,
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
                nested.max_retained_nodes = Some(
                    limits
                        .max_retained_nodes
                        .unwrap()
                        .checked_sub(self.binding_nodes)
                        .ok_or(MemoryPlanError::TargetAddressOverflow)?,
                );
                let call = self.bind_values(inputs, output, nested)?;
                call.invoke(inputs, output, work)
            },
        )?
    }
}
