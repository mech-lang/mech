use mech_syntax::document::ast::ImportDeclarationSyntax;
use mech_syntax::document::parser::canonical::{
    CanonicalRuleOutcome, parse_canonical_phase_2f_rule_for_test,
};
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    AstNode, DocumentId, ParseConfig, Revision, SyntaxKind, SyntaxNode, TextSnapshot,
    lower_legacy_import_declaration,
};

fn legacy_statements(input: &str) -> Vec<mech_core::nodes::Statement> {
    let program = mech_syntax::parser::parse(input).unwrap();
    let mut statements = Vec::new();
    for section in &program.body.sections {
        for element in &section.elements {
            if let mech_core::nodes::SectionElement::MechCode(codes) = element {
                for (node, _) in codes {
                    if let mech_core::nodes::MechCode::Statement(statement) = node {
                        statements.push(statement.clone());
                    }
                }
            }
        }
    }
    statements
}

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(933), Revision(0), text).unwrap()
}

fn find_node(root: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if root.kind() == kind {
        return Some(root.clone());
    }
    root.children().find_map(|child| find_node(&child, kind))
}

#[test]
fn invalid_completed_source_wildcards_are_committed_structural_errors() {
    for input in ["+> https://x/a*b", "+> https://x/*/y", "+> https://x/**"] {
        let parsed = parse_canonical_phase_2f_rule_for_test(
            source(input),
            rules::IMPORT_DECLARATION,
            ParseConfig::default(),
        )
        .unwrap();
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed, "{input:?}");
        assert!(find_node(&parsed.syntax(), SyntaxKind::Error).is_some());
        let diagnostic = parsed.diagnostics.iter().next().unwrap();
        assert_eq!(
            diagnostic.code.as_str(),
            "syntax/invalid-source-import-wildcard"
        );
        assert!(diagnostic.fixes.is_empty());
        let declaration = ImportDeclarationSyntax::cast(
            find_node(&parsed.syntax(), SyntaxKind::ImportDeclaration).unwrap(),
        )
        .unwrap();
        assert!(lower_legacy_import_declaration(&declaration).is_err());
    }
}

#[test]
fn valid_source_wildcards_and_prefix_only_file_forms_keep_their_boundaries() {
    for input in [
        "+> dep.mec",
        "+> dep.mec/*",
        "+> https://x/path",
        "+> x://*",
        "+> x:///*",
        "+> https://x/path/*   ",
    ] {
        let parsed = parse_canonical_phase_2f_rule_for_test(
            source(input),
            rules::IMPORT_DECLARATION,
            ParseConfig::default(),
        )
        .unwrap();
        assert!(parsed.is_strictly_clean(), "{input:?}");
    }
    for (input, expected) in [
        ("+> dep.mec/*/x", "+> dep.mec/*"),
        ("+> dep.mec/**", "+> dep.mec/*"),
        ("+> dep.mec*", "+> dep.mec"),
    ] {
        let parsed = parse_canonical_phase_2f_rule_for_test(
            source(input),
            rules::IMPORT_DECLARATION,
            ParseConfig::default(),
        )
        .unwrap();
        assert!(parsed.is_strictly_clean(), "{input:?}");
        assert_eq!(parsed.source.text(parsed.consumed).unwrap(), expected);
    }
}

#[test]
fn syntax_specifier_rules_do_not_perform_declaration_wildcard_validation() {
    for rule in [
        rules::URI_SOURCE_IMPORT_SPECIFIER,
        rules::SOURCE_IMPORT_SPECIFIER,
    ] {
        let parsed = parse_canonical_phase_2f_rule_for_test(
            source("https://x/a*b"),
            rule,
            ParseConfig::default(),
        )
        .unwrap();
        assert!(parsed.is_strictly_clean(), "{rule:?}");
    }
}

#[test]
fn legacy_statement_selection_retains_source_import_and_paragraph_boundaries() {
    for input in [
        "+> dep.mec",
        "+> ./dep.mec",
        "+> ../lib/dep.mec",
        "+> /lib/dep.mec",
        "+> https://example.com/dep.mec",
        "+> memory://scratch/dep",
    ] {
        let statements = legacy_statements(input);
        assert!(matches!(
            statements.as_slice(),
            [mech_core::nodes::Statement::ImportDeclaration(_)]
        ));
    }
    assert!(legacy_statements("+> dep.mec is a source file example.").is_empty());
    assert!(matches!(
        legacy_statements("+> https://example.com/dep is a source import example.").as_slice(),
        [mech_core::nodes::Statement::ImportDeclaration(_)]
    ));
    assert!(matches!(
        legacy_statements("<+ value").as_slice(),
        [mech_core::nodes::Statement::ExportDeclaration(_)]
    ));
    assert!(matches!(
        legacy_statements("@ui := fs://workspace").as_slice(),
        [mech_core::nodes::Statement::ContextDeclaration(_)]
    ));
}

#[test]
fn wildcard_diagnostics_keep_physical_unicode_ranges_and_all_labels() {
    use mech_syntax::document::{DiagnosticAnchor, TextRange, TextSize};
    let input = "+> x://é/*/💡*\u{a0}\u{2009}";
    let parsed = parse_canonical_phase_2f_rule_for_test(
        source(input),
        rules::IMPORT_DECLARATION,
        ParseConfig::default(),
    )
    .unwrap();
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
    let diagnostics: Vec<_> = parsed.diagnostics.iter().collect();
    assert_eq!(diagnostics.len(), 1);
    let diagnostic = diagnostics[0];
    assert_eq!(
        diagnostic.primary,
        DiagnosticAnchor::Absolute {
            revision: Revision(0),
            range: TextRange::new(TextSize(10), TextSize(11)),
        }
    );
    assert_eq!(diagnostic.labels.len(), 1);
    assert_eq!(
        diagnostic.labels[0].anchor,
        DiagnosticAnchor::Absolute {
            revision: Revision(0),
            range: TextRange::new(TextSize(16), TextSize(17)),
        }
    );
    assert_eq!(diagnostic.labels[0].message, "additional wildcard");
    for suffix in ["\u{a0}\u{2009}", "\t ", "\u{3000}"] {
        let text = format!("+> x://é/*{suffix}");
        let parsed = parse_canonical_phase_2f_rule_for_test(
            source(&text),
            rules::IMPORT_DECLARATION,
            ParseConfig::default(),
        )
        .unwrap();
        assert!(parsed.is_strictly_clean(), "{text:?}");
    }
}

#[test]
fn source_import_resource_finalization_preserves_lossless_enclosing_owners() {
    use mech_syntax::document::validate_lossless_range;
    for input in [
        "+> ./dep.mec",
        "+> ../lib/dep.mec",
        "+> x://é/*",
        "+> x://é/**",
        "+> x://é/a*b.mec",
        "+> x:// ",
    ] {
        for fuel in 0..=80 {
            for max_events in [8, 16, 64, 1024] {
                let mut config = ParseConfig::default();
                config.limits.fuel = fuel;
                config.limits.max_events = max_events;
                let parsed = parse_canonical_phase_2f_rule_for_test(
                    source(input),
                    rules::IMPORT_DECLARATION,
                    config,
                )
                .unwrap();
                validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed)
                    .unwrap_or_else(|error| {
                        panic!("{input:?}, fuel {fuel}, events {max_events}: {error:?}")
                    });
            }
        }
    }
}
