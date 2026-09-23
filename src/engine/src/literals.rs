use crate::*;
#[cfg(all(test, feature = "convert"))]
use mech_core::snapshot::{Complex64Bits, F32Bits, F64Bits, OptionDraft};
#[cfg(any(feature = "kind_annotation", feature = "convert"))]
use mech_core::snapshot::{ReifiedKind, ReifiedType, ReifiedTypeDraft};
#[cfg(any(feature = "kind_annotation", feature = "convert"))]
use std::collections::BTreeMap;

// Literals
// ----------------------------------------------------------------------------

pub fn literal(ltrl: &Literal, p: &InterpreterExecution<'_>) -> MResult<SpecializationInput> {
    let input = match &ltrl {
        Literal::Empty(_) => Ok(SpecializationInput::Absent),
        #[cfg(feature = "bool")]
        Literal::Boolean(bln) => boolean(bln).map(SpecializationInput::Cell),
        Literal::Number(num) => number(num, p).map(SpecializationInput::Cell),
        #[cfg(feature = "string")]
        Literal::String(strng) => string(strng).map(SpecializationInput::Cell),
        #[cfg(feature = "atom")]
        Literal::Atom(atm) => atom(atm, p).map(SpecializationInput::Cell),
        #[cfg(feature = "kind_annotation")]
        Literal::Kind(knd) => kind_value(knd, p).map(SpecializationInput::Cell),
        #[cfg(feature = "convert")]
        Literal::TypedLiteral((ltrl, kind)) => {
            typed_literal(ltrl, kind, p).map(SpecializationInput::Cell)
        }
        #[cfg(not(all(
            feature = "bool",
            feature = "string",
            feature = "atom",
            feature = "kind_annotation",
            feature = "convert"
        )))]
        _ => Err(MechError::new(FeatureNotEnabledError, None).with_compiler_loc()),
    }?;
    match input {
        SpecializationInput::Cell(cell) => cell
            .import_owned_in(p.memory_domain())
            .map(SpecializationInput::Cell),
        SpecializationInput::Absent => Ok(SpecializationInput::Absent),
        SpecializationInput::MatrixAllSelection => Ok(SpecializationInput::MatrixAllSelection),
    }
}

#[cfg(feature = "kind_annotation")]
pub fn kind_value(
    knd: &mech_core::nodes::Kind,
    p: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    let mut named = SourceNamedKinds::default();
    let kind = canonical_kind_annotation(knd, p, &mut named)?;
    let reified = ReifiedKind::from_closed_kind(&kind, &[], &named).map_err(|error| {
        MechError::new(ValueCellSnapshotFailure { error }, None).with_compiler_loc()
    })?;
    ValueCell::from_schema_data(
        SchemaBody::ReifiedType,
        ValueDataDraft::Type(ReifiedTypeDraft::CanonicalKind(
            reified.canonical_bytes().to_vec().into_boxed_slice(),
        )),
    )
}

#[cfg(feature = "kind_annotation")]
#[derive(Default)]
struct SourceNamedKinds(BTreeMap<KindId, CanonicalNominalPath>);

#[cfg(feature = "kind_annotation")]
impl NamedKindPathResolver for SourceNamedKinds {
    fn canonical_path(&self, id: KindId) -> Option<&CanonicalNominalPath> {
        self.0.get(&id)
    }
}

