//! Canonical position maps preserve nested selected-write occurrences.
use super::*;

pub(super) fn bind_identity_indices(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_full_write(
        request,
        1,
        ShapeRule::Declared,
        ChangeDetectionPolicy::KernelReported,
    )?;
    let [source] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let (rows, columns) = declared_matrix_dimensions(request, source)?;
    let Some(schema) = request.schemas.get(request.output.schema_id) else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    if !matches!(schema.body(), SchemaBody::Matrix { element, .. } if element.as_ref() == &SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64))
        || request.output.kind != ResidentValueKind::Snapshot
        || declared_matrix_dimensions(request, &request.output)? != (rows, columns)
        || request.output.shape != ResidentShape::SCALAR
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    Ok(BoundResidentKernel::new(
        identity_indices,
        vec![rows as u64, columns as u64].into_boxed_slice(),
    )
    .with_snapshot_output(snapshot_output_metadata(request))
    .with_snapshot_schemas(request.schemas.clone()))
}

fn identity_indices(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let [rows, columns] = kernel.parameters() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let count = rows
        .checked_mul(*columns)
        .and_then(|n| usize::try_from(n).ok())
        .ok_or(ResidentKernelError::InvalidShape)?;
    if inputs.len() != 1 {
        return Err(ResidentKernelError::InvalidShape);
    }
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    preflight_snapshot_arithmetic(kernel, schemas, &[], &output, 0, count, count)?;
    let mut values = Vec::with_capacity(count);
    for row in 0..*rows {
        for column in 0..*columns {
            let index = column
                .checked_mul(*rows)
                .and_then(|v| v.checked_add(row))
                .and_then(|v| v.checked_add(1))
                .ok_or(ResidentKernelError::InvalidShape)?;
            values.push(ValueDataDraft::U64(index));
        }
    }
    write_snapshot_data_with_work_budget(
        kernel,
        output,
        ValueDataDraft::Matrix(values.into_boxed_slice()),
        None,
    )
}

pub(super) fn bind_selection_order(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    // Reuse the existing checked flatten binder, then retain source geometry
    // for canonical row-major occurrence order instead of linear index order.
    let _validated = bind_all_elements_range(request)?;
    let (rows, columns) = declared_matrix_dimensions(request, &request.inputs[0])?;
    if request.inputs[0].kind == ResidentValueKind::Snapshot {
        let schema = request
            .schemas
            .get(request.inputs[0].schema_id)
            .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
        let SchemaBody::Matrix { element, .. } = schema.body() else {
            return Err(ResidentKernelBindError::UnsupportedLayout);
        };
        if !snapshot_arithmetic_element_supported(SemanticArithmetic::Add, element) {
            return Err(ResidentKernelBindError::UnsupportedLayout);
        }
        return Ok(
            BoundResidentKernel::new(snapshot_selection_order, Box::new([]))
                .with_snapshot_output(snapshot_output_metadata(request))
                .with_snapshot_schemas(request.schemas.clone()),
        );
    }
    Ok(BoundResidentKernel::new(
        selection_order,
        vec![rows as u64, columns as u64].into_boxed_slice(),
    ))
}

fn snapshot_selection_order(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 1 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let ResidentValueRef::Snapshot([Some(source)]) = input(inputs, 0)? else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let count = snapshot_numeric_element_count(source)?;
    preflight_snapshot_arithmetic(kernel, schemas, &[source], &output, count, count, 0)?;
    let data = source
        .canonical_data_draft()
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    write_snapshot_data_with_work_budget(kernel, output, data, None)
}

fn selection_order(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let [rows, columns] = kernel.parameters() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let rows = usize::try_from(*rows).map_err(|_| ResidentKernelError::InvalidShape)?;
    let columns = usize::try_from(*columns).map_err(|_| ResidentKernelError::InvalidShape)?;
    let count = rows
        .checked_mul(columns)
        .ok_or(ResidentKernelError::InvalidShape)?;
    if inputs.len() != 1 || output.len() != count {
        return Err(ResidentKernelError::InvalidShape);
    }
    macro_rules! copy_order {
        ($source:expr, $target:expr, $equal:expr) => {{
            if $source.len() != count {
                return Err(ResidentKernelError::InvalidShape);
            }
            let mut changed = false;
            for row in 0..rows {
                for column in 0..columns {
                    let source = &$source[column * rows + row];
                    let target = &mut $target[row * columns + column];
                    changed |= !$equal(target, source);
                    *target = source.clone();
                }
            }
            Ok(changed)
        }};
    }
    match (input(inputs, 0)?, output) {
        (ResidentValueRef::Index(source), ResidentValueMut::Index(target)) => {
            copy_order!(source, target, |a: &u64, b: &u64| a == b)
        }
        (ResidentValueRef::F64(source), ResidentValueMut::F64(target)) => {
            copy_order!(source, target, |a: &f64, b: &f64| a.to_bits()
                == b.to_bits())
        }
        _ => Err(ResidentKernelError::InvalidInput),
    }
}
