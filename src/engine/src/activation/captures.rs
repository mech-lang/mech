use super::ActivationPatternCaptureKindUnsupported;
use crate::{
    DimensionExpr, FloatWidth, FunctionInstance, FunctionInvocation, IntegerWidth, MResult,
    MechError, MechFunction, PatternBindingSink, PatternMatch, Plan, ReactiveNodeId, SchemaBody,
    ValueCell, ValueCellSnapshotFailure, ValueDataDraft,
};
use mech_core::snapshot::{
    EnumDraft, MapEntryDraft, NamedValueDraft, OptionDraft, TableColumnDraft,
};

pub(super) fn generation() -> (ValueCell, ValueCell) {
    let generation = ValueCell::from_exact(1_usize)
        .expect("the canonical activation generation schema is valid");
    (generation.clone(), generation)
}

pub(super) fn bool_state(value: bool) -> ValueCell {
    ValueCell::from_schema_data(SchemaBody::Bool, ValueDataDraft::Bool(value))
        .expect("the canonical activation bool schema is valid")
}

pub(super) fn index_state(value: usize) -> ValueCell {
    ValueCell::from_exact(value).expect("the canonical activation index schema is valid")
}

pub(super) fn read_bool(cell: &ValueCell) -> MResult<bool> {
    match cell.snapshot()?.data() {
        mech_core::snapshot::ValueData::Bool(value) => Ok(*value),
        _ => Err(MechError::new(
            ActivationPatternCaptureKindUnsupported,
            None,
        )),
    }
}

pub(super) fn write_bool(cell: &ValueCell, value: bool) -> MResult<()> {
    cell.replace(
        &ValueCell::from_schema_data(SchemaBody::Bool, ValueDataDraft::Bool(value))?.snapshot()?,
    )
}

pub(super) fn read_index(cell: &ValueCell) -> MResult<usize> {
    match cell.snapshot()?.data() {
        mech_core::snapshot::ValueData::Index(value) => usize::try_from(*value)
            .map_err(|_| MechError::new(ActivationPatternCaptureKindUnsupported, None)),
        _ => Err(MechError::new(
            ActivationPatternCaptureKindUnsupported,
            None,
        )),
    }
}

pub(super) fn write_index(cell: &ValueCell, value: usize) -> MResult<()> {
    cell.replace(&ValueCell::from_exact(value)?.snapshot()?)
}

pub(super) fn selected_arm_state(arm: usize) -> ValueCell {
    index_state(encode_selected_arm(arm))
}

pub(super) fn read_selected_arm(cell: &ValueCell) -> MResult<usize> {
    let value = read_index(cell)?;
    if value == usize::MAX {
        Ok(usize::MAX)
    } else {
        value
            .checked_sub(1)
            .ok_or_else(|| MechError::new(ActivationPatternCaptureKindUnsupported, None))
    }
}

pub(super) fn write_selected_arm(cell: &ValueCell, arm: usize) -> MResult<()> {
    write_index(cell, encode_selected_arm(arm))
}

fn encode_selected_arm(arm: usize) -> usize {
    if arm == usize::MAX {
        usize::MAX
    } else {
        arm.saturating_add(1)
    }
}

pub(super) fn increment(cell: &ValueCell) -> MResult<()> {
    write_index(cell, read_index(cell)?.saturating_add(1))
}

pub(super) fn register_node(
    plan: &Plan,
    implementation: Box<dyn MechFunction>,
    output: ValueCell,
    inputs: Vec<ValueCell>,
) -> MResult<ReactiveNodeId> {
    plan.register_instance(FunctionInstance::new(
        implementation,
        FunctionInvocation::variadic(output, inputs.into_boxed_slice()),
    ))
}

#[derive(Clone)]
pub(super) struct ActivationPatternCapture {
    pub(super) id: u64,
    pub(super) name: String,
    pub(super) schema: SchemaBody,
    pub(super) proposed: ValueCell,
    pub(super) committed: ValueCell,
}

