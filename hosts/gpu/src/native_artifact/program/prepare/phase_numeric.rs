//! Bounded and partial numeric phases over flat private frames.
//! Recursive/captured-owner trees retain their independent must-current proof;
//! both proofs convert to the same native representation before code emission.
use super::numeric_phase::Coverage;
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
#[cfg(all(test, feature = "canonical-native-program-runtime"))]
thread_local! {
    static TEST_DECLINE_NESTED_PHASE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
#[cfg(all(test, feature = "canonical-native-program-runtime"))]
pub(in crate::native_artifact) fn test_decline_nested_phase() -> bool {
    TEST_DECLINE_NESTED_PHASE.with(std::cell::Cell::get)
}
#[cfg(all(test, feature = "canonical-native-program-runtime"))]
pub(in crate::native_artifact) fn test_without_nested_phase<T>(run: impl FnOnce() -> T) -> T {
    struct Reset(bool);
    impl Drop for Reset {
        fn drop(&mut self) {
            TEST_DECLINE_NESTED_PHASE.with(|flag| flag.set(self.0));
        }
    }
    let _reset = Reset(TEST_DECLINE_NESTED_PHASE.with(|flag| flag.replace(true)));
    run()
}

#[path = "numeric_exits.rs"]
mod exits;
#[path = "phase_numeric_results.rs"]
mod results;
#[path = "phase_numeric_versions.rs"]
mod versions;
pub(in crate::native_artifact) use results::ResultRecipe;

// Existing leaf and parent adapters use the common numeric representation.
pub(in crate::native_artifact) use super::numeric_plan::{
    Array, FrameWords, Operation, Plan as Proof,
};

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
    children: BTreeMap<(u32, usize), usize>,
    native_sites: Option<&'a BTreeSet<usize>>,
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
        if let Some(field) = field
            && let Some(origin) = self.children.get(&(value, field))
        {
            return Some(*origin);
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
        let definitions = self
            .facts
            .definition_sites(value)
            .ok()?
            .iter()
            .filter(|d| {
                self.native_sites
                    .is_none_or(|sites| d.site().is_some_and(|s| sites.contains(&(s as usize))))
            })
            .cloned()
            .collect::<Vec<_>>();
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
                        || call.supports_unsigned_matrix_append()
                        || call.supports_unsigned_matrix_append_le_bytes())
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
    visits: &[crate::native_artifact::iteration_abi::NativeIterationVisit],
) -> Result<Proof, &'static str> {
    derive_with_children(
        plan,
        tail,
        frames,
        aggregates,
        calls,
        visits,
        Coverage::Complete,
    )
    .or_else(|reason| {
        let cold = || {
            derive_with_children(
                plan,
                tail,
                frames,
                aggregates,
                calls,
                visits,
                Coverage::ScalarCold,
            )
        };
        let fueled = || derive_fueled(plan, tail, frames, aggregates, calls, visits);
        // A scalar enum state carries no buffer owner for a standalone numeric
        // leaf. Keep its captured reads in the enclosing phase when exact
        // per-visit funding can replace a missing static trip envelope. Other
        // ordinary child boundaries retain their existing selection priority.
        let scalar_state_child = plan.blocks()[tail.header() as usize]
            .iteration()
            .is_some_and(|root| {
                plan.iterations().iter().any(|child| {
                    matches!(child, NativeControlLoop::Tail(child)
                        if child.parent() == Some(root)
                            && aggregates.slot(child.carrier()).is_some()
                            && matches!(plan.values()[child.carrier().get() as usize].definition().body(), SchemaBody::Enum { .. }))
                })
            });
        let selected = if scalar_state_child {
            fueled().or_else(|_| cold())
        } else {
            cold().or_else(|_| fueled())
        };
        selected
            .or_else(|_| derive_hybrid(plan, tail, frames, aggregates, calls, visits))
            .or_else(|_| derive_interval(plan, tail, frames, aggregates, calls, visits))
            .map_err(|_| reason)
    })
}

pub(in crate::native_artifact) fn derive_fueled(
    plan: &Plan,
    tail: &NativeTailLoop,
    frames: &NativeBufferFrames,
    aggregates: &NativeScalarAggregates,
    calls: &[Option<NativePreparedLibraryCall>],
    visits: &[crate::native_artifact::iteration_abi::NativeIterationVisit],
) -> Result<Proof, &'static str> {
    derive_with_children(
        plan,
        tail,
        frames,
        aggregates,
        calls,
        visits,
        Coverage::Fueled,
    )
}

pub(in crate::native_artifact) fn derive_hybrid(
    plan: &Plan,
    tail: &NativeTailLoop,
    frames: &NativeBufferFrames,
    aggregates: &NativeScalarAggregates,
    calls: &[Option<NativePreparedLibraryCall>],
    visits: &[crate::native_artifact::iteration_abi::NativeIterationVisit],
) -> Result<Proof, &'static str> {
    derive_with_children(
        plan,
        tail,
        frames,
        aggregates,
        calls,
        visits,
        Coverage::Hybrid,
    )
}

