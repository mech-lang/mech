use crate::document::{RuleId, SyntaxKind};

use super::super::super::rule::rules;
use super::super::super::{CleanSubtree, Parser, ParserCheckpoint};
use super::super::{base, combinator, structure_shell};
use super::{
    Attempt, BracketForm, ExpressionForm, FactAttempt, child_result, comprehensions, expressions,
    kinds, nesting_limit, transactional_fact,
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
        let first = parse_matrix_column(parser);
        if let Some(result) = child_result(parser, node, SyntaxKind::MatrixRow, first) {
            return result;
        }
        loop {
            let before = parser.offset();
            match parse_matrix_column(parser) {
                Attempt::Matched if parser.offset() > before => {}
                Attempt::Matched | Attempt::NoMatch => break,
                Attempt::Committed => {
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
        let child = parse_fancy_table_header(parser);
        if let Some(result) = child_result(parser, node, SyntaxKind::FancyTable, child) {
            return result;
        }
        let first = fancy_row(parser);
        if let Some(result) = child_result(parser, node, SyntaxKind::FancyTable, first) {
            return result;
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
                    node.complete(parser, SyntaxKind::FancyTable);
                    return Attempt::Committed;
                }
            }
        }
        node.complete(parser, SyntaxKind::FancyTable);
        Attempt::Matched
    })
}

