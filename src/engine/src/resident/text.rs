use mech_core::{
    AccessMode, AliasPolicy, BoundResidentKernel, ChangeDetectionPolicy, DeliveryMode,
    ExternalInteraction, FunctionCatalogBuilder, ImplementationMemoryClass, MResult,
    OutputConstruction, RegionPolicy, ResidentKernelBindError, ResidentKernelBindRequest,
    ResidentKernelError, ResidentKernelInputs, ResidentKernelWorkAccounting, ResidentShape,
    ResidentSnapshotOutput, ResidentValueKind, ResidentValueMut, ResidentValueRef,
    ResolvedOperationContract, SchemaBody, ShapeRule, ValueData,
};

#[path = "text_selection_snapshot.rs"]
mod snapshot_selection;

pub(crate) fn install(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(any(
        feature = "pure_text",
        feature = "map_ops",
        feature = "sequence_buffers"
    ))]
    super::native_collection_semantics::register_string_concat(builder)?;
    #[cfg(not(any(
        feature = "pure_text",
        feature = "map_ops",
        feature = "sequence_buffers"
    )))]
    builder.insert_resident_factory(
        ["string"],
        "concat",
        ImplementationMemoryClass::NoAdditionalScratch,
        bind_concat,
    )?;
    builder.insert_resident_factory(
        ["matrix"],
        "assign-range-all",
        ImplementationMemoryClass::NoAdditionalScratch,
        bind_string_all_assign,
    )?;
    builder.insert_resident_factory(
        ["matrix"],
        "assign-scalar",
        ImplementationMemoryClass::NoAdditionalScratch,
        bind_string_index_assign,
    )?;
    builder.insert_resident_factory(
        ["matrix"],
        "assign-range",
        ImplementationMemoryClass::NoAdditionalScratch,
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
    publication_comparison_work: usize,
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
            comparison_work: publication_comparison_work,
            compute_work: compute_work
                .checked_add(publication_comparison_work)
                .ok_or(ResidentKernelError::InvalidShape)?,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StringMutationSelection<'a> {
    Bitmap(&'a [u8]),
    Indices(&'a [u64]),
    One(usize),
    All,
}

impl StringMutationSelection<'_> {
    fn is_selected(&self, index: usize) -> bool {
        match self {
            Self::Bitmap(selected) => selected[index] != 0,
            Self::Indices(selected) => selected
                .iter()
                .any(|selected| selected.checked_sub(1) == Some(index as u64)),
            Self::One(selected) => *selected == index,
            Self::All => true,
        }
    }

    fn retained_nodes(&self) -> Result<u64, ResidentKernelError> {
        match self {
            Self::Bitmap(_) | Self::Indices(_) | Self::One(_) | Self::All => Ok(1),
        }
    }
}

fn prepare_string_mutation<'a>(
    plan: StringMutationSelection<'a>,
    output_elements: usize,
    output_payload_bytes: usize,
    compute_work: usize,
    publication_comparison_work: usize,
    selector_bytes: usize,
    index_bytes: usize,
) -> Result<super::budget::AdmittedMutationPlan<StringMutationSelection<'a>>, ResidentKernelError> {
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
            comparison_work: publication_comparison_work,
            compute_work: compute_work
                .checked_add(publication_comparison_work)
                .ok_or(ResidentKernelError::InvalidShape)?,
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
    let publication_work =
        output
            .iter()
            .enumerate()
            .try_fold(0usize, |work, (index, current)| {
                let output_row = index % columns;
                let output_column = index / columns;
                work.checked_add(
                    current
                        .len()
                        .max(input[output_column + output_row * rows].len()),
                )
                .ok_or(ResidentKernelError::InvalidShape)
            })?;
    admit_string_materialization(
        output.len(),
        payload_bytes,
        payload_bytes,
        output.len(),
        publication_work,
        output.len(),
        0,
        0,
    )?;
    let mut changed = false;
    for index in 0..output.len() {
        let output_row = index % columns;
        let output_column = index / columns;
        let incoming = &input[output_column + output_row * rows];
        changed |= output[index] != *incoming;
        output[index] = incoming.clone();
    }
    Ok(changed)
}

