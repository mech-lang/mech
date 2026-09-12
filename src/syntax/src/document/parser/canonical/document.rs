//! Canonical document grammar interpreter.

use crate::document::{ExpectedSyntax, NodeFlags, RuleId, SyntaxKind};

use super::super::Parser;
use super::super::recovery;
use super::super::rule::rules;
use super::combinator::Attempt;
use super::document_grammar::{
    DOCUMENT_RULE_COUNT, DOCUMENT_RULES, DocumentRule, GrammarExpression,
};
use super::{
    base, control_operators, declarations, imports, kinds, literals, mechdown, operators, paths,
    pattern_primitives, recursive_core, source_imports, statements, structure_shell,
    subscript_primitives,
};

#[derive(Clone, Copy, Debug, Default)]
struct GrammarState {
    codeblock_delimiter: Option<RuleId>,
}

pub(crate) fn parse_document_root(parser: &mut Parser<'_>) {
    let result = parse_rule(parser, rules::PARSE);
    if !result.accepted() && !parser.is_halted() {
        let document = parser.start();
        parser.with_canonical_rule(rules::PARSE, |parser| {
            let _ = recovery::abandon_to_restart(
                parser,
                rules::PARSE,
                &[],
                "syntax/invalid-document",
                "source does not form a canonical document",
            );
        });
        document.complete_with_flags(parser, SyntaxKind::Document, NodeFlags::REPARSE_ROOT);
    }
}

pub(crate) fn supports(rule: RuleId) -> bool {
    debug_assert_eq!(DOCUMENT_RULES.len(), DOCUMENT_RULE_COUNT);
    DOCUMENT_RULES
        .iter()
        .any(|candidate| candidate.rule == rule)
}

pub(crate) fn parse_rule(parser: &mut Parser<'_>, rule: RuleId) -> Attempt {
    let Some(specification) = DOCUMENT_RULES
        .iter()
        .find(|candidate| candidate.rule == rule)
    else {
        return Attempt::NoMatch;
    };
    if !document_rule_enabled(specification) {
        return Attempt::NoMatch;
    }
    let checkpoint = parser.checkpoint();
    let Some(result) = parser.with_nesting(|parser| {
        parser.with_canonical_rule(rule, |parser| parse_document_rule(parser, specification))
    }) else {
        recovery::nesting_limit(parser);
        return Attempt::Committed;
    };
    if result == Attempt::NoMatch {
        parser.rewind(checkpoint);
    }
    result
}

fn parse_document_rule(parser: &mut Parser<'_>, specification: &DocumentRule) -> Attempt {
    if specification.rule == rules::EVAL_INLINE_MECH_CODE && parser.cursor().starts_with("{{") {
        return Attempt::NoMatch;
    }
    let marker = specification.kind.map(|_| parser.start());
    let mut state = GrammarState::default();
    let mut result = if specification.rule == rules::MECH_CODE_ALT
        && comment_wins_at_mech_item_boundary(parser)
    {
        let leading = parse_any_rule(parser, rules::WHITESPACE0);
        if !leading.accepted() {
            leading
        } else {
            parse_any_rule(parser, rules::COMMENT)
        }
    } else {
        parse_expression(parser, &specification.expression, &mut state)
    };
    if parser.is_halted() {
        result = Attempt::Committed;
    }

    if specification.rule == rules::PARSE && result.accepted() && !parser.is_eof() {
        let _ = recovery::abandon_to_restart(
            parser,
            rules::PARSE,
            &[],
            "syntax/unexpected-document-source",
            "unexpected source after the canonical document",
        );
        result = Attempt::Committed;
    }

    if let Some(marker) = marker {
        if result == Attempt::NoMatch {
            marker.abandon(parser);
        } else {
            let flags = if specification.root {
                NodeFlags::REPARSE_ROOT
            } else {
                NodeFlags::NONE
            };
            marker.complete_with_flags(
                parser,
                specification
                    .kind
                    .expect("document rule marker requires a node kind"),
                flags,
            );
        }
    }
    result
}

