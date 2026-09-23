//! Owned precedence, atom parents, and seeded formula completion. Recursive
//! structure selection, match, and FSM-value children remain under B3 conversion.
use super::super::super::{literals as leaf_literals, paths, primitives, strings};
mod brace;
mod bracket;
mod collection;
mod comprehension;
mod definition;
mod entry;
mod expression;
mod fsm;
mod inline;
mod map;
mod mapping_probe;
mod match_arm;
mod matrix_row;
mod parenthesis;
mod pattern;
mod postfix;
mod record;
mod table;
mod table_row;
use super::super::{ExpressionForm, FactAttempt};
use super::super::{
    kinds,
    required::{Closer, Required},
};
use super::*;
use crate::document::TextSize;
use crate::document::parser::recovery::{NestingContinuation, NestingProgress};
use alloc::{boxed::Box, vec::Vec};

pub(crate) enum Progress {
    Complete(Attempt),
    NeedInput,
    NeedsProcessing,
    Limited,
}
#[derive(Clone, Copy)]
struct LevelSpec {
    rule: RuleId,
    kind: SyntaxKind,
    operators: &'static [RuleId],
}
const LEVELS: [LevelSpec; 7] = [
    LevelSpec {
        rule: rules::L1,
        kind: SyntaxKind::LogicExpression,
        operators: &[rules::LOGIC_OPERATOR],
    },
    LevelSpec {
        rule: rules::L2,
        kind: SyntaxKind::ComparisonExpression,
        operators: &[rules::COMPARISON_OPERATOR],
    },
    LevelSpec {
        rule: rules::L3,
        kind: SyntaxKind::AdditiveExpression,
        operators: &[rules::ADD_SUB_OPERATOR],
    },
    LevelSpec {
        rule: rules::L4,
        kind: SyntaxKind::MultiplicativeExpression,
        operators: &[rules::MUL_DIV_OPERATOR, rules::MATRIX_OPERATOR],
    },
    LevelSpec {
        rule: rules::L5,
        kind: SyntaxKind::PowerExpression,
        operators: &[rules::POWER_OPERATOR],
    },
    LevelSpec {
        rule: rules::L6,
        kind: SyntaxKind::TableExpression,
        operators: &[rules::TABLE_OPERATOR],
    },
    LevelSpec {
        rule: rules::L7,
        kind: SyntaxKind::SetExpression,
        operators: &[rules::SET_OPERATOR],
    },
];
pub(crate) fn supports(rule: RuleId) -> bool {
    matches!(
        rule,
        rules::EXPRESSION
            | rules::VARIABLE_DEFINE
            | rules::MATCH_ARM
            | rules::BINDING
            | rules::MAPPING
            | rules::MAP
            | rules::INLINE_TABLE_ROW
            | rules::MATRIX_COLUMN
            | rules::MATRIX_ROW
            | rules::MATRIX
            | rules::MATRIX_COMPREHENSION
    ) || fsm::supports(rule)
        || pattern::supports(rule)
        || comprehension::supports(rule)
        || collection::supports(rule)
        || table_row::supports(rule)
        || postfix::supports(rule)
        || structures::continuation_supports(rule)
        || kinds::continuation_supports(rule)
        || LEVELS.iter().any(|spec| spec.rule == rule)
        || matches!(
            rule,
            rules::FORMULA
                | rules::LITERAL
                | rules::VAR
                | rules::KIND_ANNOTATION
                | rules::FACTOR
                | rules::NEGATE_FACTOR
                | rules::NOT_FACTOR
                | rules::RANGE_EXPRESSION
                | rules::PARENTHETICAL_TERM
        )
}
#[derive(Clone, Copy)]
struct Level {
    index: usize,
    marker: Marker,
    pairs: u32,
    committed: bool,
}
enum Frame {
    Brace(Box<brace::Phase>),
    Bracket(Box<bracket::Phase>),
    Inline(Box<inline::Phase>),
    Table(Box<table::Phase>),
    MatrixRow(Box<matrix_row::Phase>),
    TableRow(Box<table_row::Phase>),
    Missing(Box<crate::document::parser::recovery::MissingContinuation<'static>>),
    Record(Box<record::Phase>),
    Shell(Box<super::super::super::structure_shell::Continuation>),
    MappingProbe(Box<mapping_probe::Phase>),
    Map(Box<map::Phase>),
    Entry(Box<entry::Phase>),
    Parenthesis(Box<parenthesis::Phase>),
    Collection(Box<collection::Phase>),
    CollectionWrap(Marker, ParserCheckpoint),
    Comprehension(Box<comprehension::Phase>),
    MatchArm(Box<match_arm::Phase>),
    Pattern(Box<pattern::Phase>),
    Definition(Box<definition::Phase>),
    Fsm(Box<fsm::Phase>),
    Expression(Box<expression::Phase>),
    Postfix(Box<postfix::Phase>),
    Primitive(Box<primitives::Continuation>),
    Closer(Box<Closer<'static>>),
    Range(bool),
    RangeFirst(Marker, ParserCheckpoint, bool),
    RangeOperator(Marker, ParserCheckpoint, bool, bool),
    RangeMiddle(Marker, bool, RuleId),
    RangeRecovered(Marker, RuleId),
    RangeSecond(Marker, bool, RuleId),
    RangeLast(Marker, bool, RuleId),
    RangeEnd(Marker, bool),
    Paren,
    ParenOpen(Marker),
    ParenSpace,
    ParenFormula,
    ParenRecovered,
    ParenTrailing(bool),
    ParenClose(bool),
    ParenFinish(Marker),
    Nesting(Box<NestingContinuation>),
    Structure(Box<structures::StructureContinuation>),
    Call(RuleId),
    FactorBody,
    FactorNegated,
    FactorNot,
    FactorTable,
    FactorLiteral,
    FactorStem(ParserCheckpoint),
    FactorSelection(ParserCheckpoint, bool),
    FactorValue(Marker),
    FactorSuffix(Marker, bool),
    Unary(bool),
    UnaryOperator(Marker, bool),
    UnaryChild(Marker, bool),
    UnaryMissing(Marker, bool),
    PopNesting,
    LeafBase(Box<base::continuation::Continuation>),
    LeafLiteral(Box<leaf_literals::Continuation>),
    LeafString(Box<strings::Continuation>),
    LeafPath(Box<paths::Continuation>),
    LiteralChoice(Marker, usize),
    LiteralSelected(Marker),
    LiteralSuffix(Marker),
    Variable(bool),
    VariablePath(Marker, bool),
    VariableStem(Marker, bool),
    VariableCandidate(Marker),
    VariableComparison(Marker, ParserCheckpoint),
    VariableSuffix(Marker),
    Annotation(bool),
    LeafKind(Box<kinds::KindContinuation>),
    Enter(usize),
    Exit(ParserCheckpoint),
    First(Level),
    Loop(Level),
    OperatorResult(Level, TextSize),
    OperandResult(Level, TextSize),
    Recovered(Level, TextSize),
    Operator(usize, usize),
    OperatorNext(usize, usize),
    LeafOperator(Box<operators::Continuation>),
    Factor,
    RecoverOperand(usize),
    Required(Box<Required<'static>>),
    UnaryRecovered(Marker, bool),
    SeedTranspose(FormulaSeed, bool),
    SeedLevel(FormulaSeed, usize, bool),
    SeedResult(FormulaSeed, usize, bool),
}
pub(crate) struct Continuation {
    frames: Vec<Frame>,
    result: Attempt,
    facts: FactAttempt<ExpressionForm>,
    definition_operator: bool,
    pattern_facts: FactAttempt<super::super::PatternFacts>,
    array_token: Option<pattern::ArrayToken>,
    qualifier_kind: Option<super::super::QualifierKind>,
    binding_candidate: Option<structures::BindingCandidate>,
    bracket_facts: FactAttempt<super::super::BracketForm>,
    pub work: u64,
}
impl Continuation {
    pub fn new(rule: RuleId) -> Self {
        assert!(supports(rule), "canonical recursive owner");
        Self {
            frames: alloc::vec![Frame::Call(rule)],
            result: Attempt::NoMatch,
            facts: FactAttempt::NoMatch,
            definition_operator: false,
            pattern_facts: FactAttempt::NoMatch,
            array_token: None,
            qualifier_kind: None,
            binding_candidate: None,
            bracket_facts: FactAttempt::NoMatch,
            work: 0,
        }
    }
    pub(crate) fn annotation(recover: bool) -> Self {
        Self {
            frames: alloc::vec![Frame::Annotation(recover)],
            result: Attempt::NoMatch,
            facts: FactAttempt::NoMatch,
            definition_operator: false,
            pattern_facts: FactAttempt::NoMatch,
            array_token: None,
            qualifier_kind: None,
            binding_candidate: None,
            bracket_facts: FactAttempt::NoMatch,
            work: 0,
        }
    }
    fn base(&mut self, rule: RuleId) {
        self.push(Frame::LeafBase(Box::new(
            base::continuation::Continuation::new(rule),
        )));
    }
    fn transaction(&mut self, parser: &mut Parser<'_>, rule: RuleId) {
        let checkpoint = parser.checkpoint();
        parser.state.rules.push_canonical(rule);
        self.push(Frame::Exit(checkpoint));
    }
    fn literal_choice(&mut self, node: Marker, index: usize) {
        self.push(Frame::LiteralChoice(node, index));
        match index {
            0 => self.push(Frame::LeafLiteral(Box::new(
                leaf_literals::Continuation::new(rules::NUMBER),
            ))),
            1 => self.push(Frame::LeafString(Box::new(strings::Continuation::new(
                rules::STRING,
            )))),
            2 => self.push(Frame::LeafLiteral(Box::new(
                leaf_literals::Continuation::new(rules::ATOM),
            ))),
            3 => self.push(Frame::LeafLiteral(Box::new(
                leaf_literals::Continuation::new(rules::BOOLEAN),
            ))),
            4 => self.push(Frame::LeafLiteral(Box::new(
                leaf_literals::Continuation::new(rules::EMPTY),
            ))),
            5 => self.push(Frame::Annotation(true)),
            _ => unreachable!("canonical literal alternatives"),
        }
    }
    fn range_second(&mut self, node: Marker, committed: bool, target: RuleId) {
        self.push(Frame::RangeSecond(node, committed, target));
        self.push(Frame::LeafOperator(Box::new(operators::Continuation::new(
            rules::RANGE_OPERATOR,
        ))));
    }
    pub(in super::super) fn operator(index: usize) -> Self {
        Self {
            frames: alloc::vec![Frame::Operator(index, 0)],
            result: Attempt::NoMatch,
            facts: FactAttempt::NoMatch,
            definition_operator: false,
            pattern_facts: FactAttempt::NoMatch,
            array_token: None,
            qualifier_kind: None,
            binding_candidate: None,
            bracket_facts: FactAttempt::NoMatch,
            work: 0,
        }
    }
    pub(in super::super) fn drive(&mut self, parser: &mut Parser<'_>) -> Attempt {
        loop {
            let mut allowance = u64::MAX;
            match self.advance(parser, true, &mut allowance) {
                Progress::Complete(result) => return result,
                Progress::NeedsProcessing => {}
                _ => unreachable!("sealed precedence input"),
            }
        }
    }
    fn push(&mut self, frame: Frame) {
        self.frames.push(frame);
    }
    fn operand(&mut self, index: usize) {
        self.push(if index + 1 < LEVELS.len() {
            Frame::Enter(index + 1)
        } else {
            Frame::Factor
        });
    }
    fn finish_level(&mut self, parser: &mut Parser<'_>, level: Level) {
        self.result = if level.committed || parser.is_halted() {
            level.marker.complete(parser, LEVELS[level.index].kind);
            Attempt::Committed
        } else {
            if level.pairs == 0 {
                level.marker.abandon(parser);
            } else {
                level.marker.complete(parser, LEVELS[level.index].kind);
            }
            Attempt::Matched
        };
    }
    // Keep the retained range transitions off unrelated recursive call paths.
    #[inline(never)]
    fn range_frame(&mut self, parser: &mut Parser<'_>, frame: Frame) {
        match frame {
            Frame::Range(require_range) => {
                let checkpoint = parser.checkpoint();
                let node = parser.start();
                self.push(Frame::RangeFirst(node, checkpoint, require_range));
                self.push(Frame::Call(rules::FORMULA));
            }
            Frame::RangeFirst(node, checkpoint, require_range) => match self.result {
                Attempt::NoMatch => parser.rewind(checkpoint),
                Attempt::Committed if parser.is_halted() => {
                    expressions::finish_provisional_formula_marker(parser, node)
                }
                result => {
                    self.push(Frame::RangeOperator(
                        node,
                        checkpoint,
                        require_range,
                        result == Attempt::Committed,
                    ));
                    self.push(Frame::LeafOperator(Box::new(operators::Continuation::new(
                        rules::RANGE_OPERATOR,
                    ))));
                }
            },
            Frame::RangeOperator(node, checkpoint, require_range, committed) => match self.result {
                Attempt::NoMatch => {
                    node.abandon(parser);
                    if require_range {
                        parser.rewind(checkpoint);
                    } else {
                        self.result = if committed {
                            Attempt::Committed
                        } else {
                            Attempt::Matched
                        };
                    }
                }
                Attempt::Committed => expressions::finish_provisional_formula_marker(parser, node),
                Attempt::Matched => {
                    self.push(Frame::RangeMiddle(
                        node,
                        committed,
                        parser.current_rule().unwrap_or(rules::RANGE_EXPRESSION),
                    ));
                    self.push(Frame::Call(rules::FORMULA));
                }
            },
            Frame::RangeMiddle(node, committed, target) => match self.result {
                Attempt::NoMatch => {
                    self.push(Frame::RangeRecovered(node, target));
                    self.push(Frame::Required(Box::new(Required::new(
                        target,
                        "syntax/missing-range-bound",
                        "missing range bound after range operator",
                        "formula",
                        &[],
                        &[".."],
                        None,
                    ))));
                }
                Attempt::Committed if parser.is_halted() => {
                    node.complete(parser, SyntaxKind::RangeExpression);
                }
                result => {
                    self.range_second(node, committed || result == Attempt::Committed, target)
                }
            },
            Frame::RangeRecovered(node, target) => self.range_second(node, true, target),
            Frame::RangeSecond(node, committed, target) => match self.result {
                Attempt::Matched => {
                    self.push(Frame::RangeLast(node, committed, target));
                    self.push(Frame::Call(rules::FORMULA));
                }
                result => self.push(Frame::RangeEnd(
                    node,
                    committed || result == Attempt::Committed,
                )),
            },
            Frame::RangeLast(node, committed, target) => {
                if self.result == Attempt::NoMatch {
                    self.push(Frame::RangeEnd(node, true));
                    self.push(Frame::Required(Box::new(Required::new(
                        target,
                        "syntax/missing-range-bound",
                        "missing final range bound after range operator",
                        "formula",
                        &[],
                        &[],
                        None,
                    ))));
                } else {
                    self.push(Frame::RangeEnd(
                        node,
                        committed || self.result == Attempt::Committed,
                    ));
                }
            }
            Frame::RangeEnd(node, committed) => {
                node.complete(parser, SyntaxKind::RangeExpression);
                self.result = if committed {
                    Attempt::Committed
                } else {
                    Attempt::Matched
                };
            }
            Frame::Paren => {
                self.transaction(parser, rules::PARENTHETICAL_TERM);
                let node = parser.start();
                self.push(Frame::ParenOpen(node));
                self.base(rules::LEFT_PARENTHESIS);
            }
            Frame::ParenOpen(node) => {
                if self.result == Attempt::NoMatch {
                    node.abandon(parser);
                } else {
                    self.push(Frame::ParenFinish(node));
                    if parser.push_nesting() {
                        self.push(Frame::PopNesting);
                        self.push(Frame::ParenSpace);
                        self.base(rules::SPACE_TAB0);
                    } else {
                        self.push(Frame::Nesting(Box::new(NestingContinuation::new())));
                    }
                }
            }
            Frame::ParenSpace => {
                if self.result == Attempt::Matched {
                    self.push(Frame::ParenFormula);
                    self.push(Frame::Call(rules::FORMULA));
                }
            }
            Frame::ParenFormula => {
                if self.result == Attempt::NoMatch {
                    self.push(Frame::ParenRecovered);
                    self.push(Frame::Required(Box::new(Required::new(
                        rules::PARENTHETICAL_TERM,
                        "syntax/missing-parenthetical-expression",
                        "missing expression after opening parenthesis",
                        "formula",
                        &[],
                        &[],
                        None,
                    ))));
                } else {
                    self.push(Frame::ParenTrailing(self.result == Attempt::Committed));
                    self.base(rules::SPACE_TAB0);
                }
            }
            Frame::ParenRecovered => {
                self.push(Frame::ParenTrailing(true));
                self.base(rules::SPACE_TAB0);
            }
            Frame::ParenTrailing(committed) => {
                self.push(Frame::ParenClose(committed));
                self.base(rules::RIGHT_PARENTHESIS);
            }
            Frame::ParenClose(committed) => {
                if self.result == Attempt::NoMatch {
                    self.push(Frame::Closer(Box::new(Closer::new(
                        rules::PARENTHETICAL_TERM,
                        rules::RIGHT_PARENTHESIS,
                        SyntaxKind::RightParen,
                        ")",
                    ))));
                } else {
                    self.result = if committed {
                        Attempt::Committed
                    } else {
                        Attempt::Matched
                    };
                }
            }
            Frame::ParenFinish(node) => {
                self.result = finish(
                    node,
                    parser,
                    SyntaxKind::ParentheticalExpression,
                    self.result,
                )
            }
            _ => unreachable!("range or parenthetical frame"),
        }
    }
    pub fn advance(
        &mut self,
        parser: &mut Parser<'_>,
        final_input: bool,
        allowance: &mut u64,
    ) -> Progress {
        while !self.frames.is_empty() || parser.state.tree_cache.pending() {
            if !final_input && parser.is_halted() {
                return Progress::Limited;
            }
            if parser.state.tree_cache.pending() {
                let before = *allowance;
                let complete = parser.advance_tree_cache(allowance);
                self.work += before - *allowance;
                if !complete {
                    return Progress::NeedsProcessing;
                }
                if self.frames.is_empty() {
                    break;
                }
            }
            if *allowance == 0 {
                return Progress::NeedsProcessing;
            }
            let frame = self.frames.pop().expect("retained precedence phase");
            if !matches!(
                frame,
                Frame::Missing(_)
                    | Frame::Shell(_)
                    | Frame::Primitive(_)
                    | Frame::Closer(_)
                    | Frame::Structure(_)
                    | Frame::Nesting(_)
                    | Frame::LeafKind(_)
                    | Frame::Required(_)
                    | Frame::LeafOperator(_)
                    | Frame::LeafBase(_)
                    | Frame::LeafLiteral(_)
                    | Frame::LeafString(_)
                    | Frame::LeafPath(_)
            ) {
                *allowance -= 1;
                self.work += 1;
            }
            match frame {
                Frame::LeafBase(mut child) => {
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
                            self.push(Frame::LeafBase(child));
                            return Progress::NeedInput;
                        }
                        base::continuation::Progress::NeedsProcessing => {
                            self.push(Frame::LeafBase(child));
                            return Progress::NeedsProcessing;
                        }
                        base::continuation::Progress::Limited => {
                            self.push(Frame::LeafBase(child));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::LeafLiteral(mut child) => {
                    let before = *allowance;
                    let progress = child.advance(parser, final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        leaf_literals::Progress::Complete(result) => self.result = result,
                        leaf_literals::Progress::NeedInput => {
                            self.push(Frame::LeafLiteral(child));
                            return Progress::NeedInput;
                        }
                        leaf_literals::Progress::NeedsProcessing => {
                            self.push(Frame::LeafLiteral(child));
                            return Progress::NeedsProcessing;
                        }
                        leaf_literals::Progress::Limited => {
                            self.push(Frame::LeafLiteral(child));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::LeafString(mut child) => {
                    let before = *allowance;
                    let progress = child.advance(parser, final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        strings::Progress::Complete(result) => self.result = result,
                        strings::Progress::NeedInput => {
                            self.push(Frame::LeafString(child));
                            return Progress::NeedInput;
                        }
                        strings::Progress::NeedsProcessing => {
                            self.push(Frame::LeafString(child));
                            return Progress::NeedsProcessing;
                        }
                        strings::Progress::Limited => {
                            self.push(Frame::LeafString(child));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::LeafPath(mut child) => {
                    let before = *allowance;
                    let progress = child.advance(parser, final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        paths::Progress::Complete(result) => self.result = result,
                        paths::Progress::NeedInput => {
                            self.push(Frame::LeafPath(child));
                            return Progress::NeedInput;
                        }
                        paths::Progress::NeedsProcessing => {
                            self.push(Frame::LeafPath(child));
                            return Progress::NeedsProcessing;
                        }
                        paths::Progress::Limited => {
                            self.push(Frame::LeafPath(child));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::Call(rule) => {
                    if let Some(index) = LEVELS.iter().position(|spec| spec.rule == rule) {
                        self.push(Frame::Enter(index));
                    } else if rule == rules::MAP {
                        self.push(Frame::Map(Box::new(map::Phase::Enter)));
                    } else if matches!(rule, rules::BINDING | rules::MAPPING) {
                        self.push(Frame::Entry(Box::new(entry::Phase::Enter(
                            rule == rules::BINDING,
                            None,
                        ))));
                    } else if matches!(rule, rules::MATRIX | rules::MATRIX_COMPREHENSION) {
                        self.push(Frame::Bracket(Box::new(bracket::Phase::Enter(
                            if rule == rules::MATRIX {
                                structures::BracketMode::MatrixOnly
                            } else {
                                structures::BracketMode::ComprehensionOnly
                            },
                        ))));
                    } else if rule == rules::INLINE_TABLE_ROW {
                        self.push(Frame::Inline(Box::new(inline::Phase::Enter)));
                    } else if matches!(rule, rules::MATRIX_COLUMN | rules::MATRIX_ROW) {
                        self.push(Frame::MatrixRow(Box::new(matrix_row::Phase::Enter(rule))));
                    } else if table_row::supports(rule) {
                        self.push(Frame::TableRow(Box::new(table_row::Phase::Enter(rule))));
                    } else if collection::supports(rule) {
                        self.push(Frame::Collection(Box::new(collection::Phase::Enter(rule))));
                    } else if comprehension::supports(rule) {
                        self.push(Frame::Comprehension(Box::new(comprehension::Phase::Enter(
                            rule,
                        ))));
                    } else if pattern::supports(rule) {
                        self.push(Frame::Pattern(Box::new(pattern::Phase::Enter(rule))));
                    } else if rule == rules::MATCH_ARM {
                        self.push(Frame::MatchArm(Box::new(match_arm::Phase::Enter)));
                    } else if rule == rules::VARIABLE_DEFINE {
                        self.push(Frame::Definition(Box::new(definition::Phase::Start)));
                    } else if fsm::supports(rule) {
                        self.push(Frame::Fsm(Box::new(fsm::Phase::Enter(rule))));
                    } else if rule == rules::EXPRESSION {
                        self.push(Frame::Expression(Box::new(expression::Phase::Enter)));
                    } else if postfix::supports(rule) {
                        self.push(Frame::Postfix(Box::new(postfix::Phase::Enter(rule))));
                    } else if structures::continuation_supports(rule) {
                        self.push(Frame::Structure(Box::new(
                            structures::StructureContinuation::new(rule),
                        )));
                    } else if kinds::continuation_supports(rule) {
                        self.push(Frame::LeafKind(Box::new(kinds::KindContinuation::new(
                            rule,
                        ))));
                    } else {
                        match rule {
                            rules::FORMULA => {
                                self.transaction(parser, rule);
                                self.push(Frame::Enter(0));
                            }
                            rules::LITERAL => {
                                self.transaction(parser, rule);
                                let node = parser.start();
                                self.literal_choice(node, 0);
                            }
                            rules::RANGE_EXPRESSION => {
                                self.transaction(parser, rule);
                                self.push(Frame::Range(true));
                            }
                            rules::PARENTHETICAL_TERM => self.push(Frame::Paren),
                            rules::FACTOR => self.push(Frame::Factor),
                            rules::NEGATE_FACTOR => self.push(Frame::Unary(false)),
                            rules::NOT_FACTOR => self.push(Frame::Unary(true)),
                            rules::VAR => self.push(Frame::Variable(false)),
                            rules::KIND_ANNOTATION => self.push(Frame::Annotation(true)),
                            _ => unreachable!("canonical recursive rule"),
                        }
                    }
                }
                Frame::LiteralChoice(node, index) => {
                    if self.result == Attempt::NoMatch && index < 5 {
                        self.literal_choice(node, index + 1);
                    } else {
                        self.push(Frame::LiteralSelected(node));
                    }
                }
                Frame::LiteralSelected(node) => match self.result {
                    Attempt::NoMatch => node.abandon(parser),
                    Attempt::Committed => {
                        node.complete(parser, SyntaxKind::Literal);
                    }
                    Attempt::Matched => {
                        self.push(Frame::LiteralSuffix(node));
                        self.push(Frame::Annotation(true));
                    }
                },
                Frame::LiteralSuffix(node) => {
                    node.complete(parser, SyntaxKind::Literal);
                    if self.result != Attempt::Committed {
                        self.result = Attempt::Matched;
                    }
                }
                Frame::Variable(allow_comparison) => {
                    self.transaction(parser, rules::VAR);
                    let node = parser.start();
                    self.push(Frame::VariablePath(node, allow_comparison));
                    self.push(Frame::LeafPath(Box::new(paths::Continuation::new(
                        rules::PREFIXED_CONTEXT_PATH,
                    ))));
                }
                Frame::VariablePath(node, allow_comparison) => {
                    self.push(Frame::VariableStem(node, allow_comparison));
                    if self.result == Attempt::NoMatch {
                        self.base(rules::IDENTIFIER);
                    }
                }
                Frame::VariableStem(node, allow_comparison) => {
                    if self.result == Attempt::NoMatch {
                        node.abandon(parser);
                    } else if allow_comparison {
                        self.push(Frame::VariableCandidate(node));
                        self.push(Frame::Annotation(false));
                    } else {
                        self.push(Frame::VariableSuffix(node));
                        self.push(Frame::Annotation(true));
                    }
                }
                Frame::VariableCandidate(node) => {
                    if self.result != Attempt::NoMatch {
                        node.complete(parser, SyntaxKind::Variable);
                    } else {
                        self.push(Frame::VariableComparison(node, parser.checkpoint()));
                        self.push(Frame::LeafOperator(Box::new(operators::Continuation::new(
                            rules::COMPARISON_OPERATOR,
                        ))));
                    }
                }
                Frame::VariableComparison(node, suffix) => {
                    let comparison = self.result == Attempt::Matched;
                    parser.rewind(suffix);
                    if comparison && !parser.is_halted() {
                        node.complete(parser, SyntaxKind::Variable);
                        self.result = Attempt::Matched;
                    } else {
                        self.push(Frame::VariableSuffix(node));
                        self.push(Frame::Annotation(true));
                    }
                }
                Frame::VariableSuffix(node) => {
                    node.complete(parser, SyntaxKind::Variable);
                    if self.result != Attempt::Committed {
                        self.result = Attempt::Matched;
                    }
                }
                Frame::Annotation(recover) => self.push(Frame::LeafKind(Box::new(
                    kinds::KindContinuation::annotation(recover),
                ))),
                Frame::LeafKind(mut child) => {
                    let before = *allowance;
                    let progress = child.advance(parser, final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        Progress::Complete(result) => self.result = result,
                        Progress::NeedInput => {
                            self.push(Frame::LeafKind(child));
                            return Progress::NeedInput;
                        }
                        Progress::NeedsProcessing => {
                            self.push(Frame::LeafKind(child));
                            return Progress::NeedsProcessing;
                        }
                        Progress::Limited => {
                            self.push(Frame::LeafKind(child));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::Enter(index) => {
                    let checkpoint = parser.checkpoint();
                    parser.state.rules.push_canonical(LEVELS[index].rule);
                    let level = Level {
                        index,
                        marker: parser.start(),
                        pairs: 0,
                        committed: false,
                    };
                    self.push(Frame::Exit(checkpoint));
                    self.push(Frame::First(level));
                    self.operand(index);
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
                Frame::First(mut level) => {
                    if self.result != Attempt::NoMatch {
                        level.committed = self.result == Attempt::Committed;
                        self.push(Frame::Loop(level));
                    }
                }
                Frame::Loop(level) => {
                    if parser.is_halted() {
                        self.finish_level(parser, level);
                    } else {
                        self.push(Frame::OperatorResult(level, parser.offset()));
                        self.push(Frame::Operator(level.index, 0));
                    }
                }
                Frame::OperatorResult(mut level, before) => match self.result {
                    Attempt::NoMatch => self.finish_level(parser, level),
                    Attempt::Committed => {
                        level.committed = true;
                        self.finish_level(parser, level);
                    }
                    Attempt::Matched => {
                        level.pairs += 1;
                        self.push(Frame::OperandResult(level, before));
                        self.operand(level.index);
                    }
                },
                Frame::OperandResult(mut level, before) => match self.result {
                    Attempt::Matched if parser.offset() <= before => self.result = Attempt::NoMatch,
                    Attempt::NoMatch => {
                        level.committed = true;
                        self.push(Frame::Recovered(level, before));
                        self.push(Frame::RecoverOperand(level.index));
                    }
                    result => {
                        level.committed |= result == Attempt::Committed;
                        if parser.offset() <= before {
                            self.finish_level(parser, level);
                        } else {
                            self.push(Frame::Loop(level));
                        }
                    }
                },
                Frame::Recovered(level, before) => {
                    if parser.offset() <= before {
                        self.finish_level(parser, level);
                    } else {
                        self.push(Frame::Loop(level));
                    }
                }
                Frame::Operator(index, choice) => {
                    self.push(Frame::OperatorNext(index, choice));
                    self.push(Frame::LeafOperator(Box::new(operators::Continuation::new(
                        LEVELS[index].operators[choice],
                    ))));
                }
                Frame::OperatorNext(index, choice) => {
                    if self.result == Attempt::NoMatch && choice + 1 < LEVELS[index].operators.len()
                    {
                        self.push(Frame::Operator(index, choice + 1));
                    }
                }
                Frame::LeafOperator(mut continuation) => {
                    let before = *allowance;
                    let progress = continuation.advance(parser, final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        operators::Progress::Complete(result) => self.result = result,
                        operators::Progress::NeedInput => {
                            self.push(Frame::LeafOperator(continuation));
                            return Progress::NeedInput;
                        }
                        operators::Progress::NeedsProcessing => {
                            self.push(Frame::LeafOperator(continuation));
                            return Progress::NeedsProcessing;
                        }
                        operators::Progress::Limited => {
                            self.push(Frame::LeafOperator(continuation));
                            return Progress::Limited;
                        }
                    }
                }
                frame @ (Frame::Range(_)
                | Frame::RangeFirst(..)
                | Frame::RangeOperator(..)
                | Frame::RangeMiddle(..)
                | Frame::RangeRecovered(..)
                | Frame::RangeSecond(..)
                | Frame::RangeLast(..)
                | Frame::RangeEnd(..)
                | Frame::Paren
                | Frame::ParenOpen(_)
                | Frame::ParenSpace
                | Frame::ParenFormula
                | Frame::ParenRecovered
                | Frame::ParenTrailing(_)
                | Frame::ParenClose(_)
                | Frame::ParenFinish(_)) => self.range_frame(parser, frame),
                Frame::Closer(mut child) => {
                    let before = *allowance;
                    let progress = child.advance(parser, final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        Progress::Complete(result) => self.result = result,
                        Progress::NeedInput => {
                            self.push(Frame::Closer(child));
                            return Progress::NeedInput;
                        }
                        Progress::NeedsProcessing => {
                            self.push(Frame::Closer(child));
                            return Progress::NeedsProcessing;
                        }
                        Progress::Limited => {
                            self.push(Frame::Closer(child));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::Factor => {
                    self.transaction(parser, rules::FACTOR);
                    let node = parser.start();
                    self.push(Frame::FactorValue(node));
                    self.push(Frame::FactorBody);
                }
                Frame::FactorBody => {
                    if !final_input && parser.is_eof() {
                        self.push(Frame::FactorBody);
                        return Progress::NeedInput;
                    }
                    if parser.cursor().starts_with("(") {
                        self.push(Frame::Parenthesis(Box::new(parenthesis::Phase::Enter)));
                    } else {
                        self.push(Frame::FactorNegated);
                        self.push(Frame::Unary(false));
                    }
                }
                Frame::FactorNegated => {
                    if self.result == Attempt::NoMatch {
                        self.push(Frame::FactorNot);
                        self.push(Frame::Unary(true));
                    }
                }
                Frame::FactorNot => {
                    if self.result == Attempt::NoMatch {
                        if !final_input && parser.is_eof() {
                            self.push(Frame::FactorNot);
                            return Progress::NeedInput;
                        }
                        if parser.cursor().starts_with("[") {
                            self.push(Frame::Bracket(Box::new(bracket::Phase::Project)));
                        } else if parser.cursor().starts_with("{") {
                            self.push(Frame::Brace(Box::new(brace::Phase::Enter(false))));
                        } else if parser.cursor().starts_with(":") {
                            self.push(Frame::Collection(Box::new(collection::Phase::Colon)));
                        } else {
                            self.push(Frame::FactorTable);
                            self.push(Frame::Structure(Box::new(
                                structures::StructureContinuation::non_delimited(),
                            )));
                        }
                    }
                }
                Frame::FactorTable => {
                    if self.result == Attempt::NoMatch {
                        self.push(Frame::FactorLiteral);
                        self.push(Frame::Call(rules::LITERAL));
                    }
                }
                Frame::FactorLiteral => {
                    if self.result == Attempt::NoMatch {
                        self.push(Frame::FactorStem(parser.checkpoint()));
                        self.base(rules::IDENTIFIER);
                    }
                }
                Frame::FactorStem(stem) => {
                    let local = self.result == Attempt::Matched;
                    self.push(Frame::FactorSelection(stem, local));
                    if !local {
                        self.push(Frame::LeafPath(Box::new(paths::Continuation::new(
                            rules::PREFIXED_CONTEXT_PATH,
                        ))));
                    }
                }
                Frame::FactorSelection(stem, local) => {
                    if local || self.result.accepted() {
                        if !final_input
                            && (parser.is_eof()
                                || (parser.cursor().byte() == Some(b'.')
                                    && parser.cursor().byte_at(1).is_none()))
                        {
                            self.push(Frame::FactorSelection(stem, local));
                            return Progress::NeedInput;
                        }
                        let call = local && parser.cursor().starts_with("(");
                        // A standalone period terminates a function body. It
                        // selects a slice only when an adjacent field/ordinal
                        // can follow; a split trailing dot still suspends above.
                        let slice = (parser.cursor().starts_with(".")
                            && !parser.cursor().starts_with("..")
                            && parser
                                .cursor()
                                .byte_at(1)
                                .is_some_and(|next| !next.is_ascii_whitespace()))
                            || parser.cursor().starts_with("[")
                            || parser.cursor().starts_with("{");
                        parser.rewind(stem);
                        if call {
                            self.push(Frame::Call(rules::FUNCTION_CALL));
                        } else if slice {
                            self.push(Frame::Call(rules::SLICE));
                        } else {
                            self.push(Frame::Variable(true));
                        }
                    } else {
                        parser.rewind(stem);
                        self.result = Attempt::NoMatch;
                    }
                }
                Frame::FactorValue(node) => {
                    if self.result == Attempt::NoMatch {
                        node.abandon(parser);
                    } else {
                        self.push(Frame::FactorSuffix(node, self.result == Attempt::Committed));
                        self.push(Frame::LeafOperator(Box::new(operators::Continuation::new(
                            rules::TRANSPOSE,
                        ))));
                    }
                }
                Frame::FactorSuffix(node, committed) => {
                    node.complete(parser, SyntaxKind::Factor);
                    self.result = if committed || self.result == Attempt::Committed {
                        Attempt::Committed
                    } else {
                        Attempt::Matched
                    };
                }
                Frame::Unary(not) => {
                    self.transaction(
                        parser,
                        if not {
                            rules::NOT_FACTOR
                        } else {
                            rules::NEGATE_FACTOR
                        },
                    );
                    let node = parser.start();
                    self.push(Frame::UnaryOperator(node, not));
                    if not {
                        self.push(Frame::LeafOperator(Box::new(operators::Continuation::new(
                            rules::NOT,
                        ))));
                    } else {
                        self.base(rules::DASH);
                    }
                }
                Frame::UnaryOperator(node, not) => {
                    if self.result != Attempt::Matched {
                        node.abandon(parser);
                        self.result = Attempt::NoMatch;
                    } else {
                        self.push(Frame::UnaryChild(node, not));
                        if parser.push_nesting() {
                            self.push(Frame::PopNesting);
                            self.push(Frame::Factor);
                        } else {
                            self.push(Frame::Nesting(Box::new(NestingContinuation::new())));
                        }
                    }
                }
                Frame::PopNesting => parser.pop_nesting(),
                Frame::UnaryChild(node, not) => {
                    if self.result == Attempt::NoMatch {
                        self.push(Frame::UnaryMissing(node, not));
                    } else {
                        node.complete(
                            parser,
                            if not {
                                SyntaxKind::NotFactor
                            } else {
                                SyntaxKind::NegateFactor
                            },
                        );
                    }
                }
                Frame::UnaryMissing(node, not) => {
                    self.push(Frame::UnaryRecovered(node, not));
                    self.push(Frame::Required(Box::new(Required::new(
                        if not {
                            rules::NOT_FACTOR
                        } else {
                            rules::NEGATE_FACTOR
                        },
                        "syntax/missing-unary-operand",
                        "missing operand after unary operator",
                        "factor",
                        &[],
                        &[],
                        None,
                    ))));
                }
                Frame::UnaryRecovered(node, not) => {
                    node.complete(
                        parser,
                        if not {
                            SyntaxKind::NotFactor
                        } else {
                            SyntaxKind::NegateFactor
                        },
                    );
                    self.result = Attempt::Committed;
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
                Frame::Structure(mut child) => {
                    let before = *allowance;
                    let progress = child.advance(parser, final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        Progress::Complete(result) => self.result = result,
                        Progress::NeedInput => {
                            self.push(Frame::Structure(child));
                            return Progress::NeedInput;
                        }
                        Progress::NeedsProcessing => {
                            self.push(Frame::Structure(child));
                            return Progress::NeedsProcessing;
                        }
                        Progress::Limited => {
                            self.push(Frame::Structure(child));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::Postfix(phase) => self.postfix_frame(parser, phase),
                Frame::Primitive(mut child) => {
                    let before = *allowance;
                    let progress = child.advance(parser, final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        primitives::Progress::Complete(result) => self.result = result,
                        primitives::Progress::NeedInput => {
                            self.push(Frame::Primitive(child));
                            return Progress::NeedInput;
                        }
                        primitives::Progress::NeedsProcessing => {
                            self.push(Frame::Primitive(child));
                            return Progress::NeedsProcessing;
                        }
                        primitives::Progress::Limited => {
                            self.push(Frame::Primitive(child));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::Definition(phase) => self.definition_frame(parser, phase),
                Frame::Brace(phase) => self.brace_frame(parser, phase),
                Frame::Bracket(phase) => {
                    if let Some(progress) = self.bracket_frame(parser, phase, final_input) {
                        return progress;
                    }
                }
                Frame::Inline(phase) => self.inline_frame(parser, phase),
                Frame::Table(phase) => self.table_frame(parser, phase),
                Frame::MatrixRow(phase) => self.matrix_row_frame(parser, phase),
                Frame::TableRow(phase) => self.row_frame(parser, phase),
                Frame::Missing(mut child) => {
                    use crate::document::parser::recovery::MissingProgress;
                    let before = *allowance;
                    let progress = child.advance(parser, final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        MissingProgress::Complete(_) => self.result = Attempt::Committed,
                        MissingProgress::NeedInput => {
                            self.push(Frame::Missing(child));
                            return Progress::NeedInput;
                        }
                        MissingProgress::NeedsProcessing => {
                            self.push(Frame::Missing(child));
                            return Progress::NeedsProcessing;
                        }
                        MissingProgress::Limited => {
                            self.push(Frame::Missing(child));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::Record(phase) => self.record_frame(parser, phase),
                Frame::Shell(mut child) => {
                    use super::super::super::structure_shell;
                    let before = *allowance;
                    let progress = child.advance(parser, final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        structure_shell::Progress::Complete(result) => self.result = result,
                        structure_shell::Progress::NeedInput => {
                            self.push(Frame::Shell(child));
                            return Progress::NeedInput;
                        }
                        structure_shell::Progress::NeedsProcessing => {
                            self.push(Frame::Shell(child));
                            return Progress::NeedsProcessing;
                        }
                        structure_shell::Progress::Limited => {
                            self.push(Frame::Shell(child));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::MappingProbe(phase) => {
                    if let Some(progress) = self.mapping_probe_frame(parser, phase, final_input) {
                        return progress;
                    }
                }
                Frame::Map(phase) => self.map_frame(parser, phase),
                Frame::Entry(phase) => self.entry_frame(parser, phase),
                Frame::Parenthesis(phase) => {
                    if let Some(progress) = self.parenthesis_frame(parser, phase, final_input) {
                        return progress;
                    }
                }
                Frame::CollectionWrap(node, checkpoint) => {
                    if self.result == Attempt::NoMatch {
                        parser.rewind(checkpoint);
                    } else {
                        node.complete(parser, SyntaxKind::Structure);
                    }
                }
                Frame::Collection(phase) => self.collection_frame(parser, phase),
                Frame::Comprehension(phase) => self.comprehension_frame(parser, phase),
                Frame::MatchArm(phase) => self.arm_frame(parser, phase),
                Frame::Pattern(phase) => self.pattern_frame(parser, phase),
                Frame::Fsm(phase) => self.fsm_frame(parser, phase),
                Frame::Expression(phase) => {
                    if let Some(progress) = self.expression_frame(parser, phase, final_input) {
                        return progress;
                    }
                }
                Frame::RecoverOperand(index) => {
                    self.push(Frame::Required(Box::new(Required::new(
                        parser.current_rule().unwrap_or(rules::EXPRESSION),
                        "syntax/missing-operator-operand",
                        "missing expression after operator",
                        "expression",
                        &[],
                        &[],
                        Some(index),
                    ))));
                }
                Frame::Required(mut child) => {
                    let before = *allowance;
                    let progress = child.advance(parser, final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        Progress::Complete(_) => {}
                        Progress::NeedInput => {
                            self.push(Frame::Required(child));
                            return Progress::NeedInput;
                        }
                        Progress::NeedsProcessing => {
                            self.push(Frame::Required(child));
                            return Progress::NeedsProcessing;
                        }
                        Progress::Limited => {
                            self.push(Frame::Required(child));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::SeedTranspose(seed, committed) => {
                    if self.result == Attempt::Committed {
                        self.result = seed.commit(parser);
                    } else {
                        seed.factor.complete(parser, SyntaxKind::Factor);
                        self.push(Frame::SeedLevel(seed, 6, committed));
                    }
                }
                Frame::SeedLevel(seed, index, committed) => {
                    let marker = [
                        seed.l1, seed.l2, seed.l3, seed.l4, seed.l5, seed.l6, seed.l7,
                    ][index];
                    self.push(Frame::SeedResult(seed, index, committed));
                    self.push(Frame::Loop(Level {
                        index,
                        marker,
                        committed,
                        pairs: 0,
                    }));
                }
                Frame::SeedResult(seed, index, mut committed) => match self.result {
                    Attempt::NoMatch => parser.rewind(seed.checkpoint),
                    Attempt::Committed if parser.is_halted() => {
                        complete_seeded_outer(seed, parser, index)
                    }
                    result => {
                        committed |= result == Attempt::Committed;
                        if index == 0 {
                            self.result = if committed {
                                Attempt::Committed
                            } else {
                                Attempt::Matched
                            };
                        } else {
                            self.push(Frame::SeedLevel(seed, index - 1, committed));
                        }
                    }
                },
            }
        }
        Progress::Complete(self.result)
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::super::continuation_test_support::assert_partitions;
    use super::*;

    #[test]
    fn every_frozen_recursive_rule_has_a_canonical_continuation() {
        for rule in super::super::super::PHASE_2I_RULES {
            assert!(supports(*rule), "missing continuation for {rule:?}");
        }
    }

    #[test]
    fn shared_structures_and_table_bodies_preserve_all_input_cuts() {
        assert_partitions::<Continuation>(&[
            (rules::STRUCTURE, "{1,2}"),
            (rules::STRUCTURE, "[1 2]"),
            (rules::STRUCTURE, "()"),
            (rules::MATRIX, "[]"),
            (rules::MATRIX, "[1,2;3,4]"),
            (rules::MATRIX, "[1+ 2]"),
            (rules::MATRIX, "[x | x <- xs]"),
            (rules::MATRIX, "[1"),
            (rules::MATRIX_COMPREHENSION, "[x | x <- xs]"),
            (rules::MATRIX_COMPREHENSION, "[x | x > 0]"),
            (rules::MATRIX_COMPREHENSION, "[x | x <- ]"),
            (rules::MATRIX_COLUMN, "1, "),
            (rules::MATRIX_ROW, "1 2;\n"),
            (rules::INLINE_TABLE_ROW, "1 2|"),
            (rules::INLINE_TABLE_ROW, "|"),
            (rules::INLINE_TABLE, "|a<u8>|1|"),
            (rules::INLINE_TABLE, "||1|"),
            (rules::REGULAR_TABLE, "|a<u8>|\n|1|"),
            (rules::REGULAR_TABLE, "|a<u8>|\n"),
            (rules::FANCY_TABLE, "╭─╮\n│a│\n│1│"),
            (rules::EXPRESSION, "{a:1,b:2}"),
            (rules::EXPRESSION, "{a:1,1:2}"),
            (rules::EXPRESSION, "{a: {b:2,1:3},1:4}"),
            (rules::EXPRESSION, "{a:,b:2}"),
            (rules::EXPRESSION, "{1:2,3:4}"),
            (rules::EXPRESSION, "{1:,3:4}"),
            (rules::EXPRESSION, "{1,2}"),
            (rules::EXPRESSION, "{,2}"),
            (rules::EXPRESSION, "{1,}"),
            (rules::EXPRESSION, "{x | x <- xs}"),
            (rules::EXPRESSION, "{x | x <-}"),
            (rules::EXPRESSION, "{a:1"),
            (rules::EXPRESSION, "{{1}}"),
            (rules::EXPRESSION, "[x | x > 0]"),
            (rules::EXPRESSION, "[[1 2] [3 4]]"),
        ]);
    }

    #[test]
    fn shared_brace_bracket_and_table_lists_keep_retained_work_linear() {
        use super::super::super::super::continuation_test_support::run;
        for (rule, prefix, unit, tail) in [
            (rules::EXPRESSION, "{", "1,", "2}"),
            (rules::EXPRESSION, "[", "1,", "2]"),
            (rules::EXPRESSION, "{", "a:1,", "2:3}"),
            (rules::INLINE_TABLE_ROW, "", "1 ", "2|"),
            (rules::TABLE_HEADER, "", "x<u8> ", "y<u8>|"),
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
                let (expected, sealed_work) =
                    run::<Continuation>(rule, &text, &[&text], u64::MAX, false);
                assert_eq!(observed, expected);
                assert_eq!(observed.end.to_usize(), text.len());
                assert_eq!(observed.result, Attempt::Matched);
                if let Some((prior, prior_sealed)) = previous {
                    assert!(work <= prior * 3);
                    assert!(sealed_work <= prior_sealed * 3);
                }
                previous = Some((work, sealed_work));
            }
        }
    }

    #[test]
    fn record_and_table_row_owners_retain_recovery_and_speculation_at_all_cuts() {
        assert_partitions::<Continuation>(&[
            (rules::RECORD, "{a:1}"),
            (rules::RECORD, "{a:,b:2}"),
            (rules::RECORD, "{a:1,1:2}"),
            (rules::RECORD, "{a:1,\"x:y\":2}"),
            (rules::RECORD, "{a:1,\"\"\"x:y\"\"\":2}"),
            (rules::RECORD, "{a:1,<u8>:2}"),
            (rules::RECORD, "{a:1,[1,2]:3}"),
            (rules::RECORD, "{a:1"),
            (rules::TABLE_HEADER, "a<u8>|"),
            (rules::TABLE_HEADER, "a<u8> b<u16>|"),
            (rules::TABLE_HEADER, "a< b<u16>|"),
            (rules::TABLE_HEADER, "a<u8>"),
            (rules::INLINE_TABLE_HEADER, "a<u8>|"),
            (rules::FANCY_TABLE_HEADER, "a│b│"),
            (rules::FANCY_TABLE_HEADER, "a<│b│"),
            (rules::TABLE_ROW, "|1 2|"),
            (rules::TABLE_ROW, "| \n"),
            (rules::TABLE_ROW, "|1+ 2|"),
            (rules::TABLE_ROW, "|"),
            (rules::TABLE_ROW2, "|1|2|"),
            (rules::TABLE_ROW2, "||2|"),
            (rules::TABLE_ROW2, "|1+|2|"),
            (rules::TABLE_ROW2, "|1|"),
        ]);
    }

    #[test]
    fn collection_candidates_and_entry_suffixes_preserve_every_partition() {
        assert_partitions::<Continuation>(&[
            (rules::TUPLE, "()"),
            (rules::TUPLE, "(1,2)"),
            (rules::TUPLE, "(1,)"),
            (rules::TUPLE, "("),
            (rules::TUPLE_STRUCT, ":Pair(1)"),
            (rules::TUPLE_STRUCT, ":Pair()"),
            (rules::TUPLE_STRUCT, ":Pair(1,2)"),
            (rules::SET, "{1,2}"),
            (rules::SET, "{,2}"),
            (rules::SET, "{1 2}"),
            (rules::SET, "{}"),
            (rules::MAP, "{1:2,3:4}"),
            (rules::MAP, "{1:,3:4}"),
            (rules::MAP, "{1:2,,3:4}"),
            (rules::MAP, "{"),
            (rules::MAPPING, "1:2,"),
            (rules::MAPPING, "1+:2"),
            (rules::MAPPING, "1:"),
            (rules::MAPPING, "1"),
            (rules::BINDING, "x:1, "),
            (rules::BINDING, "x<u8>:1"),
            (rules::BINDING, "x<:1"),
            (rules::BINDING, "x:"),
            (rules::FIELD, "x<u8>"),
            (rules::FIELD, "x<"),
            (rules::HEADER_FIELD, "x<u8>"),
            (rules::HEADER_FIELD, "x"),
            (rules::FACTOR, "()"),
            (rules::FACTOR, "(1)"),
            (rules::FACTOR, "(1,2)"),
            (rules::FACTOR, "(1,)"),
            (rules::FACTOR, "(1+)"),
            (rules::FACTOR, "(1+"),
            (rules::FACTOR, "(\n1)"),
            (rules::FACTOR, "((1))"),
            (rules::FACTOR, ":Pair(1)"),
            (rules::FACTOR, ":atom"),
        ]);
    }

    #[test]
    fn growing_patterns_tuples_and_mapping_entries_retain_linear_work() {
        use super::super::super::super::continuation_test_support::run;
        for (rule, prefix, unit, tail) in [
            (rules::PATTERN_ARRAY, "[", "1,", "2]"),
            (rules::TUPLE, "(", "1,", "2)"),
            (rules::FACTOR, "(", "1+", "2)"),
            (rules::MAP, "{", "1:2,", "3:4}"),
            (rules::BINDING, "x:\"", "a", "\""),
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
                let (expected, sealed_work) =
                    run::<Continuation>(rule, &text, &[&text], u64::MAX, false);
                assert_eq!(observed, expected);
                assert_eq!(observed.end.to_usize(), text.len());
                assert_eq!(observed.result, Attempt::Matched);
                if let Some((prior, prior_sealed)) = previous {
                    assert!(work <= prior * 3);
                    assert!(sealed_work <= prior_sealed * 3);
                }
                previous = Some((work, sealed_work));
            }
        }
    }

    #[test]
    fn match_and_comprehension_owners_preserve_guard_and_generator_facts_at_all_cuts() {
        assert_partitions::<Continuation>(&[
            (rules::MATCH_ARM, "| _ => 1"),
            (rules::MATCH_ARM, "| x, x > 0 => x;"),
            (rules::MATCH_ARM, "| => 1"),
            (rules::MATCH_ARM, "| x @@@ => 1"),
            (rules::MATCH_ARM, "| x =>"),
            (rules::MATCH_ARM, "| x"),
            (rules::COMPREHENSION_QUALIFIER, "x <- xs"),
            (rules::COMPREHENSION_QUALIFIER, "x = 1"),
            (rules::COMPREHENSION_QUALIFIER, "x > 1"),
            (rules::COMPREHENSION_QUALIFIER, "x <-"),
            (rules::GENERATOR, "[x,] <- xs"),
            (rules::GENERATOR, "x ← xs"),
            (rules::GENERATOR, "x <-"),
            (rules::GENERATOR, "x <"),
            (rules::SET_COMPREHENSION, "{x | x <- xs}"),
            (rules::SET_COMPREHENSION, "{x | x <-, y = 1}"),
            (rules::SET_COMPREHENSION, "{x | }"),
            (rules::SET_COMPREHENSION, "{x | x <- xs,"),
            (rules::EXPRESSION, "[x | x <- xs]..10"),
            (rules::EXPRESSION, "x ? | _ => 1 ."),
        ]);
    }

    #[test]
    fn definition_fsm_and_pattern_owners_preserve_every_cut_and_resource_limit() {
        assert_partitions::<Continuation>(&[
            (rules::VARIABLE_DEFINE, "x = 1"),
            (rules::VARIABLE_DEFINE, "~x<u8> = 1"),
            (rules::VARIABLE_DEFINE, "x := 1"),
            (rules::VARIABLE_DEFINE, "x ="),
            (rules::VARIABLE_DEFINE, "x"),
            (rules::VARIABLE_DEFINE, "x<"),
            (rules::FSM_INSTANCE, "#name(1,x:2)"),
            (rules::FSM_INSTANCE, "#"),
            (rules::FSM_PIPE, "#name -> :start => 1"),
            (rules::FSM_PIPE, "# ->"),
            (rules::FSM_STATE_TRANSITION, "-> :start"),
            (rules::FSM_STATE_TRANSITION, "->"),
            (rules::FSM_ASYNC_TRANSITION, "~> 1"),
            (rules::FSM_OUTPUT, "=> _"),
            (rules::FSM_VALUE, "_"),
            (rules::FSM_VALUE, "[1,..,x]"),
            (rules::PATTERN, "_"),
            (rules::PATTERN, "1+2"),
            (rules::PATTERN, ":Tag(1,_ )"),
            (rules::PATTERN_TUPLE_STRUCT, "`Tag(1,_ )"),
            (rules::PATTERN_ATOM_STRUCT, ":Tag()"),
            (rules::PATTERN_TUPLE, "(1,_)"),
            (rules::PATTERN_TUPLE, "(1,)"),
            (rules::PATTERN_TUPLE, "("),
            (rules::PATTERN_ARRAY, "[]"),
            (rules::PATTERN_ARRAY, "[1,_,2]"),
            (rules::PATTERN_ARRAY, "[1,..,x]"),
            (rules::PATTERN_ARRAY, "[1,..,x,y]"),
            (rules::PATTERN_ARRAY, "[...,...]"),
            (rules::PATTERN_ARRAY, "[1,,2]"),
            (rules::PATTERN_ARRAY, "["),
            (rules::PATTERN_ARRAY_ITEM, "_"),
            (rules::PATTERN_ARRAY_TOKEN, "..."),
            (rules::PATTERN_ARRAY_TOKEN, ".."),
            (rules::PATTERN_ARRAY_TOKEN, "1"),
        ]);
    }

    #[test]
    fn expression_facts_and_match_range_alternatives_survive_all_cuts() {
        assert_partitions::<Continuation>(&[
            (rules::EXPRESSION, "1+2"),
            (rules::EXPRESSION, "f(1,2)"),
            (rules::EXPRESSION, "x[1..3]"),
            (rules::EXPRESSION, "1..2..9"),
            (rules::EXPRESSION, "1.."),
            (rules::EXPRESSION, "1.. ..9"),
            (rules::EXPRESSION, "1?"),
            (rules::EXPRESSION, "1? |x=>2."),
            (rules::EXPRESSION, "{1,2}"),
            (rules::EXPRESSION, "[1 2]"),
            (rules::EXPRESSION, "{x | x <- xs}"),
            (rules::EXPRESSION, "[x | x <- xs]"),
            (rules::EXPRESSION, "{a:1,1:2}"),
            (rules::EXPRESSION, "#m -> :x"),
            (rules::EXPRESSION, "(1+2)*3"),
            (rules::EXPRESSION, "f(x:)"),
            (rules::EXPRESSION, ""),
            (rules::EXPRESSION, "@"),
        ]);
    }

    #[test]
    fn postfix_owners_keep_lists_bound_arguments_and_delimiters_across_cuts() {
        assert_partitions::<Continuation>(&[
            (rules::ARGUMENT_LIST, "()"),
            (rules::ARGUMENT_LIST, "(1,2)"),
            (rules::ARGUMENT_LIST, "(x:1,y:2)"),
            (rules::ARGUMENT_LIST, "(1,)"),
            (rules::ARGUMENT_LIST, "("),
            (rules::FUNCTION_CALL, "f(1,2)"),
            (rules::FUNCTION_CALL, "f(x:1)"),
            (rules::CALL_ARG, "1+2"),
            (rules::CALL_ARG_WITH_BINDING, "x:1"),
            (rules::CALL_ARG_WITH_BINDING, "x:"),
            (rules::CALL_ARG_WITH_BINDING, "x+1"),
            (rules::FSM_ARGS, "(1,x:2)"),
            (rules::SLICE, "x[1,2]"),
            (rules::SLICE, "x.a"),
            (rules::SLICE, "@ctx/value[1]"),
            (rules::SUBSCRIPT, ".a[1]{2}"),
            (rules::SUBSCRIPT, ".1"),
            (rules::SUBSCRIPT, ".a,b"),
            (rules::BRACKET_SUBSCRIPT, "[1..3,4]"),
            (rules::BRACKET_SUBSCRIPT, "[:,1]"),
            (rules::BRACKET_SUBSCRIPT, "[1,]"),
            (rules::BRACKET_SUBSCRIPT, "[]"),
            (rules::BRACE_SUBSCRIPT, "{1..3}"),
            (rules::BRACE_SUBSCRIPT, "{1,}"),
            (rules::FORMULA_SUBSCRIPT, "x+1"),
            (rules::RANGE_SUBSCRIPT, "1..3"),
            (rules::RANGE_SUBSCRIPT, "1"),
            (rules::RANGE_SUBSCRIPT, "1.."),
        ]);
    }

    #[test]
    fn range_and_parenthetical_owners_preserve_all_input_and_work_partitions() {
        assert_partitions::<Continuation>(&[
            (rules::RANGE_EXPRESSION, "1..3"),
            (rules::RANGE_EXPRESSION, "1..2..9"),
            (rules::RANGE_EXPRESSION, "1.."),
            (rules::RANGE_EXPRESSION, "1.. ..9"),
            (rules::RANGE_EXPRESSION, "1..@..9"),
            (rules::RANGE_EXPRESSION, "1..2.."),
            (rules::RANGE_EXPRESSION, "1"),
            (rules::RANGE_EXPRESSION, "x..y"),
            (rules::PARENTHETICAL_TERM, "(1 + 2)"),
            (rules::PARENTHETICAL_TERM, "(1 +)"),
            (rules::PARENTHETICAL_TERM, "()"),
            (rules::PARENTHETICAL_TERM, "("),
            (rules::PARENTHETICAL_TERM, "(1"),
            (rules::PARENTHETICAL_TERM, "(1 @)"),
            (rules::TABLE, "|x|1|"),
            (rules::INLINE_TABLE, "|x|1|"),
            (rules::REGULAR_TABLE, "|x|\n|1|"),
            (rules::FANCY_TABLE, "┌───┐\n│x│\n│1│"),
            (rules::MATRIX, "[1 2]"),
            (rules::RECORD, "{x:1}"),
            (rules::MATRIX, "x"),
            (rules::RECORD, "x"),
        ]);
    }

    #[test]
    fn open_formula_advances_past_completed_operands_before_final_eof() {
        use crate::document::parser::LexicalMode;
        use crate::document::{DocumentId, IdGenerator, ParseConfig, Revision, TextSnapshot};
        // Long N/2N/4N/8N cases below qualify work growth; this check isolates
        // cursor progress at temporary EOF with a short, unfinished suffix.
        for (rule, text) in [
            (rules::FORMULA, "1+".repeat(64) + "1 "),
            (rules::EXPRESSION, "1+".repeat(64) + "1 "),
            (
                rules::FUNCTION_CALL,
                alloc::string::String::from("f(") + &"1,".repeat(64) + "1 ",
            ),
            (
                rules::SLICE,
                alloc::string::String::from("x[") + &"1,".repeat(64) + "1 ",
            ),
        ] {
            let source = TextSnapshot::new(DocumentId(826), Revision(0), text.as_str()).unwrap();
            let mut ids = IdGenerator::new();
            let mut parser = Parser::new(
                &source,
                LexicalMode::CanonicalSourceFragment,
                ParseConfig::default(),
                &mut ids,
            );
            let _root = parser.start();
            let mut child = Continuation::new(rule);
            loop {
                let mut allowance = 1;
                match child.advance(&mut parser, false, &mut allowance) {
                    Progress::NeedsProcessing => {}
                    Progress::NeedInput => break,
                    _ => panic!("open formula must retain the unfinished frontier"),
                }
                assert!(
                    child.work < 2_000_000,
                    "rule {rule:?}, offset {}, length {}",
                    parser.offset().0,
                    text.len()
                );
            }
            assert!(
                parser.offset().to_usize() >= text.len() - 4,
                "completed operands were deferred until final EOF"
            );
            assert!(parser.stats().parser_steps > 256);
            assert!(!parser.is_halted());
        }
    }

    #[test]
    fn scalar_formula_chains_and_payloads_retain_work_while_input_is_open() {
        use super::super::super::super::continuation_test_support::run;
        use alloc::string::String;
        for (rule, prefix, unit, tail) in [
            (rules::EXPRESSION, "", "1+", "2"),
            (rules::FUNCTION_CALL, "f(", "1,", "2)"),
            (rules::SLICE, "x[", "1,", "2]"),
            (rules::FORMULA, "", "1+", "2"),
            (rules::FORMULA, "", "x+", "y"),
            (rules::FACTOR, "\"", "é", "\""),
            (rules::FACTOR, "", "1", ""),
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
                let (expected, sealed_work) =
                    run::<Continuation>(rule, &text, &[&text], u64::MAX, false);
                assert_eq!(observed, expected);
                assert_eq!(observed.stats.source_bytes as usize, text.len());
                assert_eq!(observed.end.to_usize(), text.len());
                assert_eq!(observed.result, Attempt::Matched);
                if let Some((prior, prior_sealed)) = previous {
                    assert!(work <= prior * 3);
                    assert!(sealed_work <= prior_sealed * 3);
                }
                previous = Some((work, sealed_work));
            }
        }
    }

    #[test]
    fn growing_kind_lists_and_nested_annotations_keep_retained_work_linear() {
        use super::super::super::super::continuation_test_support::run;
        use alloc::string::String;
        for (rule, prefix, unit, tail) in [
            (rules::KIND_TUPLE, "(", "u8,", "u16)"),
            (rules::KIND_TABLE, "|", "x<u8>,", "y<u16>|"),
            (rules::KIND_RECORD, "{", "x<u8>,", "y<u16>}"),
            (rules::KIND_MATRIX, "[u8]:", "3,", "4"),
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
                let (expected, sealed_work) =
                    run::<Continuation>(rule, &text, &[&text], u64::MAX, false);
                assert_eq!(observed, expected);
                assert_eq!(observed.stats.source_bytes as usize, text.len());
                assert_eq!(observed.end.to_usize(), text.len());
                assert_eq!(observed.result, Attempt::Matched);
                if let Some((prior, prior_sealed)) = previous {
                    assert!(work <= prior * 3);
                    assert!(sealed_work <= prior_sealed * 3);
                }
                previous = Some((work, sealed_work));
            }
        }
        for rule in [rules::KIND, rules::KIND_ANNOTATION] {
            let mut previous = None;
            for n in [4, 8, 16, 32] {
                let text = if rule == rules::KIND {
                    "[".repeat(n) + "u8" + &"]".repeat(n)
                } else {
                    "<".repeat(n) + "u8" + &">".repeat(n)
                };
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
                assert_eq!(observed.end.to_usize(), text.len());
                assert_eq!(observed.result, Attempt::Matched);
                if let Some(prior) = previous {
                    assert!(work <= prior * 3);
                }
                previous = Some(work);
            }
        }
    }

    #[test]
    fn recursive_kind_owners_retain_delimiters_suffixes_and_recovery() {
        assert_partitions::<Continuation>(&[
            (rules::KIND_SET, "{u8}:3"),
            (rules::KIND_SET, "{u8}:N"),
            (rules::KIND_SET, "{u8}:"),
            (rules::KIND_SET, "{u8"),
            (rules::KIND_MAP, "{u8:u16}"),
            (rules::KIND_MAP, "{[u8]:u16}"),
            (rules::KIND_MAP, "{u8:}"),
            (rules::KIND_MAP, "{u8:"),
            (rules::KIND_RECORD, "{x<u8>,y<u16>}"),
            (rules::KIND_RECORD, "{x<u8>,…}"),
            (rules::KIND_RECORD, "{x<u8>,}"),
            (rules::KIND_RECORD, "{x<u8>"),
            (rules::KIND, "{ x<u8> }"),
            (rules::KIND, "{x<u8>,…}"),
            (rules::KIND, "{x:y}"),
            (rules::KIND, "{x:}"),
            (rules::KIND, "{[u8]:}"),
            (rules::KIND, "{[u8]:u16}"),
            (rules::KIND, "{{u8}:N}"),
            (rules::KIND, "{u8}:3"),
            (rules::KIND, "u8"),
            (rules::KIND, "[u8]"),
            (rules::KIND, "[[u8]]"),
            (rules::KIND, "{u8}"),
            (rules::KIND, "{u8:u16}"),
            (rules::KIND_WITH_OPTION, "u8?"),
            (rules::KIND_WITH_OPTION, "[u8]?"),
            (rules::KIND_ANNOTATION, "<u8>"),
            (rules::KIND_ANNOTATION, "<[u8]>"),
            (rules::KIND_ANNOTATION, "<u8"),
            (rules::KIND_ANNOTATION, "<>"),
            (rules::KIND_KIND, "<u8>"),
            (rules::KIND_KIND, "<<u8>>"),
            (rules::KIND_KIND, "<>"),
            (rules::KIND_KIND, "<"),
            (rules::KIND_SCALAR, "u8:1..3"),
            (rules::KIND_SCALAR, "u8:"),
            (rules::KIND_MATRIX, "[u8]:3,4"),
            (rules::KIND_MATRIX, "[u8]3"),
            (rules::KIND_MATRIX, "[u8]:"),
            (rules::KIND_MATRIX, "[]:3"),
            (rules::KIND_MATRIX, "[u8]:3,"),
            (rules::KIND_MATRIX, "["),
            (rules::KIND_TUPLE, "(u8,u16)"),
            (rules::KIND_TUPLE, "(u8,)"),
            (rules::KIND_TUPLE, "()"),
            (rules::KIND_TUPLE, "("),
            (rules::KIND_TABLE, "|x<u8>,y<u16>|:3"),
            (rules::KIND_TABLE, "|x y|"),
            (rules::KIND_TABLE, "|x,|"),
            (rules::KIND_TABLE, "||"),
            (rules::KIND_TABLE, "|x|:"),
            (rules::KIND_TABLE, "|"),
        ]);
    }

    #[test]
    fn recursive_atom_owners_retain_children_across_every_cut() {
        assert_partitions::<Continuation>(&[
            (rules::FACTOR, "-alpha'"),
            (rules::FACTOR, "!false"),
            (rules::NEGATE_FACTOR, "---1"),
            (rules::NOT_FACTOR, "!!true"),
            (rules::NEGATE_FACTOR, "-;"),
            (rules::NOT_FACTOR, "!"),
            (rules::FACTOR, "x..3"),
            (rules::FACTOR, "x.y"),
            (rules::FACTOR, "f(x)"),
            (rules::LITERAL, "123456"),
            (rules::LITERAL, "0xAB12<u8>"),
            (rules::LITERAL, "\"é\u{301}\""),
            (rules::LITERAL, "`raw`"),
            (rules::LITERAL, ":atom"),
            (rules::LITERAL, "true"),
            (rules::LITERAL, "_"),
            (rules::LITERAL, "<u8>"),
            (rules::LITERAL, "<"),
            (rules::LITERAL, "!"),
            (rules::VAR, "alpha-beta"),
            (rules::VAR, "@ctx/value"),
            (rules::VAR, "x<u8>"),
            (rules::VAR, "x<"),
            (rules::VAR, "x < y"),
            (rules::VAR, "x = 1"),
            (rules::VAR, ""),
            (rules::VAR, "💡"),
            (rules::KIND_ANNOTATION, "<u8>"),
            (rules::KIND_ANNOTATION, "<[u8]>"),
            (rules::KIND_ANNOTATION, "<"),
            (rules::KIND_ANNOTATION, "!"),
            (rules::FORMULA, "1 + 2 * 3"),
        ]);
    }

    #[test]
    fn precedence_owners_preserve_transactions_across_final_input_work_yields() {
        assert_partitions::<Continuation>(&[
            (rules::L1, "1 + 2 * 3"),
            (rules::L1, "(1 + 2) * 3"),
            (rules::L1, "x + {1, 2}"),
            (rules::L1, "[1 2] + [3 4]"),
            (rules::L1, "1 < 2 & 3 < 4"),
            (rules::L1, "1 + ; 2"),
            (rules::L1, "1 + @@@ + 3"),
            (rules::L1, "1 +"),
            (rules::L1, ""),
            (rules::L2, "x < y < z"),
            (rules::L2, "x ⟨ y"),
            (rules::L3, "a + b - c"),
            (rules::L3, "a + \r\nb"),
            (rules::L4, "a * b / c"),
            (rules::L4, "a ** b"),
            (rules::L5, "a ^ b ^ c"),
            (rules::L6, "a"),
            (rules::L7, "{1, 2}"),
            (rules::L7, "\"é\u{301}\""),
            (rules::L7, "💡'"),
        ]);
    }
}