pub(super) fn bind_string_gather(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    if canonical_string_source(request) {
        return bind_canonical_string_selection(request, false);
    }
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
    let mut output_ordinal = 0usize;
    let mut publication_work = 0usize;
    super::numeric::selector_for_each_access_index(selector, source.len(), |index| {
        publication_work = publication_work
            .checked_add(output[output_ordinal].len().max(source[index].len()))
            .ok_or(ResidentKernelError::InvalidShape)?;
        output_ordinal = output_ordinal
            .checked_add(1)
            .ok_or(ResidentKernelError::InvalidShape)?;
        Ok(())
    })?;
    admit_string_materialization(
        output.len(),
        payload_bytes,
        payload_bytes,
        output.len(),
        publication_work,
        output.len(),
        0,
        index_bytes,
    )?;
    let mut output_ordinal = 0usize;
    let mut changed = false;
    super::numeric::selector_for_each_access_index(selector, source.len(), |index| {
        let incoming = &source[index];
        changed |= output[output_ordinal] != *incoming;
        output[output_ordinal] = incoming.clone();
        output_ordinal += 1;
        Ok(())
    })?;
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

fn scalar_string_layout(
    request: &ResidentKernelBindRequest<'_>,
    layout: &mech_core::ResidentPortLayout,
) -> bool {
    matches!(
        layout.kind,
        ResidentValueKind::String | ResidentValueKind::Snapshot
    ) && layout.shape == ResidentShape::SCALAR
        && layout.shape_instance.parameter_values().is_empty()
        && request.schemas.get(layout.schema_id).is_some_and(|schema| {
            schema.body() == &SchemaBody::String
                && schema.dimension_parameters().is_empty()
                && schema.key() == layout.schema_key
        })
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
                    || !scalar_string_layout(request, layout)
            })
        || !matches!(
            request.output.kind,
            ResidentValueKind::Bool | ResidentValueKind::Snapshot
        )
        || request.output.shape != ResidentShape::SCALAR
        || !request.output.shape_instance.parameter_values().is_empty()
        || !request
            .schemas
            .get(request.output.schema_id)
            .is_some_and(|schema| {
                schema.body() == &SchemaBody::Bool
                    && schema.dimension_parameters().is_empty()
                    && schema.key() == request.output.schema_key
            })
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
    Ok(super::set::membership_output::bind(
        BoundResidentKernel::new(executor, Box::new([])),
        request,
    )
    .with_work_accounting(ResidentKernelWorkAccounting::RuntimeMetered))
}

fn string_equal(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let left = snapshot_selection::source_at(
        kernel,
        inputs.get(0).ok_or(ResidentKernelError::InvalidInput)?,
        0,
    )?;
    let right = snapshot_selection::source_at(
        kernel,
        inputs.get(1).ok_or(ResidentKernelError::InvalidInput)?,
        1,
    )?;
    admit_scalar_string_comparison(kernel, left, right)?;
    super::set::membership_output::write(kernel, output, left == right)
}

fn string_not_equal(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let left = snapshot_selection::source_at(
        kernel,
        inputs.get(0).ok_or(ResidentKernelError::InvalidInput)?,
        0,
    )?;
    let right = snapshot_selection::source_at(
        kernel,
        inputs.get(1).ok_or(ResidentKernelError::InvalidInput)?,
        1,
    )?;
    admit_scalar_string_comparison(kernel, left, right)?;
    super::set::membership_output::write(kernel, output, left != right)
}

fn admit_scalar_string_comparison(
    kernel: &BoundResidentKernel,
    left: &str,
    right: &str,
) -> Result<(), ResidentKernelError> {
    let comparison_work = left.len().max(right.len()).max(1);
    let footprint = super::set::membership_output::footprint(kernel)?;
    super::budget::PreparedKernel::new(
        (),
        super::budget::resident_cost! {
            comparison_work,
            compute_work: comparison_work,
            output_elements: footprint.elements,
            output_bytes: footprint.retained_bytes,
            retained_nodes: footprint.retained_nodes,
            ..super::budget::KernelCostEstimate::default()
        },
    )
    .admit()?
    .into_plan();
    Ok(())
}

pub(super) fn bind_string_scalar_access(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    if canonical_string_source(request) {
        return bind_canonical_string_selection(request, true);
    }
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
    Ok(BoundResidentKernel::new(string_scalar_access, Box::new([]))
        .with_work_accounting(ResidentKernelWorkAccounting::RuntimeMetered))
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
        target.len().max(next.len()),
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

fn canonical_string_source(request: &ResidentKernelBindRequest<'_>) -> bool {
    request.inputs.first().is_some_and(|source| {
        request
            .schemas
            .get(source.schema_id)
            .is_some_and(|schema| matches!(schema.body(), SchemaBody::String))
    })
}

// Snapshot selector matrices carry current geometry in the immutable value.
// Keep the actual schema and shape lifetime authority, rather than treating an
// activation lower-bound seed as the geometry of every subsequent selection.
struct CanonicalStringSelectorPlan {
    schema: mech_core::SchemaId,
    schema_key: mech_core::SchemaKey,
    shape: mech_core::ShapeInstance,
    activation_fixed_shape: bool,
}

fn canonical_string_selector_geometry(
    kernel: &BoundResidentKernel,
    selector: ResidentValueRef<'_>,
    rows: usize,
    columns: usize,
) -> Result<(usize, usize), ResidentKernelError> {
    let ResidentValueRef::Snapshot([Some(value)]) = selector else {
        return if matches!(selector, ResidentValueRef::Snapshot(_)) {
            Err(ResidentKernelError::InvalidInput)
        } else {
            Ok((rows, columns))
        };
    };
    let plan = kernel
        .retained_state::<CanonicalStringSelectorPlan>()
        .ok_or(ResidentKernelError::InvalidInput)?;
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidInput)?;
    if value.schema() != plan.schema || value.schema_key() != plan.schema_key {
        return Err(ResidentKernelError::InvalidInput);
    }
    let schema = value
        .validate_against(schemas)
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    let actual_count = match (schema.body(), value.data()) {
        (SchemaBody::Matrix { .. }, ValueData::Matrix(matrix)) => matrix.elements().len(),
        (SchemaBody::Matrix { .. }, _) => return Err(ResidentKernelError::InvalidInput),
        _ => 1,
    };
    canonical_string_selector_shape_geometry(plan, schema, value.shape(), actual_count)
}

