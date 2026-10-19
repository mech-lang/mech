//! A selected ordinary Histogram consumer of one complete String producer.
//! Only a private native compiler view changes; every canonical source visit
//! and every yielded String/error remains in the generated CFG.
use super::iteration_validation::{ControlPlan, ControlType};
use super::{
    NativeCollectionResult as ResultKind, NativeHistogramSink as Sink,
    NativeIterationAction as Action, NativeIterationTerminator as Terminator,
    NativeMatchAction as Scalar, NativeMatchTerminator as Term, NativeProgramArtifactPlan as Plan,
};
const BAD: &str = "invalid selected native full Histogram sink";

fn adjacent(
    plan: &impl ControlPlan,
    collection: &super::NativeProgramCollection,
    consumer: u32,
) -> bool {
    let mut next = collection.finish() as usize;
    for _ in 0..plan.blocks().len() {
        let Some(block) = plan.blocks().get(next) else {
            return false;
        };
        if block.iteration() != collection.parent() || plan.lifecycle_barrier(next) {
            return false;
        }
        if next == consumer as usize {
            return true;
        }
        if next != collection.finish() as usize
            && !matches!(block.action(), Action::Scalar(Scalar::None))
        {
            return false;
        }
        let Terminator::Scalar(Term::Jump(target)) = block.terminator() else {
            return false;
        };
        next = target as usize;
    }
    false
}
#[cfg(feature = "map_ops")]
fn selected(template: &super::NativeProgramLibraryTemplate) -> bool {
    template.primitive() == super::NativeCollectionPrimitive::StringHistogram
        && template.implementation_memory()
            == mech_core::ImplementationMemoryClass::CanonicalSortUnique
}
#[cfg(not(feature = "map_ops"))]
fn selected(_: &super::NativeProgramLibraryTemplate) -> bool {
    false
}

/// Removing a producer allocation must not remove a possible semantic shape
/// or cardinality error. These constructors accept every accepted length and
/// every actual distinct-key count, including zero; runtime separately proves
/// all finite-work-reachable backing/encoding arithmetic before first discard.
fn universal(input: &mech_core::Schema, output: &mech_core::Schema) -> bool {
    use mech_core::{CardinalitySpec, DimensionExpr as Expr, SchemaBody as Body};
    let Body::Matrix {
        element,
        dimensions,
    } = input.body()
    else {
        return false;
    };
    if element.as_ref() != &Body::String
        || dimensions.len() != 2
        || dimensions[0] != Expr::Constant(1)
        || input.dimension_parameters().len() != 1
    {
        return false;
    }
    let Expr::Parameter(parameter) = dimensions[1] else {
        return false;
    };
    let bound = &input.dimension_parameters()[0];
    parameter.get() == 0
        && bound.lifetime() == mech_core::DimensionLifetime::Turn
        && bound.lower_bound() == &Expr::Constant(0)
        && bound.upper_bound().is_none()
        && output.dimension_parameters().is_empty()
        && matches!(output.body(),Body::Map{key,value,cardinality}
            if key.as_ref()==&Body::String
            &&value.as_ref()==&Body::UnsignedInteger(mech_core::IntegerWidth::W64)
            &&cardinality==&CardinalitySpec::Dynamic{upper_bound:None})
}

pub(super) fn validate(plan: &impl ControlPlan) -> Result<(), &'static str> {
    let uses = super::guarded_histogram_query::semantic_use_counts(plan)?;
    for (id, collection) in plan.collections().iter().enumerate() {
        let ResultKind::Histogram(sink) = collection.result() else {
            continue;
        };
        let consumer = plan.blocks().get(sink.consumer() as usize).ok_or(BAD)?;
        if collection.kind() != crate::ComprehensionKind::Matrix
            || uses.get(collection.output().get() as usize) != Some(&1)
            || !selected(sink.template())
            || !adjacent(plan, collection, sink.consumer())
            || !matches!(consumer.action(),Action::HistogramFinish{collection}if *collection as usize==id)
        {
            return Err(BAD);
        }
        let input = plan
            .values()
            .get(collection.output().get() as usize)
            .and_then(ControlType::definition)
            .ok_or(BAD)?;
        let output = plan
            .values()
            .get(sink.output().get() as usize)
            .and_then(ControlType::definition)
            .ok_or(BAD)?;
        if !universal(input, output) {
            return Err(BAD);
        }
        plan.library(
            consumer.origin(),
            sink.template(),
            &[collection.output()],
            sink.output(),
        )?;
    }
    Ok(())
}

