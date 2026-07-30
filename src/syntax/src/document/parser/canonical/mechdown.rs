//! Canonical Phase 2B Mechdown productions.
//!
//! This module contains only the exact closed grammar island selected for
//! Phase 2B. It does not call prototype or legacy production parsers.

use alloc::string::String;
use alloc::sync::Arc;

use crate::document::{
    DiagnosticAnchor, DiagnosticLabel, DiagnosticStore, ExpectedSyntax, GreenNode, IdGenerator,
    NodeIndex, ParseStats, RuleId, SyntaxKind, SyntaxNode, TextRange, TextSnapshot,
};

use super::super::rule::rules;
use super::super::{LexicalMode, ParseConfig, Parser, sink};
use super::base;
use super::combinator::{self, Attempt};
use super::statements;
use super::terminal_spec::{TerminalSpacing, fixed_terminal_spec};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CodeblockDelimiter {
    Grave,
    Tilde,
}

/// A narrow prefix snapshot for deterministic Phase 2B rule-contract tests.
#[derive(Clone, Debug)]
pub struct CanonicalMechdownRuleSnapshot {
    pub source: TextSnapshot,
    pub rule: RuleId,
    pub root: Arc<GreenNode>,
    pub diagnostics: DiagnosticStore,
    pub nodes: NodeIndex,
    pub stats: ParseStats,
    pub matched: bool,
    pub consumed: TextRange,
}

impl CanonicalMechdownRuleSnapshot {
    pub fn syntax(&self) -> SyntaxNode {
        SyntaxNode::new_root_at(self.root.clone(), self.source.clone(), self.consumed.start)
    }

    pub fn is_strictly_clean(&self) -> bool {
        self.matched && self.diagnostics.is_empty()
    }
}

/// Parse one of the exact 13 Phase 2B productions as a deterministic prefix.
///
/// This is intentionally a test-only contract surface, analogous to the
/// Phase 2A lexical-rule prefix wrapper. It is not a production document root.
#[doc(hidden)]
pub fn parse_canonical_mechdown_rule_for_test(
    source: TextSnapshot,
    rule: RuleId,
    config: ParseConfig,
) -> Option<CanonicalMechdownRuleSnapshot> {
    is_closed_rule(rule).then(|| parse_rule_prefix(source, rule, config))
}

/// Parse the transparent `codeblock-sigil` production and return the exact
/// delimiter needed by the direct rule contract.
pub(crate) fn parse_codeblock_sigil(parser: &mut Parser<'_>) -> Option<CodeblockDelimiter> {
    let checkpoint = parser.checkpoint();
    let delimiter = parser.with_canonical_rule(rules::CODEBLOCK_SIGIL, |parser| {
        if base::parse_rule(parser, rules::GRAVE_CODEBLOCK_SIGIL) {
            Some(CodeblockDelimiter::Grave)
        } else if base::parse_rule(parser, rules::TILDE_CODEBLOCK_SIGIL) {
            Some(CodeblockDelimiter::Tilde)
        } else {
            None
        }
    });
    if delimiter.is_none() {
        parser.rewind(checkpoint);
    }
    delimiter
}

pub(crate) fn parse_inline_code(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::INLINE_CODE, |parser| {
        if parser
            .cursor()
            .grapheme_literal_end(
                fixed_terminal_spec(rules::GRAVE_CODEBLOCK_SIGIL)
                    .expect("grave codeblock sigil is a canonical terminal")
                    .literal,
            )
            .is_some()
        {
            return Attempt::NoMatch;
        }

        let inline = parser.start();
        let opening_start = parser.offset();
        if !base::parse_rule(parser, rules::GRAVE) {
            inline.abandon(parser);
            return Attempt::NoMatch;
        }
        let opening = TextRange::new(opening_start, parser.offset());

        while !at_paragraph_boundary(parser) && !starts_with_rule(parser, rules::GRAVE) {
            let before = parser.offset();
            if !base::parse_rule(parser, rules::TEXT) {
                break;
            }
            if parser.offset() == before || parser.is_halted() {
                break;
            }
        }

        if base::parse_rule(parser, rules::GRAVE) {
            inline.complete(parser, SyntaxKind::InlineCode);
            return Attempt::Matched;
        }

        combinator::insert_missing(
            parser,
            "syntax/unclosed-inline-code",
            "expected a closing grave for inline code",
            ExpectedSyntax::Token(SyntaxKind::Grave),
            Some(SyntaxKind::Grave),
            Some("`"),
        );
        label_opening(parser, opening, "inline code starts here");
        inline.complete(parser, SyntaxKind::InlineCode);
        Attempt::Committed
    })
}

