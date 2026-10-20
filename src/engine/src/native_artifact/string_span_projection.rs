//! A constructor-private storage strategy for one selected range/String
//! selection and one already-proved streaming sink. Every original visit,
//! generated arithmetic operation and branch remains in the same native CFG.
use super::iteration_validation::ControlPlan;
#[cfg(any(feature = "pure_text", feature = "map_ops"))]
use super::iteration_validation::ControlType;
#[cfg(any(feature = "pure_text", feature = "map_ops"))]
use super::{
    NativeCollectionResult as ResultKind, NativeIterationTerminator as Terminator,
    NativeMatchAction as Scalar, NativeMatchTerminator as Term, NativeMatchValue as Value,
};
use super::{
    NativeIterationAction as Action, NativeProgramArtifactPlan as Plan,
    NativeStringSpanProjection as Projection,
};

const BAD: &str = "invalid native selected String span projection";

#[cfg(any(feature = "pure_text", feature = "map_ops"))]
fn adjacent(plan: &impl ControlPlan, from: u32, to: u32) -> bool {
    let Some(first) = plan.blocks().get(from as usize) else {
        return false;
    };
    let scope = first.iteration();
    let mut site = from;
    for _ in 0..plan.blocks().len() {
        let Some(block) = plan.blocks().get(site as usize) else {
            return false;
        };
        if block.iteration() != scope || plan.lifecycle_barrier(site as usize) {
            return false;
        }
        if site == to {
            return site != from;
        }
        if site != from && !matches!(block.action(), Action::Scalar(Scalar::None)) {
            return false;
        }
        let Terminator::Scalar(Term::Jump(next)) = block.terminator() else {
            return false;
        };
        site = next;
    }
    false
}

#[cfg(all(
    test,
    feature = "pure_text",
    feature = "map_ops",
    feature = "convert",
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
        let source = format!("+> text\n+> map\n{source}");
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x5350414e), Revision(1), source.as_str()).unwrap(),
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
    fn string_span_projection_is_generic_selected_and_rederived_for_all_sink_kinds() {
        for source in [
            "text/join([(alphabet<string>)[i..=i] | i <- 1u64..=limit<u64>], separator<string>)\n",
            "map/histogram([(alphabet<string>)[i..i + 1u64] | i <- 1u64..=limit<u64>])\n",
            "lookup(alphabet<string>, limit<u64>, needle<string>) => (out<u64>) :=\n  words := [alphabet[i..=i] | i <- 1u64..=limit]\n  counts := map/histogram(words)\n  keys<{string}> := map/keys(counts)\n  count := (needle ∈ keys)?\n    | true => counts{needle}\n    | false => 0u64.\n  out := count.\nlookup(alphabet<string>, limit<u64>, needle<string>)\n",
        ] {
            for plan in plans(source) {
                assert_eq!(
                    plan.collections()
                        .iter()
                        .filter(|c| c.string_span_projection().is_some())
                        .count(),
                    1,
                    "{source}"
                );
                plan.validate_current_visits().unwrap();
                let p = plan
                    .collections()
                    .iter()
                    .find_map(|c| c.string_span_projection())
                    .unwrap();
                assert!(plan.blocks().iter().all(|b| !matches!(b.action(), Action::LibraryCall { output, .. } if *output == p.selector() || *output == p.output())));
                let original = super::super::fusion_semantics::semantic_actions(&plan).unwrap();
                assert!(
                    matches!(&original[p.range_site() as usize], Action::LibraryCall { output, .. } if *output == p.selector())
                );
                assert!(
                    matches!(&original[p.selection_site() as usize], Action::LibraryCall { output, .. } if *output == p.output())
                );
            }
        }
    }

    #[test]
    fn string_span_projection_rejects_missing_replayed_wrong_scope_and_fake_values() {
        for plan in plans(
            "text/join([(alphabet<string>)[i..=i] | i <- 1u64..=limit<u64>], separator<string>)\n",
        ) {
            let (id, data) = plan
                .collections()
                .iter()
                .enumerate()
                .find(|(_, c)| c.string_span_projection().is_some())
                .unwrap();
            let p = data.string_span_projection().unwrap();
            for site in [p.range_site(), p.selection_site(), data.push()] {
                let mut omitted = plan.clone();
                omitted.blocks[site as usize].action = Action::Scalar(Scalar::None);
                assert!(omitted.validate_current_visits().is_err());
            }
            let mut wrong_lane = plan.clone();
            wrong_lane.blocks[p.selection_site() as usize].action = Action::ProjectStringSpan {
                collection: u32::MAX,
            };
            assert!(wrong_lane.validate_current_visits().is_err());
            let mut wrong_scope = plan.clone();
            wrong_scope.blocks[p.range_site() as usize].iteration = None;
            assert!(wrong_scope.validate_current_visits().is_err());
            let mut replay = plan.clone();
            replay.blocks[data.push() as usize].action = Action::ProjectStringSpan {
                collection: id as u32,
            };
            assert!(replay.validate_current_visits().is_err());
            let mut fake = plan.clone();
            fake.blocks[p.selection_site() as usize].action = Action::LibraryCall {
                template: p.selection_template().clone(),
                operands: vec![p.source(), p.selector()].into(),
                output: p.output(),
            };
            assert!(fake.validate_current_visits().is_err());
            let mut detached = plan.clone();
            detached.collections[id].string_span_projection = None;
            assert!(detached.validate_current_visits().is_err());
            let mut ordinary_read = plan.clone();
            ordinary_read.blocks[data.push() as usize].action = Action::CollectionPush {
                collection: id as u32,
                value: p.source(),
            };
            assert!(ordinary_read.validate_current_visits().is_err());
        }
    }

    #[test]
    fn string_span_projection_declines_extra_observations_stepped_selectors_and_materialized_results()
     {
        for source in [
            "text/join([(alphabet<string>)[i..=i] + alphabet[i..=i] | i <- 1u64..=limit<u64>], separator<string>)\n",
            "text/join([(alphabet<string>)[i..2u64..limit<u64>] | i <- 1u64..=limit], separator<string>)\n",
            "pieces := [(alphabet<string>)[i..=i] | i <- 1u64..=limit<u64>]\ntext/join(pieces, separator<string>)\npieces\n",
        ] {
            for plan in plans(source) {
                assert!(
                    plan.collections()
                        .iter()
                        .all(|c| c.string_span_projection().is_none()),
                    "{source}"
                );
                plan.validate_current_visits().unwrap();
            }
        }
    }
}

