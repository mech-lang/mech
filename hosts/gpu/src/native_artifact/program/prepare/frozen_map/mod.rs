//! Complete-use proof for a closed packed map retained through private reads.
//! This selects no storage and grants no authority. A same-invocation runtime
//! owner must still validate every exit, alias, query and retirement boundary.
use mech_engine::native_artifact::{
    NativeCollectionPrimitive as Primitive, NativeCollectionResult as ResultKind,
    NativeCollectionSource as Source, NativeControlLoop as Loop, NativeIterationAction as Action,
    NativeIterationTerminator as Terminator, NativeIterationValueScope as Scope,
    NativeMapOwnerWords, NativeMatchAction as Scalar, NativeMatchTerminator as Term,
    NativeMatchValue as Value, NativeProgramArtifactPlan as Plan,
    NativeStructuralObservation as Observation,
};
use std::collections::BTreeSet;

const MAX_SITES: usize = 16_384;
const MAX_VALUES: usize = 65_536;
const MAX_ALIASES: usize = 256;
const MAX_REGIONS: usize = 256;
const BAD: &str = "invalid frozen-map semantic inventory";

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Region {
    pub owner: usize,
    pub closed: usize,
    pub parent: Option<u32>,
    pub exit: usize,
    /// Original source value indices, never runtime handles or pointers.
    pub output: u32,
    pub aliases: Vec<u32>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Layout {
    pub regions: Vec<Region>,
    pub exits: Vec<Option<usize>>,
    pub copies: Vec<Option<usize>>,
    pub reads: Vec<Option<usize>>,
}

#[derive(Clone, Copy)]
enum Use {
    Action(usize),
    Other,
}
struct Inventory {
    uses: Vec<Vec<Use>>,
    definitions: Vec<Vec<usize>>,
}
impl Inventory {
    fn read(&mut self, value: Value, usage: Use) -> Result<(), &'static str> {
        self.uses
            .get_mut(value.get() as usize)
            .ok_or(BAD)?
            .push(usage);
        Ok(())
    }
    fn define(&mut self, value: Value, site: usize) -> Result<(), &'static str> {
        self.definitions
            .get_mut(value.get() as usize)
            .ok_or(BAD)?
            .push(site);
        Ok(())
    }

    /// Exhaustive original semantic inventory, including reads hidden by
    /// collection storage strategies. Only *actual* ordinary copies/reads are
    /// eligible below: a virtual consumer never silently becomes a read hook.
    fn derive(plan: &Plan) -> Result<Self, &'static str> {
        let mut out = Self {
            uses: vec![Vec::new(); plan.values().len()],
            definitions: vec![Vec::new(); plan.values().len()],
        };
        for (site, block) in plan.blocks().iter().enumerate() {
            let usage = Use::Action(site);
            match block.action() {
                Action::Scalar(Scalar::None) | Action::CollectionBegin { .. } => {}
                Action::Scalar(Scalar::Copy { source, output }) => {
                    out.read(*source, usage)?;
                    out.define(*output, site)?;
                }
                Action::Scalar(Scalar::Primitive {
                    operands, output, ..
                })
                | Action::LibraryCall {
                    operands, output, ..
                } => {
                    for &value in operands {
                        out.read(value, usage)?;
                    }
                    out.define(*output, site)?;
                }
                Action::Structural {
                    source,
                    observation,
                    output,
                    ..
                } => {
                    out.read(*source, usage)?;
                    if let Observation::StringEqual { peer } = observation {
                        out.read(*peer, usage)?;
                    }
                    out.define(*output, site)?;
                }
                Action::CollectionPush { value, .. } => out.read(*value, usage)?,
                Action::CollectionFinish { collection } => {
                    out.define(
                        plan.collections()
                            .get(*collection as usize)
                            .ok_or(BAD)?
                            .output(),
                        site,
                    )?;
                }
                Action::CollectionElement { iteration } => {
                    let Loop::Collection(g) =
                        plan.iterations().get(*iteration as usize).ok_or(BAD)?
                    else {
                        return Err(BAD);
                    };
                    if let Some(value) = g.element() {
                        out.define(value, site)?;
                    }
                }
                Action::Initialize { iteration } | Action::Latch { iteration } => {
                    match plan.iterations().get(*iteration as usize).ok_or(BAD)? {
                        Loop::Iteration(g) => out.define(g.carrier(), site)?,
                        Loop::Tail(g) => out.define(g.carrier(), site)?,
                        Loop::Collection(_) => {}
                    }
                }
                Action::PrepareU64Range { iteration } => {
                    let Loop::Collection(g) =
                        plan.iterations().get(*iteration as usize).ok_or(BAD)?
                    else {
                        return Err(BAD);
                    };
                    let Source::U64Range(cursor) = g.source() else {
                        return Err(BAD);
                    };
                    for &value in cursor.operands() {
                        out.read(value, usage)?;
                    }
                    out.define(cursor.semantic_output(), site)?;
                }
                Action::PrepareStringSpanSelector { collection } => {
                    let p = plan
                        .collections()
                        .get(*collection as usize)
                        .and_then(|c| c.string_span_projection())
                        .ok_or(BAD)?;
                    for &value in p.operands() {
                        out.read(value, usage)?;
                    }
                    out.define(p.selector(), site)?;
                }
                Action::ProjectStringSpan { collection } => {
                    let p = plan
                        .collections()
                        .get(*collection as usize)
                        .and_then(|c| c.string_span_projection())
                        .ok_or(BAD)?;
                    out.read(p.source(), usage)?;
                    out.read(p.selector(), usage)?;
                    out.define(p.output(), site)?;
                }
                Action::ProjectStringMapEntry { collection } => {
                    let p = plan
                        .collections()
                        .get(*collection as usize)
                        .and_then(|c| c.string_element_map_projection())
                        .ok_or(BAD)?;
                    out.read(p.map(), usage)?;
                    out.read(p.element(), usage)?;
                    out.define(p.output(), site)?;
                }
                Action::TextJoinFinish { collection } => {
                    let c = plan.collections().get(*collection as usize).ok_or(BAD)?;
                    let ResultKind::TextJoin(sink) = c.result() else {
                        return Err(BAD);
                    };
                    out.read(c.output(), usage)?;
                    out.read(sink.separator(), usage)?;
                    out.define(sink.output(), site)?;
                }
                Action::HistogramFinish { collection } => {
                    let c = plan.collections().get(*collection as usize).ok_or(BAD)?;
                    let ResultKind::Histogram(sink) = c.result() else {
                        return Err(BAD);
                    };
                    out.read(c.output(), usage)?;
                    out.define(sink.output(), site)?;
                }
                Action::HistogramQueryStep { collection, step } => {
                    let c = plan.collections().get(*collection as usize).ok_or(BAD)?;
                    let ResultKind::GuardedHistogramQuery(sink) = c.result() else {
                        return Err(BAD);
                    };
                    let step = sink.steps().get(*step as usize).ok_or(BAD)?;
                    for &value in step.operands() {
                        out.read(value, usage)?;
                    }
                    if let Some(value) = step.output() {
                        out.define(value, site)?;
                    }
                }
            }
            match block.terminator() {
                Terminator::Scalar(Term::Branch { condition, .. }) => {
                    out.read(condition, Use::Other)?
                }
                Terminator::Scalar(Term::LiteralBranch { left, right, .. }) => {
                    out.read(left, Use::Other)?;
                    out.read(right, Use::Other)?;
                }
                _ => {}
            }
        }
        // These are semantic escapes, even where a fused implementation does
        // not materialize the corresponding collection or carrier value.
        for control in plan.iterations() {
            match control {
                Loop::Collection(g) => out.read(g.source().semantic_value(), Use::Other)?,
                Loop::Iteration(g) => {
                    out.read(g.seed(), Use::Other)?;
                    out.read(g.next(), Use::Other)?;
                }
                Loop::Tail(g) => {
                    out.read(g.seed(), Use::Other)?;
                    out.read(g.next(), Use::Other)?;
                    out.read(g.returned(), Use::Other)?;
                }
            }
        }
        for index in 0..plan.transport_output_count() {
            out.read(plan.transport_output_value(index), Use::Other)?;
        }
        for index in 0..plan.transport_input_count() {
            out.define(plan.transport_input_value(index), usize::MAX)?;
        }
        for constant in plan.constants() {
            out.define(constant.value(), usize::MAX)?;
        }
        Ok(out)
    }
}

