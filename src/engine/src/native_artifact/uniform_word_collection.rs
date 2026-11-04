//! Same-plan facts for a uniform unsigned collection, without execution authority.
use super::{NativeProgramFacts, NativeProgramProducerRole as Role, NativeProgramReadRole as Read};
use crate::native_artifact::{
    NativeCollectionResult, NativeCollectionSource, NativeControlLoop,
    NativeIterationAction as Action, NativeIterationTerminator as Terminator,
    NativeIterationValueScope, NativeMatchAction as Scalar, NativeMatchTerminator as Term,
    NativeMatchValue as Value, NativeProgramArtifactPlan as Plan, NativeScalarKind, NativeScalarOp,
};
use std::collections::BTreeSet;

/// A pure uniform unsigned range collection. It either yields once for every
/// selected range element, or retains the established `0..=limit, i < limit`
/// prefix form. All outer words are already current before source preparation.
/// This supplies no allocation, visit or range-error authority.
#[derive(Clone, Debug, serde::Serialize)]
pub struct NativeUniformWordCollection {
    collection: u32,
    iteration: u32,
    header: u32,
    word: Value,
    start: Value,
    limit: Value,
    inclusive: bool,
    filtered: bool,
    kind: NativeScalarKind,
}
impl NativeUniformWordCollection {
    pub const fn collection(&self) -> u32 {
        self.collection
    }
    pub const fn iteration(&self) -> u32 {
        self.iteration
    }
    pub const fn header(&self) -> u32 {
        self.header
    }
    pub const fn word(&self) -> Value {
        self.word
    }
    pub const fn start(&self) -> Value {
        self.start
    }
    pub const fn inclusive(&self) -> bool {
        self.inclusive
    }
    pub const fn filtered(&self) -> bool {
        self.filtered
    }
    pub const fn limit(&self) -> Value {
        self.limit
    }
    pub const fn kind(&self) -> NativeScalarKind {
        self.kind
    }
}

