//! Constructor-private outlining of independent source comprehension bodies.
//! A region authorizes one original element-to-push CFG, never a host algorithm.
use mech_engine::native_artifact::{
    NativeCollectionResult as ResultKind, NativeCollectionSource as Source,
    NativeControlLoop as Loop, NativeIterationAction as Action,
    NativeIterationTerminator as Terminator, NativeIterationValueScope as Scope,
    NativeMatchAction as Scalar, NativeMatchTerminator as Term,
    NativeMatchValue as Value, NativeProgramArtifactPlan as Plan,
    NativeStructuralObservation as Observation,
};
use std::collections::BTreeSet;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Region {
    pub collection: u32,
    pub iteration: u32,
    pub begin: u32,
    pub initialize: u32,
    pub header: u32,
    pub body: u32,
    pub push: u32,
    pub output: u32,
    pub captures: Box<[u32]>,
    pub sites: Box<[u32]>,
}
impl Region {
    pub fn contains(&self, site: u32) -> bool { self.sites.binary_search(&site).is_ok() }
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Layout { pub regions: Box<[Region]> }

pub(super) fn successors(term: Terminator) -> [Option<u32>; 2] {
    match term {
        Terminator::Scalar(Term::Jump(next)) => [Some(next), None],
        Terminator::Scalar(Term::Branch { yes, no, .. } | Term::LiteralBranch { yes, no, .. }) => [Some(yes), Some(no)],
        Terminator::Header { body, exit, .. } => [Some(body), Some(exit)],
        Terminator::TailHeader { body, .. } => [Some(body), None],
        _ => [None, None],
    }
}

impl Layout {
    pub fn derive(plan: &Plan) -> Result<Self, &'static str> {
        plan.validate_current_visits()?;
        let aggregates = plan.scalar_aggregates();
        let mut regions = Vec::new();
        for (id, collection) in plan.collections().iter().enumerate() {
            // The first version parks one root builder and serializes nested
            // collections. Selected histogram/span storage has a different
            // yield protocol and remains on its ordinary path.
            if collection.parent().is_some() || collection.parent_collection().is_some()
                || collection.generators().len() != 1
                || collection.string_span_projection().is_some()
                || collection.string_element_map_projection().is_some()
                || !matches!(collection.result(), ResultKind::Materialized | ResultKind::TextJoin(_))
            { continue; }
            let iteration = collection.generators()[0];
            let Loop::Collection(generator) = &plan.iterations()[iteration as usize] else { continue; };
            if generator.parent().is_some() || !matches!(generator.source(), Source::Value(_)) { continue; }
            if !matches!(plan.values()[generator.source().semantic_value().get() as usize].definition().body(),
                mech_core::SchemaBody::Matrix { .. } | mech_core::SchemaBody::Set { .. }) { continue; }
            // The private entry replays exactly this original builder/source
            // prelude. Any extra prerequisite remains on the ordinary path.
            if plan.blocks()[collection.begin() as usize].terminator()
                    != Terminator::Scalar(Term::Jump(generator.initialize()))
                || plan.blocks()[generator.initialize() as usize].terminator()
                    != Terminator::Scalar(Term::Jump(generator.header()))
                || plan.blocks()[generator.header() as usize].terminator()
                    != (Terminator::Header { iteration, body: generator.body(), exit: generator.exit() }) {
                continue;
            }
            if !matches!(plan.blocks()[generator.body() as usize].action(), Action::CollectionElement { iteration: owner } if *owner == iteration) { continue; }
            let Action::CollectionPush { value: output, .. } = plan.blocks()[collection.push() as usize].action() else { continue; };
            let mut sites = BTreeSet::new();
            let mut pending = vec![generator.body()];
            let mut valid = true;
            while let Some(site) = pending.pop() {
                if site == collection.push() || !sites.insert(site) { continue; }
                if site == generator.latch() || site == generator.exit()
                    || site == collection.finish() || site == generator.header()
                    || plan.activation_boundary() == Some(site)
                { valid = false; break; }
                let block = &plan.blocks()[site as usize];
                if matches!(block.terminator(), Terminator::Scalar(Term::Complete)) {
                    valid = false; break;
                }
                pending.extend(successors(block.terminator()).into_iter().flatten());
            }
            if !valid { continue; }
            // Avoid creating native threads for a few scalar/string leaves.
            // A source loop in the body exposes potentially substantial work;
            // this is a cost heuristic, not a language/workload-name test.
            if !sites.iter().any(|&site| matches!(plan.blocks()[site as usize].action(),
                Action::Initialize { iteration } if matches!(plan.iterations()[*iteration as usize], Loop::Tail(_) | Loop::Iteration(_)))) {
                continue;
            }
            sites.insert(collection.push());
            sites.insert(collection.begin());
            sites.insert(generator.initialize());
            sites.insert(generator.header());
            // Only immutable root captures may cross the worker boundary.
            // Current-visit validation proves their definitions dominate every
            // read. Worker-local carrier/immutable slots must belong to this
            // exact outer generator or one of its descendants.
            let mut captures = BTreeSet::new();
            let mut capture = |value: Value| -> Result<(), &'static str> {
                let scope = *plan.value_scopes().get(value.get() as usize).ok_or("parallel capture outside values")?;
                match scope {
                    Scope::Immutable { iteration: None } => { captures.insert(value.get()); }
                    Scope::Immutable { iteration: Some(mut owner) } | Scope::Carrier { iteration: mut owner } => {
                        while owner != iteration {
                            owner = plan.iterations().get(owner as usize).ok_or("parallel capture scope")?
                                .as_ref().parent().ok_or("parallel capture crosses sibling invocation")?;
                        }
                    }
                }
                Ok(())
            };
            if capture(generator.source().semantic_value()).is_err() { continue; }
            if let ResultKind::TextJoin(sink) = collection.result() {
                if capture(sink.separator()).is_err() { continue; }
            }
            for &site in &sites {
                let block = &plan.blocks()[site as usize];
                if reads(plan, block.action(), &mut capture).is_err() { valid = false; break; }
                let result = match block.terminator() {
                    Terminator::Scalar(Term::Branch { condition, .. }) => capture(condition),
                    Terminator::Scalar(Term::LiteralBranch { left, right, .. }) => capture(left).and_then(|_| capture(right)),
                    _ => Ok(()),
                };
                if result.is_err() { valid = false; break; }
            }
            drop(capture);
            // Private aggregate words are not canonical Values. A later
            // extension can transport their exact selected representation.
            if !valid || captures.iter().any(|&slot| aggregates.slot_at(slot as usize).is_some()) { continue; }
            regions.push(Region { collection: id as u32, iteration, begin: collection.begin(),
                initialize: generator.initialize(), header: generator.header(), body: generator.body(),
                push: collection.push(), output: output.get(), captures: captures.into_iter().collect(),
                sites: sites.into_iter().collect() });
        }
        Ok(Self { regions: regions.into() })
    }
}

