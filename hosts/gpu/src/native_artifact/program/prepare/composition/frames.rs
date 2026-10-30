//! Current frame/array sources for one acyclic parent piece. Sources are checked
//! descriptions; the runtime must still establish current invocation availability.
use super::Segment;
use crate::native_artifact::program::prepare::readonly_numeric::FrameWords;
use mech_core::{IntegerWidth, SchemaBody};
use mech_engine::native_artifact::{
    NativeBufferFrameLane as Lane, NativeBufferFrameOperation as Frame, NativeBufferFrames,
    NativeIterationAction as Action, NativeMatchValue, NativeProgramArtifactPlan as Plan,
    NativeProgramFacts as Facts, NativeProgramReadRole as Read,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, serde::Serialize)]
pub(in crate::native_artifact) enum Source {
    Value(u32),
    Field { value: u32, ordinal: usize },
}
#[derive(Debug, Eq, PartialEq, serde::Serialize)]
pub(in crate::native_artifact) struct Exit {
    pub from: u32,
    pub to: u32,
    pub frames: Vec<NativeMatchValue>,
}
#[derive(Debug, Eq, PartialEq, serde::Serialize)]
pub(in crate::native_artifact) struct Layout {
    pub sources: Vec<Source>,
    pub frames: Vec<FrameWords>,
    pub inputs: Vec<u32>,
    pub scalar_words: usize,
    pub retirements: Vec<(u32, Vec<u32>)>,
    pub exits: Vec<Exit>,
}

struct Origins<'a, 'p> {
    plan: &'p Plan,
    facts: &'a Facts<'p>,
    frames: &'a NativeBufferFrames,
    members: &'a BTreeSet<u32>,
    writes: BTreeMap<u32, Vec<usize>>,
    cache: BTreeMap<(u32, usize), Source>,
    active: BTreeSet<(u32, usize)>,
}
impl Origins<'_, '_> {
    fn value(&self, value: u32) -> Result<Source, &'static str> {
        if !unsigned_matrix(self.plan.values()[value as usize].definition().body()) {
            return Err("parent piece requires an unsigned matrix source");
        }
        if self
            .facts
            .definition_sites(value)?
            .iter()
            .filter_map(|d| d.site())
            .any(|s| self.members.contains(&s))
        {
            return Err("parent piece changes an arena array source");
        }
        Ok(Source::Value(value))
    }
    fn field(&mut self, value: u32, ordinal: usize) -> Result<Source, &'static str> {
        let key = (value, ordinal);
        if let Some(source) = self.cache.get(&key) {
            return Ok(source.clone());
        }
        if !self.active.insert(key) {
            return Err("parent frame forwarding is cyclic");
        }
        let definitions = self.writes.get(&value).cloned().unwrap_or_default();
        let result = if definitions.is_empty() {
            let body = self.plan.values()[value as usize].definition().body();
            let field = match body {
                SchemaBody::Tuple(fields) => fields.get(ordinal),
                SchemaBody::Record(fields) => fields.get(ordinal).map(|f| &f.schema),
                _ => None,
            }
            .ok_or("parent frame source lacks an exact field schema")?;
            if !unsigned_matrix(field) {
                return Err("parent frame source is not an unsigned matrix");
            }
            Ok(Source::Field { value, ordinal })
        } else {
            let mut selected = None;
            for site in definitions {
                let source = match self.frames.action(site).unwrap().operation() {
                    Frame::Pack { .. } => {
                        let Action::LibraryCall { operands, .. } =
                            self.plan.blocks()[site].action()
                        else {
                            return Err("parent frame pack lacks selected operands");
                        };
                        self.value(
                            operands
                                .get(ordinal)
                                .ok_or("parent frame pack field is outside operands")?
                                .get(),
                        )?
                    }
                    Frame::Copy { source, .. } => self.field(source.get(), ordinal)?,
                    Frame::Project { .. } => {
                        return Err("parent managed projection crossed a boundary");
                    }
                };
                if selected.as_ref().is_some_and(|prior| *prior != source) {
                    return Err("parent frame field changes across alternative definitions");
                }
                selected = Some(source);
            }
            selected.ok_or("parent frame has no current definition")
        };
        self.active.remove(&key);
        let result = result?;
        self.cache.insert(key, result.clone());
        Ok(result)
    }
}

