//! Constructor-private outlining of independent source comprehension bodies.
//! A region authorizes one original element-to-push CFG, never a host algorithm.
use mech_engine::native_artifact::{
    NativeCollectionResult as ResultKind, NativeCollectionSource as Source,
    NativeControlLoop as Loop, NativeIterationAction as Action,
    NativeIterationTerminator as Terminator, NativeIterationValueScope as Scope,
    NativeMatchAction as Scalar, NativeMatchTerminator as Term, NativeMatchValue as Value,
    NativeProgramArtifactPlan as Plan,
};
use std::collections::BTreeSet;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Region {
    pub collection: u32,
    pub iteration: u32,
    pub begin: u32,
    pub initialize: u32,
    pub header: u32,
    pub body: u32,
    pub push: u32,
    pub output: u32,
    pub source: u32,
    pub captures: Box<[u32]>,
    pub sites: Box<[u32]>,
    pub group: Box<[u32]>,
}
impl Region {
    pub fn contains(&self, site: u32) -> bool {
        self.sites.binary_search(&site).is_ok()
    }
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Layout {
    pub regions: Box<[Region]>,
}

impl Layout {
    pub fn derive(
        facts: &mech_engine::native_artifact::NativeProgramFacts<'_>,
        aggregates: &mech_engine::native_artifact::NativeScalarAggregates,
    ) -> Result<Self, &'static str> {
        let plan = facts.plan();
        plan.validate_current_visits()?;
        facts.is_reachable(plan.entry())?;
        let mut regions = Vec::new();
        for (id, collection) in plan.collections().iter().enumerate() {
            // The first version parks one root builder and serializes nested
            // collections. Selected histogram/span storage has a different
            // yield protocol and remains on its ordinary path.
            if !facts.is_reachable(collection.begin())?
                || collection.parent().is_some()
                || collection.parent_collection().is_some()
                || collection.generators().len() != 1
                || collection.string_span_projection().is_some()
                || collection.string_element_map_projection().is_some()
                || !matches!(
                    collection.result(),
                    ResultKind::Materialized | ResultKind::TextJoin(_)
                )
            {
                continue;
            }
            let iteration = collection.generators()[0];
            let Loop::Collection(generator) = &plan.iterations()[iteration as usize] else {
                continue;
            };
            if generator.parent().is_some() || !matches!(generator.source(), Source::Value(_)) {
                continue;
            }
            if !matches!(
                plan.values()[generator.source().semantic_value().get() as usize]
                    .definition()
                    .body(),
                mech_core::SchemaBody::Matrix { .. } | mech_core::SchemaBody::Set { .. }
            ) {
                continue;
            }
            // The private entry replays exactly this original builder/source
            // prelude, including lexical no-op blocks emitted by collection
            // lowering. Additional computations remain on the ordinary path.
            let Some(prelude) = empty_prelude(plan, collection.begin(), generator.initialize())
            else {
                continue;
            };
            if plan.blocks()[generator.initialize() as usize].terminator()
                != Terminator::Scalar(Term::Jump(generator.header()))
                || plan.blocks()[generator.header() as usize].terminator()
                    != (Terminator::Header {
                        iteration,
                        body: generator.body(),
                        exit: generator.exit(),
                    })
            {
                continue;
            }
            if !matches!(plan.blocks()[generator.body() as usize].action(), Action::CollectionElement { iteration: owner } if *owner == iteration)
            {
                continue;
            }
            let Action::CollectionPush { value: output, .. } =
                plan.blocks()[collection.push() as usize].action()
            else {
                continue;
            };
            let mut sites = BTreeSet::new();
            let mut pending = vec![generator.body()];
            let mut valid = true;
            while let Some(site) = pending.pop() {
                if site == collection.push() || !sites.insert(site) {
                    continue;
                }
                if site == generator.latch()
                    || site == generator.exit()
                    || site == collection.finish()
                    || site == generator.header()
                    || plan.activation_boundary() == Some(site)
                {
                    valid = false;
                    break;
                }
                let block = &plan.blocks()[site as usize];
                if matches!(block.terminator(), Terminator::Scalar(Term::Complete)) {
                    valid = false;
                    break;
                }
                pending.extend(facts.successors(site)?.into_iter().flatten());
            }
            if !valid {
                continue;
            }
            // Avoid creating native threads for a few scalar/string leaves.
            // A source loop in the body exposes potentially substantial work;
            // this is a cost heuristic, not a language/workload-name test.
            if !sites.iter().any(|&site| matches!(plan.blocks()[site as usize].action(),
                Action::Initialize { iteration } if matches!(plan.iterations()[*iteration as usize], Loop::Tail(_) | Loop::Iteration(_)))) {
                continue;
            }
            sites.insert(collection.push());
            sites.insert(collection.begin());
            sites.extend(prelude);
            sites.insert(generator.initialize());
            sites.insert(generator.header());
            // Only immutable root captures may cross the worker boundary.
            // Current-visit validation proves their definitions dominate every
            // read. Worker-local carrier/immutable slots must belong to this
            // exact outer generator or one of its descendants.
            let mut captures = BTreeSet::new();
            let mut capture = |value: Value| -> Result<(), &'static str> {
                let scope = *plan
                    .value_scopes()
                    .get(value.get() as usize)
                    .ok_or("parallel capture outside values")?;
                match scope {
                    Scope::Immutable { iteration: None } => {
                        captures.insert(value.get());
                    }
                    Scope::Immutable {
                        iteration: Some(mut owner),
                    }
                    | Scope::Carrier {
                        iteration: mut owner,
                    } => {
                        while owner != iteration {
                            owner = plan
                                .iterations()
                                .get(owner as usize)
                                .ok_or("parallel capture scope")?
                                .as_ref()
                                .parent()
                                .ok_or("parallel capture crosses sibling invocation")?;
                        }
                    }
                }
                Ok(())
            };
            if capture(generator.source().semantic_value()).is_err() {
                continue;
            }
            if let ResultKind::TextJoin(sink) = collection.result() {
                if capture(sink.separator()).is_err() {
                    continue;
                }
            }
            for &site in &sites {
                let block = &plan.blocks()[site as usize];
                if let Action::CollectionBegin { collection } = block.action() {
                    let child = &plan.collections()[*collection as usize];
                    if !matches!(
                        child.result(),
                        ResultKind::Materialized | ResultKind::TextJoin(_)
                    ) || child.string_span_projection().is_some()
                        || child.string_element_map_projection().is_some()
                    {
                        valid = false;
                        break;
                    }
                }
                if facts.parallel_action_reads(site, &mut capture).is_err() {
                    valid = false;
                    break;
                }
                let result = match block.terminator() {
                    Terminator::Scalar(Term::Branch { condition, .. }) => capture(condition),
                    Terminator::Scalar(Term::LiteralBranch { left, right, .. }) => {
                        capture(left).and_then(|_| capture(right))
                    }
                    _ => Ok(()),
                };
                if result.is_err() {
                    valid = false;
                    break;
                }
            }
            drop(capture);
            // Private aggregate words are not canonical Values. A later
            // extension can transport their exact selected representation.
            if !valid
                || captures
                    .iter()
                    .any(|&slot| aggregates.slot_at(slot as usize).is_some())
            {
                continue;
            }
            regions.push(Region {
                collection: id as u32,
                iteration,
                begin: collection.begin(),
                initialize: generator.initialize(),
                header: generator.header(),
                body: generator.body(),
                push: collection.push(),
                output: output.get(),
                source: generator.source().semantic_value().get(),
                captures: captures.into_iter().collect(),
                sites: sites.into_iter().collect(),
                group: Box::new([]),
            });
        }
        for leader in 0..regions.len() {
            let mut group = vec![leader as u32];
            let mut current = leader;
            while let Some(next) = next_sibling(facts, &regions, current) {
                if group.contains(&(next as u32))
                    || !regions[next]
                        .captures
                        .iter()
                        .all(|&slot| available_before(facts, slot, regions[leader].initialize))
                {
                    break;
                }
                group.push(next as u32);
                current = next;
            }
            regions[leader].group = group.into();
        }
        Ok(Self {
            regions: regions.into(),
        })
    }
}