fn comment_wins_at_mech_item_boundary(parser: &mut Parser<'_>) -> bool {
    let checkpoint = parser.checkpoint();
    let _ = base::parse_rule(parser, rules::WHITESPACE0);
    if !parser.cursor().starts_with("--") {
        parser.rewind(checkpoint);
        return false;
    }
    if matches!(
        parser.cursor().byte_at(2),
        None | Some(b' ' | b'\t' | b'\r' | b'\n')
    ) {
        parser.rewind(checkpoint);
        return true;
    }
    let expression = parse_any_rule(parser, rules::EXPRESSION);
    let complete_expression = expression == Attempt::Committed
        || expression == Attempt::Matched
            && parse_any_rule(parser, rules::CODE_TERMINAL).accepted();
    parser.rewind(checkpoint);
    !complete_expression
}

fn document_rule_enabled(specification: &DocumentRule) -> bool {
    match specification.feature {
        None => true,
        Some("mika") => cfg!(feature = "mika"),
        Some("invariant_define") => cfg!(feature = "invariant_define"),
        Some(feature) => unreachable!("unknown generated document feature {feature}"),
    }
}

fn parse_expression(
    parser: &mut Parser<'_>,
    expression: &GrammarExpression,
    state: &mut GrammarState,
) -> Attempt {
    if parser.is_halted() {
        return Attempt::Committed;
    }
    match expression {
        GrammarExpression::Rule(rule) => {
            if *rule == rules::MIKA_SECTION_CLOSE && !cfg!(feature = "mika") {
                return Attempt::NoMatch;
            }
            let start = parser.offset();
            let mut result = parse_any_rule(parser, *rule);
            if parser.is_halted() {
                result = Attempt::Committed;
            }
            if *rule == rules::CODEBLOCK_SIGIL && result.accepted() {
                let range = crate::document::TextRange::new(start, parser.offset());
                state.codeblock_delimiter =
                    parser
                        .source()
                        .text(range)
                        .ok()
                        .and_then(|text| match text.as_str() {
                            "```" => Some(rules::GRAVE_CODEBLOCK_SIGIL),
                            "~~~" => Some(rules::TILDE_CODEBLOCK_SIGIL),
                            _ => None,
                        });
            }
            result
        }
        GrammarExpression::Builtin(name) => parse_builtin(parser, name, state),
        GrammarExpression::Literal(literal) => {
            if parser.cursor().grapheme_literal_end(literal).is_none() {
                return Attempt::NoMatch;
            }
            let Ok(length) = u32::try_from(literal.len()) else {
                return Attempt::NoMatch;
            };
            let result = parser
                .bump_bytes_token(length, SyntaxKind::Text)
                .map(|_| Attempt::Matched)
                .unwrap_or(Attempt::NoMatch);
            if parser.is_halted() {
                Attempt::Committed
            } else {
                result
            }
        }
        GrammarExpression::MikaExpressionTriples(triples) => {
            parse_registered_mika_expression(parser, triples)
        }
        GrammarExpression::Empty => Attempt::Matched,
        GrammarExpression::Sequence(items) => parse_sequence(parser, items, state),
        GrammarExpression::Choice(items) => {
            for item in *items {
                let checkpoint = parser.checkpoint();
                let initial_state = *state;
                let result = parse_expression(parser, item, state);
                if parser.is_halted() {
                    return Attempt::Committed;
                }
                if result.accepted() {
                    return result;
                }
                parser.rewind(checkpoint);
                *state = initial_state;
            }
            Attempt::NoMatch
        }
        GrammarExpression::BestChoice(items) => {
            let start = parser.checkpoint();
            let start_offset = parser.offset();
            let initial_state = *state;
            let mut selected = None::<(usize, u32)>;
            for (index, item) in items.iter().enumerate() {
                parser.rewind(start);
                *state = initial_state;
                let result = parse_expression(parser, item, state);
                if parser.is_halted() {
                    return Attempt::Committed;
                }
                if result == Attempt::Committed {
                    if selected.is_none() {
                        return Attempt::Committed;
                    }
                    continue;
                }
                if result == Attempt::Matched {
                    let consumed = (parser.offset() - start_offset).0;
                    if selected.is_none_or(|(_, selected_consumed)| consumed > selected_consumed) {
                        selected = Some((index, consumed));
                    }
                }
            }
            parser.rewind(start);
            *state = initial_state;
            selected
                .map(|(index, _)| parse_expression(parser, &items[index], state))
                .unwrap_or(Attempt::NoMatch)
        }
        GrammarExpression::Optional(item) => {
            let checkpoint = parser.checkpoint();
            let initial_state = *state;
            let result = parse_expression(parser, item, state);
            if result == Attempt::NoMatch {
                parser.rewind(checkpoint);
                *state = initial_state;
                Attempt::Matched
            } else {
                result
            }
        }
        GrammarExpression::ZeroOrMore(item) => parse_repetition(parser, item, false, state),
        GrammarExpression::OneOrMore(item) => parse_repetition(parser, item, true, state),
        GrammarExpression::Peek(item) => {
            let checkpoint = parser.checkpoint();
            let initial_state = *state;
            let result = parse_expression(parser, item, state);
            parser.rewind(checkpoint);
            *state = initial_state;
            if result.accepted() {
                Attempt::Matched
            } else {
                Attempt::NoMatch
            }
        }
        GrammarExpression::Not(item) => {
            let checkpoint = parser.checkpoint();
            let initial_state = *state;
            let result = parse_expression(parser, item, state);
            parser.rewind(checkpoint);
            *state = initial_state;
            if result.accepted() {
                Attempt::NoMatch
            } else {
                Attempt::Matched
            }
        }
        GrammarExpression::Separated { separator, item } => {
            let first = parse_expression(parser, item, state);
            if !first.accepted() {
                return Attempt::NoMatch;
            }
            let mut committed = first == Attempt::Committed;
            loop {
                let checkpoint = parser.checkpoint();
                let initial_state = *state;
                let before = parser.offset();
                let separator_result = parse_expression(parser, separator, state);
                if !separator_result.accepted() {
                    parser.rewind(checkpoint);
                    *state = initial_state;
                    break;
                }
                let item_result = parse_expression(parser, item, state);
                if !item_result.accepted() {
                    parser.rewind(checkpoint);
                    *state = initial_state;
                    break;
                }
                committed |=
                    separator_result == Attempt::Committed || item_result == Attempt::Committed;
                if parser.offset() == before || parser.is_halted() {
                    break;
                }
            }
            if committed {
                Attempt::Committed
            } else {
                Attempt::Matched
            }
        }
    }
}