fn successors(term: Terminator) -> [Option<u32>; 2] {
    match term {
        Terminator::Scalar(Term::Jump(next)) => [Some(next), None],
        Terminator::Scalar(Term::Branch { yes, no, .. } | Term::LiteralBranch { yes, no, .. }) => {
            [Some(yes), Some(no)]
        }
        Terminator::Header { body, exit, .. } => [Some(body), Some(exit)],
        Terminator::TailHeader { body, .. } => [Some(body), None],
        Terminator::Scalar(Term::Complete | Term::Reject) => [None, None],
    }
}

/// Reachability in one original parent visit. Nested control executes normally,
/// but reaching this parent's latch/return ends the visit; a previous visit's
/// definition must never prove an alias in the next visit.
fn reachable(plan: &Plan, parent: Option<u32>, start: usize, cut: Option<usize>) -> Vec<bool> {
    let mut seen = vec![false; plan.blocks().len()];
    let mut pending = vec![start];
    while let Some(site) = pending.pop() {
        if Some(site) == cut || seen[site] {
            continue;
        }
        seen[site] = true;
        if let Some(parent) = parent {
            let loop_ = &plan.iterations()[parent as usize];
            if site == loop_.as_ref().latch() as usize
                || matches!(loop_, Loop::Tail(tail) if site == tail.finish() as usize)
            {
                continue;
            }
        }
        pending.extend(
            successors(plan.blocks()[site].terminator())
                .into_iter()
                .flatten()
                .map(|s| s as usize),
        );
    }
    seen
}

