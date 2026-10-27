//! Constructor-private outlining of independent source comprehension bodies.
//! A region authorizes one original element-to-push CFG, never a host algorithm.
use mech_engine::native_artifact::{
    NativeCollectionResult as ResultKind, NativeCollectionSource as Source,
    NativeControlLoop as Loop, NativeIterationAction as Action,
    NativeIterationTerminator as Terminator, NativeIterationValueScope as Scope,
    NativeMatchAction as Scalar, NativeMatchTerminator as Term, NativeMatchValue as Value,
    NativeProgramArtifactPlan as Plan, NativeStructuralObservation as Observation,
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
pub(super) struct Layout {
    pub regions: Box<[Region]>,
}

pub(super) fn successors(term: Terminator) -> [Option<u32>; 2] {
    match term {
        Terminator::Scalar(Term::Jump(next)) => [Some(next), None],
        Terminator::Scalar(Term::Branch { yes, no, .. } | Term::LiteralBranch { yes, no, .. }) => {
            [Some(yes), Some(no)]
        }
        Terminator::Header { body, exit, .. } => [Some(body), Some(exit)],
        Terminator::TailHeader { body, .. } => [Some(body), None],
        _ => [None, None],
    }
}

impl Layout {
    pub fn derive(plan: &Plan) -> Result<Self, &'static str> {
        plan.validate_current_visits()?;
        let aggregates = plan.scalar_aggregates();
        let mut reachable = BTreeSet::new();
        let mut pending = vec![plan.entry()];
        while let Some(site) = pending.pop() {
            if reachable.insert(site) {
                pending.extend(
                    successors(plan.blocks()[site as usize].terminator())
                        .into_iter()
                        .flatten(),
                );
            }
        }
        let mut regions = Vec::new();
        for (id, collection) in plan.collections().iter().enumerate() {
            // The first version parks one root builder and serializes nested
            // collections. Selected histogram/span storage has a different
            // yield protocol and remains on its ordinary path.
            if !reachable.contains(&collection.begin())
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
                pending.extend(successors(block.terminator()).into_iter().flatten());
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
                if reads(plan, block.action(), &mut capture).is_err() {
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
            while let Some(next) = next_sibling(plan, &regions, current) {
                if group.contains(&(next as u32))
                    || !regions[next]
                        .captures
                        .iter()
                        .all(|&slot| available_before(plan, slot, regions[leader].initialize))
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
fn next_sibling(plan: &Plan, regions: &[Region], current: usize) -> Option<usize> {
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
            Action::LibraryCall { template, .. } => pure(template).ok()?,
            Action::CollectionFinish { collection }
                if *collection == regions[current].collection => {}
            Action::TextJoinFinish { collection } => {
                let ResultKind::TextJoin(sink) = plan.collections()[*collection as usize].result()
                else {
                    return None;
                };
                pure(sink.template()).ok()?;
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
fn available_before(plan: &Plan, slot: u32, frontier: u32) -> bool {
    if plan
        .constants()
        .iter()
        .any(|constant| constant.value().get() == slot)
        || (0..plan.transport_input_count())
            .any(|index| plan.transport_input_value(index).get() == slot)
    {
        return true;
    }
    let mut producers = plan
        .blocks()
        .iter()
        .enumerate()
        .filter_map(|(site, block)| {
            let output = match block.action() {
                Action::Scalar(Scalar::Primitive { output, .. } | Scalar::Copy { output, .. })
                | Action::LibraryCall { output, .. }
                | Action::Structural { output, .. } => Some(*output),
                Action::CollectionFinish { collection }
                    if matches!(
                        plan.collections()[*collection as usize].result(),
                        ResultKind::Materialized
                    ) =>
                {
                    Some(plan.collections()[*collection as usize].output())
                }
                Action::TextJoinFinish { collection } => {
                    match plan.collections()[*collection as usize].result() {
                        ResultKind::TextJoin(sink) => Some(sink.output()),
                        _ => None,
                    }
                }
                _ => None,
            };
            (output.is_some_and(|output| output.get() == slot))
                .then_some((site as u32, block.iteration()))
        });
    let Some((producer, None)) = producers.next() else {
        return false;
    };
    if producers.next().is_some() || producer == frontier {
        return false;
    }
    let mut pending = vec![plan.entry()];
    let mut seen = BTreeSet::new();
    while let Some(site) = pending.pop() {
        if site == producer || !seen.insert(site) {
            continue;
        }
        if site == frontier {
            return false;
        }
        pending.extend(
            successors(plan.blocks()[site as usize].terminator())
                .into_iter()
                .flatten(),
        );
    }
    true
}

fn reads(
    plan: &Plan,
    action: &Action,
    read: &mut impl FnMut(Value) -> Result<(), &'static str>,
) -> Result<(), &'static str> {
    match action {
        Action::LibraryCall {
            template, operands, ..
        } => {
            pure(template)?;
            for &value in operands {
                read(value)?;
            }
        }
        Action::Scalar(Scalar::Primitive { operands, .. }) => {
            for &value in operands {
                read(value)?;
            }
        }
        Action::Scalar(Scalar::Copy { source, .. })
        | Action::CollectionPush { value: source, .. } => read(*source)?,
        Action::Structural {
            source,
            observation,
            ..
        } => {
            read(*source)?;
            if let Observation::StringEqual { peer } = observation {
                read(*peer)?;
            }
        }
        Action::Initialize { iteration } | Action::Latch { iteration } => {
            match &plan.iterations()[*iteration as usize] {
                Loop::Collection(generator) => match generator.source() {
                    Source::Value(value) => read(*value)?,
                    Source::U64Range(cursor) => {
                        pure(cursor.template())?;
                        for &value in cursor.operands() {
                            read(value)?;
                        }
                    }
                },
                Loop::Iteration(loop_) => {
                    read(loop_.seed())?;
                    read(loop_.next())?;
                }
                Loop::Tail(loop_) => {
                    read(loop_.seed())?;
                    read(loop_.next())?;
                    read(loop_.returned())?;
                }
            }
        }
        Action::PrepareU64Range { iteration } => {
            let Loop::Collection(generator) = &plan.iterations()[*iteration as usize] else {
                return Err("parallel range owner");
            };
            let Source::U64Range(cursor) = generator.source() else {
                return Err("parallel range source");
            };
            pure(cursor.template())?;
            for &value in cursor.operands() {
                read(value)?;
            }
        }
        Action::CollectionBegin { collection } | Action::TextJoinFinish { collection } => {
            if let ResultKind::TextJoin(sink) = plan.collections()[*collection as usize].result() {
                pure(sink.template())?;
                read(sink.separator())?;
            }
        }
        Action::PrepareStringSpanSelector { collection }
        | Action::ProjectStringSpan { collection } => {
            let projection = plan.collections()[*collection as usize]
                .string_span_projection()
                .ok_or("parallel span proof")?;
            pure(projection.range_template())?;
            pure(projection.selection_template())?;
            read(projection.source())?;
            for &value in projection.operands() {
                read(value)?;
            }
        }
        Action::ProjectStringMapEntry { collection } => {
            let projection = plan.collections()[*collection as usize]
                .string_element_map_projection()
                .ok_or("parallel map proof")?;
            pure(projection.selection_template())?;
            read(projection.map())?;
            read(projection.element())?;
        }
        Action::HistogramQueryStep { collection, step } => {
            let ResultKind::GuardedHistogramQuery(sink) =
                plan.collections()[*collection as usize].result()
            else {
                return Err("parallel query proof");
            };
            if let Some(template) = sink.steps()[*step as usize].template() {
                pure(template)?;
            }
            for &value in sink.steps()[*step as usize].operands() {
                read(value)?;
            }
        }
        Action::HistogramFinish { collection } => {
            let collection = &plan.collections()[*collection as usize];
            let ResultKind::Histogram(sink) = collection.result() else {
                return Err("parallel histogram proof");
            };
            pure(sink.template())?;
            read(collection.output())?;
        }
        Action::Scalar(Scalar::None)
        | Action::CollectionElement { .. }
        | Action::CollectionFinish { .. } => {}
    }
    Ok(())
}

fn pure(
    template: &mech_engine::native_artifact::NativeProgramLibraryTemplate,
) -> Result<(), &'static str> {
    match template.contract() {
        mech_core::ResolvedOperationContract::Declared(contract)
            if contract.interaction == mech_core::ExternalInteraction::Pure =>
        {
            Ok(())
        }
        _ => Err("parallel body contains an external interaction"),
    }
}

// These are compiler-enforced requirements, not unsafe assertions. All mutable
// Rc-backed execution/cache state is constructed on its owning worker thread.
const _: fn() = || {
    fn check<T: Send + Sync>() {}
    check::<mech_core::Value>();
    check::<super::NativeProgramDescriptor>();
    check::<super::NativeProgramExecutionError>();
};
