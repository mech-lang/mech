//! Canonical grammar-metalanguage productions.

use alloc::string::{String, ToString};

use crate::document::{
    Diagnostic, DiagnosticAnchor, DiagnosticCode, DiagnosticPhase, DiagnosticTags, ExpectedSyntax,
    FoundSyntax, NodeFlags, RecoveryAction, Severity, SyntaxKind, TextRange, TextSize,
};

use super::super::Parser;
use super::super::rule::rules;
use super::combinator::{
    Attempt, consume_define_operator, consume_grammar_ignored_trivia, consume_logical_grapheme,
    consume_rule_literal, emit_synthetic_final_newline, insert_missing, is_grammar_ignored,
    logical_starts_with, peek_logical_char, transactional,
};

pub(crate) fn parse_grammar(parser: &mut Parser<'_>) -> bool {
    parser.with_canonical_rule(rules::GRAMMAR, |parser| {
        let grammar = parser.start();
        consume_grammar_ignored_trivia(parser);

        let mut rule_count = 0_usize;
        while !parser.is_eof() && !parser.is_halted() {
            if parse_grammar_rule(parser).accepted() {
                rule_count = rule_count.saturating_add(1);
                consume_grammar_ignored_trivia(parser);
                continue;
            }

            if recover_unexpected(
                parser,
                "syntax/unexpected-grammar-token",
                "unexpected source while parsing a grammar rule",
                true,
            ) {
                consume_grammar_ignored_trivia(parser);
                continue;
            }
            break;
        }

        if rule_count == 0 && !parser.is_halted() {
            insert_missing(
                parser,
                "syntax/missing-grammar-rule",
                "expected at least one grammar rule",
                ExpectedSyntax::Production(String::from("grammar-rule")),
                None,
                None,
            );
        }

        parser.with_canonical_rule(rules::NEW_LINE, |parser| {
            emit_synthetic_final_newline(parser);
        });
        grammar.complete_with_flags(parser, SyntaxKind::Grammar, NodeFlags::REPARSE_ROOT);
        true
    })
}

pub(crate) fn parse_grammar_rule(parser: &mut Parser<'_>) -> Attempt {
    transactional(parser, rules::GRAMMAR_RULE, |parser| {
        let rule = parser.start();
        consume_grammar_ignored_trivia(parser);
        if !parse_grammar_identifier(parser) {
            return Attempt::NoMatch;
        }

        consume_grammar_ignored_trivia(parser);
        if !consume_define_operator(parser) && !parser.is_halted() {
            insert_missing(
                parser,
                "syntax/missing-define-operator",
                "expected `:=` after the grammar rule name",
                ExpectedSyntax::Token(SyntaxKind::DefineOperatorToken),
                Some(SyntaxKind::DefineOperatorToken),
                Some(":="),
            );
        }

        consume_grammar_ignored_trivia(parser);
        if !parse_grammar_expression(parser).accepted() && !parser.is_halted() {
            if logical_starts_with(parser, ";") || peek_logical_char(parser).is_none() {
                parser.with_canonical_rule(rules::GRAMMAR_EXPRESSION, |parser| {
                    insert_missing(
                        parser,
                        "syntax/missing-grammar-expression",
                        "expected a grammar expression",
                        ExpectedSyntax::Production(String::from("grammar-expression")),
                        None,
                        None,
                    );
                });
            } else {
                let _ = recover_unexpected(
                    parser,
                    "syntax/unexpected-grammar-token",
                    "unexpected source in a grammar expression",
                    false,
                );
            }
        }

        consume_grammar_ignored_trivia(parser);
        if !consume_rule_literal(parser, rules::SEMICOLON, ";", SyntaxKind::Semicolon)
            && !parser.is_halted()
        {
            insert_missing(
                parser,
                "syntax/missing-semicolon",
                "expected `;` after the grammar rule",
                ExpectedSyntax::Token(SyntaxKind::Semicolon),
                Some(SyntaxKind::Semicolon),
                Some(";"),
            );
        }

        rule.complete_with_flags(parser, SyntaxKind::GrammarRule, NodeFlags::REPARSE_ROOT);
        Attempt::Matched
    })
}

