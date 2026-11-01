//! A conservative, selected-factory range/source fusion. This changes only a
//! private compiler view; the canonical artifact and every candidate remain.
use super::{
    NativeCollectionSource as Source, NativeControlLoop, NativeIterationAction as Action,
    NativeProgramArtifactPlan as Plan, NativeU64RangeCursor as Cursor,
};
#[cfg(any(feature = "pure_text", feature = "map_ops"))]
use super::{
    NativeIterationTerminator as Terminator, NativeMatchAction as Scalar,
    NativeMatchTerminator as Term,
};

const BAD: &str = "invalid native selected U64 range cursor";

fn uses(plan: &Plan) -> Result<Vec<usize>, &'static str> {
    super::guarded_histogram_query::semantic_use_counts(plan)
}

#[cfg(any(feature = "pure_text", feature = "map_ops"))]
fn candidate(plan: &Plan, id: usize, counts: &[usize]) -> Option<Cursor> {
    use super::{NativeCollectionPrimitive as Primitive, NativeScalarKind};
    use mech_core::{DimensionExpr, IntegerWidth, SchemaBody};
    let NativeControlLoop::Collection(generator) = plan.iterations().get(id)? else {
        return None;
    };
    let Source::Value(source) = generator.source() else {
        return None;
    };
    let collection = plan.collections().get(generator.collection() as usize)?;
    if collection.kind() != crate::ComprehensionKind::Matrix
        || counts.get(source.get() as usize) != Some(&1)
    {
        return None;
    }
    let (prepare, template, operands) =
        plan.blocks()
            .iter()
            .enumerate()
            .find_map(|(index, block)| match block.action() {
                Action::LibraryCall {
                    template,
                    operands,
                    output,
                } if output == source => Some((index, template, operands)),
                _ => None,
            })?;
    if !matches!(
        template.primitive(),
        Primitive::RangeInclusive | Primitive::RangeExclusive
    ) || operands.len() != 2
        || template.inputs().len() != 2
        || template.implementation_memory()
            != mech_core::ImplementationMemoryClass::NoAdditionalScratch
        || template.inputs().iter().any(|ty| {
            !ty.dimension_parameters().is_empty()
                || ty.body() != &SchemaBody::UnsignedInteger(IntegerWidth::W64)
        })
        || operands.iter().any(|value| {
            plan.values()
                .get(value.get() as usize)
                .is_none_or(|ty| ty.scalar_kind() != Some(NativeScalarKind::U64))
        })
        || generator.element().is_some_and(|value| {
            plan.values()[value.get() as usize].scalar_kind() != Some(NativeScalarKind::U64)
        })
        || plan.blocks()[prepare].iteration() != generator.parent()
    {
        return None;
    }
    let SchemaBody::Matrix {
        element,
        dimensions,
    } = template.output().body()
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
        || template.output().dimension_parameters().len() > 1
    {
        return None;
    }
    // Only an adjacent straight-line setup is fused. No conditional use,
    // intervening fallible operation, lifecycle checkpoint or cross-scope use.
    let mut next = prepare;
    let mut before_collection_begin = false;
    let mut remaining = plan.blocks().len();
    loop {
        remaining = remaining.checked_sub(1)?;
        let block = plan.blocks().get(next)?;
        if next == generator.initialize() as usize {
            break;
        }
        if next != prepare {
            if block.iteration() != generator.parent()
                || Some(next as u32) == plan.activation_boundary()
            {
                return None;
            }
            match block.action() {
                Action::Scalar(Scalar::None) => {}
                Action::CollectionBegin { collection: owner }
                    if *owner == generator.collection() && !before_collection_begin =>
                {
                    before_collection_begin = true;
                }
                _ => return None,
            }
        }
        let Terminator::Scalar(Term::Jump(target)) = block.terminator() else {
            return None;
        };
        next = target as usize;
    }
    Some(Cursor {
        prepare: u32::try_from(prepare).ok()?,
        template: template.clone(),
        operands: [operands[0], operands[1]],
        output: *source,
        before_collection_begin,
    })
}
#[cfg(not(any(feature = "pure_text", feature = "map_ops")))]
fn candidate(_: &Plan, _: usize, _: &[usize]) -> Option<Cursor> {
    None
}

pub(crate) fn lower(plan: &mut Plan) -> Result<(), &'static str> {
    let counts = uses(plan)?;
    let cursors = (0..plan.iterations().len())
        .filter_map(|id| candidate(plan, id, &counts).map(|cursor| (id, cursor)))
        .collect::<Vec<_>>();
    for (id, cursor) in cursors {
        let NativeControlLoop::Collection(generator) = &mut plan.iterations[id] else {
            return Err(BAD);
        };
        plan.blocks[cursor.prepare as usize].action = Action::PrepareU64Range {
            iteration: id as u32,
        };
        generator.source = Source::U64Range(cursor);
    }
    Ok(())
}

