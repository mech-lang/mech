//! Expression ownership preserves canonical discriminator facts and shared ranges.
use super::super::super::{ExpressionForm, FactAttempt};
use super::*;
pub(super) enum Phase {
    Enter,
    EnterGrouped,
    CompleteBody,
    Finish(Marker, bool),
    ApplicationEnter,
    ApplicationFirst(Marker),
    ApplicationLoop(Marker, FactAttempt<ExpressionForm>, bool, bool),
    ApplicationNext(Marker, FactAttempt<ExpressionForm>, bool, bool, TextSize),
    AfterApplication,
    Body,
    Fsm,
    Delimited(Marker, FormulaSeed),
    DelimitedOperator(Marker, FormulaSeed, ParserCheckpoint, usize),
    DelimitedMatchSpace(Marker, FormulaSeed, ParserCheckpoint),
    DelimitedMatchQuestion(Marker, FormulaSeed, ParserCheckpoint),
    Formula(Marker),
    Seeded(Marker),
    RangeOperator(Marker, bool),
    RangeDone,
    AfterMatch(bool, TextSize, FactAttempt<ExpressionForm>),
    Match,
    MatchSpace(ParserCheckpoint),
    MatchQuestion(ParserCheckpoint),
    MatchLeading,
    MatchFirst,
    MatchRecovered,
    MatchLoop(bool),
    MatchNext(bool, TextSize),
    MatchPeriod(bool),
}
impl Continuation {
    fn expression(&mut self, phase: Phase) {
        self.push(Frame::Expression(Box::new(phase)));
    }
    pub(super) fn expression_result(&mut self, result: FactAttempt<ExpressionForm>) {
        self.facts = result;
        self.result = result.attempt();
    }
    fn expression_range(&mut self, node: Marker, committed: bool) {
        self.expression(Phase::RangeOperator(node, committed));
        self.push(Frame::LeafOperator(Box::new(operators::Continuation::new(
            rules::RANGE_OPERATOR,
        ))));
    }
    #[inline(never)]
    pub(super) fn expression_frame(
        &mut self,
        parser: &mut Parser<'_>,
        phase: Box<Phase>,
        final_input: bool,
    ) -> Option<Progress> {
        let phase = *phase;
        match phase {
            Phase::Enter | Phase::EnterGrouped => {
                let grouped = matches!(phase, Phase::EnterGrouped);
                self.transaction(parser, rules::EXPRESSION);
                let node = parser.start();
                self.expression(Phase::Finish(node, grouped));
                self.expression(Phase::CompleteBody);
            }
            Phase::CompleteBody => {
                if parser.is_halted() {
                    self.expression_result(FactAttempt::Committed);
                } else {
                    self.expression(Phase::AfterApplication);
                    self.expression(Phase::ApplicationEnter);
                }
            }
            Phase::Finish(node, grouped) => {
                if parser.is_halted() {
                    self.expression_result(FactAttempt::Committed);
                }
                self.result = self.facts.attempt();
                if self.result == Attempt::NoMatch
                    || grouped
                        && matches!(self.facts, FactAttempt::Matched(ExpressionForm::Formula))
                {
                    node.abandon(parser);
                } else {
                    node.complete(parser, SyntaxKind::Expression);
                }
            }
            Phase::ApplicationEnter => {
                self.transaction(parser, rules::APPLICATION_EXPRESSION);
                let node = parser.start();
                self.expression(Phase::ApplicationFirst(node));
                self.expression(Phase::Body);
            }
            Phase::ApplicationFirst(node) => {
                // Delimited seeds finish with a failed operator probe. Their
                // semantic discriminator, rather than that probe, owns the
                // seed result before the application transaction can close.
                self.result = if parser.is_halted() {
                    Attempt::Committed
                } else {
                    self.facts.attempt()
                };
                if self.result == Attempt::NoMatch {
                    node.abandon(parser);
                } else {
                    self.expression(Phase::ApplicationLoop(
                        node,
                        self.facts,
                        false,
                        self.result == Attempt::Committed,
                    ));
                }
            }
            Phase::ApplicationLoop(node, seed, stages, committed) => {
                if parser.is_halted() {
                    if stages {
                        node.complete(parser, SyntaxKind::ApplicationExpression);
                    } else {
                        super::super::super::finish_provisional_marker(
                            parser,
                            node,
                            SyntaxKind::ApplicationExpression,
                        );
                    }
                    self.expression_result(FactAttempt::Committed);
                    return None;
                }
                self.expression(Phase::ApplicationNext(
                    node,
                    seed,
                    stages,
                    committed,
                    parser.offset(),
                ));
                self.push(Frame::Application(Box::new(application::Phase::Enter(
                    rules::APPLICATION_STAGE,
                ))));
            }
            Phase::ApplicationNext(node, seed, stages, committed, before) => {
                if parser.is_halted() {
                    if stages {
                        node.complete(parser, SyntaxKind::ApplicationExpression);
                    } else {
                        super::super::super::finish_provisional_marker(
                            parser,
                            node,
                            SyntaxKind::ApplicationExpression,
                        );
                    }
                    self.expression_result(FactAttempt::Committed);
                } else if self.result != Attempt::NoMatch && parser.offset() > before {
                    self.expression(Phase::ApplicationLoop(
                        node,
                        seed,
                        true,
                        committed || self.result == Attempt::Committed,
                    ));
                } else {
                    if stages {
                        node.complete(parser, SyntaxKind::ApplicationExpression);
                    } else {
                        node.abandon(parser);
                    }
                    self.expression_result(if committed {
                        FactAttempt::Committed
                    } else if stages {
                        FactAttempt::Matched(ExpressionForm::Application)
                    } else {
                        seed
                    });
                }
            }
            Phase::AfterApplication => {
                if parser.is_halted() {
                    self.expression_result(FactAttempt::Committed);
                } else if self.result != Attempt::NoMatch {
                    self.expression(Phase::AfterMatch(
                        self.result == Attempt::Committed,
                        parser.offset(),
                        self.facts,
                    ));
                    self.expression(Phase::Match);
                }
            }
            Phase::Body => {
                if !final_input && parser.is_eof() {
                    self.expression(Phase::Body);
                    return Some(Progress::NeedInput);
                }
                if parser.cursor().starts_with("#") {
                    self.expression(Phase::Fsm);
                    self.push(Frame::Call(rules::FSM_PIPE));
                } else {
                    let range = parser.start();
                    if parser.cursor().starts_with("{") || parser.cursor().starts_with("[") {
                        let brace = parser.cursor().starts_with("{");
                        let seed = FormulaSeed::start(parser);
                        self.expression(Phase::Delimited(range, seed));
                        if brace {
                            self.push(Frame::Brace(Box::new(brace::Phase::Enter(true))));
                        } else {
                            self.push(Frame::Bracket(Box::new(bracket::Phase::Project)));
                        }
                    } else {
                        self.expression(Phase::Formula(range));
                        self.push(Frame::Call(rules::FORMULA));
                    }
                }
            }
            Phase::Fsm => self.expression_result(match self.result {
                Attempt::Matched => FactAttempt::Matched(ExpressionForm::FsmPipe),
                Attempt::Committed => FactAttempt::Committed,
                Attempt::NoMatch => FactAttempt::NoMatch,
            }),
            Phase::Delimited(range, seed) => {
                let committed = match self.facts {
                    FactAttempt::Matched(
                        ExpressionForm::SetComprehension | ExpressionForm::MatrixComprehension,
                    ) => {
                        let checkpoint = parser.checkpoint();
                        self.expression(Phase::DelimitedOperator(range, seed, checkpoint, 0));
                        self.push(Frame::LeafOperator(Box::new(operators::Continuation::new(
                            rules::TRANSPOSE,
                        ))));
                        return None;
                    }
                    FactAttempt::Recovered(
                        ExpressionForm::SetComprehension | ExpressionForm::MatrixComprehension,
                    ) if !parser.is_halted() => {
                        seed.abandon(parser);
                        range.abandon(parser);
                        return None;
                    }
                    FactAttempt::NoMatch => {
                        seed.abandon(parser);
                        range.abandon(parser);
                        return None;
                    }
                    FactAttempt::Matched(ExpressionForm::Formula) => false,
                    FactAttempt::Recovered(_) | FactAttempt::Committed => {
                        if parser.is_halted() {
                            seed.commit(parser);
                            expressions::finish_provisional_formula_marker(parser, range);
                            self.expression_result(FactAttempt::Committed);
                            return None;
                        }
                        true
                    }
                    FactAttempt::Matched(_) => unreachable!("closed delimited expression forms"),
                };
                self.expression(Phase::Seeded(range));
                self.push(Frame::SeedTranspose(seed, committed));
                self.push(Frame::LeafOperator(Box::new(operators::Continuation::new(
                    rules::TRANSPOSE,
                ))));
            }
            Phase::DelimitedOperator(range, seed, checkpoint, probe) => {
                if self.result.accepted() {
                    parser.rewind(checkpoint);
                    self.expression(Phase::Seeded(range));
                    self.push(Frame::SeedTranspose(seed, false));
                    if probe == 0 {
                        // The first lookahead consumed the apostrophe. After
                        // rewinding, parse it again into the seeded factor.
                        self.push(Frame::LeafOperator(Box::new(operators::Continuation::new(
                            rules::TRANSPOSE,
                        ))));
                    }
                } else if probe < LEVELS.len() {
                    parser.rewind(checkpoint);
                    self.expression(Phase::DelimitedOperator(range, seed, checkpoint, probe + 1));
                    self.push(Frame::Operator(probe, 0));
                } else if probe == LEVELS.len() {
                    // Range has its own expression continuation. Probe it
                    // before concluding that the delimited seed ended here.
                    parser.rewind(checkpoint);
                    self.expression(Phase::DelimitedOperator(range, seed, checkpoint, probe + 1));
                    self.push(Frame::LeafOperator(Box::new(operators::Continuation::new(
                        rules::RANGE_OPERATOR,
                    ))));
                } else {
                    parser.rewind(checkpoint);
                    self.expression(Phase::DelimitedMatchSpace(range, seed, checkpoint));
                    self.base(rules::WHITESPACE0);
                }
            }
            Phase::DelimitedMatchSpace(range, seed, checkpoint) => {
                if self.result.accepted() {
                    self.expression(Phase::DelimitedMatchQuestion(range, seed, checkpoint));
                    self.base(rules::QUESTION);
                } else {
                    parser.rewind(checkpoint);
                    seed.abandon(parser);
                    range.abandon(parser);
                }
            }
            Phase::DelimitedMatchQuestion(range, seed, checkpoint) => {
                parser.rewind(checkpoint);
                if self.result.accepted() {
                    self.expression(Phase::Seeded(range));
                    self.push(Frame::SeedTranspose(seed, false));
                } else {
                    seed.abandon(parser);
                    range.abandon(parser);
                }
            }
            Phase::Formula(range) | Phase::Seeded(range) => match self.result {
                Attempt::NoMatch => {
                    range.abandon(parser);
                    self.expression_result(FactAttempt::NoMatch);
                }
                Attempt::Committed if parser.is_halted() => {
                    expressions::finish_provisional_formula_marker(parser, range);
                    self.expression_result(FactAttempt::Committed);
                }
                result => self.expression_range(range, result == Attempt::Committed),
            },
            Phase::RangeOperator(range, committed) => match self.result {
                Attempt::NoMatch => {
                    range.abandon(parser);
                    self.expression_result(if committed {
                        FactAttempt::Committed
                    } else {
                        FactAttempt::Matched(ExpressionForm::Formula)
                    });
                }
                Attempt::Committed => {
                    expressions::finish_provisional_formula_marker(parser, range);
                    self.expression_result(FactAttempt::Committed);
                }
                Attempt::Matched => {
                    self.expression(Phase::RangeDone);
                    self.push(Frame::RangeMiddle(range, committed, rules::EXPRESSION));
                    self.push(Frame::Call(rules::FORMULA));
                }
            },
            Phase::RangeDone => self.expression_result(if self.result == Attempt::Matched {
                FactAttempt::Matched(ExpressionForm::Range)
            } else {
                FactAttempt::Committed
            }),
            Phase::AfterMatch(committed, before, seed) => {
                if committed {
                    self.expression_result(FactAttempt::Committed);
                } else if parser.offset() == before {
                    self.expression_result(seed);
                }
            }
            Phase::Match => {
                self.expression(Phase::MatchSpace(parser.checkpoint()));
                self.base(rules::WHITESPACE0);
            }
            Phase::MatchSpace(checkpoint) => {
                if self.result == Attempt::NoMatch {
                    parser.rewind(checkpoint);
                    self.expression_result(FactAttempt::Matched(ExpressionForm::Formula));
                } else {
                    self.expression(Phase::MatchQuestion(checkpoint));
                    self.base(rules::QUESTION);
                }
            }
            Phase::MatchQuestion(checkpoint) => {
                if self.result == Attempt::NoMatch {
                    parser.rewind(checkpoint);
                    self.expression_result(FactAttempt::Matched(ExpressionForm::Formula));
                } else {
                    self.expression(Phase::MatchLeading);
                    self.base(rules::WHITESPACE0);
                }
            }
            Phase::MatchLeading => {
                if self.result == Attempt::NoMatch {
                    self.expression_result(FactAttempt::NoMatch);
                } else {
                    self.expression(Phase::MatchFirst);
                    self.push(Frame::Call(rules::MATCH_ARM));
                }
            }
            Phase::MatchFirst => {
                if self.result == Attempt::NoMatch {
                    self.expression(Phase::MatchRecovered);
                    self.push(Frame::Required(Box::new(Required::new(
                        rules::EXPRESSION,
                        "syntax/missing-match-arm",
                        "missing match arm after question mark",
                        "match-arm",
                        &[],
                        &[],
                        None,
                    ))));
                } else {
                    self.expression(Phase::MatchLoop(self.result == Attempt::Committed));
                }
            }
            Phase::MatchRecovered => self.expression(Phase::MatchLoop(true)),
            Phase::MatchLoop(committed) => {
                self.expression(Phase::MatchNext(committed, parser.offset()));
                self.push(Frame::Call(rules::MATCH_ARM));
            }
            Phase::MatchNext(committed, before) => {
                if self.result != Attempt::NoMatch && parser.offset() > before {
                    self.expression(Phase::MatchLoop(
                        committed || self.result == Attempt::Committed,
                    ));
                } else {
                    self.expression(Phase::MatchPeriod(committed));
                    self.base(rules::PERIOD);
                }
            }
            Phase::MatchPeriod(committed) => self.expression_result(if committed {
                FactAttempt::Committed
            } else {
                FactAttempt::Matched(ExpressionForm::Match)
            }),
        }
        None
    }
}