pub(in crate::native_artifact) fn derive_interval(
    plan: &Plan,
    tail: &NativeTailLoop,
    frames: &NativeBufferFrames,
    aggregates: &NativeScalarAggregates,
    calls: &[Option<NativePreparedLibraryCall>],
    visits: &[crate::native_artifact::iteration_abi::NativeIterationVisit],
) -> Result<Proof, &'static str> {
    derive_with_children(
        plan,
        tail,
        frames,
        aggregates,
        calls,
        visits,
        Coverage::Intervals,
    )
}

fn derive_with_children(
    plan: &Plan,
    tail: &NativeTailLoop,
    frames: &NativeBufferFrames,
    aggregates: &NativeScalarAggregates,
    calls: &[Option<NativePreparedLibraryCall>],
    visits: &[crate::native_artifact::iteration_abi::NativeIterationVisit],
    mode: Coverage,
) -> Result<Proof, &'static str> {
    let fields = frames
        .fields(tail.carrier())
        .ok_or("carrier escapes or lacks an exact private frame")?;
    if fields
        .iter()
        .any(|f| matches!(f.lane(), Lane::Composite { .. } | Lane::Tagged { .. }))
    {
        return Err("numeric carrier requires flat fields");
    }
    let forest = match mode {
        Coverage::Complete => super::numeric_phase::Forest::derive(plan, tail)?,
        Coverage::ScalarCold => super::numeric_phase::Forest::with_cold_children(plan, tail)?,
        Coverage::Intervals => super::numeric_phase::Forest::intervals(plan, tail)?,
        Coverage::Fueled => super::numeric_phase::Forest::fueled(plan, tail)?,
        Coverage::Hybrid => super::numeric_phase::Forest::hybrid(plan, tail)?,
    };
    if mode == Coverage::Fueled && forest.loops.len() < 2 {
        return Err("checked child funding requires a nested Tail");
    }
    if mode == Coverage::Hybrid && (forest.loops.len() < 2 || forest.stops.is_empty()) {
        return Err("nested cold selection requires both Tail children and a cold boundary");
    }
    let nested = forest.loops.len() > 1 || !forest.cold.is_empty() || !forest.stops.is_empty();
    #[cfg(all(test, feature = "canonical-native-program-runtime"))]
    if nested && test_decline_nested_phase() {
        return Err("test declines nested phases to exercise ordinary parent segments");
    }
    let trips = if forest.loops.len() > 1 {
        forest.validate_child_actions(plan, frames, aggregates, calls)?;
        if forest.checked_children {
            Vec::new()
        } else {
            forest.bounded_children(plan, frames, aggregates, calls)?
        }
    } else {
        Vec::new()
    };
    let sites = forest.sites.clone();
    let facts = plan.facts();
    let mut provenance = Provenance {
        facts: &facts,
        frames,
        tail,
        calls,
        cache: BTreeMap::new(),
        active: BTreeSet::new(),
        children: BTreeMap::new(),
        native_sites: matches!(mode, Coverage::Intervals | Coverage::Hybrid).then_some(&sites),
    };
    for child in forest.loops.iter().skip(1) {
        let NativeControlLoop::Tail(child_tail) = &plan.iterations()[child.iteration as usize]
        else {
            unreachable!()
        };
        // Current buffer returns are bindings, not managed values produced at
        // the call boundary. Every action/result below still proves its exact
        // root, schema and functional version, including the zero-trip edge.
        if let Some(child_fields) = frames.fields(child_tail.carrier()) {
            for (ordinal, field) in child_fields.iter().enumerate() {
                if matches!(field.lane(), Lane::Managed { .. }) {
                    let origin = provenance
                        .array(child_tail.seed().get(), Some(ordinal))
                        .ok_or("child argument is not a phase buffer binding")?;
                    provenance
                        .children
                        .insert((child_tail.carrier().get(), ordinal), origin);
                }
            }
        }
    }
    // A child's next state can use a deeper child's current return. Register
    // the full input binding tree before checking those returns; an incomplete
    // tree cannot establish that a descendant changed its buffer identity.
    provenance.cache.clear();
    for child in forest.loops.iter().skip(1) {
        let NativeControlLoop::Tail(child_tail) = &plan.iterations()[child.iteration as usize]
        else {
            unreachable!()
        };
        if let Some(child_fields) = frames.fields(child_tail.carrier()) {
            for (ordinal, field) in child_fields.iter().enumerate() {
                if matches!(field.lane(), Lane::Managed { .. })
                    && provenance.array(child_tail.next().get(), Some(ordinal))
                        != provenance
                            .children
                            .get(&(child_tail.carrier().get(), ordinal))
                            .copied()
                {
                    return Err("child changes its buffer binding");
                }
            }
        }
    }
    let mut arrays = Vec::new();
    for (ordinal, field) in fields.iter().enumerate() {
        if matches!(field.lane(), Lane::Managed { .. }) {
            if sites.contains(&(tail.latch() as usize))
                && provenance.array(tail.next().get(), Some(ordinal)) != Some(ordinal)
            {
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
    let result = if sites.contains(&(tail.finish() as usize)) {
        results::derive(
            plan,
            tail,
            frames,
            aggregates,
            fields,
            &arrays,
            &mut provenance,
        )?
    } else {
        ResultRecipe::Unreachable
    };
    let mut actions = Vec::new();
    let mut frame_values = BTreeSet::from([tail.carrier().get()]);
    if let Some(value) = result.frame() {
        frame_values.insert(value);
    }
    for &site in &sites {
        if forest.stops.contains(&site) || forest.cold.iter().any(|child| child.initialize == site)
        {
            actions.push((site, Operation::Fallback));
            continue;
        }
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
                    .any(|f| matches!(f.lane(), Lane::Composite { .. } | Lane::Tagged { .. }))
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
                Frame::EnumPack { .. } | Frame::Observe { .. } => {
                    return Err("tagged frame outside flat numeric phase");
                }
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
                if nested {
                    aggregates.slot(value).is_some()
                } else {
                    results::scalar_aggregate(plan, aggregates, value)
                }
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
            let is_bulk = call.supports_unsigned_matrix_append_le_bytes();
            if !(is_length || is_read || is_write || is_append || is_bulk) {
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
            arrays[at].writable |= is_write || is_append || is_bulk;
            arrays[at].appends |= is_append || is_bulk;
            if is_length {
                Operation::Length {
                    array: at,
                    output: output.get(),
                }
            } else if is_bulk {
                if w != 1
                    || operands.len() != 3
                    || plan.values()[operands[1].get() as usize].scalar_kind()
                        != Some(NativeScalarKind::U64)
                    || plan.values()[operands[2].get() as usize]
                        .scalar_kind()
                        .and_then(|kind| kind.unsigned_maximum())
                        != Some(u8::MAX as u64)
                {
                    return Err("unsupported bulk append operands");
                }
                Operation::AppendLeBytes {
                    array: at,
                    incoming: operands[1].get(),
                    count: operands[2].get(),
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
    if arrays.iter().any(|array| array.witness == usize::MAX)
        && !actions.iter().any(|(_, action)| {
            matches!(action, Operation::Read { array, .. }
                if !arrays[*array].writable && !arrays[*array].appends)
        })
    {
        return Err("unwitnessed retained array");
    }
    if arrays.iter().any(|array| {
        array.witness == usize::MAX && (array.width != 0 || array.writable || array.appends)
    }) {
        return Err("unwitnessed retained array has content effects");
    }
    // A prefix without a native backedge retains the maintained acyclic parent
    // pieces. A Tail phase adds a second entry and cold export without keeping
    // any recurrence in native code. Storage effects retain their usual sites.
    if !forest.prepaid_sites.is_empty()
        && !exits::retains_native_backedge(plan, tail, provenance.facts, &forest)
    {
        return Err("acyclic interval retains the ordinary parent pieces");
    }
    let exits = exits::derive(
        plan,
        tail,
        frames,
        aggregates,
        &forest,
        &mut provenance,
        &arrays,
    )?;
    versions::validate(
        plan,
        tail,
        frames,
        &mut provenance,
        &sites,
        &actions,
        &arrays,
        &result,
        &forest,
        &exits,
    )?;
    let mut scalar_words = 0;
    let frame_words: Vec<FrameWords> = frame_values
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
                    Lane::Composite { .. } | Lane::Tagged { .. } => {
                        unreachable!("flat numeric proof")
                    }
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
    let mut words = scalar_words
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
    let phase = if nested {
        let fields = result
            .frame()
            .and_then(|value| frames.fields_at(value as usize))
            .map_or(0, |f| f.len());
        let boundary_work = super::numeric_phase::boundary_work(
            plan,
            aggregates,
            &forest,
            words
                .checked_add(forest.loops.len())
                .and_then(|n| {
                    n.checked_add(if forest.checked_children {
                        forest.loops.len() - 1
                    } else {
                        0
                    })
                })
                .and_then(|n| n.checked_add(usize::from(!exits.is_empty())))
                .ok_or("phase counter size overflows")?,
        )?;
        let schedule = super::numeric_phase::Schedule::derive(
            &forest,
            trips,
            visits,
            &actions,
            fields,
            words,
            boundary_work,
        )?;
        if super::numeric_phase::mutation_needs_funding(plan, &forest, &actions)?
            && !forest.checked_children
        {
            return Err("mutable children require current-account visit funding");
        }
        words = words
            .checked_add(schedule.words())
            .ok_or("phase counter size overflows")?;
        Some(schedule)
    } else {
        None
    };
    let (fresh_values, live_at_header) =
        exits::fresh_values(plan, tail, frames, aggregates, &forest, provenance.facts)?;
    let boundary_values = exits::boundary_values(
        plan,
        tail,
        provenance.facts,
        &forest,
        &frame_words,
        live_at_header,
    )?;
    Ok(Proof {
        arrays,
        frames: frame_words,
        actions,
        sites: sites.into_iter().collect(),
        prepaid_sites: forest.prepaid_sites.into_iter().collect(),
        scalar_words,
        words,
        result,
        phase,
        exits,
        fresh_values,
        boundary_values,
    })
}
