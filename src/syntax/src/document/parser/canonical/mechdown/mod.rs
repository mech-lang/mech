//! Canonical Phase 2B Mechdown productions.
//!
//! This module contains only the exact closed grammar island selected for
//! Phase 2B. It does not call prototype or legacy production parsers.

use crate::document::{
    DiagnosticAnchor, DiagnosticLabel, ExpectedSyntax, RuleId, SyntaxKind, TextRange, TextSnapshot,
};
use alloc::string::String;

use super::super::rule::rules;
use super::super::{ParseConfig, Parser};
use super::base;
use super::combinator::{self, Attempt};
use super::statements;
use super::terminal_spec::fixed_terminal_spec;
use super::test_support::{CanonicalSourceRuleSnapshot, parse_source_rule_prefix};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CodeblockDelimiter {
    Grave,
    Tilde,
}

/// Backwards-compatible name for Phase 2B direct-rule snapshots.
pub type CanonicalMechdownRuleSnapshot = CanonicalSourceRuleSnapshot;

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
    is_closed_rule(rule).then(|| {
        parse_source_rule_prefix(source, rule, config, |parser| match rule {
            rules::COMMENT_SIGIL => statements::parse_comment_sigil(parser)
                .then_some(Attempt::Matched)
                .unwrap_or(Attempt::NoMatch),
            rules::COMMENT => statements::parse_comment(parser),
            rules::CODEBLOCK_SIGIL => parse_codeblock_sigil(parser)
                .map(|_| Attempt::Matched)
                .unwrap_or(Attempt::NoMatch),
            rules::INLINE_CODE => parse_inline_code(parser),
            rules::INLINE_EQUATION => parse_inline_equation(parser),
            rules::RAW_HYPERLINK => parse_raw_hyperlink(parser),
            rules::FOOTNOTE_REFERENCE => parse_footnote_reference(parser),
            rules::REFERENCE => parse_reference(parser),
            rules::SECTION_REFERENCE => parse_section_reference(parser),
            rules::PARAGRAPH_TEXT => parse_paragraph_text(parser),
            rules::THEMATIC_BREAK => parse_thematic_break(parser),
            rules::BLANK_LINE => parse_blank_line(parser),
            rules::EQUATION => parse_equation(parser),
            _ => unreachable!("closed-rule guard rejects every other RuleId"),
        })
    })
}

mod continuation;
pub(crate) use continuation::{Continuation, Progress};

pub(crate) fn supports(rule: RuleId) -> bool {
    is_closed_rule(rule)
        && !matches!(
            rule,
            rules::COMMENT_SIGIL | rules::COMMENT | rules::PARAGRAPH_TEXT
        )
}
fn drive(parser: &mut Parser<'_>, rule: RuleId) -> Continuation {
    let mut continuation = Continuation::new(rule);
    loop {
        let mut allowance = u64::MAX;
        match continuation.advance(parser, true, &mut allowance) {
            Progress::Complete(_) => return continuation,
            Progress::NeedsProcessing => {}
            _ => unreachable!("final Mechdown input"),
        }
    }
}
pub(crate) fn parse_codeblock_sigil(parser: &mut Parser<'_>) -> Option<CodeblockDelimiter> {
    drive(parser, rules::CODEBLOCK_SIGIL).delimiter
}
pub(crate) fn parse_inline_code(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::INLINE_CODE).result
}
pub(crate) fn parse_inline_equation(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::INLINE_EQUATION).result
}
pub(crate) fn parse_raw_hyperlink(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::RAW_HYPERLINK).result
}
pub(crate) fn parse_footnote_reference(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::FOOTNOTE_REFERENCE).result
}
pub(crate) fn parse_reference(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::REFERENCE).result
}
pub(crate) fn parse_section_reference(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::SECTION_REFERENCE).result
}
pub(crate) fn parse_thematic_break(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::THEMATIC_BREAK).result
}
pub(crate) fn parse_blank_line(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::BLANK_LINE).result
}
pub(crate) fn parse_equation(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::EQUATION).result
}
pub(crate) fn parse_paragraph_text(parser: &mut Parser<'_>) -> Attempt {
    super::prose::parse_rule(parser, rules::PARAGRAPH_TEXT)
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
