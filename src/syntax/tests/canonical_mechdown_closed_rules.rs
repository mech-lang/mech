use mech_syntax::document::ast::mechdown::{BlankLineSyntax, CommentSyntax};
use mech_syntax::document::parser::canonical::parse_canonical_mechdown_rule_for_test;
use mech_syntax::document::parser::{canonical_rule_name, rules};
use mech_syntax::document::{
    AstNode, DocumentId, ExpectedSyntax, FixApplicability, FoundSyntax, NodeFlags, ParseConfig,
    RecoveryAction, Revision, RuleId, SyntaxKind, TextRange, TextSize, TextSnapshot, TokenFlags,
    reconstruct_source_range, validate_lossless_range,
};

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(201), Revision(0), text).unwrap()
}

fn parse(
    text: &str,
    rule: RuleId,
) -> mech_syntax::document::parser::canonical::CanonicalMechdownRuleSnapshot {
    parse_canonical_mechdown_rule_for_test(source(text), rule, ParseConfig::default())
        .unwrap_or_else(|| {
            panic!(
                "{} is not a Phase 2B rule",
                canonical_rule_name(rule).unwrap()
            )
        })
}

fn assert_contract(rule: RuleId, accepted: &[&str], rejected: &[&str]) {
    for input in accepted {
        let parsed = parse(input, rule);
        assert!(parsed.matched, "{rule:?} did not accept {input:?}");
        assert!(parsed.diagnostics.is_empty(), "{rule:?} on {input:?}");
        assert_eq!(parsed.consumed.start, TextSize::ZERO, "{input:?}");
        assert!(!parsed.consumed.is_empty(), "{rule:?} on {input:?}");
        validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed).unwrap();
        assert_eq!(
            reconstruct_source_range(&parsed.root, &parsed.source, parsed.consumed).unwrap(),
            &input[..parsed.consumed.end.0 as usize],
            "{rule:?} did not preserve its consumed prefix for {input:?}",
        );
    }
    for input in rejected {
        let parsed = parse(input, rule);
        assert!(!parsed.matched, "{rule:?} unexpectedly accepted {input:?}");
        assert_eq!(parsed.consumed, TextRange::empty(TextSize::ZERO));
        assert!(parsed.diagnostics.is_empty(), "{rule:?} on {input:?}");
    }
}

#[test]
fn all_13_closed_rules_enforce_their_canonical_acceptance_contracts() {
    assert_contract(
        rules::COMMENT_SIGIL,
        &["--tail", "//tail"],
        &["-tail", "/tail"],
    );
    assert_contract(
        rules::COMMENT,
        &[
            "--",
            "// text",
            " \t-- text\nnext",
            " \t// text\rnext",
            " \t// text\r\nnext",
            "\u{00a0}// text",
            "\u{2009}// text",
        ],
        &["not a comment"],
    );
    assert_contract(
        rules::CODEBLOCK_SIGIL,
        &["```text", "~~~text"],
        &["``text", "~~text"],
    );
    assert_contract(
        rules::INLINE_CODE,
        &["`text`tail", "``tail", "`x := 1`tail"],
        &["```text```"],
    );
    assert_contract(
        rules::INLINE_EQUATION,
        &["$$x$$tail", "$$\\alpha$$tail"],
        &["not an equation"],
    );
    assert_contract(
        rules::RAW_HYPERLINK,
        &[
            "http://example.com",
            "http://example.com/path tail",
            "http://example.com\tpath\nnext",
            "https",
        ],
        &[],
    );
    assert_contract(
        rules::FOOTNOTE_REFERENCE,
        &["[^note]tail", "[^a b]tail"],
        &["note"],
    );
    assert_contract(
        rules::REFERENCE,
        &["[abc]tail", "[abc](target)", "[123]tail"],
        &["[a-b]", "[]"],
    );
    assert_contract(
        rules::SECTION_REFERENCE,
        &["§1.2 tail", "§abc-tail"],
        &["plain"],
    );
    assert_contract(
        rules::PARAGRAPH_TEXT,
        &["plain prose", "punctuation, emoji 🧪", "plain§next"],
        &[],
    );
    assert_contract(
        rules::THEMATIC_BREAK,
        &["*\nnext", "*** \t\rnext", "**\r\nnext"],
        &["plain\n"],
    );
    assert_contract(
        rules::BLANK_LINE,
        &["\nnext", " \t\rnext", "\u{00a0}\r\nnext"],
        &["plain"],
    );
    assert_contract(
        rules::EQUATION,
        &["$$x+y\nnext", "$$\\alpha\nnext", "$$x$$\nnext"],
        &["plain"],
    );
}

