//! Compile-time, same-Program element bounds. No serialized range certificate
//! or invocation cache is trusted. Every fact is reconstructed from this exact
//! checked Program's immutable values and every reaching constructor/write.
//!
//! Program sessions retain their private arena across generated checkpoints;
//! callers cannot import internal slots. Worker captures are copies from the
//! same checked parent's immutable frontier, never arbitrary caller arguments.
//! These facts MUST NOT be reused by generic resumable iteration entry points.
use crate::native_artifact::codegen::control::plan::CodePlan;
use crate::native_artifact::program::prepare::readonly_numeric::{Operation, Proof as Numeric};
use mech_core::{IntegerWidth, SchemaBody, ValueData};
use mech_engine::native_artifact::{
    NativeCollectionPrimitive as Primitive, NativeCollectionResult, NativeControlLoop,
    NativeIterationAction as Action, NativeIterationTerminator as Term,
    NativeIterationValueScope as Scope, NativeMatchAction as Scalar,
    NativeMatchTerminator as Branch, NativeProgramArtifactPlan as Plan, NativeScalarOp as Op,
    NativeStructuralStep,
};
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
#[path = "producer_element_ranges/contents.rs"]
mod contents;
#[path = "producer_element_ranges/length.rs"]
mod length;

// Exhaustion is a proof miss, never a compilation or source execution error.
const LIMIT: usize = 16_384;
const MAX_PROOF_WORK: usize = 4_194_304;
fn spend(remaining: &Cell<usize>, work: usize) -> Option<()> {
    let next = remaining.get().checked_sub(work);
    remaining.set(next.unwrap_or(0));
    next.map(|_| ())
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct Ref(u32, Vec<usize>);
impl Ref {
    fn value(v: u32) -> Self {
        Self(v, vec![])
    }
    fn field(v: u32, f: usize) -> Self {
        Self(v, vec![f])
    }
    fn with(&self, value: u32) -> Self {
        Self(value, self.1.clone())
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum Atom {
    Source(Ref),
    Word(u64),
    Bool(bool),
}
#[derive(Clone, Debug, Eq, PartialEq)]
enum Ty {
    Word,
    Bool,
    Array,
    Tuple(Vec<Ty>),
    Other,
}
impl Ty {
    fn from(body: &SchemaBody, remaining: &Cell<usize>) -> Option<Self> {
        Self::walk(body, remaining, 0)
    }
    fn walk(body: &SchemaBody, remaining: &Cell<usize>, depth: usize) -> Option<Self> {
        if depth > 64 {
            return None;
        }
        spend(remaining, 1)?;
        Some(match body {
            SchemaBody::UnsignedInteger(
                IntegerWidth::W8 | IntegerWidth::W16 | IntegerWidth::W32 | IntegerWidth::W64,
            ) => Self::Word,
            SchemaBody::Bool => Self::Bool,
            SchemaBody::Matrix { .. } => Self::Array,
            SchemaBody::Tuple(fields) => Self::Tuple(
                fields
                    .iter()
                    .map(|field| Self::walk(field, remaining, depth + 1))
                    .collect::<Option<Vec<_>>>()?,
            ),
            _ => Self::Other,
        })
    }
    fn at(&self, path: &[usize]) -> Option<&Self> {
        let mut ty = self;
        for &field in path {
            let Self::Tuple(fields) = ty else { return None };
            ty = fields.get(field)?;
        }
        Some(ty)
    }
    fn leaves(
        &self,
        prefix: Vec<usize>,
        out: &mut Vec<Vec<usize>>,
        remaining: &Cell<usize>,
    ) -> Option<()> {
        spend(remaining, prefix.len().checked_add(1)?)?;
        if let Self::Tuple(fields) = self {
            for (i, field) in fields.iter().enumerate() {
                let mut p = prefix.clone();
                p.push(i);
                field.leaves(p, out, remaining)?;
            }
        } else {
            out.push(prefix);
        }
        Some(())
    }
}
#[derive(Clone)]
enum Def {
    Copy(Ref),
    Project(Ref),
    Scalar(Op, Vec<u32>),
    Library(Primitive, Vec<u32>),
    Opaque,
}
#[derive(Clone)]
struct Tail {
    id: u32,
    parent: Option<u32>,
    seed: u32,
    carrier: u32,
    next: u32,
    returned: u32,
    result: u32,
    header: usize,
    body: usize,
    latch: usize,
    finish: usize,
    exit: usize,
}
struct Graph {
    types: Vec<Ty>,
    definitions: Vec<Vec<(usize, Def)>>,
    constants: BTreeMap<u32, Atom>,
    tails: BTreeMap<u32, Tail>,
    invariant: BTreeMap<Ref, Ref>,
    remaining: Cell<usize>,
}
fn unsigned_cast(op: Op) -> bool {
    matches!(
        op,
        Op::U8ToU16
            | Op::U8ToU32
            | Op::U8ToU64
            | Op::U16ToU8
            | Op::U16ToU32
            | Op::U16ToU64
            | Op::U32ToU8
            | Op::U32ToU16
            | Op::U32ToU64
            | Op::U64ToU8
            | Op::U64ToU16
            | Op::U64ToU32
    )
}
impl Graph {
    fn new(plan: &Plan) -> Option<Self> {
        if plan.blocks().len() > LIMIT || plan.values().len() > LIMIT {
            return None;
        }
        let remaining = Cell::new(MAX_PROOF_WORK);
        let types = plan
            .values()
            .iter()
            .map(|v| Ty::from(v.definition().body(), &remaining))
            .collect::<Option<Vec<_>>>()?;
        let mut definitions = vec![vec![]; types.len()];
        for (site, block) in plan.blocks().iter().enumerate() {
            spend(&remaining, 1)?;
            match block.action() {
                Action::Scalar(Scalar::Primitive { operands, .. })
                | Action::LibraryCall { operands, .. } => spend(&remaining, operands.len())?,
                Action::Structural { path, .. } => spend(&remaining, path.len())?,
                _ => (),
            }
            let definition = match block.action() {
                Action::Scalar(Scalar::Copy { source, output }) => {
                    Some((output.get(), Def::Copy(Ref::value(source.get()))))
                }
                Action::Scalar(Scalar::Primitive {
                    op,
                    operands,
                    output,
                }) => Some((
                    output.get(),
                    Def::Scalar(*op, operands.iter().map(|v| v.get()).collect()),
                )),
                Action::Structural {
                    source,
                    path,
                    output,
                    ..
                } => {
                    let path = path
                        .iter()
                        .map(|step| match step {
                            NativeStructuralStep::TupleItem { ordinal } => Some(*ordinal as usize),
                            _ => None,
                        })
                        .collect::<Option<Vec<_>>>();
                    Some((
                        output.get(),
                        path.map(|path| Def::Project(Ref(source.get(), path)))
                            .unwrap_or(Def::Opaque),
                    ))
                }
                Action::LibraryCall {
                    template,
                    operands,
                    output,
                } => Some((
                    output.get(),
                    Def::Library(
                        template.primitive(),
                        operands.iter().map(|v| v.get()).collect(),
                    ),
                )),
                _ => None,
            };
            if let Some((output, definition)) = definition {
                definitions
                    .get_mut(output as usize)?
                    .push((site, definition));
            }
        }
        let constants = plan
            .constants()
            .iter()
            .filter_map(|c| {
                let atom = match c.constant().data() {
                    ValueData::Bool(v) => Atom::Bool(*v),
                    ValueData::U8(v) => Atom::Word(u64::from(*v)),
                    ValueData::U16(v) => Atom::Word(u64::from(*v)),
                    ValueData::U32(v) => Atom::Word(u64::from(*v)),
                    ValueData::U64(v) => Atom::Word(*v),
                    _ => return None,
                };
                Some((c.value().get(), atom))
            })
            .collect();
        let tails = plan
            .iterations()
            .iter()
            .enumerate()
            .filter_map(|(i, it)| {
                let NativeControlLoop::Tail(t) = it else {
                    return None;
                };
                Some((
                    t.carrier().get(),
                    Tail {
                        id: i as u32,
                        parent: t.parent(),
                        seed: t.seed().get(),
                        carrier: t.carrier().get(),
                        next: t.next().get(),
                        returned: t.returned().get(),
                        result: t.result().get(),
                        header: t.header() as usize,
                        body: t.body() as usize,
                        latch: t.latch() as usize,
                        finish: t.finish() as usize,
                        exit: t.exit() as usize,
                    },
                ))
            })
            .collect();
        let mut graph = Self {
            types,
            definitions,
            constants,
            tails,
            invariant: BTreeMap::new(),
            remaining,
        };
        graph.charge(
            plan.blocks()
                .len()
                .checked_add(plan.values().len())?
                .checked_add(plan.constants().len())?
                .checked_add(plan.iterations().len())?,
        )?;
        // Exact carried fields only. Differing next definitions are never
        // collapsed to a source identity or a content/version assertion.
        for _ in 0..LIMIT {
            let mut added = vec![];
            for t in graph.tails.values() {
                graph.charge(1)?;
                let mut leaves = vec![];
                graph.types[t.carrier as usize].leaves(vec![], &mut leaves, &graph.remaining)?;
                for path in leaves {
                    graph.charge(1)?;
                    let field = Ref(t.carrier, path);
                    if graph.invariant.contains_key(&field) {
                        continue;
                    }
                    let stop = BTreeSet::from([field.clone()]);
                    if graph.resolve(&field.with(t.next), &stop)
                        == Some(BTreeSet::from([Atom::Source(field.clone())]))
                    {
                        added.push((field.clone(), field.with(t.seed)));
                    }
                }
            }
            if added.is_empty() {
                if graph.exhausted() {
                    return None;
                }
                return Some(graph);
            }
            graph.invariant.extend(added);
        }
        None
    }
    fn ty(&self, r: &Ref) -> Option<&Ty> {
        self.charge(r.1.len().checked_add(1)?)?;
        self.types.get(r.0 as usize)?.at(&r.1)
    }
    fn charge(&self, work: usize) -> Option<()> {
        spend(&self.remaining, work)
    }
    fn exhausted(&self) -> bool {
        self.remaining.get() == 0
    }
    fn tail(&self, id: u32) -> Option<&Tail> {
        self.charge(self.tails.len())?;
        self.tails.values().find(|tail| tail.id == id)
    }
    fn resolve(&self, r: &Ref, stop: &BTreeSet<Ref>) -> Option<BTreeSet<Atom>> {
        fn go(
            g: &Graph,
            r: &Ref,
            stop: &BTreeSet<Ref>,
            active: &mut BTreeSet<Ref>,
            remaining: &mut usize,
        ) -> Option<BTreeSet<Atom>> {
            g.charge(r.1.len().checked_add(1)?)?;
            *remaining = remaining.checked_sub(1)?;
            if active.len() >= 128 {
                return None;
            }
            if stop.contains(r) {
                return Some(BTreeSet::from([Atom::Source(r.clone())]));
            }
            if !active.insert(r.clone()) {
                return None;
            }
            let result = (|| {
                if let Some(seed) = g.invariant.get(r) {
                    return go(g, seed, stop, active, remaining);
                }
                if r.1.is_empty()
                    && let Some(c) = g.constants.get(&r.0)
                {
                    return Some(BTreeSet::from([c.clone()]));
                }
                if g.tails.contains_key(&r.0) {
                    return Some(BTreeSet::from([Atom::Source(r.clone())]));
                }
                let defs = g.definitions.get(r.0 as usize)?;
                if defs.is_empty() {
                    return Some(BTreeSet::from([Atom::Source(r.clone())]));
                }
                let mut result = BTreeSet::new();
                for (_, definition) in defs {
                    g.charge(1)?;
                    let child = match definition {
                        Def::Copy(source) => Some(Ref(source.0, r.1.clone())),
                        Def::Project(source) => {
                            let mut p = source.1.clone();
                            p.extend(&r.1);
                            Some(Ref(source.0, p))
                        }
                        Def::Scalar(op, operands)
                            if unsigned_cast(*op) && r.1.is_empty() && operands.len() == 1 =>
                        {
                            Some(Ref::value(operands[0]))
                        }
                        Def::Library(Primitive::CompositePack(_), operands) if !r.1.is_empty() => {
                            Some(Ref(*operands.get(r.1[0])?, r.1[1..].to_vec()))
                        }
                        _ => None,
                    };
                    if let Some(child) = child {
                        result.extend(go(g, &child, stop, active, remaining)?);
                    } else {
                        result.insert(Atom::Source(r.clone()));
                    }
                }
                Some(result)
            })();
            active.remove(r);
            result
        }
        let mut remaining = LIMIT;
        go(self, r, stop, &mut BTreeSet::new(), &mut remaining)
    }
    fn one(&self, r: &Ref, stop: &BTreeSet<Ref>) -> Option<Atom> {
        let mut all = self.resolve(r, stop)?;
        if all.len() != 1 {
            return None;
        }
        all.pop_first()
    }
    fn atom(&self, r: &Ref) -> Option<Atom> {
        self.one(r, &BTreeSet::new())
    }
    fn definition(&self, value: u32) -> Option<(usize, &Def)> {
        self.charge(1)?;
        let defs = self.definitions.get(value as usize)?;
        if defs.len() != 1 {
            return None;
        }
        Some((defs[0].0, &defs[0].1))
    }
    fn source(&self, value: u32) -> Option<Ref> {
        let Atom::Source(r) = self.atom(&Ref::value(value))? else {
            return None;
        };
        Some(r)
    }
}

fn successors(plan: &Plan, site: usize) -> Vec<usize> {
    match plan.blocks()[site].terminator() {
        Term::Scalar(Branch::Jump(next)) => vec![next as usize],
        Term::Scalar(Branch::LiteralBranch { yes, no, .. } | Branch::Branch { yes, no, .. })
        | Term::Header {
            body: yes,
            exit: no,
            ..
        } => vec![yes as usize, no as usize],
        Term::TailHeader { body, .. } => vec![body as usize],
        _ => vec![],
    }
}
fn reaches(
    plan: &Plan,
    graph: &Graph,
    start: usize,
    target: usize,
    removed: Option<(usize, usize)>,
    stop: Option<usize>,
) -> bool {
    let mut seen = BTreeSet::new();
    let mut pending = vec![start];
    while let Some(at) = pending.pop() {
        if graph.charge(1).is_none() {
            return false;
        }
        if at == target {
            return true;
        }
        if Some(at) == stop || !seen.insert(at) {
            continue;
        }
        pending.extend(
            successors(plan, at)
                .into_iter()
                .filter(|&next| removed != Some((at, next))),
        );
    }
    false
}
fn edge_dominates(
    plan: &Plan,
    graph: &Graph,
    start: usize,
    target: usize,
    edge: (usize, usize),
    stop: Option<usize>,
) -> bool {
    reaches(plan, graph, start, target, None, stop)
        && !reaches(plan, graph, start, target, Some(edge), stop)
}
fn true_edges(
    plan: &Plan,
    graph: &Graph,
    predicate: &Ref,
    stops: &BTreeSet<Ref>,
) -> Vec<(usize, usize)> {
    plan.blocks()
        .iter()
        .enumerate()
        .filter_map(|(site, block)| {
            graph.charge(1)?;
            let Branch::LiteralBranch {
                left,
                right,
                yes,
                no,
            } = (match block.terminator() {
                Term::Scalar(t) => t,
                _ => return None,
            })
            else {
                return None;
            };
            if graph.one(&Ref::value(left.get()), stops)? != Atom::Source(predicate.clone()) {
                return None;
            }
            let Atom::Bool(want) = graph.atom(&Ref::value(right.get()))? else {
                return None;
            };
            Some((site, (if want { yes } else { no }) as usize))
        })
        .collect()
}

/// Called only for a checked Program; no public constructor can import arbitrary
/// runtime internal arrays. Image restore reconstructs the same source plan.
pub(super) fn derive(
    code: CodePlan<'_>,
    numeric: &Numeric,
    controls: &crate::native_artifact::codegen::options::CompilationControls,
) -> BTreeSet<(usize, usize, u32)> {
    let CodePlan::Program(program) = code else {
        return BTreeSet::new();
    };
    if !controls
        .enabled(crate::native_artifact::codegen::options::OptimizationPass::ProducerElementRanges)
    {
        return BTreeSet::new();
    }
    let source_reads = program.producer_element_reads(source_reads);
    numeric
        .sites
        .iter()
        .filter_map(|&site| {
            let Operation::Read { array, index, .. } = numeric.action(site)? else {
                return None;
            };
            let target = &numeric.arrays[*array];
            if target.writable || target.appends || !source_reads.contains(&(site, *index)) {
                return None;
            }
            Some((site, *array, *index))
        })
        .collect()
}

fn source_reads(plan: &Plan) -> BTreeSet<(usize, u32)> {
    let Some(graph) = Graph::new(plan) else {
        return BTreeSet::new();
    };
    let mut proved = BTreeSet::new();
    for fact in length::derive(plan, &graph) {
        if graph.charge(1).is_none() {
            return BTreeSet::new();
        }
        // First implementation deliberately requires an invocation-global
        // immutable bound. It cannot accidentally compare different visits of
        // an outer carried scalar with the same logical value ordinal.
        if !fact.bound.1.is_empty()
            || !matches!(
                plan.value_scopes().get(fact.bound.0 as usize),
                Some(Scope::Immutable { iteration: None })
            )
            || !matches!(
                plan.value_scopes().get(fact.result as usize),
                Some(Scope::Immutable { iteration: None })
            )
        {
            continue;
        }
        let target = Ref(fact.result, vec![fact.array]);
        let flag = Ref(fact.result, vec![fact.success]);
        let stops = BTreeSet::from([target.clone(), flag.clone()]);
        let selection = true_edges(plan, &graph, &flag, &stops);
        for (site, block) in plan.blocks().iter().enumerate() {
            if graph.charge(1).is_none() {
                return BTreeSet::new();
            }
            let Action::LibraryCall {
                template, operands, ..
            } = block.action()
            else {
                continue;
            };
            if template.primitive() != Primitive::MatrixScalarAccess || operands.len() != 2 {
                continue;
            }
            if graph.one(&Ref::value(operands[0].get()), &stops)
                != Some(Atom::Source(target.clone()))
            {
                continue;
            }
            let index = operands[1].get();
            let Some(index_ref) = graph.source(index) else {
                continue;
            };
            if !index_ref.1.is_empty() {
                continue;
            }
            let Some((_, Def::Scalar(Op::U64Add, add))) = graph.definition(index_ref.0) else {
                continue;
            };
            if add.len() != 2 {
                continue;
            }
            let load = match (
                graph.atom(&Ref::value(add[0])),
                graph.atom(&Ref::value(add[1])),
            ) {
                (Some(Atom::Source(r)), Some(Atom::Word(1)))
                | (Some(Atom::Word(1)), Some(Atom::Source(r)))
                    if r.1.is_empty() =>
                {
                    r
                }
                _ => continue,
            };
            let Some((_, Def::Library(Primitive::MatrixScalarAccess, read))) =
                graph.definition(load.0)
            else {
                continue;
            };
            if read.len() != 2 || !contents::prove(plan, &graph, Ref::value(read[0]), &fact.bound) {
                continue;
            }
            if !selection
                .iter()
                .any(|&edge| edge_dominates(plan, &graph, plan.entry() as usize, site, edge, None))
            {
                continue;
            }
            // Zero-initialized holes are safe only after an original checked
            // target read in THIS traversal established nonempty geometry.
            // Keep that read's check; do not let two removed checks prove each
            // other. Earlier source reads are selected without recursion.
            if !contents::positive_target(plan, &graph, site, &target, &stops) {
                continue;
            }
            proved.insert((site, index));
        }
    }
    if graph.exhausted() {
        BTreeSet::new()
    } else {
        proved
    }
}
