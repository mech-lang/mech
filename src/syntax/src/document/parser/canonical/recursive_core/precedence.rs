use crate::document::{RuleId, SyntaxKind};

use super::super::super::marker::Marker;
use super::super::super::rule::rules;
use super::super::super::{Parser, ParserCheckpoint};
use super::super::{base, combinator, control_operators, operators};
use super::{
    Attempt, calls, child_result, expressions, literals, recover_closer,
    recover_required_production, recover_required_token, structures, subscripts, variables,
};

pub(super) struct FormulaSeed {
    checkpoint: ParserCheckpoint,
    l1: Marker,
    l2: Marker,
    l3: Marker,
    l4: Marker,
    l5: Marker,
    l6: Marker,
    l7: Marker,
    factor: Marker,
}

impl FormulaSeed {
    pub(super) fn start(parser: &mut Parser<'_>) -> Self {
        let checkpoint = parser.checkpoint();
        Self {
            checkpoint,
            l1: parser.start(),
            l2: parser.start(),
            l3: parser.start(),
            l4: parser.start(),
            l5: parser.start(),
            l6: parser.start(),
            l7: parser.start(),
            factor: parser.start(),
        }
    }

    pub(super) fn abandon(self, parser: &mut Parser<'_>) {
        self.factor.abandon(parser);
        self.l7.abandon(parser);
        self.l6.abandon(parser);
        self.l5.abandon(parser);
        self.l4.abandon(parser);
        self.l3.abandon(parser);
        self.l2.abandon(parser);
        self.l1.abandon(parser);
    }

    pub(super) fn commit(self, parser: &mut Parser<'_>) -> Attempt {
        self.factor.complete(parser, SyntaxKind::Factor);
        complete_seeded_outer(self, parser, 7);
        Attempt::Committed
    }

