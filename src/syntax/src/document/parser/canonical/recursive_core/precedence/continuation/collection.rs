//! Tuple and set bodies share retained value-list transitions.
use super::*;
pub(super) fn supports(rule: RuleId) -> bool {
    matches!(
        rule,
        rules::STRUCTURE
            | rules::SET
            | rules::TUPLE
            | rules::TUPLE_STRUCT
            | rules::FIELD
            | rules::HEADER_FIELD
    )
}
#[derive(Clone, Copy)]
pub(super) struct List {
    rule: RuleId,
    kind: SyntaxKind,
    close: RuleId,
    close_kind: SyntaxKind,
    text: &'static str,
}
impl List {
    fn new(rule: RuleId) -> Self {
        let kind = match rule {
            rules::SET => SyntaxKind::Set,
            rules::TUPLE => SyntaxKind::Tuple,
            _ => SyntaxKind::TupleStruct,
        };
        let (close, close_kind, text) = if rule == rules::SET {
            (rules::RIGHT_BRACE, SyntaxKind::RightBrace, "}")
        } else {
            (rules::RIGHT_PARENTHESIS, SyntaxKind::RightParen, ")")
        };
        Self {
            rule,
            kind,
            close,
            close_kind,
            text,
        }
    }
}
pub(super) enum Phase {
    Structure(Marker, usize),
    StructureNext(Marker, usize),
    Enter(RuleId),
    FieldName(Marker, bool),
    FieldAnnotation(Marker, bool),
    Prefix(Marker, List),
    Name(Marker, List),
    Open(Marker, List),
    Finish(Marker, SyntaxKind),
    Space(List),
    Empty(List),
    Item(List, bool, bool),
    Recovered(List),
    Loop(List, bool),
    Separator(List, bool),
    SpaceSeparator(List, bool),
    Trailing(List, bool),
    Close(List, bool),
    Colon,
    ColonTuple,
    ColonLiteral,
}
impl Continuation {
    fn collection(&mut self, phase: Phase) {
        self.push(Frame::Collection(Box::new(phase)));
    }
    fn collection_required(&mut self, spec: List, after: bool) {
        let (code, msg) = match spec.rule {
            rules::SET => (
                "syntax/missing-set-item",
                if after {
                    "missing set item after separator"
                } else {
                    "missing set item"
                },
            ),
            rules::TUPLE => (
                "syntax/missing-tuple-item",
                if after {
                    "missing tuple item after separator"
                } else {
                    "missing tuple item"
                },
            ),
            _ => (
                "syntax/missing-tuple-structure-value",
                "missing tuple structure value",
            ),
        };
        self.push(Frame::Required(Box::new(Required::new(
            spec.rule,
            code,
            msg,
            "expression",
            &[],
            &[],
            None,
        ))));
    }
    #[inline(never)]
    pub(super) fn collection_frame(&mut self, parser: &mut Parser<'_>, phase: Box<Phase>) {
        let phase = *phase;
        match phase {
            Phase::Structure(node, index) => {
                self.collection(Phase::StructureNext(node, index));
                if index < 2 {
                    self.push(Frame::Shell(Box::new(
                        super::super::super::super::structure_shell::Continuation::new(
                            if index == 0 {
                                rules::EMPTY_SET
                            } else {
                                rules::EMPTY_MAP
                            },
                        ),
                    )));
                } else {
                    self.push(Frame::Call(
                        [
                            rules::TABLE,
                            rules::MATRIX,
                            rules::TUPLE,
                            rules::TUPLE_STRUCT,
                            rules::RECORD,
                            rules::MAP,
                            rules::SET,
                        ][index - 2],
                    ));
                }
            }
            Phase::StructureNext(node, index) => {
                if self.result == Attempt::NoMatch && index < 8 {
                    self.collection(Phase::Structure(node, index + 1));
                } else if let Some(result) = super::super::super::child_result(
                    parser,
                    node,
                    SyntaxKind::Structure,
                    self.result,
                ) {
                    self.result = result;
                } else {
                    node.complete(parser, SyntaxKind::Structure);
                    self.result = Attempt::Matched;
                }
            }
            Phase::Enter(rule) => {
                self.transaction(parser, rule);
                let node = parser.start();
                if rule == rules::STRUCTURE {
                    self.collection(Phase::Structure(node, 0));
                    return;
                }

                if matches!(rule, rules::FIELD | rules::HEADER_FIELD) {
                    self.collection(Phase::FieldName(node, rule == rules::HEADER_FIELD));
                    self.base(rules::IDENTIFIER);
                } else {
                    let spec = List::new(rule);
                    if rule == rules::TUPLE_STRUCT {
                        self.collection(Phase::Prefix(node, spec));
                        self.base(rules::COLON);
                    } else {
                        self.collection(Phase::Open(node, spec));
                        self.base(if rule == rules::SET {
                            rules::LEFT_BRACE
                        } else {
                            rules::LEFT_PARENTHESIS
                        });
                    }
                }
            }
            Phase::FieldName(node, required) => {
                if self.result != Attempt::Matched {
                    if !parser.state.resource_finalizing {
                        node.abandon(parser);
                    }
                    self.result = Attempt::NoMatch;
                } else {
                    self.collection(Phase::FieldAnnotation(node, required));
                    self.push(Frame::Call(rules::KIND_ANNOTATION));
                }
            }
            Phase::FieldAnnotation(node, required) => {
                let kind = if required {
                    SyntaxKind::HeaderField
                } else {
                    SyntaxKind::TableField
                };
                if required {
                    if let Some(result) =
                        super::super::super::child_result(parser, node, kind, self.result)
                    {
                        self.result = result;
                    } else {
                        node.complete(parser, kind);
                        self.result = Attempt::Matched;
                    }
                } else {
                    node.complete(parser, kind);
                    if self.result != Attempt::Committed {
                        self.result = Attempt::Matched;
                    }
                }
            }
            Phase::Prefix(node, spec) => {
                if self.result == Attempt::Matched {
                    self.collection(Phase::Name(node, spec));
                    self.base(rules::IDENTIFIER);
                } else {
                    self.result = Attempt::NoMatch;
                    self.collection(Phase::Finish(node, spec.kind));
                }
            }
            Phase::Name(node, spec) => {
                if self.result == Attempt::Matched {
                    self.collection(Phase::Open(node, spec));
                    self.base(rules::LEFT_PARENTHESIS);
                } else {
                    self.result = Attempt::NoMatch;
                    self.collection(Phase::Finish(node, spec.kind));
                }
            }
            Phase::Open(node, spec) => {
                self.collection(Phase::Finish(node, spec.kind));
                if self.result != Attempt::Matched {
                    self.result = Attempt::NoMatch;
                } else if parser.push_nesting() {
                    self.push(Frame::PopNesting);
                    self.collection(Phase::Space(spec));
                    self.base(rules::WHITESPACE0);
                } else {
                    self.push(Frame::Nesting(Box::new(NestingContinuation::new())));
                }
            }
            Phase::Finish(node, kind) => {
                if self.result == Attempt::NoMatch {
                    if !parser.state.resource_finalizing {
                        node.abandon(parser);
                    }
                } else {
                    node.complete(parser, kind);
                }
            }
            Phase::Space(spec) => {
                if self.result != Attempt::Matched {
                    self.result = Attempt::NoMatch;
                } else if spec.rule == rules::TUPLE {
                    self.collection(Phase::Empty(spec));
                    self.base(spec.close);
                } else {
                    self.collection(Phase::Item(spec, false, false));
                    self.push(Frame::Call(rules::EXPRESSION));
                }
            }
            Phase::Empty(spec) => {
                if self.result != Attempt::Matched {
                    self.collection(Phase::Item(spec, false, false));
                    self.push(Frame::Call(rules::EXPRESSION));
                }
            }
            Phase::Item(spec, committed, after) => {
                if self.result == Attempt::NoMatch {
                    if spec.rule == rules::SET && !after && !parser.cursor().starts_with(",") {
                        return;
                    }
                    self.collection(Phase::Recovered(spec));
                    self.collection_required(spec, after);
                } else {
                    self.collection(Phase::Loop(
                        spec,
                        committed || self.result == Attempt::Committed,
                    ));
                }
            }
            Phase::Recovered(spec) => self.collection(Phase::Loop(spec, true)),
            Phase::Loop(spec, committed) => {
                if spec.rule == rules::TUPLE_STRUCT || parser.is_halted() {
                    self.collection(Phase::Trailing(spec, committed));
                    self.base(rules::WHITESPACE0);
                } else {
                    self.collection(Phase::Separator(spec, committed));
                    self.base(rules::LIST_SEPARATOR);
                }
            }
            Phase::Separator(spec, committed) => {
                if self.result == Attempt::Matched {
                    self.collection(Phase::Item(spec, committed, true));
                    self.push(Frame::Call(rules::EXPRESSION));
                } else if spec.rule == rules::SET {
                    self.collection(Phase::SpaceSeparator(spec, committed));
                    self.base(rules::WHITESPACE1);
                } else {
                    self.collection(Phase::Trailing(spec, committed));
                    self.base(rules::WHITESPACE0);
                }
            }
            Phase::SpaceSeparator(spec, committed) => {
                if self.result == Attempt::Matched {
                    self.collection(Phase::Item(spec, committed, true));
                    self.push(Frame::Call(rules::EXPRESSION));
                } else {
                    self.collection(Phase::Trailing(spec, committed));
                    self.base(rules::WHITESPACE0);
                }
            }
            Phase::Trailing(spec, committed) => {
                self.collection(Phase::Close(spec, committed));
                self.base(spec.close);
            }
            Phase::Close(spec, committed) => {
                if self.result == Attempt::Matched {
                    self.result = if committed {
                        Attempt::Committed
                    } else {
                        Attempt::Matched
                    };
                } else {
                    self.push(Frame::Closer(Box::new(Closer::new(
                        spec.rule,
                        spec.close,
                        spec.close_kind,
                        spec.text,
                    ))));
                }
            }
            Phase::Colon => {
                // The selected structure wrapper is owned by the same checkpoint.
                let checkpoint = parser.checkpoint();
                let node = parser.start();
                self.collection(Phase::ColonTuple);
                self.push(Frame::CollectionWrap(node, checkpoint));
                self.push(Frame::Call(rules::TUPLE_STRUCT));
            }
            Phase::ColonTuple => {
                if self.result == Attempt::NoMatch {
                    self.collection(Phase::ColonLiteral);
                    self.push(Frame::Call(rules::LITERAL));
                }
            }
            Phase::ColonLiteral => {}
        }
    }
}
