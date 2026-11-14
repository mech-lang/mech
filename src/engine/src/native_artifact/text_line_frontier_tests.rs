use super::*;
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
use std::sync::Arc;

pub(super) fn plans(source: &str) -> [Plan; 2] {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x46524f4e54), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    let mut builder = mech_core::FunctionCatalogBuilder::new();
    crate::install_intrinsic_runtime(&mut builder).unwrap();
    crate::install_intrinsic_compiler_runtime(&mut builder).unwrap();
    crate::install_intrinsic_resident(&mut builder).unwrap();
    crate::install_intrinsic_source(&mut builder).unwrap();
    let catalog = Arc::new(builder.build().unwrap());
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
    [artifact, decoded]
        .map(|artifact| super::super::lower_native_program_artifact(&artifact, &catalog).unwrap())
}

pub(super) fn scan(predicate: &str, eof: &str, extra: &str) -> String {
    let output = if extra.is_empty() { "result" } else { extra };
    format!(
        "+> text\n+> sequence\n#Scan(lines<[string]>, n<u64>) => <u64>\n  | :Step(i<u64>)\n  | :Done(out<u64>).\n#Scan(lines, n) -> :Step(1u64)\n  :Step(i)\n    | {predicate} -> :Done({eof})\n    | lines[i] == \"stop\" -> :Done(i)\n    | * -> :Step(i + 1u64)\n  :Done(out) => out.\nlines := text/lines(input<string>)\nn := sequence/length(lines)\nresult := #Scan(lines, n)\n{output}\n"
    )
}

#[test]
fn original_and_decoded_guarded_counts_have_no_early_numeric_use() {
    for source in [scan("i > n", "n + 1u64", ""), scan("n < i", "n + 1u64", "")] {
        for plan in plans(&source) {
            let frontiers = plan.text_line_count_frontiers();
            assert_eq!(frontiers.len(), 1, "{:#?}", plan.blocks());
            let frontier = &frontiers[0];
            assert_eq!(frontier.guards().len(), 1);
            assert!(!frontier.exact().is_empty());
            for guard in frontier.guards() {
                assert_ne!(guard.available(), guard.exhausted());
                assert_ne!(guard.index(), frontier.counts()[0]);
            }
        }
    }
}

#[test]
fn direct_count_escapes_and_non_frontier_predicates_stay_ordinary() {
    for source in [
        scan("i > n", "n + 1u64", "n"),
        scan("i > n", "n + 1u64", "result + n"),
        scan("i > n", "n + 1u64", "result + n + 1u64"),
        scan("i == n", "i", ""),
        scan("i >= n", "i", ""),
    ] {
        for plan in plans(&source) {
            assert!(plan.text_line_count_frontiers().is_empty(), "{source}");
        }
    }
}

#[test]
fn a_cold_non_eof_count_read_is_not_hidden_by_the_ordinary_guard() {
    let source = scan("i > n", "n + 1u64", "").replace(":Done(i)", ":Done(n + 1u64)");
    for plan in plans(&source) {
        assert_eq!(plan.text_line_matrix_regions().len(), 1);
        assert!(plan.text_line_count_frontiers().is_empty());
    }
}

#[test]
fn the_original_string_cannot_escape_its_private_line_producer() {
    let source = scan(
        "i > n",
        "n + 1u64",
        "result + sequence/length(input<string>)",
    );
    for plan in plans(&source) {
        assert_eq!(plan.text_line_matrix_regions().len(), 1);
        assert!(plan.text_line_count_frontiers().is_empty());
    }
}

#[test]
fn changed_predicates_and_lifecycle_boundaries_cannot_reuse_a_frontier() {
    for plan in plans(&scan("i > n", "n + 1u64", "")) {
        let frontier = plan.text_line_count_frontiers().pop().unwrap();
        let site = frontier.guards()[0].comparison() as usize;
        let mut changed = plan.clone();
        let Action::Scalar(Scalar::Primitive { op, .. }) = &mut changed.blocks[site].action else {
            unreachable!()
        };
        *op = Op::U64Ge;
        changed.validate_current_visits().unwrap();
        assert!(changed.text_line_count_frontiers().is_empty());
        let mut barrier = plan.clone();
        barrier.activation_boundary = Some(site as u32);
        assert!(barrier.text_line_count_frontiers().is_empty());
    }
}

#[test]
fn count_aliases_require_every_phi_arm_to_have_the_same_identity() {
    for (alternative, expected) in [("n", true), ("1u64", false)] {
        let source = scan("i > n", "n + 1u64", "").replace(
            "result := #Scan(lines, n)",
            &format!("copied := (choose<bool>)? | true => n | false => {alternative}.\nresult := #Scan(lines, copied)"),
        );
        for plan in plans(&source) {
            let frontiers = plan.text_line_count_frontiers();
            assert_eq!(!frontiers.is_empty(), expected, "{source}");
            if expected {
                assert!(!frontiers[0].aliases().is_empty());
                assert!(frontiers[0].counts().len() > 1);
            }
        }
    }
}

