//! Same-plan proof for a packed numeric leaf under a suspended parent.
//! Exact frame recipes own the connected escape analysis. This adds identity
//! provenance for every managed field, including every alternative definition.
use mech_core::{IntegerWidth, SchemaBody};
use mech_engine::native_artifact::{
    NativeBufferFrameLane as Lane, NativeBufferFrameOperation as Frame, NativeBufferFrames,
    NativeControlLoop, NativeIterationAction as Action, NativeIterationTerminator as Term,
    NativeMatchAction as Scalar, NativeMatchTerminator as Branch, NativePreparedLibraryCall,
    NativeProgramArtifactPlan as Plan, NativeProgramFacts, NativeScalarKind, NativeTailLoop,
};
use std::collections::{BTreeMap, BTreeSet};
#[path = "numeric_results.rs"]
mod results;
#[path = "numeric_versions.rs"]
mod versions;
pub(in crate::native_artifact) use results::ResultRecipe;

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
    /// Relative native scalar lane, or None for an invariant managed field.
    pub fields: Vec<Option<usize>>,
    pub arrays: Vec<Option<usize>>,
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) enum Operation {
    Frame,
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
    pub result: ResultRecipe,
}
impl Proof {
    pub fn action(&self, site: usize) -> Option<&Operation> {
        self.actions
            .iter()
            .find(|(s, _)| *s == site)
            .map(|(_, action)| action)
    }
    pub fn fields(&self, value: u32) -> Option<&[Option<usize>]> {
        self.frames
            .iter()
            .find(|f| f.value == value)
            .map(|f| f.fields.as_slice())
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

fn width(body: &SchemaBody) -> Option<u8> {
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
    calls: &[Option<NativePreparedLibraryCall>],
) -> Result<Proof, &'static str> {
    let fields = frames
        .fields(tail.carrier())
        .ok_or("carrier escapes or lacks an exact private frame")?;
    if fields
        .iter()
        .any(|f| matches!(f.lane(), Lane::Composite { .. }))
    {
        return Err("numeric carrier requires flat fields");
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
    let result = results::derive(plan, tail, frames, fields, &arrays, &mut provenance)?;
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
                if fields
                    .iter()
                    .any(|f| matches!(f.lane(), Lane::Composite { .. }))
                {
                    return Err("numeric frame requires flat fields");
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
                Frame::Project { source, .. } => remember(source.get())?,
            }
            Operation::Frame
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
    let frame_words = frame_values
        .into_iter()
        .map(|value| {
            let fields = frames
                .fields_at(value as usize)
                .unwrap()
                .iter()
                .map(|field| match field.lane() {
                    Lane::Scalar { .. } => {
                        let at = scalar_words;
                        scalar_words += 1;
                        Some(at)
                    }
                    Lane::Managed { .. } => None,
                    Lane::Composite { .. } => unreachable!("flat numeric proof"),
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
    Ok(Proof {
        arrays,
        frames: frame_words,
        actions,
        sites: sites.into_iter().collect(),
        scalar_words,
        words,
        result,
    })
}
