//! The streaming count and owned rewrite inventory must describe the same IR.
use super::super::NativeProgramArtifactPlan as Plan;
use super::*;
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
use std::collections::BTreeSet;
use std::sync::Arc;

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
    Ok(reads(plan, &semantic_actions(plan)?)?
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
            for block in plan.blocks() {
                let action = block.action();
                let semantic = semantic_action(plan, action).unwrap();
                if let Some(kind) = virtual_kind(action) {
                    selected.insert(kind);
                    assert!(matches!(semantic, Cow::Owned(_)), "{kind}");
                } else {
                    let Cow::Borrowed(borrowed) = semantic else {
                        panic!("ordinary instruction was cloned: {action:?}");
                    };
                    assert!(std::ptr::eq(borrowed, action));
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
