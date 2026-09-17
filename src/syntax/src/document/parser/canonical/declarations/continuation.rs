//! Retained declaration prefixes, capability groups, and bounded path validation.
use super::*;
use crate::document::TextSize;
use crate::document::parser::literal_scan::{LiteralProgress, LiteralScan};
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
    Tag(&'static str),
    Repeat(&'static Op),
}
const SCHEME: Op = Op::Any(&[
    rules::ALPHA_TOKEN,
    rules::DIGIT_TOKEN,
    rules::DASH,
    rules::PERIOD,
]);
const TAIL: Op = Op::Any(&[
    rules::ALPHA_TOKEN,
    rules::DIGIT_TOKEN,
    rules::DASH,
    rules::PERIOD,
    rules::SLASH,
    rules::UNDERSCORE,
]);
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
struct PathCheck {
    node: Node,
    range: TextRange,
    at: TextSize,
    stars: u8,
    last: [u8; 2],
}
enum Frame {
    Enter(RuleId),
    Exit(ParserCheckpoint),
    Accept,
    SetResult,
    Operation(Op),
    Any(&'static [RuleId], usize),
    Repeat(&'static Op, bool, TextSize),
    Sequence(Node, &'static [Op], usize, bool),
    Path(Node, TextSize),
    Validate(PathCheck),
    Scope(Node),
    ScopePath(Node),
    Group(Node),
    GroupPrefix(Node, ParserCheckpoint, usize),
    GroupSeparator(Node, ParserCheckpoint, ParserCheckpoint),
    GroupItem(Node, ParserCheckpoint, ParserCheckpoint),
    GroupTrailing(Node, ParserCheckpoint, ParserCheckpoint),
    GroupClose(Node, ParserCheckpoint, u8),
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
        assert!(supports(rule), "canonical declaration owner");
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
        if supports(rule) {
            self.push(Frame::Accept);
            self.push(Frame::Enter(rule));
        } else {
            self.base(rule);
        }
    }
    fn node(&mut self, parser: &mut Parser<'_>, kind: SyntaxKind) -> Node {
        Node {
            marker: parser.start(),
            kind,
        }
    }
    fn sequence(
        &mut self,
        parser: &mut Parser<'_>,
        kind: SyntaxKind,
        ops: &'static [Op],
        group: bool,
    ) {
        let node = self.node(parser, kind);
        self.push(Frame::Sequence(node, ops, 0, group));
        self.push(Frame::Operation(ops[0]));
    }
    fn complete_group(&mut self, parser: &mut Parser<'_>, node: Node) {
        node.complete(parser);
        self.result = Attempt::Matched;
    }
    fn trailing(&mut self, parser: &Parser<'_>, node: Node, group: ParserCheckpoint) {
        self.push(Frame::GroupTrailing(node, group, parser.checkpoint()));
        self.base(rules::LIST_SEPARATOR);
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
            let frame = self.frames.pop().expect("retained declaration phase");
            if !matches!(frame, Frame::Base(_) | Frame::Literal(_)) {
                *allowance -= 1;
                self.work += 1;
            }
            match frame {
                Frame::Enter(rule) => {
                    let checkpoint = parser.checkpoint();
                    parser.state.rules.push_canonical(rule);
                    self.push(Frame::Exit(checkpoint));
                    match rule {
                        rules::EXPORT_DECLARATION => self.sequence(
                            parser,
                            SyntaxKind::ExportDeclaration,
                            &[
                                Op::Rule(rules::WHITESPACE0),
                                Op::Rule(rules::MODULE_EXPORT_SIGIL),
                                Op::Rule(rules::WHITESPACE1),
                                Op::Rule(rules::IDENTIFIER),
                            ],
                            false,
                        ),
                        rules::CONTEXT_BASE_CONTEXT => self.sequence(
                            parser,
                            SyntaxKind::ContextBaseContext,
                            &[Op::Rule(rules::AT), Op::Rule(rules::IDENTIFIER)],
                            false,
                        ),
                        rules::CONTEXT_BASE_RESOURCE_URI => self.sequence(
                            parser,
                            SyntaxKind::ContextBaseResourceUri,
                            &[Op::Repeat(&SCHEME), Op::Tag("://"), Op::Repeat(&TAIL)],
                            false,
                        ),
                        rules::CONTEXT_CAPABILITY_DECLARATION => self.sequence(
                            parser,
                            SyntaxKind::ContextCapabilityDeclaration,
                            &[
                                Op::Rule(rules::COLON),
                                Op::Rule(rules::IDENTIFIER),
                                Op::Rule(rules::LEFT_PARENTHESIS),
                                Op::Rule(rules::CONTEXT_CAPABILITY_SCOPE),
                                Op::Rule(rules::RIGHT_PARENTHESIS),
                            ],
                            false,
                        ),
                        rules::CONTEXT_CAPABILITY_PATH_TOKEN => {
                            self.push(Frame::SetResult);
                            self.push(Frame::Operation(Op::Any(&[
                                rules::ALPHA_TOKEN,
                                rules::DIGIT_TOKEN,
                                rules::DASH,
                                rules::SLASH,
                                rules::UNDERSCORE,
                                rules::PERIOD,
                                rules::ASTERISK,
                            ])));
                        }
                        rules::CONTEXT_CAPABILITY_PATH => {
                            let node = self.node(parser, SyntaxKind::ContextCapabilityPath);
                            self.push(Frame::Path(node, parser.offset()));
                            self.push(Frame::Operation(Op::Repeat(&Op::Rule(
                                rules::CONTEXT_CAPABILITY_PATH_TOKEN,
                            ))));
                        }
                        rules::CONTEXT_CAPABILITY_SCOPE => {
                            let node = self.node(parser, SyntaxKind::ContextCapabilityScope);
                            self.push(Frame::Scope(node));
                            self.base(rules::ASTERISK);
                        }
                        rules::CONTEXT_DECLARATION => self.sequence(
                            parser,
                            SyntaxKind::ContextDeclaration,
                            &[
                                Op::Rule(rules::WHITESPACE0),
                                Op::Rule(rules::AT),
                                Op::Rule(rules::IDENTIFIER),
                                Op::Rule(rules::DEFINE_OPERATOR),
                                Op::Any(&[
                                    rules::CONTEXT_BASE_RESOURCE_URI,
                                    rules::CONTEXT_BASE_CONTEXT,
                                ]),
                            ],
                            true,
                        ),
                        _ => unreachable!("supported canonical declaration"),
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
                Frame::Operation(op) => match op {
                    Op::Rule(rule) => self.child(rule),
                    Op::Any(rules) => {
                        self.push(Frame::Any(rules, 0));
                        self.child(rules[0]);
                    }
                    Op::Repeat(op) => {
                        self.push(Frame::Repeat(op, false, parser.offset()));
                        self.push(Frame::Operation(*op));
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
                                .expect("nonempty declaration literal"),
                            ));
                        }
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
                Frame::Repeat(op, any, before) => {
                    if self.matched && parser.offset() != before && !parser.is_halted() {
                        self.push(Frame::Repeat(op, true, parser.offset()));
                        self.push(Frame::Operation(*op));
                    } else {
                        self.matched |= any;
                    }
                }
                Frame::Sequence(node, ops, index, group) => {
                    if !self.matched {
                        if parser.is_halted() {
                            node.complete(parser);
                            self.result = Attempt::Committed;
                        } else {
                            self.result = Attempt::NoMatch;
                        }
                    } else if let Some(op) = ops.get(index + 1) {
                        self.push(Frame::Sequence(node, ops, index + 1, group));
                        self.push(Frame::Operation(*op));
                    } else if group {
                        self.push(Frame::Group(node));
                    } else {
                        node.complete(parser);
                        self.result = Attempt::Matched;
                    }
                }
                Frame::Path(node, start) => {
                    if !self.matched {
                        self.result = Attempt::NoMatch;
                    } else if parser.is_halted() {
                        node.complete(parser);
                        self.result = Attempt::Committed;
                    } else {
                        self.push(Frame::Validate(PathCheck {
                            node,
                            range: TextRange::new(start, parser.offset()),
                            at: start,
                            stars: 0,
                            last: [0, 0],
                        }));
                    }
                }
                Frame::Validate(mut check) => {
                    if check.at < check.range.end {
                        let byte = parser
                            .source()
                            .byte_at(check.at)
                            .expect("retained path byte");
                        if byte == b'*' {
                            check.stars = (check.stars + 1).min(2);
                        }
                        check.last = [check.last[1], byte];
                        check.at = TextSize(check.at.0 + 1);
                        self.push(Frame::Validate(check));
                    } else if check.stars == 0
                        || (check.stars == 1 && check.last == *b"/*" && check.range.len().0 > 2)
                    {
                        check.node.complete(parser);
                        self.result = Attempt::Matched;
                    } else {
                        self.result = Attempt::NoMatch;
                    }
                }
                Frame::Scope(node) => {
                    if self.matched {
                        node.complete(parser);
                        self.result = Attempt::Matched;
                    } else {
                        self.push(Frame::ScopePath(node));
                        self.push(Frame::Enter(rules::CONTEXT_CAPABILITY_PATH));
                    }
                }
                Frame::ScopePath(node) => {
                    if self.result.accepted() {
                        node.complete(parser);
                    }
                }
                Frame::Group(node) => {
                    self.push(Frame::GroupPrefix(node, parser.checkpoint(), 0));
                    self.base(rules::WHITESPACE0);
                }
                Frame::GroupPrefix(node, group, stage) => {
                    if !self.matched {
                        if !parser.is_halted() {
                            parser.rewind(group);
                        }
                        self.complete_group(parser, node);
                    } else if stage < 3 {
                        self.push(Frame::GroupPrefix(node, group, stage + 1));
                        self.child(
                            [
                                rules::LEFT_BRACE,
                                rules::WHITESPACE0,
                                rules::CONTEXT_CAPABILITY_DECLARATION,
                            ][stage],
                        );
                    } else {
                        self.push(Frame::GroupSeparator(node, group, parser.checkpoint()));
                        self.base(rules::LIST_SEPARATOR);
                    }
                }
                Frame::GroupSeparator(node, group, separator) => {
                    if !self.matched {
                        parser.rewind(separator);
                        self.trailing(parser, node, group);
                    } else {
                        self.push(Frame::GroupItem(node, group, separator));
                        self.child(rules::CONTEXT_CAPABILITY_DECLARATION);
                    }
                }
                Frame::GroupItem(node, group, separator) => {
                    if !self.matched {
                        if parser.is_halted() {
                            self.complete_group(parser, node);
                        } else {
                            parser.rewind(separator);
                            self.trailing(parser, node, group);
                        }
                    } else {
                        self.push(Frame::GroupSeparator(node, group, parser.checkpoint()));
                        self.base(rules::LIST_SEPARATOR);
                    }
                }
                Frame::GroupTrailing(node, group, trailing) => {
                    if !self.matched {
                        parser.rewind(trailing);
                    }
                    self.push(Frame::GroupClose(node, group, 0));
                    self.base(rules::WHITESPACE0);
                }
                Frame::GroupClose(node, group, stage) => {
                    if !self.matched {
                        if !parser.is_halted() {
                            parser.rewind(group);
                        }
                        self.complete_group(parser, node);
                    } else if stage == 0 {
                        self.push(Frame::GroupClose(node, group, 1));
                        self.base(rules::RIGHT_BRACE);
                    } else {
                        self.complete_group(parser, node);
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
                            panic!("declaration continuation source bounds changed")
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
    use alloc::string::String;

    #[test]
    fn declaration_choices_groups_and_limits_survive_input_cuts() {
        assert_partitions::<Continuation>(&[
            (rules::EXPORT_DECLARATION, "<+ value"),
            (rules::EXPORT_DECLARATION, "\r\n<+\t💡"),
            (rules::EXPORT_DECLARATION, "<+value"),
            (rules::EXPORT_DECLARATION, "<+\u{a0}value"),
            (rules::CONTEXT_BASE_CONTEXT, "@main/sub"),
            (rules::CONTEXT_BASE_CONTEXT, "@💡\u{301}"),
            (rules::CONTEXT_BASE_CONTEXT, "@"),
            (rules::CONTEXT_BASE_RESOURCE_URI, "1.0://a_b/path"),
            (rules::CONTEXT_BASE_RESOURCE_URI, "fs://"),
            (rules::CONTEXT_BASE_RESOURCE_URI, "://x"),
            (rules::CONTEXT_CAPABILITY_PATH_TOKEN, "*"),
            (rules::CONTEXT_CAPABILITY_PATH_TOKEN, "é\u{301}"),
            (rules::CONTEXT_CAPABILITY_PATH_TOKEN, "*\u{301}"),
            (rules::CONTEXT_CAPABILITY_PATH, "users/*"),
            (rules::CONTEXT_CAPABILITY_PATH, "///*"),
            (rules::CONTEXT_CAPABILITY_PATH, "*"),
            (rules::CONTEXT_CAPABILITY_PATH, "/*"),
            (rules::CONTEXT_CAPABILITY_PATH, "foo*"),
            (rules::CONTEXT_CAPABILITY_PATH, "foo/*/bar"),
            (rules::CONTEXT_CAPABILITY_PATH, "foo/**"),
            (rules::CONTEXT_CAPABILITY_SCOPE, "**"),
            (rules::CONTEXT_CAPABILITY_SCOPE, "*/foo"),
            (rules::CONTEXT_CAPABILITY_SCOPE, "users/read"),
            (rules::CONTEXT_CAPABILITY_DECLARATION, ":read(*)"),
            (rules::CONTEXT_CAPABILITY_DECLARATION, ":read(users/*)"),
            (rules::CONTEXT_CAPABILITY_DECLARATION, ":read(foo*)"),
            (rules::CONTEXT_CAPABILITY_DECLARATION, ":read("),
            (rules::CONTEXT_DECLARATION, "@ui := fs://workspace"),
            (rules::CONTEXT_DECLARATION, "@ui := @main"),
            (rules::CONTEXT_DECLARATION, "@ui := @main{}"),
            (rules::CONTEXT_DECLARATION, "@ui := @main { :read(*) }"),
            (rules::CONTEXT_DECLARATION, "@ui := @main{:read(*),}"),
            (
                rules::CONTEXT_DECLARATION,
                "@ui := @main{:read(*), :write(users/*)}",
            ),
            (
                rules::CONTEXT_DECLARATION,
                "@ui := @main{:read(*), :write(foo*)}",
            ),
            (rules::CONTEXT_DECLARATION, "@ui := @main{:read(*)"),
            (rules::CONTEXT_DECLARATION, "@ui := @main{:read(*),"),
            (
                rules::CONTEXT_DECLARATION,
                "@ui := @main\r\n{\t:read(*)\r\n}",
            ),
        ]);
    }

    #[test]
    fn declaration_paths_validation_and_capability_groups_retain_linear_work() {
        for (rule, prefix, unit, tail, expected) in [
            (rules::EXPORT_DECLARATION, "", " ", "<+ value", None),
            (rules::CONTEXT_BASE_CONTEXT, "@", "a", "", None),
            (rules::CONTEXT_BASE_RESOURCE_URI, "", "a", "://path", None),
            (
                rules::CONTEXT_BASE_RESOURCE_URI,
                "fs://",
                "a/",
                "path",
                None,
            ),
            (rules::CONTEXT_CAPABILITY_PATH, "", "a", "/*", None),
            (rules::CONTEXT_CAPABILITY_PATH, "", "a", "/**", Some(0)),
            (rules::CONTEXT_CAPABILITY_PATH, "", "a", "/*/x", Some(0)),
            (
                rules::CONTEXT_DECLARATION,
                "@ui := @main{",
                ":read(*),",
                "}",
                None,
            ),
            (
                rules::CONTEXT_DECLARATION,
                "@ui := @main{",
                ":read(*),",
                ":write(foo*)}",
                Some(12),
            ),
            (
                rules::CONTEXT_DECLARATION,
                "@ui := @main{",
                ":read(*),",
                "",
                Some(12),
            ),
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
                assert_eq!(observed.end.0 as usize, expected.unwrap_or(text.len()));
                assert_eq!(
                    observed.result,
                    if expected == Some(0) {
                        Attempt::NoMatch
                    } else {
                        Attempt::Matched
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
