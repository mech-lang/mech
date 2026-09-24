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
    let mut dimensions = DimensionEnvironmentBuilder::new();
    let kind = canonical_kind_annotation(knd, p, &mut named, &mut dimensions)?;
    let reified = ReifiedKind::from_closed_kind(&kind, dimensions.declarations(), &named).map_err(
        |error| MechError::new(ValueCellSnapshotFailure { error }, None).with_compiler_loc(),
    )?;
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
    dimensions: &mut DimensionEnvironmentBuilder,
) -> MResult<KindExpr> {
    Ok(match knd {
        mech_core::nodes::Kind::Kind(inner) => KindExpr::TypeOf(Box::new(
            canonical_kind_annotation(inner, p, named, dimensions)?,
        )),
        mech_core::nodes::Kind::Any => KindExpr::Wildcard,
        mech_core::nodes::Kind::Atom(identifier) => {
            let path = source_nominal_path(&identifier.to_string())?;
            KindExpr::Atom(NominalKey::from_path(NominalKind::Atom, &path))
        }
        mech_core::nodes::Kind::Empty => KindExpr::Never,
        mech_core::nodes::Kind::Record(fields) => KindExpr::Record(
            fields
                .iter()
                .map(|(name, kind)| {
                    Ok(KindField {
                        name: name.to_string(),
                        kind: canonical_kind_annotation(kind, p, named, dimensions)?,
                    })
                })
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
        ),
        mech_core::nodes::Kind::Tuple(elements) => KindExpr::Tuple(
            elements
                .iter()
                .map(|element| canonical_kind_annotation(element, p, named, dimensions))
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
        ),
        mech_core::nodes::Kind::Map(key, value) => KindExpr::Map {
            key: Box::new(canonical_kind_annotation(key, p, named, dimensions)?),
            value: Box::new(canonical_kind_annotation(value, p, named, dimensions)?),
            cardinality: inferred_interpreter_kind_dimension(dimensions)?,
        },
        mech_core::nodes::Kind::Scalar(identifier) => {
            let name = identifier.to_string();
            let scalar_id = identifier.hash();
            if name == "id" {
                KindExpr::Id
            } else if name == "ix" || name == "index" {
                KindExpr::Index
            } else if let Ok((id, path)) = builtin_scalar_named_kind(scalar_id) {
                named.0.insert(id, path);
                KindExpr::Named(id)
            } else if p.state.borrow().enums.contains_key(&scalar_id) {
                let path = source_nominal_path(&name)?;
                KindExpr::Enum(NominalKey::from_path(NominalKind::Enum, &path))
            } else {
                return Err(SemanticModelError::BuiltinScalarKindUnresolved { scalar_id }.into());
            }
        }
        mech_core::nodes::Kind::Matrix((element, dimension_nodes)) => {
            let mut extents = dimension_nodes
                .iter()
                .map(|dimension| {
                    literal_usize(dimension, p).and_then(|value| {
                        value.map_or_else(
                            || inferred_interpreter_kind_dimension(dimensions),
                            |value| Ok(DimensionExpr::Constant(value as u64)),
                        )
                    })
                })
                .collect::<MResult<Vec<_>>>()?;
            if extents.is_empty() {
                extents.push(inferred_interpreter_kind_dimension(dimensions)?);
                extents.push(inferred_interpreter_kind_dimension(dimensions)?);
            }
            KindExpr::Matrix {
                element: Box::new(canonical_kind_annotation(element, p, named, dimensions)?),
                dimensions: extents.into_boxed_slice(),
            }
        }
        mech_core::nodes::Kind::Option(element) => KindExpr::Option(Box::new(
            canonical_kind_annotation(element, p, named, dimensions)?,
        )),
        mech_core::nodes::Kind::Table((columns, rows)) => KindExpr::Table {
            columns: columns
                .iter()
                .map(|(name, kind)| {
                    Ok(KindField {
                        name: name.to_string(),
                        kind: canonical_kind_annotation(kind, p, named, dimensions)?,
                    })
                })
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
            rows: literal_usize(rows, p)?.map_or_else(
                || inferred_interpreter_kind_dimension(dimensions),
                |value| Ok(DimensionExpr::Constant(value as u64)),
            )?,
        },
        mech_core::nodes::Kind::Set(element, cardinality) => KindExpr::Set {
            element: Box::new(canonical_kind_annotation(element, p, named, dimensions)?),
            cardinality: cardinality
                .as_ref()
                .map(|value| literal_usize(value, p))
                .transpose()?
                .flatten()
                .map_or_else(
                    || inferred_interpreter_kind_dimension(dimensions),
                    |value| Ok(DimensionExpr::Constant(value as u64)),
                )?,
        },
    })
}

#[cfg(feature = "kind_annotation")]
fn inferred_interpreter_kind_dimension(
    dimensions: &mut DimensionEnvironmentBuilder,
) -> MResult<DimensionExpr> {
    dimensions
        .declare(
            DimensionParameterOrigin::Inferred,
            DimensionLifetime::Activation,
            DimensionExpr::Constant(0),
            None,
        )
        .map(DimensionExpr::Parameter)
        .map_err(MechError::from)
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
    materialize_conversion_semantic_shape(source, target, false)
}

