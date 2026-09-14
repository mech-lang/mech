//! Inline table items retain enclosing-separator speculation.
use super::*;
pub(super) enum Phase {
    Enter,
    Finish(Marker),
    First,
    AbsentProbe(ParserCheckpoint),
    Required,
    RequiredClose,
    Item,
    ItemSpace(ParserCheckpoint),
    ItemEnd(ParserCheckpoint),
    Loop(bool),
    Probe(bool, TextSize, ParserCheckpoint),
    ProbeEnd(bool, TextSize, ParserCheckpoint),
    Next(bool, TextSize, bool, ParserCheckpoint),
    SeparatorProbe,
    SeparatorSpace(ParserCheckpoint),
    SeparatorEnd(ParserCheckpoint),
    Trailing(bool),
    Close(bool),
    Done(bool),
}
impl Continuation {
    fn inline(&mut self, phase: Phase) {
        self.push(Frame::Inline(Box::new(phase)));
    }
    #[inline(never)]
    pub(super) fn inline_frame(&mut self, parser: &mut Parser<'_>, phase: Box<Phase>) {
        match *phase {
            Phase::Enter => {
                self.transaction(parser, rules::INLINE_TABLE_ROW);
                let node = parser.start();
                self.inline(Phase::Finish(node));
                self.inline(Phase::First);
                self.inline(Phase::Item);
            }
            Phase::Finish(node) => {
                if self.result == Attempt::NoMatch {
                    if !parser.state.resource_finalizing {
                        node.abandon(parser);
                    }
                } else {
                    node.complete(parser, SyntaxKind::InlineTableRow);
                }
            }
            Phase::First => {
                if self.result == Attempt::NoMatch {
                    self.inline(Phase::AbsentProbe(parser.checkpoint()));
                    self.table_separator();
                } else {
                    self.inline(Phase::Loop(self.result == Attempt::Committed));
                }
            }
            Phase::AbsentProbe(checkpoint) => {
                let matched = self.result.accepted();
                parser.rewind(checkpoint);
                if matched {
                    self.inline(Phase::Required);
                    self.push(Frame::Required(Box::new(Required::new(
                        rules::INLINE_TABLE_ROW,
                        "syntax/missing-table-cell",
                        "missing inline table cell",
                        "expression",
                        &[],
                        &[],
                        None,
                    ))));
                } else {
                    self.result = Attempt::NoMatch;
                }
            }
            Phase::Required => {
                self.inline(Phase::RequiredClose);
                self.recover_table_separator(rules::INLINE_TABLE_ROW);
            }
            Phase::RequiredClose => self.result = Attempt::Committed,
            Phase::Item => {
                self.inline(Phase::ItemSpace(parser.checkpoint()));
                self.base(rules::SPACE_TAB0);
            }
            Phase::ItemSpace(checkpoint) => {
                if self.result != Attempt::Matched {
                    parser.rewind(checkpoint);
                    self.result = Attempt::NoMatch;
                } else {
                    self.inline(Phase::ItemEnd(checkpoint));
                    self.push(Frame::Call(rules::EXPRESSION));
                }
            }
            Phase::ItemEnd(checkpoint) => {
                if self.result == Attempt::NoMatch {
                    parser.rewind(checkpoint);
                }
            }
            Phase::Loop(committed) => {
                if parser.is_halted() {
                    self.inline(Phase::Trailing(committed));
                    self.base(rules::SPACE_TAB0);
                } else {
                    self.inline(Phase::Probe(
                        committed,
                        parser.offset(),
                        parser.checkpoint(),
                    ));
                    self.base(rules::SPACE_TAB0);
                }
            }
            Phase::Probe(committed, before, checkpoint) => {
                if self.result == Attempt::Matched {
                    self.inline(Phase::ProbeEnd(committed, before, checkpoint));
                    self.table_separator();
                } else {
                    self.inline(Phase::ProbeEnd(committed, before, checkpoint));
                    self.result = Attempt::NoMatch;
                }
            }
            Phase::ProbeEnd(committed, before, checkpoint) => {
                let separator = self.result.accepted();
                parser.rewind(checkpoint);
                if parser.is_halted() {
                    self.result = Attempt::Committed;
                } else {
                    self.inline(Phase::Next(
                        committed,
                        before,
                        separator,
                        parser.checkpoint(),
                    ));
                    self.inline(Phase::Item);
                }
            }
            Phase::Next(mut committed, before, separator, checkpoint) => {
                match self.result {
                    Attempt::Matched if parser.offset() > before => {
                        self.inline(Phase::Loop(committed));
                        return;
                    }
                    Attempt::Matched | Attempt::NoMatch => {}
                    Attempt::Committed if separator && !parser.is_halted() => {
                        parser.rewind(checkpoint)
                    }
                    Attempt::Committed => {
                        committed = true;
                        if parser.offset() > before {
                            self.inline(Phase::Loop(committed));
                            return;
                        }
                    }
                }
                self.inline(Phase::Trailing(committed));
                self.base(rules::SPACE_TAB0);
            }
            Phase::SeparatorProbe => {
                self.inline(Phase::SeparatorSpace(parser.checkpoint()));
                self.base(rules::SPACE_TAB0);
            }
            Phase::SeparatorSpace(checkpoint) => {
                if self.result == Attempt::Matched {
                    self.inline(Phase::SeparatorEnd(checkpoint));
                    self.table_separator();
                } else {
                    parser.rewind(checkpoint);
                    self.result = Attempt::NoMatch;
                }
            }
            Phase::SeparatorEnd(checkpoint) => {
                let accepted = self.result.accepted();
                parser.rewind(checkpoint);
                self.result = if accepted {
                    Attempt::Matched
                } else {
                    Attempt::NoMatch
                };
            }
            Phase::Trailing(committed) => {
                self.inline(Phase::Close(committed));
                self.table_separator();
            }
            Phase::Close(committed) => {
                if self.result == Attempt::Matched {
                    self.inline(Phase::Done(committed));
                } else {
                    self.inline(Phase::Done(true));
                    self.recover_table_separator(rules::INLINE_TABLE_ROW);
                }
            }
            Phase::Done(committed) => {
                self.result = if committed || parser.is_halted() {
                    Attempt::Committed
                } else {
                    Attempt::Matched
                }
            }
        }
    }
}
