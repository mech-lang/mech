use std::sync::Arc;

use mech_syntax::document::{
    AstNode, CodeBlockSyntax, CodeFenceScope, DiagnosticAnchor, DocumentId, DocumentSession,
    DocumentUpdate, GreenElement, GreenNode, ParseConfig, ParseLimits, SyntaxKind, SyntaxNode,
    SyntaxSnapshot, TextEdit, TextRange, TextSize, normalize_diagnostics, parse_canonical_document,
    reconstruct_source, validate_lossless,
};

fn same_tree(left: &GreenNode, right: &GreenNode) {
    assert_eq!(
        (left.kind, left.flags, left.text_len),
        (right.kind, right.flags, right.text_len)
    );
    assert_eq!(left.children.len(), right.children.len());
    for (left, right) in left.children.iter().zip(right.children.iter()) {
        match (left, right) {
            (GreenElement::Node(left), GreenElement::Node(right)) => same_tree(left, right),
            (GreenElement::Token(left), GreenElement::Token(right)) => {
                assert_eq!(
                    (left.kind, left.flags, left.text_len, left.text_hash),
                    (right.kind, right.flags, right.text_len, right.text_hash)
                );
            }
            _ => panic!("canonical tree element roles differ"),
        }
    }
}

fn check(session: &DocumentSession, config: ParseConfig, update: Option<&DocumentUpdate>) {
    fn unique_ids(
        node: &GreenNode,
        seen: &mut std::collections::BTreeSet<mech_syntax::document::SyntaxElementId>,
    ) {
        use mech_syntax::document::SyntaxElementId;
        assert!(
            seen.insert(SyntaxElementId::Node(node.id)),
            "node identity was aliased"
        );
        for child in node.children.iter() {
            match child {
                GreenElement::Node(node) => unique_ids(node, seen),
                GreenElement::Token(token) => assert!(
                    seen.insert(SyntaxElementId::Token(token.id)),
                    "token identity was aliased"
                ),
            }
        }
    }
    let snapshot = session.snapshot();
    unique_ids(&snapshot.root, &mut Default::default());
    let full = parse_canonical_document(snapshot.source.clone(), config);
    same_tree(&snapshot.root, &full.root);
    validate_lossless(&snapshot.root, &snapshot.source).unwrap();
    assert_eq!(
        reconstruct_source(&snapshot.root, &snapshot.source).unwrap(),
        snapshot.source.to_contiguous_string()
    );
    assert_eq!(
        normalize_diagnostics(&snapshot.diagnostics, snapshot.revision, &snapshot.nodes),
        normalize_diagnostics(&full.diagnostics, full.revision, &full.nodes)
    );
    for diagnostic in snapshot.diagnostics.iter() {
        for anchor in std::iter::once(&diagnostic.primary)
            .chain(diagnostic.labels.iter().map(|label| &label.anchor))
        {
            let range = anchor
                .resolve(snapshot.revision, &snapshot.nodes)
                .expect("live canonical anchor");
            snapshot
                .source
                .text(range)
                .expect("anchor uses original UTF-8 source boundaries");
            if let DiagnosticAnchor::Absolute { revision, .. } = anchor {
                assert_eq!(*revision, snapshot.revision);
            }
        }
    }
    assert_eq!(snapshot.stats.parser_steps, full.stats.parser_steps);
    assert_eq!(snapshot.stats.events_emitted, full.stats.events_emitted);
    assert_eq!(snapshot.stats.recovery_bytes, full.stats.recovery_bytes);
    assert_eq!(
        snapshot.stats.diagnostics_truncated,
        full.stats.diagnostics_truncated
    );
    assert!(snapshot.stats.parser_steps <= config.limits.fuel);
    assert!(snapshot.stats.events_emitted <= u64::from(config.limits.max_events));
    assert!(snapshot.stats.recovery_bytes <= u64::from(config.limits.max_recovery_bytes));
    assert!(snapshot.diagnostics.len() <= config.limits.max_diagnostics as usize);
    if let Some(update) = update {
        assert_eq!(
            update.stats.source_bytes,
            u64::from(snapshot.source.byte_len().0)
        );
        assert_eq!(update.stats.total_parser_steps, full.stats.parser_steps);
        assert_eq!(update.stats.total_events_emitted, full.stats.events_emitted);
        assert_eq!(update.stats.fallback_parser_steps, full.stats.parser_steps);
        assert_eq!(
            update.stats.fallback_events_emitted,
            full.stats.events_emitted
        );
        assert_eq!(
            update.stats.fragment_parser_steps
                + update.stats.validation_parser_steps
                + update.stats.rejected_parser_steps,
            0
        );
        assert_eq!(
            update.stats.fragment_events_emitted
                + update.stats.validation_events_emitted
                + update.stats.rejected_events_emitted,
            0
        );
        assert_eq!(update.stats.document_fallbacks, 1);
        assert_eq!(update.stats.attempted_roots, 1);
        assert_eq!(update.stats.reparse_root_count, 1);
        assert!(update.stats.reconciliation_steps <= update.stats.reconciliation_limit);
    }
}