fn map_type(plan: &Plan, value: Value) -> bool {
    use mech_core::{IntegerWidth, SchemaBody};
    let ty = &plan.values()[value.get() as usize];
    ty.definition().dimension_parameters().is_empty()
        && matches!(ty.definition().body(), SchemaBody::Map { key, value, .. }
            if key.as_ref() == &SchemaBody::String
                && value.as_ref() == &SchemaBody::UnsignedInteger(IntegerWidth::W64))
}

struct Candidate {
    region: Region,
    copies: BTreeSet<usize>,
    reads: BTreeSet<usize>,
}
fn candidate(
    plan: &Plan,
    owners: &NativeMapOwnerWords,
    closed: &crate::native_artifact::program::prepare::tail::Layout,
    id: usize,
    inventory: &Inventory,
) -> Option<Candidate> {
    let region = &closed.regions[id];
    region.packed.as_ref()?;
    let owner = region.map?;
    let tail = &region.tail;
    let owned = owners.regions().get(owner)?;
    let exit = tail.exit() as usize;
    let parent = tail.parent();
    let output = tail.result();
    if owned.iteration() != region.iteration
        || owned.tail().exit() != tail.exit()
        || owned.tail().result() != output
        || owners.exit(exit) != Some(owner)
        || plan.blocks()[exit].iteration() != parent
        || !map_type(plan, output)
        || !matches!(plan.blocks()[exit].action(), Action::Scalar(Scalar::Copy { source, output: to })
            if *source == tail.returned() && *to == output)
    {
        return None;
    }
    let start = parent.map_or(plan.entry(), |p| {
        plan.iterations()[p as usize].as_ref().body()
    }) as usize;
    let reached = reachable(plan, parent, start, None);
    if !reached[exit] {
        return None;
    }
    let from_exit = reachable(plan, parent, exit, None);
    if plan
        .activation_boundary()
        .is_some_and(|site| from_exit[site as usize])
    {
        return None;
    }
    let without_exit = reachable(plan, parent, start, Some(exit));
    let mut aliases = BTreeSet::from([output.get()]);
    let mut pending = vec![output];
    let mut copies = BTreeSet::new();
    let mut reads = BTreeSet::new();
    while let Some(value) = pending.pop() {
        if plan.value_scopes()[value.get() as usize] != (Scope::Immutable { iteration: parent })
            || plan.values()[value.get() as usize] != plan.values()[output.get() as usize]
        {
            return None;
        }
        let [producer] = inventory.definitions[value.get() as usize].as_slice() else {
            return None;
        };
        if *producer >= plan.blocks().len() || (*producer != exit && !copies.contains(producer)) {
            return None;
        }
        let without_definition = reachable(plan, parent, start, Some(*producer));
        for usage in &inventory.uses[value.get() as usize] {
            let Use::Action(site) = *usage else {
                return None;
            };
            if !reached[site]
                || !from_exit[site]
                || without_exit[site]
                || without_definition[site]
                || site == *producer
                || plan.blocks()[site].iteration() != parent
                || plan.activation_boundary() == Some(site as u32)
            {
                return None;
            }
            match plan.blocks()[site].action() {
                Action::Scalar(Scalar::Copy {
                    source,
                    output: next,
                }) if *source == value => {
                    copies.insert(site);
                    if aliases.insert(next.get()) {
                        if aliases.len() > MAX_ALIASES {
                            return None;
                        }
                        pending.push(*next);
                    }
                }
                Action::LibraryCall {
                    template,
                    operands,
                    output: result,
                } if matches!(
                    template.primitive(),
                    Primitive::MapContainsKey | Primitive::MapKeyAccess
                ) && operands.len() == 2
                    && operands[0] == value
                    && operands[1] != value
                    && *result != value
                    && plan.values()[operands[1].get() as usize]
                        .definition()
                        .body()
                        == &mech_core::SchemaBody::String
                    && plan.values()[operands[1].get() as usize]
                        .definition()
                        .dimension_parameters()
                        .is_empty() =>
                {
                    reads.insert(site);
                }
                _ => return None,
            }
        }
    }
    if reads.is_empty() {
        return None;
    }
    Some(Candidate {
        region: Region {
            owner,
            closed: id,
            parent,
            exit,
            output: output.get(),
            aliases: aliases.into_iter().collect(),
        },
        copies,
        reads,
    })
}

