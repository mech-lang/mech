//! Retained source-import alternatives, path validation, and wildcard diagnostics.
use super::*;
use crate::document::parser::checkpoint::ParserCheckpoint;
use crate::document::parser::grapheme_scan::ScanSource;
use crate::document::parser::literal_scan::{LiteralProgress, LiteralScan};
use crate::document::parser::marker::Marker;
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
    Sequence(&'static [Op]),
    Choice(&'static [&'static [Op]]),
    Optional(&'static [Op]),
    Repeat(RuleId, bool),
    Tag(&'static str),
}
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
    at: TextSize,
    end: TextSize,
    leaf_dot: bool,
    last: [u8; 4],
}
struct WildcardCheck {
    node: Node,
    invalid: Marker,
    wrapper: ParserCheckpoint,
    at: TextSize,
    end: TextSize,
    semantic_end: TextSize,
    first: Option<TextRange>,
    labels: Vec<DiagnosticLabel>,
}
enum Frame {
    Enter(RuleId),
    Exit(ParserCheckpoint),
    Accept,
    SetResult,
    Finish(Node),
    Aggregate(Node),
    Operation(Op),
    Any(&'static [RuleId], usize),
    Sequence(&'static [Op], usize),
    Choice(&'static [&'static [Op]], usize, ParserCheckpoint),
    Optional(ParserCheckpoint),
    Repeat(RuleId, bool, TextSize),
    TailProbe(Node, bool, ParserCheckpoint, bool),
    TailToken(Node, bool, TextSize),
    PathFirst(Node, TextSize),
    PathSlash(Node, TextSize, ParserCheckpoint),
    PathComponent(Node, TextSize, ParserCheckpoint),
    Validate(PathCheck),
    ImportPrefix(Node),
    ImportSpecifier(Node, Marker, ParserCheckpoint, TextSize),
    Wildcards(WildcardCheck),
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
        assert!(supports(rule), "canonical source-import owner");
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
    fn node(parser: &mut Parser<'_>, kind: SyntaxKind) -> Node {
        Node {
            marker: parser.start(),
            kind,
        }
    }
    fn sequence(&mut self, parser: &mut Parser<'_>, kind: SyntaxKind, ops: &'static [Op]) {
        self.push(Frame::Finish(Self::node(parser, kind)));
        self.push(Frame::Operation(Op::Sequence(ops)));
    }
    fn tail_probe(&mut self, parser: &Parser<'_>, node: Node, any: bool) {
        self.push(Frame::TailProbe(node, any, parser.checkpoint(), false));
        self.base(rules::NEW_LINE);
    }
    fn finish_tail(&mut self, parser: &mut Parser<'_>, node: Node, any: bool) {
        self.result = if any {
            node.complete(parser);
            Attempt::Matched
        } else {
            Attempt::NoMatch
        };
    }
    fn path_pair(&mut self, parser: &Parser<'_>, node: Node, start: TextSize) {
        self.push(Frame::PathSlash(node, start, parser.checkpoint()));
        self.base(rules::SLASH);
    }
    fn validate_path(&mut self, parser: &Parser<'_>, node: Node, start: TextSize) {
        self.push(Frame::Validate(PathCheck {
            node,
            at: start,
            end: parser.offset(),
            leaf_dot: false,
            last: [0; 4],
        }));
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
            let frame = self.frames.pop().expect("retained source-import phase");
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
                        rules::SOURCE_IMPORT_TAIL => {
                            let node = Self::node(parser, SyntaxKind::SourceImportTail);
                            self.tail_probe(parser, node, false);
                        }
                        rules::SOURCE_PATH_COMPONENT_TOKEN | rules::URI_SCHEME_PART => {
                            self.push(Frame::SetResult);
                            self.push(Frame::Operation(Op::Any(
                                if rule == rules::SOURCE_PATH_COMPONENT_TOKEN {
                                    &[
                                        rules::ALPHA_TOKEN,
                                        rules::DIGIT_TOKEN,
                                        rules::DASH,
                                        rules::UNDERSCORE,
                                        rules::PERIOD,
                                        rules::PERCENT,
                                    ]
                                } else {
                                    &[
                                        rules::ALPHA_TOKEN,
                                        rules::DIGIT_TOKEN,
                                        rules::PLUS,
                                        rules::DASH,
                                        rules::PERIOD,
                                        rules::PERCENT,
                                    ]
                                },
                            )));
                        }
                        rules::SOURCE_PATH_COMPONENT => self.sequence(
                            parser,
                            SyntaxKind::SourcePathComponent,
                            &[Op::Repeat(rules::SOURCE_PATH_COMPONENT_TOKEN, true)],
                        ),
                        rules::SOURCE_MEC_PATH => {
                            let node = Self::node(parser, SyntaxKind::SourceMecPath);
                            self.push(Frame::PathFirst(node, parser.offset()));
                            self.child(rules::SOURCE_PATH_COMPONENT);
                        }
                        rules::SOURCE_MEC_PATH_WILDCARD_SUFFIX => {
                            self.push(Frame::SetResult);
                            self.push(Frame::Operation(Op::Optional(&[
                                Op::Rule(rules::SLASH),
                                Op::Rule(rules::ASTERISK),
                            ])));
                        }
                        rules::RELATIVE_SOURCE_IMPORT_SPECIFIER => self.sequence(
                            parser,
                            SyntaxKind::RelativeSourceImportSpecifier,
                            &[
                                Op::Choice(&[
                                    &[
                                        Op::Rule(rules::PERIOD),
                                        Op::Rule(rules::PERIOD),
                                        Op::Rule(rules::SLASH),
                                    ],
                                    &[Op::Rule(rules::PERIOD), Op::Rule(rules::SLASH)],
                                ]),
                                Op::Rule(rules::SOURCE_MEC_PATH),
                                Op::Rule(rules::SOURCE_MEC_PATH_WILDCARD_SUFFIX),
                            ],
                        ),
                        rules::ABSOLUTE_SOURCE_IMPORT_SPECIFIER => self.sequence(
                            parser,
                            SyntaxKind::AbsoluteSourceImportSpecifier,
                            &[
                                Op::Rule(rules::SLASH),
                                Op::Rule(rules::SOURCE_MEC_PATH),
                                Op::Rule(rules::SOURCE_MEC_PATH_WILDCARD_SUFFIX),
                            ],
                        ),
                        rules::BARE_SOURCE_IMPORT_SPECIFIER => self.sequence(
                            parser,
                            SyntaxKind::BareSourceImportSpecifier,
                            &[
                                Op::Rule(rules::SOURCE_MEC_PATH),
                                Op::Rule(rules::SOURCE_MEC_PATH_WILDCARD_SUFFIX),
                            ],
                        ),
                        rules::SOURCE_IMPORT_URI_SCHEME => self.sequence(
                            parser,
                            SyntaxKind::SourceImportUriScheme,
                            &[
                                Op::Rule(rules::ALPHA_TOKEN),
                                Op::Repeat(rules::URI_SCHEME_PART, false),
                            ],
                        ),
                        rules::URI_SOURCE_IMPORT_SPECIFIER => self.sequence(
                            parser,
                            SyntaxKind::UriSourceImportSpecifier,
                            &[
                                Op::Rule(rules::SOURCE_IMPORT_URI_SCHEME),
                                Op::Tag("://"),
                                Op::Rule(rules::SOURCE_IMPORT_TAIL),
                            ],
                        ),
                        rules::SOURCE_IMPORT_SPECIFIER => {
                            self.push(Frame::Aggregate(Self::node(
                                parser,
                                SyntaxKind::SourceImportSpecifier,
                            )));
                            self.push(Frame::Operation(Op::Any(&[
                                rules::RELATIVE_SOURCE_IMPORT_SPECIFIER,
                                rules::ABSOLUTE_SOURCE_IMPORT_SPECIFIER,
                                rules::URI_SOURCE_IMPORT_SPECIFIER,
                                rules::BARE_SOURCE_IMPORT_SPECIFIER,
                            ])));
                        }
                        rules::IMPORT_DECLARATION => {
                            self.push(Frame::ImportPrefix(Self::node(
                                parser,
                                SyntaxKind::ImportDeclaration,
                            )));
                            self.push(Frame::Operation(Op::Sequence(&[
                                Op::Rule(rules::WHITESPACE0),
                                Op::Rule(rules::IMPORT_SIGIL),
                                Op::Rule(rules::SPACE_TAB1),
                            ])));
                        }
                        _ => unreachable!("supported canonical source import"),
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
                Frame::Finish(node) => {
                    self.result = if self.matched {
                        node.complete(parser);
                        Attempt::Matched
                    } else {
                        Attempt::NoMatch
                    };
                }
                Frame::Aggregate(node) => {
                    if self.result.accepted() {
                        node.complete(parser);
                    }
                }
                Frame::Operation(op) => match op {
                    Op::Rule(rule) => self.child(rule),
                    Op::Any(rules) => {
                        self.push(Frame::Any(rules, 0));
                        self.child(rules[0]);
                    }
                    Op::Sequence(ops) => {
                        self.push(Frame::Sequence(ops, 0));
                        self.push(Frame::Operation(ops[0]));
                    }
                    Op::Choice(choices) => {
                        self.push(Frame::Choice(choices, 0, parser.checkpoint()));
                        self.push(Frame::Operation(Op::Sequence(choices[0])));
                    }
                    Op::Optional(ops) => {
                        self.push(Frame::Optional(parser.checkpoint()));
                        self.push(Frame::Operation(Op::Sequence(ops)));
                    }
                    Op::Repeat(rule, require_one) => {
                        self.push(Frame::Repeat(rule, !require_one, parser.offset()));
                        self.child(rule);
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
                                .expect("nonempty source-import literal"),
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
                Frame::Sequence(ops, index) => {
                    if self.matched
                        && let Some(op) = ops.get(index + 1)
                    {
                        self.push(Frame::Sequence(ops, index + 1));
                        self.push(Frame::Operation(*op));
                    }
                }
                Frame::Choice(choices, index, checkpoint) => {
                    if !self.matched
                        && let Some(ops) = choices.get(index + 1)
                    {
                        parser.rewind(checkpoint);
                        self.push(Frame::Choice(choices, index + 1, checkpoint));
                        self.push(Frame::Operation(Op::Sequence(ops)));
                    }
                }
                Frame::Optional(checkpoint) => {
                    if !self.matched {
                        parser.rewind(checkpoint);
                    }
                    self.matched = true;
                }
                Frame::Repeat(rule, any, before) => {
                    if self.matched && parser.offset() != before && !parser.is_halted() {
                        self.push(Frame::Repeat(rule, true, parser.offset()));
                        self.child(rule);
                    } else {
                        self.matched |= any;
                    }
                }
                Frame::TailProbe(node, any, checkpoint, semicolon) => {
                    parser.rewind(checkpoint);
                    if self.matched {
                        self.finish_tail(parser, node, any);
                    } else if !semicolon {
                        self.push(Frame::TailProbe(node, any, parser.checkpoint(), true));
                        self.base(rules::SEMICOLON);
                    } else {
                        self.push(Frame::TailToken(node, any, parser.offset()));
                        self.base(rules::ANY_TOKEN);
                    }
                }
                Frame::TailToken(node, any, before) => {
                    if self.matched && parser.offset() != before && !parser.is_halted() {
                        self.tail_probe(parser, node, true);
                    } else {
                        self.finish_tail(parser, node, any);
                    }
                }
                Frame::PathFirst(node, start) => {
                    if self.matched {
                        self.path_pair(parser, node, start);
                    } else {
                        self.result = Attempt::NoMatch;
                    }
                }
                Frame::PathSlash(node, start, pair) => {
                    if self.matched {
                        self.push(Frame::PathComponent(node, start, pair));
                        self.child(rules::SOURCE_PATH_COMPONENT);
                    } else {
                        parser.rewind(pair);
                        self.validate_path(parser, node, start);
                    }
                }
                Frame::PathComponent(node, start, pair) => {
                    if !self.matched {
                        parser.rewind(pair);
                        self.validate_path(parser, node, start);
                    } else if parser.is_halted() {
                        self.validate_path(parser, node, start);
                    } else {
                        self.path_pair(parser, node, start);
                    }
                }
                Frame::Validate(mut check) => {
                    if check.at < check.end {
                        let byte = parser
                            .source()
                            .byte_at(check.at)
                            .expect("retained path byte");
                        if byte == b'/' {
                            check.leaf_dot = false;
                            check.last = [0; 4];
                        } else {
                            check.leaf_dot |= byte == b'.';
                            check.last = [check.last[1], check.last[2], check.last[3], byte];
                        }
                        check.at = TextSize(check.at.0 + 1);
                        self.push(Frame::Validate(check));
                    } else {
                        self.result = if !check.leaf_dot || check.last == *b".mec" {
                            check.node.complete(parser);
                            Attempt::Matched
                        } else {
                            Attempt::NoMatch
                        };
                    }
                }
                Frame::ImportPrefix(node) => {
                    if self.matched {
                        let wrapper = parser.checkpoint();
                        let invalid = parser.start();
                        self.push(Frame::ImportSpecifier(
                            node,
                            invalid,
                            wrapper,
                            parser.offset(),
                        ));
                        self.child(rules::SOURCE_IMPORT_SPECIFIER);
                    } else {
                        self.result = Attempt::NoMatch;
                    }
                }
                Frame::ImportSpecifier(node, invalid, wrapper, start) => {
                    if self.matched {
                        self.push(Frame::Wildcards(WildcardCheck {
                            node,
                            invalid,
                            wrapper,
                            at: start,
                            end: parser.offset(),
                            semantic_end: start,
                            first: None,
                            labels: Vec::new(),
                        }));
                    } else {
                        self.result = Attempt::NoMatch;
                    }
                }
                Frame::Wildcards(mut check) => {
                    if check.at < check.end {
                        let scalar = parser
                            .source()
                            .scalar_at(check.at.to_usize())
                            .expect("retained specifier scalar");
                        let character = scalar.chars().next().expect("one scalar");
                        let end = TextSize(check.at.0 + scalar.len() as u32);
                        assert!(end <= check.end);
                        if !character.is_whitespace() {
                            check.semantic_end = end;
                        }
                        if character == '*' {
                            let range = TextRange::new(check.at, end);
                            if check.first.is_none() {
                                check.first = Some(range);
                            } else {
                                check.labels.push(DiagnosticLabel {
                                    anchor: DiagnosticAnchor::Absolute {
                                        revision: parser.source().revision(),
                                        range,
                                    },
                                    message: String::from("additional wildcard"),
                                });
                            }
                        }
                        check.at = end;
                        self.push(Frame::Wildcards(check));
                    } else {
                        let valid = check.first.is_none()
                            || (check.labels.is_empty()
                                && check.semantic_end.0 >= 2
                                && parser.source().byte_at(TextSize(check.semantic_end.0 - 2))
                                    == Some(b'/')
                                && parser.source().byte_at(TextSize(check.semantic_end.0 - 1))
                                    == Some(b'*'));
                        if valid {
                            if parser.state.resource_finalizing {
                                parser.rewind(check.wrapper);
                            } else {
                                check.invalid.abandon(parser);
                            }
                            self.result = Attempt::Matched;
                        } else {
                            check.invalid.complete_with_flags(
                                parser,
                                SyntaxKind::Error,
                                NodeFlags::ERROR,
                            );
                            let diagnostic = Diagnostic {
                                id: parser.next_diagnostic_id(),
                                code: DiagnosticCode::from("syntax/invalid-source-import-wildcard"),
                                phase: DiagnosticPhase::Syntax,
                                severity: Severity::Error,
                                rule: parser.current_rule(),
                                context: parser.current_context(),
                                primary: DiagnosticAnchor::Absolute {
                                    revision: parser.source().revision(),
                                    range: check.first.expect("invalid wildcard"),
                                },
                                labels: check.labels,
                                expected: alloc::vec![],
                                found: None,
                                fixes: alloc::vec![],
                                related: alloc::vec![],
                                recovery: None,
                                tags: DiagnosticTags::NONE,
                                message: String::from(
                                    "source-import wildcard must be the sole final `/*` suffix",
                                ),
                            };
                            parser.push_diagnostic(
                                diagnostic,
                                None,
                                TextRange::empty(TextSize::ZERO),
                            );
                            self.result = Attempt::Committed;
                        }
                        check.node.complete(parser);
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
                            panic!("source-import continuation bounds changed")
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
    fn source_import_alternatives_validation_and_limits_survive_input_cuts() {
        assert_partitions::<Continuation>(&[
            (rules::SOURCE_IMPORT_TAIL, "dep;rest"),
            (rules::SOURCE_IMPORT_TAIL, "dep\r\nrest"),
            (rules::SOURCE_IMPORT_TAIL, "💡\u{301}\t"),
            (rules::SOURCE_IMPORT_TAIL, "\n"),
            (rules::SOURCE_PATH_COMPONENT_TOKEN, "%"),
            (rules::SOURCE_PATH_COMPONENT_TOKEN, "é\u{301}"),
            (rules::SOURCE_PATH_COMPONENT, "foo-1_bar.mec"),
            (rules::SOURCE_MEC_PATH, "foo.mec/bar.mec"),
            (rules::SOURCE_MEC_PATH, "foo.mec/bar.txt"),
            (rules::SOURCE_MEC_PATH, "foo.mec/"),
            (rules::SOURCE_MEC_PATH, "foo.mec/*"),
            (rules::SOURCE_MEC_PATH, ".mec"),
            (rules::SOURCE_MEC_PATH, "foo.MEC"),
            (rules::SOURCE_MEC_PATH, "foo.mec/bar"),
            (rules::SOURCE_MEC_PATH_WILDCARD_SUFFIX, ""),
            (rules::SOURCE_MEC_PATH_WILDCARD_SUFFIX, "/"),
            (rules::SOURCE_MEC_PATH_WILDCARD_SUFFIX, "/x"),
            (rules::SOURCE_MEC_PATH_WILDCARD_SUFFIX, "/*/x"),
            (rules::RELATIVE_SOURCE_IMPORT_SPECIFIER, "../lib/foo.mec/*"),
            (rules::RELATIVE_SOURCE_IMPORT_SPECIFIER, "./foo.mec"),
            (rules::RELATIVE_SOURCE_IMPORT_SPECIFIER, "../"),
            (rules::RELATIVE_SOURCE_IMPORT_SPECIFIER, ".."),
            (rules::ABSOLUTE_SOURCE_IMPORT_SPECIFIER, "/lib/foo.mec"),
            (rules::ABSOLUTE_SOURCE_IMPORT_SPECIFIER, "/"),
            (rules::BARE_SOURCE_IMPORT_SPECIFIER, "./foo.mec"),
            (rules::BARE_SOURCE_IMPORT_SPECIFIER, "foo.mec/*"),
            (rules::URI_SCHEME_PART, "+"),
            (rules::URI_SCHEME_PART, "%"),
            (rules::SOURCE_IMPORT_URI_SCHEME, "git+ssh"),
            (rules::SOURCE_IMPORT_URI_SCHEME, "1git"),
            (
                rules::URI_SOURCE_IMPORT_SPECIFIER,
                "https://example.com/dep.mec",
            ),
            (rules::URI_SOURCE_IMPORT_SPECIFIER, "x://   "),
            (rules::URI_SOURCE_IMPORT_SPECIFIER, "x://"),
            (rules::SOURCE_IMPORT_SPECIFIER, "foo.mec://bar"),
            (rules::SOURCE_IMPORT_SPECIFIER, "foo.mec"),
            (rules::SOURCE_IMPORT_SPECIFIER, "../foo.txt"),
            (rules::IMPORT_DECLARATION, "+> dep.mec"),
            (rules::IMPORT_DECLARATION, "+>\n dep.mec"),
            (rules::IMPORT_DECLARATION, "+> dep.mec/*/x"),
            (rules::IMPORT_DECLARATION, "+> https://x/a*b"),
            (rules::IMPORT_DECLARATION, "+> https://x/*/y"),
            (rules::IMPORT_DECLARATION, "+> https://x/**"),
            (rules::IMPORT_DECLARATION, "+> https://x/path/*   "),
            (rules::IMPORT_DECLARATION, "+> x://*"),
            (rules::IMPORT_DECLARATION, "+> x:///*"),
            (rules::IMPORT_DECLARATION, "+> x://é/**\u{a0}\u{2009}"),
            (rules::IMPORT_DECLARATION, "+> x://é/*\u{a0}\u{2009}"),
            (rules::IMPORT_DECLARATION, "+> x://é/*\u{a0}x"),
            (rules::IMPORT_DECLARATION, "+> x://é/*\u{301}"),
        ]);
    }

    #[test]
    fn source_import_paths_tails_and_diagnostic_scans_retain_linear_work() {
        for (rule, prefix, unit, tail, outcome, end, diagnostics) in [
            (
                rules::SOURCE_PATH_COMPONENT,
                "",
                "a",
                ".mec",
                Attempt::Matched,
                None,
                0,
            ),
            (
                rules::SOURCE_MEC_PATH,
                "",
                "a/",
                "leaf.mec",
                Attempt::Matched,
                None,
                0,
            ),
            (
                rules::SOURCE_MEC_PATH,
                "",
                "a/",
                "leaf.txt",
                Attempt::NoMatch,
                Some(0),
                0,
            ),
            (
                rules::SOURCE_IMPORT_URI_SCHEME,
                "a",
                "+a",
                "",
                Attempt::Matched,
                None,
                0,
            ),
            (
                rules::SOURCE_IMPORT_TAIL,
                "",
                "💡",
                "",
                Attempt::Matched,
                None,
                0,
            ),
            (
                rules::SOURCE_IMPORT_SPECIFIER,
                "",
                "a",
                ".mec",
                Attempt::Matched,
                None,
                0,
            ),
            (
                rules::IMPORT_DECLARATION,
                "+> https://x/",
                "a/",
                "leaf.mec",
                Attempt::Matched,
                None,
                0,
            ),
            (
                rules::IMPORT_DECLARATION,
                "+> https://x/*",
                "\u{2009}",
                "",
                Attempt::Matched,
                None,
                0,
            ),
            (
                rules::IMPORT_DECLARATION,
                "+> https://x/",
                "*",
                "",
                Attempt::Committed,
                None,
                1,
            ),
            (
                rules::IMPORT_DECLARATION,
                "+> https://x/*",
                "\u{2009}",
                "x",
                Attempt::Committed,
                None,
                1,
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
                assert_eq!(observed, baseline, "{rule:?}, {tail:?}");
                assert_eq!(observed.stats.source_bytes as usize, text.len());
                assert_eq!(observed.stats.diagnostics_emitted, diagnostics);
                assert_eq!(observed.result, outcome);
                assert_eq!(observed.end.to_usize(), end.unwrap_or(text.len()));
                if let Some((prior_streamed, prior_one_shot)) = previous {
                    assert!(work <= prior_streamed * 3, "append restarted {rule:?}");
                    assert!(one_shot_work <= prior_one_shot * 3);
                }
                previous = Some((work, one_shot_work));
            }
        }
    }
}
