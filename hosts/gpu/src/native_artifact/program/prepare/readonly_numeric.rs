//! Same-plan proof for a packed numeric leaf under a suspended parent.
//! Exact frame recipes own the connected escape analysis. This adds identity
//! provenance for every managed field, including every alternative definition.
use mech_core::{IntegerWidth, SchemaBody};
use mech_engine::native_artifact::{
    NativeAggregateWord, NativeBufferFrameLane as Lane, NativeBufferFrameOperation as Frame,
    NativeBufferFrames, NativeControlLoop, NativeIterationAction as Action,
    NativeIterationTerminator as Term, NativeMatchAction as Scalar,
    NativeMatchTerminator as Branch, NativePreparedLibraryCall, NativeProgramArtifactPlan as Plan,
    NativeProgramFacts, NativeScalarAggregateAction as Aggregate, NativeScalarAggregates,
    NativeScalarKind, NativeTailLoop,
};
use std::collections::{BTreeMap, BTreeSet};
#[path = "numeric_append.rs"]
mod append;
#[path = "numeric_fields.rs"]
pub(super) mod numeric_fields;
#[path = "numeric_results.rs"]
mod results;
#[path = "numeric_versions.rs"]
mod versions;
pub(in crate::native_artifact) use results::ResultRecipe;
pub(super) use results::scalar_aggregate;

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Array {
    /// Flattened structural leaf in the exact root carrier.
    pub field: usize,
    /// Same-plan current owner slot; never supplied by source or caller.
    pub physical: usize,
    pub schema: mech_core::SchemaId,
    pub width: u8,
    /// Exact selected site whose original factory validates the entry view.
    pub witness: usize,
    pub writable: bool,
    pub appends: bool,
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct FrameWords {
    pub value: u32,
    /// Relative direct scalar lane, or None for an array/composite field.
    pub fields: Vec<Option<usize>>,
    pub arrays: Vec<Option<usize>>,
    pub array_bindings: Vec<(usize, usize)>,
    pub maximum_arity: usize,
    /// Recursive scalar leaves in field order. Paths retain exact structural
    /// identity; a composite span never hides a managed array or owner.
    pub scalar_paths: Vec<(Vec<usize>, usize)>,
    /// Count exact structural prefixes once during independent preparation.
    pub scalar_node_count: usize,
}
impl FrameWords {
    pub fn scalar_slots(&self) -> impl Iterator<Item = usize> + '_ {
        self.scalar_paths.iter().map(|(_, slot)| *slot)
    }
    pub fn field_slots(&self, ordinal: usize) -> impl Iterator<Item = usize> + '_ {
        self.scalar_paths
            .iter()
            .filter(move |(path, _)| path.first() == Some(&ordinal))
            .map(|(_, slot)| *slot)
    }
    pub fn scalar_nodes(&self) -> usize {
        self.scalar_node_count
    }
}
pub(in crate::native_artifact::program::prepare) fn scalar_node_count(
    paths: &[(Vec<usize>, usize)],
) -> usize {
    let mut previous: &[usize] = &[];
    let mut count = 0;
    for (path, _) in paths {
        let common = previous
            .iter()
            .zip(path)
            .take_while(|(a, b)| a == b)
            .count();
        count += path.len() - common;
        previous = path;
    }
    count
}
/// Fixed invocation scalar-bank offsets, derived from the exact current plan.
/// Neither an offset nor a schema path grants ownership or entry authority.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct ScalarBinding {
    pub value: u32,
    pub word: usize,
    pub physical: usize,
    #[serde(deserialize_with = "deserialize_scalar_kind")]
    pub kind: NativeScalarKind,
    pub steps: usize,
}
fn deserialize_scalar_kind<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<NativeScalarKind, D::Error> {
    let value = <String as serde::Deserialize>::deserialize(deserializer)?;
    match value.as_str() {
        "Bool" => Ok(NativeScalarKind::Bool),
        "U8" => Ok(NativeScalarKind::U8),
        "U16" => Ok(NativeScalarKind::U16),
        "U32" => Ok(NativeScalarKind::U32),
        "U64" => Ok(NativeScalarKind::U64),
        "I64" => Ok(NativeScalarKind::I64),
        "F64" => Ok(NativeScalarKind::F64),
        _ => Err(serde::de::Error::custom(
            "unknown prepared scalar binding kind",
        )),
    }
}
pub(in crate::native_artifact::program::prepare) fn scalar_bindings(
    frames: &NativeBufferFrames,
    value: u32,
    paths: &[(Vec<usize>, usize)],
) -> Result<Vec<ScalarBinding>, &'static str> {
    paths
        .iter()
        .map(|(path, word)| {
            let mut span = frames
                .fields_at(value as usize)
                .ok_or("scalar binding lacks frame")?;
            let mut leaf = None;
            for (depth, ordinal) in path.iter().enumerate() {
                let field = *span.get(*ordinal).ok_or("scalar binding leaves its span")?;
                if depth + 1 == path.len() {
                    leaf = Some(field);
                } else {
                    span = frames
                        .children(field)
                        .ok_or("scalar binding loses its child")?;
                }
            }
            let Some(Lane::Scalar { at, kind }) = leaf.map(|field| field.lane()) else {
                return Err("scalar binding is not a scalar leaf");
            };
            Ok(ScalarBinding {
                value,
                word: *word,
                physical: at,
                kind,
                steps: path.len(),
            })
        })
        .collect()
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) enum Operation {
    Frame,
    Aggregate,
    ArrayCopy,
    Length {
        array: usize,
        output: u32,
    },
    Read {
        array: usize,
        index: u32,
        output: u32,
    },
    Write {
        array: usize,
        index: u32,
        incoming: u32,
    },
    Append {
        array: usize,
        incoming: u32,
    },
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct ColdAlias {
    pub value: u32,
    pub array: usize,
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Proof {
    pub nested_managed: bool,
    pub arrays: Vec<Array>,
    pub frames: Vec<FrameWords>,
    pub actions: Vec<(usize, Operation)>,
    pub sites: Vec<usize>,
    pub scalar_words: usize,
    pub scalar_inputs: Vec<ScalarBinding>,
    pub scalar_outputs: Vec<ScalarBinding>,
    pub words: usize,
    /// Independently bounded source and shadow homes for entry/exit emission.
    pub boundary_words: usize,
    pub boundary_work: u64,
    pub result: ResultRecipe,
    /// Exact node-header continuation and current ancestor array values that
    /// ordinary execution can still consume without replaying their producer.
    pub cold_aliases: Vec<(usize, Vec<ColdAlias>)>,
    /// Exact immutable scalar Frame shadows needed by a final header fallback.
    /// These are restored only after the generated loan has finished.
    pub cold_scalars: Vec<(usize, Vec<ScalarBinding>)>,
    /// Exact current managed leaves of elided immutable ancestor frames.
    pub cold_frames: Vec<(usize, Vec<(usize, usize)>)>,
    /// Maximum appends before the next tree header or final exit. Includes the
    /// ancestor continuation after a child returns, not just its lexical body.
    pub append_limits: Vec<(usize, Vec<(usize, usize)>)>,
}
impl Proof {
    pub fn action(&self, site: usize) -> Option<&Operation> {
        self.actions
            .iter()
            .find(|(s, _)| *s == site)
            .map(|(_, action)| action)
    }
    #[cfg(test)]
    pub fn fields(&self, value: u32) -> Option<&[Option<usize>]> {
        self.frames
            .iter()
            .find(|f| f.value == value)
            .map(|f| f.fields.as_slice())
    }
    pub fn frame(&self, value: u32) -> Option<&FrameWords> {
        self.frames.iter().find(|frame| frame.value == value)
    }
    pub fn view(&self, array: usize) -> usize {
        self.scalar_words + array * 4
    }
    pub fn writes(&self) -> bool {
        self.arrays.iter().any(|array| array.writable)
    }
    pub fn private_flag(&self, array: usize) -> usize {
        self.scalar_words + self.arrays.len() * 4 + array
    }
    pub fn capacity(&self, array: usize) -> Option<usize> {
        self.arrays[array].appends.then(|| {
            self.scalar_words
                + self.arrays.len() * 5
                + self.arrays[..array].iter().filter(|a| a.appends).count()
        })
    }
    pub fn append_limit(&self, header: usize, array: usize) -> Option<usize> {
        self.append_limits
            .iter()
            .find(|(site, _)| *site == header)?
            .1
            .iter()
            .find(|(at, _)| *at == array)
            .map(|(_, limit)| *limit)
    }
}

pub(super) fn width(body: &SchemaBody) -> Option<u8> {
    let SchemaBody::Matrix {
        element,
        dimensions,
    } = body
    else {
        return None;
    };
    if dimensions.len() != 2 {
        return None;
    }
    Some(match element.as_ref() {
        SchemaBody::UnsignedInteger(IntegerWidth::W8) => 1,
        SchemaBody::UnsignedInteger(IntegerWidth::W16) => 2,
        SchemaBody::UnsignedInteger(IntegerWidth::W32) => 4,
        SchemaBody::UnsignedInteger(IntegerWidth::W64) => 8,
        _ => return None,
    })
}

pub(in crate::native_artifact::program::prepare) fn scalar_children(
    frames: &NativeBufferFrames,
    field: mech_engine::native_artifact::NativeBufferFrameField,
) -> bool {
    match field.lane() {
        Lane::Scalar { .. } => true,
        Lane::Composite { .. } => frames.children(field).is_some_and(|children| {
            !children.is_empty() && children.iter().all(|child| scalar_children(frames, *child))
        }),
        Lane::Managed { .. } | Lane::Tagged { .. } => false,
    }
}

pub(in crate::native_artifact::program::prepare) fn scalar_paths(
    frames: &NativeBufferFrames,
    fields: &[mech_engine::native_artifact::NativeBufferFrameField],
    path: &mut Vec<usize>,
    words: &mut usize,
    output: &mut Vec<(Vec<usize>, usize)>,
) {
    for (ordinal, field) in fields.iter().enumerate() {
        path.push(ordinal);
        match field.lane() {
            Lane::Scalar { .. } => {
                output.push((path.clone(), *words));
                *words += 1;
            }
            Lane::Composite { .. } => scalar_paths(
                frames,
                frames.children(*field).unwrap(),
                path,
                words,
                output,
            ),
            Lane::Managed { .. } | Lane::Tagged { .. } => {}
        }
        path.pop();
    }
}

// A key denotes either an array Value or one managed field in a private frame.
// All definitions must agree. Cycles are rejected except the explicit carrier
// roots, whose next-state identity is checked separately for every field.
struct Provenance<'a, 'p> {
    facts: &'a NativeProgramFacts<'p>,
    frames: &'a NativeBufferFrames,
    tail: &'a NativeTailLoop,
    calls: &'a [Option<NativePreparedLibraryCall>],
    cache: BTreeMap<(u32, Option<usize>), Option<usize>>,
    active: BTreeSet<(u32, Option<usize>)>,
    /// Child carrier fields are identity phis rooted through the exact seed.
    /// Their next/return values are independently checked before selection.
    child_roots: BTreeMap<(u32, usize), usize>,
}
impl Provenance<'_, '_> {
    fn array(&mut self, value: u32, field: Option<usize>) -> Option<usize> {
        let key = (value, field);
        if let Some(ordinal) = field {
            if let Some(root) = self.child_roots.get(&(value, ordinal)) {
                return Some(*root);
            }
        }
        if value == self.tail.carrier().get() {
            let ordinal = field?;
            return matches!(
                numeric_fields::at(self.frames, value)?.get(ordinal)?.lane(),
                Lane::Managed { .. }
            )
            .then_some(ordinal);
        }
        if let Some(answer) = self.cache.get(&key) {
            return *answer;
        }
        if !self.active.insert(key) {
            return None;
        }
        let answer = self.definitions(value, field);
        self.active.remove(&key);
        self.cache.insert(key, answer);
        answer
    }
    fn definitions(&mut self, value: u32, field: Option<usize>) -> Option<usize> {
        let definitions = self.facts.definition_sites(value).ok()?.to_vec();
        if definitions.is_empty() {
            return None;
        }
        let mut selected = None;
        for definition in definitions {
            let site = definition.site()? as usize;
            let action = self.facts.plan().blocks()[site].action();
            let origin = if let Some(frame) = self.frames.action(site) {
                match (frame.operation(), field) {
                    (Frame::Pack { output }, Some(leaf)) if output.get() == value => {
                        let Action::LibraryCall { operands, .. } = action else {
                            return None;
                        };
                        let (ordinal, range) = (0..operands.len()).find_map(|ordinal| {
                            let range = numeric_fields::span(self.frames, value, ordinal)?;
                            range.contains(&leaf).then_some((ordinal, range))
                        })?;
                        let (source, field) = numeric_fields::operand(
                            self.frames,
                            *operands.get(ordinal)?,
                            leaf - range.start,
                        )?;
                        self.array(source, field)?
                    }
                    (Frame::Copy { source, output }, Some(ordinal)) if output.get() == value => {
                        self.array(source.get(), Some(ordinal))?
                    }
                    (
                        Frame::Project {
                            source,
                            ordinal,
                            output,
                        },
                        relative,
                    ) if output.get() == value => {
                        let range = numeric_fields::span(self.frames, source.get(), *ordinal)?;
                        let at = range.start.checked_add(relative.unwrap_or(0))?;
                        if !range.contains(&at) {
                            return None;
                        }
                        self.array(source.get(), Some(at))?
                    }
                    _ => return None,
                }
            } else if let (Action::Scalar(Scalar::Copy { source, output }), None) = (action, field)
            {
                if output.get() != value {
                    return None;
                }
                self.array(source.get(), None)?
            } else if let (
                Action::LibraryCall {
                    operands, output, ..
                },
                None,
            ) = (action, field)
            {
                let call = self.calls.get(site)?.as_ref()?;
                if output.get() != value
                    || !(call.supports_unsigned_matrix_replace()
                        || call.supports_unsigned_matrix_append())
                {
                    return None;
                }
                self.array(operands.first()?.get(), None)?
            } else {
                return None;
            };
            if selected.is_some_and(|old| old != origin) {
                return None;
            }
            selected = Some(origin);
        }
        selected
    }
}

pub(in crate::native_artifact) fn derive(
    plan: &Plan,
    tail: &NativeTailLoop,
    frames: &NativeBufferFrames,
    aggregates: &NativeScalarAggregates,
    calls: &[Option<NativePreparedLibraryCall>],
) -> Result<Proof, &'static str> {
    derive_inner(plan, tail, frames, aggregates, calls, None)
}

pub(in crate::native_artifact) fn derive_tree(
    plan: &Plan,
    tail: &NativeTailLoop,
    frames: &NativeBufferFrames,
    aggregates: &NativeScalarAggregates,
    calls: &[Option<NativePreparedLibraryCall>],
    tree: &super::numeric_tree::Proof,
) -> Result<Proof, &'static str> {
    derive_inner(plan, tail, frames, aggregates, calls, Some(tree))
}