pub(crate) fn parse_inline_equation(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::INLINE_EQUATION, |parser| {
        let equation = parser.start();
        let opening_start = parser.offset();
        if !base::parse_rule(parser, rules::EQUATION_SIGIL) {
            equation.abandon(parser);
            return Attempt::NoMatch;
        }
        let opening = TextRange::new(opening_start, parser.offset());
        let content_start = parser.offset();

        while !at_paragraph_boundary(parser) && !starts_with_rule(parser, rules::EQUATION_SIGIL) {
            let before = parser.offset();
            if !parse_equation_content_element(parser) {
                break;
            }
            if parser.offset() == before || parser.is_halted() {
                break;
            }
        }

        if parser.offset() == content_start {
            combinator::insert_missing(
                parser,
                "syntax/missing-inline-equation-content",
                "expected inline equation content",
                ExpectedSyntax::Production(String::from("inline equation content")),
                None,
                None,
            );
            label_opening(parser, opening, "inline equation starts here");
            if !base::parse_rule(parser, rules::EQUATION_SIGIL) {
                combinator::insert_missing(
                    parser,
                    "syntax/unclosed-inline-equation",
                    "expected a closing equation sigil",
                    ExpectedSyntax::Token(SyntaxKind::EquationSigil),
                    Some(SyntaxKind::EquationSigil),
                    Some("$$"),
                );
                label_opening(parser, opening, "inline equation starts here");
            }
            equation.complete(parser, SyntaxKind::InlineEquation);
            return Attempt::Committed;
        }

        if base::parse_rule(parser, rules::EQUATION_SIGIL) {
            equation.complete(parser, SyntaxKind::InlineEquation);
            return Attempt::Matched;
        }

        combinator::insert_missing(
            parser,
            "syntax/unclosed-inline-equation",
            "expected a closing equation sigil",
            ExpectedSyntax::Token(SyntaxKind::EquationSigil),
            Some(SyntaxKind::EquationSigil),
            Some("$$"),
        );
        label_opening(parser, opening, "inline equation starts here");
        equation.complete(parser, SyntaxKind::InlineEquation);
        Attempt::Committed
    })
}

pub(crate) fn parse_raw_hyperlink(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::RAW_HYPERLINK, |parser| {
        let Some(prefix) = fixed_terminal_spec(rules::HTTP_PREFIX) else {
            return Attempt::NoMatch;
        };
        if parser
            .cursor()
            .grapheme_literal_end(prefix.literal)
            .is_none()
        {
            return Attempt::NoMatch;
        }

        let hyperlink = parser.start();
        let start = parser.offset();
        while !parser.is_eof() && parser.cursor().byte() != Some(b' ') {
            let before = parser.offset();
            if !base::parse_rule(parser, rules::TEXT) {
                break;
            }
            if parser.offset() == before || parser.is_halted() {
                break;
            }
        }
        if parser.offset() == start {
            hyperlink.abandon(parser);
            return Attempt::NoMatch;
        }
        hyperlink.complete(parser, SyntaxKind::RawHyperlink);
        Attempt::Matched
    })
}

