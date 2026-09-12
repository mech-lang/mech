use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use mech_syntax::document::parser::canonical::{
    CanonicalRuleOutcome, parse_canonical_document_rule_for_test,
};
use mech_syntax::document::parser::canonical_rule_id;
use mech_syntax::document::{
    AstNode, CanonicalDocumentNode, DocumentId, DocumentSyntax, ParagraphSyntax, ParseConfig,
    Revision, SectionSyntax, SyntaxKind, SyntaxNode, TextSnapshot, validate_lossless_range,
};

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(0x5_007), Revision(7), text).unwrap()
}

fn find(node: SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if node.kind() == kind {
        return Some(node);
    }
    node.children().find_map(|child| find(child, kind))
}

fn kind(name: &str) -> SyntaxKind {
    let generated = fs::read_to_string(
        repository_root().join("src/syntax/src/document/parser/canonical/document_grammar.rs"),
    )
    .unwrap();
    let rule = format!("rule: rules::{},", name.replace('-', "_").to_uppercase());
    let start = generated
        .find(&rule)
        .unwrap_or_else(|| panic!("missing {name}"));
    let tail = &generated[start..];
    let marker = "kind: Some(SyntaxKind::";
    let start = tail
        .find(marker)
        .unwrap_or_else(|| panic!("missing kind for {name}"))
        + marker.len();
    let end = tail[start..].find(')').unwrap() + start;
    let spelling = &tail[start..end];
    mech_syntax::document::parser::CANONICAL_PORTS
        .iter()
        .find(|port| port.name == name)
        .and_then(|port| match port.node_policy {
            mech_syntax::document::parser::NodePolicy::Node(kind)
            | mech_syntax::document::parser::NodePolicy::Root(kind)
                if format!("{kind:?}") == spelling =>
            {
                Some(kind)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("invalid generated kind for {name}"))
}

#[test]
fn every_s7_rule_has_a_clean_specification_derived_source() {
    let table = fs::read_to_string(
        repository_root().join("docs/design/grammar-audit/s7-document-certification.tsv"),
    )
    .unwrap();
    let mut lines = table.lines();
    assert_eq!(
        lines.next(),
        Some("grammar-name\taccepted-source-json\tnode-policy\tsemantic-status\tspec-location")
    );
    let rows = lines.collect::<Vec<_>>();
    assert_eq!(rows.len(), 112);

    for line in rows {
        let fields = line.split('\t').collect::<Vec<_>>();
        assert_eq!(fields.len(), 5, "{line}");
        let name = fields[0];
        let text: String = serde_json::from_str(fields[1]).unwrap();
        let rule = canonical_rule_id(name).unwrap();
        let parsed =
            parse_canonical_document_rule_for_test(source(&text), rule, ParseConfig::default())
                .unwrap_or_else(|| panic!("unsupported S7 rule {name}"));
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::Matched, "{name}");
        assert!(
            parsed.diagnostics.is_empty(),
            "{name}: {:#?}",
            parsed.diagnostics.as_slice()
        );
        assert_eq!(parsed.consumed, parsed.source.full_range(), "{name}");
        validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed).unwrap();

        match fields[2] {
            "transparent" => {}
            "root:Document" => {
                let node = find(parsed.syntax(), SyntaxKind::Document).expect("document root");
                assert!(DocumentSyntax::cast(node).is_some());
            }
            policy if policy.starts_with("node:") => {
                let expected = kind(name);
                let node = find(parsed.syntax(), expected)
                    .unwrap_or_else(|| panic!("{name} did not emit {expected:?}"));
                if expected == SyntaxKind::Section {
                    assert!(SectionSyntax::cast(node).is_some());
                } else if expected == SyntaxKind::Paragraph {
                    assert!(ParagraphSyntax::cast(node).is_some());
                } else {
                    assert!(CanonicalDocumentNode::cast(node).is_some(), "{name}");
                }
            }
            policy => panic!("invalid S7 policy {policy}"),
        }
        assert_eq!(fields[4], format!("docs/design/specification.mec::{name}"));
    }
}

