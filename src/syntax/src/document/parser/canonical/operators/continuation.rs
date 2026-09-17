//! Retained operator alternatives, guards, spacing, and token ownership.
use super::spec::{Spec, specification};
use super::*;
use crate::document::TextSize;
use crate::document::parser::literal_scan::{LiteralProgress, LiteralScan};
use crate::document::parser::{checkpoint::ParserCheckpoint, marker::Marker};
use alloc::vec::Vec;

pub(crate) enum Progress {
    Complete(Attempt),
    NeedInput,
    NeedsProcessing,
    Limited,
}
#[derive(Clone, Copy)]
struct Leaf {
    alternatives: &'static [OperatorAtom],
    guard: OperatorGuard,
    spacing: Option<RuleId>,
}
enum Frame {
    Call(RuleId),
    Exit(ParserCheckpoint),
    Accept,
    SetResult,
    Aggregate(Marker, SyntaxKind, &'static [RuleId], usize),
    CompleteNode(Marker, SyntaxKind),
    Leading(Leaf),
    Guard(Leaf),
    GuardResult(Leaf, ParserCheckpoint),
    GuardAllowed(Leaf),
    Atom(Leaf, usize),
    AtomResult(Leaf, usize),
    SpacedFirst,
    SpacedSecond,
    Base(base::continuation::Continuation),
    Comment(statements::Continuation),
    Tag(&'static str),
    TagExit(ParserCheckpoint),
    Literal(LiteralScan<'static>),
    LiteralResult(Option<TextSize>),
}
pub(crate) struct Continuation {
    frames: Vec<Frame>,
    result: Attempt,
    matched: bool,
    pub work: u64,
}
impl Continuation {
    pub fn new(rule: RuleId) -> Self {
        assert!(supports(rule), "canonical operator owner");
        Self {
            frames: alloc::vec![Frame::Call(rule)],
            result: Attempt::NoMatch,
            matched: false,
            work: 0,
        }
    }
    fn push(&mut self, frame: Frame) {
        self.frames.push(frame);
    }
    fn base(&mut self, rule: RuleId) {
        self.push(Frame::Base(base::continuation::Continuation::new(rule)));
    }
    fn call_bool(&mut self, rule: RuleId) {
        self.push(Frame::Accept);
        self.push(Frame::Call(rule));
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
            let frame = self.frames.pop().expect("operator phase");
            if !matches!(
                frame,
                Frame::Base(_) | Frame::Comment(_) | Frame::Literal(_)
            ) {
                *allowance -= 1;
                self.work += 1;
            }
            match frame {
                Frame::Call(rule) => {
                    let checkpoint = parser.checkpoint();
                    parser.state.rules.push_canonical(rule);
                    self.push(Frame::Exit(checkpoint));
                    match specification(rule) {
                        Spec::Transpose => {
                            self.push(Frame::SetResult);
                            self.base(rules::APOSTROPHE);
                        }
                        Spec::SpacedSubtract => {
                            self.push(Frame::CompleteNode(
                                parser.start(),
                                SyntaxKind::SpacedSubtractOperation,
                            ));
                            self.push(Frame::SpacedFirst);
                            self.base(rules::WS1E);
                        }
                        Spec::Aggregate { kind, alternatives } => {
                            let marker = parser.start();
                            if let Some(rule) = alternatives.first() {
                                self.push(Frame::Aggregate(marker, kind, alternatives, 0));
                                self.push(Frame::Call(*rule));
                            } else {
                                self.result = Attempt::NoMatch;
                            }
                        }
                        Spec::Leaf {
                            kind,
                            alternatives,
                            guard,
                            spacing,
                        } => {
                            self.push(Frame::CompleteNode(parser.start(), kind));
                            let leaf = Leaf {
                                alternatives,
                                guard,
                                spacing,
                            };
                            if let Some(spacing) = spacing {
                                self.push(Frame::Leading(leaf));
                                self.base(spacing);
                            } else {
                                self.push(Frame::Guard(leaf));
                            }
                        }
                    }
                }
                Frame::Exit(checkpoint) => {
                    parser.state.rules.truncate(checkpoint.rule_depth);
                    if self.result == Attempt::NoMatch {
                        // The transaction owns rejected markers, including a wrapper
                        // around a child that finalized the hard-limit remainder.
                        parser.rewind(checkpoint);
                    }
                    if parser.is_halted() {
                        self.result = Attempt::Committed;
                    }
                }
                Frame::Accept => self.matched = self.result.accepted(),
                Frame::SetResult => {
                    self.result = if self.matched {
                        Attempt::Matched
                    } else {
                        Attempt::NoMatch
                    }
                }
                Frame::Aggregate(marker, kind, alternatives, index) => {
                    if self.result.accepted() {
                        marker.complete(parser, kind);
                        self.result = Attempt::Matched;
                    } else if let Some(rule) = alternatives.get(index + 1) {
                        self.push(Frame::Aggregate(marker, kind, alternatives, index + 1));
                        self.push(Frame::Call(*rule));
                    } else {
                        self.result = Attempt::NoMatch;
                    }
                }
                Frame::CompleteNode(marker, kind) => {
                    if self.matched {
                        marker.complete(parser, kind);
                        self.result = Attempt::Matched;
                    } else {
                        self.result = Attempt::NoMatch;
                    }
                }
                Frame::Leading(leaf) => {
                    if self.matched {
                        self.push(Frame::Guard(leaf));
                    }
                }
                Frame::Guard(leaf) => match leaf.guard {
                    OperatorGuard::None => {
                        self.matched = true;
                        self.push(Frame::GuardAllowed(leaf));
                    }
                    guard => {
                        self.push(Frame::GuardResult(leaf, parser.checkpoint()));
                        match guard {
                            OperatorGuard::NotCommentSigil => self.push(Frame::Comment(
                                statements::Continuation::new(rules::COMMENT_SIGIL),
                            )),
                            OperatorGuard::NotMatrixMultiply => {
                                self.call_bool(rules::MATRIX_MULTIPLY)
                            }
                            OperatorGuard::NotGeneratorArrow => self.base(rules::GENERATOR_ARROW),
                            OperatorGuard::None => unreachable!("unguarded leaf handled above"),
                        }
                    }
                },
                Frame::GuardResult(leaf, checkpoint) => {
                    parser.rewind(checkpoint);
                    self.matched = !self.matched;
                    self.push(Frame::GuardAllowed(leaf));
                }
                Frame::GuardAllowed(leaf) => {
                    if self.matched {
                        self.push(Frame::Atom(leaf, 0));
                    }
                }
                Frame::Atom(leaf, index) => {
                    if let Some(atom) = leaf.alternatives.get(index) {
                        self.push(Frame::AtomResult(leaf, index));
                        match atom {
                            OperatorAtom::CanonicalRule(rule) => self.base(*rule),
                            OperatorAtom::Text(text) => self.push(Frame::Tag(text)),
                        }
                    } else {
                        self.matched = false;
                    }
                }
                Frame::AtomResult(leaf, index) => {
                    if !self.matched {
                        self.push(Frame::Atom(leaf, index + 1));
                    } else if let Some(spacing) = leaf.spacing {
                        self.base(spacing);
                    }
                }
                Frame::SpacedFirst => {
                    if self.matched {
                        self.push(Frame::SpacedSecond);
                        self.call_bool(rules::RAW_SUBTRACT);
                    }
                }
                Frame::SpacedSecond => {
                    if self.matched {
                        self.base(rules::WS1E);
                    }
                }
                Frame::Tag(text) => {
                    let checkpoint = parser.checkpoint();
                    parser.state.rules.push_canonical(rules::TAG);
                    self.push(Frame::TagExit(checkpoint));
                    if parser.offset() > parser.cursor().end() {
                        self.matched = false;
                    } else {
                        self.push(Frame::Literal(
                            LiteralScan::new(
                                text,
                                parser.offset(),
                                final_input.then_some(parser.cursor().context_end()),
                            )
                            .expect("nonempty operator atom"),
                        ));
                    }
                }
                Frame::TagExit(checkpoint) => {
                    parser.state.rules.truncate(checkpoint.rule_depth);
                    if !self.matched {
                        parser.rewind(checkpoint);
                    }
                }
                Frame::Literal(mut scan) => {
                    let before = *allowance;
                    let progress = scan.advance(
                        parser.source(),
                        parser.cursor().end(),
                        parser.cursor().context_end(),
                        final_input,
                        allowance,
                    );
                    self.work += before - *allowance;
                    match progress {
                        LiteralProgress::Complete(end) => self.push(Frame::LiteralResult(end)),
                        LiteralProgress::NeedsProcessing => {
                            self.push(Frame::Literal(scan));
                            return Progress::NeedsProcessing;
                        }
                        LiteralProgress::NeedInput => {
                            self.push(Frame::Literal(scan));
                            return Progress::NeedInput;
                        }
                        LiteralProgress::InvalidSource => {
                            panic!("operator continuation source bounds changed")
                        }
                    }
                }
                Frame::LiteralResult(end) => {
                    self.matched = end
                        .and_then(|end| {
                            parser.bump_bytes_token((end - parser.offset()).0, SyntaxKind::Text)
                        })
                        .is_some()
                }
                Frame::Base(mut continuation) => {
                    let before = *allowance;
                    let progress = continuation.advance(parser, final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        base::continuation::Progress::Complete(result) => self.matched = result,
                        base::continuation::Progress::NeedsProcessing => {
                            self.push(Frame::Base(continuation));
                            return Progress::NeedsProcessing;
                        }
                        base::continuation::Progress::NeedInput => {
                            self.push(Frame::Base(continuation));
                            return Progress::NeedInput;
                        }
                        base::continuation::Progress::Limited => {
                            self.push(Frame::Base(continuation));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::Comment(mut continuation) => {
                    let before = *allowance;
                    let progress = continuation.advance(parser, final_input, allowance);
                    self.work += before - *allowance;
                    match progress {
                        statements::Progress::Complete(result) => {
                            self.matched = result == Attempt::Matched
                        }
                        statements::Progress::NeedsProcessing => {
                            self.push(Frame::Comment(continuation));
                            return Progress::NeedsProcessing;
                        }
                        statements::Progress::NeedInput => {
                            self.push(Frame::Comment(continuation));
                            return Progress::NeedInput;
                        }
                        statements::Progress::Limited => {
                            self.push(Frame::Comment(continuation));
                            return Progress::Limited;
                        }
                    }
                }
            }
        }
        Progress::Complete(self.result)
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::continuation_test_support::{self, Output};
    use super::*;
    use alloc::string::String;
    fn run(rule: RuleId, text: &str, chunks: &[&str], fuel: u64, step: bool) -> (Output, u64) {
        continuation_test_support::run::<Continuation>(rule, text, chunks, fuel, step)
    }
    #[test]
    fn operator_spellings_guards_spacing_and_limits_survive_all_input_cuts() {
        let cases = [
            (rules::ADD_SUB_OPERATOR, "+"),
            (rules::MUL_DIV_OPERATOR, "*"),
            (rules::POWER_OPERATOR, "^"),
            (rules::MATRIX_OPERATOR, "**"),
            (rules::RANGE_OPERATOR, "..="),
            (rules::COMPARISON_OPERATOR, "==="),
            (rules::LOGIC_OPERATOR, "&&"),
            (rules::TABLE_OPERATOR, "⋈"),
            (rules::SET_OPERATOR, "∪"),
            (rules::ADD, "+"),
            (rules::SUBTRACT, "-"),
            (rules::RAW_SUBTRACT, "-"),
            (rules::SPACED_SUBTRACT, " - "),
            (rules::MULTIPLY, "*"),
            (rules::DIVIDE, "/"),
            (rules::MODULUS, "%"),
            (rules::POWER, "^"),
            (rules::MATRIX_MULTIPLY, "**"),
            (rules::MATRIX_SOLVE, "\\"),
            (rules::DOT_PRODUCT, "·"),
            (rules::CROSS_PRODUCT, "⨯"),
            (rules::TRANSPOSE, "'"),
            (rules::RANGE_INCLUSIVE, "..="),
            (rules::RANGE_EXCLUSIVE, ".."),
            (rules::NOT_EQUAL, "!="),
            (rules::EQUAL_TO, "=="),
            (rules::STRICT_NOT_EQUAL, "!=="),
            (rules::STRICT_EQUAL, "==="),
            (rules::GREATER_THAN, ">"),
            (rules::LESS_THAN, "<"),
            (rules::GREATER_THAN_EQUAL, ">="),
            (rules::LESS_THAN_EQUAL, "<="),
            (rules::OR, "||"),
            (rules::AND, "&&"),
            (rules::NOT, "!"),
            (rules::XOR, "^^"),
            (rules::JOIN, "⋈"),
            (rules::LEFT_JOIN, "⟕"),
            (rules::RIGHT_JOIN, "⟖"),
            (rules::FULL_JOIN, "⟗"),
            (rules::LEFT_SEMI_JOIN, "⋉"),
            (rules::LEFT_ANTI_JOIN, "▷"),
            (rules::UNION_OP, "∪"),
            (rules::INTERSECTION, "∩"),
            (rules::DIFFERENCE, "∖"),
            (rules::COMPLEMENT, "∁"),
            (rules::SUBSET, "⊆"),
            (rules::SUPERSET, "⊇"),
            (rules::PROPER_SUBSET, "⊊"),
            (rules::PROPER_SUPERSET, "⊋"),
            (rules::ELEMENT_OF, "∈"),
            (rules::NOT_ELEMENT_OF, "∉"),
            (rules::SYMMETRIC_DIFFERENCE, " Δ "),
            (rules::MULTIPLY, "×"),
            (rules::DIVIDE, "÷"),
            (rules::DOT_PRODUCT, "•"),
            (rules::NOT_EQUAL, "¬="),
            (rules::NOT_EQUAL, "≠"),
            (rules::EQUAL_TO, "⩵"),
            (rules::STRICT_NOT_EQUAL, "!≡"),
            (rules::STRICT_NOT_EQUAL, "¬≡"),
            (rules::STRICT_NOT_EQUAL, "¬=="),
            (rules::STRICT_EQUAL, "≡"),
            (rules::GREATER_THAN_EQUAL, "≥"),
            (rules::LESS_THAN_EQUAL, "≤"),
            (rules::OR, "∨"),
            (rules::OR, "⋁"),
            (rules::AND, "∧"),
            (rules::AND, "⋀"),
            (rules::NOT, "¬"),
            (rules::XOR, "⊕"),
            (rules::XOR, "⊻"),
            (rules::PROPER_SUBSET, "⊂"),
            (rules::PROPER_SUPERSET, "⊃"),
            (rules::SUBTRACT, " - "),
            (rules::RANGE_EXCLUSIVE, "..="),
            (rules::RANGE_OPERATOR, ".."),
            (rules::EQUAL_TO, "==="),
            (rules::NOT_EQUAL, "!=="),
            (rules::COMPARISON_OPERATOR, "!=="),
            (rules::GREATER_THAN, ">="),
            (rules::COMPARISON_OPERATOR, ">="),
            (rules::LESS_THAN, "<="),
            (rules::COMPARISON_OPERATOR, "<="),
            (rules::ADD, " +\n"),
            (rules::SYMMETRIC_DIFFERENCE, "\tΔ\u{2009}"),
            (rules::RANGE_OPERATOR, "."),
            (rules::RANGE_OPERATOR, ".."),
            (rules::COMPARISON_OPERATOR, "="),
            (rules::COMPARISON_OPERATOR, "=="),
            (rules::SUBTRACT, "-- comment"),
            (rules::DIVIDE, "// comment"),
            (rules::LESS_THAN, "<-"),
            (rules::MULTIPLY, "**"),
            (rules::ADD, "\u{00a0}+\u{2009}"),
            (rules::ADD, " +\r\n"),
            (rules::ADD, "+\u{301}"),
            (rules::DIVIDE, "/\u{301}"),
            (rules::SPACED_SUBTRACT, " -"),
            (rules::SYMMETRIC_DIFFERENCE, " Δ"),
            (rules::SYMMETRIC_DIFFERENCE, "\tΔ\u{2009}"),
        ];
        for rule in PHASE_2D_OPERATOR_RULES {
            assert!(cases.iter().any(|(case, _)| case == rule));
        }
        for (rule, text) in cases {
            let boundaries: Vec<_> = text
                .char_indices()
                .map(|(at, _)| at)
                .chain(core::iter::once(text.len()))
                .collect();
            for fuel in [0, 1, 2, 4, 8, 16, 64, u64::MAX] {
                for split in &boundaries {
                    let (observed, _) =
                        run(rule, text, &[&text[..*split], &text[*split..]], fuel, true);
                    let accepted = &text[..observed.stats.source_bytes as usize];
                    let (baseline, _) = run(rule, accepted, &[accepted], fuel, false);
                    assert_eq!(
                        observed, baseline,
                        "{rule:?}, split {split}, fuel {fuel}, source {text:?}"
                    );
                }
                let chunks: Vec<_> = boundaries
                    .windows(2)
                    .map(|pair| &text[pair[0]..pair[1]])
                    .collect();
                let (observed, _) = run(rule, text, &chunks, fuel, true);
                let accepted = &text[..observed.stats.source_bytes as usize];
                let (baseline, _) = run(rule, accepted, &[accepted], fuel, false);
                assert_eq!(
                    observed, baseline,
                    "{rule:?}, scalar chunks, fuel {fuel}, source {text:?}"
                );
            }
        }
    }
    #[test]
    fn operator_spacing_and_negative_guards_retain_linear_work() {
        for (rule, atom) in [
            (rules::ADD, "+"),
            (rules::SUBTRACT, "-"),
            (rules::MULTIPLY, "*"),
            (rules::MUL_DIV_OPERATOR, "/"),
            (rules::COMPARISON_OPERATOR, "<"),
            (rules::SYMMETRIC_DIFFERENCE, "Δ"),
        ] {
            let mut previous = None;
            for n in [64, 128, 256, 512] {
                let text = String::from(" ").repeat(n) + atom + &" ".repeat(n);
                let boundaries: Vec<_> = text
                    .char_indices()
                    .map(|(at, _)| at)
                    .chain(core::iter::once(text.len()))
                    .collect();
                let chunks: Vec<_> = boundaries
                    .windows(2)
                    .map(|pair| &text[pair[0]..pair[1]])
                    .collect();
                let (observed, work) = run(rule, &text, &chunks, u64::MAX, true);
                let (baseline, one_shot_work) = run(rule, &text, &[&text], u64::MAX, false);
                assert_eq!(observed, baseline);
                assert_eq!(observed.result, Attempt::Matched);
                assert_eq!(observed.end.to_usize(), text.len());
                assert_eq!(observed.stats.diagnostics_emitted, 0);
                if let Some((prior_streamed, prior_one_shot)) = previous {
                    assert!(
                        work <= prior_streamed * 3,
                        "operator restarted spacing: {rule:?}"
                    );
                    assert!(one_shot_work <= prior_one_shot * 3);
                }
                previous = Some((work, one_shot_work));
            }
        }
    }
}
