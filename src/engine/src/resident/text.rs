use mech_core::{
    AccessMode, AliasPolicy, BoundResidentKernel, ChangeDetectionPolicy, DeliveryMode,
    ExternalInteraction, FunctionCatalogBuilder, MResult, OutputConstruction, RegionPolicy,
    ResidentKernelBindError, ResidentKernelBindRequest, ResidentKernelError, ResidentKernelInputs,
    ResidentShape, ResidentValueKind, ResidentValueMut, ResidentValueRef,
    ResolvedOperationContract, ShapeRule,
};

pub(crate) fn install(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    builder.insert_resident_factory(["string"], "concat", bind_concat)?;
    builder.insert_resident_factory(["convert"], "kind", bind_f64_vector_to_string)?;
    builder.insert_resident_factory(["matrix"], "assign-range-all", bind_string_all_assign)?;
    builder.insert_resident_factory(["matrix"], "assign-scalar", bind_string_index_assign)?;
    builder.insert_resident_factory(
        ["matrix"],
        "assign-range",
        bind_semantic_string_range_assign,
    )?;
    Ok(())
}

fn checked_string_payload<'a>(
    values: impl IntoIterator<Item = &'a String>,
) -> Result<usize, ResidentKernelError> {
    values.into_iter().try_fold(0usize, |bytes, value| {
        bytes
            .checked_add(value.len())
            .ok_or(ResidentKernelError::InvalidShape)
    })
}

