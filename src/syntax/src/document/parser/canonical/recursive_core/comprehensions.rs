use crate::document::{RuleId, SyntaxKind};

use super::super::super::Parser;
use super::super::super::rule::rules;
use super::super::{base, combinator};
use super::{
    Attempt, FactAttempt, QualifierKind, expressions, nesting_limit, patterns, recover_closer,
    recover_required_production, variables,
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
    let mut has_generator_or_let = false;
    let mut after_separator = false;
    loop {
        let qualifier = qualifier(parser);
        has_generator_or_let |= matches!(
            qualifier.kind,
            Some(QualifierKind::Generator | QualifierKind::Let)
        );
        match qualifier.attempt {
            Attempt::Matched => {}
            Attempt::NoMatch => {
                recover_required_production(
                    parser,
                    owner,
                    "syntax/missing-comprehension-qualifier",
                    if after_separator {
                        "missing comprehension qualifier after separator"
                    } else {
                        "missing comprehension qualifier after bar"
                    },
                    "comprehension-qualifier",
                );
                committed = true;
            }
            Attempt::Committed => committed = true,
        }
        if parser.is_halted() || !base::parse_rule(parser, rules::LIST_SEPARATOR) {
            break;
        }
        after_separator = true;
    }
    if require_generator_or_let && !has_generator_or_let && !parser.is_halted() {
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
    qualifier(parser).attempt
}

pub(super) fn parse_generator(parser: &mut Parser<'_>) -> Attempt {
    generator(parser).attempt
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
            if expression == Attempt::NoMatch || parser.is_halted() {
                return expression;
            }
            if !base::parse_rule(parser, rules::SPACE_TAB0)
                || !base::parse_rule(parser, rules::BAR)
                || !base::parse_rule(parser, rules::SPACE_TAB0)
            {
                return if expression == Attempt::Committed {
                    Attempt::Committed
                } else {
                    Attempt::NoMatch
                };
            }
            let qualifiers = finish_qualifiers(parser, close, require_generator_or_let);
            if expression == Attempt::Committed && qualifiers == Attempt::Matched {
                Attempt::Committed
            } else {
                qualifiers
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

struct QualifierAttempt {
    attempt: Attempt,
    // Set only by recognition of the actual qualifier, never by recovery alone.
    kind: Option<QualifierKind>,
}

fn qualifier(parser: &mut Parser<'_>) -> QualifierAttempt {
    let mut kind = None;
    let attempt = combinator::transactional(parser, rules::COMPREHENSION_QUALIFIER, |parser| {
        let node = parser.start();
        let selected = generator(parser);
        let outcome = if selected.attempt != Attempt::NoMatch {
            kind = selected.kind;
            selected.attempt
        } else {
            let (definition, has_operator) = variables::variable_definition(parser);
            if definition != Attempt::NoMatch {
                if has_operator {
                    kind = Some(QualifierKind::Let);
                }
                definition
            } else {
                let filter = expressions::parse_expression(parser);
                if filter != Attempt::NoMatch {
                    kind = Some(QualifierKind::Filter);
                }
                filter
            }
        };
        if outcome == Attempt::NoMatch {
            node.abandon(parser);
        } else {
            node.complete(parser, SyntaxKind::ComprehensionQualifier);
        }
        outcome
    });
    QualifierAttempt { attempt, kind }
}

fn generator(parser: &mut Parser<'_>) -> QualifierAttempt {
    let mut kind = None;
    let attempt = combinator::transactional(parser, rules::GENERATOR, |parser| {
        let node = parser.start();
        let mut committed = false;
        match patterns::pattern_with_facts(parser) {
            FactAttempt::Matched(_) => {}
            FactAttempt::NoMatch => {
                node.abandon(parser);
                return Attempt::NoMatch;
            }
            FactAttempt::Committed => {
                if parser.is_halted() {
                    node.complete(parser, SyntaxKind::Generator);
                    return Attempt::Committed;
                }
                committed = true;
            }
        }
        if !base::parse_rule(parser, rules::SPACE_TAB0)
            || (!base::parse_rule(parser, rules::GENERATOR_ARROW)
                && !base::parse_rule(parser, rules::GENERATOR_ARROW_U))
            || !base::parse_rule(parser, rules::SPACE_TAB0)
        {
            if parser.is_halted() {
                node.complete(parser, SyntaxKind::Generator);
                return Attempt::Committed;
            }
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        kind = Some(QualifierKind::Generator);
        match expressions::parse_expression(parser) {
            Attempt::Matched => {}
            Attempt::Committed => committed = true,
            Attempt::NoMatch => {
                recover_required_production(
                    parser,
                    rules::GENERATOR,
                    "syntax/missing-generator-source",
                    "missing source expression after generator arrow",
                    "expression",
                );
                committed = true;
            }
        }
        node.complete(parser, SyntaxKind::Generator);
        if committed {
            Attempt::Committed
        } else {
            Attempt::Matched
        }
    });
    QualifierAttempt { attempt, kind }
}