// The materializing Snapshot leaf and a selected non-Value range projection
// use this SAME current-shape rule. A projection supplies the original range's
// certified cardinality, not a made-up Snapshot selector or activation seed.
fn canonical_string_selector_shape_geometry(
    plan: &CanonicalStringSelectorPlan,
    schema: &mech_core::Schema,
    shape: &mech_core::ShapeInstance,
    actual_count: usize,
) -> Result<(usize, usize), ResidentKernelError> {
    if !mech_core::shape_change_allowed(schema, &plan.shape, shape)
        || (plan.activation_fixed_shape && shape != &plan.shape)
    {
        return Err(ResidentKernelError::InvalidShape);
    }
    match schema.body() {
        SchemaBody::Matrix { dimensions, .. } => {
            let [rows, columns] = dimensions.as_ref() else {
                return Err(ResidentKernelError::InvalidShape);
            };
            let dimension = |axis| {
                shape
                    .resolve_dimension(axis)
                    .ok()
                    .and_then(|value| usize::try_from(value).ok())
                    .ok_or(ResidentKernelError::InvalidShape)
            };
            let (rows, columns) = (dimension(rows)?, dimension(columns)?);
            if rows.checked_mul(columns) != Some(actual_count) {
                return Err(ResidentKernelError::InvalidShape);
            }
            Ok((rows, columns))
        }
        _ => Ok((1, 1)),
    }
}

fn canonical_string_selector_byte_range(
    index: &crate::text_index::PreparedGraphemeIndex,
    ordinal: usize,
) -> Result<core::ops::Range<usize>, ResidentKernelError> {
    index
        .byte_range(ordinal)
        .ok_or(ResidentKernelError::InvalidInput)
}

// Endpoint reduction is valid ONLY for a compiler-certified increasing unit
// U64 selector. Both conversions use the ordinary Snapshot numeric selector
// primitive, including portable u32 ordinals and InvalidInput error mapping.
// The result is bytes in a caller-pinned real String, never a canonical Value.
#[cfg(all(
    feature = "native-artifact",
    any(
        feature = "pure_text",
        feature = "map_ops",
        feature = "sequence_buffers"
    )
))]
pub(in crate::resident) fn selected_u64_string_span(
    kernel: &BoundResidentKernel,
    start: u64,
    count: u64,
    index: &super::text_index::IndexUse,
    charge: impl Fn(u64, u64) -> Result<(), ResidentKernelError>,
) -> Result<core::ops::Range<usize>, ResidentKernelError> {
    // The actual selected factory bound/validated the schema and current
    // shape on the cached binding's miss. Its immutable exact-count key makes
    // this a constant-size check, not a repeated Schema::key or deep traversal.
    if kernel.parameters() != [2, 1, count] {
        return Err(ResidentKernelError::InvalidShape);
    }
    let count = usize::try_from(count).map_err(|_| ResidentKernelError::InvalidShape)?;
    // No ordinal exists in an empty selection. The current maintained Range
    // leaf rejects empty ranges before reaching here, but keep this primitive
    // honest if a future selected producer can certify one.
    if count == 0 {
        return Ok(0..0);
    }
    // The ordinary selector scan prices two units per converted element.
    // Here one or two actual endpoint conversions replace the two whole
    // scans; no discarded selector payload or output copy is charged.
    charge(if count == 1 { 2 } else { 4 }, 0)?;
    selected_u64_string_span_endpoints(start, count, index)
}

#[cfg(all(
    feature = "native-artifact",
    any(
        feature = "pure_text",
        feature = "map_ops",
        feature = "sequence_buffers"
    )
))]
pub(in crate::resident) fn validate_selected_u64_string_span_geometry(
    kernel: &BoundResidentKernel,
    shape: &mech_core::ShapeInstance,
    count: u64,
) -> Result<(), ResidentKernelError> {
    let plan = kernel
        .retained_state::<CanonicalStringSelectorPlan>()
        .ok_or(ResidentKernelError::InvalidInput)?;
    let schema = kernel
        .snapshot_schemas()
        .and_then(|schemas| schemas.get(plan.schema))
        .ok_or(ResidentKernelError::InvalidInput)?;
    let count = usize::try_from(count).map_err(|_| ResidentKernelError::InvalidShape)?;
    let (rows, columns) = canonical_string_selector_shape_geometry(plan, schema, shape, count)?;
    if rows != 1 || columns != count {
        return Err(ResidentKernelError::InvalidShape);
    }
    Ok(())
}

#[cfg(any(
    test,
    all(
        feature = "native-artifact",
        any(
            feature = "pure_text",
            feature = "map_ops",
            feature = "sequence_buffers"
        )
    )
))]
fn selected_u64_string_span_endpoints(
    start: u64,
    count: usize,
    index: &crate::text_index::PreparedGraphemeIndex,
) -> Result<core::ops::Range<usize>, ResidentKernelError> {
    if count == 0 {
        return Ok(0..0);
    }
    let convert = |ordinal| {
        mech_core::canonical_positional_index(&ValueData::U64(ordinal), index.len())
            .map_err(|_| ResidentKernelError::InvalidInput)
    };
    let first = convert(start)?;
    let last = start
        .checked_add(u64::try_from(count - 1).map_err(|_| ResidentKernelError::InvalidInput)?)
        .ok_or(ResidentKernelError::InvalidInput)?;
    let last = if count == 1 { first } else { convert(last)? };
    let first = canonical_string_selector_byte_range(index, first)?;
    let last = canonical_string_selector_byte_range(index, last)?;
    Ok(first.start..last.end)
}

