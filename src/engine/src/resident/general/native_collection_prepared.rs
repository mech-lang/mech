//! Frozen selection plus pre-admitted live-geometry binding of ONE leaf.
//! No graph scheduling, native visit permission or alternate leaf semantics.
use super::{
    Error, NativeProgramLibraryTemplate, bind_current, current_template, finite_limits,
    invocation::{NativeCollectionInvocationError as InvokeError, metadata_envelope_for},
};
use crate::{
    ProgramArtifact,
    resident::{budget::NativeLibraryWorkBudget, general::live},
};
use mech_core::{
    CurrentMemoryFootprint, FunctionCatalog, MemoryBudgetLimits, MemoryPlanError,
    ResidentKernelFactory, ResourceDemand, SchemaTable, Value,
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
    // Only the seven sealed families can reach this constructor. Their
    // deferred AnyValue planning has <=2 allocations/port, <=8 additional
    // output stage/scratch allocations, one transaction and one witness/port.
    // Reserve extra entry slack too. Numeric extents do not create plan
    // entries; arbitrary matrix rank/schema trees are covered by the complete
    // schema clone bound inside the common envelope (including Vec growth).
    let entries = ports
        .checked_mul(3)
        .and_then(|n| n.checked_add(16))
        .ok_or_else(|| super::invalid(node))?;
    let binding_bytes = metadata_envelope_for(artifact.schemas(), ports, entries)
        .ok_or_else(|| super::invalid(node))?;
    // Covers construction/traversal and eventual complete metadata retirement
    // before either occurs, including error paths. Never a per-call refill.
    let binding_work = binding_bytes
        .checked_mul(8)
        .and_then(|n| n.checked_add(256))
        .ok_or_else(|| super::invalid(node))?;
    let output_parameters = template.output().dimension_parameters().len();
    Ok(NativePreparedLibraryCall {
        template,
        schemas: Arc::new(artifact.schemas().clone()),
        factory,
        binding_bytes,
        binding_work,
        output_parameters,
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
                let mech_core::ResolvedOperationContract::Declared(contract) =
                    self.template.contract()
                else {
                    return Err(InvokeError::InvalidInput);
                };
                for ((value, schema), port) in inputs
                    .iter()
                    .zip(self.template.inputs())
                    .zip(&contract.inputs)
                {
                    let value = value.as_ref().ok_or(InvokeError::InvalidInput)?;
                    if value.schema() != port.schema || value.schema_key() != schema.key() {
                        return Err(InvokeError::InvalidInput);
                    }
                    if value.memory_budget_retained_bytes(&memory).is_none() {
                        return Err(InvokeError::UnadmittedValue);
                    }
                }
                if let Some(value) = output.as_ref() {
                    if value.schema() != contract.outputs[0].schema
                        || value.schema_key() != self.template.output().key()
                    {
                        return Err(InvokeError::InvalidInput);
                    }
                    if value.memory_budget_retained_bytes(&memory).is_none() {
                        return Err(InvokeError::UnadmittedValue);
                    }
                }
                live::check_measurement_with_limits(
                    CurrentMemoryFootprint::default(),
                    ResourceDemand {
                        turn_peak_bytes: self.binding_bytes,
                        ..ResourceDemand::default()
                    },
                    limits,
                )?;
                let _binding = memory.reserve_capacity(self.binding_bytes)?;
                let parameters = inputs
                    .iter()
                    .map(|value| value.as_ref().unwrap().shape().parameter_values())
                    .collect::<Vec<_>>();
                let empty_shape;
                let output_parameters = if let Some(value) = output.as_ref() {
                    value.shape().parameter_values()
                } else {
                    empty_shape = vec![0; self.output_parameters];
                    &empty_shape
                };
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
                let call = bind_current(
                    &self.schemas,
                    self.template.clone(),
                    self.factory,
                    &parameters,
                    output_parameters,
                    nested,
                )
                .map_err(InvokeError::Binding)?;
                call.invoke(inputs, output, work)
            },
        )?
    }
}
