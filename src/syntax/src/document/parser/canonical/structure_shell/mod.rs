//! Canonical closed structure-shell productions for Phase 2H.
//!
//! Complete matrix, table, map, set, and structure parents remain outside this
//! direct-rule island. Every production here is transactional and retains only
//! its own physical source prefix.

use crate::document::{RuleId, SyntaxKind};

use super::super::Parser;
use super::super::rule::rules;
use super::base;
use super::combinator::Attempt;
use super::literals;

/// The exact Phase 2H direct structure-shell surface.
pub(crate) const PHASE_2H_STRUCTURE_SHELL_RULES: &[RuleId; 10] = &[
    rules::MATRIX_START,
    rules::MATRIX_END,
    rules::TABLE_START,
    rules::TABLE_END,
    rules::TABLE_SEPARATOR,
    rules::TABLE_HORZ,
    rules::TABLE_TOP,
    rules::ROW_SEPARATOR,
    rules::EMPTY_MAP,
    rules::EMPTY_SET,
];

/// Whether `rule` belongs to the closed Phase 2H structure shell.
pub(crate) fn supports(rule: RuleId) -> bool {
    PHASE_2H_STRUCTURE_SHELL_RULES.contains(&rule)
}

/// Dispatch one exact Phase 2H structure-shell production.
pub(crate) fn parse_rule(parser: &mut Parser<'_>, rule: RuleId) -> Option<Attempt> {
    supports(rule).then(|| match rule {
        rules::MATRIX_START => parse_matrix_start(parser),
        rules::MATRIX_END => parse_matrix_end(parser),
        rules::TABLE_START => parse_table_start(parser),
        rules::TABLE_END => parse_table_end(parser),
        rules::TABLE_SEPARATOR => parse_table_separator(parser),
        rules::TABLE_HORZ => parse_table_horz(parser),
        rules::TABLE_TOP => parse_table_top(parser),
        rules::ROW_SEPARATOR => parse_row_separator(parser),
        rules::EMPTY_MAP => parse_empty_map(parser),
        rules::EMPTY_SET => parse_empty_set(parser),
        _ => unreachable!("Phase 2H structure-shell support guard rejects every other RuleId"),
    })
}

mod continuation;
pub(crate) use continuation::{Continuation, Progress};
fn drive(parser: &mut Parser<'_>, rule: RuleId) -> Attempt {
    let mut continuation = Continuation::new(rule);
    loop {
        let mut allowance = u64::MAX;
        match continuation.advance(parser, true, &mut allowance) {
            Progress::Complete(result) => return result,
            Progress::NeedsProcessing => {}
            _ => unreachable!("final structure-shell input"),
        }
    }
}
pub(crate) fn parse_matrix_start(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::MATRIX_START)
}
pub(crate) fn parse_matrix_end(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::MATRIX_END)
}
pub(crate) fn parse_table_start(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::TABLE_START)
}
pub(crate) fn parse_table_end(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::TABLE_END)
}
pub(crate) fn parse_table_separator(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::TABLE_SEPARATOR)
}
pub(crate) fn parse_table_horz(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::TABLE_HORZ)
}
pub(crate) fn parse_table_top(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::TABLE_TOP)
}
pub(crate) fn parse_row_separator(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::ROW_SEPARATOR)
}
pub(crate) fn parse_empty_map(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::EMPTY_MAP)
}
pub(crate) fn parse_empty_set(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::EMPTY_SET)
}