fn default_draft(schema: &SchemaBody) -> MResult<ValueDataDraft> {
    Ok(match schema {
        SchemaBody::Dynamic => ValueDataDraft::Dynamic(None),
        SchemaBody::UnsignedInteger(IntegerWidth::W8) => ValueDataDraft::U8(0),
        SchemaBody::UnsignedInteger(IntegerWidth::W16) => ValueDataDraft::U16(0),
        SchemaBody::UnsignedInteger(IntegerWidth::W32) => ValueDataDraft::U32(0),
        SchemaBody::UnsignedInteger(IntegerWidth::W64) => ValueDataDraft::U64(0),
        SchemaBody::UnsignedInteger(IntegerWidth::W128) => ValueDataDraft::U128(0),
        SchemaBody::SignedInteger(IntegerWidth::W8) => ValueDataDraft::I8(0),
        SchemaBody::SignedInteger(IntegerWidth::W16) => ValueDataDraft::I16(0),
        SchemaBody::SignedInteger(IntegerWidth::W32) => ValueDataDraft::I32(0),
        SchemaBody::SignedInteger(IntegerWidth::W64) => ValueDataDraft::I64(0),
        SchemaBody::SignedInteger(IntegerWidth::W128) => ValueDataDraft::I128(0),
        SchemaBody::FloatingPoint(FloatWidth::W32) => {
            ValueDataDraft::F32(mech_core::snapshot::F32Bits::from_f32(0.0))
        }
        SchemaBody::FloatingPoint(FloatWidth::W64) => {
            ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_f64(0.0))
        }
        SchemaBody::Complex(FloatWidth::W32) => {
            ValueDataDraft::Complex32(mech_core::snapshot::Complex32Bits::new(
                mech_core::snapshot::F32Bits::from_f32(0.0),
                mech_core::snapshot::F32Bits::from_f32(0.0),
            ))
        }
        SchemaBody::Complex(FloatWidth::W64) => {
            ValueDataDraft::Complex64(mech_core::snapshot::Complex64Bits::new(
                mech_core::snapshot::F64Bits::from_f64(0.0),
                mech_core::snapshot::F64Bits::from_f64(0.0),
            ))
        }
        SchemaBody::Rational64 => ValueDataDraft::Rational64 {
            numerator: 0,
            denominator: 1,
        },
        SchemaBody::Bool => ValueDataDraft::Bool(false),
        SchemaBody::String => ValueDataDraft::String(String::new()),
        SchemaBody::Id => ValueDataDraft::Id(0),
        SchemaBody::Index => ValueDataDraft::Index(1),
        SchemaBody::Atom(_) => ValueDataDraft::Atom,
        SchemaBody::Option(_) => ValueDataDraft::Option(OptionDraft {
            present: false,
            value: None,
        }),
        SchemaBody::Tuple(elements) => ValueDataDraft::Tuple(
            elements
                .iter()
                .map(default_draft)
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
        ),
        SchemaBody::Record(fields) => ValueDataDraft::Record(
            fields
                .iter()
                .map(|field| {
                    Ok(NamedValueDraft {
                        name: field.name.clone(),
                        value: default_draft(&field.schema)?,
                    })
                })
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
        ),
        SchemaBody::Matrix {
            element,
            dimensions,
        } => {
            let count = dimensions
                .iter()
                .map(|dimension| match dimension {
                    DimensionExpr::Constant(value) => *value as usize,
                    _ => 0,
                })
                .product::<usize>();
            ValueDataDraft::Matrix(
                (0..count)
                    .map(|_| default_draft(element))
                    .collect::<MResult<Vec<_>>>()?
                    .into_boxed_slice(),
            )
        }
        SchemaBody::Table { columns, .. } => ValueDataDraft::Table(
            columns
                .iter()
                .map(|column| TableColumnDraft {
                    name: column.name.clone(),
                    values: Box::new([]),
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
        SchemaBody::Set { .. } => ValueDataDraft::Set(Box::new([])),
        SchemaBody::Map { .. } => {
            ValueDataDraft::Map(Vec::<MapEntryDraft>::new().into_boxed_slice())
        }
        SchemaBody::Enum { variants, .. } => {
            let Some(variant) = variants.first() else {
                return Err(MechError::new(
                    ActivationPatternCaptureKindUnsupported,
                    None,
                ));
            };
            ValueDataDraft::Enum(EnumDraft {
                ordinal: 0,
                payload: variant
                    .payload
                    .as_ref()
                    .map(default_draft)
                    .transpose()?
                    .map(Box::new),
            })
        }
        SchemaBody::ReifiedType => {
            return Err(MechError::new(
                ActivationPatternCaptureKindUnsupported,
                None,
            ));
        }
    })
}

pub(super) fn create_capture_slot_for_schema(schema: &SchemaBody) -> MResult<ValueCell> {
    if let SchemaBody::Matrix {
        element,
        dimensions,
    } = &schema
    {
        let concrete = dimensions
            .iter()
            .map(|dimension| match dimension {
                DimensionExpr::Constant(value) => *value,
                _ => 0,
            })
            .collect::<Vec<_>>();
        let ValueDataDraft::Matrix(values) = default_draft(&schema)? else {
            unreachable!()
        };
        return ValueCell::dynamic_matrix((**element).clone(), concrete.into_boxed_slice(), values);
    }
    if let SchemaBody::Table { columns, .. } = schema {
        return ValueCell::empty_dynamic_table(columns.clone());
    }
    if let SchemaBody::Set { element, .. } = schema {
        return ValueCell::empty_dynamic_set((**element).clone());
    }
    if let SchemaBody::Map { key, value, .. } = schema {
        return ValueCell::empty_dynamic_map((**key).clone(), (**value).clone());
    }
    ValueCell::from_schema_data(schema.clone(), default_draft(schema)?)
}

fn preflight_capture_slot(destination: &ValueCell, source: &ValueCell) -> MResult<SchemaBody> {
    let source_schema = source.closed_schema_body()?;
    let destination_schema = destination.closed_schema_body()?;
    let compatible = match (&destination_schema, &source_schema) {
        (
            SchemaBody::Matrix {
                element: destination_element,
                ..
            },
            SchemaBody::Matrix {
                element: source_element,
                ..
            },
        ) => destination_element == source_element,
        (
            SchemaBody::Table {
                columns: destination_columns,
                ..
            },
            SchemaBody::Table {
                columns: source_columns,
                ..
            },
        ) => destination_columns == source_columns,
        (
            SchemaBody::Set {
                element: destination_element,
                ..
            },
            SchemaBody::Set {
                element: source_element,
                ..
            },
        ) => destination_element == source_element,
        (
            SchemaBody::Map {
                key: destination_key,
                value: destination_value,
                ..
            },
            SchemaBody::Map {
                key: source_key,
                value: source_value,
                ..
            },
        ) => destination_key == source_key && destination_value == source_value,
        _ => destination_schema == source_schema,
    };
    if !compatible {
        return Err(MechError::new(
            ActivationPatternCaptureKindUnsupported,
            None,
        ));
    }
    destination.preflight_replace()?;
    Ok(source_schema)
}

pub(super) fn commit_capture_slot(destination: &ValueCell, source: &ValueCell) -> MResult<()> {
    let source_schema = preflight_capture_slot(destination, source)?;
    if let SchemaBody::Matrix { dimensions, .. } = &source_schema {
        let draft = source.snapshot()?.canonical_data_draft().map_err(|error| {
            MechError::new(ValueCellSnapshotFailure { error }, None).with_compiler_loc()
        })?;
        let ValueDataDraft::Matrix(values) = draft else {
            unreachable!()
        };
        let dimensions = dimensions
            .iter()
            .map(|dimension| match dimension {
                DimensionExpr::Constant(value) => *value,
                _ => unreachable!("closed matrix schema has concrete dimensions"),
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let replacement = destination.rebuild_matrix_drafts(dimensions, values)?;
        return destination.replace(&replacement);
    }
    if matches!(
        source_schema,
        SchemaBody::Table { .. } | SchemaBody::Set { .. } | SchemaBody::Map { .. }
    ) {
        let draft = source.snapshot()?.canonical_data_draft().map_err(|error| {
            MechError::new(ValueCellSnapshotFailure { error }, None).with_compiler_loc()
        })?;
        let replacement = destination.rebuild_data_draft(draft)?;
        return destination.replace(&replacement);
    }
    destination.replace(&source.snapshot()?)
}

pub(super) struct ReactiveBindingSink<'a> {
    pub(super) captures: &'a [ActivationPatternCapture],
}

impl PatternBindingSink for ReactiveBindingSink<'_> {
    fn commit(&mut self, pattern_match: &PatternMatch) -> MResult<()> {
        if !pattern_match.matched {
            return Ok(());
        }
        for binding in &pattern_match.bindings {
            let capture = self
                .captures
                .get(binding.index)
                .ok_or_else(|| MechError::new(ActivationPatternCaptureKindUnsupported, None))?;
            if capture.id != binding.id {
                return Err(MechError::new(
                    ActivationPatternCaptureKindUnsupported,
                    None,
                ));
            }
            preflight_capture_slot(&capture.proposed, &binding.value)?;
        }
        for binding in &pattern_match.bindings {
            commit_capture_slot(&self.captures[binding.index].proposed, &binding.value)?;
        }
        Ok(())
    }
}

pub(super) fn commit_proposed_captures(captures: &[ActivationPatternCapture]) -> MResult<()> {
    for capture in captures {
        preflight_capture_slot(&capture.committed, &capture.proposed)?;
    }
    for capture in captures {
        commit_capture_slot(&capture.committed, &capture.proposed)?;
    }
    Ok(())
}
