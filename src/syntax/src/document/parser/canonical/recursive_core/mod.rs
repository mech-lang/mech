//! Clean canonical recognizer for the frozen Phase 2I recursive core.

mod calls;
mod comprehensions;
mod expressions;
mod fsm;
mod kinds;
mod literals;
mod patterns;
mod precedence;
mod structures;
mod subscripts;
mod variables;

use crate::document::{RuleId, SyntaxKind};

use super::super::Parser;
use super::super::marker::Marker;
use super::super::recovery;
use super::super::rule::rules;
use super::combinator::Attempt;

pub(crate) const PHASE_2I_RULES: &[RuleId; 80] = &[
    rules::ARGUMENT_LIST,
    rules::BINDING,
    rules::BRACE_SUBSCRIPT,
    rules::BRACKET_SUBSCRIPT,
    rules::CALL_ARG,
    rules::CALL_ARG_WITH_BINDING,
    rules::COMPREHENSION_QUALIFIER,
    rules::EXPRESSION,
    rules::FACTOR,
    rules::FANCY_TABLE,
    rules::FANCY_TABLE_HEADER,
    rules::FIELD,
    rules::FORMULA,
    rules::FORMULA_SUBSCRIPT,
    rules::FSM_ARGS,
    rules::FSM_ASYNC_TRANSITION,
    rules::FSM_INSTANCE,
    rules::FSM_OUTPUT,
    rules::FSM_PIPE,
    rules::FSM_STATE_TRANSITION,
    rules::FSM_VALUE,
    rules::FUNCTION_CALL,
    rules::GENERATOR,
    rules::HEADER_FIELD,
    rules::INLINE_TABLE,
    rules::INLINE_TABLE_HEADER,
    rules::INLINE_TABLE_ROW,
    rules::KIND,
    rules::KIND_ANNOTATION,
    rules::KIND_KIND,
    rules::KIND_MAP,
    rules::KIND_MATRIX,
    rules::KIND_RECORD,
    rules::KIND_SCALAR,
    rules::KIND_SET,
    rules::KIND_TABLE,
    rules::KIND_TUPLE,
    rules::KIND_WITH_OPTION,
    rules::L1,
    rules::L2,
    rules::L3,
    rules::L4,
    rules::L5,
    rules::L6,
    rules::L7,
    rules::LITERAL,
    rules::MAP,
    rules::MAPPING,
    rules::MATCH_ARM,
    rules::MATRIX,
    rules::MATRIX_COLUMN,
    rules::MATRIX_COMPREHENSION,
    rules::MATRIX_ROW,
    rules::NEGATE_FACTOR,
    rules::NOT_FACTOR,
    rules::PARENTHETICAL_TERM,
    rules::PATTERN,
    rules::PATTERN_ARRAY,
    rules::PATTERN_ARRAY_ITEM,
    rules::PATTERN_ARRAY_TOKEN,
    rules::PATTERN_ATOM_STRUCT,
    rules::PATTERN_TUPLE,
    rules::PATTERN_TUPLE_STRUCT,
    rules::RANGE_EXPRESSION,
    rules::RANGE_SUBSCRIPT,
    rules::RECORD,
    rules::REGULAR_TABLE,
    rules::SET,
    rules::SET_COMPREHENSION,
    rules::SLICE,
    rules::STRUCTURE,
    rules::SUBSCRIPT,
    rules::TABLE,
    rules::TABLE_HEADER,
    rules::TABLE_ROW,
    rules::TABLE_ROW2,
    rules::TUPLE,
    rules::TUPLE_STRUCT,
    rules::VAR,
    rules::VARIABLE_DEFINE,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ExpressionForm {
    Formula,
    Range,
    SetComprehension,
    MatrixComprehension,
    FsmPipe,
    Match,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct PatternFacts {
    pub(super) contains_wildcard: bool,
    pub(super) contains_array_spread_or_rest: bool,
}

impl PatternFacts {
    pub(super) fn merge(&mut self, other: Self) {
        self.contains_wildcard |= other.contains_wildcard;
        self.contains_array_spread_or_rest |= other.contains_array_spread_or_rest;
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum QualifierKind {
    Generator,
    Let,
    Filter,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BracketForm {
    Matrix,
    Comprehension,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum FactAttempt<T> {
    NoMatch,
    Matched(T),
    Committed,
}

impl<T> FactAttempt<T> {
    pub(super) fn attempt(self) -> Attempt {
        match self {
            Self::NoMatch => Attempt::NoMatch,
            Self::Matched(_) => Attempt::Matched,
            Self::Committed => Attempt::Committed,
        }
    }
}

pub(super) fn child_result(
    parser: &mut Parser<'_>,
    marker: Marker,
    kind: SyntaxKind,
    child: Attempt,
) -> Option<Attempt> {
    match child {
        Attempt::Matched => None,
        Attempt::NoMatch => {
            marker.abandon(parser);
            Some(Attempt::NoMatch)
        }
        Attempt::Committed => {
            marker.complete(parser, kind);
            Some(Attempt::Committed)
        }
    }
}

pub(super) fn nesting_limit(parser: &mut Parser<'_>) -> Attempt {
    recovery::nesting_limit(parser);
    Attempt::Committed
}

pub(super) fn transactional_fact<T>(
    parser: &mut Parser<'_>,
    rule: RuleId,
    parse: impl FnOnce(&mut Parser<'_>) -> FactAttempt<T>,
) -> FactAttempt<T> {
    let checkpoint = parser.checkpoint();
    let result = parser.with_canonical_rule(rule, parse);
    if matches!(result, FactAttempt::NoMatch) {
        parser.rewind(checkpoint);
    }
    result
}

pub(crate) fn supports(rule: RuleId) -> bool {
    PHASE_2I_RULES.contains(&rule)
}

pub(crate) fn parse_rule(parser: &mut Parser<'_>, rule: RuleId) -> Option<Attempt> {
    let result = match rule {
        rules::ARGUMENT_LIST => calls::parse_argument_list(parser),
        rules::BINDING => structures::parse_binding(parser),
        rules::BRACE_SUBSCRIPT => subscripts::parse_brace_subscript(parser),
        rules::BRACKET_SUBSCRIPT => subscripts::parse_bracket_subscript(parser),
        rules::CALL_ARG => calls::parse_call_arg(parser),
        rules::CALL_ARG_WITH_BINDING => calls::parse_call_arg_with_binding(parser),
        rules::COMPREHENSION_QUALIFIER => comprehensions::parse_comprehension_qualifier(parser),
        rules::EXPRESSION => expressions::parse_expression(parser),
        rules::FACTOR => precedence::parse_factor(parser),
        rules::FANCY_TABLE => structures::parse_fancy_table(parser),
        rules::FANCY_TABLE_HEADER => structures::parse_fancy_table_header(parser),
        rules::FIELD => structures::parse_field(parser),
        rules::FORMULA => expressions::parse_formula(parser),
        rules::FORMULA_SUBSCRIPT => subscripts::parse_formula_subscript(parser),
        rules::FSM_ARGS => fsm::parse_fsm_args(parser),
        rules::FSM_ASYNC_TRANSITION => fsm::parse_fsm_async_transition(parser),
        rules::FSM_INSTANCE => fsm::parse_fsm_instance(parser),
        rules::FSM_OUTPUT => fsm::parse_fsm_output(parser),
        rules::FSM_PIPE => fsm::parse_fsm_pipe(parser),
        rules::FSM_STATE_TRANSITION => fsm::parse_fsm_state_transition(parser),
        rules::FSM_VALUE => fsm::parse_fsm_value(parser),
        rules::FUNCTION_CALL => calls::parse_function_call(parser),
        rules::GENERATOR => comprehensions::parse_generator(parser),
        rules::HEADER_FIELD => structures::parse_header_field(parser),
        rules::INLINE_TABLE => structures::parse_inline_table(parser),
        rules::INLINE_TABLE_HEADER => structures::parse_inline_table_header(parser),
        rules::INLINE_TABLE_ROW => structures::parse_inline_table_row(parser),
        rules::KIND => kinds::parse_kind(parser),
        rules::KIND_ANNOTATION => kinds::parse_kind_annotation(parser),
        rules::KIND_KIND => kinds::parse_kind_kind(parser),
        rules::KIND_MAP => kinds::parse_kind_map(parser),
        rules::KIND_MATRIX => kinds::parse_kind_matrix(parser),
        rules::KIND_RECORD => kinds::parse_kind_record(parser),
        rules::KIND_SCALAR => kinds::parse_kind_scalar(parser),
        rules::KIND_SET => kinds::parse_kind_set(parser),
        rules::KIND_TABLE => kinds::parse_kind_table(parser),
        rules::KIND_TUPLE => kinds::parse_kind_tuple(parser),
        rules::KIND_WITH_OPTION => kinds::parse_kind_with_option(parser),
        rules::L1 => precedence::parse_l1(parser),
        rules::L2 => precedence::parse_l2(parser),
        rules::L3 => precedence::parse_l3(parser),
        rules::L4 => precedence::parse_l4(parser),
        rules::L5 => precedence::parse_l5(parser),
        rules::L6 => precedence::parse_l6(parser),
        rules::L7 => precedence::parse_l7(parser),
        rules::LITERAL => literals::parse_literal(parser),
        rules::MAP => structures::parse_map(parser),
        rules::MAPPING => structures::parse_mapping(parser),
        rules::MATCH_ARM => precedence::parse_match_arm(parser),
        rules::MATRIX => structures::parse_matrix(parser),
        rules::MATRIX_COLUMN => structures::parse_matrix_column(parser),
        rules::MATRIX_COMPREHENSION => comprehensions::parse_matrix_comprehension(parser),
        rules::MATRIX_ROW => structures::parse_matrix_row(parser),
        rules::NEGATE_FACTOR => precedence::parse_negate_factor(parser),
        rules::NOT_FACTOR => precedence::parse_not_factor(parser),
        rules::PARENTHETICAL_TERM => precedence::parse_parenthetical_term(parser),
        rules::PATTERN => patterns::parse_pattern(parser),
        rules::PATTERN_ARRAY => patterns::parse_pattern_array(parser),
        rules::PATTERN_ARRAY_ITEM => patterns::parse_pattern_array_item(parser),
        rules::PATTERN_ARRAY_TOKEN => patterns::parse_pattern_array_token(parser),
        rules::PATTERN_ATOM_STRUCT => patterns::parse_pattern_atom_struct(parser),
        rules::PATTERN_TUPLE => patterns::parse_pattern_tuple(parser),
        rules::PATTERN_TUPLE_STRUCT => patterns::parse_pattern_tuple_struct(parser),
        rules::RANGE_EXPRESSION => precedence::parse_range_expression(parser),
        rules::RANGE_SUBSCRIPT => subscripts::parse_range_subscript(parser),
        rules::RECORD => structures::parse_record(parser),
        rules::REGULAR_TABLE => structures::parse_regular_table(parser),
        rules::SET => structures::parse_set(parser),
        rules::SET_COMPREHENSION => comprehensions::parse_set_comprehension(parser),
        rules::SLICE => subscripts::parse_slice(parser),
        rules::STRUCTURE => structures::parse_structure(parser),
        rules::SUBSCRIPT => subscripts::parse_subscript(parser),
        rules::TABLE => structures::parse_table(parser),
        rules::TABLE_HEADER => structures::parse_table_header(parser),
        rules::TABLE_ROW => structures::parse_table_row(parser),
        rules::TABLE_ROW2 => structures::parse_table_row2(parser),
        rules::TUPLE => structures::parse_tuple(parser),
        rules::TUPLE_STRUCT => structures::parse_tuple_struct(parser),
        rules::VAR => variables::parse_var(parser),
        rules::VARIABLE_DEFINE => variables::parse_variable_define(parser),
        _ => return None,
    };
    Some(result)
}
