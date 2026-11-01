//! Complete library dependencies of the immutable native execution image.
//! Scalar instructions already emitted into the Object need no resident factory.
use super::{
    NativeCollectionResult, NativeCollectionSource, NativeControlLoop, NativeIterationAction,
    NativeProgramArtifactPlan, NativeProgramLibraryTemplate,
};
use crate::OperationReference;
use std::collections::BTreeSet;

impl NativeProgramArtifactPlan {
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
