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
#[path = "numeric_results.rs"]
mod results;
#[path = "numeric_versions.rs"]
mod versions;
pub(in crate::native_artifact) use results::ResultRecipe;
pub(super) use results::scalar_aggregate;

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Array {
    pub field: usize,
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
    /// Recursive scalar leaves in field order. Paths retain exact structural
    /// identity; a composite span never hides a managed array or owner.
    pub scalar_paths: Vec<(Vec<usize>, usize)>,
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
        // Paths are prepared in depth-first field order. Count each structural
        // prefix once without allocating during invocation admission.
        let mut previous: &[usize] = &[];
        let mut count = 0;
        for (path, _) in &self.scalar_paths {
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
pub(in crate::native_artifact) struct Proof {
    pub arrays: Vec<Array>,
    pub frames: Vec<FrameWords>,
    pub actions: Vec<(usize, Operation)>,
    pub sites: Vec<usize>,
    pub scalar_words: usize,
    pub words: usize,
    /// Independently bounded source and shadow homes for entry/exit emission.
    pub boundary_words: usize,
    pub boundary_work: u64,
    pub result: ResultRecipe,
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
}
impl Provenance<'_, '_> {
    fn array(&mut self, value: u32, field: Option<usize>) -> Option<usize> {
        let key = (value, field);
        if value == self.tail.carrier().get() {
            let ordinal = field?;
            return matches!(
                self.frames.fields_at(value as usize)?.get(ordinal)?.lane(),
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
                    (Frame::Pack { output }, Some(ordinal)) if output.get() == value => {
                        let Action::LibraryCall { operands, .. } = action else {
                            return None;
                        };
                        self.array(operands.get(ordinal)?.get(), None)?
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
                        None,
                    ) if output.get() == value => self.array(source.get(), Some(*ordinal))?,
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
    let fields = frames
        .fields(tail.carrier())
        .ok_or("carrier escapes or lacks an exact private frame")?;
    if fields.iter().any(|f| {
        matches!(f.lane(), Lane::Tagged { .. })
            || (matches!(f.lane(), Lane::Composite { .. }) && !scalar_children(frames, *f))
    }) {
        return Err("numeric carrier composite requires scalar-only children");
    }
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
        if block.iteration() != Some(iteration) || plan.activation_boundary() == Some(site) {
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
    };
    let mut arrays = Vec::new();
    for (ordinal, field) in fields.iter().enumerate() {
        if matches!(field.lane(), Lane::Managed { .. }) {
            if provenance.array(tail.next().get(), Some(ordinal)) != Some(ordinal) {
                return Err("array field changes, swaps, or has unknown recursive forwarding");
            }
            arrays.push(Array {
                field: ordinal,
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
        fields,
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
                let fields = frames
                    .fields_at(value as usize)
                    .ok_or("frame lost exact fields")?;
                if fields.iter().any(|f| {
                    matches!(f.lane(), Lane::Tagged { .. })
                        || (matches!(f.lane(), Lane::Composite { .. })
                            && !scalar_children(frames, *f))
                }) {
                    return Err("numeric frame composite requires scalar-only children");
                }
                for (ordinal, field) in fields.iter().enumerate() {
                    if matches!(field.lane(), Lane::Managed { .. })
                        && !arrays
                            .iter()
                            .any(|a| provenance.array(value, Some(ordinal)) == Some(a.field))
                    {
                        return Err("frame has an unknown managed field");
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
            if !values
                .into_iter()
                .all(|value| results::scalar_aggregate(plan, aggregates, value))
            {
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
    if arrays.iter().any(|array| array.witness == usize::MAX) {
        return Err("unwitnessed retained array");
    }
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
            let array_fields = frames
                .fields_at(value as usize)
                .unwrap()
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
                .collect();
            FrameWords {
                value,
                fields,
                arrays: array_fields,
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
    Ok(Proof {
        arrays,
        frames: frame_words,
        actions,
        sites: sites.into_iter().collect(),
        scalar_words,
        words,
        boundary_words,
        boundary_work,
        result,
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
