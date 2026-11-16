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
/// carries a supported opaque map. Pure scalar enums retain their scalar owner.
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
