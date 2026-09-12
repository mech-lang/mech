use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use mech_syntax::document::parser::canonical::{
    CanonicalRuleOutcome, parse_canonical_phase_2i_rule_for_test,
};
use mech_syntax::document::parser::{canonical_rule_id, canonical_rule_name, rules};
use mech_syntax::document::{
    AstNode, DocumentId, NodeFlags, ParseConfig, RecursiveCoreSyntax, RecursiveSyntaxNode,
    Revision, SyntaxNode, TextRange, TextSize, TextSnapshot, compact_debug_tree,
    normalize_diagnostics, reconstruct_source_range, validate_lossless_range,
};

#[derive(Debug)]
struct CertificationRow {
    name: String,
    accepted: String,
    recovery: String,
    emission_policy: String,
    syntax_kind: String,
    clean_tree_hash: u64,
    typed_access_hash: u64,
    recovery_snapshot_hash: u64,
    semantic_disposition: String,
    spec_location: String,
    conformance_cases: String,
    canonical_consumer: String,
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(0x2c8), Revision(5), text).unwrap()
}

fn certification_rows() -> Vec<CertificationRow> {
    let table = fs::read_to_string(
        repository_root().join("docs/design/grammar-audit/phase-2i-certification.tsv"),
    )
    .expect("read Phase 2I certification table");
    let mut lines = table.lines();
    assert_eq!(
        lines.next(),
        Some(
            "grammar-name\taccepted-source-json\trecovery-source-json\temission-policy\tsyntax-kind\tclean-tree-hash\ttyped-access-hash\trecovery-snapshot-hash\tsemantic-disposition\tspec-location\tconformance-cases\tcanonical-consumer"
        )
    );
    lines
        .map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            assert_eq!(fields.len(), 12, "invalid certification row: {line}");
            CertificationRow {
                name: fields[0].to_owned(),
                accepted: serde_json::from_str(fields[1]).expect("accepted source JSON"),
                recovery: serde_json::from_str(fields[2]).expect("recovery source JSON"),
                emission_policy: fields[3].to_owned(),
                syntax_kind: fields[4].to_owned(),
                clean_tree_hash: fields[5].parse().expect("clean tree hash"),
                typed_access_hash: fields[6].parse().expect("typed access hash"),
                recovery_snapshot_hash: fields[7].parse().expect("recovery snapshot hash"),
                semantic_disposition: fields[8].to_owned(),
                spec_location: fields[9].to_owned(),
                conformance_cases: fields[10].to_owned(),
                canonical_consumer: fields[11].to_owned(),
            }
        })
        .collect()
}

fn inventory_contracts() -> BTreeMap<String, (String, String, String, String)> {
    let productions =
        fs::read_to_string(repository_root().join("docs/design/grammar-audit/productions.tsv"))
            .unwrap();
    let productions = productions
        .lines()
        .skip(1)
        .map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            assert_eq!(fields.len(), 17);
            (
                fields[1].to_owned(),
                (fields[13].to_owned(), fields[14].to_owned()),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let schema = fs::read_to_string(
        repository_root().join("docs/design/grammar-audit/phase-2i-syntax-schema.tsv"),
    )
    .unwrap();
    schema
        .lines()
        .skip(1)
        .map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            assert_eq!(fields.len(), 6);
            let (spec, cases) = productions
                .get(fields[0])
                .unwrap_or_else(|| panic!("production inventory for {}", fields[0]));
            (
                fields[0].to_owned(),
                (
                    fields[2].to_owned(),
                    fields[3].to_owned(),
                    spec.clone(),
                    cases.clone(),
                ),
            )
        })
        .collect()
}

#[derive(Clone, Copy)]
struct StableHash(u64);

impl StableHash {
    const fn new() -> Self {
        Self(0xcbf29ce484222325)
    }

    fn field(&mut self, value: &str) {
        for byte in (value.len() as u64).to_le_bytes() {
            self.byte(byte);
        }
        for byte in value.bytes() {
            self.byte(byte);
        }
    }

    fn byte(&mut self, byte: u8) {
        self.0 ^= u64::from(byte);
        self.0 = self.0.wrapping_mul(0x100000001b3);
    }
}

