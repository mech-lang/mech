//! Same-plan facts for a uniform unsigned collection, without execution authority.
use super::{NativeProgramFacts, NativeProgramProducerRole as Role, NativeProgramReadRole as Read};
use crate::native_artifact::{
    NativeCollectionResult, NativeCollectionSource, NativeControlLoop,
    NativeIterationAction as Action, NativeIterationTerminator as Terminator,
    NativeMatchAction as Scalar, NativeMatchTerminator as Term, NativeMatchValue as Value,
    NativeProgramArtifactPlan as Plan, NativeScalarKind, NativeScalarOp,
};
use std::collections::BTreeSet;

/// A precise pure region: `yield | i <- 0..=limit, i < limit`. Both outer
/// words are already defined on every entry. This neither admits allocation
/// nor waives the original selected range's preparation/error boundary.
#[derive(Clone, Debug, serde::Serialize)]
pub struct NativeUniformWordCollection {
    collection: u32,
    iteration: u32,
    header: u32,
    word: Value,
    limit: Value,
    kind: NativeScalarKind,
}
impl NativeUniformWordCollection {
    pub const fn collection(&self) -> u32 {
        self.collection
    }
    pub const fn iteration(&self) -> u32 {
        self.iteration
    }
    pub const fn header(&self) -> u32 {
        self.header
    }
    pub const fn word(&self) -> Value {
        self.word
    }
    pub const fn limit(&self) -> Value {
        self.limit
    }
    pub const fn kind(&self) -> NativeScalarKind {
        self.kind
    }
}

impl NativeProgramFacts<'_> {
    pub fn uniform_unsigned_collections(&self) -> Vec<NativeUniformWordCollection> {
        self.plan()
            .iterations()
            .iter()
            .enumerate()
            .filter_map(|(index, control)| self.uniform_unsigned_collection(index, control))
            .collect()
    }

    fn uniform_unsigned_collection(
        &self,
        index: usize,
        control: &NativeControlLoop,
    ) -> Option<NativeUniformWordCollection> {
        let NativeControlLoop::Collection(generator) = control else {
            return None;
        };
        let NativeCollectionSource::U64Range(cursor) = generator.source() else {
            return None;
        };
        if cursor.template().primitive().name() != "range/inclusive" {
            return None;
        }
        let plan = self.plan();
        let definition = plan.collections().get(generator.collection() as usize)?;
        if definition.generators() != [index as u32]
            || !matches!(definition.result(), NativeCollectionResult::Materialized)
        {
            return None;
        }
        let [start, limit] = *cursor.operands();
        if !plan.constants().iter().any(|constant| {
            constant.value() == start
                && matches!(constant.constant().data(), mech_core::ValueData::U64(0))
        }) || !self.available_before(limit, generator.initialize())
        {
            return None;
        }
        let element = generator.element()?;
        let mut sites = BTreeSet::new();
        let mut site = generator.body();
        let mut saw_element = false;
        let (condition, yes, no) = loop {
            if !sites.insert(site) {
                return None;
            }
            let block = plan.blocks().get(site as usize)?;
            if block.iteration() != Some(index as u32) {
                return None;
            }
            match (block.action(), block.terminator()) {
                (Action::Scalar(Scalar::None), Terminator::Scalar(Term::Jump(next))) => site = next,
                (Action::CollectionElement { iteration }, Terminator::Scalar(Term::Jump(next)))
                    if *iteration == index as u32 && !saw_element =>
                {
                    saw_element = true;
                    site = next;
                }
                (
                    Action::Scalar(Scalar::Primitive {
                        op: NativeScalarOp::U64Lt,
                        operands,
                        output,
                    }),
                    Terminator::Scalar(Term::Branch { condition, yes, no }),
                ) if saw_element
                    && operands.as_ref() == [element, limit]
                    && *output == condition =>
                {
                    break (condition, yes, no);
                }
                _ => return None,
            }
        };
        let word = arm(
            plan,
            yes,
            generator.latch(),
            index as u32,
            generator.collection(),
            &mut sites,
        )??;
        if arm(
            plan,
            no,
            generator.latch(),
            index as u32,
            generator.collection(),
            &mut sites,
        )?
        .is_some()
            || !self.available_before(word, generator.initialize())
        {
            return None;
        }
        let latch = plan.blocks().get(generator.latch() as usize)?;
        if !matches!(latch.action(), Action::Latch { iteration } if *iteration == index as u32)
            || latch.terminator() != Terminator::Scalar(Term::Jump(generator.header()))
            || !sites.insert(generator.latch())
        {
            return None;
        }
        // No skipped body definition can feed a later outer operation,
        // collection source, lexical effect or published output.
        for value in [element, condition] {
            if self.read_roles(value).ok()?.iter().any(|read| {
                !matches!(read,
                    Read::Action(site) | Read::Terminator(site) if sites.contains(site))
            }) {
                return None;
            }
        }
        let kind = plan.values().get(word.get() as usize)?.scalar_kind()?;
        let body = plan
            .values()
            .get(definition.output().get() as usize)?
            .definition()
            .body();
        let mech_core::SchemaBody::Matrix {
            element,
            dimensions,
        } = body
        else {
            return None;
        };
        let expected = match kind {
            NativeScalarKind::U8 => mech_core::IntegerWidth::W8,
            NativeScalarKind::U16 => mech_core::IntegerWidth::W16,
            NativeScalarKind::U32 => mech_core::IntegerWidth::W32,
            NativeScalarKind::U64 => mech_core::IntegerWidth::W64,
            _ => return None,
        };
        if dimensions.len() != 2
            || element.as_ref() != &mech_core::SchemaBody::UnsignedInteger(expected)
        {
            return None;
        }
        Some(NativeUniformWordCollection {
            collection: generator.collection(),
            iteration: index as u32,
            header: generator.header(),
            word,
            limit,
            kind,
        })
    }

    fn available_before(&self, value: Value, frontier: u32) -> bool {
        let Ok([definition]) = self.definition_sites(value.get()) else {
            return false;
        };
        match definition.site() {
            None => matches!(definition.role(), Role::Input | Role::Constant),
            Some(site) => self.strictly_dominates(site, frontier).unwrap_or(false),
        }
    }
}

fn arm(
    plan: &Plan,
    mut site: u32,
    latch: u32,
    iteration: u32,
    collection: u32,
    sites: &mut BTreeSet<u32>,
) -> Option<Option<Value>> {
    let mut word = None;
    while site != latch {
        if !sites.insert(site) {
            return None;
        }
        let block = plan.blocks().get(site as usize)?;
        if block.iteration() != Some(iteration) {
            return None;
        }
        match block.action() {
            Action::Scalar(Scalar::None) => {}
            Action::CollectionPush {
                collection: actual,
                value,
            } if *actual == collection && word.is_none() => word = Some(*value),
            _ => return None,
        }
        let Terminator::Scalar(Term::Jump(next)) = block.terminator() else {
            return None;
        };
        site = next;
    }
    Some(word)
}
