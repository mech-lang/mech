use crate::document::SyntaxKind;

use super::super::super::Parser;
use super::super::super::rule::rules;
use super::super::{base, combinator, paths};
use super::{Attempt, child_result, expressions, kinds, recover_required_production};

pub(super) fn parse_var(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::VAR, |parser| {
        let node = parser.start();
        let stem = paths::parse_prefixed_context_path(parser).accepted()
            || base::parse_rule(parser, rules::IDENTIFIER);
        if !stem {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        if kinds::parse_kind_annotation(parser) == Attempt::Committed {
            node.complete(parser, SyntaxKind::Variable);
            return Attempt::Committed;
        }
        node.complete(parser, SyntaxKind::Variable);
        Attempt::Matched
    })
}

pub(super) fn parse_variable_define(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::VARIABLE_DEFINE, |parser| {
        let node = parser.start();
        let _ = base::parse_rule(parser, rules::TILDE);
        let child = parse_var(parser);
        if let Some(result) = child_result(parser, node, SyntaxKind::VariableDefine, child) {
            return result;
        }

        let lookahead = parser.checkpoint();
        let assign = base::parse_rule(parser, rules::ASSIGN_OPERATOR);
        parser.rewind(lookahead);
        if assign || !base::parse_rule(parser, rules::DEFINE_OPERATOR) {
            node.abandon(parser);
            return Attempt::NoMatch;
        }

        let child = expressions::parse_expression(parser);
        match child {
            Attempt::Matched => {}
            Attempt::Committed => {
                node.complete(parser, SyntaxKind::VariableDefine);
                return Attempt::Committed;
            }
            Attempt::NoMatch => {
                recover_required_production(
                    parser,
                    rules::VARIABLE_DEFINE,
                    "syntax/missing-variable-definition-value",
                    "missing value after definition operator",
                    "expression",
                );
                node.complete(parser, SyntaxKind::VariableDefine);
                return Attempt::Committed;
            }
        }
        node.complete(parser, SyntaxKind::VariableDefine);
        Attempt::Matched
    })
}