fn visit_typed_access(node: &SyntaxNode, hash: &mut StableHash) {
    if let Some(view) = RecursiveCoreSyntax::cast(node.clone()) {
        hash.field(&format!(
            "{:?}:{}:{}:{}",
            view.syntax().kind(),
            view.syntax().range().start.0,
            view.syntax().range().end.0,
            view.syntax().flags().0
        ));
        for child in view.syntax().children() {
            hash.field(&format!(
                "child:{:?}:{}:{}:{}",
                child.kind(),
                child.range().start.0,
                child.range().end.0,
                child.flags().0
            ));
        }
        for token in view.direct_tokens() {
            hash.field(&format!(
                "token:{:?}:{}:{}:{}:{}",
                token.kind(),
                token.range().start.0,
                token.range().end.0,
                token.flags().0,
                token.text().expect("clean source token text")
            ));
        }
    }
    for child in node.children() {
        visit_typed_access(&child, hash);
    }
}

fn typed_access_hash(node: &SyntaxNode) -> u64 {
    let mut hash = StableHash::new();
    visit_typed_access(node, &mut hash);
    hash.0
}

fn recovery_snapshot_hash(
    parsed: &mech_syntax::document::parser::canonical::CanonicalSourceRuleSnapshot,
) -> u64 {
    let mut hash = StableHash::new();
    hash.field(&compact_debug_tree(&parsed.syntax()));
    for diagnostic in
        normalize_diagnostics(&parsed.diagnostics, parsed.source.revision(), &parsed.nodes)
    {
        hash.field(&format!(
            "diagnostic:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}",
            diagnostic.code,
            diagnostic.phase,
            diagnostic.severity,
            diagnostic.rule,
            diagnostic.context,
            diagnostic.primary,
            diagnostic.expected,
            diagnostic.found,
            diagnostic.related,
            diagnostic.recovery,
            diagnostic.tags,
        ));
        for label in diagnostic.labels {
            hash.field(&format!("label:{:?}", label.range));
        }
        for fix in diagnostic.fixes {
            hash.field(&format!("fix:{:?}:{:?}", fix.applicability, fix.edits));
        }
    }
    hash.0
}

#[test]
fn certification_table_executes_every_direct_accept_reject_and_recovery_case() {
    let rows = certification_rows();
    let contracts = inventory_contracts();
    assert_eq!(rows.len(), 80);
    assert_eq!(contracts.len(), rows.len());
    for row in rows {
        let contract = contracts
            .get(&row.name)
            .unwrap_or_else(|| panic!("Phase 2I schema row for {}", row.name));
        assert_eq!(&row.emission_policy, &contract.0, "{}", row.name);
        assert_eq!(&row.syntax_kind, &contract.1, "{}", row.name);
        assert_eq!(&row.spec_location, &contract.2, "{}", row.name);
        assert_eq!(&row.conformance_cases, &contract.3, "{}", row.name);
        let rule = canonical_rule_id(&row.name).expect("registered canonical rule");
        assert_eq!(canonical_rule_name(rule), Some(row.name.as_str()));

        let accepted = parse_canonical_phase_2i_rule_for_test(
            source(&row.accepted),
            rule,
            ParseConfig::default(),
        )
        .expect("Phase 2I direct dispatcher");
        assert_eq!(
            accepted.outcome,
            CanonicalRuleOutcome::Matched,
            "{}",
            row.name
        );
        assert!(accepted.is_strictly_clean(), "{}", row.name);
        assert_eq!(
            accepted.consumed,
            TextRange::new(TextSize::ZERO, TextSize(row.accepted.len() as u32)),
            "{}",
            row.name
        );
        assert_eq!(
            accepted.root.structural_hash, row.clean_tree_hash,
            "{}",
            row.name
        );
        validate_lossless_range(&accepted.root, &accepted.source, accepted.consumed).unwrap();
        assert_eq!(
            reconstruct_source_range(&accepted.root, &accepted.source, accepted.consumed).unwrap(),
            row.accepted,
            "{}",
            row.name
        );
        let typed_hash = typed_access_hash(&accepted.syntax());
        assert_eq!(typed_hash, row.typed_access_hash, "{}", row.name);

        let rejected =
            parse_canonical_phase_2i_rule_for_test(source(""), rule, ParseConfig::default())
                .expect("Phase 2I direct dispatcher");
        assert_eq!(
            rejected.outcome,
            CanonicalRuleOutcome::NoMatch,
            "{}",
            row.name
        );
        assert_eq!(rejected.consumed, TextRange::empty(TextSize::ZERO));
        assert!(rejected.diagnostics.is_empty(), "{}", row.name);

        let recovered = parse_canonical_phase_2i_rule_for_test(
            source(&row.recovery),
            rule,
            ParseConfig::default(),
        )
        .expect("Phase 2I direct dispatcher");
        assert_eq!(
            recovered.outcome,
            CanonicalRuleOutcome::Committed,
            "{}",
            row.name
        );
        assert_eq!(
            recovered.consumed,
            TextRange::new(TextSize::ZERO, TextSize(row.recovery.len() as u32)),
            "{}",
            row.name
        );
        assert!(!recovered.diagnostics.is_empty(), "{}", row.name);
        assert!(recovered.root.flags.intersects(
            NodeFlags::ERROR
                | NodeFlags::MISSING
                | NodeFlags::CONTAINS_ERROR
                | NodeFlags::CONTAINS_MISSING
        ));
        validate_lossless_range(&recovered.root, &recovered.source, recovered.consumed).unwrap();
        assert_eq!(
            reconstruct_source_range(&recovered.root, &recovered.source, recovered.consumed)
                .unwrap(),
            row.recovery,
            "{}",
            row.name
        );
        let recovery_hash = recovery_snapshot_hash(&recovered);
        assert_eq!(recovery_hash, row.recovery_snapshot_hash, "{}", row.name);

        assert!(matches!(
            row.emission_policy.as_str(),
            "node" | "conditional-node" | "transparent"
        ));
        assert!(matches!(
            row.semantic_disposition.as_str(),
            "executable" | "structural" | "compile-time"
        ));
        match row.semantic_disposition.as_str() {
            "executable" => assert_eq!(row.canonical_consumer, "engine/source-semantics"),
            "structural" => assert!(row.canonical_consumer.starts_with("syntax/typed-")),
            "compile-time" => {
                assert_eq!(
                    row.canonical_consumer,
                    "engine/source-semantics/compile-time"
                );
            }
            _ => unreachable!("closed semantic disposition"),
        }
    }
}

