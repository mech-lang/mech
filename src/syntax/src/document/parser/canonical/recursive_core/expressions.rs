use crate::document::SyntaxKind;

use super::super::super::Parser;
use super::super::super::marker::Marker;
use super::super::super::rule::rules;
use super::super::{base, combinator, operators};
use super::{Attempt, ExpressionForm, FactAttempt, fsm, precedence, structures};

pub(super) fn parse_expression(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::EXPRESSION, |parser| {
        let node = parser.start();
        match expression_body(parser) {
            FactAttempt::NoMatch => {
                node.abandon(parser);
                Attempt::NoMatch
            }
            FactAttempt::Committed => {
                node.complete(parser, SyntaxKind::Expression);
                Attempt::Committed
            }
            FactAttempt::Matched(_) => {
                node.complete(parser, SyntaxKind::Expression);
                Attempt::Matched
            }
        }
    })
}

pub(super) fn parse_formula(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::FORMULA, precedence::parse_l1)
}

pub(super) fn expression_body(parser: &mut Parser<'_>) -> FactAttempt<ExpressionForm> {
    if parser.cursor().starts_with("#") {
        return match fsm::parse_fsm_pipe(parser) {
            Attempt::Matched => FactAttempt::Matched(ExpressionForm::FsmPipe),
            Attempt::Committed => FactAttempt::Committed,
            Attempt::NoMatch => FactAttempt::NoMatch,
        };
    }

    let range = parser.start();
    if parser.cursor().starts_with("{") || parser.cursor().starts_with("[") {
        let seed = precedence::FormulaSeed::start(parser);
        let selected = if parser.cursor().starts_with("{") {
            structures::brace_expression(parser)
        } else {
            structures::bracket_expression(parser)
        };
        match selected {
            FactAttempt::Matched(
                form @ (ExpressionForm::SetComprehension | ExpressionForm::MatrixComprehension),
            ) => {
                seed.abandon(parser);
                range.abandon(parser);
                return FactAttempt::Matched(form);
            }
            FactAttempt::Matched(ExpressionForm::Formula) => {
                match seed.continue_from_factor(parser) {
                    Attempt::Matched => {}
                    Attempt::NoMatch => {
                        range.abandon(parser);
                        return FactAttempt::NoMatch;
                    }
                    Attempt::Committed => {
                        finish_provisional_formula_marker(parser, range);
                        return FactAttempt::Committed;
                    }
                }
            }
            FactAttempt::Committed => {
                seed.commit(parser);
                finish_provisional_formula_marker(parser, range);
                return FactAttempt::Committed;
            }
            FactAttempt::NoMatch => {
                seed.abandon(parser);
                range.abandon(parser);
                return FactAttempt::NoMatch;
            }
            FactAttempt::Matched(_) => unreachable!("delimited expression selection is closed"),
        }
    } else {
        match parse_formula(parser) {
            Attempt::NoMatch => {
                range.abandon(parser);
                return FactAttempt::NoMatch;
            }
            Attempt::Committed => {
                finish_provisional_formula_marker(parser, range);
                return FactAttempt::Committed;
            }
            Attempt::Matched => {}
        }
    }

    finish_formula_expression(parser, range)
}

fn finish_formula_expression(
    parser: &mut Parser<'_>,
    range: Marker,
) -> FactAttempt<ExpressionForm> {
    match operators::parse_range_operator(parser) {
        Attempt::Matched => {}
        Attempt::Committed => {
            finish_provisional_formula_marker(parser, range);
            return FactAttempt::Committed;
        }
        Attempt::NoMatch => {
            range.abandon(parser);
            return finish_match_suffix(parser);
        }
    }
    match parse_formula(parser) {
        Attempt::Matched => {}
        Attempt::Committed => {
            range.complete(parser, SyntaxKind::RangeExpression);
            return FactAttempt::Committed;
        }
        Attempt::NoMatch => {
            range.abandon(parser);
            return FactAttempt::NoMatch;
        }
    }
    let second = parser.checkpoint();
    match operators::parse_range_operator(parser) {
        Attempt::Matched => match parse_formula(parser) {
            Attempt::Matched => {}
            Attempt::Committed => {
                range.complete(parser, SyntaxKind::RangeExpression);
                return FactAttempt::Committed;
            }
            Attempt::NoMatch => parser.rewind(second),
        },
        Attempt::Committed => {
            range.complete(parser, SyntaxKind::RangeExpression);
            return FactAttempt::Committed;
        }
        Attempt::NoMatch => {}
    }
    range.complete(parser, SyntaxKind::RangeExpression);
    FactAttempt::Matched(ExpressionForm::Range)
}

