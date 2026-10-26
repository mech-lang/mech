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
pub(super) struct Region {
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
pub(super) struct Layout {
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
    closed: &super::program_tail_layout::Layout,
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
        closed: &super::program_tail_layout::Layout,
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
mod tests {
    use super::*;
    use mech_syntax::document::{
        AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
        parse_canonical_document,
    };

    const COUNT: &str = r#"
+> map
+> sequence
increment(h<{string:u64}>, key<string>) => (out<{string:u64}>) :=
  previous := (key ∈ h)? | true => h{key} | false => 0u64.
  out := map/insert(h, key, previous + 1u64).
#Accumulate(s<string>, width<u64>, n<u64>, initial<{string:u64}>) => <{string:u64}>
  | :Advance(index<u64>, counts<{string:u64}>)
  | :Finish(counts<{string:u64}>).
#Accumulate(s, width, n, initial) -> :Advance(0u64, initial)
  :Advance(index, counts)
    | width == 0u64 -> :Finish(counts)
    | index >= n -> :Finish(counts)
    | width > n - index -> :Finish(counts)
    | * -> :Advance(index + 1u64, increment(counts, s[(index + 1u64)..=(index + width)]))
  :Finish(counts) => counts.
empty<{string:u64}> := map/new()
"#;

    fn artifact(source: &str) -> mech_engine::ProgramArtifact {
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x46524f5a), Revision(1), source).unwrap(),
            ParseConfig::default(),
        );
        mech_engine::CanonicalSourceFrontend
            .compile_document_with_catalog(
                &DocumentSyntax::cast(parsed.syntax()).unwrap(),
                mech_stdlib::source_catalog(),
            )
            .unwrap()
            .compile_artifact()
            .unwrap()
    }
    fn parts(
        artifact: &mech_engine::ProgramArtifact,
    ) -> (
        Plan,
        NativeMapOwnerWords,
        super::super::program_tail_layout::Layout,
    ) {
        let plan = mech_engine::native_artifact::lower_native_program_artifact(
            artifact,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        let owners = plan.map_owner_words().unwrap();
        let borrowed =
            super::super::program_borrowed_layout::Layout::derive(&plan, &plan.scalar_aggregates())
                .unwrap();
        let visits = super::super::iteration_abi::control_visits(
            plan.blocks(),
            plan.iterations().iter().map(Into::into),
            0,
        )
        .unwrap();
        let closed = super::super::program_tail_layout::Layout::derive(
            &plan,
            &borrowed,
            &visits,
            &plan.scalar_aggregates(),
            &owners,
        )
        .unwrap();
        (plan, owners, closed)
    }
    fn lookup(body: &str, expression: &str) -> String {
        format!(
            "{COUNT}\nlookup(s<string>, query<string>) => (result<u64>) :=\n  counts := #Accumulate(s, 1u64, 6u64, empty)\n{body}\n{expression}\n"
        )
    }

    #[test]
    fn frozen_map_proof_follows_complete_aliases_and_arbitrary_query_branches() {
        let source = lookup(
            "  alias := counts\n  selected := (query ∈ alias)? | true => alias{query} | false => 0u64.\n  result := selected.",
            "lookup(text<string>, query<string>)",
        );
        let original = artifact(&source);
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
        )
        .unwrap();
        let mut proofs = Vec::new();
        for artifact in [original, decoded] {
            let (plan, owners, closed) = parts(&artifact);
            let layout = Layout::derive(&plan, &owners, &closed).unwrap();
            assert_eq!(layout.regions.len(), 1);
            let region = &layout.regions[0];
            assert_eq!(region.parent, None);
            assert!(region.aliases.contains(&region.output));
            assert_eq!(layout.reads.iter().flatten().count(), 2);
            // The source alias is eliminated before this plan; both reads
            // reference the same completed result rather than a host copy.
            assert_eq!(region.aliases, vec![region.output]);
            for (site, selected) in layout.reads.iter().enumerate() {
                if selected.is_none() {
                    continue;
                }
                let Action::LibraryCall { operands, .. } = plan.blocks()[site].action() else {
                    panic!("ordinary read only");
                };
                assert!(region.aliases.contains(&operands[0].get()));
                assert_eq!(plan.blocks()[site].iteration(), region.parent);
            }
            let wire = serde_json::to_vec(&layout).unwrap();
            let restored: Layout = serde_json::from_slice(&wire).unwrap();
            assert_eq!(
                restored,
                Layout::derive(&plan, &owners, &closed).unwrap(),
                "wire data alone is not proof: rederive from the same checked plan"
            );
            proofs.push(layout);
        }
        assert_eq!(proofs[0], proofs[1]);
    }

    #[test]
    fn frozen_map_proof_keeps_each_worker_visit_local() {
        let source = lookup(
            "  selected := (query ∈ counts)? | true => counts{query} | false => 0u64.\n  result := selected.",
            "[lookup(text<string>, q) | q <- queries<[string]>]",
        );
        let (plan, owners, closed) = parts(&artifact(&source));
        let layout = Layout::derive(&plan, &owners, &closed).unwrap();
        assert_eq!(layout.regions.len(), 1);
        let region = &layout.regions[0];
        assert!(region.parent.is_some());
        let parent = plan.iterations()[region.parent.unwrap() as usize].as_ref();
        let after = reachable(&plan, region.parent, region.exit, None);
        assert!(after[parent.latch() as usize]);
        assert!(
            !after[parent.header() as usize],
            "retirement prevents a prior visit reaching the next header"
        );
        assert!(!after[parent.body() as usize]);
        for value in &region.aliases {
            assert_eq!(
                plan.value_scopes()[*value as usize],
                Scope::Immutable {
                    iteration: region.parent
                }
            );
        }
    }

    #[test]
    fn frozen_map_proof_rejects_any_unsupported_use_before_selecting_storage() {
        let cases = [
            // A whole-map output preserves canonical publication and ordering.
            format!("{COUNT}\n#Accumulate(text<string>, 1u64, 6u64, empty)\n"),
            lookup(
                "  changed := map/insert(counts, query, 7u64)\n  result := changed{query}.",
                "lookup(text<string>, query<string>)",
            ),
            lookup(
                "  keys := map/keys(counts)\n  result := sequence/length(keys) + counts{query}.",
                "lookup(text<string>, query<string>)",
            ),
            lookup(
                "  chosen := (query == \"a\")? | true => counts | false => empty.\n  result := chosen{query}.",
                "lookup(text<string>, query<string>)",
            ),
            // A descendant comprehension captures this map: its lifetime and
            // current visit need a separate proof, even though every use reads.
            format!(
                "{COUNT}\nlookup(s<string>, queries<[string]>) => (result<u64>) :=\n  counts := #Accumulate(s, 1u64, 6u64, empty)\n  values := [counts{{q}} | q <- queries]\n  result := sequence/length(values).\nlookup(text<string>, queries<[string]>)\n"
            ),
        ];
        for source in cases {
            let (plan, owners, closed) = parts(&artifact(&source));
            assert!(
                closed.regions.iter().any(|region| region.packed.is_some()),
                "the negative fixture must reach the prior packed-owner proof"
            );
            assert!(
                Layout::derive(&plan, &owners, &closed)
                    .unwrap()
                    .regions
                    .is_empty(),
                "{source}"
            );
        }
    }

    #[test]
    fn frozen_map_proof_rejects_hidden_escape_and_ambiguous_definition_inventory() {
        let source = lookup(
            "  result := counts{query}.",
            "lookup(text<string>, query<string>)",
        );
        let (plan, owners, closed) = parts(&artifact(&source));
        let selected = Layout::derive(&plan, &owners, &closed).unwrap();
        assert_eq!(selected.regions.len(), 1);
        let region = &selected.regions[0];
        let mut inventory = Inventory::derive(&plan).unwrap();
        assert!(candidate(&plan, &owners, &closed, region.closed, &inventory).is_some());
        inventory.uses[region.output as usize].push(Use::Other);
        assert!(candidate(&plan, &owners, &closed, region.closed, &inventory).is_none());
        inventory.uses[region.output as usize].pop();
        inventory.definitions[region.output as usize].push(usize::MAX);
        assert!(candidate(&plan, &owners, &closed, region.closed, &inventory).is_none());
    }
}
