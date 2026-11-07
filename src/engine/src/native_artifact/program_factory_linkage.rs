//! Complete library dependencies of the immutable native execution image.
//! Scalar instructions already emitted into the Object need no resident factory.
use super::{
    NativeCollectionResult, NativeCollectionSource, NativeControlLoop, NativeIterationAction,
    NativeProgramArtifactPlan, NativeProgramLibraryTemplate,
};
use crate::OperationReference;
use std::collections::BTreeSet;

impl NativeProgramArtifactPlan {
    /// Immutable selected call for one source site. Preparation and trusted
    /// Object restoration use the same association, including fused sources
    /// and sinks. This grants neither a current visit nor runtime admission.
    pub fn prepared_library_template(
        &self,
        site: usize,
    ) -> Result<Option<&NativeProgramLibraryTemplate>, &'static str> {
        let action = self
            .blocks()
            .get(site)
            .ok_or("prepared library site outside plan")?
            .action();
        let collection = |index: u32| {
            self.collections()
                .get(index as usize)
                .ok_or("prepared library collection outside plan")
        };
        let template = match action {
            NativeIterationAction::LibraryCall { template, .. } => template,
            NativeIterationAction::PrepareStringSpanSelector { collection: index }
            | NativeIterationAction::ProjectStringSpan { collection: index } => {
                let projection = collection(*index)?
                    .string_span_projection()
                    .ok_or("String span action lacks its selected proof")?;
                if matches!(
                    action,
                    NativeIterationAction::PrepareStringSpanSelector { .. }
                ) {
                    projection.range_template()
                } else {
                    projection.selection_template()
                }
            }
            NativeIterationAction::ProjectStringMapEntry { collection: index } => {
                collection(*index)?
                    .string_element_map_projection()
                    .ok_or("String map projection lacks its selected proof")?
                    .selection_template()
            }
            NativeIterationAction::HistogramQueryStep {
                collection: index,
                step,
            } => {
                let NativeCollectionResult::GuardedHistogramQuery(sink) =
                    collection(*index)?.result()
                else {
                    return Err("histogram query stage lacks selected proof");
                };
                return Ok(sink
                    .steps()
                    .get(*step as usize)
                    .ok_or("histogram query step outside proof")?
                    .template());
            }
            NativeIterationAction::HistogramFinish { collection: index } => {
                let NativeCollectionResult::Histogram(sink) = collection(*index)?.result() else {
                    return Err("histogram consumer lacks selected sink proof");
                };
                sink.template()
            }
            NativeIterationAction::TextJoinFinish { collection: index } => {
                let NativeCollectionResult::TextJoin(sink) = collection(*index)?.result() else {
                    return Err("text join consumer lacks selected sink proof");
                };
                sink.template()
            }
            NativeIterationAction::PrepareU64Range { iteration } => {
                let Some(NativeControlLoop::Collection(generator)) =
                    self.iterations().get(*iteration as usize)
                else {
                    return Err("range source is not a collection");
                };
                let NativeCollectionSource::U64Range(cursor) = generator.source() else {
                    return Err("range source lacks selected proof");
                };
                cursor.template()
            }
            _ => return Ok(None),
        };
        Ok(Some(template))
    }

    /// Every selected library template, including nested/fused metadata and
    /// paths that a particular input will not visit. This describes packaging
    /// only: original resource requirements and runtime admission are unchanged.
    pub fn library_operation_references(&self) -> Vec<OperationReference> {
        let mut operations = BTreeSet::new();
        let mut add = |template: &NativeProgramLibraryTemplate| {
            let key = template.selected_key();
            operations.insert(OperationReference {
                module_path: key.module_path.clone(),
                operation_name: key.operation_name.clone(),
            });
        };
        for block in self.blocks() {
            if let NativeIterationAction::LibraryCall { template, .. } = block.action() {
                add(template);
            }
        }
        for iteration in self.iterations() {
            match iteration {
                NativeControlLoop::Iteration(_) | NativeControlLoop::Tail(_) => {}
                NativeControlLoop::Collection(generator) => match generator.source() {
                    NativeCollectionSource::Value(_) => {}
                    NativeCollectionSource::U64Range(cursor) => add(cursor.template()),
                },
            }
        }
        for collection in self.collections() {
            if let Some(projection) = collection.string_span_projection() {
                add(projection.range_template());
                add(projection.selection_template());
            }
            if let Some(projection) = collection.string_element_map_projection() {
                add(projection.selection_template());
            }
            match collection.result() {
                NativeCollectionResult::Materialized => {}
                NativeCollectionResult::TextJoin(sink) => add(sink.template()),
                NativeCollectionResult::Histogram(sink) => add(sink.template()),
                NativeCollectionResult::GuardedHistogramQuery(sink) => {
                    for step in sink.steps() {
                        if let Some(template) = step.template() {
                            add(template);
                        }
                    }
                }
            }
        }
        operations.into_iter().collect()
    }
}
