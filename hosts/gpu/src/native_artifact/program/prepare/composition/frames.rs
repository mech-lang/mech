//! Current frame/owner sources for one acyclic parent piece. Sources are checked
//! descriptions; the runtime must still establish current invocation availability.
use super::Segment;
use crate::native_artifact::program::prepare::readonly_numeric::{
    FrameWords, ScalarBinding, scalar_bindings, scalar_children, scalar_node_count, scalar_paths,
};
use mech_engine::native_artifact::{
    NativeBufferFrameLane as Lane, NativeBufferFrameOperation as Frame, NativeBufferFrames,
    NativeIterationAction as Action, NativeMatchValue, NativeProgramArtifactPlan as Plan,
    NativeProgramFacts as Facts, NativeProgramReadRole as Read,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) enum Source {
    Value(u32),
    Field { value: u32, ordinal: usize },
}
#[derive(Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Exit {
    pub from: u32,
    pub to: u32,
    #[serde(deserialize_with = "super::image::values")]
    pub frames: Vec<NativeMatchValue>,
    #[serde(deserialize_with = "super::image::values")]
    pub aliases: Vec<NativeMatchValue>,
}
#[derive(Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Alias {
    #[serde(deserialize_with = "super::image::value")]
    pub value: NativeMatchValue,
    pub source: usize,
    /// A library result keeps its declared output limits even when its owner
    /// remains private. Structural forwarding has no declared call boundary.
    pub call_result: bool,
}
#[derive(Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Layout {
    pub sources: Vec<Source>,
    pub aliases: Vec<Alias>,
    pub frames: Vec<FrameWords>,
    pub inputs: Vec<u32>,
    pub scalar_words: usize,
    pub scalar_bindings: Vec<ScalarBinding>,
    pub scalar_entry_steps: usize,
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
    fn value(&mut self, value: u32) -> Result<Source, &'static str> {
        if self.plan.values()[value as usize].scalar_kind().is_some() {
            return Err("parent piece requires a managed owner source");
        }
        let definitions = self
            .facts
            .definition_sites(value)?
            .iter()
            .filter_map(|d| d.site())
            .collect::<Vec<_>>();
        if definitions.iter().any(|s| self.members.contains(s)) {
            let mut selected = None;
            for site in definitions {
                if !self.members.contains(&site) {
                    return Err("parent array alias has an external alternative definition");
                }
                let Some(Frame::Project {
                    source,
                    ordinal,
                    output,
                }) = self
                    .frames
                    .action(site as usize)
                    .map(|action| action.operation())
                else {
                    return Err("parent piece changes an arena array source");
                };
                if output.get() != value {
                    return Err("parent array alias differs from its projection result");
                }
                let origin = self.field(source.get(), *ordinal)?;
                if selected.as_ref().is_some_and(|old| *old != origin) {
                    return Err("parent array alias changes across alternative definitions");
                }
                selected = Some(origin);
            }
            return selected.ok_or("parent array alias lacks a definition");
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
            let field = self
                .frames
                .fields_at(value as usize)
                .and_then(|fields| fields.get(ordinal))
                .ok_or("parent frame source lacks an exact field schema")?;
            if !matches!(field.lane(), Lane::Managed { .. }) {
                return Err("parent frame source is not a managed owner");
            }
            Ok(Source::Field { value, ordinal })
        } else {
            let mut selected = None;
            for site in definitions {
                let source = match self.frames.action(site).unwrap().operation() {
                    Frame::EnumPack { .. } | Frame::Observe { .. } => {
                        return Err("tagged frame outside numeric parent provenance");
                    }
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
        let mut projected = BTreeMap::<u32, NativeMatchValue>::new();
        let mut values = BTreeSet::new();
        let mut retirements = Vec::new();
        for &site in &segment.sites {
            if let Some(frame) = frames.action(site as usize) {
                match frame.operation() {
                    Frame::EnumPack { .. } | Frame::Observe { .. } => {
                        return Err("tagged frame outside numeric parent composition");
                    }
                    Frame::Pack { output } => {
                        writes.entry(output.get()).or_default().push(site as usize);
                        written_values.insert(output.get(), *output);
                        values.insert(output.get());
                        if let Action::LibraryCall { operands, .. } =
                            plan.blocks()[site as usize].action()
                        {
                            values.extend(
                                operands
                                    .iter()
                                    .filter(|value| frames.fields(**value).is_some())
                                    .map(|value| value.get()),
                            );
                        }
                    }
                    Frame::Copy { source, output } => {
                        writes.entry(output.get()).or_default().push(site as usize);
                        written_values.insert(output.get(), *output);
                        values.extend([source.get(), output.get()]);
                    }
                    Frame::Project { source, output, .. } => {
                        values.insert(source.get());
                        if frames.fields(*output).is_some() {
                            writes.entry(output.get()).or_default().push(site as usize);
                            values.insert(output.get());
                            written_values.insert(output.get(), *output);
                        } else if plan.values()[output.get() as usize].scalar_kind().is_none() {
                            projected.insert(output.get(), *output);
                            written_values.insert(output.get(), *output);
                        }
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
        let mut physical_bindings = Vec::new();
        for value in values {
            let mut scalar = Vec::new();
            let mut arrays = Vec::new();
            let fields = frames
                .fields_at(value as usize)
                .ok_or("parent frame lacks exact fields")?;
            if fields.iter().any(|field| {
                matches!(field.lane(), Lane::Tagged { .. })
                    || (matches!(field.lane(), Lane::Composite { .. })
                        && !scalar_children(frames, *field))
            }) {
                return Err("parent composite frame requires scalar-only children");
            }
            let mut paths = Vec::new();
            scalar_paths(frames, fields, &mut Vec::new(), &mut words, &mut paths);
            for (ordinal, field) in frames
                .fields_at(value as usize)
                .ok_or("parent frame lacks exact fields")?
                .iter()
                .enumerate()
            {
                match field.lane() {
                    Lane::Scalar { .. } => {
                        scalar.push(
                            paths
                                .iter()
                                .find(|(path, _)| path.as_slice() == [ordinal])
                                .map(|(_, slot)| *slot),
                        );
                        arrays.push(None);
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
                    Lane::Tagged { .. } => return Err("tagged frame outside numeric parent lanes"),
                    Lane::Composite { .. } => {
                        scalar.push(None);
                        arrays.push(None);
                    }
                }
            }
            physical_bindings.extend(scalar_bindings(frames, value, &paths)?);
            recipes.push(FrameWords {
                value,
                scalar_node_count: scalar_node_count(&paths),
                scalar_paths: paths,
                fields: scalar,
                array_bindings: fields
                    .iter()
                    .zip(&arrays)
                    .filter_map(|(field, array)| match (field.lane(), array) {
                        (Lane::Managed { at }, Some(array)) => Some((at, *array)),
                        _ => None,
                    })
                    .collect(),
                maximum_arity: fields.len(),
                arrays,
            });
        }
        let mut aliases = Vec::new();
        for &value in projected.values() {
            let origin = origins.value(value.get())?;
            let source = if let Some(at) = sources.iter().position(|source| *source == origin) {
                at
            } else {
                sources.push(origin);
                sources.len() - 1
            };
            let call_result = facts
                .definition_sites(value.get())?
                .iter()
                .any(|definition| {
                    definition.site().is_some_and(|site| {
                        matches!(
                            plan.blocks()[site as usize].action(),
                            Action::LibraryCall { .. }
                        )
                    })
                });
            aliases.push(Alias {
                value,
                source,
                call_result,
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
                if let Frame::Project { output, .. } = action.operation()
                    && (projected.contains_key(&output.get()) || frames.fields(*output).is_some())
                {
                    valid.insert(output.get());
                    maybe.insert(output.get());
                }
                if let Frame::Pack { .. } = action.operation()
                    && let Action::LibraryCall { operands, .. } =
                        plan.blocks()[site as usize].action()
                    && operands.iter().any(|value| {
                        (projected.contains_key(&value.get()) || frames.fields(*value).is_some())
                            && !valid.contains(&value.get())
                    })
                {
                    return Err("parent pack reads an unreached or retired array alias");
                }
                for value in action.retire() {
                    valid.remove(&value.get());
                    maybe.remove(&value.get());
                }
            } else if let Action::LibraryCall { operands, .. } =
                plan.blocks()[site as usize].action()
                && operands.iter().any(|value| {
                    projected.contains_key(&value.get()) && !valid.contains(&value.get())
                })
            {
                return Err("parent numeric use reads an unreached or retired array alias");
            }
            current.insert(site, valid);
            possible.insert(site, maybe);
        }
        let external = written_values
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
                    .filter(|value| !projected.contains_key(value))
                    .map(|v| written_values[v])
                    .collect(),
                aliases: current[&from]
                    .intersection(&external)
                    .filter(|value| projected.contains_key(value))
                    .map(|value| written_values[value])
                    .collect(),
            });
        }
        let scalar_entry_steps = physical_bindings
            .iter()
            .try_fold(0usize, |sum, binding| {
                let frame = recipes.iter().find(|frame| frame.value == binding.value)?;
                let (path, _) = frame
                    .scalar_paths
                    .iter()
                    .find(|(_, word)| *word == binding.word)?;
                sum.checked_add(path.len().saturating_sub(1))
            })
            .ok_or("parent scalar entry price overflows")?;
        Ok(Self {
            sources,
            aliases,
            frames: recipes,
            inputs,
            scalar_words: words,
            scalar_entry_steps,
            scalar_bindings: physical_bindings,
            retirements,
            exits,
        })
    }
}