#[test]
fn line_rules_require_a_physical_newline_and_never_materialize_one() {
    for (rule, input, kind, matched) in [
        (rules::BLANK_LINE, "", SyntaxKind::BlankLine, false),
        (rules::BLANK_LINE, " \t", SyntaxKind::BlankLine, false),
        (rules::BLANK_LINE, "\n", SyntaxKind::BlankLine, true),
        (rules::BLANK_LINE, " \t\r", SyntaxKind::BlankLine, true),
        (rules::THEMATIC_BREAK, "*", SyntaxKind::ThematicBreak, false),
        (
            rules::THEMATIC_BREAK,
            "*** \t",
            SyntaxKind::ThematicBreak,
            false,
        ),
        (
            rules::THEMATIC_BREAK,
            "*\n",
            SyntaxKind::ThematicBreak,
            true,
        ),
        (
            rules::THEMATIC_BREAK,
            "*** \r",
            SyntaxKind::ThematicBreak,
            true,
        ),
    ] {
        let parsed = parse(input, rule);
        assert_eq!(parsed.matched, matched, "{input:?}");
        assert!(parsed.diagnostics.is_empty(), "{input:?}");
        if !matched {
            assert_eq!(
                parsed.consumed,
                TextRange::empty(TextSize::ZERO),
                "{input:?}"
            );
            continue;
        }

        assert_eq!(parsed.consumed, source(input).full_range(), "{input:?}");
        let node = parsed.syntax().first_child(kind).unwrap();
        assert!(
            !node
                .tokens()
                .into_iter()
                .any(|token| token.flags().contains(TokenFlags::SYNTHETIC)),
            "{input:?}",
        );
        assert_eq!(
            reconstruct_source_range(&parsed.root, &parsed.source, parsed.consumed).unwrap(),
            input,
            "{input:?}",
        );
    }
}

#[test]
fn comments_preserve_paragraph_content_and_leave_the_newline_unconsumed() {
    for (input, raw) in [
        ("--", ""),
        ("// text", " text"),
        ("  -- text\nx", " text"),
        ("  -- text\rx", " text"),
        ("\t// text\r\nx", " text"),
    ] {
        let parsed = parse(input, rules::COMMENT);
        assert!(parsed.is_strictly_clean(), "{input:?}");
        assert_eq!(parsed.rule, rules::COMMENT, "{input:?}");
        let comment = parsed
            .syntax()
            .first_child(SyntaxKind::Comment)
            .unwrap_or_else(|| panic!("missing Comment for {input:?}"));
        assert!(CommentSyntax::cast(comment.clone()).is_some(), "{input:?}");
        assert_eq!(
            comment.text().unwrap(),
            &input[..parsed.consumed.end.0 as usize]
        );
        assert!(comment.text().unwrap().ends_with(raw), "{input:?}");
        assert!(
            !comment
                .flags()
                .intersects(NodeFlags::CONTAINS_ERROR | NodeFlags::CONTAINS_MISSING),
            "{input:?}",
        );
        assert!(
            comment
                .children()
                .all(|child| { !matches!(child.kind(), SyntaxKind::Error | SyntaxKind::Missing) }),
            "{input:?}",
        );
        assert!(
            comment.tokens().into_iter().all(|token| {
                !token
                    .flags()
                    .intersects(TokenFlags::ERROR | TokenFlags::MISSING | TokenFlags::SYNTHETIC)
            }),
            "{input:?}",
        );
        validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed).unwrap();
    }

    for (input, remainder) in [
        ("  -- text\nx", "\nx"),
        ("  -- text\rx", "\rx"),
        ("\t// text\r\nx", "\r\nx"),
    ] {
        let parsed = parse(input, rules::COMMENT);
        assert_eq!(
            parsed
                .source
                .text(TextRange::new(
                    parsed.consumed.end,
                    parsed.source.byte_len()
                ))
                .unwrap(),
            remainder,
            "{input:?}"
        );
    }
}

