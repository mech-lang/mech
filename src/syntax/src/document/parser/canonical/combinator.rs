//! Transactional combinators shared by canonical production parsers.

use alloc::string::String;

use crate::document::{
    DiagnosticFix, ExpectedSyntax, FixApplicability, SyntaxKind, TextEdit, TextRange, TextSize,
    TokenFlags,
};

use super::super::Parser;
use super::super::recovery;
use super::super::rule::rules;

/// The result of an ordered grammar alternative.
///
/// `Committed` is deliberately distinct from `Matched`: once a distinctive
/// grammar prefix has been consumed, callers must not try a later factor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Attempt {
    NoMatch,
    Matched,
    Committed,
}

impl Attempt {
    pub(crate) const fn accepted(self) -> bool {
        !matches!(self, Self::NoMatch)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct LogicalGrapheme {
    pub(crate) first: char,
    pub(crate) physical_end: TextSize,
}

/// Run a non-committing production under its canonical rule scope.
///
/// Failed alternatives restore their cursor, events, diagnostics, markers,
/// nesting, and rule-stack depth. Fuel remains charged, as it does for the
/// parser's other speculative operations.
pub(crate) fn transactional(
    parser: &mut Parser<'_>,
    rule: crate::document::RuleId,
    parse: impl FnOnce(&mut Parser<'_>) -> Attempt,
) -> Attempt {
    let checkpoint = parser.checkpoint();
    let result = parser.with_canonical_rule(rule, parse);
    if result == Attempt::NoMatch {
        parser.rewind(checkpoint);
    }
    result
}

pub(crate) const fn is_grammar_ignored(character: char) -> bool {
    matches!(character, ' ' | '\t' | '\r' | '\n')
}

/// Preserve the bytes discarded by legacy grammar preprocessing as trivia.
pub(crate) fn consume_grammar_ignored_trivia(parser: &mut Parser<'_>) {
    while !parser.is_halted() {
        match (parser.cursor().byte(), parser.cursor().byte_at(1)) {
            (Some(b'\r'), Some(b'\n')) => {
                let Some(range) = parser.bump_grapheme_raw() else {
                    break;
                };
                parser.token_with_flags(SyntaxKind::Newline, range, TokenFlags::TRIVIA);
            }
            (Some(b'\r' | b'\n'), _) => {
                let Some(range) = parser.bump_grapheme_raw() else {
                    break;
                };
                parser.token_with_flags(SyntaxKind::Newline, range, TokenFlags::TRIVIA);
            }
            (Some(b' ' | b'\t'), _) => {
                let start = parser.offset();
                while matches!(parser.cursor().byte(), Some(b' ' | b'\t')) {
                    if parser.bump_char_raw().is_none() {
                        break;
                    }
                }
                if parser.offset() == start {
                    break;
                }
                parser.token_with_flags(
                    SyntaxKind::Whitespace,
                    TextRange::new(start, parser.offset()),
                    TokenFlags::TRIVIA,
                );
            }
            _ => break,
        }
    }
}

/// Inspect the next grapheme after applying the grammar root's global ASCII
/// trivia filter, without flattening the source or changing parser state.
pub(crate) fn peek_logical_grapheme(parser: &Parser<'_>) -> Option<LogicalGrapheme> {
    let (first, range) = parser
        .cursor()
        .peek_filtered_grapheme_range(is_grammar_ignored)?;
    Some(LogicalGrapheme {
        first,
        physical_end: range.end,
    })
}

pub(crate) fn peek_logical_char(parser: &Parser<'_>) -> Option<char> {
    let mut cursor = parser.cursor().clone();
    while let Some((character, _)) = cursor.bump_char() {
        if !is_grammar_ignored(character) {
            return Some(character);
        }
    }
    None
}

/// Match a literal while ignoring the same ASCII trivia as the legacy root.
pub(crate) fn logical_starts_with(parser: &Parser<'_>, literal: &str) -> bool {
    parser
        .cursor()
        .filtered_grapheme_literal_end(literal, is_grammar_ignored)
        .is_some()
}

/// Consume one filtered logical grapheme, preserving physical trivia pieces.
pub(crate) fn consume_logical_grapheme(
    parser: &mut Parser<'_>,
    kind: SyntaxKind,
    classify: impl FnOnce(char) -> bool,
) -> bool {
    let Some(grapheme) = peek_logical_grapheme(parser) else {
        return false;
    };
    if !classify(grapheme.first) {
        return false;
    }
    consume_semantic_until(parser, grapheme.physical_end, kind)
}

/// Consume an exact filtered literal and retain semantic pieces on either side
/// of physical trivia. This is needed for inputs such as `: \n =` and `. .`.
pub(crate) fn consume_logical_literal(
    parser: &mut Parser<'_>,
    literal: &str,
    kind: SyntaxKind,
) -> bool {
    if literal.is_empty() || !logical_starts_with(parser, literal) {
        return false;
    }

    let mut remaining = literal.chars().peekable();
    let mut segment_start = None;
    while remaining.peek().is_some() && !parser.is_halted() {
        if parser.cursor().peek_char().is_some_and(is_grammar_ignored) {
            flush_semantic_segment(parser, &mut segment_start, kind);
            consume_grammar_ignored_trivia(parser);
            continue;
        }

        let Some(expected) = remaining.next() else {
            break;
        };
        let start = parser.offset();
        let Some((found, _)) = parser.bump_char_raw() else {
            return false;
        };
        debug_assert_eq!(found, expected);
        segment_start.get_or_insert(start);
    }
    flush_semantic_segment(parser, &mut segment_start, kind);
    remaining.peek().is_none()
}

pub(crate) fn consume_rule_literal(
    parser: &mut Parser<'_>,
    rule: crate::document::RuleId,
    literal: &str,
    kind: SyntaxKind,
) -> bool {
    let checkpoint = parser.checkpoint();
    let matched = parser.with_canonical_rule(rule, |parser| {
        consume_logical_literal(parser, literal, kind)
    });
    if !matched {
        parser.rewind(checkpoint);
    }
    matched
}

pub(crate) fn consume_define_operator(parser: &mut Parser<'_>) -> bool {
    let checkpoint = parser.checkpoint();
    let matched = parser.with_canonical_rule(rules::DEFINE_OPERATOR, |parser| {
        if parser.cursor().grapheme_literal_end(":=").is_some() {
            return parser
                .bump_bytes_token(2, SyntaxKind::DefineOperatorToken)
                .is_some();
        }
        if !logical_starts_with(parser, ":=")
            || !consume_logical_literal(parser, ":", SyntaxKind::Colon)
        {
            return false;
        }
        consume_grammar_ignored_trivia(parser);
        consume_logical_literal(parser, "=", SyntaxKind::Equal)
    });
    if !matched {
        parser.rewind(checkpoint);
    }
    matched
}

pub(crate) fn insert_missing(
    parser: &mut Parser<'_>,
    code: &str,
    message: &str,
    expected: ExpectedSyntax,
    token: Option<SyntaxKind>,
    fix_text: Option<&str>,
) {
    let _ = recovery::insert_missing(parser, code, message, expected, token);
    if let Some(text) = fix_text {
        let at = parser.offset();
        if let Some(diagnostic) = parser.last_diagnostic_mut() {
            diagnostic.fixes.push(DiagnosticFix {
                title: format_fix_title(text),
                applicability: FixApplicability::MachineApplicable,
                edits: alloc::vec![TextEdit::insert(at, text)],
            });
        }
    }
}

pub(crate) fn emit_synthetic_final_newline(parser: &mut Parser<'_>) {
    parser.token_with_flags(
        SyntaxKind::Newline,
        TextRange::empty(parser.offset()),
        TokenFlags::SYNTHETIC | TokenFlags::TRIVIA,
    );
}

fn consume_semantic_until(parser: &mut Parser<'_>, end: TextSize, kind: SyntaxKind) -> bool {
    let start = parser.offset();
    let mut segment_start = None;
    while parser.offset() < end && !parser.is_halted() {
        if parser.cursor().peek_char().is_some_and(is_grammar_ignored) {
            flush_semantic_segment(parser, &mut segment_start, kind);
            consume_grammar_ignored_trivia(parser);
            continue;
        }
        let at = parser.offset();
        if parser.bump_char_raw().is_none() {
            break;
        }
        segment_start.get_or_insert(at);
    }
    flush_semantic_segment(parser, &mut segment_start, kind);
    parser.offset() == end && parser.offset() > start
}

fn flush_semantic_segment(parser: &mut Parser<'_>, start: &mut Option<TextSize>, kind: SyntaxKind) {
    if let Some(start) = start.take() {
        parser.token(kind, TextRange::new(start, parser.offset()));
    }
}

fn format_fix_title(text: &str) -> String {
    let mut title = String::from("Insert `");
    title.push_str(text);
    title.push('`');
    title
}
