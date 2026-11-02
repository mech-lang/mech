//! Record bodies retain bindings and the shared map discriminator probe.
use super::super::super::structures::TableDelimiter;
use super::*;
pub(super) enum Phase {
    Enter(Marker, TableDelimiter),
    Finish(Marker),
    Space(TableDelimiter),
    Loop(TableDelimiter, bool, bool),
    Binding(TableDelimiter, bool, bool, TextSize),
    RecoveredSeparator(TableDelimiter, TextSize),
    Trailing(TableDelimiter, bool),
    End(TableDelimiter, bool),
    MappingProbe(TableDelimiter),
}
impl Continuation {
    pub(in super::super::super) fn record_body(node: Marker, delimiter: TableDelimiter) -> Self {
        let mut owner = Self::new(rules::RECORD);
        owner.frames.clear();
        owner.record(Phase::Enter(node, delimiter));
        owner
    }
    fn record(&mut self, phase: Phase) {
        self.push(Frame::Record(Box::new(phase)));
    }
    fn record_recovery(&mut self, delimiter: TableDelimiter) {
        let (kind, text) = match delimiter {
            TableDelimiter::Brace => (SyntaxKind::RightBrace, "}"),
            TableDelimiter::Bar => (SyntaxKind::Bar, "|"),
            TableDelimiter::Box => (SyntaxKind::BoxDrawing, "╯"),
        };
        self.push(Frame::Closer(Box::new(Closer::set(
            rules::RECORD,
            rules::TABLE_END,
            kind,
            text,
            &['}', '|', '╯', '┘', '┛', ')', ']'],
        ))));
    }
    fn record_trailing(&mut self, delimiter: TableDelimiter, committed: bool) {
        self.record(Phase::Trailing(delimiter, committed));
        self.base(rules::WHITESPACE0);
    }
    #[inline(never)]
    pub(super) fn record_frame(&mut self, parser: &mut Parser<'_>, phase: Box<Phase>) {
        match *phase {
            Phase::Enter(node, delimiter) => {
                self.record(Phase::Finish(node));
                if parser.push_nesting() {
                    self.push(Frame::PopNesting);
                    self.record(Phase::Space(delimiter));
                    self.base(rules::WHITESPACE0);
                } else {
                    self.push(Frame::Nesting(Box::new(NestingContinuation::new())));
                }
            }
            Phase::Finish(node) => {
                if parser.is_halted() {
                    node.complete(parser, SyntaxKind::Record);
                    self.result = Attempt::Committed;
                } else {
                    self.result = finish(node, parser, SyntaxKind::Record, self.result);
                }
            }
            Phase::Space(delimiter) => {
                if self.result == Attempt::Matched {
                    self.record(Phase::Loop(delimiter, false, false));
                } else {
                    self.result = Attempt::NoMatch;
                }
            }
            Phase::Loop(delimiter, parsed_any, committed) => {
                if parser.is_halted() {
                    self.record_trailing(delimiter, committed);
                } else {
                    self.record(Phase::Binding(
                        delimiter,
                        parsed_any,
                        committed,
                        parser.offset(),
                    ));
                    self.push(Frame::Call(rules::BINDING));
                }
            }
            Phase::Binding(delimiter, parsed_any, committed, before) => match self.result {
                Attempt::Matched if parser.offset() > before => {
                    self.record(Phase::Loop(delimiter, true, committed))
                }
                Attempt::Matched => self.record_trailing(delimiter, committed),
                Attempt::NoMatch if !parsed_any => {}
                Attempt::NoMatch => self.record_trailing(delimiter, committed),
                Attempt::Committed => {
                    self.record(Phase::RecoveredSeparator(delimiter, before));
                    self.base(rules::LIST_SEPARATOR);
                }
            },
            Phase::RecoveredSeparator(delimiter, before) => {
                if parser.offset() == before {
                    self.record_trailing(delimiter, true);
                } else {
                    self.record(Phase::Loop(delimiter, true, true));
                }
            }
            Phase::Trailing(delimiter, committed) => {
                self.record(Phase::End(delimiter, committed));
                self.push(Frame::Shell(Box::new(
                    super::super::super::super::structure_shell::Continuation::new(
                        rules::TABLE_END,
                    ),
                )));
            }
            Phase::End(delimiter, committed) => {
                if self.result == Attempt::Matched {
                    self.result = if committed {
                        Attempt::Committed
                    } else {
                        Attempt::Matched
                    };
                } else if parser.is_eof() {
                    self.record_recovery(delimiter);
                } else {
                    self.record(Phase::MappingProbe(delimiter));
                    self.push(Frame::MappingProbe(Box::new(mapping_probe::Phase::Enter)));
                }
            }
            Phase::MappingProbe(delimiter) => {
                if self.result == Attempt::Matched {
                    self.result = Attempt::NoMatch;
                } else {
                    self.record_recovery(delimiter);
                }
            }
        }
    }
}
