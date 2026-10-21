//! One unobserved String generator element and selected String map entry.
//! The existing generator and TextJoin still own every visit and all control.
use super::iteration_validation::ControlPlan;
use super::{NativeIterationAction as Action, NativeProgramArtifactPlan as Plan};

/// Constructor-private proof; neither semantic String is a materialized Value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeStringElementMapProjection {
    pub(crate) iteration: u32,
    pub(crate) element_site: u32,
    pub(crate) selection_site: u32,
    pub(crate) source: super::NativeMatchValue,
    pub(crate) map: super::NativeMatchValue,
    pub(crate) element: super::NativeMatchValue,
    pub(crate) output: super::NativeMatchValue,
    pub(crate) selection_template: super::NativeProgramLibraryTemplate,
}
impl NativeStringElementMapProjection {
    pub const fn iteration(&self) -> u32 {
        self.iteration
    }
    pub const fn element_site(&self) -> u32 {
        self.element_site
    }
    pub const fn selection_site(&self) -> u32 {
        self.selection_site
    }
    pub const fn source(&self) -> super::NativeMatchValue {
        self.source
    }
    pub const fn map(&self) -> super::NativeMatchValue {
        self.map
    }
    pub const fn element(&self) -> super::NativeMatchValue {
        self.element
    }
    pub const fn output(&self) -> super::NativeMatchValue {
        self.output
    }
    pub fn selection_template(&self) -> &super::NativeProgramLibraryTemplate {
        &self.selection_template
    }
}
const BAD: &str = "invalid native selected String element/map projection";

#[cfg(all(feature = "pure_text", feature = "map_ops"))]
fn adjacent(plan: &impl ControlPlan, from: u32, to: u32) -> bool {
    use super::{
        NativeIterationTerminator as Terminator, NativeMatchAction as Scalar,
        NativeMatchTerminator as Term,
    };
    let Some(first) = plan.blocks().get(from as usize) else {
        return false;
    };
    let mut site = from;
    for _ in 0..plan.blocks().len() {
        let Some(block) = plan.blocks().get(site as usize) else {
            return false;
        };
        if block.iteration() != first.iteration() || plan.lifecycle_barrier(site as usize) {
            return false;
        }
        if site == to {
            return site != from;
        }
        if site != from && !matches!(block.action(), Action::Scalar(Scalar::None)) {
            return false;
        }
        let Terminator::Scalar(Term::Jump(next)) = block.terminator() else {
            return false;
        };
        site = next;
    }
    false
}

