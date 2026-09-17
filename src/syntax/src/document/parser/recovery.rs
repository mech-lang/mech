use alloc::{string::String, vec::Vec};

use crate::document::{
    Diagnostic, DiagnosticAnchor, DiagnosticCode, DiagnosticPhase, DiagnosticTags, ExpectedSyntax,
    FoundSyntax, NodeFlags, ParserContextId, RecoveryAction, RuleId, Severity, SyntaxKind,
    TextRange, TokenFlags,
};

use super::terminal::{is_newline_start, token_kind_for_char};
use super::{CompletedMarker, Parser};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseFailure {
    pub context: ParserContextId,
    pub range: TextRange,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Attempt<T> {
    NoMatch,
    Match(T),
    CommittedFailure(ParseFailure),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryClass {
    MechItem,
    Paragraph,
    Fence,
}

pub(crate) fn skip_error(
    parser: &mut Parser<'_>,
    class: RecoveryClass,
    code: &str,
    message: &str,
) -> Option<CompletedMarker> {
    let start = parser.offset();
    let marker = parser.start();
    while !parser.is_eof() && remaining_recovery_bytes(parser) > 0 {
        if should_stop(parser, class, start) {
            break;
        }
        let Some(character) = parser.cursor().peek_char() else {
            break;
        };
        if character.len_utf8() as u32 > remaining_recovery_bytes(parser) {
            parser.halt();
            break;
        }
        let Some((character, range)) = parser.bump_char_raw() else {
            break;
        };
        charge_recovery_bytes(parser, range.len().0);
        parser.token_with_flags(token_kind_for_char(character), range, TokenFlags::ERROR);
    }
    if remaining_recovery_bytes(parser) == 0
        && !parser.is_eof()
        && !should_stop(parser, class, start)
    {
        parser.halt();
    }
    if parser.offset() == start {
        marker.abandon(parser);
        return None;
    }
    let error = marker.complete_with_flags(parser, SyntaxKind::Error, NodeFlags::ERROR);
    let range = TextRange::new(start, parser.offset());
    let found = parser.source().text(range).ok().map(|text| FoundSyntax {
        kind: Some(SyntaxKind::Unknown),
        text: Some(text),
    });
    let diagnostic = Diagnostic {
        id: parser.next_diagnostic_id(),
        code: DiagnosticCode::from(code),
        phase: DiagnosticPhase::Syntax,
        severity: Severity::Error,
        rule: parser.current_rule(),
        context: parser.current_context(),
        primary: DiagnosticAnchor::Absolute {
            revision: parser.source().revision(),
            range,
        },
        labels: alloc::vec![],
        expected: alloc::vec![],
        found,
        fixes: alloc::vec![],
        related: alloc::vec![],
        recovery: Some(RecoveryAction::Skip { range }),
        tags: DiagnosticTags::NONE,
        message: String::from(message),
    };
    parser.push_diagnostic(
        diagnostic,
        Some(error.position()),
        TextRange::new(crate::document::TextSize::ZERO, range.len()),
    );
    Some(error)
}

/// Preserve unexpected bytes until a sibling or ancestor production can
/// restart. Boundary characters remain unconsumed for the owning production.
pub(crate) fn abandon_to_restart(
    parser: &mut Parser<'_>,
    target: RuleId,
    boundaries: &[char],
    code: &str,
    message: &str,
) -> Option<CompletedMarker> {
    abandon_to_restart_with_prefixes(parser, target, boundaries, &[], code, message)
}

pub(crate) fn abandon_to_restart_with_prefixes(
    parser: &mut Parser<'_>,
    target: RuleId,
    boundaries: &[char],
    prefixes: &[&str],
    code: &str,
    message: &str,
) -> Option<CompletedMarker> {
    abandon_until(parser, target, code, message, |parser, character| {
        boundaries.contains(&character)
            || prefixes
                .iter()
                .any(|prefix| parser.cursor().starts_with(prefix))
    })
}

pub(crate) fn abandon_until(
    parser: &mut Parser<'_>,
    target: RuleId,
    code: &str,
    message: &str,
    mut should_stop: impl FnMut(&mut Parser<'_>, char) -> bool,
) -> Option<CompletedMarker> {
    let start = parser.offset();
    let marker = parser.start();
    let mut delimiters = Vec::new();
    let mut quoted = None;
    let mut raw_triple = false;
    let mut escaped = false;

    while !parser.is_eof() && !parser.is_halted() && remaining_recovery_bytes(parser) > 0 {
        if quoted.is_none() && parser.cursor().starts_with("\"\"\"") {
            if remaining_recovery_bytes(parser) < 3 {
                parser.halt();
                break;
            }
            for _ in 0..3 {
                let Some((character, range)) = parser.bump_char_raw() else {
                    parser.halt();
                    break;
                };
                charge_recovery_bytes(parser, range.len().0);
                parser.token_with_flags(token_kind_for_char(character), range, TokenFlags::ERROR);
            }
            raw_triple = !raw_triple;
            continue;
        }
        let Some(character) = parser.cursor().peek_char() else {
            break;
        };
        if quoted.is_none()
            && !raw_triple
            && recovery_boundary(character, &delimiters, should_stop(parser, character))
        {
            break;
        }
        if character.len_utf8() as u32 > remaining_recovery_bytes(parser) {
            parser.halt();
            break;
        }

        // These canonical operator/sigil prefixes contain no angle opener.
        // In particular a generator arrow inside skipped comprehension source
        // must not hide its enclosing brace from the recovery scanner.
        let opens_angle = matches!(character, '<' | '⟨') && opens_kind_annotation(parser);
        if parser.is_halted() {
            break;
        }
        // The annotation probe may itself recover. Recheck the shared allowance
        // before this scanner consumes another scalar.
        if character.len_utf8() as u32 > remaining_recovery_bytes(parser) {
            parser.halt();
            break;
        }
        let Some((character, range)) = parser.bump_char_raw() else {
            break;
        };
        charge_recovery_bytes(parser, range.len().0);
        parser.token_with_flags(token_kind_for_char(character), range, TokenFlags::ERROR);

        if raw_triple {
            continue;
        }
        if let Some(quote) = quoted {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == quote {
                quoted = None;
            }
            continue;
        }
        match character {
            '"' => quoted = Some(character),
            '(' | '[' | '{' => delimiters.push(character),
            '<' | '⟨' if opens_angle => delimiters.push(character),
            ')' | ']' | '}' | '>' | '⟩' => {
                if delimiters
                    .last()
                    .is_some_and(|opener| delimiters_match(*opener, character))
                {
                    delimiters.pop();
                }
            }
            _ => {}
        }
    }

    let stopped_at_boundary = quoted.is_none()
        && !raw_triple
        && parser.cursor().peek_char().is_some_and(|character| {
            recovery_boundary(character, &delimiters, should_stop(parser, character))
        });
    let exhausted =
        remaining_recovery_bytes(parser) == 0 && !parser.is_eof() && !stopped_at_boundary;
    if exhausted {
        parser.halt();
    }
    if parser.offset() == start {
        marker.abandon(parser);
        return None;
    }

    let error = marker.complete_with_flags(parser, SyntaxKind::Error, NodeFlags::ERROR);
    let range = TextRange::new(start, parser.offset());
    let found = parser.source().text(range).ok().map(|text| FoundSyntax {
        kind: Some(SyntaxKind::Unknown),
        text: Some(text),
    });
    let diagnostic = Diagnostic {
        id: parser.next_diagnostic_id(),
        code: DiagnosticCode::from(code),
        phase: DiagnosticPhase::Syntax,
        severity: Severity::Error,
        rule: parser.current_rule(),
        context: parser.current_context(),
        primary: DiagnosticAnchor::Absolute {
            revision: parser.source().revision(),
            range,
        },
        labels: alloc::vec![],
        expected: alloc::vec![],
        found,
        fixes: alloc::vec![],
        related: alloc::vec![],
        recovery: Some(RecoveryAction::Abandon {
            rule: target,
            at: parser.offset(),
        }),
        tags: DiagnosticTags::NONE,
        message: String::from(message),
    };
    parser.push_diagnostic(
        diagnostic,
        Some(error.position()),
        TextRange::new(crate::document::TextSize::ZERO, range.len()),
    );
    Some(error)
}

fn opens_kind_annotation(parser: &mut Parser<'_>) -> bool {
    if ["<-", "<=", "<+"]
        .iter()
        .any(|prefix| parser.cursor().starts_with(prefix))
    {
        return false;
    }
    // Only a complete annotation may hide a sibling delimiter. The same `<`
    // also starts comparisons, so punctuation alone cannot select its owner.
    // Reuse the canonical candidate transaction and its shared resource limits;
    // failed syntax is rewound while fuel and recovery work remain charged.
    let checkpoint = parser.checkpoint();
    let matched = super::canonical::recursive_core::parse_kind_annotation_candidate(parser)
        == super::canonical::combinator::Attempt::Matched;
    parser.rewind(checkpoint);
    matched
}

fn recovery_boundary(character: char, delimiters: &[char], should_stop: bool) -> bool {
    if !should_stop {
        return false;
    }
    let Some(opener) = delimiters.last().copied() else {
        return true;
    };
    is_recovery_closer(character) && !delimiters_match(opener, character)
}

fn is_recovery_closer(character: char) -> bool {
    matches!(character, ')' | ']' | '}' | '>' | '⟩' | '╯' | '┘' | '┛')
}

fn delimiters_match(opener: char, closer: char) -> bool {
    matches!(
        (opener, closer),
        ('(', ')') | ('[', ']') | ('{', '}') | ('<' | '⟨', '>' | '⟩')
    )
}

pub(crate) fn insert_missing(
    parser: &mut Parser<'_>,
    code: &str,
    message: &str,
    expected: ExpectedSyntax,
    token: Option<SyntaxKind>,
) -> CompletedMarker {
    let at = parser.offset();
    let marker = parser.start();
    if let Some(token) = token {
        parser.missing_token(token);
    }
    let missing = marker.complete_with_flags(parser, SyntaxKind::Missing, NodeFlags::MISSING);
    let range = TextRange::empty(at);
    let diagnostic = Diagnostic {
        id: parser.next_diagnostic_id(),
        code: DiagnosticCode::from(code),
        phase: DiagnosticPhase::Syntax,
        severity: Severity::Error,
        rule: parser.current_rule(),
        context: parser.current_context(),
        primary: DiagnosticAnchor::Absolute {
            revision: parser.source().revision(),
            range,
        },
        labels: alloc::vec![],
        expected: alloc::vec![expected.clone()],
        found: Some(parser.found_syntax()),
        fixes: alloc::vec![],
        related: alloc::vec![],
        recovery: Some(RecoveryAction::Insert {
            syntax: expected,
            at,
        }),
        tags: DiagnosticTags::NONE,
        message: String::from(message),
    };
    parser.push_diagnostic(
        diagnostic,
        Some(missing.position()),
        TextRange::empty(crate::document::TextSize::ZERO),
    );
    missing
}

pub(crate) fn abandon_error(
    parser: &mut Parser<'_>,
    class: RecoveryClass,
    target: RuleId,
    code: &str,
    message: &str,
) -> ParseFailure {
    let start = parser.offset();
    let _ = skip_error(parser, class, code, message);
    let range = TextRange::new(start, parser.offset());
    let at = parser.offset();
    if let Some(diagnostic) = parser.last_diagnostic_mut() {
        diagnostic.recovery = Some(RecoveryAction::Abandon { rule: target, at });
    }
    ParseFailure {
        context: parser
            .current_context()
            .expect("abandon recovery requires a parser context"),
        range,
    }
}

fn should_stop(
    parser: &Parser<'_>,
    class: RecoveryClass,
    start: crate::document::TextSize,
) -> bool {
    if parser.offset() == start {
        return false;
    }
    match class {
        RecoveryClass::MechItem => {
            is_newline_start(parser.cursor())
                || parser.cursor().starts_with(";")
                || parser.is_strong_document_boundary()
        }
        RecoveryClass::Paragraph => {
            is_newline_start(parser.cursor()) || parser.is_context_fence_start()
        }
        RecoveryClass::Fence => false,
    }
}

pub(crate) fn nesting_limit(parser: &mut Parser<'_>) {
    let start = parser.offset();
    let marker = parser.start();
    let mut nested = 0_u32;
    while !parser.is_eof() && remaining_recovery_bytes(parser) > 0 {
        if nesting_should_stop(parser, nested) {
            break;
        }
        let Some(character) = parser.cursor().peek_char() else {
            break;
        };
        if character.len_utf8() as u32 > remaining_recovery_bytes(parser) {
            parser.halt();
            break;
        }
        let Some((character, range)) = parser.bump_char_raw() else {
            break;
        };
        if character == '(' {
            nested = nested.saturating_add(1);
        } else if character == ')' {
            nested = nested.saturating_sub(1);
        }
        charge_recovery_bytes(parser, range.len().0);
        parser.token_with_flags(token_kind_for_char(character), range, TokenFlags::ERROR);
    }
    if remaining_recovery_bytes(parser) == 0
        && !parser.is_eof()
        && !nesting_should_stop(parser, nested)
    {
        parser.halt();
    }
    if start == parser.offset() {
        marker.abandon(parser);
        let _ = insert_missing(
            parser,
            "syntax/nesting-limit",
            "syntax nesting limit reached",
            ExpectedSyntax::Production(String::from("expression")),
            None,
        );
        return;
    }
    let error = marker.complete_with_flags(parser, SyntaxKind::Error, NodeFlags::ERROR);
    let range = TextRange::new(start, parser.offset());
    let found = parser.source().text(range).ok().map(|text| FoundSyntax {
        kind: Some(SyntaxKind::Unknown),
        text: Some(text),
    });
    let diagnostic = Diagnostic {
        id: parser.next_diagnostic_id(),
        code: DiagnosticCode::syntax("nesting-limit"),
        phase: DiagnosticPhase::Syntax,
        severity: Severity::Error,
        rule: parser.current_rule(),
        context: parser.current_context(),
        primary: DiagnosticAnchor::Absolute {
            revision: parser.source().revision(),
            range,
        },
        labels: alloc::vec![],
        expected: alloc::vec![],
        found,
        fixes: alloc::vec![],
        related: alloc::vec![],
        recovery: Some(RecoveryAction::Skip { range }),
        tags: DiagnosticTags::NONE,
        message: String::from("syntax nesting limit reached"),
    };
    parser.push_diagnostic(
        diagnostic,
        Some(error.position()),
        TextRange::new(crate::document::TextSize::ZERO, range.len()),
    );
}

fn charge_recovery_bytes(parser: &mut Parser<'_>, bytes: u32) {
    parser.stats_mut().recovery_bytes = parser
        .stats()
        .recovery_bytes
        .saturating_add(u64::from(bytes));
}

fn remaining_recovery_bytes(parser: &Parser<'_>) -> u32 {
    let used = parser.stats().recovery_bytes.min(u64::from(u32::MAX)) as u32;
    parser
        .config()
        .limits
        .max_recovery_bytes
        .saturating_sub(used)
}

fn nesting_should_stop(parser: &Parser<'_>, nested: u32) -> bool {
    nested == 0
        && (parser.cursor().starts_with(")")
            || parser.cursor().starts_with(";")
            || parser.cursor().starts_with("--")
            || parser.cursor().starts_with("//")
            || is_newline_start(parser.cursor())
            || parser.is_strong_document_boundary())
}
