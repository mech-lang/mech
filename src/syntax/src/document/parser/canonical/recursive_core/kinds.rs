use crate::document::SyntaxKind;

use super::super::super::Parser;
use super::super::super::rule::rules;
use super::super::{base, combinator, kinds as leaves};
use super::{
    Attempt, child_result, literals, nesting_limit, precedence, recover_closer,
    recover_required_production,
};

pub(super) fn parse_kind_annotation(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::KIND_ANNOTATION, |parser| {
        let node = parser.start();
        if !base::parse_rule(parser, rules::LEFT_ANGLE) {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let Some(interior) = parser.with_nesting(|parser| {
            let mut committed = false;
            match parse_kind_with_option(parser) {
                Attempt::Matched => {}
                Attempt::NoMatch => {
                    recover_required_production(
                        parser,
                        rules::KIND_ANNOTATION,
                        "syntax/missing-kind-annotation-kind",
                        "missing kind inside annotation",
                        "kind",
                    );
                    committed = true;
                }
                Attempt::Committed => committed = true,
            }
            if base::parse_rule(parser, rules::RIGHT_ANGLE) {
                if committed {
                    Attempt::Committed
                } else {
                    Attempt::Matched
                }
            } else {
                recover_closer(
                    parser,
                    rules::KIND_ANNOTATION,
                    rules::RIGHT_ANGLE,
                    SyntaxKind::RightAngle,
                    '>',
                    ">",
                )
            }
        }) else {
            let result = nesting_limit(parser);
            node.complete(parser, SyntaxKind::KindAnnotation);
            return result;
        };
        finish(node, parser, SyntaxKind::KindAnnotation, interior)
    })
}

pub(super) fn parse_kind(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::KIND, |parser| {
        let node = parser.start();
        let selected = if parser.cursor().starts_with("{") {
            kind_brace_selection(parser)
        } else {
            let mut selected = Attempt::NoMatch;
            for parse in [
                leaves::parse_kind_any,
                leaves::parse_kind_atom,
                leaves::parse_kind_empty,
                parse_kind_matrix,
                parse_kind_scalar,
                parse_kind_table,
                parse_kind_tuple,
                parse_kind_kind,
            ] {
                selected = parse(parser);
                if selected != Attempt::NoMatch {
                    break;
                }
            }
            selected
        };
        if let Some(result) = child_result(parser, node, SyntaxKind::Kind, selected) {
            return result;
        }
        node.complete(parser, SyntaxKind::Kind);
        Attempt::Matched
    })
}

pub(super) fn parse_kind_with_option(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::KIND_WITH_OPTION, |parser| {
        let node = parser.start();
        let child = parse_kind(parser);
        if let Some(result) = child_result(parser, node, SyntaxKind::KindWithOption, child) {
            return result;
        }
        let _ = base::parse_rule(parser, rules::QUESTION);
        node.complete(parser, SyntaxKind::KindWithOption);
        Attempt::Matched
    })
}

pub(super) fn parse_kind_kind(parser: &mut Parser<'_>) -> Attempt {
    delimited_kind(
        parser,
        rules::KIND_KIND,
        rules::LEFT_ANGLE,
        rules::RIGHT_ANGLE,
        SyntaxKind::KindKind,
        parse_kind_with_option,
    )
}

pub(super) fn parse_kind_table(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::KIND_TABLE, |parser| {
        let node = parser.start();
        if !base::parse_rule(parser, rules::BAR) {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let mut committed = false;
        match kind_table_field(parser) {
            Attempt::Matched => {}
            Attempt::NoMatch => {
                recover_required_production(
                    parser,
                    rules::KIND_TABLE,
                    "syntax/missing-kind-table-field",
                    "missing table kind field",
                    "kind-table-field",
                );
                committed = true;
            }
            Attempt::Committed => {
                committed = true;
            }
        }
        while !parser.is_halted() {
            let separator = base::parse_rule(parser, rules::LIST_SEPARATOR)
                || base::parse_rule(parser, rules::SPACE_TAB1);
            if !separator {
                break;
            }
            match kind_table_field(parser) {
                Attempt::Matched => {}
                Attempt::NoMatch => {
                    recover_required_production(
                        parser,
                        rules::KIND_TABLE,
                        "syntax/missing-kind-table-field",
                        "missing table kind field after separator",
                        "kind-table-field",
                    );
                    committed = true;
                    break;
                }
                Attempt::Committed => {
                    committed = true;
                    break;
                }
            }
        }
        if !base::parse_rule(parser, rules::BAR) {
            recover_closer(
                parser,
                rules::KIND_TABLE,
                rules::BAR,
                SyntaxKind::Bar,
                '|',
                "|",
            );
            node.complete(parser, SyntaxKind::TableKind);
            return Attempt::Committed;
        }
        let suffix = parser.checkpoint();
        if base::parse_rule(parser, rules::COLON) {
            match literals::parse_literal(parser) {
                Attempt::Matched => {}
                Attempt::Committed => {
                    node.complete(parser, SyntaxKind::TableKind);
                    return Attempt::Committed;
                }
                Attempt::NoMatch => parser.rewind(suffix),
            }
        }
        node.complete(parser, SyntaxKind::TableKind);
        if committed {
            Attempt::Committed
        } else {
            Attempt::Matched
        }
    })
}

