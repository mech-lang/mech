//! Matrix columns, rows, and decoration retain their scanned prefixes.
use super::*;
pub(super) enum Phase {
    Enter(RuleId),
    Finish(Marker, SyntaxKind),
    ColumnSpace,
    ColumnValue,
    ColumnTailResult(bool),
    ColumnTail,
    ColumnTailSpace,
    ColumnTailComma,
    ColumnTailVert,
    ColumnTailBold,
    ColumnTailEnd,
    RowSpace,
    RowSeparator,
    RowLeading,
    RowFirst(Marker),
    RowTail(Marker, bool),
    RowProbe(Marker, bool, ParserCheckpoint),
    RowNext(Marker, bool, TextSize),
    RowEnd(Marker, bool),
    Suffix,
    SuffixSemicolon,
    SuffixNewline,
    SuffixFirst(ParserCheckpoint),
    SuffixLoop(ParserCheckpoint),
    SuffixLast(ParserCheckpoint),
    Decoration,
    DecorationProbe(ParserCheckpoint),
    DecorationChar(TextSize),
    DecorationSpace(TextSize),
}
impl Continuation {
    fn matrix_row(&mut self, phase: Phase) {
        self.push(Frame::MatrixRow(Box::new(phase)));
    }
    fn matrix_end(&mut self) {
        self.push(Frame::Shell(Box::new(
            super::super::super::super::structure_shell::Continuation::new(rules::MATRIX_END),
        )));
    }
    fn matrix_row_suffix(&mut self, node: Marker, committed: bool) {
        self.matrix_row(Phase::RowEnd(node, committed));
        self.matrix_row(Phase::Suffix);
    }
    #[inline(never)]
    pub(super) fn matrix_row_frame(&mut self, parser: &mut Parser<'_>, phase: Box<Phase>) {
        match *phase {
            Phase::Enter(rule) => {
                self.transaction(parser, rule);
                let node = parser.start();
                if rule == rules::MATRIX_COLUMN {
                    self.matrix_row(Phase::Finish(node, SyntaxKind::MatrixColumn));
                    self.matrix_row(Phase::ColumnSpace);
                } else {
                    self.matrix_row(Phase::RowFirst(node));
                    self.matrix_row(Phase::RowSpace);
                }
                self.base(rules::SPACE_TAB0);
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
            Phase::ColumnSpace => {
                if self.result == Attempt::Matched {
                    self.matrix_row(Phase::ColumnValue);
                    self.push(Frame::Call(rules::EXPRESSION));
                } else {
                    self.result = Attempt::NoMatch;
                }
            }
            Phase::ColumnValue => {
                if self.result != Attempt::NoMatch {
                    self.matrix_row(Phase::ColumnTailResult(self.result == Attempt::Committed));
                    self.matrix_row(Phase::ColumnTail);
                }
            }
            Phase::ColumnTailResult(committed) => {
                if self.result != Attempt::Matched && !parser.is_halted() {
                    self.result = Attempt::NoMatch;
                } else {
                    self.result = if committed || parser.is_halted() {
                        Attempt::Committed
                    } else {
                        Attempt::Matched
                    };
                }
            }
            Phase::ColumnTail => {
                self.matrix_row(Phase::ColumnTailSpace);
                self.base(rules::SPACE_TAB0);
            }
            Phase::ColumnTailSpace => {
                if self.result == Attempt::Matched {
                    self.matrix_row(Phase::ColumnTailComma);
                    self.base(rules::COMMA);
                } else {
                    self.result = Attempt::NoMatch;
                }
            }
            Phase::ColumnTailComma => {
                if self.result == Attempt::Matched {
                    self.matrix_row(Phase::ColumnTailEnd);
                    self.base(rules::SPACE_TAB0);
                } else {
                    self.matrix_row(Phase::ColumnTailVert);
                    self.base(rules::BOX_VERT);
                }
            }
            Phase::ColumnTailVert => {
                if self.result == Attempt::Matched {
                    self.matrix_row(Phase::ColumnTailEnd);
                    self.base(rules::SPACE_TAB0);
                } else {
                    self.matrix_row(Phase::ColumnTailBold);
                    self.base(rules::BOX_VERT_BOLD);
                }
            }
            Phase::ColumnTailBold => {
                self.matrix_row(Phase::ColumnTailEnd);
                self.base(rules::SPACE_TAB0);
            }
            Phase::ColumnTailEnd => {
                if self.result != Attempt::Matched {
                    self.result = Attempt::NoMatch;
                }
            }
            Phase::RowSpace => {
                if self.result == Attempt::Matched {
                    self.matrix_row(Phase::RowSeparator);
                    self.table_separator();
                } else {
                    self.result = Attempt::NoMatch;
                }
            }
            Phase::RowSeparator => {
                self.matrix_row(Phase::RowLeading);
                self.base(rules::SPACE_TAB0);
            }
            Phase::RowLeading => {
                if self.result == Attempt::Matched {
                    self.push(Frame::Call(rules::MATRIX_COLUMN));
                } else {
                    self.result = Attempt::NoMatch;
                }
            }
            Phase::RowFirst(node) => {
                if self.result == Attempt::NoMatch {
                    if !parser.state.resource_finalizing {
                        node.abandon(parser);
                    }
                } else {
                    self.matrix_row(Phase::RowTail(node, self.result == Attempt::Committed));
                }
            }
            Phase::RowTail(node, committed) => {
                if parser.is_halted() {
                    self.matrix_row_suffix(node, committed);
                } else if committed {
                    self.matrix_row(Phase::RowProbe(node, committed, parser.checkpoint()));
                    self.matrix_end();
                } else {
                    self.matrix_row(Phase::RowNext(node, committed, parser.offset()));
                    self.push(Frame::Call(rules::MATRIX_COLUMN));
                }
            }
            Phase::RowProbe(node, committed, checkpoint) => {
                let end = self.result.accepted();
                parser.rewind(checkpoint);
                if end || parser.is_halted() {
                    self.matrix_row_suffix(node, committed);
                } else {
                    self.matrix_row(Phase::RowNext(node, committed, parser.offset()));
                    self.push(Frame::Call(rules::MATRIX_COLUMN));
                }
            }
            Phase::RowNext(node, mut committed, before) => {
                match self.result {
                    Attempt::Matched if parser.offset() > before => {
                        self.matrix_row(Phase::RowTail(node, committed));
                        return;
                    }
                    Attempt::Matched | Attempt::NoMatch => {}
                    Attempt::Committed => {
                        committed = true;
                        if parser.offset() > before {
                            self.matrix_row(Phase::RowTail(node, committed));
                            return;
                        }
                    }
                }
                self.matrix_row_suffix(node, committed);
            }
            Phase::RowEnd(node, committed) => {
                node.complete(parser, SyntaxKind::MatrixRow);
                self.result = if committed || parser.is_halted() {
                    Attempt::Committed
                } else {
                    Attempt::Matched
                };
            }
            Phase::Suffix => {
                self.matrix_row(Phase::SuffixSemicolon);
                self.base(rules::SEMICOLON);
            }
            Phase::SuffixSemicolon => {
                self.matrix_row(Phase::SuffixNewline);
                self.base(rules::NEW_LINE);
            }
            Phase::SuffixNewline => {
                self.matrix_row(Phase::SuffixFirst(parser.checkpoint()));
                self.base(rules::BOX_DRAWING_CHAR);
            }
            Phase::SuffixFirst(checkpoint) => {
                if self.result == Attempt::Matched {
                    self.matrix_row(Phase::SuffixLoop(checkpoint));
                    self.base(rules::BOX_DRAWING_CHAR);
                } else {
                    self.result = Attempt::Matched;
                }
            }
            Phase::SuffixLoop(checkpoint) => {
                if self.result == Attempt::Matched {
                    self.matrix_row(Phase::SuffixLoop(checkpoint));
                    self.base(rules::BOX_DRAWING_CHAR);
                } else {
                    self.matrix_row(Phase::SuffixLast(checkpoint));
                    self.base(rules::NEW_LINE);
                }
            }
            Phase::SuffixLast(checkpoint) => {
                if self.result != Attempt::Matched {
                    parser.rewind(checkpoint);
                }
                self.result = Attempt::Matched;
            }
            Phase::Decoration => {
                self.matrix_row(Phase::DecorationProbe(parser.checkpoint()));
                self.matrix_end();
            }
            Phase::DecorationProbe(checkpoint) => {
                let end = self.result.accepted();
                parser.rewind(checkpoint);
                if end {
                    self.result = Attempt::Matched;
                } else {
                    self.matrix_row(Phase::DecorationChar(parser.offset()));
                    self.base(rules::BOX_DRAWING_CHAR);
                }
            }
            Phase::DecorationChar(before) => {
                if self.result == Attempt::Matched {
                    self.matrix_row(Phase::DecorationSpace(before));
                } else {
                    self.matrix_row(Phase::DecorationSpace(before));
                    self.base(rules::WHITESPACE);
                }
            }
            Phase::DecorationSpace(before) => {
                if self.result != Attempt::Matched {
                    self.result = Attempt::Matched;
                } else if parser.is_halted() {
                    self.result = Attempt::Committed;
                } else if parser.offset() > before {
                    self.matrix_row(Phase::Decoration);
                } else {
                    self.result = Attempt::Matched;
                }
            }
        }
    }
}
