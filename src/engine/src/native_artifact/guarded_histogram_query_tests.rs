//! Engine proof tests; runtime/resource qualification lives in the GPU adapter.
use super::super::{
    NativeGuardedHistogramQuery as Sink, NativeHistogramQueryPhase as Phase,
    NativeHistogramQuerySetShape as SetShape, NativeHistogramQueryStepKind as Kind,
    NativeIterationTerminator as Terminator, NativeMatchAction as Scalar,
    NativeMatchTerminator as Term, NativeMatchValue as Value,
};
use super::selected::{schema, schema_certificate};
use super::*;
use mech_core::{CardinalitySpec, DimensionExpr, IntegerWidth, SchemaBody, SchemaDraft};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
use std::sync::Arc;

const GOOD: &str = "lookup(items<[string]>, query<string>) => (out<u64>) :=\n  words := [word | word <- items]\n  counts := map/histogram(words)\n  keys<{string}> := map/keys(counts)\n  count := (query ∈ keys)?\n    | true => counts{query}\n    | false => 0u64.\n  out := count.\nlookup(source<[string]>, query<string>)\n";

fn plans(source: &str) -> [Plan; 2] {
    let source = format!("+> map\n+> sequence\n+> text\n{source}");
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x515545525950), Revision(1), source.as_str()).unwrap(),
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
    let original = crate::CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            catalog.clone(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap();
    let decoded = crate::decode_program_artifact_bytecode_v1(
        &crate::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    [original, decoded]
        .map(|artifact| super::super::lower_native_program_artifact(&artifact, &catalog).unwrap())
}
fn demand(plan: &Plan) -> (usize, Sink) {
    let selected = plan
        .collections()
        .iter()
        .enumerate()
        .filter_map(|(id, c)| match c.result() {
            ResultKind::GuardedHistogramQuery(s) => Some((id, s.clone())),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        selected.len(),
        1,
        "must select the real strategy, not a fallback"
    );
    selected.into_iter().next().unwrap()
}

#[test]
fn guarded_histogram_query_rederives_selected_stages_and_universal_free_d_shapes() {
    for plan in plans(GOOD)
        .into_iter()
        .chain(plans(&GOOD.replace("map/keys(counts)", "counts_keys")))
    {
        let (id, sink) = demand(&plan);
        plan.validate_current_visits().unwrap();
        assert!(sink.steps().iter().all(|step| matches!(
            plan.blocks()[step.site() as usize].action(),
            Action::HistogramQueryStep { collection, .. } if *collection as usize == id
        )));
        let h = sink
            .steps()
            .iter()
            .find(|s| s.kind() == Kind::Histogram)
            .unwrap();
        let k = sink
            .steps()
            .iter()
            .find(|s| s.kind() == Kind::Keys)
            .unwrap();
        let s = sink.steps().iter().find(|s| s.kind() == Kind::Set).unwrap();
        assert_eq!(schema_certificate(&plan, h, k, s), Some(sink.set_shape()));
        let keys = schema(&plan, k.output().unwrap()).unwrap();
        for distinct in [0, 1, 2, 17, u64::MAX] {
            let shape = mech_core::shape_for_resolved_extents(keys, &[1, distinct]).unwrap();
            assert_eq!(shape.parameter_values(), &[distinct]);
        }
        let set = schema(&plan, s.output().unwrap()).unwrap();
        for distinct in 0..4 {
            let data = mech_core::ValueDataDraft::Set(
                (0..distinct)
                    .map(|i| mech_core::ValueDataDraft::String(format!("key-{i}")))
                    .collect(),
            );
            let shape = mech_core::shape_for_value_data(set, &data, &[], None).unwrap();
            if sink.set_shape() == SetShape::FreeExact {
                assert_eq!(shape.parameter_values(), &[distinct as u64]);
            } else {
                assert!(shape.parameter_values().is_empty());
            }
        }
        // The Matrix/Map/keys/Set remain provenance only, not fake handles or
        // known canonical roots. Every producer read outside the component is
        // rejected by the same ordinary current-visit proof below.
        assert_eq!(
            semantic_use_counts(&plan).unwrap()[plan.collections()[id].output().get() as usize],
            1
        );
    }
}

#[test]
fn guarded_histogram_query_uses_bool_roles_not_function_names_or_arm_ordinals() {
    let source = GOOD.replace("lookup", "arbitrary-count").replace(
        "| true => counts{query}\n    | false => 0u64.",
        "| false => 0u64\n    | true => counts{query}.",
    );
    for plan in plans(&source) {
        let (_, sink) = demand(&plan);
        assert_eq!(sink.bool_tests().len(), 2);
        plan.validate_current_visits().unwrap();
    }
}

#[test]
fn guarded_histogram_query_nested_invocation_keeps_query_before_begin_and_cleans_at_latch() {
    let source = GOOD.replace(
        "lookup(source<[string]>, query<string>)",
        "matrix := source<[string]>\n[lookup(matrix, query) | query <- queries<[string]>]",
    );
    for plan in plans(&source) {
        let (id, sink) = demand(&plan);
        assert!(plan.collections()[id].parent().is_some());
        plan.validate_current_visits().unwrap();
        let mut omitted = plan.clone();
        let cleanup = sink
            .steps()
            .iter()
            .find(|s| s.kind() == Kind::Cleanup)
            .unwrap();
        omitted.blocks[cleanup.site() as usize].action = Action::Scalar(Scalar::None);
        assert!(omitted.validate_current_visits().is_err());
    }
}

#[test]
fn guarded_histogram_query_proved_identity_alias_is_private_not_a_fake_value_copy() {
    for plan in plans(GOOD) {
        let (id, sink) = demand(&plan);
        let mut original = plan.clone();
        let actions = semantic_actions(&plan).unwrap();
        for (block, action) in original.blocks.iter_mut().zip(actions.iter().cloned()) {
            block.action = action;
        }
        original.collections[id].result = ResultKind::Materialized;
        let source = original.collections[id].output();
        let alias = Value(original.values.len() as u32);
        let mut values = original.values.to_vec();
        values.push(values[source.get() as usize].clone());
        original.values = values.into();
        let mut scopes = original.value_scopes.to_vec();
        scopes.push(super::super::NativeIterationValueScope::Immutable {
            iteration: original.collections[id].parent(),
        });
        original.value_scopes = scopes.into();
        let h = sink
            .steps()
            .iter()
            .find(|s| s.kind() == Kind::Histogram)
            .unwrap();
        let finish = original.collections[id].finish() as usize;
        let terminator = original.blocks[finish].terminator();
        let site = original.blocks.len() as u32;
        let mut blocks = original.blocks.to_vec();
        blocks.push(super::super::NativeIterationBlock {
            origin: original.blocks[h.site() as usize].origin(),
            action: Action::Scalar(Scalar::Copy {
                source,
                output: alias,
            }),
            terminator,
            iteration: original.collections[id].parent(),
        });
        blocks[finish].terminator = Terminator::Scalar(Term::Jump(site));
        let Action::LibraryCall { operands, .. } = &mut blocks[h.site() as usize].action else {
            unreachable!()
        };
        operands[0] = alias;
        original.blocks = blocks.into();
        original.validate_current_visits().unwrap();
        lower(&mut original).unwrap();
        let (_, proof) = demand(&original);
        assert!(proof.steps().iter().any(|s| s.site() == site
            && matches!(
                s.kind(),
                Kind::Alias {
                    phase: Phase::ProducerReady
                }
            )));
        original.validate_current_visits().unwrap();
        let mut changed = original.clone();
        let ResultKind::GuardedHistogramQuery(s) = &mut changed.collections[id].result else {
            unreachable!()
        };
        s.steps.iter_mut().find(|s| s.site() == site).unwrap().kind = Kind::Alias {
            phase: Phase::KeysReady,
        };
        assert!(changed.validate_current_visits().is_err());
    }
}

#[test]
fn guarded_histogram_query_rejects_omitted_reordered_forged_and_leaked_private_stages() {
    for plan in plans(GOOD) {
        let (id, sink) = demand(&plan);
        for step in sink.steps() {
            let mut omitted = plan.clone();
            omitted.blocks[step.site() as usize].action = Action::Scalar(Scalar::None);
            assert!(
                omitted.validate_current_visits().is_err(),
                "omitted {:?}",
                step.kind()
            );
            let mut wrong = plan.clone();
            wrong.blocks[step.site() as usize].action = Action::HistogramQueryStep {
                collection: id as u32,
                step: u32::MAX,
            };
            assert!(wrong.validate_current_visits().is_err());
        }
        let mut materialized = plan.clone();
        materialized.collections[id].result = ResultKind::Materialized;
        assert!(materialized.validate_current_visits().is_err());
        let cleanup = sink
            .steps()
            .iter()
            .position(|s| s.kind() == Kind::Cleanup)
            .unwrap();
        let mut early = plan.clone();
        let begin = early.collections[id].begin();
        early.blocks[begin as usize].action = Action::HistogramQueryStep {
            collection: id as u32,
            step: cleanup as u32,
        };
        assert!(early.validate_current_visits().is_err());
        let mut wrong_query = plan.clone();
        let ResultKind::GuardedHistogramQuery(s) = &mut wrong_query.collections[id].result else {
            unreachable!()
        };
        s.query = plan.collections()[id].output();
        assert!(wrong_query.validate_current_visits().is_err());
        let mut changed_zero = plan.clone();
        let ResultKind::GuardedHistogramQuery(s) = &mut changed_zero.collections[id].result else {
            unreachable!()
        };
        s.zero = s.match_result();
        assert!(changed_zero.validate_current_visits().is_err());
        let lookup = sink
            .steps()
            .iter()
            .find(|s| s.kind() == Kind::Lookup)
            .unwrap();
        let mut skip_hit = plan.clone();
        let site = skip_hit
            .blocks
            .iter()
            .position(|b| b.terminator() == Terminator::Scalar(Term::Jump(lookup.site())))
            .unwrap();
        skip_hit.blocks[site].terminator = Terminator::Scalar(Term::Jump(sink.true_copy()));
        assert!(skip_hit.validate_current_visits().is_err());
        let mut escape = plan.clone();
        let site = escape
            .blocks
            .iter()
            .position(|b| b.terminator() == Terminator::Scalar(Term::Complete))
            .unwrap();
        let absent = plan.collections()[id].output();
        escape.blocks[site].action = Action::Scalar(Scalar::Copy {
            source: absent,
            output: absent,
        });
        assert!(escape.validate_current_visits().is_err());
        let mut leak = plan.clone();
        let cleanup_site = sink.steps()[cleanup].site();
        leak.blocks[cleanup_site as usize].action = Action::Scalar(Scalar::None);
        assert!(leak.validate_current_visits().is_err());
    }
}

#[test]
fn guarded_histogram_query_schema_certificate_declines_hidden_distinct_constraints() {
    for plan in plans(GOOD) {
        let (_, sink) = demand(&plan);
        let h = sink
            .steps()
            .iter()
            .find(|s| s.kind() == Kind::Histogram)
            .unwrap();
        let k = sink
            .steps()
            .iter()
            .find(|s| s.kind() == Kind::Keys)
            .unwrap();
        let s = sink.steps().iter().find(|s| s.kind() == Kind::Set).unwrap();
        for cardinality in [
            CardinalitySpec::Exact(DimensionExpr::Constant(1)),
            CardinalitySpec::Dynamic {
                upper_bound: Some(DimensionExpr::Constant(1)),
            },
        ] {
            let mut constrained = plan.clone();
            constrained.values[h.output().unwrap().get() as usize].definition = SchemaDraft {
                dimension_parameters: Box::new([]),
                body: SchemaBody::Map {
                    key: Box::new(SchemaBody::String),
                    value: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W64)),
                    cardinality: cardinality.clone(),
                },
            }
            .finalize()
            .unwrap();
            assert!(schema_certificate(&constrained, h, k, s).is_none());
            constrained = plan.clone();
            constrained.values[s.output().unwrap().get() as usize].definition = SchemaDraft {
                dimension_parameters: Box::new([]),
                body: SchemaBody::Set {
                    element: Box::new(SchemaBody::String),
                    cardinality,
                },
            }
            .finalize()
            .unwrap();
            assert!(schema_certificate(&constrained, h, k, s).is_none());
        }
        let mut constrained = plan.clone();
        let original = schema(&plan, k.output().unwrap()).unwrap();
        constrained.values[k.output().unwrap().get() as usize].definition = SchemaDraft {
            dimension_parameters: vec![mech_core::DimensionParameterDeclaration {
                id: mech_core::DimensionParameterId::new(0),
                origin: mech_core::DimensionParameterOrigin::Inferred,
                lifetime: mech_core::DimensionLifetime::Turn,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: Some(DimensionExpr::Constant(1)),
            }]
            .into(),
            body: original.body().clone(),
        }
        .finalize()
        .unwrap();
        assert!(schema_certificate(&constrained, h, k, s).is_none());
    }
}

#[test]
fn guarded_histogram_query_declines_extra_observations_different_query_and_nonzero_miss() {
    for source in [
        GOOD.replace("| false => 0u64", "| false => 1u64"),
        GOOD.replace("counts{query}", "counts{query + \"\"}"),
        GOOD.replace("query ∈ keys", "query ∉ keys"),
        GOOD.replace(
            "out := count.",
            "out := count + sequence/length(map/values(counts)).",
        ),
        GOOD.replace("| true => counts{query}", "| true => counts{query} + 1u64"),
        GOOD.replace(
            "out := count.",
            "out := count + sequence/length(map/keys(counts)).",
        ),
        GOOD.replace("keys<{string}>", "later := query + \"\"\n  keys<{string}>")
            .replace("query ∈ keys", "later ∈ keys")
            .replace("counts{query}", "counts{later}"),
    ] {
        for plan in plans(&source) {
            assert!(
                plan.collections()
                    .iter()
                    .all(|c| !matches!(c.result(), ResultKind::GuardedHistogramQuery(_))),
                "{source}"
            );
        }
    }
}
