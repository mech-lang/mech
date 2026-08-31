use crate::intrinsics::table_ops::{JoinMode, joined_table};
use mech_core::{
    AccessMode, AliasPolicy, BoundResidentKernel, ChangeDetectionPolicy, DeliveryMode,
    ExternalInteraction, FunctionCatalogBuilder, MResult, OutputConstruction,
    ResidentKernelBindError, ResidentKernelBindRequest, ResidentKernelError, ResidentKernelInputs,
    ResidentShape, ResidentSnapshotOutput, ResidentValueKind, ResidentValueMut, ResidentValueRef,
    ResolvedOperationContract, SchemaBody, ShapeRule, ValueCell, ValueData,
};

const MAX_TABLE_JOIN_WORK: usize = 65_536;
const MAX_TABLE_JOIN_OUTPUT_ROWS: usize = 65_536;
const MAX_TABLE_JOIN_OUTPUT_CELLS: usize = 65_536;

pub(crate) fn install(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    builder.insert_resident_factory(["table"], "join", bind_inner)?;
    builder.insert_resident_factory(["table"], "left-outer-join", bind_left_outer)?;
    builder.insert_resident_factory(["table"], "right-outer-join", bind_right_outer)?;
    builder.insert_resident_factory(["table"], "full-outer-join", bind_full_outer)?;
    builder.insert_resident_factory(["table"], "left-semi-join", bind_left_semi)?;
    builder.insert_resident_factory(["table"], "left-anti-join", bind_left_anti)?;
    Ok(())
}