#[cfg(test)]
mod selected_string_span_leaf_tests {
    use super::*;
    use crate::text_index::PreparedGraphemeIndex;

    fn index(source: &str) -> PreparedGraphemeIndex {
        PreparedGraphemeIndex::build_with(
            source,
            |capacity| {
                let mut offsets = Vec::new();
                offsets
                    .try_reserve_exact(capacity)
                    .map_err(|_| ResidentKernelError::InvalidShape)?;
                Ok(offsets)
            },
            || ResidentKernelError::InvalidShape,
        )
        .unwrap()
    }

    #[test]
    fn unit_u64_span_matches_canonical_snapshot_selector_order_for_unicode() {
        for source in [
            "",
            "q",
            "abcde",
            "e\u{301}x",
            "a\r\nb",
            "👩‍🔬🇺🇸z",
            "\u{301}\u{301}r",
        ] {
            let index = index(source);
            for first in 0..index.len() {
                for count in 1..=index.len() - first {
                    let start = (first + 1) as u64;
                    let span = selected_u64_string_span_endpoints(start, count, &index).unwrap();
                    let mut ordinary = String::new();
                    for ordinal in start..start + count as u64 {
                        let selected = mech_core::canonical_positional_index(
                            &ValueData::U64(ordinal),
                            index.len(),
                        )
                        .map_err(|_| ResidentKernelError::InvalidInput)
                        .unwrap();
                        ordinary.push_str(
                            &source
                                [canonical_string_selector_byte_range(&index, selected).unwrap()],
                        );
                    }
                    assert_eq!(&source[span], ordinary);
                }
            }
        }
    }

    #[test]
    fn unit_u64_span_preserves_snapshot_invalid_input_and_absent_endpoint_rules() {
        let empty = index("");
        let index = index("a\r\ne\u{301}👩‍🔬");
        for (start, count) in [
            (0, 1),
            (1, 5),
            (4, 2),
            (u32::MAX as u64 + 1, 1),
            (u64::MAX, 2),
        ] {
            assert_eq!(
                selected_u64_string_span_endpoints(start, count, &index),
                Err(ResidentKernelError::InvalidInput)
            );
        }
        assert_eq!(selected_u64_string_span_endpoints(0, 0, &index), Ok(0..0));
        assert_eq!(
            selected_u64_string_span_endpoints(u64::MAX, 0, &index),
            Ok(0..0)
        );
        assert_eq!(selected_u64_string_span_endpoints(1, 0, &empty), Ok(0..0));
    }
}

