//! Canonical typed payload and physical-source contracts for the closed Mechdown rules.
//! These retain the former parity suite's fixtures without a second parser or AST.

use mech_syntax::document::ast::mechdown::{
    EquationSyntax, FootnoteReferenceSyntax, InlineCodeSyntax, InlineEquationSyntax,
    ParagraphTextSyntax, RawHyperlinkSyntax, ReferenceSyntax, SectionReferenceSyntax,
    ThematicBreakSyntax,
};
use mech_syntax::document::parser::canonical::parse_canonical_mechdown_rule_for_test;
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    AstNode, DocumentId, NodeFlags, ParseConfig, Revision, RuleId, SyntaxKind, SyntaxNode,
    TextRange, TextSize, TextSnapshot, TokenFlags, validate_lossless_range,
};

fn find_node(root: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if root.kind() == kind {
        return Some(root.clone());
    }
    root.children().find_map(|child| find_node(&child, kind))
}

fn assert_payload<T: AstNode>(
    input: &str,
    rule: RuleId,
    kind: SyntaxKind,
    prefix: &str,
    body: &str,
    suffix: &str,
) {
    let source = TextSnapshot::new(DocumentId(202), Revision(0), input).unwrap();
    let parsed =
        parse_canonical_mechdown_rule_for_test(source, rule, ParseConfig::default()).unwrap();
    assert!(parsed.is_strictly_clean(), "{rule:?} on {input:?}");
    assert_eq!(parsed.rule, rule);
    assert_eq!(parsed.syntax().kind(), SyntaxKind::CanonicalFragment);
    assert_eq!(parsed.consumed, parsed.source.full_range(), "{input:?}");
    validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed).unwrap();
    let typed = T::cast(find_node(&parsed.syntax(), kind).unwrap()).unwrap();
    let node = typed.syntax();
    assert_eq!(node.kind(), kind);
    assert_eq!(node.range(), parsed.source.full_range(), "{input:?}");
    assert_eq!(node.text().unwrap(), format!("{prefix}{body}{suffix}"));
    let body_range = TextRange::new(
        TextSize(prefix.len() as u32),
        TextSize((input.len() - suffix.len()) as u32),
    );
    assert_eq!(node.source().text(body_range).unwrap(), body, "{input:?}");

    // Every physical token must remain in source order at its exact byte range,
    // including multibyte graphemes, escaped text, delimiters and CRLF.
    let mut cursor = TextSize::ZERO;
    let mut token_text = String::new();
    for token in node.tokens() {
        assert_eq!(token.range().start, cursor, "{input:?}");
        assert!(
            !token
                .flags()
                .intersects(TokenFlags::ERROR | TokenFlags::MISSING | TokenFlags::SYNTHETIC)
        );
        let text = token.text().unwrap();
        assert_eq!(
            text,
            input[token.range().start.0 as usize..token.range().end.0 as usize]
        );
        token_text.push_str(&text);
        cursor = token.range().end;
    }
    assert_eq!(cursor, parsed.source.byte_len());
    assert_eq!(token_text, input);
}

#[test]
fn inline_code_preserves_inert_payload_and_delimiters() {
    for (input, body) in [
        ("`text`", "text"),
        ("``", ""),
        ("`x := 1`", "x := 1"),
        ("`\\n`", "\\n"),
    ] {
        assert_payload::<InlineCodeSyntax>(
            input,
            rules::INLINE_CODE,
            SyntaxKind::InlineCode,
            "`",
            body,
            "`",
        );
    }
}

#[test]
fn inline_equations_preserve_math_payload() {
    for (input, body) in [
        ("$$x$$", "x"),
        ("$$\\alpha$$", "\\alpha"),
        ("$$x + 1$$", "x + 1"),
    ] {
        assert_payload::<InlineEquationSyntax>(
            input,
            rules::INLINE_EQUATION,
            SyntaxKind::InlineEquation,
            "$$",
            body,
            "$$",
        );
    }
}

#[test]
fn raw_hyperlinks_preserve_the_complete_url() {
    for input in [
        "http://example.com",
        "http://example.com/path",
        "http://example.com\tpath",
    ] {
        assert_payload::<RawHyperlinkSyntax>(
            input,
            rules::RAW_HYPERLINK,
            SyntaxKind::RawHyperlink,
            "",
            input,
            "",
        );
    }
}

