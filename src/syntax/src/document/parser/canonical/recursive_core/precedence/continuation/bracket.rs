//! One bracket owner retains matrix/comprehension selection and subsequent rows.
use super::super::super::structures::BracketMode;
use super::super::super::{BracketForm, finish_provisional_marker};
use super::*;
#[derive(Clone, Copy)]
pub(super) struct Owner {
    matrix: Marker,
    comprehension: Marker,
    mode: BracketMode,
    ordinary: bool,
}
#[derive(Clone, Copy)]
pub(super) struct Head {
    owner: Owner,
    row: Marker,
    column: Marker,
    committed: bool,
    after: ParserCheckpoint,
}
pub(super) enum Phase {
    Enter(BracketMode),
    Exit(ParserCheckpoint),
    Open(Owner),
    Limited(Owner),
    Start(Owner),
    Leading(Owner, ParserCheckpoint),
    Separator(Owner, ParserCheckpoint),
    Space(Owner, ParserCheckpoint, bool),
    EmptyProbe(Owner, ParserCheckpoint, ParserCheckpoint),
    EmptyClose(Owner),
    Head(Owner, ParserCheckpoint, Marker, Marker, bool),
    BarSpace(Head),
    Bar(Head),
    Qualifiers(Head),
    MatrixHead(Head),
    ColumnTail(Head),
    RowTail(Owner, bool),
    Loop(Owner, bool),
    Decoration(Owner, bool),
    EndProbe(Owner, bool, ParserCheckpoint),
    Row(Owner, bool, TextSize),
    Trailing(Owner, bool),
    Close(Owner, bool),
    Recovered(Owner),
    Project,
    Projection(Marker, ParserCheckpoint),
}
impl Continuation {
    fn bracket(&mut self, phase: Phase) {
        self.push(Frame::Bracket(Box::new(phase)));
    }
    fn bracket_result(&mut self, result: FactAttempt<BracketForm>) {
        self.bracket_facts = result;
        self.result = result.attempt();
    }
    fn bracket_head(
        &mut self,
        parser: &mut Parser<'_>,
        owner: Owner,
        after_open: ParserCheckpoint,
        leading: bool,
    ) {
        let row = parser.start();
        let column = parser.start();
        self.bracket(Phase::Head(owner, after_open, row, column, leading));
        self.push(Frame::Call(rules::EXPRESSION));
    }
    fn bracket_failed_head(
        &mut self,
        parser: &mut Parser<'_>,
        owner: Owner,
        after_open: ParserCheckpoint,
    ) {
        parser.rewind(after_open);
        if owner.mode == BracketMode::ComprehensionOnly {
            self.bracket_result(FactAttempt::NoMatch);
        } else {
            self.bracket(Phase::Loop(owner, false));
        }
    }
    fn bracket_commit(&mut self, parser: &mut Parser<'_>, owner: Owner) {
        finish_provisional_marker(parser, owner.comprehension, SyntaxKind::MatrixComprehension);
        owner.matrix.complete(parser, SyntaxKind::Matrix);
        self.bracket_result(FactAttempt::Committed);
    }
    fn bracket_recover(&mut self, owner: Owner) {
        let (kind, text) = if owner.ordinary {
            (SyntaxKind::RightBracket, "]")
        } else {
            (SyntaxKind::BoxDrawing, "╯")
        };
        self.bracket(Phase::Recovered(owner));
        self.push(Frame::Closer(Box::new(Closer::set(
            rules::MATRIX,
            rules::MATRIX_END,
            kind,
            text,
            &[']', '╯', '┘', '┛', ')', '}'],
        ))));
    }
    fn bracket_shell(&mut self, rule: RuleId) {
        self.push(Frame::Shell(Box::new(
            super::super::super::super::structure_shell::Continuation::new(rule),
        )));
    }
    #[inline(never)]
    pub(super) fn bracket_frame(
        &mut self,
        parser: &mut Parser<'_>,
        phase: Box<Phase>,
        final_input: bool,
    ) -> Option<Progress> {
        match *phase {
            Phase::Enter(mode) => {
                if !final_input && parser.is_eof() {
                    self.bracket(Phase::Enter(mode));
                    return Some(Progress::NeedInput);
                }
                let checkpoint = parser.checkpoint();
                parser
                    .state
                    .rules
                    .push_canonical(if mode == BracketMode::ComprehensionOnly {
                        rules::MATRIX_COMPREHENSION
                    } else {
                        rules::MATRIX
                    });
                let owner = Owner {
                    matrix: parser.start(),
                    comprehension: parser.start(),
                    mode,
                    ordinary: parser.cursor().starts_with("["),
                };
                self.bracket(Phase::Exit(checkpoint));
                self.bracket(Phase::Open(owner));
                self.bracket_shell(rules::MATRIX_START);
            }
            Phase::Exit(checkpoint) => {
                parser.state.rules.truncate(checkpoint.rule_depth);
                if self.bracket_facts == FactAttempt::NoMatch {
                    parser.rewind(checkpoint);
                }
                if parser.is_halted() {
                    self.bracket_result(match self.bracket_facts {
                        FactAttempt::Matched(f) | FactAttempt::Recovered(f) => {
                            FactAttempt::Recovered(f)
                        }
                        _ => FactAttempt::Committed,
                    });
                } else {
                    self.result = self.bracket_facts.attempt();
                }
            }
            Phase::Open(owner) => {
                if self.result != Attempt::Matched {
                    if !parser.state.resource_finalizing {
                        owner.comprehension.abandon(parser);
                        owner.matrix.abandon(parser);
                    }
                    self.bracket_result(FactAttempt::NoMatch);
                } else if parser.push_nesting() {
                    self.push(Frame::PopNesting);
                    self.bracket(Phase::Start(owner));
                } else {
                    self.bracket(Phase::Limited(owner));
                    self.push(Frame::Nesting(Box::new(NestingContinuation::new())));
                }
            }
            Phase::Limited(owner) => self.bracket_commit(parser, owner),
            Phase::Start(owner) => {
                let after = parser.checkpoint();
                if owner.ordinary {
                    self.bracket(Phase::Leading(owner, after));
                    self.base(rules::SPACE_TAB0);
                } else {
                    self.bracket_failed_head(parser, owner, after);
                }
            }
            Phase::Leading(owner, after) => {
                if self.result == Attempt::Matched {
                    self.bracket(Phase::Separator(owner, after));
                    self.table_separator();
                } else {
                    self.bracket_failed_head(parser, owner, after);
                }
            }
            Phase::Separator(owner, after) => {
                self.bracket(Phase::Space(owner, after, self.result == Attempt::Matched));
                self.base(rules::SPACE_TAB0);
            }
            Phase::Space(owner, after, leading) => {
                if self.result != Attempt::Matched {
                    self.bracket_result(FactAttempt::NoMatch);
                } else if !leading {
                    self.bracket(Phase::EmptyProbe(owner, after, parser.checkpoint()));
                    self.bracket_shell(rules::MATRIX_END);
                } else {
                    self.bracket_head(parser, owner, after, leading);
                }
            }
            Phase::EmptyProbe(owner, after, checkpoint) => {
                let empty = self.result.accepted();
                parser.rewind(checkpoint);
                if empty {
                    if owner.mode == BracketMode::ComprehensionOnly {
                        self.bracket_result(FactAttempt::NoMatch);
                    } else {
                        self.bracket(Phase::EmptyClose(owner));
                        self.bracket_shell(rules::MATRIX_END);
                    }
                } else {
                    self.bracket_head(parser, owner, after, false);
                }
            }
            Phase::EmptyClose(owner) => {
                if self.result == Attempt::Matched {
                    owner.comprehension.abandon(parser);
                    owner.matrix.complete(parser, SyntaxKind::Matrix);
                    self.bracket_result(FactAttempt::Matched(BracketForm::Matrix));
                } else {
                    self.bracket_result(FactAttempt::NoMatch);
                }
            }
            Phase::Head(owner, after_open, row, column, leading) => {
                if self.result == Attempt::NoMatch {
                    self.bracket_failed_head(parser, owner, after_open);
                } else if parser.is_halted() {
                    column.complete(parser, SyntaxKind::MatrixColumn);
                    row.complete(parser, SyntaxKind::MatrixRow);
                    self.bracket(Phase::Loop(owner, true));
                } else {
                    let head = Head {
                        owner,
                        row,
                        column,
                        committed: self.result == Attempt::Committed,
                        after: parser.checkpoint(),
                    };
                    if leading {
                        self.bracket(Phase::MatrixHead(head));
                    } else {
                        self.bracket(Phase::BarSpace(head));
                        self.base(rules::SPACE_TAB0);
                    }
                }
            }
            Phase::BarSpace(head) => {
                if self.result == Attempt::Matched {
                    self.bracket(Phase::Bar(head));
                    self.base(rules::BAR);
                } else {
                    self.bracket(Phase::MatrixHead(head));
                }
            }
            Phase::Bar(head) => {
                if self.result == Attempt::Matched {
                    if head.owner.mode == BracketMode::MatrixOnly {
                        self.bracket_result(FactAttempt::NoMatch);
                    } else {
                        self.bracket(Phase::Qualifiers(head));
                        self.comprehension_tail(rules::RIGHT_BRACKET, true);
                    }
                } else {
                    self.bracket(Phase::MatrixHead(head));
                }
            }
            Phase::Qualifiers(head) => match self.result {
                Attempt::Matched => {
                    head.column.abandon(parser);
                    head.row.abandon(parser);
                    head.owner
                        .comprehension
                        .complete(parser, SyntaxKind::MatrixComprehension);
                    head.owner.matrix.abandon(parser);
                    self.bracket_result(if head.committed {
                        FactAttempt::Recovered(BracketForm::Comprehension)
                    } else {
                        FactAttempt::Matched(BracketForm::Comprehension)
                    });
                }
                Attempt::NoMatch if head.owner.mode == BracketMode::Either => {
                    self.bracket(Phase::MatrixHead(head))
                }
                Attempt::NoMatch => self.bracket_result(FactAttempt::NoMatch),
                Attempt::Committed => {
                    finish_provisional_marker(parser, head.column, SyntaxKind::MatrixColumn);
                    finish_provisional_marker(parser, head.row, SyntaxKind::MatrixRow);
                    head.owner
                        .comprehension
                        .complete(parser, SyntaxKind::MatrixComprehension);
                    finish_provisional_marker(parser, head.owner.matrix, SyntaxKind::Matrix);
                    self.bracket_result(FactAttempt::Recovered(BracketForm::Comprehension));
                }
            },
            Phase::MatrixHead(head) => {
                parser.rewind(head.after);
                if head.owner.mode == BracketMode::ComprehensionOnly {
                    self.bracket_result(FactAttempt::NoMatch);
                } else {
                    self.bracket(Phase::ColumnTail(head));
                    self.push(Frame::MatrixRow(Box::new(matrix_row::Phase::ColumnTail)));
                }
            }
            Phase::ColumnTail(head) => {
                if self.result != Attempt::Matched {
                    self.bracket_result(FactAttempt::NoMatch);
                } else {
                    head.column.complete(parser, SyntaxKind::MatrixColumn);
                    self.bracket(Phase::RowTail(head.owner, head.committed));
                    self.push(Frame::MatrixRow(Box::new(matrix_row::Phase::RowTail(
                        head.row,
                        head.committed,
                    ))));
                }
            }
            Phase::RowTail(owner, committed) => {
                if self.result == Attempt::NoMatch {
                    self.bracket_result(FactAttempt::NoMatch);
                } else {
                    self.bracket(Phase::Loop(
                        owner,
                        committed || self.result == Attempt::Committed,
                    ));
                }
            }
            Phase::Loop(owner, committed) => {
                if parser.is_halted() {
                    self.bracket_commit(parser, owner);
                } else {
                    self.bracket(Phase::Decoration(owner, committed));
                    self.push(Frame::MatrixRow(Box::new(matrix_row::Phase::Decoration)));
                }
            }
            Phase::Decoration(owner, committed) => {
                if self.result == Attempt::Committed {
                    self.bracket_commit(parser, owner);
                } else {
                    self.bracket(Phase::EndProbe(owner, committed, parser.checkpoint()));
                    self.bracket_shell(rules::MATRIX_END);
                }
            }
            Phase::EndProbe(owner, committed, checkpoint) => {
                let end = self.result.accepted();
                parser.rewind(checkpoint);
                if end {
                    self.bracket(Phase::Trailing(owner, committed));
                    self.base(rules::WHITESPACE0);
                } else {
                    self.bracket(Phase::Row(owner, committed, parser.offset()));
                    self.push(Frame::Call(rules::MATRIX_ROW));
                }
            }
            Phase::Row(owner, mut committed, before) => {
                match self.result {
                    Attempt::Matched if parser.offset() > before => {}
                    Attempt::Matched => {
                        self.bracket_result(FactAttempt::NoMatch);
                        return None;
                    }
                    Attempt::NoMatch => {
                        self.bracket_recover(owner);
                        return None;
                    }
                    Attempt::Committed => {
                        committed = true;
                        if parser.offset() == before && !parser.is_halted() {
                            self.bracket_recover(owner);
                            return None;
                        }
                    }
                }
                self.bracket(Phase::Loop(owner, committed));
            }
            Phase::Trailing(owner, committed) => {
                self.bracket(Phase::Close(owner, committed));
                self.bracket_shell(rules::MATRIX_END);
            }
            Phase::Close(owner, committed) => {
                if self.result != Attempt::Matched {
                    self.bracket_recover(owner);
                } else {
                    owner.comprehension.abandon(parser);
                    owner.matrix.complete(parser, SyntaxKind::Matrix);
                    self.bracket_result(if committed {
                        FactAttempt::Committed
                    } else {
                        FactAttempt::Matched(BracketForm::Matrix)
                    });
                }
            }
            Phase::Recovered(owner) => self.bracket_commit(parser, owner),
            Phase::Project => {
                let checkpoint = parser.checkpoint();
                let node = parser.start();
                self.bracket(Phase::Projection(node, checkpoint));
                self.bracket(Phase::Enter(BracketMode::Either));
            }
            Phase::Projection(node, checkpoint) => {
                let result = match self.bracket_facts {
                    FactAttempt::Matched(BracketForm::Matrix) => {
                        node.complete(parser, SyntaxKind::Structure);
                        FactAttempt::Matched(ExpressionForm::Formula)
                    }
                    FactAttempt::Matched(BracketForm::Comprehension) => {
                        node.abandon(parser);
                        FactAttempt::Matched(ExpressionForm::MatrixComprehension)
                    }
                    FactAttempt::Recovered(BracketForm::Comprehension) if !parser.is_halted() => {
                        node.abandon(parser);
                        FactAttempt::Recovered(ExpressionForm::MatrixComprehension)
                    }
                    FactAttempt::Recovered(_) | FactAttempt::Committed => {
                        node.complete(parser, SyntaxKind::Structure);
                        FactAttempt::Committed
                    }
                    FactAttempt::NoMatch => {
                        parser.rewind(checkpoint);
                        FactAttempt::NoMatch
                    }
                };
                self.expression_result(result);
            }
        }
        None
    }
}
