//! Canonical statement-boundary and control-operator primitives for Phase 2G.
//!
//! These are direct recognition leaves only. Their statement, match, and
//! state-machine parents remain outside this closed island.

use crate::document::{RuleId, SyntaxKind};

use super::super::Parser;
use super::super::rule::rules;
use super::base;
use super::combinator::{self, Attempt};

/// The Phase 2G direct statement-boundary and control operator rules.
pub(crate) const PHASE_2G_CONTROL_RULES: &[RuleId; 9] = &[
    rules::STATEMENT_SEPARATOR,
    rules::OP_ASSIGN_OPERATOR,
    rules::ADD_ASSIGN_OPERATOR,
    rules::SUB_ASSIGN_OPERATOR,
    rules::MUL_ASSIGN_OPERATOR,
    rules::DIV_ASSIGN_OPERATOR,
    rules::EXP_ASSIGN_OPERATOR,
    rules::SEND_OPERATOR,
    rules::GUARD_OPERATOR,
];

/// Whether `rule` belongs to the Phase 2G control-operator layer.
pub(crate) fn supports(rule: RuleId) -> bool {
    PHASE_2G_CONTROL_RULES.contains(&rule)
}

/// Dispatch one exact Phase 2G control operator.
pub(crate) fn parse_rule(parser: &mut Parser<'_>, rule: RuleId) -> Option<Attempt> {
    supports(rule).then(|| match rule {
        rules::STATEMENT_SEPARATOR => parse_statement_separator(parser),
        rules::OP_ASSIGN_OPERATOR => parse_op_assign_operator(parser),
        rules::ADD_ASSIGN_OPERATOR => parse_add_assign_operator(parser),
        rules::SUB_ASSIGN_OPERATOR => parse_sub_assign_operator(parser),
        rules::MUL_ASSIGN_OPERATOR => parse_mul_assign_operator(parser),
        rules::DIV_ASSIGN_OPERATOR => parse_div_assign_operator(parser),
        rules::EXP_ASSIGN_OPERATOR => parse_exp_assign_operator(parser),
        rules::SEND_OPERATOR => parse_send_operator(parser),
        rules::GUARD_OPERATOR => parse_guard_operator(parser),
        _ => unreachable!("Phase 2G control support guard rejects every other RuleId"),
    })
}

/// Parse a transparent semicolon separator with surrounding `whitespace0`.
pub(crate) fn parse_statement_separator(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::STATEMENT_SEPARATOR, |parser| {
        if !base::parse_rule(parser, rules::WHITESPACE0)
            || !base::parse_rule(parser, rules::SEMICOLON)
            || !base::parse_rule(parser, rules::WHITESPACE0)
        {
            return Attempt::NoMatch;
        }
        Attempt::Matched
    })
}

/// Parse the ordered aggregate of assignment operator leaves.
pub(crate) fn parse_op_assign_operator(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::OP_ASSIGN_OPERATOR, |parser| {
        let operator = parser.start();
        let result = first_assignment_operator(parser);
        if result == Attempt::NoMatch {
            operator.abandon(parser);
            return Attempt::NoMatch;
        }
        operator.complete(parser, SyntaxKind::OpAssignOperator);
        result
    })
}

fn first_assignment_operator(parser: &mut Parser<'_>) -> Attempt {
    let result = parse_add_assign_operator(parser);
    if result != Attempt::NoMatch {
        return result;
    }
    let result = parse_sub_assign_operator(parser);
    if result != Attempt::NoMatch {
        return result;
    }
    let result = parse_mul_assign_operator(parser);
    if result != Attempt::NoMatch {
        return result;
    }
    let result = parse_div_assign_operator(parser);
    if result != Attempt::NoMatch {
        return result;
    }
    parse_exp_assign_operator(parser)
}

/// Parse `+=` with surrounding `whitespace0` as an anonymous text terminal.
pub(crate) fn parse_add_assign_operator(parser: &mut Parser<'_>) -> Attempt {
    parse_assign_leaf(
        parser,
        rules::ADD_ASSIGN_OPERATOR,
        "+=",
        SyntaxKind::AddAssignOperation,
    )
}

/// Parse `-=` with surrounding `whitespace0` as an anonymous text terminal.
pub(crate) fn parse_sub_assign_operator(parser: &mut Parser<'_>) -> Attempt {
    parse_assign_leaf(
        parser,
        rules::SUB_ASSIGN_OPERATOR,
        "-=",
        SyntaxKind::SubAssignOperation,
    )
}

/// Parse `*=` with surrounding `whitespace0` as an anonymous text terminal.
pub(crate) fn parse_mul_assign_operator(parser: &mut Parser<'_>) -> Attempt {
    parse_assign_leaf(
        parser,
        rules::MUL_ASSIGN_OPERATOR,
        "*=",
        SyntaxKind::MulAssignOperation,
    )
}

/// Parse `/=` with surrounding `whitespace0` as an anonymous text terminal.
pub(crate) fn parse_div_assign_operator(parser: &mut Parser<'_>) -> Attempt {
    parse_assign_leaf(
        parser,
        rules::DIV_ASSIGN_OPERATOR,
        "/=",
        SyntaxKind::DivAssignOperation,
    )
}

/// Parse `^=` with surrounding `whitespace0` as an anonymous text terminal.
pub(crate) fn parse_exp_assign_operator(parser: &mut Parser<'_>) -> Attempt {
    parse_assign_leaf(
        parser,
        rules::EXP_ASSIGN_OPERATOR,
        "^=",
        SyntaxKind::ExpAssignOperation,
    )
}

/// Parse the transparent `<-` send spelling, independent of other contexts
/// that assign a different meaning to the same text.
pub(crate) fn parse_send_operator(parser: &mut Parser<'_>) -> Attempt {
    parse_transparent_whitespace_terminal(parser, rules::SEND_OPERATOR, "<-")
}

/// Parse a transparent guard marker using the existing named base terminals.
pub(crate) fn parse_guard_operator(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::GUARD_OPERATOR, |parser| {
        if !base::parse_rule(parser, rules::WHITESPACE0)
            || !(base::parse_rule(parser, rules::BAR)
                || base::parse_rule(parser, rules::BOX_VERT)
                || base::parse_rule(parser, rules::BOX_T_LEFT)
                || base::parse_rule(parser, rules::BOX_BL))
            || !base::parse_rule(parser, rules::WHITESPACE0)
        {
            return Attempt::NoMatch;
        }
        Attempt::Matched
    })
}

fn parse_assign_leaf(
    parser: &mut Parser<'_>,
    rule: RuleId,
    spelling: &str,
    kind: SyntaxKind,
) -> Attempt {
    combinator::transactional(parser, rule, |parser| {
        let operator = parser.start();
        if !base::parse_rule(parser, rules::WHITESPACE0)
            || !base::parse_exact_tag(parser, spelling, SyntaxKind::Text)
            || !base::parse_rule(parser, rules::WHITESPACE0)
        {
            operator.abandon(parser);
            return Attempt::NoMatch;
        }
        operator.complete(parser, kind);
        Attempt::Matched
    })
}

fn parse_transparent_whitespace_terminal(
    parser: &mut Parser<'_>,
    rule: RuleId,
    spelling: &str,
) -> Attempt {
    combinator::transactional(parser, rule, |parser| {
        if !base::parse_rule(parser, rules::WHITESPACE0)
            || !base::parse_exact_tag(parser, spelling, SyntaxKind::Text)
            || !base::parse_rule(parser, rules::WHITESPACE0)
        {
            return Attempt::NoMatch;
        }
        Attempt::Matched
    })
}