fn admit_string_materialization(
    output_elements: usize,
    output_payload_bytes: usize,
    cloned_payload_bytes: usize,
    compute_work: usize,
    staged_containers: usize,
    selector_bytes: usize,
    index_bytes: usize,
) -> Result<(), ResidentKernelError> {
    let container_bytes = staged_containers
        .checked_mul(core::mem::size_of::<String>())
        .ok_or(ResidentKernelError::InvalidShape)?;
    let output_bytes = output_elements
        .checked_mul(core::mem::size_of::<String>())
        .and_then(|bytes| bytes.checked_add(output_payload_bytes))
        .ok_or(ResidentKernelError::InvalidShape)?;
    super::budget::PreparedKernel::new(
        (),
        super::budget::resident_cost! {
            compute_work,
            output_elements,
            output_bytes,
            temporary_bytes: cloned_payload_bytes,
            cloned_bytes: cloned_payload_bytes,
            container_bytes,
            selector_bytes,
            index_bytes,
            retained_nodes: output_elements,
            ..super::budget::KernelCostEstimate::default()
        },
    )
    .admit()?
    .into_plan();
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum StringMutationSelection {
    Bitmap(Box<[u8]>),
    One(usize),
    All,
}

impl StringMutationSelection {
    fn is_selected(&self, index: usize) -> bool {
        match self {
            Self::Bitmap(selected) => selected[index] != 0,
            Self::One(selected) => *selected == index,
            Self::All => true,
        }
    }

    fn retained_nodes(&self) -> Result<u64, ResidentKernelError> {
        match self {
            Self::Bitmap(selected) => super::budget::checked_u64(selected.len())?
                .checked_add(1)
                .ok_or(ResidentKernelError::InvalidShape),
            Self::One(_) | Self::All => Ok(1),
        }
    }
}

fn admit_string_selection_normalization(
    compute_work: usize,
    selector_bytes: usize,
    index_bytes: usize,
) -> Result<(), ResidentKernelError> {
    super::budget::PreparedKernel::new(
        (),
        super::budget::resident_cost! {
            compute_work,
            selector_bytes,
            index_bytes,
            ..super::budget::KernelCostEstimate::default()
        },
    )
    .admit()?
    .into_plan();
    Ok(())
}

fn prepare_string_mutation(
    plan: StringMutationSelection,
    output_elements: usize,
    output_payload_bytes: usize,
    compute_work: usize,
    selector_bytes: usize,
    index_bytes: usize,
) -> Result<super::budget::AdmittedMutationPlan<StringMutationSelection>, ResidentKernelError> {
    let container_bytes = output_elements
        .checked_mul(core::mem::size_of::<String>())
        .ok_or(ResidentKernelError::InvalidShape)?;
    let output_bytes = container_bytes
        .checked_add(output_payload_bytes)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let lane_nodes = super::budget::checked_u64(output_elements)?
        .checked_add(1)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let plan_nodes = plan.retained_nodes()?;
    super::budget::PreparedMutationPlan::new(
        plan,
        super::budget::PublishedOutputFootprint {
            elements: super::budget::checked_u64(output_elements)?,
            retained_bytes: super::budget::checked_u64(output_bytes)?,
            retained_nodes: lane_nodes,
        },
        super::budget::MutationRetainedNodeFootprint {
            current_persistent: lane_nodes,
            normalized_plan: plan_nodes,
            temporary_draft: lane_nodes,
        },
        super::budget::resident_cost! {
            compute_work,
            temporary_bytes: output_payload_bytes,
            cloned_bytes: output_payload_bytes,
            container_bytes,
            selector_bytes,
            index_bytes,
            ..super::budget::KernelCostEstimate::default()
        },
    )?
    .admit()
}

pub(super) fn bind_string_transpose(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let ResolvedOperationContract::Declared(contract) = request.contract else {
        return Err(ResidentKernelBindError::UnsupportedContract);
    };
    let [input] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    if contract.interaction != ExternalInteraction::Pure
        || contract.inputs.len() != 1
        || contract.outputs.len() != 1
        || contract.inputs[0].schema != input.schema_id
        || contract.inputs[0].access != AccessMode::Read
        || contract.inputs[0].delivery != DeliveryMode::Signal
        || input.kind != ResidentValueKind::String
        || request.output.kind != ResidentValueKind::String
        || request.output.shape.rows != input.shape.columns
        || request.output.shape.columns != input.shape.rows
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let output = &contract.outputs[0];
    if output.schema != request.output.schema_id
        || output.access != AccessMode::Write
        || output.delivery != DeliveryMode::Signal
        || output.construction
            != (OutputConstruction::FullWrite {
                shape: ShapeRule::TransposeOf { input: 0 },
            })
        || output.alias != AliasPolicy::NoAlias
        || output.change_detection != ChangeDetectionPolicy::KernelReported
    {
        return Err(ResidentKernelBindError::UnsupportedContract);
    }
    Ok(BoundResidentKernel::new(
        string_transpose,
        vec![input.shape.rows as u64, input.shape.columns as u64].into_boxed_slice(),
    ))
}

fn string_transpose(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let Some(ResidentValueRef::String(input)) = inputs.get(0) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let ResidentValueMut::String(output) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let [rows, columns] = kernel.parameters() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let (rows, columns) = (*rows as usize, *columns as usize);
    if rows.checked_mul(columns) != Some(input.len()) || output.len() != input.len() {
        return Err(ResidentKernelError::InvalidShape);
    }
    let payload_bytes = checked_string_payload(input.iter())?;
    admit_string_materialization(
        output.len(),
        payload_bytes,
        payload_bytes,
        output.len(),
        output.len(),
        0,
        0,
    )?;
    let mut next = Vec::with_capacity(output.len());
    for index in 0..output.len() {
        let output_row = index % columns;
        let output_column = index / columns;
        next.push(input[output_column + output_row * rows].clone());
    }
    let changed = output != next;
    for (target, value) in output.iter_mut().zip(next) {
        *target = value;
    }
    Ok(changed)
}

pub(super) fn bind_string_gather(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let ResolvedOperationContract::Declared(contract) = request.contract else {
        return Err(ResidentKernelBindError::UnsupportedContract);
    };
    let [source, indexes] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    if contract.interaction != ExternalInteraction::Pure
        || contract.inputs.len() != 2
        || contract.outputs.len() != 1
        || contract
            .inputs
            .iter()
            .zip(request.inputs)
            .any(|(port, layout)| {
                port.schema != layout.schema_id
                    || port.access != AccessMode::Read
                    || port.delivery != DeliveryMode::Signal
            })
        || source.kind != ResidentValueKind::String
        || source.shape.len().is_none()
        || !super::numeric::numeric_positional_selector_layout(request, indexes)
        || Some(super::numeric::declared_selector_cardinality(
            request, indexes,
        )?) != request.output.shape.len()
        || request.output.kind != ResidentValueKind::String
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let output = &contract.outputs[0];
    if output.schema != request.output.schema_id
        || output.access != AccessMode::Write
        || output.delivery != DeliveryMode::Signal
        || output.construction
            != (OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            })
        || output.alias != AliasPolicy::NoAlias
        || output.change_detection != ChangeDetectionPolicy::KernelReported
    {
        return Err(ResidentKernelBindError::UnsupportedContract);
    }
    Ok(BoundResidentKernel::new(string_gather, Box::new([])))
}

