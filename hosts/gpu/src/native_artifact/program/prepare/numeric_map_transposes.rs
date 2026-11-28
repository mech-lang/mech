//! Exact sole-consumer scalar row comprehension -> transpose publication.
//! The source builder and all original admission boundaries remain authoritative.
use super::*;
pub(in crate::native_artifact) const FINISH_WORK: u64 = 32768;
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Transpose {
    pub finish: u32,
    pub continuation: Vec<u32>,
    pub transpose: u32,
    pub output: u32,
    pub elided_values: Vec<u32>,
}
fn kind(body: &SchemaBody) -> Option<NativeScalarKind> {
    let SchemaBody::Matrix {
        element,
        dimensions,
    } = body
    else {
        return None;
    };
    (dimensions.len() == 2)
        .then(|| NativeScalarKind::from_schema_body(element))
        .flatten()
}
fn candidate(plan: &Plan, facts: &Facts<'_>, region: &Region) -> Option<Transpose> {
    if region.rectangle.is_some() || region.reduction.is_some() {
        return None;
    }
    let collection = plan.collections().get(region.collection as usize)?;
    if collection.kind() != mech_engine::ComprehensionKind::Matrix {
        return None;
    }
    let mut copies = BTreeSet::new();
    let mut elided = BTreeSet::new();
    let transpose =
        reductions::sole_user(plan, facts, collection.output(), &mut copies, &mut elided)?;
    let Action::LibraryCall {
        template,
        operands,
        output,
    } = plan.blocks()[transpose as usize].action()
    else {
        return None;
    };
    let input = template.inputs().first()?;
    if template.primitive() != Primitive::MatrixTranspose
        || operands.len() != 1
        || !elided.contains(&operands[0].get())
        || kind(input.body())? != kind(template.output().body())?
    {
        return None;
    }
    let finish = collection.finish();
    if plan.activation_boundary() == Some(finish) {
        return None;
    }
    if !matches!(plan.blocks()[finish as usize].action(),Action::CollectionFinish{collection:id} if *id==region.collection)
    {
        return None;
    }
    let Terminator::Scalar(Term::Jump(next)) = plan.blocks()[finish as usize].terminator() else {
        return None;
    };
    copies.insert(transpose);
    let continuation = reductions::path(plan, next, transpose, &copies, collection.parent())?;
    if !copies.iter().all(|site| continuation.contains(site)) {
        return None;
    }
    Some(Transpose {
        finish,
        continuation,
        transpose,
        output: output.get(),
        elided_values: elided.into_iter().collect(),
    })
}
pub(super) fn extend(plan: &Plan, layout: &mut Layout) {
    let facts = plan.facts();
    for region in &mut layout.regions {
        region.transpose = candidate(plan, &facts, region);
    }
}