#[cfg(feature = "kind_annotation")]
fn source_nominal_path(name: &str) -> MResult<CanonicalNominalPath> {
    Ok(CanonicalNominalPath::new(
        name.split('/')
            .filter(|segment| !segment.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>(),
    )?)
}

#[cfg(feature = "kind_annotation")]
fn canonical_kind_annotation(
    knd: &mech_core::nodes::Kind,
    p: &InterpreterExecution<'_>,
    named: &mut SourceNamedKinds,
) -> MResult<KindExpr> {
    Ok(match knd {
        mech_core::nodes::Kind::Kind(inner) => {
            KindExpr::TypeOf(Box::new(canonical_kind_annotation(inner, p, named)?))
        }
        mech_core::nodes::Kind::Any => KindExpr::Wildcard,
        mech_core::nodes::Kind::Atom(identifier) => {
            let path = source_nominal_path(&identifier.to_string())?;
            KindExpr::Atom(NominalKey::from_path(NominalKind::Atom, &path))
        }
        mech_core::nodes::Kind::Empty => KindExpr::Hole,
        mech_core::nodes::Kind::Record(fields) => KindExpr::Record(
            fields
                .iter()
                .map(|(name, kind)| {
                    Ok(KindField {
                        name: name.to_string(),
                        kind: canonical_kind_annotation(kind, p, named)?,
                    })
                })
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
        ),
        mech_core::nodes::Kind::Tuple(elements) => KindExpr::Tuple(
            elements
                .iter()
                .map(|element| canonical_kind_annotation(element, p, named))
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
        ),
        mech_core::nodes::Kind::Map(key, value) => KindExpr::Map {
            key: Box::new(canonical_kind_annotation(key, p, named)?),
            value: Box::new(canonical_kind_annotation(value, p, named)?),
            cardinality: DimensionExpr::Hole,
        },
        mech_core::nodes::Kind::Scalar(identifier) => {
            let scalar_id = identifier.hash();
            if let Ok((id, path)) = builtin_scalar_named_kind(scalar_id) {
                named.0.insert(id, path);
                KindExpr::Named(id)
            } else if p.state.borrow().enums.contains_key(&scalar_id) {
                let path = source_nominal_path(&identifier.to_string())?;
                KindExpr::Enum(NominalKey::from_path(NominalKind::Enum, &path))
            } else {
                return Err(SemanticModelError::BuiltinScalarKindUnresolved { scalar_id }.into());
            }
        }
        mech_core::nodes::Kind::Matrix((element, dimensions)) => KindExpr::Matrix {
            element: Box::new(canonical_kind_annotation(element, p, named)?),
            dimensions: dimensions
                .iter()
                .map(|dimension| {
                    literal_usize(dimension, p).map(|value| {
                        value.map_or(DimensionExpr::Hole, |value| {
                            DimensionExpr::Constant(value as u64)
                        })
                    })
                })
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
        },
        mech_core::nodes::Kind::Option(element) => {
            KindExpr::Option(Box::new(canonical_kind_annotation(element, p, named)?))
        }
        mech_core::nodes::Kind::Table((columns, rows)) => KindExpr::Table {
            columns: columns
                .iter()
                .map(|(name, kind)| {
                    Ok(KindField {
                        name: name.to_string(),
                        kind: canonical_kind_annotation(kind, p, named)?,
                    })
                })
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
            rows: literal_usize(rows, p)?.map_or(DimensionExpr::Hole, |value| {
                DimensionExpr::Constant(value as u64)
            }),
        },
        mech_core::nodes::Kind::Set(element, cardinality) => KindExpr::Set {
            element: Box::new(canonical_kind_annotation(element, p, named)?),
            cardinality: cardinality
                .as_ref()
                .map(|value| literal_usize(value, p))
                .transpose()?
                .flatten()
                .map_or(DimensionExpr::Hole, |value| {
                    DimensionExpr::Constant(value as u64)
                }),
        },
    })
}

#[cfg(feature = "kind_annotation")]
pub(crate) fn literal_usize(
    literal_node: &Literal,
    p: &InterpreterExecution<'_>,
) -> MResult<Option<usize>> {
    let input = literal(literal_node, p)?;
    let SpecializationInput::Cell(cell) = input else {
        return Ok(None);
    };
    let snapshot = cell.snapshot()?;
    let value = match snapshot.data() {
        ValueData::Index(value) => usize::try_from(*value).ok(),
        ValueData::U8(value) => Some(*value as usize),
        ValueData::U16(value) => Some(*value as usize),
        ValueData::U32(value) => usize::try_from(*value).ok(),
        ValueData::U64(value) => usize::try_from(*value).ok(),
        ValueData::U128(value) => usize::try_from(*value).ok(),
        ValueData::I8(value) => usize::try_from(*value).ok(),
        ValueData::I16(value) => usize::try_from(*value).ok(),
        ValueData::I32(value) => usize::try_from(*value).ok(),
        ValueData::I64(value) => usize::try_from(*value).ok(),
        ValueData::I128(value) => usize::try_from(*value).ok(),
        ValueData::F32(value) => {
            let value = value.to_f32();
            (value >= 0.0 && value.fract() == 0.0).then(|| value as usize)
        }
        ValueData::F64(value) => {
            let value = value.to_f64();
            (value >= 0.0 && value.fract() == 0.0).then(|| value as usize)
        }
        _ => None,
    };
    value
        .map(Some)
        .ok_or_else(|| MechError::new(ExpectedNumericForKindSizeError, None).with_compiler_loc())
}

#[cfg(feature = "convert")]
pub fn typed_literal(
    ltrl: &Literal,
    knd_attn: &KindAnnotation,
    p: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    let value = literal(ltrl, p)?.cell().cloned()?;
    let target = crate::structures::schema_body_from_kind(&knd_attn.kind, p)?;
    convert_literal_cell(value, &target).map_err(|error| error.with_tokens(knd_attn.tokens()))
}

#[cfg(feature = "convert")]
pub(crate) fn convert_literal_cell(value: ValueCell, target: &SchemaBody) -> MResult<ValueCell> {
    let source_type = value.resolved_type()?;
    let semantic_target =
        materialize_declared_conversion_semantic_shape(source_type.kind(), target);
    let target = materialize_declared_conversion_shape(&value.closed_schema_body()?, target);
    let target_type =
        ResolvedType::from_schema_body(&semantic_target, source_type.dimension_parameters())
            .map_err(MechError::from)?;
    let plan = plan_explicit_cast(&source_type, &target_type).map_err(|error| {
        MechError::from(error.with_origin(TypeConstraintOrigin::new("convert/kind", None)))
    })?;
    execute_conversion_plan(&value, &target, &plan)
}

/// A source annotation such as `[string]` declares an element conversion while
/// intentionally leaving the matrix extents open. Close those extents from the
/// source value before constructing the conversion plan so the plan remains the
/// sole execution authority and no runtime factory probing is required.
#[cfg(feature = "convert")]
fn materialize_declared_conversion_shape(source: &SchemaBody, target: &SchemaBody) -> SchemaBody {
    match (source, target) {
        (
            SchemaBody::Matrix {
                element: source_element,
                dimensions: source_dimensions,
            },
            SchemaBody::Matrix {
                element: target_element,
                dimensions: target_dimensions,
            },
        ) => SchemaBody::Matrix {
            element: Box::new(materialize_declared_conversion_shape(
                source_element,
                target_element,
            )),
            dimensions: if target_dimensions.is_empty() {
                source_dimensions.clone()
            } else {
                target_dimensions.clone()
            },
        },
        (SchemaBody::Option(source), SchemaBody::Option(target)) => SchemaBody::Option(Box::new(
            materialize_declared_conversion_shape(source, target),
        )),
        (source, SchemaBody::Option(target)) => SchemaBody::Option(Box::new(
            materialize_declared_conversion_shape(source, target),
        )),
        (SchemaBody::Tuple(source), SchemaBody::Tuple(target)) if source.len() == target.len() => {
            SchemaBody::Tuple(
                source
                    .iter()
                    .zip(target.iter())
                    .map(|(source, target)| materialize_declared_conversion_shape(source, target))
                    .collect(),
            )
        }
        (SchemaBody::Record(source), SchemaBody::Record(target))
            if source.len() == target.len()
                && source
                    .iter()
                    .zip(target.iter())
                    .all(|(a, b)| a.name == b.name) =>
        {
            SchemaBody::Record(
                source
                    .iter()
                    .zip(target.iter())
                    .map(|(source, target)| SchemaField {
                        name: target.name.clone(),
                        schema: materialize_declared_conversion_shape(
                            &source.schema,
                            &target.schema,
                        ),
                    })
                    .collect(),
            )
        }
        (
            SchemaBody::Set {
                element: source,
                cardinality: source_cardinality,
            },
            SchemaBody::Set {
                element: target,
                cardinality,
            },
        ) => SchemaBody::Set {
            element: Box::new(materialize_declared_conversion_shape(source, target)),
            cardinality: if matches!(cardinality, CardinalitySpec::Dynamic { upper_bound: None }) {
                source_cardinality.clone()
            } else {
                cardinality.clone()
            },
        },
        (
            SchemaBody::Map {
                key: source_key,
                value: source_value,
                cardinality: source_cardinality,
            },
            SchemaBody::Map {
                key,
                value,
                cardinality,
            },
        ) => SchemaBody::Map {
            key: Box::new(materialize_declared_conversion_shape(source_key, key)),
            value: Box::new(materialize_declared_conversion_shape(source_value, value)),
            cardinality: if matches!(cardinality, CardinalitySpec::Dynamic { upper_bound: None }) {
                source_cardinality.clone()
            } else {
                cardinality.clone()
            },
        },
        (
            SchemaBody::Table {
                columns: source,
                rows: source_rows,
            },
            SchemaBody::Table {
                columns: target,
                rows,
            },
        ) if source.len() == target.len()
            && source
                .iter()
                .zip(target.iter())
                .all(|(a, b)| a.name == b.name) =>
        {
            SchemaBody::Table {
                columns: source
                    .iter()
                    .zip(target.iter())
                    .map(|(source, target)| SchemaField {
                        name: target.name.clone(),
                        schema: materialize_declared_conversion_shape(
                            &source.schema,
                            &target.schema,
                        ),
                    })
                    .collect(),
                rows: if matches!(rows, CardinalitySpec::Dynamic { upper_bound: None }) {
                    source_rows.clone()
                } else {
                    rows.clone()
                },
            }
        }
        _ => target.clone(),
    }
}

#[cfg(feature = "convert")]
fn materialize_declared_conversion_semantic_shape(
    source: &KindExpr,
    target: &SchemaBody,
) -> SchemaBody {
    match (source, target) {
        (
            KindExpr::Matrix {
                element: source_element,
                dimensions: source_dimensions,
            },
            SchemaBody::Matrix {
                element: target_element,
                dimensions: target_dimensions,
            },
        ) => SchemaBody::Matrix {
            element: Box::new(materialize_declared_conversion_semantic_shape(
                source_element,
                target_element,
            )),
            dimensions: if target_dimensions.is_empty() {
                source_dimensions.clone()
            } else {
                target_dimensions.clone()
            },
        },
        (KindExpr::Option(source), SchemaBody::Option(target)) => SchemaBody::Option(Box::new(
            materialize_declared_conversion_semantic_shape(source, target),
        )),
        (source, SchemaBody::Option(target)) => SchemaBody::Option(Box::new(
            materialize_declared_conversion_semantic_shape(source, target),
        )),
        (KindExpr::Tuple(source), SchemaBody::Tuple(target)) if source.len() == target.len() => {
            SchemaBody::Tuple(
                source
                    .iter()
                    .zip(target.iter())
                    .map(|(source, target)| {
                        materialize_declared_conversion_semantic_shape(source, target)
                    })
                    .collect(),
            )
        }
        (KindExpr::Record(source), SchemaBody::Record(target))
            if source.len() == target.len()
                && source
                    .iter()
                    .zip(target.iter())
                    .all(|(a, b)| a.name == b.name) =>
        {
            SchemaBody::Record(
                source
                    .iter()
                    .zip(target.iter())
                    .map(|(source, target)| SchemaField {
                        name: target.name.clone(),
                        schema: materialize_declared_conversion_semantic_shape(
                            &source.kind,
                            &target.schema,
                        ),
                    })
                    .collect(),
            )
        }
        (
            KindExpr::Set {
                element: source,
                cardinality: source_cardinality,
            },
            SchemaBody::Set {
                element: target,
                cardinality,
            },
        ) => SchemaBody::Set {
            element: Box::new(materialize_declared_conversion_semantic_shape(
                source, target,
            )),
            cardinality: if matches!(cardinality, CardinalitySpec::Dynamic { upper_bound: None }) {
                CardinalitySpec::Exact(source_cardinality.clone())
            } else {
                cardinality.clone()
            },
        },
        (
            KindExpr::Map {
                key: source_key,
                value: source_value,
                cardinality: source_cardinality,
            },
            SchemaBody::Map {
                key,
                value,
                cardinality,
            },
        ) => SchemaBody::Map {
            key: Box::new(materialize_declared_conversion_semantic_shape(
                source_key, key,
            )),
            value: Box::new(materialize_declared_conversion_semantic_shape(
                source_value,
                value,
            )),
            cardinality: if matches!(cardinality, CardinalitySpec::Dynamic { upper_bound: None }) {
                CardinalitySpec::Exact(source_cardinality.clone())
            } else {
                cardinality.clone()
            },
        },
        (
            KindExpr::Table {
                columns: source,
                rows: source_rows,
            },
            SchemaBody::Table {
                columns: target,
                rows,
            },
        ) if source.len() == target.len()
            && source
                .iter()
                .zip(target.iter())
                .all(|(a, b)| a.name == b.name) =>
        {
            SchemaBody::Table {
                columns: source
                    .iter()
                    .zip(target.iter())
                    .map(|(source, target)| SchemaField {
                        name: target.name.clone(),
                        schema: materialize_declared_conversion_semantic_shape(
                            &source.kind,
                            &target.schema,
                        ),
                    })
                    .collect(),
                rows: if matches!(rows, CardinalitySpec::Dynamic { upper_bound: None }) {
                    CardinalitySpec::Exact(source_rows.clone())
                } else {
                    rows.clone()
                },
            }
        }
        _ => target.clone(),
    }
}

#[cfg(feature = "convert")]
fn execute_conversion_plan(
    value: &ValueCell,
    target: &SchemaBody,
    plan: &ConversionPlan,
) -> MResult<ValueCell> {
    let live_type = value.resolved_type()?;
    if !exact_type_equal(&live_type, &plan.source) {
        return Err(conversion_execution_error(
            ConversionExecutionError::ConversionPlanSourceMismatch,
        ));
    }
    let snapshot = value.snapshot()?;
    if matches!(plan.step, ConversionStep::Identity) {
        // The snapshot retains nested dynamic payload IDs and their complete
        // schema table. Reconstructing from a data draft loses that context.
        return ValueCell::from_runtime_snapshot(snapshot);
    }
    let draft = snapshot.canonical_data_draft().map_err(|error| {
        MechError::new(ValueCellSnapshotFailure { error }, None).with_compiler_loc()
    })?;
    let converted =
        execute_conversion_draft(draft, &plan.step).map_err(conversion_execution_error)?;
    let current_extents = match target {
        SchemaBody::Matrix { .. }
        | SchemaBody::Set {
            cardinality: CardinalitySpec::Exact(_),
            ..
        }
        | SchemaBody::Map {
            cardinality: CardinalitySpec::Exact(_),
            ..
        }
        | SchemaBody::Table {
            rows: CardinalitySpec::Exact(_),
            ..
        } => value.current_top_level_extents()?,
        _ => Box::new([]),
    };
    let mut descriptor = materialize_resolved_output(
        &plan.target,
        &ResolvedOutputSchemaRule::Declared(target.clone()),
        &[],
        current_extents,
    )
    .map_err(MechError::from)?;
    if plan.source.dimension_parameters() == plan.target.dimension_parameters()
        && descriptor.schema().dimension_parameters().len()
            == snapshot.shape().parameter_values().len()
    {
        let shape = descriptor
            .schema()
            .instantiate_shape(
                snapshot
                    .shape()
                    .parameter_values()
                    .to_vec()
                    .into_boxed_slice(),
            )
            .map_err(MechError::from)?;
        descriptor = mech_core::ResolvedValueDescriptor::new(
            plan.target.clone(),
            descriptor.schema().clone(),
            shape,
        )
        .map_err(MechError::from)?;
    }
    ValueCell::from_resolved_descriptor_data(&descriptor, converted)
}

#[cfg(feature = "convert")]
fn execute_conversion_draft_from_snapshot(
    source: &ValueCell,
    snapshot: &mech_core::Value,
    plan: &ConversionPlan,
) -> MResult<ValueDataDraft> {
    let live_type = source.resolved_type()?;
    if !exact_type_equal(&live_type, &plan.source) {
        return Err(conversion_execution_error(
            ConversionExecutionError::ConversionPlanSourceMismatch,
        ));
    }
    let draft = snapshot.canonical_data_draft().map_err(|error| {
        MechError::new(ValueCellSnapshotFailure { error }, None).with_compiler_loc()
    })?;
    execute_conversion_draft(draft, &plan.step).map_err(conversion_execution_error)
}

#[cfg(feature = "convert")]
fn conversion_target_schema(
    source: &SchemaBody,
    step: &ConversionStep,
) -> Result<SchemaBody, ConversionExecutionError> {
    Ok(match step {
        ConversionStep::Identity => source.clone(),
        ConversionStep::Scalar(ScalarConversion::Builtin { target, .. }) => target.schema_body(),
        ConversionStep::MatrixElements(element_plan) => {
            let SchemaBody::Matrix {
                element,
                dimensions,
            } = source
            else {
                return Err(ConversionExecutionError::ConversionPlanSourceMismatch);
            };
            SchemaBody::Matrix {
                element: Box::new(conversion_target_schema(element, &element_plan.step)?),
                dimensions: dimensions.clone(),
            }
        }
        ConversionStep::OptionPresent(payload_plan) => SchemaBody::Option(Box::new(
            conversion_target_schema(source, &payload_plan.step)?,
        )),
        ConversionStep::OptionPayload(payload_plan) => {
            let SchemaBody::Option(payload) = source else {
                return Err(ConversionExecutionError::ConversionPlanSourceMismatch);
            };
            SchemaBody::Option(Box::new(conversion_target_schema(
                payload,
                &payload_plan.step,
            )?))
        }
    })
}

#[cfg(feature = "convert")]
fn conversion_execution_error(error: ConversionExecutionError) -> MechError {
    MechError::new(error, None).with_compiler_loc()
}

#[cfg(feature = "convert")]
fn prospective_conversion_output_footprint(
    output: &ValueCell,
    source: &ValueCell,
    plan: &ConversionPlan,
) -> MResult<Option<CurrentMemoryFootprint>> {
    if !output.requires_canonical_output_builder()? {
        return Ok(None);
    }
    let mut footprint = output.prospective_aggregate_memory_footprint([(source, 1)])?;
    if let Some(maximum_string_bytes) = conversion_string_payload_bound(&plan.step) {
        let source_elements = source.current_memory_footprint()?.logical_elements;
        let string_payload = source_elements
            .checked_mul(maximum_string_bytes)
            .ok_or_else(|| {
                MechError::new(
                    mech_core::MemoryPlanError::ArithmeticOverflow {
                        field: "converted String payload bound",
                    },
                    None,
                )
                .with_compiler_loc()
            })?;
        footprint.payload_bytes = footprint
            .payload_bytes
            .checked_add(string_payload)
            .ok_or_else(|| {
                MechError::new(
                    mech_core::MemoryPlanError::ArithmeticOverflow {
                        field: "converted String retained bytes",
                    },
                    None,
                )
                .with_compiler_loc()
            })?;
        footprint.encoded_bytes = footprint
            .encoded_bytes
            .checked_add(string_payload)
            .ok_or_else(|| {
                MechError::new(
                    mech_core::MemoryPlanError::ArithmeticOverflow {
                        field: "converted String encoded bytes",
                    },
                    None,
                )
                .with_compiler_loc()
            })?;
    }
    Ok(Some(footprint))
}

#[cfg(feature = "convert")]
fn conversion_string_payload_bound(step: &ConversionStep) -> Option<u64> {
    let source = match step {
        ConversionStep::Scalar(ScalarConversion::Builtin {
            source,
            target: BuiltinScalarKind::String,
            ..
        }) => *source,
        ConversionStep::MatrixElements(inner)
        | ConversionStep::OptionPayload(inner)
        | ConversionStep::OptionPresent(inner) => {
            return conversion_string_payload_bound(&inner.step);
        }
        ConversionStep::Identity | ConversionStep::Scalar(_) => return None,
    };
    // These bounds cover the complete `Display` spelling emitted by the
    // selected conversion plan, including fixed decimal spellings of the
    // smallest subnormal floats and both components of complex values.
    Some(match source {
        BuiltinScalarKind::U8 => 3,
        BuiltinScalarKind::U16 => 5,
        BuiltinScalarKind::U32 => 10,
        BuiltinScalarKind::U64 => 20,
        BuiltinScalarKind::U128 => 39,
        BuiltinScalarKind::I8 => 4,
        BuiltinScalarKind::I16 => 6,
        BuiltinScalarKind::I32 => 11,
        BuiltinScalarKind::I64 => 20,
        BuiltinScalarKind::I128 => 40,
        BuiltinScalarKind::F32 => 48,
        BuiltinScalarKind::F64 => 328,
        BuiltinScalarKind::C32 => 98,
        BuiltinScalarKind::C64 => 658,
        BuiltinScalarKind::R64 => 41,
        BuiltinScalarKind::Bool => 5,
        BuiltinScalarKind::String => return None,
    })
}

#[cfg(feature = "convert")]
fn stage_conversion_output(
    frame: &mut mech_core::KernelMemoryFrame<'_>,
    source: &ValueCell,
    output: &ValueCell,
    plan: &ConversionPlan,
) -> MResult<()> {
    if let Some(footprint) = prospective_conversion_output_footprint(output, source, plan)? {
        if matches!(plan.step, ConversionStep::Identity) {
            return frame.with_admitted_canonical_output(
                output,
                footprint,
                |frame, construction| {
                    let snapshot =
                        frame.snapshot_input_cell_with_construction(source, 0, construction)?;
                    let next =
                        construction.try_rebind_snapshot_candidate_with(output, || Ok(snapshot))?;
                    Ok(((), next))
                },
            );
        }
        frame.with_admitted_canonical_output(output, footprint, |frame, construction| {
            let next = construction.try_build_canonical_candidate_with(|construction| {
                let snapshot =
                    frame.snapshot_input_cell_with_construction(source, 0, construction)?;
                let converted = execute_conversion_draft_from_snapshot(source, &snapshot, plan)?;
                if plan.source.dimension_parameters() == plan.target.dimension_parameters()
                    && output.shape().parameter_values().len()
                        == snapshot.shape().parameter_values().len()
                {
                    construction.try_rebuild_data_draft_with_shape(
                        output,
                        converted,
                        snapshot.shape(),
                    )
                } else {
                    construction.try_rebuild_data_draft(output, converted)
                }
            })?;
            Ok(((), next))
        })
    } else {
        let live_type = source.resolved_type()?;
        if !exact_type_equal(&live_type, &plan.source) {
            return Err(conversion_execution_error(
                ConversionExecutionError::ConversionPlanSourceMismatch,
            ));
        }
        frame.execute_fixed_conversion_plan(source, output, plan)
    }
}

#[cfg(feature = "convert")]
#[derive(Debug)]
struct PlannedTypeConversion {
    source: ValueCell,
    output: ValueCell,
    plan: ConversionPlan,
    reified_constraints: Option<ReifiedTargetConstraints>,
}

#[cfg(feature = "convert")]
#[derive(Debug)]
struct ReifiedTargetConstraints {
    target: SchemaBody,
    declarations: Box<[DimensionParameterDeclaration]>,
}

#[cfg(feature = "convert")]
impl ReifiedTargetConstraints {
    fn validate(&self, source: &ValueCell) -> MResult<()> {
        let mut bindings = vec![None; self.declarations.len()];
        bind_reified_target_dimensions(
            &source.closed_schema_body()?,
            &self.target,
            &self.declarations,
            &mut bindings,
        )?;
        validate_reified_parameter_bindings(&self.declarations, &bindings)
    }
}

#[cfg(feature = "convert")]
fn planned_type_conversion_instance(
    source: ValueCell,
    output: ValueCell,
    plan: ConversionPlan,
    reified_constraints: Option<ReifiedTargetConstraints>,
) -> (Box<dyn MechFunction>, FunctionInvocation) {
    (
        Box::new(PlannedTypeConversion {
            source: source.clone(),
            output: output.clone(),
            plan,
            reified_constraints,
        }),
        FunctionInvocation::unary(output, source),
    )
}

#[cfg(feature = "convert")]
fn planned_type_conversion_specialized(
    source: ValueCell,
    output: ValueCell,
    plan: ConversionPlan,
) -> MResult<SpecializedFunction> {
    let instance = planned_type_conversion_instance(source, output, plan, None);
    SpecializedFunction::syntax_directed(
        instance,
        ResolvedOperationDescriptor::from_name(
            "convert/kind",
            PURE_TYPE_CONVERSION_CONTRACT.clone(),
        )?,
        RuntimeFunctionId::from_name("convert/kind"),
        ExecutionTarget::DirectRuntime,
        mech_core::ImplementationMemoryClass::CanonicalFinalize,
    )
}

/// Bytecode/native implementation of the canonical `convert/kind`
/// instruction. The destination schema is the reified target carried by the
/// artifact, so runtime binding reconstructs and validates the same checked
/// conversion plan used during source execution.
#[cfg(feature = "convert")]
#[derive(Debug)]
pub struct RuntimeKindConversion {
    source: FunctionValueInput,
    output: FunctionValueOutput,
    plan: ConversionPlan,
}

#[cfg(feature = "convert")]
fn runtime_kind_conversion_plan(output: &ValueCell, source: &ValueCell) -> MResult<ConversionPlan> {
    let source_type = source.resolved_type()?;
    let target_type = output.resolved_type()?;
    let plan = plan_explicit_cast(&source_type, &target_type).map_err(|error| {
        MechError::from(error.with_origin(TypeConstraintOrigin::new("convert/kind", None)))
    })?;
    let target = output.closed_schema_body()?;
    let expected = conversion_target_schema(&source.closed_schema_body()?, &plan.step)
        .map_err(conversion_execution_error)?;
    if expected != target {
        return Err(conversion_execution_error(
            ConversionExecutionError::ConversionShapeMismatch,
        ));
    }
    Ok(plan)
}

#[cfg(feature = "convert")]
impl MechFunctionFactory for RuntimeKindConversion {
    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::CanonicalFinalize
    }

    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::unary(
        FunctionValueRepresentation::AnyValue,
        FunctionValueRepresentation::AnyValue,
    );

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (output, source) = invocation.expect_unary()?;
        let output = output.value();
        let source = source.value();
        let plan = runtime_kind_conversion_plan(output.cell(), source.cell())?;
        Ok(Box::new(Self {
            source,
            output,
            plan,
        }))
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_TYPE_CONVERSION_CONTRACT)
    }
}

#[cfg(feature = "convert")]
impl MechFunctionImpl for RuntimeKindConversion {
    fn planned_output_footprints(&self) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
        Ok(prospective_conversion_output_footprint(
            self.output.cell(),
            self.source.cell(),
            &self.plan,
        )?
        .map(|footprint| vec![footprint].into_boxed_slice()))
    }

    fn solve_managed(
        &self,
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        _services: &mut dyn mech_core::MechExecutionServices,
    ) -> MResult<mech_core::ReactiveSolveStatus> {
        stage_conversion_output(frame, self.source.cell(), self.output.cell(), &self.plan)?;
        Ok(mech_core::ReactiveSolveStatus::Changed)
    }

    fn semantic_operation_name(&self) -> Option<&str> {
        Some("convert/kind")
    }

    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_TYPE_CONVERSION_CONTRACT)
    }

    fn to_string(&self) -> String {
        "RuntimeKindConversion".to_owned()
    }
}

#[cfg(all(feature = "convert", feature = "semantic-compiler"))]
impl MechFunctionCompiler for RuntimeKindConversion {
    fn compiler_owned_value_cells(&self) -> Vec<ValueCell> {
        vec![self.source.cell().clone(), self.output.cell().clone()]
    }

    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let destination = self.output.compile_register(context)?;
        let source = self.source.compile_register(context)?;
        let function = context.function_id("convert/kind")?;
        context.emit_unop(function, destination, source);
        Ok(destination)
    }
}

#[cfg(feature = "convert")]
fn validate_runtime_kind_conversion(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
    let [source] = inputs else {
        return Err(function_shape_contract_violation(
            "type_conversion",
            format!("expected one semantic source input, found {}", inputs.len()),
        ));
    };
    runtime_kind_conversion_plan(output, source).map(|_| ())
}

mech_core::declare_native_runtime_factory! {
    cfg: all(feature = "convert", feature = "semantic-compiler"),
    registration: register_runtime_kind_conversion,
    installer: install_runtime_kind_conversion,
    name: "convert/kind",
    factory_type: RuntimeKindConversion,
    contract: RuntimeFunctionContract::canonical_custom(
        "type_conversion",
        RuntimeOutputAliasPolicy::DisallowInputAlias,
        validate_runtime_kind_conversion,
    ),
    compiler_family: mech_core::RuntimeFamilyId::from_name("convert/kind"),
    package: "mech-engine", crate_name: "mech_engine",
    installer_path: "mech_engine::__mech_native::install_runtime_kind_conversion",
    extra_cargo_features: ["convert", "semantic-compiler"],
}

#[cfg(feature = "convert")]
pub(crate) static PURE_TYPE_CONVERSION_CONTRACT: std::sync::LazyLock<OperationContractDeclaration> =
    std::sync::LazyLock::new(|| {
        mech_core::maintained_operation_contract("convert/kind", 1, false)
            .expect("maintained operation contract")
    });