fn derive_inner(
    plan: &Plan,
    tail: &NativeTailLoop,
    frames: &NativeBufferFrames,
    aggregates: &NativeScalarAggregates,
    calls: &[Option<NativePreparedLibraryCall>],
    tree: Option<&super::numeric_tree::Proof>,
) -> Result<Proof, &'static str> {
    let fields = numeric_fields::at(frames, tail.carrier().get())
        .ok_or("numeric carrier lacks exact untagged leaves")?;
    let iteration = plan
        .iterations()
        .iter()
        .position(|c| matches!(c, NativeControlLoop::Tail(t) if t.header() == tail.header()))
        .ok_or("missing leaf iteration")? as u32;
    // The parent remains an ordinary suspended loop. A body traversal cannot
    // enter/exit any child or change the current invocation identity.
    let mut sites = BTreeSet::new();
    let mut pending = vec![tail.header()];
    while let Some(site) = pending.pop() {
        if site == tail.exit() || !sites.insert(site as usize) {
            continue;
        }
        let block = &plan.blocks()[site as usize];
        if (tree.map_or(block.iteration() != Some(iteration), |tree| {
            tree.node(site as usize).is_none()
        })) || plan.activation_boundary() == Some(site)
        {
            return Err("not a leaf: child loop or activation boundary");
        }
        match block.terminator() {
            Term::TailHeader { body, .. } | Term::Scalar(Branch::Jump(body)) => pending.push(body),
            Term::Scalar(
                Branch::Branch { yes, no, .. } | Branch::LiteralBranch { yes, no, .. },
            ) => pending.extend([yes, no]),
            Term::Scalar(Branch::Reject | Branch::IntegrityFailure) => {}
            _ => return Err("unsupported leaf terminator"),
        }
    }
    if !sites.contains(&(tail.latch() as usize)) || !sites.contains(&(tail.finish() as usize)) {
        return Err("incomplete leaf boundaries");
    }
    let facts = plan.facts();
    let mut provenance = Provenance {
        facts: &facts,
        frames,
        tail,
        calls,
        cache: BTreeMap::new(),
        active: BTreeSet::new(),
        child_roots: BTreeMap::new(),
    };
    if let Some(tree) = tree {
        for node in tree.nodes.iter().skip(1) {
            if let Some(fields) = numeric_fields::at(frames, node.tail.carrier().get()) {
                for (ordinal, field) in fields.iter().enumerate() {
                    if matches!(field.lane(), Lane::Managed { .. }) {
                        let root = provenance
                            .array(node.tail.seed().get(), Some(ordinal))
                            .ok_or("numeric child seed has an unknown array root")?;
                        provenance
                            .child_roots
                            .insert((node.tail.carrier().get(), ordinal), root);
                    }
                }
            } else if plan.values()[node.tail.carrier().get() as usize]
                .scalar_kind()
                .is_none()
                && aggregates.slot(node.tail.carrier()).is_none()
            {
                return Err("numeric child carrier lacks exact private scalar/frame words");
            }
        }
        // A failed cyclic lookup before a later child phi was known cannot be
        // reused once the complete declared identity roots have been installed.
        provenance.cache.clear();
    }
    let mut arrays = Vec::new();
    for (ordinal, field) in fields.iter().enumerate() {
        if matches!(field.lane(), Lane::Managed { .. }) {
            if provenance.array(tail.next().get(), Some(ordinal)) != Some(ordinal) {
                return Err("array field changes, swaps, or has unknown recursive forwarding");
            }
            arrays.push(Array {
                field: ordinal,
                physical: match field.lane() {
                    Lane::Managed { at } => at,
                    _ => unreachable!(),
                },
                schema: field.schema(),
                width: 0,
                witness: usize::MAX,
                writable: false,
                appends: false,
            });
        }
    }
    if arrays.is_empty() {
        return Err("no invariant numeric arrays");
    }
    let result = results::derive(
        plan,
        tail,
        frames,
        aggregates,
        &fields,
        &arrays,
        &mut provenance,
    )?;
    let mut actions = Vec::new();
    let mut frame_values = BTreeSet::from([tail.carrier().get()]);
    if let Some(value) = result.frame() {
        frame_values.insert(value);
    }
    for &site in &sites {
        let action = plan.blocks()[site].action();
        if let Action::LibraryCall { template, .. } = action {
            if !calls
                .get(site)
                .and_then(Option::as_ref)
                .is_some_and(|call| call.template() == template)
            {
                return Err("selected factory is not bound to this exact source site");
            }
        }
        let operation = if let Some(frame) = frames.action(site) {
            let mut remember = |value: u32| -> Result<(), &'static str> {
                let fields = numeric_fields::at(frames, value)
                    .ok_or("numeric frame lost exact untagged leaves")?;
                for (ordinal, field) in fields.iter().enumerate() {
                    if matches!(field.lane(), Lane::Managed { .. }) {
                        let origin = provenance.array(value, Some(ordinal));
                        let array = arrays
                            .iter()
                            .find(|a| origin == Some(a.field))
                            .ok_or("frame has an unknown managed field")?;
                        // A physical owner binding forwards this exact Value;
                        // schema rebinding requires the ordinary constructor.
                        if field.schema() != array.schema {
                            return Err("numeric frame leaf requires schema rebinding");
                        }
                    }
                }
                frame_values.insert(value);
                Ok(())
            };
            match frame.operation() {
                Frame::Pack { output } => {
                    if !calls
                        .get(site)
                        .and_then(Option::as_ref)
                        .is_some_and(|call| call.supports_invariant_buffer_frame())
                    {
                        return Err("frame has nonidentity or constrained shape relations");
                    }
                    remember(output.get())?;
                }
                Frame::Copy { source, output } => {
                    remember(source.get())?;
                    remember(output.get())?;
                }
                Frame::Project { source, output, .. } => {
                    remember(source.get())?;
                    if frames.fields(*output).is_some() {
                        remember(output.get())?;
                    }
                }
                Frame::EnumPack { .. } | Frame::Observe { .. } => {
                    return Err("tagged frame is outside numeric array proof");
                }
            }
            Operation::Frame
        } else if let Some(action) = aggregates.action(site) {
            let mut values = Vec::new();
            match action {
                Aggregate::Pack { output, words } => {
                    values.push(*output);
                    values.extend(words.iter().filter_map(|word| match word {
                        NativeAggregateWord::Field(value, _) => Some(*value),
                        _ => None,
                    }));
                }
                Aggregate::Copy { source, output } | Aggregate::Project { source, output, .. } => {
                    values.extend([*source, *output]);
                }
                Aggregate::Observe { source, .. } => values.push(*source),
            }
            if !values.into_iter().all(|value| {
                results::scalar_aggregate(plan, aggregates, value)
                    // Internal scalar FSM tags/payloads keep exactly the same
                    // same-plan owner and guarded common emitter as ordinary
                    // scalar execution. No managed word enters this component.
                    || (tree.is_some() && aggregates.slot(value).is_some())
            }) {
                return Err("numeric aggregate requires exact private flat scalar fields");
            }
            Operation::Aggregate
        } else if let Action::LibraryCall {
            operands,
            output,
            template,
        } = action
        {
            let call = calls
                .get(site)
                .and_then(Option::as_ref)
                .ok_or("missing selected factory")?;
            let is_length = call.supports_unsigned_matrix_length();
            let is_read = call.supports_unsigned_matrix_element();
            let is_write = call.supports_unsigned_matrix_replace();
            let is_append = call.supports_unsigned_matrix_append();
            if !(is_length || is_read || is_write || is_append) {
                return Err("unsupported selected library effect or operation");
            }
            let source = operands.first().ok_or("missing selected array")?;
            let field = provenance
                .array(source.get(), None)
                .ok_or("array read is not an invariant carrier field")?;
            if plan.values()[source.get() as usize].schema() != fields[field].schema() {
                return Err("array field schema differs from its selected use");
            }
            let at = arrays
                .iter()
                .position(|a| a.field == field)
                .ok_or("array outside carrier")?;
            let w =
                width(template.inputs()[0].body()).ok_or("unsupported numeric shape or width")?;
            if arrays[at].width != 0 && arrays[at].width != w {
                return Err("array field width differs");
            }
            arrays[at].width = w;
            arrays[at].witness = arrays[at].witness.min(site);
            arrays[at].writable |= is_write || is_append;
            arrays[at].appends |= is_append;
            if is_length {
                Operation::Length {
                    array: at,
                    output: output.get(),
                }
            } else if is_append {
                if operands.len() != 2
                    || plan.values()[operands[1].get() as usize]
                        .scalar_kind()
                        .and_then(|kind| kind.unsigned_maximum())
                        != Some(match w {
                            1 => u8::MAX as u64,
                            2 => u16::MAX as u64,
                            4 => u32::MAX as u64,
                            8 => u64::MAX,
                            _ => unreachable!(),
                        })
                {
                    return Err("unsupported append element kind");
                }
                Operation::Append {
                    array: at,
                    incoming: operands[1].get(),
                }
            } else if is_write {
                if operands.len() != 3
                    || plan.values()[operands[2].get() as usize].scalar_kind()
                        != Some(NativeScalarKind::U64)
                    || plan.values()[operands[1].get() as usize]
                        .scalar_kind()
                        .and_then(|k| k.unsigned_maximum())
                        .is_none()
                {
                    return Err("unsupported write operands");
                }
                Operation::Write {
                    array: at,
                    incoming: operands[1].get(),
                    index: operands[2].get(),
                }
            } else {
                if operands.len() != 2
                    || plan.values()[operands[1].get() as usize].scalar_kind()
                        != Some(NativeScalarKind::U64)
                {
                    return Err("unsupported read selector");
                }
                Operation::Read {
                    array: at,
                    index: operands[1].get(),
                    output: output.get(),
                }
            }
        } else {
            match action {
                Action::Scalar(Scalar::None | Scalar::Primitive { .. }) => continue,
                Action::Scalar(Scalar::Copy { output, .. })
                    if plan.values()[output.get() as usize].scalar_kind().is_some() =>
                {
                    continue;
                }
                Action::Scalar(Scalar::Copy { source, output })
                    if provenance.array(source.get(), None).is_some()
                        && provenance.array(source.get(), None)
                            == provenance.array(output.get(), None) =>
                {
                    Operation::ArrayCopy
                }
                _ => return Err("unsupported leaf action or managed escape"),
            }
        };
        actions.push((site, operation));
    }
    // A retained leaf with no selected content operation remains an opaque
    // immutable owner. It grants no native pointer/view and cannot be written.
    // Current-version proof still covers every structural forwarding edge.
    if arrays.iter().any(|array| array.witness == usize::MAX)
        && !numeric_fields::has_nested_managed(frames, frames.fields(tail.carrier()).unwrap())
    {
        // Opaque retention belongs to the new recursive managed-frame proof.
        // Keep the established flat-frame eligibility and callback controls.
        return Err("unwitnessed retained array");
    }
    if arrays.iter().any(|array| {
        array.witness == usize::MAX && (array.width != 0 || array.writable || array.appends)
    }) {
        return Err("unwitnessed retained array has content effects");
    }
    let cold_homes = if let Some(tree) = tree {
        for node in tree.nodes.iter().skip(1) {
            if let Some(fields) = numeric_fields::at(frames, node.tail.carrier().get()) {
                for (ordinal, field) in fields.iter().enumerate() {
                    if matches!(field.lane(), Lane::Managed { .. })
                        && provenance.array(node.tail.next().get(), Some(ordinal))
                            != provenance
                                .child_roots
                                .get(&(node.tail.carrier().get(), ordinal))
                                .copied()
                    {
                        return Err("numeric child recursive edge changes its array root");
                    }
                }
            }
        }
        versions::validate_tree(
            plan,
            tail,
            frames,
            &mut provenance,
            &sites,
            &actions,
            &arrays,
            &result,
            tree,
        )?
    } else {
        versions::validate(
            plan,
            tail,
            frames,
            &mut provenance,
            &sites,
            &actions,
            &arrays,
            &result,
        )?;
        versions::ColdHomes::default()
    };
    let append_limits = if let Some(tree) = tree {
        append::derive(plan, tree, &sites, &actions, &arrays)?
    } else {
        Vec::new()
    };
    let mut scalar_words = 0;
    let frame_words: Vec<FrameWords> = frame_values
        .into_iter()
        .map(|value| {
            let mut paths = Vec::new();
            scalar_paths(
                frames,
                frames.fields_at(value as usize).unwrap(),
                &mut Vec::new(),
                &mut scalar_words,
                &mut paths,
            );
            let fields = frames
                .fields_at(value as usize)
                .unwrap()
                .iter()
                .enumerate()
                .map(|(ordinal, field)| match field.lane() {
                    Lane::Scalar { .. } => paths
                        .iter()
                        .find(|(path, _)| path.as_slice() == [ordinal])
                        .map(|(_, slot)| *slot),
                    Lane::Managed { .. } | Lane::Composite { .. } | Lane::Tagged { .. } => None,
                })
                .collect();
            let flat_fields = numeric_fields::at(frames, value).unwrap();
            let array_fields = flat_fields
                .iter()
                .enumerate()
                .map(|(ordinal, field)| {
                    if matches!(field.lane(), Lane::Managed { .. }) {
                        let origin = provenance.array(value, Some(ordinal)).unwrap();
                        Some(arrays.iter().position(|a| a.field == origin).unwrap())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            let array_bindings = flat_fields
                .iter()
                .zip(&array_fields)
                .filter_map(|(field, array)| match (field.lane(), array) {
                    (Lane::Managed { at }, Some(array)) => Some((at, *array)),
                    _ => None,
                })
                .collect();
            FrameWords {
                value,
                fields,
                arrays: array_fields,
                array_bindings,
                maximum_arity: numeric_fields::maximum_arity(
                    frames,
                    frames.fields_at(value as usize).unwrap(),
                ),
                scalar_node_count: numeric_fields::logical_nodes(
                    frames,
                    frames.fields_at(value as usize).unwrap(),
                ),
                scalar_paths: paths,
            }
        })
        .collect();
    let words = scalar_words
        .checked_add(
            arrays
                .len()
                .checked_mul(4)
                .ok_or("numeric view size overflows")?,
        )
        .and_then(|n| {
            n.checked_add(if arrays.iter().any(|a| a.writable) {
                arrays.len()
            } else {
                0
            })
        })
        .and_then(|n| n.checked_add(arrays.iter().filter(|a| a.appends).count()))
        .ok_or("numeric word count overflows")?;
    let boundary_words = register_bound(plan, aggregates, &sites)?
        .checked_add(words)
        .and_then(|n| n.checked_add(3))
        .ok_or("numeric boundary register count overflows")?;
    let recursive = frame_words
        .iter()
        .any(|frame: &FrameWords| frame.scalar_paths.iter().any(|(path, _)| path.len() > 1));
    let boundary_work = if recursive {
        // One entry load and three possible spill templates per admitted home;
        // the cold leave additionally loads/exports each carrier scalar. The
        // fixed allowance covers guards, fuel settlement and branch templates.
        // Keep the original flat-frame price and its exact regression receipts.
        let carrier = frame_words
            .iter()
            .find(|frame| frame.value == tail.carrier().get())
            .ok_or("numeric boundary carrier frame missing")?;
        (boundary_words as u64)
            .checked_mul(4)
            .and_then(|n| n.checked_add(carrier.scalar_paths.len() as u64 * 2))
            .and_then(|n| n.checked_add(128))
            .ok_or("numeric boundary work overflows")?
            .max(super::tail::FIXED_OVERHEAD)
    } else {
        super::tail::FIXED_OVERHEAD
    };
    let carrier = frame_words
        .iter()
        .find(|frame| frame.value == tail.carrier().get())
        .ok_or("numeric scalar carrier recipe missing")?;
    let scalar_inputs = scalar_bindings(frames, carrier.value, &carrier.scalar_paths)?;
    let scalar_outputs = if let Some(value) = result.frame() {
        let returned = frame_words
            .iter()
            .find(|frame| frame.value == value)
            .ok_or("numeric scalar result recipe missing")?;
        scalar_bindings(frames, value, &returned.scalar_paths)?
    } else {
        Vec::new()
    };
    let mut cold_scalars = Vec::new();
    let mut cold_frames = Vec::new();
    for (header, values) in cold_homes.scalars {
        let mut bindings = Vec::new();
        let mut owners = Vec::new();
        for value in values {
            let frame = frame_words
                .iter()
                .find(|frame| frame.value == value)
                .ok_or("numeric cold frame lost its shadow recipe")?;
            bindings.extend(scalar_bindings(frames, value, &frame.scalar_paths)?);
            owners.extend(frame.array_bindings.iter().copied());
        }
        if !bindings.is_empty() {
            cold_scalars.push((header, bindings));
        }
        if !owners.is_empty() {
            cold_frames.push((header, owners));
        }
    }
    let nested_managed = frame_words.iter().any(|frame| {
        numeric_fields::has_nested_managed(frames, frames.fields_at(frame.value as usize).unwrap())
    });
    Ok(Proof {
        nested_managed,
        arrays,
        frames: frame_words,
        actions,
        sites: sites.into_iter().collect(),
        scalar_words,
        scalar_inputs,
        scalar_outputs,
        words,
        boundary_words,
        boundary_work,
        result,
        cold_aliases: cold_homes.aliases,
        cold_scalars,
        cold_frames,
        append_limits,
    })
}

/// Conservative static inventory, independent of backend instructions. Whole
/// aggregate spans may overcount a projection; no source scalar home is omitted.
fn register_bound(
    plan: &Plan,
    aggregates: &NativeScalarAggregates,
    sites: &BTreeSet<usize>,
) -> Result<usize, &'static str> {
    let mut words = BTreeSet::new();
    let span = |value, words: &mut BTreeSet<usize>| -> Result<(), &'static str> {
        let (base, count) = aggregates
            .slot(value)
            .ok_or("numeric aggregate span missing")?;
        words.extend(
            base..base
                .checked_add(count)
                .ok_or("numeric aggregate span overflows")?,
        );
        Ok(())
    };
    for &site in sites {
        if let Some(action) = aggregates.action(site) {
            match action {
                Aggregate::Pack {
                    output,
                    words: inputs,
                } => {
                    span(*output, &mut words)?;
                    for input in inputs {
                        match input {
                            NativeAggregateWord::Scalar(value) => {
                                words.insert(value.get() as usize);
                            }
                            NativeAggregateWord::Field(value, _) => span(*value, &mut words)?,
                            NativeAggregateWord::Constant(_) => {}
                        }
                    }
                }
                Aggregate::Copy { source, output } | Aggregate::Project { source, output, .. } => {
                    span(*source, &mut words)?;
                    span(*output, &mut words)?;
                }
                Aggregate::Observe { source, output, .. } => {
                    span(*source, &mut words)?;
                    words.insert(output.get() as usize);
                }
            }
        }
        let mut scalar = |value: mech_engine::native_artifact::NativeMatchValue| {
            if plan.values()[value.get() as usize].scalar_kind().is_some() {
                words.insert(value.get() as usize);
            }
        };
        match plan.blocks()[site].action() {
            Action::Scalar(Scalar::Copy { source, output }) => {
                scalar(*source);
                scalar(*output);
            }
            Action::Scalar(Scalar::Primitive {
                operands, output, ..
            })
            | Action::LibraryCall {
                operands, output, ..
            } => {
                for operand in operands {
                    scalar(*operand);
                }
                scalar(*output);
            }
            Action::Structural { output, .. } => scalar(*output),
            _ => {}
        }
        match plan.blocks()[site].terminator() {
            Term::Scalar(Branch::Branch { condition, .. }) => scalar(condition),
            Term::Scalar(Branch::LiteralBranch { left, right, .. }) => {
                scalar(left);
                scalar(right);
            }
            _ => {}
        }
    }
    Ok(words.len())
}
