//! Resolve exact immutable words without treating a phi or an older visit as a
//! current value. Alternatives keep their producer site for edge proofs.
use super::*;
use mech_engine::native_artifact::{
    NativeAggregateWord as Word, NativeBufferFrameOperation as Frame,
    NativeScalarAggregateAction as Aggregate,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Atom {
    Scalar(u32),
    Field(u32, usize),
    Constant(u64),
}
pub(super) struct Resolver<'a> {
    pub plan: &'a Plan,
    pub frames: &'a NativeBufferFrames,
    pub aggregates: &'a NativeScalarAggregates,
    pub forest: &'a Forest,
}
impl Resolver<'_> {
    pub fn atom(&self, atom: Atom) -> Option<Atom> {
        self.resolve(atom, &mut BTreeSet::new())
    }
    fn resolve(&self, atom: Atom, active: &mut BTreeSet<Atom>) -> Option<Atom> {
        if !active.insert(atom) {
            return None;
        }
        let result = (|| {
            let (value, field) = match atom {
                Atom::Constant(_) => return Some(atom),
                Atom::Scalar(v) => (v, None),
                Atom::Field(v, f) => (v, Some(f)),
            };
            if field.is_some()
                && self.forest.loops.iter().any(|l| {
                    let NativeControlLoop::Tail(t) = &self.plan.iterations()[l.iteration as usize]
                    else {
                        unreachable!()
                    };
                    t.carrier().get() == value
                })
            {
                return Some(atom);
            }
            if field.is_none()
                && let Some(c) = self
                    .plan
                    .constants()
                    .iter()
                    .find(|c| c.value().get() == value)
            {
                return match c.constant().data() {
                    mech_core::ValueData::U64(v) => Some(Atom::Constant(*v)),
                    mech_core::ValueData::U32(v) => Some(Atom::Constant(u64::from(*v))),
                    _ => Some(atom),
                };
            }
            let facts = self.plan.facts();
            let defs = facts.definition_sites(value).ok()?;
            if defs.is_empty() {
                return Some(atom);
            }
            let mut resolved = None;
            for d in defs {
                let Some(site) = d.site().map(|s| s as usize) else {
                    return Some(atom);
                };
                let next = self.at(site, value, field).unwrap_or(atom);
                let answer = if next == atom {
                    atom
                } else {
                    self.resolve(next, active)?
                };
                if resolved.is_some_and(|old| old != answer) {
                    return None;
                }
                resolved = Some(answer);
            }
            resolved
        })();
        active.remove(&atom);
        result
    }
    pub(super) fn at(&self, site: usize, value: u32, field: Option<usize>) -> Option<Atom> {
        if let Some(action) = self.aggregates.action(site) {
            match (action, field) {
                (Aggregate::Pack { output, words }, Some(f)) if output.get() == value => {
                    return words.get(f).map(|word| match word {
                        Word::Scalar(v) => Atom::Scalar(v.get()),
                        Word::Field(v, f) => Atom::Field(v.get(), *f),
                        Word::Constant(v) => Atom::Constant(*v),
                    });
                }
                (Aggregate::Copy { source, output }, Some(f)) if output.get() == value => {
                    return Some(Atom::Field(source.get(), f));
                }
                (
                    Aggregate::Project {
                        source,
                        offset,
                        output,
                        ..
                    },
                    Some(f),
                ) if output.get() == value => {
                    return Some(Atom::Field(source.get(), offset.checked_add(f)?));
                }
                (
                    Aggregate::Observe {
                        source,
                        offset,
                        ordinal: None,
                        output,
                        ..
                    },
                    None,
                ) if output.get() == value => return Some(Atom::Field(source.get(), *offset)),
                _ => {}
            }
        }
        if let Some(action) = self.frames.action(site) {
            match (action.operation(), field) {
                (Frame::Pack { output }, Some(f)) if output.get() == value => {
                    let Action::LibraryCall { operands, .. } = self.plan.blocks()[site].action()
                    else {
                        return None;
                    };
                    return Some(Atom::Scalar(operands.get(f)?.get()));
                }
                (Frame::Copy { source, output }, Some(f)) if output.get() == value => {
                    return Some(Atom::Field(source.get(), f));
                }
                (
                    Frame::Project {
                        source,
                        ordinal,
                        output,
                    },
                    None,
                ) if output.get() == value => return Some(Atom::Field(source.get(), *ordinal)),
                _ => {}
            }
        }
        if let Action::Scalar(Scalar::Copy { source, output }) = self.plan.blocks()[site].action()
            && output.get() == value
        {
            return Some(
                field.map_or(Atom::Scalar(source.get()), |f| Atom::Field(source.get(), f)),
            );
        }
        None
    }
    pub fn alternatives(&self, value: u32, fields: &[usize]) -> Option<Vec<(usize, Vec<Atom>)>> {
        self.alternatives_inner(value, fields, &mut BTreeSet::new())
    }
    fn alternatives_inner(
        &self,
        value: u32,
        fields: &[usize],
        active: &mut BTreeSet<u32>,
    ) -> Option<Vec<(usize, Vec<Atom>)>> {
        if !active.insert(value) {
            return None;
        }
        let facts = self.plan.facts();
        let mut out = Vec::new();
        for d in facts.definition_sites(value).ok()? {
            let site = d.site()? as usize;
            let source = match self.aggregates.action(site) {
                Some(Aggregate::Copy { source, output }) if output.get() == value => {
                    Some(source.get())
                }
                _ => match self.frames.action(site).map(|a| a.operation()) {
                    Some(Frame::Copy { source, output }) if output.get() == value => {
                        Some(source.get())
                    }
                    _ => match self.plan.blocks()[site].action() {
                        Action::Scalar(Scalar::Copy { source, output })
                            if output.get() == value =>
                        {
                            Some(source.get())
                        }
                        _ => None,
                    },
                },
            };
            if let Some(source) = source {
                out.extend(self.alternatives_inner(source, fields, active)?);
            } else {
                let words = fields
                    .iter()
                    .map(|&f| self.at(site, value, Some(f)).and_then(|a| self.atom(a)))
                    .collect::<Option<Vec<_>>>()?;
                out.push((site, words));
            }
        }
        active.remove(&value);
        (!out.is_empty()).then_some(out)
    }
}
