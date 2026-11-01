//! Resident adapter for ordinary, exact lexicographic sort indices.
use std::sync::Arc;

use super::budget::{self, PreparedKernel, ResidentBudgetMeter};
use crate::intrinsics::sequence_sort as primitive;
use mech_core::snapshot::{
    SequenceView, SnapshotCanonicalizationBudget, SnapshotValidationContext,
};
use mech_core::{
    BoundResidentKernel, FunctionCatalogBuilder, IntegerWidth, MResult, ResidentKernelBindError,
    ResidentKernelBindRequest, ResidentKernelError, ResidentKernelInputs,
    ResidentKernelWorkAccounting, ResidentPortLayout, ResidentShape, ResidentSnapshotOutput,
    ResidentValueKind, ResidentValueMut, ResidentValueRef, ResolvedOperationContract, SchemaBody,
    SchemaId, SchemaKey, Value, ValueData, ValueDataDraft, ValueDraft,
};

pub(crate) fn install(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(any(feature = "pure_text", feature = "map_ops"))]
    return super::native_collection_semantics::register_sort(builder);
    #[cfg(not(any(feature = "pure_text", feature = "map_ops")))]
    builder.insert_resident_factory(
        ["sequence"],
        "sort-indices",
        mech_core::ImplementationMemoryClass::CanonicalFinalize,
        bind,
    )
}

#[derive(Clone, Debug)]
struct InputPlan {
    schema: SchemaId,
    key: SchemaKey,
    body: SchemaBody,
    kind: ResidentValueKind,
    dense_geometry: Option<(usize, usize)>,
}

fn geometry(
    body: &SchemaBody,
    shape: &mech_core::ShapeInstance,
) -> Result<(usize, usize), ResidentKernelError> {
    let SchemaBody::Matrix { dimensions, .. } = body else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let [rows, columns] = dimensions.as_ref() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let extent = |dimension| {
        shape
            .resolve_dimension(dimension)
            .ok()
            .and_then(|value| usize::try_from(value).ok())
            .ok_or(ResidentKernelError::InvalidShape)
    };
    Ok((extent(rows)?, extent(columns)?))
}