#[cfg(feature = "convert")]
fn schema_body_from_reified_kind(
    value: &ReifiedKind,
    context: &SpecializationContext<'_>,
) -> MResult<(SchemaBody, Box<[DimensionParameterDeclaration]>)> {
    let (kind, dimensions, named) = value.decoded_closed_kind().map_err(|error| {
        MechError::new(ValueCellSnapshotFailure { error }, None).with_compiler_loc()
    })?;

    fn schema(
        kind: &KindExpr,
        dimensions: &[DimensionParameterDeclaration],
        named: &BTreeMap<KindId, CanonicalNominalPath>,
        context: &SpecializationContext<'_>,
    ) -> MResult<SchemaBody> {
        let aggregate_error = || {
            MechError::new(
                CanonicalAggregateTypeInferenceFailure {
                    context: "reified conversion target",
                },
                None,
            )
            .with_compiler_loc()
        };
        let cardinality = |dimension: &DimensionExpr| match dimension {
            DimensionExpr::Hole => CardinalitySpec::Dynamic { upper_bound: None },
            dimension => CardinalitySpec::Exact(dimension.clone()),
        };
        Ok(match kind {
            KindExpr::Named(id) => {
                let name = named
                    .get(id)
                    .and_then(|path| path.segments().last())
                    .ok_or_else(aggregate_error)?;
                BuiltinScalarKind::ALL
                    .into_iter()
                    .find(|kind| kind.canonical_name() == name)
                    .map(BuiltinScalarKind::schema_body)
                    .ok_or_else(aggregate_error)?
            }
            KindExpr::Id => SchemaBody::Id,
            KindExpr::Index => SchemaBody::Index,
            KindExpr::Atom(key) => SchemaBody::Atom(*key),
            KindExpr::Enum(key) => {
                let variants = context
                    .schemas()
                    .entries()
                    .find_map(|entry| match entry.schema().body() {
                        SchemaBody::Enum {
                            key: resolved,
                            variants,
                        } if resolved == key => Some(variants.clone()),
                        _ => None,
                    })
                    .ok_or_else(aggregate_error)?;
                SchemaBody::Enum {
                    key: *key,
                    variants,
                }
            }
            KindExpr::Matrix {
                element,
                dimensions: extents,
            } => SchemaBody::Matrix {
                element: Box::new(schema(element, dimensions, named, context)?),
                dimensions: extents.clone(),
            },
            KindExpr::Option(element) => {
                SchemaBody::Option(Box::new(schema(element, dimensions, named, context)?))
            }
            KindExpr::Tuple(elements) => SchemaBody::Tuple(
                elements
                    .iter()
                    .map(|element| schema(element, dimensions, named, context))
                    .collect::<MResult<Vec<_>>>()?
                    .into_boxed_slice(),
            ),
            KindExpr::Record(fields) => SchemaBody::Record(
                fields
                    .iter()
                    .map(|field| {
                        Ok(SchemaField {
                            name: field.name.clone(),
                            schema: schema(&field.kind, dimensions, named, context)?,
                        })
                    })
                    .collect::<MResult<Vec<_>>>()?
                    .into_boxed_slice(),
            ),
            KindExpr::Table { columns, rows } => SchemaBody::Table {
                columns: columns
                    .iter()
                    .map(|column| {
                        Ok(SchemaField {
                            name: column.name.clone(),
                            schema: schema(&column.kind, dimensions, named, context)?,
                        })
                    })
                    .collect::<MResult<Vec<_>>>()?
                    .into_boxed_slice(),
                rows: cardinality(rows),
            },
            KindExpr::Set {
                element,
                cardinality: extent,
            } => SchemaBody::Set {
                element: Box::new(schema(element, dimensions, named, context)?),
                cardinality: cardinality(extent),
            },
            KindExpr::Map {
                key,
                value,
                cardinality: extent,
            } => SchemaBody::Map {
                key: Box::new(schema(key, dimensions, named, context)?),
                value: Box::new(schema(value, dimensions, named, context)?),
                cardinality: cardinality(extent),
            },
            KindExpr::TypeOf(_) => SchemaBody::ReifiedType,
            KindExpr::Wildcard
            | KindExpr::Never
            | KindExpr::Hole
            | KindExpr::Parameter(_)
            | KindExpr::Reference(_) => return Err(aggregate_error()),
        })
    }

    Ok((schema(&kind, &dimensions, &named, context)?, dimensions))
}

#[cfg(feature = "convert")]
fn invalid_reified_conversion_target(context: &'static str) -> MechError {
    MechError::new(CanonicalAggregateTypeInferenceFailure { context }, None).with_compiler_loc()
}

#[cfg(feature = "convert")]
fn reified_dimension_value(dimension: &DimensionExpr) -> Option<u64> {
    match dimension {
        DimensionExpr::Constant(value) => Some(*value),
        DimensionExpr::Add(children) => children.iter().try_fold(0_u64, |sum, child| {
            sum.checked_add(reified_dimension_value(child)?)
        }),
        DimensionExpr::Multiply(children) => children.iter().try_fold(1_u64, |product, child| {
            product.checked_mul(reified_dimension_value(child)?)
        }),
        DimensionExpr::Min(children) => children
            .iter()
            .map(reified_dimension_value)
            .collect::<Option<Vec<_>>>()?
            .into_iter()
            .min(),
        DimensionExpr::Max(children) => children
            .iter()
            .map(reified_dimension_value)
            .collect::<Option<Vec<_>>>()?
            .into_iter()
            .max(),
        DimensionExpr::Hole | DimensionExpr::Parameter(_) => None,
    }
}

#[cfg(feature = "convert")]
fn reified_parameter_bounds(
    declaration: &DimensionParameterDeclaration,
    bindings: &[Option<DimensionExpr>],
) -> MResult<(u64, u64)> {
    let lower = substitute_reified_dimension(&declaration.lower_bound, bindings)?;
    let lower = reified_dimension_value(&lower).ok_or_else(|| {
        invalid_reified_conversion_target("unresolved target dimension lower bound")
    })?;
    let upper = declaration
        .upper_bound
        .as_ref()
        .map(|upper| {
            let upper = substitute_reified_dimension(upper, bindings)?;
            reified_dimension_value(&upper).ok_or_else(|| {
                invalid_reified_conversion_target("unresolved target dimension upper bound")
            })
        })
        .transpose()?
        .unwrap_or(u64::MAX);
    if lower > upper {
        return Err(invalid_reified_conversion_target(
            "target dimension bounds are inconsistent",
        ));
    }
    Ok((lower, upper))
}