/// The guarded query-demand pass MUST run first. Its producer result is no
/// longer Materialized, so a full map cannot consume or obscure that proof.
pub(crate) fn lower(plan: &mut Plan) -> Result<(), &'static str> {
    let uses = super::guarded_histogram_query::semantic_use_counts(plan)?;
    for id in 0..plan.collections.len() {
        let collection = &plan.collections[id];
        if !matches!(collection.result(), ResultKind::Materialized)
            || collection.kind() != crate::ComprehensionKind::Matrix
            || uses[collection.output().get() as usize] != 1
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
            if operands.len() != 1
                || operands[0] != collection.output()
                || !selected(template)
                || !adjacent(plan, collection, site as u32)
            {
                return None;
            }
            Some((
                site,
                Sink {
                    consumer: site as u32,
                    template: template.clone(),
                    output: *output,
                },
            ))
        }) else {
            continue;
        };
        let previous = plan.blocks[consumer].action.clone();
        plan.blocks[consumer].action = Action::HistogramFinish {
            collection: id as u32,
        };
        plan.collections[id].result = ResultKind::Histogram(sink);
        if plan.validate_current_visits().is_err() {
            plan.blocks[consumer].action = previous;
            plan.collections[id].result = ResultKind::Materialized;
        }
    }
    Ok(())
}

