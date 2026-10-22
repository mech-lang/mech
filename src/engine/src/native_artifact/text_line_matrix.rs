//! Complete-use proof for an unobservable text/lines matrix. A backend may
//! retain owner-bound line geometry instead of independent canonical Strings,
//! but must still execute the original visits, validate shape and pre-admit
//! all storage/work. No canonical Matrix may be fabricated in its value slot.
use super::{
    NativeBorrowedStringAction as Borrowed, NativeBorrowedStringRegion,
    NativeCollectionPrimitive as Primitive, NativeIterationAction as Action,
    NativeIterationValueScope as Scope, NativeMatchValue as Value,
    NativeProgramArtifactPlan as Plan, NativeScalarKind, NativeSelectedTextJoin,
    fusion_semantics::{Read, reads, semantic_actions},
    iteration_validation::ControlPlan,
};
use mech_core::{DimensionExpr, SchemaBody};

/// Constructor-private metadata for one root immutable line matrix. This is
/// not a canonical value, a transport representation or execution authority.
#[derive(Clone, Debug, serde::Serialize)]
pub struct NativeTextLineMatrixRegion {
    producer: u32,
    source: Value,
    matrix: Value,
    lengths: Box<[u32]>,
    borrowed: Box<[u32]>,
    selections: Box<[u32]>,
}
impl NativeTextLineMatrixRegion {
    pub const fn producer(&self) -> u32 {
        self.producer
    }
    pub const fn source(&self) -> Value {
        self.source
    }
    /// Semantic identity only; it does not name a fabricated Matrix handle.
    pub const fn matrix(&self) -> Value {
        self.matrix
    }
    pub fn lengths(&self) -> &[u32] {
        &self.lengths
    }
    /// Ordinals in the SAME plan's rederived borrowed String regions.
    pub fn borrowed_regions(&self) -> &[u32] {
        &self.borrowed
    }
    /// Original selected-matrix producer visits, not their join consumers.
    pub fn selections(&self) -> &[u32] {
        &self.selections
    }
}

fn line_matrix(body: &SchemaBody) -> bool {
    matches!(body, SchemaBody::Matrix { element, dimensions }
        if element.as_ref() == &SchemaBody::String && dimensions.len() == 2
            && dimensions[0] == DimensionExpr::Constant(1))
}

fn region(
    plan: &Plan,
    producer: usize,
    uses: &[Vec<Read>],
    borrowed: &[NativeBorrowedStringRegion],
    joins: &[NativeSelectedTextJoin],
) -> Option<NativeTextLineMatrixRegion> {
    let block = &plan.blocks()[producer];
    let Action::LibraryCall {
        template,
        operands,
        output: matrix,
    } = block.action()
    else {
        return None;
    };
    let [source] = operands.as_ref() else {
        return None;
    };
    let [input] = template.inputs() else {
        return None;
    };
    if block.iteration().is_some()
        || plan.lifecycle_barrier(producer)
        || template.primitive() != Primitive::TextLines
        || input.body() != &SchemaBody::String
        || !input.dimension_parameters().is_empty()
        || !line_matrix(template.output().body())
        || plan.value_scopes()[matrix.get() as usize] != (Scope::Immutable { iteration: None })
    {
        return None;
    }
    let mut lengths = Vec::new();
    let mut captures = Vec::new();
    let mut selections = Vec::new();
    for usage in uses.get(matrix.get() as usize)? {
        // Outputs, effects, generators, loop carriers and control conditions
        // are all observable/unsupported. Never infer closure from just the
        // apparent producer or an already optimized consumer's hidden reads.
        let Read::Action(site) = *usage else {
            return None;
        };
        let Action::LibraryCall {
            template: consumer,
            operands,
            output,
        } = plan.blocks().get(site as usize)?.action()
        else {
            return None;
        };
        if consumer.primitive() == Primitive::SequenceLength
            && operands.as_ref() == [*matrix]
            && consumer.inputs().len() == 1
            && consumer.inputs().first() == Some(template.output())
            && plan.values()[output.get() as usize].scalar_kind() == Some(NativeScalarKind::U64)
        {
            lengths.push(site);
            continue;
        }
        if let Some((ordinal, _)) = borrowed.iter().enumerate().find(|(_, region)| {
            region.actions().iter().any(|(selected, action)| {
                *selected == site
                    && matches!(action, Borrowed::MatrixElement { source, .. }
                        if source == matrix)
            })
        }) {
            if !captures.contains(&(ordinal as u32)) {
                captures.push(ordinal as u32);
            }
            continue;
        }
        if joins.iter().any(|join| {
            join.selection() == site
                && join.source() == *matrix
                && consumer.inputs().first() == Some(template.output())
        }) {
            selections.push(site);
            continue;
        }
        return None;
    }
    // A dead producer has no useful view consumers. Keep its ordinary errors
    // and admission instead of claiming an unrelated dead-code optimization.
    if lengths.is_empty() && captures.is_empty() && selections.is_empty() {
        return None;
    }
    Some(NativeTextLineMatrixRegion {
        producer: u32::try_from(producer).ok()?,
        source: *source,
        matrix: *matrix,
        lengths: lengths.into(),
        borrowed: captures.into(),
        selections: selections.into(),
    })
}

impl Plan {
    /// Rederive every consumer from this opaque plan's complete original
    /// semantic read inventory. Unsupported/escaping consumers stay ordinary.
    /// Backends must not use a view without also providing its checked leaf
    /// fallback, current-visit validation and owner-bound lifetime handling.
    pub fn text_line_matrix_regions(&self) -> Vec<NativeTextLineMatrixRegion> {
        if !self.blocks().iter().any(|block| {
            matches!(block.action(), Action::LibraryCall { template, .. }
                if template.primitive() == Primitive::TextLines)
        }) {
            return Vec::new();
        }
        if self.validate_current_visits().is_err() {
            return Vec::new();
        }
        let Ok(actions) = semantic_actions(self) else {
            return Vec::new();
        };
        let Ok(uses) = reads(self, &actions) else {
            return Vec::new();
        };
        let borrowed = self.borrowed_string_regions();
        let joins = self.selected_text_joins();
        (0..self.blocks().len())
            .filter_map(|site| region(self, site, &uses, &borrowed, &joins))
            .collect()
    }
}

#[cfg(all(test, feature = "source", feature = "semantic-compiler"))]
#[path = "text_line_matrix_tests.rs"]
mod tests;