fn parse_registered_mika_expression(
    parser: &mut Parser<'_>,
    triples: &[(&str, &str, &str)],
) -> Attempt {
    for &(left, nose, right) in triples {
        let checkpoint = parser.checkpoint();
        let parts = [
            (rules::MIKA_EYE_LEFT, SyntaxKind::MikaEyeLeft, left),
            (rules::MIKA_NOSE, SyntaxKind::MikaNose, nose),
            (rules::MIKA_EYE_RIGHT, SyntaxKind::MikaEyeRight, right),
        ];
        if parts.into_iter().all(|(rule, kind, literal)| {
            parser.with_canonical_rule(rule, |parser| {
                let marker = parser.start();
                if parser.cursor().grapheme_literal_end(literal).is_none() {
                    marker.abandon(parser);
                    return false;
                }
                let Ok(length) = u32::try_from(literal.len()) else {
                    marker.abandon(parser);
                    return false;
                };
                if parser.bump_bytes_token(length, SyntaxKind::Text).is_none() {
                    marker.abandon(parser);
                    return false;
                }
                marker.complete(parser, kind);
                true
            })
        }) {
            return Attempt::Matched;
        }
        parser.rewind(checkpoint);
    }
    Attempt::NoMatch
}

fn parse_sequence(
    parser: &mut Parser<'_>,
    items: &[GrammarExpression],
    state: &mut GrammarState,
) -> Attempt {
    let checkpoint = parser.checkpoint();
    let initial_state = *state;
    let mut committed = false;
    let mut distinctive_prefix = false;
    let inline_mech_sequence = matches!(
        (items.first(), items.get(1)),
        (
            Some(GrammarExpression::Rule(left)),
            Some(GrammarExpression::Rule(right))
        ) if *left == rules::LEFT_BRACE && *right == rules::LEFT_BRACE
    );
    for (index, item) in items.iter().enumerate() {
        match parse_expression(parser, item, state) {
            Attempt::Matched => {
                if matches!(item, GrammarExpression::Rule(rule) if *rule == rules::CODEBLOCK_SIGIL || *rule == rules::MIKA_SECTION_OPEN)
                    || inline_mech_sequence && index == 1
                {
                    distinctive_prefix = true;
                }
            }
            Attempt::Committed => committed = true,
            Attempt::NoMatch
                if matches!(item, GrammarExpression::Builtin("matching-codeblock-sigil"))
                    && state.codeblock_delimiter.is_some() =>
            {
                let token = match state.codeblock_delimiter {
                    Some(rule) if rule == rules::GRAVE_CODEBLOCK_SIGIL => {
                        SyntaxKind::GraveCodeBlockSigil
                    }
                    Some(rule) if rule == rules::TILDE_CODEBLOCK_SIGIL => {
                        SyntaxKind::TildeCodeBlockSigil
                    }
                    _ => unreachable!("closed codeblock delimiter state"),
                };
                recovery::insert_missing(
                    parser,
                    "syntax/missing-codeblock-sigil",
                    "expected a closing codeblock delimiter matching the opener",
                    ExpectedSyntax::Token(token),
                    Some(token),
                );
                committed = true;
            }
            Attempt::NoMatch
                if distinctive_prefix && recover_required_sequence_item(parser, item) =>
            {
                committed = true;
            }
            Attempt::NoMatch if committed || distinctive_prefix => return Attempt::Committed,
            Attempt::NoMatch => {
                parser.rewind(checkpoint);
                *state = initial_state;
                return Attempt::NoMatch;
            }
        }
    }
    if committed {
        Attempt::Committed
    } else {
        Attempt::Matched
    }
}

