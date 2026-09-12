use mech_syntax::document::{
    AstNode, CodeBlockSyntax, CodeFenceScope, DocumentId, EvalInlineMechCodeSyntax, ParseConfig,
    ParseLimits, Revision, SyntaxKind, SyntaxNode, TextSnapshot, parse_canonical_document,
    reconstruct_source, validate_lossless,
};

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(0x572), Revision(7), text).unwrap()
}

fn find<T: AstNode>(node: SyntaxNode) -> Option<T> {
    T::cast(node.clone()).or_else(|| node.children().find_map(find))
}

fn has_identifier(node: SyntaxNode, name: &str) -> bool {
    (node.kind() == SyntaxKind::Identifier && node.text().unwrap() == name)
        || node.children().any(|child| has_identifier(child, name))
}

#[test]
fn executable_fences_keep_typed_bodies_in_the_original_source() {
    for info in [
        "mech",
        "mec",
        "🤖",
        "mech:hidden",
        "mech:disabled",
        "mech:child",
    ] {
        for delimiter in ["```", "~~~"] {
            let text =
                format!("{delimiter}{info}\n~answer := 0\nanswer += 1\nanswer\n{delimiter}\n");
            let parsed = parse_canonical_document(source(&text), ParseConfig::default());
            assert!(
                parsed.diagnostics.is_empty(),
                "{text:?}: {:?}",
                parsed.diagnostics
            );
            validate_lossless(&parsed.root, &parsed.source).unwrap();
            assert_eq!(
                reconstruct_source(&parsed.root, &parsed.source).unwrap(),
                text
            );
            let fence = find::<CodeBlockSyntax>(parsed.syntax()).unwrap();
            let body = fence.mech_code().unwrap();
            assert_eq!(body.items().len(), 3);
            assert_eq!(
                body.syntax().text().unwrap(),
                "~answer := 0\nanswer += 1\nanswer\n"
            );
            assert_eq!(body.syntax().source().document(), DocumentId(0x572));
            assert_eq!(body.syntax().source().revision(), Revision(7));
            assert_eq!(
                body.syntax().range().start.0 as usize,
                delimiter.len() + info.len() + 1
            );
            assert_eq!(fence.delimiters().len(), 2);
            assert_eq!(fence.info().unwrap().is_mech(), true);
        }
    }
}

#[test]
fn inert_fences_and_empty_mech_bodies_keep_their_distinct_roles() {
    for (info, scope) in [
        ("rust", CodeFenceScope::Inert),
        ("mech", CodeFenceScope::Root),
        ("mech:disabled", CodeFenceScope::Disabled),
        ("mech:child", CodeFenceScope::Named("child".to_owned())),
    ] {
        let text = format!("```{info}\n```\n");
        let parsed = parse_canonical_document(source(&text), ParseConfig::default());
        assert!(parsed.diagnostics.is_empty());
        let fence = find::<CodeBlockSyntax>(parsed.syntax()).unwrap();
        assert_eq!(fence.info().unwrap().scope, scope);
        assert_eq!(fence.mech_code().is_some(), scope != CodeFenceScope::Inert);
    }
    let parsed = parse_canonical_document(source("Value {answer + 1}.\n"), ParseConfig::default());
    let inline = find::<EvalInlineMechCodeSyntax>(parsed.syntax()).unwrap();
    assert_eq!(
        inline.expression().unwrap().syntax().text().unwrap(),
        "answer + 1"
    );
}

#[test]
fn malformed_fence_body_cannot_consume_its_physical_closer_or_next_statement() {
    for body in ["x := [1,", "x :=", "@"] {
        let text = format!("~~~mech\n{body}\n~~~\nafter := 2\n");
        let parsed = parse_canonical_document(source(&text), ParseConfig::default());
        assert!(!parsed.diagnostics.is_empty(), "{text:?}");
        validate_lossless(&parsed.root, &parsed.source).unwrap();
        assert_eq!(
            reconstruct_source(&parsed.root, &parsed.source).unwrap(),
            text
        );
        let fence = find::<CodeBlockSyntax>(parsed.syntax()).unwrap();
        let delimiters = fence.delimiters();
        assert_eq!(delimiters.len(), 2);
        assert_eq!(delimiters[1].text().unwrap(), "~~~");
        assert!(
            !delimiters[1]
                .flags()
                .contains(mech_syntax::document::TokenFlags::SYNTHETIC)
        );
        assert!(
            has_identifier(parsed.syntax(), "after"),
            "{text:?}: {}",
            mech_syntax::document::compact_debug_tree(&parsed.syntax())
        );
        let body_start = text.find('\n').unwrap() + 1;
        let body_end = text.rfind("~~~").unwrap();
        for diagnostic in parsed.diagnostics.as_slice() {
            assert_eq!(
                diagnostic.phase,
                mech_syntax::document::DiagnosticPhase::Syntax
            );
            assert_eq!(diagnostic.severity, mech_syntax::document::Severity::Error);
            assert!(diagnostic.rule.is_some());
            assert!(diagnostic.context.is_none());
            assert!(diagnostic.found.is_some());
            assert!(diagnostic.recovery.is_some());
            let range = diagnostic
                .primary
                .resolve(parsed.source.revision(), &parsed.nodes)
                .unwrap();
            assert!((range.start.0 as usize) >= body_start, "{diagnostic:?}");
            assert!((range.end.0 as usize) <= body_end, "{diagnostic:?}");
        }
    }
}

