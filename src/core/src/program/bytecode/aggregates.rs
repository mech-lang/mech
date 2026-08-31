//! Canonical bytecode aggregate reconstruction.
use crate::snapshot::{
    OptionDraft, ReifiedKind, ReifiedType, SequenceView, SnapshotValidationContext,
};
use crate::{
    BytecodeValidationError, DimensionExpr, FloatWidth, IntegerWidth, KindExpr, KindId, MResult,
    MechError, SchemaBody, Value, ValueCell, ValueData, ValueDataDraft,
};
use std::collections::BTreeMap;

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, format, vec::Vec};

fn wrong_arity(kind: &str, expected: usize, actual: usize) -> MechError {
    MechError::new(
        BytecodeValidationError {
            reason: format!("{kind} CompositePack expects {expected} children, found {actual}"),
        },
        None,
    )
    .with_compiler_loc()
}

fn canonical_sequence_values(sequence: SequenceView<'_>) -> Vec<ValueData> {
    macro_rules! values {
        ($values:expr, $variant:ident) => {
            $values.iter().cloned().map(ValueData::$variant).collect()
        };
    }
    match sequence {
        SequenceView::U8(values) => values!(values, U8),
        SequenceView::U16(values) => values!(values, U16),
        SequenceView::U32(values) => values!(values, U32),
        SequenceView::U64(values) => values!(values, U64),
        SequenceView::U128(values) => values!(values, U128),
        SequenceView::I8(values) => values!(values, I8),
        SequenceView::I16(values) => values!(values, I16),
        SequenceView::I32(values) => values!(values, I32),
        SequenceView::I64(values) => values!(values, I64),
        SequenceView::I128(values) => values!(values, I128),
        SequenceView::F32(values) => values!(values, F32),
        SequenceView::F64(values) => values!(values, F64),
        SequenceView::Complex32(values) => values!(values, Complex32),
        SequenceView::Complex64(values) => values!(values, Complex64),
        SequenceView::Rational64(values) => values!(values, Rational64),
        SequenceView::Bool(values) => values!(values, Bool),
        SequenceView::String(values) => values.iter().cloned().map(ValueData::String).collect(),
        SequenceView::Id(values) => values!(values, Id),
        SequenceView::Index(values) => values!(values, Index),
        SequenceView::Unit(count) => (0..count).map(|_| ValueData::Atom).collect(),
        SequenceView::Values(values) => values.to_vec(),
    }
}

/// Direct immutable children whose registers define a canonical composite.
pub fn canonical_bytecode_composite_children(value: &Value) -> Option<Vec<ValueData>> {
    match value.data() {
        ValueData::Tuple(values) => Some(values.to_vec()),
        ValueData::Record(value) => Some(value.fields().to_vec()),
        ValueData::Matrix(value) => Some(canonical_sequence_values(value.elements())),
        ValueData::Table(value) => Some(
            (0..value.len())
                .flat_map(|column| {
                    canonical_sequence_values(
                        value
                            .column(column)
                            .expect("canonical table column remains present"),
                    )
                })
                .collect(),
        ),
        ValueData::Set(value) => Some(
            value
                .elements()
                .iter()
                .map(|element| element.data().clone())
                .collect(),
        ),
        ValueData::Map(value) => Some(
            value
                .entries()
                .iter()
                .flat_map(|entry| [entry.key().data().clone(), entry.value().clone()])
                .collect(),
        ),
        ValueData::Enum(value) => Some(value.payload().cloned().into_iter().collect()),
        ValueData::Option(value) => Some(value.as_deref().cloned().into_iter().collect()),
        _ => None,
    }
}

