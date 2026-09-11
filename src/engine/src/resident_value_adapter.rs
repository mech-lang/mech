//! Snapshot conversion at the boundary of the compact resident representation.

use mech_core::snapshot::{F64Bits, SequenceView, SnapshotValidationContext};
use mech_core::{
    GenericError, MResult, ManagedMemoryBudget, ManagedMemoryReservation, MechError,
    MemoryRuntimeError, ResidentShape, ResidentValueMut, ResidentValueRef, SchemaBody, SchemaDraft,
    SchemaId, SchemaTableBuilder, ShapeInstance, Value, ValueData, ValueDataDraft, ValueDraft,
};

use crate::resident::general::{
    ReactiveInstance, ResidentActivationError, ResidentRegion, ResidentValueBorrow,
    TypedResidentArena,
};

impl ReactiveInstance {
    pub fn copied_output(&self, output: usize) -> Result<Value, ResidentActivationError> {
        let declaration = self
            .plan
            .outputs
            .get(output)
            .ok_or(ResidentActivationError::UnknownOutput { output })?;
        let borrowed = self
            .output_borrow(output)
            .ok_or(ResidentActivationError::UnknownOutput { output })?;
        let scalar = !matches!(
            self.plan
                .schemas
                .entry(declaration.schema)
                .expect("activated output schema remains present")
                .schema()
                .body(),
            SchemaBody::Matrix { .. }
        );
        let source = match borrowed {
            ResidentValueBorrow::Bool { values, .. } => ResidentValueRef::Bool(values),
            ResidentValueBorrow::Index { values, .. } => ResidentValueRef::Index(values),
            ResidentValueBorrow::F64 { values, .. } => ResidentValueRef::F64(values),
            ResidentValueBorrow::String { values, .. } => ResidentValueRef::String(values),
            ResidentValueBorrow::Snapshot { values, .. } => ResidentValueRef::Snapshot(values),
        };
        let mut reservation = prepare_value_export(
            self.memory_budget().as_ref(),
            &self.plan.schemas,
            &declaration.shape,
            source,
            false,
        )
        .map_err(|error| ResidentActivationError::MemoryRuntime { error })?;
        let data = match borrowed {
            ResidentValueBorrow::Bool { values, .. } if scalar => {
                ValueDataDraft::Bool(values[0] != 0)
            }
            ResidentValueBorrow::Index { values, .. } if scalar => ValueDataDraft::Index(values[0]),
            ResidentValueBorrow::F64 { values, .. } if scalar => {
                ValueDataDraft::F64(F64Bits::from_f64(values[0]))
            }
            ResidentValueBorrow::String { values, .. } if scalar => {
                ValueDataDraft::String(values[0].clone())
            }
            ResidentValueBorrow::Snapshot {
                values: [Some(value)],
                ..
            } => {
                return retain_export(value.clone(), &mut reservation)
                    .map_err(|error| ResidentActivationError::MemoryRuntime { error });
            }
            ResidentValueBorrow::Bool { values, .. } => ValueDataDraft::Matrix(
                canonical_matrix_indices(declaration.region.shape)
                    .map(|index| ValueDataDraft::Bool(values[index] != 0))
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
            ResidentValueBorrow::Index { values, .. } => ValueDataDraft::Matrix(
                canonical_matrix_indices(declaration.region.shape)
                    .map(|index| ValueDataDraft::Index(values[index]))
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
            ResidentValueBorrow::F64 { values, .. } => ValueDataDraft::Matrix(
                canonical_matrix_indices(declaration.region.shape)
                    .map(|index| ValueDataDraft::F64(F64Bits::from_f64(values[index])))
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
            ResidentValueBorrow::String { values, .. } => ValueDataDraft::Matrix(
                canonical_matrix_indices(declaration.region.shape)
                    .map(|index| ValueDataDraft::String(values[index].clone()))
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
            ResidentValueBorrow::Snapshot { .. } => {
                return Err(ResidentActivationError::InvalidSnapshotRepresentation);
            }
        };
        let value = ValueDraft {
            schema: declaration.schema,
            shape_values: declaration
                .shape
                .parameter_values()
                .to_vec()
                .into_boxed_slice(),
            data,
        }
        .finalize(&SnapshotValidationContext::new(&self.plan.schemas))
        .map_err(|_| ResidentActivationError::InvalidSnapshotRepresentation)?;
        retain_export(value, &mut reservation)
            .map_err(|error| ResidentActivationError::MemoryRuntime { error })
    }
}

pub(crate) fn materialize_resident_value(
    schemas: &mech_core::SchemaTable,
    schema: SchemaId,
    shape: &ShapeInstance,
    region: ResidentRegion,
    borrowed: ResidentValueRef<'_>,
    memory_budget: Option<&ManagedMemoryBudget>,
) -> MResult<Value> {
    let scalar = !matches!(
        schemas
            .entry(schema)
            .expect("activated schema remains present")
            .schema()
            .body(),
        SchemaBody::Matrix { .. }
    );
    let mut reservation = prepare_value_export(memory_budget, schemas, shape, borrowed, true)
        .map_err(|error| MechError::new(error, None))?;
    let data = match borrowed {
        ResidentValueRef::Bool(values) if scalar => ValueDataDraft::Bool(values[0] != 0),
        ResidentValueRef::Index(values) if scalar => ValueDataDraft::Index(values[0]),
        ResidentValueRef::F64(values) if scalar => {
            ValueDataDraft::F64(F64Bits::from_f64(values[0]))
        }
        ResidentValueRef::String(values) if scalar => ValueDataDraft::String(values[0].clone()),
        ResidentValueRef::Snapshot([Some(value)]) => {
            return retain_export(value.clone(), &mut reservation)
                .map_err(|error| MechError::new(error, None));
        }
        ResidentValueRef::Bool(values) => ValueDataDraft::Matrix(
            canonical_matrix_indices(region.shape)
                .map(|index| ValueDataDraft::Bool(values[index] != 0))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
        ResidentValueRef::Index(values) => ValueDataDraft::Matrix(
            canonical_matrix_indices(region.shape)
                .map(|index| ValueDataDraft::Index(values[index]))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
        ResidentValueRef::F64(values) => ValueDataDraft::Matrix(
            canonical_matrix_indices(region.shape)
                .map(|index| ValueDataDraft::F64(F64Bits::from_f64(values[index])))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
        ResidentValueRef::String(values) => ValueDataDraft::Matrix(
            canonical_matrix_indices(region.shape)
                .map(|index| ValueDataDraft::String(values[index].clone()))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
        ResidentValueRef::Snapshot(_) => {
            return Err(MechError::new(
                GenericError {
                    msg: "resident snapshot value is uninitialized".to_string(),
                },
                None,
            ));
        }
    };
    let value = ValueDraft {
        schema,
        shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
        data,
    }
    .finalize(&SnapshotValidationContext::new(schemas))
    .map_err(|error| {
        MechError::new(
            GenericError {
                msg: format!("resident value materialization failed: {error:?}"),
            },
            None,
        )
    })?;
    let value = close_materialized_schema(value)?;
    retain_export(value, &mut reservation).map_err(|error| MechError::new(error, None))
}

/// Admit the actual export boundary before cloning a String, creating the
/// row-major draft, or finalizing its independent immutable owner. Export is
/// not a kernel output quota: it shares the caller's total backing account.
fn prepare_value_export(
    budget: Option<&ManagedMemoryBudget>,
    schemas: &mech_core::SchemaTable,
    shape: &ShapeInstance,
    borrowed: ResidentValueRef<'_>,
    close_schema: bool,
) -> Result<Option<ManagedMemoryReservation>, MemoryRuntimeError> {
    let Some(budget) = budget else {
        return Ok(None);
    };
    if let ResidentValueRef::Snapshot([Some(value)]) = borrowed {
        let required = value.memory_budget_admission_bytes(budget)?;
        return budget.reserve_capacity(required).map(Some);
    }
    let overflow = || MemoryRuntimeError::InvalidLayout {
        object: None,
        size: u64::MAX,
        alignment: core::mem::align_of::<ValueDataDraft>() as u32,
        reason: "Resident export footprint overflows",
    };
    let elements = u64::try_from(borrowed.len()).map_err(|_| overflow())?;
    // ValueData bounds every packed scalar lane header, including Box<str>.
    // The existing R5 derivation supplies draft nodes, finalization roots,
    // shape storage, and the complete cloned schema context.
    let mut payload = elements
        .checked_mul(core::mem::size_of::<ValueData>() as u64)
        .ok_or_else(overflow)?;
    if let ResidentValueRef::String(values) = borrowed {
        for value in values {
            payload = payload
                .checked_add(value.len() as u64)
                .ok_or_else(overflow)?;
        }
    }
    let footprint = mech_core::CurrentMemoryFootprint {
        logical_elements: elements,
        payload_bytes: payload,
        encoded_bytes: payload,
        retained_nodes: elements.checked_add(1).ok_or_else(overflow)?,
        schema_bytes: schemas
            .clone_allocation_bound_bytes()
            .ok_or_else(overflow)?,
        shape_parameter_count: shape.parameter_values().len() as u64,
        ..mech_core::CurrentMemoryFootprint::default()
    };
    let draft = mech_core::canonical_snapshot_draft_bytes(footprint).map_err(|_| overflow())?;
    let finalized =
        mech_core::canonical_snapshot_finalization_bytes(footprint).map_err(|_| overflow())?;
    let mut peak = draft.checked_add(finalized).ok_or_else(overflow)?;
    if close_schema {
        // The detached effect ABI may close dynamic dimensions: the original
        // snapshot coexists with its rebound draft/finalized tree, while the
        // schema builder retains input, staged, and finished schema tables.
        peak = peak
            .checked_add(draft)
            .and_then(|bytes| bytes.checked_add(finalized))
            .and_then(|bytes| bytes.checked_add(footprint.schema_bytes))
            .and_then(|bytes| bytes.checked_add(footprint.schema_bytes))
            .ok_or_else(overflow)?;
    }
    budget.reserve_capacity(peak).map(Some)
}

fn retain_export(
    value: Value,
    reservation: &mut Option<ManagedMemoryReservation>,
) -> Result<Value, MemoryRuntimeError> {
    if let Some(reservation) = reservation {
        return value.into_memory_budget(reservation);
    }
    Ok(value)
}

/// External resident payloads are detached values. Close their semantic
/// schema to the current shape so consumers never need the artifact's live
/// dimension environment to interpret the snapshot.
fn close_materialized_schema(value: Value) -> MResult<Value> {
    let schemas = value.schemas().ok_or_else(|| {
        resident_materialization_error(format!(
            "schema {:?} has no detached schema table",
            value.schema()
        ))
    })?;
    let schema = value
        .validate_against(&schemas)
        .map_err(|error| resident_materialization_error(format!("{error:?}")))?;
    let closed_body = schema.closed_body(value.shape())?;
    if schema.dimension_parameters().is_empty() && schema.body() == &closed_body {
        return Ok(value);
    }

    let closed = SchemaDraft {
        dimension_parameters: Box::new([]),
        body: closed_body,
    }
    .finalize()?;
    let mut builder = SchemaTableBuilder::new();
    for entry in schemas.entries() {
        builder.insert(entry.schema().clone())?;
    }
    let closed_handle = builder.insert(closed)?;
    let build = builder.finish()?;
    let closed_schema = build.resolve(closed_handle)?;
    let closed_shape = build
        .table
        .get(closed_schema)
        .expect("newly inserted closed schema remains present")
        .instantiate_shape(Box::new([]))?;
    value
        .rebind(closed_schema, &closed_shape, &build.table)
        .map_err(|error| resident_materialization_error(format!("{error:?}")))
}

fn resident_materialization_error(reason: String) -> MechError {
    MechError::new(
        GenericError {
            msg: format!("resident value materialization failed: {reason}"),
        },
        None,
    )
}

pub(crate) fn write_value(
    arena: &mut TypedResidentArena,
    region: ResidentRegion,
    value: &Value,
) -> Result<(), ResidentActivationError> {
    let scope = arena
        .prepare_payload_write(region)
        .map_err(|error| ResidentActivationError::MemoryRuntime { error })?;
    if let Some(scope) = &scope {
        scope
            .admit_value(value)
            .map_err(|error| ResidentActivationError::MemoryRuntime { error })?;
        scope.start();
    }
    let result = write_value_unchecked(arena, region, value);
    arena
        .finish_payload_write(region, scope)
        .map_err(|error| ResidentActivationError::MemoryRuntime { error })?;
    result
}

fn write_value_unchecked(
    arena: &mut TypedResidentArena,
    region: ResidentRegion,
    value: &Value,
) -> Result<(), ResidentActivationError> {
    match (arena.write(region), value.data()) {
        (ResidentValueMut::Bool(target), ValueData::Bool(value)) if target.len() == 1 => {
            target[0] = u8::from(*value);
        }
        (ResidentValueMut::Index(target), ValueData::Index(value)) if target.len() == 1 => {
            target[0] = *value;
        }
        (ResidentValueMut::F64(target), ValueData::F64(value)) if target.len() == 1 => {
            target[0] = value.to_f64();
        }
        (ResidentValueMut::String(target), ValueData::String(value)) if target.len() == 1 => {
            target[0] = value.to_string();
        }
        (ResidentValueMut::Snapshot(target), _) if target.len() == 1 => {
            target[0] = Some(value.clone());
        }
        (ResidentValueMut::Bool(target), ValueData::Matrix(matrix)) => {
            let SequenceView::Bool(source) = matrix.elements() else {
                return Err(ResidentActivationError::InvalidSnapshotRepresentation);
            };
            if target.len() != source.len() {
                return Err(ResidentActivationError::InvalidSnapshotRepresentation);
            }
            for (canonical, physical) in canonical_matrix_indices(region.shape).enumerate() {
                target[physical] = u8::from(source[canonical]);
            }
        }
        (ResidentValueMut::Index(target), ValueData::Matrix(matrix)) => {
            let SequenceView::Index(source) = matrix.elements() else {
                return Err(ResidentActivationError::InvalidSnapshotRepresentation);
            };
            if target.len() != source.len() {
                return Err(ResidentActivationError::InvalidSnapshotRepresentation);
            }
            for (canonical, physical) in canonical_matrix_indices(region.shape).enumerate() {
                target[physical] = source[canonical];
            }
        }
        (ResidentValueMut::F64(target), ValueData::Matrix(matrix)) => {
            let SequenceView::F64(source) = matrix.elements() else {
                return Err(ResidentActivationError::InvalidSnapshotRepresentation);
            };
            if target.len() != source.len() {
                return Err(ResidentActivationError::InvalidSnapshotRepresentation);
            }
            for (canonical, physical) in canonical_matrix_indices(region.shape).enumerate() {
                target[physical] = source[canonical].to_f64();
            }
        }
        (ResidentValueMut::String(target), ValueData::Matrix(matrix)) => {
            let SequenceView::String(source) = matrix.elements() else {
                return Err(ResidentActivationError::InvalidSnapshotRepresentation);
            };
            if target.len() != source.len() {
                return Err(ResidentActivationError::InvalidSnapshotRepresentation);
            }
            for (canonical, physical) in canonical_matrix_indices(region.shape).enumerate() {
                target[physical] = source[canonical].to_string();
            }
        }
        _ => return Err(ResidentActivationError::InvalidSnapshotRepresentation),
    }
    Ok(())
}

fn canonical_matrix_indices(shape: ResidentShape) -> impl ExactSizeIterator<Item = usize> {
    let rows = shape.rows as usize;
    let columns = shape.columns as usize;
    (0..rows * columns).map(move |canonical| {
        let row = canonical / columns;
        let column = canonical % columns;
        column * rows + row
    })
}

#[cfg(test)]
mod export_budget_tests {
    use super::*;

    #[test]
    fn parameterized_snapshot_export_shares_shape_and_retains_its_budget_owner() {
        use mech_core::{
            DimensionExpr, DimensionLifetime, DimensionParameterDeclaration, DimensionParameterId,
            DimensionParameterOrigin, ResidentValueKind,
        };
        let mut schemas = SchemaTableBuilder::new();
        let schema = schemas
            .insert(
                SchemaDraft {
                    dimension_parameters: vec![DimensionParameterDeclaration {
                        id: DimensionParameterId::new(0),
                        origin: DimensionParameterOrigin::Explicit,
                        lifetime: DimensionLifetime::Turn,
                        lower_bound: DimensionExpr::Constant(1),
                        upper_bound: Some(DimensionExpr::Constant(8)),
                    }]
                    .into_boxed_slice(),
                    body: SchemaBody::Matrix {
                        element: Box::new(SchemaBody::Bool),
                        dimensions: vec![DimensionExpr::Parameter(DimensionParameterId::new(0))]
                            .into_boxed_slice(),
                    },
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let schemas = schemas.finish().unwrap();
        let schema = schemas.resolve(schema).unwrap();
        let original = ValueDraft {
            schema,
            shape_values: vec![3].into_boxed_slice(),
            data: ValueDataDraft::Matrix(vec![ValueDataDraft::Bool(true); 3].into_boxed_slice()),
        }
        .finalize(&SnapshotValidationContext::new(&schemas.table))
        .unwrap();
        let probe = ManagedMemoryBudget::new(u64::MAX);
        let required = original.memory_budget_admission_bytes(&probe).unwrap();
        let budget = ManagedMemoryBudget::new(required);
        let region = ResidentRegion {
            kind: ResidentValueKind::Snapshot,
            offset: 0,
            len: 1,
            shape: ResidentShape {
                rows: 3,
                columns: 1,
            },
        };
        let values = [Some(original)];
        let export = || {
            materialize_resident_value(
                &schemas.table,
                schema,
                values[0].as_ref().unwrap().shape(),
                region,
                ResidentValueRef::Snapshot(&values),
                Some(&budget),
            )
            .unwrap()
        };
        let first = export();
        let second = export();
        assert_eq!(budget.used_bytes(), required);
        assert!(core::ptr::eq(
            first.shape().parameter_values(),
            second.shape().parameter_values()
        ));
        assert!(core::ptr::eq(
            values[0].as_ref().unwrap().shape().parameter_values(),
            first.shape().parameter_values()
        ));
        drop(values);
        drop(first);
        assert_eq!(second.shape().parameter_values(), &[3]);
        assert_eq!(budget.used_bytes(), required);
        drop(second);
        assert_eq!(budget.used_bytes(), 0);
    }

    #[test]
    fn owning_string_export_admits_peak_before_copy_and_retains_final_owner() {
        let schema = SchemaDraft {
            dimension_parameters: Box::new([]),
            body: SchemaBody::String,
        }
        .finalize()
        .unwrap();
        let mut builder = SchemaTableBuilder::new();
        let handle = builder.insert(schema).unwrap();
        let build = builder.finish().unwrap();
        let schema = build.resolve(handle).unwrap();
        let shape = build
            .table
            .get(schema)
            .unwrap()
            .instantiate_shape(Box::new([]))
            .unwrap();
        let strings = ["retained export".repeat(128)];
        let source = ResidentValueRef::String(&strings);
        let probe = ManagedMemoryBudget::new(u64::MAX);
        let reservation = prepare_value_export(Some(&probe), &build.table, &shape, source, false)
            .unwrap()
            .unwrap();
        let peak = reservation.capacity_bytes();
        assert!(peak > strings[0].len() as u64);
        drop(reservation);
        assert_eq!(probe.used_bytes(), 0);

        let too_small = ManagedMemoryBudget::new(peak - 1);
        assert!(matches!(
            prepare_value_export(Some(&too_small), &build.table, &shape, source, false),
            Err(MemoryRuntimeError::BudgetExceeded { .. })
        ));
        assert_eq!(too_small.used_bytes(), 0);
        assert_eq!(strings[0], "retained export".repeat(128));

        let exact = ManagedMemoryBudget::new(peak);
        let mut reservation =
            prepare_value_export(Some(&exact), &build.table, &shape, source, false).unwrap();
        let value = ValueDraft {
            schema,
            shape_values: Box::new([]),
            data: ValueDataDraft::String(strings[0].clone()),
        }
        .finalize(&SnapshotValidationContext::new(&build.table))
        .unwrap();
        let value = retain_export(value, &mut reservation).unwrap();
        drop(reservation);
        let retained = exact.used_bytes();
        assert!(retained >= strings[0].len() as u64);
        assert!(retained <= peak);
        let clone = value.clone();
        drop(value);
        assert_eq!(exact.used_bytes(), retained);
        assert!(matches!(clone.data(), ValueData::String(text) if text.as_ref() == strings[0]));
        drop(clone);
        assert_eq!(exact.used_bytes(), 0);
    }
}
