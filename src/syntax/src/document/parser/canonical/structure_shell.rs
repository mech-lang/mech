//! Canonical closed structure-shell productions for Phase 2H.
//!
//! Complete matrix, table, map, set, and structure parents remain outside this
//! direct-rule island. Every production here is transactional and retains only
//! its own physical source prefix.

use crate::document::{RuleId, SyntaxKind};

use super::super::rule::rules;
use super::super::Parser;
use super::base;
use super::combinator::{self, Attempt};
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

/// Parse an opening matrix delimiter in its formal source order.
pub(crate) fn parse_matrix_start(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::MATRIX_START, |parser| {
        if base::parse_rule(parser, rules::BOX_TL_ROUND)
            || base::parse_rule(parser, rules::BOX_TL)
            || base::parse_rule(parser, rules::BOX_TL_BOLD)
            || base::parse_rule(parser, rules::LEFT_BRACKET)
        {
            Attempt::Matched
        } else {
            Attempt::NoMatch
        }
    })
}

/// Parse a closing matrix delimiter in its formal source order.
pub(crate) fn parse_matrix_end(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::MATRIX_END, |parser| {
        if base::parse_rule(parser, rules::BOX_BR_ROUND)
            || base::parse_rule(parser, rules::BOX_BR)
            || base::parse_rule(parser, rules::BOX_BR_BOLD)
            || base::parse_rule(parser, rules::RIGHT_BRACKET)
        {
            Attempt::Matched
        } else {
            Attempt::NoMatch
        }
    })
}

/// Parse an opening table delimiter in its formal source order.
pub(crate) fn parse_table_start(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::TABLE_START, |parser| {
        if base::parse_rule(parser, rules::BOX_TL_ROUND)
            || base::parse_rule(parser, rules::BOX_TL)
            || base::parse_rule(parser, rules::BOX_TL_BOLD)
            || base::parse_rule(parser, rules::LEFT_BRACE)
            || parse_table_separator(parser).accepted()
        {
            Attempt::Matched
        } else {
            Attempt::NoMatch
        }
    })
}

/// Parse a closing table delimiter in its formal source order.
pub(crate) fn parse_table_end(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::TABLE_END, |parser| {
        if base::parse_rule(parser, rules::BOX_BR_ROUND)
            || base::parse_rule(parser, rules::BOX_BR)
            || base::parse_rule(parser, rules::BOX_BR_BOLD)
            || base::parse_rule(parser, rules::RIGHT_BRACE)
            || parse_table_separator(parser).accepted()
        {
            Attempt::Matched
        } else {
            Attempt::NoMatch
        }
    })
}

/// Parse the horizontal-trivia-aware vertical table separator.
pub(crate) fn parse_table_separator(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::TABLE_SEPARATOR, |parser| {
        if !base::parse_rule(parser, rules::SPACE_TAB0)
            || !(base::parse_rule(parser, rules::BOX_VERT)
                || base::parse_rule(parser, rules::BOX_VERT_BOLD)
                || base::parse_rule(parser, rules::BAR))
            || !base::parse_rule(parser, rules::SPACE_TAB0)
        {
            return Attempt::NoMatch;
        }
        Attempt::Matched
    })
}

/// Parse one horizontal table-border token in its formal source order.
pub(crate) fn parse_table_horz(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::TABLE_HORZ, |parser| {
        if base::parse_rule(parser, rules::DASH) || base::parse_rule(parser, rules::BOX_HORZ) {
            Attempt::Matched
        } else {
            Attempt::NoMatch
        }
    })
}

/// Parse a transparent table top: opening delimiter, zero or more box
/// characters, then one physical newline.
pub(crate) fn parse_table_top(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::TABLE_TOP, |parser| {
        if !parse_table_start(parser).accepted() {
            return Attempt::NoMatch;
        }
        while base::parse_rule(parser, rules::BOX_DRAWING_CHAR) {
            if parser.is_halted() {
                break;
            }
        }
        if !base::parse_rule(parser, rules::NEW_LINE) {
            return Attempt::NoMatch;
        }
        Attempt::Matched
    })
}

/// Parse a table-border row separator as a structural, empty compatibility
/// row. The item choice intentionally follows the legacy source order.
pub(crate) fn parse_row_separator(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::ROW_SEPARATOR, |parser| {
        let separator = parser.start();
        if !base::parse_rule(parser, rules::SPACE_TAB0) || !parse_row_separator_item(parser) {
            separator.abandon(parser);
            return Attempt::NoMatch;
        }
        while parse_row_separator_item(parser) {
            if parser.is_halted() {
                break;
            }
        }
        if !base::parse_rule(parser, rules::SPACE_TAB0) {
            separator.abandon(parser);
            return Attempt::NoMatch;
        }
        separator.complete(parser, SyntaxKind::TableRowSeparator);
        Attempt::Matched
    })
}

/// Parse the closed empty-map form without claiming a general map parent.
pub(crate) fn parse_empty_map(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::EMPTY_MAP, |parser| {
        let map = parser.start();
        if !base::parse_rule(parser, rules::LEFT_BRACE)
            || !base::parse_rule(parser, rules::WHITESPACE0)
            || !base::parse_rule(parser, rules::COLON)
            || !base::parse_rule(parser, rules::WHITESPACE0)
            || !base::parse_rule(parser, rules::RIGHT_BRACE)
        {
            map.abandon(parser);
            return Attempt::NoMatch;
        }
        map.complete(parser, SyntaxKind::EmptyMap);
        Attempt::Matched
    })
}

/// Parse the closed empty-set form, preserving an optional empty marker in
/// concrete syntax while lowering all spellings to the same empty set value.
pub(crate) fn parse_empty_set(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::EMPTY_SET, |parser| {
        let set = parser.start();
        if !base::parse_rule(parser, rules::LEFT_BRACE)
            || !base::parse_rule(parser, rules::WHITESPACE0)
        {
            set.abandon(parser);
            return Attempt::NoMatch;
        }
        let _ = literals::parse_empty(parser);
        if !base::parse_rule(parser, rules::WHITESPACE0)
            || !base::parse_rule(parser, rules::RIGHT_BRACE)
        {
            set.abandon(parser);
            return Attempt::NoMatch;
        }
        set.complete(parser, SyntaxKind::EmptySet);
        Attempt::Matched
    })
}

fn parse_row_separator_item(parser: &mut Parser<'_>) -> bool {
    base::parse_rule(parser, rules::BOX_DRAWING_CHAR)
        || parse_table_end(parser).accepted()
        || base::parse_rule(parser, rules::SPACE_TAB)
}