fn reads(plan: &Plan, action: &Action, read: &mut impl FnMut(Value) -> Result<(), &'static str>) -> Result<(), &'static str> {
    match action {
        Action::LibraryCall { template, operands, .. } => {
            pure(template)?;
            for &value in operands { read(value)?; }
        }
        Action::Scalar(Scalar::Primitive { operands, .. }) => {
            for &value in operands { read(value)?; }
        }
        Action::Scalar(Scalar::Copy { source, .. }) | Action::CollectionPush { value: source, .. } => read(*source)?,
        Action::Structural { source, observation, .. } => {
            read(*source)?;
            if let Observation::StringEqual { peer } = observation { read(*peer)?; }
        }
        Action::Initialize { iteration } | Action::Latch { iteration } => {
            match &plan.iterations()[*iteration as usize] {
                Loop::Collection(generator) => match generator.source() {
                    Source::Value(value) => read(*value)?,
                    Source::U64Range(cursor) => {
                        pure(cursor.template())?;
                        for &value in cursor.operands() { read(value)?; }
                    }
                },
                Loop::Iteration(loop_) => { read(loop_.seed())?; read(loop_.next())?; }
                Loop::Tail(loop_) => { read(loop_.seed())?; read(loop_.next())?; read(loop_.returned())?; }
            }
        }
        Action::PrepareU64Range { iteration } => {
            let Loop::Collection(generator) = &plan.iterations()[*iteration as usize] else { return Err("parallel range owner"); };
            let Source::U64Range(cursor) = generator.source() else { return Err("parallel range source"); };
            pure(cursor.template())?;
            for &value in cursor.operands() { read(value)?; }
        }
        Action::CollectionBegin { collection } | Action::TextJoinFinish { collection } => {
            if let ResultKind::TextJoin(sink) = plan.collections()[*collection as usize].result() {
                pure(sink.template())?; read(sink.separator())?;
            }
        }
        Action::PrepareStringSpanSelector { collection } | Action::ProjectStringSpan { collection } => {
            let projection = plan.collections()[*collection as usize].string_span_projection().ok_or("parallel span proof")?;
            pure(projection.range_template())?; pure(projection.selection_template())?;
            read(projection.source())?;
            for &value in projection.operands() { read(value)?; }
        }
        Action::ProjectStringMapEntry { collection } => {
            let projection = plan.collections()[*collection as usize].string_element_map_projection().ok_or("parallel map proof")?;
            pure(projection.selection_template())?;
            read(projection.map())?; read(projection.element())?;
        }
        Action::HistogramQueryStep { collection, step } => {
            let ResultKind::GuardedHistogramQuery(sink) = plan.collections()[*collection as usize].result() else { return Err("parallel query proof"); };
            if let Some(template) = sink.steps()[*step as usize].template() { pure(template)?; }
            for &value in sink.steps()[*step as usize].operands() { read(value)?; }
        }
        Action::HistogramFinish { collection } => {
            let collection = &plan.collections()[*collection as usize];
            let ResultKind::Histogram(sink) = collection.result() else { return Err("parallel histogram proof"); };
            pure(sink.template())?;
            read(collection.output())?;
        }
        Action::Scalar(Scalar::None) | Action::CollectionElement { .. } | Action::CollectionFinish { .. } => {}
    }
    Ok(())
}

fn pure(template: &mech_engine::native_artifact::NativeProgramLibraryTemplate) -> Result<(), &'static str> {
    match template.contract() {
        mech_core::ResolvedOperationContract::Declared(contract)
            if contract.interaction == mech_core::ExternalInteraction::Pure => Ok(()),
        _ => Err("parallel body contains an external interaction"),
    }
}

// These are compiler-enforced requirements, not unsafe assertions. All mutable
// Rc-backed execution/cache state is constructed on its owning worker thread.
#[allow(dead_code)]
fn immutable_transport_is_thread_safe() {
    fn check<T: Send + Sync>() {}
    check::<mech_core::Value>();
    check::<super::NativeProgramDescriptor>();
    check::<super::NativeProgramExecutionError>();
}