fn find(snapshot: &SyntaxSnapshot, kind: SyntaxKind, text: &str) -> SyntaxNode {
    fn visit(node: SyntaxNode, kind: SyntaxKind, text: &str) -> Option<SyntaxNode> {
        if node.kind() == kind && node.text().unwrap().contains(text) {
            return Some(node);
        }
        node.children().find_map(|child| visit(child, kind, text))
    }
    visit(snapshot.syntax(), kind, text).expect("typed canonical node")
}

fn replace(session: &mut DocumentSession, old: &str, new: &str) -> DocumentUpdate {
    let text = session.snapshot().source.to_contiguous_string();
    let start = text.find(old).unwrap();
    session.apply_edits(&[TextEdit::replace(
        TextRange::new(TextSize(start as u32), TextSize((start + old.len()) as u32)),
        new,
    )])
}

#[test]
fn fence_scope_edits_use_canonical_roles_and_keep_unaffected_green_identity() {
    let config = ParseConfig::default();
    let mut session = DocumentSession::new_with_document(
        DocumentId(71),
        "Intro text 💡\n```mech\nx := (1, [2 3])\n```\n1. Stable\n---------\nstable paragraph\n",
        config,
    );
    check(&session, config, None);
    assert!(session.snapshot().is_strictly_clean());
    let stable = find(session.snapshot(), SyntaxKind::Section, "stable paragraph");
    let stable_tokens = stable
        .tokens()
        .iter()
        .map(|token| token.id())
        .collect::<Vec<_>>();
    let mut prior = "mech";
    for (tag, scope) in [
        ("text", CodeFenceScope::Inert),
        ("mech:child", CodeFenceScope::Named("child".into())),
        ("mech:disabled", CodeFenceScope::Disabled),
        ("mech:hidden", CodeFenceScope::Root),
        ("mech", CodeFenceScope::Root),
    ] {
        let update = replace(
            &mut session,
            &format!("```{prior}\n"),
            &format!("```{tag}\n"),
        );
        check(&session, config, Some(&update));
        assert!(session.snapshot().is_strictly_clean());
        let block =
            CodeBlockSyntax::cast(find(session.snapshot(), SyntaxKind::CodeBlock, "x :=")).unwrap();
        assert_eq!(block.info().unwrap().scope, scope);
        assert_eq!(block.mech_code().is_some(), scope != CodeFenceScope::Inert);
        let retained = find(session.snapshot(), SyntaxKind::Section, "stable paragraph");
        assert!(Arc::ptr_eq(stable.green(), retained.green()));
        assert_eq!(
            retained
                .tokens()
                .iter()
                .map(|token| token.id())
                .collect::<Vec<_>>(),
            stable_tokens
        );
        assert!(update.reused_roots.contains(&stable.id()));
        prior = tag;
    }
}

#[test]
fn nested_delimiters_unicode_and_open_fences_recover_to_clean_each_revision() {
    let config = ParseConfig::default();
    let mut session = DocumentSession::new(
        "```mech\nx := ((1))\n```\n1. Later\n--------\nUnicode prose 💡\n",
        config,
    );
    check(&session, config, None);
    let mut prior = "((1))";
    for (expression, clean) in [
        ("(1,,3)", false),
        ("[1 2; 3 4]", true),
        ("((1)", false),
        ("(1, 2)", true),
    ] {
        let update = replace(&mut session, prior, expression);
        check(&session, config, Some(&update));
        assert_eq!(session.snapshot().is_strictly_clean(), clean);
        prior = expression;
    }
    let update = replace(&mut session, "Unicode prose 💡", "Unicode prose ◉ ◯ 💡");
    check(&session, config, Some(&update));
    let update = replace(&mut session, "\n```\n", "\n~~~\n");
    check(&session, config, Some(&update));
    assert!(!session.snapshot().is_strictly_clean());
    let update = replace(&mut session, "\n~~~\n", "\n```\n");
    check(&session, config, Some(&update));
    assert!(session.snapshot().is_strictly_clean());
}

