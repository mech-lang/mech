//! Retained closed control, pattern, and subscript grammar phases.
use super::combinator::Attempt;
use super::{base, control_operators, literals, pattern_primitives, subscript_primitives};
use crate::document::parser::literal_scan::{LiteralProgress, LiteralScan};
use crate::document::parser::{Parser, checkpoint::ParserCheckpoint, marker::Marker, rule::rules};
use crate::document::{RuleId, SyntaxKind, TextSize};
use alloc::vec::Vec;

pub(crate) fn supports(rule: RuleId) -> bool {
    control_operators::supports(rule)
        || pattern_primitives::supports(rule)
        || subscript_primitives::supports(rule)
}
pub(crate) enum Progress {
    Complete(Attempt),
    NeedInput,
    NeedsProcessing,
    Limited,
}
#[derive(Clone, Copy)]
enum Op {
    Base(RuleId),
    Number(RuleId),
    Tag(&'static str),
    Any(&'static [RuleId]),
}
const ASSIGNMENTS: &[RuleId] = &[
    rules::ADD_ASSIGN_OPERATOR,
    rules::SUB_ASSIGN_OPERATOR,
    rules::MUL_ASSIGN_OPERATOR,
    rules::DIV_ASSIGN_OPERATOR,
    rules::EXP_ASSIGN_OPERATOR,
];
#[derive(Clone, Copy)]
struct Node {
    marker: Option<(Marker, SyntaxKind)>,
    swizzle: bool,
}
impl Node {
    fn complete(self, parser: &mut Parser<'_>) {
        if let Some((marker, kind)) = self.marker {
            marker.complete(parser, kind);
        }
    }
}
enum Frame {
    Enter(RuleId),
    Exit(ParserCheckpoint),
    Aggregate(Node, usize),
    Sequence(Node, &'static [Op], usize),
    Operation(Op),
    Any(&'static [RuleId], usize),
    Repeat(Node),
    RepeatComma(Node, ParserCheckpoint),
    RepeatIdentifier(Node, ParserCheckpoint),
    Base(base::continuation::Continuation),
    Number(literals::Continuation),
    TagExit(ParserCheckpoint),
    Literal(LiteralScan<'static>),
    LiteralResult(Option<TextSize>),
}
pub(crate) struct Continuation {
    frames: Vec<Frame>,
    result: Attempt,
    matched: bool,
    pub work: u64,
}
impl Continuation {
    pub fn new(rule: RuleId) -> Self {
        assert!(supports(rule), "canonical primitive owner");
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
    fn sequence(
        &mut self,
        parser: &mut Parser<'_>,
        kind: Option<SyntaxKind>,
        ops: &'static [Op],
        swizzle: bool,
    ) {
        let node = Node {
            marker: kind.map(|kind| (parser.start(), kind)),
            swizzle,
        };
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
            let frame = self.frames.pop().expect("retained primitive phase");
            if !matches!(frame, Frame::Base(_) | Frame::Number(_) | Frame::Literal(_)) {
                *allowance -= 1;
                self.work += 1;
            }
            match frame {
                Frame::Enter(rule) => {
                    let checkpoint = parser.checkpoint();
                    parser.state.rules.push_canonical(rule);
                    self.push(Frame::Exit(checkpoint));
                    match rule {
                        rules::SELECT_ALL => self.sequence(
                            parser,
                            Some(SyntaxKind::SelectAllSubscript),
                            &[Op::Base(rules::COLON)],
                            false,
                        ),
                        rules::DOT_SUBSCRIPT => self.sequence(
                            parser,
                            Some(SyntaxKind::DotSubscript),
                            &[Op::Base(rules::PERIOD), Op::Base(rules::IDENTIFIER)],
                            false,
                        ),
                        rules::DOT_SUBSCRIPT_INT => self.sequence(
                            parser,
                            Some(SyntaxKind::DotSubscriptInt),
                            &[Op::Base(rules::PERIOD), Op::Number(rules::INTEGER_LITERAL)],
                            false,
                        ),
                        rules::SWIZZLE_SUBSCRIPT => self.sequence(
                            parser,
                            Some(SyntaxKind::SwizzleSubscript),
                            &[
                                Op::Base(rules::PERIOD),
                                Op::Base(rules::IDENTIFIER),
                                Op::Base(rules::COMMA),
                                Op::Base(rules::IDENTIFIER),
                            ],
                            true,
                        ),
                        rules::WILDCARD => self.sequence(
                            parser,
                            Some(SyntaxKind::WildcardPattern),
                            &[Op::Base(rules::ASTERISK)],
                            false,
                        ),
                        rules::SPREAD_OPERATOR => self.sequence(
                            parser,
                            None,
                            &[Op::Any(&[
                                rules::SPREAD_OPERATOR_A,
                                rules::SPREAD_OPERATOR_U,
                            ])],
                            false,
                        ),
                        rules::STATEMENT_SEPARATOR => self.sequence(
                            parser,
                            None,
                            &[
                                Op::Base(rules::WHITESPACE0),
                                Op::Base(rules::SEMICOLON),
                                Op::Base(rules::WHITESPACE0),
                            ],
                            false,
                        ),
                        rules::SEND_OPERATOR => self.sequence(
                            parser,
                            None,
                            &[
                                Op::Base(rules::WHITESPACE0),
                                Op::Tag("<-"),
                                Op::Base(rules::WHITESPACE0),
                            ],
                            false,
                        ),
                        rules::GUARD_OPERATOR => self.sequence(
                            parser,
                            None,
                            &[
                                Op::Base(rules::WHITESPACE0),
                                Op::Any(&[
                                    rules::BAR,
                                    rules::BOX_VERT,
                                    rules::BOX_T_LEFT,
                                    rules::BOX_BL,
                                ]),
                                Op::Base(rules::WHITESPACE0),
                            ],
                            false,
                        ),
                        rules::OP_ASSIGN_OPERATOR => {
                            let node = Node {
                                marker: Some((parser.start(), SyntaxKind::OpAssignOperator)),
                                swizzle: false,
                            };
                            self.push(Frame::Aggregate(node, 0));
                            self.push(Frame::Enter(ASSIGNMENTS[0]));
                        }
                        rules::ADD_ASSIGN_OPERATOR => self.sequence(
                            parser,
                            Some(SyntaxKind::AddAssignOperation),
                            &[
                                Op::Base(rules::WHITESPACE0),
                                Op::Tag("+="),
                                Op::Base(rules::WHITESPACE0),
                            ],
                            false,
                        ),
                        rules::SUB_ASSIGN_OPERATOR => self.sequence(
                            parser,
                            Some(SyntaxKind::SubAssignOperation),
                            &[
                                Op::Base(rules::WHITESPACE0),
                                Op::Tag("-="),
                                Op::Base(rules::WHITESPACE0),
                            ],
                            false,
                        ),
                        rules::MUL_ASSIGN_OPERATOR => self.sequence(
                            parser,
                            Some(SyntaxKind::MulAssignOperation),
                            &[
                                Op::Base(rules::WHITESPACE0),
                                Op::Tag("*="),
                                Op::Base(rules::WHITESPACE0),
                            ],
                            false,
                        ),
                        rules::DIV_ASSIGN_OPERATOR => self.sequence(
                            parser,
                            Some(SyntaxKind::DivAssignOperation),
                            &[
                                Op::Base(rules::WHITESPACE0),
                                Op::Tag("/="),
                                Op::Base(rules::WHITESPACE0),
                            ],
                            false,
                        ),
                        rules::EXP_ASSIGN_OPERATOR => self.sequence(
                            parser,
                            Some(SyntaxKind::ExpAssignOperation),
                            &[
                                Op::Base(rules::WHITESPACE0),
                                Op::Tag("^="),
                                Op::Base(rules::WHITESPACE0),
                            ],
                            false,
                        ),
                        _ => unreachable!("supported canonical primitive"),
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
                Frame::Aggregate(node, index) => {
                    if self.result.accepted() {
                        node.complete(parser);
                    } else if let Some(rule) = ASSIGNMENTS.get(index + 1) {
                        self.push(Frame::Aggregate(node, index + 1));
                        self.push(Frame::Enter(*rule));
                    }
                }
                Frame::Sequence(node, ops, index) => {
                    if !self.matched {
                        self.result = Attempt::NoMatch;
                    } else if let Some(op) = ops.get(index + 1) {
                        self.push(Frame::Sequence(node, ops, index + 1));
                        self.push(Frame::Operation(*op));
                    } else if node.swizzle {
                        self.push(Frame::Repeat(node));
                    } else {
                        node.complete(parser);
                        self.result = Attempt::Matched;
                    }
                }
                Frame::Operation(op) => match op {
                    Op::Base(rule) => self.base(rule),
                    Op::Number(rule) => self.push(Frame::Number(literals::Continuation::new(rule))),
                    Op::Any(rules) => {
                        self.push(Frame::Any(rules, 0));
                        self.base(rules[0]);
                    }
                    Op::Tag(literal) => {
                        let checkpoint = parser.checkpoint();
                        parser.state.rules.push_canonical(rules::TAG);
                        self.push(Frame::TagExit(checkpoint));
                        if parser.offset() > parser.cursor().end() {
                            self.matched = false;
                        } else {
                            self.push(Frame::Literal(
                                LiteralScan::new(
                                    literal,
                                    parser.offset(),
                                    final_input.then_some(parser.cursor().context_end()),
                                )
                                .expect("nonempty control operator"),
                            ));
                        }
                    }
                },
                Frame::Any(rules, index) => {
                    if !self.matched
                        && let Some(rule) = rules.get(index + 1)
                    {
                        self.push(Frame::Any(rules, index + 1));
                        self.base(*rule);
                    }
                }
                Frame::Repeat(node) => {
                    self.push(Frame::RepeatComma(node, parser.checkpoint()));
                    self.base(rules::COMMA);
                }
                Frame::RepeatComma(node, checkpoint) => {
                    if self.matched {
                        self.push(Frame::RepeatIdentifier(node, checkpoint));
                        self.base(rules::IDENTIFIER);
                    } else {
                        parser.rewind(checkpoint);
                        node.complete(parser);
                        self.result = Attempt::Matched;
                    }
                }
                Frame::RepeatIdentifier(node, checkpoint) => {
                    if !self.matched {
                        parser.rewind(checkpoint);
                        node.complete(parser);
                        self.result = Attempt::Matched;
                    } else if parser.is_halted() {
                        node.complete(parser);
                        self.result = Attempt::Matched;
                    } else {
                        self.push(Frame::Repeat(node));
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
                Frame::Number(mut continuation) => {
                    let before = *allowance;
                    let progress = continuation.advance(parser, final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        literals::Progress::Complete(result) => self.matched = result.accepted(),
                        literals::Progress::NeedsProcessing => {
                            self.push(Frame::Number(continuation));
                            return Progress::NeedsProcessing;
                        }
                        literals::Progress::NeedInput => {
                            self.push(Frame::Number(continuation));
                            return Progress::NeedInput;
                        }
                        literals::Progress::Limited => {
                            self.push(Frame::Number(continuation));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::TagExit(checkpoint) => {
                    parser.state.rules.truncate(checkpoint.rule_depth);
                    if !self.matched {
                        parser.rewind(checkpoint);
                    }
                }
                Frame::Literal(mut scan) => {
                    let before = *allowance;
                    let progress = scan.advance(
                        parser.source(),
                        parser.cursor().end(),
                        parser.cursor().context_end(),
                        final_input,
                        allowance,
                    );
                    self.work += before - *allowance;
                    match progress {
                        LiteralProgress::Complete(end) => self.push(Frame::LiteralResult(end)),
                        LiteralProgress::NeedsProcessing => {
                            self.push(Frame::Literal(scan));
                            return Progress::NeedsProcessing;
                        }
                        LiteralProgress::NeedInput => {
                            self.push(Frame::Literal(scan));
                            return Progress::NeedInput;
                        }
                        LiteralProgress::InvalidSource => {
                            panic!("primitive continuation source bounds changed")
                        }
                    }
                }
                Frame::LiteralResult(end) => {
                    self.matched = end
                        .and_then(|end| {
                            parser.bump_bytes_token((end - parser.offset()).0, SyntaxKind::Text)
                        })
                        .is_some()
                }
            }
        }
        Progress::Complete(self.result)
    }
}
pub(super) fn parse_rule(parser: &mut Parser<'_>, rule: RuleId) -> Attempt {
    let mut continuation = Continuation::new(rule);
    loop {
        let mut allowance = u64::MAX;
        match continuation.advance(parser, true, &mut allowance) {
            Progress::Complete(result) => return result,
            Progress::NeedsProcessing => {}
            _ => unreachable!("final primitive input"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::continuation_test_support::{assert_partitions, run};
    use super::*;
    use alloc::string::String;
    #[test]
    fn primitive_choices_swizzle_rollback_and_limits_survive_input_cuts() {
        assert_partitions::<Continuation>(&[
            (rules::SELECT_ALL, ":"),
            (rules::SELECT_ALL, ":\u{301}"),
            (rules::DOT_SUBSCRIPT, "."),
            (rules::DOT_SUBSCRIPT, ".name"),
            (rules::DOT_SUBSCRIPT, ".e\u{301}"),
            (rules::DOT_SUBSCRIPT, ".👩\u{200d}💻"),
            (rules::DOT_SUBSCRIPT_INT, ".12"),
            (rules::DOT_SUBSCRIPT_INT, ".12u8"),
            (rules::DOT_SUBSCRIPT_INT, ".1_"),
            (rules::SWIZZLE_SUBSCRIPT, ".x"),
            (rules::SWIZZLE_SUBSCRIPT, ".x,"),
            (rules::SWIZZLE_SUBSCRIPT, ".x,y"),
            (rules::SWIZZLE_SUBSCRIPT, ".x,y,"),
            (rules::SWIZZLE_SUBSCRIPT, ".x,y,z"),
            (rules::SWIZZLE_SUBSCRIPT, ".x,💡,e\u{301}"),
            (rules::WILDCARD, "*"),
            (rules::WILDCARD, "**"),
            (rules::SPREAD_OPERATOR, ".."),
            (rules::SPREAD_OPERATOR, "..."),
            (rules::SPREAD_OPERATOR, " … "),
            (rules::STATEMENT_SEPARATOR, "\r\n;\t"),
            (rules::STATEMENT_SEPARATOR, " \t"),
            (rules::OP_ASSIGN_OPERATOR, "+="),
            (rules::OP_ASSIGN_OPERATOR, "^="),
            (rules::OP_ASSIGN_OPERATOR, "+"),
            (rules::ADD_ASSIGN_OPERATOR, " +=\r\n"),
            (rules::SUB_ASSIGN_OPERATOR, "-="),
            (rules::MUL_ASSIGN_OPERATOR, "*="),
            (rules::DIV_ASSIGN_OPERATOR, "/="),
            (rules::EXP_ASSIGN_OPERATOR, "^="),
            (rules::SEND_OPERATOR, "<-"),
            (rules::SEND_OPERATOR, "<"),
            (rules::SEND_OPERATOR, "\r\n<-\t"),
            (rules::GUARD_OPERATOR, "|"),
            (rules::GUARD_OPERATOR, "│"),
            (rules::GUARD_OPERATOR, "├"),
            (rules::GUARD_OPERATOR, "└"),
            (rules::GUARD_OPERATOR, " \u{301}|"),
        ]);
    }
    #[test]
    fn primitive_whitespace_and_swizzle_tails_retain_linear_work() {
        for (rule, prefix, unit, tail) in [
            (rules::SWIZZLE_SUBSCRIPT, ".x,y", ",z", ""),
            (rules::SWIZZLE_SUBSCRIPT, ".x,y", ",z", ","),
            (rules::DOT_SUBSCRIPT, ".", "a", ""),
            (rules::DOT_SUBSCRIPT_INT, ".", "1", "u8"),
            (rules::STATEMENT_SEPARATOR, "", " ", ";\r\n"),
            (rules::OP_ASSIGN_OPERATOR, "", " ", "^="),
            (rules::SEND_OPERATOR, "", "\r\n", "<-"),
            (rules::GUARD_OPERATOR, "│", " ", ""),
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
                assert_eq!(observed, baseline);
                assert_eq!(observed.result, Attempt::Matched);
                assert_eq!(observed.stats.source_bytes as usize, text.len());
                assert_eq!(observed.stats.diagnostics_emitted, 0);
                assert_eq!(
                    observed.end.to_usize(),
                    if tail == "," {
                        text.len() - 1
                    } else {
                        text.len()
                    }
                );
                if let Some((prior_streamed, prior_one_shot)) = previous {
                    assert!(work <= prior_streamed * 3, "append restarted {rule:?}");
                    assert!(one_shot_work <= prior_one_shot * 3);
                }
                previous = Some((work, one_shot_work));
            }
        }
    }
}