/// Rebuilds one canonical composite layer from a constant template and live
/// register values. Child schemas are validated by the template's schema.
pub fn rebuild_canonical_bytecode_composite(
    template: &Value,
    children: Vec<Value>,
) -> MResult<Value> {
    let schemas = template.schemas().ok_or_else(|| {
        MechError::new(
            BytecodeValidationError {
                reason: "CompositePack template has no canonical schema context".into(),
            },
            None,
        )
        .with_compiler_loc()
    })?;
    let context = SnapshotValidationContext::new(&schemas);
    let data = children
        .iter()
        .map(|child| child.data().clone())
        .collect::<Vec<_>>();
    let rebuilt = match template.data() {
        ValueData::Tuple(_) => template.rebuild_tuple(data.into_boxed_slice(), &context),
        ValueData::Record(_) => template.rebuild_record(data.into_boxed_slice(), &context),
        ValueData::Matrix(_) => template.rebuild_matrix(data.into_boxed_slice(), &context),
        ValueData::Set(_) => template.rebuild_set(data.into_boxed_slice(), &context),
        ValueData::Map(_) => {
            if data.len() % 2 != 0 {
                return Err(wrong_arity("Map", data.len() + 1, data.len()));
            }
            let entries = data
                .chunks_exact(2)
                .map(|entry| (entry[0].clone(), entry[1].clone()))
                .collect::<Vec<_>>()
                .into_boxed_slice();
            template.rebuild_map(entries, &context)
        }
        ValueData::Table(table) => {
            let mut offset = 0_usize;
            let mut columns = Vec::with_capacity(table.len());
            for index in 0..table.len() {
                let len = canonical_sequence_values(
                    table
                        .column(index)
                        .expect("canonical table column remains present"),
                )
                .len();
                let end = offset.saturating_add(len);
                if end > data.len() {
                    return Err(wrong_arity("Table", end, data.len()));
                }
                columns.push(data[offset..end].to_vec().into_boxed_slice());
                offset = end;
            }
            if offset != data.len() {
                return Err(wrong_arity("Table", offset, data.len()));
            }
            template.rebuild_table(columns.into_boxed_slice(), &context)
        }
        ValueData::Enum(value) => {
            if data.len() != usize::from(value.payload().is_some()) {
                return Err(wrong_arity(
                    "Enum",
                    usize::from(value.payload().is_some()),
                    data.len(),
                ));
            }
            template.rebuild_enum(value.ordinal(), data.into_iter().next(), &context)
        }
        ValueData::Option(value) => {
            if data.len() != usize::from(value.is_some()) {
                return Err(wrong_arity(
                    "Option",
                    usize::from(value.is_some()),
                    data.len(),
                ));
            }
            template.rebuild_option(data.into_iter().next(), &context)
        }
        ValueData::Type(ReifiedType::Kind(kind)) => {
            return rebuild_reified_composite_template(kind, children);
        }
        _ => {
            return Err(MechError::new(
                BytecodeValidationError {
                    reason: "CompositePack template is not a canonical composite value".into(),
                },
                None,
            )
            .with_compiler_loc());
        }
    };
    rebuilt.map_err(|error| {
        MechError::new(
            BytecodeValidationError {
                reason: format!("canonical CompositePack validation failed: {error:?}"),
            },
            None,
        )
        .with_compiler_loc()
    })
}

fn rebuild_reified_composite_template(kind: &ReifiedKind, children: Vec<Value>) -> MResult<Value> {
    let (kind, _, named) = kind.decoded_closed_kind().map_err(|error| {
        MechError::new(
            BytecodeValidationError {
                reason: format!("invalid canonical CompositePack kind template: {error:?}"),
            },
            None,
        )
        .with_compiler_loc()
    })?;
    let KindExpr::Matrix {
        element,
        dimensions,
    } = kind
    else {
        return Err(non_composite_template());
    };
    let [rows, columns] = dimensions.as_ref() else {
        return Err(non_composite_template());
    };
    let (DimensionExpr::Constant(rows), DimensionExpr::Constant(columns)) = (rows, columns) else {
        return Err(non_composite_template());
    };
    let expected = usize::try_from(*rows)
        .ok()
        .and_then(|rows| {
            usize::try_from(*columns)
                .ok()
                .and_then(|columns| rows.checked_mul(columns))
        })
        .ok_or_else(|| wrong_arity("Matrix", usize::MAX, children.len()))?;
    if expected != children.len() {
        return Err(wrong_arity("Matrix", expected, children.len()));
    }
    let element = if matches!(element.as_ref(), KindExpr::Wildcard) {
        infer_matrix_element_schema(&children)?
    } else {
        schema_body_from_closed_kind(&element, &named)?
    };
    let elements = children
        .iter()
        .enumerate()
        .map(|(index, child)| matrix_child_draft(index, &element, child))
        .collect::<MResult<Vec<_>>>()?
        .into_boxed_slice();
    ValueCell::from_schema_data(
        SchemaBody::Matrix {
            element: Box::new(element),
            dimensions,
        },
        ValueDataDraft::Matrix(elements),
    )?
    .snapshot()
}