// Canonical collection lowering emits lexical entry blocks between its builder
// and generator initializer. Replay only an acyclic root-scope chain of empty
// unconditional visits: no capture producer, selected operation or branch can
// be moved into a child constructor by this proof.
fn empty_prelude(plan: &Plan, begin: u32, initialize: u32) -> Option<Vec<u32>> {
    let Terminator::Scalar(Term::Jump(mut site)) = plan.blocks()[begin as usize].terminator()
    else {
        return None;
    };
    let mut sites = Vec::new();
    while site != initialize {
        if site == begin || sites.contains(&site) || plan.activation_boundary() == Some(site) {
            return None;
        }
        let block = &plan.blocks()[site as usize];
        if block.iteration().is_some() || !matches!(block.action(), Action::Scalar(Scalar::None)) {
            return None;
        }
        let Terminator::Scalar(Term::Jump(next)) = block.terminator() else {
            return None;
        };
        sites.push(site);
        site = next;
    }
    Some(sites)
}

// Only a guaranteed pure root path can join sibling work. The parent still
// executes every intervening source action and exposes its errors in order.
fn next_sibling(
    facts: &mech_engine::native_artifact::NativeProgramFacts<'_>,
    regions: &[Region],
    current: usize,
) -> Option<usize> {
    let plan = facts.plan();
    let finish = plan.collections()[regions[current].collection as usize].finish();
    let mut site = finish;
    let mut seen = BTreeSet::new();
    loop {
        if !seen.insert(site) || plan.activation_boundary() == Some(site) {
            return None;
        }
        if site != finish
            && let Some(next) = regions.iter().position(|region| region.begin == site)
        {
            return Some(next);
        }
        let block = &plan.blocks()[site as usize];
        if block.iteration().is_some() {
            return None;
        }
        match block.action() {
            Action::Scalar(_) | Action::Structural { .. } => {}
            Action::LibraryCall { .. } => {
                facts.parallel_action_reads(site, &mut |_| Ok(())).ok()?
            }
            Action::CollectionFinish { collection }
                if *collection == regions[current].collection => {}
            Action::TextJoinFinish { collection } => {
                let ResultKind::TextJoin(_) = plan.collections()[*collection as usize].result()
                else {
                    return None;
                };
                facts.parallel_action_reads(site, &mut |_| Ok(())).ok()?;
            }
            _ => return None,
        }
        let Terminator::Scalar(Term::Jump(next)) = block.terminator() else {
            return None;
        };
        site = next;
    }
}