pub(super) fn parse_kind_set(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::KIND_SET, |parser| {
        let node = parser.start();
        if !base::parse_rule(parser, rules::LEFT_BRACE) {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let Some(interior) = parser.with_nesting(|parser| {
            match parse_kind(parser) {
                Attempt::Matched => {}
                Attempt::NoMatch => return Attempt::NoMatch,
                Attempt::Committed => {
                    recover_closer(
                        parser,
                        rules::KIND_SET,
                        rules::RIGHT_BRACE,
                        SyntaxKind::RightBrace,
                        '}',
                        "}",
                    );
                    return Attempt::Committed;
                }
            }
            if base::parse_rule(parser, rules::RIGHT_BRACE) {
                Attempt::Matched
            } else {
                recover_closer(
                    parser,
                    rules::KIND_SET,
                    rules::RIGHT_BRACE,
                    SyntaxKind::RightBrace,
                    '}',
                    "}",
                )
            }
        }) else {
            let result = nesting_limit(parser);
            node.complete(parser, SyntaxKind::KindSet);
            return result;
        };
        if interior != Attempt::Matched {
            return finish(node, parser, SyntaxKind::KindSet, interior);
        }
        let literal_suffix = parser.checkpoint();
        if base::parse_rule(parser, rules::COLON) {
            match literals::parse_literal(parser) {
                Attempt::Matched => {}
                Attempt::Committed => {
                    node.complete(parser, SyntaxKind::KindSet);
                    return Attempt::Committed;
                }
                Attempt::NoMatch => parser.rewind(literal_suffix),
            }
        }
        let _ = base::parse_exact_tag(parser, ":N", SyntaxKind::Text);
        node.complete(parser, SyntaxKind::KindSet);
        Attempt::Matched
    })
}