fn validate_contract(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<(), ResidentKernelBindError> {
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
    Ok(())
}

fn bind(
    request: &ResidentKernelBindRequest<'_>,
    mode: JoinMode,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_contract(request)?;
    if request.inputs.iter().any(|input| {
        input.kind != ResidentValueKind::Snapshot
            || input.shape != ResidentShape::SCALAR
            || !matches!(
                request
                    .schemas
                    .get(input.schema_id)
                    .map(mech_core::Schema::body),
                Some(SchemaBody::Table { .. })
            )
    }) || request.output.kind != ResidentValueKind::Snapshot
        || request.output.shape != ResidentShape::SCALAR
        || !matches!(
            request
                .schemas
                .get(request.output.schema_id)
                .map(mech_core::Schema::body),
            Some(SchemaBody::Table { .. })
        )
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let (left_columns, right_columns) = match (
        request
            .schemas
            .get(request.inputs[0].schema_id)
            .map(mech_core::Schema::body),
        request
            .schemas
            .get(request.inputs[1].schema_id)
            .map(mech_core::Schema::body),
    ) {
        (
            Some(SchemaBody::Table { columns: left, .. }),
            Some(SchemaBody::Table { columns: right, .. }),
        ) => (left, right),
        _ => return Err(ResidentKernelBindError::UnsupportedLayout),
    };
    let common_columns = left_columns
        .iter()
        .filter(|left| right_columns.iter().any(|right| right.name == left.name))
        .count();
    Ok(BoundResidentKernel::new(
        table_join,
        vec![
            mode as u64,
            left_columns.len() as u64,
            right_columns.len() as u64,
            common_columns as u64,
        ]
        .into_boxed_slice(),
    )
    .with_snapshot_output(ResidentSnapshotOutput {
        schema: request.output.schema_id,
        schema_key: request.output.schema_key,
        shape: request.output.shape_instance.clone(),
        exact_cardinality: None,
        maximum_cardinality: None,
    })
    .with_snapshot_schemas(request.schemas.clone()))
}

fn bind_inner(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind(request, JoinMode::Inner)
}

fn bind_left_outer(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind(request, JoinMode::LeftOuter)
}

fn bind_right_outer(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind(request, JoinMode::RightOuter)
}

fn bind_full_outer(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind(request, JoinMode::FullOuter)
}

fn bind_left_semi(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind(request, JoinMode::LeftSemi)
}

fn bind_left_anti(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind(request, JoinMode::LeftAnti)
}

fn table_rows(value: &mech_core::snapshot::Value) -> Result<usize, ResidentKernelError> {
    let ValueData::Table(table) = value.data() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    Ok(table.column(0).map_or(0, |column| column.len()))
}

fn validate_join_bounds(
    mode: JoinMode,
    left_rows: usize,
    right_rows: usize,
    left_columns: usize,
    right_columns: usize,
    common_columns: usize,
) -> Result<(), ResidentKernelError> {
    let comparisons = left_rows
        .checked_mul(right_rows)
        .ok_or(ResidentKernelError::InvalidShape)?;
    comparisons
        .checked_mul(common_columns.max(1))
        .filter(|work| *work <= MAX_TABLE_JOIN_WORK)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let output_rows = match mode {
        JoinMode::Inner => Some(comparisons),
        JoinMode::LeftOuter => comparisons.checked_add(left_rows),
        JoinMode::RightOuter => comparisons.checked_add(right_rows),
        JoinMode::FullOuter => comparisons
            .checked_add(left_rows)
            .and_then(|count| count.checked_add(right_rows)),
        JoinMode::LeftSemi | JoinMode::LeftAnti => Some(left_rows),
    }
    .filter(|count| *count <= MAX_TABLE_JOIN_OUTPUT_ROWS)
    .ok_or(ResidentKernelError::InvalidShape)?;
    let output_columns = if matches!(mode, JoinMode::LeftSemi | JoinMode::LeftAnti) {
        left_columns
    } else {
        left_columns
            .checked_add(right_columns)
            .and_then(|count| count.checked_sub(common_columns))
            .ok_or(ResidentKernelError::InvalidShape)?
    };
    output_rows
        .checked_mul(output_columns)
        .filter(|cells| *cells <= MAX_TABLE_JOIN_OUTPUT_CELLS)
        .ok_or(ResidentKernelError::InvalidShape)?;
    Ok(())
}

fn table_join(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let (
        Some(ResidentValueRef::Snapshot([Some(left)])),
        Some(ResidentValueRef::Snapshot([Some(right)])),
    ) = (inputs.get(0), inputs.get(1))
    else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let [mode, left_columns, right_columns, common_columns] = kernel.parameters() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let mode = match *mode {
        value if value == JoinMode::Inner as u64 => JoinMode::Inner,
        value if value == JoinMode::LeftOuter as u64 => JoinMode::LeftOuter,
        value if value == JoinMode::RightOuter as u64 => JoinMode::RightOuter,
        value if value == JoinMode::FullOuter as u64 => JoinMode::FullOuter,
        value if value == JoinMode::LeftSemi as u64 => JoinMode::LeftSemi,
        value if value == JoinMode::LeftAnti as u64 => JoinMode::LeftAnti,
        _ => return Err(ResidentKernelError::InvalidInput),
    };
    validate_join_bounds(
        mode,
        table_rows(left)?,
        table_rows(right)?,
        usize::try_from(*left_columns).map_err(|_| ResidentKernelError::InvalidInput)?,
        usize::try_from(*right_columns).map_err(|_| ResidentKernelError::InvalidInput)?,
        usize::try_from(*common_columns).map_err(|_| ResidentKernelError::InvalidInput)?,
    )?;
    let left =
        ValueCell::from_snapshot(left.clone()).map_err(|_| ResidentKernelError::InvalidInput)?;
    let right =
        ValueCell::from_snapshot(right.clone()).map_err(|_| ResidentKernelError::InvalidInput)?;
    let joined = joined_table(&left, &right, mode)
        .and_then(|cell| cell.snapshot())
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    let metadata = kernel
        .snapshot_output()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let next = joined
        .rebind(metadata.schema, &metadata.shape, schemas)
        .map_err(|_| ResidentKernelError::InvalidOutput)?;
    let ResidentValueMut::Snapshot([target]) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let changed = match target.as_ref() {
        Some(current) => !current
            .language_eq(schemas, &next, schemas)
            .map_err(|_| ResidentKernelError::InvalidOutput)?,
        None => true,
    };
    *target = Some(next);
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resident_join_bounds_work_and_output_before_materialization() {
        assert_eq!(
            validate_join_bounds(JoinMode::Inner, 256, 256, 1, 1, 1),
            Ok(())
        );
        assert_eq!(
            validate_join_bounds(JoinMode::Inner, 257, 257, 1, 1, 1),
            Err(ResidentKernelError::InvalidShape)
        );
        assert_eq!(
            validate_join_bounds(JoinMode::FullOuter, 0, 65_537, 1, 1, 1),
            Err(ResidentKernelError::InvalidShape)
        );
        assert_eq!(
            validate_join_bounds(JoinMode::Inner, 256, 256, 64, 64, 0),
            Err(ResidentKernelError::InvalidShape)
        );
        assert_eq!(
            validate_join_bounds(JoinMode::Inner, 64, 64, 17, 17, 17),
            Err(ResidentKernelError::InvalidShape)
        );
    }
}