impl Layout {
    pub(super) fn derive<'p>(
        plan: &'p Plan,
        facts: &Facts<'p>,
        frames: &NativeBufferFrames,
        segment: &Segment,
    ) -> Result<Self, &'static str> {
        let members = segment.sites.iter().copied().collect::<BTreeSet<_>>();
        let mut writes = BTreeMap::<u32, Vec<usize>>::new();
        let mut written_values = BTreeMap::<u32, NativeMatchValue>::new();
        let mut values = BTreeSet::new();
        let mut retirements = Vec::new();
        for &site in &segment.sites {
            if let Some(frame) = frames.action(site as usize) {
                match frame.operation() {
                    Frame::Pack { output } => {
                        writes.entry(output.get()).or_default().push(site as usize);
                        written_values.insert(output.get(), *output);
                        values.insert(output.get());
                    }
                    Frame::Copy { source, output } => {
                        writes.entry(output.get()).or_default().push(site as usize);
                        written_values.insert(output.get(), *output);
                        values.extend([source.get(), output.get()]);
                    }
                    Frame::Project { source, .. } => {
                        values.insert(source.get());
                    }
                }
                if !frame.retire().is_empty() {
                    retirements.push((site, frame.retire().iter().map(|v| v.get()).collect()));
                }
            }
        }
        let inputs = values
            .iter()
            .filter(|v| !writes.contains_key(v))
            .copied()
            .collect::<Vec<_>>();
        let mut origins = Origins {
            plan,
            facts,
            frames,
            members: &members,
            writes,
            cache: BTreeMap::new(),
            active: BTreeSet::new(),
        };
        let mut sources = Vec::new();
        let mut words = 0usize;
        let mut recipes = Vec::new();
        for value in values {
            let mut scalar = Vec::new();
            let mut arrays = Vec::new();
            for (ordinal, field) in frames
                .fields_at(value as usize)
                .ok_or("parent frame lacks exact fields")?
                .iter()
                .enumerate()
            {
                match field.lane() {
                    Lane::Scalar { .. } => {
                        scalar.push(Some(words));
                        arrays.push(None);
                        words = words.checked_add(1).ok_or("parent scalar words overflow")?;
                    }
                    Lane::Managed { .. } => {
                        let source = origins.field(value, ordinal)?;
                        let at = if let Some(at) = sources.iter().position(|v| *v == source) {
                            at
                        } else {
                            sources.push(source);
                            sources.len() - 1
                        };
                        scalar.push(None);
                        arrays.push(Some(at));
                    }
                    Lane::Composite { .. } => return Err("parent frame requires flat fields"),
                }
            }
            recipes.push(FrameWords {
                value,
                fields: scalar,
                arrays,
            });
        }
        let mut current = BTreeMap::<u32, BTreeSet<u32>>::new();
        let mut possible = BTreeMap::<u32, BTreeSet<u32>>::new();
        for &site in &segment.sites {
            let predecessors = facts
                .predecessors(site)?
                .iter()
                .copied()
                .filter(|s| members.contains(s))
                .collect::<Vec<_>>();
            let mut valid = inputs.iter().copied().collect::<BTreeSet<_>>();
            let mut maybe = valid.clone();
            if let Some(first) = predecessors.first() {
                valid = current
                    .get(first)
                    .ok_or("parent frame order is not acyclic")?
                    .clone();
                maybe = possible[first].clone();
                for prior in &predecessors[1..] {
                    valid.retain(|v| current[prior].contains(v));
                    maybe.extend(&possible[prior]);
                }
            }
            if let Some(action) = frames.action(site as usize) {
                match action.operation() {
                    Frame::Copy { source, .. } | Frame::Project { source, .. }
                        if !valid.contains(&source.get()) =>
                    {
                        return Err("parent piece reads a missing or retired frame");
                    }
                    _ => {}
                }
                if let Frame::Pack { output } | Frame::Copy { output, .. } = action.operation() {
                    valid.insert(output.get());
                    maybe.insert(output.get());
                }
                for value in action.retire() {
                    valid.remove(&value.get());
                    maybe.remove(&value.get());
                }
            }
            current.insert(site, valid);
            possible.insert(site, maybe);
        }
        let external = origins
            .writes
            .keys()
            .filter_map(|&value| {
                match facts.read_roles(written_values[&value]) {
                    Ok(reads)
                        if reads.iter().any(|r| match r {
                            Read::Action(site) | Read::Terminator(site) => !members.contains(site),
                            // Do not reduce collection reads to initialization: finalization
                            // can read the original source again for its matrix dimensions.
                            _ => true,
                        }) =>
                    {
                        Some(Ok(value))
                    }
                    Ok(_) => None,
                    Err(error) => Some(Err(error)),
                }
            })
            .collect::<Result<BTreeSet<_>, _>>()?;
        let mut exits = Vec::new();
        for &(from, to) in &segment.exits {
            if possible[&from]
                .iter()
                .any(|v| external.contains(v) && !current[&from].contains(v))
            {
                return Err("parent exit has path-dependent exported frame validity");
            }
            exits.push(Exit {
                from,
                to,
                frames: current[&from]
                    .intersection(&external)
                    .map(|v| written_values[v])
                    .collect(),
            });
        }
        Ok(Self {
            sources,
            frames: recipes,
            inputs,
            scalar_words: words,
            retirements,
            exits,
        })
    }
}

fn unsigned_matrix(body: &SchemaBody) -> bool {
    matches!(body, SchemaBody::Matrix { element, dimensions }
        if dimensions.len() == 2 && matches!(element.as_ref(),
            SchemaBody::UnsignedInteger(IntegerWidth::W8 | IntegerWidth::W16
                | IntegerWidth::W32 | IntegerWidth::W64)))
}
