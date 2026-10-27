//! Constructor-private selected TextJoin consumer fusion. This changes only
//! the validated native compiler view, not ProgramArtifact or source semantics.
use super::iteration_validation::{ControlPlan, ControlType};
use super::{
    NativeCollectionResult as ResultKind, NativeIterationAction as Action,
    NativeMatchValue as Value, NativeProgramArtifactPlan as Plan, NativeTextJoinSink as Sink,
};

const BAD: &str = "invalid native selected TextJoin sink";

fn uses(plan: &impl ControlPlan) -> Result<Vec<usize>, &'static str> {
    super::guarded_histogram_query::semantic_use_counts(plan)
}

#[cfg(feature = "pure_text")]
fn selected(template: &super::NativeProgramLibraryTemplate) -> bool {
    template.primitive() == super::NativeCollectionPrimitive::TextJoin
        && template.implementation_memory()
            == mech_core::ImplementationMemoryClass::CanonicalFinalize
}
#[cfg(not(feature = "pure_text"))]
fn selected(_: &super::NativeProgramLibraryTemplate) -> bool {
    false
}

/// Reprove strategy, exact consumer, no escaping Matrix, types and legal
/// schedule on every opaque plan validation (including linked rederivation).
pub(super) fn validate(
    inputs: &super::fusion_semantics::ValidationInputs<'_, impl ControlPlan>,
) -> Result<(), &'static str> {
    let plan = inputs.plan();
    let counts = inputs.counts();
    for (id, data) in plan.collections().iter().enumerate() {
        let ResultKind::TextJoin(sink) = data.result() else {
            continue;
        };
        let plain_string = |value: Value| {
            plan.values()
                .get(value.get() as usize)
                .and_then(ControlType::definition)
                .is_some_and(|schema| {
                    schema.body() == &mech_core::SchemaBody::String
                        && schema.dimension_parameters().is_empty()
                })
        };
        let Some(consumer) = plan.blocks().get(sink.consumer() as usize) else {
            return Err(BAD);
        };
        if data.kind() != crate::ComprehensionKind::Matrix
            || counts.get(data.output().get() as usize) != Some(&1)
            || !selected(sink.template())
            || !plain_string(sink.separator())
            || !plain_string(sink.output())
            || !matches!(consumer.action(), Action::TextJoinFinish { collection } if *collection as usize == id)
            || !super::fusion_semantics::collection_consumer_adjacent(plan, data, sink.consumer())
        {
            return Err(BAD);
        }
        let Some(schema) = plan
            .values()
            .get(data.output().get() as usize)
            .and_then(ControlType::definition)
        else {
            return Err(BAD);
        };
        if !matches!(schema.body(), mech_core::SchemaBody::Matrix { element, .. } if element.as_ref() == &mech_core::SchemaBody::String)
        {
            return Err(BAD);
        }
        plan.library(
            consumer.origin(),
            sink.template(),
            &[data.output(), sink.separator()],
            sink.output(),
        )?;
    }
    Ok(())
}

pub(crate) fn lower(plan: &mut Plan) -> Result<(), &'static str> {
    let counts = uses(plan)?;
    for id in 0..plan.collections.len() {
        let data = &plan.collections[id];
        if data.kind() != crate::ComprehensionKind::Matrix
            || counts[data.output().get() as usize] != 1
        {
            continue;
        }
        let Some((consumer, sink)) = plan.blocks().iter().enumerate().find_map(|(site, block)| {
            let Action::LibraryCall {
                template,
                operands,
                output,
            } = block.action()
            else {
                return None;
            };
            if operands.len() != 2
                || operands[0] != data.output()
                || !selected(template)
                || !super::fusion_semantics::collection_consumer_adjacent(plan, data, site as u32)
            {
                return None;
            }
            Some((
                site,
                Sink {
                    consumer: site as u32,
                    template: template.clone(),
                    separator: operands[1],
                    output: *output,
                },
            ))
        }) else {
            continue;
        };
        let previous = plan.blocks[consumer].action.clone();
        plan.blocks[consumer].action = Action::TextJoinFinish {
            collection: id as u32,
        };
        plan.collections[id].result = ResultKind::TextJoin(sink);
        // In particular, the ordinary current-visit read of separator at
        // CollectionBegin proves immutable dominance. A later/lazy separator
        // declines optimization; never hoist its construction or errors.
        if plan.validate_current_visits().is_err() {
            plan.blocks[consumer].action = previous;
            plan.collections[id].result = ResultKind::Materialized;
        }
    }
    Ok(())
}

