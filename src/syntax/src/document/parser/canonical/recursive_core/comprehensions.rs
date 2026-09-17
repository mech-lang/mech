use crate::document::{RuleId, SyntaxKind};

use super::super::super::Parser;
use super::super::super::rule::rules;
use super::super::{base, combinator};
use super::{
    Attempt, FactAttempt, QualifierKind, expressions, nesting_limit, patterns, recover_closer,
    recover_required_production, transactional_fact, variables,
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
    let owner = if close == rules::RIGHT_BRACKET {
        rules::MATRIX_COMPREHENSION
    } else {
        rules::SET_COMPREHENSION
    };
    let mut committed = false;
    let first = qualifier(parser);
    let mut has_generator_or_let = match first {
        FactAttempt::Matched(QualifierKind::Generator | QualifierKind::Let) => true,
        FactAttempt::Matched(QualifierKind::Filter) => false,
        FactAttempt::NoMatch => {
            recover_required_production(
                parser,
                owner,
                "syntax/missing-comprehension-qualifier",
                "missing comprehension qualifier after bar",
                "comprehension-qualifier",
            );
            committed = true;
            false
        }
        FactAttempt::Committed => {
            committed = true;
            false
        }
    };
    while !parser.is_halted() {
        if !base::parse_rule(parser, rules::LIST_SEPARATOR) {
            break;
        }
        match qualifier(parser) {
            FactAttempt::Matched(QualifierKind::Generator | QualifierKind::Let) => {
                has_generator_or_let = true;
            }
            FactAttempt::Matched(QualifierKind::Filter) => {}
            FactAttempt::NoMatch => {
                recover_required_production(
                    parser,
                    owner,
                    "syntax/missing-comprehension-qualifier",
                    "missing comprehension qualifier after separator",
                    "comprehension-qualifier",
                );
                committed = true;
            }
            FactAttempt::Committed => {
                committed = true;
            }
        }
    }
    if require_generator_or_let && !has_generator_or_let && !committed {
        return Attempt::NoMatch;
    }
    let _ = base::parse_rule(parser, rules::SPACE_TAB0);
    if base::parse_rule(parser, close) {
        if committed {
            Attempt::Committed
        } else {
            Attempt::Matched
        }
    } else {
        let (kind, character, text) = if close == rules::RIGHT_BRACKET {
            (SyntaxKind::RightBracket, ']', "]")
        } else {
            (SyntaxKind::RightBrace, '}', "}")
        };
        recover_closer(parser, owner, close, kind, character, text)
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
            finish_qualifiers(parser, close, require_generator_or_let)
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
        match child {
            Attempt::Matched => {}
            Attempt::Committed => {
                node.complete(parser, SyntaxKind::Generator);
                return FactAttempt::Committed;
            }
            Attempt::NoMatch => {
                recover_required_production(
                    parser,
                    rules::GENERATOR,
                    "syntax/missing-generator-source",
                    "missing source expression after generator arrow",
                    "expression",
                );
                node.complete(parser, SyntaxKind::Generator);
                return FactAttempt::Committed;
            }
        }
        node.complete(parser, SyntaxKind::Generator);
        FactAttempt::Matched(QualifierKind::Generator)
    })
}
