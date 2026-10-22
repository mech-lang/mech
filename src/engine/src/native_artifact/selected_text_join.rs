//! Complete-use proof for a selected String matrix consumed only by TextJoin.
//! This is metadata, not a new leaf or execution authority. A backend must
//! preserve every original visit and bind real owners or proved private range
//! geometry. Neither selected output is a fabricated canonical Matrix.
use super::{
    NativeCollectionPrimitive as Primitive, NativeIterationAction as Action,
    NativeIterationTerminator as Terminator, NativeMatchAction as Scalar,
    NativeMatchTerminator as Term, NativeMatchValue as Value, NativeProgramArtifactPlan as Plan,
    NativeProgramLibraryTemplate as Template, NativeScalarKind,
    fusion_semantics::{Read, reads, semantic_actions},
    iteration_validation::ControlPlan,
};
use mech_core::{DimensionExpr, ImplementationMemoryClass, IntegerWidth, SchemaBody};

/// An optional sole-use unit-U64 Range at its original semantic visit.
/// This describes immutable operand identities, not evaluated endpoints,
/// admitted storage, a canonical selector or permission to execute a visit.
#[derive(Clone, Debug, serde::Serialize)]
pub struct NativeSelectedTextJoinRange {
    producer: u32,
    start: Value,
    end: Value,
    inclusive: bool,
}
crate::native_artifact::program_image_proofs::image_struct!(NativeSelectedTextJoinRange {
    producer: u32,
    start: Value,
    end: Value,
    inclusive: bool,
});
impl NativeSelectedTextJoinRange {
    pub const fn producer(&self) -> u32 {
        self.producer
    }
    pub const fn start(&self) -> Value {
        self.start
    }
    pub const fn end(&self) -> Value {
        self.end
    }
    pub const fn inclusive(&self) -> bool {
        self.inclusive
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct NativeSelectedTextJoin {
    selection: u32,
    consumer: u32,
    source: Value,
    selector: Value,
    selected: Value,
    separator: Value,
    output: Value,
    range: Option<NativeSelectedTextJoinRange>,
}
crate::native_artifact::program_image_proofs::image_struct!(NativeSelectedTextJoin {
    selection: u32, consumer: u32, source: Value, selector: Value, selected: Value, separator: Value, output: Value, range: Option<NativeSelectedTextJoinRange>,
});
impl NativeSelectedTextJoin {
    pub const fn selection(&self) -> u32 {
        self.selection
    }
    pub const fn consumer(&self) -> u32 {
        self.consumer
    }
    pub const fn source(&self) -> Value {
        self.source
    }
    pub const fn selector(&self) -> Value {
        self.selector
    }
    /// Semantic identity only. A selected backend representation must NEVER
    /// install a fabricated canonical Matrix in this slot.
    pub const fn selected(&self) -> Value {
        self.selected
    }
    pub const fn separator(&self) -> Value {
        self.separator
    }
    pub const fn output(&self) -> Value {
        self.output
    }
    /// Ineligible range producers leave the ordinary selection/join proof
    /// intact. A backend must choose its representation at the Range visit.
    pub const fn range(&self) -> Option<&NativeSelectedTextJoinRange> {
        self.range.as_ref()
    }
}

fn string_matrix(body: &SchemaBody) -> bool {
    matches!(body, SchemaBody::Matrix { element, dimensions }
        if element.as_ref() == &SchemaBody::String && dimensions.len() == 2)
}

fn selected_access(template: &Template) -> bool {
    let [source, selector] = template.inputs() else {
        return false;
    };
    template.primitive() == Primitive::MatrixRangeAccess
        && template.implementation_memory()
            == ImplementationMemoryClass::PackedStringMatrixOrCanonicalFinalize
        && string_matrix(source.body())
        && string_matrix(template.output().body())
        && matches!(selector.body(), SchemaBody::Matrix { element, dimensions }
            if element.as_ref() == &SchemaBody::UnsignedInteger(IntegerWidth::W64)
                && dimensions.len() == 2)
}

fn selected_join(template: &Template) -> bool {
    let [source, separator] = template.inputs() else {
        return false;
    };
    template.primitive() == Primitive::TextJoin
        && template.implementation_memory() == ImplementationMemoryClass::CanonicalFinalize
        && string_matrix(source.body())
        && separator.body() == &SchemaBody::String
        && separator.dimension_parameters().is_empty()
        && template.output().body() == &SchemaBody::String
        && template.output().dimension_parameters().is_empty()
}

/// No other action, branch, loop transition or capture boundary may intervene.
/// In particular, do not hoist a late separator's construction or its errors.
fn adjacent(plan: &Plan, producer: usize, consumer: usize) -> bool {
    let parent = plan.blocks()[producer].iteration();
    let mut next = producer;
    for _ in 0..plan.blocks().len() {
        let Some(block) = plan.blocks().get(next) else {
            return false;
        };
        if block.iteration() != parent || (next != producer && plan.lifecycle_barrier(next)) {
            return false;
        }
        if next == consumer {
            return true;
        }
        if next != producer && !matches!(block.action(), Action::Scalar(Scalar::None)) {
            return false;
        }
        let Terminator::Scalar(Term::Jump(target)) = block.terminator() else {
            return false;
        };
        next = target as usize;
    }
    false
}

fn selected_range(
    plan: &Plan,
    selection: usize,
    selector: Value,
    access: &Template,
    uses: &[Vec<Read>],
) -> Option<NativeSelectedTextJoinRange> {
    if uses.get(selector.get() as usize)?.as_slice() != [Read::Action(selection as u32)] {
        return None;
    }
    let mut producers = plan
        .blocks()
        .iter()
        .enumerate()
        .filter_map(|(site, block)| match block.action() {
            Action::LibraryCall {
                template,
                operands,
                output,
            } if *output == selector => Some((site, template, operands)),
            _ => None,
        });
    let (producer, range, operands) = producers.next()?;
    let [start, end] = operands.as_ref() else {
        return None;
    };
    let inclusive = match range.primitive() {
        Primitive::RangeInclusive => true,
        Primitive::RangeExclusive => false,
        _ => return None,
    };
    let scalar = SchemaBody::UnsignedInteger(IntegerWidth::W64);
    if producers.next().is_some()
        || range.implementation_memory() != ImplementationMemoryClass::NoAdditionalScratch
        || range.inputs().len() != 2
        || range.output() != access.inputs().get(1)?
        || range
            .inputs()
            .iter()
            .zip(operands.iter())
            .any(|(schema, value)| {
                schema.body() != &scalar
                    || !schema.dimension_parameters().is_empty()
                    || plan.values().get(value.get() as usize).is_none_or(|ty| {
                        ty.scalar_kind() != Some(NativeScalarKind::U64) || ty.definition() != schema
                    })
            })
        || plan.values().get(selector.get() as usize)?.definition() != range.output()
        || plan.value_scopes().get(selector.get() as usize)
            != Some(&super::NativeIterationValueScope::Immutable {
                iteration: plan.blocks()[producer].iteration(),
            })
        || plan.lifecycle_barrier(producer)
        || !adjacent(plan, producer, selection)
    {
        return None;
    }
    // The same conservative shape grammar as the existing selected U64
    // range/source strategy. Exact witnesses still run at the original visit.
    let SchemaBody::Matrix {
        element,
        dimensions,
    } = range.output().body()
    else {
        return None;
    };
    if element.as_ref() != &scalar
        || dimensions.len() != 2
        || dimensions[0] != DimensionExpr::Constant(1)
        || !matches!(
            &dimensions[1],
            DimensionExpr::Constant(_) | DimensionExpr::Parameter(_)
        )
        || range.output().dimension_parameters().len() > 1
    {
        return None;
    }
    Some(NativeSelectedTextJoinRange {
        producer: u32::try_from(producer).ok()?,
        start: *start,
        end: *end,
        inclusive,
    })
}

impl Plan {
    /// Derive only from this opaque plan's complete original semantic reads.
    /// Repeated, observable, escaping and unsupported selections stay ordinary.
    /// The result alone grants no permission to skip validation or admission.
    pub fn selected_text_joins(&self) -> Vec<NativeSelectedTextJoin> {
        if self.validate_current_visits().is_err() {
            return Vec::new();
        }
        let Ok(actions) = semantic_actions(self) else {
            return Vec::new();
        };
        let Ok(uses) = reads(self, &actions) else {
            return Vec::new();
        };
        self.blocks()
            .iter()
            .enumerate()
            .filter_map(|(selection, block)| {
                let Action::LibraryCall {
                    template,
                    operands,
                    output: selected,
                } = block.action()
                else {
                    return None;
                };
                let [source, selector] = operands.as_ref() else {
                    return None;
                };
                if !selected_access(template) {
                    return None;
                }
                let [Read::Action(consumer)] = uses.get(selected.get() as usize)?.as_slice() else {
                    return None;
                };
                // Match the ACTUAL action, not a reconstructed virtual sink.
                // The two independent private strategies cannot both own it.
                let Action::LibraryCall {
                    template: join,
                    operands,
                    output,
                } = self.blocks().get(*consumer as usize)?.action()
                else {
                    return None;
                };
                let [input, separator] = operands.as_ref() else {
                    return None;
                };
                if input != selected
                    || !selected_join(join)
                    || template.output() != &join.inputs()[0]
                    || !adjacent(self, selection, *consumer as usize)
                {
                    return None;
                }
                Some(NativeSelectedTextJoin {
                    selection: selection as u32,
                    consumer: *consumer,
                    source: *source,
                    selector: *selector,
                    selected: *selected,
                    separator: *separator,
                    output: *output,
                    range: selected_range(self, selection, *selector, template, &uses),
                })
            })
            .collect()
    }
}

#[cfg(all(
    test,
    feature = "source",
    feature = "artifact-codec",
    feature = "semantic-compiler"
))]
#[path = "selected_text_join_tests.rs"]
mod tests;
