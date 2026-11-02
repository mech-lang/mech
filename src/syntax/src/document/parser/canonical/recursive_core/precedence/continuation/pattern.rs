//! Pattern ownership keeps discriminator facts alongside suspended children.
use super::super::super::PatternFacts;
use super::*;
#[derive(Clone, Copy)]
pub(super) enum ArrayToken {
    Spread,
    Rest,
    Item(PatternFacts),
}
#[derive(Clone, Copy, Default)]
pub(super) struct ArrayFacts {
    facts: PatternFacts,
    spread: usize,
    rest: usize,
    after_rest: usize,
    invalid: bool,
}
impl ArrayFacts {
    fn add(&mut self, token: ArrayToken) {
        if self.rest > 0 {
            self.after_rest += 1;
        }
        match token {
            ArrayToken::Spread => {
                self.spread += 1;
                self.invalid |= self.rest > 0;
            }
            ArrayToken::Rest => {
                self.rest += 1;
                self.invalid |= self.spread > 0;
            }
            ArrayToken::Item(facts) => self.facts.merge(facts),
        }
    }
    fn finish(mut self) -> FactAttempt<PatternFacts> {
        if self.invalid
            || self.spread > 1
            || self.rest > 1
            || (self.rest > 0 && self.after_rest != 1)
        {
            FactAttempt::NoMatch
        } else {
            self.facts.contains_array_spread_or_rest |= self.spread > 0 || self.rest > 0;
            FactAttempt::Matched(self.facts)
        }
    }
}
pub(super) fn supports(rule: RuleId) -> bool {
    matches!(
        rule,
        rules::PATTERN
            | rules::PATTERN_TUPLE_STRUCT
            | rules::PATTERN_ATOM_STRUCT
            | rules::PATTERN_TUPLE
            | rules::PATTERN_ARRAY
            | rules::PATTERN_ARRAY_ITEM
            | rules::PATTERN_ARRAY_TOKEN
            | rules::FSM_VALUE
    )
}
#[derive(Clone, Copy)]
pub(super) struct List {
    rule: RuleId,
    kind: SyntaxKind,
}
pub(super) enum Phase {
    Enter(RuleId),
    Exit(ParserCheckpoint),
    Choice(Marker, usize),
    Selected(Marker, usize),
    Prefix(Marker, List),
    Name(Marker, List),
    Open(Marker, List),
    Finish(Marker, List),
    ListSpace(List),
    ListItem(List, PatternFacts, bool, bool),
    ListRecovered(List, PatternFacts),
    ListLoop(List, PatternFacts, bool),
    ListSeparator(List, PatternFacts, bool),
    ListTrailing(List, PatternFacts, bool),
    ListClose(List, PatternFacts, bool),
    ListClosed(PatternFacts),
    ArrayOpen(Marker),
    ArraySpace,
    ArrayLoop(ArrayFacts, bool),
    ArrayClose(ArrayFacts, bool),
    ArrayItem(ArrayFacts, bool, TextSize),
    ArrayMissing(Marker, ArrayFacts),
    ArrayAfter(ArrayFacts, bool),
    ArraySeparator(ArrayFacts, bool),
    ArrayRecovered(ArrayFacts),
    TokenSpread(Marker),
    TokenRest(Marker),
    TokenItem(Marker),
    Value(Marker),
}
impl Continuation {
    fn pattern(&mut self, phase: Phase) {
        self.push(Frame::Pattern(Box::new(phase)));
    }
    fn pattern_result(&mut self, result: FactAttempt<PatternFacts>) {
        self.result = result.attempt();
        self.pattern_facts = result;
    }
    fn pattern_finish(&mut self, parser: &mut Parser<'_>, node: Marker, kind: SyntaxKind) {
        let result = if parser.is_halted() {
            match self.pattern_facts {
                FactAttempt::Matched(f) | FactAttempt::Recovered(f) => FactAttempt::Recovered(f),
                _ => FactAttempt::Committed,
            }
        } else {
            self.pattern_facts
        };
        if result == FactAttempt::NoMatch {
            if !parser.state.resource_finalizing {
                node.abandon(parser);
            }
        } else {
            node.complete(parser, kind);
        }
        self.pattern_result(result);
    }
    fn pattern_required(&mut self, rule: RuleId, after: bool) {
        self.push(Frame::Required(Box::new(Required::new(
            rule,
            "syntax/missing-pattern-item",
            if after {
                "missing pattern item after separator"
            } else {
                "missing pattern item"
            },
            "pattern",
            &[],
            &[],
            None,
        ))));
    }
    fn pattern_closer(&mut self, rule: RuleId, array: bool) {
        let (close, kind, text) = if array {
            (rules::RIGHT_BRACKET, SyntaxKind::RightBracket, "]")
        } else {
            (rules::RIGHT_PARENTHESIS, SyntaxKind::RightParen, ")")
        };
        self.push(Frame::Closer(Box::new(Closer::new(
            rule, close, kind, text,
        ))));
    }
    #[inline(never)]
    pub(super) fn pattern_frame(&mut self, parser: &mut Parser<'_>, phase: Box<Phase>) {
        let phase = *phase;
        match phase {
            Phase::Enter(rule) => {
                let checkpoint = parser.checkpoint();
                parser.state.rules.push_canonical(rule);
                self.pattern(Phase::Exit(checkpoint));
                self.pattern_result(FactAttempt::NoMatch);
                self.array_token = None;
                if rule == rules::PATTERN_ARRAY_ITEM {
                    self.push(Frame::Call(rules::PATTERN));
                    return;
                }
                let node = parser.start();
                match rule {
                    rules::PATTERN => self.pattern(Phase::Choice(node, 0)),
                    rules::PATTERN_TUPLE_STRUCT | rules::PATTERN_ATOM_STRUCT => {
                        let atom = rule == rules::PATTERN_ATOM_STRUCT;
                        self.pattern(Phase::Prefix(
                            node,
                            List {
                                rule,
                                kind: if atom {
                                    SyntaxKind::AtomStructPattern
                                } else {
                                    SyntaxKind::TupleStructPattern
                                },
                            },
                        ));
                        self.base(if atom { rules::COLON } else { rules::GRAVE });
                    }
                    rules::PATTERN_TUPLE => {
                        self.pattern(Phase::Open(
                            node,
                            List {
                                rule,
                                kind: SyntaxKind::TuplePattern,
                            },
                        ));
                        self.base(rules::LEFT_PARENTHESIS);
                    }
                    rules::PATTERN_ARRAY => {
                        self.pattern(Phase::ArrayOpen(node));
                        self.base(rules::LEFT_BRACKET);
                    }
                    rules::PATTERN_ARRAY_TOKEN => {
                        self.pattern(Phase::TokenSpread(node));
                        self.push(Frame::Primitive(Box::new(primitives::Continuation::new(
                            rules::SPREAD_OPERATOR,
                        ))));
                    }
                    rules::FSM_VALUE => {
                        self.pattern(Phase::Value(node));
                        self.push(Frame::Call(rules::PATTERN));
                    }
                    _ => unreachable!("canonical pattern owner"),
                }
            }
            Phase::Exit(checkpoint) => {
                parser.state.rules.truncate(checkpoint.rule_depth);
                if self.pattern_facts == FactAttempt::NoMatch {
                    parser.rewind(checkpoint);
                }
                if parser.is_halted() {
                    self.pattern_result(match self.pattern_facts {
                        FactAttempt::Matched(f) | FactAttempt::Recovered(f) => {
                            FactAttempt::Recovered(f)
                        }
                        _ => FactAttempt::Committed,
                    });
                } else {
                    self.result = self.pattern_facts.attempt();
                }
            }
            Phase::Choice(node, index) => {
                self.pattern(Phase::Selected(node, index));
                if index == 2 {
                    self.push(Frame::Primitive(Box::new(primitives::Continuation::new(
                        rules::WILDCARD,
                    ))));
                } else {
                    self.push(Frame::Call(
                        [
                            rules::PATTERN_ATOM_STRUCT,
                            rules::PATTERN_TUPLE_STRUCT,
                            rules::WILDCARD,
                            rules::PATTERN_ARRAY,
                            rules::PATTERN_TUPLE,
                            rules::EXPRESSION,
                        ][index],
                    ));
                }
            }
            Phase::Selected(node, index) => {
                if index == 2 || index == 5 {
                    self.pattern_result(match self.result {
                        Attempt::Matched => FactAttempt::Matched(PatternFacts {
                            contains_wildcard: index == 2,
                            contains_array_spread_or_rest: false,
                        }),
                        Attempt::Committed => FactAttempt::Recovered(PatternFacts::default()),
                        Attempt::NoMatch => FactAttempt::NoMatch,
                    });
                }
                if self.pattern_facts == FactAttempt::NoMatch && index < 5 {
                    self.pattern(Phase::Choice(node, index + 1));
                } else {
                    self.pattern_finish(parser, node, SyntaxKind::Pattern);
                }
            }
            Phase::Prefix(node, spec) | Phase::Name(node, spec) | Phase::Open(node, spec) => {
                advance_pattern_prefix(self, parser, phase, node, spec)
            }
            Phase::Finish(node, spec) => self.pattern_finish(parser, node, spec.kind),
            Phase::ListSpace(spec) => {
                if self.result == Attempt::Matched {
                    self.pattern(Phase::ListItem(spec, PatternFacts::default(), false, false));
                    self.push(Frame::Call(rules::PATTERN));
                } else {
                    self.pattern_result(FactAttempt::NoMatch);
                }
            }
            Phase::ListItem(spec, mut facts, mut committed, after) => {
                match self.pattern_facts {
                    FactAttempt::Matched(f) => facts.merge(f),
                    FactAttempt::Recovered(f) => {
                        facts.merge(f);
                        committed = true;
                    }
                    FactAttempt::Committed => committed = true,
                    FactAttempt::NoMatch => {
                        self.pattern(Phase::ListRecovered(spec, facts));
                        self.pattern_required(spec.rule, after);
                        return;
                    }
                }
                self.pattern(Phase::ListLoop(spec, facts, committed));
            }
            Phase::ListRecovered(spec, facts) => self.pattern(Phase::ListLoop(spec, facts, true)),
            Phase::ListLoop(spec, facts, committed) => {
                if parser.is_halted() {
                    self.pattern(Phase::ListTrailing(spec, facts, committed));
                    self.base(rules::WHITESPACE0);
                } else {
                    self.pattern(Phase::ListSeparator(spec, facts, committed));
                    self.base(rules::LIST_SEPARATOR);
                }
            }
            Phase::ListSeparator(spec, facts, committed) => {
                if self.result == Attempt::Matched {
                    self.pattern(Phase::ListItem(spec, facts, committed, true));
                    self.push(Frame::Call(rules::PATTERN));
                } else {
                    self.pattern(Phase::ListTrailing(spec, facts, committed));
                    self.base(rules::WHITESPACE0);
                }
            }
            Phase::ListTrailing(spec, facts, committed) => {
                self.pattern(Phase::ListClose(spec, facts, committed));
                self.base(rules::RIGHT_PARENTHESIS);
            }
            Phase::ListClose(spec, facts, committed) => {
                if self.result == Attempt::Matched {
                    self.pattern_result(if committed {
                        FactAttempt::Recovered(facts)
                    } else {
                        FactAttempt::Matched(facts)
                    });
                } else {
                    self.pattern(Phase::ListClosed(facts));
                    self.pattern_closer(spec.rule, false);
                }
            }
            Phase::ListClosed(facts) => self.pattern_result(FactAttempt::Recovered(facts)),
            Phase::ArrayOpen(node) => {
                self.pattern(Phase::Finish(
                    node,
                    List {
                        rule: rules::PATTERN_ARRAY,
                        kind: SyntaxKind::ArrayPattern,
                    },
                ));
                if self.result != Attempt::Matched {
                    self.pattern_result(FactAttempt::NoMatch);
                } else if parser.push_nesting() {
                    self.push(Frame::PopNesting);
                    self.pattern(Phase::ArraySpace);
                    self.base(rules::WHITESPACE0);
                } else {
                    self.pattern_result(FactAttempt::Committed);
                    self.push(Frame::Nesting(Box::new(NestingContinuation::new())));
                }
            }
            Phase::ArraySpace => {
                if self.result == Attempt::Matched {
                    self.pattern(Phase::ArrayLoop(ArrayFacts::default(), false));
                } else {
                    self.pattern_result(FactAttempt::NoMatch);
                }
            }
            Phase::ArrayLoop(facts, committed) => {
                if parser.is_halted() {
                    let result = facts.finish();
                    self.pattern_result(match result {
                        FactAttempt::Matched(f) => FactAttempt::Recovered(f),
                        r => r,
                    });
                } else {
                    self.pattern(Phase::ArrayClose(facts, committed));
                    self.base(rules::RIGHT_BRACKET);
                }
            }
            Phase::ArrayClose(facts, committed) => {
                if self.result == Attempt::Matched {
                    let result = facts.finish();
                    self.pattern_result(match result {
                        FactAttempt::Matched(f) if committed => FactAttempt::Recovered(f),
                        r => r,
                    });
                } else {
                    self.pattern(Phase::ArrayItem(facts, committed, parser.offset()));
                    self.push(Frame::Call(rules::PATTERN_ARRAY_TOKEN));
                }
            }
            Phase::ArrayItem(mut facts, mut committed, before) => {
                match self.pattern_facts {
                    FactAttempt::Matched(_) => {
                        if parser.offset() == before {
                            self.pattern_result(FactAttempt::NoMatch);
                            return;
                        }
                        facts.add(self.array_token.expect("matched array token"));
                    }
                    FactAttempt::Recovered(_) => {
                        facts.add(self.array_token.expect("recovered array token"));
                        committed = true;
                        if parser.offset() == before {
                            self.pattern_result(match facts.finish() {
                                FactAttempt::Matched(f) => FactAttempt::Recovered(f),
                                r => r,
                            });
                            return;
                        }
                    }
                    FactAttempt::NoMatch if parser.cursor().starts_with(",") => {
                        let element = parser.start();
                        self.pattern(Phase::ArrayMissing(element, facts));
                        self.push(Frame::Required(Box::new(Required::new(
                            rules::PATTERN_ARRAY,
                            "syntax/missing-pattern-array-item",
                            "missing array pattern before separator",
                            "pattern-array-token",
                            &[],
                            &[],
                            None,
                        ))));
                        return;
                    }
                    FactAttempt::Committed if parser.offset() > before => committed = true,
                    _ => {
                        self.pattern(Phase::ArrayRecovered(facts));
                        self.pattern_closer(rules::PATTERN_ARRAY, true);
                        return;
                    }
                }
                self.pattern(Phase::ArrayAfter(facts, committed));
            }
            Phase::ArrayMissing(node, facts) => {
                node.complete(parser, SyntaxKind::ArrayPatternElement);
                self.pattern(Phase::ArrayAfter(facts, true));
            }
            Phase::ArrayAfter(facts, committed) => {
                if parser.is_halted() {
                    self.pattern(Phase::ArrayLoop(facts, committed));
                } else {
                    self.pattern(Phase::ArraySeparator(facts, committed));
                    self.base(rules::WHITESPACE0);
                }
            }
            Phase::ArraySeparator(facts, committed) => {
                if self.result == Attempt::Matched {
                    self.pattern(Phase::ArrayLoop(facts, committed));
                    self.base(rules::LIST_SEPARATOR);
                } else {
                    self.pattern_result(FactAttempt::NoMatch);
                }
            }
            Phase::ArrayRecovered(facts) => self.pattern_result(match facts.finish() {
                FactAttempt::Matched(f) => FactAttempt::Recovered(f),
                r => r,
            }),
            Phase::TokenSpread(node) => {
                if self.result == Attempt::Matched {
                    self.array_token = Some(ArrayToken::Spread);
                    self.pattern_result(FactAttempt::Matched(PatternFacts::default()));
                    self.pattern_finish(parser, node, SyntaxKind::ArrayPatternElement);
                } else {
                    self.pattern(Phase::TokenRest(node));
                    self.base(rules::ENUM_SEPARATOR);
                }
            }
            Phase::TokenRest(node) => {
                if self.result == Attempt::Matched {
                    self.array_token = Some(ArrayToken::Rest);
                    self.pattern_result(FactAttempt::Matched(PatternFacts::default()));
                    self.pattern_finish(parser, node, SyntaxKind::ArrayPatternElement);
                } else {
                    self.pattern(Phase::TokenItem(node));
                    self.push(Frame::Call(rules::PATTERN));
                }
            }
            Phase::TokenItem(node) => {
                self.array_token = match self.pattern_facts {
                    FactAttempt::Matched(f) | FactAttempt::Recovered(f) => {
                        Some(ArrayToken::Item(f))
                    }
                    _ => None,
                };
                self.pattern_finish(parser, node, SyntaxKind::ArrayPatternElement);
            }
            Phase::Value(node) => {
                let allowed = match self.pattern_facts {
                    FactAttempt::Matched(f) | FactAttempt::Recovered(f) => {
                        !f.contains_wildcard && !f.contains_array_spread_or_rest
                    }
                    FactAttempt::Committed => true,
                    FactAttempt::NoMatch => false,
                };
                if !allowed
                    && !(parser.is_halted()
                        && matches!(self.pattern_facts, FactAttempt::Recovered(_)))
                {
                    self.pattern_result(FactAttempt::NoMatch);
                }
                self.pattern_finish(parser, node, SyntaxKind::FsmValue);
            }
        }
    }
}
// Outlined prefix dispatch keeps recursive expression stack frames small.
#[inline(never)]
fn advance_pattern_prefix(
    owner: &mut Continuation,
    parser: &mut Parser<'_>,
    phase: Phase,
    node: Marker,
    spec: List,
) {
    if owner.result != Attempt::Matched {
        owner.pattern_result(FactAttempt::NoMatch);
        owner.pattern_finish(parser, node, spec.kind);
        return;
    }
    match phase {
        Phase::Prefix(..) => {
            owner.pattern(Phase::Name(node, spec));
            owner.base(rules::IDENTIFIER);
        }
        Phase::Name(..) => {
            owner.pattern(Phase::Open(node, spec));
            owner.base(rules::LEFT_PARENTHESIS);
        }
        Phase::Open(..) => {
            owner.pattern(Phase::Finish(node, spec));
            if parser.push_nesting() {
                owner.push(Frame::PopNesting);
                owner.pattern(Phase::ListSpace(spec));
                owner.base(rules::WHITESPACE0);
            } else {
                owner.pattern_result(FactAttempt::Committed);
                owner.push(Frame::Nesting(Box::new(NestingContinuation::new())));
            }
        }
        _ => unreachable!("pattern prefix"),
    }
}