fn recover_required_sequence_item(parser: &mut Parser<'_>, item: &GrammarExpression) -> bool {
    let GrammarExpression::Rule(rule) = item else {
        return false;
    };
    if *rule == rules::MECH_CODE_ALT {
        recovery::insert_missing(
            parser,
            "syntax/missing-inline-mech-body",
            "expected a body after the inline Mech opener",
            ExpectedSyntax::Production(alloc::string::String::from("inline Mech body")),
            None,
        );
        return true;
    }
    let (code, message, token) = if *rule == rules::NEW_LINE {
        (
            "syntax/missing-codeblock-header-newline",
            "expected a newline after the code-block header",
            SyntaxKind::Newline,
        )
    } else if *rule == rules::RIGHT_BRACE {
        (
            "syntax/missing-inline-mech-closer",
            "expected a closing brace for inline Mech code",
            SyntaxKind::RightBrace,
        )
    } else if *rule == rules::MIKA_SECTION_CLOSE {
        (
            "syntax/missing-mika-section-closer",
            "expected a closing Mika section delimiter",
            SyntaxKind::MikaSectionClose,
        )
    } else {
        return false;
    };
    recovery::insert_missing(
        parser,
        code,
        message,
        ExpectedSyntax::Token(token),
        Some(token),
    );
    true
}

fn parse_repetition(
    parser: &mut Parser<'_>,
    item: &GrammarExpression,
    require_one: bool,
    state: &mut GrammarState,
) -> Attempt {
    let mut count = 0_usize;
    let mut committed = false;
    loop {
        let checkpoint = parser.checkpoint();
        let initial_state = *state;
        let before = parser.offset();
        let result = parse_expression(parser, item, state);
        if !result.accepted() {
            parser.rewind(checkpoint);
            *state = initial_state;
            break;
        }
        if parser.offset() == before && (!require_one || count > 0) {
            parser.rewind(checkpoint);
            *state = initial_state;
            break;
        }
        count = count.saturating_add(1);
        committed |= result == Attempt::Committed;
        if parser.offset() == before || parser.is_halted() {
            break;
        }
    }
    if require_one && count == 0 {
        Attempt::NoMatch
    } else if committed {
        Attempt::Committed
    } else {
        Attempt::Matched
    }
}

fn parse_builtin(parser: &mut Parser<'_>, name: &str, state: &GrammarState) -> Attempt {
    let matched = match name {
        "eof" => parser.is_eof(),
        "matching-codeblock-sigil" => state
            .codeblock_delimiter
            .is_some_and(|delimiter| base::parse_rule(parser, delimiter)),
        _ => false,
    };
    if matched {
        Attempt::Matched
    } else {
        Attempt::NoMatch
    }
}

fn parse_any_rule(parser: &mut Parser<'_>, rule: RuleId) -> Attempt {
    if supports(rule) {
        return parse_rule(parser, rule);
    }
    if let Some(result) = recursive_core::parse_rule(parser, rule)
        .or_else(|| source_imports::parse_rule(parser, rule))
        .or_else(|| declarations::parse_rule(parser, rule))
        .or_else(|| imports::parse_rule(parser, rule))
        .or_else(|| operators::parse_rule(parser, rule))
        .or_else(|| control_operators::parse_rule(parser, rule))
        .or_else(|| subscript_primitives::parse_rule(parser, rule))
        .or_else(|| pattern_primitives::parse_rule(parser, rule))
        .or_else(|| structure_shell::parse_rule(parser, rule))
    {
        return result;
    }
    if let Some(result) =
        parse_phase_2c_rule(parser, rule).or_else(|| parse_mechdown_rule(parser, rule))
    {
        return result;
    }
    if base::parse_rule(parser, rule) {
        Attempt::Matched
    } else {
        Attempt::NoMatch
    }
}

