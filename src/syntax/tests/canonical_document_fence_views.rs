use mech_syntax::document::{
    AstNode, CodeBlockSyntax, CodeFenceScope, DocumentId, DocumentStream, EvalInlineMechCodeSyntax,
    ParseConfig, ParseLimits, Revision, SyntaxKind, SyntaxNode, SyntaxSnapshot, TextSnapshot,
    parse_canonical_document, reconstruct_source, validate_lossless,
};
#[path = "support/document_stream.rs"]
mod support;

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

fn comment_fence(delimiter: &str, comment: &str) -> (String, String) {
    let body = format!("~answer := 0\nanswer += 2 -- {comment}\nanswer\n");
    let text = format!(
        "{delimiter}mech\n{body}{delimiter}\n\nDistinct prose 💡 displays {{answer + 7}}.\n\nafter := 41\nafter\n"
    );
    (text, body)
}

fn assert_comment_fence(snapshot: &SyntaxSnapshot, text: &str, body_text: &str, delimiter: &str) {
    assert!(
        snapshot.diagnostics.is_empty(),
        "{text:?}: {:?}",
        snapshot.diagnostics
    );
    validate_lossless(&snapshot.root, &snapshot.source).unwrap();
    assert_eq!(
        reconstruct_source(&snapshot.root, &snapshot.source).unwrap(),
        text
    );
    let fence = find::<CodeBlockSyntax>(snapshot.syntax()).unwrap();
    let body = fence.mech_code().unwrap();
    assert_eq!(body.items().len(), 3);
    assert_eq!(body.syntax().text().unwrap(), body_text);
    assert_eq!(
        body.syntax().range().end.0 as usize,
        text.find(&format!("\n{delimiter}\n")).unwrap() + 1
    );
    let delimiters = fence.delimiters();
    assert_eq!(delimiters.len(), 2);
    assert_eq!(delimiters[1].text().unwrap(), delimiter);
    assert!(
        !delimiters[1]
            .flags()
            .contains(mech_syntax::document::TokenFlags::SYNTHETIC)
    );
    assert!(has_identifier(snapshot.syntax(), "after"));

    fn expressions(node: SyntaxNode, values: &mut Vec<String>) {
        if let Some(inline) = EvalInlineMechCodeSyntax::cast(node.clone()) {
            values.push(inline.expression().unwrap().syntax().text().unwrap());
        } else {
            for child in node.children() {
                expressions(child, values);
            }
        }
    }
    let mut values = Vec::new();
    expressions(snapshot.syntax(), &mut values);
    let expected = if body_text.contains("{ans}") {
        vec!["ans", "ans + 1", "answer + 7"]
    } else {
        vec!["answer + 7"]
    };
    assert_eq!(values, expected);
}

#[test]
fn comments_in_executable_fences_preserve_the_closer_prose_and_inline_owners() {
    for delimiter in ["```", "~~~"] {
        for comment in [
            "plain",
            "**Count** [docs](https://mech-lang.org) `literal` {{answer + 99}}: {ans}, {ans + 1}.",
        ] {
            let (text, body) = comment_fence(delimiter, comment);
            let snapshot = parse_canonical_document(source(&text), ParseConfig::default());
            assert_comment_fence(&snapshot, &text, &body, delimiter);
        }
    }
}

#[test]
fn streamed_comments_in_executable_fences_keep_local_frontiers_at_every_scalar_cut() {
    for delimiter in ["```", "~~~"] {
        let (text, body) = comment_fence(delimiter, "`literal` {{answer + 99}}: {ans}, {ans + 1}.");
        let expected = parse_canonical_document(source(&text), ParseConfig::default());
        for allowance in [13, 1] {
            for cut in text
                .char_indices()
                .map(|(at, _)| at)
                .chain(core::iter::once(text.len()))
            {
                let mut stream = DocumentStream::new(DocumentId(0x572), ParseConfig::default());
                for chunk in [&text[..cut], &text[cut..]] {
                    support::append(&mut stream, chunk, allowance);
                }
                let snapshot = support::finish(&mut stream, allowance);
                assert_comment_fence(&snapshot, &text, &body, delimiter);
                support::equivalent_to(&snapshot, &text, &expected);
            }
        }
    }
}

