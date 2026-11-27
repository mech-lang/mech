//! Fixed trip envelopes for framed FSMs under an enclosing numeric generator.
//! This proves the original CFG, including its terminal tags and guarded counter
//! updates. It inserts no source iteration limit and grants no array authority.
use super::flow::Flow;
use super::*;
use mech_engine::native_artifact::{
    NativeBufferFrameOperation as Frame, NativeMatchValue as Value,
};

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub(in crate::native_artifact) struct StateTrips {
    pub iteration: u32,
    pub field: usize,
    pub guard: usize,
    pub tag: Option<u32>,
    pub maximum: u64,
}

#[derive(Clone, Copy)]
struct Constructor {
    site: usize,
    tag: Option<u32>,
    payload: Value,
}

struct Source<'a, 'p> {
    facts: &'a NativeProgramFacts<'p>,
    frames: &'a NativeBufferFrames,
    tail: &'a NativeTailLoop,
    iteration: u32,
}
impl Source<'_, '_> {
    fn plan(&self) -> &Plan {
        self.facts.plan()
    }

    fn copy(&self, site: usize, value: Value) -> Option<Value> {
        if let Some(action) = self.frames.action(site)
            && let Frame::Copy { source, output } = action.operation()
            && *output == value
        {
            return Some(*source);
        }
        if let Action::Scalar(Scalar::Copy { source, output }) = self.plan().blocks()[site].action()
            && *output == value
        {
            return Some(*source);
        }
        None
    }

    fn unique_site(&self, value: Value) -> Option<usize> {
        let [definition] = self.facts.definition_sites(value.get()).ok()? else {
            return None;
        };
        Some(definition.site()? as usize)
    }

    fn constant(&self, mut value: Value) -> Option<u64> {
        let mut seen = BTreeSet::new();
        loop {
            if !seen.insert(value.get()) {
                return None;
            }
            if let Some(c) = self.plan().constants().iter().find(|c| c.value() == value) {
                return match c.constant().data() {
                    mech_core::ValueData::U64(n) => Some(*n),
                    _ => None,
                };
            }
            value = self.copy(self.unique_site(value)?, value)?;
        }
    }

    fn constructors(&self, value: Value) -> Option<Vec<Constructor>> {
        self.constructors_inner(value, &mut BTreeSet::new())
    }

    fn constructors_inner(
        &self,
        value: Value,
        active: &mut BTreeSet<u32>,
    ) -> Option<Vec<Constructor>> {
        // In particular, forwarding the carrier is not evidence of progress.
        if value == self.tail.carrier() || !active.insert(value.get()) {
            return None;
        }
        let mut out = Vec::new();
        for definition in self.facts.definition_sites(value.get()).ok()? {
            let site = definition.site()? as usize;
            if let Some(source) = self.copy(site, value) {
                out.extend(self.constructors_inner(source, active)?);
                continue;
            }
            match self.frames.action(site)?.operation() {
                Frame::EnumPack {
                    output,
                    payload,
                    ordinal,
                } if *output == value => {
                    out.push(Constructor {
                        site,
                        tag: Some(*ordinal),
                        payload: *payload,
                    });
                }
                Frame::Pack { output } if *output == value => {
                    out.push(Constructor {
                        site,
                        tag: None,
                        payload: value,
                    });
                }
                _ => return None,
            }
        }
        active.remove(&value.get());
        (!out.is_empty()).then_some(out)
    }

    fn field(&self, mut payload: Value, ordinal: usize) -> Option<Value> {
        let mut seen = BTreeSet::new();
        loop {
            if !seen.insert(payload.get()) {
                return None;
            }
            let site = self.unique_site(payload)?;
            if let Some(source) = self.copy(site, payload) {
                payload = source;
                continue;
            }
            let Frame::Pack { output } = self.frames.action(site)?.operation() else {
                return None;
            };
            let Action::LibraryCall {
                operands,
                output: actual,
                ..
            } = self.plan().blocks()[site].action()
            else {
                return None;
            };
            return (*output == payload && *actual == payload)
                .then(|| operands.get(ordinal).copied())
                .flatten();
        }
    }

    fn primitive(&self, mut value: Value) -> Option<(NativeScalarOp, Vec<Value>)> {
        let mut seen = BTreeSet::new();
        loop {
            if !seen.insert(value.get()) {
                return None;
            }
            let site = self.unique_site(value)?;
            if self.plan().blocks()[site].iteration() != Some(self.iteration) {
                return None;
            }
            if let Some(source) = self.copy(site, value) {
                value = source;
                continue;
            }
            let Action::Scalar(Scalar::Primitive {
                op,
                operands,
                output,
            }) = self.plan().blocks()[site].action()
            else {
                return None;
            };
            return (*output == value).then(|| (*op, operands.to_vec()));
        }
    }
}