fn parse_phase_2c_rule(parser: &mut Parser<'_>, rule: RuleId) -> Option<Attempt> {
    let result = match rule {
        rules::EMPTY => literals::parse_empty(parser),
        rules::ATOM => literals::parse_atom(parser),
        rules::STRING => literals::parse_string(parser),
        rules::UTF8_STRING => literals::parse_utf8_string(parser),
        rules::RAW_STRING => literals::parse_raw_string(parser),
        rules::BOOLEAN => literals::parse_boolean(parser),
        rules::TRUE_LITERAL => literals::parse_true_literal(parser),
        rules::FALSE_LITERAL => literals::parse_false_literal(parser),
        rules::NUMBER => literals::parse_number(parser),
        rules::COMPLEX_NUMBER => literals::parse_complex_number(parser),
        rules::REAL_NUMBER => literals::parse_real_number(parser),
        rules::UNTYPED_REAL_NUMBER => literals::parse_untyped_real_number(parser),
        rules::RATIONAL_LITERAL => literals::parse_rational_literal(parser),
        rules::SCIENTIFIC_LITERAL => literals::parse_scientific_literal(parser),
        rules::FLOAT_DECIMAL_START => literals::parse_float_decimal_start(parser),
        rules::FLOAT_FULL => literals::parse_float_full(parser),
        rules::FLOAT_LITERAL => literals::parse_float_literal(parser),
        rules::INTEGER_LITERAL => literals::parse_integer_literal(parser),
        rules::TYPED_INTEGER => literals::parse_typed_integer(parser),
        rules::UNTYPED_INTEGER => literals::parse_untyped_integer(parser),
        rules::DECIMAL_LITERAL => literals::parse_decimal_literal(parser),
        rules::HEXADECIMAL_LITERAL => literals::parse_hexadecimal_literal(parser),
        rules::OCTAL_LITERAL => literals::parse_octal_literal(parser),
        rules::BINARY_LITERAL => literals::parse_binary_literal(parser),
        rules::CONTEXT_ADDRESS_PATH_TOKEN => paths::parse_context_address_path_token(parser),
        rules::CONTEXT_ADDRESS_PATH => paths::parse_context_address_path(parser),
        rules::PREFIXED_CONTEXT_PATH => paths::parse_prefixed_context_path(parser),
        rules::KIND_ANY => kinds::parse_kind_any(parser),
        rules::KIND_EMPTY => kinds::parse_kind_empty(parser),
        rules::KIND_ATOM => kinds::parse_kind_atom(parser),
        _ => return None,
    };
    Some(result)
}

fn parse_mechdown_rule(parser: &mut Parser<'_>, rule: RuleId) -> Option<Attempt> {
    let result = match rule {
        rules::COMMENT_SIGIL => {
            if statements::parse_comment_sigil(parser) {
                Attempt::Matched
            } else {
                Attempt::NoMatch
            }
        }
        rules::COMMENT => statements::parse_comment(parser),
        rules::CODEBLOCK_SIGIL => {
            if mechdown::parse_codeblock_sigil(parser).is_some() {
                Attempt::Matched
            } else {
                Attempt::NoMatch
            }
        }
        rules::INLINE_CODE => mechdown::parse_inline_code(parser),
        rules::INLINE_EQUATION => mechdown::parse_inline_equation(parser),
        rules::RAW_HYPERLINK => mechdown::parse_raw_hyperlink(parser),
        rules::FOOTNOTE_REFERENCE => mechdown::parse_footnote_reference(parser),
        rules::REFERENCE => mechdown::parse_reference(parser),
        rules::SECTION_REFERENCE => mechdown::parse_section_reference(parser),
        rules::PARAGRAPH_TEXT => mechdown::parse_paragraph_text(parser),
        rules::THEMATIC_BREAK => mechdown::parse_thematic_break(parser),
        rules::BLANK_LINE => mechdown::parse_blank_line(parser),
        rules::EQUATION => mechdown::parse_equation(parser),
        _ => return None,
    };
    Some(result)
}
