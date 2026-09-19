//! Table headers and rows retain separator candidates and recovery ownership.
use super::*;
use alloc::string::String;
pub(super) fn supports(rule: RuleId) -> bool {
    matches!(
        rule,
        rules::FANCY_TABLE_HEADER
            | rules::TABLE_HEADER
            | rules::INLINE_TABLE_HEADER
            | rules::TABLE_ROW
            | rules::TABLE_ROW2
    )
}
#[derive(Clone, Copy)]
pub(super) struct Row {
    rule: RuleId,
    kind: SyntaxKind,
    header: bool,
    fancy: bool,
    trailing: RuleId,
}
impl Row {
    fn new(rule: RuleId) -> Self {
        let (kind, header, fancy, trailing) = match rule {
            rules::FANCY_TABLE_HEADER => {
                (SyntaxKind::FancyTableHeader, true, true, rules::WHITESPACE0)
            }
            rules::TABLE_HEADER => (SyntaxKind::TableHeader, true, false, rules::WHITESPACE0),
            rules::INLINE_TABLE_HEADER => (
                SyntaxKind::InlineTableHeader,
                true,
                false,
                rules::SPACE_TAB0,
            ),
            rules::TABLE_ROW => (SyntaxKind::TableRow, false, false, rules::SPACE_TAB0),
            _ => (SyntaxKind::FancyTableRow, false, true, rules::SPACE_TAB0),
        };
        Self {
            rule,
            kind,
            header,
            fancy,
            trailing,
        }
    }
    fn child(self) -> RuleId {
        if !self.header {
            rules::EXPRESSION
        } else if self.fancy {
            rules::FIELD
        } else {
            rules::HEADER_FIELD
        }
    }
}
pub(super) enum Phase {
    Enter(RuleId),
    Finish(Marker, Row),
    Leading(Row),
    Open(Row),
    EmptyProbe(Row, ParserCheckpoint),
    First(Row, Option<ParserCheckpoint>),
    FirstRecovered(Row),
    EmptyRecovered(Row),
    Loop(Row, bool),
    Pair(Row, bool, ParserCheckpoint, usize),
    Space(Row, bool, ParserCheckpoint),
    SpaceItem(Row, bool, ParserCheckpoint),
    Item(Row, bool, ParserCheckpoint),
    Trailing(Row, bool),
    Close(Row, bool),
    Done(Row, bool),
}
impl Continuation {
    fn row(&mut self, phase: Phase) {
        self.push(Frame::TableRow(Box::new(phase)));
    }
    pub(super) fn table_separator(&mut self) {
        self.push(Frame::Shell(Box::new(
            super::super::super::super::structure_shell::Continuation::new(rules::TABLE_SEPARATOR),
        )));
    }
    pub(super) fn recover_table_separator(&mut self, rule: RuleId) {
        self.push(Frame::Closer(Box::new(Closer::set(
            rule,
            rules::TABLE_SEPARATOR,
            SyntaxKind::Bar,
            "|",
            &['|', '│', '┃', '\n', '\r', ')', ']', '}'],
        ))));
    }
    fn row_trailing(&mut self, spec: Row, committed: bool) {
        if spec.header && spec.fancy {
            self.row(Phase::Close(spec, committed));
            self.table_separator();
        } else {
            self.row(Phase::Trailing(spec, committed));
            self.base(rules::SPACE_TAB0);
        }
    }
    #[inline(never)]
    pub(super) fn row_frame(&mut self, parser: &mut Parser<'_>, phase: Box<Phase>) {
        match *phase {
            Phase::Enter(rule) => {
                self.transaction(parser, rule);
                let spec = Row::new(rule);
                let node = parser.start();
                self.row(Phase::Finish(node, spec));
                if spec.header {
                    self.row(Phase::First(spec, None));
                    self.push(Frame::Call(spec.child()));
                } else {
                    self.row(Phase::Leading(spec));
                    self.base(rules::SPACE_TAB0);
                }
            }
            Phase::Finish(node, spec) => {
                if self.result == Attempt::NoMatch {
                    if !parser.state.resource_finalizing {
                        node.abandon(parser);
                    }
                } else {
                    node.complete(parser, spec.kind);
                }
            }
            Phase::Leading(spec) => {
                self.row(Phase::Open(spec));
                self.table_separator();
            }
            Phase::Open(spec) => {
                if self.result != Attempt::Matched {
                    self.result = Attempt::NoMatch;
                } else if spec.fancy {
                    self.row(Phase::First(spec, None));
                    self.push(Frame::Call(spec.child()));
                } else {
                    self.row(Phase::EmptyProbe(spec, parser.checkpoint()));
                    self.base(rules::SPACE_TAB0);
                }
            }
            Phase::EmptyProbe(spec, checkpoint) => {
                let empty = parser.cursor().starts_with("\n") || parser.cursor().starts_with("\r");
                parser.rewind(checkpoint);
                if empty {
                    self.row(Phase::EmptyRecovered(spec));
                    self.push(Frame::Missing(Box::new(
                        crate::document::parser::recovery::MissingContinuation::new(
                            "syntax/missing-table-cell",
                            "missing table row cell after separator",
                            crate::document::ExpectedSyntax::Production(String::from("expression")),
                            None,
                        ),
                    )));
                } else {
                    self.row(Phase::First(spec, Some(parser.checkpoint())));
                    self.push(Frame::Call(spec.child()));
                }
            }
            Phase::First(spec, checkpoint) => {
                if self.result == Attempt::NoMatch {
                    if spec.header {
                        return;
                    }
                    if let Some(checkpoint) = checkpoint {
                        parser.rewind(checkpoint);
                    }
                    self.row(Phase::FirstRecovered(spec));
                    self.push(Frame::Required(Box::new(Required::new(
                        spec.rule,
                        "syntax/missing-table-cell",
                        "missing table row cell after separator",
                        "expression",
                        &[],
                        &[],
                        None,
                    ))));
                } else {
                    self.row(Phase::Loop(spec, self.result == Attempt::Committed));
                }
            }
            Phase::FirstRecovered(spec) => {
                if spec.fancy {
                    self.row(Phase::Loop(spec, true));
                } else {
                    self.row(Phase::Done(spec, true));
                    self.recover_table_separator(spec.rule);
                }
            }
            Phase::EmptyRecovered(spec) => {
                self.row(Phase::Done(spec, true));
                self.recover_table_separator(spec.rule);
            }
            Phase::Loop(spec, committed) => {
                if parser.is_halted() {
                    self.row_trailing(spec, committed);
                } else {
                    self.row(Phase::Pair(spec, committed, parser.checkpoint(), 0));
                    if spec.fancy && spec.header {
                        self.table_separator();
                    } else {
                        self.base(if spec.fancy {
                            rules::SPACE_TAB0
                        } else {
                            rules::SPACE_TAB1
                        });
                    }
                }
            }
            Phase::Pair(spec, committed, checkpoint, index) => {
                if self.result != Attempt::Matched {
                    if spec.fancy && !spec.header {
                        parser.rewind(checkpoint);
                        if index > 0 {
                            self.row(Phase::Space(spec, committed, checkpoint));
                            self.base(rules::SPACE_TAB1);
                            return;
                        }
                    }
                    self.row_trailing(spec, committed);
                } else if spec.fancy
                    && !spec.header
                    && index == 2
                    && (parser.is_eof()
                        || parser.cursor().starts_with("\n")
                        || parser.cursor().starts_with("\r"))
                {
                    parser.rewind(checkpoint);
                    self.row_trailing(spec, committed);
                } else if !spec.fancy && !spec.header && parser.cursor().starts_with("|") {
                    parser.rewind(checkpoint);
                    self.row_trailing(spec, committed);
                } else if spec.fancy && !spec.header && index < 2 {
                    self.row(Phase::Pair(spec, committed, checkpoint, index + 1));
                    if index == 0 {
                        self.table_separator();
                    } else {
                        self.base(rules::SPACE_TAB0);
                    }
                } else {
                    self.row(Phase::Item(spec, committed, checkpoint));
                    self.push(Frame::Call(spec.child()));
                }
            }
            Phase::Space(spec, committed, checkpoint) => {
                if self.result == Attempt::Matched {
                    self.row(Phase::SpaceItem(spec, committed, checkpoint));
                    self.push(Frame::Call(spec.child()));
                } else {
                    parser.rewind(checkpoint);
                    self.row_trailing(spec, committed);
                }
            }
            Phase::SpaceItem(spec, committed, checkpoint) => {
                if self.result == Attempt::NoMatch {
                    parser.rewind(checkpoint);
                    self.row_trailing(spec, committed);
                } else {
                    self.row(Phase::Loop(
                        spec,
                        committed || self.result == Attempt::Committed,
                    ));
                }
            }
            Phase::Item(spec, committed, checkpoint) => {
                if self.result == Attempt::NoMatch {
                    parser.rewind(checkpoint);
                    self.row_trailing(spec, committed);
                } else {
                    self.row(Phase::Loop(
                        spec,
                        committed || self.result == Attempt::Committed,
                    ));
                }
            }
            Phase::Trailing(spec, committed) => {
                self.row(Phase::Close(spec, committed));
                self.table_separator();
            }
            Phase::Close(spec, committed) => {
                if self.result == Attempt::Matched {
                    self.row(Phase::Done(spec, committed));
                    self.base(spec.trailing);
                } else {
                    self.row(Phase::Done(spec, true));
                    self.recover_table_separator(spec.rule);
                }
            }
            Phase::Done(spec, committed) => {
                self.result = if committed || (!spec.fancy && parser.is_halted()) {
                    Attempt::Committed
                } else {
                    Attempt::Matched
                };
            }
        }
    }
}