#[cfg(feature = "convert")]
fn unbound_reified_parameter(
    dimension: &DimensionExpr,
    bindings: &[Option<DimensionExpr>],
    selected: &mut Option<DimensionParameterId>,
) -> MResult<()> {
    match dimension {
        DimensionExpr::Parameter(id) if bindings.get(id.get() as usize).is_none() => {
            return Err(invalid_reified_conversion_target(
                "unknown target dimension parameter",
            ));
        }
        DimensionExpr::Parameter(id) if bindings[id.get() as usize].is_none() => {
            if selected.is_some_and(|previous| previous != *id) {
                return Err(invalid_reified_conversion_target(
                    "target dimension has multiple unbound parameters",
                ));
            }
            *selected = Some(*id);
        }
        DimensionExpr::Add(children)
        | DimensionExpr::Multiply(children)
        | DimensionExpr::Min(children)
        | DimensionExpr::Max(children) => {
            for child in children {
                unbound_reified_parameter(child, bindings, selected)?;
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(feature = "convert")]
fn bind_reified_dimension(
    source: &DimensionExpr,
    target: &DimensionExpr,
    declarations: &[DimensionParameterDeclaration],
    bindings: &mut [Option<DimensionExpr>],
) -> MResult<()> {
    let mut selected = None;
    unbound_reified_parameter(target, bindings, &mut selected)?;
    let Some(id) = selected else {
        if let Ok(resolved) = substitute_reified_dimension(target, bindings)
            && let (Some(actual), Some(expected)) = (
                reified_dimension_value(source),
                reified_dimension_value(&resolved),
            )
            && actual != expected
        {
            return Err(invalid_reified_conversion_target(
                "target dimension differs from source extent",
            ));
        }
        return Ok(());
    };
    let index = id.get() as usize;
    let declaration = declarations
        .get(index)
        .filter(|declaration| declaration.id == id)
        .ok_or_else(|| invalid_reified_conversion_target("unknown target dimension parameter"))?;
    // Bounds can refer to another parameter that appears later in the body.
    // Use them to narrow a compound search when available, then validate all
    // witnesses together after traversal.
    let (lower, upper) = reified_parameter_bounds(declaration, bindings).unwrap_or((0, u64::MAX));
    let witness = if target == &DimensionExpr::Parameter(id) {
        source.clone()
    } else {
        let extent = reified_dimension_value(source).ok_or_else(|| {
            invalid_reified_conversion_target("compound target requires a concrete source extent")
        })?;
        let mut trial = bindings.to_vec();
        let mut evaluate = |candidate| {
            trial[index] = Some(DimensionExpr::Constant(candidate));
            substitute_reified_dimension(target, &trial)
                .ok()
                .and_then(|dimension| reified_dimension_value(&dimension))
        };
        let (mut low, mut high) = (lower, upper);
        while low < high {
            let middle = low + (high - low) / 2;
            if evaluate(middle).is_some_and(|value| value < extent) {
                low = middle + 1;
            } else {
                high = middle;
            }
        }
        if evaluate(low) != Some(extent) || (low < upper && evaluate(low + 1) == Some(extent)) {
            return Err(invalid_reified_conversion_target(
                "compound target dimension has no unique source witness",
            ));
        }
        DimensionExpr::Constant(low)
    };
    bindings[index] = Some(witness);
    Ok(())
}

#[cfg(feature = "convert")]
fn validate_reified_parameter_bindings(
    declarations: &[DimensionParameterDeclaration],
    bindings: &[Option<DimensionExpr>],
) -> MResult<()> {
    for declaration in declarations {
        let Some(witness) = bindings
            .get(declaration.id.get() as usize)
            .and_then(Option::as_ref)
        else {
            continue;
        };
        let (lower, upper) = reified_parameter_bounds(declaration, bindings)?;
        if let Some(value) = reified_dimension_value(witness) {
            if value < lower || value > upper {
                return Err(invalid_reified_conversion_target(
                    "source extent is outside target dimension bounds",
                ));
            }
        } else if lower != 0 || upper != u64::MAX {
            return Err(invalid_reified_conversion_target(
                "bounded target dimension requires a concrete source extent",
            ));
        }
    }
    Ok(())
}

#[cfg(feature = "convert")]
fn inherit_reified_dynamic_cardinality(
    source: &SchemaBody,
    target: &mut SchemaBody,
    declarations: &[DimensionParameterDeclaration],
) -> MResult<()> {
    fn expression_uses(dimension: &DimensionExpr, id: DimensionParameterId) -> usize {
        match dimension {
            DimensionExpr::Parameter(found) if *found == id => 1,
            DimensionExpr::Add(children)
            | DimensionExpr::Multiply(children)
            | DimensionExpr::Min(children)
            | DimensionExpr::Max(children) => children
                .iter()
                .map(|child| expression_uses(child, id))
                .sum(),
            _ => 0,
        }
    }
    fn cardinality_uses(cardinality: &CardinalitySpec, id: DimensionParameterId) -> usize {
        match cardinality {
            CardinalitySpec::Exact(dimension) => expression_uses(dimension, id),
            CardinalitySpec::Dynamic { .. } => 0,
        }
    }
    fn body_uses(body: &SchemaBody, id: DimensionParameterId) -> usize {
        match body {
            SchemaBody::Matrix {
                element,
                dimensions,
            } => {
                dimensions
                    .iter()
                    .map(|dimension| expression_uses(dimension, id))
                    .sum::<usize>()
                    + body_uses(element, id)
            }
            SchemaBody::Set {
                element,
                cardinality,
            } => cardinality_uses(cardinality, id) + body_uses(element, id),
            SchemaBody::Map {
                key,
                value,
                cardinality,
            } => cardinality_uses(cardinality, id) + body_uses(key, id) + body_uses(value, id),
            SchemaBody::Table { columns, rows } => {
                cardinality_uses(rows, id)
                    + columns
                        .iter()
                        .map(|column| body_uses(&column.schema, id))
                        .sum::<usize>()
            }
            SchemaBody::Option(payload) => body_uses(payload, id),
            SchemaBody::Tuple(elements) => {
                elements.iter().map(|element| body_uses(element, id)).sum()
            }
            SchemaBody::Record(fields) => fields
                .iter()
                .map(|field| body_uses(&field.schema, id))
                .sum(),
            SchemaBody::Enum { variants, .. } => variants
                .iter()
                .filter_map(|variant| variant.payload.as_ref())
                .map(|payload| body_uses(payload, id))
                .sum(),
            _ => 0,
        }
    }
    let use_counts = declarations
        .iter()
        .map(|declaration| body_uses(target, declaration.id))
        .collect::<Vec<_>>();
    inherit_reified_dynamic_cardinality_inner(source, target, declarations, &use_counts)
}

#[cfg(feature = "convert")]
fn inherit_reified_dynamic_cardinality_inner(
    source: &SchemaBody,
    target: &mut SchemaBody,
    declarations: &[DimensionParameterDeclaration],
    use_counts: &[usize],
) -> MResult<()> {
    fn cardinality(
        source: &CardinalitySpec,
        target: &mut CardinalitySpec,
        declarations: &[DimensionParameterDeclaration],
        use_counts: &[usize],
    ) -> MResult<()> {
        if let CardinalitySpec::Dynamic { .. } = source
            && let CardinalitySpec::Exact(dimension) = target
        {
            let DimensionExpr::Parameter(id) = dimension else {
                return Err(invalid_reified_conversion_target(
                    "dynamic source cannot bind an exact target cardinality",
                ));
            };
            let declaration = declarations
                .get(id.get() as usize)
                .filter(|declaration| declaration.id == *id)
                .ok_or_else(|| {
                    invalid_reified_conversion_target("unknown target dimension parameter")
                })?;
            if declaration.lower_bound != DimensionExpr::Constant(0)
                || declaration.upper_bound.is_some()
            {
                return Err(invalid_reified_conversion_target(
                    "dynamic source cannot establish bounded target cardinality",
                ));
            }
            if use_counts.get(id.get() as usize) != Some(&1) {
                return Err(invalid_reified_conversion_target(
                    "dynamic source cannot satisfy a shared target cardinality parameter",
                ));
            }
            *target = source.clone();
        }
        Ok(())
    }
    match (source, target) {
        (
            SchemaBody::Matrix {
                element: source, ..
            },
            SchemaBody::Matrix {
                element: target, ..
            },
        )
        | (SchemaBody::Option(source), SchemaBody::Option(target)) => {
            inherit_reified_dynamic_cardinality_inner(source, target, declarations, use_counts)?;
        }
        (source, SchemaBody::Option(target)) => {
            inherit_reified_dynamic_cardinality_inner(source, target, declarations, use_counts)?;
        }
        (SchemaBody::Tuple(source), SchemaBody::Tuple(target)) if source.len() == target.len() => {
            for (source, target) in source.iter().zip(target.iter_mut()) {
                inherit_reified_dynamic_cardinality_inner(
                    source,
                    target,
                    declarations,
                    use_counts,
                )?;
            }
        }
        (SchemaBody::Record(source), SchemaBody::Record(target))
            if source.len() == target.len()
                && source
                    .iter()
                    .zip(target.iter())
                    .all(|(source, target)| source.name == target.name) =>
        {
            for (source, target) in source.iter().zip(target.iter_mut()) {
                inherit_reified_dynamic_cardinality_inner(
                    &source.schema,
                    &mut target.schema,
                    declarations,
                    use_counts,
                )?;
            }
        }
        (
            SchemaBody::Set {
                element: source_element,
                cardinality: source_cardinality,
            },
            SchemaBody::Set {
                element: target_element,
                cardinality: target_cardinality,
            },
        ) => {
            cardinality(
                source_cardinality,
                target_cardinality,
                declarations,
                use_counts,
            )?;
            inherit_reified_dynamic_cardinality_inner(
                source_element,
                target_element,
                declarations,
                use_counts,
            )?;
        }
        (
            SchemaBody::Map {
                key: source_key,
                value: source_value,
                cardinality: source_cardinality,
            },
            SchemaBody::Map {
                key: target_key,
                value: target_value,
                cardinality: target_cardinality,
            },
        ) => {
            cardinality(
                source_cardinality,
                target_cardinality,
                declarations,
                use_counts,
            )?;
            inherit_reified_dynamic_cardinality_inner(
                source_key,
                target_key,
                declarations,
                use_counts,
            )?;
            inherit_reified_dynamic_cardinality_inner(
                source_value,
                target_value,
                declarations,
                use_counts,
            )?;
        }
        (
            SchemaBody::Table {
                columns: source,
                rows: source_rows,
            },
            SchemaBody::Table {
                columns: target,
                rows: target_rows,
            },
        ) if source.len() == target.len()
            && source
                .iter()
                .zip(target.iter())
                .all(|(source, target)| source.name == target.name) =>
        {
            cardinality(source_rows, target_rows, declarations, use_counts)?;
            for (source, target) in source.iter().zip(target.iter_mut()) {
                inherit_reified_dynamic_cardinality_inner(
                    &source.schema,
                    &mut target.schema,
                    declarations,
                    use_counts,
                )?;
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(feature = "convert")]
fn has_dynamic_cardinality(body: &SchemaBody) -> bool {
    match body {
        SchemaBody::Set {
            element,
            cardinality,
        } => {
            matches!(cardinality, CardinalitySpec::Dynamic { .. })
                || has_dynamic_cardinality(element)
        }
        SchemaBody::Map {
            key,
            value,
            cardinality,
        } => {
            matches!(cardinality, CardinalitySpec::Dynamic { .. })
                || has_dynamic_cardinality(key)
                || has_dynamic_cardinality(value)
        }
        SchemaBody::Table { columns, rows } => {
            matches!(rows, CardinalitySpec::Dynamic { .. })
                || columns
                    .iter()
                    .any(|column| has_dynamic_cardinality(&column.schema))
        }
        SchemaBody::Matrix { element, .. } | SchemaBody::Option(element) => {
            has_dynamic_cardinality(element)
        }
        SchemaBody::Tuple(elements) => elements.iter().any(has_dynamic_cardinality),
        SchemaBody::Record(fields) => fields
            .iter()
            .any(|field| has_dynamic_cardinality(&field.schema)),
        _ => false,
    }
}

#[cfg(feature = "convert")]
fn has_semantic_dimension_parameter(body: &SchemaBody) -> bool {
    fn expression_has_parameter(expression: &DimensionExpr) -> bool {
        match expression {
            DimensionExpr::Parameter(_) => true,
            DimensionExpr::Add(children)
            | DimensionExpr::Multiply(children)
            | DimensionExpr::Min(children)
            | DimensionExpr::Max(children) => children.iter().any(expression_has_parameter),
            DimensionExpr::Constant(_) | DimensionExpr::Hole => false,
        }
    }
    fn cardinality_has_parameter(cardinality: &CardinalitySpec) -> bool {
        match cardinality {
            CardinalitySpec::Exact(expression) => expression_has_parameter(expression),
            CardinalitySpec::Dynamic { upper_bound } => {
                upper_bound.as_ref().is_some_and(expression_has_parameter)
            }
        }
    }
    match body {
        SchemaBody::Matrix {
            element,
            dimensions,
        } => {
            dimensions.iter().any(expression_has_parameter)
                || has_semantic_dimension_parameter(element)
        }
        SchemaBody::Set {
            element,
            cardinality,
        } => cardinality_has_parameter(cardinality) || has_semantic_dimension_parameter(element),
        SchemaBody::Map {
            key,
            value,
            cardinality,
        } => {
            cardinality_has_parameter(cardinality)
                || has_semantic_dimension_parameter(key)
                || has_semantic_dimension_parameter(value)
        }
        SchemaBody::Table { columns, rows } => {
            cardinality_has_parameter(rows)
                || columns
                    .iter()
                    .any(|column| has_semantic_dimension_parameter(&column.schema))
        }
        SchemaBody::Option(element) => has_semantic_dimension_parameter(element),
        SchemaBody::Tuple(elements) => elements.iter().any(has_semantic_dimension_parameter),
        SchemaBody::Record(fields) => fields
            .iter()
            .any(|field| has_semantic_dimension_parameter(&field.schema)),
        SchemaBody::Enum { variants, .. } => variants
            .iter()
            .filter_map(|variant| variant.payload.as_ref())
            .any(has_semantic_dimension_parameter),
        _ => false,
    }
}

#[cfg(feature = "convert")]
fn bind_reified_target_dimensions(
    source: &SchemaBody,
    target: &SchemaBody,
    declarations: &[DimensionParameterDeclaration],
    bindings: &mut [Option<DimensionExpr>],
) -> MResult<()> {
    match (source, target) {
        (
            SchemaBody::Matrix {
                element: source_element,
                dimensions: source_dimensions,
            },
            SchemaBody::Matrix {
                element: target_element,
                dimensions: target_dimensions,
            },
        ) => {
            if source_dimensions.len() != target_dimensions.len() {
                return Err(invalid_reified_conversion_target(
                    "reified matrix target rank differs from source rank",
                ));
            }
            for (source, target) in source_dimensions.iter().zip(target_dimensions.iter()) {
                bind_reified_dimension(source, target, declarations, bindings)?;
            }
            bind_reified_target_dimensions(source_element, target_element, declarations, bindings)?;
        }
        (SchemaBody::Option(source), SchemaBody::Option(target)) => {
            bind_reified_target_dimensions(source, target, declarations, bindings)?;
        }
        (source, SchemaBody::Option(target)) => {
            bind_reified_target_dimensions(source, target, declarations, bindings)?;
        }
        (SchemaBody::Tuple(source), SchemaBody::Tuple(target)) if source.len() == target.len() => {
            for (source, target) in source.iter().zip(target.iter()) {
                bind_reified_target_dimensions(source, target, declarations, bindings)?;
            }
        }
        (SchemaBody::Record(source), SchemaBody::Record(target))
            if source.len() == target.len()
                && source
                    .iter()
                    .zip(target.iter())
                    .all(|(source, target)| source.name == target.name) =>
        {
            for (source, target) in source.iter().zip(target.iter()) {
                bind_reified_target_dimensions(
                    &source.schema,
                    &target.schema,
                    declarations,
                    bindings,
                )?;
            }
        }
        (
            SchemaBody::Set {
                element: source_element,
                cardinality: source_cardinality,
            },
            SchemaBody::Set {
                element: target_element,
                cardinality: target_cardinality,
            },
        ) => {
            if let CardinalitySpec::Exact(target_cardinality) = target_cardinality {
                let CardinalitySpec::Exact(source_cardinality) = source_cardinality else {
                    return Err(invalid_reified_conversion_target(
                        "source cardinality has no exact extent to bind",
                    ));
                };
                bind_reified_dimension(
                    source_cardinality,
                    target_cardinality,
                    declarations,
                    bindings,
                )?;
            }
            bind_reified_target_dimensions(source_element, target_element, declarations, bindings)?;
        }
        (
            SchemaBody::Map {
                key: source_key,
                value: source_value,
                cardinality: source_cardinality,
            },
            SchemaBody::Map {
                key: target_key,
                value: target_value,
                cardinality: target_cardinality,
            },
        ) => {
            if let CardinalitySpec::Exact(target_cardinality) = target_cardinality {
                let CardinalitySpec::Exact(source_cardinality) = source_cardinality else {
                    return Err(invalid_reified_conversion_target(
                        "source cardinality has no exact extent to bind",
                    ));
                };
                bind_reified_dimension(
                    source_cardinality,
                    target_cardinality,
                    declarations,
                    bindings,
                )?;
            }
            bind_reified_target_dimensions(source_key, target_key, declarations, bindings)?;
            bind_reified_target_dimensions(source_value, target_value, declarations, bindings)?;
        }
        (
            SchemaBody::Table {
                columns: source,
                rows: source_rows,
            },
            SchemaBody::Table {
                columns: target,
                rows: target_rows,
            },
        ) if source.len() == target.len()
            && source
                .iter()
                .zip(target.iter())
                .all(|(source, target)| source.name == target.name) =>
        {
            if let CardinalitySpec::Exact(target_rows) = target_rows {
                let CardinalitySpec::Exact(source_rows) = source_rows else {
                    return Err(invalid_reified_conversion_target(
                        "source table row count has no exact extent to bind",
                    ));
                };
                bind_reified_dimension(source_rows, target_rows, declarations, bindings)?;
            }
            for (source, target) in source.iter().zip(target.iter()) {
                bind_reified_target_dimensions(
                    &source.schema,
                    &target.schema,
                    declarations,
                    bindings,
                )?;
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(feature = "convert")]
fn substitute_reified_dimension(
    dimension: &DimensionExpr,
    bindings: &[Option<DimensionExpr>],
) -> MResult<DimensionExpr> {
    Ok(match dimension {
        DimensionExpr::Hole => DimensionExpr::Hole,
        DimensionExpr::Constant(value) => DimensionExpr::Constant(*value),
        DimensionExpr::Parameter(id) => bindings
            .get(id.get() as usize)
            .and_then(Option::as_ref)
            .cloned()
            .ok_or_else(|| {
                invalid_reified_conversion_target("unbound target dimension parameter")
            })?,
        DimensionExpr::Add(children) => DimensionExpr::Add(
            children
                .iter()
                .map(|child| substitute_reified_dimension(child, bindings))
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
        ),
        DimensionExpr::Multiply(children) => DimensionExpr::Multiply(
            children
                .iter()
                .map(|child| substitute_reified_dimension(child, bindings))
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
        ),
        DimensionExpr::Min(children) => DimensionExpr::Min(
            children
                .iter()
                .map(|child| substitute_reified_dimension(child, bindings))
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
        ),
        DimensionExpr::Max(children) => DimensionExpr::Max(
            children
                .iter()
                .map(|child| substitute_reified_dimension(child, bindings))
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
        ),
    })
}

#[cfg(feature = "convert")]
fn substitute_reified_cardinality(
    cardinality: &CardinalitySpec,
    bindings: &[Option<DimensionExpr>],
) -> MResult<CardinalitySpec> {
    Ok(match cardinality {
        CardinalitySpec::Exact(dimension) => {
            CardinalitySpec::Exact(substitute_reified_dimension(dimension, bindings)?)
        }
        CardinalitySpec::Dynamic { upper_bound } => CardinalitySpec::Dynamic {
            upper_bound: upper_bound
                .as_ref()
                .map(|bound| substitute_reified_dimension(bound, bindings))
                .transpose()?,
        },
    })
}

#[cfg(feature = "convert")]
fn substitute_reified_target(
    target: &SchemaBody,
    bindings: &[Option<DimensionExpr>],
) -> MResult<SchemaBody> {
    Ok(match target {
        SchemaBody::Matrix {
            element,
            dimensions,
        } => SchemaBody::Matrix {
            element: Box::new(substitute_reified_target(element, bindings)?),
            dimensions: dimensions
                .iter()
                .map(|dimension| substitute_reified_dimension(dimension, bindings))
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
        },
        SchemaBody::Option(element) => {
            SchemaBody::Option(Box::new(substitute_reified_target(element, bindings)?))
        }
        SchemaBody::Tuple(elements) => SchemaBody::Tuple(
            elements
                .iter()
                .map(|element| substitute_reified_target(element, bindings))
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
        ),
        SchemaBody::Record(fields) => SchemaBody::Record(
            fields
                .iter()
                .map(|field| {
                    Ok(SchemaField {
                        name: field.name.clone(),
                        schema: substitute_reified_target(&field.schema, bindings)?,
                    })
                })
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
        ),
        SchemaBody::Set {
            element,
            cardinality,
        } => SchemaBody::Set {
            element: Box::new(substitute_reified_target(element, bindings)?),
            cardinality: substitute_reified_cardinality(cardinality, bindings)?,
        },
        SchemaBody::Map {
            key,
            value,
            cardinality,
        } => SchemaBody::Map {
            key: Box::new(substitute_reified_target(key, bindings)?),
            value: Box::new(substitute_reified_target(value, bindings)?),
            cardinality: substitute_reified_cardinality(cardinality, bindings)?,
        },
        SchemaBody::Table { columns, rows } => SchemaBody::Table {
            columns: columns
                .iter()
                .map(|field| {
                    Ok(SchemaField {
                        name: field.name.clone(),
                        schema: substitute_reified_target(&field.schema, bindings)?,
                    })
                })
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
            rows: substitute_reified_cardinality(rows, bindings)?,
        },
        _ => target.clone(),
    })
}

/// Canonical source specialization for the frozen `convert/kind` intrinsic.
#[cfg(feature = "convert")]
pub struct ConvertKind;

#[cfg(feature = "convert")]
impl CanonicalFunctionSpecializer for ConvertKind {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        if invocation.len() != 2 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 2,
                    found: invocation.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let source = invocation
            .input(0)
            .expect("validated source")
            .cell()?
            .clone();
        let target_cell = invocation
            .input(1)
            .expect("validated target")
            .cell()?
            .clone();
        let target_value = target_cell.snapshot()?;
        let (target, reified_dimensions) = match target_value.data() {
            ValueData::Type(ReifiedType::Kind(kind)) => {
                let (target, dimensions) = schema_body_from_reified_kind(kind, context)?;
                (target, Some(dimensions))
            }
            ValueData::Type(ReifiedType::Schema(key)) => {
                (context.schema(*key)?.body().clone(), None)
            }
            _ => {
                return Err(MechError::new(
                    GenericError {
                        msg: "convert/kind requires a canonical reified-type target".to_owned(),
                    },
                    None,
                )
                .with_compiler_loc());
            }
        };
        let source_type = source.resolved_type()?;
        let is_reified_target = reified_dimensions.is_some();
        let (target, semantic_reified_target, reified_constraints) = if let Some(declarations) =
            reified_dimensions
        {
            let source_body = source.closed_schema_body()?;
            let mut target = target;
            inherit_reified_dynamic_cardinality(&source_body, &mut target, &declarations)?;
            let source_snapshot = source.snapshot()?;
            let source_schema = source_snapshot
                .schemas()
                .and_then(|schemas| schemas.get(source_snapshot.schema()).cloned())
                .ok_or_else(|| {
                    invalid_reified_conversion_target("source schema context is unavailable")
                })?;
            let mut semantic_bindings = vec![None; declarations.len()];
            bind_reified_target_dimensions(
                source_schema.body(),
                &target,
                &declarations,
                &mut semantic_bindings,
            )?;
            let semantic_target = substitute_reified_target(&target, &semantic_bindings)?;
            let mut bindings = vec![None; declarations.len()];
            bind_reified_target_dimensions(&source_body, &target, &declarations, &mut bindings)?;
            validate_reified_parameter_bindings(&declarations, &bindings)?;
            let reified_constraints = ReifiedTargetConstraints {
                target: target.clone(),
                declarations,
            };
            (
                substitute_reified_target(&target, &bindings)?,
                Some(semantic_target),
                Some(reified_constraints),
            )
        } else {
            (target, None, None)
        };
        let semantic_target = materialize_declared_conversion_semantic_shape(
            source_type.kind(),
            semantic_reified_target.as_ref().unwrap_or(&target),
        );
        let target = materialize_declared_conversion_shape(&source.closed_schema_body()?, &target);
        let target_dimensions = if is_reified_target
            && !has_dynamic_cardinality(&semantic_target)
            && !has_semantic_dimension_parameter(&semantic_target)
        {
            &[][..]
        } else {
            source_type.dimension_parameters()
        };
        let target_type = ResolvedType::from_schema_body(&semantic_target, target_dimensions)
            .map_err(MechError::from)?;
        let plan = plan_explicit_cast(&source_type, &target_type).map_err(|error| {
            MechError::from(error.with_origin(TypeConstraintOrigin::new("convert/kind", None)))
        })?;
        let output = execute_conversion_plan(&source, &target, &plan)?;
        let _ = target_cell;
        context.resolve_syntax_operation_contract(&PURE_TYPE_CONVERSION_CONTRACT)?;
        context.certify_instance(
            planned_type_conversion_instance(source, output, plan, reified_constraints),
            mech_core::RuntimeFunctionId::from_name("convert/kind"),
            mech_core::ExecutionTarget::DirectRuntime,
            mech_core::ImplementationMemoryClass::CanonicalFinalize,
        )
    }
}

#[cfg(feature = "convert")]
impl MechFunctionImpl for PlannedTypeConversion {
    fn planned_output_footprints(&self) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
        Ok(
            prospective_conversion_output_footprint(&self.output, &self.source, &self.plan)?
                .map(|footprint| vec![footprint].into_boxed_slice()),
        )
    }

    fn solve_managed(
        &self,
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        _services: &mut dyn mech_core::MechExecutionServices,
    ) -> MResult<mech_core::ReactiveSolveStatus> {
        if let Some(constraints) = &self.reified_constraints {
            constraints.validate(&self.source)?;
        }
        stage_conversion_output(frame, &self.source, &self.output, &self.plan)?;
        Ok(mech_core::ReactiveSolveStatus::Changed)
    }

    fn semantic_operation_name(&self) -> Option<&str> {
        Some("convert/kind")
    }

    #[cfg(feature = "semantic-compiler")]
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_TYPE_CONVERSION_CONTRACT)
    }

    fn to_string(&self) -> String {
        "PlannedTypeConversion".to_owned()
    }
}

#[cfg(all(feature = "convert", feature = "semantic-compiler"))]
impl MechFunctionCompiler for PlannedTypeConversion {
    fn compiler_owned_value_cells(&self) -> Vec<ValueCell> {
        vec![self.source.clone(), self.output.clone()]
    }

    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let destination = compile_runtime_produced_value_cell_register_with_seed(
            &self.output,
            &self.output.snapshot()?,
            context,
        )?;
        let source = compile_value_cell_register(&self.source, context)?;
        let function = context.function_id("convert/kind")?;
        // The resolved target is carried by the destination's canonical schema.
        // ConversionPlan and reified compiler metadata remain in-memory only;
        // bytecode-v1 therefore needs no reified-type constant or wire change.
        context.emit_unop(function, destination, source);
        Ok(destination)
    }
}

#[cfg(feature = "convert")]
pub(crate) fn convert_cell_with_plan_reactively(
    value: ValueCell,
    plan: &ConversionPlan,
    interpreter: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    if matches!(plan.step, ConversionStep::Identity) {
        return Ok(value);
    }
    let source_schema = value.closed_schema_body()?;
    let target =
        conversion_target_schema(&source_schema, &plan.step).map_err(conversion_execution_error)?;
    let output = execute_conversion_plan(&value, &target, plan)?;
    interpreter
        .plan()
        .register_specialized(planned_type_conversion_specialized(
            value,
            output.clone(),
            plan.clone(),
        )?)?;
    Ok(output)
}

/// Builds the exact lossless conversion selected by semantic input/output
/// compatibility. User-function boundaries use this path; lossy conversions
/// remain available only through the explicit `convert/kind` intrinsic.
#[cfg(feature = "convert")]
pub(crate) fn convert_cell_implicitly_reactively(
    value: ValueCell,
    target: SchemaBody,
    interpreter: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    let source_type = value.resolved_type()?;
    let semantic_target =
        materialize_declared_conversion_semantic_shape(source_type.kind(), &target);
    let target = materialize_declared_conversion_shape(&value.closed_schema_body()?, &target);
    if value.closed_schema_body()? == target {
        return Ok(value);
    }
    let target_type =
        ResolvedType::from_schema_body(&semantic_target, source_type.dimension_parameters())
            .map_err(MechError::from)?;
    let plan = plan_implicit_conversion(&source_type, &target_type).map_err(MechError::from)?;
    let output = execute_conversion_plan(&value, &target, &plan)?;
    interpreter
        .plan()
        .register_specialized(planned_type_conversion_specialized(
            value,
            output.clone(),
            plan,
        )?)?;
    Ok(output)
}

/// Builds one reactive, schema-directed conversion without routing semantic
/// values through the retired universal value representation.
#[cfg(feature = "convert")]
pub(crate) fn convert_cell_reactively(
    value: ValueCell,
    target: SchemaBody,
    interpreter: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    let source_type = value.resolved_type()?;
    let semantic_target =
        materialize_declared_conversion_semantic_shape(source_type.kind(), &target);
    let target = materialize_declared_conversion_shape(&value.closed_schema_body()?, &target);
    if value.closed_schema_body()? == target {
        return Ok(value);
    }
    let target_type =
        ResolvedType::from_schema_body(&semantic_target, source_type.dimension_parameters())
            .map_err(MechError::from)?;
    let plan = plan_explicit_cast(&source_type, &target_type).map_err(MechError::from)?;
    let output = execute_conversion_plan(&value, &target, &plan)?;
    interpreter
        .plan()
        .register_specialized(planned_type_conversion_specialized(
            value,
            output.clone(),
            plan,
        )?)?;
    Ok(output)
}

#[cfg(feature = "atom")]
pub fn atom(atm: &Atom, p: &InterpreterExecution<'_>) -> MResult<ValueCell> {
    let id = atm.name.hash();
    let name = atm.name.to_string();
    let state = p.state.borrow();
    let dictionary = state.dictionary.clone();
    {
        let mut dictionary_brrw = dictionary.borrow_mut();
        dictionary_brrw.insert(id, name.clone());
    }
    let path = CanonicalNominalPath::new(
        name.split('/')
            .filter(|segment| !segment.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>(),
    )?;
    let key = NominalKey::from_path(NominalKind::Atom, &path);
    ValueCell::from_schema_data(SchemaBody::Atom(key), ValueDataDraft::Atom)
}

pub fn number(num: &Number, p: &InterpreterExecution<'_>) -> MResult<ValueCell> {
    match num {
        Number::Real(num) => real(num, p),
        #[cfg(feature = "complex")]
        Number::Complex(num) => complex(num, p),
        #[cfg(not(feature = "complex"))]
        _ => panic!("Number type not supported."),
    }
}

#[cfg(feature = "complex")]
fn complex(num: &C64Node, p: &InterpreterExecution<'_>) -> MResult<ValueCell> {
    let im = cell_f64(&real(&num.imaginary.number, p)?)?.unwrap_or(0.0);
    let result = match &num.real {
        Some(real_val) => {
            let re = cell_f64(&real(&real_val, p)?)?.unwrap_or(0.0);
            C64::new(re, im)
        }
        None => C64::new(0.0, im),
    };
    ValueCell::from_exact(result)
}

#[cfg(any(
    feature = "math_neg",
    feature = "f64",
    feature = "floats",
    feature = "i64",
    feature = "rational",
    feature = "convert"
))]
pub fn real(
    rl: &RealNumber,
    #[cfg(any(feature = "math_neg", feature = "convert"))] p: &InterpreterExecution<'_>,
    #[cfg(not(any(feature = "math_neg", feature = "convert")))] _: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    let result = match rl {
        #[cfg(feature = "math_neg")]
        RealNumber::Negated(num) => negated(num, p)?,
        #[cfg(feature = "f64")]
        RealNumber::Integer(num) => integer(num)?,
        #[cfg(feature = "floats")]
        RealNumber::Float(num) => float(num)?,
        #[cfg(feature = "i64")]
        RealNumber::Decimal(num) => dec(num)?,
        #[cfg(feature = "i64")]
        RealNumber::Hexadecimal(num) => hex(num)?,
        #[cfg(feature = "i64")]
        RealNumber::Octal(num) => oct(num)?,
        #[cfg(feature = "i64")]
        RealNumber::Binary(num) => binary(num)?,
        #[cfg(feature = "floats")]
        RealNumber::Scientific(num) => scientific(num)?,
        #[cfg(feature = "rational")]
        RealNumber::Rational(num) => rational(num)?,
        #[cfg(feature = "convert")]
        RealNumber::TypedInteger((num_tkn, kind)) => {
            let num: Literal = Literal::Number(Number::Real(RealNumber::Integer(num_tkn.clone())));
            typed_literal(&num, kind, p)?
        }
        #[cfg(not(all(
            feature = "math_neg",
            feature = "f64",
            feature = "floats",
            feature = "i64",
            feature = "rational",
            feature = "convert"
        )))]
        _ => panic!("Number type not supported."),
    };
    Ok(result)
}

