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
    match &ltrl {
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
    if matches!(plan.step, ConversionStep::Identity) {
        return Ok(value.clone());
    }
    let draft = value.snapshot()?.canonical_data_draft().map_err(|error| {
        MechError::new(ValueCellSnapshotFailure { error }, None).with_compiler_loc()
    })?;
    let converted =
        execute_conversion_draft(draft, &plan.step).map_err(conversion_execution_error)?;
    ValueCell::from_schema_data(target.clone(), converted)?.with_resolved_output_type(&plan.target)
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
#[derive(Debug)]
struct PlannedTypeConversion {
    source: ValueCell,
    output: ValueCell,
    target: SchemaBody,
    plan: ConversionPlan,
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
    target: SchemaBody,
    plan: ConversionPlan,
}

#[cfg(feature = "convert")]
fn runtime_kind_conversion_plan(
    output: &ValueCell,
    source: &ValueCell,
) -> MResult<(SchemaBody, ConversionPlan)> {
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
    Ok((target, plan))
}

#[cfg(feature = "convert")]
impl MechFunctionFactory for RuntimeKindConversion {
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::unary(
        FunctionValueRepresentation::AnyValue,
        FunctionValueRepresentation::AnyValue,
    );

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (output, source) = invocation.expect_unary()?;
        let output = output.value();
        let source = source.value();
        let (target, plan) = runtime_kind_conversion_plan(output.cell(), source.cell())?;
        Ok(Box::new(Self {
            source,
            output,
            target,
            plan,
        }))
    }
}

#[cfg(feature = "convert")]
impl MechFunctionImpl for RuntimeKindConversion {
    fn solve_result(&self) -> MResult<()> {
        let replacement = execute_conversion_plan(self.source.cell(), &self.target, &self.plan)?;
        self.output.replace(&replacement.snapshot()?)
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
    package: "mech-engine", crate_name: "mech_engine",
    installer_path: "mech_engine::__mech_native::install_runtime_kind_conversion",
    extra_cargo_features: ["convert", "semantic-compiler"],
}

#[cfg(all(feature = "convert", feature = "semantic-compiler"))]
static PURE_TYPE_CONVERSION_CONTRACT: std::sync::LazyLock<OperationContractDeclaration> =
    std::sync::LazyLock::new(|| OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![InputPortPolicy {
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            }]
            .into_boxed_slice(),
        ),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::SameAsInput { input: 0 },
            },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    });

#[cfg(feature = "convert")]
fn schema_body_from_reified_kind(
    value: &ReifiedKind,
    context: &SpecializationContext<'_>,
) -> MResult<SchemaBody> {
    let (kind, _dimensions, named) = value.decoded_closed_kind().map_err(|error| {
        MechError::new(ValueCellSnapshotFailure { error }, None).with_compiler_loc()
    })?;

    fn schema(
        kind: &KindExpr,
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
                dimensions,
            } => SchemaBody::Matrix {
                element: Box::new(schema(element, named, context)?),
                dimensions: dimensions.clone(),
            },
            KindExpr::Option(element) => {
                SchemaBody::Option(Box::new(schema(element, named, context)?))
            }
            KindExpr::Tuple(elements) => SchemaBody::Tuple(
                elements
                    .iter()
                    .map(|element| schema(element, named, context))
                    .collect::<MResult<Vec<_>>>()?
                    .into_boxed_slice(),
            ),
            KindExpr::Record(fields) => SchemaBody::Record(
                fields
                    .iter()
                    .map(|field| {
                        Ok(SchemaField {
                            name: field.name.clone(),
                            schema: schema(&field.kind, named, context)?,
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
                            schema: schema(&column.kind, named, context)?,
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
                element: Box::new(schema(element, named, context)?),
                cardinality: cardinality(extent),
            },
            KindExpr::Map {
                key,
                value,
                cardinality: extent,
            } => SchemaBody::Map {
                key: Box::new(schema(key, named, context)?),
                value: Box::new(schema(value, named, context)?),
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

    schema(&kind, &named, context)
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
        let target = match target_value.data() {
            ValueData::Type(ReifiedType::Kind(kind)) => {
                schema_body_from_reified_kind(kind, context)?
            }
            ValueData::Type(ReifiedType::Schema(key)) => context.schema(*key)?.body().clone(),
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
        let semantic_target =
            materialize_declared_conversion_semantic_shape(source_type.kind(), &target);
        let target = materialize_declared_conversion_shape(&source.closed_schema_body()?, &target);
        let target_type =
            ResolvedType::from_schema_body(&semantic_target, source_type.dimension_parameters())
                .map_err(MechError::from)?;
        let plan = plan_explicit_cast(&source_type, &target_type).map_err(|error| {
            MechError::from(error.with_origin(TypeConstraintOrigin::new("convert/kind", None)))
        })?;
        let output = execute_conversion_plan(&source, &target, &plan)?;
        let bound = FunctionInvocation::binary(output.clone(), source.clone(), target_cell);
        Ok(SpecializedFunction::new(FunctionInstance::new(
            Box::new(PlannedTypeConversion {
                source,
                output,
                target,
                plan,
            }),
            bound,
        )))
    }
}

#[cfg(feature = "convert")]
impl MechFunctionImpl for PlannedTypeConversion {
    fn solve_result(&self) -> MResult<()> {
        let replacement = execute_conversion_plan(&self.source, &self.target, &self.plan)?;
        self.output.replace(&replacement.snapshot()?)
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
    interpreter.plan().register_instance(FunctionInstance::new(
        Box::new(PlannedTypeConversion {
            source: value.clone(),
            output: output.clone(),
            target,
            plan: plan.clone(),
        }),
        FunctionInvocation::unary(output.clone(), value),
    ))?;
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
    interpreter.plan().register_instance(FunctionInstance::new(
        Box::new(PlannedTypeConversion {
            source: value.clone(),
            output: output.clone(),
            target,
            plan,
        }),
        FunctionInvocation::unary(output.clone(), value),
    ))?;
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
    interpreter.plan().register_instance(FunctionInstance::new(
        Box::new(PlannedTypeConversion {
            source: value.clone(),
            output: output.clone(),
            target,
            plan,
        }),
        FunctionInvocation::unary(output.clone(), value),
    ))?;
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
        let mut context = SpecializationContext::for_invocation(&invocation, None).unwrap();

        let specialized = ConvertKind
            .specialize_invocation(&invocation, &mut context)
            .unwrap();

        assert!(matches!(
            specialized.output().snapshot().unwrap().data(),
            ValueData::U8(7)
        ));
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

    #[test]
    fn failed_reactive_conversion_leaves_the_output_unchanged() {
        let source = ValueCell::from_exact(12.9_f64).unwrap();
        let target = SchemaBody::SignedInteger(IntegerWidth::W32);
        let source_type = source.resolved_type().unwrap();
        let target_type = ResolvedType::from_schema_body(&target, &[]).unwrap();
        let plan = plan_explicit_cast(&source_type, &target_type).unwrap();
        let output = execute_conversion_plan(&source, &target, &plan).unwrap();
        let conversion = PlannedTypeConversion {
            source: source.clone(),
            output: output.clone(),
            target,
            plan,
        };

        source
            .replace(
                &ValueCell::from_exact(f64::INFINITY)
                    .unwrap()
                    .snapshot()
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(
            conversion.solve_result().unwrap_err().kind_name(),
            "ConversionNonFinite"
        );
        assert!(matches!(
            output.snapshot().unwrap().data(),
            ValueData::I32(12)
        ));
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