fn string_gather(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let Some(ResidentValueRef::String(source)) = inputs.get(0) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let selector = inputs.get(1).ok_or(ResidentKernelError::InvalidInput)?;
    let ResidentValueMut::String(output) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let mut indexes_len = 0usize;
    let mut payload_bytes = 0usize;
    super::numeric::selector_for_each_access_index(selector, source.len(), |index| {
        indexes_len = indexes_len
            .checked_add(1)
            .ok_or(ResidentKernelError::InvalidShape)?;
        payload_bytes = payload_bytes
            .checked_add(source[index].len())
            .ok_or(ResidentKernelError::InvalidShape)?;
        Ok(())
    })?;
    if indexes_len != output.len() {
        return Err(ResidentKernelError::InvalidShape);
    }
    let index_bytes = indexes_len
        .checked_mul(core::mem::size_of::<u64>())
        .ok_or(ResidentKernelError::InvalidShape)?;
    admit_string_materialization(
        output.len(),
        payload_bytes,
        payload_bytes,
        output.len(),
        output.len(),
        0,
        index_bytes,
    )?;
    let mut next = Vec::with_capacity(output.len());
    super::numeric::selector_for_each_access_index(selector, source.len(), |index| {
        next.push(source[index].clone());
        Ok(())
    })?;
    let changed = output != next;
    for (target, value) in output.iter_mut().zip(next) {
        *target = value;
    }
    Ok(changed)
}

pub(super) fn bind_string_equal(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_string_scalar_comparison(request, string_equal)
}

pub(super) fn bind_string_not_equal(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_string_scalar_comparison(request, string_not_equal)
}

fn bind_string_scalar_comparison(
    request: &ResidentKernelBindRequest<'_>,
    executor: mech_core::ResidentKernelExecutor,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let ResolvedOperationContract::Declared(contract) = request.contract else {
        return Err(ResidentKernelBindError::UnsupportedContract);
    };
    if contract.interaction != ExternalInteraction::Pure
        || contract.inputs.len() != 2
        || contract.outputs.len() != 1
        || contract
            .inputs
            .iter()
            .zip(request.inputs)
            .any(|(port, layout)| {
                port.schema != layout.schema_id
                    || port.access != AccessMode::Read
                    || port.delivery != DeliveryMode::Signal
                    || layout.kind != ResidentValueKind::String
                    || layout.shape != ResidentShape::SCALAR
            })
        || request.output.kind != ResidentValueKind::Bool
        || request.output.shape != ResidentShape::SCALAR
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let output = &contract.outputs[0];
    if output.schema != request.output.schema_id
        || output.access != AccessMode::Write
        || output.delivery != DeliveryMode::Signal
        || output.construction
            != (OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            })
        || output.alias != AliasPolicy::NoAlias
        || output.change_detection != ChangeDetectionPolicy::ExactScalar
    {
        return Err(ResidentKernelBindError::UnsupportedContract);
    }
    Ok(BoundResidentKernel::new(executor, Box::new([])))
}

fn string_equal(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let Some(ResidentValueRef::String([left])) = inputs.get(0) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let Some(ResidentValueRef::String([right])) = inputs.get(1) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let ResidentValueMut::Bool([target]) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let next = u8::from(left == right);
    let changed = *target != next;
    *target = next;
    Ok(changed)
}

fn string_not_equal(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let Some(ResidentValueRef::String([left])) = inputs.get(0) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let Some(ResidentValueRef::String([right])) = inputs.get(1) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let ResidentValueMut::Bool([target]) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let next = u8::from(left != right);
    let changed = *target != next;
    *target = next;
    Ok(changed)
}

pub(super) fn bind_string_scalar_access(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let ResolvedOperationContract::Declared(contract) = request.contract else {
        return Err(ResidentKernelBindError::UnsupportedContract);
    };
    let [source, index] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    if contract.interaction != ExternalInteraction::Pure
        || contract.inputs.len() != 2
        || contract.outputs.len() != 1
        || contract
            .inputs
            .iter()
            .zip(request.inputs)
            .any(|(port, layout)| {
                port.schema != layout.schema_id
                    || port.access != AccessMode::Read
                    || port.delivery != DeliveryMode::Signal
            })
        || source.kind != ResidentValueKind::String
        || source.shape.len().is_none()
        || !super::numeric::numeric_positional_selector_layout(request, index)
        || super::numeric::declared_selector_cardinality(request, index)? != 1
        || request.output.kind != ResidentValueKind::String
        || request.output.shape != ResidentShape::SCALAR
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let output = &contract.outputs[0];
    if output.schema != request.output.schema_id
        || output.access != AccessMode::Write
        || output.delivery != DeliveryMode::Signal
        || output.construction
            != (OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            })
        || output.alias != AliasPolicy::NoAlias
        || output.change_detection != ChangeDetectionPolicy::KernelReported
    {
        return Err(ResidentKernelBindError::UnsupportedContract);
    }
    Ok(BoundResidentKernel::new(string_scalar_access, Box::new([])))
}