    pub(super) fn continue_from_factor(self, parser: &mut Parser<'_>) -> Attempt {
        if operators::parse_transpose(parser) == Attempt::Committed {
            return self.commit(parser);
        }
        self.factor.complete(parser, SyntaxKind::Factor);

        let levels = [
            (
                self.l7,
                SyntaxKind::SetExpression,
                parse_factor as fn(&mut Parser<'_>) -> Attempt,
                operators::parse_set_operator as fn(&mut Parser<'_>) -> Attempt,
            ),
            (
                self.l6,
                SyntaxKind::TableExpression,
                parse_l7,
                operators::parse_table_operator,
            ),
            (
                self.l5,
                SyntaxKind::PowerExpression,
                parse_l6,
                operators::parse_power_operator,
            ),
            (
                self.l4,
                SyntaxKind::MultiplicativeExpression,
                parse_l5,
                parse_l4_operator,
            ),
            (
                self.l3,
                SyntaxKind::AdditiveExpression,
                parse_l4,
                operators::parse_add_sub_operator,
            ),
            (
                self.l2,
                SyntaxKind::ComparisonExpression,
                parse_l3,
                operators::parse_comparison_operator,
            ),
            (
                self.l1,
                SyntaxKind::LogicExpression,
                parse_l2,
                operators::parse_logic_operator,
            ),
        ];

        for (index, (marker, kind, operand, operator)) in levels.into_iter().enumerate() {
            match seeded_precedence_level(parser, marker, kind, operand, operator) {
                Attempt::Matched => {}
                Attempt::NoMatch => {
                    parser.rewind(self.checkpoint);
                    return Attempt::NoMatch;
                }
                Attempt::Committed => {
                    complete_seeded_outer(self, parser, 6_usize.saturating_sub(index));
                    return Attempt::Committed;
                }
            }
        }
        Attempt::Matched
    }
}

pub(super) fn parse_l1(parser: &mut Parser<'_>) -> Attempt {
    precedence_level(
        parser,
        rules::L1,
        SyntaxKind::LogicExpression,
        parse_l2,
        operators::parse_logic_operator,
    )
}

pub(super) fn parse_l2(parser: &mut Parser<'_>) -> Attempt {
    precedence_level(
        parser,
        rules::L2,
        SyntaxKind::ComparisonExpression,
        parse_l3,
        operators::parse_comparison_operator,
    )
}

pub(super) fn parse_l3(parser: &mut Parser<'_>) -> Attempt {
    precedence_level(
        parser,
        rules::L3,
        SyntaxKind::AdditiveExpression,
        parse_l4,
        operators::parse_add_sub_operator,
    )
}

pub(super) fn parse_l4(parser: &mut Parser<'_>) -> Attempt {
    precedence_level(
        parser,
        rules::L4,
        SyntaxKind::MultiplicativeExpression,
        parse_l5,
        parse_l4_operator,
    )
}

pub(super) fn parse_l5(parser: &mut Parser<'_>) -> Attempt {
    precedence_level(
        parser,
        rules::L5,
        SyntaxKind::PowerExpression,
        parse_l6,
        operators::parse_power_operator,
    )
}

pub(super) fn parse_l6(parser: &mut Parser<'_>) -> Attempt {
    precedence_level(
        parser,
        rules::L6,
        SyntaxKind::TableExpression,
        parse_l7,
        operators::parse_table_operator,
    )
}

pub(super) fn parse_l7(parser: &mut Parser<'_>) -> Attempt {
    precedence_level(
        parser,
        rules::L7,
        SyntaxKind::SetExpression,
        parse_factor,
        operators::parse_set_operator,
    )
}

pub(super) fn parse_factor(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::FACTOR, |parser| {
        let node = parser.start();
        let selected = factor_body(parser);
        if let Some(result) = child_result(parser, node, SyntaxKind::Factor, selected) {
            return result;
        }
        if operators::parse_transpose(parser) == Attempt::Committed {
            node.complete(parser, SyntaxKind::Factor);
            return Attempt::Committed;
        }
        node.complete(parser, SyntaxKind::Factor);
        Attempt::Matched
    })
}

pub(super) fn parse_parenthetical_term(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::PARENTHETICAL_TERM, |parser| {
        let node = parser.start();
        if !base::parse_rule(parser, rules::LEFT_PARENTHESIS) {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let Some(interior) = parser.with_nesting(|parser| {
            if !base::parse_rule(parser, rules::SPACE_TAB0) {
                return Attempt::NoMatch;
            }
            let mut committed = false;
            match expressions::parse_formula(parser) {
                Attempt::Matched => {}
                Attempt::NoMatch => {
                    recover_required_production(
                        parser,
                        rules::PARENTHETICAL_TERM,
                        "syntax/missing-parenthetical-expression",
                        "missing expression after opening parenthesis",
                        "formula",
                    );
                    committed = true;
                }
                Attempt::Committed => committed = true,
            }
            let _ = base::parse_rule(parser, rules::SPACE_TAB0);
            if base::parse_rule(parser, rules::RIGHT_PARENTHESIS) {
                if committed {
                    Attempt::Committed
                } else {
                    Attempt::Matched
                }
            } else {
                recover_closer(
                    parser,
                    rules::PARENTHETICAL_TERM,
                    rules::RIGHT_PARENTHESIS,
                    SyntaxKind::RightParen,
                    ')',
                    ")",
                )
            }
        }) else {
            super::nesting_limit(parser);
            node.complete(parser, SyntaxKind::ParentheticalExpression);
            return Attempt::Committed;
        };
        finish(node, parser, SyntaxKind::ParentheticalExpression, interior)
    })
}

pub(super) fn parse_negate_factor(parser: &mut Parser<'_>) -> Attempt {
    unary_factor(
        parser,
        rules::NEGATE_FACTOR,
        rules::DASH,
        SyntaxKind::NegateFactor,
    )
}

pub(super) fn parse_not_factor(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::NOT_FACTOR, |parser| {
        let node = parser.start();
        if operators::parse_not(parser) != Attempt::Matched {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let child = parse_factor(parser);
        match child {
            Attempt::Matched => {}
            Attempt::Committed => {
                node.complete(parser, SyntaxKind::NotFactor);
                return Attempt::Committed;
            }
            Attempt::NoMatch => {
                recover_required_production(
                    parser,
                    rules::NOT_FACTOR,
                    "syntax/missing-unary-operand",
                    "missing operand after unary operator",
                    "factor",
                );
                node.complete(parser, SyntaxKind::NotFactor);
                return Attempt::Committed;
            }
        }
        node.complete(parser, SyntaxKind::NotFactor);
        Attempt::Matched
    })
}

pub(super) fn parse_range_expression(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::RANGE_EXPRESSION, |parser| {
        expressions::formula_or_range(parser, true)
    })
}

