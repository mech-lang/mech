//! The streaming count and owned rewrite inventory must describe the same IR.
use super::super::NativeProgramArtifactPlan as Plan;
use super::*;
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
use std::collections::BTreeSet;
use std::sync::Arc;

#[path = "validation_inventory_tests.rs"]
mod validation_inventory_tests;

const SOURCES: &[&str] = &[
    "0u64\n",
    "[i | i <- 0u64..count<u64>]\n",
    "text/join([(entries<{string:string}>){piece} | piece <- source<string>], separator<string>)\n",
    "text/join([(alphabet<string>)[i..=i] | i <- 1u64..=limit<u64>], separator<string>)\n",
    "map/histogram([word | word <- items<[string]>])\n",
    "lookup(items<[string]>, query<string>) => (out<u64>) :=\n  words := [word | word <- items]\n  counts := map/histogram(words)\n  keys<{string}> := map/keys(counts)\n  count := (query ∈ keys)?\n    | true => counts{query}\n    | false => 0u64.\n  out := count.\nlookup(source<[string]>, query<string>)\n",
    "text/join(items<[string]>, separator<string>)\n",
];

fn plans(source: &str) -> [Plan; 2] {
    let source = format!("+> text\n+> map\n+> sequence\n{source}");
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x55534553), Revision(1), source.as_str()).unwrap(),
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

fn expanded_counts(plan: &Plan) -> Result<Vec<usize>, &'static str> {
    Ok(reads(plan, &semantic_actions(plan)?.into_owned())?
        .into_iter()
        .map(|uses| uses.len())
        .collect())
}

fn virtual_kind(action: &Action) -> Option<&'static str> {
    match action {
        Action::ProjectStringMapEntry { .. } => Some("map-entry"),
        Action::PrepareStringSpanSelector { .. } => Some("span-selector"),
        Action::ProjectStringSpan { .. } => Some("span"),
        Action::TextJoinFinish { .. } => Some("join"),
        Action::HistogramFinish { .. } => Some("histogram"),
        Action::HistogramQueryStep { .. } => Some("query"),
        _ => None,
    }
}

#[test]
fn streamed_counts_match_owned_inventory_for_original_and_decoded_fusions() {
    let mut selected = BTreeSet::new();
    let mut borrowed_library = false;
    let mut borrowed_range = false;
    for source in SOURCES {
        let variants = plans(source);
        for plan in &variants {
            plan.validate_current_visits().unwrap();
            assert_eq!(semantic_use_counts(plan), expanded_counts(plan), "{source}");
            let inventory = semantic_actions(plan).unwrap();
            assert_eq!(inventory.len(), plan.blocks().len());
            assert_eq!(
                reads(plan, &inventory),
                reads(plan, &semantic_actions(plan).unwrap().into_owned())
            );
            for (site, block) in plan.blocks().iter().enumerate() {
                let action = block.action();
                let semantic = semantic_action(plan, action).unwrap();
                if let Some(kind) = virtual_kind(action) {
                    selected.insert(kind);
                    assert!(matches!(semantic, Cow::Owned(_)), "{kind}");
                    assert!(matches!(inventory.0[site], Cow::Owned(_)), "{kind}");
                } else {
                    let Cow::Borrowed(borrowed) = semantic else {
                        panic!("ordinary instruction was cloned: {action:?}");
                    };
                    assert!(std::ptr::eq(borrowed, action));
                    assert!(matches!(inventory.0[site], Cow::Borrowed(_)));
                    assert!(std::ptr::eq(&inventory[site], action));
                    borrowed_library |= matches!(action, Action::LibraryCall { .. });
                    borrowed_range |= matches!(action, Action::PrepareU64Range { .. });
                }
            }
        }
        assert_eq!(
            semantic_use_counts(&variants[0]),
            semantic_use_counts(&variants[1])
        );
    }
    assert_eq!(
        selected,
        [
            "map-entry",
            "span-selector",
            "span",
            "join",
            "histogram",
            "query"
        ]
        .into_iter()
        .collect()
    );
    assert!(
        borrowed_library,
        "an ordinary library call must be borrowed"
    );
    assert!(borrowed_range, "an ordinary range action must be borrowed");
}

#[test]
fn owned_semantic_snapshot_detaches_before_plan_rewriting() {
    for mut plan in plans(SOURCES.last().unwrap()) {
        let snapshot = semantic_actions(&plan).unwrap().into_owned();
        assert!(
            snapshot
                .0
                .iter()
                .all(|action| matches!(action, Cow::Owned(_)))
        );
        let original_reads = reads(&plan, &snapshot).unwrap();
        let site = plan
            .blocks()
            .iter()
            .position(|block| matches!(block.action(), Action::LibraryCall { .. }))
            .unwrap();
        assert!(!std::ptr::eq(&snapshot[site], plan.blocks()[site].action()));
        plan.blocks[site].action = Action::Scalar(Scalar::None);
        assert!(matches!(snapshot[site], Action::LibraryCall { .. }));
        assert_eq!(reads(&plan, &snapshot).unwrap(), original_reads);
        assert_ne!(
            reads(&plan, &semantic_actions(&plan).unwrap()).unwrap(),
            original_reads
        );
    }
}