fn input_plan(
    request: &ResidentKernelBindRequest<'_>,
    layout: &ResidentPortLayout,
    position: usize,
) -> Result<InputPlan, ResidentKernelBindError> {
    let schema = request
        .schemas
        .get(layout.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let supported = match position {
        0 => {
            primitive::matrix_element(
                schema.body(),
                &SchemaBody::UnsignedInteger(IntegerWidth::W64),
            ) && layout.kind == ResidentValueKind::Snapshot
                && layout.shape == ResidentShape::SCALAR
        }
        2 => {
            primitive::matrix_element(schema.body(), &SchemaBody::String)
                && matches!(
                    layout.kind,
                    ResidentValueKind::Snapshot | ResidentValueKind::String
                )
        }
        _ => {
            schema.body() == &SchemaBody::String
                && matches!(
                    layout.kind,
                    ResidentValueKind::String | ResidentValueKind::Snapshot
                )
                && layout.shape == ResidentShape::SCALAR
        }
    };
    if !supported
        || (layout.kind == ResidentValueKind::Snapshot && layout.shape != ResidentShape::SCALAR)
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let dense_geometry = if position == 2 && layout.kind == ResidentValueKind::String {
        let actual = geometry(schema.body(), &layout.shape_instance)
            .map_err(|_| ResidentKernelBindError::UnsupportedLayout)?;
        if usize::try_from(layout.shape.rows).ok() != Some(actual.0)
            || usize::try_from(layout.shape.columns).ok() != Some(actual.1)
        {
            return Err(ResidentKernelBindError::UnsupportedLayout);
        }
        Some(actual)
    } else {
        None
    };
    Ok(InputPlan {
        schema: layout.schema_id,
        key: layout.schema_key,
        body: schema.body().clone(),
        kind: layout.kind,
        dense_geometry,
    })
}

#[derive(Clone, Debug)]
struct Plan {
    inputs: [InputPlan; 4],
}

pub(super) fn bind(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let ResolvedOperationContract::Declared(actual) = request.contract else {
        return Err(ResidentKernelBindError::UnsupportedContract);
    };
    let expected = primitive::contract();
    if request.inputs.len() != 4 || actual.inputs.len() != 4 || actual.outputs.len() != 1 {
        return Err(ResidentKernelBindError::UnsupportedContract);
    }
    let expected_inputs = expected
        .inputs
        .resolve(4)
        .map_err(|_| ResidentKernelBindError::UnsupportedContract)?;
    if actual.interaction != expected.interaction
        || actual
            .inputs
            .iter()
            .zip(request.inputs)
            .zip(expected_inputs.iter())
            .any(|((port, layout), policy)| {
                port.schema != layout.schema_id
                    || port.access != policy.access
                    || port.delivery != policy.delivery
            })
    {
        return Err(ResidentKernelBindError::UnsupportedContract);
    }
    let output = &actual.outputs[0];
    let policy = &expected.outputs[0];
    if output.schema != request.output.schema_id
        || output.access != policy.access
        || output.delivery != policy.delivery
        || output.construction != policy.construction
        || output.alias != policy.alias
        || output.change_detection != policy.change_detection
    {
        return Err(ResidentKernelBindError::UnsupportedContract);
    }
    let target = request
        .schemas
        .get(request.output.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    if !primitive::matrix_element(target.body(), &SchemaBody::Index)
        || !matches!(target.body(), SchemaBody::Matrix { dimensions, .. }
            if request.output.shape_instance.resolve_dimension(&dimensions[0]).ok() == Some(1))
        || !(request.output.kind == ResidentValueKind::Snapshot
            && request.output.shape == ResidentShape::SCALAR
            || request.output.kind == ResidentValueKind::Index && request.output.shape.rows == 1)
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    if request.output.kind == ResidentValueKind::Index {
        let (rows, columns) = geometry(target.body(), &request.output.shape_instance)
            .map_err(|_| ResidentKernelBindError::UnsupportedLayout)?;
        if rows != 1 || usize::try_from(request.output.shape.columns).ok() != Some(columns) {
            return Err(ResidentKernelBindError::UnsupportedLayout);
        }
    }
    let inputs = request
        .inputs
        .iter()
        .enumerate()
        .map(|(position, input)| input_plan(request, input, position))
        .collect::<Result<Vec<_>, _>>()?
        .try_into()
        .map_err(|_| ResidentKernelBindError::UnsupportedLayout)?;
    Ok(BoundResidentKernel::new(execute, Box::new([]))
        .with_retained_state(Arc::new(Plan { inputs }))
        .with_snapshot_output(ResidentSnapshotOutput {
            schema: request.output.schema_id,
            schema_key: request.output.schema_key,
            shape: request.output.shape_instance.clone(),
            exact_cardinality: None,
            maximum_cardinality: None,
        })
        .with_snapshot_schemas(request.schemas.clone())
        .with_work_accounting(ResidentKernelWorkAccounting::RuntimeMetered))
}

fn snapshot<'a>(
    plan: &InputPlan,
    input: ResidentValueRef<'a>,
) -> Result<&'a Value, ResidentKernelError> {
    let ResidentValueRef::Snapshot([Some(value)]) = input else {
        return Err(ResidentKernelError::InvalidInput);
    };
    if plan.kind != ResidentValueKind::Snapshot
        || value.schema() != plan.schema
        || value.schema_key() != plan.key
    {
        return Err(ResidentKernelError::InvalidInput);
    }
    Ok(value)
}

enum Strings<'a> {
    Canonical(&'a [Box<str>]),
    Dense(&'a [String], usize, usize),
}
impl Strings<'_> {
    fn len(&self) -> usize {
        match self {
            Self::Canonical(values) => values.len(),
            Self::Dense(values, ..) => values.len(),
        }
    }
    fn get(&self, ordinal: usize) -> &str {
        match self {
            Self::Canonical(values) => &values[ordinal],
            Self::Dense(values, rows, columns) => {
                &values[(ordinal % columns) * rows + ordinal / columns]
            }
        }
    }
}

fn strings<'a>(
    plan: &InputPlan,
    input: ResidentValueRef<'a>,
) -> Result<Strings<'a>, ResidentKernelError> {
    if let Some((rows, columns)) = plan.dense_geometry {
        let ResidentValueRef::String(values) = input else {
            return Err(ResidentKernelError::InvalidInput);
        };
        if rows.checked_mul(columns) != Some(values.len()) {
            return Err(ResidentKernelError::InvalidShape);
        }
        return Ok(Strings::Dense(values, rows, columns));
    }
    let value = snapshot(plan, input)?;
    let values =
        primitive::canonical_strings(value).map_err(|_| ResidentKernelError::InvalidInput)?;
    let (rows, columns) = geometry(&plan.body, value.shape())?;
    if rows.checked_mul(columns) != Some(values.len()) {
        return Err(ResidentKernelError::InvalidShape);
    }
    Ok(Strings::Canonical(values))
}

