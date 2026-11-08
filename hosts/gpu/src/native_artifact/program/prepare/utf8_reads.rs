//! Same-plan proof for a closed String-matrix -> immutable String/U64 Map
//! lookup comprehension. This module supplies no runtime Value or permit.
use mech_core::{IntegerWidth, SchemaBody, ValueData};
use mech_engine::native_artifact::{
    NativeCollectionPrimitive as Primitive, NativeCollectionResult as ResultKind,
    NativeCollectionSource as Source, NativeControlLoop as Loop, NativeIterationAction as Action,
    NativeIterationTerminator as Terminator, NativeIterationValueScope as Scope,
    NativeMatchAction as Scalar, NativeMatchTerminator as Term, NativeMatchValue as Value,
    NativeProgramArtifactPlan as Plan, NativeProgramFacts as Facts, NativeProgramReadRole as Read,
};
use std::collections::BTreeSet;

const MAX_SITES: usize = 16_384;
const MAX_VALUES: usize = 65_536;
const MAX_REGIONS: usize = 256;

/// Fixed entry/exit native template; dynamic hash/probe work is funded by the
/// separately checked whole-loop physical preflight allowance.
pub(in crate::native_artifact) const HEADER_WORK: u64 = 4096;

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields, rename = "mech.native.image.Region")]
pub(in crate::native_artifact) struct Region {
    pub collection: u32,
    pub iteration: u32,
    pub header: u32,
    /// The ordinary header's completed-loop edge; finalization stays ordinary.
    pub exit: u32,
    pub source: u32,
    pub map_root: u32,
    pub map_ordinal: Option<usize>,
    pub element_site: u32,
    pub project_site: Option<u32>,
    pub read_site: u32,
    pub push_site: u32,
    pub output: u32,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields, rename = "mech.native.image.Layout")]
pub(in crate::native_artifact) struct Layout {
    pub regions: Vec<Region>,
    pub headers: Vec<Option<usize>>,
}
impl Layout {
    pub fn derive(plan: &Plan) -> Result<Self, &'static str> {
        plan.validate_current_visits()?;
        // This is an optional proof, with bounded immutable scratch and image
        // metadata. An oversized plan keeps its complete ordinary loop.
        if plan.blocks().len() > MAX_SITES || plan.values().len() > MAX_VALUES {
            return Ok(Self::default());
        }
        let facts = plan.facts();
        let mut out = Self {
            headers: vec![None; plan.blocks().len()],
            ..Self::default()
        };
        for (id, collection) in plan.collections().iter().enumerate() {
            if out.regions.len() == MAX_REGIONS {
                break;
            }
            if collection.kind() != mech_engine::ComprehensionKind::Matrix
                || !matches!(collection.result(), ResultKind::Materialized)
                || collection.generators().len() != 1
                || collection.parent_collection().is_some()
                || collection.string_span_projection().is_some()
                || collection.string_element_map_projection().is_some()
                || !facts.is_reachable(collection.begin())?
            {
                continue;
            }
            let iteration = collection.generators()[0];
            let Loop::Collection(generator) = &plan.iterations()[iteration as usize] else {
                continue;
            };
            let (Source::Value(source), Some(element)) = (generator.source(), generator.element())
            else {
                continue;
            };
            if generator.collection() != id as u32
                || generator.parent() != collection.parent()
                || !matrix(
                    plan.values()[source.get() as usize].definition().body(),
                    &SchemaBody::String,
                )
                || !matrix(
                    plan.values()[collection.output().get() as usize]
                        .definition()
                        .body(),
                    &SchemaBody::UnsignedInteger(IntegerWidth::W64),
                )
                || !scalar(plan, element, &SchemaBody::String)
                || !immutable_before(&facts, *source, generator.initialize(), generator.parent())?
                || !matches!(plan.blocks()[generator.initialize() as usize].action(),
                    Action::Initialize { iteration: owner } if *owner == iteration)
                || plan.blocks()[generator.initialize() as usize].terminator()
                    != Terminator::Scalar(Term::Jump(generator.header()))
                || !matches!(
                    plan.blocks()[generator.header() as usize].action(),
                    Action::Scalar(Scalar::None)
                )
                || plan.blocks()[generator.header() as usize].terminator()
                    != (Terminator::Header {
                        iteration,
                        body: generator.body(),
                        exit: generator.exit(),
                    })
            {
                continue;
            }
            let Some(region) = derive_body(&facts, id as u32, iteration, *source, element)? else {
                continue;
            };
            let header = region.header as usize;
            if out.headers[header].is_none() {
                out.headers[header] = Some(out.regions.len());
                out.regions.push(region);
            }
        }
        Ok(out)
    }
    pub fn at_header(&self, site: usize) -> Option<&Region> {
        self.regions.get(self.headers.get(site).copied().flatten()?)
    }
}

