#[cfg(feature = "no_std")]
use alloc::{
    boxed::Box,
    format,
    rc::Rc,
    string::{String, ToString},
    vec::Vec,
};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, rc::Rc, string::String, vec::Vec};

use crate::{
    CallMemoryPlan, CallMemoryPlanningRequest, ConversionPlan, CurrentMemoryFootprint,
    DimensionExpr, ExecutionTarget, FunctionCatalog, FunctionInstance, FunctionInvocation,
    FunctionPortBacking, FunctionValueRepresentation, ImplementationMemoryClass, MResult,
    MechError, MechErrorKind, MechFunctionFactory, MemoryFootprintWitness, MemoryLifetime,
    MemoryPlanPoint, MemoryTargetKind, OperationContractDeclaration, OperationId,
    OutputConstruction, PhysicalStorageDescriptor, PlannedSlotKind, Ref, RegionAccessPlan,
    ResolvedOperationContract, ResolvedOutputSchemaRule, ResolvedType, ResolvedValueDescriptor,
    RuntimeBindingSelector, RuntimeFunctionEntry, RuntimeFunctionId, RuntimeFunctionInputs,
    RuntimeOperationBindingMismatch, Schema, SchemaBody, SchemaKey, SchemaTable,
    SchemaTableBuilder, ShapeInstance, TargetMemoryProfile, TypeConstraintFailure,
    TypeResolutionError, Value, ValueCell, physical_storage_descriptor, plan_call_memory,
};
use core::cell::RefCell;

#[cfg(feature = "matrix")]
use crate::{FunctionArgumentRole, matrix::Matrix};

/// One source-level function input after expression lowering.
///
/// Absence and matrix all-selection are source controls; neither is a
/// canonical runtime value. Keeping them explicit prevents either control
/// from being confused with canonical unit or option absence.
#[derive(Clone, Debug)]
pub enum SpecializationInput {
    Cell(ValueCell),
    Absent,
    MatrixAllSelection,
}

fn specialization_input_descriptors(
    inputs: &[&SpecializationInput],
) -> MResult<Box<[ResolvedValueDescriptor]>> {
    inputs
        .iter()
        .map(|input| input.cell()?.resolved_descriptor())
        .collect::<MResult<Vec<_>>>()
        .map(Vec::into_boxed_slice)
}

fn specialization_input_representations(
    inputs: &[&SpecializationInput],
) -> MResult<Box<[FunctionValueRepresentation]>> {
    inputs
        .iter()
        .map(|input| {
            input
                .representation()
                .ok_or_else(|| control_input_error("non-value", "runtime factory representation"))
        })
        .collect::<MResult<Vec<_>>>()
        .map(Vec::into_boxed_slice)
}

fn specialization_input_cells(inputs: &[&SpecializationInput]) -> MResult<Box<[ValueCell]>> {
    inputs
        .iter()
        .map(|input| input.cell().cloned())
        .collect::<MResult<Vec<_>>>()
        .map(Vec::into_boxed_slice)
}

fn call_target_profile(target: ExecutionTarget) -> MResult<TargetMemoryProfile> {
    let profile = match target {
        ExecutionTarget::DirectRuntime => TargetMemoryProfile::current_direct_host(),
        ExecutionTarget::ResidentCpu => TargetMemoryProfile::current_resident_cpu(),
        ExecutionTarget::Native => TargetMemoryProfile::current_native_host(),
        ExecutionTarget::GpuBatch => {
            return Err(MechError::new(
                crate::GenericError {
                    msg: "GPU call memory planning requires queried adapter limits".into(),
                },
                None,
            )
            .with_compiler_loc());
        }
    };
    profile.map_err(|error| MechError::new(error, None).with_compiler_loc())
}

fn call_port_lifetime() -> MemoryLifetime {
    MemoryLifetime::Turn {
        first: MemoryPlanPoint::new(0),
        last: MemoryPlanPoint::new(1),
    }
}

fn fixed_descriptor_witness(
    descriptor: &ResolvedValueDescriptor,
) -> MResult<MemoryFootprintWitness> {
    let logical_elements = descriptor
        .current_extents()
        .map_err(MechError::from)?
        .iter()
        .try_fold(1_u64, |product, extent| product.checked_mul(*extent))
        .ok_or_else(|| {
            MechError::new(
                crate::MemoryPlanError::ArithmeticOverflow {
                    field: "specialized call logical elements",
                },
                None,
            )
            .with_compiler_loc()
        })?;
    Ok(MemoryFootprintWitness::Known(CurrentMemoryFootprint {
        logical_elements,
        shape_parameter_count: u64::try_from(descriptor.shape().parameter_values().len()).map_err(
            |_| {
                MechError::new(
                    crate::MemoryPlanError::ArithmeticOverflow {
                        field: "specialized call shape parameters",
                    },
                    None,
                )
                .with_compiler_loc()
            },
        )?,
        ..CurrentMemoryFootprint::default()
    }))
}

fn cell_memory_witness(cell: &ValueCell) -> MResult<MemoryFootprintWitness> {
    let descriptor = cell.resolved_descriptor()?;
    let mut footprint = match fixed_descriptor_witness(&descriptor)? {
        MemoryFootprintWitness::Known(footprint) => footprint,
        MemoryFootprintWitness::Deferred(_) => unreachable!("fixed witness is known"),
    };
    let snapshot = cell.snapshot()?;
    let retained = snapshot
        .retained_footprint(cell.schema_table().as_ref())
        .map_err(|error| {
            MechError::new(
                crate::GenericError {
                    msg: format!("unable to measure specialized call value: {error:?}"),
                },
                None,
            )
            .with_compiler_loc()
        })?;
    footprint.payload_bytes = retained.retained_bytes;
    footprint.encoded_bytes = retained.encoded_bytes;
    footprint.retained_nodes = retained.node_count;
    Ok(MemoryFootprintWitness::Known(footprint))
}

fn witness_for_unallocated_output(
    descriptor: &ResolvedValueDescriptor,
    storage: &PhysicalStorageDescriptor,
) -> MResult<MemoryFootprintWitness> {
    if matches!(
        storage.slot,
        PlannedSlotKind::StringHeader | PlannedSlotKind::CanonicalValueHandle
    ) {
        Ok(MemoryFootprintWitness::Deferred(
            crate::MemoryWitnessStage::Turn,
        ))
    } else {
        fixed_descriptor_witness(descriptor)
    }
}

fn output_regions(bound_call: &BoundCall) -> MResult<Box<[RegionAccessPlan]>> {
    let requirements = bound_call
        .operation_descriptor()
        .contract
        .memory_requirements(bound_call.inputs().len())
        .map_err(|error| {
            MechError::new(
                crate::GenericError {
                    msg: format!("invalid operation memory requirements: {error:?}"),
                },
                None,
            )
            .with_compiler_loc()
        })?;
    match requirements.outputs.as_ref() {
        [] if bound_call.outputs().len() == 1 => {
            Ok(vec![RegionAccessPlan::WholeValue].into_boxed_slice())
        }
        outputs if outputs.len() == bound_call.outputs().len() => Ok(outputs
            .iter()
            .map(|requirement| match requirement.construction.as_ref() {
                Some(OutputConstruction::ReadModifyWrite { regions, .. }) => {
                    RegionAccessPlan::Deferred(*regions)
                }
                _ => RegionAccessPlan::WholeValue,
            })
            .collect::<Vec<_>>()
            .into_boxed_slice()),
        _ => Err(
            MechError::new(crate::MemoryPlanError::DescriptorArityMismatch, None)
                .with_compiler_loc(),
        ),
    }
}

