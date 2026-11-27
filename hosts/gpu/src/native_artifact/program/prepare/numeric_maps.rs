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

#[path = "numeric_map_nested.rs"]
pub(in crate::native_artifact) mod nested;
#[path = "numeric_rectangles.rs"]
mod rectangles;
#[path = "numeric_map_reductions.rs"]
mod reductions;
pub(in crate::native_artifact) use rectangles::{Bounds as RectangleBounds, Rectangle};
pub(in crate::native_artifact) use reductions::Reduction;
#[path = "numeric_map_transposes.rs"]
mod transposes;
pub(in crate::native_artifact) use transposes::{FINISH_WORK as TRANSPOSE_FINISH_WORK, Transpose};

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
    pub nested: Vec<nested::Reduction>,
    pub rectangle: Option<Rectangle>,
    pub reduction: Option<Reduction>,
    pub transpose: Option<Transpose>,
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
    /// Complete acyclic element body in deterministic topological order.
    /// Branches and phi copies retain their original source CFG edges.
    pub body: Vec<u32>,
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
pub(super) fn immutable_before(
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
    // A current immutable ancestor phi may have several source Copy sites.
    // Each selecting use is an actual source/operand/terminator read, whose
    // definite assignment was checked by validate_current_visits. Initialize
    // jumps directly to the empty header, and no ancestor action occurs in
    // this exact same-iteration body, so that current word/owner is available
    // before entry and cannot change during the synchronous native loan.
    if definitions.len() > 1
        && definitions.iter().all(|d| {
            d.role() == Producer::Scalar && d.iteration() == iteration && d.site().is_some()
        })
    {
        return Ok(true);
    }
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
/// Scalar reads of an admitted source site. Captured matrix owners use their
/// separate exact read-factory proof; they are never scalar transport words.
pub(in crate::native_artifact) fn scalar_reads(plan: &Plan, site: u32) -> Vec<Value> {
    let block = &plan.blocks()[site as usize];
    let mut reads = match block.action() {
        Action::Scalar(Scalar::Copy { source, .. }) => vec![*source],
        Action::Scalar(Scalar::Primitive { operands, .. }) => operands.to_vec(),
        Action::LibraryCall { operands, .. } => operands[1..].to_vec(),
        Action::CollectionPush { value, .. } => vec![*value],
        _ => Vec::new(),
    };
    match block.terminator() {
        Terminator::Scalar(Term::Branch { condition, .. }) => reads.push(condition),
        Terminator::Scalar(Term::LiteralBranch { left, right, .. }) => reads.extend([left, right]),
        _ => {}
    }
    reads
}
fn successors(term: Terminator) -> Option<Vec<u32>> {
    Some(match term {
        Terminator::Scalar(Term::Jump(to)) => vec![to],
        Terminator::Scalar(Term::Branch { yes, no, .. } | Term::LiteralBranch { yes, no, .. }) => {
            if yes == no {
                vec![yes]
            } else {
                vec![yes, no]
            }
        }
        Terminator::Scalar(Term::Reject | Term::IntegrityFailure) => vec![],
        _ => return None,
    })
}
fn ordered_body(
    plan: &Plan,
    iteration: u32,
    first: u32,
    latch: u32,
    summaries: &[super::numeric_phase::Reduction],
) -> Option<Vec<u32>> {
    let mut edges = BTreeMap::new();
    let mut pending = vec![first];
    while let Some(site) = pending.pop() {
        if site == latch || edges.contains_key(&site) {
            continue;
        }
        if edges.len() == MAX_BODY || plan.activation_boundary() == Some(site) {
            return None;
        }
        let block = plan.blocks().get(site as usize)?;
        if block.iteration() != Some(iteration) {
            return None;
        }
        let next = if let Some(summary) = summaries.iter().find(|r| {
            r.entry == site
                && plan.iterations()[r.region.iteration as usize]
                    .as_ref()
                    .parent()
                    == Some(iteration)
        }) {
            vec![summary.after]
        } else {
            successors(block.terminator())?
        };
        pending.extend(next.iter().copied());
        edges.insert(site, next);
    }
    let mut degrees: BTreeMap<_, usize> = edges.keys().map(|&at| (at, 0)).collect();
    for next in edges.values() {
        for &to in next {
            if to != latch {
                *degrees.get_mut(&to)? += 1;
            }
        }
    }
    let mut ready: BTreeSet<_> = degrees
        .iter()
        .filter_map(|(&at, &n)| (n == 0).then_some(at))
        .collect();
    let mut order = Vec::new();
    while let Some(site) = ready.pop_first() {
        order.push(site);
        for &to in &edges[&site] {
            if to == latch {
                continue;
            }
            let degree = degrees.get_mut(&to)?;
            *degree -= 1;
            if *degree == 0 {
                ready.insert(to);
            }
        }
    }
    (order.len() == edges.len() && order.first() == Some(&first)).then_some(order)
}
impl Layout {
    pub fn at_header(&self, site: usize) -> Option<&Region> {
        self.regions.get(self.headers.get(site).copied().flatten()?)
    }
    pub fn at_transpose_finish(&self, site: usize) -> Option<&Region> {
        self.regions.iter().find(|region| {
            region
                .transpose
                .as_ref()
                .is_some_and(|p| p.finish as usize == site)
        })
    }
    pub fn at_reduction_finish(&self, site: usize) -> Option<&Region> {
        self.regions.iter().find(|region| {
            region
                .reduction
                .as_ref()
                .is_some_and(|reduction| reduction.finish as usize == site)
        })
    }
    pub fn derive(plan: &Plan) -> Result<Self, &'static str> {
        let flat = Self::derive_with(plan, &[])?;
        let summaries = flat
            .regions
            .iter()
            .cloned()
            .filter_map(|r| super::numeric_phase::Reduction::from_region(plan, r))
            .collect::<Vec<_>>();
        if summaries.is_empty() {
            return Ok(flat);
        }
        Self::derive_with(plan, &summaries)
    }
    fn derive_with(
        plan: &Plan,
        summaries: &[super::numeric_phase::Reduction],
    ) -> Result<Self, &'static str> {
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
            let Some(body) = ordered_body(
                plan,
                iteration,
                generator.body(),
                generator.latch(),
                summaries,
            ) else {
                continue;
            };
            let mut sites: BTreeSet<_> = body.iter().copied().collect();
            let mut produced = BTreeMap::<u32, Value>::new();
            let mut captures = BTreeSet::new();
            let mut steps = Vec::new();
            let mut matrices = Vec::<Matrix>::new();
            let mut reads = Vec::new();
            let mut nested = Vec::new();
            let mut element_seen = false;
            let mut pushed = None;
            let mut complete = true;
            for &site in &body {
                let block = &plan.blocks()[site as usize];
                let output = if let Some(summary) = summaries.iter().find(|r| {
                    r.entry == site
                        && plan.iterations()[r.region.iteration as usize]
                            .as_ref()
                            .parent()
                            == Some(iteration)
                }) {
                    if steps.len() >= MAX_STEPS || nested.len() >= nested::MAX_CHILDREN {
                        complete = false;
                        break;
                    }
                    let Some(child) = nested::Reduction::admit(
                        plan,
                        &facts,
                        generator.initialize(),
                        generator.parent(),
                        summary.clone(),
                        &mut matrices,
                    )?
                    else {
                        complete = false;
                        break;
                    };
                    sites.extend(child.source.source_sites());
                    let output = child.output(plan);
                    steps.push(site);
                    nested.push(child);
                    Some(output)
                } else {
                    match block.action() {
                        Action::Scalar(Scalar::None) => None,
                        Action::CollectionElement { iteration: owner }
                            if *owner == iteration && site == generator.body() && !element_seen =>
                        {
                            element_seen = true;
                            Some(element)
                        }
                        Action::Scalar(action) if steps.len() < MAX_STEPS => {
                            let output = match action {
                                Scalar::Copy { source, output } => {
                                    if plan.values()[source.get() as usize].scalar_kind()
                                        != plan.values()[output.get() as usize].scalar_kind()
                                    {
                                        complete = false;
                                        break;
                                    }
                                    *output
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
                                            plan.values()[v.get() as usize].scalar_kind()
                                                != Some(*kind)
                                        })
                                    {
                                        complete = false;
                                        break;
                                    }
                                    *output
                                }
                                _ => {
                                    complete = false;
                                    break;
                                }
                            };
                            steps.push(site);
                            Some(output)
                        }
                        Action::LibraryCall {
                            template,
                            operands,
                            output,
                        } if steps.len() < MAX_STEPS => {
                            if !matches!(
                                (template.primitive(), operands.len()),
                                (Primitive::MatrixScalarAccess, 2)
                                    | (Primitive::MatrixCellAccess, 3)
                            ) {
                                complete = false;
                                break;
                            }
                            let matrix = operands[0];
                            let definition = plan.values()[matrix.get() as usize].definition();
                            let Some(width) = matrix_width(definition.body()) else {
                                complete = false;
                                break;
                            };
                            let SchemaBody::Matrix { element: kind, .. } = definition.body() else {
                                unreachable!()
                            };
                            if template.inputs().len() != operands.len()
                                || template.inputs()[0] != *definition
                                || template.output().body() != kind.as_ref()
                                || !template.output().dimension_parameters().is_empty()
                                || operands[1..].iter().any(|v| {
                                    plan.values()[v.get() as usize].scalar_kind()
                                        != Some(NativeScalarKind::U64)
                                })
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
                            let at = if let Some(at) =
                                matrices.iter().position(|m| m.value == matrix.get())
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
                            steps.push(site);
                            Some(*output)
                        }
                        Action::CollectionPush {
                            collection: owner,
                            value,
                        } if *owner == id as u32
                            && site == collection.push()
                            && pushed.is_none() =>
                        {
                            pushed = Some(*value);
                            None
                        }
                        _ => {
                            complete = false;
                            break;
                        }
                    }
                };
                if let Some(value) = output {
                    if plan.values()[value.get() as usize].scalar_kind().is_none() {
                        complete = false;
                        break;
                    }
                    produced.insert(value.get(), value);
                }
            }
            let Some(pushed) = pushed else { continue };
            let latch = &plan.blocks()[generator.latch() as usize];
            if !complete || !element_seen
                || scalar_width(plan.values()[pushed.get() as usize].definition().body()) != Some(output_width)
                || !matches!(plan.values()[collection.output().get() as usize].definition().body(),
                    SchemaBody::Matrix { element, .. } if NativeScalarKind::from_schema_body(element) == plan.values()[pushed.get() as usize].scalar_kind())
                || !matches!(latch.action(), Action::Latch { iteration: owner } if *owner == iteration)
                || latch.terminator() != Terminator::Scalar(Term::Jump(generator.header()))
                || plan.activation_boundary() == Some(generator.latch())
                // Exactly one Push dominates every successful element. Filters
                // and variable output counts retain their ordinary route.
                || !facts.strictly_dominates(collection.push(), generator.latch())?
            {
                continue;
            }
            sites.insert(generator.header());
            sites.insert(generator.latch());
            // Current-visits validation establishes definite assignment on
            // every incoming path and iteration. Permit source phi copies,
            // but require their complete definitions and reads inside this
            // exact acyclic body; no omitted word may escape the region.
            for (&id, &value) in &produced {
                let definitions = facts.definition_sites(id)?;
                if definitions.is_empty()
                    || definitions.iter().any(|d| d.site().is_none_or(|at| !sites.contains(&at)))
                    || facts.read_roles(value)?.iter().any(|role|
                        !matches!(role, Read::Action(at) | Read::Terminator(at) if sites.contains(at))) {
                    complete = false; break;
                }
            }
            for &site in &body {
                for input in nested::reads(plan, site, &nested) {
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
            }
            if !complete
                || captures.len() > 128
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
                nested,
                rectangle: None,
                reduction: None,
                transpose: None,
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
                body,
                sites: sites.into_iter().collect(),
                captures: captures.into_iter().collect(),
                matrices,
                reads,
            });
        }
        rectangles::extend(plan, &mut result)?;
        reductions::extend(plan, &mut result);
        transposes::extend(plan, &mut result);
        Ok(result)
    }
}