pub(crate) fn parse_footnote_reference(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::FOOTNOTE_REFERENCE, |parser| {
        let reference = parser.start();
        let opening_start = parser.offset();
        if !base::parse_rule(parser, rules::FOOTNOTE_PREFIX) {
            reference.abandon(parser);
            return Attempt::NoMatch;
        }
        let opening = TextRange::new(opening_start, parser.offset());
        let content_start = parser.offset();

        while !at_paragraph_boundary(parser) && !starts_with_rule(parser, rules::RIGHT_BRACKET) {
            let before = parser.offset();
            if !base::parse_rule(parser, rules::TEXT) {
                break;
            }
            if parser.offset() == before || parser.is_halted() {
                break;
            }
        }

        if parser.offset() == content_start {
            combinator::insert_missing(
                parser,
                "syntax/missing-footnote-reference-content",
                "expected footnote reference content",
                ExpectedSyntax::Production(String::from("footnote reference content")),
                None,
                None,
            );
            label_opening(parser, opening, "footnote reference starts here");
            if !base::parse_rule(parser, rules::RIGHT_BRACKET) {
                combinator::insert_missing(
                    parser,
                    "syntax/unclosed-footnote-reference",
                    "expected a closing bracket for the footnote reference",
                    ExpectedSyntax::Token(SyntaxKind::RightBracket),
                    Some(SyntaxKind::RightBracket),
                    Some("]"),
                );
                label_opening(parser, opening, "footnote reference starts here");
            }
            reference.complete(parser, SyntaxKind::FootnoteReference);
            return Attempt::Committed;
        }

        if base::parse_rule(parser, rules::RIGHT_BRACKET) {
            reference.complete(parser, SyntaxKind::FootnoteReference);
            return Attempt::Matched;
        }

        combinator::insert_missing(
            parser,
            "syntax/unclosed-footnote-reference",
            "expected a closing bracket for the footnote reference",
            ExpectedSyntax::Token(SyntaxKind::RightBracket),
            Some(SyntaxKind::RightBracket),
            Some("]"),
        );
        label_opening(parser, opening, "footnote reference starts here");
        reference.complete(parser, SyntaxKind::FootnoteReference);
        Attempt::Committed
    })
}

pub(crate) fn parse_reference(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::REFERENCE, |parser| {
        let reference = parser.start();
        if !base::parse_rule(parser, rules::LEFT_BRACKET)
            || !base::parse_rule(parser, rules::ALPHANUMERIC)
        {
            reference.abandon(parser);
            return Attempt::NoMatch;
        }
        while base::parse_rule(parser, rules::ALPHANUMERIC) {
            if parser.is_halted() {
                break;
            }
        }
        if !base::parse_rule(parser, rules::RIGHT_BRACKET) {
            if at_paragraph_boundary(parser) {
                combinator::insert_missing(
                    parser,
                    "syntax/unclosed-reference",
                    "expected a closing bracket for the reference",
                    ExpectedSyntax::Token(SyntaxKind::RightBracket),
                    Some(SyntaxKind::RightBracket),
                    Some("]"),
                );
                reference.complete(parser, SyntaxKind::Reference);
                return Attempt::Committed;
            }
            reference.abandon(parser);
            return Attempt::NoMatch;
        }
        reference.complete(parser, SyntaxKind::Reference);
        Attempt::Matched
    })
}

pub(crate) fn parse_section_reference(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::SECTION_REFERENCE, |parser| {
        let reference = parser.start();
        let opening_start = parser.offset();
        if !base::parse_rule(parser, rules::SECTION_SIGIL) {
            reference.abandon(parser);
            return Attempt::NoMatch;
        }
        let opening = TextRange::new(opening_start, parser.offset());
        let content_start = parser.offset();
        while base::parse_rule(parser, rules::ALPHANUMERIC)
            || base::parse_rule(parser, rules::PERIOD)
        {
            if parser.is_halted() {
                break;
            }
        }

        if parser.offset() == content_start {
            combinator::insert_missing(
                parser,
                "syntax/missing-section-reference",
                "expected a section reference after the section sigil",
                ExpectedSyntax::Production(String::from("section reference")),
                None,
                None,
            );
            label_opening(parser, opening, "section reference starts here");
            reference.complete(parser, SyntaxKind::SectionReference);
            return Attempt::Committed;
        }
        reference.complete(parser, SyntaxKind::SectionReference);
        Attempt::Matched
    })
}

pub(crate) fn parse_paragraph_text(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::PARAGRAPH_TEXT, |parser| {
        let paragraph_text = parser.start();
        let start = parser.offset();
        while !paragraph_exclusion_ahead(parser) {
            let before = parser.offset();
            if !base::parse_rule(parser, rules::TEXT) {
                break;
            }
            if parser.offset() == before || parser.is_halted() {
                break;
            }
        }
        if parser.offset() == start {
            paragraph_text.abandon(parser);
            return Attempt::NoMatch;
        }
        paragraph_text.complete(parser, SyntaxKind::ParagraphText);
        Attempt::Matched
    })
}