#[test]
fn batch_edits_retain_distinct_duplicate_nodes_and_shift_diagnostic_anchors() {
    let config = ParseConfig::default();
    let text = "first paragraph\nx := 1 +\n1. Stable\n---------\nsame paragraph\nsame paragraph\n";
    let mut session = DocumentSession::new(text, config);
    let stable = find(session.snapshot(), SyntaxKind::Section, "same paragraph");
    let diagnostic = session
        .snapshot()
        .diagnostics
        .iter()
        .next()
        .unwrap()
        .clone();
    let old_range = diagnostic
        .primary
        .resolve(session.snapshot().revision, &session.snapshot().nodes)
        .unwrap();
    let root_id = session.snapshot().root.id;
    let update = session.apply_edits(&[
        TextEdit::insert(TextSize(0), "💡 "),
        TextEdit::replace(TextRange::new(TextSize(6), TextSize(15)), "text"),
    ]);
    check(&session, config, Some(&update));
    assert_eq!(update.reparsed_roots, [root_id]);
    let retained = find(session.snapshot(), SyntaxKind::Section, "same paragraph");
    assert!(Arc::ptr_eq(stable.green(), retained.green()));
    let found = session
        .snapshot()
        .diagnostics
        .iter()
        .find(|d| d.id == diagnostic.id)
        .expect("unchanged finding identity");
    assert_eq!(
        found
            .primary
            .resolve(session.snapshot().revision, &session.snapshot().nodes),
        Some(old_range)
    );
    assert!(update.diagnostics.retained.contains(&diagnostic.id));
    assert_eq!(
        session.snapshot().nodes.node_count(),
        session
            .snapshot()
            .nodes
            .nodes()
            .map(|(id, _)| id)
            .collect::<std::collections::BTreeSet<_>>()
            .len()
    );
}

#[test]
fn resource_limited_sessions_match_one_bounded_canonical_parse_after_every_edit() {
    for fuel in [0, 32, 4_000_000] {
        for max_events in [0, 7, 64, 1_000_000] {
            for max_nesting in [0, 2, 256] {
                for max_diagnostics in [0, 2] {
                    let config = ParseConfig {
                        limits: ParseLimits {
                            fuel,
                            max_events,
                            max_nesting,
                            max_diagnostics,
                            max_recovery_bytes: 16,
                        },
                    };
                    let mut session = DocumentSession::new(
                        "```mech\nx := (((1)))\ny := (1,,3)\n```\nProse 💡\n",
                        config,
                    );
                    check(&session, config, None);
                    for (old, new) in [
                        ("(1,,3)", "(1, 3)"),
                        ("(((1)))", "((2))"),
                        ("Prose 💡", "Prose ◉"),
                    ] {
                        let update = replace(&mut session, old, new);
                        check(&session, config, Some(&update));
                    }
                }
            }
        }
    }
}

#[test]
fn no_op_and_invalid_edits_preserve_the_canonical_snapshot() {
    let config = ParseConfig::default();
    let mut session = DocumentSession::new("Unicode 💡\nx := 1\n", config);
    let root = Arc::clone(&session.snapshot().root);
    let revision = session.snapshot().revision;
    let update = session.apply_edits(&[]);
    assert_eq!(update.stats, Default::default());
    assert_eq!(update.old_revision, update.new_revision);
    let interior = "Unicode ".len() + 1;
    assert!(
        session
            .try_apply_edits(&[TextEdit::insert(TextSize(interior as u32), "bad")])
            .is_err()
    );
    assert!(
        session
            .try_apply_edits(&[
                TextEdit::insert(TextSize(5), "later"),
                TextEdit::insert(TextSize(1), "earlier")
            ])
            .is_err()
    );
    assert_eq!(session.snapshot().revision, revision);
    assert!(Arc::ptr_eq(&session.snapshot().root, &root));
    check(&session, config, None);
}

#[test]
fn identical_statements_in_distinct_scopes_never_share_or_steal_identities() {
    let config = ParseConfig::default();
    let mut session = DocumentSession::new(
        "```mech:left\nx := 1\n```\n```mech:right\nx := 1\n```\n",
        config,
    );
    let definitions = |snapshot: &SyntaxSnapshot| {
        fn collect(node: SyntaxNode, result: &mut Vec<SyntaxNode>) {
            if node.kind() == SyntaxKind::VariableDefine {
                result.push(node);
            } else {
                for child in node.children() {
                    collect(child, result);
                }
            }
        }
        let mut result = Vec::new();
        collect(snapshot.syntax(), &mut result);
        result
    };
    let original = definitions(session.snapshot());
    assert_eq!(original.len(), 2);
    assert_ne!(original[0].id(), original[1].id());
    for (old, new) in [("x := 1", "x := 2"), ("x := 2", "x := 1")] {
        let update = replace(&mut session, old, new);
        check(&session, config, Some(&update));
        let current = definitions(session.snapshot());
        assert_eq!(current.len(), 2);
        assert_ne!(current[0].id(), original[0].id());
        assert_ne!(current[0].id(), original[1].id());
        assert!(Arc::ptr_eq(current[1].green(), original[1].green()));
    }
}