#[cfg(not(any(
    feature = "math_neg",
    feature = "f64",
    feature = "floats",
    feature = "i64",
    feature = "rational",
    feature = "convert"
)))]
pub fn real(_: &RealNumber, _: &InterpreterExecution<'_>) -> MResult<ValueCell> {
    panic!("Number type not supported.")
}

#[cfg(all(test, feature = "convert", feature = "f64", feature = "u8"))]
mod canonical_conversion_tests {
    use super::*;

    struct NamedKinds(BTreeMap<KindId, CanonicalNominalPath>);

    impl NamedKindPathResolver for NamedKinds {
        fn canonical_path(&self, id: KindId) -> Option<&CanonicalNominalPath> {
            self.0.get(&id)
        }
    }

    fn convert_reified(source: ValueCell, kind: ReifiedKind) -> MResult<ValueCell> {
        let target = ValueCell::from_schema_data(
            SchemaBody::ReifiedType,
            ValueDataDraft::Type(ReifiedTypeDraft::CanonicalKind(
                kind.canonical_bytes().to_vec().into_boxed_slice(),
            )),
        )?;
        let invocation =
            SpecializationInvocation::from_cells(vec![source, target].into_boxed_slice());
        let operation = ResolvedOperationDescriptor::from_name(
            "convert/kind",
            PURE_TYPE_CONVERSION_CONTRACT.clone(),
        )?;
        let mut context =
            SpecializationContext::for_syntax_directed_invocation(&invocation, None, operation)?;
        Ok(ConvertKind
            .specialize_invocation(&invocation, &mut context)?
            .output()
            .clone())
    }

    fn convert_schema_identity(source: ValueCell) -> MResult<ValueCell> {
        let target = ValueCell::from_schema_data(
            SchemaBody::ReifiedType,
            ValueDataDraft::Type(ReifiedTypeDraft::Schema(source.schema_key())),
        )?;
        let invocation =
            SpecializationInvocation::from_cells(vec![source, target].into_boxed_slice());
        let operation = ResolvedOperationDescriptor::from_name(
            "convert/kind",
            PURE_TYPE_CONVERSION_CONTRACT.clone(),
        )?;
        let mut context =
            SpecializationContext::for_syntax_directed_invocation(&invocation, None, operation)?;
        Ok(ConvertKind
            .specialize_invocation(&invocation, &mut context)?
            .output()
            .clone())
    }

    #[test]
    fn nested_identity_conversion_keeps_shape_witnesses_and_separate_cell_identity() {
        let id = DimensionParameterId::new(0);
        let schema = SchemaDraft {
            body: SchemaBody::Tuple(
                [SchemaBody::Matrix {
                    element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                    dimensions: [DimensionExpr::Parameter(id), DimensionExpr::Constant(2)].into(),
                }]
                .into(),
            ),
            dimension_parameters: [DimensionParameterDeclaration {
                id,
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Turn,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: None,
            }]
            .into(),
        }
        .finalize()
        .unwrap();
        let shape = schema.instantiate_shape([1_u64].into()).unwrap();
        let descriptor = mech_core::ResolvedValueDescriptor::from_schema(schema, shape).unwrap();
        let source = ValueCell::from_resolved_descriptor_data(
            &descriptor,
            ValueDataDraft::Tuple(
                [ValueDataDraft::Matrix(
                    [
                        ValueDataDraft::F64(F64Bits::from_f64(1.0)),
                        ValueDataDraft::F64(F64Bits::from_f64(2.0)),
                    ]
                    .into(),
                )]
                .into(),
            ),
        )
        .unwrap();
        let output = convert_schema_identity(source.clone()).unwrap();
        assert_ne!(source.reactive_cell_id(), output.reactive_cell_id());
        assert_eq!(
            source.closed_schema_body().unwrap(),
            output.closed_schema_body().unwrap()
        );
        assert_eq!(
            source.snapshot().unwrap().canonical_data_draft().unwrap(),
            output.snapshot().unwrap().canonical_data_draft().unwrap(),
        );
    }

    #[test]
    fn identity_conversion_retains_nested_dynamic_schema_ids() {
        let text = ValueCell::from_exact("hello".to_owned()).unwrap();
        let number = ValueCell::from_exact(4.0_f64).unwrap();
        let source = ValueCell::table_from_cell_columns(
            [(
                SchemaField {
                    name: "value".to_owned(),
                    schema: SchemaBody::Dynamic,
                },
                [text, number].into(),
            )]
            .into(),
            CardinalitySpec::Exact(DimensionExpr::Constant(2)),
        )
        .unwrap();
        let output = convert_schema_identity(source.clone()).unwrap();
        assert_ne!(source.reactive_cell_id(), output.reactive_cell_id());
        assert_eq!(source.schema_key(), output.schema_key());
        assert_eq!(
            source.snapshot().unwrap().canonical_data_draft().unwrap(),
            output.snapshot().unwrap().canonical_data_draft().unwrap(),
        );
    }

    #[test]
    fn reactive_identity_conversion_tracks_new_matrix_extents() {
        let matrix = |rows, columns, values: &[f64]| {
            ValueCell::dynamic_matrix_from_cells(
                rows,
                columns,
                &values
                    .iter()
                    .map(|value| ValueCell::from_exact(*value).unwrap())
                    .collect::<Vec<_>>(),
            )
            .unwrap()
        };
        let source = matrix(1, 2, &[1.0, 2.0]);
        let source_type = source.resolved_type().unwrap();
        let target = source.closed_schema_body().unwrap();
        let plan = plan_explicit_cast(&source_type, &source_type).unwrap();
        let output = execute_conversion_plan(&source, &target, &plan).unwrap();
        let conversion =
            planned_type_conversion_specialized(source.clone(), output.clone(), plan).unwrap();
        source
            .replace(&matrix(2, 2, &[1.0, 2.0, 3.0, 4.0]).snapshot().unwrap())
            .unwrap();
        conversion.instance().solve_result().unwrap();
        let SchemaBody::Matrix { dimensions, .. } = output.closed_schema_body().unwrap() else {
            panic!("identity output must remain a matrix")
        };
        assert_eq!(
            dimensions.as_ref(),
            &[DimensionExpr::Constant(2), DimensionExpr::Constant(2)]
        );
        assert_ne!(source.reactive_cell_id(), output.reactive_cell_id());
    }

    #[test]
    fn nested_option_matrix_conversion_tracks_turn_shape() {
        let id = DimensionParameterId::new(0);
        let source_schema = SchemaDraft {
            body: SchemaBody::Option(Box::new(SchemaBody::Matrix {
                element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                dimensions: [DimensionExpr::Parameter(id), DimensionExpr::Constant(2)].into(),
            })),
            dimension_parameters: [DimensionParameterDeclaration {
                id,
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Turn,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: None,
            }]
            .into(),
        }
        .finalize()
        .unwrap();
        let source_for_rows = |rows: u64| {
            let shape = source_schema.instantiate_shape([rows].into()).unwrap();
            let descriptor =
                mech_core::ResolvedValueDescriptor::from_schema(source_schema.clone(), shape)
                    .unwrap();
            let values = (0..rows * 2)
                .map(|value| ValueDataDraft::F64(F64Bits::from_f64(value as f64)))
                .collect();
            ValueCell::from_resolved_descriptor_data(
                &descriptor,
                ValueDataDraft::Option(OptionDraft {
                    present: true,
                    value: Some(Box::new(ValueDataDraft::Matrix(values))),
                }),
            )
            .unwrap()
        };
        let source = source_for_rows(1);
        let target = SchemaBody::Option(Box::new(SchemaBody::Matrix {
            element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
            dimensions: [DimensionExpr::Parameter(id), DimensionExpr::Constant(2)].into(),
        }));
        let source_type = source.resolved_type().unwrap();
        let target_type =
            ResolvedType::from_schema_body(&target, source_type.dimension_parameters()).unwrap();
        let plan = plan_explicit_cast(&source_type, &target_type).unwrap();
        let output = execute_conversion_plan(&source, &target, &plan).unwrap();
        assert_eq!(output.shape().parameter_values(), &[1]);
        let conversion =
            planned_type_conversion_specialized(source.clone(), output.clone(), plan).unwrap();
        source
            .replace(&source_for_rows(2).snapshot().unwrap())
            .unwrap();
        conversion.instance().solve_result().unwrap();
        assert_eq!(output.shape().parameter_values(), &[2]);
        assert_eq!(
            output.closed_schema_body().unwrap(),
            SchemaBody::Option(Box::new(SchemaBody::Matrix {
                element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
                dimensions: [DimensionExpr::Constant(2), DimensionExpr::Constant(2)].into(),
            }))
        );
    }

    #[test]
    fn convert_kind_specializer_uses_reified_canonical_target() {
        let (id, path) = builtin_scalar_named_kind(mech_core::hash_str("u8")).unwrap();
        let named = NamedKinds(BTreeMap::from([(id, path)]));
        let kind = ReifiedKind::from_closed_kind(&KindExpr::Named(id), &[], &named).unwrap();
        let target = ValueCell::from_schema_data(
            SchemaBody::ReifiedType,
            ValueDataDraft::Type(ReifiedTypeDraft::CanonicalKind(
                kind.canonical_bytes().to_vec().into_boxed_slice(),
            )),
        )
        .unwrap();
        let source = ValueCell::from_exact(7.0_f64).unwrap();
        let invocation =
            SpecializationInvocation::from_cells(vec![source, target].into_boxed_slice());
        let operation = ResolvedOperationDescriptor::from_name(
            "convert/kind",
            PURE_TYPE_CONVERSION_CONTRACT.clone(),
        )
        .unwrap();
        let mut context =
            SpecializationContext::for_syntax_directed_invocation(&invocation, None, operation)
                .unwrap();

        let specialized = ConvertKind
            .specialize_invocation(&invocation, &mut context)
            .unwrap();

        assert!(matches!(
            specialized.output().snapshot().unwrap().data(),
            ValueData::U8(7)
        ));
    }