#[cfg(all(feature = "pure_text", feature = "map_ops"))]
fn candidate(
    plan: &impl ControlPlan,
    id: usize,
    counts: &[usize],
    actions: &super::fusion_semantics::SemanticActions<'_>,
) -> Option<NativeStringElementMapProjection> {
    use super::iteration_validation::ControlType;
    use super::{
        NativeCollectionPrimitive as Primitive, NativeCollectionResult as ResultKind,
        NativeCollectionSource as Source, NativeControlLoopRef as Loop,
        NativeIterationValueScope as Scope,
    };
    use mech_core::SchemaBody;
    let data = plan.collections().get(id)?;
    if !matches!(data.result(), ResultKind::TextJoin(_)) || data.string_span_projection().is_some()
    {
        return None;
    }
    let [iteration] = data.generators() else {
        return None;
    };
    if *iteration as usize >= plan.iteration_count() {
        return None;
    }
    let Loop::Collection(generator) = plan.iteration(*iteration as usize) else {
        return None;
    };
    let Source::Value(source) = generator.source() else {
        return None;
    };
    let element = generator.element()?;
    let element_site = generator.body();
    if generator.collection() as usize != id
        || generator.parent() != data.parent()
        || !matches!(actions.get(element_site as usize),
            Some(Action::CollectionElement { iteration: actual }) if actual == iteration)
    {
        return None;
    }
    let definition = |value: super::NativeMatchValue| {
        plan.values()
            .get(value.get() as usize)
            .and_then(ControlType::definition)
    };
    let plain_string = |value| {
        definition(value).is_some_and(|schema| {
            schema.body() == &SchemaBody::String && schema.dimension_parameters().is_empty()
        })
    };
    let Action::CollectionPush {
        collection,
        value: output,
    } = actions.get(data.push() as usize)?
    else {
        return None;
    };
    if *collection as usize != id
        || !plain_string(*source)
        || !plain_string(element)
        || !plain_string(*output)
        || counts.get(element.get() as usize) != Some(&1)
        || counts.get(output.get() as usize) != Some(&1)
    {
        return None;
    }
    let mut producers = actions
        .iter()
        .enumerate()
        .filter_map(|(site, action)| match action {
            Action::LibraryCall {
                template,
                operands,
                output: produced,
            } if produced == output => Some((site, template, operands)),
            _ => None,
        });
    let (site, template, operands) = producers.next()?;
    if producers.next().is_some()
        || template.primitive() != Primitive::MapKeyAccess
        || template.implementation_memory()
            != mech_core::ImplementationMemoryClass::CanonicalFinalize
        || operands.len() != 2
        || operands[1] != element
        || template.inputs().len() != 2
    {
        return None;
    }
    let map = operands[0];
    let schema = definition(map)?;
    if !schema.dimension_parameters().is_empty()
        || !matches!(schema.body(), SchemaBody::Map { key, value, .. }
            if key.as_ref() == &SchemaBody::String && value.as_ref() == &SchemaBody::String)
    {
        return None;
    }
    let site = u32::try_from(site).ok()?;
    let scope = Some(*iteration);
    if plan.blocks().get(element_site as usize)?.iteration() != scope
        || plan.value_scopes().get(element.get() as usize)
            != Some(&Scope::Immutable { iteration: scope })
        || plan.value_scopes().get(output.get() as usize)
            != Some(&Scope::Immutable { iteration: scope })
        || !adjacent(plan, element_site, site)
        || !adjacent(plan, site, data.push())
    {
        return None;
    }
    Some(NativeStringElementMapProjection {
        iteration: *iteration,
        element_site,
        selection_site: site,
        source: *source,
        map,
        element,
        output: *output,
        selection_template: template.clone(),
    })
}
#[cfg(not(all(feature = "pure_text", feature = "map_ops")))]
fn candidate(
    _: &impl ControlPlan,
    _: usize,
    _: &[usize],
    _: &super::fusion_semantics::SemanticActions<'_>,
) -> Option<NativeStringElementMapProjection> {
    None
}

pub(crate) fn lower(plan: &mut Plan) -> Result<(), &'static str> {
    let actions = super::fusion_semantics::semantic_actions(plan)?.into_owned();
    let counts = super::fusion_semantics::semantic_use_counts(plan)?;
    for id in 0..plan.collections.len() {
        let Some(projection) = candidate(plan, id, &counts, &actions) else {
            continue;
        };
        let site = projection.selection_site as usize;
        let previous = plan.blocks[site].action.clone();
        plan.blocks[site].action = Action::ProjectStringMapEntry {
            collection: u32::try_from(id).map_err(|_| BAD)?,
        };
        plan.collections[id].string_element_map_projection = Some(projection);
        // No capture or guard may be hoisted to make the projection usable.
        if plan.validate_current_visits().is_err() {
            plan.blocks[site].action = previous;
            plan.collections[id].string_element_map_projection = None;
        }
    }
    Ok(())
}

pub(super) fn validate(
    inputs: &super::fusion_semantics::ValidationInputs<'_, impl ControlPlan>,
) -> Result<(), &'static str> {
    let plan = inputs.plan();
    let actions = inputs.actions()?;
    let counts = inputs.counts();
    let mut expected = vec![None; plan.blocks().len()];
    for (id, data) in plan.collections().iter().enumerate() {
        let Some(p) = data.string_element_map_projection() else {
            continue;
        };
        if candidate(plan, id, counts, actions).as_ref() != Some(p)
            || expected
                .get_mut(p.selection_site as usize)
                .ok_or(BAD)?
                .replace(u32::try_from(id).map_err(|_| BAD)?)
                .is_some()
        {
            return Err(BAD);
        }
    }
    for (site, block) in plan.blocks().iter().enumerate() {
        let actual = match block.action() {
            Action::ProjectStringMapEntry { collection } => Some(*collection),
            _ => None,
        };
        if actual != expected[site] {
            return Err(BAD);
        }
    }
    Ok(())
}

#[cfg(all(
    test,
    feature = "pure_text",
    feature = "map_ops",
    feature = "source",
    feature = "artifact-codec",
    feature = "semantic-compiler",
    feature = "resident-artifact"
))]
#[path = "string_element_map_projection_tests.rs"]
mod tests;