#[test]
fn match_001_is_a_positive_canonical_conformance_case() {
    for text in ["x? | * => 1", "x ? | * => 1", "x\t?\n| * => 1"] {
        let parsed = parse_canonical_phase_2i_rule_for_test(
            source(text),
            rules::EXPRESSION,
            ParseConfig::default(),
        )
        .unwrap();
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::Matched, "{text:?}");
        assert!(parsed.is_strictly_clean(), "{text:?}");
    }
    let corrections =
        fs::read_to_string(repository_root().join("docs/design/grammar-audit/corrections.tsv"))
            .unwrap();
    let row = corrections
        .lines()
        .find(|line| line.starts_with("MATCH-001\t"))
        .expect("MATCH-001 correction");
    assert!(row.contains("\tapplied\t"));
    for case in [
        "MATCH-001-ADJACENT",
        "MATCH-001-SPACED",
        "MATCH-001-MULTILINE",
    ] {
        assert!(row.contains(case));
    }
}

#[test]
fn certification_evidence_uses_only_canonical_authorities() {
    for relative in [
        "src/syntax/tests/canonical_phase_2i_rule_surface.rs",
        "src/syntax/tests/canonical_phase_2i_recovery.rs",
        "src/syntax/tests/canonical_phase_2i_typed_views.rs",
        "src/syntax/tests/canonical_phase_2i_ambiguity.rs",
        "src/syntax/tests/canonical_phase_2i_complexity.rs",
        "src/syntax/tests/canonical_phase_2i_piece_backed.rs",
        "src/syntax/tests/canonical_phase_2i_resource_limits.rs",
        "src/syntax/tests/canonical_phase_2i_certification.rs",
        "src/engine/tests/canonical_phase_2i_semantic_certification.rs",
    ] {
        let path = repository_root().join(relative);
        let evidence = fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!(
                "unable to read certification evidence {}: {error}",
                path.display()
            )
        });
        assert_canonical_only(&path, &evidence);
    }
}

fn assert_canonical_only(path: &Path, evidence: &str) {
    for forbidden in [
        concat!("mech_syntax::", "parser"),
        concat!("document::lower::", "legacy"),
        concat!("mech_core::", "Program"),
    ] {
        assert!(
            !evidence.contains(forbidden),
            "{} imports forbidden certification authority {forbidden}",
            path.display()
        );
    }
    assert!(!evidence.contains(concat!("lower/", "legacy")));
}