    #[test]
    fn dimensionless_reified_matrix_kind_inherits_the_source_shape() {
        let (id, path) = builtin_scalar_named_kind(mech_core::hash_str("u8")).unwrap();
        let named = NamedKinds(BTreeMap::from([(id, path)]));
        let dimensions = [0, 1].map(|id| DimensionParameterDeclaration {
            id: DimensionParameterId::new(id),
            origin: DimensionParameterOrigin::Inferred,
            lifetime: DimensionLifetime::Activation,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: None,
        });
        let kind = ReifiedKind::from_closed_kind(
            &KindExpr::Matrix {
                element: Box::new(KindExpr::Named(id)),
                dimensions: vec![
                    DimensionExpr::Parameter(DimensionParameterId::new(0)),
                    DimensionExpr::Parameter(DimensionParameterId::new(1)),
                ]
                .into_boxed_slice(),
            },
            &dimensions,
            &named,
        )
        .unwrap();
        let target = ValueCell::from_schema_data(
            SchemaBody::ReifiedType,
            ValueDataDraft::Type(ReifiedTypeDraft::CanonicalKind(
                kind.canonical_bytes().to_vec().into_boxed_slice(),
            )),
        )
        .unwrap();
        let source = ValueCell::from_schema_data(
            SchemaBody::Matrix {
                element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                dimensions: vec![DimensionExpr::Constant(2), DimensionExpr::Constant(2)]
                    .into_boxed_slice(),
            },
            ValueDataDraft::Matrix(
                [1.0, 2.0, 3.0, 4.0]
                    .into_iter()
                    .map(|value| ValueDataDraft::F64(F64Bits::from_f64(value)))
                    .collect(),
            ),
        )
        .unwrap();
        let invocation =
            SpecializationInvocation::from_cells(vec![source, target].into_boxed_slice());
        let operation = ResolvedOperationDescriptor::from_name(
            "convert/kind",
            PURE_TYPE_CONVERSION_CONTRACT.clone(),
        )
        .unwrap();
        let mut context =
            SpecializationContext::for_syntax_directed_invocation(&invocation, None, operation)
                .unwrap();

        let repeated = ReifiedKind::from_closed_kind(
            &KindExpr::Matrix {
                element: Box::new(KindExpr::Named(id)),
                dimensions: vec![DimensionExpr::Parameter(DimensionParameterId::new(0)); 2]
                    .into_boxed_slice(),
            },
            &dimensions,
            &named,
        )
        .unwrap();
        assert_eq!(
            schema_body_from_reified_kind(&repeated, &context)
                .unwrap()
                .0,
            SchemaBody::Matrix {
                element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
                dimensions: vec![DimensionExpr::Parameter(DimensionParameterId::new(0)); 2]
                    .into_boxed_slice(),
            }
        );

        let specialized = ConvertKind
            .specialize_invocation(&invocation, &mut context)
            .unwrap();

        assert_eq!(
            specialized.output().closed_schema_body().unwrap(),
            SchemaBody::Matrix {
                element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
                dimensions: vec![DimensionExpr::Constant(2), DimensionExpr::Constant(2)]
                    .into_boxed_slice(),
            }
        );
        assert!(matches!(
            specialized
                .output()
                .snapshot()
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            ValueDataDraft::Matrix(values)
                if values.as_ref()
                    == [ValueDataDraft::U8(1), ValueDataDraft::U8(2),
                        ValueDataDraft::U8(3), ValueDataDraft::U8(4)]
        ));
    }

    #[test]
    fn open_reified_matrix_specializes_from_semantic_turn_axes() {
        let (id, path) = builtin_scalar_named_kind(mech_core::hash_str("u8")).unwrap();
        let named = NamedKinds(BTreeMap::from([(id, path)]));
        let dimensions = [0, 1].map(|id| DimensionParameterDeclaration {
            id: DimensionParameterId::new(id),
            origin: DimensionParameterOrigin::Inferred,
            lifetime: DimensionLifetime::Turn,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: None,
        });
        let kind = ReifiedKind::from_closed_kind(
            &KindExpr::Matrix {
                element: Box::new(KindExpr::Named(id)),
                dimensions: [
                    DimensionExpr::Parameter(DimensionParameterId::new(0)),
                    DimensionExpr::Parameter(DimensionParameterId::new(1)),
                ]
                .into(),
            },
            &dimensions,
            &named,
        )
        .unwrap();
        let source = ValueCell::dynamic_matrix_from_cells(
            1,
            2,
            &[
                ValueCell::from_exact(1.0_f64).unwrap(),
                ValueCell::from_exact(2.0_f64).unwrap(),
            ],
        )
        .unwrap();
        let converted = convert_reified(source, kind).unwrap();
        assert_eq!(
            converted.closed_schema_body().unwrap(),
            SchemaBody::Matrix {
                element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
                dimensions: [DimensionExpr::Constant(1), DimensionExpr::Constant(2)].into(),
            }
        );
    }

    #[test]
    fn open_conversion_targets_reuse_nested_shapes_and_collection_cardinalities() {
        let source = SchemaBody::Tuple(
            vec![
                SchemaBody::Matrix {
                    element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
                    dimensions: vec![DimensionExpr::Constant(2), DimensionExpr::Constant(3)].into(),
                },
                SchemaBody::Set {
                    element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
                    cardinality: CardinalitySpec::Dynamic { upper_bound: None },
                },
                SchemaBody::Record(
                    vec![SchemaField {
                        name: "map".to_owned(),
                        schema: SchemaBody::Map {
                            key: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
                            value: Box::new(SchemaBody::Table {
                                columns: vec![SchemaField {
                                    name: "values".to_owned(),
                                    schema: SchemaBody::Matrix {
                                        element: Box::new(SchemaBody::UnsignedInteger(
                                            IntegerWidth::W8,
                                        )),
                                        dimensions: vec![
                                            DimensionExpr::Constant(2),
                                            DimensionExpr::Constant(3),
                                        ]
                                        .into(),
                                    },
                                }]
                                .into(),
                                rows: CardinalitySpec::Dynamic { upper_bound: None },
                            }),
                            cardinality: CardinalitySpec::Dynamic { upper_bound: None },
                        },
                    }]
                    .into(),
                ),
            ]
            .into(),
        );
        let target = SchemaBody::Tuple(
            vec![
                SchemaBody::Matrix {
                    element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
                    dimensions: Box::new([]),
                },
                SchemaBody::Set {
                    element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
                    cardinality: CardinalitySpec::Dynamic { upper_bound: None },
                },
                SchemaBody::Record(
                    vec![SchemaField {
                        name: "map".to_owned(),
                        schema: SchemaBody::Map {
                            key: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
                            value: Box::new(SchemaBody::Table {
                                columns: vec![SchemaField {
                                    name: "values".to_owned(),
                                    schema: SchemaBody::Matrix {
                                        element: Box::new(SchemaBody::UnsignedInteger(
                                            IntegerWidth::W8,
                                        )),
                                        dimensions: Box::new([]),
                                    },
                                }]
                                .into(),
                                rows: CardinalitySpec::Dynamic { upper_bound: None },
                            }),
                            cardinality: CardinalitySpec::Dynamic { upper_bound: None },
                        },
                    }]
                    .into(),
                ),
            ]
            .into(),
        );
        assert_eq!(
            materialize_declared_conversion_shape(&source, &target),
            source
        );
        let resolved = ResolvedType::from_schema_body(&source, &[]).unwrap();
        let semantic = materialize_declared_conversion_semantic_shape(resolved.kind(), &target);
        let target_type =
            ResolvedType::from_schema_body(&semantic, resolved.dimension_parameters()).unwrap();
        assert!(exact_type_equal(&resolved, &target_type));
    }

    #[test]
    fn open_matrix_annotations_inherit_every_source_rank() {
        let target = SchemaBody::Matrix {
            element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
            dimensions: Box::new([]),
        };
        for extents in [vec![3], vec![2, 3], vec![2, 3, 4]] {
            let source = SchemaBody::Matrix {
                element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
                dimensions: extents.into_iter().map(DimensionExpr::Constant).collect(),
            };
            assert_eq!(
                materialize_declared_conversion_shape(&source, &target),
                source
            );
            let resolved = ResolvedType::from_schema_body(&source, &[]).unwrap();
            let semantic = materialize_declared_conversion_semantic_shape(resolved.kind(), &target);
            let target_type = ResolvedType::from_schema_body(&semantic, &[]).unwrap();
            assert!(exact_type_equal(&resolved, &target_type));
        }
    }

    #[test]
    fn explicit_bounded_dynamic_cardinality_survives_shape_materialization() {
        let source = SchemaBody::Set {
            element: Box::new(SchemaBody::Index),
            cardinality: CardinalitySpec::Exact(DimensionExpr::Constant(5)),
        };
        let target = SchemaBody::Set {
            element: Box::new(SchemaBody::Index),
            cardinality: CardinalitySpec::Dynamic {
                upper_bound: Some(DimensionExpr::Constant(2)),
            },
        };
        assert_eq!(
            materialize_declared_conversion_shape(&source, &target),
            target
        );
        let resolved = ResolvedType::from_schema_body(&source, &[]).unwrap();
        assert_eq!(
            materialize_declared_conversion_semantic_shape(resolved.kind(), &target),
            target
        );
    }

    #[test]
    fn bounded_reified_matrix_constraint_checks_each_source_turn() {
        let id = DimensionParameterId::new(0);
        let constraints = ReifiedTargetConstraints {
            target: SchemaBody::Matrix {
                element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                dimensions: [DimensionExpr::Parameter(id), DimensionExpr::Constant(1)].into(),
            },
            declarations: [DimensionParameterDeclaration {
                id,
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Turn,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: Some(DimensionExpr::Constant(2)),
            }]
            .into(),
        };
        let matrix = |rows, values: &[f64]| {
            ValueCell::dynamic_matrix_from_cells(
                rows,
                1,
                &values
                    .iter()
                    .map(|value| ValueCell::from_exact(*value).unwrap())
                    .collect::<Vec<_>>(),
            )
            .unwrap()
        };
        let source = matrix(1, &[1.0]);
        constraints.validate(&source).unwrap();
        source
            .replace(&matrix(3, &[1.0, 2.0, 3.0]).snapshot().unwrap())
            .unwrap();
        assert!(constraints.validate(&source).is_err());
    }

    #[test]
    fn bounded_and_compound_reified_dimensions_bind_unique_extents() {
        let id = DimensionParameterId::new(0);
        let declaration = DimensionParameterDeclaration {
            id,
            origin: DimensionParameterOrigin::Inferred,
            lifetime: DimensionLifetime::Activation,
            lower_bound: DimensionExpr::Constant(2),
            upper_bound: Some(DimensionExpr::Constant(10)),
        };
        let mut bindings = vec![None];
        bind_reified_dimension(
            &DimensionExpr::Constant(8),
            &DimensionExpr::Parameter(id),
            &[declaration.clone()],
            &mut bindings,
        )
        .unwrap();
        assert_eq!(bindings, vec![Some(DimensionExpr::Constant(8))]);
        validate_reified_parameter_bindings(&[declaration.clone()], &bindings).unwrap();
        let mut outside_bounds = vec![None];
        bind_reified_dimension(
            &DimensionExpr::Constant(11),
            &DimensionExpr::Parameter(id),
            &[declaration.clone()],
            &mut outside_bounds,
        )
        .unwrap();
        assert!(
            validate_reified_parameter_bindings(&[declaration.clone()], &outside_bounds).is_err()
        );

        let compound =
            DimensionExpr::Add([DimensionExpr::Parameter(id), DimensionExpr::Constant(1)].into());
        let mut bindings = vec![None];
        bind_reified_dimension(
            &DimensionExpr::Constant(8),
            &compound,
            &[declaration.clone()],
            &mut bindings,
        )
        .unwrap();
        assert_eq!(bindings, vec![Some(DimensionExpr::Constant(7))]);
        let mut bindings = vec![None];
        bind_reified_dimension(
            &DimensionExpr::Constant(14),
            &DimensionExpr::Multiply(
                [DimensionExpr::Constant(2), DimensionExpr::Parameter(id)].into(),
            ),
            &[declaration.clone()],
            &mut bindings,
        )
        .unwrap();
        assert_eq!(bindings, vec![Some(DimensionExpr::Constant(7))]);
        assert!(
            bind_reified_dimension(
                &DimensionExpr::Constant(5),
                &DimensionExpr::Min(
                    [DimensionExpr::Parameter(id), DimensionExpr::Constant(5)].into(),
                ),
                &[declaration],
                &mut vec![None],
            )
            .is_err()
        );
    }

