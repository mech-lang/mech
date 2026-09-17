//! Shared structure selection retains wrappers and physical opener recognition.
//! Body conversion follows these owners without replaying the opener prefix.
use super::super::super::base;
use super::super::precedence::Progress;
use super::*;
use crate::document::parser::marker::Marker;
use alloc::{boxed::Box, vec::Vec};
pub(in super::super) fn supports(rule: RuleId) -> bool {
    matches!(
        rule,
        rules::TABLE
            | rules::INLINE_TABLE
            | rules::REGULAR_TABLE
            | rules::FANCY_TABLE
            | rules::MATRIX
            | rules::RECORD
    )
}
enum Frame {
    Core(Box<super::super::Continuation>),
    Call(RuleId),
    Exit(ParserCheckpoint),
    TableChoice(Marker, usize),
    TableOpen(Marker, RuleId),
    TablePrefix(Marker, RuleId),
    TableBody(Marker, RuleId),
    RecordStart,
    RecordOpen(Marker, TableDelimiter),
    RecordBody(Marker, TableDelimiter),
    NonDelimited(usize),
    NonDelimitedNext(usize),
    Wrap(RuleId),
    WrapFinish(Marker, ParserCheckpoint),
    Base(base::continuation::Continuation),
    Shell(structure_shell::Continuation),
}
pub(in super::super) struct StructureContinuation {
    frames: Vec<Frame>,
    result: Attempt,
    pub work: u64,
}
impl StructureContinuation {
    pub fn new(rule: RuleId) -> Self {
        Self {
            frames: alloc::vec![Frame::Call(rule)],
            result: Attempt::NoMatch,
            work: 0,
        }
    }
    pub fn non_delimited() -> Self {
        Self {
            frames: alloc::vec![Frame::NonDelimited(0)],
            result: Attempt::NoMatch,
            work: 0,
        }
    }
    pub fn drive(&mut self, parser: &mut Parser<'_>) -> Attempt {
        loop {
            let mut allowance = u64::MAX;
            match self.advance(parser, true, &mut allowance) {
                Progress::Complete(result) => return result,
                Progress::NeedsProcessing => {}
                _ => unreachable!("sealed structure prefix"),
            }
        }
    }
    fn push(&mut self, frame: Frame) {
        self.frames.push(frame);
    }
    fn base(&mut self, rule: RuleId) {
        self.push(Frame::Base(base::continuation::Continuation::new(rule)));
    }
    fn shell(&mut self, rule: RuleId) {
        self.push(Frame::Shell(structure_shell::Continuation::new(rule)));
    }
    fn transaction(&mut self, parser: &mut Parser<'_>, rule: RuleId) {
        let checkpoint = parser.checkpoint();
        parser.state.rules.push_canonical(rule);
        self.push(Frame::Exit(checkpoint));
    }
    fn table_choice(&mut self, node: Marker, index: usize) {
        self.push(Frame::TableChoice(node, index));
        self.push(Frame::Call(
            [
                rules::INLINE_TABLE,
                rules::REGULAR_TABLE,
                rules::FANCY_TABLE,
            ][index],
        ));
    }
    pub fn advance(
        &mut self,
        parser: &mut Parser<'_>,
        final_input: bool,
        allowance: &mut u64,
    ) -> Progress {
        while !self.frames.is_empty() {
            if !final_input && parser.is_halted() {
                return Progress::Limited;
            }
            if *allowance == 0 {
                return Progress::NeedsProcessing;
            }
            let frame = self.frames.pop().expect("retained structure prefix");
            if !matches!(frame, Frame::Base(_) | Frame::Shell(_) | Frame::Core(_)) {
                *allowance -= 1;
                self.work += 1;
            }
            match frame {
                Frame::Call(rule) => match rule {
                    rules::TABLE => {
                        self.transaction(parser, rule);
                        let node = parser.start();
                        self.table_choice(node, 0);
                    }
                    rules::INLINE_TABLE | rules::REGULAR_TABLE | rules::FANCY_TABLE => {
                        self.transaction(parser, rule);
                        let node = parser.start();
                        self.push(Frame::TableOpen(node, rule));
                        self.shell(if rule == rules::FANCY_TABLE {
                            rules::TABLE_TOP
                        } else {
                            rules::TABLE_SEPARATOR
                        });
                    }
                    rules::MATRIX => self.push(Frame::Core(Box::new(
                        super::super::Continuation::new(rules::MATRIX),
                    ))),
                    rules::RECORD => self.push(Frame::RecordStart),
                    _ => unreachable!("structure prefix rule"),
                },
                Frame::Exit(checkpoint) => {
                    parser.state.rules.truncate(checkpoint.rule_depth);
                    if self.result == Attempt::NoMatch {
                        parser.rewind(checkpoint);
                    }
                    if parser.is_halted() {
                        self.result = Attempt::Committed;
                    }
                }
                Frame::TableChoice(node, index) => {
                    if self.result == Attempt::NoMatch && index < 2 {
                        self.table_choice(node, index + 1);
                    } else if let Some(result) =
                        child_result(parser, node, SyntaxKind::Table, self.result)
                    {
                        self.result = result;
                    } else {
                        node.complete(parser, SyntaxKind::Table);
                        self.result = Attempt::Matched;
                    }
                }
                Frame::TableOpen(node, rule) => {
                    if self.result != Attempt::Matched {
                        node.abandon(parser);
                        self.result = Attempt::NoMatch;
                    } else {
                        self.push(Frame::TablePrefix(node, rule));
                        if rule == rules::FANCY_TABLE {
                            self.shell(rules::TABLE_SEPARATOR);
                        } else {
                            self.base(if rule == rules::INLINE_TABLE {
                                rules::SPACE_TAB0
                            } else {
                                rules::WHITESPACE0
                            });
                        }
                    }
                }
                Frame::TablePrefix(node, rule) => {
                    if self.result != Attempt::Matched {
                        node.abandon(parser);
                        self.result = Attempt::NoMatch;
                    } else {
                        self.push(Frame::TableBody(node, rule));
                    }
                }
                Frame::TableBody(node, rule) => {
                    self.push(Frame::Core(Box::new(
                        super::super::Continuation::table_body(node, rule),
                    )));
                }
                Frame::RecordStart => {
                    if !final_input && parser.is_eof() {
                        self.push(Frame::RecordStart);
                        return Progress::NeedInput;
                    }
                    self.transaction(parser, rules::RECORD);
                    let node = parser.start();
                    let delimiter = if parser.cursor().starts_with("{") {
                        TableDelimiter::Brace
                    } else if parser.cursor().starts_with("|") {
                        TableDelimiter::Bar
                    } else {
                        TableDelimiter::Box
                    };
                    self.push(Frame::RecordOpen(node, delimiter));
                    self.shell(rules::TABLE_START);
                }
                Frame::RecordOpen(node, delimiter) => {
                    if self.result != Attempt::Matched {
                        node.abandon(parser);
                        self.result = Attempt::NoMatch;
                    } else {
                        self.push(Frame::RecordBody(node, delimiter));
                    }
                }
                Frame::RecordBody(node, delimiter) => {
                    self.push(Frame::Core(Box::new(
                        super::super::Continuation::record_body(node, delimiter),
                    )));
                }
                Frame::Core(mut child) => {
                    let before = *allowance;
                    let progress = child.advance(parser, final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        Progress::Complete(result) => self.result = result,
                        Progress::NeedInput => {
                            self.push(Frame::Core(child));
                            return Progress::NeedInput;
                        }
                        Progress::NeedsProcessing => {
                            self.push(Frame::Core(child));
                            return Progress::NeedsProcessing;
                        }
                        Progress::Limited => {
                            self.push(Frame::Core(child));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::NonDelimited(index) => {
                    self.push(Frame::NonDelimitedNext(index));
                    self.push(Frame::Wrap(
                        [rules::TABLE, rules::MATRIX, rules::RECORD][index],
                    ));
                }
                Frame::NonDelimitedNext(index) => {
                    if self.result == Attempt::NoMatch && index < 2 {
                        self.push(Frame::NonDelimited(index + 1));
                    }
                }
                Frame::Wrap(rule) => {
                    let checkpoint = parser.checkpoint();
                    let node = parser.start();
                    self.push(Frame::WrapFinish(node, checkpoint));
                    self.push(Frame::Call(rule));
                }
                Frame::WrapFinish(node, checkpoint) => {
                    if self.result == Attempt::NoMatch {
                        parser.rewind(checkpoint);
                    } else {
                        node.complete(parser, SyntaxKind::Structure);
                    }
                }
                Frame::Base(mut child) => {
                    let before = *allowance;
                    let progress = child.advance(parser, final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        base::continuation::Progress::Complete(matched) => {
                            self.result = if matched {
                                Attempt::Matched
                            } else {
                                Attempt::NoMatch
                            }
                        }
                        base::continuation::Progress::NeedInput => {
                            self.push(Frame::Base(child));
                            return Progress::NeedInput;
                        }
                        base::continuation::Progress::NeedsProcessing => {
                            self.push(Frame::Base(child));
                            return Progress::NeedsProcessing;
                        }
                        base::continuation::Progress::Limited => {
                            self.push(Frame::Base(child));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::Shell(mut child) => {
                    let before = *allowance;
                    let progress = child.advance(parser, final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        structure_shell::Progress::Complete(result) => self.result = result,
                        structure_shell::Progress::NeedInput => {
                            self.push(Frame::Shell(child));
                            return Progress::NeedInput;
                        }
                        structure_shell::Progress::NeedsProcessing => {
                            self.push(Frame::Shell(child));
                            return Progress::NeedsProcessing;
                        }
                        structure_shell::Progress::Limited => {
                            self.push(Frame::Shell(child));
                            return Progress::Limited;
                        }
                    }
                }
            }
        }
        Progress::Complete(self.result)
    }
}