// A unique root producer must dominate the leader's initialized frontier.
// Multi-producer joins conservatively decline; existing lexical validation
// already proves every accepted immutable capture has one current definition.
fn available_before(
    facts: &mech_engine::native_artifact::NativeProgramFacts<'_>,
    slot: u32,
    frontier: u32,
) -> bool {
    use mech_engine::native_artifact::NativeProgramProducerRole as Role;
    let Ok(definitions) = facts.definition_sites(slot) else {
        return false;
    };
    if definitions
        .iter()
        .any(|d| matches!(d.role(), Role::Constant | Role::Input))
    {
        return true;
    }
    let mut producers = definitions.iter().filter(|d| {
        matches!(
            d.role(),
            Role::Scalar
                | Role::Library
                | Role::Structural
                | Role::MaterializedCollection
                | Role::TextJoin
        )
    });
    let Some(producer) = producers.next() else {
        return false;
    };
    if producer.iteration().is_some() || producers.next().is_some() {
        return false;
    }
    producer
        .site()
        .is_some_and(|site| facts.strictly_dominates(site, frontier).unwrap_or(false))
}

// These are compiler-enforced requirements, not unsafe assertions. All mutable
// Rc-backed execution/cache state is constructed on its owning worker thread.
const _: fn() = || {
    fn check<T: Send + Sync>() {}
    check::<mech_core::Value>();
    check::<crate::native_artifact::NativeProgramDescriptor>();
    check::<crate::native_artifact::NativeProgramExecutionError>();
};