#[test]
fn comment_and_blank_line_have_their_declared_typed_views() {
    let comment = parse("--", rules::COMMENT)
        .syntax()
        .first_child(SyntaxKind::Comment)
        .unwrap();
    assert!(CommentSyntax::cast(comment).is_some());

    let blank = parse("\n", rules::BLANK_LINE)
        .syntax()
        .first_child(SyntaxKind::BlankLine)
        .unwrap();
    assert!(BlankLineSyntax::cast(blank).is_some());
}

#[derive(Clone, Copy)]
enum ExpectedDescriptor {
    Token(SyntaxKind),
    Production(&'static str),
}

impl ExpectedDescriptor {
    fn syntax(self) -> ExpectedSyntax {
        match self {
            Self::Token(kind) => ExpectedSyntax::Token(kind),
            Self::Production(name) => ExpectedSyntax::Production(name.into()),
        }
    }
}

#[derive(Clone, Copy)]
struct RecoveryExpectation {
    code: &'static str,
    expected: ExpectedDescriptor,
    found_kind: SyntaxKind,
    found_text: Option<&'static str>,
    at: usize,
    machine_fix: Option<&'static str>,
}

fn assert_structured_recovery(
    rule: RuleId,
    input: &str,
    kind: SyntaxKind,
    expectations: &[RecoveryExpectation],
) {
    let parsed = parse(input, rule);
    assert!(parsed.matched, "{input:?}");
    assert_eq!(parsed.rule, rule, "{input:?}");
    assert_eq!(
        parsed.syntax().kind(),
        SyntaxKind::CanonicalFragment,
        "{input:?}"
    );
    assert!(parsed.syntax().first_child(kind).is_some(), "{input:?}");
    assert_eq!(parsed.diagnostics.len(), expectations.len(), "{input:?}");
    assert!(
        parsed
            .root
            .flags
            .intersects(NodeFlags::CONTAINS_MISSING | NodeFlags::MISSING),
        "{input:?}",
    );

    for (diagnostic, expectation) in parsed.diagnostics.iter().zip(expectations) {
        let expected = expectation.expected.syntax();
        let at = TextSize(expectation.at as u32);
        assert_eq!(diagnostic.rule, Some(rule), "{input:?}");
        assert_eq!(diagnostic.context, None, "{input:?}");
        assert_eq!(diagnostic.code.as_str(), expectation.code, "{input:?}");
        assert_eq!(
            diagnostic.expected.as_slice(),
            &[expected.clone()],
            "{input:?}"
        );
        assert_eq!(
            diagnostic.found,
            Some(FoundSyntax {
                kind: Some(expectation.found_kind),
                text: expectation.found_text.map(str::to_owned),
            }),
            "{input:?}",
        );
        assert_eq!(
            diagnostic
                .primary
                .resolve(parsed.source.revision(), &parsed.nodes),
            Some(TextRange::empty(at)),
            "{input:?}",
        );
        assert_eq!(
            diagnostic.recovery,
            Some(RecoveryAction::Insert {
                syntax: expected,
                at,
            }),
            "{input:?}",
        );
        match expectation.machine_fix {
            Some(insert) => assert!(
                diagnostic.fixes.iter().any(|fix| {
                    fix.applicability == FixApplicability::MachineApplicable
                        && fix.edits.iter().any(|edit| {
                            edit.delete == TextRange::empty(at) && edit.insert == insert
                        })
                }),
                "{input:?} is missing its safe insertion"
            ),
            None => assert!(diagnostic.fixes.is_empty(), "{input:?}"),
        }
    }

    validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed).unwrap();
    assert_eq!(
        reconstruct_source_range(&parsed.root, &parsed.source, parsed.consumed).unwrap(),
        &input[..parsed.consumed.end.0 as usize],
        "{input:?}",
    );
}

