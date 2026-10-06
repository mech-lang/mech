//! Comprehension selection retains recognized qualifier discriminators.
use super::super::super::QualifierKind;
use super::*;
pub(super) fn supports(rule: RuleId) -> bool {
    matches!(
        rule,
        rules::SET_COMPREHENSION | rules::COMPREHENSION_QUALIFIER | rules::GENERATOR
    )
}
#[derive(Clone, Copy)]
pub(super) struct Tail {
    close: RuleId,
    require: bool,
    committed: bool,
    has: bool,
    after: bool,
}
impl Tail {
    fn rule(self) -> RuleId {
        if self.close == rules::RIGHT_BRACKET {
            rules::MATRIX_COMPREHENSION
        } else {
            rules::SET_COMPREHENSION
        }
    }
}
pub(super) enum Phase {
    Enter(RuleId),
    Finish(Marker, SyntaxKind),
    SetOpen,
    SetSpace,
    SetValue,
    SetBeforeBar(bool),
    SetBar(bool),
    SetAfterBar(bool),
    SetTail(bool),
    QualifierGenerator,
    QualifierDefinition,
    QualifierFilter,
    GeneratorPattern,
    GeneratorSpace(bool),
    GeneratorArrow(bool),
    GeneratorUnicodeArrow(bool),
    GeneratorAfterArrow(bool),
    GeneratorValue(bool),
    GeneratorRecovered,
    TailStart(Tail),
    TailSpace(Tail),
    TailItem(Tail),
    TailRecovered(Tail),
    TailLoop(Tail),
    TailSeparator(Tail),
    TailValidate(Tail),
    TailTrailing(Tail),
    TailClose(Tail),
}
impl Continuation {
    pub(super) fn comprehension_tail(&mut self, close: RuleId, require: bool) {
        self.comp(Phase::TailStart(Tail {
            close,
            require,
            committed: false,
            has: false,
            after: false,
        }));
    }
    fn comp(&mut self, phase: Phase) {
        self.push(Frame::Comprehension(Box::new(phase)));
    }
    fn qualifier_result(&mut self, kind: Option<QualifierKind>, committed: bool) {
        self.qualifier_kind = kind;
        self.result = if committed {
            Attempt::Committed
        } else {
            Attempt::Matched
        };
    }
    fn generator_reject(&mut self, parser: &Parser<'_>) {
        self.qualifier_kind = None;
        self.result = if parser.is_halted() {
            Attempt::Committed
        } else {
            Attempt::NoMatch
        };
    }
    #[inline(never)]
    pub(super) fn comprehension_frame(&mut self, parser: &mut Parser<'_>, phase: Box<Phase>) {
        let phase = *phase;
        match phase {
            Phase::Enter(rule) => {
                self.transaction(parser, rule);
                let node = parser.start();
                self.qualifier_kind = None;
                let kind = match rule {
                    rules::SET_COMPREHENSION => SyntaxKind::SetComprehension,
                    rules::COMPREHENSION_QUALIFIER => SyntaxKind::ComprehensionQualifier,
                    rules::GENERATOR => SyntaxKind::Generator,
                    _ => unreachable!("comprehension owner"),
                };
                self.comp(Phase::Finish(node, kind));
                match rule {
                    rules::SET_COMPREHENSION => {
                        self.comp(Phase::SetOpen);
                        self.base(rules::LEFT_BRACE);
                    }
                    rules::COMPREHENSION_QUALIFIER => {
                        self.comp(Phase::QualifierGenerator);
                        self.push(Frame::Call(rules::GENERATOR));
                    }
                    _ => {
                        self.comp(Phase::GeneratorPattern);
                        self.push(Frame::Call(rules::PATTERN));
                    }
                }
            }
            Phase::Finish(node, kind) => {
                if self.result == Attempt::NoMatch {
                    if !parser.state.resource_finalizing {
                        node.abandon(parser);
                    }
                } else {
                    node.complete(parser, kind);
                }
            }
            Phase::SetOpen => {
                if self.result != Attempt::Matched {
                    self.result = Attempt::NoMatch;
                } else if parser.push_nesting() {
                    self.push(Frame::PopNesting);
                    self.comp(Phase::SetSpace);
                    self.base(rules::SPACE_TAB0);
                } else {
                    self.push(Frame::Nesting(Box::new(NestingContinuation::new())));
                }
            }
            Phase::SetSpace => {
                if self.result == Attempt::Matched {
                    self.comp(Phase::SetValue);
                    self.push(Frame::Call(rules::EXPRESSION));
                } else {
                    self.result = Attempt::NoMatch;
                }
            }
            Phase::SetValue => {
                if self.result != Attempt::NoMatch && !parser.is_halted() {
                    self.comp(Phase::SetBeforeBar(self.result == Attempt::Committed));
                    self.base(rules::SPACE_TAB0);
                }
            }
            Phase::SetBeforeBar(committed) => {
                if self.result == Attempt::Matched {
                    self.comp(Phase::SetBar(committed));
                    self.base(rules::BAR);
                } else {
                    self.result = Attempt::NoMatch;
                }
            }
            Phase::SetBar(committed) => {
                if self.result == Attempt::Matched {
                    self.comp(Phase::SetAfterBar(committed));
                    self.base(rules::SPACE_TAB0);
                } else {
                    self.result = Attempt::NoMatch;
                }
            }
            Phase::SetAfterBar(committed) => {
                if self.result == Attempt::Matched {
                    self.comp(Phase::SetTail(committed));
                    self.comprehension_tail(rules::RIGHT_BRACE, false);
                } else {
                    self.result = Attempt::NoMatch;
                }
            }
            Phase::SetTail(committed) => {
                if committed && self.result == Attempt::Matched {
                    self.result = Attempt::Committed;
                }
            }
            Phase::QualifierGenerator => {
                if self.result == Attempt::NoMatch {
                    self.comp(Phase::QualifierDefinition);
                    self.push(Frame::Call(rules::VARIABLE_DEFINE));
                }
            }
            Phase::QualifierDefinition => {
                if self.result == Attempt::NoMatch {
                    self.comp(Phase::QualifierFilter);
                    self.push(Frame::Call(rules::EXPRESSION));
                } else {
                    self.qualifier_kind = if self.definition_operator() {
                        Some(QualifierKind::Let)
                    } else {
                        None
                    };
                }
            }
            Phase::QualifierFilter => {
                self.qualifier_kind = if self.result != Attempt::NoMatch {
                    Some(QualifierKind::Filter)
                } else {
                    None
                };
            }
            Phase::GeneratorPattern => {
                if self.result == Attempt::NoMatch || parser.is_halted() {
                    self.generator_reject(parser);
                } else {
                    self.comp(Phase::GeneratorSpace(self.result == Attempt::Committed));
                    self.base(rules::SPACE_TAB0);
                }
            }
            Phase::GeneratorSpace(committed) => {
                if self.result == Attempt::Matched {
                    self.comp(Phase::GeneratorArrow(committed));
                    self.base(rules::GENERATOR_ARROW);
                } else {
                    self.generator_reject(parser);
                }
            }
            Phase::GeneratorArrow(committed) => {
                if self.result == Attempt::Matched {
                    self.comp(Phase::GeneratorAfterArrow(committed));
                    self.base(rules::SPACE_TAB0);
                } else {
                    self.comp(Phase::GeneratorUnicodeArrow(committed));
                    self.base(rules::GENERATOR_ARROW_U);
                }
            }
            Phase::GeneratorUnicodeArrow(committed) => {
                if self.result == Attempt::Matched {
                    self.comp(Phase::GeneratorAfterArrow(committed));
                    self.base(rules::SPACE_TAB0);
                } else {
                    self.generator_reject(parser);
                }
            }
            Phase::GeneratorAfterArrow(committed) => {
                if self.result == Attempt::Matched {
                    self.comp(Phase::GeneratorValue(committed));
                    self.push(Frame::Call(rules::EXPRESSION));
                } else {
                    self.generator_reject(parser);
                }
            }
            Phase::GeneratorValue(committed) => {
                if self.result == Attempt::NoMatch {
                    self.comp(Phase::GeneratorRecovered);
                    self.push(Frame::Required(Box::new(Required::new(
                        rules::GENERATOR,
                        "syntax/missing-generator-source",
                        "missing source expression after generator arrow",
                        "expression",
                        &[],
                        &[],
                        None,
                    ))));
                } else {
                    self.qualifier_result(
                        Some(QualifierKind::Generator),
                        committed || self.result == Attempt::Committed,
                    );
                }
            }
            Phase::GeneratorRecovered => {
                self.qualifier_result(Some(QualifierKind::Generator), true)
            }
            Phase::TailStart(tail) => {
                self.comp(Phase::TailSpace(tail));
                self.base(rules::SPACE_TAB0);
            }
            Phase::TailSpace(tail) => {
                if self.result == Attempt::Matched {
                    self.comp(Phase::TailItem(tail));
                    self.push(Frame::Call(rules::COMPREHENSION_QUALIFIER));
                } else {
                    self.result = Attempt::NoMatch;
                }
            }
            Phase::TailItem(mut tail) => {
                tail.has |= matches!(
                    self.qualifier_kind,
                    Some(QualifierKind::Generator | QualifierKind::Let)
                );
                match self.result {
                    Attempt::NoMatch => {
                        self.comp(Phase::TailRecovered(tail));
                        self.push(Frame::Required(Box::new(Required::new(
                            tail.rule(),
                            "syntax/missing-comprehension-qualifier",
                            if tail.after {
                                "missing comprehension qualifier after separator"
                            } else {
                                "missing comprehension qualifier after bar"
                            },
                            "comprehension-qualifier",
                            &[],
                            &[],
                            None,
                        ))));
                        return;
                    }
                    Attempt::Committed => tail.committed = true,
                    Attempt::Matched => {}
                }
                self.comp(Phase::TailLoop(tail));
            }
            Phase::TailRecovered(mut tail) => {
                tail.committed = true;
                self.comp(Phase::TailLoop(tail));
            }
            Phase::TailLoop(tail) => {
                if parser.is_halted() {
                    self.comp(Phase::TailValidate(tail));
                } else {
                    self.comp(Phase::TailSeparator(tail));
                    self.base(rules::LIST_SEPARATOR);
                }
            }
            Phase::TailSeparator(mut tail) => {
                if self.result == Attempt::Matched {
                    tail.after = true;
                    self.comp(Phase::TailItem(tail));
                    self.push(Frame::Call(rules::COMPREHENSION_QUALIFIER));
                } else {
                    self.comp(Phase::TailValidate(tail));
                }
            }
            Phase::TailValidate(tail) => {
                if tail.require && !tail.has && !parser.is_halted() {
                    self.result = Attempt::NoMatch;
                } else {
                    self.comp(Phase::TailTrailing(tail));
                    self.base(rules::SPACE_TAB0);
                }
            }
            Phase::TailTrailing(tail) => {
                self.comp(Phase::TailClose(tail));
                self.base(tail.close);
            }
            Phase::TailClose(tail) => {
                if self.result == Attempt::Matched {
                    self.result = if tail.committed {
                        Attempt::Committed
                    } else {
                        Attempt::Matched
                    };
                } else {
                    let (kind, text) = if tail.close == rules::RIGHT_BRACKET {
                        (SyntaxKind::RightBracket, "]")
                    } else {
                        (SyntaxKind::RightBrace, "}")
                    };
                    self.push(Frame::Closer(Box::new(Closer::new(
                        tail.rule(),
                        tail.close,
                        kind,
                        text,
                    ))));
                }
            }
        }
    }
}
