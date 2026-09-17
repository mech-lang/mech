mod continuation;
use crate::document::{RuleId, SyntaxKind};
pub(super) use continuation::{StructureContinuation, supports as continuation_supports};

use super::super::super::rule::rules;
use super::super::super::{Parser, ParserCheckpoint};
use super::super::structure_shell;
use super::{Attempt, child_result};

pub(super) fn parse_structure(parser: &mut Parser<'_>) -> Attempt {
    super::Continuation::new(rules::STRUCTURE).drive(parser)
}

pub(super) fn parse_matrix(parser: &mut Parser<'_>) -> Attempt {
    super::Continuation::new(rules::MATRIX).drive(parser)
}

pub(super) fn parse_matrix_row(parser: &mut Parser<'_>) -> Attempt {
    super::Continuation::new(rules::MATRIX_ROW).drive(parser)
}

pub(super) fn parse_matrix_column(parser: &mut Parser<'_>) -> Attempt {
    super::Continuation::new(rules::MATRIX_COLUMN).drive(parser)
}

pub(super) fn parse_table(parser: &mut Parser<'_>) -> Attempt {
    StructureContinuation::new(rules::TABLE).drive(parser)
}

pub(super) fn parse_fancy_table(parser: &mut Parser<'_>) -> Attempt {
    StructureContinuation::new(rules::FANCY_TABLE).drive(parser)
}

pub(super) fn parse_fancy_table_header(parser: &mut Parser<'_>) -> Attempt {
    super::Continuation::new(rules::FANCY_TABLE_HEADER).drive(parser)
}

pub(super) fn parse_inline_table(parser: &mut Parser<'_>) -> Attempt {
    StructureContinuation::new(rules::INLINE_TABLE).drive(parser)
}

pub(super) fn parse_inline_table_header(parser: &mut Parser<'_>) -> Attempt {
    super::Continuation::new(rules::INLINE_TABLE_HEADER).drive(parser)
}

pub(super) fn parse_inline_table_row(parser: &mut Parser<'_>) -> Attempt {
    super::Continuation::new(rules::INLINE_TABLE_ROW).drive(parser)
}

pub(super) fn parse_regular_table(parser: &mut Parser<'_>) -> Attempt {
    StructureContinuation::new(rules::REGULAR_TABLE).drive(parser)
}

pub(super) fn parse_table_header(parser: &mut Parser<'_>) -> Attempt {
    super::Continuation::new(rules::TABLE_HEADER).drive(parser)
}

pub(super) fn parse_table_row(parser: &mut Parser<'_>) -> Attempt {
    super::Continuation::new(rules::TABLE_ROW).drive(parser)
}

pub(super) fn parse_table_row2(parser: &mut Parser<'_>) -> Attempt {
    super::Continuation::new(rules::TABLE_ROW2).drive(parser)
}

pub(super) fn parse_header_field(parser: &mut Parser<'_>) -> Attempt {
    super::Continuation::new(rules::HEADER_FIELD).drive(parser)
}

pub(super) fn parse_field(parser: &mut Parser<'_>) -> Attempt {
    super::Continuation::new(rules::FIELD).drive(parser)
}

pub(super) fn parse_map(parser: &mut Parser<'_>) -> Attempt {
    super::Continuation::new(rules::MAP).drive(parser)
}

pub(super) fn parse_mapping(parser: &mut Parser<'_>) -> Attempt {
    super::Continuation::new(rules::MAPPING).drive(parser)
}

pub(super) fn parse_record(parser: &mut Parser<'_>) -> Attempt {
    StructureContinuation::new(rules::RECORD).drive(parser)
}

pub(super) fn parse_binding(parser: &mut Parser<'_>) -> Attempt {
    super::Continuation::new(rules::BINDING).drive(parser)
}

pub(super) fn parse_set(parser: &mut Parser<'_>) -> Attempt {
    super::Continuation::new(rules::SET).drive(parser)
}

pub(super) fn parse_tuple(parser: &mut Parser<'_>) -> Attempt {
    super::Continuation::new(rules::TUPLE).drive(parser)
}

pub(super) fn parse_tuple_struct(parser: &mut Parser<'_>) -> Attempt {
    super::Continuation::new(rules::TUPLE_STRUCT).drive(parser)
}

pub(super) fn matrix_comprehension(parser: &mut Parser<'_>) -> Attempt {
    super::Continuation::new(rules::MATRIX_COMPREHENSION).drive(parser)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BracketMode {
    MatrixOnly,
    ComprehensionOnly,
    Either,
}

#[derive(Clone, Copy)]
pub(super) enum TableDelimiter {
    Brace,
    Bar,
    Box,
}

#[derive(Clone, Copy)]
pub(super) struct BindingCandidate {
    pub(super) value_start: ParserCheckpoint,
    pub(super) value_end: ParserCheckpoint,
}
