//! Escape analysis for private, fixed-width U64 tuples and enum state.
//! Canonical types/selected operations remain authoritative. Only an entire
//! connected nonescaping storage component may become native words.
use super::{
    NativeCollectionPrimitive as Primitive, NativeControlLoop, NativeIterationAction as Action,
    NativeMatchAction as Scalar, NativeMatchValue as Value, NativeProgramArtifactPlan as Plan,
    NativeStructuralObservation as Observation, NativeStructuralStep as Step,
};
use mech_core::{IntegerWidth, SchemaBody as Schema, ValueData};

#[derive(Clone, Debug, serde::Serialize)]
pub enum NativeScalarAggregateAction {
    Pack {
        output: Value,
        words: Box<[NativeAggregateWord]>,
    },
    Copy {
        source: Value,
        output: Value,
    },
    /// A guarded fixed-width slice. Used only by a separately derived private
    /// owner layout; an opaque word never becomes a canonical Value here.
    Project {
        source: Value,
        offset: usize,
        guards: Box<[(usize, u32)]>,
        output: Value,
    },
    Observe {
        source: Value,
        offset: usize,
        ordinal: Option<u32>,
        guards: Box<[(usize, u32)]>,
        output: Value,
    },
}
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub enum NativeAggregateWord {
    Scalar(Value),
    Field(Value, usize),
    Constant(u64),
}
super::program_image_proofs::image_enum!(NativeScalarAggregateAction {
    Pack { output: Value, words: Box<[NativeAggregateWord]> },
    Copy { source: Value, output: Value },
    Project { source: Value, offset: usize, guards: Box<[(usize, u32)]>, output: Value },
    Observe { source: Value, offset: usize, ordinal: Option<u32>, guards: Box<[(usize, u32)]>, output: Value },
});
impl super::program_image_proofs::ImageRead for NativeAggregateWord {
    fn read<'de, D: serde::Deserializer<'de>>(image: D) -> Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        enum Wire {
            Scalar(u32),
            Field(u32, usize),
            Constant(u64),
        }
        Ok(match <Wire as serde::Deserialize>::deserialize(image)? {
            Wire::Scalar(v) => Self::Scalar(Value(v)),
            Wire::Field(v, i) => Self::Field(Value(v), i),
            Wire::Constant(v) => Self::Constant(v),
        })
    }
}
#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct NativeScalarAggregates {
    pub(super) slots: Box<[Option<(usize, usize)>]>,
    pub(super) actions: Box<[Option<NativeScalarAggregateAction>]>,
    pub(super) constants: Box<[(Value, Box<[u64]>)]>,
    pub(super) words: usize,
}
crate::native_artifact::program_image_proofs::image_struct!(NativeScalarAggregates {
    slots: Box<[Option<(usize, usize)>]>, actions: Box<[Option<NativeScalarAggregateAction>]>, constants: Box<[(Value, Box<[u64]>)]>, words: usize,
});
impl NativeScalarAggregates {
    pub fn slot(&self, value: Value) -> Option<(usize, usize)> {
        self.slot_at(value.get() as usize)
    }
    pub fn slot_at(&self, index: usize) -> Option<(usize, usize)> {
        self.slots.get(index).copied().flatten()
    }
    pub fn action(&self, site: usize) -> Option<&NativeScalarAggregateAction> {
        self.actions.get(site).and_then(Option::as_ref)
    }
    pub fn constants(&self) -> &[(Value, Box<[u64]>)] {
        &self.constants
    }
    pub const fn words(&self) -> usize {
        self.words
    }
    pub fn value_count(&self) -> usize {
        self.slots.iter().filter(|slot| slot.is_some()).count()
    }
}
fn payload_words(schema: &Schema) -> Option<usize> {
    match schema {
        Schema::UnsignedInteger(IntegerWidth::W64) => Some(1),
        Schema::Tuple(items)
            if !items.is_empty()
                && items.len() <= 32
                && items
                    .iter()
                    .all(|s| *s == Schema::UnsignedInteger(IntegerWidth::W64)) =>
        {
            Some(items.len())
        }
        _ => None,
    }
}
fn words(schema: &Schema) -> Option<usize> {
    match schema {
        Schema::Tuple(_) => payload_words(schema),
        Schema::Enum { variants, .. } if !variants.is_empty() => variants
            .iter()
            .map(|v| payload_words(v.payload.as_ref()?))
            .collect::<Option<Vec<_>>>()?
            .into_iter()
            .max()?
            .checked_add(1),
        _ => None,
    }
}
fn constant_words(data: &ValueData, count: usize) -> Option<Box<[u64]>> {
    fn payload(data: &ValueData, result: &mut Vec<u64>) -> Option<()> {
        match data {
            ValueData::U64(value) => result.push(*value),
            ValueData::Tuple(items) => {
                for item in items.iter() {
                    let ValueData::U64(value) = item else {
                        return None;
                    };
                    result.push(*value);
                }
            }
            _ => return None,
        }
        Some(())
    }
    let mut result = Vec::new();
    if let ValueData::Enum(value) = data {
        result.push(u64::from(value.ordinal()));
        payload(value.payload()?, &mut result)?;
    } else {
        payload(data, &mut result)?;
    }
    if result.len() > count {
        return None;
    }
    result.resize(count, 0);
    Some(result.into())
}
fn pack(plan: &Plan, action: &Action) -> Option<(Value, Box<[NativeAggregateWord]>)> {
    let Action::LibraryCall {
        template,
        operands,
        output,
    } = action
    else {
        return None;
    };
    let definition = plan.values()[output.get() as usize].definition();
    if !definition.dimension_parameters().is_empty() || definition != template.output() {
        return None;
    }
    let count = words(definition.body())?;
    match (template.primitive(), definition.body()) {
        (Primitive::CompositePack(arity), Schema::Tuple(items))
            if arity == items.len()
                && operands.len() == arity
                && operands.iter().all(|v| {
                    plan.values()[v.get() as usize].scalar_kind()
                        == Some(super::NativeScalarKind::U64)
                }) =>
        {
            Some((
                *output,
                operands
                    .iter()
                    .copied()
                    .map(NativeAggregateWord::Scalar)
                    .collect(),
            ))
        }
        (Primitive::EnumPack, Schema::Enum { variants, .. }) if operands.len() == 2 => {
            // EnumPack's selected ordinal is ONE-based, whereas canonical
            // Enum payloads/structural observations use ZERO-based ordinals.
            let ordinal = plan.constants().iter().find_map(|entry| {
                if entry.value() != operands[0] {
                    return None;
                }
                let ValueData::Index(n) = entry.constant().data() else {
                    return None;
                };
                n.checked_sub(1).and_then(|n| u32::try_from(n).ok())
            })?;
            let payload = variants.get(ordinal as usize)?.payload.as_ref()?;
            if plan.values()[operands[1].get() as usize]
                .definition()
                .body()
                != payload
            {
                return None;
            }
            let mut result = vec![NativeAggregateWord::Constant(u64::from(ordinal))];
            if matches!(payload, Schema::Tuple(_)) {
                result.extend(
                    (0..payload_words(payload)?)
                        .map(|i| NativeAggregateWord::Field(operands[1], i)),
                );
            } else {
                result.push(NativeAggregateWord::Scalar(operands[1]));
            }
            result.resize(count, NativeAggregateWord::Constant(0));
            Some((*output, result.into()))
        }
        _ => None,
    }
}
fn observation(plan: &Plan, action: &Action) -> Option<NativeScalarAggregateAction> {
    let Action::Structural {
        source,
        path,
        observation,
        output,
    } = action
    else {
        return None;
    };
    let mut schema = plan.values()[source.get() as usize].definition().body();
    words(schema)?;
    let mut offset = 0;
    let mut guards = Vec::new();
    for step in path {
        schema = match (step, schema) {
            (Step::EnumPayload { ordinal }, Schema::Enum { variants, .. }) => {
                guards.push((offset, *ordinal));
                offset += 1;
                variants.get(*ordinal as usize)?.payload.as_ref()?
            }
            (Step::TupleItem { ordinal }, Schema::Tuple(items)) => {
                offset += *ordinal as usize;
                items.get(*ordinal as usize)?
            }
            _ => return None,
        };
    }
    let ordinal = match (observation, schema) {
        (Observation::U64, Schema::UnsignedInteger(IntegerWidth::W64)) => None,
        (Observation::EnumOrdinal { ordinal }, Schema::Enum { .. }) => Some(*ordinal),
        _ => return None,
    };
    Some(NativeScalarAggregateAction::Observe {
        source: *source,
        offset,
        ordinal,
        guards: guards.into(),
        output: *output,
    })
}
impl Plan {
    pub fn scalar_aggregates(&self) -> NativeScalarAggregates {
        self.facts().into_scalar_aggregates()
    }
}

