//! One selected, constant, unambiguous record field. Consumers prove storage.
use super::{
    NativeCollectionPrimitive as Primitive, NativeIterationAction as Action,
    NativeMatchValue as Value, NativeProgramArtifactPlan as Plan,
};
use mech_core::{SchemaBody, ValueData};

pub(super) fn constant_field(plan: &Plan, action: &Action) -> Option<(Value, usize, Value)> {
    let Action::LibraryCall {
        template,
        operands,
        output,
    } = action
    else {
        return None;
    };
    if template.primitive() != Primitive::RecordFieldAccess
        || operands.len() != 2
        || template.inputs().len() != 2
        || template.inputs()[1].body() != &SchemaBody::Id
        || template.output() != plan.values()[output.get() as usize].definition()
        || template.inputs()[0] != *plan.values()[operands[0].get() as usize].definition()
    {
        return None;
    }
    let selected = plan
        .constants()
        .iter()
        .find(|entry| entry.value() == operands[1])?;
    let ValueData::Id(id) = selected.constant().data() else {
        return None;
    };
    let SchemaBody::Record(fields) = template.inputs()[0].body() else {
        return None;
    };
    // Ambiguous hashes retain the maintained canonical selector path.
    let mut matching = fields
        .iter()
        .enumerate()
        .filter(|(_, field)| mech_core::hash_str(&field.name) == *id);
    let (ordinal, _) = matching.next()?;
    if matching.next().is_some() {
        return None;
    }
    Some((operands[0], ordinal, *output))
}
