//! Found-syntax classification for canonical grammar diagnostics.

use alloc::string::{String, ToString};

use crate::document::{FoundSyntax, SyntaxKind, TextSize};

use super::super::Parser;
use super::combinator::is_grammar_ignored;
use super::terminal_spec::{FIXED_TERMINALS, FixedTerminalSpec};

/// Classify the next logical syntax at `at` using the canonical lexical
/// contracts and the grammar root's ignored-trivia behavior.
pub(crate) fn found_syntax(parser: &Parser<'_>, at: TextSize) -> FoundSyntax {
    let mut continuation = FilteredContinuation::new(at);
    loop {
        let mut allowance = u64::MAX;
        match continuation.advance(
            parser.source(),
            parser.cursor().context_end(),
            true,
            &mut allowance,
        ) {
            SourceProgress::Complete(found) => return found,
            SourceProgress::NeedsProcessing => {}
            _ => unreachable!("final filtered found syntax"),
        }
    }
}

/// Classify physical source syntax without grammar-global trivia removal.
pub(crate) fn source_found_syntax(parser: &Parser<'_>, at: TextSize) -> FoundSyntax {
    let mut continuation = SourceContinuation::new(at);
    loop {
        let mut allowance = u64::MAX;
        match continuation.advance(
            parser.source(),
            parser.cursor().context_end(),
            true,
            &mut allowance,
        ) {
            SourceProgress::Complete(found) => return found,
            SourceProgress::NeedsProcessing => {}
            _ => unreachable!("final physical found syntax"),
        }
    }
}

mod filtered;
pub(crate) use filtered::FilteredContinuation;
mod source;
pub(crate) use source::{SourceContinuation, SourceProgress};

fn is_canonical_emoji(first: char) -> bool {
    !first.is_alphanumeric() && !first.is_ascii()
}

fn eof() -> FoundSyntax {
    FoundSyntax {
        kind: Some(SyntaxKind::Eof),
        text: None,
    }
}