impl NativeProgramFacts<'_> {
    pub fn uniform_unsigned_collections(&self) -> Vec<NativeUniformWordCollection> {
        self.plan()
            .iterations()
            .iter()
            .enumerate()
            .filter_map(|(index, control)| self.uniform_unsigned_collection(index, control))
            .collect()
    }

    fn uniform_unsigned_collection(
        &self,
        index: usize,
        control: &NativeControlLoop,
    ) -> Option<NativeUniformWordCollection> {
        let NativeControlLoop::Collection(generator) = control else {
            return None;
        };
        let NativeCollectionSource::U64Range(cursor) = generator.source() else {
            return None;
        };
        #[cfg(any(
            feature = "pure_text",
            feature = "map_ops",
            feature = "sequence_buffers"
        ))]
        let range_kind = match cursor.template().primitive().name() {
            "range/inclusive" => Some(true),
            "range/exclusive" => Some(false),
            _ => None,
        };
        // The base profile has no typed selected-library vocabulary. Decline
        // this optional proof rather than infer factory semantics from a name.
        #[cfg(not(any(
            feature = "pure_text",
            feature = "map_ops",
            feature = "sequence_buffers"
        )))]
        let range_kind: Option<bool> = None;
        let inclusive = range_kind?;
        let plan = self.plan();
        let definition = plan.collections().get(generator.collection() as usize)?;
        if definition.generators() != [index as u32]
            || !matches!(definition.result(), NativeCollectionResult::Materialized)
        {
            return None;
        }
        let [start, limit] = *cursor.operands();
        if !self.available_before(start, generator.initialize())
            || !self.available_before(limit, generator.initialize())
        {
            return None;
        }
        let mut sites = BTreeSet::new();
        let complete = complete_body(
            plan,
            generator.body(),
            generator.latch(),
            index as u32,
            generator.collection(),
            &mut sites,
        );
        let filtered = complete.is_none();
        let mut condition = None;
        let word = if let Some(word) = complete {
            word
        } else {
            sites.clear();
            if !inclusive
                || !plan.constants().iter().any(|constant| {
                    constant.value() == start
                        && matches!(constant.constant().data(), mech_core::ValueData::U64(0))
                })
            {
                return None;
            }
            let element = generator.element()?;
            let mut site = generator.body();
            let mut saw_element = false;
            let (tested, yes, no) = loop {
                if !sites.insert(site) {
                    return None;
                }
                let block = plan.blocks().get(site as usize)?;
                if block.iteration() != Some(index as u32) {
                    return None;
                }
                match (block.action(), block.terminator()) {
                    (Action::Scalar(Scalar::None), Terminator::Scalar(Term::Jump(next))) => {
                        site = next
                    }
                    (
                        Action::CollectionElement { iteration },
                        Terminator::Scalar(Term::Jump(next)),
                    ) if *iteration == index as u32 && !saw_element => {
                        saw_element = true;
                        site = next;
                    }
                    (
                        Action::Scalar(Scalar::Primitive {
                            op: NativeScalarOp::U64Lt,
                            operands,
                            output,
                        }),
                        Terminator::Scalar(Term::Branch { condition, yes, no }),
                    ) if saw_element
                        && operands.as_ref() == [element, limit]
                        && *output == condition =>
                    {
                        break (condition, yes, no);
                    }
                    _ => return None,
                }
            };
            let word = arm(
                plan,
                yes,
                generator.latch(),
                index as u32,
                generator.collection(),
                &mut sites,
            )??;
            if arm(
                plan,
                no,
                generator.latch(),
                index as u32,
                generator.collection(),
                &mut sites,
            )?
            .is_some()
            {
                return None;
            }
            condition = Some(tested);
            word
        };
        if !self.available_before(word, generator.initialize()) {
            return None;
        }
        let latch = plan.blocks().get(generator.latch() as usize)?;
        if !matches!(latch.action(), Action::Latch { iteration } if *iteration == index as u32)
            || latch.terminator() != Terminator::Scalar(Term::Jump(generator.header()))
            || !sites.insert(generator.latch())
        {
            return None;
        }
        // No skipped body definition can feed a later outer operation,
        // collection source, lexical effect or published output.
        for value in generator.element().into_iter().chain(condition) {
            if self.read_roles(value).ok()?.iter().any(|read| {
                !matches!(read,
                    Read::Action(site) | Read::Terminator(site) if sites.contains(site))
            }) {
                return None;
            }
        }
        // The selected body contains every owned visit. No activation,
        // additional entry, hidden effect or unvisited owned block is omitted.
        for (site, block) in plan.blocks().iter().enumerate() {
            if block.iteration() == Some(index as u32)
                && site as u32 != generator.initialize()
                && site as u32 != generator.header()
                && !sites.contains(&(site as u32))
            {
                return None;
            }
        }
        if sites
            .iter()
            .any(|&site| plan.activation_boundary() == Some(site))
        {
            return None;
        }
        let kind = plan.values().get(word.get() as usize)?.scalar_kind()?;
        let body = plan
            .values()
            .get(definition.output().get() as usize)?
            .definition()
            .body();
        let mech_core::SchemaBody::Matrix {
            element,
            dimensions,
        } = body
        else {
            return None;
        };
        let expected = match kind {
            NativeScalarKind::U8 => mech_core::IntegerWidth::W8,
            NativeScalarKind::U16 => mech_core::IntegerWidth::W16,
            NativeScalarKind::U32 => mech_core::IntegerWidth::W32,
            NativeScalarKind::U64 => mech_core::IntegerWidth::W64,
            _ => return None,
        };
        if dimensions.len() != 2
            || element.as_ref() != &mech_core::SchemaBody::UnsignedInteger(expected)
        {
            return None;
        }
        Some(NativeUniformWordCollection {
            collection: generator.collection(),
            iteration: index as u32,
            header: generator.header(),
            word,
            start,
            limit,
            inclusive,
            filtered,
            kind,
        })
    }

    fn available_before(&self, value: Value, frontier: u32) -> bool {
        let Ok(definitions) = self.definition_sites(value.get()) else {
            return false;
        };
        if let [definition] = definitions {
            return match definition.site() {
                None => matches!(definition.role(), Role::Input | Role::Constant),
                Some(site) => self.strictly_dominates(site, frontier).unwrap_or(false),
            };
        }
        // A conditional result has one Copy definition on each arm. No single
        // arm dominates the consumer, but the complete set can cover every
        // path. Only immutable, same-scope Copy joins qualify; carriers and
        // generator elements must keep their current-visit protocols.
        let plan = self.plan();
        let Some(NativeIterationValueScope::Immutable { iteration: scope }) =
            plan.value_scopes().get(value.get() as usize)
        else {
            return false;
        };
        let Some(block) = plan.blocks().get(frontier as usize) else {
            return false;
        };
        if definitions.is_empty()
            || !self.lexical_ancestor(*scope, block.iteration())
            || !self.is_reachable(frontier).unwrap_or(false)
        {
            return false;
        }
        let mut writes = BTreeSet::new();
        for definition in definitions {
            let Some(site) = definition.site() else {
                return false;
            };
            if definition.role() != Role::Scalar
                || definition.iteration() != *scope
                || !matches!(plan.blocks()[site as usize].action(),
                    Action::Scalar(Scalar::Copy { output, .. }) if *output == value)
            {
                return false;
            }
            writes.insert(site);
        }
        // Start each lexical visit afresh. A write from a previous iteration
        // cannot establish availability for the current one.
        let entry = match scope {
            None => plan.entry(),
            Some(iteration) => match &plan.iterations()[*iteration as usize] {
                NativeControlLoop::Collection(loop_) => loop_.header(),
                NativeControlLoop::Iteration(loop_) => loop_.header(),
                NativeControlLoop::Tail(loop_) => loop_.header(),
            },
        };
        if scope.is_some() && !self.strictly_dominates(entry, frontier).unwrap_or(false) {
            return false;
        }
        let mut seen = vec![false; plan.blocks().len()];
        let mut pending = vec![entry];
        while let Some(site) = pending.pop() {
            // Check the consumer before its action: a write at the consumer
            // itself cannot establish prior availability.
            if site == frontier {
                return false;
            }
            if writes.contains(&site) || core::mem::replace(&mut seen[site as usize], true) {
                continue;
            }
            let Ok(next) = self.successors(site) else {
                return false;
            };
            pending.extend(next.into_iter().flatten());
        }
        true
    }
}