#[test]
fn only_the_three_specified_rules_use_best_choice() {
    let generated = fs::read_to_string(
        repository_root().join("src/syntax/src/document/parser/canonical/document_grammar.rs"),
    )
    .unwrap();
    assert_eq!(
        generated.matches("GrammarExpression::BestChoice").count(),
        3
    );
    for name in ["statement", "mech-code-alt", "section-element"] {
        let rule = format!("rule: rules::{},", name.replace('-', "_").to_uppercase());
        let block = generated
            .split_once(&rule)
            .unwrap_or_else(|| panic!("missing {name}"))
            .1
            .split_once("DocumentRule {")
            .map_or_else(|| generated.as_str(), |(block, _)| block);
        assert!(
            block.contains("GrammarExpression::BestChoice"),
            "{name} did not retain alt_best selection"
        );
    }
}

#[test]
fn mika_expression_inner_accepts_only_registered_triples() {
    let rule = canonical_rule_id("mika-expression-inner").unwrap();
    for text in ["ˆ◯ˆ", "ㆆ⍜ㆆ", "⌐▰◯▰", "¬◯¬"] {
        let parsed =
            parse_canonical_document_rule_for_test(source(text), rule, ParseConfig::default())
                .unwrap();
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::Matched, "{text:?}");
        assert_eq!(parsed.consumed, parsed.source.full_range(), "{text:?}");
        assert!(parsed.diagnostics.is_empty(), "{text:?}");
        for kind in [
            SyntaxKind::MikaEyeLeft,
            SyntaxKind::MikaNose,
            SyntaxKind::MikaEyeRight,
        ] {
            assert!(find(parsed.syntax(), kind).is_some(), "{text:?}: {kind:?}");
        }
    }
    for text in ["¬∘¬", "ˆ◯ಠ", "ㆆ◯ㆆ"] {
        let parsed =
            parse_canonical_document_rule_for_test(source(text), rule, ParseConfig::default())
                .unwrap();
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::NoMatch, "{text:?}");
        assert_eq!(parsed.consumed.start, parsed.consumed.end, "{text:?}");
        assert!(parsed.diagnostics.is_empty(), "{text:?}");
    }
}

#[test]
fn s7_dispositions_cover_the_exact_remaining_inventory() {
    let table =
        fs::read_to_string(repository_root().join("docs/design/grammar-audit/s7-dispositions.tsv"))
            .unwrap();
    let mut lines = table.lines();
    assert_eq!(lines.next(), Some("grammar-name\tdisposition\trationale"));
    let rows = lines
        .map(|line| line.split('\t').collect::<Vec<_>>())
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 131);
    let disposition_names = rows.iter().map(|row| row[0]).collect::<BTreeSet<_>>();
    assert_eq!(disposition_names.len(), rows.len());

    let ports =
        fs::read_to_string(repository_root().join("docs/design/grammar-audit/ports.tsv")).unwrap();
    let remaining_at_s7 = ports
        .lines()
        .skip(1)
        .filter_map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            (fields[5] == "S7" || fields[2] == "unported").then_some(fields[0])
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(disposition_names, remaining_at_s7);
    assert_eq!(
        rows.iter()
            .filter(|row| row[1] == "document-dependency")
            .count(),
        110
    );
    assert_eq!(
        rows.iter()
            .filter(|row| row[1] == "maintained-root")
            .count(),
        2
    );
    assert_eq!(
        rows.iter()
            .filter(|row| row[1] == "historical-command")
            .count(),
        17
    );
    assert_eq!(
        rows.iter()
            .filter(|row| row[1] == "outside-document-closure")
            .map(|row| row[0])
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["match-expression", "table-column"])
    );
    assert_eq!(
        rows.iter()
            .filter(|row| row[1] == "maintained-root")
            .map(|row| row[0])
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["parse", "parse-mech"])
    );
}
