//! Owned phases for the canonical document grammar combinators. Recognition and
//! recovery still use Attempt; scheduling yields without unwinding these phases.
use super::*;
use crate::document::parser::checkpoint::ParserCheckpoint;
use crate::document::parser::delimiter_scan::{DelimiterProgress, DelimiterScan};
use crate::document::parser::event::Event;
use crate::document::parser::grapheme_scan::ScanSource;
use crate::document::parser::literal_scan::{LiteralProgress, LiteralScan};
use crate::document::parser::marker::Marker;
use crate::document::{TextRange, TextSize, TokenFlags};
use alloc::{boxed::Box, string::String, vec::Vec};

#[derive(Clone, Copy)]
struct Sequence<'g> {
    items: &'g [GrammarExpression],
    index: usize,
    checkpoint: ParserCheckpoint,
    initial: GrammarState,
    committed: bool,
    distinctive: bool,
    inline: bool,
}
#[derive(Clone, Copy)]
struct Choice<'g> {
    items: &'g [GrammarExpression],
    index: usize,
    checkpoint: ParserCheckpoint,
    initial: GrammarState,
}
#[derive(Clone, Copy)]
struct Repetition<'g> {
    item: &'g GrammarExpression,
    require_one: bool,
    count: usize,
    committed: bool,
    checkpoint: ParserCheckpoint,
    initial: GrammarState,
}
#[derive(Clone, Copy)]
struct Separated<'g> {
    separator: &'g GrammarExpression,
    item: &'g GrammarExpression,
    committed: bool,
    checkpoint: ParserCheckpoint,
    initial: GrammarState,
}

struct RuleExit<'g> {
    specification: &'g DocumentRule,
    checkpoint: ParserCheckpoint,
    marker: Option<Marker>,
    outer: GrammarState,
}

