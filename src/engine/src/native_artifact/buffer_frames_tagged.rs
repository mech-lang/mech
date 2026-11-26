//! Closed tagged owners use the same invocation-local frame bank. Maps remain
//! opaque canonical owners; this proof grants no map mutation or array access.
use super::*;

pub(super) fn opaque_map(body: &SchemaBody) -> bool {
    fn atom(body: &SchemaBody) -> bool {
        super::super::NativeScalarKind::from_schema_body(body).is_some()
            || matches!(
                body,
                SchemaBody::String | SchemaBody::Id | SchemaBody::Index | SchemaBody::Atom(_)
            )
    }
    matches!(body, SchemaBody::Map { key, value, .. } if atom(key) && atom(value))
        && mech_core::SchemaDraft {
            body: body.clone(),
            dimension_parameters: Box::new([]),
        }
        .finalize()
        .is_ok()
}

fn payload(body: &SchemaBody, depth: usize) -> Option<bool> {
    if depth >= 16 {
        return None;
    }
    if super::super::NativeScalarKind::from_schema_body(body).is_some() {
        return Some(false);
    }
    if opaque_map(body) {
        return Some(true);
    }
    if matches!(body, SchemaBody::Matrix { element, dimensions }
        if dimensions.len() == 2
            && super::super::NativeScalarKind::from_schema_body(element).is_some())
    {
        return Some(true);
    }
    let fields = match body {
        SchemaBody::Tuple(fields) if !fields.is_empty() && fields.len() <= 32 => {
            fields.iter().collect::<Vec<_>>()
        }
        SchemaBody::Record(fields) if !fields.is_empty() && fields.len() <= 32 => {
            fields.iter().map(|f| &f.schema).collect()
        }
        _ => return None,
    };
    let mut managed = false;
    for field in fields {
        managed |= payload(field, depth + 1)?;
    }
    Some(managed)
}

/// Every variant has a closed represented payload and at least one variant
/// carries a supported opaque map or numeric matrix owner. Pure scalar enums
/// retain their scalar owner.
/// Payload-free/open/nested enum layouts keep the ordinary canonical route.
pub(super) fn schemas(plan: &Plan, value: Value) -> Option<Vec<SchemaId>> {
    let definition = plan.values()[value.get() as usize].definition();
    if !definition.dimension_parameters().is_empty() {
        return None;
    }
    let SchemaBody::Enum { variants, .. } = definition.body() else {
        return None;
    };
    if variants.is_empty() || variants.len() > 32 {
        return None;
    }
    let mut managed = false;
    let mut result = Vec::with_capacity(variants.len());
    for variant in variants {
        let body = variant.payload.as_ref()?;
        managed |= payload(body, 0)?;
        let schema = plan
            .values()
            .iter()
            .find(|ty| {
                ty.definition().dimension_parameters().is_empty() && ty.definition().body() == body
            })?
            .schema();
        result.push(schema);
    }
    managed.then_some(result)
}

pub(super) fn pack(plan: &Plan, action: &Action) -> Option<(Value, Value, u32)> {
    let Action::LibraryCall {
        template,
        operands,
        output,
    } = action
    else {
        return None;
    };
    if template.primitive() != Primitive::EnumPack
        || operands.len() != 2
        || template.inputs().len() != 2
        || template.output() != plan.values()[output.get() as usize].definition()
        || operands
            .iter()
            .zip(template.inputs())
            .any(|(v, schema)| plan.values()[v.get() as usize].definition() != schema)
    {
        return None;
    }
    let variants = schemas(plan, *output)?;
    // Selected EnumPack ordinals are one-based. Structural observations and
    // the canonical Enum value carry the checked zero-based ordinal.
    let ordinal = plan.constants().iter().find_map(|entry| {
        if entry.value() != operands[0] {
            return None;
        }
        let mech_core::ValueData::Index(n) = entry.constant().data() else {
            return None;
        };
        n.checked_sub(1).and_then(|n| u32::try_from(n).ok())
    })?;
    (variants.get(ordinal as usize).copied()
        == Some(plan.values()[operands[1].get() as usize].schema()))
    .then_some((*output, operands[1], ordinal))
}