pub(super) fn parse_fancy_table_header(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::FANCY_TABLE_HEADER, |parser| {
        let node = parser.start();
        let first = parse_field(parser);
        if let Some(result) = child_result(parser, node, SyntaxKind::FancyTableHeader, first) {
            return result;
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
                    node.complete(parser, SyntaxKind::FancyTableHeader);
                    return Attempt::Committed;
                }
            }
        }
        if structure_shell::parse_table_separator(parser) != Attempt::Matched
            || !base::parse_rule(parser, rules::WHITESPACE0)
        {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
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
        if let Some(result) = child_result(parser, node, SyntaxKind::InlineTable, child) {
            return result;
        }
        if !base::parse_rule(parser, rules::SPACE_TAB0) {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let child = parse_inline_table_row(parser);
        if let Some(result) = child_result(parser, node, SyntaxKind::InlineTable, child) {
            return result;
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
        if let Some(result) = child_result(parser, node, SyntaxKind::InlineTableRow, first) {
            return result;
        }
        loop {
            let before = parser.offset();
            match inline_table_item(parser) {
                Attempt::Matched if parser.offset() > before => {}
                Attempt::Matched | Attempt::NoMatch => break,
                Attempt::Committed => {
                    node.complete(parser, SyntaxKind::InlineTableRow);
                    return Attempt::Committed;
                }
            }
        }
        if !base::parse_rule(parser, rules::SPACE_TAB0)
            || structure_shell::parse_table_separator(parser) != Attempt::Matched
        {
            node.abandon(parser);
            return Attempt::NoMatch;
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
        let child = parse_table_header(parser);
        if let Some(result) = child_result(parser, node, SyntaxKind::RegularTable, child) {
            return result;
        }
        let first = parse_table_row(parser);
        if let Some(result) = child_result(parser, node, SyntaxKind::RegularTable, first) {
            return result;
        }
        loop {
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
                    node.complete(parser, SyntaxKind::RegularTable);
                    return Attempt::Committed;
                }
            }
        }
        node.complete(parser, SyntaxKind::RegularTable);
        Attempt::Matched
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
        if let Some(result) = child_result(parser, node, SyntaxKind::FancyTableRow, first) {
            return result;
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
                    node.complete(parser, SyntaxKind::FancyTableRow);
                    return Attempt::Committed;
                }
            }
        }
        if !base::parse_rule(parser, rules::SPACE_TAB0)
            || structure_shell::parse_table_separator(parser) != Attempt::Matched
            || !base::parse_rule(parser, rules::SPACE_TAB0)
        {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
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
        if let Some(result) = child_result(parser, node, SyntaxKind::MapEntry, child) {
            return result;
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
        if structure_shell::parse_table_start(parser) != Attempt::Matched {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let Some(interior) = parser.with_nesting(|parser| {
            if !base::parse_rule(parser, rules::WHITESPACE0) {
                return Attempt::NoMatch;
            }
            let first = parse_binding(parser);
            if first != Attempt::Matched {
                return first;
            }
            loop {
                let before = parser.offset();
                match parse_binding(parser) {
                    Attempt::Matched if parser.offset() > before => {}
                    Attempt::Matched | Attempt::NoMatch => break,
                    Attempt::Committed => return Attempt::Committed,
                }
            }
            if !base::parse_rule(parser, rules::WHITESPACE0) {
                return Attempt::NoMatch;
            }
            structure_shell::parse_table_end(parser)
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

fn binding_with_marker(
    parser: &mut Parser<'_>,
) -> FactAttempt<BindingCandidate> {
    transactional_fact(parser, rules::BINDING, |parser| {
        let node = parser.start();
        if !base::parse_rule(parser, rules::WHITESPACE0)
            || !base::parse_rule(parser, rules::IDENTIFIER)
        {
            node.abandon(parser);
            return FactAttempt::NoMatch;
        }
        if kinds::parse_kind_annotation(parser) == Attempt::Committed {
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
                node.abandon(parser);
                return FactAttempt::NoMatch;
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
            let first = expressions::parse_expression(parser);
            if first != Attempt::Matched {
                return first;
            }
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
                    Attempt::Committed => return Attempt::Committed,
                }
            }
            if !base::parse_rule(parser, rules::WHITESPACE0)
                || !base::parse_rule(parser, rules::RIGHT_BRACE)
            {
                Attempt::NoMatch
            } else {
                Attempt::Matched
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
            let first = expressions::parse_expression(parser);
            if first != Attempt::Matched {
                return first;
            }
            loop {
                let pair = parser.checkpoint();
                if !base::parse_rule(parser, rules::LIST_SEPARATOR) {
                    break;
                }
                match expressions::parse_expression(parser) {
                    Attempt::Matched => {}
                    Attempt::NoMatch => {
                        parser.rewind(pair);
                        break;
                    }
                    Attempt::Committed => return Attempt::Committed,
                }
            }
            if !base::parse_rule(parser, rules::WHITESPACE0)
                || !base::parse_rule(parser, rules::RIGHT_PARENTHESIS)
            {
                Attempt::NoMatch
            } else {
                Attempt::Matched
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
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let Some(interior) = parser.with_nesting(|parser| {
            if !base::parse_rule(parser, rules::WHITESPACE0) {
                return Attempt::NoMatch;
            }
            let value = expressions::parse_expression(parser);
            if value != Attempt::Matched {
                return value;
            }
            if !base::parse_rule(parser, rules::WHITESPACE0)
                || !base::parse_rule(parser, rules::RIGHT_PARENTHESIS)
            {
                Attempt::NoMatch
            } else {
                Attempt::Matched
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
            parenthetical.abandon(parser);
            tuple.complete(parser, SyntaxKind::Tuple);
            structure.complete(parser, SyntaxKind::Structure);
            return Attempt::Matched;
        }
        parser.rewind(after_open);

        if !base::parse_rule(parser, rules::SPACE_TAB0) {
            return Attempt::NoMatch;
        }
        let expression = parser.start();
        let mut body = expressions::expression_body(parser);
        if body == FactAttempt::NoMatch {
            parser.rewind(after_open);
            if !base::parse_rule(parser, rules::WHITESPACE0) {
                return Attempt::NoMatch;
            }
            let expression = parser.start();
            body = expressions::expression_body(parser);
            match body {
                FactAttempt::Matched(_) => expression.complete(parser, SyntaxKind::Expression),
                FactAttempt::NoMatch => return Attempt::NoMatch,
                FactAttempt::Committed => {
                    expression.complete(parser, SyntaxKind::Expression);
                    parenthetical.complete(parser, SyntaxKind::ParentheticalExpression);
                    tuple.complete(parser, SyntaxKind::Tuple);
                    structure.complete(parser, SyntaxKind::Structure);
                    return Attempt::Committed;
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
                        tuple.abandon(parser);
                        structure.abandon(parser);
                        return Attempt::Matched;
                    }
                    parser.rewind(after_body);
                    expression.complete(parser, SyntaxKind::Expression);
                }
                FactAttempt::Matched(_) => {
                    expression.complete(parser, SyntaxKind::Expression);
                }
                FactAttempt::Committed => {
                    expression.complete(parser, SyntaxKind::Expression);
                    parenthetical.complete(parser, SyntaxKind::ParentheticalExpression);
                    tuple.complete(parser, SyntaxKind::Tuple);
                    structure.complete(parser, SyntaxKind::Structure);
                    return Attempt::Committed;
                }
                FactAttempt::NoMatch => return Attempt::NoMatch,
            }
        }

        parenthetical.abandon(parser);
        loop {
            let item = parser.checkpoint();
            if !base::parse_rule(parser, rules::LIST_SEPARATOR) {
                break;
            }
            match expressions::parse_expression(parser) {
                Attempt::Matched => {}
                Attempt::NoMatch => {
                    parser.rewind(item);
                    break;
                }
                Attempt::Committed => {
                    tuple.complete(parser, SyntaxKind::Tuple);
                    structure.complete(parser, SyntaxKind::Structure);
                    return Attempt::Committed;
                }
            }
        }
        if !base::parse_rule(parser, rules::WHITESPACE0)
            || !base::parse_rule(parser, rules::RIGHT_PARENTHESIS)
        {
            return Attempt::NoMatch;
        }
        tuple.complete(parser, SyntaxKind::Tuple);
        structure.complete(parser, SyntaxKind::Structure);
        Attempt::Matched
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
                match expressions::parse_expression(parser) {
                    Attempt::Matched => {
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
                                    FactAttempt::Matched(BracketForm::Comprehension)
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
                                finish_provisional_marker(
                                    parser,
                                    comprehension,
                                    SyntaxKind::MatrixComprehension,
                                );
                                matrix.complete(parser, SyntaxKind::Matrix);
                                return FactAttempt::Committed;
                            }
                        }
                        return finish_matrix_body(parser, matrix, comprehension);
                    }
                    Attempt::NoMatch => parser.rewind(after_open),
                    Attempt::Committed => {
                        column.complete(parser, SyntaxKind::MatrixColumn);
                        row.complete(parser, SyntaxKind::MatrixRow);
                        finish_provisional_marker(
                            parser,
                            comprehension,
                            SyntaxKind::MatrixComprehension,
                        );
                        matrix.complete(parser, SyntaxKind::Matrix);
                        return FactAttempt::Committed;
                    }
                }
            } else {
                parser.rewind(after_open);
            }

            if mode == BracketMode::ComprehensionOnly {
                return FactAttempt::NoMatch;
            }
            finish_matrix_body(parser, matrix, comprehension)
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
) -> FactAttempt<BracketForm> {
    loop {
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
            Attempt::Matched | Attempt::NoMatch => return FactAttempt::NoMatch,
            Attempt::Committed => {
                finish_provisional_marker(parser, comprehension, SyntaxKind::MatrixComprehension);
                matrix.complete(parser, SyntaxKind::Matrix);
                return FactAttempt::Committed;
            }
        }
    }
    if !base::parse_rule(parser, rules::WHITESPACE0)
        || structure_shell::parse_matrix_end(parser) != Attempt::Matched
    {
        return FactAttempt::NoMatch;
    }
    comprehension.abandon(parser);
    matrix.complete(parser, SyntaxKind::Matrix);
    FactAttempt::Matched(BracketForm::Matrix)
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
                record.complete(parser, SyntaxKind::Record);
                comprehension.complete(parser, SyntaxKind::SetComprehension);
                set.complete(parser, SyntaxKind::Set);
                map.complete(parser, SyntaxKind::Map);
                structure.complete(parser, SyntaxKind::Structure);
                return FactAttempt::Committed;
            }
            FactAttempt::NoMatch => record.abandon(parser),
        }
        let entry = parser.start();
        match expressions::parse_expression(parser) {
            Attempt::Matched => {}
            Attempt::NoMatch => return FactAttempt::NoMatch,
            Attempt::Committed => {
                entry.complete(parser, SyntaxKind::MapEntry);
                comprehension.complete(parser, SyntaxKind::SetComprehension);
                set.complete(parser, SyntaxKind::Set);
                map.complete(parser, SyntaxKind::Map);
                structure.complete(parser, SyntaxKind::Structure);
                return FactAttempt::Committed;
            }
        }

        let after_expression = parser.checkpoint();
        if base::parse_rule(parser, rules::SPACE_TAB0) && base::parse_rule(parser, rules::BAR) {
            if mode == BraceMode::StructureOnly {
                return FactAttempt::NoMatch;
            }
            entry.abandon(parser);
            match comprehensions::finish_qualifiers(parser, rules::RIGHT_BRACE, false) {
                Attempt::Matched => {
                    comprehension.complete(parser, SyntaxKind::SetComprehension);
                    set.abandon(parser);
                    map.abandon(parser);
                    structure.abandon(parser);
                    return FactAttempt::Matched(ExpressionForm::SetComprehension);
                }
                Attempt::NoMatch => return FactAttempt::NoMatch,
                Attempt::Committed => {
                    comprehension.complete(parser, SyntaxKind::SetComprehension);
                    set.complete(parser, SyntaxKind::Set);
                    map.complete(parser, SyntaxKind::Map);
                    structure.complete(parser, SyntaxKind::Structure);
                    return FactAttempt::Committed;
                }
            }
        }
        parser.rewind(after_expression);

        if base::parse_rule(parser, rules::WHITESPACE0)
            && base::parse_rule(parser, rules::COLON)
            && base::parse_rule(parser, rules::WHITESPACE0)
        {
            match expressions::parse_expression(parser) {
                Attempt::Matched => {}
                Attempt::NoMatch => return FactAttempt::NoMatch,
                Attempt::Committed => {
                    entry.complete(parser, SyntaxKind::MapEntry);
                    comprehension.complete(parser, SyntaxKind::SetComprehension);
                    set.complete(parser, SyntaxKind::Set);
                    map.complete(parser, SyntaxKind::Map);
                    structure.complete(parser, SyntaxKind::Structure);
                    return FactAttempt::Committed;
                }
            }
            if !base::parse_rule(parser, rules::WHITESPACE0) {
                return FactAttempt::NoMatch;
            }
            let _ = base::parse_rule(parser, rules::COMMA);
            if !base::parse_rule(parser, rules::WHITESPACE0) {
                return FactAttempt::NoMatch;
            }
            entry.complete(parser, SyntaxKind::MapEntry);
            comprehension.abandon(parser);
            set.abandon(parser);
            loop {
                let before = parser.offset();
                match parse_mapping(parser) {
                    Attempt::Matched if parser.offset() > before => {}
                    Attempt::Matched | Attempt::NoMatch => break,
                    Attempt::Committed => {
                        map.complete(parser, SyntaxKind::Map);
                        structure.complete(parser, SyntaxKind::Structure);
                        return FactAttempt::Committed;
                    }
                }
            }
            if !base::parse_rule(parser, rules::WHITESPACE0)
                || !base::parse_rule(parser, rules::RIGHT_BRACE)
            {
                return FactAttempt::NoMatch;
            }
            map.complete(parser, SyntaxKind::Map);
            structure.complete(parser, SyntaxKind::Structure);
            return FactAttempt::Matched(ExpressionForm::Formula);
        }
        parser.rewind(after_expression);

        entry.abandon(parser);
        comprehension.abandon(parser);
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
                    set.complete(parser, SyntaxKind::Set);
                    map.complete(parser, SyntaxKind::Map);
                    structure.complete(parser, SyntaxKind::Structure);
                    return FactAttempt::Committed;
                }
            }
        }
        if !base::parse_rule(parser, rules::WHITESPACE0)
            || !base::parse_rule(parser, rules::RIGHT_BRACE)
        {
            return FactAttempt::NoMatch;
        }
        set.complete(parser, SyntaxKind::Set);
        map.abandon(parser);
        structure.complete(parser, SyntaxKind::Structure);
        FactAttempt::Matched(ExpressionForm::Formula)
    }) else {
        nesting_limit(parser);
        record.complete(parser, SyntaxKind::Record);
        comprehension.abandon(parser);
        set.complete(parser, SyntaxKind::Set);
        map.abandon(parser);
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
        comprehension.abandon(parser);
        set.abandon(parser);
        map.abandon(parser);
        structure.complete(parser, SyntaxKind::Structure);
        return FactAttempt::Matched(ExpressionForm::Formula);
    }
    parser.rewind(close);

    let cached_values = bindings
        .iter()
        .map(|binding| parser.cache_clean_subtree(binding.value_start, binding.value_end))
        .collect::<Option<alloc::vec::Vec<_>>>();

    parser.rewind(interior);
    record.abandon(parser);
    comprehension.abandon(parser);
    set.abandon(parser);

    if let Some(cached_values) = cached_values.as_ref() {
        for cached_value in cached_values {
            match parse_mapping_with_cached_value(parser, Some(cached_value)) {
                Attempt::Matched => {}
                Attempt::NoMatch => return FactAttempt::NoMatch,
                Attempt::Committed => {
                    map.complete(parser, SyntaxKind::Map);
                    structure.complete(parser, SyntaxKind::Structure);
                    return FactAttempt::Committed;
                }
            }
        }
    } else {
        match parse_mapping(parser) {
            Attempt::Matched => {}
            Attempt::NoMatch => return FactAttempt::NoMatch,
            Attempt::Committed => {
                map.complete(parser, SyntaxKind::Map);
                structure.complete(parser, SyntaxKind::Structure);
                return FactAttempt::Committed;
            }
        }
    }
    loop {
        let before = parser.offset();
        match parse_mapping(parser) {
            Attempt::Matched if parser.offset() > before => {}
            Attempt::Matched | Attempt::NoMatch => break,
            Attempt::Committed => {
                map.complete(parser, SyntaxKind::Map);
                structure.complete(parser, SyntaxKind::Structure);
                return FactAttempt::Committed;
            }
        }
    }
    if !base::parse_rule(parser, rules::WHITESPACE0)
        || !base::parse_rule(parser, rules::RIGHT_BRACE)
    {
        return FactAttempt::NoMatch;
    }
    map.complete(parser, SyntaxKind::Map);
    structure.complete(parser, SyntaxKind::Structure);
    FactAttempt::Matched(ExpressionForm::Formula)
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
            let first = item(parser);
            if first != Attempt::Matched {
                return first;
            }
            loop {
                let before = parser.offset();
                match item(parser) {
                    Attempt::Matched if parser.offset() > before => {}
                    Attempt::Matched | Attempt::NoMatch => break,
                    Attempt::Committed => return Attempt::Committed,
                }
            }
            if !base::parse_rule(parser, rules::WHITESPACE0) || !base::parse_rule(parser, close) {
                Attempt::NoMatch
            } else {
                Attempt::Matched
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
        if let Some(result) = child_result(parser, node, kind, first) {
            return result;
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
                    node.complete(parser, kind);
                    return Attempt::Committed;
                }
            }
        }
        if !base::parse_rule(parser, rules::SPACE_TAB0)
            || structure_shell::parse_table_separator(parser) != Attempt::Matched
            || !(if trailing_whitespace {
                base::parse_rule(parser, rules::WHITESPACE0)
            } else {
                base::parse_rule(parser, rules::SPACE_TAB0)
            })
        {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
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
        let first = expressions::parse_expression(parser);
        if let Some(result) = child_result(parser, node, kind, first) {
            return result;
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
                    node.complete(parser, kind);
                    return Attempt::Committed;
                }
            }
        }
        if !base::parse_rule(parser, rules::SPACE_TAB0)
            || structure_shell::parse_table_separator(parser) != Attempt::Matched
            || !base::parse_rule(parser, rules::SPACE_TAB0)
        {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        node.complete(parser, kind);
        Attempt::Matched
    })
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