fn direction(
    plan: &InputPlan,
    input: ResidentValueRef<'_>,
) -> Result<primitive::Direction, ResidentKernelError> {
    let text = match input {
        ResidentValueRef::String([value]) if plan.kind == ResidentValueKind::String => {
            value.as_str()
        }
        value if plan.kind == ResidentValueKind::Snapshot => match snapshot(plan, value)?.data() {
            ValueData::String(value) => value.as_ref(),
            _ => return Err(ResidentKernelError::InvalidInput),
        },
        _ => return Err(ResidentKernelError::InvalidInput),
    };
    // Parsing compares only the exact three/four-byte spellings. All longer
    // values reject by length; no payload-sized scan or temporary is needed.
    primitive::Direction::parse(text).map_err(|_| ResidentKernelError::InvalidInput)
}

fn execute(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let plan = kernel
        .retained_state::<Plan>()
        .ok_or(ResidentKernelError::InvalidInput)?;
    let primary_value = snapshot(
        &plan.inputs[0],
        inputs.get(0).ok_or(ResidentKernelError::InvalidInput)?,
    )?;
    let primary =
        primitive::canonical_u64s(primary_value).map_err(|_| ResidentKernelError::InvalidInput)?;
    let (rows, columns) = geometry(&plan.inputs[0].body, primary_value.shape())?;
    if rows.checked_mul(columns) != Some(primary.len()) {
        return Err(ResidentKernelError::InvalidShape);
    }
    let secondary = strings(
        &plan.inputs[2],
        inputs.get(2).ok_or(ResidentKernelError::InvalidInput)?,
    )?;
    if primary.len() != secondary.len() {
        return Err(ResidentKernelError::InvalidShape);
    }
    let mut meter = ResidentBudgetMeter::default();
    // Prepay both constant-size direction checks before reading their bytes.
    meter.charge_compute_work(16)?;
    meter.charge_comparison_work(8)?;
    let primary_order = direction(
        &plan.inputs[1],
        inputs.get(1).ok_or(ResidentKernelError::InvalidInput)?,
    )?;
    let secondary_order = direction(
        &plan.inputs[3],
        inputs.get(3).ok_or(ResidentKernelError::InvalidInput)?,
    )?;
    let n = primary.len();
    // This first admission precedes even the String-length metadata pass.
    meter.charge_compute_work(budget::checked_u64(n)?)?;
    let mut payload = 0usize;
    let mut maximum = 0usize;
    for position in 0..n {
        let length = secondary.get(position).len();
        payload = payload
            .checked_add(length)
            .ok_or(ResidentKernelError::InvalidShape)?;
        maximum = maximum.max(length);
    }
    let work =
        primitive::sorting_work(n, maximum).map_err(|_| ResidentKernelError::InvalidShape)?;
    let metadata = kernel
        .snapshot_output()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let schemas = kernel
        .shared_snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let schema = schemas
        .get(metadata.schema)
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let shape_work = schema
        .clone_allocation_bound_bytes()
        .and_then(|bytes| {
            (metadata.shape.parameter_values().len() as u64)
                .checked_mul(32)?
                .checked_add(bytes)
        })
        .and_then(|bytes| bytes.checked_add(512))
        .ok_or(ResidentKernelError::InvalidShape)?;
    meter.charge_compute_work(shape_work)?;
    meter.charge_temporary_bytes(shape_work)?;
    PreparedKernel::new((), meter.estimate())
        .admit()?
        .into_plan();
    let shape = mech_core::shape_for_resolved_extents(schema, &[1, budget::checked_u64(n)?])
        .map_err(|_| ResidentKernelError::InvalidShape)?;
    let footprint =
        primitive::index_footprint(budget::checked_u64(n)?, shape.parameter_values().len())
            .map_err(|_| ResidentKernelError::InvalidShape)?;
    let equality = match &output {
        ResidentValueMut::Snapshot([Some(previous)]) => {
            let prior = budget::published_canonical_footprint(&mut meter, previous, schemas)?;
            budget::projected_language_equality_work(
                schemas,
                previous,
                prior,
                metadata.schema,
                shape.parameter_values().len(),
                footprint,
            )?
        }
        ResidentValueMut::Snapshot([None]) => 0,
        ResidentValueMut::Index(values) if values.len() == n => budget::checked_u64(n)?,
        _ => return Err(ResidentKernelError::InvalidOutput),
    };
    let buffers = n
        .checked_mul(
            core::mem::size_of::<usize>()
                + core::mem::size_of::<ValueData>()
                + core::mem::size_of::<ValueDataDraft>(),
        )
        .and_then(|bytes| bytes.checked_add(payload))
        .ok_or(ResidentKernelError::InvalidShape)?;
    let measured = meter.estimate();
    let finalization_work = budget::checked_u64(n)?
        .checked_mul(8)
        .and_then(|value| value.checked_add(1))
        .ok_or(ResidentKernelError::InvalidShape)?;
    PreparedKernel::new((), budget::resident_cost! {
        compute_work: measured.compute_work().checked_add(work).and_then(|value| value.checked_add(payload as u64))
            .and_then(|value| value.checked_add(finalization_work)).and_then(|value| value.checked_add(equality)).ok_or(ResidentKernelError::InvalidShape)?,
        comparison_work: measured.comparison_work().checked_add(work).and_then(|value| value.checked_add(equality)).ok_or(ResidentKernelError::InvalidShape)?,
        output_elements: n, output_bytes: footprint.retained_bytes,
        temporary_bytes: measured.temporary_bytes().checked_add(buffers as u64).and_then(|value| value.checked_add(footprint.retained_bytes.checked_mul(2)?)).ok_or(ResidentKernelError::InvalidShape)?,
        cloned_bytes: payload, retained_nodes: footprint.node_count.checked_add(n as u64).and_then(|value| value.checked_add(2)).ok_or(ResidentKernelError::InvalidShape)?,
        ..budget::KernelCostEstimate::default()
    }).admit()?.into_plan();
    let mut keys = Vec::new();
    keys.try_reserve_exact(n)
        .map_err(|_| ResidentKernelError::InvalidShape)?;
    for position in 0..n {
        keys.push(ValueData::String(secondary.get(position).into()));
    }
    let mut order = Vec::new();
    order
        .try_reserve_exact(n)
        .map_err(|_| ResidentKernelError::InvalidShape)?;
    order.resize(n, 0);
    primitive::sort_positions(&mut order, primary, primary_order, &keys, secondary_order)
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    let mut values = Vec::new();
    values
        .try_reserve_exact(n)
        .map_err(|_| ResidentKernelError::InvalidShape)?;
    for position in order {
        values.push(ValueDataDraft::Index(
            budget::checked_u64(position)?
                .checked_add(1)
                .ok_or(ResidentKernelError::InvalidShape)?,
        ));
    }
    let canonicalization = SnapshotCanonicalizationBudget::new(finalization_work);
    let next = ValueDraft {
        schema: metadata.schema,
        shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
        data: ValueDataDraft::Matrix(values.into_boxed_slice()),
    }
    .finalize(
        &SnapshotValidationContext::with_shared_schemas(schemas)
            .with_canonicalization_budget(&canonicalization),
    )
    .map_err(|_| ResidentKernelError::InvalidOutput)?;
    match output {
        ResidentValueMut::Snapshot([target]) => {
            let changed = target
                .as_ref()
                .map(|old| old.snapshot_eq(schemas, &next, schemas).map(|same| !same))
                .transpose()
                .map_err(|_| ResidentKernelError::InvalidOutput)?
                .unwrap_or(true);
            *target = Some(next);
            Ok(changed)
        }
        ResidentValueMut::Index(target) => {
            let ValueData::Matrix(matrix) = next.data() else {
                return Err(ResidentKernelError::InvalidOutput);
            };
            let SequenceView::Index(values) = matrix.elements() else {
                return Err(ResidentKernelError::InvalidOutput);
            };
            if values.len() != target.len() {
                return Err(ResidentKernelError::InvalidOutput);
            }
            let changed = values != target;
            target.copy_from_slice(values);
            Ok(changed)
        }
        _ => Err(ResidentKernelError::InvalidOutput),
    }
}