#[cfg(any(feature = "pure_text", feature = "map_ops"))]
fn candidate(
    plan: &impl ControlPlan,
    collection: usize,
    counts: &[usize],
    actions: &super::fusion_semantics::SemanticActions<'_>,
) -> Option<Projection> {
    use super::NativeCollectionPrimitive as Primitive;
    use mech_core::{DimensionExpr, IntegerWidth, SchemaBody};
    let data = plan.collections().get(collection)?;
    if !matches!(
        data.result(),
        ResultKind::TextJoin(_) | ResultKind::Histogram(_) | ResultKind::GuardedHistogramQuery(_)
    ) {
        return None;
    }
    let Action::CollectionPush { value: output, .. } = actions.get(data.push() as usize)? else {
        return None;
    };
    let definition = |value: Value| {
        plan.values()
            .get(value.get() as usize)
            .and_then(ControlType::definition)
    };
    let plain_string = |value: Value| {
        definition(value).is_some_and(|schema| {
            schema.body() == &SchemaBody::String && schema.dimension_parameters().is_empty()
        })
    };
    if counts.get(output.get() as usize) != Some(&1) || !plain_string(*output) {
        return None;
    }
    let mut producers = actions
        .iter()
        .enumerate()
        .filter_map(|(site, action)| match action {
            Action::LibraryCall {
                template,
                operands,
                output: produced,
            } if produced == output => Some((site, template, operands)),
            _ => None,
        });
    let (selection_site, selection_template, selection_operands) = producers.next()?;
    if producers.next().is_some()
        || selection_template.primitive() != Primitive::StringRangeAccess
        || selection_operands.len() != 2
        || selection_template.inputs().len() != 2
    {
        return None;
    }
    let source = selection_operands[0];
    let selector = selection_operands[1];
    if !plain_string(source) || counts.get(selector.get() as usize) != Some(&1) {
        return None;
    }
    let mut producers = actions
        .iter()
        .enumerate()
        .filter_map(|(site, action)| match action {
            Action::LibraryCall {
                template,
                operands,
                output,
            } if *output == selector => Some((site, template, operands)),
            _ => None,
        });
    let (range_site, range_template, operands) = producers.next()?;
    if producers.next().is_some()
        || !matches!(
            range_template.primitive(),
            Primitive::RangeInclusive | Primitive::RangeExclusive
        )
        || range_template.implementation_memory()
            != mech_core::ImplementationMemoryClass::NoAdditionalScratch
        || operands.len() != 2
        || range_template.inputs().len() != 2
        || operands.iter().any(|value| {
            definition(*value).is_none_or(|schema| {
                schema.body() != &SchemaBody::UnsignedInteger(IntegerWidth::W64)
                    || !schema.dimension_parameters().is_empty()
            })
        })
    {
        return None;
    }
    let schema = definition(selector)?;
    let SchemaBody::Matrix {
        element,
        dimensions,
    } = schema.body()
    else {
        return None;
    };
    if element.as_ref() != &SchemaBody::UnsignedInteger(IntegerWidth::W64)
        || dimensions.len() != 2
        || dimensions[0] != DimensionExpr::Constant(1)
        || !matches!(
            &dimensions[1],
            DimensionExpr::Constant(_) | DimensionExpr::Parameter(_)
        )
        || schema.dimension_parameters().len() > 1
        || !adjacent(
            plan,
            u32::try_from(range_site).ok()?,
            u32::try_from(selection_site).ok()?,
        )
        || !adjacent(plan, u32::try_from(selection_site).ok()?, data.push())
    {
        return None;
    }
    let scope = plan.blocks()[range_site].iteration();
    if plan.value_scopes().get(selector.get() as usize)
        != Some(&super::NativeIterationValueScope::Immutable { iteration: scope })
        || plan.value_scopes().get(output.get() as usize)
            != Some(&super::NativeIterationValueScope::Immutable { iteration: scope })
    {
        return None;
    }
    Some(Projection {
        range_site: u32::try_from(range_site).ok()?,
        selection_site: u32::try_from(selection_site).ok()?,
        range_template: range_template.clone(),
        selection_template: selection_template.clone(),
        operands: [operands[0], operands[1]],
        source,
        selector,
        output: *output,
    })
}

