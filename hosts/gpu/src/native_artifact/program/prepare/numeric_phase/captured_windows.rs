//! Current carrier coordinates for an immutable, checked affine read loop.
//! Reuse the existing resolver and selected-edge recurrence proof. These are
//! candidates for a fresh runtime gate, never saved read permissions.
use super::*;
use atoms::{Atom, Resolver};

pub(in crate::native_artifact) struct Read {
    pub site: usize,
    pub base: u32,
    pub array: usize,
    pub width: u8,
}

pub(in crate::native_artifact) struct Coordinates {
    pub induction: usize,
    pub limit: usize,
    pub step: u64,
    pub bases: Vec<(usize, usize)>,
}

pub(in crate::native_artifact) fn derive(
    facts: &NativeProgramFacts<'_>,
    frames: &NativeBufferFrames,
    aggregates: &NativeScalarAggregates,
    tail: &NativeTailLoop,
    reads: &[Read],
) -> Option<Coordinates> {
    let plan = facts.plan();
    if reads.len() < 2 {
        return None;
    }
    let forest = Forest::derive(plan, tail).ok()?;
    let root = forest.loops.first()?;
    let resolver = Resolver {
        plan,
        facts,
        frames,
        aggregates,
        forest: &forest,
    };
    let flow = flow::Flow {
        plan,
        facts,
        frames,
        aggregates,
        forest: &forest,
        loop_: root,
    };
    let fields = (0..frames.fields_at(tail.carrier().get() as usize)?.len()).collect::<Vec<_>>();
    let alternatives = resolver.alternatives(tail.next().get(), &fields)?;
    if alternatives.is_empty() {
        return None;
    }
    let (_, guards) = forest
        .candidate_guards(facts, frames, aggregates)
        .into_iter()
        .find(|(id, _)| *id == root.iteration)?;
    for guard in guards {
        if guard.op != NativeScalarOp::U64Le || guard.offset == 0 {
            continue;
        }
        let Some(Atom::Field(owner, limit)) = resolver.atom(Atom::Scalar(guard.other)) else {
            continue;
        };
        if owner != tail.carrier().get() || limit == guard.field {
            continue;
        }
        let valid = alternatives.iter().all(|(site, values)| {
            flow.edge_dominates(&guard, *site, None)
                && fields.iter().zip(values).all(|(&field, &value)| {
                    if field != guard.field {
                        return resolver.atom(value) == Some(Atom::Field(owner, field));
                    }
                    let Atom::Scalar(value) = value else {
                        return false;
                    };
                    let Some((op, operands)) = primitive(facts, value) else {
                        return false;
                    };
                    matches!(op, NativeScalarOp::U64Add | NativeScalarOp::U64WrappingAdd)
                        && operands.len() == 2
                        && resolver.atom(Atom::Scalar(operands[0]))
                            == Some(Atom::Field(owner, field))
                        && resolver.atom(Atom::Scalar(operands[1]))
                            == Some(Atom::Constant(guard.offset))
                })
        });
        if !valid {
            continue;
        }
        let bases = reads
            .iter()
            .map(|read| {
                if u64::from(read.width) > guard.offset
                    || !flow.edge_dominates(&guard, read.site, None)
                {
                    return None;
                }
                let (op, operands) = primitive(facts, read.base)?;
                if !matches!(op, NativeScalarOp::U64Add | NativeScalarOp::U64WrappingAdd)
                    || operands.len() != 2
                {
                    return None;
                }
                let a = resolver.atom(Atom::Scalar(operands[0]))?;
                let b = resolver.atom(Atom::Scalar(operands[1]))?;
                let current = Atom::Field(owner, guard.field);
                let captured = if a == current {
                    b
                } else if b == current {
                    a
                } else {
                    return None;
                };
                let Atom::Field(base_owner, base) = captured else {
                    return None;
                };
                (base_owner == owner && base != guard.field).then_some((read.array, base))
            })
            .collect::<Option<Vec<_>>>();
        if let Some(bases) = bases {
            return Some(Coordinates {
                induction: guard.field,
                limit,
                step: guard.offset,
                bases,
            });
        }
    }
    None
}

fn primitive(facts: &NativeProgramFacts<'_>, value: u32) -> Option<(NativeScalarOp, Vec<u32>)> {
    let plan = facts.plan();
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