fn matrix(body: &SchemaBody, expected: &SchemaBody) -> bool {
    matches!(body, SchemaBody::Matrix { element, dimensions }
        if element.as_ref() == expected && dimensions.len() == 2)
}

fn scalar(plan: &Plan, value: Value, body: &SchemaBody) -> bool {
    let definition = plan.values()[value.get() as usize].definition();
    definition.body() == body && definition.dimension_parameters().is_empty()
}

fn string_u64_map(body: &SchemaBody) -> bool {
    matches!(body, SchemaBody::Map { key, value, .. }
        if key.as_ref() == &SchemaBody::String
        && value.as_ref() == &SchemaBody::UnsignedInteger(IntegerWidth::W64))
}

/// The loop lends genuine immutable roots, not a prior visit's carrier word.
/// Whole-plan validation proves lexical currentness; the unique definition and
/// strict dominance below additionally keep all skipped producer work outside
/// this generator activation. Runtime still checks the exact arena generation.
fn immutable_before(
    facts: &Facts<'_>,
    value: Value,
    frontier: u32,
    parent: Option<u32>,
) -> Result<bool, &'static str> {
    let Scope::Immutable { iteration } = facts.plan().value_scopes()[value.get() as usize] else {
        return Ok(false);
    };
    if !facts.lexical_ancestor(iteration, parent) {
        return Ok(false);
    }
    let [definition] = facts.definition_sites(value.get())? else {
        return Ok(false);
    };
    Ok(match definition.site() {
        Some(site) => facts.strictly_dominates(site, frontier)?,
        None => true,
    })
}