#[cfg(not(any(feature = "pure_text", feature = "map_ops")))]
fn candidate(
    _: &impl ControlPlan,
    _: usize,
    _: &[usize],
    _: &super::fusion_semantics::SemanticActions<'_>,
) -> Option<Projection> {
    None
}

pub(crate) fn lower(plan: &mut Plan) -> Result<(), &'static str> {
    let actions = super::fusion_semantics::semantic_actions(plan)?.into_owned();
    let counts = super::fusion_semantics::semantic_use_counts(plan)?;
    for collection in 0..plan.collections.len() {
        let Some(projection) = candidate(plan, collection, &counts, &actions) else {
            continue;
        };
        let collection_id = u32::try_from(collection).map_err(|_| BAD)?;
        plan.blocks[projection.range_site as usize].action = Action::PrepareStringSpanSelector {
            collection: collection_id,
        };
        plan.blocks[projection.selection_site as usize].action = Action::ProjectStringSpan {
            collection: collection_id,
        };
        plan.collections[collection].string_span_projection = Some(projection);
    }
    Ok(())
}

/// Rederive the exact same selected-factory/semantic use proof at every opaque
/// plan validation. The shared inventory expands private actions but never
/// treats the absent semantic selector/String identities as known Values.
pub(super) fn validate(plan: &impl ControlPlan) -> Result<(), &'static str> {
    let actions = super::fusion_semantics::semantic_actions(plan)?;
    let counts = super::fusion_semantics::semantic_use_counts(plan)?;
    let mut expected = vec![None; plan.blocks().len()];
    for (collection, data) in plan.collections().iter().enumerate() {
        let Some(projection) = data.string_span_projection() else {
            continue;
        };
        if candidate(plan, collection, &counts, &actions).as_ref() != Some(projection) {
            return Err(BAD);
        }
        for (site, phase) in [
            (projection.range_site, false),
            (projection.selection_site, true),
        ] {
            if expected
                .get_mut(site as usize)
                .ok_or(BAD)?
                .replace((collection as u32, phase))
                .is_some()
            {
                return Err(BAD);
            }
        }
    }
    for (site, block) in plan.blocks().iter().enumerate() {
        let actual = match block.action() {
            Action::PrepareStringSpanSelector { collection } => Some((*collection, false)),
            Action::ProjectStringSpan { collection } => Some((*collection, true)),
            _ => None,
        };
        if actual != expected[site] {
            return Err(BAD);
        }
    }
    Ok(())
}