#[test]
fn embedded_mech_shares_document_resource_limits() {
    let text = "~~~mech\n~answer := 0\nanswer += (1 + 2)\nanswer\n~~~\nafter := 3\n";
    for budget in 0..=160 {
        for events in [false, true] {
            let mut limits = ParseLimits::default();
            if events {
                limits.max_events = budget;
            } else {
                limits.fuel = u64::from(budget);
            }
            let parsed = parse_canonical_document(source(text), ParseConfig { limits });
            assert!(parsed.stats.events_emitted <= u64::from(limits.max_events));
            assert!(parsed.stats.parser_steps <= limits.fuel);
            assert!(parsed.diagnostics.len() <= limits.max_diagnostics as usize);
            validate_lossless(&parsed.root, &parsed.source).unwrap();
            assert_eq!(
                reconstruct_source(&parsed.root, &parsed.source).unwrap(),
                text
            );
        }
    }
    for nesting in 0..=12 {
        let limits = ParseLimits {
            max_nesting: nesting,
            ..ParseLimits::default()
        };
        let parsed = parse_canonical_document(source(text), ParseConfig { limits });
        validate_lossless(&parsed.root, &parsed.source)
            .unwrap_or_else(|error| panic!("nesting={nesting}: {error:?}"));
        assert_eq!(
            reconstruct_source(&parsed.root, &parsed.source).unwrap(),
            text
        );
        for diagnostic in parsed.diagnostics.as_slice() {
            assert!(
                diagnostic.rule.is_some(),
                "nesting={nesting}: {diagnostic:?}"
            );
            assert!(diagnostic.context.is_none());
            assert_eq!(
                diagnostic.phase,
                mech_syntax::document::DiagnosticPhase::Syntax
            );
            assert_eq!(diagnostic.severity, mech_syntax::document::Severity::Error);
            assert!(diagnostic.found.is_some());
            assert!(diagnostic.recovery.is_some());
            let range = diagnostic
                .primary
                .resolve(parsed.source.revision(), &parsed.nodes)
                .unwrap();
            assert!(range.end.0 as usize <= text.len());
            if nesting == 0 {
                assert_eq!(
                    diagnostic.rule,
                    Some(mech_syntax::document::parser::rules::PARSE)
                );
                assert_eq!(range.start.0, 0);
            }
        }
    }
    let malformed = "~~~mech\nx := [@ @ @ @ @\n~~~\nafter := 1\n";
    for diagnostics in 0..=2 {
        for recovery_bytes in 0..=malformed.len() as u32 {
            let limits = ParseLimits {
                max_diagnostics: diagnostics,
                max_recovery_bytes: recovery_bytes,
                ..ParseLimits::default()
            };
            let parsed = parse_canonical_document(source(malformed), ParseConfig { limits });
            assert!(parsed.diagnostics.len() <= diagnostics as usize);
            assert!(parsed.stats.recovery_bytes <= u64::from(recovery_bytes));
            assert!(parsed.stats.events_emitted <= u64::from(limits.max_events));
            assert!(parsed.stats.parser_steps <= limits.fuel);
            validate_lossless(&parsed.root, &parsed.source).unwrap();
            assert_eq!(
                reconstruct_source(&parsed.root, &parsed.source).unwrap(),
                malformed
            );
        }
    }
}

#[test]
fn piece_backed_crlf_fence_bodies_keep_physical_unicode_ranges() {
    let parts = [
        "~",
        "~~me",
        "ch\r\nword := \"e",
        "\u{301}\"\r",
        "\nword\r\n~",
        "~~\r\n",
    ];
    let text = parts.concat();
    let mut pieces = source("");
    for part in parts {
        pieces = pieces.append(part.to_owned()).unwrap();
    }
    for snapshot in [source(&text), pieces] {
        let parsed = parse_canonical_document(snapshot, ParseConfig::default());
        assert!(parsed.diagnostics.is_empty());
        validate_lossless(&parsed.root, &parsed.source).unwrap();
        assert_eq!(
            reconstruct_source(&parsed.root, &parsed.source).unwrap(),
            text
        );
        let fence = find::<CodeBlockSyntax>(parsed.syntax()).unwrap();
        let body = fence.mech_code().unwrap();
        assert_eq!(body.items().len(), 2);
        assert_eq!(body.syntax().range().start.0, 9);
        assert_eq!(
            body.syntax().text().unwrap(),
            "word := \"e\u{301}\"\r\nword\r\n"
        );
    }
}