fn defined_here(facts: &Facts<'_>, value: Value, site: u32) -> Result<bool, &'static str> {
    Ok(matches!(facts.definition_sites(value.get())?, [definition]
        if definition.site() == Some(site)))
}

fn only_action_read(facts: &Facts<'_>, value: Value, site: u32) -> Result<bool, &'static str> {
    Ok(facts.read_roles(value)? == [Read::Action(site)])
}

/// One linear source traversal, with no filters, child calls, hidden effects,
/// control escapes or observable intermediate Values. All semantic reads are
/// checked through the exhaustive same-plan inventory, including public and
/// collection/control uses which a visible-operand count alone would miss.
fn derive_body(
    facts: &Facts<'_>,
    collection_id: u32,
    iteration: u32,
    source: Value,
    element: Value,
) -> Result<Option<Region>, &'static str> {
    let plan = facts.plan();
    let collection = &plan.collections()[collection_id as usize];
    let Loop::Collection(generator) = &plan.iterations()[iteration as usize] else {
        return Ok(None);
    };
    let mut sites = BTreeSet::new();
    let mut site = generator.body();
    let mut element_site = None;
    let mut projection: Option<(u32, Value, Value, usize)> = None;
    let mut read: Option<(u32, Value, Value)> = None;
    let mut pushed = false;
    while site != generator.latch() {
        if !sites.insert(site) || plan.activation_boundary() == Some(site) {
            return Ok(None);
        }
        let block = &plan.blocks()[site as usize];
        if block.iteration() != Some(iteration) {
            return Ok(None);
        }
        let Terminator::Scalar(Term::Jump(next)) = block.terminator() else {
            return Ok(None);
        };
        match block.action() {
            Action::Scalar(Scalar::None) => {}
            Action::CollectionElement { iteration: owner }
                if *owner == iteration && element_site.is_none() && read.is_none() =>
            {
                if !defined_here(facts, element, site)? {
                    return Ok(None);
                }
                element_site = Some(site);
            }
            Action::LibraryCall {
                template,
                operands,
                output,
            } if template.primitive() == Primitive::RecordFieldAccess
                && element_site.is_some()
                && projection.is_none()
                && read.is_none()
                && operands.len() == 2
                && template.inputs().len() == 2 =>
            {
                let root = operands[0];
                let definition = plan.values()[root.get() as usize].definition();
                if template.inputs()[0] != *definition
                    || !definition.dimension_parameters().is_empty()
                    || !template.output().dimension_parameters().is_empty()
                    || !string_u64_map(template.output().body())
                    || template.inputs()[1].body() != &SchemaBody::Id
                    || !scalar(plan, operands[1], &SchemaBody::Id)
                    || !immutable_before(facts, root, generator.initialize(), generator.parent())?
                    || !defined_here(facts, *output, site)?
                {
                    return Ok(None);
                }
                let SchemaBody::Record(fields) = definition.body() else {
                    return Ok(None);
                };
                let Some(ValueData::Id(selector)) = plan
                    .constants()
                    .iter()
                    .find(|constant| constant.value() == operands[1])
                    .map(|constant| constant.constant().data())
                else {
                    return Ok(None);
                };
                let mut matching = fields
                    .iter()
                    .enumerate()
                    .filter(|(_, field)| mech_core::hash_str(&field.name) == *selector);
                let Some((ordinal, field)) = matching.next() else {
                    return Ok(None);
                };
                if matching.next().is_some() || &field.schema != template.output().body() {
                    return Ok(None);
                }
                projection = Some((site, *output, root, ordinal));
            }
            Action::LibraryCall {
                template,
                operands,
                output,
            } if template.primitive() == Primitive::MapKeyAccess
                && element_site.is_some()
                && read.is_none()
                && operands.len() == 2
                && template.inputs().len() == 2 =>
            {
                let map = operands[0];
                let definition = plan.values()[map.get() as usize].definition();
                if operands[1] != element
                    || !string_u64_map(definition.body())
                    || !definition.dimension_parameters().is_empty()
                    || template.inputs()[0] != *definition
                    || template.inputs()[1] != *plan.values()[element.get() as usize].definition()
                    || template.output().body() != &SchemaBody::UnsignedInteger(IntegerWidth::W64)
                    || !template.output().dimension_parameters().is_empty()
                    || !scalar(
                        plan,
                        *output,
                        &SchemaBody::UnsignedInteger(IntegerWidth::W64),
                    )
                    || !defined_here(facts, *output, site)?
                    || !only_action_read(facts, element, site)?
                {
                    return Ok(None);
                }
                if let Some((_, projected, _, _)) = projection {
                    if map != projected || !only_action_read(facts, projected, site)? {
                        return Ok(None);
                    }
                } else if !immutable_before(facts, map, generator.initialize(), generator.parent())?
                {
                    return Ok(None);
                }
                read = Some((site, *output, map));
            }
            Action::CollectionPush {
                collection: owner,
                value,
            } if *owner == collection_id
                && site == collection.push()
                && !pushed
                && read.is_some_and(|(_, output, _)| output == *value) =>
            {
                if !only_action_read(facts, *value, site)? {
                    return Ok(None);
                }
                pushed = true;
            }
            _ => return Ok(None),
        }
        site = next;
    }
    let (Some(element_site), Some((read_site, _, map))) = (element_site, read) else {
        return Ok(None);
    };
    let latch = &plan.blocks()[generator.latch() as usize];
    if !pushed
        || !matches!(latch.action(), Action::Latch { iteration: owner } if *owner == iteration)
        || latch.terminator() != Terminator::Scalar(Term::Jump(generator.header()))
        || plan.activation_boundary() == Some(generator.header())
        || plan.activation_boundary() == Some(generator.latch())
    {
        return Ok(None);
    }
    sites.insert(generator.header());
    sites.insert(generator.latch());
    // Every reachable owned visit must be the sealed chain above. A second
    // arm, child initializer or externally entered body cannot be skipped.
    for (site, block) in plan.blocks().iter().enumerate() {
        if block.iteration() == Some(iteration)
            && facts.is_reachable(site as u32)?
            && !sites.contains(&(site as u32))
        {
            return Ok(None);
        }
    }
    let (project_site, map_root, map_ordinal) = match projection {
        Some((site, _, root, ordinal)) => (Some(site), root, Some(ordinal)),
        None => (None, map, None),
    };
    Ok(Some(Region {
        collection: collection_id,
        iteration,
        header: generator.header(),
        exit: generator.exit(),
        source: source.get(),
        map_root: map_root.get(),
        map_ordinal,
        element_site,
        project_site,
        read_site,
        push_site: collection.push(),
        output: collection.output().get(),
    }))
}
