//! Same-plan descriptive facts. This API grants no validation or execution
//! authority; each consumer keeps its current-visit policy and eligibility.
use super::{BAD, Read, SemanticInputs, visit_role_reads};
use crate::native_artifact::{
    NativeCollectionResult as ResultKind, NativeCollectionSource as Source,
    NativeControlLoop as Loop, NativeIterationAction as Action,
    NativeIterationTerminator as Terminator, NativeMatchAction as Scalar,
    NativeMatchTerminator as Term, NativeMatchValue as Value, NativeProgramArtifactPlan as Plan,
    NativeStructuralObservation as Observation,
};
use std::{borrow::Cow, cell::OnceCell};

#[path = "predicate_facts.rs"]
mod predicates;

#[cfg(feature = "sequence_buffers")]
#[path = "old_value_reads.rs"]
mod old_value_reads;

#[path = "uniform_word_collection.rs"]
mod uniform_words;
pub use uniform_words::NativeUniformWordCollection;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeProgramReadRole {
    Action(u32),
    Terminator(u32),
    CollectionSource(u32),
    LoopSeed(u32),
    LoopNext(u32),
    LoopReturned(u32),
    Output(usize),
    Effect(usize),
}
impl NativeProgramReadRole {
    pub(super) fn fusion_read(self) -> Read {
        match self {
            Self::Action(site) => Read::Action(site),
            Self::Terminator(site) => Read::Terminator(site),
            _ => Read::Other,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeProgramProducerRole {
    Input,
    Constant,
    Scalar,
    Library,
    Structural,
    MaterializedCollection,
    TextJoin,
    VirtualConsumer,
    CollectionBuilder,
    CollectionElement,
    InitializeCarrier,
    LatchCarrier,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeProgramDefinition {
    site: Option<u32>,
    iteration: Option<u32>,
    role: NativeProgramProducerRole,
}
impl NativeProgramDefinition {
    pub fn site(self) -> Option<u32> {
        self.site
    }
    pub fn iteration(self) -> Option<u32> {
        self.iteration
    }
    pub fn role(self) -> NativeProgramProducerRole {
        self.role
    }
}
struct Graph {
    successors: Vec<[Option<u32>; 2]>,
    predecessors: Vec<Vec<u32>>,
    reachable: Vec<bool>,
}
/// Fresh scratch for one immutable plan borrow. No Deserialize, mutable plan
/// getter, persistent validated flag, or cross-rewrite cache is exposed.
pub struct NativeProgramFacts<'p> {
    inputs: SemanticInputs<'p, Plan>,
    graph: OnceCell<Result<Graph, &'static str>>,
    reads: OnceCell<Result<Vec<Vec<NativeProgramReadRole>>, &'static str>>,
    definitions: OnceCell<Result<Vec<Vec<NativeProgramDefinition>>, &'static str>>,
    #[cfg(any(
        feature = "pure_text",
        feature = "map_ops",
        feature = "sequence_buffers"
    ))]
    fusion_reads: OnceCell<Result<Vec<Vec<Read>>, &'static str>>,
    #[cfg(any(
        feature = "pure_text",
        feature = "map_ops",
        feature = "sequence_buffers"
    ))]
    aggregates: OnceCell<crate::native_artifact::NativeScalarAggregates>,
    #[cfg(feature = "pure_text")]
    retention: OnceCell<Vec<crate::native_artifact::NativeTextLineRetentionFrontier>>,
}
impl Plan {
    pub fn facts(&self) -> NativeProgramFacts<'_> {
        NativeProgramFacts {
            inputs: SemanticInputs::new(self),
            graph: OnceCell::new(),
            reads: OnceCell::new(),
            definitions: OnceCell::new(),
            #[cfg(any(
                feature = "pure_text",
                feature = "map_ops",
                feature = "sequence_buffers"
            ))]
            fusion_reads: OnceCell::new(),
            #[cfg(any(
                feature = "pure_text",
                feature = "map_ops",
                feature = "sequence_buffers"
            ))]
            aggregates: OnceCell::new(),
            #[cfg(feature = "pure_text")]
            retention: OnceCell::new(),
        }
    }
}
impl<'p> NativeProgramFacts<'p> {
    #[cfg(any(
        feature = "pure_text",
        feature = "map_ops",
        feature = "sequence_buffers"
    ))]
    pub(in crate::native_artifact) fn actions(
        &self,
    ) -> Result<&super::SemanticActions<'p>, &'static str> {
        self.inputs.actions()
    }
    #[cfg(any(
        feature = "pure_text",
        feature = "map_ops",
        feature = "sequence_buffers"
    ))]
    pub(in crate::native_artifact) fn fusion_reads(&self) -> Result<&[Vec<Read>], &'static str> {
        self.fusion_reads
            .get_or_init(|| super::reads(self.plan(), self.actions()?))
            .as_ref()
            .map(Vec::as_slice)
            .map_err(|e| *e)
    }
    #[cfg(any(
        feature = "pure_text",
        feature = "map_ops",
        feature = "sequence_buffers"
    ))]
    pub fn scalar_aggregates(&self) -> &crate::native_artifact::NativeScalarAggregates {
        self.aggregates
            .get_or_init(|| crate::native_artifact::scalar_aggregates::derive(self.plan(), self))
    }
    #[cfg(any(
        feature = "pure_text",
        feature = "map_ops",
        feature = "sequence_buffers"
    ))]
    pub fn into_scalar_aggregates(self) -> crate::native_artifact::NativeScalarAggregates {
        self.scalar_aggregates();
        self.aggregates
            .into_inner()
            .expect("same-plan aggregate inventory initialized")
    }
    #[cfg(feature = "map_ops")]
    pub fn map_owner_words(
        &self,
    ) -> Result<crate::native_artifact::NativeMapOwnerWords, &'static str> {
        crate::native_artifact::map_owner_words::derive(self.plan(), self)
    }
    #[cfg(feature = "pure_text")]
    pub fn text_line_read_frontiers(
        &self,
    ) -> Vec<crate::native_artifact::NativeTextLineReadFrontier> {
        crate::native_artifact::text_line_frontier::availability::derive(self.plan(), self)
    }
    #[cfg(feature = "pure_text")]
    pub fn text_line_retention_frontiers(
        &self,
    ) -> &[crate::native_artifact::NativeTextLineRetentionFrontier] {
        self.retention.get_or_init(|| {
            crate::native_artifact::text_line_frontier::retention::derive(self.plan(), self)
        })
    }
    #[cfg(feature = "pure_text")]
    pub fn into_retention_frontiers(
        self,
    ) -> Vec<crate::native_artifact::NativeTextLineRetentionFrontier> {
        self.text_line_retention_frontiers();
        self.retention
            .into_inner()
            .expect("same-plan retention inventory initialized")
    }
    /// Move the already-derived inventories into their single host owner after
    /// layout composition. No plan, proof or boxed metadata is cloned.
    #[cfg(feature = "pure_text")]
    pub fn into_preparation_inventories(
        self,
    ) -> (
        crate::native_artifact::NativeScalarAggregates,
        Vec<crate::native_artifact::NativeTextLineRetentionFrontier>,
    ) {
        self.scalar_aggregates();
        self.text_line_retention_frontiers();
        (
            self.aggregates
                .into_inner()
                .expect("same-plan aggregate inventory initialized"),
            self.retention
                .into_inner()
                .expect("same-plan retention inventory initialized"),
        )
    }

    /// Descriptive control edges only, shared with runtime boundary checks.
    pub fn control_successors(terminator: Terminator) -> [Option<u32>; 2] {
        successors(terminator)
    }

    pub fn plan(&self) -> &'p Plan {
        self.inputs.plan
    }
    pub fn semantic_use_counts(&self) -> Result<&[usize], &'static str> {
        self.inputs.counts()
    }
    pub fn read_roles(&self, value: Value) -> Result<&[NativeProgramReadRole], &'static str> {
        let reads = self
            .reads
            .get_or_init(|| {
                let mut reads = vec![Vec::new(); self.plan().values().len()];
                visit_role_reads(
                    self.plan(),
                    self.inputs.actions()?.iter().map(|a| Ok(Cow::Borrowed(a))),
                    |value, role| {
                        reads.get_mut(value.get() as usize).ok_or(BAD)?.push(role);
                        Ok(())
                    },
                )?;
                Ok(reads)
            })
            .as_ref()
            .map_err(|e| *e)?;
        reads
            .get(value.get() as usize)
            .map(Vec::as_slice)
            .ok_or(BAD)
    }
    pub fn successors(&self, site: u32) -> Result<[Option<u32>; 2], &'static str> {
        Ok(successors(
            self.plan()
                .blocks()
                .get(site as usize)
                .ok_or(BAD)?
                .terminator(),
        ))
    }
    fn graph(&self) -> Result<&Graph, &'static str> {
        self.graph
            .get_or_init(|| {
                let plan = self.plan();
                let successors = plan
                    .blocks()
                    .iter()
                    .map(|b| successors(b.terminator()))
                    .collect::<Vec<_>>();
                let mut predecessors = vec![Vec::new(); successors.len()];
                for (site, next) in successors.iter().enumerate() {
                    for &next in next.iter().flatten() {
                        predecessors
                            .get_mut(next as usize)
                            .ok_or(BAD)?
                            .push(site as u32);
                    }
                }
                let mut reachable = vec![false; successors.len()];
                let mut pending = vec![plan.entry()];
                while let Some(site) = pending.pop() {
                    let seen = reachable.get_mut(site as usize).ok_or(BAD)?;
                    if core::mem::replace(seen, true) {
                        continue;
                    }
                    pending.extend(successors[site as usize].into_iter().flatten());
                }
                Ok(Graph {
                    successors,
                    predecessors,
                    reachable,
                })
            })
            .as_ref()
            .map_err(|e| *e)
    }
    pub fn predecessors(&self, site: u32) -> Result<&[u32], &'static str> {
        self.graph()?
            .predecessors
            .get(site as usize)
            .map(Vec::as_slice)
            .ok_or(BAD)
    }
    pub fn is_reachable(&self, site: u32) -> Result<bool, &'static str> {
        self.graph()?
            .reachable
            .get(site as usize)
            .copied()
            .ok_or(BAD)
    }
    /// Whole-graph strict dominance describes immutable root availability only;
    /// it never establishes current-loop-visit validity or permission to skip a guard.
    pub fn strictly_dominates(&self, producer: u32, frontier: u32) -> Result<bool, &'static str> {
        let graph = self.graph()?;
        if producer == frontier || !self.is_reachable(producer)? || !self.is_reachable(frontier)? {
            return Ok(false);
        }
        let mut seen = vec![false; graph.successors.len()];
        let mut pending = vec![self.plan().entry()];
        while let Some(site) = pending.pop() {
            if site == producer || core::mem::replace(&mut seen[site as usize], true) {
                continue;
            }
            if site == frontier {
                return Ok(false);
            }
            pending.extend(graph.successors[site as usize].into_iter().flatten());
        }
        Ok(true)
    }
    pub fn lexical_ancestor(&self, ancestor: Option<u32>, scope: Option<u32>) -> bool {
        super::lexical_ancestor(self.plan(), ancestor, scope)
    }
    pub fn definition_sites(&self, value: u32) -> Result<&[NativeProgramDefinition], &'static str> {
        self.definitions
            .get_or_init(|| definitions(self.plan()))
            .as_ref()
            .map_err(|e| *e)?
            .get(value as usize)
            .map(Vec::as_slice)
            .ok_or(BAD)
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
        _ => [None, None],
    }
}
fn definitions(plan: &Plan) -> Result<Vec<Vec<NativeProgramDefinition>>, &'static str> {
    use NativeProgramProducerRole as Role;
    let mut out = vec![Vec::new(); plan.values().len()];
    let mut add = |value: Value, site, iteration, role| -> Result<(), &'static str> {
        out.get_mut(value.get() as usize)
            .ok_or(BAD)?
            .push(NativeProgramDefinition {
                site,
                iteration,
                role,
            });
        Ok(())
    };
    for index in 0..plan.transport_input_count() {
        add(plan.transport_input_value(index), None, None, Role::Input)?;
    }
    for constant in plan.constants() {
        add(constant.value(), None, None, Role::Constant)?;
    }
    for (site, block) in plan.blocks().iter().enumerate() {
        let site = Some(site as u32);
        let scope = block.iteration();
        match block.action() {
            Action::Scalar(Scalar::Primitive { output, .. } | Scalar::Copy { output, .. }) => {
                add(*output, site, scope, Role::Scalar)?
            }
            Action::LibraryCall { output, .. } => add(*output, site, scope, Role::Library)?,
            Action::Structural { output, .. } => add(*output, site, scope, Role::Structural)?,
            Action::CollectionBegin { collection } => add(
                plan.collections()[*collection as usize].output(),
                site,
                scope,
                Role::CollectionBuilder,
            )?,
            Action::CollectionFinish { collection }
                if matches!(
                    plan.collections()[*collection as usize].result(),
                    ResultKind::Materialized
                ) =>
            {
                add(
                    plan.collections()[*collection as usize].output(),
                    site,
                    scope,
                    Role::MaterializedCollection,
                )?
            }
            Action::TextJoinFinish { collection } => {
                if let ResultKind::TextJoin(sink) =
                    plan.collections()[*collection as usize].result()
                {
                    add(sink.output(), site, scope, Role::TextJoin)?;
                }
            }
            Action::Initialize { iteration } | Action::Latch { iteration } => {
                let role = if matches!(block.action(), Action::Initialize { .. }) {
                    Role::InitializeCarrier
                } else {
                    Role::LatchCarrier
                };
                match &plan.iterations()[*iteration as usize] {
                    Loop::Iteration(g) => add(g.carrier(), site, scope, role)?,
                    Loop::Tail(g) => add(g.carrier(), site, scope, role)?,
                    Loop::Collection(_) => {}
                }
            }
            Action::CollectionElement { iteration } => {
                if let Loop::Collection(g) = &plan.iterations()[*iteration as usize] {
                    if let Some(element) = g.element() {
                        add(element, site, scope, Role::CollectionElement)?;
                    }
                }
            }
            action => {
                // Virtual consumers retain their original producer class; they
                // must not become eligible as ordinary Library captures.
                if let Action::LibraryCall { output, .. }
                | Action::Scalar(Scalar::Copy { output, .. }) =
                    super::semantic_action(plan, action)?.as_ref()
                {
                    add(*output, site, scope, Role::VirtualConsumer)?;
                }
            }
        }
    }
    Ok(out)
}

fn parallel_reads(
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
    template: &crate::native_artifact::NativeProgramLibraryTemplate,
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

impl NativeProgramFacts<'_> {
    /// The parallel policy intentionally includes seed/next/returned reads at
    /// both transitions and checks template purity in its historical order.
    /// This is distinct from fusion use counts and grants no worker authority.
    pub fn parallel_action_reads(
        &self,
        site: u32,
        read: &mut impl FnMut(Value) -> Result<(), &'static str>,
    ) -> Result<(), &'static str> {
        parallel_reads(
            self.plan(),
            self.plan().blocks().get(site as usize).ok_or(BAD)?.action(),
            read,
        )
    }
}