impl Layout {
    pub fn derive(
        plan: &Plan,
        owners: &NativeMapOwnerWords,
        closed: &crate::native_artifact::program::prepare::tail::Layout,
    ) -> Result<Self, &'static str> {
        plan.validate_current_visits()?;
        let mut layout = Self {
            exits: vec![None; plan.blocks().len()],
            copies: vec![None; plan.blocks().len()],
            reads: vec![None; plan.blocks().len()],
            ..Self::default()
        };
        if plan.blocks().len() > MAX_SITES
            || plan.values().len() > MAX_VALUES
            || closed.regions.len() > MAX_REGIONS
        {
            return Ok(layout);
        }
        let inventory = Inventory::derive(plan)?;
        for id in 0..closed.regions.len() {
            let Some(candidate) = candidate(plan, owners, closed, id, &inventory) else {
                continue;
            };
            if layout.exits[candidate.region.exit].is_some()
                || candidate
                    .copies
                    .iter()
                    .any(|&site| layout.copies[site].is_some())
                || candidate
                    .reads
                    .iter()
                    .any(|&site| layout.reads[site].is_some())
            {
                continue;
            }
            let index = layout.regions.len();
            layout.exits[candidate.region.exit] = Some(index);
            for site in candidate.copies {
                layout.copies[site] = Some(index);
            }
            for site in candidate.reads {
                layout.reads[site] = Some(index);
            }
            layout.regions.push(candidate.region);
        }
        Ok(layout)
    }
}

#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "tests.rs"]
pub(in crate::native_artifact::program::prepare::frozen_map) mod tests;
