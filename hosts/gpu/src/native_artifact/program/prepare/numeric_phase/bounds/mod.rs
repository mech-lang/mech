//! Conservative trip envelopes for synchronous read-only children. These are
//! admission bounds only: successful execution debits actual loop traversals.
//! Unknown recurrences decline; no iteration cap is inserted into source code.
use super::atoms::{Atom, Resolver};
use super::flow::Flow;
use super::*;
mod upper;
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) enum Bound {
    Constant(u64),
    Length(u32),
    Root(usize),
    Add(Box<Bound>, Box<Bound>),
    Multiply(Box<Bound>, Box<Bound>),
    Maximum(Box<Bound>, Box<Bound>),
}
impl Bound {
    pub(super) fn plus(self, n: u64) -> Self {
        Self::add(self, Self::Constant(n))
    }
    pub(in crate::native_artifact) fn add(a: Self, b: Self) -> Self {
        if a == Self::Constant(0) {
            return b;
        }
        if b == Self::Constant(0) {
            return a;
        }
        if let (Self::Constant(a), Self::Constant(b)) = (&a, &b)
            && let Some(n) = a.checked_add(*b)
        {
            return Self::Constant(n);
        }
        Self::Add(Box::new(a), Box::new(b))
    }
    pub(in crate::native_artifact) fn multiply(a: Self, b: Self) -> Self {
        if a == Self::Constant(0) || b == Self::Constant(0) {
            return Self::Constant(0);
        }
        if a == Self::Constant(1) {
            return b;
        }
        if b == Self::Constant(1) {
            return a;
        }
        if let (Self::Constant(a), Self::Constant(b)) = (&a, &b)
            && let Some(n) = a.checked_mul(*b)
        {
            return Self::Constant(n);
        }
        Self::Multiply(Box::new(a), Box::new(b))
    }
    fn maximum(a: Self, b: Self) -> Self {
        if a == b {
            return a;
        }
        if let (Self::Constant(a), Self::Constant(b)) = (&a, &b) {
            return Self::Constant((*a).max(*b));
        }
        Self::Maximum(Box::new(a), Box::new(b))
    }
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields, rename = "mech.native.image.Trips")]
pub(in crate::native_artifact) struct Trips {
    pub iteration: u32,
    pub field: usize,
    pub guard: usize,
    pub tag: Option<u64>,
    pub bound: Bound,
}
impl Forest {
    pub(in crate::native_artifact) fn bounded_children(
        &self,
        plan: &Plan,
        frames: &NativeBufferFrames,
        aggregates: &NativeScalarAggregates,
        calls: &[Option<NativePreparedLibraryCall>],
    ) -> Result<Vec<Trips>, &'static str> {
        let resolver = Resolver {
            plan,
            frames,
            aggregates,
            forest: self,
        };
        let candidates = self.candidate_guards(plan, frames, aggregates);
        let mut trips = Vec::new();
        for loop_ in self.loops.iter().skip(1) {
            let flow = Flow {
                plan,
                frames,
                aggregates,
                forest: self,
                loop_,
            };
            let guards = &candidates
                .iter()
                .find(|(id, _)| *id == loop_.iteration)
                .unwrap()
                .1;
            let proof = guards
                .iter()
                .find_map(|guard| prove(&flow, &resolver, guard, &trips, calls));
            trips.push(proof.ok_or("numeric child has no proved bounded recurrence")?);
        }
        Ok(trips)
    }
}
fn primitive(plan: &Plan, value: u32) -> Option<(NativeScalarOp, Vec<u32>)> {
    let facts = plan.facts();
    let [definition] = facts.definition_sites(value).ok()? else {
        return None;
    };
    let Action::Scalar(Scalar::Primitive {
        op,
        operands,
        output,
    }) = plan.blocks()[definition.site()? as usize].action()
    else {
        return None;
    };
    (output.get() == value).then(|| (*op, operands.iter().map(|v| v.get()).collect()))
}
fn tagged(plan: &Plan, tail: &NativeTailLoop) -> bool {
    matches!(
        plan.values()[tail.carrier().get() as usize]
            .definition()
            .body(),
        mech_core::SchemaBody::Enum { .. }
    )
}
fn prove(
    flow: &Flow<'_>,
    resolver: &Resolver<'_>,
    guard: &Guard,
    prior: &[Trips],
    calls: &[Option<NativePreparedLibraryCall>],
) -> Option<Trips> {
    if !matches!(
        guard.op,
        NativeScalarOp::U64Lt | NativeScalarOp::U64Le | NativeScalarOp::U64Ne
    ) || guard.yes == guard.no
    {
        return None;
    }
    let tail = flow.tail();
    let is_tagged = tagged(flow.plan, tail);
    let fields = if is_tagged {
        vec![0, guard.field]
    } else {
        vec![guard.field]
    };
    let next = resolver.alternatives(tail.next().get(), &fields)?;
    let seed = resolver.alternatives(tail.seed().get(), &fields)?;
    let tag_at = |words: &[Atom]| -> Option<Option<u64>> {
        if is_tagged {
            match words[0] {
                Atom::Constant(tag) => Some(Some(tag)),
                _ => None,
            }
        } else {
            Some(None)
        }
    };
    let mut tags = BTreeSet::new();
    for (_, words) in next.iter().chain(&seed) {
        tags.insert(tag_at(words)?);
    }
    let active = tags
        .iter()
        .copied()
        .filter(|&tag| flow.reaches(flow.loop_.latch, None, tag))
        .collect::<Vec<_>>();
    let [tag] = active.as_slice() else {
        return None;
    };
    let tag = *tag;
    let coordinate = Atom::Field(tail.carrier().get(), guard.field);
    let mut stride = 0u64;
    for (site, words) in &next {
        if tag_at(words)? != tag {
            continue;
        } // Every other reachable tag was proved terminal above.
        let Atom::Scalar(step) = words[usize::from(is_tagged)] else {
            return None;
        };
        let (op, args) = primitive(flow.plan, step)?;
        if !matches!(op, NativeScalarOp::U64Add | NativeScalarOp::U64WrappingAdd)
            || args.len() != 2
            || resolver.atom(Atom::Scalar(args[0]))? != coordinate
            || !flow.edge_dominates(guard, *site, tag)
        {
            return None;
        }
        let Atom::Constant(amount) = resolver.atom(Atom::Scalar(args[1]))? else {
            return None;
        };
        if amount == 0 {
            return None;
        }
        stride = stride.max(amount);
    }
    if stride == 0 {
        return None;
    }
    // A counter can advance only on the selected comparison edge. The opposite
    // edge cannot continue the active state, even through an unchanged pack.
    let other = resolver.atom(Atom::Scalar(guard.other))?;
    let mut upper = upper::Upper {
        flow,
        resolver,
        prior,
        calls,
        tag,
        active: BTreeSet::new(),
    };
    if !upper.invariant_value(other) {
        return None;
    }
    let bound = match guard.op {
        NativeScalarOp::U64Lt | NativeScalarOp::U64Le => {
            let upper = upper.value(other, guard.site)?;
            // The checked affine guard implies field <= opposite. Every
            // continuing update advances by at least one. This conservative
            // padding also proves the largest wrapping step cannot wrap: the
            // emitted envelope must fit at entry. Keep unit-stride prices and
            // envelopes unchanged; the admission maximum is never the debit.
            upper.plus(stride.checked_add(if guard.op == NativeScalarOp::U64Le {
                2
            } else {
                1
            })?)
        }
        NativeScalarOp::U64Ne => {
            // A larger step can skip an equality target. A checked offset
            // changes that target, so retain the original exact unit proof.
            if stride != 1 || guard.offset != 0 {
                return None;
            }
            let mut count = None;
            for (_, words) in seed {
                if tag_at(&words)? != tag {
                    continue;
                }
                let Atom::Scalar(start) = words[usize::from(is_tagged)] else {
                    return None;
                };
                let (op, args) = primitive(flow.plan, start)?;
                if op != NativeScalarOp::U64WrappingSub
                    || args.len() != 2
                    || resolver.atom(Atom::Scalar(args[0]))? != other
                {
                    return None;
                }
                let n = upper.value(Atom::Scalar(args[1]), flow.loop_.initialize)?;
                count = Some(count.map_or(n.clone(), |old| Bound::maximum(old, n)));
            }
            count?.plus(2)
        }
        _ => unreachable!(),
    };
    // An own-carrier opposite operand must be invariant across all continuing
    // transitions. External immutable captures retain their source identity.
    if let Atom::Field(value, field) = other
        && value == tail.carrier().get()
    {
        if !upper.invariant(field, tag)? {
            return None;
        }
    }
    Some(Trips {
        iteration: flow.loop_.iteration,
        field: guard.field,
        guard: guard.site,
        tag,
        bound,
    })
}