#[cfg(all(
    test,
    feature = "source",
    feature = "artifact-codec",
    feature = "semantic-compiler",
    feature = "resident-artifact",
    any(feature = "pure_text", feature = "map_ops")
))]
mod tests {
    use super::*;
    use mech_syntax::document::{
        AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
        parse_canonical_document,
    };
    use std::sync::Arc;
    fn artifacts(source: &str) -> ([crate::ProgramArtifact; 2], Arc<mech_core::FunctionCatalog>) {
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x43555253), Revision(1), source).unwrap(),
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
        ([artifact, decoded], catalog)
    }
    #[test]
    fn range_cursor_proof_elides_only_one_direct_selected_source_and_rejects_forged_lifecycle() {
        let (variants, catalog) = artifacts("[i | i <- 0u64..count<u64>]\n");
        for artifact in variants {
            let plan = super::super::lower_native_program_artifact(&artifact, &catalog).unwrap();
            let (id, cursor) = plan
                .iterations()
                .iter()
                .enumerate()
                .find_map(|(id, data)| match data {
                    NativeControlLoop::Collection(data) => match data.source() {
                        Source::U64Range(cursor) => Some((id, cursor.clone())),
                        _ => None,
                    },
                    _ => None,
                })
                .expect("single-use selected range is actually fused");
            plan.validate_current_visits().unwrap();
            assert!(plan.blocks().iter().all(|block| !matches!(block.action(), Action::LibraryCall { output, .. } if *output == cursor.semantic_output())));
            let mut omitted = plan.clone();
            omitted.blocks[cursor.prepare() as usize].action = Action::Scalar(Scalar::None);
            assert!(omitted.validate_current_visits().is_err());
            let mut fake_value = plan.clone();
            let NativeControlLoop::Collection(data) = &mut fake_value.iterations[id] else {
                unreachable!()
            };
            data.source = Source::Value(cursor.semantic_output());
            assert!(fake_value.validate_current_visits().is_err());
            let mut wrong_iteration = plan.clone();
            wrong_iteration.blocks[cursor.prepare() as usize].action = Action::PrepareU64Range {
                iteration: u32::MAX,
            };
            assert!(wrong_iteration.validate_current_visits().is_err());
            let mut wrong_owner = plan.clone();
            let NativeControlLoop::Collection(data) = &mut wrong_owner.iterations[id] else {
                unreachable!()
            };
            let Source::U64Range(cursor) = &mut data.source else {
                unreachable!()
            };
            cursor.before_collection_begin = !cursor.before_collection_begin;
            assert!(wrong_owner.validate_current_visits().is_err());
            let mut stale = plan.clone();
            let NativeControlLoop::Collection(data) = &mut stale.iterations[id] else {
                unreachable!()
            };
            let Source::U64Range(cursor) = &mut data.source else {
                unreachable!()
            };
            cursor.operands[0] = cursor.output;
            assert!(stale.validate_current_visits().is_err());
        }
    }
    #[test]
    fn range_cursor_proof_leaves_observed_stepped_and_floating_ranges_materialized() {
        for source in [
            "range := 0u64..count<u64>\n[i | i <- range]\nrange\n",
            "[i | i <- 0u64..2u64..count<u64>]\n",
            "[i | i <- 0.0..count<f64>]\n",
        ] {
            let (variants, catalog) = artifacts(source);
            for artifact in variants {
                let plan =
                    super::super::lower_native_program_artifact(&artifact, &catalog).unwrap();
                assert!(
                    plan.blocks()
                        .iter()
                        .all(|block| !matches!(block.action(), Action::PrepareU64Range { .. })),
                    "{source}"
                );
                plan.validate_current_visits().unwrap();
            }
        }
    }
    #[test]
    fn nested_range_cursors_have_distinct_current_visit_sources_and_reject_stale_inner_reads() {
        let (variants, catalog) = artifacts("[i + j | i <- 1u64..=outer<u64>, j <- 1u64..=i]\n");
        for artifact in variants {
            let plan = super::super::lower_native_program_artifact(&artifact, &catalog).unwrap();
            let cursors = plan
                .iterations()
                .iter()
                .enumerate()
                .filter_map(|(id, data)| match data {
                    NativeControlLoop::Collection(data) => match data.source() {
                        Source::U64Range(cursor) => Some((id, data, cursor)),
                        _ => None,
                    },
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(
                cursors.len(),
                2,
                "both actual selected ranges must be cursors"
            );
            let (outer, outer_data, outer_cursor) = cursors[0];
            let (inner, inner_data, inner_cursor) = cursors[1];
            assert_eq!(inner_data.parent(), Some(outer as u32));
            // Inline qualifier operations execute after this comprehension's
            // CollectionBegin; both preparations belong to its live builder.
            assert!(!outer_cursor.before_collection_begin());
            assert!(!inner_cursor.before_collection_begin());
            assert_eq!(inner_cursor.operands()[1], outer_data.element().unwrap());
            plan.validate_current_visits().unwrap();
            let mut missing_inner = plan.clone();
            missing_inner.blocks[inner_cursor.prepare() as usize].action =
                Action::Scalar(Scalar::None);
            assert!(missing_inner.validate_current_visits().is_err());
            let mut reused_outer = plan.clone();
            let NativeControlLoop::Collection(data) = &mut reused_outer.iterations[inner] else {
                unreachable!()
            };
            let Source::U64Range(cursor) = &mut data.source else {
                unreachable!()
            };
            cursor.prepare = outer_cursor.prepare();
            assert!(reused_outer.validate_current_visits().is_err());
            let mut stale_inner = plan.clone();
            let NativeControlLoop::Collection(data) = &mut stale_inner.iterations[outer] else {
                unreachable!()
            };
            let Source::U64Range(cursor) = &mut data.source else {
                unreachable!()
            };
            // A previous child invocation is NOT a definition in the next
            // outer initializer, even though its word has the same U64 kind.
            cursor.operands[1] = inner_data.element().unwrap();
            assert!(stale_inner.validate_current_visits().is_err());
            let mut wrong_scope = plan.clone();
            wrong_scope.blocks[inner_cursor.prepare() as usize].iteration = outer_data.parent();
            assert!(wrong_scope.validate_current_visits().is_err());
        }
    }

    #[test]
    fn hoisted_range_cursor_preserves_pre_builder_preparation_and_rejects_phase_forgery() {
        let (variants, catalog) = artifacts(
            "collect(count<u64>) => (out<[u64]>) :=\n  range := 0u64..count\n  out := [i | i <- range].\ncollect(count<u64>)\n",
        );
        for artifact in variants {
            let plan = super::super::lower_native_program_artifact(&artifact, &catalog).unwrap();
            let (id, cursor) = plan
                .iterations()
                .iter()
                .enumerate()
                .find_map(|(id, data)| match data {
                    NativeControlLoop::Collection(data) => match data.source() {
                        Source::U64Range(cursor) => Some((id, cursor)),
                        _ => None,
                    },
                    _ => None,
                })
                .expect("sole-use hoisted range must really select a cursor");
            assert!(cursor.before_collection_begin());
            plan.validate_current_visits().unwrap();
            let mut forged = plan.clone();
            let NativeControlLoop::Collection(data) = &mut forged.iterations[id] else {
                unreachable!()
            };
            let Source::U64Range(cursor) = &mut data.source else {
                unreachable!()
            };
            cursor.before_collection_begin = false;
            assert!(forged.validate_current_visits().is_err());
        }
    }
}

/// Reconstruct the materializing compiler view and derive the same strict
/// proof again. This runs during engine construction/link rederivation, never
/// per source element. No caller-supplied range strategy is trusted.
pub(crate) fn validate(plan: &Plan) -> Result<(), &'static str> {
    let mut original = None;
    for (id, iteration) in plan.iterations().iter().enumerate() {
        let NativeControlLoop::Collection(data) = iteration else {
            continue;
        };
        let Source::U64Range(cursor) = data.source() else {
            continue;
        };
        let view = original.get_or_insert_with(|| plan.clone());
        let block = view.blocks.get_mut(cursor.prepare as usize).ok_or(BAD)?;
        if !matches!(block.action(), Action::PrepareU64Range { iteration } if *iteration as usize == id)
        {
            return Err(BAD);
        }
        block.action = Action::LibraryCall {
            template: cursor.template.clone(),
            operands: cursor.operands.into(),
            output: cursor.output,
        };
        let NativeControlLoop::Collection(generator) = &mut view.iterations[id] else {
            return Err(BAD);
        };
        generator.source = Source::Value(cursor.output);
    }
    let Some(original) = original else {
        return if plan
            .blocks()
            .iter()
            .any(|block| matches!(block.action(), Action::PrepareU64Range { .. }))
        {
            Err(BAD)
        } else {
            Ok(())
        };
    };
    // The reconstructed view must itself be a valid canonical current-visit
    // graph; this also verifies every retained exact selected template/type.
    super::phase_validation::validate(&original)?;
    super::iteration_validation::validate(&original)?;
    let counts = uses(&original)?;
    let mut seen = vec![false; plan.blocks().len()];
    for (id, iteration) in plan.iterations().iter().enumerate() {
        let NativeControlLoop::Collection(data) = iteration else {
            continue;
        };
        let Source::U64Range(cursor) = data.source() else {
            continue;
        };
        if candidate(&original, id, &counts).as_ref() != Some(cursor)
            || core::mem::replace(seen.get_mut(cursor.prepare as usize).ok_or(BAD)?, true)
        {
            return Err(BAD);
        }
    }
    for (id, block) in plan.blocks().iter().enumerate() {
        if matches!(block.action(), Action::PrepareU64Range { .. }) != seen[id] {
            return Err(BAD);
        }
    }
    Ok(())
}