pub(crate) fn parse_grammar_identifier(parser: &mut Parser<'_>) -> bool {
    let result = transactional(parser, rules::GRAMMAR_IDENTIFIER, |parser| {
        let identifier = parser.start();
        if !consume_logical_grapheme(parser, SyntaxKind::Alpha, char::is_alphabetic) {
            return Attempt::NoMatch;
        }

        loop {
            let checkpoint = parser.checkpoint();
            if consume_logical_grapheme(parser, SyntaxKind::Alpha, char::is_alphabetic)
                || consume_logical_grapheme(parser, SyntaxKind::Digit, char::is_numeric)
                || consume_rule_literal(parser, rules::DASH, "-", SyntaxKind::Dash)
            {
                if parser.is_halted() {
                    break;
                }
                continue;
            }
            parser.rewind(checkpoint);
            break;
        }

        identifier.complete(parser, SyntaxKind::GrammarIdentifier);
        Attempt::Matched
    });
    result.accepted()
}

pub(crate) fn parse_grammar_expression(parser: &mut Parser<'_>) -> Attempt {
    transactional(parser, rules::GRAMMAR_EXPRESSION, |parser| {
        let expression = parser.start();
        consume_grammar_ignored_trivia(parser);
        if !parse_grammar_term(parser).accepted() {
            return Attempt::NoMatch;
        }

        loop {
            let checkpoint = parser.checkpoint();
            consume_grammar_ignored_trivia(parser);
            if !consume_rule_literal(parser, rules::BAR, "|", SyntaxKind::Bar) {
                parser.rewind(checkpoint);
                break;
            }
            consume_grammar_ignored_trivia(parser);
            if !parse_grammar_term(parser).accepted() {
                insert_missing_factor(parser);
                break;
            }
        }

        expression.complete(parser, SyntaxKind::GrammarExpression);
        Attempt::Matched
    })
}

pub(crate) fn parse_grammar_term(parser: &mut Parser<'_>) -> Attempt {
    transactional(parser, rules::GRAMMAR_TERM, |parser| {
        let term = parser.start();
        consume_grammar_ignored_trivia(parser);
        if !parse_grammar_factor(parser).accepted() {
            return Attempt::NoMatch;
        }

        loop {
            let checkpoint = parser.checkpoint();
            consume_grammar_ignored_trivia(parser);
            if !consume_rule_literal(parser, rules::COMMA, ",", SyntaxKind::Comma) {
                parser.rewind(checkpoint);
                break;
            }
            consume_grammar_ignored_trivia(parser);
            if !parse_grammar_factor(parser).accepted() {
                insert_missing_factor(parser);
                break;
            }
        }

        term.complete(parser, SyntaxKind::GrammarTerm);
        Attempt::Matched
    })
}

pub(crate) fn parse_grammar_factor(parser: &mut Parser<'_>) -> Attempt {
    transactional(parser, rules::GRAMMAR_FACTOR, |parser| {
        let factor = parser.start();
        consume_grammar_ignored_trivia(parser);

        let result = parse_grammar_repeat0(parser)
            .or_else(|| parse_grammar_repeat1(parser))
            .or_else(|| parse_grammar_optional(parser))
            .or_else(|| parse_grammar_peek(parser))
            .or_else(|| parse_grammar_not(parser))
            .or_else(|| parse_grammar_group(parser))
            .or_else(|| parse_grammar_list(parser))
            .or_else(|| parse_grammar_definition(parser))
            .or_else(|| parse_grammar_range(parser))
            .or_else(|| parse_grammar_terminal(parser));

        let Some(result) = result else {
            return Attempt::NoMatch;
        };
        factor.complete(parser, SyntaxKind::GrammarFactor);
        result
    })
}

fn parse_grammar_definition(parser: &mut Parser<'_>) -> Option<Attempt> {
    let result = transactional(parser, rules::GRAMMAR_DEFINITION, |parser| {
        let definition = parser.start();
        if !parse_grammar_identifier(parser) {
            return Attempt::NoMatch;
        }
        definition.complete(parser, SyntaxKind::GrammarDefinition);
        Attempt::Matched
    });
    result.accepted().then_some(result)
}

fn parse_grammar_repeat0(parser: &mut Parser<'_>) -> Option<Attempt> {
    parse_unary_factor(
        parser,
        rules::GRAMMAR_REPEAT0,
        rules::ASTERISK,
        "*",
        SyntaxKind::Asterisk,
        SyntaxKind::GrammarRepeat0,
    )
}

fn parse_grammar_repeat1(parser: &mut Parser<'_>) -> Option<Attempt> {
    parse_unary_factor(
        parser,
        rules::GRAMMAR_REPEAT1,
        rules::PLUS,
        "+",
        SyntaxKind::Plus,
        SyntaxKind::GrammarRepeat1,
    )
}

