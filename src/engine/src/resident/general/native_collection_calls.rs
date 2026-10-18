//! Current-shape NativeHost bindings for the maintained individual leaves.
//! This does not schedule a program or grant a generated visit permission.
use super::native_collection_templates::{
    NativeCollectionOperationTemplate, native_collection_operation_template,
};
use crate::ProgramArtifact;
use crate::native_artifact::NativeArtifactLoweringError as Error;
use mech_core::{
    BoundCall, BoundResidentKernel, CallMemoryPlan, CallMemoryPlanningRequest, ExecutionTarget,
    FunctionCatalog, FunctionValueRepresentation, MemoryBudgetLimits, MemoryFootprintWitness,
    MemoryLifetime, MemoryPlanPoint, MemoryWitnessStage, NodeId, RegionAccessPlan,
    ResidentKernelBindRequest, ResidentPortLayout, ResidentShape, ResidentValueKind,
    ResolvedOperationContract, ResolvedOperationDescriptor, ResolvedValueDescriptor, Schema,
    SchemaId, TargetMemoryProfile, physical_storage_descriptor, plan_call_memory,
};

#[path = "native_collection_invocation.rs"]
mod invocation;
pub use invocation::NativeCollectionInvocationError;

/// A constructor-private binding of one actual artifact call at its current
/// shapes. The selected factory, canonical contracts, actual handle
/// storage and finite native policy are retained together. Runtime invocation
/// must still verify the current visit and actual values, then refine/admit a
/// real turn plan and payload scope before entering the maintained leaf.
#[derive(Clone, Debug)]
pub struct NativeCollectionCall {
    template: NativeCollectionOperationTemplate,
    kernel: BoundResidentKernel,
    memory: CallMemoryPlan,
    inputs: Box<[ResidentPortLayout]>,
    output: ResidentPortLayout,
    schemas: std::sync::Arc<mech_core::SchemaTable>,
    invocation_metadata_bytes: u64,
}
impl NativeCollectionCall {
    pub fn template(&self) -> &NativeCollectionOperationTemplate {
        &self.template
    }
    pub fn bound_call(&self) -> &BoundCall {
        self.kernel
            .bound_call()
            .expect("native call binding is retained")
    }
    pub fn memory_plan(&self) -> &CallMemoryPlan {
        &self.memory
    }
    pub fn inputs(&self) -> &[ResidentPortLayout] {
        &self.inputs
    }
    pub fn output(&self) -> &ResidentPortLayout {
        &self.output
    }
}

fn invalid(node: NodeId) -> Error {
    Error::InvalidBinding { node: Some(node) }
}
fn layout(
    node: NodeId,
    id: SchemaId,
    schema: &Schema,
    parameters: &[u64],
) -> Result<ResidentPortLayout, Error> {
    Ok(ResidentPortLayout {
        schema_id: id,
        schema_key: schema.key(),
        // The physical carrier is one immutable value handle; its semantic
        // matrix/map dimensions remain in the exact schema/shape instance.
        kind: ResidentValueKind::Snapshot,
        shape: ResidentShape::SCALAR,
        shape_instance: schema
            .instantiate_shape(parameters.into())
            .map_err(|_| invalid(node))?,
        activation_fixed_shape: false,
        resolved_selector: None,
    })
}