fn bind_canonical_string_selection(
    request: &ResidentKernelBindRequest<'_>,
    scalar: bool,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let ResolvedOperationContract::Declared(contract) = request.contract else {
        return Err(ResidentKernelBindError::UnsupportedContract);
    };
    let Some(source) = request.inputs.first() else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    if (scalar && request.inputs.len() != 2)
        || (!scalar && !matches!(request.inputs.len(), 1 | 2))
        || !matches!(
            source.kind,
            ResidentValueKind::String | ResidentValueKind::Snapshot
        )
        || source.shape != ResidentShape::SCALAR
        || !matches!(
            request.output.kind,
            ResidentValueKind::String | ResidentValueKind::Snapshot
        )
        || request.output.shape != ResidentShape::SCALAR
        || !request
            .schemas
            .get(request.output.schema_id)
            .is_some_and(|schema| matches!(schema.body(), SchemaBody::String))
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    if contract.interaction != ExternalInteraction::Pure
        || contract.inputs.len() != request.inputs.len()
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
        || output.change_detection != ChangeDetectionPolicy::KernelReported
    {
        return Err(ResidentKernelBindError::UnsupportedContract);
    }
    let finish = |mut kernel: BoundResidentKernel| {
        if request.output.kind == ResidentValueKind::Snapshot {
            kernel = kernel.with_snapshot_output(mech_core::ResidentSnapshotOutput {
                schema: request.output.schema_id,
                schema_key: request.output.schema_key,
                shape: request.output.shape_instance.clone(),
                exact_cardinality: None,
                maximum_cardinality: None,
            });
        }
        if source.kind == ResidentValueKind::Snapshot
            || request.output.kind == ResidentValueKind::Snapshot
        {
            kernel = kernel.with_snapshot_schemas(request.schemas.clone());
        }
        kernel.with_work_accounting(ResidentKernelWorkAccounting::RuntimeMetered)
    };
    let Some(selector) = request.inputs.get(1) else {
        return Ok(finish(BoundResidentKernel::new(
            canonical_string_selection,
            Box::new([0, 0, 0]),
        )));
    };
    let schema = request
        .schemas
        .get(selector.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let (rows, columns, logical) = match schema.body() {
        SchemaBody::Matrix {
            element,
            dimensions,
        } => {
            let [rows, columns] = dimensions.as_ref() else {
                return Err(ResidentKernelBindError::UnsupportedLayout);
            };
            let dimension = |value| {
                selector
                    .shape_instance
                    .resolve_dimension(value)
                    .ok()
                    .and_then(|value| u32::try_from(value).ok())
                    .ok_or(ResidentKernelBindError::UnsupportedLayout)
            };
            (
                dimension(rows)?,
                dimension(columns)?,
                element.as_ref() == &SchemaBody::Bool,
            )
        }
        _ => (1, 1, false),
    };
    if (logical
        && (scalar
            || !matches!(
                selector.kind,
                ResidentValueKind::Bool | ResidentValueKind::Snapshot
            )))
        || (!logical && !super::numeric::numeric_positional_selector_layout(request, selector))
        || (selector.kind == ResidentValueKind::Snapshot && selector.shape != ResidentShape::SCALAR)
        || (selector.kind != ResidentValueKind::Snapshot
            && selector.shape != (ResidentShape { rows, columns }))
        || (scalar && (rows != 1 || columns != 1))
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let mut kernel = BoundResidentKernel::new(
        canonical_string_selection,
        Box::new([
            if scalar { 1 } else { 2 },
            u64::from(rows),
            u64::from(columns),
        ]),
    );
    if selector.kind == ResidentValueKind::Snapshot {
        kernel = kernel
            .with_snapshot_schemas(request.schemas.clone())
            .with_retained_state(std::sync::Arc::new(CanonicalStringSelectorPlan {
                schema: selector.schema_id,
                schema_key: selector.schema_key,
                shape: selector.shape_instance.clone(),
                activation_fixed_shape: selector.activation_fixed_shape,
            }));
    }
    Ok(finish(kernel))
}

// Dense resident matrices store columns contiguously, whereas canonical
// selectors (including Snapshot matrices) visit their elements row by row.
fn visit_canonical_string_selector(
    selector: ResidentValueRef<'_>,
    rows: usize,
    columns: usize,
    upper: usize,
    mut visit: impl FnMut(usize) -> Result<(), ResidentKernelError>,
) -> Result<(), ResidentKernelError> {
    let count = rows
        .checked_mul(columns)
        .ok_or(ResidentKernelError::InvalidShape)?;
    if let ResidentValueRef::Snapshot([Some(value)]) = selector {
        let actual_count = match value.data() {
            ValueData::Matrix(matrix) => matrix.elements().len(),
            _ => 1,
        };
        if actual_count != count {
            return Err(ResidentKernelError::InvalidShape);
        }
        return super::numeric::selector_for_each_access_index(selector, upper, visit);
    }
    if selector.len() != count {
        return Err(ResidentKernelError::InvalidShape);
    }
    if matches!(selector, ResidentValueRef::Bool(_)) && count != upper {
        return Err(ResidentKernelError::InvalidShape);
    }
    for row in 0..rows {
        for column in 0..columns {
            let lane_index = column * rows + row;
            let value = match selector {
                ResidentValueRef::Index(values) => ValueData::Index(values[lane_index]),
                ResidentValueRef::F64(values) => {
                    ValueData::F64(mech_core::snapshot::F64Bits::from_f64(values[lane_index]))
                }
                ResidentValueRef::Bool(values) => {
                    match values[lane_index] {
                        0 => {}
                        1 => visit(row * columns + column)?,
                        _ => return Err(ResidentKernelError::InvalidInput),
                    }
                    continue;
                }
                _ => return Err(ResidentKernelError::InvalidInput),
            };
            let index = mech_core::canonical_positional_index(&value, upper)
                .map_err(|_| ResidentKernelError::InvalidInput)?;
            visit(index)?;
        }
    }
    Ok(())
}

fn admit_string_selection_scratch(
    compute_work: usize,
    scratch_bytes: usize,
) -> Result<(), ResidentKernelError> {
    super::budget::PreparedKernel::new(
        (),
        super::budget::resident_cost! {
            compute_work,
            temporary_bytes: scratch_bytes,
            ..super::budget::KernelCostEstimate::default()
        },
    )
    .admit()?
    .into_plan();
    Ok(())
}

fn canonical_string_selection(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let source_value = inputs.get(0).ok_or(ResidentKernelError::InvalidInput)?;
    let source = snapshot_selection::source(kernel, source_value)?;
    let [mode, rows, columns] = kernel.parameters() else {
        return Err(ResidentKernelError::InvalidShape);
    };
    if *mode == 0 {
        snapshot_selection::admit(kernel, &output, source.len(), source.len(), 0)?;
        let next = source.to_owned();
        return snapshot_selection::publish(kernel, output, next);
    }
    let selector = inputs.get(1).ok_or(ResidentKernelError::InvalidInput)?;
    let rows = usize::try_from(*rows).map_err(|_| ResidentKernelError::InvalidShape)?;
    let columns = usize::try_from(*columns).map_err(|_| ResidentKernelError::InvalidShape)?;
    // This uses only immutable schema/shape and sequence-length metadata. The
    // current count must govern admission before any selector/payload scan.
    let (rows, columns) = canonical_string_selector_geometry(kernel, selector, rows, columns)?;
    let selector_count = rows
        .checked_mul(columns)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let index = match source_value {
        ResidentValueRef::String([text]) => super::text_index::prepared(text)?,
        ResidentValueRef::Snapshot([Some(value)]) => super::text_index::prepared_snapshot(value)?,
        _ => return Err(ResidentKernelError::InvalidInput),
    };
    let extent = index.len();
    let scan_work = selector_count
        .checked_mul(2)
        .and_then(|work| work.checked_add(index.compute_work))
        .ok_or(ResidentKernelError::InvalidShape)?;
    admit_string_selection_scratch(scan_work, index.scratch_bytes)?;
    let mut selected_count = 0usize;
    let mut payload_bytes = 0usize;
    visit_canonical_string_selector(selector, rows, columns, extent, |ordinal| {
        selected_count = selected_count
            .checked_add(1)
            .ok_or(ResidentKernelError::InvalidShape)?;
        payload_bytes = payload_bytes
            .checked_add(source[canonical_string_selector_byte_range(&index, ordinal)?].len())
            .ok_or(ResidentKernelError::InvalidShape)?;
        Ok(())
    })?;
    if *mode == 1 && selected_count != 1 {
        return Err(ResidentKernelError::InvalidShape);
    }
    let compute_work = scan_work
        .checked_add(payload_bytes)
        .ok_or(ResidentKernelError::InvalidShape)?;
    // Repeated graphemes each contribute their full UTF-8 payload. Keep the
    // span table in the peak demand while staging the complete new String.
    snapshot_selection::admit(
        kernel,
        &output,
        payload_bytes,
        compute_work,
        index.scratch_bytes,
    )?;
    let mut next = String::with_capacity(payload_bytes);
    visit_canonical_string_selector(selector, rows, columns, extent, |ordinal| {
        next.push_str(&source[canonical_string_selector_byte_range(&index, ordinal)?]);
        Ok(())
    })?;
    snapshot_selection::publish(kernel, output, next)
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

pub(super) fn bind_concat(
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
                    || !scalar_string_layout(request, layout)
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
        || !scalar_string_layout(request, &request.output)
    {
        return Err(ResidentKernelBindError::UnsupportedContract);
    }
    let kernel = BoundResidentKernel::new(concat, Box::new([]));
    let kernel = if request.output.kind == ResidentValueKind::Snapshot {
        kernel
            .with_snapshot_schemas(request.schemas.clone())
            .with_snapshot_output(ResidentSnapshotOutput {
                schema: request.output.schema_id,
                schema_key: request.output.schema_key,
                shape: request.output.shape_instance.clone(),
                exact_cardinality: None,
                maximum_cardinality: None,
            })
    } else {
        kernel
    };
    Ok(kernel.with_work_accounting(ResidentKernelWorkAccounting::RuntimeMetered))
}

fn concat(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let left = snapshot_selection::source_at(
        kernel,
        inputs.get(0).ok_or(ResidentKernelError::InvalidInput)?,
        0,
    )?;
    let right = snapshot_selection::source_at(
        kernel,
        inputs.get(1).ok_or(ResidentKernelError::InvalidInput)?,
        1,
    )?;
    let length = left
        .len()
        .checked_add(right.len())
        .ok_or(ResidentKernelError::InvalidShape)?;
    snapshot_selection::admit(kernel, &output, length, length, 0)?;
    let mut next = String::with_capacity(length);
    next.push_str(left);
    next.push_str(right);
    snapshot_selection::publish(kernel, output, next)
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
    let mut selected = 0usize;
    for selected_value in indexes {
        if *selected_value > 1 {
            return Err(ResidentKernelError::InvalidInput);
        }
        selected = selected
            .checked_add(usize::from(*selected_value != 0))
            .ok_or(ResidentKernelError::InvalidShape)?;
    }
    if selected == 0 {
        return Ok(false);
    }
    let plan = StringMutationSelection::Bitmap(indexes);
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
    let publication_work =
        target
            .iter()
            .enumerate()
            .try_fold(0usize, |work, (ordinal, current)| {
                let next_len = if plan.is_selected(ordinal) {
                    source.len()
                } else {
                    current.len()
                };
                work.checked_add(current.len().max(next_len))
                    .ok_or(ResidentKernelError::InvalidShape)
            })?;
    let admitted = prepare_string_mutation(
        plan,
        target.len(),
        output_payload_bytes,
        compute_work,
        publication_work,
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
    let publication_work =
        target
            .iter()
            .enumerate()
            .try_fold(0usize, |work, (ordinal, current)| {
                let next_len = if plan.is_selected(ordinal) {
                    source.len()
                } else {
                    current.len()
                };
                work.checked_add(current.len().max(next_len))
                    .ok_or(ResidentKernelError::InvalidShape)
            })?;
    let admitted = prepare_string_mutation(
        plan,
        target.len(),
        output_payload_bytes,
        compute_work,
        publication_work,
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
        .and_then(|work| {
            target
                .len()
                .checked_mul(indexes.len())
                .and_then(|selection| selection.checked_mul(3))
                .and_then(|selection| work.checked_add(selection))
        })
        .ok_or(ResidentKernelError::InvalidShape)?;
    for index in indexes {
        checked_string_index(*index, target.len())?;
    }
    let plan = StringMutationSelection::Indices(indexes);
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
    let publication_work =
        target
            .iter()
            .enumerate()
            .try_fold(0usize, |work, (ordinal, current)| {
                let next_len = if plan.is_selected(ordinal) {
                    source.len()
                } else {
                    current.len()
                };
                work.checked_add(current.len().max(next_len))
                    .ok_or(ResidentKernelError::InvalidShape)
            })?;
    let admitted = prepare_string_mutation(
        plan,
        target.len(),
        output_payload_bytes,
        compute_work,
        publication_work,
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
    let publication_work = target.iter().try_fold(0usize, |work, current| {
        work.checked_add(current.len().max(source.len()))
            .ok_or(ResidentKernelError::InvalidShape)
    })?;
    let admitted = prepare_string_mutation(
        StringMutationSelection::All,
        target.len(),
        output_payload_bytes,
        compute_work,
        publication_work,
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
    fn snapshot_string_selector_geometry_obeys_turn_activation_and_fixed_authority() {
        use mech_core::{
            DimensionExpr, DimensionLifetime, DimensionParameterDeclaration, DimensionParameterId,
            DimensionParameterOrigin, IntegerWidth, SchemaDraft, SchemaTableBuilder,
            ValueDataDraft, ValueDraft, snapshot::SnapshotValidationContext,
        };
        let declaration = |id, lifetime| DimensionParameterDeclaration {
            id: DimensionParameterId::new(id),
            origin: DimensionParameterOrigin::Explicit,
            lifetime,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: Some(DimensionExpr::Constant(8)),
        };
        let schema = SchemaDraft {
            dimension_parameters: vec![
                declaration(0, DimensionLifetime::Activation),
                declaration(1, DimensionLifetime::Turn),
            ]
            .into_boxed_slice(),
            body: SchemaBody::Matrix {
                element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W64)),
                dimensions: Box::new([
                    DimensionExpr::Parameter(DimensionParameterId::new(0)),
                    DimensionExpr::Parameter(DimensionParameterId::new(1)),
                ]),
            },
        }
        .finalize()
        .unwrap();
        let seed = schema
            .instantiate_shape(vec![1, 0].into_boxed_slice())
            .unwrap();
        let key = schema.key();
        let mut builder = SchemaTableBuilder::new();
        let handle = builder.insert(schema).unwrap();
        let build = builder.finish().unwrap();
        let id = build.resolve(handle).unwrap();
        let (schemas, _) = build.into_parts();
        let kernel = |fixed| {
            BoundResidentKernel::new(canonical_string_selection, Box::new([2, 1, 0]))
                .with_snapshot_schemas(schemas.clone())
                .with_retained_state(std::sync::Arc::new(CanonicalStringSelectorPlan {
                    schema: id,
                    schema_key: key,
                    shape: seed.clone(),
                    activation_fixed_shape: fixed,
                }))
        };
        let value = |rows: u64, columns: u64, elements: &[u64]| {
            ValueDraft {
                schema: id,
                shape_values: vec![rows, columns].into_boxed_slice(),
                data: ValueDataDraft::Matrix(
                    elements
                        .iter()
                        .copied()
                        .map(ValueDataDraft::U64)
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                ),
            }
            .finalize(&SnapshotValidationContext::new(&schemas))
            .unwrap()
        };
        let changing = kernel(false);
        for columns in [0, 3, 1, 4, 0] {
            let elements = (1..=columns).collect::<Vec<_>>();
            let current = [Some(value(1, columns, &elements))];
            assert_eq!(
                canonical_string_selector_geometry(
                    &changing,
                    ResidentValueRef::Snapshot(&current),
                    1,
                    0
                )
                .unwrap(),
                (1, columns as usize)
            );
        }
        let wrong_activation = [Some(value(2, 1, &[1, 2]))];
        assert!(matches!(
            canonical_string_selector_geometry(
                &changing,
                ResidentValueRef::Snapshot(&wrong_activation),
                1,
                0
            ),
            Err(ResidentKernelError::InvalidShape)
        ));
        let fixed = kernel(true);
        let changed_turn = [Some(value(1, 1, &[1]))];
        assert!(matches!(
            canonical_string_selector_geometry(
                &fixed,
                ResidentValueRef::Snapshot(&changed_turn),
                1,
                0
            ),
            Err(ResidentKernelError::InvalidShape)
        ));
        let source = ["e\u{301}👩🏽‍💻Z🇺🇸".to_owned()];
        let selected = [Some(value(1, 3, &[3, 1, 3]))];
        let mut output = ["previous".to_owned()];
        changing
            .execute(
                &Refs(vec![
                    ResidentValueRef::String(&source),
                    ResidentValueRef::Snapshot(&selected),
                ]),
                ResidentValueMut::String(&mut output),
            )
            .unwrap();
        assert_eq!(output[0], "Ze\u{301}Z");
        let rectangle = BoundResidentKernel::new(canonical_string_selection, Box::new([2, 2, 0]))
            .with_snapshot_schemas(schemas.clone())
            .with_retained_state(std::sync::Arc::new(CanonicalStringSelectorPlan {
                schema: id,
                schema_key: key,
                shape: schemas
                    .get(id)
                    .unwrap()
                    .instantiate_shape(vec![2, 0].into_boxed_slice())
                    .unwrap(),
                activation_fixed_shape: false,
            }));
        let row_major = [Some(value(2, 2, &[3, 1, 4, 2]))];
        let mut rectangular_output = [String::new()];
        rectangle
            .execute(
                &Refs(vec![
                    ResidentValueRef::String(&source),
                    ResidentValueRef::Snapshot(&row_major),
                ]),
                ResidentValueMut::String(&mut rectangular_output),
            )
            .unwrap();
        assert_eq!(rectangular_output[0], "Ze\u{301}🇺🇸👩🏽‍💻");
        let invalid = [Some(value(1, 2, &[1, 9]))];
        assert!(
            changing
                .execute(
                    &Refs(vec![
                        ResidentValueRef::String(&source),
                        ResidentValueRef::Snapshot(&invalid)
                    ]),
                    ResidentValueMut::String(&mut output)
                )
                .is_err()
        );
        assert_eq!(output[0], "Ze\u{301}Z");
        changing
            .execute(
                &Refs(vec![
                    ResidentValueRef::String(&source),
                    ResidentValueRef::Snapshot(&selected),
                ]),
                ResidentValueMut::String(&mut output),
            )
            .unwrap();
        assert_eq!(output[0], "Ze\u{301}Z");
    }

    #[test]
    fn dense_scalar_text_adapters_need_no_bound_call_and_keep_raw_unicode_identity() {
        for (left, right, same) in [
            ("", "", true),
            ("é", "e\u{301}", false),
            ("🧑‍🚀", "🧑‍🚀", true),
            ("", "x", false),
        ] {
            let inputs = Inputs([left.to_owned(), right.to_owned()]);
            for (executor, expected) in [
                (string_equal as mech_core::ResidentKernelExecutor, same),
                (string_not_equal as mech_core::ResidentKernelExecutor, !same),
            ] {
                let kernel = BoundResidentKernel::new(executor, Box::new([]));
                assert!(kernel.bound_call().is_none());
                let mut output = [9_u8];
                assert!(
                    kernel
                        .execute(&inputs, ResidentValueMut::Bool(&mut output))
                        .unwrap()
                );
                assert_eq!(output, [u8::from(expected)]);
                assert!(
                    !kernel
                        .execute(&inputs, ResidentValueMut::Bool(&mut output))
                        .unwrap()
                );
            }
            let kernel = BoundResidentKernel::new(concat, Box::new([]));
            let mut output = [String::new()];
            kernel
                .execute(&inputs, ResidentValueMut::String(&mut output))
                .unwrap();
            assert_eq!(output[0], format!("{left}{right}"));
            assert!(
                !kernel
                    .execute(&inputs, ResidentValueMut::String(&mut output))
                    .unwrap()
            );
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
            Err(ResidentKernelError::ResourceLimit {
                phase: mech_core::ResidentResourcePhase::Admission,
                dimension: mech_core::MemoryBudgetDimension::OutputBytes,
                required: 16777241,
                limit: 16777216
            })
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
            Err(ResidentKernelError::ResourceLimit {
                phase: mech_core::ResidentResourcePhase::Admission,
                dimension: mech_core::MemoryBudgetDimension::OutputBytes,
                required: 16777275,
                limit: 16777216
            }),
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
            Err(ResidentKernelError::ResourceLimit {
                phase: mech_core::ResidentResourcePhase::Admission,
                dimension: mech_core::MemoryBudgetDimension::OutputBytes,
                required: 16777275,
                limit: 16777216
            }),
        );
        assert_eq!(output[0].len(), oversized.len());
        assert_eq!(output[1], "old");

        let mut output = [oversized, "old".to_owned()];
        assert_eq!(
            BoundResidentKernel::new(string_indices_assign, Box::new([]))
                .execute(&inputs, ResidentValueMut::String(&mut output)),
            Err(ResidentKernelError::ResourceLimit {
                phase: mech_core::ResidentResourcePhase::Admission,
                dimension: mech_core::MemoryBudgetDimension::OutputBytes,
                required: 16_777_275,
                limit: 16_777_216,
            }),
        );
        assert_eq!(
            output[0].len(),
            super::super::budget::MAX_RESIDENT_OUTPUT_BYTES as usize
        );
        assert_eq!(output[1], "old");

        let source = ["y".repeat(super::super::budget::MAX_RESIDENT_COMPARISON_WORK as usize / 2)];
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

    #[test]
    fn scalar_string_comparisons_meter_payloads_before_writing() {
        let oversized = "x".repeat(
            usize::try_from(super::super::budget::MAX_RESIDENT_COMPARISON_WORK).unwrap() + 1,
        );
        let inputs = Inputs([oversized.clone(), oversized]);
        for executor in [string_equal, string_not_equal] {
            let kernel = BoundResidentKernel::new(executor, Box::new([]));
            let mut output = [9_u8];
            assert_eq!(
                kernel.execute(&inputs, ResidentValueMut::Bool(&mut output)),
                Err(ResidentKernelError::ResourceLimit {
                    phase: mech_core::ResidentResourcePhase::Admission,
                    dimension: mech_core::MemoryBudgetDimension::ComparisonWork,
                    required: 65537,
                    limit: 65536
                }),
            );
            assert_eq!(output, [9]);
        }
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
