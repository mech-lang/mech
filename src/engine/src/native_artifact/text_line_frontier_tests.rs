use super::*;
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
use std::sync::Arc;

fn plans(source: &str) -> [Plan; 2] {
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

fn scan(predicate: &str, eof: &str, extra: &str) -> String {
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