pub(super) fn parse_kind_map(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::KIND_MAP, |parser| {
        let node = parser.start();
        if !base::parse_rule(parser, rules::LEFT_BRACE) {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let Some(interior) = parser.with_nesting(|parser| {
            for required in [parse_kind as fn(&mut Parser<'_>) -> Attempt] {
                let result = required(parser);
                if result != Attempt::Matched {
                    return result;
                }
            }
            if !base::parse_rule(parser, rules::COLON) {
                return Attempt::NoMatch;
            }
            match parse_kind(parser) {
                Attempt::Matched => {}
                Attempt::Committed => {
                    recover_closer(
                        parser,
                        rules::KIND_MAP,
                        rules::RIGHT_BRACE,
                        SyntaxKind::RightBrace,
                        '}',
                        "}",
                    );
                    return Attempt::Committed;
                }
                Attempt::NoMatch => {
                    recover_required_production(
                        parser,
                        rules::KIND_MAP,
                        "syntax/missing-kind-map-value",
                        "missing map value kind after colon",
                        "kind",
                    );
                    recover_closer(
                        parser,
                        rules::KIND_MAP,
                        rules::RIGHT_BRACE,
                        SyntaxKind::RightBrace,
                        '}',
                        "}",
                    );
                    return Attempt::Committed;
                }
            }
            if base::parse_rule(parser, rules::RIGHT_BRACE) {
                Attempt::Matched
            } else {
                recover_closer(
                    parser,
                    rules::KIND_MAP,
                    rules::RIGHT_BRACE,
                    SyntaxKind::RightBrace,
                    '}',
                    "}",
                )
            }
        }) else {
            let result = nesting_limit(parser);
            node.complete(parser, SyntaxKind::KindMap);
            return result;
        };
        finish(node, parser, SyntaxKind::KindMap, interior)
    })
}

pub(super) fn parse_kind_record(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::KIND_RECORD, |parser| {
        let node = parser.start();
        if !base::parse_rule(parser, rules::LEFT_BRACE) {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let Some(interior) = parser.with_nesting(|parser| kind_record_interior(parser)) else {
            let result = nesting_limit(parser);
            node.complete(parser, SyntaxKind::KindRecord);
            return result;
        };
        finish(node, parser, SyntaxKind::KindRecord, interior)
    })
}

pub(super) fn parse_kind_matrix(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::KIND_MATRIX, |parser| {
        let node = parser.start();
        if !base::parse_rule(parser, rules::LEFT_BRACKET) {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let Some(interior) = parser.with_nesting(|parser| {
            let mut committed = false;
            match parse_kind_with_option(parser) {
                Attempt::Matched => {}
                Attempt::NoMatch => {
                    recover_required_production(
                        parser,
                        rules::KIND_MATRIX,
                        "syntax/missing-kind-matrix-element",
                        "missing matrix element kind",
                        "kind",
                    );
                    committed = true;
                }
                Attempt::Committed => committed = true,
            }
            if base::parse_rule(parser, rules::RIGHT_BRACKET) {
                if committed {
                    Attempt::Committed
                } else {
                    Attempt::Matched
                }
            } else {
                recover_closer(
                    parser,
                    rules::KIND_MATRIX,
                    rules::RIGHT_BRACKET,
                    SyntaxKind::RightBracket,
                    ']',
                    "]",
                )
            }
        }) else {
            let result = nesting_limit(parser);
            node.complete(parser, SyntaxKind::KindMatrix);
            return result;
        };
        if interior != Attempt::Matched {
            return finish(node, parser, SyntaxKind::KindMatrix, interior);
        }
        let _ = base::parse_rule(parser, rules::COLON);
        match literals::parse_literal(parser) {
            Attempt::Matched => loop {
                let pair = parser.checkpoint();
                if !base::parse_rule(parser, rules::LIST_SEPARATOR) {
                    break;
                }
                match literals::parse_literal(parser) {
                    Attempt::Matched => {}
                    Attempt::NoMatch => {
                        parser.rewind(pair);
                        break;
                    }
                    Attempt::Committed => {
                        node.complete(parser, SyntaxKind::KindMatrix);
                        return Attempt::Committed;
                    }
                }
            },
            Attempt::NoMatch => {}
            Attempt::Committed => {
                node.complete(parser, SyntaxKind::KindMatrix);
                return Attempt::Committed;
            }
        }
        node.complete(parser, SyntaxKind::KindMatrix);
        Attempt::Matched
    })
}

pub(super) fn parse_kind_tuple(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::KIND_TUPLE, |parser| {
        let node = parser.start();
        if !base::parse_rule(parser, rules::LEFT_PARENTHESIS) {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let Some(interior) = parser.with_nesting(|parser| {
            let mut committed = false;
            match parse_kind(parser) {
                Attempt::Matched => {}
                Attempt::NoMatch => {
                    recover_required_production(
                        parser,
                        rules::KIND_TUPLE,
                        "syntax/missing-kind-tuple-item",
                        "missing tuple kind item",
                        "kind",
                    );
                    committed = true;
                }
                Attempt::Committed => committed = true,
            }
            while !parser.is_halted() {
                if !base::parse_rule(parser, rules::LIST_SEPARATOR) {
                    break;
                }
                match parse_kind(parser) {
                    Attempt::Matched => {}
                    Attempt::NoMatch => {
                        recover_required_production(
                            parser,
                            rules::KIND_TUPLE,
                            "syntax/missing-kind-tuple-item",
                            "missing tuple kind item after separator",
                            "kind",
                        );
                        committed = true;
                        break;
                    }
                    Attempt::Committed => {
                        committed = true;
                        break;
                    }
                }
            }
            if base::parse_rule(parser, rules::RIGHT_PARENTHESIS) {
                if committed {
                    Attempt::Committed
                } else {
                    Attempt::Matched
                }
            } else {
                recover_closer(
                    parser,
                    rules::KIND_TUPLE,
                    rules::RIGHT_PARENTHESIS,
                    SyntaxKind::RightParen,
                    ')',
                    ")",
                )
            }
        }) else {
            let result = nesting_limit(parser);
            node.complete(parser, SyntaxKind::KindTuple);
            return result;
        };
        finish(node, parser, SyntaxKind::KindTuple, interior)
    })
}

pub(super) fn parse_kind_scalar(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::KIND_SCALAR, |parser| {
        let node = parser.start();
        if !base::parse_rule(parser, rules::IDENTIFIER) {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let suffix = parser.checkpoint();
        if base::parse_rule(parser, rules::COLON) {
            match precedence::parse_range_expression(parser) {
                Attempt::Matched => {}
                Attempt::Committed => {
                    node.complete(parser, SyntaxKind::KindScalar);
                    return Attempt::Committed;
                }
                Attempt::NoMatch => {
                    parser.rewind(suffix);
                }
            }
        }
        node.complete(parser, SyntaxKind::KindScalar);
        Attempt::Matched
    })
}

fn kind_brace_selection(parser: &mut Parser<'_>) -> Attempt {
    let checkpoint = parser.checkpoint();
    let map = parser.start();
    let set = parser.start();
    let record = parser.start();
    if !base::parse_rule(parser, rules::LEFT_BRACE) {
        parser.rewind(checkpoint);
        return Attempt::NoMatch;
    }
    let Some(result) = parser.with_nesting(|parser| {
        let after_open = parser.checkpoint();
        if base::parse_rule(parser, rules::WHITESPACE1) {
            match kind_record_field(parser) {
                Attempt::Matched => {}
                Attempt::NoMatch => return Attempt::NoMatch,
                Attempt::Committed => {
                    record.complete(parser, SyntaxKind::KindRecord);
                    set.complete(parser, SyntaxKind::KindSet);
                    map.complete(parser, SyntaxKind::KindMap);
                    return Attempt::Committed;
                }
            }
            return finish_selected_kind_record(parser, record, set, map);
        }
        parser.rewind(after_open);

        if base::parse_rule(parser, rules::IDENTIFIER) {
            match parse_kind_annotation(parser) {
                Attempt::Matched => return finish_selected_kind_record(parser, record, set, map),
                Attempt::Committed => {
                    record.complete(parser, SyntaxKind::KindRecord);
                    set.complete(parser, SyntaxKind::KindSet);
                    map.complete(parser, SyntaxKind::KindMap);
                    return Attempt::Committed;
                }
                Attempt::NoMatch => {}
            }
        }
        parser.rewind(after_open);
        record.abandon(parser);

        match parse_kind(parser) {
            Attempt::Matched => {}
            Attempt::NoMatch => return Attempt::NoMatch,
            Attempt::Committed => {
                set.complete(parser, SyntaxKind::KindSet);
                map.complete(parser, SyntaxKind::KindMap);
                return Attempt::Committed;
            }
        }
        if base::parse_rule(parser, rules::COLON) {
            set.abandon(parser);
            match parse_kind(parser) {
                Attempt::Matched => {}
                Attempt::NoMatch => return Attempt::NoMatch,
                Attempt::Committed => {
                    map.complete(parser, SyntaxKind::KindMap);
                    return Attempt::Committed;
                }
            }
            if !base::parse_rule(parser, rules::RIGHT_BRACE) {
                return Attempt::NoMatch;
            }
            map.complete(parser, SyntaxKind::KindMap);
            return Attempt::Matched;
        }
        if !base::parse_rule(parser, rules::RIGHT_BRACE) {
            return Attempt::NoMatch;
        }
        match kind_set_suffix(parser) {
            Attempt::Matched => {
                set.complete(parser, SyntaxKind::KindSet);
                map.abandon(parser);
                Attempt::Matched
            }
            Attempt::Committed => {
                set.complete(parser, SyntaxKind::KindSet);
                map.complete(parser, SyntaxKind::KindMap);
                Attempt::Committed
            }
            Attempt::NoMatch => Attempt::NoMatch,
        }
    }) else {
        let result = nesting_limit(parser);
        record.complete(parser, SyntaxKind::KindRecord);
        set.complete(parser, SyntaxKind::KindSet);
        map.complete(parser, SyntaxKind::KindMap);
        return result;
    };
    if result == Attempt::NoMatch {
        parser.rewind(checkpoint);
    }
    result
}

fn kind_record_interior(parser: &mut Parser<'_>) -> Attempt {
    if !base::parse_rule(parser, rules::WHITESPACE0) {
        return Attempt::NoMatch;
    }
    match kind_record_field(parser) {
        Attempt::Matched => {}
        other => return other,
    }
    finish_kind_record_fields(parser)
}

fn finish_kind_record_fields(parser: &mut Parser<'_>) -> Attempt {
    loop {
        let pair = parser.checkpoint();
        if !base::parse_rule(parser, rules::LIST_SEPARATOR)
            && !base::parse_rule(parser, rules::WHITESPACE1)
        {
            break;
        }
        match kind_record_field(parser) {
            Attempt::Matched => {}
            Attempt::NoMatch => {
                parser.rewind(pair);
                break;
            }
            Attempt::Committed => return Attempt::Committed,
        }
    }
    let _ = base::parse_exact_tag(parser, ",…", SyntaxKind::Text);
    let _ = base::parse_rule(parser, rules::WHITESPACE0);
    if !base::parse_rule(parser, rules::RIGHT_BRACE) {
        return recover_closer(
            parser,
            rules::KIND_RECORD,
            rules::RIGHT_BRACE,
            SyntaxKind::RightBrace,
            '}',
            "}",
        );
    }
    Attempt::Matched
}

fn finish_selected_kind_record(
    parser: &mut Parser<'_>,
    record: super::super::super::marker::Marker,
    set: super::super::super::marker::Marker,
    map: super::super::super::marker::Marker,
) -> Attempt {
    match finish_kind_record_fields(parser) {
        Attempt::Matched => {
            record.complete(parser, SyntaxKind::KindRecord);
            set.abandon(parser);
            map.abandon(parser);
            Attempt::Matched
        }
        Attempt::NoMatch => Attempt::NoMatch,
        Attempt::Committed => {
            record.complete(parser, SyntaxKind::KindRecord);
            set.complete(parser, SyntaxKind::KindSet);
            map.complete(parser, SyntaxKind::KindMap);
            Attempt::Committed
        }
    }
}

fn kind_set_suffix(parser: &mut Parser<'_>) -> Attempt {
    let literal_suffix = parser.checkpoint();
    if base::parse_rule(parser, rules::COLON) {
        match literals::parse_literal(parser) {
            Attempt::Matched => {}
            Attempt::Committed => return Attempt::Committed,
            Attempt::NoMatch => parser.rewind(literal_suffix),
        }
    }
    let _ = base::parse_exact_tag(parser, ":N", SyntaxKind::Text);
    Attempt::Matched
}

fn kind_record_field(parser: &mut Parser<'_>) -> Attempt {
    if !base::parse_rule(parser, rules::IDENTIFIER) {
        return Attempt::NoMatch;
    }
    parse_kind_annotation(parser)
}

fn kind_table_field(parser: &mut Parser<'_>) -> Attempt {
    if !base::parse_rule(parser, rules::IDENTIFIER) {
        return Attempt::NoMatch;
    }
    match parse_kind_annotation(parser) {
        Attempt::Committed => Attempt::Committed,
        Attempt::Matched | Attempt::NoMatch => Attempt::Matched,
    }
}

fn delimited_kind(
    parser: &mut Parser<'_>,
    rule: crate::document::RuleId,
    open: crate::document::RuleId,
    close: crate::document::RuleId,
    kind: SyntaxKind,
    content: fn(&mut Parser<'_>) -> Attempt,
) -> Attempt {
    combinator::transactional(parser, rule, |parser| {
        let node = parser.start();
        if !base::parse_rule(parser, open) {
            node.abandon(parser);
            return Attempt::NoMatch;
        }
        let Some(interior) = parser.with_nesting(|parser| {
            let mut committed = false;
            match content(parser) {
                Attempt::Matched => {}
                Attempt::NoMatch => {
                    recover_required_production(
                        parser,
                        rule,
                        "syntax/missing-delimited-kind",
                        "missing kind after opening delimiter",
                        "kind",
                    );
                    committed = true;
                }
                Attempt::Committed => committed = true,
            }
            if base::parse_rule(parser, close) {
                if committed {
                    Attempt::Committed
                } else {
                    Attempt::Matched
                }
            } else {
                recover_closer(parser, rule, close, SyntaxKind::RightAngle, '>', ">")
            }
        }) else {
            let result = nesting_limit(parser);
            node.complete(parser, kind);
            return result;
        };
        finish(node, parser, kind, interior)
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
