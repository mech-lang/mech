//! One parenthesis candidate owns tuple and parenthetical alternatives.
use super::super::super::finish_provisional_marker;
use super::*;
#[derive(Clone, Copy)]
pub(super) struct Owner {
    checkpoint: ParserCheckpoint,
    structure: Marker,
    tuple: Marker,
    parenthetical: Marker,
}
#[derive(Clone, Copy)]
pub(super) struct Body {
    owner: Owner,
    selected: bool,
    committed: bool,
}
pub(super) enum Phase {
    Enter,
    Exit(ParserCheckpoint),
    Open(Owner),
    Limited(Owner),
    EmptySpace(Owner, ParserCheckpoint),
    EmptyClose(Owner, ParserCheckpoint),
    FirstSpace(Owner, ParserCheckpoint),
    FirstBody(Owner, ParserCheckpoint, Marker),
    RetrySpace(Owner),
    RetryBody(Owner, Marker),
    FormulaSpace(Owner, Marker, ParserCheckpoint),
    FormulaClose(Owner, Marker, ParserCheckpoint),
    RecoveredProbe(Owner, ParserCheckpoint),
    StartList(Body),
    Loop(Body),
    Separator(Body),
    Item(Body),
    RecoveredItem(Body),
    Trailing(Body),
    Close(Body),
    Finish(Body),
}
impl Continuation {
    fn parenthesis(&mut self, phase: Phase) {
        self.push(Frame::Parenthesis(Box::new(phase)));
    }
    fn parenthesis_recovery_probe(&mut self, parser: &mut Parser<'_>, owner: Owner) {
        self.parenthesis(Phase::RecoveredProbe(owner, parser.checkpoint()));
        self.base(rules::WHITESPACE0);
    }
    fn parenthesis_finish(&mut self, parser: &mut Parser<'_>, body: Body) {
        let owner = body.owner;
        if body.selected {
            owner
                .parenthetical
                .complete(parser, SyntaxKind::ParentheticalExpression);
            finish_provisional_marker(parser, owner.tuple, SyntaxKind::Tuple);
            finish_provisional_marker(parser, owner.structure, SyntaxKind::Structure);
        } else {
            owner.tuple.complete(parser, SyntaxKind::Tuple);
            owner.structure.complete(parser, SyntaxKind::Structure);
        }
        self.result = if body.committed {
            Attempt::Committed
        } else {
            Attempt::Matched
        };
    }
    #[inline(never)]
    pub(super) fn parenthesis_frame(
        &mut self,
        parser: &mut Parser<'_>,
        phase: Box<Phase>,
        final_input: bool,
    ) -> Option<Progress> {
        let phase = *phase;
        match phase {
            Phase::Enter => {
                let owner = Owner {
                    checkpoint: parser.checkpoint(),
                    structure: parser.start(),
                    tuple: parser.start(),
                    parenthetical: parser.start(),
                };
                self.parenthesis(Phase::Exit(owner.checkpoint));
                self.parenthesis(Phase::Open(owner));
                self.base(rules::LEFT_PARENTHESIS);
            }
            Phase::Exit(checkpoint) => {
                if self.result == Attempt::NoMatch {
                    parser.rewind(checkpoint);
                }
            }
            Phase::Open(owner) => {
                if self.result != Attempt::Matched {
                    self.result = Attempt::NoMatch;
                } else if parser.push_nesting() {
                    self.push(Frame::PopNesting);
                    self.parenthesis(Phase::EmptySpace(owner, parser.checkpoint()));
                    self.base(rules::WHITESPACE0);
                } else {
                    self.parenthesis(Phase::Limited(owner));
                    self.push(Frame::Nesting(Box::new(NestingContinuation::new())));
                }
            }
            Phase::Limited(owner) => {
                owner
                    .parenthetical
                    .complete(parser, SyntaxKind::ParentheticalExpression);
                owner.tuple.complete(parser, SyntaxKind::Tuple);
                owner.structure.complete(parser, SyntaxKind::Structure);
                self.result = Attempt::Committed;
            }
            Phase::EmptySpace(owner, after_open) => {
                if self.result == Attempt::Matched {
                    self.parenthesis(Phase::EmptyClose(owner, after_open));
                    self.base(rules::RIGHT_PARENTHESIS);
                } else {
                    self.result = Attempt::NoMatch;
                }
            }
            Phase::EmptyClose(owner, after_open) => {
                if self.result == Attempt::Matched {
                    finish_provisional_marker(
                        parser,
                        owner.parenthetical,
                        SyntaxKind::ParentheticalExpression,
                    );
                    self.parenthesis_finish(
                        parser,
                        Body {
                            owner,
                            selected: false,
                            committed: false,
                        },
                    );
                } else {
                    parser.rewind(after_open);
                    self.parenthesis(Phase::FirstSpace(owner, after_open));
                    self.base(rules::SPACE_TAB0);
                }
            }
            Phase::FirstSpace(owner, after_open) => {
                if self.result == Attempt::Matched {
                    let node = parser.start();
                    self.parenthesis(Phase::FirstBody(owner, after_open, node));
                    self.push(Frame::Expression(Box::new(expression::Phase::Body)));
                } else {
                    self.result = Attempt::NoMatch;
                }
            }
            Phase::FirstBody(owner, after_open, node) => match self.facts {
                FactAttempt::NoMatch => {
                    parser.rewind(after_open);
                    self.parenthesis(Phase::RetrySpace(owner));
                    self.base(rules::WHITESPACE0);
                }
                FactAttempt::Matched(ExpressionForm::Formula) => {
                    self.parenthesis(Phase::FormulaSpace(owner, node, parser.checkpoint()));
                    self.base(rules::SPACE_TAB0);
                }
                FactAttempt::Matched(_) => {
                    node.complete(parser, SyntaxKind::Expression);
                    self.parenthesis(Phase::StartList(Body {
                        owner,
                        selected: false,
                        committed: false,
                    }));
                }
                FactAttempt::Recovered(_) | FactAttempt::Committed => {
                    node.complete(parser, SyntaxKind::Expression);
                    self.parenthesis_recovery_probe(parser, owner);
                }
            },
            Phase::RetrySpace(owner) => {
                if self.result == Attempt::Matched {
                    let node = parser.start();
                    self.parenthesis(Phase::RetryBody(owner, node));
                    self.push(Frame::Expression(Box::new(expression::Phase::Body)));
                } else {
                    self.result = Attempt::NoMatch;
                }
            }
            Phase::RetryBody(owner, node) => match self.facts {
                FactAttempt::NoMatch => self.result = Attempt::NoMatch,
                FactAttempt::Matched(_) => {
                    node.complete(parser, SyntaxKind::Expression);
                    self.parenthesis(Phase::StartList(Body {
                        owner,
                        selected: false,
                        committed: false,
                    }));
                }
                FactAttempt::Recovered(_) | FactAttempt::Committed => {
                    node.complete(parser, SyntaxKind::Expression);
                    self.parenthesis_recovery_probe(parser, owner);
                }
            },
            Phase::FormulaSpace(owner, node, checkpoint) => {
                if self.result == Attempt::Matched {
                    self.parenthesis(Phase::FormulaClose(owner, node, checkpoint));
                    self.base(rules::RIGHT_PARENTHESIS);
                } else {
                    parser.rewind(checkpoint);
                    node.complete(parser, SyntaxKind::Expression);
                    self.parenthesis(Phase::StartList(Body {
                        owner,
                        selected: true,
                        committed: false,
                    }));
                }
            }
            Phase::FormulaClose(owner, node, checkpoint) => {
                if self.result == Attempt::Matched {
                    node.abandon(parser);
                    self.parenthesis_finish(
                        parser,
                        Body {
                            owner,
                            selected: true,
                            committed: false,
                        },
                    );
                } else {
                    parser.rewind(checkpoint);
                    node.complete(parser, SyntaxKind::Expression);
                    self.parenthesis(Phase::StartList(Body {
                        owner,
                        selected: true,
                        committed: false,
                    }));
                }
            }
            Phase::RecoveredProbe(owner, checkpoint) => {
                if !final_input && parser.is_eof() && !parser.is_halted() {
                    self.parenthesis(Phase::RecoveredProbe(owner, checkpoint));
                    return Some(Progress::NeedInput);
                }
                let selected = parser.cursor().starts_with(")");
                parser.rewind(checkpoint);
                self.parenthesis(Phase::StartList(Body {
                    owner,
                    selected,
                    committed: true,
                }));
            }
            Phase::StartList(body) => {
                if !body.selected {
                    finish_provisional_marker(
                        parser,
                        body.owner.parenthetical,
                        SyntaxKind::ParentheticalExpression,
                    );
                }
                self.parenthesis(Phase::Loop(body));
            }
            Phase::Loop(body) => {
                if parser.is_halted() {
                    self.parenthesis(Phase::Trailing(body));
                    self.base(rules::WHITESPACE0);
                } else {
                    self.parenthesis(Phase::Separator(body));
                    self.base(rules::LIST_SEPARATOR);
                }
            }
            Phase::Separator(mut body) => {
                if self.result == Attempt::Matched {
                    if body.selected {
                        finish_provisional_marker(
                            parser,
                            body.owner.parenthetical,
                            SyntaxKind::ParentheticalExpression,
                        );
                        body.selected = false;
                    }
                    self.parenthesis(Phase::Item(body));
                    self.push(Frame::Call(rules::EXPRESSION));
                } else {
                    self.parenthesis(Phase::Trailing(body));
                    self.base(rules::WHITESPACE0);
                }
            }
            Phase::Item(mut body) => {
                if self.result == Attempt::NoMatch {
                    self.parenthesis(Phase::RecoveredItem(body));
                    self.push(Frame::Required(Box::new(Required::new(
                        rules::TUPLE,
                        "syntax/missing-tuple-item",
                        "missing tuple item after separator",
                        "expression",
                        &[],
                        &[],
                        None,
                    ))));
                } else {
                    body.committed |= self.result == Attempt::Committed;
                    self.parenthesis(Phase::Loop(body));
                }
            }
            Phase::RecoveredItem(mut body) => {
                body.committed = true;
                self.parenthesis(Phase::Loop(body));
            }
            Phase::Trailing(body) => {
                self.parenthesis(Phase::Close(body));
                self.base(rules::RIGHT_PARENTHESIS);
            }
            Phase::Close(mut body) => {
                if self.result == Attempt::Matched {
                    self.parenthesis_finish(parser, body);
                } else {
                    body.committed = true;
                    self.parenthesis(Phase::Finish(body));
                    self.push(Frame::Closer(Box::new(Closer::new(
                        if body.selected {
                            rules::PARENTHETICAL_TERM
                        } else {
                            rules::TUPLE
                        },
                        rules::RIGHT_PARENTHESIS,
                        SyntaxKind::RightParen,
                        ")",
                    ))));
                }
            }
            Phase::Finish(body) => self.parenthesis_finish(parser, body),
        }
        None
    }
}