pub(super) fn observation(plan: &Plan, action: &Action) -> Option<(Value, Value)> {
    let Action::Structural {
        source,
        path,
        observation,
        output,
    } = action
    else {
        return None;
    };
    schemas(plan, *source)?;
    let mut body = plan.values()[source.get() as usize].definition().body();
    for step in path {
        body = match (step, body) {
            (Step::EnumPayload { ordinal }, SchemaBody::Enum { variants, .. }) => {
                variants.get(*ordinal as usize)?.payload.as_ref()?
            }
            (Step::TupleItem { ordinal }, SchemaBody::Tuple(fields)) => {
                fields.get(*ordinal as usize)?
            }
            _ => return None,
        };
    }
    let target = plan.values()[output.get() as usize].definition();
    let valid = match observation {
        Observation::EnumOrdinal { ordinal } => {
            matches!(body, SchemaBody::Enum { variants, .. } if (*ordinal as usize) < variants.len())
                && target.body() == &SchemaBody::Bool
        }
        Observation::U64 => {
            body == &SchemaBody::UnsignedInteger(IntegerWidth::W64) && target.body() == body
        }
        Observation::CanonicalValue => {
            target.dimension_parameters().is_empty() && target.body() == body
        }
        _ => false,
    };
    valid.then_some((*source, *output))
}

/// A compiler-only view of one guarded enum variant. The original layout and
/// runtime tag bank are unchanged. Only the caller's independently proved hot
/// source sites may use this view; entry must check the current carrier tag and
/// every retained backedge must reestablish the same variant.
impl NativeBufferFrames {
    pub fn numeric_variant_projection(
        &self,
        plan: &Plan,
        carrier: Value,
        ordinal: u32,
    ) -> Option<Self> {
        let schema = plan.values().get(carrier.get() as usize)?.schema();
        let [root] = self.fields(carrier)? else {
            return None;
        };
        let target = self.tagged_variant(*root, ordinal)?;
        if !matches!(target.lane(), NativeBufferFrameLane::Composite { .. }) {
            return None;
        }
        let mut projected = self.clone();
        for (value, ty) in plan.values().iter().enumerate() {
            if ty.schema() != schema {
                continue;
            }
            let [root] = self.fields_at(value)? else {
                return None;
            };
            let target = self.tagged_variant(*root, ordinal)?;
            let NativeBufferFrameLane::Composite { at, count } = target.lane() else {
                return None;
            };
            projected.slots[value] = Some((at, count));
        }
        for action in projected.actions.iter_mut().flatten() {
            let replacement = match &action.operation {
                NativeBufferFrameOperation::EnumPack {
                    output,
                    payload,
                    ordinal: active,
                } if plan.values()[output.get() as usize].schema() == schema
                    && *active == ordinal
                    && self.fields(*payload).is_some() =>
                {
                    Some(NativeBufferFrameOperation::Copy {
                        source: *payload,
                        output: *output,
                    })
                }
                NativeBufferFrameOperation::Observe {
                    source,
                    path,
                    observation,
                    output,
                } if plan.values()[source.get() as usize].schema() == schema
                    && matches!(observation, Observation::U64 | Observation::CanonicalValue) =>
                {
                    match path.as_ref() {
                        [Step::EnumPayload { ordinal: active }]
                            if *active == ordinal && self.fields(*output).is_some() =>
                        {
                            Some(NativeBufferFrameOperation::Copy {
                                source: *source,
                                output: *output,
                            })
                        }
                        [
                            Step::EnumPayload { ordinal: active },
                            Step::TupleItem { ordinal: field },
                        ] if *active == ordinal => Some(NativeBufferFrameOperation::Project {
                            source: *source,
                            ordinal: *field as usize,
                            output: *output,
                        }),
                        _ => None,
                    }
                }
                _ => None,
            };
            if let Some(replacement) = replacement {
                action.operation = replacement;
            }
        }
        Some(projected)
    }
}
