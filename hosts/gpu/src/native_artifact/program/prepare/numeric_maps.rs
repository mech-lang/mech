//! Same-plan facts for a scalar transform of an immutable Matrix or U64 cursor.
//! These facts grant neither a source loan nor publication or budget authority.
use mech_core::SchemaBody;
use mech_engine::native_artifact::{
    NativeCollectionPrimitive as Primitive, NativeCollectionResult as ResultKind,
    NativeCollectionSource as Source, NativeControlLoop as Loop, NativeIterationAction as Action,
    NativeIterationTerminator as Terminator, NativeIterationValueScope as Scope,
    NativeMatchAction as Scalar, NativeMatchTerminator as Term, NativeMatchValue as Value,
    NativeProgramArtifactPlan as Plan, NativeProgramFacts as Facts,
    NativeProgramProducerRole as Producer, NativeProgramReadRole as Read, NativeScalarKind,
};
use std::collections::{BTreeMap, BTreeSet};

pub(in crate::native_artifact) const HEADER_WORK: u64 = 32768;
const MAX_SITES: usize = 16384;
const MAX_VALUES: usize = 65536;
const MAX_BODY: usize = 256;
const MAX_STEPS: usize = 64;
pub(in crate::native_artifact) const MAX_MATRICES: usize = 16;

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Matrix {
    pub value: u32,
    pub witness: u32,
    pub width: u8,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct MatrixRead {
    pub site: u32,
    pub matrix: usize,
    pub indices: Vec<u32>,
    pub output: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields, rename = "mech.native.image.Region")]
