use alloc::string::String;

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

mod skip;
pub(crate) use skip::{SkipContinuation, SkipProgress};

pub(crate) fn skip_error(
    parser: &mut Parser<'_>,
    class: RecoveryClass,
    code: &str,
    message: &str,
) -> Option<CompletedMarker> {
    let mut continuation = SkipContinuation::new(class, code, message);
    loop {
        let mut allowance = u64::MAX;
        match continuation.advance(parser, true, &mut allowance) {
            SkipProgress::Complete(result) => return result,
            SkipProgress::NeedsProcessing => {}
            _ => unreachable!("final recovery input"),
        }
    }
}

/// Preserve unexpected bytes until a sibling or ancestor production can
/// restart. Boundary characters remain unconsumed for the owning production.
#[cfg(test)]
pub(crate) fn abandon_to_restart(
    parser: &mut Parser<'_>,
    target: RuleId,
    boundaries: &[char],
    code: &str,
    message: &str,
) -> Option<CompletedMarker> {
    abandon_to_restart_with_prefixes(parser, target, boundaries, &[], code, message)
}

#[cfg(test)]
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

mod abandon;
pub(crate) use abandon::{AbandonContinuation, AbandonProgress, BoundaryProgress};

#[cfg(test)]
pub(crate) fn abandon_until(
    parser: &mut Parser<'_>,
    target: RuleId,
    code: &str,
    message: &str,
    mut should_stop: impl FnMut(&mut Parser<'_>, char) -> bool,
) -> Option<CompletedMarker> {
    let mut continuation = AbandonContinuation::new(target, code, message);
    loop {
        let mut allowance = u64::MAX;
        match continuation.advance(
            parser,
            true,
            &mut allowance,
            |parser, character, _, allowance| {
                if *allowance == 0 {
                    return BoundaryProgress::NeedsProcessing;
                }
                *allowance -= 1;
                BoundaryProgress::Complete(should_stop(parser, character))
            },
        ) {
            AbandonProgress::Complete(result) => return result,
            AbandonProgress::NeedsProcessing => {}
            _ => unreachable!("final abandonment input"),
        }
    }
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

mod missing;
pub(crate) use missing::{MissingContinuation, MissingProgress};

pub(crate) fn insert_missing(
    parser: &mut Parser<'_>,
    code: &str,
    message: &str,
    expected: ExpectedSyntax,
    token: Option<SyntaxKind>,
) -> CompletedMarker {
    let mut continuation = MissingContinuation::new(code, message, expected, token);
    loop {
        let mut allowance = u64::MAX;
        match continuation.advance(parser, true, &mut allowance) {
            MissingProgress::Complete(marker) => return marker,
            MissingProgress::NeedsProcessing => {}
            _ => unreachable!("final missing recovery"),
        }
    }
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

mod nesting;
pub(crate) use nesting::{NestingContinuation, NestingProgress};
pub(crate) fn nesting_limit(parser: &mut Parser<'_>) {
    let mut child = NestingContinuation::new();
    loop {
        let mut allowance = u64::MAX;
        match child.advance(parser, true, &mut allowance) {
            NestingProgress::Complete => return,
            NestingProgress::NeedsProcessing => {}
            _ => unreachable!("sealed nesting recovery"),
        }
    }
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
