//! Bound payloads only when every definition of the full view transports a
//! read-only literal. A changed backing, partial pack, or runtime input loses
//! the fact even if it has the same element type and dimensions.
use super::{
    flow::Flow,
    layout::{ARRAY_OWNER, ARRAY_WORDS, Kind, Layout},
    recipes::Recipe,
};
use crate::native_artifact::program::codegen::cfg_cleanup::composed_reads::Literal;
use mech_core::ValueData;
use mech_engine::native_artifact::{
    NativeAggregateWord as Word, NativeControlLoop as Loop, NativeIterationAction as Action,
    NativeProgramArtifactPlan as Plan, NativeScalarAggregateAction as Aggregate,
};
use std::collections::{BTreeMap, BTreeSet};

enum Source {
    Literal(Literal),
    Copy(usize),
    Unknown,
}

#[derive(Default)]
pub(super) struct LiteralViews(BTreeMap<usize, Vec<Source>>);

fn literal(data: &ValueData) -> Option<Literal> {
    let ValueData::Matrix(matrix) = data else {
        return None;
    };
    let elements = matrix.elements();
    let retain_bytes =
        elements.len() <= 256 && std::env::var_os("MECH_COMPOSE_IMMUTABLE_COST_CODES").is_some();
    let mut bytes = retain_bytes.then(|| Vec::with_capacity(elements.len()));
    let mut maximum = 0;
    for index in 0..elements.len() {
        let value = elements.value_at(index)?;
        if let ValueData::U8(byte) = value {
            if let Some(bytes) = &mut bytes {
                bytes.push(byte);
            }
        } else {
            bytes = None;
        }
        let word = match value {
            ValueData::U8(value) => u64::from(value),
            ValueData::U16(value) => u64::from(value),
            ValueData::U32(value) => u64::from(value),
            ValueData::U64(value) => value,
            _ => return None,
        };
        maximum = maximum.max(word);
    }
    Some(Literal {
        length: elements.len() as u64,
        maximum,
        bytes: bytes.map(Into::into),
    })
}

impl LiteralViews {
    pub fn derive(plan: &Plan, layout: &Layout, flow: &Flow, recipes: &[Recipe]) -> Self {
        let mut views = Self::default();
        for port in plan.inputs() {
            let value = port.value().get();
            for &owner in &layout.ty(value).owners {
                views.push(layout.slot(value) + owner - ARRAY_OWNER, Source::Unknown);
            }
        }
        for constant in plan.constants() {
            let value = constant.value().get();
            let ty = layout.ty(value);
            let maximum = if matches!(ty.kind, Kind::Array { .. }) {
                literal(constant.constant().data())
            } else {
                None
            };
            for &owner in &ty.owners {
                views.push(
                    layout.slot(value) + owner - ARRAY_OWNER,
                    maximum
                        .as_ref()
                        .map_or(Source::Unknown, |value| Source::Literal(value.clone())),
                );
            }
        }
        let copy = |source: u32, field: usize| {
            if layout.ty(source).owners.contains(&(field + ARRAY_OWNER)) {
                Source::Copy(layout.slot(source) + field)
            } else {
                Source::Unknown
            }
        };
        for (site, recipe) in recipes.iter().enumerate() {
            if !flow.reachable[site] {
                continue;
            }
            for &value in &flow.defs[site] {
                for &owner in &layout.ty(value).owners {
                    let field = owner - ARRAY_OWNER;
                    let source = match recipe {
                        Recipe::Aggregate(Aggregate::Copy { source, output })
                            if output.get() == value =>
                        {
                            copy(source.get(), field)
                        }
                        Recipe::Aggregate(Aggregate::Project {
                            source,
                            output,
                            offset,
                            ..
                        }) if output.get() == value => copy(source.get(), field + offset),
                        Recipe::Aggregate(Aggregate::Pack { output, words })
                            if output.get() == value =>
                        {
                            match words.get(field..field + ARRAY_WORDS) {
                                Some([Word::Field(from, start), rest @ ..])
                                    if rest.iter().enumerate().all(|(i, word)| {
                                        matches!(word, Word::Field(v, lane) if v == from && *lane == *start + i + 1)
                                    }) =>
                                {
                                    copy(from.get(), *start)
                                }
                                _ => Source::Unknown,
                            }
                        }
                        Recipe::None => match plan.blocks()[site].action() {
                            action @ (Action::Initialize { iteration }
                            | Action::Latch { iteration }) => {
                                match &plan.iterations()[*iteration as usize] {
                                    Loop::Tail(tail) if tail.carrier().get() == value => {
                                        let source = if matches!(action, Action::Initialize { .. })
                                        {
                                            tail.seed()
                                        } else {
                                            tail.next()
                                        };
                                        copy(source.get(), field)
                                    }
                                    _ => Source::Unknown,
                                }
                            }
                            _ => Source::Unknown,
                        },
                        _ => Source::Unknown,
                    };
                    views.push(layout.slot(value) + field, source);
                }
            }
        }
        views
    }