pub(in crate::native_artifact) struct Region {
    pub collection: u32,
    pub iteration: u32,
    pub header: u32,
    pub exit: u32,
    pub source: u32,
    pub range: bool,
    pub element: u32,
    pub result: u32,
    pub output: u32,
    pub input_width: u8,
    pub output_width: u8,
    /// Exact scalar source blocks; operations still come from the sealed plan.
    pub steps: Vec<u32>,
    /// Complete body/header/latch inventory, including empty source visits.
    pub sites: Vec<u32>,
    pub captures: Vec<u32>,
    pub matrices: Vec<Matrix>,
    pub reads: Vec<MatrixRead>,
}
#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields, rename = "mech.native.image.Layout")]
pub(in crate::native_artifact) struct Layout {
    pub regions: Vec<Region>,
    pub headers: Vec<Option<usize>>,
}
fn matrix_width(body: &SchemaBody) -> Option<u8> {
    match body {
        SchemaBody::Matrix {
            element,
            dimensions,
        } if dimensions.len() == 2 => scalar_width(element),
        _ => None,
    }
}
fn scalar_width(body: &SchemaBody) -> Option<u8> {
    Some(match NativeScalarKind::from_schema_body(body)? {
        NativeScalarKind::Bool | NativeScalarKind::U8 => 1,
        NativeScalarKind::U16 => 2,
        NativeScalarKind::U32 => 4,
        NativeScalarKind::U64 | NativeScalarKind::I64 | NativeScalarKind::F64 => 8,
    })
}
fn immutable_before(
    facts: &Facts<'_>,
    value: Value,
    at: u32,
    parent: Option<u32>,
) -> Result<bool, &'static str> {
    let Scope::Immutable { iteration } = facts.plan().value_scopes()[value.get() as usize] else {
        return Ok(false);
    };
    if !facts.lexical_ancestor(iteration, parent) {
        return Ok(false);
    }
    let definitions = facts.definition_sites(value.get())?;
    let definition = match definitions {
        [definition] if definition.role() != Producer::CollectionBuilder => definition,
        [_, _] => {
            // Materialized collections have two deliberately different roles:
            // allocation starts the builder; only finish publishes the Value.
            // Require their exact common collection and publication dominance.
            let Some(builder) = definitions
                .iter()
                .find(|d| d.role() == Producer::CollectionBuilder)
            else {
                return Ok(false);
            };
            let Some(published) = definitions
                .iter()
                .find(|d| d.role() == Producer::MaterializedCollection)
            else {
                return Ok(false);
            };
            let Some(collection) = facts.plan().collections().iter().find(|c| {
                c.output() == value
                    && Some(c.begin()) == builder.site()
                    && Some(c.finish()) == published.site()
            }) else {
                return Ok(false);
            };
            if !facts.strictly_dominates(collection.begin(), collection.finish())? {
                return Ok(false);
            }
            published
        }
        _ => return Ok(false),
    };
    Ok(match definition.site() {
        Some(site) => facts.strictly_dominates(site, at)?,
        None => true,
    })
}
fn defined_here(facts: &Facts<'_>, value: Value, site: u32) -> Result<bool, &'static str> {
    Ok(
        matches!(facts.definition_sites(value.get())?, [definition] if definition.site() == Some(site)),
    )
}
impl Layout {
    pub fn at_header(&self, site: usize) -> Option<&Region> {
        self.regions.get(self.headers.get(site).copied().flatten()?)
    }
    pub fn derive(plan: &Plan) -> Result<Self, &'static str> {
        plan.validate_current_visits()?;
        if plan.blocks().len() > MAX_SITES || plan.values().len() > MAX_VALUES {
            return Ok(Self::default());
        }
        let facts = plan.facts();
        let mut result = Self::default();
        for (id, collection) in plan.collections().iter().enumerate() {
            if result.regions.len() == 256 {
                break;
            }
            if collection.kind() != mech_engine::ComprehensionKind::Matrix
                || !matches!(collection.result(), ResultKind::Materialized)
                || collection.generators().len() != 1
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
            let Some(element) = generator.element() else {
                continue;
            };
            let (source, range, input_width) = match generator.source() {
                Source::Value(source) => {
                    let Some(width) =
                        matrix_width(plan.values()[source.get() as usize].definition().body())
                    else {
                        continue;
                    };
                    let SchemaBody::Matrix { element: kind, .. } =
                        plan.values()[source.get() as usize].definition().body()
                    else {
                        unreachable!()
                    };
                    if NativeScalarKind::from_schema_body(kind)
                        != plan.values()[element.get() as usize].scalar_kind()
                        || !immutable_before(
                            &facts,
                            *source,
                            generator.initialize(),
                            generator.parent(),
                        )?
                    {
                        continue;
                    }
                    (*source, false, width)
                }
                Source::U64Range(cursor) => {
                    if plan.values()[element.get() as usize].scalar_kind()
                        != Some(NativeScalarKind::U64)
                    {
                        continue;
                    }
                    (cursor.semantic_output(), true, 8)
                }
            };
            let Some(output_width) = matrix_width(
                plan.values()[collection.output().get() as usize]
                    .definition()
                    .body(),
            ) else {
                continue;
            };
            if generator.collection() != id as u32
                || generator.parent() != collection.parent()
                || scalar_width(plan.values()[element.get() as usize].definition().body())
                    != Some(input_width)
                || !matches!(plan.blocks()[generator.initialize() as usize].action(), Action::Initialize { iteration: owner } if *owner == iteration)
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
                || plan.activation_boundary() == Some(generator.header())
            {
                continue;
            }
            let mut sites = BTreeSet::new();
            let mut produced = BTreeMap::new();
            let mut captures = BTreeSet::new();
            let mut steps = Vec::new();
            let mut matrices = Vec::<Matrix>::new();
            let mut reads = Vec::new();
            let mut element_seen = false;
            let mut pushed = None;
            let mut site = generator.body();
            let mut complete = true;
            while site != generator.latch() {
                if sites.len() == MAX_BODY
                    || !sites.insert(site)
                    || plan.activation_boundary() == Some(site)
                {
                    complete = false;
                    break;
                }
                let block = &plan.blocks()[site as usize];
                let Terminator::Scalar(Term::Jump(next)) = block.terminator() else {
                    complete = false;
                    break;
                };
                if block.iteration() != Some(iteration) {
                    complete = false;
                    break;
                }
                match block.action() {
                    Action::Scalar(Scalar::None) => {}
                    Action::CollectionElement { iteration: owner }
                        if *owner == iteration && !element_seen && pushed.is_none() =>
                    {
                        if !defined_here(&facts, element, site)? {
                            complete = false;
                            break;
                        }
                        produced.insert(element.get(), site);
                        element_seen = true;
                    }
                    Action::Scalar(action)
                        if element_seen && pushed.is_none() && steps.len() < MAX_STEPS =>
                    {
                        let (inputs, output) = match action {
                            Scalar::Copy { source, output } => {
                                if plan.values()[source.get() as usize].scalar_kind()
                                    != plan.values()[output.get() as usize].scalar_kind()
                                {
                                    complete = false;
                                    break;
                                }
                                (std::slice::from_ref(source), *output)
                            }
                            Scalar::Primitive {
                                operands,
                                output,
                                op,
                            } => {
                                let (expected, result) = op.signature();
                                if expected.len() != operands.len()
                                    || plan.values()[output.get() as usize].scalar_kind()
                                        != Some(result)
                                    || operands.iter().zip(expected).any(|(v, kind)| {
                                        plan.values()[v.get() as usize].scalar_kind() != Some(*kind)
                                    })
                                {
                                    complete = false;
                                    break;
                                }
                                (&operands[..], *output)
                            }
                            _ => {
                                complete = false;
                                break;
                            }
                        };
                        if plan.values()[output.get() as usize].scalar_kind().is_none()
                            || produced.contains_key(&output.get())
                            || !defined_here(&facts, output, site)?
                        {
                            complete = false;
                            break;
                        }
                        for &input in inputs {
                            if plan.values()[input.get() as usize].scalar_kind().is_none() {
                                complete = false;
                                break;
                            }
                            if !produced.contains_key(&input.get()) {
                                if !immutable_before(
                                    &facts,
                                    input,
                                    generator.initialize(),
                                    generator.parent(),
                                )? {
                                    complete = false;
                                    break;
                                }
                                captures.insert(input.get());
                            }
                        }
                        if !complete {
                            break;
                        }
                        produced.insert(output.get(), site);
                        steps.push(site);
                    }
                    Action::LibraryCall {
                        template,
                        operands,
                        output,
                    } if element_seen && pushed.is_none() && steps.len() < MAX_STEPS => {
                        if !matches!(
                            (template.primitive(), operands.len()),
                            (Primitive::MatrixScalarAccess, 2) | (Primitive::MatrixCellAccess, 3)
                        ) || template.inputs().len() != operands.len()
                            || template.output().dimension_parameters().len() != 0
                            || plan.values()[output.get() as usize].scalar_kind().is_none()
                            || produced.contains_key(&output.get())
                            || !defined_here(&facts, *output, site)?
                        {
                            complete = false;
                            break;
                        }
                        let matrix = operands[0];
                        let body = plan.values()[matrix.get() as usize].definition().body();
                        let Some(width) = matrix_width(body) else {
                            complete = false;
                            break;
                        };
                        let SchemaBody::Matrix { element: kind, .. } = body else {
                            unreachable!()
                        };
                        if template.inputs()[0].body() != body
                            || template.output().body() != kind.as_ref()
                            || !immutable_before(
                                &facts,
                                matrix,
                                generator.initialize(),
                                generator.parent(),
                            )?
                        {
                            complete = false;
                            break;
                        }
                        for &input in &operands[1..] {
                            if plan.values()[input.get() as usize].scalar_kind()
                                != Some(NativeScalarKind::U64)
                            {
                                complete = false;
                                break;
                            }
                            if !produced.contains_key(&input.get()) {
                                if !immutable_before(
                                    &facts,
                                    input,
                                    generator.initialize(),
                                    generator.parent(),
                                )? {
                                    complete = false;
                                    break;
                                }
                                captures.insert(input.get());
                            }
                        }
                        if !complete {
                            break;
                        }
                        let at = if let Some(at) =
                            matrices.iter().position(|v| v.value == matrix.get())
                        {
                            at
                        } else {
                            if matrices.len() == MAX_MATRICES {
                                complete = false;
                                break;
                            }
                            matrices.push(Matrix {
                                value: matrix.get(),
                                witness: site,
                                width,
                            });
                            matrices.len() - 1
                        };
                        reads.push(MatrixRead {
                            site,
                            matrix: at,
                            indices: operands[1..].iter().map(|v| v.get()).collect(),
                            output: output.get(),
                        });
                        produced.insert(output.get(), site);
                        steps.push(site);
                    }
                    Action::CollectionPush {
                        collection: owner,
                        value,
                    } if *owner == id as u32
                        && site == collection.push()
                        && pushed.is_none()
                        && produced.contains_key(&value.get()) =>
                    {
                        pushed = Some(*value)
                    }
                    _ => {
                        complete = false;
                        break;
                    }
                }
                site = next;
            }
            let Some(pushed) = pushed else { continue };
            let latch = &plan.blocks()[generator.latch() as usize];
            if !complete
                || !element_seen
                || captures.len() > 128
                || scalar_width(plan.values()[pushed.get() as usize].definition().body())
                    != Some(output_width)
                || !matches!(plan.values()[collection.output().get() as usize].definition().body(),
                    SchemaBody::Matrix { element, .. } if NativeScalarKind::from_schema_body(element) == plan.values()[pushed.get() as usize].scalar_kind())
                || !matches!(latch.action(), Action::Latch { iteration: owner } if *owner == iteration)
                || latch.terminator() != Terminator::Scalar(Term::Jump(generator.header()))
                || plan.activation_boundary() == Some(generator.latch())
            {
                continue;
            }
            sites.insert(generator.header());
            sites.insert(generator.latch());
            // No hidden external read may observe omitted element/scalar homes.
            for value in produced.keys() {
                let value = if *value == element.get() {
                    element
                } else {
                    match plan.blocks()[produced[value] as usize].action() {
                        Action::Scalar(
                            Scalar::Copy { output, .. } | Scalar::Primitive { output, .. },
                        )
                        | Action::LibraryCall { output, .. } => *output,
                        _ => unreachable!(),
                    }
                };
                if facts
                    .read_roles(value)?
                    .iter()
                    .any(|role| !matches!(role, Read::Action(at) if sites.contains(at)))
                {
                    complete = false;
                    break;
                }
            }
            if !complete
                || plan.blocks().iter().enumerate().any(|(at, block)| {
                    block.iteration() == Some(iteration)
                        && !sites.contains(&(at as u32))
                        && facts.is_reachable(at as u32).unwrap_or(true)
                })
            {
                continue;
            }
            if result.headers.is_empty() {
                result.headers.resize(plan.blocks().len(), None)
            }
            result.headers[generator.header() as usize] = Some(result.regions.len());
            result.regions.push(Region {
                collection: id as u32,
                iteration,
                header: generator.header(),
                exit: generator.exit(),
                source: source.get(),
                range,
                element: element.get(),
                result: pushed.get(),
                output: collection.output().get(),
                input_width,
                output_width,
                steps,
                sites: sites.into_iter().collect(),
                captures: captures.into_iter().collect(),
                matrices,
                reads,
            });
        }
        Ok(result)
    }
}
