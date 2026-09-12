use crate::document::SyntaxKind;

use super::super::super::Parser;
use super::super::super::rule::rules;
use super::super::{base, combinator, kinds as leaves};
use super::{
    Attempt, child_result, finish_provisional_marker, literals, nesting_limit, precedence,
    recover_closer, recover_required_production,
};

pub(super) fn parse_kind_annotation(parser: &mut Parser<'_>) -> Attempt {
    kind_annotation(parser, true)
}

pub(crate) fn parse_kind_annotation_candidate(parser: &mut Parser<'_>) -> Attempt {
    kind_annotation(parser, false)
}

fn kind_annotation(parser: &mut Parser<'_>, recover: bool) -> Attempt {
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
                Attempt::NoMatch | Attempt::Committed if !recover => return Attempt::NoMatch,
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
            } else if !recover {
                Attempt::NoMatch
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

#[derive(Clone, Copy)]
enum KindPosition {
    Ordinary,
    MapKey,
    BraceElement,
}

pub(super) fn parse_kind(parser: &mut Parser<'_>) -> Attempt {
    kind(parser, KindPosition::Ordinary)
}

fn kind(parser: &mut Parser<'_>, position: KindPosition) -> Attempt {
    combinator::transactional(parser, rules::KIND, |parser| {
        let node = parser.start();
        let selected = if parser.cursor().starts_with("{") {
            kind_brace_selection(parser)
        } else if parser.cursor().starts_with("[") {
            kind_matrix(parser, position)
        } else {
            let mut selected = Attempt::NoMatch;
            for parse in [
                leaves::parse_kind_any,
                leaves::parse_kind_atom,
                leaves::parse_kind_empty,
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
        if child == Attempt::NoMatch {
            node.abandon(parser);
            return child;
        }
        if !parser.is_halted() {
            let _ = base::parse_rule(parser, rules::QUESTION);
        }
        node.complete(parser, SyntaxKind::KindWithOption);
        child
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
                }
                Attempt::Committed => {
                    committed = true;
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
        if interior == Attempt::NoMatch || parser.is_halted() {
            return finish(node, parser, SyntaxKind::KindSet, interior);
        }
        let suffix = kind_set_suffix(parser);
        node.complete(parser, SyntaxKind::KindSet);
        if interior == Attempt::Committed || suffix == Attempt::Committed {
            Attempt::Committed
        } else {
            Attempt::Matched
        }
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
            let committed = match kind(parser, KindPosition::MapKey) {
                Attempt::Matched => false,
                Attempt::Committed if parser.is_halted() => return Attempt::Committed,
                Attempt::Committed => true,
                Attempt::NoMatch => return Attempt::NoMatch,
            };
            if !base::parse_rule(parser, rules::COLON) {
                return Attempt::NoMatch;
            }
            finish_kind_map_value(parser, committed)
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
    kind_matrix(parser, KindPosition::Ordinary)
}

fn kind_matrix(parser: &mut Parser<'_>, position: KindPosition) -> Attempt {
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
        if interior == Attempt::NoMatch || parser.is_halted() {
            return finish(node, parser, SyntaxKind::KindMatrix, interior);
        }
        let suffix = parser.checkpoint();
        let colon = base::parse_rule(parser, rules::COLON);
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
            Attempt::NoMatch => {
                // A bare colon is valid matrix syntax. Only an enclosing map
                // candidate may reserve it as the key/value separator. Shared
                // braces still accept a clean set containing `[u8]:`.
                let map_separator = !parser.cursor().starts_with(":")
                    && match position {
                        KindPosition::Ordinary => false,
                        KindPosition::MapKey => true,
                        KindPosition::BraceElement => {
                            interior == Attempt::Committed || !parser.cursor().starts_with("}")
                        }
                    };
                if colon && map_separator {
                    parser.rewind(suffix);
                }
            }
            Attempt::Committed => {
                node.complete(parser, SyntaxKind::KindMatrix);
                return Attempt::Committed;
            }
        }
        node.complete(parser, SyntaxKind::KindMatrix);
        interior
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
                    }
                    Attempt::Committed => {
                        committed = true;
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
            let committed = match kind_record_field(parser) {
                Attempt::Matched => false,
                Attempt::NoMatch => return Attempt::NoMatch,
                Attempt::Committed => true,
            };
            return finish_selected_kind_record(parser, record, set, map, committed);
        }
        parser.rewind(after_open);

        if base::parse_rule(parser, rules::IDENTIFIER) {
            match parse_kind_annotation(parser) {
                Attempt::Matched => {
                    return finish_selected_kind_record(parser, record, set, map, false);
                }
                Attempt::Committed => {
                    return finish_selected_kind_record(parser, record, set, map, true);
                }
                Attempt::NoMatch => {}
            }
        }
        parser.rewind(after_open);
        finish_provisional_marker(parser, record, SyntaxKind::KindRecord);

        let scalar_map = parser.checkpoint();
        if parse_plain_scalar_kind(parser) == Attempt::Matched
            && base::parse_rule(parser, rules::COLON)
        {
            match parse_kind(parser) {
                Attempt::Matched => {
                    if !base::parse_rule(parser, rules::RIGHT_BRACE) {
                        recover_closer(
                            parser,
                            rules::KIND_MAP,
                            rules::RIGHT_BRACE,
                            SyntaxKind::RightBrace,
                            '}',
                            "}",
                        );
                        finish_provisional_marker(parser, set, SyntaxKind::KindSet);
                        map.complete(parser, SyntaxKind::KindMap);
                        return Attempt::Committed;
                    }
                    finish_provisional_marker(parser, set, SyntaxKind::KindSet);
                    map.complete(parser, SyntaxKind::KindMap);
                    return Attempt::Matched;
                }
                Attempt::Committed => {
                    recover_closer(
                        parser,
                        rules::KIND_MAP,
                        rules::RIGHT_BRACE,
                        SyntaxKind::RightBrace,
                        '}',
                        "}",
                    );
                    finish_provisional_marker(parser, set, SyntaxKind::KindSet);
                    map.complete(parser, SyntaxKind::KindMap);
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
                    finish_provisional_marker(parser, set, SyntaxKind::KindSet);
                    map.complete(parser, SyntaxKind::KindMap);
                    return Attempt::Committed;
                }
            }
        }
        parser.rewind(scalar_map);

        let committed = match kind(parser, KindPosition::BraceElement) {
            Attempt::Matched => false,
            Attempt::NoMatch => return Attempt::NoMatch,
            Attempt::Committed => true,
        };
        if !parser.is_halted() && base::parse_rule(parser, rules::COLON) {
            finish_provisional_marker(parser, set, SyntaxKind::KindSet);
            let result = finish_kind_map_value(parser, committed);
            map.complete(parser, SyntaxKind::KindMap);
            return result;
        }
        if committed {
            recover_closer(
                parser,
                rules::KIND_SET,
                rules::RIGHT_BRACE,
                SyntaxKind::RightBrace,
                '}',
                "}",
            );
            let _ = kind_set_suffix(parser);
            set.complete(parser, SyntaxKind::KindSet);
            finish_provisional_marker(parser, map, SyntaxKind::KindMap);
            return Attempt::Committed;
        }
        if !base::parse_rule(parser, rules::RIGHT_BRACE) {
            return Attempt::NoMatch;
        }
        match kind_set_suffix(parser) {
            Attempt::Matched => {
                set.complete(parser, SyntaxKind::KindSet);
                finish_provisional_marker(parser, map, SyntaxKind::KindMap);
                Attempt::Matched
            }
            Attempt::Committed => {
                set.complete(parser, SyntaxKind::KindSet);
                finish_provisional_marker(parser, map, SyntaxKind::KindMap);
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

// The colon selects the map even when the key needed recovery. Shared brace
// selection and the direct rule must preserve the same value and closer.
fn finish_kind_map_value(parser: &mut Parser<'_>, mut committed: bool) -> Attempt {
    match parse_kind(parser) {
        Attempt::Matched => {}
        Attempt::Committed => committed = true,
        Attempt::NoMatch => {
            recover_required_production(
                parser,
                rules::KIND_MAP,
                "syntax/missing-kind-map-value",
                "missing map value kind after colon",
                "kind",
            );
            committed = true;
        }
    }
    if !base::parse_rule(parser, rules::RIGHT_BRACE) {
        return recover_closer(
            parser,
            rules::KIND_MAP,
            rules::RIGHT_BRACE,
            SyntaxKind::RightBrace,
            '}',
            "}",
        );
    }
    if committed {
        Attempt::Committed
    } else {
        Attempt::Matched
    }
}

fn parse_plain_scalar_kind(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::KIND, |parser| {
        let kind = parser.start();
        let scalar = combinator::transactional(parser, rules::KIND_SCALAR, |parser| {
            let scalar = parser.start();
            if !base::parse_rule(parser, rules::IDENTIFIER) {
                scalar.abandon(parser);
                return Attempt::NoMatch;
            }
            scalar.complete(parser, SyntaxKind::KindScalar);
            Attempt::Matched
        });
        if scalar == Attempt::NoMatch {
            kind.abandon(parser);
            return Attempt::NoMatch;
        }
        kind.complete(parser, SyntaxKind::Kind);
        Attempt::Matched
    })
}

fn kind_record_interior(parser: &mut Parser<'_>) -> Attempt {
    if !base::parse_rule(parser, rules::WHITESPACE0) {
        return Attempt::NoMatch;
    }
    let committed = match kind_record_field(parser) {
        Attempt::Matched => false,
        Attempt::Committed => true,
        Attempt::NoMatch => return Attempt::NoMatch,
    };
    finish_kind_record_fields(parser, committed)
}

fn finish_kind_record_fields(parser: &mut Parser<'_>, mut committed: bool) -> Attempt {
    while !parser.is_halted() {
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
            Attempt::Committed => committed = true,
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
    if committed {
        Attempt::Committed
    } else {
        Attempt::Matched
    }
}

fn finish_selected_kind_record(
    parser: &mut Parser<'_>,
    record: super::super::super::marker::Marker,
    set: super::super::super::marker::Marker,
    map: super::super::super::marker::Marker,
    committed: bool,
) -> Attempt {
    match finish_kind_record_fields(parser, committed) {
        Attempt::Matched => {
            record.complete(parser, SyntaxKind::KindRecord);
            finish_provisional_marker(parser, set, SyntaxKind::KindSet);
            finish_provisional_marker(parser, map, SyntaxKind::KindMap);
            Attempt::Matched
        }
        Attempt::NoMatch => Attempt::NoMatch,
        Attempt::Committed => {
            record.complete(parser, SyntaxKind::KindRecord);
            finish_provisional_marker(parser, set, SyntaxKind::KindSet);
            finish_provisional_marker(parser, map, SyntaxKind::KindMap);
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
