use crate::document::{RuleId, SyntaxKind};

use super::super::super::Parser;
use super::super::super::rule::rules;
use super::super::{base, combinator};
use super::{
    Attempt, FactAttempt, QualifierKind, child_result, expressions, nesting_limit, patterns,
    transactional_fact, variables,
};

pub(super) fn parse_set_comprehension(parser: &mut Parser<'_>) -> Attempt {
    comprehension(
        parser,
        rules::SET_COMPREHENSION,
        rules::LEFT_BRACE,
        rules::RIGHT_BRACE,
        SyntaxKind::SetComprehension,
        false,
    )
}

pub(super) fn parse_matrix_comprehension(parser: &mut Parser<'_>) -> Attempt {
    super::structures::matrix_comprehension(parser)
}

pub(super) fn finish_qualifiers(
    parser: &mut Parser<'_>,
    close: RuleId,
    require_generator_or_let: bool,
) -> Attempt {
    if !base::parse_rule(parser, rules::SPACE_TAB0) {
        return Attempt::NoMatch;
    }
    let first = qualifier(parser);
    let mut has_generator_or_let = match first {
        FactAttempt::Matched(QualifierKind::Generator | QualifierKind::Let) => true,
        FactAttempt::Matched(QualifierKind::Filter) => false,
        FactAttempt::NoMatch => return Attempt::NoMatch,
        FactAttempt::Committed => return Attempt::Committed,
    };
    loop {
        let pair = parser.checkpoint();
        if !base::parse_rule(parser, rules::LIST_SEPARATOR) {
            break;
        }
        match qualifier(parser) {
            FactAttempt::Matched(QualifierKind::Generator | QualifierKind::Let) => {
                has_generator_or_let = true;
            }
            FactAttempt::Matched(QualifierKind::Filter) => {}
            FactAttempt::NoMatch => {
                parser.rewind(pair);
                break;
            }
            FactAttempt::Committed => return Attempt::Committed,
        }
    }
    if require_generator_or_let && !has_generator_or_let {
        return Attempt::NoMatch;
    }
    if !base::parse_rule(parser, rules::SPACE_TAB0) || !base::parse_rule(parser, close) {
        Attempt::NoMatch
    } else {
        Attempt::Matched
    }
}

pub(super) fn parse_comprehension_qualifier(parser: &mut Parser<'_>) -> Attempt {
    qualifier(parser).attempt()
}

pub(super) fn parse_generator(parser: &mut Parser<'_>) -> Attempt {
    generator(parser).attempt()
}

fn comprehension(
    parser: &mut Parser<'_>,
    rule: RuleId,
    open: RuleId,
    close: RuleId,
    kind: SyntaxKind,
    require_generator_or_let: bool,
) -> Attempt {
    combinator::transactional(parser, rule, |parser| {
        let node = parser.start();
        if !base::parse_rule(parser, open) {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let Some(interior) = parser.with_nesting(|parser| {
            if !base::parse_rule(parser, rules::SPACE_TAB0) {
                return Attempt::NoMatch;
            }
            let expression = expressions::parse_expression(parser);
            if expression != Attempt::Matched {
                return expression;
            }
            if !base::parse_rule(parser, rules::SPACE_TAB0)
                || !base::parse_rule(parser, rules::BAR)
                || !base::parse_rule(parser, rules::SPACE_TAB0)
            {
                return Attempt::NoMatch;
            }
            let first = qualifier(parser);
            let mut has_generator_or_let = match first {
                FactAttempt::Matched(QualifierKind::Generator | QualifierKind::Let) => true,
                FactAttempt::Matched(QualifierKind::Filter) => false,
                FactAttempt::NoMatch => return Attempt::NoMatch,
                FactAttempt::Committed => return Attempt::Committed,
            };
            loop {
                let pair = parser.checkpoint();
                if !base::parse_rule(parser, rules::LIST_SEPARATOR) {
                    break;
                }
                match qualifier(parser) {
                    FactAttempt::Matched(QualifierKind::Generator | QualifierKind::Let) => {
                        has_generator_or_let = true;
                    }
                    FactAttempt::Matched(QualifierKind::Filter) => {}
                    FactAttempt::NoMatch => {
                        parser.rewind(pair);
                        break;
                    }
                    FactAttempt::Committed => return Attempt::Committed,
                }
            }
            if require_generator_or_let && !has_generator_or_let {
                return Attempt::NoMatch;
            }
            if !base::parse_rule(parser, rules::SPACE_TAB0) || !base::parse_rule(parser, close) {
                Attempt::NoMatch
            } else {
                Attempt::Matched
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

fn qualifier(parser: &mut Parser<'_>) -> FactAttempt<QualifierKind> {
    transactional_fact(parser, rules::COMPREHENSION_QUALIFIER, |parser| {
        let node = parser.start();
        let selected = generator(parser);
        let kind = match selected {
            FactAttempt::Matched(kind) => kind,
            FactAttempt::Committed => {
                node.complete(parser, SyntaxKind::ComprehensionQualifier);
                return FactAttempt::Committed;
            }
            FactAttempt::NoMatch => match variables::parse_variable_define(parser) {
                Attempt::Matched => QualifierKind::Let,
                Attempt::Committed => {
                    node.complete(parser, SyntaxKind::ComprehensionQualifier);
                    return FactAttempt::Committed;
                }
                Attempt::NoMatch => match expressions::parse_expression(parser) {
                    Attempt::Matched => QualifierKind::Filter,
                    Attempt::Committed => {
                        node.complete(parser, SyntaxKind::ComprehensionQualifier);
                        return FactAttempt::Committed;
                    }
                    Attempt::NoMatch => {
                        node.abandon(parser);
                        return FactAttempt::NoMatch;
                    }
                },
            },
        };
        node.complete(parser, SyntaxKind::ComprehensionQualifier);
        FactAttempt::Matched(kind)
    })
}

fn generator(parser: &mut Parser<'_>) -> FactAttempt<QualifierKind> {
    transactional_fact(parser, rules::GENERATOR, |parser| {
        let node = parser.start();
        match patterns::pattern_with_facts(parser) {
            FactAttempt::Matched(_) => {}
            FactAttempt::NoMatch => {
                node.abandon(parser);
                return FactAttempt::NoMatch;
            }
            FactAttempt::Committed => {
                node.complete(parser, SyntaxKind::Generator);
                return FactAttempt::Committed;
            }
        }
        if !base::parse_rule(parser, rules::SPACE_TAB0)
            || (!base::parse_rule(parser, rules::GENERATOR_ARROW)
                && !base::parse_rule(parser, rules::GENERATOR_ARROW_U))
            || !base::parse_rule(parser, rules::SPACE_TAB0)
        {
            node.abandon(parser);
            return FactAttempt::NoMatch;
        }
        let child = expressions::parse_expression(parser);
        if let Some(result) = child_result(parser, node, SyntaxKind::Generator, child) {
            return match result {
                Attempt::Committed => FactAttempt::Committed,
                _ => FactAttempt::NoMatch,
            };
        }
        node.complete(parser, SyntaxKind::Generator);
        FactAttempt::Matched(QualifierKind::Generator)
    })
}