/// A complete one-loop framed child. Other complete phases retain their own
/// independently proved nested trip expressions; this does not replace them.
pub(in crate::native_artifact) fn fixed_state_trips(
    facts: &NativeProgramFacts<'_>,
    frames: &NativeBufferFrames,
    aggregates: &NativeScalarAggregates,
    tail: &NativeTailLoop,
) -> Result<StateTrips, &'static str> {
    let plan = facts.plan();
    let forest = Forest::derive(plan, tail)?;
    if forest.loops.len() != 1
        || !forest.reductions.is_empty()
        || !forest.maps.is_empty()
        || !forest.cold.is_empty()
        || !forest.stops.is_empty()
    {
        return Err("fixed FSM trip proof requires a complete single traversal");
    }
    let loop_ = &forest.loops[0];
    let source = Source {
        facts,
        frames,
        tail,
        iteration: loop_.iteration,
    };
    let next = source
        .constructors(tail.next())
        .ok_or("FSM next state lacks exact constructors")?;
    let seeds = source
        .constructors(tail.seed())
        .ok_or("FSM seed lacks exact constructors")?;
    let flow = Flow {
        plan,
        facts,
        frames,
        aggregates,
        forest: &forest,
        loop_,
    };
    let tags = seeds
        .iter()
        .chain(&next)
        .map(|c| c.tag)
        .collect::<BTreeSet<_>>();
    let recurring = tags
        .iter()
        .copied()
        .filter(|tag| flow.reaches(loop_.latch, None, tag.map(u64::from)))
        .collect::<Vec<_>>();
    let [tag] = recurring.as_slice() else {
        return Err("FSM requires exactly one recurring state");
    };
    let tag = *tag;
    let words = words::Words {
        plan,
        facts,
        frames,
        aggregates,
        tail,
        iteration: loop_.iteration,
    };
    for &site in &loop_.sites {
        let conditions = match plan.blocks()[site].terminator() {
            Term::Scalar(Branch::Branch { condition, .. }) => vec![condition],
            Term::Scalar(Branch::LiteralBranch { left, right, .. }) => vec![left, right],
            _ => Vec::new(),
        };
        for predicate in conditions {
            let Some((no, yes)) = facts.boolean_branch_edges(site as u32, predicate) else {
                continue;
            };
            let Some((op, args)) = source.primitive(predicate) else {
                continue;
            };
            if args.len() != 2
                || yes == no
                || !matches!(
                    op,
                    NativeScalarOp::U64Lt
                        | NativeScalarOp::U64Le
                        | NativeScalarOp::U64Gt
                        | NativeScalarOp::U64Ge
                )
            {
                continue;
            }
            let Some(field) = words.framed_carrier(args[0].get(), tag) else {
                continue;
            };
            let Some(other) = source.constant(args[1]) else {
                continue;
            };
            let guard = Guard {
                site,
                predicate: predicate.get(),
                field,
                offset: 0,
                other: args[1].get(),
                op,
                yes,
                no,
            };
            if let Some(maximum) = prove(&source, &flow, &words, &guard, tag, other, &next, &seeds)
            {
                return Ok(StateTrips {
                    iteration: loop_.iteration,
                    field,
                    guard: site,
                    tag,
                    maximum,
                });
            }
        }
    }
    Err("FSM has no guarded monotone counter with a fixed trip envelope")
}

fn prove(
    source: &Source<'_, '_>,
    flow: &Flow<'_, '_>,
    words: &words::Words<'_, '_>,
    guard: &Guard,
    tag: Option<u32>,
    other: u64,
    next: &[Constructor],
    seeds: &[Constructor],
) -> Option<u64> {
    let forward = matches!(guard.op, NativeScalarOp::U64Lt | NativeScalarOp::U64Le);
    let mut stride = 0;
    for state in next.iter().filter(|state| state.tag == tag) {
        if !flow.reaches(state.site, None, tag.map(u64::from)) {
            continue;
        }
        if !flow.edge_dominates(guard, state.site, tag.map(u64::from)) {
            return None;
        }
        let (op, args) = source.primitive(source.field(state.payload, guard.field)?)?;
        if args.len() != 2
            || words.framed_carrier(args[0].get(), tag) != Some(guard.field)
            || if forward {
                !matches!(op, NativeScalarOp::U64Add | NativeScalarOp::U64WrappingAdd)
            } else {
                !matches!(op, NativeScalarOp::U64Sub | NativeScalarOp::U64WrappingSub)
            }
        {
            return None;
        }
        let amount = source.constant(args[1])?;
        if amount == 0 {
            return None;
        }
        stride = stride.max(amount);
    }
    if stride == 0 {
        return None;
    }
    if forward {
        // Advancing by at least one bounds the number of active traversals.
        // The checked envelope also proves that a wrapping update cannot wrap.
        let padding = if guard.op == NativeScalarOp::U64Le {
            2
        } else {
            1
        };
        return other.checked_add(stride)?.checked_add(padding);
    }
    // Descending counters need an entry upper bound and a guard that prevents
    // underflow on every continuing update, including wrapping subtraction.
    let minimum = if guard.op == NativeScalarOp::U64Ge {
        other
    } else {
        other.checked_add(1)?
    };
    if minimum < stride {
        return None;
    }
    let mut start = None;
    for state in seeds.iter().filter(|state| state.tag == tag) {
        let n = source.constant(source.field(state.payload, guard.field)?)?;
        start = Some(start.map_or(n, |old: u64| old.max(n)));
    }
    // One final failed comparison plus one traversal of a terminal tag.
    start?.checked_add(2)
}