pub(super) fn parse_match_arm(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::MATCH_ARM, |parser| {
        let node = parser.start();
        if control_operators::parse_guard_operator(parser) != Attempt::Matched {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let pattern = super::patterns::parse_pattern(parser);
        let mut committed = match pattern {
            Attempt::Matched => false,
            Attempt::Committed => true,
            Attempt::NoMatch => {
                recover_required_production(
                    parser,
                    rules::MATCH_ARM,
                    "syntax/missing-match-arm-pattern",
                    "missing pattern after match arm guard",
                    "pattern",
                );
                true
            }
        };
        let guard = parser.checkpoint();
        if base::parse_rule(parser, rules::LIST_SEPARATOR)
            && base::parse_rule(parser, rules::WHITESPACE0)
        {
            match expressions::parse_expression(parser) {
                Attempt::Matched => {}
                Attempt::NoMatch => parser.rewind(guard),
                Attempt::Committed => committed = true,
            }
        }
        if !base::parse_rule(parser, rules::OUTPUT_OPERATOR) {
            recover_required_token(
                parser,
                rules::MATCH_ARM,
                "syntax/missing-match-arm-output-operator",
                "missing output operator after match arm pattern",
                SyntaxKind::OutputOperator,
                "=>",
            );
            node.complete(parser, SyntaxKind::MatchArm);
            return Attempt::Committed;
        }
        let child = expressions::parse_expression(parser);
        match child {
            Attempt::Matched => {}
            Attempt::Committed => committed = true,
            Attempt::NoMatch => {
                recover_required_production(
                    parser,
                    rules::MATCH_ARM,
                    "syntax/missing-match-arm-value",
                    "missing expression after match arm output operator",
                    "expression",
                );
                node.complete(parser, SyntaxKind::MatchArm);
                return Attempt::Committed;
            }
        }
        let suffix = parser.checkpoint();
        if !base::parse_rule(parser, rules::WHITESPACE1)
            && control_operators::parse_statement_separator(parser) == Attempt::NoMatch
        {
            parser.rewind(suffix);
        }
        node.complete(parser, SyntaxKind::MatchArm);
        if committed {
            Attempt::Committed
        } else {
            Attempt::Matched
        }
    })
}

fn factor_body(parser: &mut Parser<'_>) -> Attempt {
    if parser.cursor().starts_with("(") {
        return structures::parenthesis_factor(parser);
    }
    let negate = parse_negate_factor(parser);
    if negate != Attempt::NoMatch {
        return negate;
    }
    let not = parse_not_factor(parser);
    if not != Attempt::NoMatch {
        return not;
    }
    if parser.cursor().starts_with("[") {
        return structures::bracket_factor(parser);
    }
    if parser.cursor().starts_with("{") {
        return structures::brace_factor(parser);
    }
    if parser.cursor().starts_with(":") {
        return structures::colon_factor(parser);
    }

    let table = structures::structure_non_delimited(parser);
    if table != Attempt::NoMatch {
        return table;
    }

    let literal = literals::parse_literal(parser);
    if literal != Attempt::NoMatch {
        return literal;
    }

    let stem = parser.checkpoint();
    let local = base::parse_rule(parser, rules::IDENTIFIER);
    let context = if local {
        false
    } else {
        super::super::paths::parse_prefixed_context_path(parser).accepted()
    };
    if local || context {
        let call = local && parser.cursor().starts_with("(");
        let slice = (parser.cursor().starts_with(".") && !parser.cursor().starts_with(".."))
            || parser.cursor().starts_with("[")
            || parser.cursor().starts_with("{");
        parser.rewind(stem);
        if call {
            return calls::parse_function_call(parser);
        }
        if slice {
            return subscripts::parse_slice(parser);
        }
        return variables::parse_var(parser);
    }
    parser.rewind(stem);
    Attempt::NoMatch
}

fn complete_seeded_outer(seed: FormulaSeed, parser: &mut Parser<'_>, count: usize) {
    if count >= 7 {
        seed.l7.complete(parser, SyntaxKind::SetExpression);
    }
    if count >= 6 {
        seed.l6.complete(parser, SyntaxKind::TableExpression);
    }
    if count >= 5 {
        seed.l5.complete(parser, SyntaxKind::PowerExpression);
    }
    if count >= 4 {
        seed.l4
            .complete(parser, SyntaxKind::MultiplicativeExpression);
    }
    if count >= 3 {
        seed.l3.complete(parser, SyntaxKind::AdditiveExpression);
    }
    if count >= 2 {
        seed.l2.complete(parser, SyntaxKind::ComparisonExpression);
    }
    if count >= 1 {
        seed.l1.complete(parser, SyntaxKind::LogicExpression);
    }
}

