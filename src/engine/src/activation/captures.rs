use super::ActivationPatternCaptureKindUnsupported;
use crate::{
    AccessMode, AliasPolicy, ChangeDetectionPolicy, DeliveryMode, DimensionExpr, ExecutionTarget,
    ExternalInteraction, FloatWidth, FunctionInstance, FunctionInvocation, InputPortLayout,
    InputPortPolicy, IntegerWidth, MResult, MechError, MechFunction, OperationContractDeclaration,
    OutputConstruction, OutputPortPolicy, PatternBindingSink, PatternMatch, Plan, ReactiveNodeId,
    ResolvedOperationDescriptor, RuntimeFunctionId, SchemaBody, ShapeRule, SpecializedFunction,
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
    let contract = OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                };
                inputs.len()
            ]
            .into_boxed_slice(),
        ),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    };
    let instance = FunctionInstance::new(
        implementation,
        FunctionInvocation::variadic(output, inputs.into_boxed_slice()),
    );
    plan.register_specialized(SpecializedFunction::syntax_directed(
        instance,
        ResolvedOperationDescriptor::from_name("activation/pattern-node", contract)?,
        RuntimeFunctionId::from_name("ActivationPatternNode"),
        ExecutionTarget::DirectRuntime,
    )?)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CardinalitySpec, ExtentSpec, MechFunctionImpl, SchemaField};
    use mech_core::snapshot::{F64Bits, MapEntryDraft, NamedValueDraft, TableColumnDraft};

    fn f64_draft(value: f64) -> ValueDataDraft {
        ValueDataDraft::F64(F64Bits::from_f64(value))
    }

    fn assert_commit_preserves_identity(source: ValueCell) {
        let destination = create_capture_slot_for_schema(&source.closed_schema_body().unwrap())
            .expect("capture schema must have a canonical slot");
        let alias = destination.clone();
        commit_capture_slot(&destination, &source).unwrap();
        assert!(destination.same_cell(&alias));
        assert!(destination.snapshot_eq(&source).unwrap());
    }

    #[test]
    fn canonical_capture_slots_cover_scalar_tuple_record_and_matrix_schemas() {
        #[cfg(feature = "u8")]
        assert_commit_preserves_identity(ValueCell::from_exact(42_u8).unwrap());
        #[cfg(feature = "u16")]
        assert_commit_preserves_identity(ValueCell::from_exact(42_u16).unwrap());
        #[cfg(feature = "u32")]
        assert_commit_preserves_identity(ValueCell::from_exact(42_u32).unwrap());
        #[cfg(feature = "u64")]
        assert_commit_preserves_identity(ValueCell::from_exact(42_u64).unwrap());
        #[cfg(feature = "u128")]
        assert_commit_preserves_identity(ValueCell::from_exact(42_u128).unwrap());
        #[cfg(feature = "i8")]
        assert_commit_preserves_identity(ValueCell::from_exact(-17_i8).unwrap());
        #[cfg(feature = "i16")]
        assert_commit_preserves_identity(ValueCell::from_exact(-17_i16).unwrap());
        #[cfg(feature = "i32")]
        assert_commit_preserves_identity(ValueCell::from_exact(-17_i32).unwrap());
        #[cfg(feature = "i64")]
        assert_commit_preserves_identity(ValueCell::from_exact(-17_i64).unwrap());
        #[cfg(feature = "i128")]
        assert_commit_preserves_identity(ValueCell::from_exact(-17_i128).unwrap());
        #[cfg(feature = "f32")]
        assert_commit_preserves_identity(ValueCell::from_exact(3.25_f32).unwrap());
        #[cfg(feature = "f64")]
        assert_commit_preserves_identity(ValueCell::from_exact(6.5_f64).unwrap());
        #[cfg(feature = "c64")]
        assert_commit_preserves_identity(ValueCell::from_exact(crate::C64::new(3.0, 4.0)).unwrap());
        #[cfg(feature = "r64")]
        assert_commit_preserves_identity(ValueCell::from_exact(crate::R64::new(3, 4)).unwrap());
        #[cfg(feature = "bool")]
        assert_commit_preserves_identity(ValueCell::from_exact(true).unwrap());
        #[cfg(feature = "string")]
        assert_commit_preserves_identity(ValueCell::from_exact("captured".to_string()).unwrap());
        assert_commit_preserves_identity(ValueCell::from_exact(42_usize).unwrap());

        #[cfg(all(feature = "f64", feature = "bool"))]
        assert_commit_preserves_identity(
            ValueCell::from_schema_data(
                SchemaBody::Tuple(
                    vec![SchemaBody::FloatingPoint(FloatWidth::W64), SchemaBody::Bool]
                        .into_boxed_slice(),
                ),
                ValueDataDraft::Tuple(
                    vec![f64_draft(3.0), ValueDataDraft::Bool(true)].into_boxed_slice(),
                ),
            )
            .unwrap(),
        );
        #[cfg(feature = "f64")]
        assert_commit_preserves_identity(
            ValueCell::from_schema_data(
                SchemaBody::Record(
                    vec![SchemaField {
                        name: "value".into(),
                        schema: SchemaBody::FloatingPoint(FloatWidth::W64),
                    }]
                    .into_boxed_slice(),
                ),
                ValueDataDraft::Record(
                    vec![NamedValueDraft {
                        name: "value".into(),
                        value: f64_draft(9.0),
                    }]
                    .into_boxed_slice(),
                ),
            )
            .unwrap(),
        );
        #[cfg(feature = "f64")]
        assert_commit_preserves_identity(
            ValueCell::dynamic_matrix(
                SchemaBody::FloatingPoint(FloatWidth::W64),
                vec![1, 3].into_boxed_slice(),
                vec![f64_draft(1.0), f64_draft(2.0), f64_draft(3.0)].into_boxed_slice(),
            )
            .unwrap(),
        );
    }

    #[cfg(feature = "string")]
    #[test]
    fn canonical_capture_slot_preserves_identity_across_repeated_updates() {
        let destination = ValueCell::from_exact(String::new()).unwrap();
        let alias = destination.clone();
        for value in ["first", "second"] {
            let source = ValueCell::from_exact(value.to_owned()).unwrap();
            commit_capture_slot(&destination, &source).unwrap();
            assert!(destination.same_cell(&alias));
            assert!(destination.snapshot_eq(&source).unwrap());
        }
    }

    #[test]
    fn canonical_capture_slots_support_dynamic_set_map_and_table_extents() {
        let f64_schema = SchemaBody::FloatingPoint(FloatWidth::W64);
        assert_commit_preserves_identity(
            ValueCell::from_schema_data(
                SchemaBody::Set {
                    element: Box::new(f64_schema.clone()),
                    cardinality: CardinalitySpec::Dynamic { upper_bound: None },
                },
                ValueDataDraft::Set(vec![f64_draft(1.0), f64_draft(2.0)].into_boxed_slice()),
            )
            .unwrap(),
        );
        assert_commit_preserves_identity(
            ValueCell::from_schema_data(
                SchemaBody::Map {
                    key: Box::new(SchemaBody::String),
                    value: Box::new(f64_schema.clone()),
                    cardinality: ExtentSpec::Dynamic { upper_bound: None },
                },
                ValueDataDraft::Map(
                    vec![MapEntryDraft {
                        items: vec![ValueDataDraft::String("x".into()), f64_draft(3.0)]
                            .into_boxed_slice(),
                    }]
                    .into_boxed_slice(),
                ),
            )
            .unwrap(),
        );
        assert_commit_preserves_identity(
            ValueCell::from_schema_data(
                SchemaBody::Table {
                    columns: vec![SchemaField {
                        name: "x".into(),
                        schema: f64_schema,
                    }]
                    .into_boxed_slice(),
                    rows: ExtentSpec::Dynamic { upper_bound: None },
                },
                ValueDataDraft::Table(
                    vec![TableColumnDraft {
                        name: "x".into(),
                        values: vec![f64_draft(4.0), f64_draft(5.0)].into_boxed_slice(),
                    }]
                    .into_boxed_slice(),
                ),
            )
            .unwrap(),
        );
    }

    #[test]
    fn capture_batch_preflight_is_atomic() {
        let first = ActivationPatternCapture {
            id: 1,
            name: "first".into(),
            schema: SchemaBody::FloatingPoint(FloatWidth::W64),
            proposed: ValueCell::from_exact(10.0_f64).unwrap(),
            committed: ValueCell::from_exact(1.0_f64).unwrap(),
        };
        let second = ActivationPatternCapture {
            id: 2,
            name: "second".into(),
            schema: SchemaBody::FloatingPoint(FloatWidth::W64),
            proposed: ValueCell::from_exact(true).unwrap(),
            committed: ValueCell::from_exact(2.0_f64).unwrap(),
        };
        let error = commit_proposed_captures(&[first.clone(), second]).unwrap_err();

        assert_eq!(error.kind_name(), "ActivationPatternCaptureKindUnsupported");
        let snapshot = first.committed.snapshot().unwrap();
        let mech_core::ValueData::F64(value) = snapshot.data() else {
            panic!("expected retained f64 capture")
        };
        assert_eq!(value.to_f64(), 1.0);
    }

    #[test]
    fn selected_and_unselected_capture_gates_commit_atomically() {
        let valid = ActivationPatternCapture {
            id: 1,
            name: "valid".into(),
            schema: SchemaBody::FloatingPoint(FloatWidth::W64),
            proposed: ValueCell::from_exact(10.0_f64).unwrap(),
            committed: ValueCell::from_exact(1.0_f64).unwrap(),
        };
        let invalid = ActivationPatternCapture {
            id: 2,
            name: "invalid".into(),
            schema: SchemaBody::FloatingPoint(FloatWidth::W64),
            proposed: ValueCell::from_exact(true).unwrap(),
            committed: ValueCell::from_exact(2.0_f64).unwrap(),
        };
        let selected = selected_arm_state(0);
        let pulse = generation().0;
        let selected_gate = super::super::registers::Gate {
            arm: 0,
            selected: selected.clone(),
            captures: vec![valid.clone(), invalid],
            out: pulse.clone(),
        };
        let error = selected_gate.solve_reactive().unwrap_err();
        assert_eq!(error.kind_name(), "ActivationPatternCaptureKindUnsupported");
        assert!(
            valid
                .committed
                .snapshot_eq(&ValueCell::from_exact(1.0_f64).unwrap())
                .unwrap()
        );
        assert!(
            pulse
                .snapshot_eq(&ValueCell::from_exact(1_usize).unwrap())
                .unwrap()
        );

        let retained = ActivationPatternCapture {
            id: 3,
            name: "retained".into(),
            schema: SchemaBody::String,
            proposed: ValueCell::from_exact("proposed".to_owned()).unwrap(),
            committed: ValueCell::from_exact("committed".to_owned()).unwrap(),
        };
        write_selected_arm(&selected, 1).unwrap();
        let unselected_gate = super::super::registers::Gate {
            arm: 0,
            selected,
            captures: vec![retained.clone()],
            out: pulse.clone(),
        };
        assert_eq!(
            unselected_gate.solve_reactive().unwrap(),
            crate::ReactiveSolveStatus::Unchanged
        );
        assert!(
            retained
                .committed
                .snapshot_eq(&ValueCell::from_exact("committed".to_owned()).unwrap())
                .unwrap()
        );
        assert!(
            pulse
                .snapshot_eq(&ValueCell::from_exact(1_usize).unwrap())
                .unwrap()
        );
    }
}