#[cfg(feature = "convert")]
fn materialize_conversion_semantic_shape(
    source: &KindExpr,
    target: &SchemaBody,
    preserve_source_axes: bool,
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
            element: Box::new(materialize_conversion_semantic_shape(
                source_element,
                target_element,
                preserve_source_axes,
            )),
            dimensions: if preserve_source_axes || target_dimensions.is_empty() {
                source_dimensions.clone()
            } else {
                target_dimensions.clone()
            },
        },
        (KindExpr::Option(source), SchemaBody::Option(target)) => SchemaBody::Option(Box::new(
            materialize_conversion_semantic_shape(source, target, preserve_source_axes),
        )),
        (source, SchemaBody::Option(target)) => SchemaBody::Option(Box::new(
            materialize_conversion_semantic_shape(source, target, preserve_source_axes),
        )),
        (KindExpr::Tuple(source), SchemaBody::Tuple(target)) if source.len() == target.len() => {
            SchemaBody::Tuple(
                source
                    .iter()
                    .zip(target.iter())
                    .map(|(source, target)| {
                        materialize_conversion_semantic_shape(source, target, preserve_source_axes)
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
                        schema: materialize_conversion_semantic_shape(
                            &source.kind,
                            &target.schema,
                            preserve_source_axes,
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
            element: Box::new(materialize_conversion_semantic_shape(
                source,
                target,
                preserve_source_axes,
            )),
            cardinality: if preserve_source_axes
                || matches!(cardinality, CardinalitySpec::Dynamic { upper_bound: None })
            {
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
            key: Box::new(materialize_conversion_semantic_shape(
                source_key,
                key,
                preserve_source_axes,
            )),
            value: Box::new(materialize_conversion_semantic_shape(
                source_value,
                value,
                preserve_source_axes,
            )),
            cardinality: if preserve_source_axes
                || matches!(cardinality, CardinalitySpec::Dynamic { upper_bound: None })
            {
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
                        schema: materialize_conversion_semantic_shape(
                            &source.kind,
                            &target.schema,
                            preserve_source_axes,
                        ),
                    })
                    .collect(),
                rows: if preserve_source_axes
                    || matches!(rows, CardinalitySpec::Dynamic { upper_bound: None })
                {
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
    reified_target: Option<ValueCell>,
}

#[cfg(feature = "convert")]
#[derive(Debug)]
struct ReifiedTargetConstraints {
    target: SchemaBody,
    declared_target: SchemaBody,
    declarations: Box<[DimensionParameterDeclaration]>,
    activation_witnesses: Box<[(DimensionParameterId, Option<DimensionExpr>)]>,
}

#[cfg(feature = "convert")]
impl ReifiedTargetConstraints {
    fn activation_witnesses(
        declarations: &[DimensionParameterDeclaration],
        bindings: &[Option<DimensionExpr>],
    ) -> Box<[(DimensionParameterId, Option<DimensionExpr>)]> {
        declarations
            .iter()
            .filter(|declaration| declaration.lifetime == DimensionLifetime::Activation)
            .map(|declaration| {
                (
                    declaration.id,
                    bindings[declaration.id.get() as usize].clone(),
                )
            })
            .collect()
    }

    fn validate_activation_witnesses(&self, initial: &Self) -> MResult<()> {
        if self.activation_witnesses != initial.activation_witnesses {
            return Err(invalid_reified_conversion_target(
                "activation target dimension witness changed",
            ));
        }
        Ok(())
    }

    fn resolved_target(&self, source: &ValueCell) -> MResult<SchemaBody> {
        let bindings = solve_reified_target_bindings_with_declared(
            &source.closed_schema_body()?,
            &self.target,
            &self.declared_target,
            &self.declarations,
        )?;
        validate_reified_parameter_bindings(&self.declarations, &bindings)?;
        substitute_reified_target(&self.target, Some(&self.declared_target), &bindings)
    }

    fn validate(&self, source: &ValueCell) -> MResult<()> {
        self.resolved_target(source).map(|_| ())
    }
}

#[cfg(feature = "convert")]
fn planned_type_conversion_instance(
    source: ValueCell,
    output: ValueCell,
    plan: ConversionPlan,
    reified_constraints: Option<ReifiedTargetConstraints>,
    reified_target: Option<ValueCell>,
) -> (Box<dyn MechFunction>, FunctionInvocation) {
    let invocation = match &reified_target {
        Some(target) => FunctionInvocation::binary(output.clone(), source.clone(), target.clone()),
        None => FunctionInvocation::unary(output.clone(), source.clone()),
    };
    (
        Box::new(PlannedTypeConversion {
            source: source.clone(),
            output: output.clone(),
            plan,
            reified_constraints,
            reified_target,
        }),
        invocation,
    )
}

#[cfg(feature = "convert")]
fn planned_type_conversion_specialized(
    source: ValueCell,
    output: ValueCell,
    plan: ConversionPlan,
) -> MResult<SpecializedFunction> {
    let instance = planned_type_conversion_instance(source, output, plan, None, None);
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
#[derive(Debug)]
pub struct RuntimeReifiedKindConversion {
    source: FunctionValueInput,
    target: FunctionValueInput,
    output: FunctionValueOutput,
    plan: ConversionPlan,
    reified_constraints: ReifiedTargetConstraints,
}

#[cfg(feature = "convert")]
fn runtime_reified_constraints(
    source: &ValueCell,
    target_value: &mech_core::Value,
    expected_output: &SchemaBody,
) -> MResult<ReifiedTargetConstraints> {
    let schemas = source.retained_schema_table();
    let source_body = source.closed_schema_body()?;
    let (mut target, declarations, close_enum_payloads) = match target_value.data() {
        ValueData::Type(ReifiedType::Kind(kind)) => {
            let (target, declarations) = schema_body_from_reified_kind(kind, &schemas)?;
            (target, declarations, true)
        }
        ValueData::Type(ReifiedType::Schema(key)) => {
            let target_schemas = target_value.schemas();
            let schema = target_schemas
                .as_ref()
                .and_then(|table| table.find_by_key(*key).and_then(|id| table.get(id)))
                .or_else(|| schemas.find_by_key(*key).and_then(|id| schemas.get(id)))
                .ok_or_else(|| invalid_reified_conversion_target("unknown target schema key"))?;
            (
                schema.body().clone(),
                schema_target_declarations(schema),
                false,
            )
        }
        _ => {
            return Err(invalid_reified_conversion_target(
                "compiled reified conversion requires a type target",
            ));
        }
    };
    if close_enum_payloads {
        validate_reified_schema_materialization_size(&source_body)?;
        close_reified_enum_targets(&source_body, &mut target);
    }
    let declared_target = target.clone();
    inherit_reified_dynamic_cardinality(&source_body, &mut target, &declarations)?;
    let bindings = solve_reified_target_bindings_with_declared(
        &source_body,
        &target,
        &declared_target,
        &declarations,
    )?;
    validate_reified_parameter_bindings(&declarations, &bindings)?;
    let resolved = substitute_reified_target(&target, Some(&declared_target), &bindings)?;
    let expected = materialize_declared_conversion_shape(&source_body, &resolved);
    if expected_output != &expected {
        return Err(invalid_reified_conversion_target(
            "compiled reified conversion output differs from target kind",
        ));
    }
    let constraints = ReifiedTargetConstraints {
        target,
        declared_target,
        activation_witnesses: ReifiedTargetConstraints::activation_witnesses(
            &declarations,
            &bindings,
        ),
        declarations,
    };
    Ok(constraints)
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
impl MechFunctionFactory for RuntimeReifiedKindConversion {
    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::CanonicalFinalize
    }

    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
        FunctionValueRepresentation::AnyValue,
        FunctionValueRepresentation::AnyValue,
        FunctionValueRepresentation::AnyValue,
    );

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (output, source, target) = invocation.expect_binary()?;
        let output = output.value();
        let source = source.value();
        let target = target.value();
        let plan = runtime_kind_conversion_plan(output.cell(), source.cell())?;
        let reified_constraints = runtime_reified_constraints(
            source.cell(),
            &target.cell().snapshot()?,
            &output.cell().closed_schema_body()?,
        )?;
        Ok(Box::new(Self {
            source,
            target,
            output,
            plan,
            reified_constraints,
        }))
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&CHECKED_TYPE_CONVERSION_CONTRACT)
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

#[cfg(feature = "convert")]
impl MechFunctionImpl for RuntimeReifiedKindConversion {
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
        let source_schema = self.source.cell().closed_schema_body()?;
        let expected = conversion_target_schema(&source_schema, &self.plan.step)
            .map_err(conversion_execution_error)?;
        let target = frame.snapshot_input_cell(self.target.cell(), 1)?;
        let constraints = runtime_reified_constraints(self.source.cell(), &target, &expected)?;
        constraints.validate_activation_witnesses(&self.reified_constraints)?;
        stage_conversion_output(frame, self.source.cell(), self.output.cell(), &self.plan)?;
        Ok(mech_core::ReactiveSolveStatus::Changed)
    }

    fn semantic_operation_name(&self) -> Option<&str> {
        Some("convert/kind/reified")
    }

    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&CHECKED_TYPE_CONVERSION_CONTRACT)
    }

    fn to_string(&self) -> String {
        "RuntimeReifiedKindConversion".to_owned()
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

#[cfg(all(feature = "convert", feature = "semantic-compiler"))]
impl MechFunctionCompiler for RuntimeReifiedKindConversion {
    fn compiler_owned_value_cells(&self) -> Vec<ValueCell> {
        vec![
            self.source.cell().clone(),
            self.target.cell().clone(),
            self.output.cell().clone(),
        ]
    }

    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let destination = self.output.compile_register(context)?;
        let source = self.source.compile_register(context)?;
        let target = self.target.compile_register(context)?;
        let function = context.function_id("convert/kind/reified")?;
        context.emit_binop(function, destination, source, target);
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

#[cfg(feature = "convert")]
fn validate_runtime_reified_kind_conversion(
    output: &ValueCell,
    inputs: &[ValueCell],
) -> MResult<()> {
    let [source, target] = inputs else {
        return Err(function_shape_contract_violation(
            "type_conversion_reified",
            format!("expected source and target inputs, found {}", inputs.len()),
        ));
    };
    runtime_kind_conversion_plan(output, source)?;
    runtime_reified_constraints(source, &target.snapshot()?, &output.closed_schema_body()?)
        .map(|_| ())
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

mech_core::declare_native_runtime_factory! {
    cfg: all(feature = "convert", feature = "semantic-compiler"),
    registration: register_runtime_reified_kind_conversion,
    installer: install_runtime_reified_kind_conversion,
    name: "convert/kind/reified",
    factory_type: RuntimeReifiedKindConversion,
    contract: RuntimeFunctionContract::canonical_custom(
        "type_conversion_reified",
        RuntimeOutputAliasPolicy::DisallowInputAlias,
        validate_runtime_reified_kind_conversion,
    ),
    compiler_family: mech_core::RuntimeFamilyId::from_name("convert/kind/reified"),
    package: "mech-engine", crate_name: "mech_engine",
    installer_path: "mech_engine::__mech_native::install_runtime_reified_kind_conversion",
    extra_cargo_features: ["convert", "semantic-compiler"],
}

#[cfg(feature = "convert")]
pub(crate) static PURE_TYPE_CONVERSION_CONTRACT: std::sync::LazyLock<OperationContractDeclaration> =
    std::sync::LazyLock::new(|| {
        mech_core::maintained_operation_contract("convert/kind", 1, false)
            .expect("maintained operation contract")
    });

#[cfg(feature = "convert")]
static CHECKED_TYPE_CONVERSION_CONTRACT: std::sync::LazyLock<OperationContractDeclaration> =
    std::sync::LazyLock::new(|| {
        mech_core::maintained_operation_contract("convert/kind", 2, false)
            .expect("maintained reified conversion contract")
    });

#[cfg(feature = "convert")]
fn close_reified_enum_targets(source: &SchemaBody, target: &mut SchemaBody) {
    match (source, target) {
        (
            SchemaBody::Enum {
                key: source_key,
                variants: source_variants,
            },
            SchemaBody::Enum {
                key: target_key,
                variants: target_variants,
            },
        ) if source_key == target_key => *target_variants = source_variants.clone(),
        (SchemaBody::Option(source), SchemaBody::Option(target)) => {
            close_reified_enum_targets(source, target)
        }
        (source, SchemaBody::Option(target)) => close_reified_enum_targets(source, target),
        (
            SchemaBody::Matrix {
                element: source, ..
            },
            SchemaBody::Matrix {
                element: target, ..
            },
        )
        | (
            SchemaBody::Set {
                element: source, ..
            },
            SchemaBody::Set {
                element: target, ..
            },
        ) => {
            close_reified_enum_targets(source, target);
        }
        (SchemaBody::Tuple(source), SchemaBody::Tuple(target)) => {
            for (source, target) in source.iter().zip(target.iter_mut()) {
                close_reified_enum_targets(source, target);
            }
        }
        (SchemaBody::Record(source), SchemaBody::Record(target)) => {
            for (source, target) in source.iter().zip(target.iter_mut()) {
                if source.name == target.name {
                    close_reified_enum_targets(&source.schema, &mut target.schema);
                }
            }
        }
        (
            SchemaBody::Map {
                key: source_key,
                value: source_value,
                ..
            },
            SchemaBody::Map {
                key: target_key,
                value: target_value,
                ..
            },
        ) => {
            close_reified_enum_targets(source_key, target_key);
            close_reified_enum_targets(source_value, target_value);
        }
        (
            SchemaBody::Table {
                columns: source, ..
            },
            SchemaBody::Table {
                columns: target, ..
            },
        ) => {
            for (source, target) in source.iter().zip(target.iter_mut()) {
                if source.name == target.name {
                    close_reified_enum_targets(&source.schema, &mut target.schema);
                }
            }
        }
        _ => {}
    }
}

#[cfg(feature = "convert")]
fn schema_target_declarations(schema: &mech_core::Schema) -> Box<[DimensionParameterDeclaration]> {
    schema
        .dimension_parameters()
        .iter()
        .enumerate()
        .map(|(index, parameter)| DimensionParameterDeclaration {
            id: DimensionParameterId::new(index as u32),
            origin: DimensionParameterOrigin::Inferred,
            lifetime: parameter.lifetime(),
            lower_bound: parameter.lower_bound().clone(),
            upper_bound: parameter.upper_bound().cloned(),
        })
        .collect()
}

#[cfg(feature = "convert")]
const MAX_REIFIED_TARGET_MATERIALIZATION_NODES: usize = 65_536;

#[cfg(feature = "convert")]
fn reified_dimension_materialization_nodes(dimension: &DimensionExpr) -> usize {
    match dimension {
        DimensionExpr::Add(children)
        | DimensionExpr::Multiply(children)
        | DimensionExpr::Min(children)
        | DimensionExpr::Max(children) => children.iter().fold(1, |nodes, child| {
            nodes.saturating_add(reified_dimension_materialization_nodes(child))
        }),
        DimensionExpr::Hole | DimensionExpr::Constant(_) | DimensionExpr::Parameter(_) => 1,
    }
}

#[cfg(feature = "convert")]
fn reified_cardinality_materialization_nodes(cardinality: &CardinalitySpec) -> usize {
    match cardinality {
        CardinalitySpec::Exact(dimension) => reified_dimension_materialization_nodes(dimension),
        CardinalitySpec::Dynamic { upper_bound } => upper_bound
            .as_ref()
            .map(reified_dimension_materialization_nodes)
            .unwrap_or(1),
    }
}

#[cfg(feature = "convert")]
fn reified_schema_materialization_nodes(body: &SchemaBody) -> usize {
    match body {
        SchemaBody::Matrix {
            element,
            dimensions,
        } => dimensions.iter().fold(
            1usize.saturating_add(reified_schema_materialization_nodes(element)),
            |nodes, dimension| {
                nodes.saturating_add(reified_dimension_materialization_nodes(dimension))
            },
        ),
        SchemaBody::Option(element) => {
            1usize.saturating_add(reified_schema_materialization_nodes(element))
        }
        SchemaBody::Tuple(elements) => elements.iter().fold(1usize, |nodes, element| {
            nodes.saturating_add(reified_schema_materialization_nodes(element))
        }),
        SchemaBody::Record(fields) => fields.iter().fold(1usize, |nodes, field| {
            nodes
                .saturating_add(field.name.len())
                .saturating_add(reified_schema_materialization_nodes(&field.schema))
        }),
        SchemaBody::Enum { variants, .. } => variants.iter().fold(1usize, |nodes, variant| {
            nodes.saturating_add(variant.name.len()).saturating_add(
                variant
                    .payload
                    .as_ref()
                    .map(reified_schema_materialization_nodes)
                    .unwrap_or(1),
            )
        }),
        SchemaBody::Set {
            element,
            cardinality,
        } => 1usize
            .saturating_add(reified_cardinality_materialization_nodes(cardinality))
            .saturating_add(reified_schema_materialization_nodes(element)),
        SchemaBody::Map {
            key,
            value,
            cardinality,
        } => 1usize
            .saturating_add(reified_cardinality_materialization_nodes(cardinality))
            .saturating_add(reified_schema_materialization_nodes(key))
            .saturating_add(reified_schema_materialization_nodes(value)),
        SchemaBody::Table { columns, rows } => columns.iter().fold(
            1usize.saturating_add(reified_cardinality_materialization_nodes(rows)),
            |nodes, column| {
                nodes
                    .saturating_add(column.name.len())
                    .saturating_add(reified_schema_materialization_nodes(&column.schema))
            },
        ),
        _ => 1,
    }
}

#[cfg(feature = "convert")]
fn validate_reified_schema_materialization_size(body: &SchemaBody) -> MResult<()> {
    if reified_schema_materialization_nodes(body) > MAX_REIFIED_TARGET_MATERIALIZATION_NODES {
        Err(invalid_reified_conversion_target(
            "reified conversion target exceeds the materialization limit",
        ))
    } else {
        Ok(())
    }
}

#[cfg(feature = "convert")]
fn reified_kind_materialization_nodes(
    kind: &KindExpr,
    enums: &BTreeMap<NominalKey, &[EnumVariantSchema]>,
) -> Option<usize> {
    Some(match kind {
        KindExpr::Enum(key) => enums.get(key)?.iter().fold(1usize, |nodes, variant| {
            nodes.saturating_add(variant.name.len()).saturating_add(
                variant
                    .payload
                    .as_ref()
                    .map(reified_schema_materialization_nodes)
                    .unwrap_or(1),
            )
        }),
        KindExpr::Matrix {
            element,
            dimensions,
        } => dimensions.iter().fold(
            1usize.saturating_add(reified_kind_materialization_nodes(element, enums)?),
            |nodes, dimension| {
                nodes.saturating_add(reified_dimension_materialization_nodes(dimension))
            },
        ),
        KindExpr::Option(element) => {
            1usize.saturating_add(reified_kind_materialization_nodes(element, enums)?)
        }
        KindExpr::Tuple(elements) => elements.iter().try_fold(1usize, |nodes, element| {
            Some(nodes.saturating_add(reified_kind_materialization_nodes(element, enums)?))
        })?,
        KindExpr::Record(fields) => fields.iter().try_fold(1usize, |nodes, field| {
            Some(
                nodes
                    .saturating_add(field.name.len())
                    .saturating_add(reified_kind_materialization_nodes(&field.kind, enums)?),
            )
        })?,
        KindExpr::Table { columns, rows } => columns.iter().try_fold(
            1usize.saturating_add(reified_dimension_materialization_nodes(rows)),
            |nodes, column| {
                Some(
                    nodes
                        .saturating_add(column.name.len())
                        .saturating_add(reified_kind_materialization_nodes(&column.kind, enums)?),
                )
            },
        )?,
        KindExpr::Set {
            element,
            cardinality,
        } => 1usize
            .saturating_add(reified_dimension_materialization_nodes(cardinality))
            .saturating_add(reified_kind_materialization_nodes(element, enums)?),
        KindExpr::Map {
            key,
            value,
            cardinality,
        } => 1usize
            .saturating_add(reified_dimension_materialization_nodes(cardinality))
            .saturating_add(reified_kind_materialization_nodes(key, enums)?)
            .saturating_add(reified_kind_materialization_nodes(value, enums)?),
        _ => 1,
    })
}

#[cfg(feature = "convert")]
fn schema_body_from_reified_kind(
    value: &ReifiedKind,
    schemas: &mech_core::SchemaTable,
) -> MResult<(SchemaBody, Box<[DimensionParameterDeclaration]>)> {
    let (kind, dimensions, named) = value.decoded_closed_kind().map_err(|error| {
        MechError::new(ValueCellSnapshotFailure { error }, None).with_compiler_loc()
    })?;
    let mut enums = BTreeMap::<NominalKey, &[EnumVariantSchema]>::new();
    for entry in schemas.entries() {
        if let SchemaBody::Enum { key, variants } = entry.schema().body() {
            enums.entry(*key).or_insert(variants);
        }
    }
    let materialization_nodes = reified_kind_materialization_nodes(&kind, &enums)
        .ok_or_else(|| invalid_reified_conversion_target("unknown reified nominal enum"))?;
    if materialization_nodes > MAX_REIFIED_TARGET_MATERIALIZATION_NODES {
        return Err(invalid_reified_conversion_target(
            "reified conversion target exceeds the materialization limit",
        ));
    }

    fn schema(
        kind: &KindExpr,
        named: &BTreeMap<KindId, CanonicalNominalPath>,
        enums: &BTreeMap<NominalKey, &[EnumVariantSchema]>,
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
                let path = named.get(id).ok_or_else(aggregate_error)?;
                BuiltinScalarKind::ALL
                    .into_iter()
                    .find(|kind| {
                        kind.canonical_path()
                            .is_ok_and(|candidate| &candidate == path)
                    })
                    .map(BuiltinScalarKind::schema_body)
                    .ok_or_else(aggregate_error)?
            }
            KindExpr::Id => SchemaBody::Id,
            KindExpr::Index => SchemaBody::Index,
            KindExpr::Atom(key) => SchemaBody::Atom(*key),
            KindExpr::Enum(key) => {
                let variants = enums.get(key).ok_or_else(aggregate_error)?.to_vec().into();
                SchemaBody::Enum {
                    key: *key,
                    variants,
                }
            }
            KindExpr::Matrix {
                element,
                dimensions: extents,
            } => SchemaBody::Matrix {
                element: Box::new(schema(element, named, enums)?),
                dimensions: extents.clone(),
            },
            KindExpr::Option(element) => {
                SchemaBody::Option(Box::new(schema(element, named, enums)?))
            }
            KindExpr::Tuple(elements) => SchemaBody::Tuple(
                elements
                    .iter()
                    .map(|element| schema(element, named, enums))
                    .collect::<MResult<Vec<_>>>()?
                    .into_boxed_slice(),
            ),
            KindExpr::Record(fields) => SchemaBody::Record(
                fields
                    .iter()
                    .map(|field| {
                        Ok(SchemaField {
                            name: field.name.clone(),
                            schema: schema(&field.kind, named, enums)?,
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
                            schema: schema(&column.kind, named, enums)?,
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
                element: Box::new(schema(element, named, enums)?),
                cardinality: cardinality(extent),
            },
            KindExpr::Map {
                key,
                value,
                cardinality: extent,
            } => SchemaBody::Map {
                key: Box::new(schema(key, named, enums)?),
                value: Box::new(schema(value, named, enums)?),
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

    Ok((schema(&kind, &named, &enums)?, dimensions))
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
    selected: &mut Vec<DimensionParameterId>,
) -> MResult<()> {
    match dimension {
        DimensionExpr::Parameter(id) if bindings.get(id.get() as usize).is_none() => {
            return Err(invalid_reified_conversion_target(
                "unknown target dimension parameter",
            ));
        }
        DimensionExpr::Parameter(id) if bindings[id.get() as usize].is_none() => {
            if !selected.contains(id) {
                selected.push(*id);
            }
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
    let mut selected = Vec::new();
    unbound_reified_parameter(target, bindings, &mut selected)?;
    if selected.len() > 1 {
        // Other occurrences may bind these parameters independently. The
        // final pass checks this compound equation once they are known.
        return Ok(());
    }
    let Some(id) = selected.first().copied() else {
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
        if evaluate(low) != Some(extent) {
            return Err(invalid_reified_conversion_target(
                "compound target dimension has no source witness",
            ));
        }
        if low < upper && evaluate(low + 1) == Some(extent) {
            // A later occurrence can determine the shared parameter. Verify
            // this equation again after all witnesses have been collected.
            return Ok(());
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
const MAX_REIFIED_BINDING_PARAMETERS: usize = 128;
#[cfg(feature = "convert")]
const MAX_REIFIED_SOLVER_PARAMETERS: usize = 64;
#[cfg(feature = "convert")]
const MAX_REIFIED_SOLVER_EQUATIONS: usize = 128;
#[cfg(feature = "convert")]
const MAX_REIFIED_BINDING_VISITS: usize = 1_000_000;

#[cfg(feature = "convert")]
fn reified_binding_traversal_size(body: &SchemaBody) -> usize {
    fn dimension_size(dimension: &DimensionExpr) -> usize {
        match dimension {
            DimensionExpr::Add(children)
            | DimensionExpr::Multiply(children)
            | DimensionExpr::Min(children)
            | DimensionExpr::Max(children) => children.iter().fold(1usize, |size, child| {
                size.saturating_add(dimension_size(child))
            }),
            _ => 1,
        }
    }
    fn cardinality_size(cardinality: &CardinalitySpec) -> usize {
        match cardinality {
            CardinalitySpec::Exact(dimension) => dimension_size(dimension),
            CardinalitySpec::Dynamic { upper_bound } => {
                upper_bound.as_ref().map(dimension_size).unwrap_or(1)
            }
        }
    }
    match body {
        SchemaBody::Matrix {
            element,
            dimensions,
        } => dimensions.iter().fold(
            1usize.saturating_add(reified_binding_traversal_size(element)),
            |size, dimension| size.saturating_add(dimension_size(dimension)),
        ),
        SchemaBody::Option(inner) => 1usize.saturating_add(reified_binding_traversal_size(inner)),
        SchemaBody::Tuple(elements) => elements.iter().fold(1usize, |size, element| {
            size.saturating_add(reified_binding_traversal_size(element))
        }),
        SchemaBody::Record(fields) => fields.iter().fold(1usize, |size, field| {
            size.saturating_add(reified_binding_traversal_size(&field.schema))
        }),
        SchemaBody::Set {
            element,
            cardinality,
        } => 1usize
            .saturating_add(cardinality_size(cardinality))
            .saturating_add(reified_binding_traversal_size(element)),
        SchemaBody::Map {
            key,
            value,
            cardinality,
        } => 1usize
            .saturating_add(cardinality_size(cardinality))
            .saturating_add(reified_binding_traversal_size(key))
            .saturating_add(reified_binding_traversal_size(value)),
        SchemaBody::Table { columns, rows } => columns.iter().fold(
            1usize.saturating_add(cardinality_size(rows)),
            |size, column| size.saturating_add(reified_binding_traversal_size(&column.schema)),
        ),
        SchemaBody::Enum { variants, .. } => variants.iter().fold(1usize, |size, variant| {
            size.saturating_add(
                variant
                    .payload
                    .as_ref()
                    .map(reified_binding_traversal_size)
                    .unwrap_or(1),
            )
        }),
        _ => 1,
    }
}

#[cfg(feature = "convert")]
fn solve_reified_target_bindings_with_declared(
    source: &SchemaBody,
    target: &SchemaBody,
    declared_target: &SchemaBody,
    declarations: &[DimensionParameterDeclaration],
) -> MResult<Vec<Option<DimensionExpr>>> {
    // Both the exact-bound propagation and dimension-binding traversal can
    // revisit the whole target once per parameter. Limit that repeated work
    // before either loop runs; the joint equation limit applies only to axes
    // still unresolved after these direct-binding passes.
    if declarations.len() > MAX_REIFIED_BINDING_PARAMETERS {
        return Err(invalid_reified_conversion_target(
            "joint target dimension system exceeds the solver limit",
        ));
    }
    if reified_binding_traversal_size(target)
        .max(reified_binding_traversal_size(declared_target))
        .saturating_mul(declarations.len().saturating_add(1))
        > MAX_REIFIED_BINDING_VISITS
    {
        return Err(invalid_reified_conversion_target(
            "joint target dimension system exceeds the solver limit",
        ));
    }
    let mut bindings = vec![None; declarations.len()];
    // Exact bounds are witnesses even when their parameters occur only inside
    // a compound equation. Resolve dependencies between exact bounds first.
    for _ in 0..declarations.len() {
        let mut changed = false;
        for declaration in declarations {
            let index = declaration.id.get() as usize;
            if bindings.get(index).is_none() {
                return Err(invalid_reified_conversion_target(
                    "unknown target dimension parameter",
                ));
            }
            if bindings[index].is_some() || declaration.upper_bound.is_none() {
                continue;
            }
            if let Ok((lower, upper)) = reified_parameter_bounds(declaration, &bindings)
                && lower == upper
            {
                bindings[index] = Some(DimensionExpr::Constant(lower));
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    for _ in 0..=declarations.len() {
        let prior = bindings.clone();
        bind_reified_target_dimensions(source, declared_target, declarations, &mut bindings)?;
        for declaration in declarations {
            let index = declaration.id.get() as usize;
            if let (Some(witness), Some(upper)) = (
                bindings.get(index).and_then(Option::as_ref),
                declaration.upper_bound.as_ref(),
            ) && upper == &declaration.lower_bound
            {
                let witness = witness.clone();
                // An exact declaration is an equation. A body witness can
                // determine a dependency that occurs only in that bound.
                let _ = bind_reified_dimension(
                    &witness,
                    &declaration.lower_bound,
                    declarations,
                    &mut bindings,
                );
            }
        }
        if prior == bindings {
            break;
        }
    }
    if bindings.iter().any(Option::is_none) {
        solve_joint_reified_dimensions(source, declared_target, declarations, &mut bindings)?;
    }
    // With all available witnesses bound, every deferred compound equation
    // must match. Unresolved parameters remain an invalid ambiguous target.
    substitute_reified_target(target, None, &bindings)?;
    bind_reified_target_dimensions(source, declared_target, declarations, &mut bindings)?;
    Ok(bindings)
}

#[cfg(all(test, feature = "convert"))]
fn solve_reified_target_bindings(
    source: &SchemaBody,
    target: &SchemaBody,
    declarations: &[DimensionParameterDeclaration],
) -> MResult<Vec<Option<DimensionExpr>>> {
    solve_reified_target_bindings_with_declared(source, target, target, declarations)
}

#[cfg(feature = "convert")]
fn collect_reified_dimension_equations<'a>(
    source: &'a SchemaBody,
    target: &'a SchemaBody,
    equations: &mut Vec<(&'a DimensionExpr, &'a DimensionExpr)>,
) {
    match (source, target) {
        (
            SchemaBody::Matrix {
                element: source_element,
                dimensions: source_axes,
            },
            SchemaBody::Matrix {
                element: target_element,
                dimensions: target_axes,
            },
        ) => {
            equations.extend(source_axes.iter().zip(target_axes.iter()));
            collect_reified_dimension_equations(source_element, target_element, equations);
        }
        (SchemaBody::Option(source), SchemaBody::Option(target)) => {
            collect_reified_dimension_equations(source, target, equations);
        }
        (source, SchemaBody::Option(target)) => {
            collect_reified_dimension_equations(source, target, equations);
        }
        (SchemaBody::Tuple(source), SchemaBody::Tuple(target)) => {
            for (source, target) in source.iter().zip(target.iter()) {
                collect_reified_dimension_equations(source, target, equations);
            }
        }
        (SchemaBody::Record(source), SchemaBody::Record(target)) => {
            for (source, target) in source.iter().zip(target.iter()) {
                if source.name == target.name {
                    collect_reified_dimension_equations(&source.schema, &target.schema, equations);
                }
            }
        }
        (
            SchemaBody::Enum {
                key: source_key,
                variants: source,
            },
            SchemaBody::Enum {
                key: target_key,
                variants: target,
            },
        ) if source_key == target_key => {
            for (source, target) in source.iter().zip(target.iter()) {
                if source.name == target.name {
                    if let (Some(source), Some(target)) = (&source.payload, &target.payload) {
                        collect_reified_dimension_equations(source, target, equations);
                    }
                }
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
            if let Some((source, target)) =
                reified_cardinality_dimension_equation(source_cardinality, target_cardinality)
            {
                equations.push((source, target));
            }
            collect_reified_dimension_equations(source_element, target_element, equations);
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
            if let Some((source, target)) =
                reified_cardinality_dimension_equation(source_cardinality, target_cardinality)
            {
                equations.push((source, target));
            }
            collect_reified_dimension_equations(source_key, target_key, equations);
            collect_reified_dimension_equations(source_value, target_value, equations);
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
        ) => {
            if let Some((source, target)) =
                reified_cardinality_dimension_equation(source_rows, target_rows)
            {
                equations.push((source, target));
            }
            for (source, target) in source.iter().zip(target.iter()) {
                if source.name == target.name {
                    collect_reified_dimension_equations(&source.schema, &target.schema, equations);
                }
            }
        }
        _ => {}
    }
}

#[cfg(feature = "convert")]
fn reified_cardinality_dimension_equation<'a>(
    source: &'a CardinalitySpec,
    target: &'a CardinalitySpec,
) -> Option<(&'a DimensionExpr, &'a DimensionExpr)> {
    match (source, target) {
        (CardinalitySpec::Exact(source), CardinalitySpec::Exact(target))
        | (
            CardinalitySpec::Dynamic {
                upper_bound: Some(source),
            },
            CardinalitySpec::Dynamic {
                upper_bound: Some(target),
            },
        ) => Some((source, target)),
        _ => None,
    }
}

#[cfg(feature = "convert")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DimensionRatio {
    numerator: i128,
    denominator: i128,
}

#[cfg(feature = "convert")]
impl DimensionRatio {
    fn integer(value: i128) -> Self {
        Self {
            numerator: value,
            denominator: 1,
        }
    }

    fn new(numerator: i128, denominator: i128) -> Option<Self> {
        if denominator == 0 {
            return None;
        }
        let (numerator, denominator) = if denominator < 0 {
            (numerator.checked_neg()?, denominator.checked_neg()?)
        } else {
            (numerator, denominator)
        };
        let mut a = numerator.unsigned_abs();
        let mut b = denominator as u128;
        while b != 0 {
            (a, b) = (b, a % b);
        }
        let divisor = i128::try_from(a).ok()?;
        Some(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }

    fn multiply(self, other: Self) -> Option<Self> {
        Self::new(
            self.numerator.checked_mul(other.numerator)?,
            self.denominator.checked_mul(other.denominator)?,
        )
    }

    fn subtract(self, other: Self) -> Option<Self> {
        Self::new(
            self.numerator
                .checked_mul(other.denominator)?
                .checked_sub(other.numerator.checked_mul(self.denominator)?)?,
            self.denominator.checked_mul(other.denominator)?,
        )
    }

    fn divide(self, other: Self) -> Option<Self> {
        Self::new(
            self.numerator.checked_mul(other.denominator)?,
            self.denominator.checked_mul(other.numerator)?,
        )
    }
}

#[cfg(feature = "convert")]
fn affine_reified_dimension(
    dimension: &DimensionExpr,
    bindings: &[Option<DimensionExpr>],
    unknown: &[usize],
) -> Option<(i128, Vec<i128>)> {
    let zero = || vec![0; unknown.len()];
    match dimension {
        DimensionExpr::Constant(value) => Some((i128::from(*value), zero())),
        DimensionExpr::Parameter(id) => {
            let index = id.get() as usize;
            if let Some(bound) = bindings.get(index)?.as_ref() {
                Some((i128::from(reified_dimension_value(bound)?), zero()))
            } else {
                let mut coefficients = zero();
                coefficients[unknown.binary_search(&index).ok()?] = 1;
                Some((0, coefficients))
            }
        }
        DimensionExpr::Add(children) => {
            let mut constant = 0_i128;
            let mut coefficients = zero();
            for child in children {
                let (value, terms) = affine_reified_dimension(child, bindings, unknown)?;
                constant = constant.checked_add(value)?;
                for (coefficient, term) in coefficients.iter_mut().zip(terms) {
                    *coefficient = coefficient.checked_add(term)?;
                }
            }
            Some((constant, coefficients))
        }
        DimensionExpr::Multiply(children) => {
            let mut constant = 1_i128;
            let mut coefficients = zero();
            for child in children {
                let (value, terms) = affine_reified_dimension(child, bindings, unknown)?;
                if coefficients.iter().any(|coefficient| *coefficient != 0)
                    && terms.iter().any(|term| *term != 0)
                {
                    return None;
                }
                let next = coefficients
                    .iter()
                    .zip(&terms)
                    .map(|(coefficient, term)| {
                        coefficient
                            .checked_mul(value)?
                            .checked_add(term.checked_mul(constant)?)
                    })
                    .collect::<Option<Vec<_>>>()?;
                constant = constant.checked_mul(value)?;
                coefficients = next;
            }
            Some((constant, coefficients))
        }
        _ => None,
    }
}

#[cfg(feature = "convert")]
fn exact_reified_parameter_bound(
    declaration: &DimensionParameterDeclaration,
    bindings: &[Option<DimensionExpr>],
    unknown: &[usize],
) -> bool {
    let Some(upper) = declaration.upper_bound.as_ref() else {
        return false;
    };
    if upper == &declaration.lower_bound {
        return true;
    }
    match (
        affine_reified_dimension(&declaration.lower_bound, bindings, unknown),
        affine_reified_dimension(upper, bindings, unknown),
    ) {
        (Some(lower), Some(upper)) => lower == upper,
        _ => false,
    }
}

#[cfg(feature = "convert")]
fn joint_row_other_range(
    coefficients: &[i128],
    domains: &[(u64, u64)],
    except: usize,
) -> Option<(i128, i128)> {
    coefficients.iter().zip(domains).enumerate().try_fold(
        (0_i128, 0_i128),
        |(minimum, maximum), (index, (coefficient, (lower, upper)))| {
            if index == except {
                return Some((minimum, maximum));
            }
            let (low, high) = if *coefficient < 0 {
                (*upper, *lower)
            } else {
                (*lower, *upper)
            };
            Some((
                minimum.checked_add(coefficient.checked_mul(i128::from(low))?)?,
                maximum.checked_add(coefficient.checked_mul(i128::from(high))?)?,
            ))
        },
    )
}

#[cfg(feature = "convert")]
fn narrow_joint_reified_domains(
    equations: &[(Vec<i128>, i128)],
    domains: &mut [(u64, u64)],
) -> MResult<()> {
    // Bounds move inward only. A fixed iteration ceiling keeps dependent
    // constraints from consuming unbounded work before the search limit.
    for _ in 0..=domains.len().saturating_mul(2) {
        let mut changed = false;
        for (coefficients, target) in equations {
            for (index, coefficient) in coefficients.iter().enumerate() {
                if *coefficient == 0 {
                    continue;
                }
                let Some((other_minimum, other_maximum)) =
                    joint_row_other_range(coefficients, domains, index)
                else {
                    continue;
                };
                let (minimum, maximum, divisor) = if *coefficient > 0 {
                    (
                        target.checked_sub(other_maximum),
                        target.checked_sub(other_minimum),
                        Some(*coefficient),
                    )
                } else {
                    (
                        other_minimum.checked_sub(*target),
                        other_maximum.checked_sub(*target),
                        coefficient.checked_abs(),
                    )
                };
                let (Some(minimum), Some(maximum), Some(divisor)) = (minimum, maximum, divisor)
                else {
                    continue;
                };
                let lower =
                    minimum.div_euclid(divisor) + i128::from(minimum.rem_euclid(divisor) != 0);
                let upper = maximum.div_euclid(divisor);
                let lower = lower.max(0);
                let upper = upper.min(i128::from(u64::MAX));
                if lower > upper {
                    return Err(invalid_reified_conversion_target(
                        "joint target dimensions have no source witness",
                    ));
                }
                let domain = &mut domains[index];
                let next = (domain.0.max(lower as u64), domain.1.min(upper as u64));
                if next.0 > next.1 {
                    return Err(invalid_reified_conversion_target(
                        "joint target dimensions have no source witness",
                    ));
                }
                changed |= *domain != next;
                *domain = next;
            }
        }
        if !changed {
            break;
        }
    }
    Ok(())
}

#[cfg(feature = "convert")]
fn narrow_joint_reified_inequalities(
    inequalities: &[(Vec<i128>, i128)],
    domains: &mut [(u64, u64)],
) -> MResult<()> {
    for (coefficients, target) in inequalities {
        let Some((minimum, _)) = joint_row_other_range(coefficients, domains, usize::MAX) else {
            continue;
        };
        if minimum > *target {
            return Err(invalid_reified_conversion_target(
                "joint target dimensions have no source witness",
            ));
        }
        for (index, coefficient) in coefficients.iter().copied().enumerate() {
            if coefficient == 0 {
                continue;
            }
            let Some((other_minimum, _)) = joint_row_other_range(coefficients, domains, index)
            else {
                continue;
            };
            let domain = &mut domains[index];
            if coefficient > 0 {
                let Some(limit) = target.checked_sub(other_minimum) else {
                    continue;
                };
                let upper = limit.div_euclid(coefficient);
                if upper < 0 {
                    return Err(invalid_reified_conversion_target(
                        "joint target dimensions have no source witness",
                    ));
                }
                domain.1 = domain.1.min(upper.min(i128::from(u64::MAX)) as u64);
            } else {
                let (Some(numerator), Some(divisor)) = (
                    other_minimum.checked_sub(*target),
                    coefficient.checked_abs(),
                ) else {
                    continue;
                };
                let lower =
                    numerator.div_euclid(divisor) + i128::from(numerator.rem_euclid(divisor) != 0);
                if lower > i128::from(u64::MAX) {
                    return Err(invalid_reified_conversion_target(
                        "joint target dimensions have no source witness",
                    ));
                }
                domain.0 = domain.0.max(lower.max(0) as u64);
            }
            if domain.0 > domain.1 {
                return Err(invalid_reified_conversion_target(
                    "joint target dimensions have no source witness",
                ));
            }
        }
    }
    Ok(())
}

#[cfg(feature = "convert")]
fn reified_dimension_range(
    dimension: &DimensionExpr,
    bindings: &[Option<DimensionExpr>],
    unknown: &[usize],
    domains: &[(u64, u64)],
) -> Option<(u64, u64)> {
    match dimension {
        DimensionExpr::Constant(value) => Some((*value, *value)),
        DimensionExpr::Parameter(id) => {
            let index = id.get() as usize;
            if let Some(bound) = bindings.get(index)?.as_ref() {
                return reified_dimension_value(bound).map(|value| (value, value));
            }
            domains.get(unknown.binary_search(&index).ok()?).copied()
        }
        DimensionExpr::Add(children) => children.iter().try_fold((0_u64, 0_u64), |total, child| {
            let range = reified_dimension_range(child, bindings, unknown, domains)?;
            Some((
                total.0.saturating_add(range.0),
                total.1.saturating_add(range.1),
            ))
        }),
        DimensionExpr::Multiply(children) => {
            children.iter().try_fold((1_u64, 1_u64), |total, child| {
                let range = reified_dimension_range(child, bindings, unknown, domains)?;
                Some((
                    total.0.saturating_mul(range.0),
                    total.1.saturating_mul(range.1),
                ))
            })
        }
        DimensionExpr::Min(children) => {
            let mut ranges = children
                .iter()
                .map(|child| reified_dimension_range(child, bindings, unknown, domains));
            let first = ranges.next()??;
            ranges.try_fold(first, |total, range| {
                let range = range?;
                Some((total.0.min(range.0), total.1.min(range.1)))
            })
        }
        DimensionExpr::Max(children) => {
            let mut ranges = children
                .iter()
                .map(|child| reified_dimension_range(child, bindings, unknown, domains));
            let first = ranges.next()??;
            ranges.try_fold(first, |total, range| {
                let range = range?;
                Some((total.0.max(range.0), total.1.max(range.1)))
            })
        }
        DimensionExpr::Hole => None,
    }
}

#[cfg(feature = "convert")]
fn narrow_concrete_reified_equations(
    equations: &[(DimensionExpr, DimensionExpr)],
    bindings: &[Option<DimensionExpr>],
    unknown: &[usize],
    domains: &mut [(u64, u64)],
) -> bool {
    for (source, target) in equations {
        let (Some(source_range), Some(target_range)) = (
            reified_dimension_range(source, bindings, unknown, domains),
            reified_dimension_range(target, bindings, unknown, domains),
        ) else {
            continue;
        };
        if source_range.0 > target_range.1 || target_range.0 > source_range.1 {
            return false;
        }
        let (extent, extent_upper) = source_range;
        if extent != extent_upper {
            continue;
        }
        // Dimension operators are monotone over nonnegative extents. Even
        // with an unbounded parameter, the minimum target value with that
        // parameter fixed gives a sound upper limit. For example, q*r = 6
        // with q >= 3 and r >= 2 narrows both unbounded domains to 3 and 2.
        for position in 0..domains.len() {
            let (mut lower, mut upper) = domains[position];
            let minimum_at = |value| {
                let mut candidate = domains.to_vec();
                candidate[position] = (value, value);
                reified_dimension_range(target, bindings, unknown, &candidate).map(|range| range.0)
            };
            let Some(minimum) = minimum_at(lower) else {
                continue;
            };
            if minimum > extent {
                return false;
            }
            if minimum_at(upper).is_some_and(|minimum| minimum <= extent) {
                continue;
            }
            while lower < upper {
                let middle = lower + (upper - lower) / 2 + (upper - lower) % 2;
                if minimum_at(middle).is_some_and(|minimum| minimum <= extent) {
                    lower = middle;
                } else {
                    upper = middle - 1;
                }
            }
            domains[position].1 = lower;
        }
    }
    true
}

#[cfg(feature = "convert")]
fn solve_bounded_joint_reified_dimensions(
    equations: &[(Vec<i128>, i128)],
    inequalities: &[(Vec<i128>, i128)],
    concrete_equations: &[(DimensionExpr, DimensionExpr)],
    declarations: &[DimensionParameterDeclaration],
    bindings: &[Option<DimensionExpr>],
    unknown: &[usize],
) -> MResult<Option<Vec<u64>>> {
    const MAX_RANGE_SEARCH_STATES: usize = 65_536;
    let domains = unknown
        .iter()
        .map(|index| {
            let declaration = declarations
                .iter()
                .find(|declaration| declaration.id.get() as usize == *index)
                .ok_or_else(|| {
                    invalid_reified_conversion_target("unknown target dimension parameter")
                })?;
            let lower = substitute_reified_dimension(&declaration.lower_bound, bindings)
                .ok()
                .and_then(|bound| reified_dimension_value(&bound))
                .unwrap_or(0);
            let upper = declaration
                .upper_bound
                .as_ref()
                .and_then(|bound| substitute_reified_dimension(bound, bindings).ok())
                .and_then(|bound| reified_dimension_value(&bound))
                .unwrap_or(u64::MAX);
            if lower > upper {
                return Err(invalid_reified_conversion_target(
                    "target dimension bounds are inconsistent",
                ));
            }
            Ok((lower, upper))
        })
        .collect::<MResult<Vec<_>>>()?;
    fn search(
        equations: &[(Vec<i128>, i128)],
        inequalities: &[(Vec<i128>, i128)],
        concrete_equations: &[(DimensionExpr, DimensionExpr)],
        declarations: &[DimensionParameterDeclaration],
        bindings: &[Option<DimensionExpr>],
        unknown: &[usize],
        domains: Vec<(u64, u64)>,
        visited: &mut usize,
        solutions: &mut Vec<Vec<u64>>,
    ) -> MResult<()> {
        if solutions.len() >= 2 {
            return Ok(());
        }
        *visited += 1;
        if *visited > MAX_RANGE_SEARCH_STATES {
            return Err(invalid_reified_conversion_target(
                "joint target dimension range search exceeds the solver limit",
            ));
        }
        let mut domains = domains;
        for _ in 0..=unknown.len().saturating_mul(2) {
            let previous = domains.clone();
            if narrow_joint_reified_domains(equations, &mut domains).is_err() {
                return Ok(());
            }
            if narrow_joint_reified_inequalities(inequalities, &mut domains).is_err() {
                return Ok(());
            }
            if !narrow_concrete_reified_equations(
                concrete_equations,
                bindings,
                unknown,
                &mut domains,
            ) {
                return Ok(());
            }
            for (position, index) in unknown.iter().copied().enumerate() {
                let declaration = declarations
                    .get(index)
                    .filter(|declaration| declaration.id.get() as usize == index)
                    .ok_or_else(|| {
                        invalid_reified_conversion_target("unknown target dimension parameter")
                    })?;
                if let Some((lower, _)) =
                    reified_dimension_range(&declaration.lower_bound, bindings, unknown, &domains)
                {
                    domains[position].0 = domains[position].0.max(lower);
                }
                if let Some((_, upper)) = declaration
                    .upper_bound
                    .as_ref()
                    .and_then(|bound| reified_dimension_range(bound, bindings, unknown, &domains))
                {
                    domains[position].1 = domains[position].1.min(upper);
                }
                if domains[position].0 > domains[position].1 {
                    return Ok(());
                }
            }
            if domains == previous {
                break;
            }
        }
        if domains.iter().all(|(lower, upper)| lower == upper) {
            let values = domains.iter().map(|domain| domain.0).collect::<Vec<_>>();
            let equations_hold = equations.iter().all(|(coefficients, target)| {
                coefficients
                    .iter()
                    .zip(&values)
                    .try_fold(0_i128, |total, (coefficient, value)| {
                        total.checked_add(coefficient.checked_mul(i128::from(*value))?)
                    })
                    == Some(*target)
            });
            if equations_hold {
                let mut candidate = bindings.to_vec();
                for (index, value) in unknown.iter().zip(&values) {
                    candidate[*index] = Some(DimensionExpr::Constant(*value));
                }
                let concrete_equations_hold = concrete_equations.iter().all(|(source, target)| {
                    substitute_reified_dimension(source, &candidate)
                        .ok()
                        .and_then(|resolved| reified_dimension_value(&resolved))
                        .is_some_and(|source_value| {
                            substitute_reified_dimension(target, &candidate)
                                .ok()
                                .and_then(|resolved| reified_dimension_value(&resolved))
                                == Some(source_value)
                        })
                });
                if concrete_equations_hold
                    && validate_reified_parameter_bindings(declarations, &candidate).is_ok()
                {
                    solutions.push(values);
                }
            }
            return Ok(());
        }
        let Some((index, (lower, upper))) = domains
            .iter()
            .copied()
            .enumerate()
            .filter(|(_, (lower, upper))| lower < upper && *upper != u64::MAX)
            .min_by_key(|(_, (lower, upper))| upper - lower)
        else {
            return Ok(());
        };
        let mut value = lower;
        loop {
            let mut branch = domains.clone();
            branch[index] = (value, value);
            search(
                equations,
                inequalities,
                concrete_equations,
                declarations,
                bindings,
                unknown,
                branch,
                visited,
                solutions,
            )?;
            if solutions.len() >= 2 || value == upper {
                break;
            }
            value += 1;
        }
        Ok(())
    }

    let mut solutions = Vec::new();
    search(
        equations,
        inequalities,
        concrete_equations,
        declarations,
        bindings,
        unknown,
        domains,
        &mut 0,
        &mut solutions,
    )?;
    Ok((solutions.len() == 1).then(|| solutions.remove(0)))
}

#[cfg(feature = "convert")]
fn solve_joint_reified_dimensions(
    source: &SchemaBody,
    target: &SchemaBody,
    declarations: &[DimensionParameterDeclaration],
    bindings: &mut [Option<DimensionExpr>],
) -> MResult<()> {
    // The elimination below is dense. Bound it before constructing any rows
    // so a small sparse source cannot force quadratic allocation or cubic work.
    let unknown = bindings
        .iter()
        .enumerate()
        .filter_map(|(index, binding)| binding.is_none().then_some(index))
        .collect::<Vec<_>>();
    if unknown.is_empty() {
        return Ok(());
    }
    if unknown.len() > MAX_REIFIED_SOLVER_PARAMETERS {
        return Err(invalid_reified_conversion_target(
            "joint target dimension system exceeds the solver limit",
        ));
    }
    let mut collected = Vec::new();
    collect_reified_dimension_equations(source, target, &mut collected);
    let mut equations = Vec::new();
    for (source, target) in collected {
        if reified_dimension_value(source).is_none() {
            continue;
        }
        let mut unbound = Vec::new();
        unbound_reified_parameter(target, bindings, &mut unbound)?;
        if !unbound.is_empty() {
            equations.push((source, target));
        }
    }
    let exact_bounds = declarations
        .iter()
        .filter(|declaration| exact_reified_parameter_bound(declaration, bindings, &unknown))
        .collect::<Vec<_>>();
    if equations.len().saturating_add(exact_bounds.len()) > MAX_REIFIED_SOLVER_EQUATIONS {
        return Err(invalid_reified_conversion_target(
            "joint target dimension system exceeds the solver limit",
        ));
    }
    let mut rows = Vec::<Vec<DimensionRatio>>::new();
    let mut integer_rows = Vec::<(Vec<i128>, i128)>::new();
    let mut add_equation = |source: &DimensionExpr, target: &DimensionExpr| -> MResult<()> {
        let (
            Some((source_constant, source_coefficients)),
            Some((target_constant, target_coefficients)),
        ) = (
            affine_reified_dimension(source, bindings, &unknown),
            affine_reified_dimension(target, bindings, &unknown),
        )
        else {
            return Ok(());
        };
        let coefficients = (0..unknown.len())
            .map(|column| {
                target_coefficients[column]
                    .checked_sub(source_coefficients[column])
                    .ok_or_else(|| {
                        invalid_reified_conversion_target(
                            "joint dimension equation exceeds exact arithmetic",
                        )
                    })
            })
            .collect::<MResult<Vec<_>>>()?;
        let target = source_constant
            .checked_sub(target_constant)
            .ok_or_else(|| {
                invalid_reified_conversion_target(
                    "joint dimension equation exceeds exact arithmetic",
                )
            })?;
        let mut row = coefficients
            .iter()
            .copied()
            .map(DimensionRatio::integer)
            .collect::<Vec<_>>();
        row.push(DimensionRatio::integer(target));
        rows.push(row);
        integer_rows.push((coefficients, target));
        Ok(())
    };
    for &(source, target) in &equations {
        if reified_dimension_value(source).is_some() {
            add_equation(source, target)?;
        }
    }
    for declaration in &exact_bounds {
        add_equation(
            &DimensionExpr::Parameter(declaration.id),
            &declaration.lower_bound,
        )?;
    }
    // Declaration ranges constrain dependencies even when the declared
    // parameter already has a body witness. Keep their affine inequalities
    // for the bounded search instead of considering only unknown declarations.
    let mut inequalities = Vec::<(Vec<i128>, i128)>::new();
    for declaration in declarations {
        let subject = DimensionExpr::Parameter(declaration.id);
        for (lower, upper) in [
            Some((&declaration.lower_bound, &subject)),
            declaration
                .upper_bound
                .as_ref()
                .map(|upper| (&subject, upper)),
        ]
        .into_iter()
        .flatten()
        {
            let (
                Some((lower_constant, lower_coefficients)),
                Some((upper_constant, upper_coefficients)),
            ) = (
                affine_reified_dimension(lower, bindings, &unknown),
                affine_reified_dimension(upper, bindings, &unknown),
            )
            else {
                continue;
            };
            let coefficients = lower_coefficients
                .iter()
                .zip(&upper_coefficients)
                .map(|(lower, upper)| lower.checked_sub(*upper))
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| {
                    invalid_reified_conversion_target(
                        "joint dimension inequality exceeds exact arithmetic",
                    )
                })?;
            if coefficients.iter().any(|coefficient| *coefficient != 0) {
                let target = upper_constant.checked_sub(lower_constant).ok_or_else(|| {
                    invalid_reified_conversion_target(
                        "joint dimension inequality exceeds exact arithmetic",
                    )
                })?;
                inequalities.push((coefficients, target));
            }
        }
    }
    if integer_rows.len().saturating_add(inequalities.len()) > MAX_REIFIED_SOLVER_EQUATIONS {
        return Err(invalid_reified_conversion_target(
            "joint target dimension system exceeds the solver limit",
        ));
    }
    let mut pivot_row = 0;
    for column in 0..unknown.len() {
        let Some(found) = (pivot_row..rows.len()).find(|row| rows[*row][column].numerator != 0)
        else {
            continue;
        };
        rows.swap(pivot_row, found);
        let pivot = rows[pivot_row][column];
        for entry in &mut rows[pivot_row] {
            *entry = entry.divide(pivot).ok_or_else(|| {
                invalid_reified_conversion_target(
                    "joint dimension equation exceeds exact arithmetic",
                )
            })?;
        }
        let pivot_values = rows[pivot_row].clone();
        for row in 0..rows.len() {
            if row == pivot_row {
                continue;
            }
            let factor = rows[row][column];
            for entry in 0..=unknown.len() {
                rows[row][entry] = rows[row][entry]
                    .subtract(factor.multiply(pivot_values[entry]).ok_or_else(|| {
                        invalid_reified_conversion_target(
                            "joint dimension equation exceeds exact arithmetic",
                        )
                    })?)
                    .ok_or_else(|| {
                        invalid_reified_conversion_target(
                            "joint dimension equation exceeds exact arithmetic",
                        )
                    })?;
            }
        }
        pivot_row += 1;
    }
    if rows.iter().any(|row| {
        row[..unknown.len()]
            .iter()
            .all(|value| value.numerator == 0)
            && row[unknown.len()].numerator != 0
    }) {
        return Err(invalid_reified_conversion_target(
            "joint target dimensions have no source witness",
        ));
    }
    if pivot_row != unknown.len() {
        let mut concrete_equations = equations
            .iter()
            .filter(|(source, _)| reified_dimension_value(source).is_some())
            .map(|(source, target)| ((*source).clone(), (*target).clone()))
            .collect::<Vec<_>>();
        concrete_equations.extend(exact_bounds.iter().map(|declaration| {
            (
                DimensionExpr::Parameter(declaration.id),
                declaration.lower_bound.clone(),
            )
        }));
        if let Some(values) = solve_bounded_joint_reified_dimensions(
            &integer_rows,
            &inequalities,
            &concrete_equations,
            declarations,
            bindings,
            &unknown,
        )? {
            for (index, value) in unknown.iter().zip(values) {
                bindings[*index] = Some(DimensionExpr::Constant(value));
            }
        }
        return Ok(());
    }
    let unknown_count = unknown.len();
    for (column, index) in unknown.into_iter().enumerate() {
        let value = rows[column][unknown_count];
        if value.denominator != 1 {
            return Err(invalid_reified_conversion_target(
                "joint target dimensions require fractional extents",
            ));
        }
        let value = u64::try_from(value.numerator).map_err(|_| {
            invalid_reified_conversion_target("joint target dimension is outside the extent range")
        })?;
        bindings[index] = Some(DimensionExpr::Constant(value));
    }
    Ok(())
}

#[cfg(feature = "convert")]
fn inherit_reified_dynamic_cardinality(
    source: &SchemaBody,
    target: &mut SchemaBody,
    declarations: &[DimensionParameterDeclaration],
) -> MResult<()> {
    if !has_dynamic_cardinality(source) {
        return Ok(());
    }
    fn count_expression(dimension: &DimensionExpr, counts: &mut [usize]) {
        match dimension {
            DimensionExpr::Parameter(id) => {
                if let Some(count) = counts.get_mut(id.get() as usize) {
                    *count = count.saturating_add(1);
                }
            }
            DimensionExpr::Add(children)
            | DimensionExpr::Multiply(children)
            | DimensionExpr::Min(children)
            | DimensionExpr::Max(children) => {
                for child in children {
                    count_expression(child, counts);
                }
            }
            _ => {}
        }
    }
    fn count_cardinality(cardinality: &CardinalitySpec, counts: &mut [usize]) {
        match cardinality {
            CardinalitySpec::Exact(dimension) => count_expression(dimension, counts),
            CardinalitySpec::Dynamic {
                upper_bound: Some(dimension),
            } => count_expression(dimension, counts),
            CardinalitySpec::Dynamic { upper_bound: None } => {}
        }
    }
    fn count_body(body: &SchemaBody, counts: &mut [usize]) {
        match body {
            SchemaBody::Matrix {
                element,
                dimensions,
            } => {
                for dimension in dimensions {
                    count_expression(dimension, counts);
                }
                count_body(element, counts);
            }
            SchemaBody::Set {
                element,
                cardinality,
            } => {
                count_cardinality(cardinality, counts);
                count_body(element, counts);
            }
            SchemaBody::Map {
                key,
                value,
                cardinality,
            } => {
                count_cardinality(cardinality, counts);
                count_body(key, counts);
                count_body(value, counts);
            }
            SchemaBody::Table { columns, rows } => {
                count_cardinality(rows, counts);
                for column in columns {
                    count_body(&column.schema, counts);
                }
            }
            SchemaBody::Option(payload) => count_body(payload, counts),
            SchemaBody::Tuple(elements) => {
                for element in elements {
                    count_body(element, counts);
                }
            }
            SchemaBody::Record(fields) => {
                for field in fields {
                    count_body(&field.schema, counts);
                }
            }
            SchemaBody::Enum { variants, .. } => {
                for payload in variants
                    .iter()
                    .filter_map(|variant| variant.payload.as_ref())
                {
                    count_body(payload, counts);
                }
            }
            _ => {}
        }
    }
    let mut use_counts = vec![0; declarations.len()];
    count_body(target, &mut use_counts);
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
            SchemaBody::Enum {
                key: source_key,
                variants: source,
            },
            SchemaBody::Enum {
                key: target_key,
                variants: target,
            },
        ) if source_key == target_key && source.len() == target.len() => {
            for (source, target) in source.iter().zip(target.iter_mut()) {
                if source.name == target.name {
                    if let (Some(source), Some(target)) = (&source.payload, target.payload.as_mut())
                    {
                        inherit_reified_dynamic_cardinality_inner(
                            source,
                            target,
                            declarations,
                            use_counts,
                        )?;
                    }
                }
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
        SchemaBody::Enum { variants, .. } => variants
            .iter()
            .filter_map(|variant| variant.payload.as_ref())
            .any(has_dynamic_cardinality),
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
    fn bind_cardinality(
        source: &CardinalitySpec,
        target: &CardinalitySpec,
        declarations: &[DimensionParameterDeclaration],
        bindings: &mut [Option<DimensionExpr>],
    ) -> MResult<()> {
        if let Some((source, target)) = reified_cardinality_dimension_equation(source, target) {
            bind_reified_dimension(source, target, declarations, bindings)
        } else {
            // Dynamic-to-exact compatibility is decided before solving by
            // inherit_reified_dynamic_cardinality. Keeping the declared target
            // exact here distinguishes target-owned dynamic bounds from bounds
            // copied into the solver template from the source.
            Ok(())
        }
    }
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
            SchemaBody::Enum {
                key: source_key,
                variants: source,
            },
            SchemaBody::Enum {
                key: target_key,
                variants: target,
            },
        ) if source_key == target_key && source.len() == target.len() => {
            for (source, target) in source.iter().zip(target.iter()) {
                if source.name == target.name {
                    if let (Some(source), Some(target)) = (&source.payload, &target.payload) {
                        bind_reified_target_dimensions(source, target, declarations, bindings)?;
                    }
                }
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
            bind_cardinality(
                source_cardinality,
                target_cardinality,
                declarations,
                bindings,
            )?;
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
            bind_cardinality(
                source_cardinality,
                target_cardinality,
                declarations,
                bindings,
            )?;
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
            bind_cardinality(source_rows, target_rows, declarations, bindings)?;
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
    let resolved = match dimension {
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
    };
    // The output schema is finalized to concrete extents. Keep the same
    // canonical representation after substituting a compound target.
    Ok(reified_dimension_value(&resolved)
        .map(DimensionExpr::Constant)
        .unwrap_or(resolved))
}

#[cfg(feature = "convert")]
fn substitute_reified_cardinality(
    cardinality: &CardinalitySpec,
    declared_cardinality: Option<&CardinalitySpec>,
    bindings: &[Option<DimensionExpr>],
) -> MResult<CardinalitySpec> {
    Ok(match cardinality {
        CardinalitySpec::Exact(dimension) => {
            CardinalitySpec::Exact(substitute_reified_dimension(dimension, bindings)?)
        }
        CardinalitySpec::Dynamic { upper_bound } => CardinalitySpec::Dynamic {
            upper_bound: match declared_cardinality {
                // Dynamic cardinality present in the declared target owns its
                // bound, so resolve it in the target parameter environment.
                Some(CardinalitySpec::Dynamic { .. }) => upper_bound
                    .as_ref()
                    .map(|bound| substitute_reified_dimension(bound, bindings))
                    .transpose()?,
                // An exact declared cardinality can be replaced with a
                // dynamic source cardinality during inheritance. Its bound
                // remains in the source dimension environment.
                _ => upper_bound.clone(),
            },
        },
    })
}

#[cfg(feature = "convert")]
fn substitute_reified_target(
    target: &SchemaBody,
    declared_target: Option<&SchemaBody>,
    bindings: &[Option<DimensionExpr>],
) -> MResult<SchemaBody> {
    Ok(match target {
        SchemaBody::Matrix {
            element,
            dimensions,
        } => SchemaBody::Matrix {
            element: Box::new(substitute_reified_target(
                element,
                match declared_target {
                    Some(SchemaBody::Matrix { element, .. }) => Some(element),
                    _ => None,
                },
                bindings,
            )?),
            dimensions: dimensions
                .iter()
                .map(|dimension| substitute_reified_dimension(dimension, bindings))
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
        },
        SchemaBody::Option(element) => SchemaBody::Option(Box::new(substitute_reified_target(
            element,
            match declared_target {
                Some(SchemaBody::Option(element)) => Some(element),
                _ => None,
            },
            bindings,
        )?)),
        SchemaBody::Tuple(elements) => SchemaBody::Tuple(
            elements
                .iter()
                .enumerate()
                .map(|(index, element)| {
                    substitute_reified_target(
                        element,
                        match declared_target {
                            Some(SchemaBody::Tuple(elements)) => elements.get(index),
                            _ => None,
                        },
                        bindings,
                    )
                })
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
        ),
        SchemaBody::Record(fields) => SchemaBody::Record(
            fields
                .iter()
                .enumerate()
                .map(|(index, field)| {
                    Ok(SchemaField {
                        name: field.name.clone(),
                        schema: substitute_reified_target(
                            &field.schema,
                            match declared_target {
                                Some(SchemaBody::Record(fields)) => {
                                    fields.get(index).map(|field| &field.schema)
                                }
                                _ => None,
                            },
                            bindings,
                        )?,
                    })
                })
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
        ),
        SchemaBody::Enum { key, variants } => SchemaBody::Enum {
            key: *key,
            variants: variants
                .iter()
                .enumerate()
                .map(|(index, variant)| {
                    Ok(EnumVariantSchema {
                        name: variant.name.clone(),
                        payload: variant
                            .payload
                            .as_ref()
                            .map(|payload| {
                                substitute_reified_target(
                                    payload,
                                    match declared_target {
                                        Some(SchemaBody::Enum { variants, .. }) => variants
                                            .get(index)
                                            .and_then(|variant| variant.payload.as_ref()),
                                        _ => None,
                                    },
                                    bindings,
                                )
                            })
                            .transpose()?,
                    })
                })
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
        },
        SchemaBody::Set {
            element,
            cardinality,
        } => SchemaBody::Set {
            element: Box::new(substitute_reified_target(
                element,
                match declared_target {
                    Some(SchemaBody::Set { element, .. }) => Some(element),
                    _ => None,
                },
                bindings,
            )?),
            cardinality: substitute_reified_cardinality(
                cardinality,
                match declared_target {
                    Some(SchemaBody::Set { cardinality, .. }) => Some(cardinality),
                    _ => None,
                },
                bindings,
            )?,
        },
        SchemaBody::Map {
            key,
            value,
            cardinality,
        } => SchemaBody::Map {
            key: Box::new(substitute_reified_target(
                key,
                match declared_target {
                    Some(SchemaBody::Map { key, .. }) => Some(key),
                    _ => None,
                },
                bindings,
            )?),
            value: Box::new(substitute_reified_target(
                value,
                match declared_target {
                    Some(SchemaBody::Map { value, .. }) => Some(value),
                    _ => None,
                },
                bindings,
            )?),
            cardinality: substitute_reified_cardinality(
                cardinality,
                match declared_target {
                    Some(SchemaBody::Map { cardinality, .. }) => Some(cardinality),
                    _ => None,
                },
                bindings,
            )?,
        },
        SchemaBody::Table { columns, rows } => SchemaBody::Table {
            columns: columns
                .iter()
                .enumerate()
                .map(|(index, field)| {
                    Ok(SchemaField {
                        name: field.name.clone(),
                        schema: substitute_reified_target(
                            &field.schema,
                            match declared_target {
                                Some(SchemaBody::Table { columns, .. }) => {
                                    columns.get(index).map(|field| &field.schema)
                                }
                                _ => None,
                            },
                            bindings,
                        )?,
                    })
                })
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
            rows: substitute_reified_cardinality(
                rows,
                match declared_target {
                    Some(SchemaBody::Table { rows, .. }) => Some(rows),
                    _ => None,
                },
                bindings,
            )?,
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
        let (target, reified_dimensions, close_enum_payloads) = match target_value.data() {
            ValueData::Type(ReifiedType::Kind(kind)) => {
                let (target, dimensions) = schema_body_from_reified_kind(kind, context.schemas())?;
                (target, Some(dimensions), true)
            }
            ValueData::Type(ReifiedType::Schema(key)) => {
                let schema = context.schema(*key)?;
                (
                    schema.body().clone(),
                    Some(schema_target_declarations(schema)),
                    false,
                )
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
        let (target, reified_constraints) = if let Some(declarations) = reified_dimensions {
            let source_body = source.closed_schema_body()?;
            let mut concrete_template = target;
            if close_enum_payloads {
                validate_reified_schema_materialization_size(&source_body)?;
                close_reified_enum_targets(&source_body, &mut concrete_template);
            }
            let declared_target = concrete_template.clone();
            inherit_reified_dynamic_cardinality(
                &source_body,
                &mut concrete_template,
                &declarations,
            )?;
            let bindings = solve_reified_target_bindings_with_declared(
                &source_body,
                &concrete_template,
                &declared_target,
                &declarations,
            )?;
            validate_reified_parameter_bindings(&declarations, &bindings)?;
            let reified_constraints = ReifiedTargetConstraints {
                target: concrete_template.clone(),
                declared_target: declared_target.clone(),
                activation_witnesses: ReifiedTargetConstraints::activation_witnesses(
                    &declarations,
                    &bindings,
                ),
                declarations,
            };
            (
                substitute_reified_target(&concrete_template, Some(&declared_target), &bindings)?,
                Some(reified_constraints),
            )
        } else {
            (target, None)
        };
        let semantic_target =
            materialize_conversion_semantic_shape(source_type.kind(), &target, is_reified_target);
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
        let reified_target = Some(target_cell);
        context.resolve_syntax_operation_contract(&CHECKED_TYPE_CONVERSION_CONTRACT)?;
        context.certify_instance(
            planned_type_conversion_instance(
                source,
                output,
                plan,
                reified_constraints,
                reified_target,
            ),
            mech_core::RuntimeFunctionId::from_name("convert/kind/reified"),
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
        if let Some(target) = &self.reified_target {
            let source_schema = self.source.closed_schema_body()?;
            let expected = conversion_target_schema(&source_schema, &self.plan.step)
                .map_err(conversion_execution_error)?;
            let target = frame.snapshot_input_cell(target, 1)?;
            let constraints = runtime_reified_constraints(&self.source, &target, &expected)?;
            if let Some(initial) = &self.reified_constraints {
                constraints.validate_activation_witnesses(initial)?;
            }
        } else if let Some(constraints) = &self.reified_constraints {
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
        Some(if self.reified_target.is_some() {
            &CHECKED_TYPE_CONVERSION_CONTRACT
        } else {
            &PURE_TYPE_CONVERSION_CONTRACT
        })
    }

    fn to_string(&self) -> String {
        "PlannedTypeConversion".to_owned()
    }
}

#[cfg(all(feature = "convert", feature = "semantic-compiler"))]
impl MechFunctionCompiler for PlannedTypeConversion {
    fn compiler_owned_value_cells(&self) -> Vec<ValueCell> {
        let mut cells = vec![self.source.clone(), self.output.clone()];
        cells.extend(self.reified_target.iter().cloned());
        cells
    }

    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let destination = compile_runtime_produced_value_cell_register_with_seed(
            &self.output,
            &self.output.snapshot()?,
            context,
        )?;
        let source = compile_value_cell_register(&self.source, context)?;
        if let Some(target) = &self.reified_target {
            let target = compile_value_cell_register(target, context)?;
            let function = context.function_id("convert/kind/reified")?;
            context.emit_binop(function, destination, source, target);
        } else {
            let function = context.function_id("convert/kind")?;
            context.emit_unop(function, destination, source);
        }
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

    #[test]
    fn interpreter_open_kind_values_declare_their_axes() {
        for (source, expected_axes) in [
            ("<[u8]>", 2),
            ("<{u8:f64}>", 1),
            ("<{u8}>", 1),
            ("<|a<u8>|>", 1),
        ] {
            let tree = mech_syntax::parser::parse(source).unwrap();
            let mut interpreter = Interpreter::with_function_catalog(
                0,
                10_000,
                crate::test_support::catalog::function_catalog(),
            );
            let output = interpreter.interpret(&tree).unwrap().unwrap();
            let snapshot = output.snapshot().unwrap();
            let ValueData::Type(ReifiedType::Kind(kind)) = snapshot.data() else {
                panic!("{source}: expected a reified kind")
            };
            let (_, dimensions, _) = kind.decoded_closed_kind().unwrap();
            assert_eq!(dimensions.len(), expected_axes, "{source}");
        }
    }

    #[test]
    fn interpreter_identity_kind_values_match_source_kinds() {
        for (source, expected) in [("<id>", KindExpr::Id), ("<ix>", KindExpr::Index)] {
            let tree = mech_syntax::parser::parse(source).unwrap();
            let mut interpreter = Interpreter::with_function_catalog(
                0,
                10_000,
                crate::test_support::catalog::function_catalog(),
            );
            let output = interpreter.interpret(&tree).unwrap().unwrap();
            let snapshot = output.snapshot().unwrap();
            let ValueData::Type(ReifiedType::Kind(kind)) = snapshot.data() else {
                panic!("{source}: expected a reified kind")
            };
            assert_eq!(kind.decoded_closed_kind().unwrap().0, expected);
        }
    }

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
            schema_body_from_reified_kind(&repeated, context.schemas())
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
    fn compound_reified_matrix_specializes_from_semantic_turn_axes() {
        let (id, path) = builtin_scalar_named_kind(mech_core::hash_str("u8")).unwrap();
        let named = NamedKinds(BTreeMap::from([(id, path)]));
        let p = DimensionParameterId::new(0);
        let q = DimensionParameterId::new(1);
        let dimensions = [p, q].map(|id| DimensionParameterDeclaration {
            id,
            origin: DimensionParameterOrigin::Inferred,
            lifetime: DimensionLifetime::Activation,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: None,
        });
        let kind = ReifiedKind::from_closed_kind(
            &KindExpr::Matrix {
                element: Box::new(KindExpr::Named(id)),
                dimensions: [
                    DimensionExpr::Add(
                        [DimensionExpr::Parameter(p), DimensionExpr::Constant(1)].into(),
                    ),
                    DimensionExpr::Parameter(q),
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
            },
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
        let target = SchemaBody::Matrix {
            element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
            dimensions: [DimensionExpr::Parameter(id), DimensionExpr::Constant(1)].into(),
        };
        let constraints = ReifiedTargetConstraints {
            target: target.clone(),
            declared_target: target,
            activation_witnesses: Box::new([]),
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
    fn runtime_reified_conversion_rechecks_bound_before_publication() {
        let (kind_id, path) = builtin_scalar_named_kind(mech_core::hash_str("f64")).unwrap();
        let named = NamedKinds(BTreeMap::from([(kind_id, path)]));
        let id = DimensionParameterId::new(0);
        let kind = ReifiedKind::from_closed_kind(
            &KindExpr::Matrix {
                element: Box::new(KindExpr::Named(kind_id)),
                dimensions: [DimensionExpr::Parameter(id), DimensionExpr::Constant(1)].into(),
            },
            &[DimensionParameterDeclaration {
                id,
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Turn,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: Some(DimensionExpr::Constant(2)),
            }],
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
        let source_type = source.resolved_type().unwrap();
        let plan = plan_explicit_cast(&source_type, &source_type).unwrap();
        let output =
            execute_conversion_plan(&source, &source.closed_schema_body().unwrap(), &plan).unwrap();
        let invocation = FunctionInvocation::binary(output.clone(), source.clone(), target.clone());
        let function = RuntimeReifiedKindConversion::new_invocation(invocation.clone()).unwrap();
        let conversion = SpecializedFunction::syntax_directed(
            (function, invocation),
            ResolvedOperationDescriptor::from_name(
                "convert/kind/reified",
                CHECKED_TYPE_CONVERSION_CONTRACT.clone(),
            )
            .unwrap(),
            RuntimeFunctionId::from_name("convert/kind/reified"),
            ExecutionTarget::DirectRuntime,
            mech_core::ImplementationMemoryClass::CanonicalFinalize,
        )
        .unwrap();
        source
            .replace(&matrix(2, &[1.0, 2.0]).snapshot().unwrap())
            .unwrap();
        conversion.instance().solve_result().unwrap();
        let narrower = ReifiedKind::from_closed_kind(
            &KindExpr::Matrix {
                element: Box::new(KindExpr::Named(kind_id)),
                dimensions: [DimensionExpr::Parameter(id), DimensionExpr::Constant(1)].into(),
            },
            &[DimensionParameterDeclaration {
                id,
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Turn,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: Some(DimensionExpr::Constant(1)),
            }],
            &named,
        )
        .unwrap();
        let narrower = ValueCell::from_schema_data(
            SchemaBody::ReifiedType,
            ValueDataDraft::Type(ReifiedTypeDraft::CanonicalKind(
                narrower.canonical_bytes().to_vec().into_boxed_slice(),
            )),
        )
        .unwrap();
        let original_target = target.snapshot().unwrap();
        target.replace(&narrower.snapshot().unwrap()).unwrap();
        assert!(conversion.instance().solve_result().is_err());
        assert_eq!(
            output.current_top_level_extents().unwrap().as_ref(),
            &[2, 1]
        );
        let (other_id, other_path) = builtin_scalar_named_kind(mech_core::hash_str("u8")).unwrap();
        let other_kind = ReifiedKind::from_closed_kind(
            &KindExpr::Named(other_id),
            &[],
            &NamedKinds(BTreeMap::from([(other_id, other_path)])),
        )
        .unwrap();
        let other_target = ValueCell::from_schema_data(
            SchemaBody::ReifiedType,
            ValueDataDraft::Type(ReifiedTypeDraft::CanonicalKind(
                other_kind.canonical_bytes().to_vec().into_boxed_slice(),
            )),
        )
        .unwrap();
        target.replace(&other_target.snapshot().unwrap()).unwrap();
        assert!(conversion.instance().solve_result().is_err());
        assert_eq!(
            output.current_top_level_extents().unwrap().as_ref(),
            &[2, 1]
        );
        target.replace(&original_target).unwrap();
        source
            .replace(&matrix(3, &[1.0, 2.0, 3.0]).snapshot().unwrap())
            .unwrap();
        assert!(conversion.instance().solve_result().is_err());
        assert_eq!(
            output.current_top_level_extents().unwrap().as_ref(),
            &[2, 1]
        );
    }

    #[test]
    fn planned_and_runtime_reified_conversions_keep_activation_witnesses() {
        let (kind_id, path) = builtin_scalar_named_kind(mech_core::hash_str("f64")).unwrap();
        let named = NamedKinds(BTreeMap::from([(kind_id, path)]));
        let parameter = DimensionParameterId::new(0);
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
        for lifetime in [DimensionLifetime::Activation, DimensionLifetime::Turn] {
            let kind = ReifiedKind::from_closed_kind(
                &KindExpr::Matrix {
                    element: Box::new(KindExpr::Named(kind_id)),
                    dimensions: [
                        DimensionExpr::Parameter(parameter),
                        DimensionExpr::Constant(1),
                    ]
                    .into(),
                },
                &[DimensionParameterDeclaration {
                    id: parameter,
                    origin: DimensionParameterOrigin::Inferred,
                    lifetime,
                    lower_bound: DimensionExpr::Constant(0),
                    upper_bound: None,
                }],
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
            let source = matrix(1, &[1.0]);
            let invocation = SpecializationInvocation::from_cells(
                vec![source.clone(), target.clone()].into_boxed_slice(),
            );
            let operation = ResolvedOperationDescriptor::from_name(
                "convert/kind",
                PURE_TYPE_CONVERSION_CONTRACT.clone(),
            )
            .unwrap();
            let mut context =
                SpecializationContext::for_syntax_directed_invocation(&invocation, None, operation)
                    .unwrap();
            let planned = ConvertKind
                .specialize_invocation(&invocation, &mut context)
                .unwrap();
            let output = planned.output().clone();
            let runtime_invocation =
                FunctionInvocation::binary(output.clone(), source.clone(), target);
            let runtime_function =
                RuntimeReifiedKindConversion::new_invocation(runtime_invocation.clone()).unwrap();
            let runtime = SpecializedFunction::syntax_directed(
                (runtime_function, runtime_invocation),
                ResolvedOperationDescriptor::from_name(
                    "convert/kind/reified",
                    CHECKED_TYPE_CONVERSION_CONTRACT.clone(),
                )
                .unwrap(),
                RuntimeFunctionId::from_name("convert/kind/reified"),
                ExecutionTarget::DirectRuntime,
                mech_core::ImplementationMemoryClass::CanonicalFinalize,
            )
            .unwrap();
            source
                .replace(&matrix(2, &[1.0, 2.0]).snapshot().unwrap())
                .unwrap();
            for conversion in [&planned, &runtime] {
                let result = conversion.instance().solve_result();
                if lifetime == DimensionLifetime::Activation {
                    let error = result.unwrap_err();
                    assert!(
                        format!("{error:?}")
                            .contains("activation target dimension witness changed"),
                        "{error:?}"
                    );
                    assert_eq!(
                        output.current_top_level_extents().unwrap().as_ref(),
                        &[1, 1]
                    );
                } else {
                    result.unwrap();
                    assert_eq!(
                        output.current_top_level_extents().unwrap().as_ref(),
                        &[2, 1]
                    );
                }
            }
        }
    }

    #[test]
    fn direct_reified_conversion_tracks_and_rechecks_target_cell() {
        let reified_scalar = |name| {
            let (id, path) = builtin_scalar_named_kind(mech_core::hash_str(name)).unwrap();
            let kind = ReifiedKind::from_closed_kind(
                &KindExpr::Named(id),
                &[],
                &NamedKinds(BTreeMap::from([(id, path)])),
            )
            .unwrap();
            ValueCell::from_schema_data(
                SchemaBody::ReifiedType,
                ValueDataDraft::Type(ReifiedTypeDraft::CanonicalKind(
                    kind.canonical_bytes().to_vec().into_boxed_slice(),
                )),
            )
            .unwrap()
        };
        let source = ValueCell::from_exact(7.0_f64).unwrap();
        let output = ValueCell::from_exact(7.0_f64).unwrap();
        let source_type = source.resolved_type().unwrap();
        let plan = plan_explicit_cast(&source_type, &source_type).unwrap();
        let target = reified_scalar("f64");
        let instance =
            planned_type_conversion_instance(source, output, plan, None, Some(target.clone()));
        assert!(instance.1.clone().expect_binary().is_ok());
        let conversion = SpecializedFunction::syntax_directed(
            instance,
            ResolvedOperationDescriptor::from_name(
                "convert/kind/reified",
                CHECKED_TYPE_CONVERSION_CONTRACT.clone(),
            )
            .unwrap(),
            RuntimeFunctionId::from_name("convert/kind/reified"),
            ExecutionTarget::DirectRuntime,
            mech_core::ImplementationMemoryClass::CanonicalFinalize,
        )
        .unwrap();
        conversion.instance().solve_result().unwrap();
        target
            .replace(&reified_scalar("u8").snapshot().unwrap())
            .unwrap();
        assert!(conversion.instance().solve_result().is_err());
    }

    #[test]
    fn schema_target_remains_a_live_conversion_dependency() {
        let source = ValueCell::from_exact(7.0_f64).unwrap();
        let target = ValueCell::from_schema_data(
            SchemaBody::ReifiedType,
            ValueDataDraft::Type(ReifiedTypeDraft::Schema(source.schema_key())),
        )
        .unwrap();
        let invocation = SpecializationInvocation::from_cells(
            vec![source.clone(), target.clone()].into_boxed_slice(),
        );
        let operation = ResolvedOperationDescriptor::from_name(
            "convert/kind",
            PURE_TYPE_CONVERSION_CONTRACT.clone(),
        )
        .unwrap();
        let mut context =
            SpecializationContext::for_syntax_directed_invocation(&invocation, None, operation)
                .unwrap();
        let conversion = ConvertKind
            .specialize_invocation(&invocation, &mut context)
            .unwrap();
        assert_eq!(conversion.instance().inputs().len(), 2);
        conversion.instance().solve_result().unwrap();
        let runtime = FunctionInvocation::binary(
            ValueCell::from_exact(0.0_f64).unwrap(),
            source,
            target.clone(),
        );
        assert!(RuntimeReifiedKindConversion::new_invocation(runtime).is_ok());
        let other = ValueCell::from_exact(0_u8).unwrap();
        let changed_target = ValueCell::from_schema_data(
            SchemaBody::ReifiedType,
            ValueDataDraft::Type(ReifiedTypeDraft::Schema(other.schema_key())),
        )
        .unwrap();
        target.replace(&changed_target.snapshot().unwrap()).unwrap();
        assert!(conversion.instance().solve_result().is_err());
    }

    #[test]
    fn runtime_schema_target_can_resolve_from_its_own_table() {
        let source = ValueCell::from_exact(7.0_f64).unwrap();
        let schema = SchemaDraft {
            dimension_parameters: Box::new([]),
            body: SchemaBody::Bool,
        }
        .finalize()
        .unwrap();
        let key = schema.key();
        assert!(
            source
                .snapshot()
                .unwrap()
                .schemas()
                .unwrap()
                .find_by_key(key)
                .is_none()
        );
        let target = ValueCell::from_schema_data(
            SchemaBody::ReifiedType,
            ValueDataDraft::Type(ReifiedTypeDraft::Schema(key)),
        )
        .unwrap();
        let mut builder = mech_core::SchemaTableBuilder::new();
        builder.insert(schema).unwrap();
        builder
            .insert(
                SchemaDraft {
                    dimension_parameters: Box::new([]),
                    body: SchemaBody::ReifiedType,
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let table = std::rc::Rc::new(builder.finish().unwrap().table);
        let target = ValueCell::from_value(target.snapshot().unwrap(), table).unwrap();
        assert!(
            runtime_reified_constraints(&source, &target.snapshot().unwrap(), &SchemaBody::Bool,)
                .is_ok()
        );
    }

    #[test]
    fn runtime_schema_lookup_does_not_snapshot_source_payload() {
        let template = ValueCell::from_exact(7_u8).unwrap();
        let reference = Ref::new(7_u8);
        let source = ValueCell::from_ref(
            reference.clone(),
            template.schema(),
            template.shape().clone(),
            template.retained_schema_table(),
        )
        .unwrap();
        let target = ValueCell::from_schema_data(
            SchemaBody::ReifiedType,
            ValueDataDraft::Type(ReifiedTypeDraft::Schema(source.schema_key())),
        )
        .unwrap();
        let target_value = target.snapshot().unwrap();
        let expected = source.closed_schema_body().unwrap();
        let _exclusive_borrow = reference.borrow_mut();
        assert!(source.snapshot().is_err());
        assert!(runtime_reified_constraints(&source, &target_value, &expected).is_ok());
    }

    #[test]
    fn parameterized_schema_target_closes_against_each_source_shape() {
        let matrix = |rows, values: &[f64]| {
            ValueCell::dynamic_matrix_from_cells(
                rows,
                2,
                &values
                    .iter()
                    .map(|value| ValueCell::from_exact(*value).unwrap())
                    .collect::<Vec<_>>(),
            )
            .unwrap()
        };
        let source = matrix(1, &[1.0, 2.0]);
        let target = ValueCell::from_schema_data(
            SchemaBody::ReifiedType,
            ValueDataDraft::Type(ReifiedTypeDraft::Schema(source.schema_key())),
        )
        .unwrap();
        let invocation = SpecializationInvocation::from_cells(
            vec![source.clone(), target.clone()].into_boxed_slice(),
        );
        let operation = ResolvedOperationDescriptor::from_name(
            "convert/kind",
            PURE_TYPE_CONVERSION_CONTRACT.clone(),
        )
        .unwrap();
        let mut context =
            SpecializationContext::for_syntax_directed_invocation(&invocation, None, operation)
                .unwrap();
        let conversion = ConvertKind
            .specialize_invocation(&invocation, &mut context)
            .unwrap();
        conversion.instance().solve_result().unwrap();
        let output = conversion.output().clone();
        assert!(
            RuntimeReifiedKindConversion::new_invocation(FunctionInvocation::binary(
                output.clone(),
                source.clone(),
                target,
            ))
            .is_ok()
        );
        source
            .replace(&matrix(2, &[1.0, 2.0, 3.0, 4.0]).snapshot().unwrap())
            .unwrap();
        conversion.instance().solve_result().unwrap();
        assert_eq!(
            output.closed_schema_body().unwrap(),
            source.closed_schema_body().unwrap()
        );
    }

    #[test]
    fn runtime_reified_conversion_rejects_output_that_differs_from_target() {
        let (kind_id, path) = builtin_scalar_named_kind(mech_core::hash_str("f64")).unwrap();
        let kind = ReifiedKind::from_closed_kind(
            &KindExpr::Named(kind_id),
            &[],
            &NamedKinds(BTreeMap::from([(kind_id, path)])),
        )
        .unwrap();
        let target = ValueCell::from_schema_data(
            SchemaBody::ReifiedType,
            ValueDataDraft::Type(ReifiedTypeDraft::CanonicalKind(
                kind.canonical_bytes().to_vec().into_boxed_slice(),
            )),
        )
        .unwrap();
        let source = ValueCell::from_exact(1.5_f64).unwrap();
        let wrong_output = ValueCell::from_exact(0_u8).unwrap();
        let invocation = FunctionInvocation::binary(wrong_output, source.clone(), target.clone());
        assert!(RuntimeReifiedKindConversion::new_invocation(invocation).is_err());
        let matching_output = ValueCell::from_exact(0.0_f64).unwrap();
        let invocation = FunctionInvocation::binary(matching_output, source, target);
        assert!(RuntimeReifiedKindConversion::new_invocation(invocation).is_ok());
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
        let ambiguous = SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: [DimensionExpr::Min(
                [DimensionExpr::Parameter(id), DimensionExpr::Constant(5)].into(),
            )]
            .into(),
        };
        let source = SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: [DimensionExpr::Constant(5)].into(),
        };
        assert!(solve_reified_target_bindings(&source, &ambiguous, &[declaration]).is_err());
    }

    #[test]
    fn joint_affine_axes_determine_shared_reified_parameters() {
        let p = DimensionParameterId::new(0);
        let q = DimensionParameterId::new(1);
        let declarations = [p, q].map(|id| DimensionParameterDeclaration {
            id,
            origin: DimensionParameterOrigin::Inferred,
            lifetime: DimensionLifetime::Activation,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: None,
        });
        let target = SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: [
                DimensionExpr::Add(
                    [DimensionExpr::Parameter(p), DimensionExpr::Parameter(q)].into(),
                ),
                DimensionExpr::Add(
                    [
                        DimensionExpr::Multiply(
                            [DimensionExpr::Constant(2), DimensionExpr::Parameter(p)].into(),
                        ),
                        DimensionExpr::Parameter(q),
                    ]
                    .into(),
                ),
            ]
            .into(),
        };
        let source = |first, second| SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: [
                DimensionExpr::Constant(first),
                DimensionExpr::Constant(second),
            ]
            .into(),
        };
        assert_eq!(
            solve_reified_target_bindings(&source(5, 7), &target, &declarations).unwrap(),
            vec![
                Some(DimensionExpr::Constant(2)),
                Some(DimensionExpr::Constant(3))
            ],
        );
        assert!(solve_reified_target_bindings(&source(5, 4), &target, &declarations).is_err());
    }

    #[test]
    fn equivalent_affine_bounds_determine_an_unbounded_parameter() {
        let p = DimensionParameterId::new(0);
        let q = DimensionParameterId::new(1);
        let declarations = [
            DimensionParameterDeclaration {
                id: p,
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Activation,
                lower_bound: DimensionExpr::Add(
                    [DimensionExpr::Parameter(q), DimensionExpr::Parameter(q)].into(),
                ),
                upper_bound: Some(DimensionExpr::Multiply(
                    [DimensionExpr::Constant(2), DimensionExpr::Parameter(q)].into(),
                )),
            },
            DimensionParameterDeclaration {
                id: q,
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Activation,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: None,
            },
        ];
        let source = SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: [DimensionExpr::Constant(10)].into(),
        };
        let target = SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: [DimensionExpr::Parameter(p)].into(),
        };
        assert_eq!(
            solve_reified_target_bindings(&source, &target, &declarations).unwrap(),
            vec![
                Some(DimensionExpr::Constant(10)),
                Some(DimensionExpr::Constant(5))
            ],
        );
    }

    #[test]
    fn long_dependent_reified_binding_chain_hits_early_complexity_limit() {
        let count = MAX_REIFIED_BINDING_PARAMETERS + 1;
        let declarations = (0..count)
            .map(|index| DimensionParameterDeclaration {
                id: DimensionParameterId::new(index as u32),
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Activation,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: None,
            })
            .collect::<Vec<_>>();
        let source = SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: (0..count)
                .map(|index| DimensionExpr::Constant(if index + 1 == count { 1 } else { 2 }))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        };
        let target = SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: (0..count)
                .map(|index| {
                    let parameter =
                        DimensionExpr::Parameter(DimensionParameterId::new(index as u32));
                    if index + 1 == count {
                        parameter
                    } else {
                        DimensionExpr::Add(
                            [
                                parameter,
                                DimensionExpr::Parameter(DimensionParameterId::new(
                                    (index + 1) as u32,
                                )),
                            ]
                            .into(),
                        )
                    }
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        };
        assert!(solve_reified_target_bindings(&source, &target, &declarations).is_err());
    }

    #[test]
    fn concrete_axes_do_not_consume_joint_solver_equation_limit() {
        let axes = vec![DimensionExpr::Constant(1); MAX_REIFIED_SOLVER_EQUATIONS + 1];
        let shape = SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: axes.into_boxed_slice(),
        };
        assert_eq!(
            solve_reified_target_bindings(&shape, &shape, &[]).unwrap(),
            vec![]
        );
        let id = DimensionParameterId::new(0);
        let declaration = DimensionParameterDeclaration {
            id,
            origin: DimensionParameterOrigin::Inferred,
            lifetime: DimensionLifetime::Activation,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: None,
        };
        let repeated = SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: vec![DimensionExpr::Parameter(id); MAX_REIFIED_SOLVER_EQUATIONS + 1]
                .into_boxed_slice(),
        };
        assert_eq!(
            solve_reified_target_bindings(&shape, &repeated, &[declaration]).unwrap(),
            vec![Some(DimensionExpr::Constant(1))],
        );
    }

    #[test]
    fn finite_ranges_can_make_joint_reified_dimensions_unique() {
        let p = DimensionParameterId::new(0);
        let q = DimensionParameterId::new(1);
        let declaration = |id, lower, upper| DimensionParameterDeclaration {
            id,
            origin: DimensionParameterOrigin::Inferred,
            lifetime: DimensionLifetime::Activation,
            lower_bound: DimensionExpr::Constant(lower),
            upper_bound: Some(DimensionExpr::Constant(upper)),
        };
        let source = |extent| SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: [DimensionExpr::Constant(extent)].into(),
        };
        let target = |first, second| SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: [DimensionExpr::Add([first, second].into())].into(),
        };
        let sum = target(DimensionExpr::Parameter(p), DimensionExpr::Parameter(q));
        assert_eq!(
            solve_reified_target_bindings(
                &source(5),
                &sum,
                &[declaration(p, 2, 3), declaration(q, 0, 2)],
            )
            .unwrap(),
            vec![
                Some(DimensionExpr::Constant(3)),
                Some(DimensionExpr::Constant(2))
            ],
        );
        assert_eq!(
            solve_reified_target_bindings(
                &source(0),
                &sum,
                &[
                    declaration(p, 0, 1_000_000_000),
                    declaration(q, 0, 1_000_000_000)
                ],
            )
            .unwrap(),
            vec![
                Some(DimensionExpr::Constant(0)),
                Some(DimensionExpr::Constant(0))
            ],
        );
        let weighted = target(
            DimensionExpr::Multiply(
                [DimensionExpr::Constant(2), DimensionExpr::Parameter(p)].into(),
            ),
            DimensionExpr::Multiply(
                [DimensionExpr::Constant(3), DimensionExpr::Parameter(q)].into(),
            ),
        );
        assert_eq!(
            solve_reified_target_bindings(
                &source(5),
                &weighted,
                &[declaration(p, 0, 2), declaration(q, 0, 2)],
            )
            .unwrap(),
            vec![
                Some(DimensionExpr::Constant(1)),
                Some(DimensionExpr::Constant(1))
            ],
        );
        assert!(
            solve_reified_target_bindings(
                &source(5),
                &sum,
                &[declaration(p, 2, 4), declaration(q, 0, 3)],
            )
            .is_err()
        );
    }

    #[test]
    fn finite_ranges_validate_non_affine_reified_axes() {
        let p = DimensionParameterId::new(0);
        let q = DimensionParameterId::new(1);
        let declaration = |id, lower, upper| DimensionParameterDeclaration {
            id,
            origin: DimensionParameterOrigin::Inferred,
            lifetime: DimensionLifetime::Activation,
            lower_bound: DimensionExpr::Constant(lower),
            upper_bound: Some(DimensionExpr::Constant(upper)),
        };
        let source = SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: [DimensionExpr::Constant(2), DimensionExpr::Constant(3)].into(),
        };
        let target = SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: [
                DimensionExpr::Min(
                    [DimensionExpr::Parameter(p), DimensionExpr::Parameter(q)].into(),
                ),
                DimensionExpr::Max(
                    [DimensionExpr::Parameter(p), DimensionExpr::Parameter(q)].into(),
                ),
            ]
            .into(),
        };
        let declarations = [declaration(p, 0, 2), declaration(q, 3, 4)];
        assert_eq!(
            solve_reified_target_bindings(&source, &target, &declarations).unwrap(),
            vec![
                Some(DimensionExpr::Constant(2)),
                Some(DimensionExpr::Constant(3))
            ]
        );
        let product = SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: [
                DimensionExpr::Multiply(
                    [DimensionExpr::Parameter(p), DimensionExpr::Parameter(q)].into(),
                ),
                DimensionExpr::Max(
                    [DimensionExpr::Parameter(p), DimensionExpr::Parameter(q)].into(),
                ),
            ]
            .into(),
        };
        let product_source = SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: [DimensionExpr::Constant(6), DimensionExpr::Constant(3)].into(),
        };
        assert_eq!(
            solve_reified_target_bindings(&product_source, &product, &declarations).unwrap(),
            vec![
                Some(DimensionExpr::Constant(2)),
                Some(DimensionExpr::Constant(3))
            ]
        );
    }

    #[test]
    fn bounded_search_recomputes_dependent_parameter_ranges() {
        let p = DimensionParameterId::new(0);
        let q = DimensionParameterId::new(1);
        let r = DimensionParameterId::new(2);
        let declaration = |id, lower_bound, upper_bound| DimensionParameterDeclaration {
            id,
            origin: DimensionParameterOrigin::Inferred,
            lifetime: DimensionLifetime::Activation,
            lower_bound,
            upper_bound: Some(upper_bound),
        };
        let declarations = [
            declaration(p, DimensionExpr::Constant(0), DimensionExpr::Constant(1)),
            declaration(
                q,
                DimensionExpr::Parameter(p),
                DimensionExpr::Add(
                    [DimensionExpr::Parameter(p), DimensionExpr::Constant(1)].into(),
                ),
            ),
            declaration(
                r,
                DimensionExpr::Parameter(q),
                DimensionExpr::Add(
                    [DimensionExpr::Parameter(q), DimensionExpr::Constant(1)].into(),
                ),
            ),
        ];
        let source = |extent| SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: [DimensionExpr::Constant(extent)].into(),
        };
        let target = |parameters: Vec<DimensionExpr>| SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: [DimensionExpr::Max(parameters.into_boxed_slice())].into(),
        };
        assert_eq!(
            solve_reified_target_bindings(
                &source(2),
                &target(vec![
                    DimensionExpr::Parameter(p),
                    DimensionExpr::Parameter(q)
                ]),
                &declarations[..2],
            )
            .unwrap(),
            vec![
                Some(DimensionExpr::Constant(1)),
                Some(DimensionExpr::Constant(2))
            ],
        );
        assert_eq!(
            solve_reified_target_bindings(
                &source(3),
                &target(vec![
                    DimensionExpr::Parameter(p),
                    DimensionExpr::Parameter(q),
                    DimensionExpr::Parameter(r),
                ]),
                &declarations,
            )
            .unwrap(),
            vec![
                Some(DimensionExpr::Constant(1)),
                Some(DimensionExpr::Constant(2)),
                Some(DimensionExpr::Constant(3)),
            ],
        );
    }

    #[test]
    fn reified_nominal_enum_target_uses_closed_source_payloads() {
        let path = CanonicalNominalPath::new(vec!["test".to_owned(), "Sized".to_owned()]).unwrap();
        let key = NominalKey::from_path(NominalKind::Enum, &path);
        let source = SchemaBody::Enum {
            key,
            variants: [EnumVariantSchema {
                name: "Some".to_owned(),
                payload: Some(SchemaBody::Matrix {
                    element: Box::new(SchemaBody::Index),
                    dimensions: [DimensionExpr::Constant(2)].into(),
                }),
            }]
            .into(),
        };
        let mut target = SchemaBody::Enum {
            key,
            variants: [EnumVariantSchema {
                name: "Some".to_owned(),
                payload: Some(SchemaBody::Matrix {
                    element: Box::new(SchemaBody::Index),
                    dimensions: [DimensionExpr::Parameter(DimensionParameterId::new(0))].into(),
                }),
            }]
            .into(),
        };
        close_reified_enum_targets(&source, &mut target);
        assert_eq!(target, source);
    }

    #[test]
    fn schema_enum_payload_keeps_its_dimension_bound() {
        let path =
            CanonicalNominalPath::new(vec!["test".to_owned(), "Bounded".to_owned()]).unwrap();
        let key = NominalKey::from_path(NominalKind::Enum, &path);
        let id = DimensionParameterId::new(0);
        let enum_body = |dimension| SchemaBody::Enum {
            key,
            variants: [EnumVariantSchema {
                name: "Some".to_owned(),
                payload: Some(SchemaBody::Matrix {
                    element: Box::new(SchemaBody::Index),
                    dimensions: [dimension].into(),
                }),
            }]
            .into(),
        };
        let source = enum_body(DimensionExpr::Constant(3));
        let target = enum_body(DimensionExpr::Parameter(id));
        let declaration = DimensionParameterDeclaration {
            id,
            origin: DimensionParameterOrigin::Inferred,
            lifetime: DimensionLifetime::Activation,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: Some(DimensionExpr::Constant(2)),
        };
        let bindings =
            solve_reified_target_bindings(&source, &target, &[declaration.clone()]).unwrap();
        assert_eq!(bindings, vec![Some(DimensionExpr::Constant(3))]);
        assert!(validate_reified_parameter_bindings(&[declaration.clone()], &bindings).is_err());

        let source_cell = ValueCell::from_schema_data(
            source.clone(),
            ValueDataDraft::Enum(mech_core::snapshot::EnumDraft {
                ordinal: 0,
                payload: Some(Box::new(ValueDataDraft::Matrix(
                    (1..=3).map(ValueDataDraft::Index).collect(),
                ))),
            }),
        )
        .unwrap();
        let target_schema = SchemaDraft {
            dimension_parameters: [declaration].into(),
            body: target,
        }
        .finalize()
        .unwrap();
        let key = target_schema.key();
        let target_cell = ValueCell::from_schema_data(
            SchemaBody::ReifiedType,
            ValueDataDraft::Type(ReifiedTypeDraft::Schema(key)),
        )
        .unwrap();
        let mut builder = mech_core::SchemaTableBuilder::new();
        builder.insert(target_schema).unwrap();
        builder
            .insert(
                SchemaDraft {
                    dimension_parameters: Box::new([]),
                    body: SchemaBody::ReifiedType,
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let target_cell = ValueCell::from_value(
            target_cell.snapshot().unwrap(),
            std::rc::Rc::new(builder.finish().unwrap().table),
        )
        .unwrap();
        assert!(
            runtime_reified_constraints(
                &source_cell,
                &target_cell.snapshot().unwrap(),
                &source_cell.closed_schema_body().unwrap(),
            )
            .is_err()
        );

        let valid_source = ValueCell::from_schema_data(
            enum_body(DimensionExpr::Constant(2)),
            ValueDataDraft::Enum(mech_core::snapshot::EnumDraft {
                ordinal: 0,
                payload: Some(Box::new(ValueDataDraft::Matrix(
                    (1..=2).map(ValueDataDraft::Index).collect(),
                ))),
            }),
        )
        .unwrap();
        let invocation = SpecializationInvocation::from_cells(
            vec![valid_source.clone(), target_cell.clone()].into_boxed_slice(),
        );
        let operation = ResolvedOperationDescriptor::from_name(
            "convert/kind",
            PURE_TYPE_CONVERSION_CONTRACT.clone(),
        )
        .unwrap();
        let mut context =
            SpecializationContext::for_syntax_directed_invocation(&invocation, None, operation)
                .unwrap();
        let conversion = ConvertKind
            .specialize_invocation(&invocation, &mut context)
            .unwrap();
        conversion.instance().solve_result().unwrap();
    }

    #[test]
    fn schema_target_substitutes_its_dynamic_upper_bound() {
        let id = DimensionParameterId::new(0);
        let source_body = SchemaBody::Tuple(
            [
                SchemaBody::Matrix {
                    element: Box::new(SchemaBody::Index),
                    dimensions: [DimensionExpr::Constant(2)].into(),
                },
                SchemaBody::Set {
                    element: Box::new(SchemaBody::Index),
                    cardinality: CardinalitySpec::Dynamic {
                        upper_bound: Some(DimensionExpr::Constant(2)),
                    },
                },
            ]
            .into(),
        );
        let source = ValueCell::from_schema_data(
            source_body.clone(),
            ValueDataDraft::Tuple(
                [
                    ValueDataDraft::Matrix(
                        [ValueDataDraft::Index(1), ValueDataDraft::Index(2)].into(),
                    ),
                    ValueDataDraft::Set(
                        [ValueDataDraft::Index(1), ValueDataDraft::Index(2)].into(),
                    ),
                ]
                .into(),
            ),
        )
        .unwrap();
        let target_schema = SchemaDraft {
            dimension_parameters: [DimensionParameterDeclaration {
                id,
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Activation,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: None,
            }]
            .into(),
            body: SchemaBody::Tuple(
                [
                    SchemaBody::Matrix {
                        element: Box::new(SchemaBody::Index),
                        dimensions: [DimensionExpr::Parameter(id)].into(),
                    },
                    SchemaBody::Set {
                        element: Box::new(SchemaBody::Index),
                        cardinality: CardinalitySpec::Dynamic {
                            upper_bound: Some(DimensionExpr::Parameter(id)),
                        },
                    },
                ]
                .into(),
            ),
        }
        .finalize()
        .unwrap();
        let key = target_schema.key();
        let target = ValueCell::from_schema_data(
            SchemaBody::ReifiedType,
            ValueDataDraft::Type(ReifiedTypeDraft::Schema(key)),
        )
        .unwrap();
        let mut builder = mech_core::SchemaTableBuilder::new();
        builder.insert(target_schema).unwrap();
        builder
            .insert(
                SchemaDraft {
                    dimension_parameters: Box::new([]),
                    body: SchemaBody::ReifiedType,
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let target = ValueCell::from_value(
            target.snapshot().unwrap(),
            std::rc::Rc::new(builder.finish().unwrap().table),
        )
        .unwrap();
        let invocation = SpecializationInvocation::from_cells(
            vec![source.clone(), target.clone()].into_boxed_slice(),
        );
        let operation = ResolvedOperationDescriptor::from_name(
            "convert/kind",
            PURE_TYPE_CONVERSION_CONTRACT.clone(),
        )
        .unwrap();
        let mut context =
            SpecializationContext::for_syntax_directed_invocation(&invocation, None, operation)
                .unwrap();
        let conversion = ConvertKind
            .specialize_invocation(&invocation, &mut context)
            .unwrap();
        conversion.instance().solve_result().unwrap();
        assert_eq!(
            conversion.output().closed_schema_body().unwrap(),
            source_body
        );
        assert!(
            RuntimeReifiedKindConversion::new_invocation(FunctionInvocation::binary(
                conversion.output().clone(),
                source,
                target,
            ))
            .is_ok()
        );
    }

    #[test]
    fn dynamic_collection_bounds_bind_for_sets_maps_and_tables() {
        let declarations = [0, 1, 2].map(|index| DimensionParameterDeclaration {
            id: DimensionParameterId::new(index),
            origin: DimensionParameterOrigin::Inferred,
            lifetime: DimensionLifetime::Activation,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: None,
        });
        let dynamic = |upper_bound| CardinalitySpec::Dynamic {
            upper_bound: Some(upper_bound),
        };
        let source = SchemaBody::Tuple(
            [
                SchemaBody::Set {
                    element: Box::new(SchemaBody::Index),
                    cardinality: dynamic(DimensionExpr::Constant(2)),
                },
                SchemaBody::Map {
                    key: Box::new(SchemaBody::Index),
                    value: Box::new(SchemaBody::Bool),
                    cardinality: dynamic(DimensionExpr::Constant(3)),
                },
                SchemaBody::Table {
                    columns: [SchemaField {
                        name: "value".to_owned(),
                        schema: SchemaBody::Index,
                    }]
                    .into(),
                    rows: dynamic(DimensionExpr::Constant(4)),
                },
            ]
            .into(),
        );
        let target = SchemaBody::Tuple(
            [
                SchemaBody::Set {
                    element: Box::new(SchemaBody::Index),
                    cardinality: dynamic(DimensionExpr::Parameter(declarations[0].id)),
                },
                SchemaBody::Map {
                    key: Box::new(SchemaBody::Index),
                    value: Box::new(SchemaBody::Bool),
                    cardinality: dynamic(DimensionExpr::Parameter(declarations[1].id)),
                },
                SchemaBody::Table {
                    columns: [SchemaField {
                        name: "value".to_owned(),
                        schema: SchemaBody::Index,
                    }]
                    .into(),
                    rows: dynamic(DimensionExpr::Parameter(declarations[2].id)),
                },
            ]
            .into(),
        );
        let bindings = solve_reified_target_bindings(&source, &target, &declarations).unwrap();
        assert_eq!(
            bindings,
            vec![
                Some(DimensionExpr::Constant(2)),
                Some(DimensionExpr::Constant(3)),
                Some(DimensionExpr::Constant(4)),
            ]
        );
        assert_eq!(
            substitute_reified_target(&target, Some(&target), &bindings).unwrap(),
            source
        );
    }

    #[test]
    fn repeated_nominal_enum_targets_respect_the_materialization_limit() {
        let path = CanonicalNominalPath::new(vec!["test".to_owned(), "Large".to_owned()]).unwrap();
        let key = NominalKey::from_path(NominalKind::Enum, &path);
        let schema = SchemaDraft {
            dimension_parameters: Box::new([]),
            body: SchemaBody::Enum {
                key,
                variants: [EnumVariantSchema {
                    name: "Many".to_owned(),
                    payload: Some(SchemaBody::Tuple(
                        vec![SchemaBody::Index; 512].into_boxed_slice(),
                    )),
                }]
                .into(),
            },
        }
        .finalize()
        .unwrap();
        let mut builder = mech_core::SchemaTableBuilder::new();
        builder.insert(schema).unwrap();
        let table = builder.finish().unwrap().table;
        let kind = KindExpr::Tuple(
            vec![KindExpr::Enum(key); MAX_REIFIED_TARGET_MATERIALIZATION_NODES / 512 + 1]
                .into_boxed_slice(),
        );
        let reified =
            ReifiedKind::from_closed_kind(&kind, &[], &NamedKinds(BTreeMap::new())).unwrap();
        assert!(schema_body_from_reified_kind(&reified, &table).is_err());
    }

    #[test]
    fn reified_named_scalar_requires_its_full_canonical_path() {
        let id = KindId::new(0);
        let spoofed = CanonicalNominalPath::new(vec![
            "user".to_owned(),
            "package".to_owned(),
            "u8".to_owned(),
        ])
        .unwrap();
        let reified = ReifiedKind::from_closed_kind(
            &KindExpr::Named(id),
            &[],
            &NamedKinds(BTreeMap::from([(id, spoofed)])),
        )
        .unwrap();
        let table = mech_core::SchemaTableBuilder::new().finish().unwrap().table;
        assert!(schema_body_from_reified_kind(&reified, &table).is_err());
    }

    #[test]
    fn enum_payload_dynamic_cardinality_uses_the_same_inheritance_rule() {
        let path = CanonicalNominalPath::new(vec!["test".to_owned(), "Open".to_owned()]).unwrap();
        let key = NominalKey::from_path(NominalKind::Enum, &path);
        let id = DimensionParameterId::new(0);
        let enum_body = |cardinality| SchemaBody::Enum {
            key,
            variants: [EnumVariantSchema {
                name: "Items".to_owned(),
                payload: Some(SchemaBody::Set {
                    element: Box::new(SchemaBody::Index),
                    cardinality,
                }),
            }]
            .into(),
        };
        let source = enum_body(CardinalitySpec::Dynamic { upper_bound: None });
        let mut target = enum_body(CardinalitySpec::Exact(DimensionExpr::Parameter(id)));
        let declaration = DimensionParameterDeclaration {
            id,
            origin: DimensionParameterOrigin::Inferred,
            lifetime: DimensionLifetime::Activation,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: None,
        };
        inherit_reified_dynamic_cardinality(&source, &mut target, &[declaration]).unwrap();
        assert_eq!(target, source);
    }

    #[test]
    fn exact_reified_bound_propagates_a_body_witness_back_to_its_dependency() {
        let q = DimensionParameterId::new(0);
        let p = DimensionParameterId::new(1);
        let declarations = [
            DimensionParameterDeclaration {
                id: q,
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Activation,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: None,
            },
            DimensionParameterDeclaration {
                id: p,
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Activation,
                lower_bound: DimensionExpr::Parameter(q),
                upper_bound: Some(DimensionExpr::Parameter(q)),
            },
        ];
        let source = SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: [DimensionExpr::Constant(5)].into(),
        };
        let target = SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: [DimensionExpr::Parameter(p)].into(),
        };
        assert_eq!(
            solve_reified_target_bindings(&source, &target, &declarations).unwrap(),
            vec![
                Some(DimensionExpr::Constant(5)),
                Some(DimensionExpr::Constant(5)),
            ]
        );
    }

    #[test]
    fn non_affine_exact_bound_propagates_a_unique_dependency() {
        let q = DimensionParameterId::new(0);
        let p = DimensionParameterId::new(1);
        let bound =
            DimensionExpr::Max([DimensionExpr::Parameter(q), DimensionExpr::Constant(3)].into());
        let declarations = [
            DimensionParameterDeclaration {
                id: q,
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Activation,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: None,
            },
            DimensionParameterDeclaration {
                id: p,
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Activation,
                lower_bound: bound.clone(),
                upper_bound: Some(bound),
            },
        ];
        let target = SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: [DimensionExpr::Parameter(p)].into(),
        };
        let source = |extent| SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: [DimensionExpr::Constant(extent)].into(),
        };
        assert_eq!(
            solve_reified_target_bindings(&source(5), &target, &declarations).unwrap(),
            vec![
                Some(DimensionExpr::Constant(5)),
                Some(DimensionExpr::Constant(5)),
            ]
        );
        let ambiguous = solve_reified_target_bindings(&source(3), &target, &declarations).unwrap();
        assert!(validate_reified_parameter_bindings(&declarations, &ambiguous).is_err());
    }

    #[test]
    fn non_affine_exact_bound_narrows_two_unbounded_dependencies() {
        let p = DimensionParameterId::new(0);
        let q = DimensionParameterId::new(1);
        let r = DimensionParameterId::new(2);
        let product = DimensionExpr::Multiply(
            [DimensionExpr::Parameter(q), DimensionExpr::Parameter(r)].into(),
        );
        let declarations = [
            DimensionParameterDeclaration {
                id: p,
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Activation,
                lower_bound: product.clone(),
                upper_bound: Some(product),
            },
            DimensionParameterDeclaration {
                id: q,
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Activation,
                lower_bound: DimensionExpr::Constant(3),
                upper_bound: None,
            },
            DimensionParameterDeclaration {
                id: r,
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Activation,
                lower_bound: DimensionExpr::Constant(2),
                upper_bound: None,
            },
        ];
        let matrix = |extent| SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: [extent].into(),
        };
        let target = matrix(DimensionExpr::Parameter(p));
        let bindings = solve_reified_target_bindings(
            &matrix(DimensionExpr::Constant(6)),
            &target,
            &declarations,
        )
        .unwrap();
        assert_eq!(
            bindings,
            [6, 3, 2].map(|extent| Some(DimensionExpr::Constant(extent)))
        );
        validate_reified_parameter_bindings(&declarations, &bindings).unwrap();
        let impossible = solve_reified_target_bindings(
            &matrix(DimensionExpr::Constant(5)),
            &target,
            &declarations,
        )
        .unwrap();
        assert!(validate_reified_parameter_bindings(&declarations, &impossible).is_err());
    }

    #[test]
    fn bound_body_witness_narrows_an_unbounded_dependency() {
        let q = DimensionParameterId::new(0);
        let p = DimensionParameterId::new(1);
        let declarations = [
            DimensionParameterDeclaration {
                id: q,
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Activation,
                lower_bound: DimensionExpr::Constant(5),
                upper_bound: None,
            },
            DimensionParameterDeclaration {
                id: p,
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Activation,
                lower_bound: DimensionExpr::Parameter(q),
                upper_bound: Some(DimensionExpr::Add(
                    [DimensionExpr::Parameter(q), DimensionExpr::Constant(1)].into(),
                )),
            },
        ];
        let source = SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: [DimensionExpr::Constant(5)].into(),
        };
        let target = SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: [DimensionExpr::Parameter(p)].into(),
        };
        let bindings = solve_reified_target_bindings(&source, &target, &declarations).unwrap();
        assert_eq!(
            bindings,
            vec![
                Some(DimensionExpr::Constant(5)),
                Some(DimensionExpr::Constant(5)),
            ]
        );
        validate_reified_parameter_bindings(&declarations, &bindings).unwrap();
    }

    #[test]
    fn repeated_binding_work_is_bounded_by_target_size() {
        let count = MAX_REIFIED_BINDING_PARAMETERS;
        let declarations = (0..count)
            .map(|index| DimensionParameterDeclaration {
                id: DimensionParameterId::new(index as u32),
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Activation,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: None,
            })
            .collect::<Vec<_>>();
        let mut source_axes = vec![DimensionExpr::Constant(1); 9_000];
        let mut target_axes = source_axes.clone();
        for index in 0..count {
            source_axes.push(DimensionExpr::Constant(if index + 1 == count {
                1
            } else {
                2
            }));
            let parameter = DimensionExpr::Parameter(DimensionParameterId::new(index as u32));
            target_axes.push(if index + 1 == count {
                parameter
            } else {
                DimensionExpr::Add(
                    [
                        parameter,
                        DimensionExpr::Parameter(DimensionParameterId::new((index + 1) as u32)),
                    ]
                    .into(),
                )
            });
        }
        let matrix = |dimensions: Vec<DimensionExpr>| SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: dimensions.into_boxed_slice(),
        };
        assert!(
            solve_reified_target_bindings(
                &matrix(source_axes),
                &matrix(target_axes),
                &declarations,
            )
            .is_err()
        );
    }

    #[test]
    fn joint_reified_solver_rejects_oversized_sparse_system_before_dense_rows() {
        let parameters = (0..65).map(DimensionParameterId::new).collect::<Vec<_>>();
        let declarations = parameters
            .iter()
            .map(|id| DimensionParameterDeclaration {
                id: *id,
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Activation,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: None,
            })
            .collect::<Vec<_>>();
        let source = SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: [DimensionExpr::Constant(5)].into(),
        };
        let target = SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: [DimensionExpr::Add(
                parameters
                    .iter()
                    .copied()
                    .map(DimensionExpr::Parameter)
                    .collect(),
            )]
            .into(),
        };
        assert!(solve_reified_target_bindings(&source, &target, &declarations).is_err());
    }

    #[test]
    fn fixed_reified_bound_determines_other_compound_parameter() {
        let p = DimensionParameterId::new(0);
        let q = DimensionParameterId::new(1);
        let declarations = [
            DimensionParameterDeclaration {
                id: p,
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Activation,
                lower_bound: DimensionExpr::Constant(2),
                upper_bound: Some(DimensionExpr::Constant(2)),
            },
            DimensionParameterDeclaration {
                id: q,
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Activation,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: None,
            },
        ];
        let target = SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: [DimensionExpr::Add(
                [DimensionExpr::Parameter(p), DimensionExpr::Parameter(q)].into(),
            )]
            .into(),
        };
        let source = SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: [DimensionExpr::Constant(5)].into(),
        };
        let bindings = solve_reified_target_bindings(&source, &target, &declarations).unwrap();
        assert_eq!(
            bindings,
            vec![
                Some(DimensionExpr::Constant(2)),
                Some(DimensionExpr::Constant(3)),
            ],
        );
        validate_reified_parameter_bindings(&declarations, &bindings).unwrap();
    }

    #[test]
    fn dependent_fixed_reified_bounds_determine_compound_parameters() {
        let p = DimensionParameterId::new(0);
        let q = DimensionParameterId::new(1);
        let r = DimensionParameterId::new(2);
        let exact_q =
            DimensionExpr::Add([DimensionExpr::Parameter(p), DimensionExpr::Constant(1)].into());
        let declarations = [
            DimensionParameterDeclaration {
                id: p,
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Activation,
                lower_bound: DimensionExpr::Constant(2),
                upper_bound: Some(DimensionExpr::Constant(2)),
            },
            DimensionParameterDeclaration {
                id: q,
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Activation,
                lower_bound: exact_q.clone(),
                upper_bound: Some(exact_q),
            },
            DimensionParameterDeclaration {
                id: r,
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Activation,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: None,
            },
        ];
        let target = SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: [DimensionExpr::Add(
                [DimensionExpr::Parameter(q), DimensionExpr::Parameter(r)].into(),
            )]
            .into(),
        };
        let source = SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: [DimensionExpr::Constant(6)].into(),
        };
        let bindings = solve_reified_target_bindings(&source, &target, &declarations).unwrap();
        assert_eq!(
            bindings,
            vec![
                Some(DimensionExpr::Constant(2)),
                Some(DimensionExpr::Constant(3)),
                Some(DimensionExpr::Constant(3)),
            ],
        );
        validate_reified_parameter_bindings(&declarations, &bindings).unwrap();
    }

    #[test]
    fn semantic_reified_conversion_retains_source_axis_provenance() {
        let source_id = DimensionParameterId::new(0);
        let source = KindExpr::Matrix {
            element: Box::new(KindExpr::Index),
            dimensions: [DimensionExpr::Parameter(source_id)].into(),
        };
        let target = SchemaBody::Matrix {
            element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
            dimensions: [DimensionExpr::Constant(8)].into(),
        };
        assert_eq!(
            materialize_conversion_semantic_shape(&source, &target, true),
            SchemaBody::Matrix {
                element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
                dimensions: [DimensionExpr::Parameter(source_id)].into(),
            },
        );
    }

    #[test]
    fn shared_compound_reified_axes_bind_regardless_of_occurrence_order() {
        let id = DimensionParameterId::new(0);
        let declaration = DimensionParameterDeclaration {
            id,
            origin: DimensionParameterOrigin::Inferred,
            lifetime: DimensionLifetime::Activation,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: Some(DimensionExpr::Constant(10)),
        };
        let compound =
            DimensionExpr::Min([DimensionExpr::Parameter(id), DimensionExpr::Constant(5)].into());
        for (source_axes, target_axes) in [
            (
                [DimensionExpr::Constant(5), DimensionExpr::Constant(7)],
                [compound.clone(), DimensionExpr::Parameter(id)],
            ),
            (
                [DimensionExpr::Constant(7), DimensionExpr::Constant(5)],
                [DimensionExpr::Parameter(id), compound.clone()],
            ),
        ] {
            let source = SchemaBody::Matrix {
                element: Box::new(SchemaBody::Index),
                dimensions: source_axes.into(),
            };
            let target = SchemaBody::Matrix {
                element: Box::new(SchemaBody::Index),
                dimensions: target_axes.into(),
            };
            let bindings = solve_reified_target_bindings(&source, &target, &[declaration.clone()])
                .expect("the bare axis determines the shared witness");
            assert_eq!(bindings, vec![Some(DimensionExpr::Constant(7))]);
            validate_reified_parameter_bindings(&[declaration.clone()], &bindings).unwrap();
        }
    }

    #[test]
    fn shared_compound_reified_axis_waits_for_multiple_later_witnesses() {
        let p = DimensionParameterId::new(0);
        let q = DimensionParameterId::new(1);
        let declarations = [p, q].map(|id| DimensionParameterDeclaration {
            id,
            origin: DimensionParameterOrigin::Inferred,
            lifetime: DimensionLifetime::Activation,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: Some(DimensionExpr::Constant(20)),
        });
        let source = SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: [
                DimensionExpr::Constant(12),
                DimensionExpr::Constant(5),
                DimensionExpr::Constant(7),
            ]
            .into(),
        };
        let target = SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: [
                DimensionExpr::Add(
                    [DimensionExpr::Parameter(p), DimensionExpr::Parameter(q)].into(),
                ),
                DimensionExpr::Parameter(p),
                DimensionExpr::Parameter(q),
            ]
            .into(),
        };
        assert_eq!(
            solve_reified_target_bindings(&source, &target, &declarations).unwrap(),
            vec![
                Some(DimensionExpr::Constant(5)),
                Some(DimensionExpr::Constant(7)),
            ]
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
            substitute_reified_target(&target, None, &bindings).unwrap(),
            source
        );
    }

    #[test]
    fn inherited_dynamic_bound_keeps_its_source_dimension_parameter() {
        let id = DimensionParameterId::new(0);
        let source = SchemaBody::Set {
            element: Box::new(SchemaBody::Index),
            cardinality: CardinalitySpec::Dynamic {
                upper_bound: Some(DimensionExpr::Parameter(id)),
            },
        };
        let mut target = SchemaBody::Set {
            element: Box::new(SchemaBody::Index),
            cardinality: CardinalitySpec::Exact(DimensionExpr::Parameter(id)),
        };
        let declared_target = target.clone();
        let declaration = DimensionParameterDeclaration {
            id,
            origin: DimensionParameterOrigin::Inferred,
            lifetime: DimensionLifetime::Activation,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: None,
        };
        inherit_reified_dynamic_cardinality(&source, &mut target, &[declaration.clone()]).unwrap();
        let bindings = solve_reified_target_bindings_with_declared(
            &source,
            &target,
            &declared_target,
            &[declaration],
        )
        .unwrap();
        assert_eq!(
            substitute_reified_target(&target, Some(&declared_target), &bindings).unwrap(),
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
        let mut target = SchemaBody::Tuple([set(exact.clone()), set(exact.clone())].into());
        assert!(inherit_reified_dynamic_cardinality(&source, &mut target, &[declaration]).is_err());

        let declaration = DimensionParameterDeclaration {
            id,
            origin: DimensionParameterOrigin::Inferred,
            lifetime: DimensionLifetime::Activation,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: None,
        };
        let mut target = SchemaBody::Tuple(
            [
                set(exact),
                set(CardinalitySpec::Dynamic {
                    upper_bound: Some(DimensionExpr::Parameter(id)),
                }),
            ]
            .into(),
        );
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
        let target = ValueCell::from_schema_data(
            SchemaBody::ReifiedType,
            ValueDataDraft::Type(ReifiedTypeDraft::CanonicalKind(
                kind.canonical_bytes().to_vec().into_boxed_slice(),
            )),
        )
        .unwrap();
        let invocation = SpecializationInvocation::from_cells(
            vec![source.clone(), target.clone()].into_boxed_slice(),
        );
        let operation = ResolvedOperationDescriptor::from_name(
            "convert/kind",
            PURE_TYPE_CONVERSION_CONTRACT.clone(),
        )
        .unwrap();
        let mut context =
            SpecializationContext::for_syntax_directed_invocation(&invocation, None, operation)
                .unwrap();
        let conversion = ConvertKind
            .specialize_invocation(&invocation, &mut context)
            .unwrap();
        conversion.instance().solve_result().unwrap();
        assert!(
            RuntimeReifiedKindConversion::new_invocation(FunctionInvocation::binary(
                conversion.output().clone(),
                source.clone(),
                target,
            ))
            .is_ok()
        );
        let converted = convert_reified(source, kind).unwrap();
        assert_eq!(converted.closed_schema_body().unwrap(), schema);
    }

    #[test]
    fn product_bound_reified_matrix_converts_direct_and_runtime() {
        let (id, path) = builtin_scalar_named_kind(mech_core::hash_str("u8")).unwrap();
        let named = NamedKinds(BTreeMap::from([(id, path)]));
        let p = DimensionParameterId::new(0);
        let q = DimensionParameterId::new(1);
        let r = DimensionParameterId::new(2);
        let product = DimensionExpr::Multiply(
            [DimensionExpr::Parameter(q), DimensionExpr::Parameter(r)].into(),
        );
        let declarations = [
            DimensionParameterDeclaration {
                id: p,
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Activation,
                lower_bound: product.clone(),
                upper_bound: Some(product),
            },
            DimensionParameterDeclaration {
                id: q,
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Activation,
                lower_bound: DimensionExpr::Constant(3),
                upper_bound: None,
            },
            DimensionParameterDeclaration {
                id: r,
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Activation,
                lower_bound: DimensionExpr::Constant(2),
                upper_bound: None,
            },
        ];
        let kind = ReifiedKind::from_closed_kind(
            &KindExpr::Matrix {
                element: Box::new(KindExpr::Named(id)),
                dimensions: [DimensionExpr::Parameter(p), DimensionExpr::Constant(1)].into(),
            },
            &declarations,
            &named,
        )
        .unwrap();
        let source = ValueCell::from_schema_data(
            SchemaBody::Matrix {
                element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
                dimensions: [DimensionExpr::Constant(6), DimensionExpr::Constant(1)].into(),
            },
            ValueDataDraft::Matrix((0..6).map(ValueDataDraft::U8).collect()),
        )
        .unwrap();
        let target = ValueCell::from_schema_data(
            SchemaBody::ReifiedType,
            ValueDataDraft::Type(ReifiedTypeDraft::CanonicalKind(
                kind.canonical_bytes().to_vec().into_boxed_slice(),
            )),
        )
        .unwrap();
        let invocation = SpecializationInvocation::from_cells(
            vec![source.clone(), target.clone()].into_boxed_slice(),
        );
        let operation = ResolvedOperationDescriptor::from_name(
            "convert/kind",
            PURE_TYPE_CONVERSION_CONTRACT.clone(),
        )
        .unwrap();
        let mut context =
            SpecializationContext::for_syntax_directed_invocation(&invocation, None, operation)
                .unwrap();
        let conversion = ConvertKind
            .specialize_invocation(&invocation, &mut context)
            .unwrap();
        conversion.instance().solve_result().unwrap();
        assert!(
            RuntimeReifiedKindConversion::new_invocation(FunctionInvocation::binary(
                conversion.output().clone(),
                source.clone(),
                target,
            ))
            .is_ok()
        );
        assert_eq!(
            convert_reified(source, kind)
                .unwrap()
                .current_top_level_extents()
                .unwrap()
                .as_ref(),
            &[6, 1]
        );
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
        let substituted = substitute_reified_target(&target, None, &bindings).unwrap();
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