fn finish_match_suffix(parser: &mut Parser<'_>) -> FactAttempt<ExpressionForm> {
    let match_suffix = parser.checkpoint();
    if !base::parse_rule(parser, rules::WHITESPACE0) || !base::parse_rule(parser, rules::QUESTION) {
        parser.rewind(match_suffix);
        return FactAttempt::Matched(ExpressionForm::Formula);
    }
    if !base::parse_rule(parser, rules::WHITESPACE0) {
        return FactAttempt::NoMatch;
    }
    match precedence::parse_match_arm(parser) {
        Attempt::Matched => {}
        Attempt::Committed => return FactAttempt::Committed,
        Attempt::NoMatch => return FactAttempt::NoMatch,
    }
    loop {
        let before = parser.offset();
        match precedence::parse_match_arm(parser) {
            Attempt::Matched if parser.offset() > before => {}
            Attempt::Matched | Attempt::NoMatch => break,
            Attempt::Committed => return FactAttempt::Committed,
        }
    }
    let _ = base::parse_rule(parser, rules::PERIOD);
    FactAttempt::Matched(ExpressionForm::Match)
}

pub(super) fn formula_or_range(parser: &mut Parser<'_>, require_range: bool) -> Attempt {
    let checkpoint = parser.checkpoint();
    let range = parser.start();
    let formula = parse_formula(parser);
    match formula {
        Attempt::Matched => {}
        Attempt::NoMatch => {
            parser.rewind(checkpoint);
            return Attempt::NoMatch;
        }
        Attempt::Committed => {
            finish_provisional_formula_marker(parser, range);
            return Attempt::Committed;
        }
    }
    match operators::parse_range_operator(parser) {
        Attempt::NoMatch => {
            range.abandon(parser);
            if require_range {
                parser.rewind(checkpoint);
                Attempt::NoMatch
            } else {
                Attempt::Matched
            }
        }
        Attempt::Committed => {
            finish_provisional_formula_marker(parser, range);
            Attempt::Committed
        }
        Attempt::Matched => {
            match parse_formula(parser) {
                Attempt::Matched => {}
                Attempt::NoMatch => {
                    parser.rewind(checkpoint);
                    return Attempt::NoMatch;
                }
                Attempt::Committed => {
                    range.complete(parser, SyntaxKind::RangeExpression);
                    return Attempt::Committed;
                }
            }
            let optional = parser.checkpoint();
            match operators::parse_range_operator(parser) {
                Attempt::Matched => match parse_formula(parser) {
                    Attempt::Matched => {}
                    Attempt::NoMatch => parser.rewind(optional),
                    Attempt::Committed => {
                        range.complete(parser, SyntaxKind::RangeExpression);
                        return Attempt::Committed;
                    }
                },
                Attempt::Committed => {
                    range.complete(parser, SyntaxKind::RangeExpression);
                    return Attempt::Committed;
                }
                Attempt::NoMatch => {}
            }
            range.complete(parser, SyntaxKind::RangeExpression);
            Attempt::Matched
        }
    }
}

fn finish_provisional_formula_marker(parser: &mut Parser<'_>, marker: Marker) {
    if parser.is_halted() {
        marker.complete(parser, SyntaxKind::Expression);
    } else {
        marker.abandon(parser);
    }
}