    #[test]
    fn reified_bound_dependencies_do_not_depend_on_axis_order() {
        let p0 = DimensionParameterId::new(0);
        let p1 = DimensionParameterId::new(1);
        let declarations = [
            DimensionParameterDeclaration {
                id: p0,
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Activation,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: None,
            },
            DimensionParameterDeclaration {
                id: p1,
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Activation,
                lower_bound: DimensionExpr::Parameter(p0),
                upper_bound: None,
            },
        ];
        let target = SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: [DimensionExpr::Parameter(p1), DimensionExpr::Parameter(p0)].into(),
        };
        for (first, valid) in [(5, true), (1, false)] {
            let source = SchemaBody::Matrix {
                element: Box::new(SchemaBody::Index),
                dimensions: [DimensionExpr::Constant(first), DimensionExpr::Constant(2)].into(),
            };
            let mut bindings = vec![None; declarations.len()];
            bind_reified_target_dimensions(&source, &target, &declarations, &mut bindings).unwrap();
            assert_eq!(
                validate_reified_parameter_bindings(&declarations, &bindings).is_ok(),
                valid,
            );
        }
    }

    #[test]
    fn open_reified_collection_targets_keep_dynamic_cardinality() {
        let source = SchemaBody::Tuple(
            [
                SchemaBody::Set {
                    element: Box::new(SchemaBody::Index),
                    cardinality: CardinalitySpec::Dynamic { upper_bound: None },
                },
                SchemaBody::Map {
                    key: Box::new(SchemaBody::Index),
                    value: Box::new(SchemaBody::Index),
                    cardinality: CardinalitySpec::Dynamic { upper_bound: None },
                },
                SchemaBody::Table {
                    columns: [SchemaField {
                        name: "value".to_owned(),
                        schema: SchemaBody::Index,
                    }]
                    .into(),
                    rows: CardinalitySpec::Dynamic { upper_bound: None },
                },
            ]
            .into(),
        );
        let parameters = [0, 1, 2].map(|index| DimensionParameterDeclaration {
            id: DimensionParameterId::new(index),
            origin: DimensionParameterOrigin::Inferred,
            lifetime: DimensionLifetime::Activation,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: None,
        });
        let exact = |index| {
            CardinalitySpec::Exact(DimensionExpr::Parameter(DimensionParameterId::new(index)))
        };
        let mut target = SchemaBody::Tuple(
            [
                SchemaBody::Set {
                    element: Box::new(SchemaBody::Index),
                    cardinality: exact(0),
                },
                SchemaBody::Map {
                    key: Box::new(SchemaBody::Index),
                    value: Box::new(SchemaBody::Index),
                    cardinality: exact(1),
                },
                SchemaBody::Table {
                    columns: [SchemaField {
                        name: "value".to_owned(),
                        schema: SchemaBody::Index,
                    }]
                    .into(),
                    rows: exact(2),
                },
            ]
            .into(),
        );
        inherit_reified_dynamic_cardinality(&source, &mut target, &parameters).unwrap();
        assert_eq!(target, source);
        let mut bindings = vec![None; parameters.len()];
        bind_reified_target_dimensions(&source, &target, &parameters, &mut bindings).unwrap();
        assert_eq!(
            substitute_reified_target(&target, &bindings).unwrap(),
            source
        );
    }

    #[test]
    fn dynamic_cardinality_cannot_discard_a_shared_dimension_constraint() {
        let id = DimensionParameterId::new(0);
        let declaration = DimensionParameterDeclaration {
            id,
            origin: DimensionParameterOrigin::Inferred,
            lifetime: DimensionLifetime::Activation,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: None,
        };
        let set = |cardinality| SchemaBody::Set {
            element: Box::new(SchemaBody::Index),
            cardinality,
        };
        let dynamic = CardinalitySpec::Dynamic { upper_bound: None };
        let exact = CardinalitySpec::Exact(DimensionExpr::Parameter(id));
        let source = SchemaBody::Tuple(
            [
                set(dynamic.clone()),
                SchemaBody::Matrix {
                    element: Box::new(SchemaBody::Index),
                    dimensions: [DimensionExpr::Constant(2)].into(),
                },
            ]
            .into(),
        );
        let mut target = SchemaBody::Tuple(
            [
                set(exact.clone()),
                SchemaBody::Matrix {
                    element: Box::new(SchemaBody::Index),
                    dimensions: [DimensionExpr::Parameter(id)].into(),
                },
            ]
            .into(),
        );
        assert!(
            inherit_reified_dynamic_cardinality(&source, &mut target, &[declaration.clone()])
                .is_err()
        );

        let source = SchemaBody::Tuple([set(dynamic.clone()), set(dynamic)].into());
        let mut target = SchemaBody::Tuple([set(exact.clone()), set(exact)].into());
        assert!(inherit_reified_dynamic_cardinality(&source, &mut target, &[declaration]).is_err());
    }

    #[test]
    fn dynamic_set_converts_to_its_open_reified_kind() {
        let (id, path) = builtin_scalar_named_kind(mech_core::hash_str("u8")).unwrap();
        let named = NamedKinds(BTreeMap::from([(id, path)]));
        let parameter = DimensionParameterDeclaration {
            id: DimensionParameterId::new(0),
            origin: DimensionParameterOrigin::Inferred,
            lifetime: DimensionLifetime::Activation,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: None,
        };
        let kind = ReifiedKind::from_closed_kind(
            &KindExpr::Set {
                element: Box::new(KindExpr::Named(id)),
                cardinality: DimensionExpr::Parameter(parameter.id),
            },
            &[parameter],
            &named,
        )
        .unwrap();
        for upper_bound in [None, Some(DimensionExpr::Constant(4))] {
            let schema = SchemaBody::Set {
                element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
                cardinality: CardinalitySpec::Dynamic { upper_bound },
            };
            let source = ValueCell::from_schema_data(
                schema.clone(),
                ValueDataDraft::Set([ValueDataDraft::U8(1), ValueDataDraft::U8(2)].into()),
            )
            .unwrap();
            let converted = convert_reified(source, kind.clone()).unwrap();
            assert_eq!(converted.closed_schema_body().unwrap(), schema);
        }
    }

    #[test]
    fn compound_bounded_reified_matrix_extent_converts_end_to_end() {
        let (id, path) = builtin_scalar_named_kind(mech_core::hash_str("u8")).unwrap();
        let named = NamedKinds(BTreeMap::from([(id, path)]));
        let parameter = DimensionParameterDeclaration {
            id: DimensionParameterId::new(0),
            origin: DimensionParameterOrigin::Inferred,
            lifetime: DimensionLifetime::Activation,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: Some(DimensionExpr::Constant(16)),
        };
        let kind = ReifiedKind::from_closed_kind(
            &KindExpr::Matrix {
                element: Box::new(KindExpr::Named(id)),
                dimensions: [
                    DimensionExpr::Constant(2),
                    DimensionExpr::Add(
                        [
                            DimensionExpr::Parameter(parameter.id),
                            DimensionExpr::Constant(1),
                        ]
                        .into(),
                    ),
                ]
                .into(),
            },
            &[parameter],
            &named,
        )
        .unwrap();
        let schema = SchemaBody::Matrix {
            element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
            dimensions: [DimensionExpr::Constant(2), DimensionExpr::Constant(8)].into(),
        };
        let source = ValueCell::from_schema_data(
            schema.clone(),
            ValueDataDraft::Matrix((0..16).map(ValueDataDraft::U8).collect()),
        )
        .unwrap();
        let converted = convert_reified(source, kind).unwrap();
        assert_eq!(converted.closed_schema_body().unwrap(), schema);
    }

    #[test]
    fn repeated_reified_extents_bind_only_equal_source_axes() {
        let (id, path) = builtin_scalar_named_kind(mech_core::hash_str("u8")).unwrap();
        let named = NamedKinds(BTreeMap::from([(id, path)]));
        let dimensions = [DimensionParameterDeclaration {
            id: DimensionParameterId::new(0),
            origin: DimensionParameterOrigin::Inferred,
            lifetime: DimensionLifetime::Activation,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: None,
        }];
        let target = ReifiedKind::from_closed_kind(
            &KindExpr::Matrix {
                element: Box::new(KindExpr::Named(id)),
                dimensions: vec![DimensionExpr::Parameter(DimensionParameterId::new(0)); 2]
                    .into_boxed_slice(),
            },
            &dimensions,
            &named,
        )
        .unwrap();
        let matrix = |rows: u64, columns: u64| {
            ValueCell::from_schema_data(
                SchemaBody::Matrix {
                    element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
                    dimensions: vec![
                        DimensionExpr::Constant(rows),
                        DimensionExpr::Constant(columns),
                    ]
                    .into_boxed_slice(),
                },
                ValueDataDraft::Matrix(
                    (0..rows * columns)
                        .map(|value| ValueDataDraft::U8(value as u8))
                        .collect(),
                ),
            )
            .unwrap()
        };
        convert_reified(matrix(2, 2), target.clone()).unwrap();
        assert!(convert_reified(matrix(2, 3), target).is_err());
    }

    #[test]
    fn shared_reified_collection_cardinality_rejects_unequal_sources() {
        let parameter = DimensionParameterDeclaration {
            id: DimensionParameterId::new(0),
            origin: DimensionParameterOrigin::Inferred,
            lifetime: DimensionLifetime::Activation,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: None,
        };
        let source = |left, right| {
            SchemaBody::Tuple(
                [left, right]
                    .map(|size| SchemaBody::Set {
                        element: Box::new(SchemaBody::Index),
                        cardinality: CardinalitySpec::Exact(DimensionExpr::Constant(size)),
                    })
                    .into(),
            )
        };
        let target = SchemaBody::Tuple(
            [0, 1]
                .map(|_| SchemaBody::Set {
                    element: Box::new(SchemaBody::Index),
                    cardinality: CardinalitySpec::Exact(DimensionExpr::Parameter(parameter.id)),
                })
                .into(),
        );
        let mut bindings = vec![None];
        assert!(
            bind_reified_target_dimensions(
                &source(2, 3),
                &target,
                &[parameter.clone()],
                &mut bindings,
            )
            .is_err()
        );
        let mut bindings = vec![None];
        bind_reified_target_dimensions(&source(2, 2), &target, &[parameter], &mut bindings)
            .unwrap();
        let substituted = substitute_reified_target(&target, &bindings).unwrap();
        assert_eq!(
            substituted,
            SchemaBody::Tuple(
                [0, 1]
                    .map(|_| SchemaBody::Set {
                        element: Box::new(SchemaBody::Index),
                        cardinality: CardinalitySpec::Exact(DimensionExpr::Constant(2)),
                    })
                    .into(),
            )
        );
    }

    #[test]
    fn optional_open_matrix_target_wraps_only_a_two_axis_source() {
        let (id, path) = builtin_scalar_named_kind(mech_core::hash_str("u8")).unwrap();
        let named = NamedKinds(BTreeMap::from([(id, path)]));
        let dimensions = [0, 1].map(|index| DimensionParameterDeclaration {
            id: DimensionParameterId::new(index),
            origin: DimensionParameterOrigin::Inferred,
            lifetime: DimensionLifetime::Activation,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: None,
        });
        let target = ReifiedKind::from_closed_kind(
            &KindExpr::Option(Box::new(KindExpr::Matrix {
                element: Box::new(KindExpr::Named(id)),
                dimensions: vec![
                    DimensionExpr::Parameter(DimensionParameterId::new(0)),
                    DimensionExpr::Parameter(DimensionParameterId::new(1)),
                ]
                .into_boxed_slice(),
            })),
            &dimensions,
            &named,
        )
        .unwrap();
        let source = ValueCell::from_schema_data(
            SchemaBody::Matrix {
                element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
                dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(2)].into(),
            },
            ValueDataDraft::Matrix(
                vec![ValueDataDraft::U8(1), ValueDataDraft::U8(2)].into_boxed_slice(),
            ),
        )
        .unwrap();
        let wrapped = convert_reified(source, target.clone()).unwrap();
        assert!(matches!(
            wrapped.closed_schema_body().unwrap(),
            SchemaBody::Option(_)
        ));
        let rank_one = SchemaBody::Matrix {
            element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
            dimensions: vec![DimensionExpr::Constant(2)].into(),
        };
        let target_body = SchemaBody::Option(Box::new(SchemaBody::Matrix {
            element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
            dimensions: vec![
                DimensionExpr::Parameter(DimensionParameterId::new(0)),
                DimensionExpr::Parameter(DimensionParameterId::new(1)),
            ]
            .into(),
        }));
        let mut bindings = vec![None; 2];
        assert!(
            bind_reified_target_dimensions(&rank_one, &target_body, &dimensions, &mut bindings,)
                .is_err()
        );
    }

    #[cfg(all(feature = "bool", feature = "string"))]
    #[test]
    fn bool_to_string_uses_the_checked_explicit_plan() {
        let output =
            convert_literal_cell(ValueCell::from_exact(true).unwrap(), &SchemaBody::String)
                .unwrap();
        assert!(matches!(
            output.snapshot().unwrap().data(),
            ValueData::String(value) if value.as_ref() == "true"
        ));
    }

    #[test]
    fn float_to_integer_truncates_and_range_checks() {
        let output = convert_literal_cell(
            ValueCell::from_exact(-12.9_f64).unwrap(),
            &SchemaBody::SignedInteger(IntegerWidth::W32),
        )
        .unwrap();
        assert!(matches!(
            output.snapshot().unwrap().data(),
            ValueData::I32(-12)
        ));

        let error = convert_literal_cell(
            ValueCell::from_exact(f64::INFINITY).unwrap(),
            &SchemaBody::SignedInteger(IntegerWidth::W32),
        )
        .unwrap_err();
        assert!(error.kind_message().contains("finite"));
    }

    #[test]
    fn integer_conversion_never_passes_through_f64() {
        let exact = 9_007_199_254_740_993_u64;
        let output = convert_literal_cell(
            ValueCell::from_exact(exact).unwrap(),
            &SchemaBody::UnsignedInteger(IntegerWidth::W128),
        )
        .unwrap();
        assert!(matches!(
            output.snapshot().unwrap().data(),
            ValueData::U128(value) if *value == u128::from(exact)
        ));
    }

    #[test]
    fn nonfinite_float_values_survive_lossless_float_and_complex_conversions() {
        for value in [f32::INFINITY, f32::NEG_INFINITY, f32::NAN] {
            let widened = execute_scalar_conversion(
                ValueDataDraft::F32(F32Bits::from_f32(value)),
                BuiltinScalarKind::F32,
                BuiltinScalarKind::F64,
            )
            .unwrap();
            let ValueDataDraft::F64(widened) = widened else {
                panic!("f32 to f64 must produce f64")
            };
            assert_eq!(widened.to_f64().is_nan(), value.is_nan());
            assert_eq!(widened.to_f64().is_infinite(), value.is_infinite());

            let complex = execute_scalar_conversion(
                ValueDataDraft::F32(F32Bits::from_f32(value)),
                BuiltinScalarKind::F32,
                BuiltinScalarKind::C32,
            )
            .unwrap();
            let ValueDataDraft::Complex32(complex) = complex else {
                panic!("f32 to c32 must produce c32")
            };
            assert_eq!(complex.real().to_f32().is_nan(), value.is_nan());
            assert_eq!(complex.real().to_f32().is_infinite(), value.is_infinite());
            assert_eq!(complex.imaginary().to_f32(), 0.0);
        }
    }

    #[test]
    fn finite_float_narrowing_rejects_only_overflow() {
        assert!(matches!(
            execute_scalar_conversion(
                ValueDataDraft::F64(F64Bits::from_f64(f64::MAX)),
                BuiltinScalarKind::F64,
                BuiltinScalarKind::F32,
            ),
            Err(ConversionExecutionError::ConversionOutOfRange)
        ));
        for value in [f64::INFINITY, f64::NEG_INFINITY, f64::NAN, -0.0] {
            let converted = execute_scalar_conversion(
                ValueDataDraft::F64(F64Bits::from_f64(value)),
                BuiltinScalarKind::F64,
                BuiltinScalarKind::F32,
            )
            .unwrap();
            let ValueDataDraft::F32(converted) = converted else {
                panic!("f64 to f32 must produce f32")
            };
            assert_eq!(converted.to_f32().is_nan(), value.is_nan());
            assert_eq!(converted.to_f32().is_infinite(), value.is_infinite());
            if value == 0.0 {
                assert!(converted.to_f32().is_sign_negative());
            }
        }
    }

    #[test]
    fn integer_and_float_cast_boundaries_never_wrap() {
        for (draft, source, target) in [
            (
                ValueDataDraft::I16(-1),
                BuiltinScalarKind::I16,
                BuiltinScalarKind::U8,
            ),
            (
                ValueDataDraft::I16(256),
                BuiltinScalarKind::I16,
                BuiltinScalarKind::U8,
            ),
            (
                ValueDataDraft::U128(u128::MAX),
                BuiltinScalarKind::U128,
                BuiltinScalarKind::I128,
            ),
        ] {
            assert!(matches!(
                execute_scalar_conversion(draft, source, target),
                Err(ConversionExecutionError::ConversionOutOfRange)
            ));
        }

        for (value, expected) in [(12.9, 12), (-12.9, -12), (-0.0, 0)] {
            let converted = execute_scalar_conversion(
                ValueDataDraft::F64(F64Bits::from_f64(value)),
                BuiltinScalarKind::F64,
                BuiltinScalarKind::I32,
            )
            .unwrap();
            assert!(matches!(converted, ValueDataDraft::I32(actual) if actual == expected));
        }
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(matches!(
                execute_scalar_conversion(
                    ValueDataDraft::F64(F64Bits::from_f64(value)),
                    BuiltinScalarKind::F64,
                    BuiltinScalarKind::I32,
                ),
                Err(ConversionExecutionError::ConversionNonFinite)
            ));
        }
    }

    #[cfg(feature = "complex")]
    #[test]
    fn complex_to_real_requires_an_exact_zero_imaginary_part() {
        let complex = |imaginary| {
            ValueDataDraft::Complex64(Complex64Bits::new(
                F64Bits::from_f64(7.5),
                F64Bits::from_f64(imaginary),
            ))
        };
        let converted = execute_scalar_conversion(
            complex(-0.0),
            BuiltinScalarKind::C64,
            BuiltinScalarKind::F64,
        )
        .unwrap();
        assert!(matches!(converted, ValueDataDraft::F64(value) if value.to_f64() == 7.5));
        assert!(matches!(
            execute_scalar_conversion(complex(1.0), BuiltinScalarKind::C64, BuiltinScalarKind::F64,),
            Err(ConversionExecutionError::ConversionImaginaryPartNonZero)
        ));
    }

    #[cfg(feature = "complex")]
    #[test]
    fn canonical_c32_executes_selected_fixed_conversions_and_recovers_atomically() {
        let c32 = |real: f32, imaginary: f32| {
            ValueCell::from_schema_data(
                SchemaBody::Complex(FloatWidth::W32),
                ValueDataDraft::Complex32(mech_core::snapshot::Complex32Bits::new(
                    F32Bits::from_f32(real),
                    F32Bits::from_f32(imaginary),
                )),
            )
            .unwrap()
        };

        let source = c32(1.5, 0.0);
        let target = SchemaBody::FloatingPoint(FloatWidth::W64);
        let source_type = source.resolved_type().unwrap();
        let target_type = ResolvedType::from_schema_body(&target, &[]).unwrap();
        let plan = plan_explicit_cast(&source_type, &target_type).unwrap();
        let output = execute_conversion_plan(&source, &target, &plan).unwrap();
        let conversion =
            planned_type_conversion_specialized(source.clone(), output.clone(), plan).unwrap();

        conversion.instance().solve_result().unwrap();
        assert!(matches!(
            output.snapshot().unwrap().data(),
            ValueData::F64(value) if value.to_f64() == 1.5
        ));
        let successful_version = output.published_version();

        source.replace(&c32(1.5, 2.0).snapshot().unwrap()).unwrap();
        assert_eq!(
            conversion
                .instance()
                .solve_result()
                .unwrap_err()
                .kind_name(),
            "ConversionImaginaryPartNonZero"
        );
        assert!(matches!(
            output.snapshot().unwrap().data(),
            ValueData::F64(value) if value.to_f64() == 1.5
        ));
        assert_eq!(output.published_version(), successful_version);

        source.replace(&c32(2.5, 0.0).snapshot().unwrap()).unwrap();
        conversion.instance().solve_result().unwrap();
        assert!(matches!(
            output.snapshot().unwrap().data(),
            ValueData::F64(value) if value.to_f64() == 2.5
        ));
        assert!(output.published_version() > successful_version);

        let complex_source = c32(1.5, 2.0);
        let complex_target = SchemaBody::Complex(FloatWidth::W64);
        let source_type = complex_source.resolved_type().unwrap();
        let target_type = ResolvedType::from_schema_body(&complex_target, &[]).unwrap();
        let plan = plan_implicit_conversion(&source_type, &target_type).unwrap();
        let complex_output =
            execute_conversion_plan(&complex_source, &complex_target, &plan).unwrap();
        let complex_conversion =
            planned_type_conversion_specialized(complex_source, complex_output.clone(), plan)
                .unwrap();
        complex_conversion.instance().solve_result().unwrap();
        assert!(matches!(
            complex_output.snapshot().unwrap().data(),
            ValueData::Complex64(value)
                if value.real().to_f64() == 1.5 && value.imaginary().to_f64() == 2.0
        ));
    }

    #[test]
    fn reactive_conversion_stages_success_and_keeps_failures_atomic() {
        let source = ValueCell::from_exact(12.9_f64).unwrap();
        let target = SchemaBody::SignedInteger(IntegerWidth::W32);
        let source_type = source.resolved_type().unwrap();
        let target_type = ResolvedType::from_schema_body(&target, &[]).unwrap();
        let plan = plan_explicit_cast(&source_type, &target_type).unwrap();
        let output = execute_conversion_plan(&source, &target, &plan).unwrap();
        let conversion =
            planned_type_conversion_specialized(source.clone(), output.clone(), plan).unwrap();

        source
            .replace(&ValueCell::from_exact(13.9_f64).unwrap().snapshot().unwrap())
            .unwrap();
        conversion.instance().solve_result().unwrap();
        assert!(matches!(
            output.snapshot().unwrap().data(),
            ValueData::I32(13)
        ));

        source
            .replace(
                &ValueCell::from_exact(f64::INFINITY)
                    .unwrap()
                    .snapshot()
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(
            conversion
                .instance()
                .solve_result()
                .unwrap_err()
                .kind_name(),
            "ConversionNonFinite"
        );
        assert!(matches!(
            output.snapshot().unwrap().data(),
            ValueData::I32(13)
        ));
    }

    #[cfg(all(feature = "matrix", feature = "u8", feature = "f64"))]
    #[test]
    fn managed_matrix_conversion_is_atomic_after_a_valid_prefix_and_recovers() {
        let matrix = |values: &[f64]| {
            let cells = values
                .iter()
                .map(|value| ValueCell::from_exact(*value).unwrap())
                .collect::<Vec<_>>();
            ValueCell::dynamic_matrix_from_cells(2, 3, &cells).unwrap()
        };
        let source = matrix(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let SchemaBody::Matrix { dimensions, .. } = source.closed_schema_body().unwrap() else {
            panic!("fixture must be matrix-backed")
        };
        let target = SchemaBody::Matrix {
            element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
            dimensions,
        };
        let source_type = source.resolved_type().unwrap();
        let KindExpr::Matrix { dimensions, .. } = source_type.kind() else {
            panic!("fixture must resolve to a matrix")
        };
        let target_type = ResolvedType::new(
            KindExpr::Matrix {
                element: Box::new(BuiltinScalarKind::U8.kind_expr()),
                dimensions: dimensions.clone(),
            },
            source_type
                .dimension_parameters()
                .to_vec()
                .into_boxed_slice(),
        )
        .unwrap();
        let plan = plan_explicit_cast(&source_type, &target_type).unwrap();
        let output = execute_conversion_plan(&source, &target, &plan).unwrap();
        let conversion =
            planned_type_conversion_specialized(source.clone(), output.clone(), plan).unwrap();

        let values = |cell: &ValueCell| {
            let snapshot = cell.snapshot().unwrap();
            let ValueData::Matrix(matrix) = snapshot.data() else {
                panic!("converted value must remain a matrix")
            };
            let mech_core::snapshot::SequenceView::U8(values) = matrix.elements() else {
                panic!("converted matrix must use u8 elements")
            };
            values.to_vec()
        };

        source
            .replace(
                &matrix(&[10.0, 11.0, 12.0, 13.0, 14.0, 15.0])
                    .snapshot()
                    .unwrap(),
            )
            .unwrap();
        conversion.instance().solve_result().unwrap();
        assert_eq!(values(&output), vec![10, 11, 12, 13, 14, 15]);
        let successful_version = output.published_version();

        // The final lane is out of range. The managed conversion writes only
        // its private candidate, so the valid prefix cannot become visible.
        source
            .replace(
                &matrix(&[20.0, 21.0, 22.0, 23.0, 24.0, 300.0])
                    .snapshot()
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(
            conversion
                .instance()
                .solve_result()
                .unwrap_err()
                .kind_name(),
            "ConversionOutOfRange"
        );
        assert_eq!(values(&output), vec![10, 11, 12, 13, 14, 15]);
        assert_eq!(output.published_version(), successful_version);

        source
            .replace(
                &matrix(&[30.0, 31.0, 32.0, 33.0, 34.0, 35.0])
                    .snapshot()
                    .unwrap(),
            )
            .unwrap();
        conversion.instance().solve_result().unwrap();
        assert_eq!(values(&output), vec![30, 31, 32, 33, 34, 35]);
        assert!(output.published_version() > successful_version);
    }

    #[cfg(feature = "matrix")]
    #[test]
    fn matrix_conversion_preserves_dimensions_and_element_order() {
        let source = ValueCell::dynamic_matrix_from_cells(
            1,
            3,
            &[
                ValueCell::from_exact(1.0_f32).unwrap(),
                ValueCell::from_exact(2.0_f32).unwrap(),
                ValueCell::from_exact(3.0_f32).unwrap(),
            ],
        )
        .unwrap();
        let SchemaBody::Matrix { dimensions, .. } = source.closed_schema_body().unwrap() else {
            panic!("fixture must be matrix-backed")
        };
        let target = SchemaBody::Matrix {
            element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
            dimensions,
        };
        let source_type = source.resolved_type().unwrap();
        let KindExpr::Matrix { dimensions, .. } = source_type.kind() else {
            panic!("fixture must resolve to a matrix")
        };
        let target_type = ResolvedType::new(
            KindExpr::Matrix {
                element: Box::new(BuiltinScalarKind::F64.kind_expr()),
                dimensions: dimensions.clone(),
            },
            source_type
                .dimension_parameters()
                .to_vec()
                .into_boxed_slice(),
        )
        .unwrap();
        let plan = plan_implicit_conversion(&source_type, &target_type).unwrap();
        let output = execute_conversion_plan(&source, &target, &plan).unwrap();
        assert_eq!(
            output.current_top_level_extents().unwrap().as_ref(),
            &[1, 3]
        );
        let snapshot = output.snapshot().unwrap();
        let ValueData::Matrix(matrix) = snapshot.data() else {
            panic!("converted value must remain a matrix")
        };
        let mech_core::snapshot::SequenceView::F64(values) = matrix.elements() else {
            panic!("converted matrix must use f64 elements")
        };
        assert_eq!(
            values
                .iter()
                .map(|value| value.to_f64())
                .collect::<Vec<_>>(),
            vec![1.0, 2.0, 3.0],
        );
    }

    #[cfg(all(feature = "matrix", feature = "string"))]
    #[test]
    fn numeric_to_string_conversion_plans_the_converted_payload() {
        let source = ValueCell::dynamic_matrix_from_cells(
            1,
            4,
            &[
                ValueCell::from_exact(f64::MAX).unwrap(),
                ValueCell::from_exact(f64::MIN).unwrap(),
                ValueCell::from_exact(f64::from_bits(1)).unwrap(),
                ValueCell::from_exact(-f64::from_bits(1)).unwrap(),
            ],
        )
        .unwrap();
        let SchemaBody::Matrix { dimensions, .. } = source.closed_schema_body().unwrap() else {
            panic!("fixture must be matrix-backed")
        };
        let target = SchemaBody::Matrix {
            element: Box::new(SchemaBody::String),
            dimensions,
        };
        let source_type = source.resolved_type().unwrap();
        let KindExpr::Matrix { dimensions, .. } = source_type.kind() else {
            panic!("fixture must resolve to a matrix")
        };
        let target_type = ResolvedType::new(
            KindExpr::Matrix {
                element: Box::new(BuiltinScalarKind::String.kind_expr()),
                dimensions: dimensions.clone(),
            },
            source_type
                .dimension_parameters()
                .to_vec()
                .into_boxed_slice(),
        )
        .unwrap();
        let plan = plan_explicit_cast(&source_type, &target_type).unwrap();
        let output = execute_conversion_plan(&source, &target, &plan).unwrap();
        let planned = prospective_conversion_output_footprint(&output, &source, &plan)
            .unwrap()
            .unwrap();
        let actual = output.current_memory_footprint().unwrap();
        assert!(planned.payload_bytes >= actual.payload_bytes);
        assert!(planned.encoded_bytes >= actual.encoded_bytes);
        assert!(planned.retained_nodes >= actual.retained_nodes);

        let conversion = planned_type_conversion_specialized(source, output, plan).unwrap();
        conversion.instance().solve_result().unwrap();
    }

    #[cfg(all(feature = "matrix", feature = "string"))]
    #[test]
    fn open_matrix_annotation_inherits_source_dimensions() {
        let source = ValueCell::dynamic_matrix_from_cells(
            1,
            3,
            &[
                ValueCell::from_exact(1.0_f64).unwrap(),
                ValueCell::from_exact(2.0_f64).unwrap(),
                ValueCell::from_exact(3.0_f64).unwrap(),
            ],
        )
        .unwrap();
        let target = SchemaBody::Matrix {
            element: Box::new(SchemaBody::String),
            dimensions: Box::new([]),
        };

        let output = convert_literal_cell(source, &target).unwrap();

        assert_eq!(
            output.current_top_level_extents().unwrap().as_ref(),
            &[1, 3]
        );
        let snapshot = output.snapshot().unwrap();
        let ValueData::Matrix(matrix) = snapshot.data() else {
            panic!("converted value must remain a matrix")
        };
        let mech_core::snapshot::SequenceView::String(values) = matrix.elements() else {
            panic!("converted matrix must use string elements")
        };
        assert_eq!(
            values.iter().map(|value| &**value).collect::<Vec<_>>(),
            vec!["1", "2", "3"],
        );
    }

    #[test]
    fn option_conversion_preserves_absence_and_converts_payloads() {
        let payload_plan = plan_implicit_conversion(
            &ResolvedType::new(BuiltinScalarKind::U8.kind_expr(), Box::new([])).unwrap(),
            &ResolvedType::new(BuiltinScalarKind::U16.kind_expr(), Box::new([])).unwrap(),
        )
        .unwrap();
        let step = ConversionStep::OptionPayload(Box::new(payload_plan));
        let absent = execute_conversion_draft(
            ValueDataDraft::Option(OptionDraft {
                present: false,
                value: None,
            }),
            &step,
        )
        .unwrap();
        assert!(matches!(
            absent,
            ValueDataDraft::Option(OptionDraft {
                present: false,
                value: None,
            })
        ));
        let present = execute_conversion_draft(
            ValueDataDraft::Option(OptionDraft {
                present: true,
                value: Some(Box::new(ValueDataDraft::U8(255))),
            }),
            &step,
        )
        .unwrap();
        assert!(matches!(
            present,
            ValueDataDraft::Option(OptionDraft {
                present: true,
                value: Some(value),
            }) if matches!(*value, ValueDataDraft::U16(255))
        ));
    }
}

#[cfg(feature = "math_neg")]
pub fn negated(num: &RealNumber, p: &InterpreterExecution<'_>) -> MResult<ValueCell> {
    let num_val = real(&num, p)?;
    let snapshot = num_val.snapshot()?;
    match snapshot.data() {
        #[cfg(feature = "i8")]
        ValueData::I8(value) => ValueCell::from_exact(-*value),
        #[cfg(feature = "i16")]
        ValueData::I16(value) => ValueCell::from_exact(-*value),
        #[cfg(feature = "i32")]
        ValueData::I32(value) => ValueCell::from_exact(-*value),
        #[cfg(feature = "i64")]
        ValueData::I64(value) => ValueCell::from_exact(-*value),
        #[cfg(feature = "i128")]
        ValueData::I128(value) => ValueCell::from_exact(-*value),
        #[cfg(feature = "f64")]
        ValueData::F64(value) => ValueCell::from_exact(-value.to_f64()),
        #[cfg(feature = "f32")]
        ValueData::F32(value) => ValueCell::from_exact(-value.to_f32()),
        _ => Err(MechError::new(ExpectedNumericForKindSizeError, None).with_compiler_loc()),
    }
}

#[cfg(feature = "complex")]
fn cell_f64(cell: &ValueCell) -> MResult<Option<f64>> {
    Ok(match cell.snapshot()?.data() {
        ValueData::F64(value) => Some(value.to_f64()),
        _ => None,
    })
}

#[cfg(feature = "rational")]
pub fn rational(rat: &(Token, Token)) -> MResult<ValueCell> {
    let (num, denom) = rat;
    let num = num.chars.iter().collect::<String>().parse::<i64>().unwrap();
    let denom = denom
        .chars
        .iter()
        .collect::<String>()
        .parse::<i64>()
        .unwrap();
    if denom == 0 {
        panic!("Denominator cannot be zero in a rational number");
    }
    let rat_num = R64::new(num, denom);
    ValueCell::from_exact(rat_num)
}

#[cfg(feature = "i64")]
pub fn dec(bnry: &Token) -> MResult<ValueCell> {
    let binary_str: String = bnry.chars.iter().collect();
    let num = i64::from_str_radix(&binary_str, 10).unwrap();
    ValueCell::from_exact(num)
}

#[cfg(feature = "i64")]
pub fn binary(bnry: &Token) -> MResult<ValueCell> {
    let binary_str: String = bnry.chars.iter().collect();
    let num = i64::from_str_radix(&binary_str, 2).unwrap();
    ValueCell::from_exact(num)
}

#[cfg(feature = "i64")]
pub fn oct(octl: &Token) -> MResult<ValueCell> {
    let hex_str: String = octl.chars.iter().collect();
    let num = i64::from_str_radix(&hex_str, 8).unwrap();
    ValueCell::from_exact(num)
}

#[cfg(feature = "i64")]
pub fn hex(hxdcml: &Token) -> MResult<ValueCell> {
    let hex_str: String = hxdcml.chars.iter().collect();
    let num = i64::from_str_radix(&hex_str, 16).unwrap();
    ValueCell::from_exact(num)
}

#[cfg(feature = "f64")]
pub fn scientific(sci: &(Base, Exponent)) -> MResult<ValueCell> {
    let (base, exp): &(Base, Exponent) = sci;
    let (whole, part): &(Whole, Part) = base;
    let (sign, exp_whole, exp_part): &(Sign, Whole, Part) = exp;

    let a = whole.chars.iter().collect::<String>();
    let b = part.chars.iter().collect::<String>();
    let c = exp_whole.chars.iter().collect::<String>();
    let d = exp_part.chars.iter().collect::<String>();
    let num_f64: f64 = format!("{}.{}", a, b).parse::<f64>().unwrap();
    let mut exp_f64: f64 = format!("{}.{}", c, d).parse::<f64>().unwrap();
    if *sign {
        exp_f64 = -exp_f64;
    }
    let num = num_f64 * 10f64.powf(exp_f64);
    ValueCell::from_exact(num)
}

#[cfg(feature = "floats")]
pub fn float(flt: &(Token, Token)) -> MResult<ValueCell> {
    let a = flt.0.chars.iter().collect::<String>();
    let b = flt.1.chars.iter().collect::<String>();
    let num: f64 = format!("{}.{}", a, b).parse::<f64>().unwrap();
    ValueCell::from_exact(num)
}

#[cfg(feature = "f64")]
pub fn integer(int: &Token) -> MResult<ValueCell> {
    let num: f64 = int.chars.iter().collect::<String>().parse::<f64>().unwrap();
    ValueCell::from_exact(num)
}

#[cfg(feature = "string")]
pub fn string(tkn: &MechString) -> MResult<ValueCell> {
    let strng: String = tkn.text.chars.iter().collect::<String>();
    ValueCell::from_exact(strng)
}

pub fn empty() -> ValueCell {
    ValueCell::unit()
}

#[cfg(feature = "bool")]
pub fn boolean(tkn: &Token) -> MResult<ValueCell> {
    let val = match tkn.kind {
        TokenKind::True => true,
        TokenKind::False => false,
        _ => unreachable!(),
    };
    ValueCell::from_exact(val)
}

#[derive(Debug, Clone)]
pub struct ExpectedNumericForKindSizeError;
impl MechErrorKind for ExpectedNumericForKindSizeError {
    fn name(&self) -> &str {
        "ExpectedNumericForKindSize"
    }
    fn message(&self) -> String {
        "Expected a numeric value for kind size, but received a non-numeric value.".to_string()
    }
}