#[test]
fn reference_payloads_preserve_spelling_and_unicode() {
    for (input, body) in [("[^note]", "note"), ("[^a b]", "a b"), ("[^\\n]", "\\n")] {
        assert_payload::<FootnoteReferenceSyntax>(
            input,
            rules::FOOTNOTE_REFERENCE,
            SyntaxKind::FootnoteReference,
            "[^",
            body,
            "]",
        );
    }
    for (input, body) in [("[abc]", "abc"), ("[123]", "123"), ("[Δ2]", "Δ2")] {
        assert_payload::<ReferenceSyntax>(
            input,
            rules::REFERENCE,
            SyntaxKind::Reference,
            "[",
            body,
            "]",
        );
    }
    for (input, body) in [("§1.2", "1.2"), ("§abc", "abc"), ("§Δ.٣", "Δ.٣")] {
        assert_payload::<SectionReferenceSyntax>(
            input,
            rules::SECTION_REFERENCE,
            SyntaxKind::SectionReference,
            "§",
            body,
            "",
        );
    }
}

#[test]
fn paragraph_text_preserves_physical_unicode_and_escapes() {
    for input in [
        "plain prose",
        "punctuation, works.",
        "Unicode Δ and emoji 🧪",
        "escaped \\n text",
    ] {
        assert_payload::<ParagraphTextSyntax>(
            input,
            rules::PARAGRAPH_TEXT,
            SyntaxKind::ParagraphText,
            "",
            input,
            "",
        );
    }
}

#[test]
fn thematic_breaks_preserve_each_physical_newline() {
    for (input, body, ending) in [
        ("*\n", "*", "\n"),
        ("*** \t\r", "*** \t", "\r"),
        ("**\r\n", "**", "\r\n"),
    ] {
        assert_payload::<ThematicBreakSyntax>(
            input,
            rules::THEMATIC_BREAK,
            SyntaxKind::ThematicBreak,
            "",
            body,
            ending,
        );
    }
}

#[test]
fn equations_preserve_their_complete_math_payload() {
    for (input, body) in [("$$x+y", "x+y"), ("$$\\alpha", "\\alpha"), ("$$x$$", "x$$")] {
        assert_payload::<EquationSyntax>(
            input,
            rules::EQUATION,
            SyntaxKind::Equation,
            "$$",
            body,
            "",
        );
    }
}

#[test]
fn missing_syntax_is_diagnostic_and_never_a_clean_typed_payload() {
    for (rule, input, kind, code) in [
        (
            rules::INLINE_CODE,
            "`missing",
            SyntaxKind::InlineCode,
            "syntax/unclosed-inline-code",
        ),
        (
            rules::INLINE_EQUATION,
            "$$x",
            SyntaxKind::InlineEquation,
            "syntax/unclosed-inline-equation",
        ),
        (
            rules::FOOTNOTE_REFERENCE,
            "[^note",
            SyntaxKind::FootnoteReference,
            "syntax/unclosed-footnote-reference",
        ),
        (
            rules::SECTION_REFERENCE,
            "§",
            SyntaxKind::SectionReference,
            "syntax/missing-section-reference",
        ),
        (
            rules::EQUATION,
            "$$",
            SyntaxKind::Equation,
            "syntax/missing-equation-content",
        ),
    ] {
        let source = TextSnapshot::new(DocumentId(203), Revision(0), input).unwrap();
        let parsed =
            parse_canonical_mechdown_rule_for_test(source, rule, ParseConfig::default()).unwrap();
        assert!(parsed.matched);
        assert!(!parsed.is_strictly_clean(), "{input:?}");
        let node = find_node(&parsed.syntax(), kind).unwrap();
        assert!(
            node.flags()
                .intersects(NodeFlags::CONTAINS_MISSING | NodeFlags::MISSING),
            "{input:?}"
        );
        assert_eq!(parsed.diagnostics.len(), 1, "{input:?}");
        assert_eq!(
            parsed.diagnostics.iter().next().unwrap().code.as_str(),
            code,
            "{input:?}"
        );
        assert_eq!(
            parsed.diagnostics.iter().next().unwrap().rule,
            Some(rule),
            "{input:?}"
        );
        validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed).unwrap();
    }
}
