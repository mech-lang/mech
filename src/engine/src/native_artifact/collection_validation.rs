//! Structural/type checks for canonical native collection builders. Body
//! definitions and builder lifetime are proved by the shared CFG analysis.
use super::iteration_validation::{ControlPlan, ControlType};
use super::{
    NativeControlLoopRef as Loop, NativeIterationAction as Action,
    NativeIterationValueScope as Scope,
};
use mech_core::SchemaBody;

const BAD: &str = "invalid native collection ownership or element types";
pub(super) fn validate(plan: &impl ControlPlan) -> Result<(), &'static str> {
    let definition = |value: super::NativeMatchValue| {
        plan.values()
            .get(value.get() as usize)
            .and_then(ControlType::definition)
            .ok_or(BAD)
    };
    for (id, collection) in plan.collections().iter().enumerate() {
        if collection
            .parent()
            .is_some_and(|id| id as usize >= plan.iteration_count())
            || collection
                .parent_collection()
                .is_some_and(|parent| parent as usize >= id)
            || !matches!(
                collection.kind(),
                crate::ComprehensionKind::Matrix | crate::ComprehensionKind::MatrixPreserveShape
            )
        {
            return Err(BAD);
        }
        let begin = plan.blocks().get(collection.begin() as usize).ok_or(BAD)?;
        let push = plan.blocks().get(collection.push() as usize).ok_or(BAD)?;
        let finish = plan.blocks().get(collection.finish() as usize).ok_or(BAD)?;
        if begin.iteration() != collection.parent()
            || finish.iteration() != collection.parent()
            || !matches!(begin.action(), Action::CollectionBegin { collection } if *collection as usize == id)
            || !matches!(finish.action(), Action::CollectionFinish { collection } if *collection as usize == id)
        {
            return Err(BAD);
        }
        let Action::CollectionPush {
            collection: owner,
            value,
        } = push.action()
        else {
            return Err(BAD);
        };
        if *owner as usize != id
            || push.iteration()
                != collection
                    .generators()
                    .last()
                    .copied()
                    .or(collection.parent())
            || plan.value_scopes().get(collection.output().get() as usize)
                != Some(&Scope::Immutable {
                    iteration: collection.parent(),
                })
        {
            return Err(BAD);
        }
        let SchemaBody::Matrix { element, .. } = definition(collection.output())?.body() else {
            return Err(BAD);
        };
        let yielded = definition(*value)?;
        if !yielded.dimension_parameters().is_empty()
            || element.as_ref() != yielded.body()
            || !crate::resident::general::native_comprehension_lowering::element_supported(element)
        {
            return Err(BAD);
        }
        let mut parent = collection.parent();
        for &generator in collection.generators() {
            if generator as usize >= plan.iteration_count() {
                return Err(BAD);
            }
            let Loop::Collection(data) = plan.iteration(generator as usize) else {
                return Err(BAD);
            };
            if data.collection() as usize != id || data.parent() != parent {
                return Err(BAD);
            }
            parent = Some(generator);
        }
        if collection.kind() == crate::ComprehensionKind::MatrixPreserveShape {
            if collection.generators().len() != 1 {
                return Err(BAD);
            }
            let Loop::Collection(data) = plan.iteration(collection.generators()[0] as usize) else {
                return Err(BAD);
            };
            if !matches!(definition(data.source())?.body(), SchemaBody::Matrix { .. }) {
                return Err(BAD);
            }
        }
    }
    for id in 0..plan.iteration_count() {
        let Loop::Collection(data) = plan.iteration(id) else {
            continue;
        };
        let collection = plan
            .collections()
            .get(data.collection() as usize)
            .ok_or(BAD)?;
        if !collection.generators().contains(&(id as u32)) {
            return Err(BAD);
        }
        let source = definition(data.source())?;
        let element = match source.body() {
            SchemaBody::Matrix { element, .. } | SchemaBody::Set { element, .. } => {
                element.as_ref()
            }
            SchemaBody::String => &SchemaBody::String,
            _ => return Err(BAD),
        };
        if let Some(parameter) = data.element() {
            let target = definition(parameter)?;
            if !target.dimension_parameters().is_empty()
                || target.body() != element
                || plan.value_scopes().get(parameter.get() as usize)
                    != Some(&Scope::Immutable {
                        iteration: Some(id as u32),
                    })
            {
                return Err(BAD);
            }
        }
    }
    Ok(())
}
