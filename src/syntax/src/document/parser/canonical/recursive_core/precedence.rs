use crate::document::{RuleId, SyntaxKind};

use super::super::super::marker::Marker;
use super::super::super::rule::rules;
use super::super::super::{Parser, ParserCheckpoint};
use super::super::{base, combinator, control_operators, operators};
use super::{
    Attempt, calls, expressions, literals, recover_closer, recover_required_production, structures,
    subscripts, variables,
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

    pub(super) fn continue_from_factor(
        self,
        parser: &mut Parser<'_>,
        mut committed: bool,
    ) -> Attempt {
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
            match finish_precedence_level(parser, marker, kind, operand, operator, committed) {
                Attempt::Matched => {}
                Attempt::NoMatch => {
                    parser.rewind(self.checkpoint);
                    return Attempt::NoMatch;
                }
                Attempt::Committed => {
                    if parser.is_halted() {
                        complete_seeded_outer(self, parser, 6_usize.saturating_sub(index));
                        return Attempt::Committed;
                    }
                    committed = true;
                }
            }
        }
        if committed {
            Attempt::Committed
        } else {
            Attempt::Matched
        }
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
        let committed = match factor_body(parser) {
            Attempt::Matched => false,
            Attempt::Committed => true,
            Attempt::NoMatch => {
                node.abandon(parser);
                return Attempt::NoMatch;
            }
        };
        let suffix = operators::parse_transpose(parser);
        node.complete(parser, SyntaxKind::Factor);
        if committed || suffix == Attempt::Committed {
            Attempt::Committed
        } else {
            Attempt::Matched
        }
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
    if parser.cursor().starts_with("--") {
        return Attempt::NoMatch;
    }
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
        let child = parser
            .with_nesting(parse_factor)
            .unwrap_or_else(|| super::nesting_limit(parser));
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
        let result = parser
            .with_nesting(match_arm_body)
            .unwrap_or_else(|| super::nesting_limit(parser));
        finish(node, parser, SyntaxKind::MatchArm, result)
    })
}

fn match_arm_body(parser: &mut Parser<'_>) -> Attempt {
    let pattern = super::patterns::parse_pattern(parser);
    let mut committed = match pattern {
        Attempt::Matched => false,
        Attempt::Committed => true,
        Attempt::NoMatch => {
            super::recover_required_production_with_prefixes(
                parser,
                rules::MATCH_ARM,
                "syntax/missing-match-arm-pattern",
                "missing pattern after match arm guard",
                "pattern",
                &["=>", "⇒"],
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
        super::recover_required_token_with_prefixes(
            parser,
            rules::MATCH_ARM,
            "syntax/missing-match-arm-output-operator",
            "missing output operator after match arm pattern",
            SyntaxKind::OutputOperator,
            "=>",
            &["=>", "⇒"],
        );
        committed = true;
        if !base::parse_rule(parser, rules::OUTPUT_OPERATOR) {
            return Attempt::Committed;
        }
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
            return Attempt::Committed;
        }
    }
    let suffix = parser.checkpoint();
    if !base::parse_rule(parser, rules::WHITESPACE1)
        && control_operators::parse_statement_separator(parser) == Attempt::NoMatch
    {
        parser.rewind(suffix);
    }
    if committed {
        Attempt::Committed
    } else {
        Attempt::Matched
    }
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
        return variables::factor_variable(parser);
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

fn precedence_level(
    parser: &mut Parser<'_>,
    rule: RuleId,
    kind: SyntaxKind,
    operand: fn(&mut Parser<'_>) -> Attempt,
    operator: fn(&mut Parser<'_>) -> Attempt,
) -> Attempt {
    combinator::transactional(parser, rule, |parser| {
        let node = parser.start();
        let committed = match operand(parser) {
            Attempt::Matched => false,
            Attempt::Committed => true,
            Attempt::NoMatch => {
                node.abandon(parser);
                return Attempt::NoMatch;
            }
        };
        finish_precedence_level(parser, node, kind, operand, operator, committed)
    })
}

fn finish_precedence_level(
    parser: &mut Parser<'_>,
    marker: Marker,
    kind: SyntaxKind,
    operand: fn(&mut Parser<'_>) -> Attempt,
    operator: fn(&mut Parser<'_>) -> Attempt,
    mut committed: bool,
) -> Attempt {
    let mut pairs = 0_u32;
    while !parser.is_halted() {
        let before = parser.offset();
        match operator(parser) {
            Attempt::NoMatch => break,
            Attempt::Committed => {
                committed = true;
                break;
            }
            Attempt::Matched => {}
        }
        pairs += 1;
        match operand(parser) {
            Attempt::Matched if parser.offset() > before => {}
            Attempt::Matched => return Attempt::NoMatch,
            Attempt::Committed => committed = true,
            Attempt::NoMatch => {
                committed = true;
                // Reuse the selected operator production at every recovery restart,
                // including after skipped invalid source. Its transaction leaves the
                // operator for the next pair and charges the shared parser budget.
                let target = parser.current_rule().unwrap_or(rules::EXPRESSION);
                let mut rejected_trivia_end = parser.offset();
                let mut previous_probe = None;
                super::recover_required_production_before(
                    parser,
                    target,
                    "syntax/missing-operator-operand",
                    "missing expression after operator",
                    "expression",
                    |parser| {
                        let offset = parser.offset();
                        if let Some((previous, found)) = previous_probe
                            && previous == offset
                        {
                            return found;
                        }
                        if offset < rejected_trivia_end {
                            return false;
                        }
                        let next = parser.checkpoint();
                        let at_operator = operator(parser).accepted();
                        parser.rewind(next);
                        if !at_operator && !parser.is_halted() {
                            // Use the actual SPACE_TAB grammar to bound a rejected
                            // leading trivia run, including NBSP and thin space. Later
                            // probes skip only that exact extent, never source beyond it.
                            let _ = base::parse_rule(parser, rules::SPACE_TAB0);
                            rejected_trivia_end = parser.offset();
                            parser.rewind(next);
                        }
                        previous_probe = Some((offset, at_operator));
                        at_operator
                    },
                );
            }
        }
        if parser.offset() <= before {
            break;
        }
    }
    if committed || parser.is_halted() {
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
        let child = parser
            .with_nesting(parse_factor)
            .unwrap_or_else(|| super::nesting_limit(parser));
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