#[cfg(all(
    test,
    feature = "map_ops",
    feature = "source",
    feature = "artifact-codec",
    feature = "semantic-compiler",
    feature = "resident-artifact"
))]
mod tests {
    use super::*;
    use mech_syntax::document::{
        AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
        parse_canonical_document,
    };
    use std::sync::Arc;
    fn plans(source: &str) -> [Plan; 2] {
        let source = format!("+> map\n{source}");
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x48495354), Revision(1), source.as_str()).unwrap(),
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
    fn full_histogram_requires_constructor_universal_source_and_map_schemas() {
        use mech_core::{CardinalitySpec, DimensionExpr as Expr, SchemaBody as Body};
        let input = |rows: u64, lower: u64, upper: Option<u64>| {
            mech_core::SchemaDraft {
                dimension_parameters: Box::new([mech_core::DimensionParameterDeclaration {
                    id: mech_core::DimensionParameterId::new(0),
                    origin: mech_core::DimensionParameterOrigin::Inferred,
                    lifetime: mech_core::DimensionLifetime::Turn,
                    lower_bound: Expr::Constant(lower),
                    upper_bound: upper.map(Expr::Constant),
                }]),
                body: Body::Matrix {
                    element: Box::new(Body::String),
                    dimensions: Box::new([
                        Expr::Constant(rows),
                        Expr::Parameter(mech_core::DimensionParameterId::new(0)),
                    ]),
                },
            }
            .finalize()
            .unwrap()
        };
        let output = |cardinality| {
            mech_core::SchemaDraft {
                dimension_parameters: Box::new([]),
                body: Body::Map {
                    key: Box::new(Body::String),
                    value: Box::new(Body::UnsignedInteger(mech_core::IntegerWidth::W64)),
                    cardinality,
                },
            }
            .finalize()
            .unwrap()
        };
        let any = output(CardinalitySpec::Dynamic { upper_bound: None });
        let source = input(1, 0, None);
        assert!(universal(&source, &any));
        for accepted in [0, 1, 2, 1000, u32::MAX as u64] {
            mech_core::shape_for_resolved_extents(&source, &[1, accepted]).unwrap();
        }
        for constrained in [input(2, 0, None), input(1, 1, None), input(1, 0, Some(5))] {
            assert!(!universal(&constrained, &any));
        }
        for lifetime in [
            mech_core::DimensionLifetime::CompileTime,
            mech_core::DimensionLifetime::Activation,
        ] {
            let constrained = mech_core::SchemaDraft {
                dimension_parameters: Box::new([mech_core::DimensionParameterDeclaration {
                    id: mech_core::DimensionParameterId::new(0),
                    origin: mech_core::DimensionParameterOrigin::Inferred,
                    lifetime,
                    lower_bound: Expr::Constant(0),
                    upper_bound: None,
                }]),
                body: source.body().clone(),
            }
            .finalize();
            if lifetime == mech_core::DimensionLifetime::CompileTime {
                assert!(matches!(
                    constrained,
                    Err(mech_core::SemanticModelError::CompileTimeDimensionParameterV1)
                ));
            } else {
                assert!(!universal(&constrained.unwrap(), &any));
            }
        }
        for constrained in [
            output(CardinalitySpec::Exact(Expr::Constant(0))),
            output(CardinalitySpec::Dynamic {
                upper_bound: Some(Expr::Constant(5)),
            }),
        ] {
            assert!(!universal(&source, &constrained));
        }
    }
    #[test]
    fn selected_full_histogram_has_real_map_result_and_rejects_forged_matrix_lifecycle() {
        for plan in plans("map/histogram([part | part <- source<[string]>])\n") {
            let (id, sink) = plan
                .collections()
                .iter()
                .enumerate()
                .find_map(|(id, c)| match c.result() {
                    ResultKind::Histogram(s) => Some((id, s.clone())),
                    _ => None,
                })
                .expect("actual full Histogram strategy");
            plan.validate_current_visits().unwrap();
            let mut missing = plan.clone();
            missing.blocks[sink.consumer() as usize].action = Action::Scalar(Scalar::None);
            assert!(missing.validate_current_visits().is_err());
            let mut fake = plan.clone();
            fake.collections[id].result = ResultKind::Materialized;
            assert!(fake.validate_current_visits().is_err());
            let mut wrong = plan.clone();
            wrong.blocks[sink.consumer() as usize].action = Action::HistogramFinish {
                collection: u32::MAX,
            };
            assert!(wrong.validate_current_visits().is_err());
            let mut early = plan.clone();
            let begin = early.collections[id].begin();
            early.blocks[begin as usize].action = Action::HistogramFinish {
                collection: id as u32,
            };
            assert!(early.validate_current_visits().is_err());
            let mut escaped = plan.clone();
            let output = escaped.collections[id].output();
            let site = escaped
                .blocks
                .iter()
                .position(|b| b.terminator() == Terminator::Scalar(Term::Complete))
                .unwrap();
            escaped.blocks[site].action = Action::Scalar(Scalar::Copy {
                source: output,
                output,
            });
            assert!(escaped.validate_current_visits().is_err());
        }
    }
    #[test]
    fn selected_full_histogram_declines_escaping_matrix_multiple_and_lazy_consumers() {
        for source in [
            "parts := [part | part <- source<string>]\nmap/histogram(parts)\nparts\n",
            "parts := [part | part <- source<string>]\nmap/histogram(parts)\nmap/histogram(parts)\n",
            "parts := [part | part <- source<string>]\nenabled<bool>?\n | true => map/histogram(parts)\n | false => map/histogram([\"\"])\n",
        ] {
            for plan in plans(source) {
                assert!(
                    plan.collections()
                        .iter()
                        .all(|c| !matches!(c.result(), ResultKind::Histogram(_))),
                    "{source}"
                );
            }
        }
    }
}