#[test]
fn malformed_committed_forms_have_structured_source_fragment_recovery() {
    assert_structured_recovery(
        rules::INLINE_CODE,
        "`text\nheading",
        SyntaxKind::InlineCode,
        &[RecoveryExpectation {
            code: "syntax/unclosed-inline-code",
            expected: ExpectedDescriptor::Token(SyntaxKind::Grave),
            found_kind: SyntaxKind::Newline,
            found_text: Some("\n"),
            at: 5,
            machine_fix: Some("`"),
        }],
    );
    assert_structured_recovery(
        rules::INLINE_EQUATION,
        "$$$$",
        SyntaxKind::InlineEquation,
        &[RecoveryExpectation {
            code: "syntax/missing-inline-equation-content",
            expected: ExpectedDescriptor::Production("inline equation content"),
            found_kind: SyntaxKind::EquationSigil,
            found_text: Some("$$"),
            at: 2,
            machine_fix: None,
        }],
    );
    assert_structured_recovery(
        rules::INLINE_EQUATION,
        "$$x\nheading",
        SyntaxKind::InlineEquation,
        &[RecoveryExpectation {
            code: "syntax/unclosed-inline-equation",
            expected: ExpectedDescriptor::Token(SyntaxKind::EquationSigil),
            found_kind: SyntaxKind::Newline,
            found_text: Some("\n"),
            at: 3,
            machine_fix: Some("$$"),
        }],
    );
    assert_structured_recovery(
        rules::FOOTNOTE_REFERENCE,
        "[^]",
        SyntaxKind::FootnoteReference,
        &[RecoveryExpectation {
            code: "syntax/missing-footnote-reference-content",
            expected: ExpectedDescriptor::Production("footnote reference content"),
            found_kind: SyntaxKind::RightBracket,
            found_text: Some("]"),
            at: 2,
            machine_fix: None,
        }],
    );
    assert_structured_recovery(
        rules::FOOTNOTE_REFERENCE,
        "[^note\nheading",
        SyntaxKind::FootnoteReference,
        &[RecoveryExpectation {
            code: "syntax/unclosed-footnote-reference",
            expected: ExpectedDescriptor::Token(SyntaxKind::RightBracket),
            found_kind: SyntaxKind::Newline,
            found_text: Some("\n"),
            at: 6,
            machine_fix: Some("]"),
        }],
    );
    assert_structured_recovery(
        rules::REFERENCE,
        "[abc",
        SyntaxKind::Reference,
        &[RecoveryExpectation {
            code: "syntax/unclosed-reference",
            expected: ExpectedDescriptor::Token(SyntaxKind::RightBracket),
            found_kind: SyntaxKind::Eof,
            found_text: None,
            at: 4,
            machine_fix: Some("]"),
        }],
    );
    assert_structured_recovery(
        rules::SECTION_REFERENCE,
        "§",
        SyntaxKind::SectionReference,
        &[RecoveryExpectation {
            code: "syntax/missing-section-reference",
            expected: ExpectedDescriptor::Production("section reference"),
            found_kind: SyntaxKind::Eof,
            found_text: None,
            at: "§".len(),
            machine_fix: None,
        }],
    );
    assert_structured_recovery(
        rules::EQUATION,
        "$$",
        SyntaxKind::Equation,
        &[RecoveryExpectation {
            code: "syntax/missing-equation-content",
            expected: ExpectedDescriptor::Production("equation content"),
            found_kind: SyntaxKind::Eof,
            found_text: None,
            at: 2,
            machine_fix: None,
        }],
    );
}

#[test]
fn inline_code_rejects_fence_openers_and_invalid_references_stay_noncommitting() {
    let fence = parse("```text", rules::INLINE_CODE);
    assert!(!fence.matched);
    assert_eq!(fence.consumed, TextRange::empty(TextSize::ZERO));
    assert!(fence.diagnostics.is_empty());

    for input in ["[a-b]", "[]"] {
        let parsed = parse(input, rules::REFERENCE);
        assert!(!parsed.matched, "{input:?}");
        assert_eq!(parsed.consumed, TextRange::empty(TextSize::ZERO));
        assert!(parsed.diagnostics.is_empty(), "{input:?}");
    }
}

#[test]
fn paragraph_text_rechecks_every_negative_lookahead() {
    for prefix in [
        "§", "[^", "!!", "$$", "![", "http", "{", "[", "<", "⟨", "]", "~", "*", "_", "`", ":=",
        "|", "⸢", "⸥",
    ] {
        let at_start = parse(prefix, rules::PARAGRAPH_TEXT);
        assert!(!at_start.matched, "excluded prefix {prefix:?}");

        let input = if prefix == ":=" {
            String::from("plain :=")
        } else {
            format!("plain{prefix}")
        };
        let after_text = parse(&input, rules::PARAGRAPH_TEXT);
        assert!(after_text.is_strictly_clean(), "{input:?}");
        assert_eq!(
            after_text.consumed.end,
            TextSize(5),
            "negative lookahead was not rerun for {input:?}",
        );
    }

    let inline = parse("`x := 1`", rules::INLINE_CODE);
    assert!(inline.is_strictly_clean());
    assert_eq!(inline.consumed.end, TextSize(8));
}