fn semantic_input_cells<'a>(
    bound_call: &BoundCall,
    input_cells: &'a [ValueCell],
    output_cell: Option<&'a ValueCell>,
) -> MResult<Vec<&'a ValueCell>> {
    if bound_call.inputs().len() == input_cells.len() {
        return Ok(input_cells.iter().collect());
    }
    if bound_call.inputs().len() != input_cells.len().saturating_add(1) {
        return Err(
            MechError::new(crate::MemoryPlanError::DescriptorArityMismatch, None)
                .with_compiler_loc(),
        );
    }
    let base_input = bound_call
        .operation_descriptor()
        .contract
        .outputs
        .iter()
        .find_map(|output| match output.construction {
            OutputConstruction::ReadModifyWrite { base_input, .. } => Some(base_input as usize),
            _ => None,
        })
        .ok_or_else(|| {
            MechError::new(crate::MemoryPlanError::DescriptorArityMismatch, None)
                .with_compiler_loc()
        })?;
    let output_cell = output_cell.ok_or_else(|| {
        MechError::new(crate::MemoryPlanError::DescriptorArityMismatch, None).with_compiler_loc()
    })?;
    if base_input > input_cells.len() {
        return Err(
            MechError::new(crate::MemoryPlanError::DescriptorArityMismatch, None)
                .with_compiler_loc(),
        );
    }
    let mut semantic = Vec::with_capacity(bound_call.inputs().len());
    for ordinal in 0..bound_call.inputs().len() {
        if ordinal == base_input {
            semantic.push(output_cell);
        } else {
            let physical = ordinal - usize::from(ordinal > base_input);
            semantic.push(input_cells.get(physical).ok_or_else(|| {
                MechError::new(crate::MemoryPlanError::DescriptorArityMismatch, None)
                    .with_compiler_loc()
            })?);
        }
    }
    Ok(semantic)
}

fn plan_specialized_call(
    bound_call: &BoundCall,
    input_cells: &[ValueCell],
    output_representation: FunctionValueRepresentation,
    output_cell: Option<&ValueCell>,
    implementation_memory: ImplementationMemoryClass,
) -> MResult<CallMemoryPlan> {
    let target = call_target_profile(bound_call.target())?;
    debug_assert_ne!(target.kind, MemoryTargetKind::Gpu);
    let lifetime = call_port_lifetime();
    let semantic_input_cells = semantic_input_cells(bound_call, input_cells, output_cell)?;
    let input_storage = semantic_input_cells
        .iter()
        .map(|cell| physical_storage_descriptor(cell.representation(), &target, lifetime))
        .collect::<Vec<_>>();
    let output_storage = bound_call
        .outputs()
        .iter()
        .map(|_| physical_storage_descriptor(output_representation, &target, lifetime))
        .collect::<Vec<_>>();
    let input_witnesses = semantic_input_cells
        .iter()
        .map(|cell| cell_memory_witness(cell))
        .collect::<MResult<Vec<_>>>()?;
    let output_witnesses = bound_call
        .outputs()
        .iter()
        .zip(&output_storage)
        .map(|(descriptor, storage)| match output_cell {
            Some(cell) => cell_memory_witness(cell),
            None => witness_for_unallocated_output(descriptor, storage),
        })
        .collect::<MResult<Vec<_>>>()?;
    let regions = output_regions(bound_call)?;
    plan_call_memory(CallMemoryPlanningRequest {
        bound_call,
        input_storage: &input_storage,
        output_storage: &output_storage,
        input_witnesses: &input_witnesses,
        output_witnesses: &output_witnesses,
        implementation_memory,
        target: &target,
        regions: &regions,
    })
    .map_err(|error| MechError::new(error, None).with_compiler_loc())
}

fn validate_resolved_inputs(
    call: &ResolvedCall,
    inputs: &[ResolvedValueDescriptor],
) -> MResult<()> {
    validate_bound_descriptors(&call.converted_inputs, inputs, "input")
}