    fn push(&mut self, slot: usize, source: Source) {
        self.0.entry(slot).or_default().push(source);
    }

    pub fn literal(&self, slot: usize) -> Option<Literal> {
        let mut literal: Option<Literal> = None;
        let mut pending = vec![slot];
        let mut seen = BTreeSet::new();
        while let Some(slot) = pending.pop() {
            if !seen.insert(slot) {
                continue;
            }
            for source in self.0.get(&slot)? {
                match source {
                    Source::Literal(value) => {
                        if let Some(known) = &mut literal {
                            if value.length != known.length {
                                return None;
                            }
                            known.maximum = known.maximum.max(value.maximum);
                            if known.bytes != value.bytes {
                                known.bytes = None;
                            }
                        } else {
                            literal = Some(value.clone());
                        }
                    }
                    Source::Copy(slot) => pending.push(*slot),
                    Source::Unknown => return None,
                }
            }
        }
        literal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload(bytes: &[u8]) -> Source {
        Source::Literal(Literal {
            length: bytes.len() as u64,
            maximum: u64::from(*bytes.iter().max().unwrap()),
            bytes: Some(bytes.into()),
        })
    }

    #[test]
    fn exact_bytes_survive_equivalent_inputs_and_a_copy_cycle() {
        let mut views = LiteralViews::default();
        views.push(0, Source::Copy(1));
        views.push(0, Source::Copy(2));
        views.push(1, Source::Copy(0));
        views.push(1, payload(&[0, 9]));
        views.push(2, payload(&[0, 9]));
        let literal = views.literal(0).unwrap();
        assert_eq!(literal.bytes.as_deref(), Some([0, 9].as_slice()));
        assert_eq!((literal.length, literal.maximum), (2, 9));
    }

    #[test]
    fn equal_length_and_maximum_do_not_imply_equal_bytes() {
        let mut views = LiteralViews::default();
        views.push(0, payload(&[0, 9]));
        views.push(0, payload(&[9, 0]));
        let literal = views.literal(0).unwrap();
        assert_eq!((literal.length, literal.maximum), (2, 9));
        assert!(literal.bytes.is_none());
    }

    #[test]
    fn absent_payload_does_not_recover_exact_bytes_at_a_merge() {
        let mut views = LiteralViews::default();
        views.push(0, payload(&[0, 9]));
        views.push(
            0,
            Source::Literal(Literal {
                length: 2,
                maximum: 15,
                bytes: None,
            }),
        );
        views.push(0, payload(&[0, 9]));
        let literal = views.literal(0).unwrap();
        assert_eq!(literal.maximum, 15);
        assert!(literal.bytes.is_none());
    }

    #[test]
    fn rebound_or_differently_sized_views_decline_literal_admission() {
        let mut views = LiteralViews::default();
        views.push(0, payload(&[0, 9]));
        views.push(0, Source::Copy(1));
        views.push(1, Source::Unknown);
        assert!(views.literal(0).is_none());
        views.push(2, payload(&[0, 9]));
        views.push(2, payload(&[0, 9, 0]));
        assert!(views.literal(2).is_none());
    }
}