fn string_scalar_access(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let Some(ResidentValueRef::String(source)) = inputs.get(0) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let selector = inputs.get(1).ok_or(ResidentKernelError::InvalidInput)?;
    let mut selected = None;
    super::numeric::selector_for_each_access_index(selector, source.len(), |index| {
        if selected.replace(index).is_some() {
            return Err(ResidentKernelError::InvalidShape);
        }
        Ok(())
    })?;
    let next = &source[selected.ok_or(ResidentKernelError::InvalidShape)?];
    let ResidentValueMut::String([target]) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    admit_string_materialization(
        1,
        next.len(),
        next.len(),
        1,
        0,
        0,
        core::mem::size_of::<u64>(),
    )?;
    let next = next.clone();
    let changed = *target != next;
    if changed {
        *target = next;
    }
    Ok(changed)
}

fn bind_f64_vector_to_string(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let ResolvedOperationContract::Declared(contract) = request.contract else {
        return Err(ResidentKernelBindError::UnsupportedContract);
    };
    let [input] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let [input_contract] = contract.inputs.as_ref() else {
        return Err(ResidentKernelBindError::UnsupportedContract);
    };
    let [output_contract] = contract.outputs.as_ref() else {
        return Err(ResidentKernelBindError::UnsupportedContract);
    };
    if contract.interaction != ExternalInteraction::Pure
        || input_contract.schema != input.schema_id
        || input_contract.access != AccessMode::Read
        || input_contract.delivery != DeliveryMode::Signal
        || output_contract.schema != request.output.schema_id
        || output_contract.access != AccessMode::Write
        || output_contract.delivery != DeliveryMode::Signal
        || output_contract.construction
            != (OutputConstruction::FullWrite {
                shape: ShapeRule::SameAsInput { input: 0 },
            })
        || output_contract.alias != AliasPolicy::NoAlias
        || output_contract.change_detection != ChangeDetectionPolicy::KernelReported
        || input.kind != ResidentValueKind::F64
        || request.output.kind != ResidentValueKind::String
        || input.shape != request.output.shape
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    Ok(BoundResidentKernel::new(f64_vector_to_string, Box::new([])))
}

fn f64_vector_to_string(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let Some(ResidentValueRef::F64(source)) = inputs.get(0) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let ResidentValueMut::String(target) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    if source.len() != target.len() {
        return Err(ResidentKernelError::InvalidShape);
    }
    let payload_upper = source
        .len()
        .checked_mul(32)
        .ok_or(ResidentKernelError::InvalidShape)?;
    admit_string_materialization(
        source.len(),
        payload_upper,
        payload_upper,
        source.len(),
        source.len(),
        0,
        0,
    )?;
    let next = source.iter().map(f64::to_string).collect::<Vec<_>>();
    let changed = target != next;
    for (target, value) in target.iter_mut().zip(next) {
        *target = value;
    }
    Ok(changed)
}

fn validate_string_axis_zero_assignment(
    request: &ResidentKernelBindRequest<'_>,
    index_kind: ResidentValueKind,
    regions: RegionPolicy,
) -> Result<(), ResidentKernelBindError> {
    let ResolvedOperationContract::Declared(contract) = request.contract else {
        return Err(ResidentKernelBindError::UnsupportedContract);
    };
    let [base, source, index] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    if contract.interaction != ExternalInteraction::Pure
        || contract.inputs.len() != 3
        || contract.outputs.len() != 1
        || contract
            .inputs
            .iter()
            .zip(request.inputs)
            .any(|(port, layout)| {
                port.schema != layout.schema_id
                    || port.access != AccessMode::Read
                    || port.delivery != DeliveryMode::Signal
            })
        || base.kind != ResidentValueKind::String
        || source.kind != ResidentValueKind::String
        || source.shape != ResidentShape::SCALAR
        || index.kind != index_kind
        || base.schema_id != request.output.schema_id
        || base.schema_key != request.output.schema_key
        || base.shape != request.output.shape
        || request.output.kind != ResidentValueKind::String
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let output = &contract.outputs[0];
    if output.schema != request.output.schema_id
        || output.access != AccessMode::ReadWrite
        || output.delivery != DeliveryMode::Signal
        || output.construction
            != (OutputConstruction::ReadModifyWrite {
                base_input: 0,
                regions,
            })
        || output.alias != (AliasPolicy::MayAlias { input: 0 })
        || output.change_detection != ChangeDetectionPolicy::KernelReported
    {
        return Err(ResidentKernelBindError::UnsupportedContract);
    }
    Ok(())
}