fn seeded_precedence_level(
    parser: &mut Parser<'_>,
    marker: Marker,
    kind: SyntaxKind,
    operand: fn(&mut Parser<'_>) -> Attempt,
    operator: fn(&mut Parser<'_>) -> Attempt,
) -> Attempt {
    let mut pairs = 0_u32;
    loop {
        let before = parser.offset();
        match operator(parser) {
            Attempt::NoMatch if parser.is_halted() => {
                marker.complete(parser, kind);
                return Attempt::Committed;
            }
            Attempt::NoMatch => break,
            Attempt::Committed => {
                marker.complete(parser, kind);
                return Attempt::Committed;
            }
            Attempt::Matched => {}
        }
        match operand(parser) {
            Attempt::Matched if parser.offset() > before => pairs += 1,
            Attempt::NoMatch if parser.is_halted() => {
                marker.complete(parser, kind);
                return Attempt::Committed;
            }
            Attempt::Matched => return Attempt::NoMatch,
            Attempt::NoMatch => {
                let target = parser.current_rule().unwrap_or(rules::EXPRESSION);
                recover_required_production(
                    parser,
                    target,
                    "syntax/missing-operator-operand",
                    "missing expression after operator",
                    "expression",
                );
                marker.complete(parser, kind);
                return Attempt::Committed;
            }
            Attempt::Committed => {
                marker.complete(parser, kind);
                return Attempt::Committed;
            }
        }
    }
    if parser.is_halted() {
        marker.complete(parser, kind);
        Attempt::Committed
    } else {
        if pairs == 0 {
            marker.abandon(parser);
        } else {
            marker.complete(parser, kind);
        }
        Attempt::Matched
    }
}

fn precedence_level(
    parser: &mut Parser<'_>,
    rule: RuleId,
    kind: SyntaxKind,
    operand: fn(&mut Parser<'_>) -> Attempt,
    operator: fn(&mut Parser<'_>) -> Attempt,
) -> Attempt {
    combinator::transactional(parser, rule, |parser| {
        let node = parser.start();
        match operand(parser) {
            Attempt::Matched => {}
            Attempt::NoMatch if parser.is_halted() => {
                node.complete(parser, kind);
                return Attempt::Committed;
            }
            Attempt::NoMatch => {
                node.abandon(parser);
                return Attempt::NoMatch;
            }
            Attempt::Committed => {
                node.complete(parser, kind);
                return Attempt::Committed;
            }
        }
        let mut pairs = 0_u32;
        loop {
            let before = parser.offset();
            match operator(parser) {
                Attempt::NoMatch if parser.is_halted() => {
                    node.complete(parser, kind);
                    return Attempt::Committed;
                }
                Attempt::NoMatch => break,
                Attempt::Committed => {
                    node.complete(parser, kind);
                    return Attempt::Committed;
                }
                Attempt::Matched => {}
            }
            match operand(parser) {
                Attempt::Matched if parser.offset() > before => pairs += 1,
                Attempt::NoMatch if parser.is_halted() => {
                    node.complete(parser, kind);
                    return Attempt::Committed;
                }
                Attempt::Matched => return Attempt::NoMatch,
                Attempt::NoMatch => {
                    recover_required_production(
                        parser,
                        rule,
                        "syntax/missing-operator-operand",
                        "missing expression after operator",
                        "expression",
                    );
                    node.complete(parser, kind);
                    return Attempt::Committed;
                }
                Attempt::Committed => {
                    node.complete(parser, kind);
                    return Attempt::Committed;
                }
            }
        }
        if parser.is_halted() {
            node.complete(parser, kind);
            return Attempt::Committed;
        } else if pairs == 0 {
            node.abandon(parser);
        } else {
            node.complete(parser, kind);
        }
        Attempt::Matched
    })
}

fn parse_l4_operator(parser: &mut Parser<'_>) -> Attempt {
    let mul = operators::parse_mul_div_operator(parser);
    if mul == Attempt::NoMatch {
        operators::parse_matrix_operator(parser)
    } else {
        mul
    }
}

fn unary_factor(
    parser: &mut Parser<'_>,
    rule: RuleId,
    operator: RuleId,
    kind: SyntaxKind,
) -> Attempt {
    combinator::transactional(parser, rule, |parser| {
        let node = parser.start();
        if !base::parse_rule(parser, operator) {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let child = parse_factor(parser);
        match child {
            Attempt::Matched => {}
            Attempt::Committed => {
                node.complete(parser, kind);
                return Attempt::Committed;
            }
            Attempt::NoMatch => {
                recover_required_production(
                    parser,
                    rule,
                    "syntax/missing-unary-operand",
                    "missing operand after unary operator",
                    "factor",
                );
                node.complete(parser, kind);
                return Attempt::Committed;
            }
        }
        node.complete(parser, kind);
        Attempt::Matched
    })
}

fn finish(
    node: super::super::super::marker::Marker,
    parser: &mut Parser<'_>,
    kind: SyntaxKind,
    result: Attempt,
) -> Attempt {
    match result {
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
}