pub(crate) fn parse_thematic_break(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::THEMATIC_BREAK, |parser| {
        let thematic_break = parser.start();
        if !base::parse_rule(parser, rules::ASTERISK) {
            thematic_break.abandon(parser);
            return Attempt::NoMatch;
        }
        while base::parse_rule(parser, rules::ASTERISK) {
            if parser.is_halted() {
                break;
            }
        }
        let _ = base::parse_rule(parser, rules::SPACE_TAB0);
        if !base::parse_rule(parser, rules::NEW_LINE) {
            thematic_break.abandon(parser);
            return Attempt::NoMatch;
        }
        thematic_break.complete(parser, SyntaxKind::ThematicBreak);
        Attempt::Matched
    })
}

pub(crate) fn parse_blank_line(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::BLANK_LINE, |parser| {
        let blank_line = parser.start();
        let _ = base::parse_rule(parser, rules::SPACE_TAB0);
        if !base::parse_rule(parser, rules::NEW_LINE) {
            blank_line.abandon(parser);
            return Attempt::NoMatch;
        }
        blank_line.complete(parser, SyntaxKind::BlankLine);
        Attempt::Matched
    })
}

pub(crate) fn parse_equation(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::EQUATION, |parser| {
        let equation = parser.start();
        let opening_start = parser.offset();
        if !base::parse_rule(parser, rules::EQUATION_SIGIL) {
            equation.abandon(parser);
            return Attempt::NoMatch;
        }
        let opening = TextRange::new(opening_start, parser.offset());
        let content_start = parser.offset();
        while !at_paragraph_boundary(parser) {
            let before = parser.offset();
            if !parse_equation_content_element(parser) {
                break;
            }
            if parser.offset() == before || parser.is_halted() {
                break;
            }
        }
        if parser.offset() == content_start {
            combinator::insert_missing(
                parser,
                "syntax/missing-equation-content",
                "expected block equation content",
                ExpectedSyntax::Production(String::from("equation content")),
                None,
                None,
            );
            label_opening(parser, opening, "equation starts here");
            equation.complete(parser, SyntaxKind::Equation);
            return Attempt::Committed;
        }
        equation.complete(parser, SyntaxKind::Equation);
        Attempt::Matched
    })
}

fn parse_equation_content_element(parser: &mut Parser<'_>) -> bool {
    base::parse_rule(parser, rules::BACKSLASH) || base::parse_rule(parser, rules::TEXT)
}

fn at_paragraph_boundary(parser: &Parser<'_>) -> bool {
    parser.is_eof() || matches!(parser.cursor().byte(), Some(b'\r' | b'\n'))
}

fn starts_with_rule(parser: &Parser<'_>, rule: crate::document::RuleId) -> bool {
    fixed_terminal_spec(rule).is_some_and(|spec| {
        spec.spacing == TerminalSpacing::Exact
            && parser.cursor().grapheme_literal_end(spec.literal).is_some()
    })
}

fn paragraph_exclusion_ahead(parser: &Parser<'_>) -> bool {
    const EXACT_EXCLUSIONS: &[crate::document::RuleId] = &[
        rules::SECTION_SIGIL,
        rules::FOOTNOTE_PREFIX,
        rules::HIGHLIGHT_SIGIL,
        rules::EQUATION_SIGIL,
        rules::IMG_PREFIX,
        rules::HTTP_PREFIX,
        rules::LEFT_BRACE,
        rules::LEFT_BRACKET,
        rules::LEFT_ANGLE1,
        rules::LEFT_ANGLE2,
        rules::RIGHT_BRACKET,
        rules::TILDE,
        rules::ASTERISK,
        rules::UNDERSCORE,
        rules::GRAVE,
        rules::BAR,
        rules::MIKA_SECTION_OPEN,
        rules::MIKA_SECTION_CLOSE,
    ];

    EXACT_EXCLUSIONS
        .iter()
        .copied()
        .any(|rule| starts_with_rule(parser, rule))
        || define_operator_ahead(parser)
}