#[test]
fn streamed_minimal_fence_comments_make_progress_at_unit_allowance() {
    for text in [
        "answer := 1 -- plain\nanswer\n",
        "```mech\nanswer := 1 -- plain\nanswer\n```\n",
        "```mech\nanswer := 1 -- `literal`\nanswer\n```\n",
        "```mech\nanswer := 1 -- {{answer + 99}}\nanswer\n```\n",
        "```mech\nanswer := 1 -- {ans}\nanswer\n```\n",
    ] {
        let mut stream = DocumentStream::new(DocumentId(0x572), ParseConfig::default());
        support::append(&mut stream, text, 1);
        let snapshot = support::finish(&mut stream, 1);
        assert!(snapshot.is_strictly_clean());
        support::equivalent(&snapshot, text);
    }
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

#[test]
fn fence_presentation_uses_typed_options_and_shared_string_decoding() {
    for value in ["false", "no", "off", "\"0\"", "\"OFF\""] {
        let text = format!(
            "```mech:worker{{output: {value}, color: red, border: \"1px solid red\"}}\nx := 1\n```\n"
        );
        let parsed = parse_canonical_document(source(&text), ParseConfig::default());
        assert!(
            parsed.diagnostics.is_empty(),
            "{text}: {:?}",
            parsed.diagnostics
        );
        let fence = find::<CodeBlockSyntax>(parsed.syntax()).unwrap();
        let presentation = fence.presentation().unwrap();
        assert!(!presentation.show_output);
        assert_eq!(
            presentation.styles,
            vec![
                ("color".into(), "red".into()),
                ("border".into(), "1px solid red".into())
            ]
        );
        assert_eq!(
            fence.info().unwrap().scope,
            CodeFenceScope::Named("worker".into())
        );
    }
    let text = "```mech{output: true, label: \"a\\n\\u{1f4a1}\"}\nx := 1\n```\n";
    let parsed = parse_canonical_document(source(text), ParseConfig::default());
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let presentation = find::<CodeBlockSyntax>(parsed.syntax())
        .unwrap()
        .presentation()
        .unwrap();
    assert!(presentation.show_output);
    assert_eq!(presentation.styles, vec![("label".into(), "a\n💡".into())]);

    let text = "```mech:hidden{output: true, color: red}\nx := 1\n```\n";
    let parsed = parse_canonical_document(source(text), ParseConfig::default());
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let presentation = find::<CodeBlockSyntax>(parsed.syntax())
        .unwrap()
        .presentation()
        .unwrap();
    assert!(!presentation.show_output);
    assert_eq!(presentation.styles, vec![("color".into(), "red".into())]);
}

#[test]
fn fence_information_normalizes_optional_colon_and_repeated_prefixes() {
    use mech_syntax::document::CodeFenceInfo;
    for prefix in ["mech", "mec", "🤖", "mechmechmecmec🤖🤖"] {
        for separator in ["", ":"] {
            for (name, scope, hidden) in [
                ("", CodeFenceScope::Root, false),
                ("hidden", CodeFenceScope::Root, true),
                ("disabled", CodeFenceScope::Disabled, false),
                ("worker", CodeFenceScope::Named("worker".into()), false),
            ] {
                let info = format!("{prefix}{separator}{name}");
                // `mec` + `hidden` spells `mechidden`: the longer `mech`
                // language prefix wins, leaving the namespace `idden`.
                let (scope, hidden) = if info == "mechidden" {
                    (CodeFenceScope::Named("idden".into()), false)
                } else {
                    (scope, hidden)
                };
                assert_eq!(
                    CodeFenceInfo::from_info_string(&info),
                    CodeFenceInfo {
                        scope: scope.clone(),
                        hidden
                    }
                );
                let parsed = parse_canonical_document(
                    source(&format!("```{info}\nx := 1\n```\n")),
                    ParseConfig::default(),
                );
                assert!(
                    parsed.diagnostics.is_empty(),
                    "{info}: {:?}",
                    parsed.diagnostics
                );
                let fence = find::<CodeBlockSyntax>(parsed.syntax()).unwrap();
                assert_eq!(fence.info().unwrap().scope, scope);
                assert_eq!(fence.mech_code().unwrap().items().len(), 1);
            }
        }
    }
}

#[test]
fn configured_crlf_fences_keep_information_and_decodable_option_values() {
    for newline in ["\n", "\r\n"] {
        for prefix in ["mechworker", "mech:worker"] {
            let info = format!("{prefix}{{output: false, label: \"a\\n\"}}");
            let text = format!("```{info}{newline}x := 1{newline}```{newline}");
            let parsed = parse_canonical_document(source(&text), ParseConfig::default());
            assert!(parsed.is_strictly_clean());
            let fence = find::<CodeBlockSyntax>(parsed.syntax()).unwrap();
            assert_eq!(
                parsed.source.text(fence.info_range().unwrap()).unwrap(),
                info
            );
            assert_eq!(
                fence.info().unwrap().scope,
                CodeFenceScope::Named("worker".into())
            );
            let presentation = fence.presentation().unwrap();
            assert!(!presentation.show_output);
            assert_eq!(presentation.styles, vec![("label".into(), "a\n".into())]);
        }
    }
}

#[test]
fn recovered_fence_body_keeps_header_presentation_and_later_document_nodes() {
    use mech_syntax::document::{NodeFlags, TextRange, TextSize};
    let text = include_str!("fixtures/document/recovery/fenced-unclosed-matrix.mec");
    let expected = parse_canonical_document(source(text), ParseConfig::default());
    assert!(!expected.is_strictly_clean());
    let fence = find::<CodeBlockSyntax>(expected.syntax()).unwrap();
    assert!(fence.presentation().is_some());
    assert_eq!(fence.delimiters().len(), 2);
    assert!(
        fence
            .mech_code()
            .unwrap()
            .syntax()
            .flags()
            .contains(NodeFlags::CONTAINS_MISSING)
    );
    let diagnostic = expected.diagnostics.iter().next().unwrap();
    assert_eq!(diagnostic.code.as_str(), "syntax/missing-delimiter");
    let opening = text.find('[').unwrap() as u32;
    assert_eq!(
        diagnostic
            .primary
            .resolve(expected.revision, &expected.nodes),
        Some(TextRange::new(TextSize(opening), TextSize(opening + 1)))
    );
    let boundary = text.rfind("```\n").unwrap() as u32;
    assert_eq!(
        diagnostic.labels[0]
            .anchor
            .resolve(expected.revision, &expected.nodes),
        Some(TextRange::empty(TextSize(boundary)))
    );
    assert_eq!(
        diagnostic.fixes[0].edits[0].delete,
        TextRange::empty(TextSize(boundary))
    );
    assert_eq!(
        find::<mech_syntax::document::UlSubtitleSyntax>(expected.syntax())
            .unwrap()
            .syntax()
            .flags(),
        NodeFlags::NONE
    );

    for split in 0..=text.len() {
        let mut stream = DocumentStream::new(DocumentId(0x572), ParseConfig::default());
        support::append(&mut stream, &text[..split], 31);
        support::append(&mut stream, &text[split..], 31);
        support::equivalent_to(&support::finish(&mut stream, 31), text, &expected);
    }
}

#[test]
fn recovered_bodies_preserve_options_while_malformed_options_remain_invalid() {
    for body in ["x := [1 2\nx\n", "x := [1, +, 2]\n", "x :=\n"] {
        let text = format!("```mech{{output: false, color: red}}\n{body}```\n");
        let parsed = parse_canonical_document(source(&text), ParseConfig::default());
        assert!(!parsed.is_strictly_clean());
        let presentation = find::<CodeBlockSyntax>(parsed.syntax())
            .unwrap()
            .presentation()
            .unwrap();
        assert!(!presentation.show_output);
        assert_eq!(presentation.styles, vec![("color".into(), "red".into())]);
    }
    let text = "```mech{output: }\nx := 1\n```\n";
    let parsed = parse_canonical_document(source(text), ParseConfig::default());
    assert!(!parsed.is_strictly_clean());
    assert!(
        find::<CodeBlockSyntax>(parsed.syntax())
            .unwrap()
            .presentation()
            .is_none()
    );
}

#[test]
fn unfenced_delimiter_recovery_preserves_section_and_statement_restarts() {
    for body in [
        "answer := [1 2 3\n",
        "answer := (1 + 2\n",
        "answer := {x: 1 y: 2\n",
    ] {
        let text = format!("{body}\n1. Section One\n---\n\nLater prose.\n");
        let expected = parse_canonical_document(source(&text), ParseConfig::default());
        let section = find::<mech_syntax::document::UlSubtitleSyntax>(expected.syntax()).unwrap();
        assert!(section.syntax().text().unwrap().contains("Section One"));
        assert_eq!(
            section.syntax().flags(),
            mech_syntax::document::NodeFlags::NONE
        );
        assert!(expected.diagnostics.iter().all(|d| {
            d.primary
                .resolve(expected.revision, &expected.nodes)
                .unwrap()
                .end
                .0 as usize
                <= body.len()
        }));
        for split in 0..=text.len() {
            let mut stream = DocumentStream::new(DocumentId(0x572), ParseConfig::default());
            support::append(&mut stream, &text[..split], 31);
            support::append(&mut stream, &text[split..], 31);
            support::equivalent_to(&support::finish(&mut stream, 31), &text, &expected);
        }
    }
    for definition in ["next := 7", "next⟨i64⟩ := 7", "~next := 7", "next = 7"] {
        let text = format!("answer := [1 2 3\n{definition}\nnext\n");
        let parsed = parse_canonical_document(source(&text), ParseConfig::default());
        assert!(has_identifier(parsed.syntax(), "next"));
        let code = find::<mech_syntax::document::MechCodeSyntax>(parsed.syntax()).unwrap();
        assert_eq!(
            code.items().len(),
            3,
            "{}",
            mech_syntax::document::compact_debug_tree(&parsed.syntax())
        );
        assert_eq!(parsed.diagnostics.len(), 1);
        for split in text
            .char_indices()
            .map(|(at, _)| at)
            .chain(core::iter::once(text.len()))
        {
            let mut stream = DocumentStream::new(DocumentId(0x572), ParseConfig::default());
            support::append(&mut stream, &text[..split], 31);
            support::append(&mut stream, &text[split..], 31);
            support::equivalent_to(&support::finish(&mut stream, 31), &text, &parsed);
        }
    }
}

#[test]
fn restart_probes_preserve_valid_multiline_source() {
    for body in [
        "answer := [1 2;\n3 4]\n",
        "answer := [1 2;\n\n3 4]\n",
        "x := 1\nanswer := [1 2; x 4]\n",
        "x := 1\nanswer := [x == 1; x == 2]\n",
        "x := 1\nanswer := [x == 1]\n",
        "answer := {x: 1\ny: 2}\n",
        "answer := (1,\n2)\n",
    ] {
        let text = format!("{body}\n1. Section One\n---\n\nLater prose.\n");
        let parsed = parse_canonical_document(source(&text), ParseConfig::default());
        assert!(
            parsed.is_strictly_clean(),
            "{text}: {:?}",
            parsed.diagnostics
        );
        for split in 0..=text.len() {
            let mut stream = DocumentStream::new(DocumentId(0x572), ParseConfig::default());
            support::append(&mut stream, &text[..split], 17);
            support::append(&mut stream, &text[split..], 17);
            support::equivalent_to(&support::finish(&mut stream, 17), &text, &parsed);
        }
    }
}

#[test]
fn recovery_attaches_after_statement_separators_and_at_section_prefixes() {
    for body in [
        "answer := [1 2; next := 7; next\n",
        "answer := [1 2\n  next := 7\nnext\n",
        "answer := [1 @\nnext := 7\nnext\n",
        "answer := [1 @; next := 7; next\n",
    ] {
        let parsed = parse_canonical_document(source(body), ParseConfig::default());
        let code = find::<mech_syntax::document::MechCodeSyntax>(parsed.syntax()).unwrap();
        assert_eq!(
            code.items().len(),
            3,
            "{body}: {}",
            mech_syntax::document::compact_debug_tree(&parsed.syntax())
        );
        assert!(!parsed.is_strictly_clean());
        validate_lossless(&parsed.root, &parsed.source).unwrap();
        for split in 0..=body.len() {
            let mut stream = DocumentStream::new(DocumentId(0x572), ParseConfig::default());
            support::append(&mut stream, &body[..split], 17);
            support::append(&mut stream, &body[split..], 17);
            support::equivalent_to(&support::finish(&mut stream, 17), body, &parsed);
        }
    }
    for (element, kind) in [
        ("(i)> Later information.\n", SyntaxKind::InfoBlock),
        ("(?)> Later question.\n", SyntaxKind::QuestionBlock),
        ("(!)> Later warning.\n", SyntaxKind::WarningBlock),
        ("![Later image](image.png)\n", SyntaxKind::Img),
        ("- Later list item\n", SyntaxKind::MechdownList),
        ("```mech\nnext := 7\n```\n", SyntaxKind::CodeBlock),
    ] {
        let text = format!("answer := [1 2\n\n{element}");
        let parsed = parse_canonical_document(source(&text), ParseConfig::default());
        fn clean_node(node: SyntaxNode, kind: SyntaxKind) -> bool {
            (node.kind() == kind && node.flags() == mech_syntax::document::NodeFlags::NONE)
                || node.children().any(|child| clean_node(child, kind))
        }
        assert!(
            clean_node(parsed.syntax(), kind),
            "{text}: {}",
            mech_syntax::document::compact_debug_tree(&parsed.syntax())
        );
        validate_lossless(&parsed.root, &parsed.source).unwrap();
        for split in 0..=text.len() {
            let mut stream = DocumentStream::new(DocumentId(0x572), ParseConfig::default());
            support::append(&mut stream, &text[..split], 17);
            support::append(&mut stream, &text[split..], 17);
            support::equivalent_to(&support::finish(&mut stream, 17), &text, &parsed);
        }
    }
}
