//! Successful whole-producer array-content facts, never partial-loop authority.
use super::*;
use crate::native_artifact::program::prepare::ProgramMetadata;
use mech_engine::native_artifact::{NativeCollectionPrimitive as Primitive, NativeScalarOp as Op};

const MAX_DEPTH: usize = 64;
const MAX_CHAIN: usize = 32;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, serde::Serialize)]
enum Origin {
    Factory(usize),
    Returned { region: usize, array: usize },
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub(in crate::native_artifact) struct ValidatedElements {
    pub producer: usize,
    pub array: usize,
    pub bound: usize,
    pub checked_appends: Vec<(usize, usize)>,
}

struct Context<'a> {
    metadata: &'a ProgramMetadata,
    facts: NativeProgramFacts<'a>,
    origins: BTreeMap<numeric_fields::Key, Option<Origin>>,
    active: BTreeSet<numeric_fields::Key>,
}
impl<'a> Context<'a> {
    fn new(metadata: &'a ProgramMetadata) -> Self {
        Self { metadata, facts: metadata.plan.facts(), origins: BTreeMap::new(), active: BTreeSet::new() }
    }
    fn definition(&self, value: u32) -> Option<usize> {
        let [definition] = self.facts.definition_sites(value).ok()? else { return None; };
        Some(definition.site()? as usize)
    }
    /// One exact structural leaf transfer, independent of native physical homes.
    fn source(&self, key: numeric_fields::Key) -> Option<numeric_fields::Key> {
        let site = self.definition(key.0)?;
        let plan = &self.metadata.plan;
        let frames = &self.metadata.buffer_frames;
        let operation = frames.action(site).map(|r| r.operation());
        match operation {
            Some(Frame::Pack { output }) if output.get() == key.0 => {
                let leaf = key.1?;
                let Action::LibraryCall { operands, .. } = plan.blocks()[site].action() else { return None; };
                for (ordinal, operand) in operands.iter().enumerate() {
                    let range = numeric_fields::span(frames, key.0, ordinal)?;
                    if range.contains(&leaf) {
                        return numeric_fields::operand(frames, *operand, leaf - range.start);
                    }
                }
                None
            }
            Some(Frame::Copy { source, output }) if output.get() == key.0 => Some((source.get(), key.1)),
            Some(Frame::Project { source, ordinal, output }) if output.get() == key.0 => {
                let range = numeric_fields::span(frames, source.get(), *ordinal)?;
                let offset = key.1.unwrap_or(0);
                (offset < range.len()).then_some((source.get(), Some(range.start + offset)))
            }
            _ => match plan.blocks()[site].action() {
                Action::Scalar(Scalar::Copy { source, output }) if output.get() == key.0 => Some((source.get(), key.1)),
                _ => None,
            },
        }
    }
    fn origin(&mut self, key: numeric_fields::Key, depth: usize) -> Option<Origin> {
        if depth == MAX_DEPTH { return None; }
        if let Some(answer) = self.origins.get(&key) { return *answer; }
        if !self.active.insert(key) { return None; }
        let answer = self.origin_inner(key, depth);
        self.active.remove(&key);
        self.origins.insert(key, answer);
        answer
    }
    fn origin_inner(&mut self, key: numeric_fields::Key, depth: usize) -> Option<Origin> {
        for (region, r) in self.metadata.closed_tails.regions.iter().enumerate() {
            if r.tail.result().get() != key.0 { continue; }
            let numeric = r.numeric.as_ref()?;
            // The source result definition, not a carrier/partial-return home,
            // is the only boundary allowed to export completed contents.
            let definition = self.definition(key.0)?;
            if definition != r.tail.exit() as usize { return None; }
            let array = match (&numeric.result, key.1) {
                (ResultRecipe::Array { array }, None) => *array,
                (ResultRecipe::Frame { arrays, .. }, Some(leaf)) => arrays.iter().find(|(field, _)| *field == leaf)?.1,
                _ => return None,
            };
            return Some(Origin::Returned { region, array });
        }
        if let Some(source) = self.source(key) { return self.origin(source, depth + 1); }
        let site = self.definition(key.0)?;
        match self.metadata.plan.blocks()[site].action() {
            Action::LibraryCall { output, .. } if output.get() == key.0 && key.1.is_none() => Some(Origin::Factory(site)),
            _ => None,
        }
    }
    fn entry(&mut self, region: usize, array: usize) -> Option<Origin> {
        let r = &self.metadata.closed_tails.regions[region];
        let array = r.numeric.as_ref()?.arrays.get(array)?;
        let key = match array.storage {
            ArrayStorage::Carrier { field, .. } => (r.tail.seed().get(), Some(field)),
            ArrayStorage::Capture { value } => (value, None),
            ArrayStorage::FrameCapture { value, field, .. } => (value, Some(field)),
        };
        self.origin(key, 0)
    }
    fn u64(&self, key: numeric_fields::Key, depth: usize) -> Option<u64> {
        if depth == MAX_DEPTH { return None; }
        if key.1.is_none() {
            if let Some(c) = self.metadata.plan.constants().iter().find(|c| c.value().get() == key.0) {
                if let mech_core::ValueData::U64(v) = c.constant().data() { return Some(*v); }
            }
        }
        self.u64(self.source(key)?, depth + 1)
    }
    fn one_based(&self, mut index: u32, incoming: u32) -> bool {
        for _ in 0..MAX_DEPTH {
            let Some(site) = self.definition(index) else { return false; };
            match self.metadata.plan.blocks()[site].action() {
                Action::Scalar(Scalar::Copy { source, .. }) => index = source.get(),
                Action::Scalar(Scalar::Primitive { op: Op::U64Add, operands, .. }) if operands.len() == 2 => {
                    return (operands[0].get() == incoming && self.u64((operands[1].get(), None), 0) == Some(1))
                        || (operands[1].get() == incoming && self.u64((operands[0].get(), None), 0) == Some(1));
                }
                _ => return false,
            }
        }
        false
    }
    fn validation(&self, numeric: &Proof, site: usize, incoming: u32) -> Option<(usize, usize)> {
        let mut site = site;
        for _ in 0..MAX_CHAIN {
            let Term::Scalar(Branch::Jump(next)) = self.metadata.plan.blocks()[site].terminator() else { return None; };
            site = next as usize;
            if !numeric.sites.contains(&site) { return None; }
            match numeric.action(site) {
                Some(Operation::Read { array, index, .. }) if !numeric.arrays[*array].appends && self.one_based(*index, incoming) => return Some((site, *array)),
                Some(Operation::Frame | Operation::Aggregate) => {}
                None if matches!(self.metadata.plan.blocks()[site].action(), Action::Scalar(_)) => {}
                _ => return None,
            }
        }
        None
    }
    fn producer(&mut self, region: usize, array: usize) -> Option<ValidatedElements> {
        let metadata = self.metadata;
        let r = &metadata.closed_tails.regions[region];
        let numeric = r.numeric.as_ref()?;
        let a = numeric.arrays.get(array)?;
        if a.width != 8 || !a.appends || !numeric.exits.is_empty()
            || numeric.actions.iter().any(|(_, op)| matches!(op, Operation::Fallback)) { return None; }
        let Origin::Factory(seed) = self.entry(region, array)? else { return None; };
        let Action::LibraryCall { template, .. } = self.metadata.plan.blocks()[seed].action() else { return None; };
        if template.primitive() != Primitive::SequenceWithCapacity { return None; }
        let mut checked_appends = Vec::new();
        let mut bound = None;
        for &(site, ref op) in &numeric.actions {
            match op {
                Operation::Append { array: at, incoming } if *at == array => {
                    let (read, target) = self.validation(numeric, site, *incoming)?;
                    if target == array || bound.is_some_and(|old| old != target) { return None; }
                    bound = Some(target);
                    checked_appends.push((site, read));
                }
                Operation::Write { array: at, .. } | Operation::AppendLeBytes { array: at, .. } if *at == array => return None,
                _ => {}
            }
        }
        Some(ValidatedElements { producer: r.tail.header() as usize, array, bound: bound?, checked_appends })
    }
}

pub(in crate::native_artifact) fn producer_candidates(metadata: &ProgramMetadata) -> Vec<ValidatedElements> {
    if metadata.plan.blocks().len() > 16_384 || metadata.plan.values().len() > 65_536 { return Vec::new(); }
    let mut cx = Context::new(metadata);
    let mut out = Vec::new();
    for (region, r) in metadata.closed_tails.regions.iter().enumerate() {
        let Some(numeric) = &r.numeric else { continue; };
        for array in 0..numeric.arrays.len() {
            if let Some(fact) = cx.producer(region, array) { out.push(fact); }
        }
    }
    out
}

#[cfg(test)]
#[path = "completed_element_ranges_tests.rs"]
mod tests;
