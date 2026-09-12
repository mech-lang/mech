use crate::document::{RuleId, SyntaxKind};

use super::super::super::rule::rules;
use super::super::super::{CleanSubtree, Parser, ParserCheckpoint};
use super::super::{base, combinator, structure_shell};
use super::{
    Attempt, BracketForm, ExpressionForm, FactAttempt, child_result, comprehensions, expressions,
    kinds, missing_production, nesting_limit, recover_closer, recover_closer_set,
    recover_required_production, transactional_fact,
};

pub(super) fn parse_structure(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::STRUCTURE, |parser| {
        let node = parser.start();
        let selected = structure_body(parser);
        if let Some(result) = child_result(parser, node, SyntaxKind::Structure, selected) {
            return result;
        }
        node.complete(parser, SyntaxKind::Structure);
        Attempt::Matched
    })
}

pub(super) fn parse_matrix(parser: &mut Parser<'_>) -> Attempt {
    bracket_body(parser, BracketMode::MatrixOnly).attempt()
}

pub(super) fn parse_matrix_row(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::MATRIX_ROW, |parser| {
        let node = parser.start();
        if !base::parse_rule(parser, rules::SPACE_TAB0) {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let _ = structure_shell::parse_table_separator(parser);
        if !base::parse_rule(parser, rules::SPACE_TAB0) {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        match parse_matrix_column(parser) {
            Attempt::Matched => {}
            Attempt::NoMatch => {
                node.abandon(parser);
                return Attempt::NoMatch;
            }
            Attempt::Committed => {
                matrix_row_suffix(parser);
                node.complete(parser, SyntaxKind::MatrixRow);
                return Attempt::Committed;
            }
        }
        loop {
            let before = parser.offset();
            match parse_matrix_column(parser) {
                Attempt::Matched if parser.offset() > before => {}
                Attempt::Matched | Attempt::NoMatch => break,
                Attempt::Committed => {
                    matrix_row_suffix(parser);
                    node.complete(parser, SyntaxKind::MatrixRow);
                    return Attempt::Committed;
                }
            }
        }
        let _ = base::parse_rule(parser, rules::SEMICOLON);
        let _ = base::parse_rule(parser, rules::NEW_LINE);
        let border = parser.checkpoint();
        if base::parse_rule(parser, rules::BOX_DRAWING_CHAR) {
            while base::parse_rule(parser, rules::BOX_DRAWING_CHAR) {}
            if !base::parse_rule(parser, rules::NEW_LINE) {
                parser.rewind(border);
            }
        }
        node.complete(parser, SyntaxKind::MatrixRow);
        Attempt::Matched
    })
}

pub(super) fn parse_matrix_column(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::MATRIX_COLUMN, |parser| {
        let node = parser.start();
        if !base::parse_rule(parser, rules::SPACE_TAB0) {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let child = expressions::parse_expression(parser);
        if let Some(result) = child_result(parser, node, SyntaxKind::MatrixColumn, child) {
            return result;
        }
        if !base::parse_rule(parser, rules::SPACE_TAB0) {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        if !base::parse_rule(parser, rules::COMMA) && !base::parse_rule(parser, rules::BOX_VERT) {
            let _ = base::parse_rule(parser, rules::BOX_VERT_BOLD);
        }
        if !base::parse_rule(parser, rules::SPACE_TAB0) {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        node.complete(parser, SyntaxKind::MatrixColumn);
        Attempt::Matched
    })
}

pub(super) fn parse_table(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::TABLE, |parser| {
        let node = parser.start();
        let selected = choice(
            parser,
            &[parse_inline_table, parse_regular_table, parse_fancy_table],
        );
        if let Some(result) = child_result(parser, node, SyntaxKind::Table, selected) {
            return result;
        }
        node.complete(parser, SyntaxKind::Table);
        Attempt::Matched
    })
}

pub(super) fn parse_fancy_table(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::FANCY_TABLE, |parser| {
        let node = parser.start();
        if structure_shell::parse_table_top(parser) != Attempt::Matched
            || structure_shell::parse_table_separator(parser) != Attempt::Matched
        {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let mut committed = false;
        let child = parse_fancy_table_header(parser);
        match child {
            Attempt::Matched => {}
            Attempt::NoMatch => {
                recover_required_production(
                    parser,
                    rules::FANCY_TABLE,
                    "syntax/missing-fancy-table-header",
                    "missing framed table header",
                    "fancy-table-header",
                );
                node.complete(parser, SyntaxKind::FancyTable);
                return Attempt::Committed;
            }
            Attempt::Committed => {
                committed = true;
                let _ = base::parse_rule(parser, rules::WHITESPACE0);
            }
        }
        let first = fancy_row(parser);
        match first {
            Attempt::Matched => {}
            Attempt::NoMatch => {
                recover_required_production(
                    parser,
                    rules::FANCY_TABLE,
                    "syntax/missing-fancy-table-row",
                    "missing framed table row",
                    "table-row2",
                );
                node.complete(parser, SyntaxKind::FancyTable);
                return Attempt::Committed;
            }
            Attempt::Committed => {
                committed = true;
            }
        }
        loop {
            let pair = parser.checkpoint();
            if !base::parse_rule(parser, rules::NEW_LINE) {
                break;
            }
            match fancy_row(parser) {
                Attempt::Matched => {}
                Attempt::NoMatch => {
                    parser.rewind(pair);
                    break;
                }
                Attempt::Committed => {
                    committed = true;
                }
            }
        }
        node.complete(parser, SyntaxKind::FancyTable);
        if committed {
            Attempt::Committed
        } else {
            Attempt::Matched
        }
    })
}

pub(super) fn parse_fancy_table_header(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::FANCY_TABLE_HEADER, |parser| {
        let node = parser.start();
        let first = parse_field(parser);
        match first {
            Attempt::Matched => {}
            Attempt::NoMatch => {
                node.abandon(parser);
                return Attempt::NoMatch;
            }
            Attempt::Committed => {
                recover_table_separator(parser, rules::FANCY_TABLE_HEADER);
                node.complete(parser, SyntaxKind::FancyTableHeader);
                return Attempt::Committed;
            }
        }
        loop {
            let pair = parser.checkpoint();
            if structure_shell::parse_table_separator(parser) != Attempt::Matched {
                break;
            }
            match parse_field(parser) {
                Attempt::Matched => {}
                Attempt::NoMatch => {
                    parser.rewind(pair);
                    break;
                }
                Attempt::Committed => {
                    recover_table_separator(parser, rules::FANCY_TABLE_HEADER);
                    node.complete(parser, SyntaxKind::FancyTableHeader);
                    return Attempt::Committed;
                }
            }
        }
        if structure_shell::parse_table_separator(parser) != Attempt::Matched {
            recover_table_separator(parser, rules::FANCY_TABLE_HEADER);
            node.complete(parser, SyntaxKind::FancyTableHeader);
            return Attempt::Committed;
        }
        let _ = base::parse_rule(parser, rules::WHITESPACE0);
        node.complete(parser, SyntaxKind::FancyTableHeader);
        Attempt::Matched
    })
}

pub(super) fn parse_inline_table(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::INLINE_TABLE, |parser| {
        let node = parser.start();
        if structure_shell::parse_table_separator(parser) != Attempt::Matched
            || !base::parse_rule(parser, rules::SPACE_TAB0)
        {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let child = parse_inline_table_header(parser);
        match child {
            Attempt::Matched => {}
            Attempt::NoMatch => {
                node.abandon(parser);
                return Attempt::NoMatch;
            }
            Attempt::Committed => {
                if parser.cursor().starts_with("\n") || parser.cursor().starts_with("\r") {
                    node.abandon(parser);
                    return Attempt::NoMatch;
                }
                node.complete(parser, SyntaxKind::InlineTable);
                return Attempt::Committed;
            }
        }
        if !base::parse_rule(parser, rules::SPACE_TAB0) {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let child = parse_inline_table_row(parser);
        match child {
            Attempt::Matched => {}
            Attempt::NoMatch => {
                if parser.cursor().starts_with("\n") || parser.cursor().starts_with("\r") {
                    node.abandon(parser);
                    return Attempt::NoMatch;
                }
                recover_required_production(
                    parser,
                    rules::INLINE_TABLE,
                    "syntax/missing-inline-table-row",
                    "missing inline table row",
                    "inline-table-row",
                );
                node.complete(parser, SyntaxKind::InlineTable);
                return Attempt::Committed;
            }
            Attempt::Committed => {
                node.complete(parser, SyntaxKind::InlineTable);
                return Attempt::Committed;
            }
        }
        loop {
            let before = parser.offset();
            match parse_inline_table_row(parser) {
                Attempt::Matched if parser.offset() > before => {}
                Attempt::Matched | Attempt::NoMatch => break,
                Attempt::Committed => {
                    node.complete(parser, SyntaxKind::InlineTable);
                    return Attempt::Committed;
                }
            }
        }
        node.complete(parser, SyntaxKind::InlineTable);
        Attempt::Matched
    })
}

pub(super) fn parse_inline_table_header(parser: &mut Parser<'_>) -> Attempt {
    header_list(
        parser,
        rules::INLINE_TABLE_HEADER,
        SyntaxKind::InlineTableHeader,
        false,
    )
}

pub(super) fn parse_inline_table_row(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::INLINE_TABLE_ROW, |parser| {
        let node = parser.start();
        let first = inline_table_item(parser);
        match first {
            Attempt::Matched => {}
            Attempt::NoMatch => {
                if !ahead(parser, structure_shell::parse_table_separator) {
                    node.abandon(parser);
                    return Attempt::NoMatch;
                }
                recover_required_production(
                    parser,
                    rules::INLINE_TABLE_ROW,
                    "syntax/missing-table-cell",
                    "missing inline table cell",
                    "expression",
                );
                recover_table_separator(parser, rules::INLINE_TABLE_ROW);
                node.complete(parser, SyntaxKind::InlineTableRow);
                return Attempt::Committed;
            }
            Attempt::Committed => {
                recover_table_separator(parser, rules::INLINE_TABLE_ROW);
                node.complete(parser, SyntaxKind::InlineTableRow);
                return Attempt::Committed;
            }
        }
        loop {
            let before = parser.offset();
            match inline_table_item(parser) {
                Attempt::Matched if parser.offset() > before => {}
                Attempt::Matched | Attempt::NoMatch => break,
                Attempt::Committed => {
                    recover_table_separator(parser, rules::INLINE_TABLE_ROW);
                    node.complete(parser, SyntaxKind::InlineTableRow);
                    return Attempt::Committed;
                }
            }
        }
        let _ = base::parse_rule(parser, rules::SPACE_TAB0);
        if structure_shell::parse_table_separator(parser) != Attempt::Matched {
            recover_table_separator(parser, rules::INLINE_TABLE_ROW);
            node.complete(parser, SyntaxKind::InlineTableRow);
            return Attempt::Committed;
        }
        node.complete(parser, SyntaxKind::InlineTableRow);
        Attempt::Matched
    })
}

pub(super) fn parse_regular_table(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::REGULAR_TABLE, |parser| {
        let node = parser.start();
        if structure_shell::parse_table_separator(parser) != Attempt::Matched
            || !base::parse_rule(parser, rules::WHITESPACE0)
        {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let mut committed = false;
        let child = parse_table_header(parser);
        match child {
            Attempt::Matched => {}
            Attempt::NoMatch => {
                node.abandon(parser);
                return Attempt::NoMatch;
            }
            Attempt::Committed => {
                committed = true;
                let _ = base::parse_rule(parser, rules::WHITESPACE0);
            }
        }
        let first = parse_table_row(parser);
        match first {
            Attempt::Matched => {}
            Attempt::NoMatch => {
                recover_required_production(
                    parser,
                    rules::REGULAR_TABLE,
                    "syntax/missing-regular-table-row",
                    "missing regular table row",
                    "table-row",
                );
                node.complete(parser, SyntaxKind::RegularTable);
                return Attempt::Committed;
            }
            Attempt::Committed => {
                committed = true;
            }
        }
        while !parser.is_halted() {
            let pair = parser.checkpoint();
            if !base::parse_rule(parser, rules::WHITESPACE0) {
                break;
            }
            match parse_table_row(parser) {
                Attempt::Matched => {}
                Attempt::NoMatch => {
                    parser.rewind(pair);
                    break;
                }
                Attempt::Committed => {
                    committed = true;
                }
            }
        }
        node.complete(parser, SyntaxKind::RegularTable);
        if committed {
            Attempt::Committed
        } else {
            Attempt::Matched
        }
    })
}

pub(super) fn parse_table_header(parser: &mut Parser<'_>) -> Attempt {
    header_list(parser, rules::TABLE_HEADER, SyntaxKind::TableHeader, true)
}

pub(super) fn parse_table_row(parser: &mut Parser<'_>) -> Attempt {
    spaced_table_row(parser, rules::TABLE_ROW, SyntaxKind::TableRow)
}

pub(super) fn parse_table_row2(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::TABLE_ROW2, |parser| {
        let node = parser.start();
        if structure_shell::parse_table_separator(parser) != Attempt::Matched {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let first = expressions::parse_expression(parser);
        match first {
            Attempt::Matched => {}
            Attempt::NoMatch => {
                recover_required_production(
                    parser,
                    rules::TABLE_ROW2,
                    "syntax/missing-table-cell",
                    "missing table row cell after separator",
                    "expression",
                );
                recover_table_separator(parser, rules::TABLE_ROW2);
                node.complete(parser, SyntaxKind::FancyTableRow);
                return Attempt::Committed;
            }
            Attempt::Committed => {
                recover_table_separator(parser, rules::TABLE_ROW2);
                node.complete(parser, SyntaxKind::FancyTableRow);
                return Attempt::Committed;
            }
        }
        loop {
            let pair = parser.checkpoint();
            if !base::parse_rule(parser, rules::SPACE_TAB0)
                || structure_shell::parse_table_separator(parser) != Attempt::Matched
                || !base::parse_rule(parser, rules::SPACE_TAB0)
            {
                parser.rewind(pair);
                break;
            }
            match expressions::parse_expression(parser) {
                Attempt::Matched => {}
                Attempt::NoMatch => {
                    parser.rewind(pair);
                    break;
                }
                Attempt::Committed => {
                    recover_table_separator(parser, rules::TABLE_ROW2);
                    node.complete(parser, SyntaxKind::FancyTableRow);
                    return Attempt::Committed;
                }
            }
        }
        let _ = base::parse_rule(parser, rules::SPACE_TAB0);
        if structure_shell::parse_table_separator(parser) != Attempt::Matched {
            recover_table_separator(parser, rules::TABLE_ROW2);
            node.complete(parser, SyntaxKind::FancyTableRow);
            return Attempt::Committed;
        }
        let _ = base::parse_rule(parser, rules::SPACE_TAB0);
        node.complete(parser, SyntaxKind::FancyTableRow);
        Attempt::Matched
    })
}

pub(super) fn parse_header_field(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::HEADER_FIELD, |parser| {
        let node = parser.start();
        if !base::parse_rule(parser, rules::IDENTIFIER) {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let child = kinds::parse_kind_annotation(parser);
        if let Some(result) = child_result(parser, node, SyntaxKind::HeaderField, child) {
            return result;
        }
        node.complete(parser, SyntaxKind::HeaderField);
        Attempt::Matched
    })
}

pub(super) fn parse_field(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::FIELD, |parser| {
        let node = parser.start();
        if !base::parse_rule(parser, rules::IDENTIFIER) {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        if kinds::parse_kind_annotation(parser) == Attempt::Committed {
            node.complete(parser, SyntaxKind::TableField);
            return Attempt::Committed;
        }
        node.complete(parser, SyntaxKind::TableField);
        Attempt::Matched
    })
}

pub(super) fn parse_map(parser: &mut Parser<'_>) -> Attempt {
    delimited_repeated(
        parser,
        rules::MAP,
        SyntaxKind::Map,
        rules::LEFT_BRACE,
        rules::RIGHT_BRACE,
        parse_mapping,
    )
}

pub(super) fn parse_mapping(parser: &mut Parser<'_>) -> Attempt {
    parse_mapping_with_cached_value(parser, None)
}

fn parse_mapping_with_cached_value(
    parser: &mut Parser<'_>,
    cached_value: Option<&CleanSubtree>,
) -> Attempt {
    combinator::transactional(parser, rules::MAPPING, |parser| {
        let node = parser.start();
        if !base::parse_rule(parser, rules::WHITESPACE0) {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let child = expressions::parse_expression(parser);
        if let Some(result) = child_result(parser, node, SyntaxKind::MapEntry, child) {
            return result;
        }
        if !base::parse_rule(parser, rules::WHITESPACE0)
            || !base::parse_rule(parser, rules::COLON)
            || !base::parse_rule(parser, rules::WHITESPACE0)
        {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let child = if cached_value.is_some_and(|value| parser.reuse_clean_subtree(value)) {
            Attempt::Matched
        } else if parser.is_halted() {
            Attempt::Committed
        } else {
            expressions::parse_expression(parser)
        };
        match child {
            Attempt::Matched => {}
            Attempt::Committed => {
                node.complete(parser, SyntaxKind::MapEntry);
                return Attempt::Committed;
            }
            Attempt::NoMatch => {
                recover_required_production(
                    parser,
                    rules::MAPPING,
                    "syntax/missing-mapping-value",
                    "missing value after mapping colon",
                    "expression",
                );
                node.complete(parser, SyntaxKind::MapEntry);
                return Attempt::Committed;
            }
        }
        if !base::parse_rule(parser, rules::WHITESPACE0) {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let _ = base::parse_rule(parser, rules::COMMA);
        if !base::parse_rule(parser, rules::WHITESPACE0) {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        node.complete(parser, SyntaxKind::MapEntry);
        Attempt::Matched
    })
}

pub(super) fn parse_record(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::RECORD, |parser| {
        let node = parser.start();
        let delimiter = if parser.cursor().starts_with("{") {
            TableDelimiter::Brace
        } else if parser.cursor().starts_with("|") {
            TableDelimiter::Bar
        } else {
            TableDelimiter::Box
        };
        if structure_shell::parse_table_start(parser) != Attempt::Matched {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let Some(interior) = parser.with_nesting(|parser| {
            if !base::parse_rule(parser, rules::WHITESPACE0) {
                return Attempt::NoMatch;
            }
            let mut parsed_any = false;
            let mut committed = false;
            while !parser.is_halted() {
                let before = parser.offset();
                match parse_binding(parser) {
                    Attempt::Matched if parser.offset() > before => parsed_any = true,
                    Attempt::Matched => break,
                    Attempt::NoMatch if !parsed_any => return Attempt::NoMatch,
                    Attempt::NoMatch => break,
                    Attempt::Committed => {
                        parsed_any = true;
                        committed = true;
                        if !base::parse_rule(parser, rules::LIST_SEPARATOR) {
                            break;
                        }
                    }
                }
            }
            let _ = base::parse_rule(parser, rules::WHITESPACE0);
            if structure_shell::parse_table_end(parser) == Attempt::Matched {
                if committed {
                    Attempt::Committed
                } else {
                    Attempt::Matched
                }
            } else if parser.is_eof() {
                recover_table_end(parser, rules::RECORD, delimiter)
            } else if has_mapping_separator(parser) {
                Attempt::NoMatch
            } else {
                recover_table_end(parser, rules::RECORD, delimiter)
            }
        }) else {
            let result = nesting_limit(parser);
            node.complete(parser, SyntaxKind::Record);
            return result;
        };
        finish(node, parser, SyntaxKind::Record, interior)
    })
}

pub(super) fn parse_binding(parser: &mut Parser<'_>) -> Attempt {
    binding_with_marker(parser).attempt()
}

fn binding_with_marker(parser: &mut Parser<'_>) -> FactAttempt<BindingCandidate> {
    transactional_fact(parser, rules::BINDING, |parser| {
        let node = parser.start();
        if !base::parse_rule(parser, rules::WHITESPACE0)
            || !base::parse_rule(parser, rules::IDENTIFIER)
        {
            node.abandon(parser);
            return FactAttempt::NoMatch;
        }
        if parser.is_halted() || kinds::parse_kind_annotation(parser) == Attempt::Committed {
            node.complete(parser, SyntaxKind::RecordBinding);
            return FactAttempt::Committed;
        }
        if !base::parse_rule(parser, rules::WHITESPACE0)
            || !base::parse_rule(parser, rules::COLON)
            || !base::parse_rule(parser, rules::WHITESPACE0)
        {
            node.abandon(parser);
            return FactAttempt::NoMatch;
        }
        let value_start = parser.checkpoint();
        match expressions::parse_expression(parser) {
            Attempt::Matched => {}
            Attempt::NoMatch => {
                recover_required_production(
                    parser,
                    rules::BINDING,
                    "syntax/missing-binding-value",
                    "missing value after binding colon",
                    "expression",
                );
                node.complete(parser, SyntaxKind::RecordBinding);
                return FactAttempt::Committed;
            }
            Attempt::Committed => {
                node.complete(parser, SyntaxKind::RecordBinding);
                return FactAttempt::Committed;
            }
        }
        let value_end = parser.checkpoint();
        if !base::parse_rule(parser, rules::WHITESPACE0) {
            node.abandon(parser);
            return FactAttempt::NoMatch;
        }
        let _ = base::parse_rule(parser, rules::COMMA);
        if !base::parse_rule(parser, rules::WHITESPACE0) {
            node.abandon(parser);
            return FactAttempt::NoMatch;
        }
        node.complete(parser, SyntaxKind::RecordBinding);
        FactAttempt::Matched(BindingCandidate {
            value_start,
            value_end,
        })
    })
}

pub(super) fn parse_set(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::SET, |parser| {
        let node = parser.start();
        if !base::parse_rule(parser, rules::LEFT_BRACE) {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let Some(interior) = parser.with_nesting(|parser| {
            if !base::parse_rule(parser, rules::WHITESPACE0) {
                return Attempt::NoMatch;
            }
            let mut committed = false;
            match expressions::parse_expression(parser) {
                Attempt::Matched => {}
                Attempt::NoMatch => return Attempt::NoMatch,
                Attempt::Committed => committed = true,
            }
            while !parser.is_halted() {
                if !base::parse_rule(parser, rules::LIST_SEPARATOR)
                    && !base::parse_rule(parser, rules::WHITESPACE1)
                {
                    break;
                }
                match expressions::parse_expression(parser) {
                    Attempt::Matched => {}
                    Attempt::NoMatch => {
                        recover_required_production(
                            parser,
                            rules::SET,
                            "syntax/missing-set-item",
                            "missing set item after separator",
                            "expression",
                        );
                        committed = true;
                    }
                    Attempt::Committed => {
                        committed = true;
                    }
                }
            }
            let _ = base::parse_rule(parser, rules::WHITESPACE0);
            if base::parse_rule(parser, rules::RIGHT_BRACE) {
                if committed {
                    Attempt::Committed
                } else {
                    Attempt::Matched
                }
            } else {
                recover_closer(
                    parser,
                    rules::SET,
                    rules::RIGHT_BRACE,
                    SyntaxKind::RightBrace,
                    '}',
                    "}",
                )
            }
        }) else {
            let result = nesting_limit(parser);
            node.complete(parser, SyntaxKind::Set);
            return result;
        };
        finish(node, parser, SyntaxKind::Set, interior)
    })
}

pub(super) fn parse_tuple(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::TUPLE, |parser| {
        let node = parser.start();
        if !base::parse_rule(parser, rules::LEFT_PARENTHESIS) {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let Some(interior) = parser.with_nesting(|parser| {
            if !base::parse_rule(parser, rules::WHITESPACE0) {
                return Attempt::NoMatch;
            }
            if base::parse_rule(parser, rules::RIGHT_PARENTHESIS) {
                return Attempt::Matched;
            }
            let mut committed = false;
            match expressions::parse_expression(parser) {
                Attempt::Matched => {}
                Attempt::NoMatch => {
                    recover_required_production(
                        parser,
                        rules::TUPLE,
                        "syntax/missing-tuple-item",
                        "missing tuple item",
                        "expression",
                    );
                    committed = true;
                }
                Attempt::Committed => committed = true,
            }
            while !parser.is_halted() {
                if !base::parse_rule(parser, rules::LIST_SEPARATOR) {
                    break;
                }
                match expressions::parse_expression(parser) {
                    Attempt::Matched => {}
                    Attempt::NoMatch => {
                        recover_required_production(
                            parser,
                            rules::TUPLE,
                            "syntax/missing-tuple-item",
                            "missing tuple item after separator",
                            "expression",
                        );
                        committed = true;
                    }
                    Attempt::Committed => {
                        committed = true;
                    }
                }
            }
            let _ = base::parse_rule(parser, rules::WHITESPACE0);
            if base::parse_rule(parser, rules::RIGHT_PARENTHESIS) {
                if committed {
                    Attempt::Committed
                } else {
                    Attempt::Matched
                }
            } else {
                recover_closer(
                    parser,
                    rules::TUPLE,
                    rules::RIGHT_PARENTHESIS,
                    SyntaxKind::RightParen,
                    ')',
                    ")",
                )
            }
        }) else {
            let result = nesting_limit(parser);
            node.complete(parser, SyntaxKind::Tuple);
            return result;
        };
        finish(node, parser, SyntaxKind::Tuple, interior)
    })
}

pub(super) fn parse_tuple_struct(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::TUPLE_STRUCT, |parser| {
        let node = parser.start();
        if !base::parse_rule(parser, rules::COLON)
            || !base::parse_rule(parser, rules::IDENTIFIER)
            || !base::parse_rule(parser, rules::LEFT_PARENTHESIS)
        {
            return finish(node, parser, SyntaxKind::TupleStruct, Attempt::NoMatch);
        }
        let Some(interior) = parser.with_nesting(|parser| {
            if !base::parse_rule(parser, rules::WHITESPACE0) {
                return Attempt::NoMatch;
            }
            let mut committed = false;
            match expressions::parse_expression(parser) {
                Attempt::Matched => {}
                Attempt::NoMatch => {
                    recover_required_production(
                        parser,
                        rules::TUPLE_STRUCT,
                        "syntax/missing-tuple-structure-value",
                        "missing tuple structure value",
                        "expression",
                    );
                    committed = true;
                }
                Attempt::Committed => committed = true,
            }
            let _ = base::parse_rule(parser, rules::WHITESPACE0);
            if base::parse_rule(parser, rules::RIGHT_PARENTHESIS) {
                if committed {
                    Attempt::Committed
                } else {
                    Attempt::Matched
                }
            } else {
                recover_closer(
                    parser,
                    rules::TUPLE_STRUCT,
                    rules::RIGHT_PARENTHESIS,
                    SyntaxKind::RightParen,
                    ')',
                    ")",
                )
            }
        }) else {
            let result = nesting_limit(parser);
            node.complete(parser, SyntaxKind::TupleStruct);
            return result;
        };
        finish(node, parser, SyntaxKind::TupleStruct, interior)
    })
}

pub(super) fn parenthesis_factor(parser: &mut Parser<'_>) -> Attempt {
    let checkpoint = parser.checkpoint();
    let structure = parser.start();
    let tuple = parser.start();
    let parenthetical = parser.start();
    if !base::parse_rule(parser, rules::LEFT_PARENTHESIS) {
        parser.rewind(checkpoint);
        return Attempt::NoMatch;
    }

    let Some(result) = parser.with_nesting(|parser| {
        let after_open = parser.checkpoint();
        if !base::parse_rule(parser, rules::WHITESPACE0) {
            return Attempt::NoMatch;
        }
        if base::parse_rule(parser, rules::RIGHT_PARENTHESIS) {
            finish_provisional_marker(parser, parenthetical, SyntaxKind::ParentheticalExpression);
            tuple.complete(parser, SyntaxKind::Tuple);
            structure.complete(parser, SyntaxKind::Structure);
            return Attempt::Matched;
        }
        parser.rewind(after_open);

        if !base::parse_rule(parser, rules::SPACE_TAB0) {
            return Attempt::NoMatch;
        }
        let expression = parser.start();
        let mut parenthetical_selected = false;
        let mut committed = false;
        let mut body = expressions::expression_body(parser);
        if body == FactAttempt::NoMatch {
            parser.rewind(after_open);
            if !base::parse_rule(parser, rules::WHITESPACE0) {
                return Attempt::NoMatch;
            }
            let expression = parser.start();
            body = expressions::expression_body(parser);
            match body {
                FactAttempt::Matched(_) => {
                    expression.complete(parser, SyntaxKind::Expression);
                }
                FactAttempt::NoMatch => return Attempt::NoMatch,
                FactAttempt::Committed => {
                    expression.complete(parser, SyntaxKind::Expression);
                    committed = true;
                    let after_body = parser.checkpoint();
                    let _ = base::parse_rule(parser, rules::WHITESPACE0);
                    parenthetical_selected = parser.cursor().starts_with(")");
                    parser.rewind(after_body);
                }
            };
        } else {
            match body {
                FactAttempt::Matched(ExpressionForm::Formula) => {
                    let after_body = parser.checkpoint();
                    if base::parse_rule(parser, rules::SPACE_TAB0)
                        && base::parse_rule(parser, rules::RIGHT_PARENTHESIS)
                    {
                        expression.abandon(parser);
                        parenthetical.complete(parser, SyntaxKind::ParentheticalExpression);
                        finish_provisional_marker(parser, tuple, SyntaxKind::Tuple);
                        finish_provisional_marker(parser, structure, SyntaxKind::Structure);
                        return Attempt::Matched;
                    }
                    parser.rewind(after_body);
                    expression.complete(parser, SyntaxKind::Expression);
                    parenthetical_selected = true;
                }
                FactAttempt::Matched(_) => {
                    expression.complete(parser, SyntaxKind::Expression);
                }
                FactAttempt::Committed => {
                    expression.complete(parser, SyntaxKind::Expression);
                    committed = true;
                    let after_body = parser.checkpoint();
                    let _ = base::parse_rule(parser, rules::WHITESPACE0);
                    parenthetical_selected = parser.cursor().starts_with(")");
                    parser.rewind(after_body);
                }
                FactAttempt::NoMatch => return Attempt::NoMatch,
            }
        }

        if !parenthetical_selected {
            finish_provisional_marker(parser, parenthetical, SyntaxKind::ParentheticalExpression);
        }
        while !parser.is_halted() {
            if !base::parse_rule(parser, rules::LIST_SEPARATOR) {
                break;
            }
            if parenthetical_selected {
                finish_provisional_marker(
                    parser,
                    parenthetical,
                    SyntaxKind::ParentheticalExpression,
                );
                parenthetical_selected = false;
            }
            match expressions::parse_expression(parser) {
                Attempt::Matched => {}
                Attempt::NoMatch => {
                    recover_required_production(
                        parser,
                        rules::TUPLE,
                        "syntax/missing-tuple-item",
                        "missing tuple item after separator",
                        "expression",
                    );
                    committed = true;
                    continue;
                }
                Attempt::Committed => {
                    committed = true;
                }
            }
        }
        let _ = base::parse_rule(parser, rules::WHITESPACE0);
        if !base::parse_rule(parser, rules::RIGHT_PARENTHESIS) {
            recover_closer(
                parser,
                if parenthetical_selected {
                    rules::PARENTHETICAL_TERM
                } else {
                    rules::TUPLE
                },
                rules::RIGHT_PARENTHESIS,
                SyntaxKind::RightParen,
                ')',
                ")",
            );
            committed = true;
        }
        if parenthetical_selected {
            parenthetical.complete(parser, SyntaxKind::ParentheticalExpression);
            finish_provisional_marker(parser, tuple, SyntaxKind::Tuple);
            finish_provisional_marker(parser, structure, SyntaxKind::Structure);
        } else {
            tuple.complete(parser, SyntaxKind::Tuple);
            structure.complete(parser, SyntaxKind::Structure);
        }
        if committed {
            Attempt::Committed
        } else {
            Attempt::Matched
        }
    }) else {
        nesting_limit(parser);
        parenthetical.complete(parser, SyntaxKind::ParentheticalExpression);
        tuple.complete(parser, SyntaxKind::Tuple);
        structure.complete(parser, SyntaxKind::Structure);
        return Attempt::Committed;
    };
    if result == Attempt::NoMatch {
        parser.rewind(checkpoint);
    }
    result
}

pub(super) fn bracket_factor(parser: &mut Parser<'_>) -> Attempt {
    let checkpoint = parser.checkpoint();
    let structure = parser.start();
    match bracket_body(parser, BracketMode::Either) {
        FactAttempt::Matched(BracketForm::Matrix) => {
            structure.complete(parser, SyntaxKind::Structure);
            Attempt::Matched
        }
        FactAttempt::Matched(BracketForm::Comprehension) => {
            structure.abandon(parser);
            Attempt::Matched
        }
        FactAttempt::Committed => {
            structure.complete(parser, SyntaxKind::Structure);
            Attempt::Committed
        }
        FactAttempt::NoMatch => {
            parser.rewind(checkpoint);
            Attempt::NoMatch
        }
    }
}

pub(super) fn bracket_expression(parser: &mut Parser<'_>) -> FactAttempt<ExpressionForm> {
    let checkpoint = parser.checkpoint();
    let structure = parser.start();
    match bracket_body(parser, BracketMode::Either) {
        FactAttempt::Matched(BracketForm::Matrix) => {
            structure.complete(parser, SyntaxKind::Structure);
            FactAttempt::Matched(ExpressionForm::Formula)
        }
        FactAttempt::Matched(BracketForm::Comprehension) => {
            structure.abandon(parser);
            FactAttempt::Matched(ExpressionForm::MatrixComprehension)
        }
        FactAttempt::Committed => {
            structure.complete(parser, SyntaxKind::Structure);
            FactAttempt::Committed
        }
        FactAttempt::NoMatch => {
            parser.rewind(checkpoint);
            FactAttempt::NoMatch
        }
    }
}

pub(super) fn matrix_comprehension(parser: &mut Parser<'_>) -> Attempt {
    bracket_body(parser, BracketMode::ComprehensionOnly).attempt()
}

pub(super) fn brace_factor(parser: &mut Parser<'_>) -> Attempt {
    brace_body(parser, BraceMode::StructureOnly).attempt()
}

pub(super) fn brace_expression(parser: &mut Parser<'_>) -> FactAttempt<ExpressionForm> {
    brace_body(parser, BraceMode::ExpressionEither)
}

pub(super) fn colon_factor(parser: &mut Parser<'_>) -> Attempt {
    let tuple = wrap_structure(parser, parse_tuple_struct);
    if tuple != Attempt::NoMatch {
        return tuple;
    }
    super::literals::parse_literal(parser)
}

pub(super) fn structure_non_delimited(parser: &mut Parser<'_>) -> Attempt {
    for parse in [
        wrap_table_structure as fn(&mut Parser<'_>) -> Attempt,
        |parser| wrap_structure(parser, parse_matrix),
        |parser| wrap_structure(parser, parse_record),
    ] {
        let result = parse(parser);
        if result != Attempt::NoMatch {
            return result;
        }
    }
    Attempt::NoMatch
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BracketMode {
    MatrixOnly,
    ComprehensionOnly,
    Either,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BraceMode {
    StructureOnly,
    ExpressionEither,
}

#[derive(Clone, Copy)]
enum TableDelimiter {
    Brace,
    Bar,
    Box,
}

#[derive(Clone, Copy)]
struct BindingCandidate {
    value_start: ParserCheckpoint,
    value_end: ParserCheckpoint,
}

fn bracket_body(parser: &mut Parser<'_>, mode: BracketMode) -> FactAttempt<BracketForm> {
    let rule = if mode == BracketMode::ComprehensionOnly {
        rules::MATRIX_COMPREHENSION
    } else {
        rules::MATRIX
    };
    transactional_fact(parser, rule, |parser| {
        let matrix = parser.start();
        let comprehension = parser.start();
        let ordinary_bracket = parser.cursor().starts_with("[");
        if structure_shell::parse_matrix_start(parser) != Attempt::Matched {
            comprehension.abandon(parser);
            matrix.abandon(parser);
            return FactAttempt::NoMatch;
        }
        let Some(result) = parser.with_nesting(|parser| {
            let after_open = parser.checkpoint();
            if ordinary_bracket && base::parse_rule(parser, rules::SPACE_TAB0) {
                let leading_separator =
                    structure_shell::parse_table_separator(parser) == Attempt::Matched;
                if !base::parse_rule(parser, rules::SPACE_TAB0) {
                    return FactAttempt::NoMatch;
                }
                if !leading_separator && ahead(parser, structure_shell::parse_matrix_end) {
                    if mode == BracketMode::ComprehensionOnly {
                        return FactAttempt::NoMatch;
                    }
                    if structure_shell::parse_matrix_end(parser) != Attempt::Matched {
                        return FactAttempt::NoMatch;
                    }
                    comprehension.abandon(parser);
                    matrix.complete(parser, SyntaxKind::Matrix);
                    return FactAttempt::Matched(BracketForm::Matrix);
                }

                let row = parser.start();
                let column = parser.start();
                let head = expressions::parse_expression(parser);
                match head {
                    Attempt::Matched | Attempt::Committed => {
                        if parser.is_halted() {
                            column.complete(parser, SyntaxKind::MatrixColumn);
                            row.complete(parser, SyntaxKind::MatrixRow);
                            return finish_matrix_body(
                                parser,
                                matrix,
                                comprehension,
                                ordinary_bracket,
                                true,
                            );
                        }
                        let after_expression = parser.checkpoint();
                        let selected_comprehension = !leading_separator
                            && base::parse_rule(parser, rules::SPACE_TAB0)
                            && base::parse_rule(parser, rules::BAR);
                        if selected_comprehension {
                            if mode == BracketMode::MatrixOnly {
                                return FactAttempt::NoMatch;
                            }
                            column.abandon(parser);
                            row.abandon(parser);
                            return match comprehensions::finish_qualifiers(
                                parser,
                                rules::RIGHT_BRACKET,
                                true,
                            ) {
                                Attempt::Matched => {
                                    comprehension.complete(parser, SyntaxKind::MatrixComprehension);
                                    matrix.abandon(parser);
                                    if head == Attempt::Committed {
                                        FactAttempt::Committed
                                    } else {
                                        FactAttempt::Matched(BracketForm::Comprehension)
                                    }
                                }
                                Attempt::NoMatch => FactAttempt::NoMatch,
                                Attempt::Committed => {
                                    comprehension.complete(parser, SyntaxKind::MatrixComprehension);
                                    finish_provisional_marker(parser, matrix, SyntaxKind::Matrix);
                                    FactAttempt::Committed
                                }
                            };
                        }
                        parser.rewind(after_expression);
                        if mode == BracketMode::ComprehensionOnly {
                            return FactAttempt::NoMatch;
                        }
                        if !matrix_column_tail(parser) {
                            return FactAttempt::NoMatch;
                        }
                        column.complete(parser, SyntaxKind::MatrixColumn);
                        match finish_seeded_matrix_row(parser, row) {
                            Attempt::Matched => {}
                            Attempt::NoMatch => return FactAttempt::NoMatch,
                            Attempt::Committed => {
                                return finish_matrix_body(
                                    parser,
                                    matrix,
                                    comprehension,
                                    ordinary_bracket,
                                    true,
                                );
                            }
                        }
                        return finish_matrix_body(
                            parser,
                            matrix,
                            comprehension,
                            ordinary_bracket,
                            head == Attempt::Committed,
                        );
                    }
                    Attempt::NoMatch => parser.rewind(after_open),
                }
            } else {
                parser.rewind(after_open);
            }

            if mode == BracketMode::ComprehensionOnly {
                return FactAttempt::NoMatch;
            }
            finish_matrix_body(parser, matrix, comprehension, ordinary_bracket, false)
        }) else {
            nesting_limit(parser);
            finish_provisional_marker(parser, comprehension, SyntaxKind::MatrixComprehension);
            matrix.complete(parser, SyntaxKind::Matrix);
            return FactAttempt::Committed;
        };
        result
    })
}

fn finish_seeded_matrix_row(
    parser: &mut Parser<'_>,
    row: super::super::super::marker::Marker,
) -> Attempt {
    loop {
        let before = parser.offset();
        match parse_matrix_column(parser) {
            Attempt::Matched if parser.offset() > before => {}
            Attempt::Matched | Attempt::NoMatch => break,
            Attempt::Committed => {
                matrix_row_suffix(parser);
                row.complete(parser, SyntaxKind::MatrixRow);
                return Attempt::Committed;
            }
        }
    }
    matrix_row_suffix(parser);
    row.complete(parser, SyntaxKind::MatrixRow);
    Attempt::Matched
}

fn finish_matrix_body(
    parser: &mut Parser<'_>,
    matrix: super::super::super::marker::Marker,
    comprehension: super::super::super::marker::Marker,
    ordinary_bracket: bool,
    mut committed: bool,
) -> FactAttempt<BracketForm> {
    loop {
        if parser.is_halted() {
            finish_provisional_marker(parser, comprehension, SyntaxKind::MatrixComprehension);
            matrix.complete(parser, SyntaxKind::Matrix);
            return FactAttempt::Committed;
        }
        match consume_matrix_decoration(parser) {
            Attempt::Matched => {}
            Attempt::Committed => {
                finish_provisional_marker(parser, comprehension, SyntaxKind::MatrixComprehension);
                matrix.complete(parser, SyntaxKind::Matrix);
                return FactAttempt::Committed;
            }
            Attempt::NoMatch => unreachable!("matrix decoration is optional"),
        }
        if ahead(parser, structure_shell::parse_matrix_end) {
            break;
        }
        let before = parser.offset();
        match parse_matrix_row(parser) {
            Attempt::Matched if parser.offset() > before => {}
            Attempt::Matched => return FactAttempt::NoMatch,
            Attempt::NoMatch => {
                recover_matrix_closer(parser, ordinary_bracket);
                finish_provisional_marker(parser, comprehension, SyntaxKind::MatrixComprehension);
                matrix.complete(parser, SyntaxKind::Matrix);
                return FactAttempt::Committed;
            }
            Attempt::Committed => {
                committed = true;
                if parser.offset() == before && !parser.is_halted() {
                    recover_matrix_closer(parser, ordinary_bracket);
                    finish_provisional_marker(
                        parser,
                        comprehension,
                        SyntaxKind::MatrixComprehension,
                    );
                    matrix.complete(parser, SyntaxKind::Matrix);
                    return FactAttempt::Committed;
                }
            }
        }
    }
    let _ = base::parse_rule(parser, rules::WHITESPACE0);
    if structure_shell::parse_matrix_end(parser) != Attempt::Matched {
        recover_matrix_closer(parser, ordinary_bracket);
        finish_provisional_marker(parser, comprehension, SyntaxKind::MatrixComprehension);
        matrix.complete(parser, SyntaxKind::Matrix);
        return FactAttempt::Committed;
    }
    comprehension.abandon(parser);
    matrix.complete(parser, SyntaxKind::Matrix);
    if committed {
        FactAttempt::Committed
    } else {
        FactAttempt::Matched(BracketForm::Matrix)
    }
}

fn recover_matrix_closer(parser: &mut Parser<'_>, ordinary_bracket: bool) -> Attempt {
    let (kind, fix) = if ordinary_bracket {
        (SyntaxKind::RightBracket, "]")
    } else {
        (SyntaxKind::BoxDrawing, "╯")
    };
    recover_closer_set(
        parser,
        rules::MATRIX,
        &[']', '╯', '┘', '┛', ')', '}'],
        kind,
        fix,
        |parser| structure_shell::parse_matrix_end(parser) == Attempt::Matched,
    )
}

fn consume_matrix_decoration(parser: &mut Parser<'_>) -> Attempt {
    loop {
        if ahead(parser, structure_shell::parse_matrix_end) {
            break;
        }
        let before = parser.offset();
        if !base::parse_rule(parser, rules::BOX_DRAWING_CHAR)
            && !base::parse_rule(parser, rules::WHITESPACE)
        {
            break;
        }
        if parser.is_halted() {
            return Attempt::Committed;
        }
        if parser.offset() == before {
            break;
        }
    }
    Attempt::Matched
}

fn matrix_column_tail(parser: &mut Parser<'_>) -> bool {
    if !base::parse_rule(parser, rules::SPACE_TAB0) {
        return false;
    }
    if !base::parse_rule(parser, rules::COMMA) && !base::parse_rule(parser, rules::BOX_VERT) {
        let _ = base::parse_rule(parser, rules::BOX_VERT_BOLD);
    }
    base::parse_rule(parser, rules::SPACE_TAB0)
}

fn matrix_row_suffix(parser: &mut Parser<'_>) {
    let _ = base::parse_rule(parser, rules::SEMICOLON);
    let _ = base::parse_rule(parser, rules::NEW_LINE);
    let border = parser.checkpoint();
    if base::parse_rule(parser, rules::BOX_DRAWING_CHAR) {
        while base::parse_rule(parser, rules::BOX_DRAWING_CHAR) {}
        if !base::parse_rule(parser, rules::NEW_LINE) {
            parser.rewind(border);
        }
    }
}

fn brace_body(parser: &mut Parser<'_>, mode: BraceMode) -> FactAttempt<ExpressionForm> {
    for parse in [
        structure_shell::parse_empty_set as fn(&mut Parser<'_>) -> Attempt,
        structure_shell::parse_empty_map,
    ] {
        match wrap_structure(parser, parse) {
            Attempt::Matched => return FactAttempt::Matched(ExpressionForm::Formula),
            Attempt::Committed => return FactAttempt::Committed,
            Attempt::NoMatch => {}
        }
    }
    match wrap_table_structure(parser) {
        Attempt::Matched => return FactAttempt::Matched(ExpressionForm::Formula),
        Attempt::Committed => return FactAttempt::Committed,
        Attempt::NoMatch => {}
    }

    brace_general(parser, mode)
}

fn brace_general(parser: &mut Parser<'_>, mode: BraceMode) -> FactAttempt<ExpressionForm> {
    let checkpoint = parser.checkpoint();
    let structure = parser.start();
    let map = parser.start();
    let set = parser.start();
    let comprehension = parser.start();
    let record = parser.start();
    if !base::parse_rule(parser, rules::LEFT_BRACE) {
        parser.rewind(checkpoint);
        return FactAttempt::NoMatch;
    }
    let Some(result) = parser.with_nesting(|parser| {
        if !base::parse_rule(parser, rules::WHITESPACE0) {
            return FactAttempt::NoMatch;
        }
        let interior = parser.checkpoint();
        match binding_with_marker(parser) {
            FactAttempt::Matched(first) => {
                return finish_shared_record_or_map(
                    parser,
                    structure,
                    map,
                    set,
                    comprehension,
                    record,
                    interior,
                    first,
                );
            }
            FactAttempt::Committed => {
                recover_closer(
                    parser,
                    rules::RECORD,
                    rules::RIGHT_BRACE,
                    SyntaxKind::RightBrace,
                    '}',
                    "}",
                );
                record.complete(parser, SyntaxKind::Record);
                finish_provisional_marker(parser, comprehension, SyntaxKind::SetComprehension);
                finish_provisional_marker(parser, set, SyntaxKind::Set);
                finish_provisional_marker(parser, map, SyntaxKind::Map);
                structure.complete(parser, SyntaxKind::Structure);
                return FactAttempt::Committed;
            }
            FactAttempt::NoMatch => finish_provisional_marker(parser, record, SyntaxKind::Record),
        }
        let entry = parser.start();
        let head_committed = match expressions::parse_expression(parser) {
            Attempt::Matched => false,
            Attempt::NoMatch => return FactAttempt::NoMatch,
            Attempt::Committed => true,
        };
        if parser.is_halted() {
            entry.complete(parser, SyntaxKind::MapEntry);
            comprehension.complete(parser, SyntaxKind::SetComprehension);
            set.complete(parser, SyntaxKind::Set);
            map.complete(parser, SyntaxKind::Map);
            structure.complete(parser, SyntaxKind::Structure);
            return FactAttempt::Committed;
        }

        let after_expression = parser.checkpoint();
        if base::parse_rule(parser, rules::SPACE_TAB0) && base::parse_rule(parser, rules::BAR) {
            if mode == BraceMode::StructureOnly {
                return FactAttempt::NoMatch;
            }
            finish_provisional_marker(parser, entry, SyntaxKind::MapEntry);
            match comprehensions::finish_qualifiers(parser, rules::RIGHT_BRACE, false) {
                Attempt::Matched => {
                    comprehension.complete(parser, SyntaxKind::SetComprehension);
                    finish_provisional_marker(parser, set, SyntaxKind::Set);
                    finish_provisional_marker(parser, map, SyntaxKind::Map);
                    finish_provisional_marker(parser, structure, SyntaxKind::Structure);
                    return if head_committed {
                        FactAttempt::Committed
                    } else {
                        FactAttempt::Matched(ExpressionForm::SetComprehension)
                    };
                }
                Attempt::NoMatch => return FactAttempt::NoMatch,
                Attempt::Committed => {
                    comprehension.complete(parser, SyntaxKind::SetComprehension);
                    finish_provisional_marker(parser, set, SyntaxKind::Set);
                    finish_provisional_marker(parser, map, SyntaxKind::Map);
                    finish_provisional_marker(parser, structure, SyntaxKind::Structure);
                    return FactAttempt::Committed;
                }
            }
        }
        parser.rewind(after_expression);

        if head_committed {
            finish_provisional_marker(parser, entry, SyntaxKind::MapEntry);
            finish_provisional_marker(parser, comprehension, SyntaxKind::SetComprehension);
            recover_closer(
                parser,
                rules::SET,
                rules::RIGHT_BRACE,
                SyntaxKind::RightBrace,
                '}',
                "}",
            );
            set.complete(parser, SyntaxKind::Set);
            finish_provisional_marker(parser, map, SyntaxKind::Map);
            structure.complete(parser, SyntaxKind::Structure);
            return FactAttempt::Committed;
        }

        if base::parse_rule(parser, rules::WHITESPACE0)
            && base::parse_rule(parser, rules::COLON)
            && base::parse_rule(parser, rules::WHITESPACE0)
        {
            let mut committed = false;
            match expressions::parse_expression(parser) {
                Attempt::Matched => {}
                Attempt::NoMatch => {
                    recover_required_production(
                        parser,
                        rules::MAP,
                        "syntax/missing-mapping-value",
                        "missing value after mapping colon",
                        "expression",
                    );
                    committed = true;
                }
                Attempt::Committed => committed = true,
            }
            if !base::parse_rule(parser, rules::WHITESPACE0) {
                return FactAttempt::NoMatch;
            }
            let _ = base::parse_rule(parser, rules::COMMA);
            if !base::parse_rule(parser, rules::WHITESPACE0) {
                return FactAttempt::NoMatch;
            }
            entry.complete(parser, SyntaxKind::MapEntry);
            finish_provisional_marker(parser, comprehension, SyntaxKind::SetComprehension);
            finish_provisional_marker(parser, set, SyntaxKind::Set);
            loop {
                let before = parser.offset();
                match parse_mapping(parser) {
                    Attempt::Matched if parser.offset() > before => {}
                    Attempt::Matched | Attempt::NoMatch => break,
                    Attempt::Committed => {
                        committed = true;
                        if !base::parse_rule(parser, rules::LIST_SEPARATOR) {
                            break;
                        }
                    }
                }
            }
            let _ = base::parse_rule(parser, rules::WHITESPACE0);
            if !base::parse_rule(parser, rules::RIGHT_BRACE) {
                recover_closer(
                    parser,
                    rules::MAP,
                    rules::RIGHT_BRACE,
                    SyntaxKind::RightBrace,
                    '}',
                    "}",
                );
                map.complete(parser, SyntaxKind::Map);
                structure.complete(parser, SyntaxKind::Structure);
                return FactAttempt::Committed;
            }
            map.complete(parser, SyntaxKind::Map);
            structure.complete(parser, SyntaxKind::Structure);
            return if committed {
                FactAttempt::Committed
            } else {
                FactAttempt::Matched(ExpressionForm::Formula)
            };
        }
        parser.rewind(after_expression);

        finish_provisional_marker(parser, entry, SyntaxKind::MapEntry);
        finish_provisional_marker(parser, comprehension, SyntaxKind::SetComprehension);
        let mut committed = false;
        loop {
            let pair = parser.checkpoint();
            if !base::parse_rule(parser, rules::LIST_SEPARATOR)
                && !base::parse_rule(parser, rules::WHITESPACE1)
            {
                break;
            }
            match expressions::parse_expression(parser) {
                Attempt::Matched => {}
                Attempt::NoMatch => {
                    parser.rewind(pair);
                    break;
                }
                Attempt::Committed => {
                    committed = true;
                }
            }
        }
        let _ = base::parse_rule(parser, rules::WHITESPACE0);
        if !base::parse_rule(parser, rules::RIGHT_BRACE) {
            recover_closer(
                parser,
                rules::SET,
                rules::RIGHT_BRACE,
                SyntaxKind::RightBrace,
                '}',
                "}",
            );
            set.complete(parser, SyntaxKind::Set);
            finish_provisional_marker(parser, map, SyntaxKind::Map);
            structure.complete(parser, SyntaxKind::Structure);
            return FactAttempt::Committed;
        }
        set.complete(parser, SyntaxKind::Set);
        finish_provisional_marker(parser, map, SyntaxKind::Map);
        structure.complete(parser, SyntaxKind::Structure);
        if committed {
            FactAttempt::Committed
        } else {
            FactAttempt::Matched(ExpressionForm::Formula)
        }
    }) else {
        nesting_limit(parser);
        record.complete(parser, SyntaxKind::Record);
        finish_provisional_marker(parser, comprehension, SyntaxKind::SetComprehension);
        set.complete(parser, SyntaxKind::Set);
        finish_provisional_marker(parser, map, SyntaxKind::Map);
        structure.complete(parser, SyntaxKind::Structure);
        return FactAttempt::Committed;
    };
    if result == FactAttempt::NoMatch {
        parser.rewind(checkpoint);
    }
    result
}

fn finish_shared_record_or_map(
    parser: &mut Parser<'_>,
    structure: super::super::super::marker::Marker,
    map: super::super::super::marker::Marker,
    set: super::super::super::marker::Marker,
    comprehension: super::super::super::marker::Marker,
    record: super::super::super::marker::Marker,
    interior: ParserCheckpoint,
    first: BindingCandidate,
) -> FactAttempt<ExpressionForm> {
    let mut bindings = alloc::vec![first];
    loop {
        match binding_with_marker(parser) {
            FactAttempt::Matched(binding) => bindings.push(binding),
            FactAttempt::Committed => {
                recover_closer(
                    parser,
                    rules::RECORD,
                    rules::RIGHT_BRACE,
                    SyntaxKind::RightBrace,
                    '}',
                    "}",
                );
                record.complete(parser, SyntaxKind::Record);
                finish_provisional_marker(parser, comprehension, SyntaxKind::SetComprehension);
                finish_provisional_marker(parser, set, SyntaxKind::Set);
                finish_provisional_marker(parser, map, SyntaxKind::Map);
                structure.complete(parser, SyntaxKind::Structure);
                return FactAttempt::Committed;
            }
            FactAttempt::NoMatch => break,
        }
    }

    let close = parser.checkpoint();
    if base::parse_rule(parser, rules::WHITESPACE0) && base::parse_rule(parser, rules::RIGHT_BRACE)
    {
        record.complete(parser, SyntaxKind::Record);
        finish_provisional_marker(parser, comprehension, SyntaxKind::SetComprehension);
        finish_provisional_marker(parser, set, SyntaxKind::Set);
        finish_provisional_marker(parser, map, SyntaxKind::Map);
        structure.complete(parser, SyntaxKind::Structure);
        return FactAttempt::Matched(ExpressionForm::Formula);
    }
    parser.rewind(close);

    let cached_values = bindings
        .iter()
        .map(|binding| parser.cache_clean_subtree(binding.value_start, binding.value_end))
        .collect::<Option<alloc::vec::Vec<_>>>();

    parser.rewind(interior);
    finish_provisional_marker(parser, record, SyntaxKind::Record);
    finish_provisional_marker(parser, comprehension, SyntaxKind::SetComprehension);
    finish_provisional_marker(parser, set, SyntaxKind::Set);
    let mut committed = false;

    if let Some(cached_values) = cached_values.as_ref() {
        for cached_value in cached_values {
            match parse_mapping_with_cached_value(parser, Some(cached_value)) {
                Attempt::Matched => {}
                Attempt::NoMatch => return FactAttempt::NoMatch,
                Attempt::Committed => {
                    committed = true;
                    let _ = base::parse_rule(parser, rules::LIST_SEPARATOR);
                }
            }
        }
    } else {
        match parse_mapping(parser) {
            Attempt::Matched => {}
            Attempt::NoMatch => return FactAttempt::NoMatch,
            Attempt::Committed => {
                committed = true;
                let _ = base::parse_rule(parser, rules::LIST_SEPARATOR);
            }
        }
    }
    loop {
        let before = parser.offset();
        match parse_mapping(parser) {
            Attempt::Matched if parser.offset() > before => {}
            Attempt::Matched | Attempt::NoMatch => break,
            Attempt::Committed => {
                committed = true;
                if !base::parse_rule(parser, rules::LIST_SEPARATOR) {
                    break;
                }
            }
        }
    }
    if !base::parse_rule(parser, rules::WHITESPACE0) {
        return FactAttempt::NoMatch;
    }
    if !base::parse_rule(parser, rules::RIGHT_BRACE) {
        if !committed {
            return FactAttempt::NoMatch;
        }
        recover_closer(
            parser,
            rules::MAP,
            rules::RIGHT_BRACE,
            SyntaxKind::RightBrace,
            '}',
            "}",
        );
    }
    map.complete(parser, SyntaxKind::Map);
    structure.complete(parser, SyntaxKind::Structure);
    if committed {
        FactAttempt::Committed
    } else {
        FactAttempt::Matched(ExpressionForm::Formula)
    }
}

fn structure_body(parser: &mut Parser<'_>) -> Attempt {
    let empty_set = structure_shell::parse_empty_set(parser);
    if empty_set != Attempt::NoMatch {
        return empty_set;
    }
    let empty_map = structure_shell::parse_empty_map(parser);
    if empty_map != Attempt::NoMatch {
        return empty_map;
    }
    choice(
        parser,
        &[
            parse_table,
            parse_matrix,
            parse_tuple,
            parse_tuple_struct,
            parse_record,
            parse_map,
            parse_set,
        ],
    )
}

fn wrap_structure(parser: &mut Parser<'_>, parse: fn(&mut Parser<'_>) -> Attempt) -> Attempt {
    let checkpoint = parser.checkpoint();
    let node = parser.start();
    match parse(parser) {
        Attempt::Matched => {
            node.complete(parser, SyntaxKind::Structure);
            Attempt::Matched
        }
        Attempt::Committed => {
            node.complete(parser, SyntaxKind::Structure);
            Attempt::Committed
        }
        Attempt::NoMatch => {
            parser.rewind(checkpoint);
            Attempt::NoMatch
        }
    }
}

fn wrap_table_structure(parser: &mut Parser<'_>) -> Attempt {
    let checkpoint = parser.checkpoint();
    let structure = parser.start();
    let table = parse_table(parser);
    match table {
        Attempt::Matched => {
            structure.complete(parser, SyntaxKind::Structure);
            Attempt::Matched
        }
        Attempt::Committed => {
            structure.complete(parser, SyntaxKind::Structure);
            Attempt::Committed
        }
        Attempt::NoMatch => {
            parser.rewind(checkpoint);
            Attempt::NoMatch
        }
    }
}

fn delimited_repeated(
    parser: &mut Parser<'_>,
    rule: RuleId,
    kind: SyntaxKind,
    open: RuleId,
    close: RuleId,
    item: fn(&mut Parser<'_>) -> Attempt,
) -> Attempt {
    combinator::transactional(parser, rule, |parser| {
        let node = parser.start();
        if !base::parse_rule(parser, open) {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let Some(interior) = parser.with_nesting(|parser| {
            if !base::parse_rule(parser, rules::WHITESPACE0) {
                return Attempt::NoMatch;
            }
            let mut parsed_any = false;
            let mut committed = false;
            while !parser.is_halted() {
                let before = parser.offset();
                match item(parser) {
                    Attempt::Matched if parser.offset() > before => parsed_any = true,
                    Attempt::Matched => break,
                    Attempt::NoMatch if !parsed_any => return Attempt::NoMatch,
                    Attempt::NoMatch => break,
                    Attempt::Committed => {
                        parsed_any = true;
                        committed = true;
                        if !base::parse_rule(parser, rules::LIST_SEPARATOR) {
                            break;
                        }
                    }
                }
            }
            let _ = base::parse_rule(parser, rules::WHITESPACE0);
            if base::parse_rule(parser, close) {
                if committed {
                    Attempt::Committed
                } else {
                    Attempt::Matched
                }
            } else {
                recover_closer(parser, rule, close, SyntaxKind::RightBrace, '}', "}")
            }
        }) else {
            let result = nesting_limit(parser);
            node.complete(parser, kind);
            return result;
        };
        finish(node, parser, kind, interior)
    })
}

fn inline_table_item(parser: &mut Parser<'_>) -> Attempt {
    let checkpoint = parser.checkpoint();
    if !base::parse_rule(parser, rules::SPACE_TAB0) {
        parser.rewind(checkpoint);
        return Attempt::NoMatch;
    }
    match expressions::parse_expression(parser) {
        Attempt::Matched => Attempt::Matched,
        Attempt::Committed => Attempt::Committed,
        Attempt::NoMatch => {
            parser.rewind(checkpoint);
            Attempt::NoMatch
        }
    }
}

fn header_list(
    parser: &mut Parser<'_>,
    rule: RuleId,
    kind: SyntaxKind,
    trailing_whitespace: bool,
) -> Attempt {
    combinator::transactional(parser, rule, |parser| {
        let node = parser.start();
        let first = parse_header_field(parser);
        match first {
            Attempt::Matched => {}
            Attempt::NoMatch => {
                node.abandon(parser);
                return Attempt::NoMatch;
            }
            Attempt::Committed => {
                recover_table_separator(parser, rule);
                node.complete(parser, kind);
                return Attempt::Committed;
            }
        }
        loop {
            let pair = parser.checkpoint();
            if !base::parse_rule(parser, rules::SPACE_TAB1) {
                break;
            }
            match parse_header_field(parser) {
                Attempt::Matched => {}
                Attempt::NoMatch => {
                    parser.rewind(pair);
                    break;
                }
                Attempt::Committed => {
                    recover_table_separator(parser, rule);
                    node.complete(parser, kind);
                    return Attempt::Committed;
                }
            }
        }
        let _ = base::parse_rule(parser, rules::SPACE_TAB0);
        if structure_shell::parse_table_separator(parser) != Attempt::Matched {
            recover_table_separator(parser, rule);
            node.complete(parser, kind);
            return Attempt::Committed;
        }
        let _ = if trailing_whitespace {
            base::parse_rule(parser, rules::WHITESPACE0)
        } else {
            base::parse_rule(parser, rules::SPACE_TAB0)
        };
        node.complete(parser, kind);
        Attempt::Matched
    })
}

fn spaced_table_row(parser: &mut Parser<'_>, rule: RuleId, kind: SyntaxKind) -> Attempt {
    combinator::transactional(parser, rule, |parser| {
        let node = parser.start();
        if structure_shell::parse_table_separator(parser) != Attempt::Matched {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let after_separator = parser.checkpoint();
        let _ = base::parse_rule(parser, rules::SPACE_TAB0);
        let row_is_empty = parser.cursor().starts_with("\n") || parser.cursor().starts_with("\r");
        parser.rewind(after_separator);
        if row_is_empty {
            missing_production(
                parser,
                "syntax/missing-table-cell",
                "missing table row cell after separator",
                "expression",
            );
            recover_table_separator(parser, rule);
            node.complete(parser, kind);
            return Attempt::Committed;
        }
        let cell = parser.checkpoint();
        let first = expressions::parse_expression(parser);
        match first {
            Attempt::Matched => {}
            Attempt::NoMatch => {
                parser.rewind(cell);
                recover_required_production(
                    parser,
                    rule,
                    "syntax/missing-table-cell",
                    "missing table row cell after separator",
                    "expression",
                );
                recover_table_separator(parser, rule);
                node.complete(parser, kind);
                return Attempt::Committed;
            }
            Attempt::Committed => {
                recover_table_separator(parser, rule);
                node.complete(parser, kind);
                return Attempt::Committed;
            }
        }
        loop {
            let pair = parser.checkpoint();
            if !base::parse_rule(parser, rules::SPACE_TAB1) {
                break;
            }
            match expressions::parse_expression(parser) {
                Attempt::Matched => {}
                Attempt::NoMatch => {
                    parser.rewind(pair);
                    break;
                }
                Attempt::Committed => {
                    recover_table_separator(parser, rule);
                    node.complete(parser, kind);
                    return Attempt::Committed;
                }
            }
        }
        let _ = base::parse_rule(parser, rules::SPACE_TAB0);
        if structure_shell::parse_table_separator(parser) != Attempt::Matched {
            recover_table_separator(parser, rule);
            node.complete(parser, kind);
            return Attempt::Committed;
        }
        let _ = base::parse_rule(parser, rules::SPACE_TAB0);
        node.complete(parser, kind);
        Attempt::Matched
    })
}

fn recover_table_separator(parser: &mut Parser<'_>, target: RuleId) -> Attempt {
    recover_closer_set(
        parser,
        target,
        &['|', '│', '┃', '\n', '\r', ')', ']', '}'],
        SyntaxKind::Bar,
        "|",
        |parser| structure_shell::parse_table_separator(parser) == Attempt::Matched,
    )
}

fn recover_table_end(
    parser: &mut Parser<'_>,
    target: RuleId,
    delimiter: TableDelimiter,
) -> Attempt {
    let (kind, fix) = match delimiter {
        TableDelimiter::Brace => (SyntaxKind::RightBrace, "}"),
        TableDelimiter::Bar => (SyntaxKind::Bar, "|"),
        TableDelimiter::Box => (SyntaxKind::BoxDrawing, "╯"),
    };
    recover_closer_set(
        parser,
        target,
        &['}', '|', '╯', '┘', '┛', ')', ']'],
        kind,
        fix,
        |parser| structure_shell::parse_table_end(parser) == Attempt::Matched,
    )
}

fn fancy_row(parser: &mut Parser<'_>) -> Attempt {
    let row = parse_table_row2(parser);
    if row == Attempt::NoMatch {
        structure_shell::parse_row_separator(parser)
    } else {
        row
    }
}

fn choice(parser: &mut Parser<'_>, choices: &[fn(&mut Parser<'_>) -> Attempt]) -> Attempt {
    for parse in choices {
        let result = parse(parser);
        if result != Attempt::NoMatch {
            return result;
        }
    }
    Attempt::NoMatch
}

fn ahead(parser: &mut Parser<'_>, parse: fn(&mut Parser<'_>) -> Attempt) -> bool {
    let checkpoint = parser.checkpoint();
    let matched = parse(parser).accepted();
    parser.rewind(checkpoint);
    matched
}

fn has_mapping_separator(parser: &Parser<'_>) -> bool {
    let mut cursor = parser.cursor().clone();
    let mut delimiters = alloc::vec::Vec::new();
    let mut quoted = false;
    let mut raw_triple = false;
    let mut escaped = false;
    let mut key_started = false;

    while let Some(character) = cursor.peek_char() {
        if !quoted && cursor.starts_with("\"\"\"") {
            for _ in 0..3 {
                let _ = cursor.bump_char();
            }
            raw_triple = !raw_triple;
            if delimiters.is_empty() {
                key_started = true;
            }
            continue;
        }
        if raw_triple {
            let _ = cursor.bump_char();
            continue;
        }
        if quoted {
            let _ = cursor.bump_char();
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                quoted = false;
            }
            continue;
        }
        match character {
            '"' => {
                quoted = true;
                if delimiters.is_empty() {
                    key_started = true;
                }
            }
            '(' | '[' | '{' => {
                if delimiters.is_empty() {
                    key_started = true;
                }
                delimiters.push(character);
            }
            ')' | ']' | '}' => {
                if delimiters.is_empty() {
                    return false;
                }
                delimiters.pop();
            }
            ':' if delimiters.is_empty() && key_started => return true,
            ':' if delimiters.is_empty() => key_started = true,
            ',' if delimiters.is_empty() => return false,
            character if delimiters.is_empty() && !character.is_whitespace() => {
                key_started = true;
            }
            _ => {}
        }
        let _ = cursor.bump_char();
    }
    false
}

fn finish_provisional_marker(
    parser: &mut Parser<'_>,
    marker: super::super::super::marker::Marker,
    kind: SyntaxKind,
) {
    if parser.is_halted() {
        marker.complete(parser, kind);
    } else {
        marker.abandon(parser);
    }
}

fn finish(
    node: super::super::super::marker::Marker,
    parser: &mut Parser<'_>,
    kind: SyntaxKind,
    result: Attempt,
) -> Attempt {
    if parser.is_halted() {
        node.complete(parser, kind);
        return Attempt::Committed;
    }
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
