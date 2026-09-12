use crate::document::{RuleId, SyntaxKind};

use super::super::super::Parser;
use super::super::super::rule::rules;
use super::super::{base, combinator};
use super::{
    Attempt, child_result, expressions, nesting_limit, recover_closer, recover_required_production,
};

pub(super) fn parse_argument_list(parser: &mut Parser<'_>) -> Attempt {
    argument_list(parser, rules::ARGUMENT_LIST, SyntaxKind::ArgumentList)
}

pub(super) fn parse_function_call(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::FUNCTION_CALL, |parser| {
        let node = parser.start();
        if !base::parse_rule(parser, rules::IDENTIFIER) {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let child = parse_argument_list(parser);
        if let Some(result) = child_result(parser, node, SyntaxKind::FunctionCall, child) {
            return result;
        }
        node.complete(parser, SyntaxKind::FunctionCall);
        Attempt::Matched
    })
}

pub(super) fn parse_call_arg_with_binding(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::CALL_ARG_WITH_BINDING, |parser| {
        let node = parser.start();
        if !base::parse_rule(parser, rules::IDENTIFIER)
            || !base::parse_rule(parser, rules::WHITESPACE0)
            || !base::parse_rule(parser, rules::COLON)
            || !base::parse_rule(parser, rules::WHITESPACE0)
        {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let child = expressions::parse_expression(parser);
        match child {
            Attempt::Matched => {}
            Attempt::Committed => {
                node.complete(parser, SyntaxKind::BoundCallArgument);
                return Attempt::Committed;
            }
            Attempt::NoMatch => {
                recover_required_production(
                    parser,
                    rules::CALL_ARG_WITH_BINDING,
                    "syntax/missing-bound-call-argument-value",
                    "missing value for bound call argument",
                    "expression",
                );
                node.complete(parser, SyntaxKind::BoundCallArgument);
                return Attempt::Committed;
            }
        }
        node.complete(parser, SyntaxKind::BoundCallArgument);
        Attempt::Matched
    })
}

pub(super) fn parse_call_arg(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::CALL_ARG, |parser| {
        let node = parser.start();
        let child = expressions::parse_expression(parser);
        if let Some(result) = child_result(parser, node, SyntaxKind::CallArgument, child) {
            return result;
        }
        node.complete(parser, SyntaxKind::CallArgument);
        Attempt::Matched
    })
}

pub(super) fn argument_list(parser: &mut Parser<'_>, rule: RuleId, kind: SyntaxKind) -> Attempt {
    combinator::transactional(parser, rule, |parser| {
        let node = parser.start();
        if !base::parse_rule(parser, rules::LEFT_PARENTHESIS) {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let Some(interior) = parser.with_nesting(|parser| {
            if base::parse_rule(parser, rules::RIGHT_PARENTHESIS) {
                return Attempt::Matched;
            }
            let mut committed = false;
            match call_argument(parser) {
                Attempt::Matched => {}
                Attempt::NoMatch => {
                    recover_required_production(
                        parser,
                        rule,
                        "syntax/missing-call-argument",
                        "missing call argument",
                        "call-arg",
                    );
                    committed = true;
                }
                Attempt::Committed => committed = true,
            }
            while !parser.is_halted() {
                let pair = parser.checkpoint();
                if !base::parse_rule(parser, rules::LIST_SEPARATOR) {
                    break;
                }
                match call_argument(parser) {
                    Attempt::Matched if parser.offset() > pair.cursor.offset => {}
                    Attempt::Matched | Attempt::NoMatch => {
                        recover_required_production(
                            parser,
                            rule,
                            "syntax/missing-call-argument",
                            "missing call argument after separator",
                            "call-arg",
                        );
                        committed = true;
                        break;
                    }
                    Attempt::Committed => {
                        committed = true;
                        break;
                    }
                }
            }
            if base::parse_rule(parser, rules::RIGHT_PARENTHESIS) {
                if committed {
                    Attempt::Committed
                } else {
                    Attempt::Matched
                }
            } else {
                recover_closer(
                    parser,
                    rule,
                    rules::RIGHT_PARENTHESIS,
                    SyntaxKind::RightParen,
                    ')',
                    ")",
                )
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

fn call_argument(parser: &mut Parser<'_>) -> Attempt {
    let bound = parse_call_arg_with_binding(parser);
    if bound == Attempt::NoMatch {
        parse_call_arg(parser)
    } else {
        bound
    }
}