pub(in crate::native_artifact) fn derive(
    plan: &Plan,
    facts: &crate::native_artifact::NativeProgramFacts<'_>,
) -> NativeScalarAggregates {
    let count = plan.values().len();
    let sizes = plan
        .values()
        .iter()
        .map(|ty| {
            ty.definition()
                .dimension_parameters()
                .is_empty()
                .then(|| words(ty.definition().body()))
                .flatten()
        })
        .collect::<Vec<_>>();
    let mut bad = sizes.iter().map(Option::is_none).collect::<Vec<_>>();
    let mut defined = vec![false; count];
    let mut edges = vec![Vec::<usize>::new(); count];
    let link = |a: Value, b: Value, edges: &mut Vec<Vec<usize>>| {
        let (a, b) = (a.get() as usize, b.get() as usize);
        edges[a].push(b);
        edges[b].push(a);
    };
    for index in 0..plan.transport_input_count() {
        bad[plan.transport_input_value(index).get() as usize] = true;
    }
    for index in 0..plan.transport_output_count() {
        bad[plan.transport_output_value(index).get() as usize] = true;
    }
    for entry in plan.constants() {
        let i = entry.value().get() as usize;
        defined[i] = sizes[i]
            .and_then(|n| constant_words(entry.constant().data(), n))
            .is_some();
    }
    // Expand every earlier fusion before inspecting uses. A removed
    // Matrix/Map consumer must never conceal an escaping aggregate read.
    let Ok(actions) = facts.actions() else {
        return NativeScalarAggregates::default();
    };
    for action in actions.iter() {
        match action {
            Action::Scalar(Scalar::Copy { source, output }) => {
                defined[output.get() as usize] = true;
                link(*source, *output, &mut edges);
            }
            Action::LibraryCall {
                operands, output, ..
            } => {
                if let Some((_, packed)) = pack(plan, action) {
                    defined[output.get() as usize] = true;
                    for word in packed.iter() {
                        if let NativeAggregateWord::Field(source, _) = word {
                            link(*source, *output, &mut edges);
                        }
                    }
                } else {
                    bad[output.get() as usize] = true;
                    for operand in operands {
                        bad[operand.get() as usize] = true;
                    }
                }
            }
            Action::Structural {
                source,
                observation: obs,
                ..
            } => {
                if observation(plan, action).is_none() {
                    bad[source.get() as usize] = true;
                }
                if let Observation::StringEqual { peer } = obs {
                    bad[peer.get() as usize] = true;
                }
            }
            Action::Scalar(Scalar::Primitive {
                operands, output, ..
            }) => {
                bad[output.get() as usize] = true;
                for operand in operands {
                    bad[operand.get() as usize] = true;
                }
            }
            Action::CollectionPush { value, .. } => bad[value.get() as usize] = true,
            _ => {}
        }
    }
    for control in plan.iterations() {
        match control {
            NativeControlLoop::Tail(t) => {
                defined[t.carrier().get() as usize] = true;
                link(t.seed(), t.carrier(), &mut edges);
                link(t.next(), t.carrier(), &mut edges);
                link(t.returned(), t.result(), &mut edges);
            }
            NativeControlLoop::Iteration(t) => {
                defined[t.carrier().get() as usize] = true;
                link(t.seed(), t.carrier(), &mut edges);
                link(t.next(), t.carrier(), &mut edges);
                link(t.carrier(), t.result(), &mut edges);
            }
            NativeControlLoop::Collection(t) => {
                bad[t.source().semantic_value().get() as usize] = true;
                if let Some(value) = t.element() {
                    bad[value.get() as usize] = true;
                }
            }
        }
    }
    for c in plan.collections() {
        bad[c.output().get() as usize] = true;
    }
    for (i, known) in defined.into_iter().enumerate() {
        if !known {
            bad[i] = true;
        }
    }
    let mut pending = bad
        .iter()
        .enumerate()
        .filter_map(|(i, b)| b.then_some(i))
        .collect::<Vec<_>>();
    while let Some(i) = pending.pop() {
        for &other in &edges[i] {
            if !bad[other] {
                bad[other] = true;
                pending.push(other);
            }
        }
    }
    let mut extra = 0;
    let base = count + plan.iterations().len();
    let slots = sizes
        .into_iter()
        .enumerate()
        .map(|(i, n)| {
            if bad[i] {
                return None;
            }
            let n = n?;
            let slot = (base + extra, n);
            extra += n;
            Some(slot)
        })
        .collect::<Box<[_]>>();
    let selected = |v: Value| slots[v.get() as usize].is_some();
    let actions = plan
        .blocks()
        .iter()
        .map(|block| {
            if let Some((output, words)) = pack(plan, block.action()) {
                return selected(output)
                    .then_some(NativeScalarAggregateAction::Pack { output, words });
            }
            match block.action() {
                Action::Scalar(Scalar::Copy { source, output }) if selected(*output) => {
                    Some(NativeScalarAggregateAction::Copy {
                        source: *source,
                        output: *output,
                    })
                }
                Action::Structural { source, .. } if selected(*source) => {
                    observation(plan, block.action())
                }
                Action::Initialize { iteration } | Action::Latch { iteration } => {
                    let init = matches!(block.action(), Action::Initialize { .. });
                    let (source, output) = match &plan.iterations()[*iteration as usize] {
                        NativeControlLoop::Tail(t) => {
                            (if init { t.seed() } else { t.next() }, t.carrier())
                        }
                        NativeControlLoop::Iteration(t) => {
                            (if init { t.seed() } else { t.next() }, t.carrier())
                        }
                        _ => return None,
                    };
                    selected(output).then_some(NativeScalarAggregateAction::Copy { source, output })
                }
                _ => None,
            }
        })
        .collect();
    let constants = plan
        .constants()
        .iter()
        .filter_map(|entry| {
            let (_, n) = slots[entry.value().get() as usize]?;
            Some((entry.value(), constant_words(entry.constant().data(), n)?))
        })
        .collect();
    NativeScalarAggregates {
        slots,
        actions,
        constants,
        words: extra,
    }
}