/// Bind one certified root leaf without observing or executing any values.
/// Input parameters describe actual arguments, not a 1x1 transport slot.
/// Output parameters describe the currently published shape, not a promise
/// about the next data-dependent result. Invocation must validate its inputs
/// against these descriptors and refine candidate/published facts separately
/// before any result is admitted or published.
pub fn bind_native_collection_operation(
    artifact: &ProgramArtifact,
    node: NodeId,
    catalog: &FunctionCatalog,
    input_parameters: &[&[u64]],
    output_parameters: &[u64],
    limits: MemoryBudgetLimits,
) -> Result<NativeCollectionCall, Error> {
    let template = native_collection_operation_template(artifact, node, catalog)?;
    if input_parameters.len() != template.inputs().len() {
        return Err(invalid(node));
    }
    // NativeHost has no implicit quota. Require an explicit finite policy for
    // every dimension consumed by these kernels; do not alter resident caps.
    if [
        limits.max_output_elements,
        limits.max_output_bytes,
        limits.max_temporary_bytes,
        limits.max_cloned_bytes,
        limits.max_retained_nodes,
        limits.max_comparison_work,
        limits.max_compute_work,
    ]
    .iter()
    .any(Option::is_none)
    {
        return Err(Error::UnsupportedNode {
            node,
            feature: "native collection calls require explicit finite limits",
        });
    }
    let ResolvedOperationContract::Declared(contract) = template.contract() else {
        return Err(invalid(node));
    };
    let inputs = contract
        .inputs
        .iter()
        .zip(template.inputs())
        .zip(input_parameters)
        .map(|((port, schema), parameters)| layout(node, port.schema, schema, parameters))
        .collect::<Result<Box<[_]>, _>>()?;
    let output = layout(
        node,
        contract.outputs[0].schema,
        template.output(),
        output_parameters,
    )?;
    let descriptor = |schema: &Schema, layout: &ResidentPortLayout| {
        ResolvedValueDescriptor::from_schema(schema.clone(), layout.shape_instance.clone())
            .map_err(|_| invalid(node))
    };
    let semantic = ResolvedOperationDescriptor::from_resolved_contract(
        template.primitive().name().to_owned(),
        template.contract(),
    )
    .map_err(|_| invalid(node))?;
    let call = BoundCall::artifact_operation_for_target(
        semantic,
        template
            .inputs()
            .iter()
            .zip(&inputs)
            .map(|(schema, layout)| descriptor(schema, layout))
            .collect::<Result<Box<[_]>, _>>()?,
        Box::new([descriptor(template.output(), &output)?]),
        template.selected_key().clone(),
        ExecutionTarget::Native,
    )
    .map_err(|_| invalid(node))?;
    let mut target = TargetMemoryProfile::current_native_host().map_err(|_| invalid(node))?;
    target.limits = limits;
    // These are actual Option<Value> host lanes, not resident arenas with a
    // changed label. Keep the physical slot layout tied to that Rust carrier.
    target.primitives.canonical_value_handle = mech_core::SlotLayout {
        bytes: core::mem::size_of::<Option<mech_core::Value>>() as u64,
        alignment: core::mem::align_of::<Option<mech_core::Value>>() as u32,
    };
    let lifetime = MemoryLifetime::Turn {
        first: MemoryPlanPoint::new(0),
        last: MemoryPlanPoint::new(1),
    };
    let storage =
        physical_storage_descriptor(FunctionValueRepresentation::AnyValue, &target, lifetime);
    let input_storage = vec![storage.clone(); inputs.len()];
    let output_storage = [storage];
    let input_witnesses =
        vec![MemoryFootprintWitness::Deferred(MemoryWitnessStage::Turn); inputs.len()];
    let output_witnesses = [MemoryFootprintWitness::Deferred(MemoryWitnessStage::Turn)];
    let memory = plan_call_memory(CallMemoryPlanningRequest {
        bound_call: &call,
        input_storage: &input_storage,
        output_storage: &output_storage,
        input_witnesses: &input_witnesses,
        output_witnesses: &output_witnesses,
        published_output_witnesses: &output_witnesses,
        implementation_memory: template.implementation_memory(),
        target: &target,
        regions: &[RegionAccessPlan::WholeValue],
    })
    .map_err(|error| Error::ResidentBinding {
        error: super::ResidentActivationError::ResidentMemoryPlanRejected { error },
    })?;
    let key = template.selected_key();
    let factory = catalog
        .resident_factory(&key.module_path, &key.operation_name)
        .ok_or(Error::MissingSemanticWitness { node })?;
    let request = ResidentKernelBindRequest {
        contract: template.contract(),
        schemas: artifact.schemas(),
        inputs: &inputs,
        output: output.clone(),
    };
    let kernel = (factory.factory())(&request).map_err(|error| Error::ResidentBinding {
        error: super::ResidentActivationError::KernelBind { node, error },
    })?;
    let kernel = crate::resident::native_semantics::bind_call(kernel, &request, call);
    let invocation_metadata_bytes =
        invocation::metadata_envelope(artifact.schemas(), &memory).ok_or_else(|| invalid(node))?;
    let schemas = kernel
        .shared_snapshot_schemas()
        .cloned()
        .unwrap_or_else(|| std::sync::Arc::new(artifact.schemas().clone()));
    Ok(NativeCollectionCall {
        template,
        kernel,
        memory,
        inputs,
        output,
        schemas,
        invocation_metadata_bytes,
    })
}
