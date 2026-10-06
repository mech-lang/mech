//! Retained structure delimiters, table borders, and closed empty collections.
use super::*;
use crate::document::parser::{checkpoint::ParserCheckpoint, marker::Marker};
use alloc::vec::Vec;

pub(crate) enum Progress {
    Complete(Attempt),
    NeedInput,
    NeedsProcessing,
    Limited,
}
#[derive(Clone, Copy)]
enum Op {
    Rule(RuleId),
    Any(&'static [RuleId]),
    Optional(RuleId),
}
#[derive(Clone, Copy)]
struct Node(Option<(Marker, SyntaxKind)>);
impl Node {
    fn complete(self, parser: &mut Parser<'_>) {
        if let Some((marker, kind)) = self.0 {
            marker.complete(parser, kind);
        }
    }
}
enum Frame {
    Enter(RuleId),
    Exit(ParserCheckpoint),
    Accept,
    SetResult,
    Optional,
    Operation(Op),
    Any(&'static [RuleId], usize),
    Sequence(Node, &'static [Op], usize),
    TopOpen,
    TopBody,
    RowLeading(Node),
    RowFirst(Node),
    RowAgain(Node),
    RowEnd(Node),
    RowItem,
    RowBox,
    RowSpaceProbe(ParserCheckpoint),
    RowTableEnd,
    RowSpaceRun(bool),
    Base(base::continuation::Continuation),
    Empty(literals::Continuation),
}
pub(crate) struct Continuation {
    frames: Vec<Frame>,
    result: Attempt,
    matched: bool,
    pub work: u64,
}
impl Continuation {
    pub fn new(rule: RuleId) -> Self {
        assert!(supports(rule), "canonical structure shell owner");
        Self {
            frames: alloc::vec![Frame::Enter(rule)],
            result: Attempt::NoMatch,
            matched: false,
            work: 0,
        }
    }
    fn push(&mut self, frame: Frame) {
        self.frames.push(frame);
    }
    fn base(&mut self, rule: RuleId) {
        self.push(Frame::Base(base::continuation::Continuation::new(rule)));
    }
    fn child(&mut self, rule: RuleId) {
        if rule == rules::EMPTY {
            self.push(Frame::Empty(literals::Continuation::new(rule)));
        } else if supports(rule) {
            self.push(Frame::Accept);
            self.push(Frame::Enter(rule));
        } else {
            self.base(rule);
        }
    }
    fn sequence(&mut self, parser: &mut Parser<'_>, kind: Option<SyntaxKind>, ops: &'static [Op]) {
        let node = Node(kind.map(|kind| (parser.start(), kind)));
        self.push(Frame::Sequence(node, ops, 0));
        self.push(Frame::Operation(ops[0]));
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
            let frame = self.frames.pop().expect("retained structure shell phase");
            if !matches!(frame, Frame::Base(_) | Frame::Empty(_)) {
                *allowance -= 1;
                self.work += 1;
            }
            match frame {
                Frame::Enter(rule) => {
                    let checkpoint = parser.checkpoint();
                    parser.state.rules.push_canonical(rule);
                    self.push(Frame::Exit(checkpoint));
                    match rule {
                        rules::MATRIX_START => self.sequence(
                            parser,
                            None,
                            &[Op::Any(&[
                                rules::BOX_TL_ROUND,
                                rules::BOX_TL,
                                rules::BOX_TL_BOLD,
                                rules::LEFT_BRACKET,
                            ])],
                        ),
                        rules::MATRIX_END => self.sequence(
                            parser,
                            None,
                            &[Op::Any(&[
                                rules::BOX_BR_ROUND,
                                rules::BOX_BR,
                                rules::BOX_BR_BOLD,
                                rules::RIGHT_BRACKET,
                            ])],
                        ),
                        rules::TABLE_START => self.sequence(
                            parser,
                            None,
                            &[Op::Any(&[
                                rules::BOX_TL_ROUND,
                                rules::BOX_TL,
                                rules::BOX_TL_BOLD,
                                rules::LEFT_BRACE,
                                rules::TABLE_SEPARATOR,
                            ])],
                        ),
                        rules::TABLE_END => self.sequence(
                            parser,
                            None,
                            &[Op::Any(&[
                                rules::BOX_BR_ROUND,
                                rules::BOX_BR,
                                rules::BOX_BR_BOLD,
                                rules::RIGHT_BRACE,
                                rules::TABLE_SEPARATOR,
                            ])],
                        ),
                        rules::TABLE_SEPARATOR => self.sequence(
                            parser,
                            None,
                            &[
                                Op::Rule(rules::SPACE_TAB0),
                                Op::Any(&[rules::BOX_VERT, rules::BOX_VERT_BOLD, rules::BAR]),
                                Op::Rule(rules::SPACE_TAB0),
                            ],
                        ),
                        rules::TABLE_HORZ => {
                            self.sequence(parser, None, &[Op::Any(&[rules::DASH, rules::BOX_HORZ])])
                        }
                        rules::TABLE_TOP => {
                            self.push(Frame::TopOpen);
                            self.child(rules::TABLE_START);
                        }
                        rules::ROW_SEPARATOR => {
                            let node = Node(Some((parser.start(), SyntaxKind::TableRowSeparator)));
                            self.push(Frame::RowLeading(node));
                            self.base(rules::SPACE_TAB0);
                        }
                        rules::EMPTY_MAP => self.sequence(
                            parser,
                            Some(SyntaxKind::EmptyMap),
                            &[
                                Op::Rule(rules::LEFT_BRACE),
                                Op::Rule(rules::WHITESPACE0),
                                Op::Rule(rules::COLON),
                                Op::Rule(rules::WHITESPACE0),
                                Op::Rule(rules::RIGHT_BRACE),
                            ],
                        ),
                        rules::EMPTY_SET => self.sequence(
                            parser,
                            Some(SyntaxKind::EmptySet),
                            &[
                                Op::Rule(rules::LEFT_BRACE),
                                Op::Rule(rules::WHITESPACE0),
                                Op::Optional(rules::EMPTY),
                                Op::Rule(rules::WHITESPACE0),
                                Op::Rule(rules::RIGHT_BRACE),
                            ],
                        ),
                        _ => unreachable!("supported structure shell"),
                    }
                }
                Frame::Exit(checkpoint) => {
                    parser.state.rules.truncate(checkpoint.rule_depth);
                    if self.result == Attempt::NoMatch {
                        parser.rewind(checkpoint);
                    }
                    if parser.is_halted() {
                        self.result = Attempt::Committed;
                    }
                }
                Frame::Accept => self.matched = self.result.accepted(),
                Frame::SetResult => {
                    self.result = if self.matched {
                        Attempt::Matched
                    } else {
                        Attempt::NoMatch
                    }
                }
                Frame::Optional => self.matched = true,
                Frame::Operation(op) => match op {
                    Op::Rule(rule) => self.child(rule),
                    Op::Optional(rule) => {
                        self.push(Frame::Optional);
                        self.child(rule);
                    }
                    Op::Any(rules) => {
                        self.push(Frame::Any(rules, 0));
                        self.child(rules[0]);
                    }
                },
                Frame::Any(rules, index) => {
                    if !self.matched
                        && let Some(rule) = rules.get(index + 1)
                    {
                        self.push(Frame::Any(rules, index + 1));
                        self.child(*rule);
                    }
                }
                Frame::Sequence(node, ops, index) => {
                    if !self.matched {
                        self.result = Attempt::NoMatch;
                    } else if let Some(op) = ops.get(index + 1) {
                        self.push(Frame::Sequence(node, ops, index + 1));
                        self.push(Frame::Operation(*op));
                    } else {
                        node.complete(parser);
                        self.result = Attempt::Matched;
                    }
                }
                Frame::TopOpen => {
                    if self.matched {
                        self.push(Frame::TopBody);
                        self.base(rules::BOX_DRAWING_CHAR);
                    } else {
                        self.result = Attempt::NoMatch;
                    }
                }
                Frame::TopBody => {
                    if self.matched && !parser.is_halted() {
                        self.push(Frame::TopBody);
                        self.base(rules::BOX_DRAWING_CHAR);
                    } else {
                        self.push(Frame::SetResult);
                        self.base(rules::NEW_LINE);
                    }
                }
                Frame::RowLeading(node) => {
                    if self.matched {
                        self.push(Frame::RowFirst(node));
                        self.push(Frame::RowItem);
                    } else {
                        self.result = Attempt::NoMatch;
                    }
                }
                Frame::RowFirst(node) => {
                    if self.matched {
                        self.push(Frame::RowAgain(node));
                        self.push(Frame::RowItem);
                    } else {
                        self.result = Attempt::NoMatch;
                    }
                }
                Frame::RowAgain(node) => {
                    if self.matched && !parser.is_halted() {
                        self.push(Frame::RowAgain(node));
                        self.push(Frame::RowItem);
                    } else {
                        self.push(Frame::RowEnd(node));
                        self.base(rules::SPACE_TAB0);
                    }
                }
                Frame::RowEnd(node) => {
                    if self.matched {
                        node.complete(parser);
                        self.result = Attempt::Matched;
                    } else {
                        self.result = Attempt::NoMatch;
                    }
                }
                Frame::RowItem => {
                    self.push(Frame::RowBox);
                    self.base(rules::BOX_DRAWING_CHAR);
                }
                Frame::RowBox => {
                    if !self.matched {
                        self.push(Frame::RowSpaceProbe(parser.checkpoint()));
                        self.base(rules::SPACE_TAB);
                    }
                }
                Frame::RowSpaceProbe(checkpoint) => {
                    parser.rewind(checkpoint);
                    if self.matched {
                        self.push(Frame::RowTableEnd);
                    }
                    self.child(rules::TABLE_END);
                }
                Frame::RowTableEnd => {
                    if !self.matched {
                        self.push(Frame::RowSpaceRun(false));
                        self.base(rules::SPACE_TAB);
                    }
                }
                Frame::RowSpaceRun(any) => {
                    if self.matched && !parser.is_halted() {
                        self.push(Frame::RowSpaceRun(true));
                        self.base(rules::SPACE_TAB);
                    } else {
                        self.matched |= any;
                    }
                }
                Frame::Base(mut continuation) => {
                    let before = *allowance;
                    let progress = continuation.advance(parser, final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        base::continuation::Progress::Complete(matched) => self.matched = matched,
                        base::continuation::Progress::NeedsProcessing => {
                            self.push(Frame::Base(continuation));
                            return Progress::NeedsProcessing;
                        }
                        base::continuation::Progress::NeedInput => {
                            self.push(Frame::Base(continuation));
                            return Progress::NeedInput;
                        }
                        base::continuation::Progress::Limited => {
                            self.push(Frame::Base(continuation));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::Empty(mut continuation) => {
                    let before = *allowance;
                    let progress = continuation.advance(parser, final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        literals::Progress::Complete(result) => self.matched = result.accepted(),
                        literals::Progress::NeedsProcessing => {
                            self.push(Frame::Empty(continuation));
                            return Progress::NeedsProcessing;
                        }
                        literals::Progress::NeedInput => {
                            self.push(Frame::Empty(continuation));
                            return Progress::NeedInput;
                        }
                        literals::Progress::Limited => {
                            self.push(Frame::Empty(continuation));
                            return Progress::Limited;
                        }
                    }
                }
            }
        }
        Progress::Complete(self.result)
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::continuation_test_support::{assert_partitions, run};
    use super::*;
    use alloc::string::String;
    #[test]
    fn structure_delimiters_border_choices_and_limits_survive_input_cuts() {
        assert_partitions::<Continuation>(&[
            (rules::MATRIX_START, "["),
            (rules::MATRIX_START, "╭"),
            (rules::MATRIX_START, "┌"),
            (rules::MATRIX_START, "┏"),
            (rules::MATRIX_END, "]"),
            (rules::MATRIX_END, "╯"),
            (rules::MATRIX_END, "┘"),
            (rules::MATRIX_END, "┛"),
            (rules::TABLE_START, "{"),
            (rules::TABLE_START, " | "),
            (rules::TABLE_END, "}"),
            (rules::TABLE_END, " ┃\t"),
            (rules::TABLE_SEPARATOR, " |\t"),
            (rules::TABLE_SEPARATOR, "│"),
            (rules::TABLE_SEPARATOR, "┃"),
            (rules::TABLE_SEPARATOR, " \t"),
            (rules::TABLE_HORZ, "-"),
            (rules::TABLE_HORZ, "─"),
            (rules::TABLE_TOP, "┌──┐\r\n"),
            (rules::TABLE_TOP, "|---\n"),
            (rules::TABLE_TOP, "┌─"),
            (rules::TABLE_TOP, "┌─\u{301}\n"),
            (rules::ROW_SEPARATOR, "├──┤"),
            (rules::ROW_SEPARATOR, " --- | "),
            (rules::ROW_SEPARATOR, "-   x"),
            (rules::ROW_SEPARATOR, "-   |"),
            (rules::ROW_SEPARATOR, "-\t  }"),
            (rules::ROW_SEPARATOR, " \t"),
            (rules::EMPTY_MAP, "{:}"),
            (rules::EMPTY_MAP, "{\r\n:\t}"),
            (rules::EMPTY_MAP, "{:"),
            (rules::EMPTY_MAP, "{"),
            (rules::EMPTY_SET, "{}"),
            (rules::EMPTY_SET, "{_}"),
            (rules::EMPTY_SET, "{___}"),
            (rules::EMPTY_SET, "{\r\n_\t}"),
            (rules::EMPTY_SET, "{_"),
            (rules::EMPTY_SET, "{_\u{301}}"),
        ]);
    }
    #[test]
    fn structure_borders_and_empty_collections_retain_linear_work() {
        for (rule, prefix, unit, tail) in [
            (rules::TABLE_TOP, "┌", "─", "\r\n"),
            (rules::TABLE_TOP, "┌", "─", ""),
            (rules::ROW_SEPARATOR, "-", " ", "x"),
            (rules::ROW_SEPARATOR, "-", " ", "|"),
            (rules::ROW_SEPARATOR, "", "─", ""),
            (rules::TABLE_SEPARATOR, "", " ", "|"),
            (rules::EMPTY_MAP, "{", " ", ":}"),
            (rules::EMPTY_SET, "{", "_", "}"),
        ] {
            let mut previous = None;
            for n in [64, 128, 256, 512] {
                let text = String::from(prefix) + &unit.repeat(n) + tail;
                let boundaries: Vec<_> = text
                    .char_indices()
                    .map(|(at, _)| at)
                    .chain(core::iter::once(text.len()))
                    .collect();
                let chunks: Vec<_> = boundaries
                    .windows(2)
                    .map(|pair| &text[pair[0]..pair[1]])
                    .collect();
                let (observed, work) = run::<Continuation>(rule, &text, &chunks, u64::MAX, true);
                let (baseline, one_shot_work) =
                    run::<Continuation>(rule, &text, &[&text], u64::MAX, false);
                assert_eq!(observed, baseline, "{rule:?}, tail {tail:?}");
                assert_eq!(observed.stats.source_bytes as usize, text.len());
                assert_eq!(observed.stats.diagnostics_emitted, 0);
                if let Some((prior_streamed, prior_one_shot)) = previous {
                    assert!(work <= prior_streamed * 3, "append restarted {rule:?}");
                    assert!(one_shot_work <= prior_one_shot * 3);
                }
                previous = Some((work, one_shot_work));
            }
        }
    }
}
