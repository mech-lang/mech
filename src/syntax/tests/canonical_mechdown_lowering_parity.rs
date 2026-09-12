use std::fmt::Debug;

use mech_core::{ParagraphElement, SectionElement};
use mech_syntax::document::ast::mechdown::{
    EquationSyntax, FootnoteReferenceSyntax, InlineCodeSyntax, InlineEquationSyntax,
    ParagraphTextSyntax, RawHyperlinkSyntax, ReferenceSyntax, SectionReferenceSyntax,
    ThematicBreakSyntax,
};
use mech_syntax::document::parser::canonical::parse_canonical_mechdown_rule_for_test;
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    AstNode, DocumentId, ParseConfig, Revision, RuleId, SyntaxKind, SyntaxNode, TextSnapshot,
    lower_legacy_equation, lower_legacy_footnote_reference, lower_legacy_inline_code,
    lower_legacy_inline_equation, lower_legacy_paragraph_text, lower_legacy_raw_hyperlink,
    lower_legacy_reference, lower_legacy_section_reference, lower_legacy_thematic_break,
};

fn canonical_node(input: &str, rule: RuleId, kind: SyntaxKind) -> SyntaxNode {
    let source = TextSnapshot::new(DocumentId(202), Revision(0), input).unwrap();
    let parsed =
        parse_canonical_mechdown_rule_for_test(source, rule, ParseConfig::default()).unwrap();
    assert!(parsed.is_strictly_clean(), "{rule:?} on {input:?}");
    assert_eq!(parsed.rule, rule, "{input:?}");
    assert_eq!(
        parsed.syntax().kind(),
        SyntaxKind::CanonicalFragment,
        "{input:?}"
    );
    assert_eq!(parsed.consumed.end, parsed.source.byte_len(), "{input:?}");
    find_node(&parsed.syntax(), kind)
        .unwrap_or_else(|| panic!("{rule:?} did not emit {kind:?} for {input:?}"))
}

fn find_node(root: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if root.kind() == kind {
        return Some(root.clone());
    }
    root.children().find_map(|child| find_node(&child, kind))
}

fn legacy_value<Output>(
    input: &str,
    parser: for<'source> fn(
        mech_syntax::ParseString<'source>,
    ) -> mech_syntax::ParseResult<'source, Output>,
) -> Output {
    let graphemes = mech_syntax::graphemes::init_tag(input);
    let (remaining, value) = parser(mech_syntax::ParseString::new(&graphemes)).unwrap();
    assert_eq!(remaining.cursor, graphemes.len(), "{input:?}");
    assert!(remaining.error_log.is_empty(), "{input:?}");
    value
}

fn assert_exact_legacy_value<T>(canonical: T, legacy: T, input: &str)
where
    T: Debug + Eq,
{
    // The legacy value equality includes the enum variant, ordered child
    // values, and every token's kind, characters, and physical source range.
    assert_eq!(canonical, legacy, "{input:?}");
}

#[test]
fn inline_code_values_match_legacy_exactly() {
    for input in ["`text`", "``", "`x := 1`", "`\\n`"] {
        let node = canonical_node(input, rules::INLINE_CODE, SyntaxKind::InlineCode);
        let canonical = lower_legacy_inline_code(&InlineCodeSyntax::cast(node).unwrap()).unwrap();
        let legacy = legacy_value(input, mech_syntax::inline_code);
        assert_exact_legacy_value(canonical, legacy, input);
    }
}

#[test]
fn inline_equation_values_match_legacy_exactly() {
    for input in ["$$x$$", "$$\\alpha$$", "$$x + 1$$"] {
        let node = canonical_node(input, rules::INLINE_EQUATION, SyntaxKind::InlineEquation);
        let canonical =
            lower_legacy_inline_equation(&InlineEquationSyntax::cast(node).unwrap()).unwrap();
        let legacy = legacy_value(input, mech_syntax::inline_equation);
        assert_exact_legacy_value(canonical, legacy, input);
    }
}

#[test]
fn raw_hyperlink_values_keep_the_legacy_duplicate_url_shape() {
    for input in [
        "http://example.com",
        "http://example.com/path",
        "http://example.com\tpath",
    ] {
        let node = canonical_node(input, rules::RAW_HYPERLINK, SyntaxKind::RawHyperlink);
        let canonical =
            lower_legacy_raw_hyperlink(&RawHyperlinkSyntax::cast(node).unwrap()).unwrap();
        let legacy = legacy_value(input, mech_syntax::raw_hyperlink);
        assert_exact_legacy_value(canonical.clone(), legacy, input);

        let ParagraphElement::Hyperlink((paragraph, url)) = canonical else {
            panic!("raw hyperlink lowered to the wrong paragraph variant");
        };
        assert_eq!(paragraph.elements, vec![ParagraphElement::Text(url)]);
    }
}

