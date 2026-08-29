use crate::*;
#[cfg(feature = "kind_annotation")]
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
            let legacy_id = identifier.hash();
            if let Ok((id, path)) = builtin_scalar_named_kind(legacy_id) {
                named.0.insert(id, path);
                KindExpr::Named(id)
            } else if p.state.borrow().enums.contains_key(&legacy_id) {
                let path = source_nominal_path(&identifier.to_string())?;
                KindExpr::Enum(NominalKey::from_path(NominalKind::Enum, &path))
            } else {
                return Err(SemanticModelError::LegacyNamedKindUnresolved { legacy_id }.into());
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
    convert_literal_cell(value, &target)
}

#[cfg(feature = "convert")]
pub(crate) fn convert_literal_cell(value: ValueCell, target: &SchemaBody) -> MResult<ValueCell> {
    if value.closed_schema_body()? == *target {
        return Ok(value);
    }
    if let SchemaBody::Matrix {
        element: target_element,
        dimensions: target_dimensions,
    } = target
        && let Some(elements) = value.matrix_elements()?
    {
        let shape = value.shape().parameter_values().to_vec();
        if !target_dimensions.is_empty()
            && (target_dimensions.len() != shape.len()
                || target_dimensions
                    .iter()
                    .zip(&shape)
                    .any(|(expected, actual)| {
                        matches!(expected, DimensionExpr::Constant(expected) if expected != actual)
                    }))
        {
            return Err(MechError::new(
                CanonicalKindConversionUnsupported {
                    source: value.closed_schema_body()?,
                    target: target.clone(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let converted = elements
            .into_iter()
            .map(|element| convert_literal_cell(element, target_element))
            .collect::<MResult<Vec<_>>>()?;
        let [rows, columns] = shape.as_slice() else {
            return Err(MechError::new(
                CanonicalKindConversionUnsupported {
                    source: value.closed_schema_body()?,
                    target: target.clone(),
                },
                None,
            )
            .with_compiler_loc());
        };
        return ValueCell::dynamic_matrix_from_cells(
            usize::try_from(*rows).map_err(|_| {
                MechError::new(ExpectedNumericForKindSizeError, None).with_compiler_loc()
            })?,
            usize::try_from(*columns).map_err(|_| {
                MechError::new(ExpectedNumericForKindSizeError, None).with_compiler_loc()
            })?,
            &converted,
        );
    }
    let snapshot = value.snapshot()?;
    let numeric = match snapshot.data() {
        ValueData::U8(value) => Some(*value as f64),
        ValueData::U16(value) => Some(*value as f64),
        ValueData::U32(value) => Some(*value as f64),
        ValueData::U64(value) => Some(*value as f64),
        ValueData::U128(value) => Some(*value as f64),
        ValueData::I8(value) => Some(*value as f64),
        ValueData::I16(value) => Some(*value as f64),
        ValueData::I32(value) => Some(*value as f64),
        ValueData::I64(value) => Some(*value as f64),
        ValueData::I128(value) => Some(*value as f64),
        ValueData::F32(value) => Some(value.to_f32() as f64),
        ValueData::F64(value) => Some(value.to_f64()),
        _ => None,
    };
    if let (Some(number), SchemaBody::String) = (numeric, target) {
        return ValueCell::from_exact(number.to_string());
    }
    let converted = match (numeric, target) {
        #[cfg(feature = "u8")]
        (Some(number), SchemaBody::UnsignedInteger(IntegerWidth::W8)) => {
            ValueCell::from_exact(number as u8)
        }
        #[cfg(feature = "u16")]
        (Some(number), SchemaBody::UnsignedInteger(IntegerWidth::W16)) => {
            ValueCell::from_exact(number as u16)
        }
        #[cfg(feature = "u32")]
        (Some(number), SchemaBody::UnsignedInteger(IntegerWidth::W32)) => {
            ValueCell::from_exact(number as u32)
        }
        #[cfg(feature = "u64")]
        (Some(number), SchemaBody::UnsignedInteger(IntegerWidth::W64)) => {
            ValueCell::from_exact(number as u64)
        }
        #[cfg(feature = "u128")]
        (Some(number), SchemaBody::UnsignedInteger(IntegerWidth::W128)) => {
            ValueCell::from_exact(number as u128)
        }
        #[cfg(feature = "i8")]
        (Some(number), SchemaBody::SignedInteger(IntegerWidth::W8)) => {
            ValueCell::from_exact(number as i8)
        }
        #[cfg(feature = "i16")]
        (Some(number), SchemaBody::SignedInteger(IntegerWidth::W16)) => {
            ValueCell::from_exact(number as i16)
        }
        #[cfg(feature = "i32")]
        (Some(number), SchemaBody::SignedInteger(IntegerWidth::W32)) => {
            ValueCell::from_exact(number as i32)
        }
        #[cfg(feature = "i64")]
        (Some(number), SchemaBody::SignedInteger(IntegerWidth::W64)) => {
            ValueCell::from_exact(number as i64)
        }
        #[cfg(feature = "i128")]
        (Some(number), SchemaBody::SignedInteger(IntegerWidth::W128)) => {
            ValueCell::from_exact(number as i128)
        }
        #[cfg(feature = "f32")]
        (Some(number), SchemaBody::FloatingPoint(FloatWidth::W32)) => {
            ValueCell::from_exact(number as f32)
        }
        #[cfg(feature = "f64")]
        (Some(number), SchemaBody::FloatingPoint(FloatWidth::W64)) => ValueCell::from_exact(number),
        _ => Err(MechError::new(
            CanonicalKindConversionUnsupported {
                source: value.closed_schema_body()?,
                target: target.clone(),
            },
            None,
        )
        .with_compiler_loc()),
    }?;
    Ok(converted)
}

#[cfg(feature = "convert")]
#[derive(Debug)]
struct CanonicalKindConversion {
    source: ValueCell,
    output: ValueCell,
    target: SchemaBody,
}

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
                match name.as_str() {
                    "u8" => SchemaBody::UnsignedInteger(IntegerWidth::W8),
                    "u16" => SchemaBody::UnsignedInteger(IntegerWidth::W16),
                    "u32" => SchemaBody::UnsignedInteger(IntegerWidth::W32),
                    "u64" => SchemaBody::UnsignedInteger(IntegerWidth::W64),
                    "u128" => SchemaBody::UnsignedInteger(IntegerWidth::W128),
                    "i8" => SchemaBody::SignedInteger(IntegerWidth::W8),
                    "i16" => SchemaBody::SignedInteger(IntegerWidth::W16),
                    "i32" => SchemaBody::SignedInteger(IntegerWidth::W32),
                    "i64" => SchemaBody::SignedInteger(IntegerWidth::W64),
                    "i128" => SchemaBody::SignedInteger(IntegerWidth::W128),
                    "f32" => SchemaBody::FloatingPoint(FloatWidth::W32),
                    "f64" => SchemaBody::FloatingPoint(FloatWidth::W64),
                    "c64" => SchemaBody::Complex(FloatWidth::W64),
                    "r64" => SchemaBody::Rational64,
                    "string" => SchemaBody::String,
                    "bool" => SchemaBody::Bool,
                    _ => return Err(aggregate_error()),
                }
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
        let output = convert_literal_cell(source.clone(), &target)?;
        let bound = FunctionInvocation::binary(output.clone(), source.clone(), target_cell);
        Ok(SpecializedFunction::new(FunctionInstance::new(
            Box::new(CanonicalKindConversion {
                source,
                output,
                target,
            }),
            bound,
        )))
    }
}

#[cfg(feature = "convert")]
impl MechFunctionImpl for CanonicalKindConversion {
    fn solve_result(&self) -> MResult<()> {
        let replacement = convert_literal_cell(self.source.clone(), &self.target)?;
        self.output.replace(&replacement.snapshot()?)
    }

    fn semantic_operation_name(&self) -> Option<&str> {
        Some("convert/kind")
    }

    fn to_string(&self) -> String {
        "CanonicalKindConversion".to_owned()
    }
}

#[cfg(all(feature = "convert", feature = "semantic-compiler"))]
impl MechFunctionCompiler for CanonicalKindConversion {
    fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Err(MechError::new(
            GenericError {
                msg: "canonical kind conversion cannot yet be emitted as bytecode".to_owned(),
            },
            None,
        )
        .with_compiler_loc())
    }
}

/// Builds one reactive, schema-directed conversion without routing semantic
/// values through the retired universal value representation.
#[cfg(feature = "convert")]
pub(crate) fn convert_cell_reactively(
    value: ValueCell,
    target: SchemaBody,
    interpreter: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    if value.closed_schema_body()? == target {
        return Ok(value);
    }
    let output = convert_literal_cell(value.clone(), &target)?;
    interpreter.plan().register_instance(FunctionInstance::new(
        Box::new(CanonicalKindConversion {
            source: value.clone(),
            output: output.clone(),
            target,
        }),
        FunctionInvocation::unary(output.clone(), value),
    ))?;
    Ok(output)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalKindConversionUnsupported {
    pub source: SchemaBody,
    pub target: SchemaBody,
}

impl MechErrorKind for CanonicalKindConversionUnsupported {
    fn name(&self) -> &str {
        "CanonicalKindConversionUnsupported"
    }

    fn message(&self) -> String {
        format!(
            "canonical conversion from {:?} to {:?} is unsupported",
            self.source, self.target
        )
    }
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
