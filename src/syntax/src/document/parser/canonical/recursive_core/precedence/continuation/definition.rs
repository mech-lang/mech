//! Definition selection retains the physically recognized operator fact.
use super::super::super::child_result;
use super::*;
pub(super) enum Phase {
    Start,
    Tilde(Marker),
    Variable(Marker),
    Assign(Marker, Attempt, ParserCheckpoint),
    Define(Marker, Attempt),
    Value(Marker, Attempt),
    Recovered(Marker),
}
impl Continuation {
    pub(in super::super::super) fn definition_operator(&self) -> bool {
        self.definition_operator && self.result != Attempt::NoMatch
    }
    fn definition(&mut self, phase: Phase) {
        self.push(Frame::Definition(Box::new(phase)));
    }
    fn rejected_definition(&mut self, parser: &mut Parser<'_>, node: Marker, variable: Attempt) {
        self.definition_operator = false;
        if variable == Attempt::Committed || parser.is_halted() {
            node.complete(parser, SyntaxKind::VariableDefine);
            self.result = Attempt::Committed;
        } else {
            node.abandon(parser);
            self.result = Attempt::NoMatch;
        }
    }
    #[inline(never)]
    pub(super) fn definition_frame(&mut self, parser: &mut Parser<'_>, phase: Box<Phase>) {
        let phase = *phase;
        match phase {
            Phase::Start => {
                self.definition_operator = false;
                self.transaction(parser, rules::VARIABLE_DEFINE);
                let node = parser.start();
                self.definition(Phase::Tilde(node));
                self.base(rules::TILDE);
            }
            Phase::Tilde(node) => {
                self.definition(Phase::Variable(node));
                self.push(Frame::Variable(false));
            }
            Phase::Variable(node) => {
                if self.result == Attempt::NoMatch || parser.is_halted() {
                    self.result =
                        child_result(parser, node, SyntaxKind::VariableDefine, self.result)
                            .expect("absent or halted variable owner");
                    self.definition_operator = false;
                } else {
                    self.definition(Phase::Assign(node, self.result, parser.checkpoint()));
                    self.base(rules::ASSIGN_OPERATOR);
                }
            }
            Phase::Assign(node, variable, checkpoint) => {
                let assignment = self.result == Attempt::Matched;
                parser.rewind(checkpoint);
                if assignment {
                    self.rejected_definition(parser, node, variable);
                } else {
                    self.definition(Phase::Define(node, variable));
                    self.base(rules::DEFINE_OPERATOR);
                }
            }
            Phase::Define(node, variable) => {
                if self.result == Attempt::NoMatch {
                    self.rejected_definition(parser, node, variable);
                } else {
                    self.definition(Phase::Value(node, variable));
                    self.push(Frame::Call(rules::EXPRESSION));
                }
            }
            Phase::Value(node, variable) => {
                if self.result == Attempt::NoMatch {
                    self.definition(Phase::Recovered(node));
                    self.push(Frame::Required(Box::new(Required::new(
                        rules::VARIABLE_DEFINE,
                        "syntax/missing-variable-definition-value",
                        "missing value after definition operator",
                        "expression",
                        &[],
                        &[],
                        None,
                    ))));
                } else {
                    node.complete(parser, SyntaxKind::VariableDefine);
                    self.definition_operator = true;
                    if self.result != Attempt::Committed {
                        self.result = variable;
                    }
                }
            }
            Phase::Recovered(node) => {
                node.complete(parser, SyntaxKind::VariableDefine);
                self.definition_operator = true;
                self.result = Attempt::Committed;
            }
        }
    }
}