#[test]
fn streamed_counts_reject_invalid_virtual_metadata_and_value_reads() {
    for plan in plans(SOURCES[0]) {
        for action in [
            Action::ProjectStringMapEntry {
                collection: u32::MAX,
            },
            Action::PrepareStringSpanSelector {
                collection: u32::MAX,
            },
            Action::ProjectStringSpan {
                collection: u32::MAX,
            },
            Action::TextJoinFinish {
                collection: u32::MAX,
            },
            Action::HistogramFinish {
                collection: u32::MAX,
            },
            Action::HistogramQueryStep {
                collection: u32::MAX,
                step: 0,
            },
            Action::PrepareU64Range {
                iteration: u32::MAX,
            },
            Action::Scalar(Scalar::Copy {
                source: Value(u32::MAX),
                output: Value(0),
            }),
        ] {
            let mut invalid = plan.clone();
            invalid.blocks[0].action = action;
            assert_eq!(semantic_use_counts(&invalid), Err(BAD));
            assert_eq!(expanded_counts(&invalid), Err(BAD));
        }
        for terminator in [
            Term::Branch {
                condition: Value(u32::MAX),
                yes: 0,
                no: 0,
            },
            Term::LiteralBranch {
                left: Value(0),
                right: Value(u32::MAX),
                yes: 0,
                no: 0,
            },
        ] {
            let mut invalid = plan.clone();
            invalid.blocks[0].terminator = Terminator::Scalar(terminator);
            assert_eq!(semantic_use_counts(&invalid), Err(BAD));
            assert_eq!(expanded_counts(&invalid), Err(BAD));
        }
    }
}

#[test]
fn streamed_counts_reject_wrong_virtual_strategy_and_missing_query_step() {
    for source in &SOURCES[2..] {
        for plan in plans(source) {
            for (id, collection) in plan.collections().iter().enumerate() {
                let mut invalid = plan.clone();
                invalid.blocks[0].action = Action::HistogramQueryStep {
                    collection: id as u32,
                    step: u32::MAX,
                };
                assert_eq!(semantic_use_counts(&invalid), Err(BAD));
                assert_eq!(expanded_counts(&invalid), Err(BAD));
                let wrong = match collection.result() {
                    ResultKind::TextJoin(_) => Action::HistogramFinish {
                        collection: id as u32,
                    },
                    _ => Action::TextJoinFinish {
                        collection: id as u32,
                    },
                };
                invalid.blocks[0].action = wrong;
                assert_eq!(semantic_use_counts(&invalid), Err(BAD));
                assert_eq!(expanded_counts(&invalid), Err(BAD));
            }
        }
    }
}

#[test]
fn same_plan_facts_preserve_read_roles_lazy_inventory_and_definition_classes() {
    use crate::native_artifact::NativeProgramProducerRole as Role;
    for source in SOURCES {
        for plan in plans(source) {
            let facts = plan.facts();
            assert!(std::ptr::eq(facts.plan(), &plan));
            assert_eq!(
                facts.semantic_use_counts().unwrap(),
                semantic_use_counts(&plan).unwrap()
            );
            for (index, &count) in facts.semantic_use_counts().unwrap().iter().enumerate() {
                assert_eq!(facts.read_roles(Value(index as u32)).unwrap().len(), count);
            }
            assert!(std::ptr::eq(
                facts.scalar_aggregates(),
                facts.scalar_aggregates()
            ));
            assert_eq!(
                serde_json::to_vec(facts.scalar_aggregates()).unwrap(),
                serde_json::to_vec(&plan.scalar_aggregates()).unwrap()
            );
            for index in 0..plan.transport_input_count() {
                assert!(
                    facts
                        .definition_sites(plan.transport_input_value(index).get())
                        .unwrap()
                        .iter()
                        .any(|d| d.role() == Role::Input)
                );
            }
            for collection in plan.collections() {
                if matches!(collection.result(), ResultKind::Materialized) {
                    let definitions = facts.definition_sites(collection.output().get()).unwrap();
                    assert!(
                        definitions
                            .iter()
                            .any(|d| d.role() == Role::CollectionBuilder)
                    );
                    assert!(
                        definitions
                            .iter()
                            .any(|d| d.role() == Role::MaterializedCollection)
                    );
                }
            }
        }
    }
}

#[test]
fn parallel_transition_reads_stay_distinct_from_fusion_and_strict_dominance() {
    for plan in plans(SOURCES[1]) {
        let facts = plan.facts();
        let [super::super::NativeControlLoop::Collection(generator)] = plan.iterations() else {
            panic!("range fixture must retain one loop")
        };
        let super::super::NativeCollectionSource::U64Range(cursor) = generator.source() else {
            panic!("range fixture must retain its original source")
        };
        for site in [generator.initialize(), generator.latch()] {
            let mut actual = Vec::new();
            facts
                .parallel_action_reads(site, &mut |value| {
                    actual.push(value);
                    Ok(())
                })
                .unwrap();
            assert_eq!(actual, cursor.operands());
            for &value in &actual {
                assert!(
                    !facts
                        .read_roles(value)
                        .unwrap()
                        .contains(&NativeProgramReadRole::Action(site))
                );
            }
        }
        assert!(
            facts
                .strictly_dominates(generator.initialize(), generator.header())
                .unwrap()
        );
        assert!(
            !facts
                .strictly_dominates(generator.header(), generator.initialize())
                .unwrap()
        );
        assert!(
            !facts
                .strictly_dominates(generator.header(), generator.header())
                .unwrap()
        );
        assert!(facts.lexical_ancestor(None, Some(0)));
        assert!(facts.lexical_ancestor(Some(0), Some(0)));
        assert!(!facts.lexical_ancestor(Some(0), None));
        assert!(
            facts
                .predecessors(generator.header())
                .unwrap()
                .contains(&generator.initialize())
        );
    }
}
