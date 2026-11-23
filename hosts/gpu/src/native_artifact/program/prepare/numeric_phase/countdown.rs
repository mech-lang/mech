//! Bound a single root-traversal append by the current U64 countdown.
use super::{
    atoms::{Atom, Resolver},
    flow::Flow,
    *,
};

pub(in crate::native_artifact) fn unit_append_countdown(
    facts: &NativeProgramFacts<'_>,
    tail: &NativeTailLoop,
    frames: &NativeBufferFrames,
    aggregates: &NativeScalarAggregates,
    append: usize,
) -> Option<usize> {
    let plan = facts.plan();
    if matches!(
        plan.values()[tail.carrier().get() as usize]
            .definition()
            .body(),
        mech_core::SchemaBody::Enum { .. }
    ) {
        return None;
    }
    let forest = Forest::derive(plan, tail).ok()?;
    let root = forest
        .loops
        .iter()
        .find(|l| l.header == tail.header() as usize)?;
    // With only the declared latches cut, every other cycle was rejected.
    // The single append belongs to this traversal, outside all children.
    if !root.sites.contains(&append) {
        return None;
    }
    let flow = Flow {
        plan,
        facts,
        frames,
        aggregates,
        forest: &forest,
        loop_: root,
    };
    let resolver = Resolver {
        plan,
        facts,
        frames,
        aggregates,
        forest: &forest,
    };
    let guards = forest.candidate_guards(facts, frames, aggregates);
    guards
        .iter()
        .find(|(id, _)| *id == root.iteration)?
        .1
        .iter()
        .find_map(|guard| {
            if guard.op != NativeScalarOp::U64Ne
                || guard.offset != 0
                || resolver.atom(Atom::Scalar(guard.other))? != Atom::Constant(0)
                || !flow.edge_dominates(guard, append, None)
                || !flow.edge_dominates(guard, root.latch, None)
            {
                return None;
            }
            let source = Selected {
                flow: &flow,
                resolver: &resolver,
                guard,
                facts,
            };
            let next = Atom::Field(tail.next().get(), guard.field);
            let mut fuel = 256;
            source
                .prove(next, Expected::Decrement, &mut fuel)
                .then_some(guard.field)
        })
}

#[derive(Clone, Copy, Debug)]
enum Expected {
    Decrement,
    Current,
    One,
}

struct Selected<'a, 'b, 'p> {
    flow: &'a Flow<'b, 'p>,
    resolver: &'a Resolver<'b, 'p>,
    guard: &'a Guard,
    facts: &'a NativeProgramFacts<'p>,
}
impl Selected<'_, '_, '_> {
    fn prove(&self, atom: Atom, expected: Expected, fuel: &mut usize) -> bool {
        let Some(next) = fuel.checked_sub(1) else {
            return false;
        };
        *fuel = next;
        let tail = self.flow.tail();
        if atom == Atom::Field(tail.carrier().get(), self.guard.field) {
            return matches!(expected, Expected::Current);
        }
        let (value, field) = match atom {
            Atom::Constant(value) => return value == 1 && matches!(expected, Expected::One),
            Atom::Scalar(value) => {
                if let Some(Atom::Constant(n)) = self.resolver.atom(atom) {
                    return n == 1 && matches!(expected, Expected::One);
                }
                (value, None)
            }
            Atom::Field(value, field) => (value, Some(field)),
        };
        let plan = self.flow.plan;
        // The canonical iteration validator begins each traversal with only
        // its carrier and captured values defined. Root-local immutable words
        // must have a current definition at every use, including the latch.
        let scope = mech_engine::native_artifact::NativeIterationValueScope::Immutable {
            iteration: Some(self.flow.loop_.iteration),
        };
        if plan.value_scopes()[value as usize] != scope {
            return false;
        }
        let Ok(definitions) = self.facts.definition_sites(value) else {
            return false;
        };
        let mut found = false;
        for def in definitions {
            let Some(site) = def.site().map(|s| s as usize) else {
                return false;
            };
            if !self.flow.loop_.sites.contains(&site) {
                return false;
            }
            // The guard dominates the latch. A fresh root-local definition on
            // an incompatible branch cannot supply its current next value.
            if !self
                .flow
                .reaches_with(site, None, None, Some((self.guard.predicate, true)))
            {
                continue;
            }
            found = true;
            if let Some(input) = self.resolver.at(site, value, field) {
                if !self.prove(input, expected, fuel) {
                    return false;
                }
                continue;
            }
            if field.is_some() || !matches!(expected, Expected::Decrement) {
                return false;
            }
            let Action::Scalar(Scalar::Primitive {
                op: NativeScalarOp::U64WrappingSub,
                operands,
                output,
            }) = plan.blocks()[site].action()
            else {
                return false;
            };
            let [left, right] = operands.as_ref() else {
                return false;
            };
            if output.get() != value
                || !self.prove(Atom::Scalar(left.get()), Expected::Current, fuel)
                || !self.prove(Atom::Scalar(right.get()), Expected::One, fuel)
            {
                return false;
            }
        }
        found
    }
}
