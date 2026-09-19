//! Owned kind parents: primitive choice, suffixes, delimiters and recovery.
use super::super::{
    precedence::{Continuation as Core, Progress},
    required::{Closer, Required},
};
use super::*;
use crate::document::RuleId;
use crate::document::parser::ParserCheckpoint;
use crate::document::parser::recovery::{NestingContinuation, NestingProgress};
use alloc::{boxed::Box, vec::Vec};

pub(in super::super) fn supports(rule: RuleId) -> bool {
    matches!(
        rule,
        rules::KIND
            | rules::KIND_WITH_OPTION
            | rules::KIND_ANNOTATION
            | rules::KIND_KIND
            | rules::KIND_MATRIX
            | rules::KIND_SCALAR
            | rules::KIND_TUPLE
            | rules::KIND_TABLE
            | rules::KIND_SET
            | rules::KIND_MAP
            | rules::KIND_RECORD
    )
}
#[derive(Clone, Copy)]
struct Delimited {
    rule: RuleId,
    close: RuleId,
    close_kind: SyntaxKind,
    close_text: &'static str,
    content: RuleId,
    code: &'static str,
    message: &'static str,
    recover: bool,
}
#[derive(Clone, Copy)]
struct Brace {
    checkpoint: ParserCheckpoint,
    map: Marker,
    set: Marker,
    record: Marker,
}
enum Frame {
    Nesting(Box<NestingContinuation>),
    Tag(Box<base::ExactTag<'static>>),
    DirectBrace(Marker, RuleId),
    SetChild,
    SetClose,
    SetInterior(Marker),
    SetFinish(Marker, bool),
    MapKey,
    MapColon(bool),
    MapValue(bool),
    MapRecovered,
    MapClose(bool),
    SetSuffix,
    SetSuffixColon(ParserCheckpoint),
    SetSuffixLiteral(ParserCheckpoint),
    Matched,
    RecordStart,
    RecordFirst,
    RecordLoop(bool),
    RecordSeparator(bool, ParserCheckpoint),
    RecordSeparatorSpace(bool, ParserCheckpoint),
    RecordNext(bool, ParserCheckpoint),
    RecordSuffix(bool),
    RecordSpace(bool),
    RecordClose(bool),
    Brace,
    BraceExit(ParserCheckpoint),
    BraceOpen(Brace),
    BraceLimited(Brace),
    BraceWhitespace(Brace, ParserCheckpoint),
    BraceRecord(Brace, ParserCheckpoint, bool),
    BraceRecordFinish(Brace),
    PlainKind,
    PlainFinish(Marker, SyntaxKind),
    BraceScalar(Brace, ParserCheckpoint),
    BraceScalarColon(Brace, ParserCheckpoint),
    BraceScalarValue(Brace),
    BraceScalarClose,
    BraceScalarRecover,
    BraceScalarFinish(Brace),
    BraceElement(Brace),
    BraceElementColon(Brace, bool),
    BraceMapFinish(Brace),
    BraceSetRecovered(Brace),
    BraceSetClose(Brace),
    BraceSetFinish(Brace, bool),
    Call(RuleId),
    Kind(KindPosition),
    Matrix(KindPosition),
    Annotation(bool),
    Exit(ParserCheckpoint),
    Finish(Marker, SyntaxKind),
    Base(base::continuation::Continuation),
    Primitive(leaves::Continuation),
    Core(Box<Core>),
    Required(Box<Required<'static>>),
    Closer(Box<Closer<'static>>),
    KindSelect(KindPosition),
    Choice(usize),
    OptionChild(Marker),
    OptionSuffix(Marker, Attempt),
    ScalarStem(Marker),
    ScalarColon(Marker, ParserCheckpoint),
    ScalarRange(Marker, ParserCheckpoint),
    Open(Delimited),
    DelimitedChild(Delimited),
    DelimitedRecovered(Delimited),
    DelimitedClose(Delimited, bool),
    PopNesting,
    MatrixOpen(Marker, KindPosition),
    MatrixInterior(Marker, KindPosition),
    MatrixColon(Marker, KindPosition, Attempt, ParserCheckpoint),
    MatrixLiteral(Marker, KindPosition, Attempt, ParserCheckpoint, bool),
    MatrixLoop(Marker, Attempt),
    MatrixSeparator(Marker, Attempt, ParserCheckpoint),
    MatrixNextLiteral(Marker, Attempt, ParserCheckpoint),
    TupleOpen(Marker),
    TupleItem(Marker, bool, bool),
    TupleRecovered(Marker),
    TupleLoop(Marker, bool),
    TupleSeparator(Marker, bool),
    TupleClose(bool),
    TableOpen(Marker),
    TableField(Marker, bool, bool),
    TableFieldRecovered(Marker),
    TableLoop(Marker, bool),
    TableSeparator(Marker, bool),
    TableSeparatorSpace(Marker, bool),
    TableClose(Marker, bool),
    TableColon(Marker, bool, ParserCheckpoint),
    TableSuffix(Marker, bool, ParserCheckpoint),
    Field(bool),
    FieldStem(bool),
    FieldAnnotation(bool),
}
pub(in super::super) struct KindContinuation {
    frames: Vec<Frame>,
    result: Attempt,
    pub work: u64,
}
impl KindContinuation {
    pub fn new(rule: RuleId) -> Self {
        Self {
            frames: alloc::vec![Frame::Call(rule)],
            result: Attempt::NoMatch,
            work: 0,
        }
    }
    pub fn kind(position: KindPosition) -> Self {
        Self {
            frames: alloc::vec![Frame::Kind(position)],
            result: Attempt::NoMatch,
            work: 0,
        }
    }
    pub fn matrix(position: KindPosition) -> Self {
        Self {
            frames: alloc::vec![Frame::Matrix(position)],
            result: Attempt::NoMatch,
            work: 0,
        }
    }
    pub fn annotation(recover: bool) -> Self {
        Self {
            frames: alloc::vec![Frame::Annotation(recover)],
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
                _ => unreachable!("sealed kind input"),
            }
        }
    }
    fn push(&mut self, frame: Frame) {
        self.frames.push(frame);
    }
    fn base(&mut self, rule: RuleId) {
        self.push(Frame::Base(base::continuation::Continuation::new(rule)));
    }
    fn transaction(&mut self, parser: &mut Parser<'_>, rule: RuleId) {
        let checkpoint = parser.checkpoint();
        parser.state.rules.push_canonical(rule);
        self.push(Frame::Exit(checkpoint));
    }
    fn finish(&mut self, parser: &mut Parser<'_>, node: Marker, kind: SyntaxKind) {
        self.result = finish(node, parser, kind, self.result);
    }
    fn recover(&mut self, spec: Delimited) {
        self.push(Frame::Required(Box::new(Required::new(
            spec.rule,
            spec.code,
            spec.message,
            "kind",
            &[],
            &[],
            None,
        ))));
    }
    fn closer(&mut self, spec: Delimited) {
        self.push(Frame::Closer(Box::new(Closer::new(
            spec.rule,
            spec.close,
            spec.close_kind,
            spec.close_text,
        ))));
    }
    fn tag(&mut self, literal: &'static str) {
        self.push(Frame::Tag(Box::new(base::ExactTag::new(
            literal,
            SyntaxKind::Text,
        ))));
    }
    fn brace_closer(&mut self, rule: RuleId) {
        self.push(Frame::Closer(Box::new(Closer::new(
            rule,
            rules::RIGHT_BRACE,
            SyntaxKind::RightBrace,
            "}",
        ))));
    }
    fn map_value(&mut self, committed: bool) {
        self.push(Frame::MapValue(committed));
        self.push(Frame::Call(rules::KIND));
    }
    fn choose(&mut self, index: usize) {
        self.push(Frame::Choice(index));
        let rule = [
            rules::KIND_ANY,
            rules::KIND_ATOM,
            rules::KIND_EMPTY,
            rules::KIND_SCALAR,
            rules::KIND_TABLE,
            rules::KIND_TUPLE,
            rules::KIND_KIND,
        ][index];
        if index < 3 {
            self.push(Frame::Primitive(leaves::Continuation::new(rule)));
        } else {
            self.push(Frame::Call(rule));
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
            let frame = self.frames.pop().expect("retained kind frame");
            if !matches!(
                frame,
                Frame::Nesting(_)
                    | Frame::Tag(_)
                    | Frame::Base(_)
                    | Frame::Primitive(_)
                    | Frame::Core(_)
                    | Frame::Required(_)
                    | Frame::Closer(_)
            ) {
                *allowance -= 1;
                self.work += 1;
            }
            match frame {
                Frame::Call(rule) => match rule {
                    rules::KIND => self.push(Frame::Kind(KindPosition::Ordinary)),
                    rules::KIND_MATRIX => self.push(Frame::Matrix(KindPosition::Ordinary)),
                    rules::KIND_ANNOTATION => self.push(Frame::Annotation(true)),
                    rules::KIND_WITH_OPTION => {
                        self.transaction(parser, rule);
                        let node = parser.start();
                        self.push(Frame::OptionChild(node));
                        self.push(Frame::Call(rules::KIND));
                    }
                    rules::KIND_SCALAR => {
                        self.transaction(parser, rule);
                        let node = parser.start();
                        self.push(Frame::ScalarStem(node));
                        self.base(rules::IDENTIFIER);
                    }
                    rules::KIND_KIND => {
                        self.transaction(parser, rule);
                        let node = parser.start();
                        self.push(Frame::Finish(node, SyntaxKind::KindKind));
                        self.push(Frame::Open(Delimited {
                            rule,
                            close: rules::RIGHT_ANGLE,
                            close_kind: SyntaxKind::RightAngle,
                            close_text: ">",
                            content: rules::KIND_WITH_OPTION,
                            code: "syntax/missing-delimited-kind",
                            message: "missing kind after opening delimiter",
                            recover: true,
                        }));
                        self.base(rules::LEFT_ANGLE);
                    }
                    rules::KIND_TUPLE => {
                        self.transaction(parser, rule);
                        let node = parser.start();
                        self.push(Frame::Finish(node, SyntaxKind::KindTuple));
                        self.push(Frame::TupleOpen(node));
                        self.base(rules::LEFT_PARENTHESIS);
                    }
                    rules::KIND_TABLE => {
                        self.transaction(parser, rule);
                        let node = parser.start();
                        self.push(Frame::TableOpen(node));
                        self.base(rules::BAR);
                    }
                    rules::KIND_SET | rules::KIND_MAP | rules::KIND_RECORD => {
                        self.transaction(parser, rule);
                        let node = parser.start();
                        self.push(Frame::DirectBrace(node, rule));
                        self.base(rules::LEFT_BRACE);
                    }
                    _ => unreachable!("owned kind rule"),
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
                Frame::Finish(node, kind) => self.finish(parser, node, kind),
                Frame::Kind(position) => {
                    self.transaction(parser, rules::KIND);
                    let node = parser.start();
                    self.push(Frame::Finish(node, SyntaxKind::Kind));
                    self.push(Frame::KindSelect(position));
                }
                Frame::KindSelect(position) => {
                    if !final_input && parser.is_eof() {
                        self.push(Frame::KindSelect(position));
                        return Progress::NeedInput;
                    }
                    if parser.cursor().starts_with("{") {
                        self.push(Frame::Brace);
                    } else if parser.cursor().starts_with("[") {
                        self.push(Frame::Matrix(position));
                    } else {
                        self.choose(0);
                    }
                }
                Frame::Choice(index) => {
                    if self.result == Attempt::NoMatch && index < 6 {
                        self.choose(index + 1);
                    }
                }
                Frame::OptionChild(node) => {
                    if self.result == Attempt::NoMatch {
                        node.abandon(parser);
                    } else {
                        self.push(Frame::OptionSuffix(node, self.result));
                        if !parser.is_halted() {
                            self.base(rules::QUESTION);
                        }
                    }
                }
                Frame::OptionSuffix(node, result) => {
                    node.complete(parser, SyntaxKind::KindWithOption);
                    self.result = result;
                }
                Frame::ScalarStem(node) => {
                    if self.result == Attempt::NoMatch {
                        node.abandon(parser);
                    } else {
                        self.push(Frame::ScalarColon(node, parser.checkpoint()));
                        self.base(rules::COLON);
                    }
                }
                Frame::ScalarColon(node, checkpoint) => {
                    if self.result == Attempt::Matched {
                        self.push(Frame::ScalarRange(node, checkpoint));
                        self.push(Frame::Core(Box::new(Core::new(rules::RANGE_EXPRESSION))));
                    } else {
                        node.complete(parser, SyntaxKind::KindScalar);
                        self.result = Attempt::Matched;
                    }
                }
                Frame::ScalarRange(node, checkpoint) => {
                    if self.result == Attempt::NoMatch {
                        parser.rewind(checkpoint);
                    }
                    node.complete(parser, SyntaxKind::KindScalar);
                    if self.result != Attempt::Committed {
                        self.result = Attempt::Matched;
                    }
                }
                Frame::Annotation(recover) => {
                    self.transaction(parser, rules::KIND_ANNOTATION);
                    let node = parser.start();
                    self.push(Frame::Finish(node, SyntaxKind::KindAnnotation));
                    self.push(Frame::Open(Delimited {
                        rule: rules::KIND_ANNOTATION,
                        close: rules::RIGHT_ANGLE,
                        close_kind: SyntaxKind::RightAngle,
                        close_text: ">",
                        content: rules::KIND_WITH_OPTION,
                        code: "syntax/missing-kind-annotation-kind",
                        message: "missing kind inside annotation",
                        recover,
                    }));
                    self.base(rules::LEFT_ANGLE);
                }
                Frame::Open(spec) => {
                    if self.result == Attempt::Matched {
                        if parser.push_nesting() {
                            self.push(Frame::PopNesting);
                            self.push(Frame::DelimitedChild(spec));
                            self.push(Frame::Call(spec.content));
                        } else {
                            self.push(Frame::Nesting(Box::new(NestingContinuation::new())));
                        }
                    }
                }
                Frame::PopNesting => parser.pop_nesting(),
                Frame::DelimitedChild(spec) => {
                    if !spec.recover && self.result != Attempt::Matched {
                        self.result = Attempt::NoMatch;
                    } else if self.result == Attempt::NoMatch {
                        self.push(Frame::DelimitedRecovered(spec));
                        self.recover(spec);
                    } else {
                        self.push(Frame::DelimitedClose(
                            spec,
                            self.result == Attempt::Committed,
                        ));
                        self.base(spec.close);
                    }
                }
                Frame::DelimitedRecovered(spec) => {
                    self.push(Frame::DelimitedClose(spec, true));
                    self.base(spec.close);
                }
                Frame::DelimitedClose(spec, committed) => {
                    if self.result == Attempt::Matched {
                        self.result = if committed {
                            Attempt::Committed
                        } else {
                            Attempt::Matched
                        };
                    } else if !spec.recover {
                        self.result = Attempt::NoMatch;
                    } else {
                        self.closer(spec);
                    }
                }
                Frame::Matrix(position) => {
                    self.transaction(parser, rules::KIND_MATRIX);
                    let node = parser.start();
                    self.push(Frame::MatrixOpen(node, position));
                    self.base(rules::LEFT_BRACKET);
                }
                Frame::MatrixOpen(node, position) => {
                    if self.result == Attempt::NoMatch {
                        node.abandon(parser);
                    } else if parser.push_nesting() {
                        self.push(Frame::MatrixInterior(node, position));
                        self.push(Frame::PopNesting);
                        self.push(Frame::DelimitedChild(Delimited {
                            rule: rules::KIND_MATRIX,
                            close: rules::RIGHT_BRACKET,
                            close_kind: SyntaxKind::RightBracket,
                            close_text: "]",
                            content: rules::KIND_WITH_OPTION,
                            code: "syntax/missing-kind-matrix-element",
                            message: "missing matrix element kind",
                            recover: true,
                        }));
                        self.push(Frame::Call(rules::KIND_WITH_OPTION));
                    } else {
                        self.push(Frame::Finish(node, SyntaxKind::KindMatrix));
                        self.push(Frame::Nesting(Box::new(NestingContinuation::new())));
                    }
                }
                Frame::MatrixInterior(node, position) => {
                    if self.result == Attempt::NoMatch || parser.is_halted() {
                        self.finish(parser, node, SyntaxKind::KindMatrix);
                    } else {
                        self.push(Frame::MatrixColon(
                            node,
                            position,
                            self.result,
                            parser.checkpoint(),
                        ));
                        self.base(rules::COLON);
                    }
                }
                Frame::MatrixColon(node, position, interior, checkpoint) => {
                    self.push(Frame::MatrixLiteral(
                        node,
                        position,
                        interior,
                        checkpoint,
                        self.result == Attempt::Matched,
                    ));
                    self.push(Frame::Core(Box::new(Core::new(rules::LITERAL))));
                }
                Frame::MatrixLiteral(node, position, interior, checkpoint, colon) => {
                    match self.result {
                        Attempt::Matched => self.push(Frame::MatrixLoop(node, interior)),
                        Attempt::Committed => {
                            node.complete(parser, SyntaxKind::KindMatrix);
                        }
                        Attempt::NoMatch => {
                            if !final_input && parser.is_eof() {
                                self.push(Frame::MatrixLiteral(
                                    node, position, interior, checkpoint, colon,
                                ));
                                return Progress::NeedInput;
                            }
                            let map_separator = !parser.cursor().starts_with(":")
                                && match position {
                                    KindPosition::Ordinary => false,
                                    KindPosition::MapKey => true,
                                    KindPosition::BraceElement => {
                                        interior == Attempt::Committed
                                            || !parser.cursor().starts_with("}")
                                    }
                                };
                            if colon && map_separator {
                                parser.rewind(checkpoint);
                            }
                            node.complete(parser, SyntaxKind::KindMatrix);
                            self.result = interior;
                        }
                    }
                }
                Frame::MatrixLoop(node, interior) => {
                    self.push(Frame::MatrixSeparator(node, interior, parser.checkpoint()));
                    self.base(rules::LIST_SEPARATOR);
                }
                Frame::MatrixSeparator(node, interior, pair) => {
                    if self.result == Attempt::Matched {
                        self.push(Frame::MatrixNextLiteral(node, interior, pair));
                        self.push(Frame::Core(Box::new(Core::new(rules::LITERAL))));
                    } else {
                        node.complete(parser, SyntaxKind::KindMatrix);
                        self.result = interior;
                    }
                }
                Frame::MatrixNextLiteral(node, interior, pair) => match self.result {
                    Attempt::Matched => self.push(Frame::MatrixLoop(node, interior)),
                    Attempt::NoMatch => {
                        parser.rewind(pair);
                        node.complete(parser, SyntaxKind::KindMatrix);
                        self.result = interior;
                    }
                    Attempt::Committed => {
                        node.complete(parser, SyntaxKind::KindMatrix);
                    }
                },
                Frame::TupleOpen(node) => {
                    if self.result == Attempt::Matched {
                        if parser.push_nesting() {
                            self.push(Frame::PopNesting);
                            self.push(Frame::TupleItem(node, false, false));
                            self.push(Frame::Call(rules::KIND));
                        } else {
                            self.push(Frame::Nesting(Box::new(NestingContinuation::new())));
                        }
                    }
                }
                Frame::TupleItem(node, mut committed, after_separator) => {
                    if self.result == Attempt::NoMatch {
                        self.push(Frame::TupleRecovered(node));
                        self.push(Frame::Required(Box::new(Required::new(
                            rules::KIND_TUPLE,
                            "syntax/missing-kind-tuple-item",
                            if after_separator {
                                "missing tuple kind item after separator"
                            } else {
                                "missing tuple kind item"
                            },
                            "kind",
                            &[],
                            &[],
                            None,
                        ))));
                    } else {
                        committed |= self.result == Attempt::Committed;
                        self.push(Frame::TupleLoop(node, committed));
                    }
                }
                Frame::TupleRecovered(node) => self.push(Frame::TupleLoop(node, true)),
                Frame::TupleLoop(node, committed) => {
                    if parser.is_halted() {
                        self.push(Frame::TupleClose(committed));
                        self.base(rules::RIGHT_PARENTHESIS);
                    } else {
                        self.push(Frame::TupleSeparator(node, committed));
                        self.base(rules::LIST_SEPARATOR);
                    }
                }
                Frame::TupleSeparator(node, committed) => {
                    if self.result == Attempt::Matched {
                        self.push(Frame::TupleItem(node, committed, true));
                        self.push(Frame::Call(rules::KIND));
                    } else {
                        self.push(Frame::TupleClose(committed));
                        self.base(rules::RIGHT_PARENTHESIS);
                    }
                }
                Frame::TupleClose(committed) => {
                    if self.result == Attempt::Matched {
                        self.result = if committed {
                            Attempt::Committed
                        } else {
                            Attempt::Matched
                        };
                    } else {
                        self.push(Frame::Closer(Box::new(Closer::new(
                            rules::KIND_TUPLE,
                            rules::RIGHT_PARENTHESIS,
                            SyntaxKind::RightParen,
                            ")",
                        ))));
                    }
                }
                Frame::TableOpen(node) => {
                    if self.result == Attempt::NoMatch {
                        node.abandon(parser);
                    } else {
                        self.push(Frame::TableField(node, false, false));
                        self.push(Frame::Field(true));
                    }
                }
                Frame::Field(optional_annotation) => {
                    self.push(Frame::FieldStem(optional_annotation));
                    self.base(rules::IDENTIFIER);
                }
                Frame::FieldStem(optional_annotation) => {
                    if self.result == Attempt::Matched {
                        self.push(Frame::FieldAnnotation(optional_annotation));
                        self.push(Frame::Annotation(true));
                    }
                }
                Frame::FieldAnnotation(optional_annotation) => {
                    if optional_annotation && self.result == Attempt::NoMatch {
                        self.result = Attempt::Matched;
                    }
                }
                Frame::TableField(node, mut committed, after_separator) => {
                    if self.result == Attempt::NoMatch {
                        self.push(Frame::TableFieldRecovered(node));
                        self.push(Frame::Required(Box::new(Required::new(
                            rules::KIND_TABLE,
                            "syntax/missing-kind-table-field",
                            if after_separator {
                                "missing table kind field after separator"
                            } else {
                                "missing table kind field"
                            },
                            "kind-table-field",
                            &[],
                            &[],
                            None,
                        ))));
                    } else {
                        committed |= self.result == Attempt::Committed;
                        self.push(Frame::TableLoop(node, committed));
                    }
                }
                Frame::TableFieldRecovered(node) => self.push(Frame::TableLoop(node, true)),
                Frame::TableLoop(node, committed) => {
                    if parser.is_halted() {
                        self.push(Frame::TableClose(node, committed));
                        self.base(rules::BAR);
                    } else {
                        self.push(Frame::TableSeparator(node, committed));
                        self.base(rules::LIST_SEPARATOR);
                    }
                }
                Frame::TableSeparator(node, committed) => {
                    self.push(Frame::TableSeparatorSpace(node, committed));
                    if self.result == Attempt::NoMatch {
                        self.base(rules::SPACE_TAB1);
                    }
                }
                Frame::TableSeparatorSpace(node, committed) => {
                    if self.result == Attempt::Matched {
                        self.push(Frame::TableField(node, committed, true));
                        self.push(Frame::Field(true));
                    } else {
                        self.push(Frame::TableClose(node, committed));
                        self.base(rules::BAR);
                    }
                }
                Frame::TableClose(node, committed) => {
                    if self.result == Attempt::NoMatch {
                        self.push(Frame::Finish(node, SyntaxKind::TableKind));
                        self.push(Frame::Closer(Box::new(Closer::new(
                            rules::KIND_TABLE,
                            rules::BAR,
                            SyntaxKind::Bar,
                            "|",
                        ))));
                    } else {
                        self.push(Frame::TableColon(node, committed, parser.checkpoint()));
                        self.base(rules::COLON);
                    }
                }
                Frame::TableColon(node, committed, checkpoint) => {
                    if self.result == Attempt::Matched {
                        self.push(Frame::TableSuffix(node, committed, checkpoint));
                        self.push(Frame::Core(Box::new(Core::new(rules::LITERAL))));
                    } else {
                        node.complete(parser, SyntaxKind::TableKind);
                        self.result = if committed {
                            Attempt::Committed
                        } else {
                            Attempt::Matched
                        };
                    }
                }
                Frame::TableSuffix(node, committed, checkpoint) => {
                    if self.result == Attempt::NoMatch {
                        parser.rewind(checkpoint);
                    }
                    node.complete(parser, SyntaxKind::TableKind);
                    self.result = if committed || self.result == Attempt::Committed {
                        Attempt::Committed
                    } else {
                        Attempt::Matched
                    };
                }
                Frame::DirectBrace(node, rule) => {
                    if self.result == Attempt::NoMatch {
                        node.abandon(parser);
                    } else {
                        let kind = if rule == rules::KIND_SET {
                            SyntaxKind::KindSet
                        } else if rule == rules::KIND_MAP {
                            SyntaxKind::KindMap
                        } else {
                            SyntaxKind::KindRecord
                        };
                        if parser.push_nesting() {
                            if rule == rules::KIND_SET {
                                self.push(Frame::SetInterior(node));
                            } else {
                                self.push(Frame::Finish(node, kind));
                            }
                            self.push(Frame::PopNesting);
                            if rule == rules::KIND_SET {
                                self.push(Frame::SetChild);
                                self.push(Frame::Call(rules::KIND));
                            } else if rule == rules::KIND_MAP {
                                self.push(Frame::MapKey);
                                self.push(Frame::Kind(KindPosition::MapKey));
                            } else {
                                self.push(Frame::RecordStart);
                                self.base(rules::WHITESPACE0);
                            }
                        } else {
                            self.push(Frame::Finish(node, kind));
                            self.push(Frame::Nesting(Box::new(NestingContinuation::new())));
                        }
                    }
                }
                Frame::SetChild => match self.result {
                    Attempt::NoMatch => {}
                    Attempt::Committed => self.brace_closer(rules::KIND_SET),
                    Attempt::Matched => {
                        self.push(Frame::SetClose);
                        self.base(rules::RIGHT_BRACE);
                    }
                },
                Frame::SetClose => {
                    if self.result == Attempt::NoMatch {
                        self.brace_closer(rules::KIND_SET);
                    }
                }
                Frame::SetInterior(node) => {
                    if self.result == Attempt::NoMatch || parser.is_halted() {
                        self.finish(parser, node, SyntaxKind::KindSet);
                    } else {
                        self.push(Frame::SetFinish(node, self.result == Attempt::Committed));
                        self.push(Frame::SetSuffix);
                    }
                }
                Frame::SetFinish(node, committed) => {
                    node.complete(parser, SyntaxKind::KindSet);
                    self.result = if committed || self.result == Attempt::Committed {
                        Attempt::Committed
                    } else {
                        Attempt::Matched
                    };
                }
                Frame::MapKey => {
                    if self.result != Attempt::NoMatch
                        && !(self.result == Attempt::Committed && parser.is_halted())
                    {
                        self.push(Frame::MapColon(self.result == Attempt::Committed));
                        self.base(rules::COLON);
                    }
                }
                Frame::MapColon(committed) => {
                    if self.result == Attempt::Matched {
                        self.map_value(committed);
                    }
                }
                Frame::MapValue(mut committed) => {
                    if self.result == Attempt::NoMatch {
                        self.push(Frame::MapRecovered);
                        self.push(Frame::Required(Box::new(Required::new(
                            rules::KIND_MAP,
                            "syntax/missing-kind-map-value",
                            "missing map value kind after colon",
                            "kind",
                            &[],
                            &[],
                            None,
                        ))));
                    } else {
                        committed |= self.result == Attempt::Committed;
                        self.push(Frame::MapClose(committed));
                        self.base(rules::RIGHT_BRACE);
                    }
                }
                Frame::MapRecovered => {
                    self.push(Frame::MapClose(true));
                    self.base(rules::RIGHT_BRACE);
                }
                Frame::MapClose(committed) => {
                    if self.result == Attempt::NoMatch {
                        self.brace_closer(rules::KIND_MAP);
                    } else if committed {
                        self.result = Attempt::Committed;
                    }
                }
                Frame::SetSuffix => {
                    self.push(Frame::SetSuffixColon(parser.checkpoint()));
                    self.base(rules::COLON);
                }
                Frame::SetSuffixColon(checkpoint) => {
                    if self.result == Attempt::Matched {
                        self.push(Frame::SetSuffixLiteral(checkpoint));
                        self.push(Frame::Core(Box::new(Core::new(rules::LITERAL))));
                    } else {
                        self.push(Frame::Matched);
                        self.tag(":N");
                    }
                }
                Frame::SetSuffixLiteral(checkpoint) => {
                    if self.result != Attempt::Committed {
                        if self.result == Attempt::NoMatch {
                            parser.rewind(checkpoint);
                        }
                        self.push(Frame::Matched);
                        self.tag(":N");
                    }
                }
                Frame::Matched => self.result = Attempt::Matched,
                Frame::RecordStart => {
                    if self.result == Attempt::Matched {
                        self.push(Frame::RecordFirst);
                        self.push(Frame::Field(false));
                    }
                }
                Frame::RecordFirst => {
                    if self.result != Attempt::NoMatch {
                        self.push(Frame::RecordLoop(self.result == Attempt::Committed));
                    }
                }
                Frame::RecordLoop(committed) => {
                    if parser.is_halted() {
                        self.push(Frame::RecordSuffix(committed));
                        self.tag(",…");
                    } else {
                        self.push(Frame::RecordSeparator(committed, parser.checkpoint()));
                        self.base(rules::LIST_SEPARATOR);
                    }
                }
                Frame::RecordSeparator(committed, checkpoint) => {
                    self.push(Frame::RecordSeparatorSpace(committed, checkpoint));
                    if self.result == Attempt::NoMatch {
                        self.base(rules::WHITESPACE1);
                    }
                }
                Frame::RecordSeparatorSpace(committed, checkpoint) => {
                    if self.result == Attempt::Matched {
                        self.push(Frame::RecordNext(committed, checkpoint));
                        self.push(Frame::Field(false));
                    } else {
                        self.push(Frame::RecordSuffix(committed));
                        self.tag(",…");
                    }
                }
                Frame::RecordNext(committed, checkpoint) => {
                    if self.result == Attempt::NoMatch {
                        parser.rewind(checkpoint);
                        self.push(Frame::RecordSuffix(committed));
                        self.tag(",…");
                    } else {
                        self.push(Frame::RecordLoop(
                            committed || self.result == Attempt::Committed,
                        ));
                    }
                }
                Frame::RecordSuffix(committed) => {
                    self.push(Frame::RecordSpace(committed));
                    self.base(rules::WHITESPACE0);
                }
                Frame::RecordSpace(committed) => {
                    self.push(Frame::RecordClose(committed));
                    self.base(rules::RIGHT_BRACE);
                }
                Frame::RecordClose(committed) => {
                    if self.result == Attempt::NoMatch {
                        self.brace_closer(rules::KIND_RECORD);
                    } else {
                        self.result = if committed {
                            Attempt::Committed
                        } else {
                            Attempt::Matched
                        };
                    }
                }
                Frame::Brace => {
                    let brace = Brace {
                        checkpoint: parser.checkpoint(),
                        map: parser.start(),
                        set: parser.start(),
                        record: parser.start(),
                    };
                    self.push(Frame::BraceExit(brace.checkpoint));
                    self.push(Frame::BraceOpen(brace));
                    self.base(rules::LEFT_BRACE);
                }
                Frame::BraceExit(checkpoint) => {
                    if self.result == Attempt::NoMatch {
                        parser.rewind(checkpoint);
                    }
                }
                Frame::BraceOpen(brace) => {
                    if self.result == Attempt::Matched {
                        if parser.push_nesting() {
                            self.push(Frame::PopNesting);
                            self.push(Frame::BraceWhitespace(brace, parser.checkpoint()));
                            self.base(rules::WHITESPACE1);
                        } else {
                            self.push(Frame::BraceLimited(brace));
                            self.push(Frame::Nesting(Box::new(NestingContinuation::new())));
                        }
                    }
                }
                Frame::BraceLimited(brace) => {
                    brace.record.complete(parser, SyntaxKind::KindRecord);
                    brace.set.complete(parser, SyntaxKind::KindSet);
                    brace.map.complete(parser, SyntaxKind::KindMap);
                    self.result = Attempt::Committed;
                }
                Frame::BraceWhitespace(brace, after_open) => {
                    let forced = self.result == Attempt::Matched;
                    if !forced {
                        parser.rewind(after_open);
                    }
                    self.push(Frame::BraceRecord(brace, after_open, forced));
                    self.push(Frame::Field(false));
                }
                Frame::BraceRecord(brace, after_open, forced) => {
                    if self.result != Attempt::NoMatch {
                        self.push(Frame::BraceRecordFinish(brace));
                        self.push(Frame::RecordLoop(self.result == Attempt::Committed));
                    } else if !forced {
                        parser.rewind(after_open);
                        finish_provisional_marker(parser, brace.record, SyntaxKind::KindRecord);
                        self.push(Frame::BraceScalar(brace, parser.checkpoint()));
                        self.push(Frame::PlainKind);
                    }
                }
                Frame::BraceRecordFinish(brace) => {
                    if self.result != Attempt::NoMatch {
                        brace.record.complete(parser, SyntaxKind::KindRecord);
                        finish_provisional_marker(parser, brace.set, SyntaxKind::KindSet);
                        finish_provisional_marker(parser, brace.map, SyntaxKind::KindMap);
                    }
                }
                Frame::PlainKind => {
                    self.transaction(parser, rules::KIND);
                    let kind = parser.start();
                    self.push(Frame::PlainFinish(kind, SyntaxKind::Kind));
                    self.transaction(parser, rules::KIND_SCALAR);
                    let scalar = parser.start();
                    self.push(Frame::PlainFinish(scalar, SyntaxKind::KindScalar));
                    self.base(rules::IDENTIFIER);
                }
                Frame::PlainFinish(node, kind) => {
                    if self.result == Attempt::NoMatch {
                        node.abandon(parser);
                    } else {
                        node.complete(parser, kind);
                        self.result = Attempt::Matched;
                    }
                }
                Frame::BraceScalar(brace, checkpoint) => {
                    if self.result == Attempt::Matched {
                        self.push(Frame::BraceScalarColon(brace, checkpoint));
                        self.base(rules::COLON);
                    } else {
                        parser.rewind(checkpoint);
                        self.push(Frame::BraceElement(brace));
                        self.push(Frame::Kind(KindPosition::BraceElement));
                    }
                }
                Frame::BraceScalarColon(brace, checkpoint) => {
                    if self.result == Attempt::Matched {
                        self.push(Frame::BraceScalarValue(brace));
                        self.push(Frame::Call(rules::KIND));
                    } else {
                        parser.rewind(checkpoint);
                        self.push(Frame::BraceElement(brace));
                        self.push(Frame::Kind(KindPosition::BraceElement));
                    }
                }
                Frame::BraceScalarValue(brace) => {
                    self.push(Frame::BraceScalarFinish(brace));
                    match self.result {
                        Attempt::Matched => {
                            self.push(Frame::BraceScalarClose);
                            self.base(rules::RIGHT_BRACE);
                        }
                        Attempt::Committed => self.brace_closer(rules::KIND_MAP),
                        Attempt::NoMatch => {
                            self.push(Frame::BraceScalarRecover);
                            self.push(Frame::Required(Box::new(Required::new(
                                rules::KIND_MAP,
                                "syntax/missing-kind-map-value",
                                "missing map value kind after colon",
                                "kind",
                                &[],
                                &[],
                                None,
                            ))));
                        }
                    }
                }
                Frame::BraceScalarClose => {
                    if self.result == Attempt::NoMatch {
                        self.brace_closer(rules::KIND_MAP);
                    }
                }
                Frame::BraceScalarRecover => self.brace_closer(rules::KIND_MAP),
                Frame::BraceScalarFinish(brace) => {
                    finish_provisional_marker(parser, brace.set, SyntaxKind::KindSet);
                    brace.map.complete(parser, SyntaxKind::KindMap);
                }
                Frame::BraceElement(brace) => {
                    if self.result != Attempt::NoMatch {
                        let committed = self.result == Attempt::Committed;
                        self.push(Frame::BraceElementColon(brace, committed));
                        if !parser.is_halted() {
                            self.base(rules::COLON);
                        } else {
                            self.result = Attempt::NoMatch;
                        }
                    }
                }
                Frame::BraceElementColon(brace, committed) => {
                    if self.result == Attempt::Matched {
                        finish_provisional_marker(parser, brace.set, SyntaxKind::KindSet);
                        self.push(Frame::BraceMapFinish(brace));
                        self.map_value(committed);
                    } else if committed {
                        self.push(Frame::BraceSetRecovered(brace));
                        self.brace_closer(rules::KIND_SET);
                    } else {
                        self.push(Frame::BraceSetClose(brace));
                        self.base(rules::RIGHT_BRACE);
                    }
                }
                Frame::BraceMapFinish(brace) => {
                    brace.map.complete(parser, SyntaxKind::KindMap);
                }
                Frame::BraceSetRecovered(brace) => {
                    self.push(Frame::BraceSetFinish(brace, true));
                    self.push(Frame::SetSuffix);
                }
                Frame::BraceSetClose(brace) => {
                    if self.result == Attempt::Matched {
                        self.push(Frame::BraceSetFinish(brace, false));
                        self.push(Frame::SetSuffix);
                    }
                }
                Frame::BraceSetFinish(brace, committed) => {
                    if committed || self.result != Attempt::NoMatch {
                        brace.set.complete(parser, SyntaxKind::KindSet);
                        finish_provisional_marker(parser, brace.map, SyntaxKind::KindMap);
                        if committed {
                            self.result = Attempt::Committed;
                        }
                    }
                }
                Frame::Tag(mut child) => {
                    let before = *allowance;
                    let progress = child.advance(parser, final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        base::continuation::Progress::Complete(result) => {
                            self.result = if result {
                                Attempt::Matched
                            } else {
                                Attempt::NoMatch
                            }
                        }
                        base::continuation::Progress::NeedInput => {
                            self.push(Frame::Tag(child));
                            return Progress::NeedInput;
                        }
                        base::continuation::Progress::NeedsProcessing => {
                            self.push(Frame::Tag(child));
                            return Progress::NeedsProcessing;
                        }
                        base::continuation::Progress::Limited => {
                            self.push(Frame::Tag(child));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::Nesting(mut child) => {
                    let before = *allowance;
                    let progress = child.advance(parser, final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        NestingProgress::Complete => self.result = Attempt::Committed,
                        NestingProgress::NeedInput => {
                            self.push(Frame::Nesting(child));
                            return Progress::NeedInput;
                        }
                        NestingProgress::NeedsProcessing => {
                            self.push(Frame::Nesting(child));
                            return Progress::NeedsProcessing;
                        }
                        NestingProgress::Limited => {
                            self.push(Frame::Nesting(child));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::Base(mut child) => {
                    let before = *allowance;
                    let progress = child.advance(parser, final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        base::continuation::Progress::Complete(result) => {
                            self.result = if result {
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
                Frame::Primitive(mut child) => {
                    let before = *allowance;
                    let progress = child.advance(parser, final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        leaves::Progress::Complete(result) => self.result = result,
                        leaves::Progress::NeedInput => {
                            self.push(Frame::Primitive(child));
                            return Progress::NeedInput;
                        }
                        leaves::Progress::NeedsProcessing => {
                            self.push(Frame::Primitive(child));
                            return Progress::NeedsProcessing;
                        }
                        leaves::Progress::Limited => {
                            self.push(Frame::Primitive(child));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::Core(mut child) => {
                    let before = *allowance;
                    let progress = child.advance(parser, final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        super::super::precedence::Progress::Complete(result) => {
                            self.result = result
                        }
                        super::super::precedence::Progress::NeedInput => {
                            self.push(Frame::Core(child));
                            return Progress::NeedInput;
                        }
                        super::super::precedence::Progress::NeedsProcessing => {
                            self.push(Frame::Core(child));
                            return Progress::NeedsProcessing;
                        }
                        super::super::precedence::Progress::Limited => {
                            self.push(Frame::Core(child));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::Required(mut child) => {
                    let before = *allowance;
                    let progress = child.advance(parser, final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        super::super::precedence::Progress::Complete(result) => {
                            self.result = result
                        }
                        super::super::precedence::Progress::NeedInput => {
                            self.push(Frame::Required(child));
                            return Progress::NeedInput;
                        }
                        super::super::precedence::Progress::NeedsProcessing => {
                            self.push(Frame::Required(child));
                            return Progress::NeedsProcessing;
                        }
                        super::super::precedence::Progress::Limited => {
                            self.push(Frame::Required(child));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::Closer(mut child) => {
                    let before = *allowance;
                    let progress = child.advance(parser, final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        super::super::precedence::Progress::Complete(result) => {
                            self.result = result
                        }
                        super::super::precedence::Progress::NeedInput => {
                            self.push(Frame::Closer(child));
                            return Progress::NeedInput;
                        }
                        super::super::precedence::Progress::NeedsProcessing => {
                            self.push(Frame::Closer(child));
                            return Progress::NeedsProcessing;
                        }
                        super::super::precedence::Progress::Limited => {
                            self.push(Frame::Closer(child));
                            return Progress::Limited;
                        }
                    }
                }
            }
        }
        Progress::Complete(self.result)
    }
}
