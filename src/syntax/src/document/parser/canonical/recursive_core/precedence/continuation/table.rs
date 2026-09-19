//! Table parents retain header recovery, rows, and nested delimiter selection.
use super::*;
#[derive(Clone, Copy)]
pub(super) struct Table {
    rule: RuleId,
    kind: SyntaxKind,
    header: RuleId,
    row: RuleId,
}
impl Table {
    fn new(rule: RuleId) -> Self {
        let (kind, header, row) = match rule {
            rules::INLINE_TABLE => (
                SyntaxKind::InlineTable,
                rules::INLINE_TABLE_HEADER,
                rules::INLINE_TABLE_ROW,
            ),
            rules::REGULAR_TABLE => (
                SyntaxKind::RegularTable,
                rules::TABLE_HEADER,
                rules::TABLE_ROW,
            ),
            _ => (
                SyntaxKind::FancyTable,
                rules::FANCY_TABLE_HEADER,
                rules::TABLE_ROW2,
            ),
        };
        Self {
            rule,
            kind,
            header,
            row,
        }
    }
    fn inline(self) -> bool {
        self.rule == rules::INLINE_TABLE
    }
    fn fancy(self) -> bool {
        self.rule == rules::FANCY_TABLE
    }
}
pub(super) enum Phase {
    Enter(Marker, RuleId),
    Finish(Marker, Table),
    Header(Table),
    HeaderProbe(Table, ParserCheckpoint),
    HeaderRequired(Table),
    HeaderSeparator(Table),
    HeaderSpace(Table),
    AfterHeader(Table, bool),
    InlineSpace(Table, bool),
    Body(Table, bool),
    Row(Table),
    FancyRow,
    First(Table, bool),
    MissingFirst,
    Loop(Table, bool),
    Pair(Table, bool, ParserCheckpoint),
    Next(Table, bool, ParserCheckpoint),
    InlineProbe(Table, bool, TextSize),
    InlineNext(Table, bool, TextSize, bool, ParserCheckpoint),
}
impl Continuation {
    pub(in super::super::super) fn table_body(node: Marker, rule: RuleId) -> Self {
        let mut owner = Self::new(rule);
        owner.frames.clear();
        owner.table(Phase::Enter(node, rule));
        owner
    }
    fn table(&mut self, phase: Phase) {
        self.push(Frame::Table(Box::new(phase)));
    }
    fn table_result(&mut self, committed: bool) {
        self.result = if committed {
            Attempt::Committed
        } else {
            Attempt::Matched
        };
    }
    fn table_missing_header(&mut self, spec: Table) {
        let (code, msg, expected) = if spec.inline() {
            (
                "syntax/missing-inline-table-header",
                "missing inline table header",
                "inline-table-header",
            )
        } else {
            (
                "syntax/missing-fancy-table-header",
                "missing framed table header",
                "fancy-table-header",
            )
        };
        self.table(Phase::HeaderRequired(spec));
        self.push(Frame::Required(Box::new(Required::new(
            spec.rule,
            code,
            msg,
            expected,
            &[],
            &[],
            None,
        ))));
    }
    #[inline(never)]
    pub(super) fn table_frame(&mut self, parser: &mut Parser<'_>, phase: Box<Phase>) {
        match *phase {
            Phase::Enter(node, rule) => {
                let spec = Table::new(rule);
                self.table(Phase::Finish(node, spec));
                self.table(Phase::Header(spec));
                self.push(Frame::Call(spec.header));
            }
            Phase::Finish(node, spec) => {
                if self.result == Attempt::NoMatch {
                    if !parser.state.resource_finalizing {
                        node.abandon(parser);
                    }
                } else {
                    node.complete(parser, spec.kind);
                }
            }
            Phase::Header(spec) => match self.result {
                Attempt::Matched => self.table(Phase::AfterHeader(spec, false)),
                Attempt::NoMatch if spec.inline() => {
                    self.table(Phase::HeaderProbe(spec, parser.checkpoint()));
                    self.table_separator();
                }
                Attempt::NoMatch if spec.fancy() => self.table_missing_header(spec),
                Attempt::NoMatch => {}
                Attempt::Committed if spec.inline() => {
                    if parser.cursor().starts_with("\n") || parser.cursor().starts_with("\r") {
                        self.result = Attempt::NoMatch;
                    } else {
                        self.table(Phase::AfterHeader(spec, true));
                    }
                }
                Attempt::Committed => {
                    self.table(Phase::HeaderSpace(spec));
                    self.base(rules::WHITESPACE0);
                }
            },
            Phase::HeaderProbe(spec, checkpoint) => {
                let matched = self.result.accepted();
                parser.rewind(checkpoint);
                if matched {
                    self.table_missing_header(spec);
                } else {
                    self.result = Attempt::NoMatch;
                }
            }
            Phase::HeaderRequired(spec) => {
                self.table(Phase::HeaderSeparator(spec));
                self.recover_table_separator(spec.rule);
            }
            Phase::HeaderSeparator(spec) => {
                if spec.fancy() {
                    self.table(Phase::HeaderSpace(spec));
                    self.base(rules::WHITESPACE0);
                } else {
                    self.table(Phase::AfterHeader(spec, true));
                }
            }
            Phase::HeaderSpace(spec) => self.table(Phase::AfterHeader(spec, true)),
            Phase::AfterHeader(spec, committed) => {
                if spec.inline() {
                    if parser.is_halted() {
                        self.result = Attempt::Committed;
                    } else {
                        self.table(Phase::InlineSpace(spec, committed));
                        self.base(rules::SPACE_TAB0);
                    }
                } else {
                    self.table(Phase::Body(spec, committed));
                }
            }
            Phase::InlineSpace(spec, committed) => {
                if self.result == Attempt::Matched {
                    self.table(Phase::Body(spec, committed));
                } else {
                    self.result = Attempt::NoMatch;
                }
            }
            Phase::Body(spec, committed) => {
                if parser.push_nesting() {
                    self.push(Frame::PopNesting);
                    self.table(Phase::First(spec, committed));
                    self.table(Phase::Row(spec));
                } else {
                    self.push(Frame::Nesting(Box::new(NestingContinuation::new())));
                }
            }
            Phase::Row(spec) => {
                if spec.fancy() {
                    self.table(Phase::FancyRow);
                }
                self.push(Frame::Call(spec.row));
            }
            Phase::FancyRow => {
                if self.result == Attempt::NoMatch {
                    self.push(Frame::Shell(Box::new(
                        super::super::super::super::structure_shell::Continuation::new(
                            rules::ROW_SEPARATOR,
                        ),
                    )));
                }
            }
            Phase::First(spec, committed) => {
                if self.result == Attempt::NoMatch {
                    if spec.inline()
                        && (parser.cursor().starts_with("\n") || parser.cursor().starts_with("\r"))
                    {
                        return;
                    }
                    let (code, msg, expected) = if spec.inline() {
                        (
                            "syntax/missing-inline-table-row",
                            "missing inline table row",
                            "inline-table-row",
                        )
                    } else if spec.fancy() {
                        (
                            "syntax/missing-fancy-table-row",
                            "missing framed table row",
                            "table-row2",
                        )
                    } else {
                        (
                            "syntax/missing-regular-table-row",
                            "missing regular table row",
                            "table-row",
                        )
                    };
                    self.table(Phase::MissingFirst);
                    self.push(Frame::Required(Box::new(Required::new(
                        spec.rule,
                        code,
                        msg,
                        expected,
                        &[],
                        &[],
                        None,
                    ))));
                } else {
                    self.table(Phase::Loop(
                        spec,
                        committed || self.result == Attempt::Committed,
                    ));
                }
            }
            Phase::MissingFirst => self.result = Attempt::Committed,
            Phase::Loop(spec, committed) => {
                if !spec.fancy() && parser.is_halted() {
                    self.table_result(committed);
                } else if spec.inline() {
                    self.table(Phase::InlineProbe(spec, committed, parser.offset()));
                    self.push(Frame::Inline(Box::new(inline::Phase::SeparatorProbe)));
                } else {
                    self.table(Phase::Pair(spec, committed, parser.checkpoint()));
                    self.base(if spec.fancy() {
                        rules::NEW_LINE
                    } else {
                        rules::WHITESPACE0
                    });
                }
            }
            Phase::Pair(spec, committed, checkpoint) => {
                if self.result == Attempt::Matched {
                    self.table(Phase::Next(spec, committed, checkpoint));
                    self.table(Phase::Row(spec));
                } else {
                    self.table_result(committed);
                }
            }
            Phase::Next(spec, committed, checkpoint) => {
                if self.result == Attempt::NoMatch {
                    parser.rewind(checkpoint);
                    self.table_result(committed);
                } else {
                    self.table(Phase::Loop(
                        spec,
                        committed || self.result == Attempt::Committed,
                    ));
                }
            }
            Phase::InlineProbe(spec, committed, before) => {
                if parser.is_halted() {
                    self.result = Attempt::Committed;
                } else {
                    self.table(Phase::InlineNext(
                        spec,
                        committed,
                        before,
                        self.result.accepted(),
                        parser.checkpoint(),
                    ));
                    self.push(Frame::Call(spec.row));
                }
            }
            Phase::InlineNext(spec, mut committed, before, separator, checkpoint) => {
                match self.result {
                    Attempt::Matched if parser.offset() > before => {
                        self.table(Phase::Loop(spec, committed));
                        return;
                    }
                    Attempt::Matched | Attempt::NoMatch => {}
                    Attempt::Committed if separator && !parser.is_halted() => {
                        parser.rewind(checkpoint)
                    }
                    Attempt::Committed => {
                        committed = true;
                        if parser.offset() > before {
                            self.table(Phase::Loop(spec, committed));
                            return;
                        }
                    }
                }
                self.table_result(committed);
            }
        }
    }
}
