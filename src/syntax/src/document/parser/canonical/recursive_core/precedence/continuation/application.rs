//! Ordinary application stages share the resumable recursive expression owner.
use super::*;

pub(super) fn supports(rule: RuleId) -> bool {
    matches!(rule, rules::APPLICATION_STAGE | rules::APPLICATION_OPERATOR)
}

pub(super) enum Phase {
    Enter(RuleId),
    StageSpace(Marker),
    StageOperator(Marker),
    StageNameSpace(Marker, bool),
    StageName(Marker, bool),
    StageRecovered(Marker),
    OperatorBar(Marker),
    OperatorDigits(Marker, bool),
    OperatorDigit(Marker),
    OperatorCount(Marker, Marker, Marker),
    OperatorAngle(Marker),
    OperatorRecovered(Marker),
}

impl Continuation {
    fn application(&mut self, phase: Phase) {
        self.push(Frame::Application(Box::new(phase)));
    }

    pub(super) fn application_frame(
        &mut self,
        parser: &mut Parser<'_>,
        phase: Box<Phase>,
        final_input: bool,
    ) -> Option<Progress> {
        if parser.is_halted() {
            let (node, kind) = match *phase {
                Phase::Enter(_) => {
                    self.result = Attempt::Committed;
                    return None;
                }
                Phase::StageSpace(node)
                | Phase::StageOperator(node)
                | Phase::StageNameSpace(node, _)
                | Phase::StageName(node, _)
                | Phase::StageRecovered(node) => (node, SyntaxKind::ApplicationStage),
                Phase::OperatorBar(node)
                | Phase::OperatorDigits(node, _)
                | Phase::OperatorDigit(node)
                | Phase::OperatorAngle(node)
                | Phase::OperatorRecovered(node) => (node, SyntaxKind::ApplicationOperator),
                Phase::OperatorCount(node, factor, structure) => {
                    structure.complete(parser, SyntaxKind::Structure);
                    factor.complete(parser, SyntaxKind::Factor);
                    (node, SyntaxKind::ApplicationOperator)
                }
            };
            node.complete(parser, kind);
            self.result = Attempt::Committed;
            return None;
        }
        match *phase {
            Phase::Enter(rule) => {
                self.transaction(parser, rule);
                let node = parser.start();
                if rule == rules::APPLICATION_STAGE {
                    self.application(Phase::StageSpace(node));
                    self.base(rules::WHITESPACE0);
                } else {
                    self.application(Phase::OperatorBar(node));
                    self.base(rules::BAR);
                }
            }
            Phase::StageSpace(node) => {
                self.application(Phase::StageOperator(node));
                self.application(Phase::Enter(rules::APPLICATION_OPERATOR));
            }
            Phase::StageOperator(node) => {
                if self.result == Attempt::NoMatch {
                    node.abandon(parser);
                } else {
                    self.application(Phase::StageNameSpace(
                        node,
                        self.result == Attempt::Committed,
                    ));
                    self.base(rules::WHITESPACE0);
                }
            }
            Phase::StageNameSpace(node, committed) => {
                self.application(Phase::StageName(node, committed));
                self.base(rules::IDENTIFIER);
            }
            Phase::StageName(node, committed) => {
                if self.result == Attempt::NoMatch {
                    self.application(Phase::StageRecovered(node));
                    self.push(Frame::Required(Box::new(Required::new(
                        rules::APPLICATION_STAGE,
                        "syntax/missing-application-function",
                        "missing unary function after application operator",
                        "identifier",
                        &[],
                        &[],
                        None,
                    ))));
                } else {
                    node.complete(parser, SyntaxKind::ApplicationStage);
                    if committed {
                        self.result = Attempt::Committed;
                    }
                }
            }
            Phase::StageRecovered(node) => {
                node.complete(parser, SyntaxKind::ApplicationStage);
                self.result = Attempt::Committed;
            }
            Phase::OperatorBar(node) => {
                if self.result == Attempt::NoMatch {
                    node.abandon(parser);
                } else {
                    self.application(Phase::OperatorDigits(node, false));
                }
            }
            Phase::OperatorDigits(node, counted) => {
                if !final_input && parser.is_eof() {
                    self.application(Phase::OperatorDigits(node, counted));
                    return Some(Progress::NeedInput);
                }
                let digit = parser
                    .cursor()
                    .peek_char()
                    .is_some_and(|character| character.is_ascii_digit());
                if !counted && parser.cursor().starts_with("[") {
                    let factor = parser.start();
                    let structure = parser.start();
                    self.application(Phase::OperatorCount(node, factor, structure));
                    self.push(Frame::Call(rules::MATRIX));
                } else if digit {
                    self.application(Phase::OperatorDigit(node));
                    self.base(rules::DIGIT_TOKEN);
                } else if parser.cursor().starts_with(">") || counted && parser.is_eof() {
                    self.application(Phase::OperatorAngle(node));
                    self.base(rules::RIGHT_ANGLE);
                } else {
                    // Digits alone do not commit an application: numeric and
                    // typed-literal table rows follow the same bar. Only `>`
                    // distinguishes a complete local counted token; EOF may
                    // recover its missing terminator.
                    node.abandon(parser);
                    self.result = Attempt::NoMatch;
                }
            }
            Phase::OperatorDigit(node) => {
                self.application(Phase::OperatorDigits(node, true));
            }
            Phase::OperatorCount(node, factor, structure) => {
                if !final_input && parser.is_eof() {
                    self.application(Phase::OperatorCount(node, factor, structure));
                    return Some(Progress::NeedInput);
                }
                if self.result == Attempt::NoMatch {
                    structure.abandon(parser);
                    factor.abandon(parser);
                    node.abandon(parser);
                } else if self.result == Attempt::Matched
                    && !parser.cursor().starts_with(">")
                    && !parser.is_eof()
                {
                    // A following matrix may be the next table row rather
                    // than a count. Like literal counts, a clean bracketed
                    // candidate commits only at `>`; EOF can recover it.
                    structure.abandon(parser);
                    factor.abandon(parser);
                    node.abandon(parser);
                    self.result = Attempt::NoMatch;
                } else {
                    structure.complete(parser, SyntaxKind::Structure);
                    factor.complete(parser, SyntaxKind::Factor);
                    self.application(Phase::OperatorAngle(node));
                    self.base(rules::RIGHT_ANGLE);
                }
            }
            Phase::OperatorAngle(node) => {
                if self.result == Attempt::NoMatch {
                    self.application(Phase::OperatorRecovered(node));
                    self.push(Frame::Required(Box::new(Required::new(
                        rules::APPLICATION_OPERATOR,
                        "syntax/missing-application-angle",
                        "missing > after application count",
                        "right-angle",
                        &[],
                        &[],
                        None,
                    ))));
                } else {
                    node.complete(parser, SyntaxKind::ApplicationOperator);
                }
            }
            Phase::OperatorRecovered(node) => {
                node.complete(parser, SyntaxKind::ApplicationOperator);
                self.result = Attempt::Committed;
            }
        }
        None
    }
}