fn arm(
    plan: &Plan,
    mut site: u32,
    latch: u32,
    iteration: u32,
    collection: u32,
    sites: &mut BTreeSet<u32>,
) -> Option<Option<Value>> {
    let mut word = None;
    while site != latch {
        if !sites.insert(site) {
            return None;
        }
        let block = plan.blocks().get(site as usize)?;
        if block.iteration() != Some(iteration) {
            return None;
        }
        match block.action() {
            Action::Scalar(Scalar::None) => {}
            Action::CollectionPush {
                collection: actual,
                value,
            } if *actual == collection && word.is_none() => word = Some(*value),
            _ => return None,
        }
        let Terminator::Scalar(Term::Jump(next)) = block.terminator() else {
            return None;
        };
        site = next;
    }
    Some(word)
}

// Full source order is irrelevant only because the source element is unused
// by the yielded word, and the exhaustive outer-use inventory is checked by
// the caller. Preserve exactly one source visit and one push per iteration.
fn complete_body(
    plan: &Plan,
    mut site: u32,
    latch: u32,
    iteration: u32,
    collection: u32,
    sites: &mut BTreeSet<u32>,
) -> Option<Value> {
    let mut element = false;
    let mut word = None;
    while site != latch {
        if !sites.insert(site) {
            return None;
        }
        let block = plan.blocks().get(site as usize)?;
        if block.iteration() != Some(iteration) {
            return None;
        }
        match block.action() {
            Action::Scalar(Scalar::None) => {}
            Action::CollectionElement { iteration: actual }
                if *actual == iteration && !element && word.is_none() =>
            {
                element = true
            }
            Action::CollectionPush {
                collection: actual,
                value,
            } if *actual == collection && element && word.is_none() => word = Some(*value),
            _ => return None,
        }
        let Terminator::Scalar(Term::Jump(next)) = block.terminator() else {
            return None;
        };
        site = next;
    }
    word
}
