use crate::document::{RuleId, SyntaxKind};

use super::super::super::Parser;
use super::super::super::rule::rules;
use super::super::{base, combinator, paths, subscript_primitives};
use super::{Attempt, child_result, expressions, nesting_limit, precedence};

pub(super) fn parse_subscript(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::SUBSCRIPT, |parser| {
        let node = parser.start();
        let first = subscript_item(parser);
        if let Some(result) = child_result(parser, node, SyntaxKind::SubscriptList, first) {
            return result;
        }
        loop {
            let before = parser.offset();
            match subscript_item(parser) {
                Attempt::Matched if parser.offset() > before => {}
                Attempt::Matched | Attempt::NoMatch => break,
                Attempt::Committed => {
                    node.complete(parser, SyntaxKind::SubscriptList);
                    return Attempt::Committed;
                }
            }
        }
        node.complete(parser, SyntaxKind::SubscriptList);
        Attempt::Matched
    })
}

pub(super) fn parse_slice(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::SLICE, |parser| {
        let node = parser.start();
        if !paths::parse_prefixed_context_path(parser).accepted()
            && !base::parse_rule(parser, rules::IDENTIFIER)
        {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let child = parse_subscript(parser);
        if let Some(result) = child_result(parser, node, SyntaxKind::Slice, child) {
            return result;
        }
        node.complete(parser, SyntaxKind::Slice);
        Attempt::Matched
    })
}

pub(super) fn parse_bracket_subscript(parser: &mut Parser<'_>) -> Attempt {
    delimited_subscript(
        parser,
        rules::BRACKET_SUBSCRIPT,
        rules::LEFT_BRACKET,
        rules::RIGHT_BRACKET,
        SyntaxKind::BracketSubscript,
    )
}

pub(super) fn parse_brace_subscript(parser: &mut Parser<'_>) -> Attempt {
    delimited_subscript(
        parser,
        rules::BRACE_SUBSCRIPT,
        rules::LEFT_BRACE,
        rules::RIGHT_BRACE,
        SyntaxKind::BraceSubscript,
    )
}

pub(super) fn parse_formula_subscript(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::FORMULA_SUBSCRIPT, |parser| {
        let node = parser.start();
        let child = expressions::parse_formula(parser);
        if let Some(result) = child_result(parser, node, SyntaxKind::FormulaSubscript, child) {
            return result;
        }
        node.complete(parser, SyntaxKind::FormulaSubscript);
        Attempt::Matched
    })
}

pub(super) fn parse_range_subscript(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::RANGE_SUBSCRIPT, |parser| {
        let node = parser.start();
        let child = precedence::parse_range_expression(parser);
        if let Some(result) = child_result(parser, node, SyntaxKind::RangeSubscript, child) {
            return result;
        }
        node.complete(parser, SyntaxKind::RangeSubscript);
        Attempt::Matched
    })
}

fn subscript_item(parser: &mut Parser<'_>) -> Attempt {
    for parse in [
        subscript_primitives::parse_swizzle_subscript,
        subscript_primitives::parse_dot_subscript,
        subscript_primitives::parse_dot_subscript_int,
        parse_bracket_subscript,
        parse_brace_subscript,
    ] {
        let result = parse(parser);
        if result != Attempt::NoMatch {
            return result;
        }
    }
    Attempt::NoMatch
}

fn delimited_subscript(
    parser: &mut Parser<'_>,
    rule: RuleId,
    open: RuleId,
    close: RuleId,
    kind: SyntaxKind,
) -> Attempt {
    combinator::transactional(parser, rule, |parser| {
        let node = parser.start();
        if !base::parse_rule(parser, open) {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let Some(interior) = parser.with_nesting(|parser| {
            let first = subscript_value(parser);
            if first != Attempt::Matched {
                return first;
            }
            loop {
                let pair = parser.checkpoint();
                let before = parser.offset();
                if !base::parse_rule(parser, rules::LIST_SEPARATOR) {
                    break;
                }
                match subscript_value(parser) {
                    Attempt::Matched if parser.offset() > before => {}
                    Attempt::Matched | Attempt::NoMatch => {
                        parser.rewind(pair);
                        break;
                    }
                    Attempt::Committed => return Attempt::Committed,
                }
            }
            if base::parse_rule(parser, close) {
                Attempt::Matched
            } else {
                Attempt::NoMatch
            }
        }) else {
            let result = nesting_limit(parser);
            node.complete(parser, kind);
            return result;
        };
        match interior {
            Attempt::Matched => {
                node.complete(parser, kind);
                Attempt::Matched
            }
            Attempt::NoMatch => {
                node.abandon(parser);
                Attempt::NoMatch
            }
            Attempt::Committed => {
                node.complete(parser, kind);
                Attempt::Committed
            }
        }
    })
}

fn subscript_value(parser: &mut Parser<'_>) -> Attempt {
    let all = subscript_primitives::parse_select_all(parser);
    if all != Attempt::NoMatch {
        return all;
    }
    let range = parse_range_subscript(parser);
    if range != Attempt::NoMatch {
        return range;
    }
    parse_formula_subscript(parser)
}
