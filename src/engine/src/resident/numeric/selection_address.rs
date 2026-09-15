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

pub(super) fn bind_broadcast(
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
    if source.kind != request.output.kind
        || !matches!(
            source.kind,
            ResidentValueKind::F64 | ResidentValueKind::Snapshot
        )
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let input_schema = request
        .schemas
        .get(source.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let output_schema = request
        .schemas
        .get(request.output.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let (
        SchemaBody::Matrix {
            element: input_element,
            ..
        },
        SchemaBody::Matrix {
            element: output_element,
            ..
        },
    ) = (input_schema.body(), output_schema.body())
    else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    if input_element != output_element
        || !snapshot_arithmetic_element_supported(SemanticArithmetic::Add, input_element)
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let (input_rows, input_columns) = declared_matrix_dimensions(request, source)?;
    let (rows, columns) = declared_matrix_dimensions(request, &request.output)?;
    let shape = |rows, columns| -> Result<ResidentShape, ResidentKernelBindError> {
        Ok(ResidentShape {
            rows: u32::try_from(rows).map_err(|_| ResidentKernelBindError::UnsupportedLayout)?,
            columns: u32::try_from(columns)
                .map_err(|_| ResidentKernelBindError::UnsupportedLayout)?,
        })
    };
    let input_shape = shape(input_rows, input_columns)?;
    let output_shape = shape(rows, columns)?;
    if (source.kind == ResidentValueKind::F64
        && (source.shape != input_shape || request.output.shape != output_shape))
        || (source.kind == ResidentValueKind::Snapshot
            && (source.shape != ResidentShape::SCALAR
                || request.output.shape != ResidentShape::SCALAR))
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let mode = binary_broadcast_mode(input_shape, output_shape)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    Ok(BoundResidentKernel::new(
        broadcast,
        vec![
            input_rows as u64,
            input_columns as u64,
            rows as u64,
            columns as u64,
            mode,
        ]
        .into_boxed_slice(),
    )
    .with_snapshot_output(snapshot_output_metadata(request))
    .with_snapshot_schemas(request.schemas.clone()))
}

fn broadcast(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let [input_rows, input_columns, rows, columns, mode] = kernel.parameters() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let rows = usize::try_from(*rows).map_err(|_| ResidentKernelError::InvalidShape)?;
    let columns = usize::try_from(*columns).map_err(|_| ResidentKernelError::InvalidShape)?;
    let count = rows
        .checked_mul(columns)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let input_count = input_rows
        .checked_mul(*input_columns)
        .and_then(|n| usize::try_from(n).ok())
        .ok_or(ResidentKernelError::InvalidShape)?;
    if inputs.len() != 1 {
        return Err(ResidentKernelError::InvalidInput);
    }
    match (input(inputs, 0)?, output) {
        (ResidentValueRef::F64(source), ResidentValueMut::F64(target)) => {
            if source.len() != input_count || target.len() != count {
                return Err(ResidentKernelError::InvalidShape);
            }
            let mut changed = false;
            for (index, destination) in target.iter_mut().enumerate() {
                let value = source[binary_broadcast_index(*mode, index, rows)];
                changed |= destination.to_bits() != value.to_bits();
                *destination = value;
            }
            Ok(changed)
        }
        (ResidentValueRef::Snapshot([Some(source)]), output @ ResidentValueMut::Snapshot(_)) => {
            let schemas = kernel
                .snapshot_schemas()
                .ok_or(ResidentKernelError::InvalidInput)?;
            preflight_snapshot_arithmetic(
                kernel,
                schemas,
                &[source],
                &output,
                input_count,
                count,
                count,
            )?;
            let source = snapshot_numeric_elements(source)?;
            if source.len() != input_count {
                return Err(ResidentKernelError::InvalidShape);
            }
            let mut values = Vec::with_capacity(count);
            for row in 0..rows {
                for column in 0..columns {
                    let index = match *mode {
                        BINARY_BROADCAST_SCALAR => 0,
                        BINARY_BROADCAST_EXACT => row * columns + column,
                        BINARY_BROADCAST_COLUMN => row,
                        BINARY_BROADCAST_ROW => column,
                        _ => return Err(ResidentKernelError::InvalidInput),
                    };
                    values.push(source[index].clone());
                }
            }
            write_snapshot_data_with_work_budget(
                kernel,
                output,
                ValueDataDraft::Matrix(values.into_boxed_slice()),
                None,
            )
        }
        _ => Err(ResidentKernelError::InvalidInput),
    }
}
