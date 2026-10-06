//! Map repetition retains each entry and separator recovery.
use super::*;
pub(super) enum Phase {
    Enter,
    Open(Marker),
    Finish(Marker),
    Space,
    Entries,
    Trailing(bool),
    Close(bool),
    Loop(bool, bool),
    Item(bool, bool, TextSize),
    Missing(Marker, TextSize),
    Comma(Marker, TextSize),
    MissingFinish(Marker, TextSize),
}
impl Continuation {
    fn map(&mut self, phase: Phase) {
        self.push(Frame::Map(Box::new(phase)));
    }
    fn map_result(&mut self, parser: &Parser<'_>, committed: bool) {
        self.result = if committed || parser.is_halted() {
            Attempt::Committed
        } else {
            Attempt::Matched
        };
    }
    #[inline(never)]
    pub(super) fn map_frame(&mut self, parser: &mut Parser<'_>, phase: Box<Phase>) {
        let phase = *phase;
        match phase {
            Phase::Enter => {
                self.transaction(parser, rules::MAP);
                let node = parser.start();
                self.map(Phase::Open(node));
                self.base(rules::LEFT_BRACE);
            }
            Phase::Open(node) => {
                self.map(Phase::Finish(node));
                if self.result != Attempt::Matched {
                    self.result = Attempt::NoMatch;
                } else if parser.push_nesting() {
                    self.push(Frame::PopNesting);
                    self.map(Phase::Space);
                    self.base(rules::WHITESPACE0);
                } else {
                    self.push(Frame::Nesting(Box::new(NestingContinuation::new())));
                }
            }
            Phase::Finish(node) => {
                self.result = finish(node, parser, SyntaxKind::Map, self.result);
            }
            Phase::Space => {
                if self.result == Attempt::Matched {
                    self.map(Phase::Entries);
                    self.map(Phase::Loop(false, false));
                } else {
                    self.result = Attempt::NoMatch;
                }
            }
            Phase::Entries => {
                if self.result != Attempt::NoMatch {
                    self.map(Phase::Trailing(self.result == Attempt::Committed));
                    self.base(rules::WHITESPACE0);
                }
            }
            Phase::Trailing(committed) => {
                self.map(Phase::Close(committed));
                self.base(rules::RIGHT_BRACE);
            }
            Phase::Close(committed) => {
                if self.result == Attempt::Matched {
                    self.result = if committed {
                        Attempt::Committed
                    } else {
                        Attempt::Matched
                    };
                } else {
                    self.push(Frame::Closer(Box::new(Closer::new(
                        rules::MAP,
                        rules::RIGHT_BRACE,
                        SyntaxKind::RightBrace,
                        "}",
                    ))));
                }
            }
            Phase::Loop(parsed_any, committed) => {
                if parser.is_halted() {
                    self.map_result(parser, committed);
                } else {
                    self.map(Phase::Item(parsed_any, committed, parser.offset()));
                    self.push(Frame::Call(rules::MAPPING));
                }
            }
            Phase::Item(mut parsed_any, mut committed, before) => {
                match self.result {
                    Attempt::Matched if parser.offset() > before => parsed_any = true,
                    Attempt::Matched => {
                        self.map_result(parser, committed);
                        return;
                    }
                    Attempt::NoMatch if !parsed_any => return,
                    Attempt::NoMatch if parser.cursor().starts_with(",") => {
                        let node = parser.start();
                        self.map(Phase::Missing(node, before));
                        self.push(Frame::Required(Box::new(Required::new(
                            rules::MAP,
                            "syntax/missing-map-entry",
                            "missing map entry after separator",
                            "mapping",
                            &[],
                            &[],
                            None,
                        ))));
                        return;
                    }
                    Attempt::NoMatch => {
                        self.map_result(parser, committed);
                        return;
                    }
                    Attempt::Committed => {
                        parsed_any = true;
                        committed = true;
                    }
                }
                if parser.offset() == before {
                    self.map_result(parser, committed);
                } else {
                    self.map(Phase::Loop(parsed_any, committed));
                }
            }
            Phase::Missing(node, before) => {
                self.map(Phase::Comma(node, before));
                self.base(rules::COMMA);
            }
            Phase::Comma(node, before) => {
                self.map(Phase::MissingFinish(node, before));
                self.base(rules::WHITESPACE0);
            }
            Phase::MissingFinish(node, before) => {
                node.complete(parser, SyntaxKind::MapEntry);
                if parser.offset() == before {
                    self.map_result(parser, true);
                } else {
                    self.map(Phase::Loop(true, true));
                }
            }
        }
    }
}