fn infer_matrix_element_schema(children: &[Value]) -> MResult<SchemaBody> {
    let unit = SchemaBody::Tuple(Box::new([]));
    let mut inferred = None::<SchemaBody>;
    let mut saw_absent = false;
    for child in children {
        let schemas = child.schemas().ok_or_else(non_composite_template)?;
        let body = schemas
            .get(child.schema())
            .ok_or_else(non_composite_template)?
            .body()
            .clone();
        if body == unit {
            saw_absent = true;
            continue;
        }
        match &inferred {
            Some(existing) if existing != &body => {
                return Err(MechError::new(
                    BytecodeValidationError {
                        reason: "HeterogeneousMatrixLiteral: no single canonical element schema"
                            .into(),
                    },
                    None,
                )
                .with_compiler_loc());
            }
            Some(_) => {}
            None => inferred = Some(body),
        }
    }
    let inferred = inferred.unwrap_or(unit);
    Ok(if saw_absent && !children.is_empty() {
        SchemaBody::Option(Box::new(inferred))
    } else {
        inferred
    })
}

fn matrix_child_draft(
    index: usize,
    expected: &SchemaBody,
    child: &Value,
) -> MResult<ValueDataDraft> {
    let schemas = child.schemas().ok_or_else(non_composite_template)?;
    let actual = schemas
        .get(child.schema())
        .ok_or_else(non_composite_template)?
        .body();
    if let SchemaBody::Option(inner) = expected {
        if matches!(actual, SchemaBody::Tuple(elements) if elements.is_empty()) {
            return Ok(ValueDataDraft::Option(OptionDraft {
                present: false,
                value: None,
            }));
        }
        if actual == inner.as_ref() {
            return Ok(ValueDataDraft::Option(OptionDraft {
                present: true,
                value: Some(Box::new(
                    child
                        .canonical_data_draft()
                        .map_err(canonical_template_error)?,
                )),
            }));
        }
    }
    if actual != expected {
        return Err(MechError::new(
            BytecodeValidationError {
                reason: format!(
                    "HeterogeneousMatrixLiteral: child {index} has schema {actual:?}, expected {expected:?}",
                ),
            },
            None,
        )
        .with_compiler_loc());
    }
    child
        .canonical_data_draft()
        .map_err(canonical_template_error)
}

fn canonical_template_error(error: crate::SnapshotValueError) -> MechError {
    MechError::new(
        BytecodeValidationError {
            reason: format!("canonical CompositePack template conversion failed: {error:?}"),
        },
        None,
    )
    .with_compiler_loc()
}

fn non_composite_template() -> MechError {
    MechError::new(
        BytecodeValidationError {
            reason: "CompositePack template is not a canonical composite value".into(),
        },
        None,
    )
    .with_compiler_loc()
}

fn schema_body_from_closed_kind(
    kind: &KindExpr,
    named: &BTreeMap<KindId, crate::CanonicalNominalPath>,
) -> MResult<SchemaBody> {
    let body = match kind {
        KindExpr::Named(id) => {
            let path = named.get(id).ok_or_else(non_composite_template)?;
            let [mech, builtin, scalar, name] = path.segments() else {
                return Err(non_composite_template());
            };
            if (mech.as_str(), builtin.as_str(), scalar.as_str()) != ("mech", "builtin", "scalar") {
                return Err(non_composite_template());
            }
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
                _ => return Err(non_composite_template()),
            }
        }
        KindExpr::Id => SchemaBody::Id,
        KindExpr::Index => SchemaBody::Index,
        KindExpr::Option(inner) => {
            SchemaBody::Option(Box::new(schema_body_from_closed_kind(inner, named)?))
        }
        KindExpr::Tuple(elements) => SchemaBody::Tuple(
            elements
                .iter()
                .map(|element| schema_body_from_closed_kind(element, named))
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
        ),
        KindExpr::Matrix {
            element,
            dimensions,
        } => SchemaBody::Matrix {
            element: Box::new(schema_body_from_closed_kind(element, named)?),
            dimensions: dimensions.clone(),
        },
        KindExpr::Reference(inner) => schema_body_from_closed_kind(inner, named)?,
        KindExpr::Wildcard => return Err(non_composite_template()),
        _ => return Err(non_composite_template()),
    };
    Ok(body)
}
