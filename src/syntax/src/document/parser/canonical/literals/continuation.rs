//! Retained canonical numeric candidates, based payloads, and primitive literals.
use super::*;
use crate::document::RuleId;
use crate::document::parser::{
    checkpoint::ParserCheckpoint,
    literal_scan::{LiteralProgress, LiteralScan},
    marker::Marker,
};
use alloc::vec::Vec;

pub(crate) enum Progress {
    Complete(Attempt),
    NeedInput,
    NeedsProcessing,
    Limited,
}
#[derive(Clone, Copy)]
enum Op {
    Base(RuleId),
    Call(RuleId),
    Tag(&'static str),
    Any(&'static [Op]),
    OptionalBase(RuleId),
}
const FLOAT_OR_INTEGER: &[Op] = &[
    Op::Call(rules::FLOAT_LITERAL),
    Op::Call(rules::INTEGER_LITERAL),
];
const IMAGINARY: &[Op] = &[Op::Tag("i"), Op::Tag("j")];
const HEX_PAYLOAD: &[Op] = &[
    Op::Base(rules::DIGIT_TOKEN),
    Op::Base(rules::UNDERSCORE),
    Op::Base(rules::ALPHA_TOKEN),
];
#[derive(Clone, Copy)]
struct Node {
    marker: Marker,
    kind: SyntaxKind,
}
impl Node {
    fn complete(self, parser: &mut Parser<'_>) {
        self.marker.complete(parser, self.kind);
    }
}
#[derive(Clone, Copy)]
struct Based {
    node: Node,
    rule: RuleId,
    recover: bool,
}
enum Frame {
    Enter(RuleId, bool),
    Exit(ParserCheckpoint),
    Accept,
    SetResult,
    Operation(Op),
    Any(&'static [Op], usize),
    Optional,
    Sequence(Node, &'static [Op], usize, bool),
    NodeResult(Node),
    Choice(&'static [RuleId], usize, bool),
    EmptyFirst(Node),
    EmptyRepeat(Node),
    RealLeading(Node, bool, bool),
    ComplexFirst(Node),
    ComplexUnit(Node),
    ComplexSign(Node),
    ComplexSecond(Node),
    ComplexLast(Node),
    BasedPrefix(Based),
    BasedPayload(Based),
    HexElement(Based, TextSize, bool),
    Base(base::continuation::Continuation),
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
        assert!(supports(rule), "canonical literal owner");
        Self {
            frames: alloc::vec![Frame::Enter(rule, true)],
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
    fn call(&mut self, rule: RuleId, recover: bool) {
        self.push(Frame::Enter(rule, recover));
    }
    fn node(&mut self, parser: &mut Parser<'_>, kind: SyntaxKind) -> Node {
        Node {
            marker: parser.start(),
            kind,
        }
    }
    fn choice(&mut self, rules: &'static [RuleId], recover: bool) {
        self.push(Frame::Choice(rules, 0, recover));
        self.call(rules[0], recover);
    }
    fn aggregate(&mut self, parser: &mut Parser<'_>, kind: SyntaxKind, rules: &'static [RuleId]) {
        let node = self.node(parser, kind);
        self.push(Frame::NodeResult(node));
        self.choice(rules, true);
    }
    fn sequence(
        &mut self,
        parser: &mut Parser<'_>,
        kind: SyntaxKind,
        ops: &'static [Op],
        keep_limited: bool,
    ) {
        let node = self.node(parser, kind);
        self.push(Frame::Sequence(node, ops, 0, keep_limited));
        self.push(Frame::Operation(ops[0]));
    }
    fn based(
        &mut self,
        parser: &mut Parser<'_>,
        rule: RuleId,
        kind: SyntaxKind,
        prefix: &'static str,
        recover: bool,
    ) {
        let node = self.node(parser, kind);
        self.push(Frame::BasedPrefix(Based {
            node,
            rule,
            recover,
        }));
        self.push(Frame::Operation(Op::Tag(prefix)));
    }
    fn failed(&mut self, parser: &mut Parser<'_>, node: Node, keep_limited: bool) {
        if keep_limited && parser.is_halted() {
            node.complete(parser);
            self.result = Attempt::Committed;
        } else {
            self.result = Attempt::NoMatch;
        }
    }
    fn finish_based(&mut self, parser: &mut Parser<'_>, body: Based, payload: bool) {
        if payload {
            body.node.complete(parser);
            self.result = Attempt::Matched;
        } else if !body.recover {
            self.result = Attempt::NoMatch;
        } else {
            let (payload, code, token) = match body.rule {
                rules::DECIMAL_LITERAL => (
                    "decimal digits",
                    "syntax/missing-decimal-digits",
                    Some(SyntaxKind::Digit),
                ),
                rules::HEXADECIMAL_LITERAL => (
                    "hexadecimal digits",
                    "syntax/missing-hexadecimal-digits",
                    None,
                ),
                rules::OCTAL_LITERAL => (
                    "octal digits",
                    "syntax/missing-octal-digits",
                    Some(SyntaxKind::Digit),
                ),
                rules::BINARY_LITERAL => (
                    "binary digits",
                    "syntax/missing-binary-digits",
                    Some(SyntaxKind::Digit),
                ),
                _ => unreachable!("based number recovery owner"),
            };
            insert_missing_based_payload(parser, payload, code, token);
            body.node.complete(parser);
            self.result = Attempt::Committed;
        }
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
            let frame = self.frames.pop().expect("retained literal phase");
            if !matches!(frame, Frame::Base(_) | Frame::Literal(_)) {
                *allowance -= 1;
                self.work += 1;
            }
            match frame {
                Frame::Enter(rule, recover) => {
                    let checkpoint = parser.checkpoint();
                    parser.state.rules.push_canonical(rule);
                    self.push(Frame::Exit(checkpoint));
                    match rule {
                        rules::EMPTY => {
                            let node = self.node(parser, SyntaxKind::EmptyLiteral);
                            self.push(Frame::EmptyFirst(node));
                            self.base(rules::UNDERSCORE);
                        }
                        rules::ATOM => self.sequence(
                            parser,
                            SyntaxKind::AtomLiteral,
                            &[Op::Base(rules::COLON), Op::Base(rules::IDENTIFIER)],
                            false,
                        ),
                        rules::BOOLEAN => {
                            self.choice(&[rules::TRUE_LITERAL, rules::FALSE_LITERAL], true)
                        }
                        rules::TRUE_LITERAL => {
                            self.push(Frame::SetResult);
                            self.push(Frame::Operation(Op::Any(&[
                                Op::Base(rules::ENGLISH_TRUE_LITERAL),
                                Op::Base(rules::CHECK_MARK),
                            ])));
                        }
                        rules::FALSE_LITERAL => {
                            self.push(Frame::SetResult);
                            self.push(Frame::Operation(Op::Any(&[
                                Op::Base(rules::ENGLISH_FALSE_LITERAL),
                                Op::Base(rules::CROSS),
                            ])));
                        }
                        rules::NUMBER => self.aggregate(
                            parser,
                            SyntaxKind::Number,
                            &[rules::COMPLEX_NUMBER, rules::REAL_NUMBER],
                        ),
                        rules::COMPLEX_NUMBER => {
                            let node = self.node(parser, SyntaxKind::ComplexNumber);
                            self.push(Frame::ComplexFirst(node));
                            self.call(rules::UNTYPED_REAL_NUMBER, false);
                        }
                        rules::REAL_NUMBER | rules::UNTYPED_REAL_NUMBER => {
                            let untyped = rule == rules::UNTYPED_REAL_NUMBER;
                            let node = self.node(
                                parser,
                                if untyped {
                                    SyntaxKind::UntypedRealNumber
                                } else {
                                    SyntaxKind::RealNumber
                                },
                            );
                            self.push(Frame::RealLeading(node, untyped, recover));
                            self.base(rules::DASH);
                        }
                        rules::RATIONAL_LITERAL => self.sequence(
                            parser,
                            SyntaxKind::RationalLiteral,
                            &[
                                Op::Call(rules::INTEGER_LITERAL),
                                Op::Base(rules::SLASH),
                                Op::Call(rules::INTEGER_LITERAL),
                            ],
                            true,
                        ),
                        rules::SCIENTIFIC_LITERAL => self.sequence(
                            parser,
                            SyntaxKind::ScientificLiteral,
                            &[
                                Op::Any(FLOAT_OR_INTEGER),
                                Op::Any(&[Op::Tag("e"), Op::Tag("E")]),
                                Op::OptionalBase(rules::PLUS),
                                Op::OptionalBase(rules::DASH),
                                Op::Any(FLOAT_OR_INTEGER),
                            ],
                            true,
                        ),
                        rules::FLOAT_DECIMAL_START => self.sequence(
                            parser,
                            SyntaxKind::FloatDecimalStart,
                            &[Op::Base(rules::PERIOD), Op::Base(rules::DIGIT_SEQUENCE)],
                            true,
                        ),
                        rules::FLOAT_FULL => self.sequence(
                            parser,
                            SyntaxKind::FloatFull,
                            &[
                                Op::Base(rules::DIGIT_SEQUENCE),
                                Op::Base(rules::PERIOD),
                                Op::Base(rules::DIGIT_SEQUENCE),
                            ],
                            true,
                        ),
                        rules::FLOAT_LITERAL => self.aggregate(
                            parser,
                            SyntaxKind::FloatLiteral,
                            &[rules::FLOAT_DECIMAL_START, rules::FLOAT_FULL],
                        ),
                        rules::INTEGER_LITERAL => self.aggregate(
                            parser,
                            SyntaxKind::IntegerLiteral,
                            &[rules::TYPED_INTEGER, rules::UNTYPED_INTEGER],
                        ),
                        rules::TYPED_INTEGER => self.sequence(
                            parser,
                            SyntaxKind::TypedInteger,
                            &[Op::Base(rules::DIGIT_SEQUENCE), Op::Base(rules::IDENTIFIER)],
                            true,
                        ),
                        rules::UNTYPED_INTEGER => self.sequence(
                            parser,
                            SyntaxKind::UntypedInteger,
                            &[Op::Base(rules::DIGIT_SEQUENCE)],
                            false,
                        ),
                        rules::DECIMAL_LITERAL => {
                            self.based(parser, rule, SyntaxKind::DecimalLiteral, "0d", recover)
                        }
                        rules::HEXADECIMAL_LITERAL => {
                            self.based(parser, rule, SyntaxKind::HexadecimalLiteral, "0x", recover)
                        }
                        rules::OCTAL_LITERAL => {
                            self.based(parser, rule, SyntaxKind::OctalLiteral, "0o", recover)
                        }
                        rules::BINARY_LITERAL => {
                            self.based(parser, rule, SyntaxKind::BinaryLiteral, "0b", recover)
                        }
                        _ => unreachable!("supported literal rule"),
                    }
                }
                Frame::Exit(checkpoint) => {
                    parser.state.rules.truncate(checkpoint.rule_depth);
                    // The transaction owns failed wrappers and preserves any child
                    // that already finalized the hard-limit remainder.
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
                Frame::Operation(op) => match op {
                    Op::Base(rule) => self.base(rule),
                    Op::Call(rule) => {
                        self.push(Frame::Accept);
                        self.call(rule, true);
                    }
                    Op::OptionalBase(rule) => {
                        self.push(Frame::Optional);
                        self.base(rule);
                    }
                    Op::Any(ops) => {
                        self.push(Frame::Any(ops, 0));
                        self.push(Frame::Operation(ops[0]));
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
                                .expect("nonempty literal atom"),
                            ));
                        }
                    }
                },
                Frame::Optional => self.matched = true,
                Frame::Any(ops, index) => {
                    if !self.matched
                        && let Some(op) = ops.get(index + 1)
                    {
                        self.push(Frame::Any(ops, index + 1));
                        self.push(Frame::Operation(*op));
                    }
                }
                Frame::Sequence(node, ops, index, keep_limited) => {
                    if !self.matched {
                        self.failed(parser, node, keep_limited);
                    } else if let Some(op) = ops.get(index + 1) {
                        self.push(Frame::Sequence(node, ops, index + 1, keep_limited));
                        self.push(Frame::Operation(*op));
                    } else {
                        node.complete(parser);
                        self.result = Attempt::Matched;
                    }
                }
                Frame::NodeResult(node) => {
                    if self.result.accepted() {
                        node.complete(parser);
                    }
                }
                Frame::Choice(rules, index, recover) => {
                    if self.result == Attempt::NoMatch
                        && let Some(rule) = rules.get(index + 1)
                    {
                        self.push(Frame::Choice(rules, index + 1, recover));
                        self.call(*rule, recover);
                    }
                }
                Frame::EmptyFirst(node) => {
                    if self.matched {
                        self.push(Frame::EmptyRepeat(node));
                        self.base(rules::UNDERSCORE);
                    } else {
                        self.result = Attempt::NoMatch;
                    }
                }
                Frame::EmptyRepeat(node) => {
                    if self.matched && !parser.is_halted() {
                        self.push(Frame::EmptyRepeat(node));
                        self.base(rules::UNDERSCORE);
                    } else {
                        node.complete(parser);
                        self.result = Attempt::Matched;
                    }
                }
                Frame::RealLeading(node, untyped, recover) => {
                    self.push(Frame::NodeResult(node));
                    self.choice(
                        if untyped {
                            &[
                                rules::HEXADECIMAL_LITERAL,
                                rules::DECIMAL_LITERAL,
                                rules::OCTAL_LITERAL,
                                rules::BINARY_LITERAL,
                                rules::SCIENTIFIC_LITERAL,
                                rules::RATIONAL_LITERAL,
                                rules::FLOAT_LITERAL,
                                rules::UNTYPED_INTEGER,
                            ]
                        } else {
                            &[
                                rules::HEXADECIMAL_LITERAL,
                                rules::DECIMAL_LITERAL,
                                rules::OCTAL_LITERAL,
                                rules::BINARY_LITERAL,
                                rules::SCIENTIFIC_LITERAL,
                                rules::RATIONAL_LITERAL,
                                rules::FLOAT_LITERAL,
                                rules::INTEGER_LITERAL,
                            ]
                        },
                        recover,
                    );
                }
                Frame::ComplexFirst(node) => match self.result {
                    Attempt::Matched => {
                        self.push(Frame::ComplexUnit(node));
                        self.push(Frame::Operation(Op::Any(IMAGINARY)));
                    }
                    Attempt::Committed => node.complete(parser),
                    Attempt::NoMatch => {}
                },
                Frame::ComplexUnit(node) => {
                    if self.matched {
                        node.complete(parser);
                        self.result = Attempt::Matched;
                    } else {
                        self.push(Frame::ComplexSign(node));
                        self.push(Frame::Operation(Op::Any(&[
                            Op::Base(rules::PLUS),
                            Op::Base(rules::DASH),
                        ])));
                    }
                }
                Frame::ComplexSign(node) => {
                    if self.matched {
                        self.push(Frame::ComplexSecond(node));
                        self.call(rules::UNTYPED_REAL_NUMBER, false);
                    } else {
                        self.failed(parser, node, true);
                    }
                }
                Frame::ComplexSecond(node) => {
                    if self.result == Attempt::Matched {
                        self.push(Frame::ComplexLast(node));
                        self.push(Frame::Operation(Op::Any(IMAGINARY)));
                    } else {
                        self.failed(parser, node, true);
                    }
                }
                Frame::ComplexLast(node) => {
                    if self.matched {
                        node.complete(parser);
                        self.result = Attempt::Matched;
                    } else {
                        self.failed(parser, node, true);
                    }
                }
                Frame::BasedPrefix(body) => {
                    if !self.matched {
                        self.result = Attempt::NoMatch;
                    } else if body.rule == rules::HEXADECIMAL_LITERAL {
                        self.push(Frame::HexElement(body, parser.offset(), false));
                        self.push(Frame::Operation(Op::Any(HEX_PAYLOAD)));
                    } else {
                        self.push(Frame::BasedPayload(body));
                        self.base(rules::DIGIT_SEQUENCE);
                    }
                }
                Frame::BasedPayload(body) => self.finish_based(parser, body, self.matched),
                Frame::HexElement(body, before, payload) => {
                    if self.matched && !parser.is_halted() && parser.offset() != before {
                        self.push(Frame::HexElement(body, parser.offset(), true));
                        self.push(Frame::Operation(Op::Any(HEX_PAYLOAD)));
                    } else {
                        self.finish_based(parser, body, payload || self.matched);
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
                            panic!("literal continuation source bounds changed")
                        }
                    }
                }
                Frame::LiteralResult(end) => {
                    self.matched = end
                        .and_then(|end| {
                            parser.bump_bytes_token((end - parser.offset()).0, SyntaxKind::Text)
                        })
                        .is_some();
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
            }
        }
        Progress::Complete(self.result)
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::continuation_test_support::{assert_partitions, run};
    use super::*;
    #[test]
    fn numeric_candidates_payload_recovery_and_limits_survive_input_cuts() {
        assert_partitions::<Continuation>(&[
            (rules::EMPTY, ""),
            (rules::EMPTY, "___"),
            (rules::EMPTY, "_\u{301}"),
            (rules::ATOM, ":"),
            (rules::ATOM, ":e\u{301}"),
            (rules::ATOM, ":👩\u{200d}💻"),
            (rules::BOOLEAN, "true"),
            (rules::BOOLEAN, "false"),
            (rules::TRUE_LITERAL, "✓"),
            (rules::FALSE_LITERAL, "✗"),
            (rules::NUMBER, ""),
            (rules::NUMBER, "-"),
            (rules::NUMBER, "12"),
            (rules::NUMBER, "1_234u8"),
            (rules::NUMBER, "0x"),
            (rules::NUMBER, "0xG_2"),
            (rules::NUMBER, "2i"),
            (rules::NUMBER, "2+3j"),
            (rules::NUMBER, "2-3i"),
            (rules::NUMBER, "2+0x"),
            (rules::NUMBER, "2+0xAi"),
            (rules::COMPLEX_NUMBER, "2"),
            (rules::COMPLEX_NUMBER, "2i"),
            (rules::COMPLEX_NUMBER, "2+3j"),
            (rules::COMPLEX_NUMBER, "2+"),
            (rules::COMPLEX_NUMBER, "0x"),
            (rules::COMPLEX_NUMBER, "2+0b"),
            (rules::REAL_NUMBER, "-0x"),
            (rules::REAL_NUMBER, "-12.5"),
            (rules::UNTYPED_REAL_NUMBER, "12u8"),
            (rules::UNTYPED_REAL_NUMBER, "0b"),
            (rules::RATIONAL_LITERAL, "12/3"),
            (rules::RATIONAL_LITERAL, "12/"),
            (rules::SCIENTIFIC_LITERAL, "1.0e+-2.0"),
            (rules::SCIENTIFIC_LITERAL, "1e2"),
            (rules::SCIENTIFIC_LITERAL, "1.0E"),
            (rules::SCIENTIFIC_LITERAL, "1.0e+"),
            (rules::FLOAT_DECIMAL_START, ".1"),
            (rules::FLOAT_DECIMAL_START, "."),
            (rules::FLOAT_FULL, "12.5"),
            (rules::FLOAT_FULL, "12."),
            (rules::FLOAT_LITERAL, ".1"),
            (rules::FLOAT_LITERAL, "12.5"),
            (rules::INTEGER_LITERAL, "1_2u8"),
            (rules::INTEGER_LITERAL, "1_"),
            (rules::TYPED_INTEGER, "1e\u{301}"),
            (rules::TYPED_INTEGER, "1💡"),
            (rules::UNTYPED_INTEGER, "1_2"),
            (rules::UNTYPED_INTEGER, "1\u{301}"),
            (rules::DECIMAL_LITERAL, "0d"),
            (rules::DECIMAL_LITERAL, "0d12_3"),
            (rules::HEXADECIMAL_LITERAL, "0x"),
            (rules::HEXADECIMAL_LITERAL, "0xG_2"),
            (rules::OCTAL_LITERAL, "0o"),
            (rules::OCTAL_LITERAL, "0o89"),
            (rules::BINARY_LITERAL, "0b"),
            (rules::BINARY_LITERAL, "0b23"),
            (rules::NUMBER, "12\r\n"),
            (rules::NUMBER, "0x\u{301}"),
        ]);
    }
    #[test]
    fn numeric_candidate_replay_and_growing_payloads_retain_linear_work() {
        for (rule, prefix, unit, tail) in [
            (rules::NUMBER, "", "1", ""),
            (rules::NUMBER, "", "1", "i"),
            (rules::COMPLEX_NUMBER, "", "1", "+2j"),
            (rules::REAL_NUMBER, "-", "1", ".2"),
            (rules::RATIONAL_LITERAL, "", "1", "/2"),
            (rules::SCIENTIFIC_LITERAL, "", "1", ".0e+2.0"),
            (rules::TYPED_INTEGER, "1", "a", ""),
            (rules::DECIMAL_LITERAL, "0d", "1_", "2"),
            (rules::HEXADECIMAL_LITERAL, "0x", "Ab_", "2"),
            (rules::OCTAL_LITERAL, "0o", "7", ""),
            (rules::BINARY_LITERAL, "0b", "1", ""),
            (rules::EMPTY, "", "_", ""),
            (rules::ATOM, ":", "a", ""),
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
                assert_eq!(observed.result, Attempt::Matched, "{rule:?}");
                assert_eq!(observed.end.to_usize(), text.len(), "{rule:?}");
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