#[cfg(all(
    test,
    feature = "pure_text",
    feature = "source",
    feature = "artifact-codec",
    feature = "semantic-compiler",
    feature = "resident-artifact"
))]
mod tests {
    use super::*;
    use crate::native_artifact::{
        NativeIterationTerminator as Terminator, NativeMatchAction as Scalar,
        NativeMatchTerminator as Term,
    };
    use mech_syntax::document::{
        AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
        parse_canonical_document,
    };
    use std::sync::Arc;
    fn plans(source: &str) -> [Plan; 2] {
        let source = format!("+> text\n{source}");
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x4a4f494e), Revision(1), source.as_str()).unwrap(),
            ParseConfig::default(),
        );
        assert!(
            parsed.is_strictly_clean(),
            "{source}: {:?}",
            parsed.diagnostics
        );
        let mut catalog = mech_core::FunctionCatalogBuilder::new();
        crate::install_intrinsic_runtime(&mut catalog).unwrap();
        crate::install_intrinsic_compiler_runtime(&mut catalog).unwrap();
        crate::install_intrinsic_resident(&mut catalog).unwrap();
        crate::install_intrinsic_source(&mut catalog).unwrap();
        let catalog = Arc::new(catalog.build().unwrap());
        let artifact = crate::CanonicalSourceFrontend
            .compile_document_with_catalog(
                &DocumentSyntax::cast(parsed.syntax()).unwrap(),
                catalog.clone(),
            )
            .unwrap()
            .compile_artifact()
            .unwrap();
        let decoded = crate::decode_program_artifact_bytecode_v1(
            &crate::encode_program_artifact_bytecode_v1(&artifact).unwrap(),
        )
        .unwrap();
        [artifact, decoded].map(|artifact| {
            super::super::lower_native_program_artifact(&artifact, &catalog).unwrap()
        })
    }
    #[test]
    fn selected_join_sink_rejects_forged_elided_value_and_lifecycle() {
        for plan in plans("text/join([part | part <- source<string>], separator<string>)\n") {
            let (id, sink) = plan
                .collections()
                .iter()
                .enumerate()
                .find_map(|(id, c)| match c.result() {
                    ResultKind::TextJoin(s) => Some((id, s.clone())),
                    _ => None,
                })
                .expect("actual selected sink");
            plan.validate_current_visits().unwrap();
            let mut omitted = plan.clone();
            omitted.blocks[sink.consumer() as usize].action = Action::Scalar(Scalar::None);
            assert!(omitted.validate_current_visits().is_err());
            let mut fake = plan.clone();
            fake.collections[id].result = ResultKind::Materialized;
            assert!(fake.validate_current_visits().is_err());
            let mut stale = plan.clone();
            let semantic = stale.collections[id].output();
            let ResultKind::TextJoin(s) = &mut stale.collections[id].result else {
                unreachable!()
            };
            s.separator = semantic;
            assert!(stale.validate_current_visits().is_err());
            let mut escaping = plan.clone();
            let target = escaping
                .blocks
                .iter()
                .position(|b| b.terminator() == Terminator::Scalar(Term::Complete))
                .unwrap();
            escaping.blocks[target].action = Action::Scalar(Scalar::Copy {
                source: escaping.collections[id].output(),
                output: sink.output(),
            });
            assert!(escaping.validate_current_visits().is_err());
            let mut early = plan.clone();
            let begin = early.collections[id].begin();
            early.blocks[begin as usize].action = Action::TextJoinFinish {
                collection: id as u32,
            };
            assert!(early.validate_current_visits().is_err());
        }
    }
    #[test]
    fn selected_join_sink_declines_escape_late_separator_and_control_barrier() {
        for source in [
            "parts := [part | part <- source<string>]\ntext/join(parts, \"\")\nparts\n",
            "parts := [part | part <- source<string>]\nseparator := source[1u64]\ntext/join(parts, separator)\n",
            "parts := [part | part <- source<string>]\ntext/join(parts, \"\") + text/join(parts, \"-\")\n",
        ] {
            for plan in plans(source) {
                assert!(
                    plan.collections()
                        .iter()
                        .all(|c| matches!(c.result(), ResultKind::Materialized)),
                    "{source}"
                );
            }
        }
    }
}