fn bind_string_mask_assign(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_string_axis_zero_assignment(
        request,
        ResidentValueKind::Bool,
        RegionPolicy::IndexedAxis { axis: 0 },
    )?;
    if request.inputs[2].shape.len() != request.inputs[0].shape.len() {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    Ok(BoundResidentKernel::new(string_mask_assign, Box::new([])))
}

fn bind_string_index_assign(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_string_axis_zero_assignment(
        request,
        ResidentValueKind::Index,
        RegionPolicy::IndexedAxis { axis: 0 },
    )?;
    if request.inputs[2].shape != ResidentShape::SCALAR {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    Ok(BoundResidentKernel::new(string_index_assign, Box::new([])))
}

fn bind_string_indices_assign(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_string_axis_zero_assignment(
        request,
        ResidentValueKind::Index,
        RegionPolicy::IndexedAxis { axis: 0 },
    )?;
    Ok(BoundResidentKernel::new(
        string_indices_assign,
        Box::new([]),
    ))
}

fn bind_semantic_string_range_assign(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_string_mask_assign(request).or_else(|_| bind_string_indices_assign(request))
}

fn bind_string_all_assign(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_string_axis_zero_assignment(
        request,
        ResidentValueKind::Bool,
        RegionPolicy::WholeValue,
    )?;
    if request.inputs[2].shape != ResidentShape::SCALAR {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    Ok(BoundResidentKernel::new(string_all_assign, Box::new([])))
}

fn bind_concat(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let ResolvedOperationContract::Declared(contract) = request.contract else {
        return Err(ResidentKernelBindError::UnsupportedContract);
    };
    if contract.interaction != ExternalInteraction::Pure
        || contract.inputs.len() != 2
        || request.inputs.len() != 2
        || contract.outputs.len() != 1
        || contract
            .inputs
            .iter()
            .zip(request.inputs)
            .any(|(port, layout)| {
                port.schema != layout.schema_id
                    || port.access != AccessMode::Read
                    || port.delivery != DeliveryMode::Signal
                    || layout.kind != ResidentValueKind::String
                    || layout.shape != ResidentShape::SCALAR
            })
    {
        return Err(ResidentKernelBindError::UnsupportedContract);
    }
    let output = &contract.outputs[0];
    if output.schema != request.output.schema_id
        || output.access != AccessMode::Write
        || output.delivery != DeliveryMode::Signal
        || output.construction
            != (OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            })
        || output.alias != AliasPolicy::NoAlias
        || output.change_detection != ChangeDetectionPolicy::ExactScalar
        || request.output.kind != ResidentValueKind::String
        || request.output.shape != ResidentShape::SCALAR
    {
        return Err(ResidentKernelBindError::UnsupportedContract);
    }
    Ok(BoundResidentKernel::new(concat, Box::new([])))
}

fn concat(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let Some(ResidentValueRef::String([left])) = inputs.get(0) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let Some(ResidentValueRef::String([right])) = inputs.get(1) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let ResidentValueMut::String([target]) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let length = left
        .len()
        .checked_add(right.len())
        .ok_or(ResidentKernelError::InvalidShape)?;
    admit_string_materialization(1, length, length, length, 0, 0, 0)?;
    let mut next = String::with_capacity(length);
    next.push_str(left);
    next.push_str(right);
    let changed = *target != next;
    *target = next;
    Ok(changed)
}

fn string_mask_assign(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let Some(ResidentValueRef::String([source])) = inputs.get(0) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let Some(ResidentValueRef::Bool(indexes)) = inputs.get(1) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let ResidentValueMut::String(target) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    if indexes.len() != target.len() {
        return Err(ResidentKernelError::InvalidShape);
    }
    let compute_work = target
        .len()
        .checked_mul(4)
        .ok_or(ResidentKernelError::InvalidShape)?;
    admit_string_selection_normalization(compute_work, indexes.len(), 0)?;
    let mut selected = 0usize;
    let plan = indexes
        .iter()
        .map(|selected_value| {
            if *selected_value > 1 {
                return Err(ResidentKernelError::InvalidInput);
            }
            selected = selected
                .checked_add(usize::from(*selected_value != 0))
                .ok_or(ResidentKernelError::InvalidShape)?;
            Ok(*selected_value)
        })
        .collect::<Result<Vec<_>, ResidentKernelError>>()?
        .into_boxed_slice();
    if selected == 0 {
        return Ok(false);
    }
    let plan = StringMutationSelection::Bitmap(plan);
    let output_payload_bytes =
        target
            .iter()
            .enumerate()
            .try_fold(0usize, |bytes, (ordinal, target)| {
                bytes
                    .checked_add(if plan.is_selected(ordinal) {
                        source.len()
                    } else {
                        target.len()
                    })
                    .ok_or(ResidentKernelError::InvalidShape)
            })?;
    let admitted = prepare_string_mutation(
        plan,
        target.len(),
        output_payload_bytes,
        compute_work,
        indexes.len(),
        0,
    )?;
    let plan = admitted.into_plan();
    let next = target
        .iter()
        .enumerate()
        .map(|(ordinal, target)| {
            if plan.is_selected(ordinal) {
                source.clone()
            } else {
                target.clone()
            }
        })
        .collect::<Vec<_>>();
    let changed = target != next;
    for (target, next) in target.iter_mut().zip(next) {
        *target = next;
    }
    Ok(changed)
}

fn string_index_assign(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let Some(ResidentValueRef::String([source])) = inputs.get(0) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let Some(ResidentValueRef::Index([index])) = inputs.get(1) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let ResidentValueMut::String(target) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let Some(target_index) = usize::try_from(*index)
        .ok()
        .and_then(|index| index.checked_sub(1))
        .filter(|index| *index < target.len())
    else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let plan = StringMutationSelection::One(target_index);
    let output_payload_bytes =
        target
            .iter()
            .enumerate()
            .try_fold(0usize, |bytes, (ordinal, value)| {
                bytes
                    .checked_add(if plan.is_selected(ordinal) {
                        source.len()
                    } else {
                        value.len()
                    })
                    .ok_or(ResidentKernelError::InvalidShape)
            })?;
    let compute_work = target
        .len()
        .checked_mul(3)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let admitted = prepare_string_mutation(
        plan,
        target.len(),
        output_payload_bytes,
        compute_work,
        0,
        core::mem::size_of::<u64>(),
    )?;
    let plan = admitted.into_plan();
    let next = target
        .iter()
        .enumerate()
        .map(|(ordinal, value)| {
            if plan.is_selected(ordinal) {
                source.clone()
            } else {
                value.clone()
            }
        })
        .collect::<Vec<_>>();
    let changed = target != next;
    for (target, next) in target.iter_mut().zip(next) {
        *target = next;
    }
    Ok(changed)
}

fn string_indices_assign(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let Some(ResidentValueRef::String([source])) = inputs.get(0) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let Some(ResidentValueRef::Index(indexes)) = inputs.get(1) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let ResidentValueMut::String(target) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    if indexes.is_empty() {
        return Ok(false);
    }
    let index_bytes = indexes
        .len()
        .checked_mul(core::mem::size_of::<u64>())
        .ok_or(ResidentKernelError::InvalidShape)?;
    let compute_work = target
        .len()
        .checked_mul(4)
        .and_then(|work| work.checked_add(indexes.len()))
        .ok_or(ResidentKernelError::InvalidShape)?;
    admit_string_selection_normalization(compute_work, 0, index_bytes)?;
    let mut selected = vec![0u8; target.len()];
    for index in indexes {
        selected[checked_string_index(*index, target.len())?] = 1;
    }
    let plan = StringMutationSelection::Bitmap(selected.into_boxed_slice());
    let output_payload_bytes =
        target
            .iter()
            .enumerate()
            .try_fold(0usize, |bytes, (ordinal, value)| {
                bytes
                    .checked_add(if plan.is_selected(ordinal) {
                        source.len()
                    } else {
                        value.len()
                    })
                    .ok_or(ResidentKernelError::InvalidShape)
            })?;
    let admitted = prepare_string_mutation(
        plan,
        target.len(),
        output_payload_bytes,
        compute_work,
        0,
        index_bytes,
    )?;
    let plan = admitted.into_plan();
    let next = target
        .iter()
        .enumerate()
        .map(|(ordinal, value)| {
            if plan.is_selected(ordinal) {
                source.clone()
            } else {
                value.clone()
            }
        })
        .collect::<Vec<_>>();
    let changed = target != next;
    for (target, next) in target.iter_mut().zip(next) {
        *target = next;
    }
    Ok(changed)
}

fn checked_string_index(index: u64, upper: usize) -> Result<usize, ResidentKernelError> {
    if index == 0 || index > upper as u64 {
        return Err(ResidentKernelError::IndexOutOfRange {
            index,
            upper_bound: upper as u64,
        });
    }
    Ok(index as usize - 1)
}

fn string_all_assign(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let Some(ResidentValueRef::String([source])) = inputs.get(0) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let Some(ResidentValueRef::Bool([selected])) = inputs.get(1) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let ResidentValueMut::String(target) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    if *selected > 1 {
        return Err(ResidentKernelError::InvalidInput);
    }
    if *selected == 0 {
        return Ok(false);
    }
    let output_payload_bytes = source
        .len()
        .checked_mul(target.len())
        .ok_or(ResidentKernelError::InvalidShape)?;
    let compute_work = target
        .len()
        .checked_mul(2)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let admitted = prepare_string_mutation(
        StringMutationSelection::All,
        target.len(),
        output_payload_bytes,
        compute_work,
        1,
        0,
    )?;
    let plan = admitted.into_plan();
    let next = (0..target.len())
        .map(|ordinal| {
            debug_assert!(plan.is_selected(ordinal));
            source.clone()
        })
        .collect::<Vec<_>>();
    let changed = target != next;
    for (target, next) in target.iter_mut().zip(next) {
        *target = next;
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Inputs([String; 2]);

    impl ResidentKernelInputs for Inputs {
        fn len(&self) -> usize {
            self.0.len()
        }

        fn get(&self, index: usize) -> Option<ResidentValueRef<'_>> {
            self.0
                .get(index)
                .map(core::slice::from_ref)
                .map(ResidentValueRef::String)
        }
    }

    struct Refs<'a>(Vec<ResidentValueRef<'a>>);

    impl ResidentKernelInputs for Refs<'_> {
        fn len(&self) -> usize {
            self.0.len()
        }

        fn get(&self, index: usize) -> Option<ResidentValueRef<'_>> {
            self.0.get(index).copied()
        }
    }

    #[test]
    fn scalar_concat_writes_the_normal_resident_output() {
        let kernel = BoundResidentKernel::new(concat, Box::new([]));
        let inputs = Inputs(["Hello, ".to_string(), "Ada".to_string()]);
        let mut output = [String::new()];

        assert!(
            kernel
                .execute(&inputs, ResidentValueMut::String(&mut output))
                .unwrap()
        );
        assert_eq!(output[0], "Hello, Ada");
    }

    #[test]
    fn string_gather_first_middle_and_last_failures_are_atomic() {
        let kernel = BoundResidentKernel::new(string_gather, Box::new([]));
        let source = ["a".to_owned(), "b".to_owned(), "c".to_owned()];
        for invalid in 0..3 {
            let mut indexes = [1_u64, 2, 3];
            indexes[invalid] = 4;
            let inputs = Refs(vec![
                ResidentValueRef::String(&source),
                ResidentValueRef::Index(&indexes),
            ]);
            let mut output = ["x".to_owned(), "y".to_owned(), "z".to_owned()];
            assert!(matches!(
                kernel.execute(&inputs, ResidentValueMut::String(&mut output)),
                Err(ResidentKernelError::IndexOutOfRange { .. })
            ));
            assert_eq!(output, ["x", "y", "z"]);
        }
    }

    #[test]
    fn string_mask_first_middle_and_last_failures_are_atomic() {
        let kernel = BoundResidentKernel::new(string_mask_assign, Box::new([]));
        let source = ["replacement".to_owned()];
        for invalid in 0..3 {
            let mut mask = [1_u8, 0, 1];
            mask[invalid] = 2;
            let inputs = Refs(vec![
                ResidentValueRef::String(&source),
                ResidentValueRef::Bool(&mask),
            ]);
            let mut output = ["x".to_owned(), "y".to_owned(), "z".to_owned()];
            assert_eq!(
                kernel.execute(&inputs, ResidentValueMut::String(&mut output)),
                Err(ResidentKernelError::InvalidInput)
            );
            assert_eq!(output, ["x", "y", "z"]);
        }
    }

    #[test]
    fn string_concat_rejects_clone_amplification_before_publication() {
        let kernel = BoundResidentKernel::new(concat, Box::new([]));
        let inputs = Inputs(["x".repeat(16 * 1024 * 1024 + 1), String::new()]);
        let mut output = ["unchanged".to_owned()];
        assert_eq!(
            kernel.execute(&inputs, ResidentValueMut::String(&mut output)),
            Err(ResidentKernelError::InvalidShape)
        );
        assert_eq!(output, ["unchanged"]);
    }

    #[test]
    fn partial_string_assignments_admit_the_complete_post_write_output() {
        let source = ["replacement".to_owned()];
        let oversized = "x".repeat(super::super::budget::MAX_RESIDENT_OUTPUT_BYTES as usize);

        let mask = [0_u8, 1];
        let inputs = Refs(vec![
            ResidentValueRef::String(&source),
            ResidentValueRef::Bool(&mask),
        ]);
        let mut output = [oversized.clone(), "old".to_owned()];
        assert_eq!(
            BoundResidentKernel::new(string_mask_assign, Box::new([]))
                .execute(&inputs, ResidentValueMut::String(&mut output)),
            Err(ResidentKernelError::InvalidShape),
        );
        assert_eq!(output[0].len(), oversized.len());
        assert_eq!(output[1], "old");

        let index = [2_u64];
        let inputs = Refs(vec![
            ResidentValueRef::String(&source),
            ResidentValueRef::Index(&index),
        ]);
        let mut output = [oversized.clone(), "old".to_owned()];
        assert_eq!(
            BoundResidentKernel::new(string_index_assign, Box::new([]))
                .execute(&inputs, ResidentValueMut::String(&mut output)),
            Err(ResidentKernelError::InvalidShape),
        );
        assert_eq!(output[0].len(), oversized.len());
        assert_eq!(output[1], "old");

        let mut output = [oversized, "old".to_owned()];
        assert_eq!(
            BoundResidentKernel::new(string_indices_assign, Box::new([]))
                .execute(&inputs, ResidentValueMut::String(&mut output)),
            Err(ResidentKernelError::InvalidShape),
        );
        assert_eq!(
            output[0].len(),
            super::super::budget::MAX_RESIDENT_OUTPUT_BYTES as usize
        );
        assert_eq!(output[1], "old");

        let source = ["y".repeat(super::super::budget::MAX_RESIDENT_CLONED_BYTES as usize / 2)];
        let repeated = [2_u64, 2, 2];
        let inputs = Refs(vec![
            ResidentValueRef::String(&source),
            ResidentValueRef::Index(&repeated),
        ]);
        let mut output = ["left".to_owned(), "right".to_owned()];
        assert_eq!(
            BoundResidentKernel::new(string_indices_assign, Box::new([]))
                .execute(&inputs, ResidentValueMut::String(&mut output)),
            Ok(true),
        );
        assert_eq!(output[0], "left");
        assert_eq!(output[1], source[0]);
    }

    #[test]
    fn scalar_string_not_equal_writes_boolean_output() {
        let kernel = BoundResidentKernel::new(string_not_equal, Box::new([]));
        let inputs = Inputs(["left".to_owned(), "right".to_owned()]);
        let mut output = [0_u8];

        assert_eq!(
            kernel.execute(&inputs, ResidentValueMut::Bool(&mut output)),
            Ok(true),
        );
        assert_eq!(output, [1]);
    }

    struct NumericInput([f64; 4]);

    impl ResidentKernelInputs for NumericInput {
        fn len(&self) -> usize {
            1
        }

        fn get(&self, index: usize) -> Option<ResidentValueRef<'_>> {
            (index == 0).then_some(ResidentValueRef::F64(&self.0))
        }
    }

    #[test]
    fn f64_vector_conversion_matches_source_string_formatting() {
        let kernel = BoundResidentKernel::new(f64_vector_to_string, Box::new([]));
        let inputs = NumericInput([1.0, -2.5, f64::INFINITY, f64::NAN]);
        let mut output = [String::new(), String::new(), String::new(), String::new()];

        assert!(
            kernel
                .execute(&inputs, ResidentValueMut::String(&mut output))
                .unwrap()
        );
        assert_eq!(output, ["1", "-2.5", "inf", "NaN"]);
    }

    struct MaskInputs {
        source: [String; 1],
        indexes: [u8; 4],
    }

    impl ResidentKernelInputs for MaskInputs {
        fn len(&self) -> usize {
            2
        }

        fn get(&self, index: usize) -> Option<ResidentValueRef<'_>> {
            match index {
                0 => Some(ResidentValueRef::String(&self.source)),
                1 => Some(ResidentValueRef::Bool(&self.indexes)),
                _ => None,
            }
        }
    }

    #[test]
    fn string_mask_assignment_updates_only_selected_resident_elements() {
        let kernel = BoundResidentKernel::new(string_mask_assign, Box::new([]));
        let inputs = MaskInputs {
            source: ["Fizz".to_owned()],
            indexes: [0, 1, 0, 1],
        };
        let mut output = ["1", "2", "3", "4"].map(str::to_owned);

        assert!(
            kernel
                .execute(&inputs, ResidentValueMut::String(&mut output))
                .unwrap()
        );
        assert_eq!(output, ["1", "Fizz", "3", "Fizz"]);
    }
}