fn parse_grammar_optional(parser: &mut Parser<'_>) -> Option<Attempt> {
    parse_unary_factor(
        parser,
        rules::GRAMMAR_OPTIONAL,
        rules::QUESTION,
        "?",
        SyntaxKind::Question,
        SyntaxKind::GrammarOptional,
    )
}

fn parse_grammar_peek(parser: &mut Parser<'_>) -> Option<Attempt> {
    let result = transactional(parser, rules::GRAMMAR_PEEK, |parser| {
        let peek = parser.start();
        if !consume_right_angle(parser) {
            return Attempt::NoMatch;
        }
        consume_grammar_ignored_trivia(parser);
        if !parse_nested_factor(parser) {
            insert_missing_factor(parser);
        }
        peek.complete(parser, SyntaxKind::GrammarPeek);
        Attempt::Committed
    });
    result.accepted().then_some(result)
}

fn parse_grammar_not(parser: &mut Parser<'_>) -> Option<Attempt> {
    parse_unary_factor(
        parser,
        rules::GRAMMAR_NOT,
        rules::NEGATE,
        "¬",
        SyntaxKind::Not,
        SyntaxKind::GrammarNot,
    )
}

fn parse_grammar_list(parser: &mut Parser<'_>) -> Option<Attempt> {
    let result = transactional(parser, rules::GRAMMAR_LIST, |parser| {
        let list = parser.start();
        if !consume_rule_literal(parser, rules::LEFT_BRACKET, "[", SyntaxKind::LeftBracket) {
            return Attempt::NoMatch;
        }

        consume_grammar_ignored_trivia(parser);
        if !parse_nested_factor(parser) {
            insert_missing_factor(parser);
        }

        consume_grammar_ignored_trivia(parser);
        if !consume_rule_literal(parser, rules::COMMA, ",", SyntaxKind::Comma) {
            insert_missing(
                parser,
                "syntax/missing-grammar-factor",
                "expected `,` between grammar list factors",
                ExpectedSyntax::Token(SyntaxKind::Comma),
                Some(SyntaxKind::Comma),
                None,
            );
        }

        consume_grammar_ignored_trivia(parser);
        if !parse_nested_factor(parser) {
            insert_missing_factor(parser);
        }

        consume_grammar_ignored_trivia(parser);
        if !consume_rule_literal(parser, rules::RIGHT_BRACKET, "]", SyntaxKind::RightBracket)
            && !parser.is_halted()
        {
            insert_missing(
                parser,
                "syntax/unclosed-grammar-list",
                "missing `]` to close grammar list",
                ExpectedSyntax::Token(SyntaxKind::RightBracket),
                Some(SyntaxKind::RightBracket),
                Some("]"),
            );
        }

        list.complete(parser, SyntaxKind::GrammarList);
        Attempt::Committed
    });
    result.accepted().then_some(result)
}

fn parse_grammar_range(parser: &mut Parser<'_>) -> Option<Attempt> {
    let result = transactional(parser, rules::GRAMMAR_RANGE, |parser| {
        if !range_intent(parser) {
            return Attempt::NoMatch;
        }

        let range = parser.start();
        if !parse_grammar_terminal_token(parser).accepted() {
            return Attempt::NoMatch;
        }
        consume_grammar_ignored_trivia(parser);
        let first_period = consume_rule_literal(parser, rules::PERIOD, ".", SyntaxKind::Period);
        consume_grammar_ignored_trivia(parser);
        let second_period = consume_rule_literal(parser, rules::PERIOD, ".", SyntaxKind::Period);
        consume_grammar_ignored_trivia(parser);

        if !first_period || !second_period || !parse_grammar_terminal_token(parser).accepted() {
            emit_invalid_range(parser);
        }

        range.complete(parser, SyntaxKind::GrammarRange);
        Attempt::Committed
    });
    result.accepted().then_some(result)
}