#[test]
fn reference_values_match_legacy_exactly() {
    for input in ["[^note]", "[^a b]", "[^\\n]"] {
        let node = canonical_node(
            input,
            rules::FOOTNOTE_REFERENCE,
            SyntaxKind::FootnoteReference,
        );
        let canonical =
            lower_legacy_footnote_reference(&FootnoteReferenceSyntax::cast(node).unwrap()).unwrap();
        let legacy = legacy_value(input, mech_syntax::footnote_reference);
        assert_exact_legacy_value(canonical, legacy, input);
    }

    for input in ["[abc]", "[123]", "[Δ2]"] {
        let node = canonical_node(input, rules::REFERENCE, SyntaxKind::Reference);
        let canonical = lower_legacy_reference(&ReferenceSyntax::cast(node).unwrap()).unwrap();
        let legacy = legacy_value(input, mech_syntax::reference);
        assert_exact_legacy_value(canonical, legacy, input);
    }

    for input in ["§1.2", "§abc", "§Δ.٣"] {
        let node = canonical_node(
            input,
            rules::SECTION_REFERENCE,
            SyntaxKind::SectionReference,
        );
        let canonical =
            lower_legacy_section_reference(&SectionReferenceSyntax::cast(node).unwrap()).unwrap();
        let legacy = legacy_value(input, mech_syntax::section_reference);
        assert_exact_legacy_value(canonical, legacy, input);
    }
}

#[test]
fn paragraph_text_values_match_legacy_exactly() {
    for input in [
        "plain prose",
        "punctuation, works.",
        "Unicode Δ and emoji 🧪",
        "escaped \\n text",
    ] {
        let node = canonical_node(input, rules::PARAGRAPH_TEXT, SyntaxKind::ParagraphText);
        let canonical =
            lower_legacy_paragraph_text(&ParagraphTextSyntax::cast(node).unwrap()).unwrap();
        let legacy = legacy_value(input, mech_syntax::paragraph_text);
        assert_exact_legacy_value(canonical, legacy, input);
    }
}

#[test]
fn thematic_break_values_match_legacy_exactly() {
    for input in ["*\n", "*** \t\r", "**\r\n"] {
        let node = canonical_node(input, rules::THEMATIC_BREAK, SyntaxKind::ThematicBreak);
        let canonical =
            lower_legacy_thematic_break(&ThematicBreakSyntax::cast(node).unwrap()).unwrap();
        let legacy = legacy_value(input, mech_syntax::thematic_break);
        assert_exact_legacy_value(canonical.clone(), legacy, input);
        assert_eq!(canonical, SectionElement::ThematicBreak);
    }
}

#[test]
fn equation_values_match_legacy_exactly() {
    for input in ["$$x+y", "$$\\alpha", "$$x$$"] {
        let node = canonical_node(input, rules::EQUATION, SyntaxKind::Equation);
        let canonical = lower_legacy_equation(&EquationSyntax::cast(node).unwrap()).unwrap();
        let legacy = legacy_value(input, mech_syntax::equation);
        assert_exact_legacy_value(canonical, legacy, input);
    }
}

#[test]
fn lowerers_reject_actual_missing_syntax() {
    for (rule, input, kind) in [
        (rules::INLINE_CODE, "`missing", SyntaxKind::InlineCode),
        (rules::INLINE_EQUATION, "$$x", SyntaxKind::InlineEquation),
        (
            rules::FOOTNOTE_REFERENCE,
            "[^note",
            SyntaxKind::FootnoteReference,
        ),
        (rules::SECTION_REFERENCE, "§", SyntaxKind::SectionReference),
        (rules::EQUATION, "$$", SyntaxKind::Equation),
    ] {
        let source = TextSnapshot::new(DocumentId(203), Revision(0), input).unwrap();
        let parsed =
            parse_canonical_mechdown_rule_for_test(source, rule, ParseConfig::default()).unwrap();
        let node = find_node(&parsed.syntax(), kind).unwrap();
        let error = match kind {
            SyntaxKind::InlineCode => {
                lower_legacy_inline_code(&InlineCodeSyntax::cast(node).unwrap()).unwrap_err()
            }
            SyntaxKind::InlineEquation => {
                lower_legacy_inline_equation(&InlineEquationSyntax::cast(node).unwrap())
                    .unwrap_err()
            }
            SyntaxKind::FootnoteReference => {
                lower_legacy_footnote_reference(&FootnoteReferenceSyntax::cast(node).unwrap())
                    .unwrap_err()
            }
            SyntaxKind::SectionReference => {
                lower_legacy_section_reference(&SectionReferenceSyntax::cast(node).unwrap())
                    .unwrap_err()
            }
            SyntaxKind::Equation => {
                lower_legacy_equation(&EquationSyntax::cast(node).unwrap()).unwrap_err()
            }
            _ => unreachable!(),
        };
        assert_eq!(
            error.as_slice()[0].phase,
            mech_syntax::document::DiagnosticPhase::Lowering,
            "{input:?}",
        );
    }
}