fn define_operator_ahead(parser: &Parser<'_>) -> bool {
    let mut cursor = parser.cursor().clone();
    loop {
        match (cursor.byte(), cursor.byte_at(1)) {
            (Some(b' ' | b'\t'), _) => {
                if cursor.bump_char().is_none() {
                    return false;
                }
            }
            (Some(b'\r'), Some(b'\n')) => {
                if cursor.bump_bytes(2).is_none() {
                    return false;
                }
            }
            (Some(b'\r' | b'\n'), _) => {
                if cursor.bump_char().is_none() {
                    return false;
                }
            }
            _ => break,
        }
    }
    cursor.grapheme_literal_end(":=").is_some()
}

fn label_opening(parser: &mut Parser<'_>, opening: TextRange, message: &str) {
    let revision = parser.source().revision();
    if let Some(diagnostic) = parser.last_diagnostic_mut() {
        diagnostic.labels.push(DiagnosticLabel {
            anchor: DiagnosticAnchor::Absolute {
                revision,
                range: opening,
            },
            message: String::from(message),
        });
    }
}

fn is_closed_rule(rule: RuleId) -> bool {
    matches!(
        rule,
        rules::COMMENT_SIGIL
            | rules::COMMENT
            | rules::CODEBLOCK_SIGIL
            | rules::INLINE_CODE
            | rules::INLINE_EQUATION
            | rules::RAW_HYPERLINK
            | rules::FOOTNOTE_REFERENCE
            | rules::REFERENCE
            | rules::SECTION_REFERENCE
            | rules::PARAGRAPH_TEXT
            | rules::THEMATIC_BREAK
            | rules::BLANK_LINE
            | rules::EQUATION
    )
}

fn parse_rule_prefix(
    source: TextSnapshot,
    rule: RuleId,
    config: ParseConfig,
) -> CanonicalMechdownRuleSnapshot {
    let mut ids = IdGenerator::new();
    let mut parser = Parser::new(
        &source,
        LexicalMode::CanonicalSourceFragment,
        config,
        &mut ids,
    );
    parser.set_resource_rule(rule);
    let fragment = parser.start();
    let start = parser.offset();
    let matched = match rule {
        rules::COMMENT_SIGIL => statements::parse_comment_sigil(&mut parser),
        rules::COMMENT => statements::parse_comment(&mut parser).accepted(),
        rules::CODEBLOCK_SIGIL => parse_codeblock_sigil(&mut parser).is_some(),
        rules::INLINE_CODE => parse_inline_code(&mut parser).accepted(),
        rules::INLINE_EQUATION => parse_inline_equation(&mut parser).accepted(),
        rules::RAW_HYPERLINK => parse_raw_hyperlink(&mut parser).accepted(),
        rules::FOOTNOTE_REFERENCE => parse_footnote_reference(&mut parser).accepted(),
        rules::REFERENCE => parse_reference(&mut parser).accepted(),
        rules::SECTION_REFERENCE => parse_section_reference(&mut parser).accepted(),
        rules::PARAGRAPH_TEXT => parse_paragraph_text(&mut parser).accepted(),
        rules::THEMATIC_BREAK => parse_thematic_break(&mut parser).accepted(),
        rules::BLANK_LINE => parse_blank_line(&mut parser).accepted(),
        rules::EQUATION => parse_equation(&mut parser).accepted(),
        _ => unreachable!("closed-rule guard rejects every other RuleId"),
    };
    let end = parser.offset();
    fragment.complete(&mut parser, SyntaxKind::CanonicalFragment);
    let output = parser.finish();
    let sink_result = sink(&output.events, &source, &mut ids)
        .expect("canonical Phase 2B rule events must form one root");

    let mut diagnostics = DiagnosticStore::new(source.revision());
    for mut pending in output.diagnostics {
        if let Some(event) = pending.event
            && let Some(node) = sink_result.event_nodes.get(&event)
        {
            pending.diagnostic.primary = DiagnosticAnchor::Element {
                element: crate::document::SyntaxElementId::Node(*node),
                relative: pending.relative,
            };
        }
        diagnostics.push(pending.diagnostic);
    }

    let consumed = TextRange::new(start, end);
    let nodes = NodeIndex::build_at(&sink_result.root, consumed.start);
    let mut stats = output.stats;
    stats.new_node_count = nodes.node_count() as u64;
    CanonicalMechdownRuleSnapshot {
        source,
        rule,
        root: sink_result.root,
        diagnostics,
        nodes,
        stats,
        matched,
        consumed,
    }
}