fn parse_grammar_group(parser: &mut Parser<'_>) -> Option<Attempt> {
    let result = transactional(parser, rules::GRAMMAR_GROUP, |parser| {
        let group = parser.start();
        if !consume_rule_literal(parser, rules::LEFT_PARENTHESIS, "(", SyntaxKind::LeftParen) {
            return Attempt::NoMatch;
        }

        consume_grammar_ignored_trivia(parser);
        if parser.push_nesting() {
            if !parse_grammar_expression(parser).accepted() {
                parser.with_canonical_rule(rules::GRAMMAR_EXPRESSION, |parser| {
                    insert_missing(
                        parser,
                        "syntax/missing-grammar-expression",
                        "expected an expression inside grammar group",
                        ExpectedSyntax::Production(String::from("grammar-expression")),
                        None,
                        None,
                    );
                });
            }
            parser.pop_nesting();
        } else {
            insert_missing_factor(parser);
        }

        consume_grammar_ignored_trivia(parser);
        if !consume_rule_literal(
            parser,
            rules::RIGHT_PARENTHESIS,
            ")",
            SyntaxKind::RightParen,
        ) && !parser.is_halted()
        {
            insert_missing(
                parser,
                "syntax/unclosed-grammar-group",
                "missing `)` to close grammar group",
                ExpectedSyntax::Token(SyntaxKind::RightParen),
                Some(SyntaxKind::RightParen),
                Some(")"),
            );
        }

        group.complete(parser, SyntaxKind::GrammarGroup);
        Attempt::Committed
    });
    result.accepted().then_some(result)
}

fn parse_grammar_terminal(parser: &mut Parser<'_>) -> Option<Attempt> {
    let result = transactional(parser, rules::GRAMMAR_TERMINAL, |parser| {
        let terminal = parser.start();
        if !parse_grammar_terminal_token(parser).accepted() {
            return Attempt::NoMatch;
        }
        terminal.complete(parser, SyntaxKind::GrammarTerminal);
        Attempt::Committed
    });
    result.accepted().then_some(result)
}

pub(crate) fn parse_grammar_terminal_token(parser: &mut Parser<'_>) -> Attempt {
    transactional(parser, rules::GRAMMAR_TERMINAL_TOKEN, |parser| {
        let terminal = parser.start();
        consume_grammar_ignored_trivia(parser);
        if !consume_rule_literal(parser, rules::QUOTE, "\"", SyntaxKind::Quote) {
            return Attempt::NoMatch;
        }

        let mut content = 0_usize;
        let mut rule_boundary = None;
        let mut recovered_unclosed = false;
        loop {
            consume_grammar_ignored_trivia(parser);
            if parser.is_halted() {
                break;
            }
            if parser.is_eof() {
                if let Some((checkpoint, content_at_boundary)) = rule_boundary {
                    parser.rewind(checkpoint);
                    content = content_at_boundary;
                    recovered_unclosed = true;
                }
                break;
            }
            if logical_starts_with(parser, "\"") {
                if let Some((checkpoint, content_at_boundary)) = rule_boundary
                    && !closing_quote_can_end_terminal(parser)
                {
                    parser.rewind(checkpoint);
                    content = content_at_boundary;
                    recovered_unclosed = true;
                }
                break;
            }
            if rule_boundary.is_none() && semicolon_precedes_rule_start(parser) {
                rule_boundary = Some((parser.checkpoint(), content));
            }
            if !consume_logical_grapheme(parser, SyntaxKind::Any, |_| true) {
                break;
            }
            content = content.saturating_add(1);
        }

        if content == 0 && !parser.is_halted() {
            insert_missing(
                parser,
                "syntax/missing-grammar-factor",
                "grammar terminal must contain at least one character",
                ExpectedSyntax::Production(String::from("grammar-terminal-content")),
                None,
                None,
            );
        }

        consume_grammar_ignored_trivia(parser);
        if (recovered_unclosed
            || !consume_rule_literal(parser, rules::QUOTE, "\"", SyntaxKind::Quote))
            && !parser.is_halted()
        {
            insert_missing(
                parser,
                "syntax/unclosed-grammar-terminal",
                "missing `\"` to close grammar terminal",
                ExpectedSyntax::Token(SyntaxKind::Quote),
                Some(SyntaxKind::Quote),
                Some("\""),
            );
        }

        terminal.complete(parser, SyntaxKind::GrammarTerminalToken);
        Attempt::Committed
    })
}

fn parse_unary_factor(
    parser: &mut Parser<'_>,
    production_rule: crate::document::RuleId,
    prefix_rule: crate::document::RuleId,
    literal: &str,
    token: SyntaxKind,
    node: SyntaxKind,
) -> Option<Attempt> {
    let result = transactional(parser, production_rule, |parser| {
        let unary = parser.start();
        if !consume_rule_literal(parser, prefix_rule, literal, token) {
            return Attempt::NoMatch;
        }
        consume_grammar_ignored_trivia(parser);
        if !parse_nested_factor(parser) {
            insert_missing_factor(parser);
        }
        unary.complete(parser, node);
        Attempt::Committed
    });
    result.accepted().then_some(result)
}