#[test]
fn a_root_frontier_does_not_require_any_fsm_or_source_names() {
    // Keep the capture checkpoint at an earlier live scalar instruction;
    // line-matrix views intentionally decline a producer on that checkpoint.
    let source = "+> text\n+> sequence\noffset := position<u64> + 0u64\nitems := text/lines(payload<string>)\nsize := sequence/length(items)\nanswer := (offset > size)? | true => size + 1u64 | false => 0u64.\nanswer\n";
    for plan in plans(source) {
        let frontiers = plan.text_line_count_frontiers();
        assert_eq!(frontiers.len(), 1);
        // An explicit exhaustive Bool match has two literal tests of the
        // same predicate, not one native Branch plus an implicit wildcard.
        assert_eq!(frontiers[0].guards().len(), 2);
        assert!(plan.iterations().is_empty());
    }
}

#[test]
fn both_edges_to_one_successor_cannot_fake_eof_dominance() {
    for plan in plans(&scan("i > n", "n + 1u64", "")) {
        let frontier = plan.text_line_count_frontiers().pop().unwrap();
        let selected = &frontier.guards()[0];
        let site = selected.branch() as usize;
        let successor = selected.exhausted();
        let mut changed = plan.clone();
        match &mut changed.blocks[site].terminator {
            Terminator::Scalar(
                Term::Branch { yes, no, .. } | Term::LiteralBranch { yes, no, .. },
            ) => {
                *yes = successor;
                *no = successor;
            }
            _ => unreachable!(),
        }
        // Exercise guard classification itself, independently of enclosing
        // CFG validation (which can also reject a mutated canonical graph).
        let actions = semantic_actions(&changed).unwrap();
        let uses = reads(&changed, &actions).unwrap();
        let definitions = definitions(&changed, &actions);
        let mut counts = vec![false; changed.values().len()];
        for count in frontier.counts() {
            counts[count.get() as usize] = true;
        }
        assert!(
            guards_for_comparison(
                &changed,
                selected.comparison(),
                &counts,
                &uses,
                &definitions
            )
            .is_none()
        );
        assert!(changed.text_line_count_frontiers().is_empty());
    }
}

#[test]
fn shared_boolean_edge_facts_keep_literal_polarity_and_unique_copies() {
    let source = "predicate := first<u64> < second<u64>\ncopied := predicate\nanswer := (copied? | true => 7u64 | false => 11u64)\n(answer, copied, true, false)\n";
    for plan in plans(source) {
        let facts = plan.facts();
        let (site, predicate, (no, yes)) = plan
            .blocks()
            .iter()
            .enumerate()
            .find_map(|(site, block)| {
                let candidates = match block.terminator() {
                    Terminator::Scalar(Term::Branch { condition, .. }) => vec![condition],
                    Terminator::Scalar(Term::LiteralBranch { left, right, .. }) => {
                        vec![left, right]
                    }
                    _ => return None,
                };
                candidates.into_iter().find_map(|v| {
                    facts
                        .boolean_branch_edges(site as u32, v)
                        .map(|edges| (site, v, edges))
                })
            })
            .unwrap();
        let literal = |wanted| {
            plan.constants()
                .iter()
                .find_map(|c| {
                    matches!(c.constant().data(),mech_core::ValueData::Bool(v) if *v == wanted)
                        .then_some(c.value())
                })
                .unwrap()
        };
        for truth in [false, true] {
            for swapped in [false, true] {
                let mut changed = plan.clone();
                let (left, right) = if swapped {
                    (literal(truth), predicate)
                } else {
                    (predicate, literal(truth))
                };
                changed.blocks[site].terminator = Terminator::Scalar(Term::LiteralBranch {
                    left,
                    right,
                    yes,
                    no,
                });
                assert_eq!(
                    changed.facts().boolean_branch_edges(site as u32, predicate),
                    Some(if truth { (no, yes) } else { (yes, no) })
                );
            }
        }
        let mut changed = plan.clone();
        changed.blocks[site].terminator = Terminator::Scalar(Term::Branch {
            condition: predicate,
            yes,
            no: yes,
        });
        assert!(
            changed
                .facts()
                .boolean_branch_edges(site as u32, predicate)
                .is_none()
        );
        // Source aliases are normally eliminated. Exercise the actual Copy
        // fact by replacing the unique producer in this descriptive test plan.
        let [definition] = facts.definition_sites(predicate.get()).unwrap() else {
            panic!("the comparison must have one current definition")
        };
        let copy = definition.site().unwrap() as usize;
        let source = literal(true);
        let output = predicate;
        let mut copied = plan.clone();
        copied.blocks[copy].action = Action::Scalar(Scalar::Copy { source, output });
        assert_eq!(copied.facts().unique_identity_copy(output), Some(source));
        let mut redefined = copied;
        let other = plan
            .blocks()
            .iter()
            .enumerate()
            .find(|(s, b)| *s != copy && matches!(b.action(), Action::Scalar(Scalar::None)))
            .map(|(s, _)| s)
            .unwrap();
        redefined.blocks[other].action = Action::Scalar(Scalar::Copy { source, output });
        assert!(redefined.facts().unique_identity_copy(output).is_none());
    }
}
