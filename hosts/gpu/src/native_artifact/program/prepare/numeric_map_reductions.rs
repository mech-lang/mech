//! Exact sole-consumer row-map -> ordered F64 sum -> scalar extraction.
//! These source facts provide no runtime account or publication permission.
use super::*;

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Reduction {
    pub finish: u32,
    /// Original exit/finish path; native execution ends before these visits.
    pub finish_path: Vec<u32>,
    /// Original straight source continuation, including sum and extraction.
    pub continuation: Vec<u32>,
    pub sum: u32,
    pub select: u32,
    pub scalar: u32,
    pub elided_values: Vec<u32>,
}
fn f64_matrix(body: &SchemaBody) -> bool {
    matches!(body,SchemaBody::Matrix{element,dimensions} if dimensions.len()==2
        && element.as_ref()==&SchemaBody::FloatingPoint(mech_core::FloatWidth::W64))
}
fn sole_user(
    plan: &Plan,
    facts: &Facts<'_>,
    mut value: Value,
    copies: &mut BTreeSet<u32>,
    elided: &mut BTreeSet<u32>,
) -> Option<u32> {
    for _ in 0..16 {
        elided.insert(value.get());
        let [Read::Action(site)] = facts.read_roles(value).ok()? else {
            return None;
        };
        match plan.blocks().get(*site as usize)?.action() {
            Action::Scalar(Scalar::Copy { source, output })
                if *source == value
                    && plan.values()[source.get() as usize].schema()
                        == plan.values()[output.get() as usize].schema()
                    && facts.definition_sites(output.get()).ok()?.len() == 1 =>
            {
                copies.insert(*site);
                value = *output;
            }
            _ => return Some(*site),
        }
    }
    None
}
fn path(
    plan: &Plan,
    mut at: u32,
    until: u32,
    allowed: &BTreeSet<u32>,
    parent: Option<u32>,
) -> Option<Vec<u32>> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    while out.len() < 32 && seen.insert(at) {
        let block = plan.blocks().get(at as usize)?;
        if plan.activation_boundary() == Some(at) || block.iteration() != parent {
            return None;
        }
        if at != until
            && !allowed.contains(&at)
            && !matches!(block.action(), Action::Scalar(Scalar::None))
        {
            return None;
        }
        out.push(at);
        if at == until {
            return Some(out);
        }
        let Terminator::Scalar(Term::Jump(next)) = block.terminator() else {
            return None;
        };
        at = next;
    }
    None
}
fn candidate(plan: &Plan, facts: &Facts<'_>, region: &Region) -> Option<Reduction> {
    if region.rectangle.is_some()
        || plan.values()[region.result as usize].scalar_kind() != Some(NativeScalarKind::F64)
    {
        return None;
    }
    let collection = plan.collections().get(region.collection as usize)?;
    if collection.kind() != mech_engine::ComprehensionKind::Matrix {
        return None;
    }
    let mut copies = BTreeSet::new();
    let mut elided = BTreeSet::new();
    let sum = sole_user(plan, facts, collection.output(), &mut copies, &mut elided)?;
    let Action::LibraryCall {
        template,
        operands,
        output,
    } = plan.blocks()[sum as usize].action()
    else {
        return None;
    };
    if template.primitive() != Primitive::SumColumns
        || operands.len() != 1
        || !f64_matrix(template.inputs().first()?.body())
        || !f64_matrix(template.output().body())
        || !elided.contains(&operands[0].get())
    {
        return None;
    }
    let select = sole_user(plan, facts, *output, &mut copies, &mut elided)?;
    let Action::LibraryCall {
        template,
        operands,
        output,
    } = plan.blocks()[select as usize].action()
    else {
        return None;
    };
    if template.primitive() != Primitive::MatrixScalarAccess
        || operands.len() != 2
        || !elided.contains(&operands[0].get())
        || !f64_matrix(template.inputs().first()?.body())
        || plan.values()[output.get() as usize].scalar_kind() != Some(NativeScalarKind::F64)
        || !plan.constants().iter().any(|c| {
            c.value() == operands[1] && matches!(c.constant().data(), mech_core::ValueData::U64(1))
        })
    {
        return None;
    }
    let finish = collection.finish();
    let finish_path = path(
        plan,
        region.exit,
        finish,
        &BTreeSet::new(),
        collection.parent(),
    )?;
    let Terminator::Scalar(Term::Jump(next)) = plan.blocks()[finish as usize].terminator() else {
        return None;
    };
    let mut allowed = copies;
    allowed.extend([sum, select]);
    let continuation = path(plan, next, select, &allowed, collection.parent())?;
    if !allowed.iter().all(|site| continuation.contains(site)) || !continuation.contains(&sum) {
        return None;
    }
    if !matches!(plan.blocks()[finish as usize].action(),Action::CollectionFinish{collection:id} if *id==region.collection)
    {
        return None;
    }
    Some(Reduction {
        finish,
        finish_path,
        continuation,
        sum,
        select,
        scalar: output.get(),
        elided_values: elided.into_iter().collect(),
    })
}
pub(super) fn extend(plan: &Plan, layout: &mut Layout) {
    let facts = plan.facts();
    for region in &mut layout.regions {
        region.reduction = candidate(plan, &facts, region);
    }
}