fn parse_nested_factor(parser: &mut Parser<'_>) -> bool {
    if !parser.push_nesting() {
        return false;
    }
    let matched = parse_grammar_factor(parser).accepted();
    parser.pop_nesting();
    matched
}

fn consume_right_angle(parser: &mut Parser<'_>) -> bool {
    if logical_starts_with(parser, ">") {
        consume_rule_literal(parser, rules::RIGHT_ANGLE, ">", SyntaxKind::RightAngle)
    } else {
        consume_rule_literal(parser, rules::RIGHT_ANGLE, "⟩", SyntaxKind::RightAngle)
    }
}

fn insert_missing_factor(parser: &mut Parser<'_>) {
    parser.with_canonical_rule(rules::GRAMMAR_FACTOR, |parser| {
        insert_missing(
            parser,
            "syntax/missing-grammar-factor",
            "expected a grammar factor",
            ExpectedSyntax::Production(String::from("grammar-factor")),
            None,
            None,
        );
    });
}

fn range_intent(parser: &Parser<'_>) -> bool {
    let mut cursor = parser.cursor().clone();
    skip_filtered_trivia(&mut cursor);
    if !cursor_consume_logical_literal(&mut cursor, "\"") {
        return false;
    }

    let mut content = 0_usize;
    loop {
        skip_filtered_trivia(&mut cursor);
        if cursor_consume_logical_literal(&mut cursor, "\"") {
            break;
        }
        let Some((_, range)) = cursor.peek_filtered_grapheme_range(is_grammar_ignored) else {
            return false;
        };
        if cursor.bump_bytes(range.len().0).is_none() {
            return false;
        }
        content = content.saturating_add(1);
    }
    if content == 0 {
        return false;
    }
    skip_filtered_trivia(&mut cursor);
    cursor_logical_starts_with(&cursor, ".")
}

fn skip_filtered_trivia(cursor: &mut super::super::Cursor<'_>) {
    while cursor.peek_char().is_some_and(is_grammar_ignored) {
        let _ = cursor.bump_char();
    }
}

fn semicolon_precedes_rule_start(parser: &Parser<'_>) -> bool {
    let mut cursor = parser.cursor().clone();
    skip_filtered_trivia(&mut cursor);
    if !cursor_consume_logical_literal(&mut cursor, ";") {
        return false;
    }
    cursor_looks_like_rule_start(&mut cursor)
}

fn closing_quote_can_end_terminal(parser: &Parser<'_>) -> bool {
    let mut cursor = parser.cursor().clone();
    skip_filtered_trivia(&mut cursor);
    if !cursor_consume_logical_literal(&mut cursor, "\"") {
        return false;
    }
    skip_filtered_trivia(&mut cursor);
    cursor.is_eof()
        || [".", ",", "|", ";", ")", "]"]
            .iter()
            .any(|literal| cursor_logical_starts_with(&cursor, literal))
}

fn emit_invalid_range(parser: &mut Parser<'_>) {
    let start = parser.offset();
    let error = parser.start();
    let skipped = skip_until_rule_sync(parser, false);
    let range = TextRange::new(start, parser.offset());
    let completed = error.complete_with_flags(parser, SyntaxKind::Error, NodeFlags::ERROR);
    let recovery = if skipped {
        RecoveryAction::Skip { range }
    } else {
        RecoveryAction::Abandon {
            rule: rules::GRAMMAR_RULE,
            at: parser.offset(),
        }
    };
    push_grammar_diagnostic(
        parser,
        "syntax/invalid-grammar-range",
        "grammar range requires two periods and a quoted end point",
        range,
        alloc::vec![ExpectedSyntax::Production(String::from("grammar-range",))],
        recovery,
        Some((completed.position(), first_found(parser, start))),
    );
}

fn recover_unexpected(
    parser: &mut Parser<'_>,
    code: &str,
    message: &str,
    consume_semicolon: bool,
) -> bool {
    let start = parser.offset();
    let error = parser.start();
    let skipped = skip_until_rule_sync(parser, consume_semicolon);
    if !skipped {
        error.abandon(parser);
        return false;
    }
    let completed = error.complete_with_flags(parser, SyntaxKind::Error, NodeFlags::ERROR);
    let range = TextRange::new(start, parser.offset());
    let found = first_found(parser, start);
    parser.with_canonical_rule(rules::GRAMMAR, |parser| {
        push_grammar_diagnostic(
            parser,
            code,
            message,
            range,
            alloc::vec![],
            RecoveryAction::Skip { range },
            Some((completed.position(), found)),
        );
    });
    true
}