#[derive(Clone, Copy)]
struct Mika<'g> {
    triples: &'g [(&'g str, &'g str, &'g str)],
    index: usize,
    part: usize,
    checkpoint: ParserCheckpoint,
}
struct Fence<'g> {
    scan: DelimiterScan,
    safe_end: TextSize,
    sealed: bool,
    child: Box<Continuation<'g>>,
}
enum Frame<'g> {
    RootResult,
    RootComplete(Marker, usize),
    Header(Sequence<'g>, TextSize, TextSize, String),

    Missing(recovery::MissingContinuation<'static>),
    Abandon(recovery::AbandonContinuation<'static>),
    Nesting(recovery::NestingContinuation),
    NestingResult(&'g DocumentRule, Option<Marker>, usize),
    NestingComplete(&'g DocumentRule, Option<Marker>, usize),
    RuleComplete(RuleExit<'g>),
    FenceComplete(Marker),

    MikaStart(&'g [(&'g str, &'g str, &'g str)], usize),
    MikaPart(Mika<'g>),
    MikaScan(Mika<'g>, Marker, usize, LiteralScan<'g>),
    MikaToken(Mika<'g>, Marker, usize, Option<TextSize>),
    CallRule(RuleId),
    Base(base::continuation::Continuation),
    String(strings::Continuation),
    Comment(statements::Continuation),
    Prose(prose::Continuation),
    Operator(operators::Continuation),
    Path(paths::Continuation),
    Kind(kinds::Continuation),
    Mechdown(mechdown::Continuation),
    Number(literals::Continuation),
    Primitive(primitives::Continuation),
    Structure(structure_shell::Continuation),
    Declaration(declarations::Continuation),
    SourceImport(source_imports::Continuation),
    ModuleImport(imports::Continuation),
    Precedence(recursive_core::Continuation),
    RuleExit(RuleExit<'g>),
    NamedRuleResult(RuleId, TextSize),
    LeadingComment,
    CommentProbe(&'g GrammarExpression, ParserCheckpoint),
    CommentExpression(&'g GrammarExpression, ParserCheckpoint, bool),
    CommentRecoveryCheck(&'g GrammarExpression, ParserCheckpoint, usize, bool),
    CommentTerminal(&'g GrammarExpression, ParserCheckpoint, bool),
    Builtin(&'g str),
    Expression(&'g GrammarExpression),
    Literal(LiteralScan<'g>),
    LiteralResult(Option<TextSize>),
    FenceBody,
    FenceScan(Box<Fence<'g>>),
    FenceAdvance(Box<Fence<'g>>, TextRange),
    FenceChild(Box<Fence<'g>>),
    FenceResult,
    FenceFallback(Marker),
    Sequence(Sequence<'g>),
    Choice(Choice<'g>),
    BestChoice(Choice<'g>, Option<(usize, u32)>),
    Optional(ParserCheckpoint, GrammarState),
    Lookahead(ParserCheckpoint, GrammarState, bool),
    Repetition(Repetition<'g>),
    FirstSeparated(&'g GrammarExpression, &'g GrammarExpression),
    Separator(Separated<'g>),
    SeparatedItem(Separated<'g>, bool),
}

pub(crate) enum Progress {
    Complete(Attempt),
    NeedsProcessing,
    NeedInput,
    Limited,
}

pub(crate) struct Continuation<'g> {
    frames: Vec<Frame<'g>>,
    result: Attempt,
    pub(super) state: GrammarState,
    pub steps: u64,
    pub child_work: u64,
    pub peak_frames: usize,
}

impl<'g> Continuation<'g> {
    pub(super) fn new(expression: &'g GrammarExpression, state: GrammarState) -> Self {
        Self {
            frames: alloc::vec![Frame::Expression(expression)],
            result: Attempt::NoMatch,
            state,
            steps: 0,
            child_work: 0,
            peak_frames: 1,
        }
    }
    pub fn for_rule(specification: &'g DocumentRule) -> Self {
        let mut continuation = Self::new(&specification.expression, GrammarState::default());
        continuation.frames[0] = Frame::CallRule(specification.rule);
        continuation
    }

    pub fn document_root() -> Self {
        let mut continuation = Self::for_rule(
            DOCUMENT_RULES
                .iter()
                .find(|spec| spec.rule == rules::PARSE)
                .expect("document root"),
        );
        continuation.frames.insert(0, Frame::RootResult);
        continuation
    }

    fn enter_rule(&mut self, parser: &mut Parser<'_>, specification: &'g DocumentRule) {
        if !document_rule_enabled(specification) {
            self.result = Attempt::NoMatch;
            return;
        }
        let checkpoint = parser.checkpoint();
        if !parser.push_nesting() {
            let depth = parser.state.rules.len();
            parser.state.rules.push_canonical(specification.rule);
            let marker = specification.root.then(|| parser.start());
            self.push(Frame::NestingResult(specification, marker, depth));
            self.push(Frame::Nesting(recovery::NestingContinuation::new()));
            return;
        }
        parser.state.rules.push_canonical(specification.rule);
        if specification.rule == rules::EVAL_INLINE_MECH_CODE && parser.cursor().starts_with("{{") {
            parser.state.rules.truncate(checkpoint.rule_depth);
            parser.pop_nesting();
            parser.rewind(checkpoint);
            self.result = Attempt::NoMatch;
            return;
        }
        let marker = specification.kind.map(|_| parser.start());
        let outer = core::mem::take(&mut self.state);
        self.push(Frame::RuleExit(RuleExit {
            specification,
            checkpoint,
            marker,
            outer,
        }));
        if specification.rule == rules::MECH_CODE_ALT {
            self.push(Frame::CommentProbe(
                &specification.expression,
                parser.checkpoint(),
            ));
            self.push(Frame::CallRule(rules::WHITESPACE0));
        } else {
            self.push(Frame::Expression(&specification.expression));
        }
    }

    fn comment_selected(
        &mut self,
        parser: &mut Parser<'_>,
        expression: &'g GrammarExpression,
        checkpoint: ParserCheckpoint,
        comment: bool,
    ) {
        parser.rewind(checkpoint);
        if comment {
            self.push(Frame::LeadingComment);
            self.push(Frame::CallRule(rules::WHITESPACE0));
        } else {
            self.push(Frame::Expression(expression));
        }
    }
    fn push(&mut self, frame: Frame<'g>) {
        self.frames.push(frame);
        self.peak_frames = self.peak_frames.max(self.frames.len());
    }
    fn sequence(&mut self, parser: &mut Parser<'_>, frame: Sequence<'g>) {
        let header = frame.index == 6
            && frame.items.len() == 9
            && matches!(frame.items.first(), Some(GrammarExpression::Rule(rule)) if *rule == rules::CODEBLOCK_SIGIL);
        if header {
            self.push(Frame::Header(
                frame,
                frame.checkpoint.cursor.offset + TextSize(3),
                parser.offset(),
                String::new(),
            ));
        } else {
            self.push(Frame::Sequence(frame));
            self.push(Frame::Expression(&frame.items[frame.index]));
        }
    }
    fn repeat(&mut self, parser: &Parser<'_>, mut frame: Repetition<'g>) {
        frame.checkpoint = parser.checkpoint();
        frame.initial = self.state;
        self.push(Frame::Repetition(frame));
        self.push(Frame::Expression(frame.item));
    }
    fn separated(&mut self, parser: &Parser<'_>, mut frame: Separated<'g>) {
        frame.checkpoint = parser.checkpoint();
        frame.initial = self.state;
        self.push(Frame::Separator(frame));
        self.push(Frame::Expression(frame.separator));
    }
    fn aggregate(committed: bool) -> Attempt {
        if committed {
            Attempt::Committed
        } else {
            Attempt::Matched
        }
    }

    /// The allowance counts grammar-frame transitions. Ordinary rule/recovery
    /// calls still use their existing hard parser limits; converting their own
    /// suspendable phases is required before this becomes the stream API.
    pub fn advance(
        &mut self,
        parser: &mut Parser<'_>,
        input_final: bool,
        allowance: &mut u64,
    ) -> Progress {
        while !self.frames.is_empty()
            || parser.state.resource_found.is_some()
            || parser.state.tree_cache.pending()
        {
            if input_final && parser.state.resource_found.is_some() {
                let before = *allowance;
                let complete = parser.advance_resource_found(allowance);
                self.child_work += before - *allowance;
                if !complete {
                    return Progress::NeedsProcessing;
                }
                if self.frames.is_empty() {
                    break;
                }
            }
            if !input_final && parser.is_halted() {
                return Progress::Limited;
            }
            if parser.state.tree_cache.pending() {
                let before = *allowance;
                let complete = parser.advance_tree_cache(allowance);
                self.child_work += before - *allowance;
                if !complete {
                    return Progress::NeedsProcessing;
                }
                if self.frames.is_empty() {
                    break;
                }
            }
            let final_input = input_final || !parser.state.cursor_frontier;
            if *allowance == 0 {
                return Progress::NeedsProcessing;
            }
            if !matches!(
                self.frames.last(),
                Some(
                    Frame::Missing(_)
                        | Frame::Abandon(_)
                        | Frame::Nesting(_)
                        | Frame::MikaScan(..)
                        | Frame::Literal(_)
                        | Frame::Base(_)
                        | Frame::String(_)
                        | Frame::Comment(_)
                        | Frame::Prose(_)
                        | Frame::Operator(_)
                        | Frame::Path(_)
                        | Frame::Kind(_)
                        | Frame::Mechdown(_)
                        | Frame::Number(_)
                        | Frame::Primitive(_)
                        | Frame::Structure(_)
                        | Frame::Declaration(_)
                        | Frame::SourceImport(_)
                        | Frame::ModuleImport(_)
                        | Frame::Precedence(_)
                        | Frame::FenceScan(..)
                        | Frame::FenceChild(_)
                )
            ) {
                *allowance -= 1;
                self.steps += 1;
            }
            match self.frames.pop().expect("pending grammar phase") {
                Frame::RootResult => {
                    if !self.result.accepted() && !parser.is_halted() {
                        let marker = parser.start();
                        let depth = parser.state.rules.len();
                        parser.state.rules.push_canonical(rules::PARSE);
                        self.push(Frame::RootComplete(marker, depth));
                        self.push(Frame::Abandon(recovery::AbandonContinuation::new(
                            rules::PARSE,
                            "syntax/invalid-document",
                            "source does not form a canonical document",
                        )));
                    }
                }
                Frame::RootComplete(marker, depth) => {
                    parser.state.rules.truncate(depth);
                    marker.complete_with_flags(
                        parser,
                        SyntaxKind::Document,
                        NodeFlags::REPARSE_ROOT,
                    );
                }
                Frame::Header(frame, at, end, mut prefix) => {
                    let prefixes = crate::document::CodeFenceInfo::MECH_PREFIXES;
                    let matched = prefixes
                        .iter()
                        .any(|candidate| prefix.starts_with(candidate));
                    let impossible = !prefixes
                        .iter()
                        .any(|candidate| candidate.starts_with(prefix.as_str()));
                    if matched || impossible || at >= end {
                        self.push(Frame::Sequence(frame));
                        self.push(if matched {
                            Frame::FenceBody
                        } else {
                            Frame::Expression(&frame.items[frame.index])
                        });
                    } else {
                        let scalar = parser
                            .source()
                            .scalar_at(at.to_usize())
                            .expect("header scalar");
                        let next = at + TextSize(scalar.len() as u32);
                        if scalar == "{" {
                            self.push(Frame::Sequence(frame));
                            self.push(Frame::Expression(&frame.items[frame.index]));
                        } else {
                            if !prefix.is_empty()
                                || !scalar.chars().next().expect("scalar").is_whitespace()
                            {
                                prefix.push_str(scalar);
                            }
                            self.push(Frame::Header(frame, next, end, prefix));
                        }
                    }
                }
                Frame::Missing(mut child) => {
                    let before = *allowance;
                    let progress = child.advance(parser, final_input, allowance);
                    self.child_work += before - *allowance;
                    match progress {
                        recovery::MissingProgress::Complete(_) => self.result = Attempt::Committed,
                        recovery::MissingProgress::NeedsProcessing => {
                            self.push(Frame::Missing(child));
                            return Progress::NeedsProcessing;
                        }
                        recovery::MissingProgress::NeedInput => {
                            self.push(Frame::Missing(child));
                            return Progress::NeedInput;
                        }
                        recovery::MissingProgress::Limited => {
                            self.push(Frame::Missing(child));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::Abandon(mut child) => {
                    let before = *allowance;
                    let progress =
                        child.advance(parser, final_input, allowance, |_, _, _, allowance| {
                            if *allowance == 0 {
                                return recovery::BoundaryProgress::NeedsProcessing;
                            }
                            *allowance -= 1;
                            recovery::BoundaryProgress::Complete(false)
                        });
                    self.child_work += before - *allowance;
                    match progress {
                        recovery::AbandonProgress::Complete(marker) => {
                            debug_assert!(marker.is_none_or(|node| node.kind == SyntaxKind::Error));
                            self.result = Attempt::Committed;
                        }
                        recovery::AbandonProgress::NeedsProcessing => {
                            self.push(Frame::Abandon(child));
                            return Progress::NeedsProcessing;
                        }
                        recovery::AbandonProgress::NeedInput => {
                            self.push(Frame::Abandon(child));
                            return Progress::NeedInput;
                        }
                        recovery::AbandonProgress::Limited => {
                            self.push(Frame::Abandon(child));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::Nesting(mut child) => {
                    let before = *allowance;
                    let progress = child.advance(parser, final_input, allowance);
                    self.child_work += before - *allowance;
                    match progress {
                        recovery::NestingProgress::Complete => self.result = Attempt::Committed,
                        recovery::NestingProgress::NeedsProcessing => {
                            self.push(Frame::Nesting(child));
                            return Progress::NeedsProcessing;
                        }
                        recovery::NestingProgress::NeedInput => {
                            self.push(Frame::Nesting(child));
                            return Progress::NeedInput;
                        }
                        recovery::NestingProgress::Limited => {
                            self.push(Frame::Nesting(child));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::NestingResult(specification, marker, depth) => {
                    let abandon = marker.is_some() && !parser.is_eof() && !parser.is_halted();
                    self.push(Frame::NestingComplete(specification, marker, depth));
                    if abandon {
                        self.push(Frame::Abandon(recovery::AbandonContinuation::new(
                            specification.rule,
                            "syntax/unexpected-document-source",
                            "source exceeds the canonical document nesting limit",
                        )));
                    }
                }
                Frame::NestingComplete(specification, marker, depth) => {
                    if let Some(marker) = marker {
                        marker.complete_with_flags(
                            parser,
                            specification.kind.expect("root node kind"),
                            NodeFlags::REPARSE_ROOT,
                        );
                    }
                    parser.state.rules.truncate(depth);
                    self.result = Attempt::Committed;
                }
                Frame::MikaStart(triples, index) => {
                    if index < triples.len() {
                        self.push(Frame::MikaPart(Mika {
                            triples,
                            index,
                            part: 0,
                            checkpoint: parser.checkpoint(),
                        }));
                    } else {
                        self.result = Attempt::NoMatch;
                    }
                }
                Frame::MikaPart(mika) => {
                    let (left, nose, right) = mika.triples[mika.index];
                    let (rule, literal) = [
                        (rules::MIKA_EYE_LEFT, left),
                        (rules::MIKA_NOSE, nose),
                        (rules::MIKA_EYE_RIGHT, right),
                    ][mika.part];
                    let depth = parser.state.rules.len();
                    parser.state.rules.push_canonical(rule);
                    let node = parser.start();
                    if let Some(scan) = LiteralScan::new(
                        literal,
                        parser.offset(),
                        (!parser.state.context_frontier).then_some(parser.cursor().context_end()),
                    ) {
                        self.push(Frame::MikaScan(mika, node, depth, scan));
                    } else {
                        self.push(Frame::MikaToken(mika, node, depth, None));
                    }
                }
                Frame::MikaScan(mika, node, depth, mut scan) => {
                    let before = *allowance;
                    let progress = scan.advance(
                        parser.source(),
                        parser.cursor().end(),
                        parser.cursor().context_end(),
                        final_input,
                        allowance,
                    );
                    self.child_work += before - *allowance;
                    match progress {
                        LiteralProgress::Complete(end) => {
                            self.push(Frame::MikaToken(mika, node, depth, end))
                        }
                        LiteralProgress::NeedInput => {
                            self.push(Frame::MikaScan(mika, node, depth, scan));
                            return Progress::NeedInput;
                        }
                        LiteralProgress::NeedsProcessing => {
                            self.push(Frame::MikaScan(mika, node, depth, scan));
                            return Progress::NeedsProcessing;
                        }
                        LiteralProgress::InvalidSource => {
                            self.push(Frame::MikaToken(mika, node, depth, None))
                        }
                    }
                }
                Frame::MikaToken(mut mika, node, depth, end) => {
                    let matched = end
                        .and_then(|end| {
                            parser.bump_bytes_token((end - parser.offset()).0, SyntaxKind::Text)
                        })
                        .is_some();
                    if matched {
                        node.complete(
                            parser,
                            [
                                SyntaxKind::MikaEyeLeft,
                                SyntaxKind::MikaNose,
                                SyntaxKind::MikaEyeRight,
                            ][mika.part],
                        );
                    } else if !parser.state.resource_finalizing {
                        node.abandon(parser);
                    }
                    parser.state.rules.truncate(depth);
                    if matched {
                        mika.part += 1;
                        if mika.part == 3 {
                            self.result = Attempt::Matched;
                        } else {
                            self.push(Frame::MikaPart(mika));
                        }
                    } else {
                        parser.rewind(mika.checkpoint);
                        self.push(Frame::MikaStart(mika.triples, mika.index + 1));
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
                    self.child_work += before - *allowance;
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
                            self.push(Frame::LiteralResult(None));
                        }
                    }
                }
                Frame::LiteralResult(end) => {
                    self.result = end
                        .and_then(|end| {
                            parser.bump_bytes_token((end - parser.offset()).0, SyntaxKind::Text)
                        })
                        .map_or(Attempt::NoMatch, |_| Attempt::Matched);
                    if parser.is_halted() {
                        self.result = Attempt::Committed;
                    }
                }
                Frame::CallRule(rule) => {
                    if let Some(specification) =
                        DOCUMENT_RULES.iter().find(|spec| spec.rule == rule)
                    {
                        if specification.rule == rules::EVAL_INLINE_MECH_CODE
                            && !final_input
                            && parser.cursor().starts_with("{")
                            && parser.cursor().byte_at(1).is_none()
                        {
                            self.push(Frame::CallRule(rule));
                            return Progress::NeedInput;
                        }
                        self.enter_rule(parser, specification);
                    } else if recursive_core::continuation_supports(rule) {
                        self.push(Frame::Precedence(recursive_core::Continuation::new(rule)));
                    } else if imports::supports(rule) {
                        self.push(Frame::ModuleImport(imports::Continuation::new(rule)));
                    } else if source_imports::supports(rule) {
                        self.push(Frame::SourceImport(source_imports::Continuation::new(rule)));
                    } else if declarations::supports(rule) {
                        self.push(Frame::Declaration(declarations::Continuation::new(rule)));
                    } else if structure_shell::supports(rule) {
                        self.push(Frame::Structure(structure_shell::Continuation::new(rule)));
                    } else if primitives::supports(rule) {
                        self.push(Frame::Primitive(primitives::Continuation::new(rule)));
                    } else if literals::supports(rule) {
                        self.push(Frame::Number(literals::Continuation::new(rule)));
                    } else if mechdown::supports(rule) {
                        self.push(Frame::Mechdown(mechdown::Continuation::new(rule)));
                    } else if paths::supports(rule) {
                        self.push(Frame::Path(paths::Continuation::new(rule)));
                    } else if kinds::supports(rule) {
                        self.push(Frame::Kind(kinds::Continuation::new(rule)));
                    } else if operators::supports(rule) {
                        self.push(Frame::Operator(operators::Continuation::new(rule)));
                    } else if rule == rules::PARAGRAPH_TEXT {
                        self.push(Frame::Prose(prose::Continuation::new(rule)));
                    } else if statements::supports(rule) {
                        self.push(Frame::Comment(statements::Continuation::new(rule)));
                    } else if strings::supports(rule) {
                        self.push(Frame::String(strings::Continuation::new(rule)));
                    } else {
                        self.push(Frame::Base(base::continuation::Continuation::new(rule)));
                    }
                }
                Frame::Base(mut continuation) => {
                    let before = *allowance;
                    let progress = continuation.advance(parser, final_input, allowance);
                    self.child_work += before - *allowance;
                    match progress {
                        base::continuation::Progress::Complete(result) => {
                            self.result = if result {
                                Attempt::Matched
                            } else {
                                Attempt::NoMatch
                            };
                        }
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
                Frame::String(mut continuation) => {
                    let before = *allowance;
                    let progress = continuation.advance(parser, final_input, allowance);
                    self.child_work += before - *allowance;
                    match progress {
                        strings::Progress::Complete(result) => self.result = result,
                        strings::Progress::NeedsProcessing => {
                            self.push(Frame::String(continuation));
                            return Progress::NeedsProcessing;
                        }
                        strings::Progress::NeedInput => {
                            self.push(Frame::String(continuation));
                            return Progress::NeedInput;
                        }
                        strings::Progress::Limited => {
                            self.push(Frame::String(continuation));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::Comment(mut continuation) => {
                    let before = *allowance;
                    let progress = continuation.advance(parser, final_input, allowance);
                    self.child_work += before - *allowance;
                    match progress {
                        statements::Progress::Complete(result) => self.result = result,
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
                Frame::Prose(mut continuation) => {
                    let before = *allowance;
                    let progress = continuation.advance(parser, final_input, allowance);
                    self.child_work += before - *allowance;
                    match progress {
                        prose::Progress::Complete(result) => self.result = result,
                        prose::Progress::NeedsProcessing => {
                            self.push(Frame::Prose(continuation));
                            return Progress::NeedsProcessing;
                        }
                        prose::Progress::NeedInput => {
                            self.push(Frame::Prose(continuation));
                            return Progress::NeedInput;
                        }
                        prose::Progress::Limited => {
                            self.push(Frame::Prose(continuation));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::Path(mut continuation) => {
                    let before = *allowance;
                    let progress = continuation.advance(parser, final_input, allowance);
                    self.child_work += before - *allowance;
                    match progress {
                        paths::Progress::Complete(result) => self.result = result,
                        paths::Progress::NeedsProcessing => {
                            self.push(Frame::Path(continuation));
                            return Progress::NeedsProcessing;
                        }
                        paths::Progress::NeedInput => {
                            self.push(Frame::Path(continuation));
                            return Progress::NeedInput;
                        }
                        paths::Progress::Limited => {
                            self.push(Frame::Path(continuation));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::Mechdown(mut continuation) => {
                    let before = *allowance;
                    let progress = continuation.advance(parser, final_input, allowance);
                    self.child_work += before - *allowance;
                    match progress {
                        mechdown::Progress::Complete(result) => self.result = result,
                        mechdown::Progress::NeedsProcessing => {
                            self.push(Frame::Mechdown(continuation));
                            return Progress::NeedsProcessing;
                        }
                        mechdown::Progress::NeedInput => {
                            self.push(Frame::Mechdown(continuation));
                            return Progress::NeedInput;
                        }
                        mechdown::Progress::Limited => {
                            self.push(Frame::Mechdown(continuation));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::Number(mut continuation) => {
                    let before = *allowance;
                    let progress = continuation.advance(parser, final_input, allowance);
                    self.child_work += before - *allowance;
                    match progress {
                        literals::Progress::Complete(result) => self.result = result,
                        literals::Progress::NeedsProcessing => {
                            self.push(Frame::Number(continuation));
                            return Progress::NeedsProcessing;
                        }
                        literals::Progress::NeedInput => {
                            self.push(Frame::Number(continuation));
                            return Progress::NeedInput;
                        }
                        literals::Progress::Limited => {
                            self.push(Frame::Number(continuation));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::Primitive(mut continuation) => {
                    let before = *allowance;
                    let progress = continuation.advance(parser, final_input, allowance);
                    self.child_work += before - *allowance;
                    match progress {
                        primitives::Progress::Complete(result) => self.result = result,
                        primitives::Progress::NeedsProcessing => {
                            self.push(Frame::Primitive(continuation));
                            return Progress::NeedsProcessing;
                        }
                        primitives::Progress::NeedInput => {
                            self.push(Frame::Primitive(continuation));
                            return Progress::NeedInput;
                        }
                        primitives::Progress::Limited => {
                            self.push(Frame::Primitive(continuation));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::Structure(mut continuation) => {
                    let before = *allowance;
                    let progress = continuation.advance(parser, final_input, allowance);
                    self.child_work += before - *allowance;
                    match progress {
                        structure_shell::Progress::Complete(result) => self.result = result,
                        structure_shell::Progress::NeedsProcessing => {
                            self.push(Frame::Structure(continuation));
                            return Progress::NeedsProcessing;
                        }
                        structure_shell::Progress::NeedInput => {
                            self.push(Frame::Structure(continuation));
                            return Progress::NeedInput;
                        }
                        structure_shell::Progress::Limited => {
                            self.push(Frame::Structure(continuation));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::Precedence(mut continuation) => {
                    let before = *allowance;
                    let progress = continuation.advance(parser, final_input, allowance);
                    self.child_work += before - *allowance;
                    match progress {
                        recursive_core::Progress::Complete(result) => self.result = result,
                        recursive_core::Progress::NeedsProcessing => {
                            self.push(Frame::Precedence(continuation));
                            return Progress::NeedsProcessing;
                        }
                        recursive_core::Progress::NeedInput => {
                            self.push(Frame::Precedence(continuation));
                            return Progress::NeedInput;
                        }
                        recursive_core::Progress::Limited => {
                            self.push(Frame::Precedence(continuation));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::ModuleImport(mut continuation) => {
                    let before = *allowance;
                    let progress = continuation.advance(parser, final_input, allowance);
                    self.child_work += before - *allowance;
                    match progress {
                        imports::Progress::Complete(result) => self.result = result,
                        imports::Progress::NeedsProcessing => {
                            self.push(Frame::ModuleImport(continuation));
                            return Progress::NeedsProcessing;
                        }
                        imports::Progress::NeedInput => {
                            self.push(Frame::ModuleImport(continuation));
                            return Progress::NeedInput;
                        }
                        imports::Progress::Limited => {
                            self.push(Frame::ModuleImport(continuation));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::SourceImport(mut continuation) => {
                    let before = *allowance;
                    let progress = continuation.advance(parser, final_input, allowance);
                    self.child_work += before - *allowance;
                    match progress {
                        source_imports::Progress::Complete(result) => self.result = result,
                        source_imports::Progress::NeedsProcessing => {
                            self.push(Frame::SourceImport(continuation));
                            return Progress::NeedsProcessing;
                        }
                        source_imports::Progress::NeedInput => {
                            self.push(Frame::SourceImport(continuation));
                            return Progress::NeedInput;
                        }
                        source_imports::Progress::Limited => {
                            self.push(Frame::SourceImport(continuation));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::Declaration(mut continuation) => {
                    let before = *allowance;
                    let progress = continuation.advance(parser, final_input, allowance);
                    self.child_work += before - *allowance;
                    match progress {
                        declarations::Progress::Complete(result) => self.result = result,
                        declarations::Progress::NeedsProcessing => {
                            self.push(Frame::Declaration(continuation));
                            return Progress::NeedsProcessing;
                        }
                        declarations::Progress::NeedInput => {
                            self.push(Frame::Declaration(continuation));
                            return Progress::NeedInput;
                        }
                        declarations::Progress::Limited => {
                            self.push(Frame::Declaration(continuation));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::Kind(mut continuation) => {
                    let before = *allowance;
                    let progress = continuation.advance(parser, final_input, allowance);
                    self.child_work += before - *allowance;
                    match progress {
                        kinds::Progress::Complete(result) => self.result = result,
                        kinds::Progress::NeedsProcessing => {
                            self.push(Frame::Kind(continuation));
                            return Progress::NeedsProcessing;
                        }
                        kinds::Progress::NeedInput => {
                            self.push(Frame::Kind(continuation));
                            return Progress::NeedInput;
                        }
                        kinds::Progress::Limited => {
                            self.push(Frame::Kind(continuation));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::Operator(mut continuation) => {
                    let before = *allowance;
                    let progress = continuation.advance(parser, final_input, allowance);
                    self.child_work += before - *allowance;
                    match progress {
                        operators::Progress::Complete(result) => self.result = result,
                        operators::Progress::NeedsProcessing => {
                            self.push(Frame::Operator(continuation));
                            return Progress::NeedsProcessing;
                        }
                        operators::Progress::NeedInput => {
                            self.push(Frame::Operator(continuation));
                            return Progress::NeedInput;
                        }
                        operators::Progress::Limited => {
                            self.push(Frame::Operator(continuation));
                            return Progress::Limited;
                        }
                    }
                }
                Frame::RuleExit(frame) => {
                    if parser.is_halted() {
                        self.result = Attempt::Committed;
                    }
                    let abandon = frame.specification.rule == rules::PARSE
                        && self.result.accepted()
                        && !parser.is_eof();
                    self.push(Frame::RuleComplete(frame));
                    if abandon {
                        self.push(Frame::Abandon(recovery::AbandonContinuation::new(
                            rules::PARSE,
                            "syntax/unexpected-document-source",
                            "unexpected source after the canonical document",
                        )));
                    }
                }
                Frame::RuleComplete(frame) => {
                    if let Some(marker) = frame.marker {
                        if self.result == Attempt::NoMatch {
                            marker.abandon(parser);
                        } else {
                            let flags = if frame.specification.root {
                                NodeFlags::REPARSE_ROOT
                            } else {
                                NodeFlags::NONE
                            };
                            marker.complete_with_flags(
                                parser,
                                frame.specification.kind.expect("document rule node kind"),
                                flags,
                            );
                        }
                    }
                    parser.state.rules.truncate(frame.checkpoint.rule_depth);
                    parser.pop_nesting();
                    if self.result == Attempt::NoMatch {
                        parser.rewind(frame.checkpoint);
                    }
                    self.state = frame.outer;
                }
                Frame::NamedRuleResult(rule, start) => {
                    if parser.is_halted() {
                        self.result = Attempt::Committed;
                    }
                    if rule == rules::CODEBLOCK_SIGIL && self.result.accepted() {
                        self.state.codeblock_delimiter = parser
                            .source()
                            .text(crate::document::TextRange::new(start, parser.offset()))
                            .ok()
                            .and_then(|text| match text.as_str() {
                                "```" => Some(rules::GRAVE_CODEBLOCK_SIGIL),
                                "~~~" => Some(rules::TILDE_CODEBLOCK_SIGIL),
                                _ => None,
                            });
                    }
                }
                Frame::CommentProbe(expression, checkpoint) => {
                    if !parser.cursor().starts_with("--") {
                        if !final_input
                            && parser.cursor().starts_with("-")
                            && parser.cursor().byte_at(1).is_none()
                        {
                            self.push(Frame::CommentProbe(expression, checkpoint));
                            return Progress::NeedInput;
                        }
                        self.comment_selected(parser, expression, checkpoint, false);
                    } else {
                        match parser.cursor().byte_at(2) {
                            None if !final_input => {
                                self.push(Frame::CommentProbe(expression, checkpoint));
                                return Progress::NeedInput;
                            }
                            None | Some(b' ' | b'\t' | b'\r' | b'\n') => {
                                self.comment_selected(parser, expression, checkpoint, true)
                            }
                            _ => {
                                let previous = parser.replace_consuming_recovery(false);
                                self.push(Frame::CommentExpression(
                                    expression, checkpoint, previous,
                                ));
                                self.push(Frame::CallRule(rules::EXPRESSION));
                            }
                        }
                    }
                }
                Frame::CommentExpression(expression, checkpoint, previous) => {
                    if self.result == Attempt::Committed {
                        self.push(Frame::CommentRecoveryCheck(
                            expression,
                            checkpoint,
                            checkpoint.events,
                            previous,
                        ));
                    } else if self.result == Attempt::Matched {
                        self.push(Frame::CommentTerminal(expression, checkpoint, previous));
                        self.push(Frame::CallRule(rules::CODE_TERMINAL));
                    } else {
                        parser.replace_consuming_recovery(previous);
                        self.comment_selected(parser, expression, checkpoint, true);
                    }
                }
                Frame::CommentRecoveryCheck(expression, checkpoint, index, previous) => {
                    // One retained event per metered transition; a suspended probe
                    // keeps source-consuming recovery disabled until selection.
                    if let Some(event) = parser.state.events.get(index) {
                        if matches!(event, Event::Token { range, flags, .. }
                            if !range.is_empty() && flags.contains(TokenFlags::ERROR))
                        {
                            parser.replace_consuming_recovery(previous);
                            self.comment_selected(parser, expression, checkpoint, true);
                        } else {
                            self.push(Frame::CommentRecoveryCheck(
                                expression,
                                checkpoint,
                                index + 1,
                                previous,
                            ));
                        }
                    } else {
                        self.push(Frame::CommentTerminal(expression, checkpoint, previous));
                        self.push(Frame::CallRule(rules::CODE_TERMINAL));
                    }
                }
                Frame::CommentTerminal(expression, checkpoint, previous) => {
                    parser.replace_consuming_recovery(previous);
                    self.comment_selected(
                        parser,
                        expression,
                        checkpoint,
                        self.result != Attempt::Matched,
                    )
                }
                Frame::LeadingComment => {
                    if self.result.accepted() {
                        self.push(Frame::CallRule(rules::COMMENT));
                    }
                }
                Frame::Expression(expression) => self.expression(parser, expression),
                Frame::Builtin(name) => match name {
                    "eof" if parser.is_eof() && !final_input => {
                        self.push(Frame::Builtin(name));
                        return Progress::NeedInput;
                    }
                    "eof" => {
                        self.result = if parser.is_eof() {
                            Attempt::Matched
                        } else {
                            Attempt::NoMatch
                        }
                    }
                    "matching-codeblock-sigil" => {
                        if let Some(rule) = self.state.codeblock_delimiter {
                            self.push(Frame::CallRule(rule));
                        } else {
                            self.result = Attempt::NoMatch;
                        }
                    }
                    _ => self.result = Attempt::NoMatch,
                },
                Frame::FenceBody => {
                    let delimiter = match self.state.codeblock_delimiter {
                        Some(rule) if rule == rules::GRAVE_CODEBLOCK_SIGIL => "```",
                        Some(rule) if rule == rules::TILDE_CODEBLOCK_SIGIL => "~~~",
                        _ => unreachable!("fence body requires an opening delimiter"),
                    };
                    let scanner = DelimiterScan::new(
                        delimiter,
                        parser.offset(),
                        (!parser.state.cursor_frontier).then_some(parser.cursor().end()),
                    )
                    .expect("fence delimiter");
                    let mut child = Self::for_rule(
                        DOCUMENT_RULES
                            .iter()
                            .find(|spec| spec.rule == rules::MECH_CODE)
                            .expect("Mech body rule"),
                    );
                    child.frames.insert(0, Frame::FenceResult);
                    self.push(Frame::FenceScan(Box::new(Fence {
                        scan: scanner,
                        safe_end: parser.offset(),
                        sealed: false,
                        child: Box::new(child),
                    })));
                }
                Frame::FenceScan(mut fence) => {
                    if parser.is_halted() {
                        fence.sealed = true;
                        fence.safe_end = fence.safe_end.max(parser.offset());
                        self.push(Frame::FenceChild(fence));
                        continue;
                    }
                    let before = *allowance;
                    let progress = fence.scan.advance(
                        parser.source(),
                        parser.cursor().end(),
                        final_input,
                        allowance,
                    );
                    self.child_work += before - *allowance;
                    match progress {
                        DelimiterProgress::Grapheme(range) => {
                            self.push(Frame::FenceAdvance(fence, range))
                        }
                        DelimiterProgress::Found(end) | DelimiterProgress::End(end) => {
                            fence.safe_end = end;
                            fence.sealed = true;
                            self.push(Frame::FenceChild(fence));
                        }
                        DelimiterProgress::NeedsProcessing => {
                            self.push(Frame::FenceScan(fence));
                            return Progress::NeedsProcessing;
                        }
                        DelimiterProgress::NeedInput => {
                            self.push(Frame::FenceScan(fence));
                            return Progress::NeedInput;
                        }
                        DelimiterProgress::InvalidSource => unreachable!("valid fence scan input"),
                    }
                }
                Frame::FenceAdvance(mut fence, range) => {
                    let _ = parser.charge();
                    fence.safe_end = range.end;
                    self.push(Frame::FenceChild(fence));
                }
                Frame::FenceChild(mut fence) => {
                    if parser.is_halted() && input_final {
                        fence.sealed = true;
                    }
                    let outer = parser.enter_cursor_scope(fence.safe_end);
                    // This scope is restored before every yield. Its known-safe
                    // body frontier can grow; only the delimiter declares EOF.
                    parser.state.cursor_frontier = !fence.sealed;
                    parser.state.context_frontier = !fence.sealed;
                    let before = *allowance;
                    let progress = fence.child.advance(parser, fence.sealed, allowance);
                    self.child_work += before - *allowance;
                    parser.leave_cursor_scope(outer);
                    match progress {
                        Progress::Complete(result) => self.result = result,
                        Progress::NeedInput => {
                            self.push(Frame::FenceScan(fence));
                        }
                        Progress::NeedsProcessing => {
                            self.push(Frame::FenceChild(fence));
                            return Progress::NeedsProcessing;
                        }
                        Progress::Limited => {
                            self.push(Frame::FenceChild(fence));
                            if !input_final {
                                return Progress::Limited;
                            }
                        }
                    }
                }
                Frame::FenceResult => {
                    if !final_input && parser.is_eof() && !parser.is_halted() {
                        self.push(Frame::FenceResult);
                        return Progress::NeedInput;
                    }
                    if parser.is_halted() {
                        self.result = Attempt::Committed;
                    } else if self.result == Attempt::NoMatch {
                        let body = parser.start();
                        self.push(Frame::FenceFallback(body));
                        self.push(Frame::CallRule(rules::WHITESPACE0));
                    } else if !parser.is_eof() {
                        self.push(Frame::Abandon(recovery::AbandonContinuation::new(
                            rules::MECH_CODE,
                            "syntax/unexpected-fenced-mech-source",
                            "unexpected source after executable fence code",
                        )));
                    }
                }
                Frame::FenceFallback(body) => {
                    if !final_input && parser.is_eof() && !parser.is_halted() {
                        self.push(Frame::FenceFallback(body));
                        return Progress::NeedInput;
                    }
                    self.push(Frame::FenceComplete(body));
                    if parser.is_eof() {
                        self.result = Attempt::Matched;
                    } else {
                        self.push(Frame::Abandon(recovery::AbandonContinuation::new(
                            rules::MECH_CODE,
                            "syntax/invalid-fenced-mech",
                            "expected canonical Mech code in the executable fence",
                        )));
                    }
                }
                Frame::FenceComplete(body) => {
                    body.complete(parser, SyntaxKind::MechCode);
                }
                Frame::Sequence(mut frame) => {
                    let item = &frame.items[frame.index];
                    match self.result {
                        Attempt::Matched => {
                            if matches!(item, GrammarExpression::Rule(rule) if *rule == rules::CODEBLOCK_SIGIL || *rule == rules::MIKA_SECTION_OPEN)
                                || frame.inline && frame.index == 1
                            {
                                frame.distinctive = true;
                            }
                        }
                        Attempt::Committed => frame.committed = true,
                        Attempt::NoMatch
                            if matches!(
                                item,
                                GrammarExpression::Builtin("matching-codeblock-sigil")
                            ) && self.state.codeblock_delimiter.is_some() =>
                        {
                            let token = match self.state.codeblock_delimiter {
                                Some(rule) if rule == rules::GRAVE_CODEBLOCK_SIGIL => {
                                    SyntaxKind::GraveCodeBlockSigil
                                }
                                Some(rule) if rule == rules::TILDE_CODEBLOCK_SIGIL => {
                                    SyntaxKind::TildeCodeBlockSigil
                                }
                                _ => unreachable!("closed codeblock delimiter state"),
                            };
                            self.push(Frame::Sequence(frame));
                            self.push(Frame::Missing(recovery::MissingContinuation::new(
                                "syntax/missing-codeblock-sigil",
                                "expected a closing codeblock delimiter matching the opener",
                                ExpectedSyntax::Token(token),
                                Some(token),
                            )));
                            continue;
                        }
                        Attempt::NoMatch
                            if frame.distinctive && required_sequence_item(item).is_some() =>
                        {
                            self.push(Frame::Sequence(frame));
                            self.push(Frame::Missing(
                                required_sequence_item(item).expect("required item"),
                            ));
                            continue;
                        }
                        Attempt::NoMatch if frame.committed || frame.distinctive => {
                            self.result = Attempt::Committed;
                            continue;
                        }
                        Attempt::NoMatch => {
                            parser.rewind(frame.checkpoint);
                            self.state = frame.initial;
                            continue;
                        }
                    }
                    frame.index += 1;
                    if frame.index == frame.items.len() {
                        self.result = Self::aggregate(frame.committed);
                    } else {
                        self.sequence(parser, frame);
                    }
                }
                Frame::Choice(mut frame) => {
                    if parser.is_halted() {
                        self.result = Attempt::Committed;
                    } else if !self.result.accepted() {
                        parser.rewind(frame.checkpoint);
                        self.state = frame.initial;
                        frame.index += 1;
                        if let Some(item) = frame.items.get(frame.index) {
                            self.push(Frame::Choice(frame));
                            self.push(Frame::Expression(item));
                        }
                    }
                }
                Frame::BestChoice(mut frame, mut selected) => {
                    if parser.is_halted() {
                        self.result = Attempt::Committed;
                        continue;
                    }
                    if self.result == Attempt::Committed && selected.is_none() {
                        continue;
                    }
                    if self.result == Attempt::Matched {
                        let consumed = (parser.offset() - frame.checkpoint.cursor.offset).0;
                        if selected.is_none_or(|(_, old)| consumed > old) {
                            selected = Some((frame.index, consumed));
                        }
                    }
                    frame.index += 1;
                    parser.rewind(frame.checkpoint);
                    self.state = frame.initial;
                    if let Some(item) = frame.items.get(frame.index) {
                        self.push(Frame::BestChoice(frame, selected));
                        self.push(Frame::Expression(item));
                    } else if let Some((index, _)) = selected {
                        self.push(Frame::Expression(&frame.items[index]));
                    } else {
                        self.result = Attempt::NoMatch;
                    }
                }
                Frame::Optional(checkpoint, initial) => {
                    if self.result == Attempt::NoMatch {
                        parser.rewind(checkpoint);
                        self.state = initial;
                        self.result = Attempt::Matched;
                    }
                }
                Frame::Lookahead(checkpoint, initial, negate) => {
                    parser.rewind(checkpoint);
                    self.state = initial;
                    self.result = if self.result.accepted() != negate {
                        Attempt::Matched
                    } else {
                        Attempt::NoMatch
                    };
                }
                Frame::Repetition(mut frame) => {
                    let no_progress = parser.offset() == frame.checkpoint.cursor.offset;
                    if !self.result.accepted()
                        || no_progress && (!frame.require_one || frame.count > 0)
                    {
                        parser.rewind(frame.checkpoint);
                        self.state = frame.initial;
                        self.result = if frame.require_one && frame.count == 0 {
                            Attempt::NoMatch
                        } else {
                            Self::aggregate(frame.committed)
                        };
                    } else {
                        frame.count = frame.count.saturating_add(1);
                        frame.committed |= self.result == Attempt::Committed;
                        if no_progress || parser.is_halted() {
                            self.result = Self::aggregate(frame.committed);
                        } else {
                            self.repeat(parser, frame);
                        }
                    }
                }
                Frame::FirstSeparated(separator, item) => {
                    if self.result.accepted() {
                        self.separated(
                            parser,
                            Separated {
                                separator,
                                item,
                                committed: self.result == Attempt::Committed,
                                checkpoint: parser.checkpoint(),
                                initial: self.state,
                            },
                        );
                    } else {
                        self.result = Attempt::NoMatch;
                    }
                }
                Frame::Separator(frame) => {
                    if self.result.accepted() {
                        self.push(Frame::SeparatedItem(
                            frame,
                            self.result == Attempt::Committed,
                        ));
                        self.push(Frame::Expression(frame.item));
                    } else {
                        parser.rewind(frame.checkpoint);
                        self.state = frame.initial;
                        self.result = Self::aggregate(frame.committed);
                    }
                }
                Frame::SeparatedItem(mut frame, separator_committed) => {
                    if !self.result.accepted() {
                        parser.rewind(frame.checkpoint);
                        self.state = frame.initial;
                        self.result = Self::aggregate(frame.committed);
                    } else {
                        frame.committed |= separator_committed || self.result == Attempt::Committed;
                        if parser.offset() == frame.checkpoint.cursor.offset || parser.is_halted() {
                            self.result = Self::aggregate(frame.committed);
                        } else {
                            self.separated(parser, frame);
                        }
                    }
                }
            }
        }
        Progress::Complete(self.result)
    }

    fn expression(&mut self, parser: &mut Parser<'_>, expression: &'g GrammarExpression) {
        if parser.is_halted() {
            self.result = Attempt::Committed;
            return;
        }
        match expression {
            GrammarExpression::Rule(rule) => {
                if *rule == rules::MIKA_SECTION_CLOSE && !cfg!(feature = "mika") {
                    self.result = Attempt::NoMatch;
                    return;
                }
                self.push(Frame::NamedRuleResult(*rule, parser.offset()));
                self.push(Frame::CallRule(*rule));
            }

            GrammarExpression::Builtin(name) => self.push(Frame::Builtin(name)),
            GrammarExpression::Literal(literal) => {
                if let Some(scan) = LiteralScan::new(
                    literal,
                    parser.offset(),
                    (!parser.state.context_frontier).then_some(parser.cursor().context_end()),
                ) {
                    self.push(Frame::Literal(scan));
                } else {
                    self.result = Attempt::NoMatch;
                }
            }
            GrammarExpression::MikaExpressionTriples(triples) => {
                self.push(Frame::MikaStart(triples, 0))
            }
            GrammarExpression::Empty => self.result = Attempt::Matched,
            GrammarExpression::Sequence(items) => {
                if items.is_empty() {
                    self.result = Attempt::Matched;
                    return;
                }
                let inline = matches!((items.first(), items.get(1)), (Some(GrammarExpression::Rule(left)), Some(GrammarExpression::Rule(right)))
                    if *left == rules::LEFT_BRACE && *right == rules::LEFT_BRACE);
                self.sequence(
                    parser,
                    Sequence {
                        items,
                        index: 0,
                        checkpoint: parser.checkpoint(),
                        initial: self.state,
                        committed: false,
                        distinctive: false,
                        inline,
                    },
                );
            }
            GrammarExpression::Choice(items) | GrammarExpression::BestChoice(items) => {
                if items.is_empty() {
                    self.result = Attempt::NoMatch;
                    return;
                }
                let frame = Choice {
                    items,
                    index: 0,
                    checkpoint: parser.checkpoint(),
                    initial: self.state,
                };
                if matches!(expression, GrammarExpression::BestChoice(_)) {
                    parser.rewind(frame.checkpoint);
                    self.push(Frame::BestChoice(frame, None));
                } else {
                    self.push(Frame::Choice(frame));
                }
                self.push(Frame::Expression(&items[0]));
            }
            GrammarExpression::Optional(item) => {
                self.push(Frame::Optional(parser.checkpoint(), self.state));
                self.push(Frame::Expression(item));
            }
            GrammarExpression::Peek(item) | GrammarExpression::Not(item) => {
                self.push(Frame::Lookahead(
                    parser.checkpoint(),
                    self.state,
                    matches!(expression, GrammarExpression::Not(_)),
                ));
                self.push(Frame::Expression(item));
            }
            GrammarExpression::ZeroOrMore(item) | GrammarExpression::OneOrMore(item) => {
                self.repeat(
                    parser,
                    Repetition {
                        item,
                        require_one: matches!(expression, GrammarExpression::OneOrMore(_)),
                        count: 0,
                        committed: false,
                        checkpoint: parser.checkpoint(),
                        initial: self.state,
                    },
                );
            }
            GrammarExpression::Separated { separator, item } => {
                self.push(Frame::FirstSeparated(separator, item));
                self.push(Frame::Expression(item));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::parser::LexicalMode;
    use crate::document::{DocumentId, IdGenerator, ParseConfig, Revision, TextSize, TextSnapshot};

    use crate::document::parser::canonical::continuation_test_support::{
        TestContinuation, TestProgress, assert_partitions,
    };
    impl TestContinuation for Continuation<'static> {
        fn new(rule: RuleId) -> Self {
            Self::for_rule(
                DOCUMENT_RULES
                    .iter()
                    .find(|spec| spec.rule == rule)
                    .unwrap(),
            )
        }
        fn work(&self) -> u64 {
            self.steps + self.child_work
        }
        fn advance(
            &mut self,
            parser: &mut Parser<'_>,
            final_input: bool,
            allowance: &mut u64,
        ) -> TestProgress {
            match self.advance(parser, final_input, allowance) {
                Progress::Complete(result) => TestProgress::Complete(result),
                Progress::NeedInput => TestProgress::NeedInput,
                Progress::NeedsProcessing => TestProgress::NeedsProcessing,
                Progress::Limited => TestProgress::Limited,
            }
        }
    }
    #[test]
    #[cfg(any(feature = "base", feature = "full"))]
    fn document_rules_preserve_all_open_input_cuts() {
        assert_partitions::<Continuation>(&[
            (rules::PARSE, "hello world\n"),
            (rules::PARSE, "x := [1,2]\ny := x + 3\n"),
            (rules::PARSE, "```mech\nx := 1\n```\n"),
            (rules::PARSE, "~~~text\nhello\n~~~\n"),
            (rules::PARSE, "Heading\n=======\ntext {x + 1}\n"),
            (rules::PARSE, "```mech\nx := [1,\n"),
            (rules::MECH_CODE_ALT, "-- comment\n"),
            (rules::MECH_CODE_ALT, "--x + 2\n"),
            (rules::EVAL_INLINE_MECH_CODE, "{x + 1}"),
            (rules::EVAL_INLINE_MECH_CODE, "{x +}"),
            (rules::PARSE, "╭◉╮\n(◉ ◯ ◉)\n"),
        ]);
    }

    fn run(
        expression: &GrammarExpression,
        text: &str,
        fuel: u64,
        step: bool,
    ) -> (
        Attempt,
        TextSize,
        alloc::string::String,
        alloc::string::String,
        crate::document::ParseStats,
    ) {
        let source = TextSnapshot::new(DocumentId(826), Revision(0), text).unwrap();
        let mut ids = IdGenerator::new();
        let mut config = ParseConfig::default();
        config.limits.fuel = fuel;
        let mut parser = Parser::new(
            &source,
            LexicalMode::CanonicalSourceFragment,
            config,
            &mut ids,
        );
        let root = parser.start();
        let mut continuation = Continuation::new(expression, GrammarState::default());
        let result = loop {
            let before = continuation.steps + continuation.child_work;
            let mut allowance = if step { 1 } else { u64::MAX };
            let result = continuation.advance(&mut parser, true, &mut allowance);
            if step {
                assert!(continuation.steps + continuation.child_work - before <= 1);
            }
            assert!(
                continuation.steps < 10_000,
                "continuation did not make bounded progress"
            );
            match result {
                Progress::Complete(result) => break result,
                Progress::NeedInput | Progress::Limited => {
                    panic!("finite document input must drain")
                }
                Progress::NeedsProcessing => {
                    let state = parser.suspend();
                    parser = Parser::resume(&source, state, &mut ids);
                }
            }
        };
        let consumed = parser.offset();
        root.complete(&mut parser, SyntaxKind::Document);
        let output = parser.finish();
        assert!(continuation.peak_frames < 32);
        (
            result,
            consumed,
            alloc::format!("{:?}", output.events),
            alloc::format!(
                "{:?}",
                output
                    .diagnostics
                    .iter()
                    .map(|pending| &pending.diagnostic)
                    .collect::<Vec<_>>()
            ),
            output.stats,
        )
    }

    #[test]
    #[cfg(any(feature = "base", feature = "full"))]
    fn all_document_rule_phases_survive_one_transition_resumes() {
        let table = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/design/grammar-audit/s7-document-certification.tsv"
        ));
        let mut count = 0;
        for row in table.lines().skip(1) {
            let fields: Vec<_> = row.split('\t').collect();
            let text: alloc::string::String = serde_json::from_str(fields[1]).unwrap();
            let rule = crate::document::parser::canonical_rule_id(fields[0]).unwrap();
            let specification = DOCUMENT_RULES
                .iter()
                .find(|spec| spec.rule == rule)
                .unwrap();
            let mut baseline = None;
            for step in [false, true] {
                let source =
                    TextSnapshot::new(DocumentId(826), Revision(0), text.as_str()).unwrap();
                let mut ids = IdGenerator::new();
                let mut parser = Parser::new(
                    &source,
                    LexicalMode::CanonicalSourceFragment,
                    ParseConfig::default(),
                    &mut ids,
                );
                let wrapper = parser.start();
                let mut continuation = Continuation::for_rule(specification);
                let result = loop {
                    let before = continuation.steps + continuation.child_work;
                    let mut allowance = if step { 1 } else { u64::MAX };
                    let result = continuation.advance(&mut parser, true, &mut allowance);
                    if step {
                        assert!(continuation.steps + continuation.child_work - before <= 1);
                    }
                    assert!(
                        continuation.steps < 100_000,
                        "{} failed to progress",
                        fields[0]
                    );
                    match result {
                        Progress::Complete(result) => break result,
                        Progress::NeedInput | Progress::Limited => {
                            panic!("finite document input must drain")
                        }
                        Progress::NeedsProcessing => {
                            let state = parser.suspend();
                            parser = Parser::resume(&source, state, &mut ids);
                        }
                    }
                };
                assert_eq!(result, Attempt::Matched, "{}", fields[0]);
                assert_eq!(parser.offset(), source.byte_len(), "{}", fields[0]);
                assert_eq!(parser.state.rules.len(), 0);
                wrapper.complete(&mut parser, SyntaxKind::Document);
                let output = parser.finish();
                let observed = (alloc::format!("{:?}", output.events), output.stats);
                assert!(output.diagnostics.is_empty(), "{}", fields[0]);
                if let Some(baseline) = &baseline {
                    assert_eq!(&observed, baseline, "{}", fields[0]);
                } else {
                    baseline = Some(observed);
                }
            }
            count += 1;
        }
        assert_eq!(count, 112);
    }

    #[test]
    fn scheduling_yields_preserve_alternatives_repetition_and_resource_recovery() {
        const A: GrammarExpression = GrammarExpression::Literal("a");
        const AB: GrammarExpression = GrammarExpression::Literal("ab");
        const COMMA: GrammarExpression = GrammarExpression::Literal(",");
        const EMPTY: GrammarExpression = GrammarExpression::Empty;
        let cases = [
            (
                GrammarExpression::Sequence(&[
                    GrammarExpression::Peek(&AB),
                    GrammarExpression::Not(&COMMA),
                    AB,
                ]),
                "ab",
                Attempt::Matched,
                2,
            ),
            (
                GrammarExpression::Sequence(&[
                    GrammarExpression::Optional(&AB),
                    GrammarExpression::OneOrMore(&A),
                ]),
                "aaa",
                Attempt::Matched,
                3,
            ),
            (
                GrammarExpression::Choice(&[AB, A]),
                "a!",
                Attempt::Matched,
                1,
            ),
            (
                GrammarExpression::BestChoice(&[A, AB]),
                "ab!",
                Attempt::Matched,
                2,
            ),
            (
                GrammarExpression::Separated {
                    separator: &COMMA,
                    item: &A,
                },
                "a,a,a!",
                Attempt::Matched,
                5,
            ),
            (
                GrammarExpression::Separated {
                    separator: &COMMA,
                    item: &A,
                },
                "a,",
                Attempt::Matched,
                1,
            ),
            (
                GrammarExpression::ZeroOrMore(&EMPTY),
                "",
                Attempt::Matched,
                0,
            ),
            (
                GrammarExpression::OneOrMore(&EMPTY),
                "",
                Attempt::Matched,
                0,
            ),
            (
                GrammarExpression::Sequence(&[A, AB]),
                "ax",
                Attempt::NoMatch,
                0,
            ),
        ];
        for (expression, source, expected, consumed) in cases {
            let full = run(&expression, source, 4_000_000, false);
            assert_eq!(
                (full.0, full.1),
                (expected, TextSize(consumed)),
                "{source:?}"
            );
            assert_eq!(full, run(&expression, source, 4_000_000, true));
            for fuel in [0, 1, 2, 4, 8, 16] {
                assert_eq!(
                    run(&expression, source, fuel, false),
                    run(&expression, source, fuel, true),
                    "fuel={fuel}, source={source:?}"
                );
            }
        }
    }
}