fn validate_binding_selector(call: &ResolvedCall, selector: RuntimeBindingSelector) -> MResult<()> {
    if matches!(selector, RuntimeBindingSelector::Operation(operation) if operation == call.operation.id)
    {
        Ok(())
    } else {
        Err(MechError::new(
            RuntimeOperationBindingMismatch {
                operation: Some(call.operation.id),
                reason: "a resolved source call must bind through its exact operation ID".into(),
            },
            None,
        )
        .with_compiler_loc())
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum StorageSpecificity {
    ErasedCanonical,
    FullyDynamicMatrix,
    InvariantAxisDynamicMatrix,
    FixedShapeMatrix,
    ExactScalarOrAggregate,
}

fn storage_specificity(representation: FunctionValueRepresentation) -> StorageSpecificity {
    use crate::{FunctionMatrixRepresentation as Matrix, FunctionMatrixStoragePattern as Storage};
    match representation {
        FunctionValueRepresentation::AnyValue => StorageSpecificity::ErasedCanonical,
        FunctionValueRepresentation::Matrix {
            storage: Storage::Exact(Matrix::MatrixD),
            ..
        } => StorageSpecificity::FullyDynamicMatrix,
        FunctionValueRepresentation::Matrix {
            storage: Storage::Exact(Matrix::RowVectorD | Matrix::VectorD),
            ..
        } => StorageSpecificity::InvariantAxisDynamicMatrix,
        FunctionValueRepresentation::Matrix {
            storage: Storage::Exact(_),
            ..
        } => StorageSpecificity::FixedShapeMatrix,
        FunctionValueRepresentation::Matrix {
            storage: Storage::AnyStorage,
            ..
        } => StorageSpecificity::ErasedCanonical,
        _ => StorageSpecificity::ExactScalarOrAggregate,
    }
}

const fn execution_profile(target: ExecutionTarget) -> &'static str {
    match target {
        ExecutionTarget::DirectRuntime => "direct runtime",
        ExecutionTarget::ResidentCpu => "resident CPU",
        ExecutionTarget::Native => "native",
        ExecutionTarget::GpuBatch => "GPU batch",
    }
}

impl SpecializationInput {
    pub const fn is_absent(&self) -> bool {
        matches!(self, Self::Absent)
    }

    pub fn require_matrix_all_selection(&self) -> MResult<()> {
        match self {
            Self::MatrixAllSelection => Ok(()),
            Self::Cell(_) => Err(control_input_error("value", "matrix all-selection")),
            Self::Absent => Err(control_input_error(
                "source absence",
                "matrix all-selection",
            )),
        }
    }

    pub fn cell(&self) -> MResult<&ValueCell> {
        match self {
            Self::Cell(cell) => Ok(cell),
            Self::Absent => Err(control_input_error("source absence", "cell")),
            Self::MatrixAllSelection => Err(control_input_error("matrix all-selection", "cell")),
        }
    }

    pub fn try_ref<T: FunctionPortBacking>(&self) -> MResult<Ref<T>> {
        self.cell()?.try_ref::<T>()
    }

    #[cfg(feature = "matrix")]
    pub fn try_matrix<T>(&self, input_index: usize) -> MResult<Matrix<T>>
    where
        T: FunctionPortBacking + Clone,
    {
        crate::function::argument::matrix_from_cell(
            self.cell()?,
            FunctionArgumentRole::Input(input_index),
        )
    }

    pub fn snapshot(&self) -> MResult<Value> {
        self.cell()?.snapshot()
    }

    pub fn schema_key(&self) -> Option<SchemaKey> {
        match self {
            Self::Cell(cell) => Some(cell.schema_key()),
            Self::Absent | Self::MatrixAllSelection => None,
        }
    }

    pub fn closed_schema_body(&self) -> MResult<Option<SchemaBody>> {
        match self {
            Self::Cell(cell) => cell.closed_schema_body().map(Some),
            Self::Absent | Self::MatrixAllSelection => Ok(None),
        }
    }

    pub fn shape(&self) -> Option<ShapeInstance> {
        match self {
            Self::Cell(cell) => Some(cell.shape().clone()),
            Self::Absent | Self::MatrixAllSelection => None,
        }
    }

    pub fn representation(&self) -> Option<FunctionValueRepresentation> {
        match self {
            Self::Cell(cell) => Some(cell.representation()),
            Self::Absent | Self::MatrixAllSelection => None,
        }
    }

    #[cfg(feature = "matrix")]
    pub fn matrix_descriptor(&self) -> MResult<Option<crate::FunctionMatrixDescriptor>> {
        match self {
            Self::Cell(cell) => crate::function::argument::canonical_matrix_descriptor(cell),
            Self::Absent | Self::MatrixAllSelection => Ok(None),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SpecializationInvocation {
    inputs: Box<[SpecializationInput]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedOperationDescriptor {
    pub id: OperationId,
    pub canonical_name: Box<str>,
    pub contract: OperationContractDeclaration,
}

impl ResolvedOperationDescriptor {
    pub fn new(
        id: OperationId,
        canonical_name: impl Into<Box<str>>,
        contract: OperationContractDeclaration,
    ) -> MResult<Self> {
        let canonical_name = canonical_name.into();
        if canonical_name.is_empty() || OperationId::from_name(&canonical_name) != id {
            return Err(MechError::new(
                RuntimeOperationBindingMismatch {
                    operation: Some(id),
                    reason: format!(
                        "canonical operation name {canonical_name:?} does not match operation ID 0x{:016x}",
                        id.raw(),
                    ),
                },
                None,
            )
            .with_compiler_loc());
        }
        Ok(Self {
            id,
            canonical_name,
            contract,
        })
    }

    pub fn from_name(
        canonical_name: impl Into<Box<str>>,
        contract: OperationContractDeclaration,
    ) -> MResult<Self> {
        let canonical_name = canonical_name.into();
        Self::new(
            OperationId::from_name(&canonical_name),
            canonical_name,
            contract,
        )
    }

    pub fn from_resolved_contract(
        canonical_name: impl Into<Box<str>>,
        contract: &ResolvedOperationContract,
    ) -> MResult<Self> {
        let declaration = match contract {
            ResolvedOperationContract::Declared(contract) => OperationContractDeclaration {
                inputs: crate::InputPortLayout::Fixed(
                    contract
                        .inputs
                        .iter()
                        .map(|input| crate::InputPortPolicy {
                            access: input.access,
                            delivery: input.delivery,
                        })
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                ),
                outputs: contract
                    .outputs
                    .iter()
                    .map(|output| crate::OutputPortPolicy {
                        access: output.access,
                        delivery: output.delivery,
                        construction: output.construction.clone(),
                        alias: output.alias,
                        change_detection: output.change_detection,
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
                interaction: contract.interaction.clone(),
            },
        };
        Self::from_name(canonical_name, declaration)
    }

    pub fn validate(&self) -> MResult<()> {
        Self::new(self.id, self.canonical_name.clone(), self.contract.clone()).map(|_| ())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedCall {
    pub operation: ResolvedOperationDescriptor,
    pub overload_id: u32,
    pub original_inputs: Box<[ResolvedType]>,
    pub converted_inputs: Box<[ResolvedType]>,
    pub input_conversions: Box<[ConversionPlan]>,
    pub outputs: Box<[ResolvedType]>,
    pub output_schema_rules: Box<[ResolvedOutputSchemaRule]>,
}

impl ResolvedCall {
    pub fn validate(&self) -> MResult<()> {
        self.operation.validate()?;
        if self.original_inputs.len() != self.input_conversions.len()
            || self.converted_inputs.len() != self.input_conversions.len()
            || self.outputs.len() != self.output_schema_rules.len()
        {
            return Err(MechError::from(TypeResolutionError::incompatible(
                "resolved call",
                TypeConstraintFailure::InvalidScheme {
                    reason: "resolved call vector lengths are inconsistent".into(),
                },
            )));
        }
        for ((original, converted), plan) in self
            .original_inputs
            .iter()
            .zip(&self.converted_inputs)
            .zip(&self.input_conversions)
        {
            if !crate::exact_type_equal(original, &plan.source)
                || !crate::exact_type_equal(converted, &plan.target)
            {
                return Err(MechError::from(TypeResolutionError::incompatible(
                    "resolved call",
                    TypeConstraintFailure::InvalidScheme {
                        reason: "conversion endpoints do not match resolved inputs".into(),
                    },
                )));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BoundCallOrigin {
    ResolvedOverload(u32),
    ArtifactOperation,
    SyntaxDirected,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BoundImplementationId {
    Runtime(RuntimeFunctionId),
    Resident(crate::ResidentOperationKey),
}

/// Immutable certificate joining semantic resolution to one physical runtime
/// implementation. It contains no allocation or lifetime policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundCall {
    operation: ResolvedOperationDescriptor,
    origin: BoundCallOrigin,
    inputs: Box<[ResolvedValueDescriptor]>,
    outputs: Box<[ResolvedValueDescriptor]>,
    implementation: BoundImplementationId,
    target: ExecutionTarget,
}

impl BoundCall {
    pub fn from_resolved_call(
        call: &ResolvedCall,
        inputs: Box<[ResolvedValueDescriptor]>,
        outputs: Box<[ResolvedValueDescriptor]>,
        runtime_function: RuntimeFunctionId,
        target: ExecutionTarget,
    ) -> MResult<Self> {
        call.validate()?;
        validate_bound_descriptors(&call.converted_inputs, &inputs, "input")?;
        validate_bound_descriptors(&call.outputs, &outputs, "output")?;
        Ok(Self {
            operation: call.operation.clone(),
            origin: BoundCallOrigin::ResolvedOverload(call.overload_id),
            inputs,
            outputs,
            implementation: BoundImplementationId::Runtime(runtime_function),
            target,
        })
    }

    pub fn syntax_directed(
        operation: ResolvedOperationDescriptor,
        inputs: Box<[ResolvedValueDescriptor]>,
        outputs: Box<[ResolvedValueDescriptor]>,
        runtime_function: RuntimeFunctionId,
        target: ExecutionTarget,
    ) -> MResult<Self> {
        Ok(Self {
            operation,
            origin: BoundCallOrigin::SyntaxDirected,
            inputs,
            outputs,
            implementation: BoundImplementationId::Runtime(runtime_function),
            target,
        })
    }

    pub fn artifact_operation(
        operation: ResolvedOperationDescriptor,
        inputs: Box<[ResolvedValueDescriptor]>,
        outputs: Box<[ResolvedValueDescriptor]>,
        resident_operation: crate::ResidentOperationKey,
    ) -> MResult<Self> {
        let canonical_name = format!(
            "{}/{}",
            resident_operation.module_path.join("/"),
            resident_operation.operation_name
        );
        if canonical_name != operation.canonical_name.as_ref() {
            return Err(MechError::new(
                crate::RuntimeOperationBindingMismatch {
                    operation: Some(operation.id),
                    reason: format!(
                        "resident operation {canonical_name:?} does not match the artifact operation"
                    ),
                },
                None,
            )
            .with_compiler_loc());
        }
        Ok(Self {
            operation,
            origin: BoundCallOrigin::ArtifactOperation,
            inputs,
            outputs,
            implementation: BoundImplementationId::Resident(resident_operation),
            target: ExecutionTarget::ResidentCpu,
        })
    }

    pub const fn operation(&self) -> OperationId {
        self.operation.id
    }

    pub const fn operation_descriptor(&self) -> &ResolvedOperationDescriptor {
        &self.operation
    }

    pub fn resolve_operation_contract(
        &mut self,
        contract: &OperationContractDeclaration,
    ) -> MResult<()> {
        self.operation.contract = contract.clone();
        self.operation.validate()
    }

    pub const fn origin(&self) -> &BoundCallOrigin {
        &self.origin
    }

    pub fn inputs(&self) -> &[ResolvedValueDescriptor] {
        &self.inputs
    }

    pub fn outputs(&self) -> &[ResolvedValueDescriptor] {
        &self.outputs
    }

    pub const fn implementation(&self) -> &BoundImplementationId {
        &self.implementation
    }

    pub const fn runtime_function(&self) -> Option<RuntimeFunctionId> {
        match &self.implementation {
            BoundImplementationId::Runtime(function) => Some(*function),
            BoundImplementationId::Resident(_) => None,
        }
    }

    pub const fn resident_operation(&self) -> Option<&crate::ResidentOperationKey> {
        match &self.implementation {
            BoundImplementationId::Runtime(_) => None,
            BoundImplementationId::Resident(operation) => Some(operation),
        }
    }

    pub const fn target(&self) -> ExecutionTarget {
        self.target
    }
}

fn validate_bound_descriptors(
    expected: &[ResolvedType],
    actual: &[ResolvedValueDescriptor],
    category: &'static str,
) -> MResult<()> {
    if expected.len() != actual.len() {
        return Err(MechError::new(
            ResolvedValueDescriptorMismatch {
                category,
                index: expected.len().min(actual.len()),
                expected: format!("{} descriptors", expected.len()),
                actual: format!("{} descriptors", actual.len()),
            },
            None,
        )
        .with_compiler_loc());
    }
    for (index, (expected, actual)) in expected.iter().zip(actual).enumerate() {
        if !crate::exact_type_equal(expected, actual.resolved_type()) {
            return Err(MechError::new(
                ResolvedValueDescriptorMismatch {
                    category,
                    index,
                    expected: expected.semantic_name(),
                    actual: actual.resolved_type().semantic_name(),
                },
                None,
            )
            .with_compiler_loc());
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedValueDescriptorMismatch {
    pub category: &'static str,
    pub index: usize,
    pub expected: String,
    pub actual: String,
}

impl MechErrorKind for ResolvedValueDescriptorMismatch {
    fn name(&self) -> &str {
        "ResolvedValueDescriptorMismatch"
    }

    fn message(&self) -> String {
        format!(
            "bound {} descriptor {} expected {}, received {}",
            self.category, self.index, self.expected, self.actual
        )
    }
}

impl SpecializationInvocation {
    pub fn new(inputs: Box<[SpecializationInput]>) -> Self {
        Self { inputs }
    }

    pub fn from_cells(inputs: Box<[ValueCell]>) -> Self {
        Self::new(
            inputs
                .into_vec()
                .into_iter()
                .map(SpecializationInput::Cell)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        )
    }

    pub fn inputs(&self) -> &[SpecializationInput] {
        &self.inputs
    }

    pub fn input(&self, index: usize) -> Option<&SpecializationInput> {
        self.inputs.get(index)
    }

    pub fn len(&self) -> usize {
        self.inputs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inputs.is_empty()
    }
}

/// Explicit, invocation-local facilities used while selecting a concrete
/// function implementation. No global schema or catalog state is consulted.
pub struct SpecializationContext<'a> {
    schemas: Rc<SchemaTable>,
    catalog: Option<&'a FunctionCatalog>,
    selected_operation: Option<OperationId>,
    diagnostic_operation: Option<String>,
    syntax_operation: RefCell<Option<ResolvedOperationDescriptor>>,
    resolved_call: Option<ResolvedCall>,
}

impl<'a> SpecializationContext<'a> {
    pub fn new(schemas: Rc<SchemaTable>) -> Self {
        Self {
            schemas,
            catalog: None,
            selected_operation: None,
            diagnostic_operation: None,
            syntax_operation: RefCell::new(None),
            resolved_call: None,
        }
    }

    pub fn with_catalog(schemas: Rc<SchemaTable>, catalog: &'a FunctionCatalog) -> Self {
        Self {
            schemas,
            catalog: Some(catalog),
            selected_operation: None,
            diagnostic_operation: None,
            syntax_operation: RefCell::new(None),
            resolved_call: None,
        }
    }

    pub fn for_invocation(
        invocation: &SpecializationInvocation,
        catalog: Option<&'a FunctionCatalog>,
    ) -> MResult<Self> {
        let mut builder = SchemaTableBuilder::new();
        for input in invocation.inputs() {
            let SpecializationInput::Cell(cell) = input else {
                continue;
            };
            let schemas = cell.schema_table();
            for entry in schemas.entries() {
                builder.insert(entry.schema().clone())?;
            }
        }
        let schemas = if builder.is_empty() {
            empty_schema_table()
        } else {
            Rc::new(builder.finish()?.table)
        };
        Ok(Self {
            schemas,
            catalog,
            selected_operation: None,
            diagnostic_operation: None,
            syntax_operation: RefCell::new(None),
            resolved_call: None,
        })
    }

    pub fn for_syntax_directed_invocation(
        invocation: &SpecializationInvocation,
        catalog: Option<&'a FunctionCatalog>,
        operation: ResolvedOperationDescriptor,
    ) -> MResult<Self> {
        let mut context = Self::for_invocation(invocation, catalog)?;
        operation.validate()?;
        context.selected_operation = Some(operation.id);
        context.diagnostic_operation = Some(operation.canonical_name.to_string());
        *context.syntax_operation.borrow_mut() = Some(operation);
        Ok(context)
    }

    pub fn for_resolved_invocation(
        invocation: &SpecializationInvocation,
        catalog: Option<&'a FunctionCatalog>,
        selected_operation: OperationId,
        diagnostic_operation: impl Into<String>,
        resolved_call: ResolvedCall,
    ) -> MResult<Self> {
        if resolved_call.operation.id != selected_operation {
            return Err(MechError::new(
                RuntimeOperationBindingMismatch {
                    operation: Some(selected_operation),
                    reason: format!(
                        "selected specializer operation 0x{:016x} differs from resolved operation 0x{:016x}",
                        selected_operation.raw(),
                        resolved_call.operation.id.raw(),
                    ),
                },
                None,
            )
            .with_compiler_loc());
        }
        resolved_call.validate()?;
        let mut context = Self::for_invocation(invocation, catalog)?;
        context.selected_operation = Some(selected_operation);
        context.diagnostic_operation = Some(diagnostic_operation.into());
        *context.syntax_operation.borrow_mut() = None;
        context.resolved_call = Some(resolved_call);
        Ok(context)
    }

    pub fn schemas(&self) -> &SchemaTable {
        self.schemas.as_ref()
    }

    /// Resolves a syntax-directed operation's invocation-specific memory
    /// declaration before any physical implementation is certified.
    pub fn resolve_syntax_operation_contract(
        &self,
        contract: &OperationContractDeclaration,
    ) -> MResult<()> {
        let diagnostic_operation = self.diagnostic_operation_name();
        let mut syntax_operation = self.syntax_operation.borrow_mut();
        let descriptor = syntax_operation.as_mut().ok_or_else(|| {
            MechError::new(
                SpecializationSemanticCallUnavailable {
                    semantic_operation: diagnostic_operation,
                },
                None,
            )
            .with_compiler_loc()
        })?;
        descriptor.contract = contract.clone();
        descriptor.validate()
    }

    /// Resolves an invocation-specific syntax operation whose semantic
    /// identity, as well as its memory contract, depends on the parsed
    /// selector form. The finalized identity is recorded before an
    /// implementation is certified and later artifact lowering consumes it
    /// verbatim.
    pub fn resolve_syntax_operation(
        &mut self,
        canonical_name: impl Into<Box<str>>,
        contract: &OperationContractDeclaration,
    ) -> MResult<()> {
        if self.resolved_call.is_some() {
            return Err(MechError::new(
                SpecializationSemanticCallUnavailable {
                    semantic_operation: self.diagnostic_operation_name(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let descriptor = ResolvedOperationDescriptor::from_name(canonical_name, contract.clone())?;
        self.selected_operation = Some(descriptor.id);
        self.diagnostic_operation = Some(descriptor.canonical_name.to_string());
        *self.syntax_operation.borrow_mut() = Some(descriptor);
        Ok(())
    }

    pub fn catalog(&self) -> Option<&FunctionCatalog> {
        self.catalog
    }

    pub fn resolved_call(&self) -> MResult<&ResolvedCall> {
        self.resolved_call.as_ref().ok_or_else(|| {
            MechError::new(
                SpecializationSemanticCallUnavailable {
                    semantic_operation: self
                        .diagnostic_operation
                        .clone()
                        .unwrap_or_else(|| "named operation".into()),
                },
                None,
            )
            .with_compiler_loc()
        })
    }

    pub fn resolved_input(&self, index: usize) -> MResult<&ResolvedType> {
        self.resolved_call()?
            .converted_inputs
            .get(index)
            .ok_or_else(|| resolved_call_index_error(self, "input", index))
    }

    pub fn resolved_output(&self, index: usize) -> MResult<&ResolvedType> {
        self.resolved_call()?
            .outputs
            .get(index)
            .ok_or_else(|| resolved_call_index_error(self, "output", index))
    }

    pub fn input_conversion(&self, index: usize) -> MResult<&ConversionPlan> {
        self.resolved_call()?
            .input_conversions
            .get(index)
            .ok_or_else(|| resolved_call_index_error(self, "input conversion", index))
    }

    pub fn schema(&self, key: SchemaKey) -> MResult<&Schema> {
        self.schemas
            .find_by_key(key)
            .and_then(|id| self.schemas.get(id))
            .ok_or_else(|| {
                MechError::new(SpecializationUnknownSchema { key }, None).with_compiler_loc()
            })
    }

    pub fn resolve_shape(
        &self,
        schema: SchemaKey,
        parameter_values: Box<[u64]>,
    ) -> MResult<ShapeInstance> {
        Ok(self.schema(schema)?.instantiate_shape(parameter_values)?)
    }

    pub fn resolve_dimension(
        &self,
        shape: &ShapeInstance,
        dimension: &DimensionExpr,
    ) -> MResult<u64> {
        Ok(shape.resolve_dimension(dimension)?)
    }

    pub fn typed_cell<T>(
        &self,
        reference: Ref<T>,
        schema: SchemaKey,
        shape: ShapeInstance,
    ) -> MResult<ValueCell>
    where
        T: crate::CanonicalCellBacking,
    {
        let schema = self.schemas.find_by_key(schema).ok_or_else(|| {
            MechError::new(SpecializationUnknownSchema { key: schema }, None).with_compiler_loc()
        })?;
        ValueCell::from_ref(reference, schema, shape, self.schemas.clone())
    }

    pub fn value_cell(&self, value: Value) -> MResult<ValueCell> {
        ValueCell::from_runtime_value(value, self.schemas.clone())
    }

    /// Certifies an implementation selected by syntax-directed lowering or by
    /// an operation-specific specializer that does not use the runtime
    /// catalog. The canonical cells must already carry their final semantic
    /// descriptors; this method only records and validates them.
    pub fn certify_instance(
        &self,
        instance: FunctionInstance,
        runtime_function: RuntimeFunctionId,
        target: ExecutionTarget,
        implementation_memory: ImplementationMemoryClass,
    ) -> MResult<SpecializedFunction> {
        let inputs = instance
            .inputs()
            .iter()
            .map(ValueCell::resolved_descriptor)
            .collect::<MResult<Vec<_>>>()?
            .into_boxed_slice();
        self.certify_instance_with_descriptors(
            instance,
            runtime_function,
            target,
            inputs,
            implementation_memory,
        )
    }

    /// Certifies a lowered implementation whose semantic inputs were folded
    /// into an immutable runtime value during specialization.
    pub fn certify_instance_for_inputs(
        &self,
        instance: FunctionInstance,
        runtime_function: RuntimeFunctionId,
        target: ExecutionTarget,
        inputs: &[&SpecializationInput],
        implementation_memory: ImplementationMemoryClass,
    ) -> MResult<SpecializedFunction> {
        let inputs = specialization_input_descriptors(inputs)?;
        self.certify_instance_with_descriptors(
            instance,
            runtime_function,
            target,
            inputs,
            implementation_memory,
        )
    }

    fn certify_instance_with_descriptors(
        &self,
        instance: FunctionInstance,
        runtime_function: RuntimeFunctionId,
        target: ExecutionTarget,
        inputs: Box<[ResolvedValueDescriptor]>,
        implementation_memory: ImplementationMemoryClass,
    ) -> MResult<SpecializedFunction> {
        let operation = self.selected_operation.ok_or_else(|| {
            MechError::new(
                SpecializationSemanticCallUnavailable {
                    semantic_operation: self.diagnostic_operation_name(),
                },
                None,
            )
            .with_compiler_loc()
        })?;
        let operation_descriptor = if let Some(call) = self.resolved_call.as_ref() {
            call.operation.clone()
        } else {
            self.syntax_operation.borrow().clone().ok_or_else(|| {
                MechError::new(
                    SpecializationSemanticCallUnavailable {
                        semantic_operation: format!(
                            "{} has no authoritative operation-memory contract",
                            self.diagnostic_operation_name()
                        ),
                    },
                    None,
                )
                .with_compiler_loc()
            })?
        };
        if operation_descriptor.id != operation {
            return Err(MechError::new(
                RuntimeOperationBindingMismatch {
                    operation: Some(operation),
                    reason:
                        "specialization semantic descriptor disagrees with the selected operation"
                            .into(),
                },
                None,
            )
            .with_compiler_loc());
        }
        instance
            .invocation()
            .check_operation_memory_contract(&operation_descriptor.contract)?;
        let outputs = vec![instance.output().resolved_descriptor()?].into_boxed_slice();
        let bound_call = if let Some(call) = self.resolved_call.as_ref() {
            BoundCall::from_resolved_call(call, inputs, outputs, runtime_function, target)?
        } else {
            BoundCall::syntax_directed(
                operation_descriptor,
                inputs,
                outputs,
                runtime_function,
                target,
            )?
        };
        let memory_plan = plan_specialized_call(
            &bound_call,
            instance.inputs(),
            instance.output().representation(),
            Some(instance.output()),
            implementation_memory,
        )?;
        SpecializedFunction::new(instance, bound_call, memory_plan)
    }

    fn semantic_binding_inputs(&self) -> MResult<Box<[String]>> {
        Ok(self
            .resolved_call()?
            .converted_inputs
            .iter()
            .map(ResolvedType::semantic_name)
            .collect::<Vec<_>>()
            .into_boxed_slice())
    }

    pub fn resolved_output_descriptor(
        &self,
        output_index: usize,
        current_extents: Box<[u64]>,
        inputs: &[&SpecializationInput],
    ) -> MResult<ResolvedValueDescriptor> {
        let call = self.resolved_call()?;
        let resolved = call
            .outputs
            .get(output_index)
            .ok_or_else(|| resolved_call_index_error(self, "output", output_index))?;
        let rule = call
            .output_schema_rules
            .get(output_index)
            .ok_or_else(|| resolved_call_index_error(self, "output schema rule", output_index))?;
        let inputs = specialization_input_descriptors(inputs)?;
        crate::materialize_resolved_output(resolved, rule, &inputs, current_extents)
            .map_err(MechError::from)
    }

    pub fn bind_resolved_runtime(
        &self,
        selector: RuntimeBindingSelector,
        target: ExecutionTarget,
        output_extents: Box<[Box<[u64]>]>,
        inputs: &[&SpecializationInput],
    ) -> MResult<SpecializedFunction> {
        let call = self.resolved_call()?;
        validate_binding_selector(call, selector)?;
        let input_descriptors = specialization_input_descriptors(inputs)?;
        validate_resolved_inputs(call, &input_descriptors)?;
        if call.outputs.len() != 1 || output_extents.len() != 1 {
            return Err(MechError::from(TypeResolutionError::incompatible(
                self.diagnostic_operation_name(),
                TypeConstraintFailure::InvalidScheme {
                    reason: format!(
                        "runtime invocation requires one output; resolved {} outputs and received {} extent lists",
                        call.outputs.len(),
                        output_extents.len(),
                    ),
                },
            )));
        }
        let output_descriptor = crate::materialize_resolved_output(
            &call.outputs[0],
            &call.output_schema_rules[0],
            &input_descriptors,
            output_extents[0].clone(),
        )
        .map_err(MechError::from)?;
        let input_representations = specialization_input_representations(inputs)?;
        let (entry, rejections) = self.select_runtime_candidate(
            selector,
            target,
            &input_representations,
            &output_descriptor,
            None,
        )?;
        let input_cells = specialization_input_cells(inputs)?;
        let bound_call = BoundCall::from_resolved_call(
            call,
            input_descriptors,
            vec![output_descriptor.clone()].into_boxed_slice(),
            entry.id,
            target,
        )?;
        let memory_plan = plan_specialized_call(
            &bound_call,
            &input_cells,
            entry.signature().output,
            None,
            entry.implementation_memory_class(),
        )?;
        let output =
            ValueCell::allocate_for_descriptor(&output_descriptor, entry.signature().output)?;
        let invocation =
            invocation_for_runtime_inputs(entry.signature().inputs, output, input_cells)?;
        let instance = entry.bind_resolved_invocation(call.operation.id, target, invocation)?;
        let _ = rejections;
        SpecializedFunction::new(instance, bound_call, memory_plan)
    }

    pub fn bind_resolved_runtime_existing_output(
        &self,
        selector: RuntimeBindingSelector,
        target: ExecutionTarget,
        output: &SpecializationInput,
        inputs: &[&SpecializationInput],
    ) -> MResult<SpecializedFunction> {
        let call = self.resolved_call()?;
        validate_binding_selector(call, selector)?;
        if call.outputs.len() != 1 {
            return Err(MechError::from(TypeResolutionError::incompatible(
                self.diagnostic_operation_name(),
                TypeConstraintFailure::InvalidScheme {
                    reason: format!(
                        "existing-output invocation requires one output, resolved {}",
                        call.outputs.len()
                    ),
                },
            )));
        }
        let output_cell = output.cell()?;
        let output_descriptor = output_cell.resolved_descriptor()?;
        let runtime_input_descriptors = specialization_input_descriptors(inputs)?;
        let input_descriptors = core::iter::once(output_descriptor.clone())
            .chain(runtime_input_descriptors.iter().cloned())
            .collect::<Vec<_>>()
            .into_boxed_slice();
        validate_resolved_inputs(call, &input_descriptors)?;
        let current_extents = output_descriptor
            .current_extents()
            .map_err(MechError::from)?;
        let expected_output = crate::materialize_resolved_output(
            &call.outputs[0],
            &call.output_schema_rules[0],
            &input_descriptors,
            current_extents,
        )
        .map_err(MechError::from)?;
        if output_descriptor != expected_output {
            return Err(MechError::new(
                ResolvedValueDescriptorMismatch {
                    category: "output",
                    index: 0,
                    expected: format!("{expected_output:?}"),
                    actual: format!("{output_descriptor:?}"),
                },
                None,
            )
            .with_compiler_loc());
        }
        output_cell.validate_descriptor(&expected_output)?;
        let input_representations = specialization_input_representations(inputs)?;
        let output_representation = output_cell.representation();
        let (entry, _) = self.select_runtime_candidate(
            selector,
            target,
            &input_representations,
            &output_descriptor,
            Some(output_representation),
        )?;
        let input_cells = specialization_input_cells(inputs)?;
        let invocation = invocation_for_runtime_inputs(
            entry.signature().inputs,
            output_cell.clone(),
            input_cells,
        )?;
        let instance = entry.bind_resolved_invocation(call.operation.id, target, invocation)?;
        let bound_call = BoundCall::from_resolved_call(
            call,
            input_descriptors,
            vec![output_descriptor].into_boxed_slice(),
            entry.id,
            target,
        )?;
        let memory_plan = plan_specialized_call(
            &bound_call,
            instance.inputs(),
            output_cell.representation(),
            Some(&output_cell),
            entry.implementation_memory_class(),
        )?;
        SpecializedFunction::new(instance, bound_call, memory_plan)
    }

    fn select_runtime_candidate(
        &self,
        selector: RuntimeBindingSelector,
        target: ExecutionTarget,
        input_representations: &[FunctionValueRepresentation],
        output: &ResolvedValueDescriptor,
        existing_output: Option<FunctionValueRepresentation>,
    ) -> MResult<(&RuntimeFunctionEntry, Box<[RuntimeCandidateRejection]>)> {
        let catalog = self.catalog.ok_or_else(|| {
            MechError::new(
                SpecializationRuntimeCatalogUnavailable {
                    semantic_operation: self.diagnostic_operation_name(),
                    semantic_inputs: self.semantic_binding_inputs().unwrap_or_default(),
                    semantic_output: output.resolved_type().semantic_name(),
                    execution_profile: execution_profile(target),
                },
                None,
            )
            .with_compiler_loc()
        })?;
        let mut eligible = Vec::new();
        let mut rejected = Vec::new();
        for entry in catalog.runtime_entries_for_binding(selector, target) {
            let reason = if !runtime_inputs_match(entry.signature().inputs, input_representations) {
                Some("physical input signature mismatch".into())
            } else if existing_output
                .is_some_and(|actual| !entry.signature().output.matches(actual))
            {
                Some("existing output backing does not match the physical output signature".into())
            } else {
                let capabilities =
                    crate::runtime_storage::actual_backing_capabilities(entry.signature().output);
                crate::check_schema_storage_compatibility(
                    output.schema(),
                    output.shape(),
                    &capabilities,
                )
                .err()
                .map(|error| format!("output storage is incompatible: {error:?}"))
            };
            if let Some(reason) = reason {
                rejected.push(RuntimeCandidateRejection {
                    function: entry.id,
                    name: entry.name.clone(),
                    reason,
                });
            } else {
                eligible.push((storage_specificity(entry.signature().output), entry));
            }
        }
        let Some(best_specificity) = eligible.iter().map(|(specificity, _)| *specificity).max()
        else {
            return Err(MechError::new(
                SpecializationRuntimeFactoryUnavailable {
                    operation: self.resolved_call()?.operation.id,
                    semantic_operation: self.diagnostic_operation_name(),
                    semantic_inputs: self.semantic_binding_inputs()?,
                    semantic_outputs: vec![output.resolved_type().semantic_name()]
                        .into_boxed_slice(),
                    target,
                    candidates: rejected.into_boxed_slice(),
                },
                None,
            )
            .with_compiler_loc());
        };
        let tied = eligible
            .into_iter()
            .filter(|(specificity, _)| *specificity == best_specificity)
            .map(|(_, entry)| entry)
            .collect::<Vec<_>>();
        if tied.len() != 1 {
            return Err(MechError::new(
                SpecializationRuntimeFactoryAmbiguous {
                    operation: self.resolved_call()?.operation.id,
                    semantic_operation: self.diagnostic_operation_name(),
                    target,
                    candidates: tied
                        .iter()
                        .map(|entry| (entry.id, entry.name.clone()))
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                },
                None,
            )
            .with_compiler_loc());
        }
        Ok((tied[0], rejected.into_boxed_slice()))
    }

    fn diagnostic_operation_name(&self) -> String {
        self.diagnostic_operation.clone().unwrap_or_else(|| {
            format!(
                "operation 0x{:016x}",
                self.resolved_call
                    .as_ref()
                    .map_or(0, |call| call.operation.id.raw())
            )
        })
    }
}

fn runtime_inputs_match(
    signature: RuntimeFunctionInputs,
    inputs: &[FunctionValueRepresentation],
) -> bool {
    match (signature, inputs) {
        (RuntimeFunctionInputs::Nullary, []) => true,
        (RuntimeFunctionInputs::Unary(expected), [actual]) => expected.matches(*actual),
        (RuntimeFunctionInputs::Binary(first, second), [actual_first, actual_second]) => {
            first.matches(*actual_first) && second.matches(*actual_second)
        }
        (
            RuntimeFunctionInputs::Ternary(first, second, third),
            [actual_first, actual_second, actual_third],
        ) => {
            first.matches(*actual_first)
                && second.matches(*actual_second)
                && third.matches(*actual_third)
        }
        (
            RuntimeFunctionInputs::Quaternary(first, second, third, fourth),
            [actual_first, actual_second, actual_third, actual_fourth],
        ) => {
            first.matches(*actual_first)
                && second.matches(*actual_second)
                && third.matches(*actual_third)
                && fourth.matches(*actual_fourth)
        }
        (RuntimeFunctionInputs::Variadic { element }, inputs) => {
            inputs.iter().all(|actual| element.matches(*actual))
        }
        _ => false,
    }
}

fn invocation_for_runtime_inputs(
    signature: RuntimeFunctionInputs,
    output: ValueCell,
    inputs: Box<[ValueCell]>,
) -> MResult<FunctionInvocation> {
    let inputs = inputs.into_vec();
    Ok(match (signature, inputs.as_slice()) {
        (RuntimeFunctionInputs::Nullary, []) => FunctionInvocation::nullary(output),
        (RuntimeFunctionInputs::Unary(_), [input]) => {
            FunctionInvocation::unary(output, input.clone())
        }
        (RuntimeFunctionInputs::Binary(_, _), [first, second]) => {
            FunctionInvocation::binary(output, first.clone(), second.clone())
        }
        (RuntimeFunctionInputs::Ternary(_, _, _), [first, second, third]) => {
            FunctionInvocation::ternary(output, first.clone(), second.clone(), third.clone())
        }
        (RuntimeFunctionInputs::Quaternary(_, _, _, _), [first, second, third, fourth]) => {
            FunctionInvocation::quaternary(
                output,
                first.clone(),
                second.clone(),
                third.clone(),
                fourth.clone(),
            )
        }
        (RuntimeFunctionInputs::Variadic { .. }, inputs) => {
            FunctionInvocation::variadic(output, inputs.to_vec().into_boxed_slice())
        }
        (signature, inputs) => {
            return Err(MechError::new(
                crate::IncorrectNumberOfArguments {
                    expected: match signature {
                        RuntimeFunctionInputs::Nullary => 0,
                        RuntimeFunctionInputs::Unary(_) => 1,
                        RuntimeFunctionInputs::Binary(_, _) => 2,
                        RuntimeFunctionInputs::Ternary(_, _, _) => 3,
                        RuntimeFunctionInputs::Quaternary(_, _, _, _) => 4,
                        RuntimeFunctionInputs::Variadic { .. } => 0,
                    },
                    found: inputs.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
    })
}

pub struct SpecializedFunction {
    instance: FunctionInstance,
    bound_call: BoundCall,
    memory_plan: CallMemoryPlan,
}

impl SpecializedFunction {
    pub fn new(
        instance: FunctionInstance,
        bound_call: BoundCall,
        memory_plan: CallMemoryPlan,
    ) -> MResult<Self> {
        if memory_plan.bound_call != bound_call {
            return Err(
                MechError::new(crate::MemoryPlanError::DescriptorMismatch, None)
                    .with_compiler_loc(),
            );
        }
        Ok(Self {
            instance,
            bound_call,
            memory_plan,
        })
    }

    pub fn instance(&self) -> &FunctionInstance {
        &self.instance
    }

    pub fn output(&self) -> &ValueCell {
        self.instance.output()
    }

    pub fn bound_call(&self) -> &BoundCall {
        &self.bound_call
    }

    pub fn memory_plan(&self) -> &CallMemoryPlan {
        &self.memory_plan
    }

    pub fn into_parts(self) -> (FunctionInstance, BoundCall, CallMemoryPlan) {
        (self.instance, self.bound_call, self.memory_plan)
    }

    pub fn syntax_directed(
        instance: FunctionInstance,
        operation: ResolvedOperationDescriptor,
        runtime_function: RuntimeFunctionId,
        target: ExecutionTarget,
        implementation_memory: ImplementationMemoryClass,
    ) -> MResult<Self> {
        operation.validate()?;
        instance
            .invocation()
            .check_operation_memory_contract(&operation.contract)?;
        let inputs = instance
            .inputs()
            .iter()
            .map(ValueCell::resolved_descriptor)
            .collect::<MResult<Vec<_>>>()?
            .into_boxed_slice();
        let outputs = vec![instance.output().resolved_descriptor()?].into_boxed_slice();
        let bound_call =
            BoundCall::syntax_directed(operation, inputs, outputs, runtime_function, target)?;
        let memory_plan = plan_specialized_call(
            &bound_call,
            instance.inputs(),
            instance.output().representation(),
            Some(instance.output()),
            implementation_memory,
        )?;
        Self::new(instance, bound_call, memory_plan)
    }

    /// Binds a canonical output and canonical source inputs directly to a
    /// runtime factory while preserving the factory's declared arity.
    pub fn bind_factory<F>(
        output: ValueCell,
        inputs: Box<[ValueCell]>,
        bound_call: BoundCall,
    ) -> MResult<Self>
    where
        F: MechFunctionFactory,
    {
        let invocation = match (F::SIGNATURE.inputs, inputs.into_vec().as_slice()) {
            (RuntimeFunctionInputs::Nullary, []) => FunctionInvocation::nullary(output),
            (RuntimeFunctionInputs::Unary(_), [input]) => {
                FunctionInvocation::unary(output, input.clone())
            }
            (RuntimeFunctionInputs::Binary(_, _), [first, second]) => {
                FunctionInvocation::binary(output, first.clone(), second.clone())
            }
            (RuntimeFunctionInputs::Ternary(_, _, _), [first, second, third]) => {
                FunctionInvocation::ternary(output, first.clone(), second.clone(), third.clone())
            }
            (RuntimeFunctionInputs::Quaternary(_, _, _, _), [first, second, third, fourth]) => {
                FunctionInvocation::quaternary(
                    output,
                    first.clone(),
                    second.clone(),
                    third.clone(),
                    fourth.clone(),
                )
            }
            (RuntimeFunctionInputs::Variadic { .. }, inputs) => {
                FunctionInvocation::variadic(output, inputs.to_vec().into_boxed_slice())
            }
            (signature, inputs) => {
                let expected = match signature {
                    RuntimeFunctionInputs::Nullary => 0,
                    RuntimeFunctionInputs::Unary(_) => 1,
                    RuntimeFunctionInputs::Binary(_, _) => 2,
                    RuntimeFunctionInputs::Ternary(_, _, _) => 3,
                    RuntimeFunctionInputs::Quaternary(_, _, _, _) => 4,
                    RuntimeFunctionInputs::Variadic { .. } => inputs.len(),
                };
                return Err(MechError::new(
                    crate::IncorrectNumberOfArguments {
                        expected,
                        found: inputs.len(),
                    },
                    None,
                )
                .with_compiler_loc());
            }
        };
        let implementation = F::new_invocation(invocation.clone())?;
        let instance = FunctionInstance::new(implementation, invocation);
        let memory_plan = plan_specialized_call(
            &bound_call,
            instance.inputs(),
            instance.output().representation(),
            Some(instance.output()),
            F::implementation_memory_class(),
        )?;
        Self::new(instance, bound_call, memory_plan)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpecializationInputAbsent {
    pub control: String,
    pub requested: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpecializationUnknownSchema {
    pub key: SchemaKey,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpecializationSemanticCallUnavailable {
    pub semantic_operation: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpecializationResolvedCallIndexUnavailable {
    pub semantic_operation: String,
    pub category: &'static str,
    pub index: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpecializationRuntimeCatalogUnavailable {
    pub semantic_operation: String,
    pub semantic_inputs: Box<[String]>,
    pub semantic_output: String,
    pub execution_profile: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpecializationRuntimeFactoryUnavailable {
    pub operation: OperationId,
    pub semantic_operation: String,
    pub semantic_inputs: Box<[String]>,
    pub semantic_outputs: Box<[String]>,
    pub target: ExecutionTarget,
    pub candidates: Box<[RuntimeCandidateRejection]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpecializationRuntimeFactoryAmbiguous {
    pub operation: OperationId,
    pub semantic_operation: String,
    pub target: ExecutionTarget,
    pub candidates: Box<[(RuntimeFunctionId, String)]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeCandidateRejection {
    pub function: RuntimeFunctionId,
    pub name: String,
    pub reason: String,
}

impl MechErrorKind for SpecializationUnknownSchema {
    fn name(&self) -> &str {
        "SpecializationUnknownSchema"
    }

    fn message(&self) -> String {
        format!(
            "source specialization referenced unknown schema {:?}",
            self.key,
        )
    }
}

impl MechErrorKind for SpecializationSemanticCallUnavailable {
    fn name(&self) -> &str {
        "SpecializationSemanticCallUnavailable"
    }

    fn message(&self) -> String {
        format!(
            "named operation {:?} reached physical specialization without a resolved semantic call",
            self.semantic_operation,
        )
    }
}

impl MechErrorKind for SpecializationResolvedCallIndexUnavailable {
    fn name(&self) -> &str {
        "SpecializationResolvedCallIndexUnavailable"
    }

    fn message(&self) -> String {
        format!(
            "named operation {:?} has no resolved {} at index {}",
            self.semantic_operation, self.category, self.index,
        )
    }
}

impl MechErrorKind for SpecializationRuntimeCatalogUnavailable {
    fn name(&self) -> &str {
        "SpecializationRuntimeCatalogUnavailable"
    }

    fn message(&self) -> String {
        format!(
            "semantic operation `{}` with inputs {:?} and output {} cannot bind for the {} execution profile because its runtime catalog is unavailable",
            self.semantic_operation,
            self.semantic_inputs,
            self.semantic_output,
            self.execution_profile,
        )
    }
}

impl MechErrorKind for SpecializationRuntimeFactoryUnavailable {
    fn name(&self) -> &str {
        "SpecializationRuntimeFactoryUnavailable"
    }

    fn message(&self) -> String {
        format!(
            "semantic operation `{}` (0x{:016x}) has no {:?} execution implementation for ({}) -> ({}); {} candidates were rejected",
            self.semantic_operation,
            self.operation.raw(),
            self.target,
            self.semantic_inputs.join(", "),
            self.semantic_outputs.join(", "),
            self.candidates.len(),
        )
    }
}

impl MechErrorKind for SpecializationRuntimeFactoryAmbiguous {
    fn name(&self) -> &str {
        "SpecializationRuntimeFactoryAmbiguous"
    }

    fn message(&self) -> String {
        format!(
            "semantic operation `{}` (0x{:016x}) has {} equally specific {:?} execution implementations",
            self.semantic_operation,
            self.operation.raw(),
            self.candidates.len(),
            self.target,
        )
    }
}

impl MechErrorKind for SpecializationInputAbsent {
    fn name(&self) -> &str {
        "SpecializationInputAbsent"
    }

    fn message(&self) -> String {
        format!(
            "source specialization requested {} from {} control input",
            self.requested, self.control,
        )
    }
}

fn control_input_error(control: &'static str, requested: &'static str) -> MechError {
    MechError::new(
        SpecializationInputAbsent {
            control: String::from(control),
            requested: String::from(requested),
        },
        None,
    )
    .with_compiler_loc()
}

fn resolved_call_index_error(
    context: &SpecializationContext<'_>,
    category: &'static str,
    index: usize,
) -> MechError {
    MechError::new(
        SpecializationResolvedCallIndexUnavailable {
            semantic_operation: context
                .diagnostic_operation
                .clone()
                .unwrap_or_else(|| "named operation".into()),
            category,
            index,
        },
        None,
    )
    .with_compiler_loc()
}

fn empty_schema_table() -> Rc<SchemaTable> {
    Rc::new(
        SchemaTableBuilder::new()
            .finish()
            .expect("an empty schema table is valid")
            .table,
    )
}

#[doc(hidden)]
#[macro_export]
macro_rules! __mech_for_each_canonical_binop_factory_group {
    (
        $callback:path,
        $context:tt,
        $lib:ident,
        $scalar:ty,
        $scalar_name:literal,
        $scalar_token:ident;
        $cfg:meta;
        $($suffix:ident),+ $(,)?
    ) => {
        $(
            #[cfg($cfg)]
            $callback!($context, $lib, $suffix, $scalar, $scalar_name, $scalar_token);
        )+
    };
}

/// Enumerates the exact concrete binary factory surface for canonical source
/// specialization. Source execution and native registration share this exact
/// type grid without requiring an erased value projection.
#[macro_export]
macro_rules! for_each_canonical_binop_factory {
    ($callback:path, $context:tt, $lib:ident, $scalar:ty, $scalar_name:literal, $scalar_token:ident) => {
        $callback!($context, $lib, SS, $scalar, $scalar_name, $scalar_token);

        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "matrix1"; SM1, M1S, M1M1);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "matrix2"; SM2, M2S, M2M2);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "matrix3"; SM3, M3S, M3M3);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "matrix4"; SM4, M4S, M4M4);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "matrix2x3"; SM2x3, M2x3S, M2x3M2x3);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "matrix3x2"; SM3x2, M3x2S, M3x2M3x2);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "matrixd"; SMD, MDS, MDMD);

        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "row_vector2"; SR2, R2S, R2R2);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "row_vector3"; SR3, R3S, R3R3);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "row_vector4"; SR4, R4S, R4R4);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "row_vectord"; SRD, RDS, RDRD);

        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "vector2"; SV2, V2S, V2V2);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "vector3"; SV3, V3S, V3V3);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "vector4"; SV4, V4S, V4V4);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "vectord"; SVD, VDS, VDVD);

        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrix2", feature = "vector2"); M2V2, V2M2);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrix3", feature = "vector3"); M3V3, V3M3);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrix4", feature = "vector4"); M4V4, V4M4);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrix2x3", feature = "vector2"); M2x3V2, V2M2x3);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrix3x2", feature = "vector3"); M3x2V3, V3M3x2);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrixd", feature = "vectord"); MDVD, VDMD);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrixd", feature = "vector2"); MDV2, V2MD);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrixd", feature = "vector3"); MDV3, V3MD);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrixd", feature = "vector4"); MDV4, V4MD);

        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrix2", feature = "row_vector2"); M2R2, R2M2);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrix3", feature = "row_vector3"); M3R3, R3M3);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrix4", feature = "row_vector4"); M4R4, R4M4);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrix2x3", feature = "row_vector3"); M2x3R3, R3M2x3);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrix3x2", feature = "row_vector2"); M3x2R2, R2M3x2);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrixd", feature = "row_vectord"); MDRD, RDMD);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrixd", feature = "row_vector2"); MDR2, R2MD);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrixd", feature = "row_vector3"); MDR3, R3MD);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrixd", feature = "row_vector4"); MDR4, R4MD);
    };
}