fn skip_until_rule_sync(parser: &mut Parser<'_>, consume_semicolon: bool) -> bool {
    let start = parser.offset();
    let limit = parser.config().limits.max_recovery_bytes;
    let mut recovered = 0_u32;

    while !parser.is_eof() && !parser.is_halted() && recovered < limit {
        consume_grammar_ignored_trivia(parser);
        if parser.is_eof() {
            break;
        }
        if logical_starts_with(parser, ";") {
            if consume_semicolon {
                let _ = consume_rule_literal(parser, rules::SEMICOLON, ";", SyntaxKind::Semicolon);
            }
            break;
        }
        if parser.offset() != start && looks_like_rule_start(parser) {
            break;
        }

        let before = parser.offset();
        if !consume_logical_grapheme(parser, SyntaxKind::Unknown, |_| true) {
            break;
        }
        recovered = recovered.saturating_add((parser.offset() - before).0);
    }

    if recovered >= limit && !parser.is_eof() {
        parser.halt();
    }
    parser.stats_mut().recovery_bytes = parser
        .stats()
        .recovery_bytes
        .saturating_add(u64::from(recovered));
    parser.offset() > start
}

fn looks_like_rule_start(parser: &Parser<'_>) -> bool {
    let mut cursor = parser.cursor().clone();
    cursor_looks_like_rule_start(&mut cursor)
}

fn cursor_looks_like_rule_start(cursor: &mut super::super::Cursor<'_>) -> bool {
    skip_filtered_trivia(cursor);
    let Some((first, first_range)) = cursor.peek_filtered_grapheme_range(is_grammar_ignored) else {
        return false;
    };
    if !first.is_alphabetic() || cursor.bump_bytes(first_range.len().0).is_none() {
        return false;
    }

    loop {
        let Some((first, range)) = cursor.peek_filtered_grapheme_range(is_grammar_ignored) else {
            return false;
        };
        if first.is_alphabetic() || first.is_numeric() {
            if cursor.bump_bytes(range.len().0).is_none() {
                return false;
            }
            continue;
        }
        if cursor_consume_logical_literal(cursor, "-") {
            continue;
        }
        break;
    }

    skip_filtered_trivia(cursor);
    if !cursor_consume_logical_literal(cursor, ":") {
        return false;
    }
    skip_filtered_trivia(cursor);
    cursor_logical_starts_with(cursor, "=")
}

fn cursor_logical_starts_with(
    cursor: &super::super::Cursor<'_>,
    literal: &str,
) -> bool {
    cursor
        .filtered_grapheme_literal_end(literal, is_grammar_ignored)
        .is_some()
}

fn cursor_consume_logical_literal(
    cursor: &mut super::super::Cursor<'_>,
    literal: &str,
) -> bool {
    let Some(end) =
        cursor.filtered_grapheme_literal_end(literal, is_grammar_ignored)
    else {
        return false;
    };
    cursor.bump_bytes((end - cursor.offset()).0).is_some()
}

fn first_found(parser: &Parser<'_>, at: TextSize) -> FoundSyntax {
    super::found::found_syntax(parser, at)
}

fn push_grammar_diagnostic(
    parser: &mut Parser<'_>,
    code: &str,
    message: &str,
    range: TextRange,
    expected: alloc::vec::Vec<ExpectedSyntax>,
    recovery: RecoveryAction,
    event_and_found: Option<(usize, FoundSyntax)>,
) {
    let (event, found) = event_and_found
        .map(|(event, found)| (Some(event), Some(found)))
        .unwrap_or_else(|| (None, Some(parser.found_syntax())));
    let diagnostic = Diagnostic {
        id: parser.next_diagnostic_id(),
        code: DiagnosticCode::from(code),
        phase: DiagnosticPhase::Syntax,
        severity: Severity::Error,
        rule: parser.current_rule(),
        context: parser.current_context(),
        primary: DiagnosticAnchor::Absolute {
            revision: parser.source().revision(),
            range,
        },
        labels: alloc::vec![],
        expected,
        found,
        fixes: alloc::vec![],
        related: alloc::vec![],
        recovery: Some(recovery),
        tags: DiagnosticTags::NONE,
        message: String::from(message),
    };
    let relative = if event.is_some() {
        TextRange::new(TextSize::ZERO, range.len())
    } else {
        range
    };
    parser.push_diagnostic(diagnostic, event, relative);
}
