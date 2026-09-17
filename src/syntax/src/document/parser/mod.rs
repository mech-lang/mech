use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::document::{
    Diagnostic, DiagnosticAnchor, DiagnosticCode, DiagnosticPhase, DiagnosticStore, DiagnosticTags,
    FoundSyntax, GreenBuilder, GreenNode, IdGenerator, NodeFlags, ParseStats, ParserContextId,
    RecoveryAction, RestartEntry, RestartIndex, RestartMode, RuleId, Severity, SyntaxKind,
    SyntaxSnapshot, TextRange, TextSize, TextSnapshot, TokenFlags,
};

pub mod canonical;
mod canonical_ports;
mod canonical_rules;
pub mod checkpoint;
mod context_probe;
pub mod cursor;
mod delimiter_scan;
pub mod document;
pub mod event;
pub mod fragment;
mod grapheme_scan;
pub mod limits;
mod literal_scan;
pub mod marker;
pub mod mech;
pub mod mechdown;
pub mod recovery;
mod resource_found;
pub mod rule;
pub mod terminal;

pub use checkpoint::*;
pub use cursor::*;
pub use event::*;
pub use fragment::*;
pub use limits::*;
pub use marker::*;
pub use recovery::*;
pub use rule::*;

struct PendingDiagnostic {
    diagnostic: Diagnostic,
    event: Option<usize>,
    relative: TextRange,
}

struct ParserOutput {
    events: Vec<Event>,
    diagnostics: Vec<PendingDiagnostic>,
    stats: ParseStats,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParserImplementation {
    Prototype,
    Canonical,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParseRoot {
    Document,
    Grammar,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParseRequestError {
    Unsupported {
        implementation: ParserImplementation,
        root: ParseRoot,
    },
}

/// Internal lexical classification selected by the parser entry point.
///
/// Canonical grammar parsing intentionally ignores grammar-level whitespace,
/// while standalone canonical productions classify the physical source exactly
/// as supplied. Diagnostic attribution is deliberately independent of this
/// mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LexicalMode {
    PrototypeDocument,
    CanonicalGrammar,
    CanonicalSourceFragment,
}

pub(crate) struct Parser<'a> {
    source: &'a TextSnapshot,
    cursor: Cursor<'a>,
    ids: &'a mut IdGenerator,
    state: ParserState,
}

/// Borrow-free cursor scope retained by an embedded grammar continuation.
#[derive(Clone, Copy)]
pub(crate) struct CursorScope {
    consume_end: TextSize,
    context_end: TextSize,
    cursor_frontier: bool,
    context_frontier: bool,
}

/// Input-independent ownership of a live parse. Moving this state never
/// finalizes events, loses rule/marker context, or replenishes resource fuel.
pub(crate) struct ParserState {
    lexical_mode: LexicalMode,
    parse_range: TextRange,
    events: Vec<Event>,
    open_markers: Vec<usize>,
    covered_end: TextSize,
    diagnostics: Vec<PendingDiagnostic>,
    rules: RuleStack,
    config: ParseConfig,
    fuel: u64,
    nesting: u32,
    halted: bool,
    resource_diagnostic_emitted: bool,
    resource_found: Option<resource_found::Continuation>,
    resource_finalizing: bool,
    allow_consuming_recovery: bool,
    resource_rule: Option<RuleId>,
    stats: ParseStats,
    cursor: cursor::CursorCheckpoint,
    cursor_end: TextSize,
    context_end: TextSize,
    input_frontier: bool,
    cursor_frontier: bool,
    context_frontier: bool,
}

pub(crate) struct CleanSubtree {
    start: TextSize,
    end: TextSize,
    node: Arc<GreenNode>,
}

impl ParserState {
    fn new(
        range: TextRange,
        context_end: TextSize,
        lexical_mode: LexicalMode,
        config: ParseConfig,
        input_frontier: bool,
    ) -> Self {
        Self {
            lexical_mode,
            parse_range: range,
            events: Vec::new(),
            open_markers: Vec::new(),
            covered_end: range.start,
            diagnostics: Vec::new(),
            rules: RuleStack::default(),
            config,
            fuel: config.limits.fuel,
            nesting: 0,
            halted: false,
            resource_diagnostic_emitted: false,
            resource_found: None,
            resource_finalizing: false,
            allow_consuming_recovery: true,
            resource_rule: None,
            stats: ParseStats {
                source_bytes: u64::from(range.len().0),
                ..ParseStats::default()
            },
            cursor: cursor::CursorCheckpoint {
                offset: range.start,
            },
            cursor_end: range.end,
            context_end,
            input_frontier,
            cursor_frontier: input_frontier,
            context_frontier: true,
        }
    }
}

impl<'a> Parser<'a> {
    fn new(
        source: &'a TextSnapshot,
        lexical_mode: LexicalMode,
        config: ParseConfig,
        ids: &'a mut IdGenerator,
    ) -> Self {
        let state = ParserState::new(
            source.full_range(),
            source.byte_len(),
            lexical_mode,
            config,
            true,
        );
        Self::resume(source, state, ids)
    }

    fn for_range(
        source: &'a TextSnapshot,
        range: TextRange,
        lexical_mode: LexicalMode,
        resource_rule: Option<RuleId>,
        initial_nesting: u32,
        config: ParseConfig,
        ids: &'a mut IdGenerator,
    ) -> Self {
        let mut state = ParserState::new(range, source.byte_len(), lexical_mode, config, false);
        state.nesting = initial_nesting;
        state.resource_rule = resource_rule;
        state.halted = source.validate_range(range).is_err();
        Self::resume(source, state, ids)
    }

    /// The continuation owner accepts append-only input; an edit must discard
    /// this state. Fixed embedded-body bounds never expand with the frontier.
    fn resume(source: &'a TextSnapshot, mut state: ParserState, ids: &'a mut IdGenerator) -> Self {
        if state.input_frontier {
            state.parse_range.end = source.byte_len();
        }
        if state.cursor_frontier {
            state.cursor_end = source.byte_len();
        }
        if state.context_frontier {
            state.context_end = source.byte_len();
        }
        state.stats.source_bytes = u64::from(state.parse_range.len().0);
        let cursor = Cursor::for_range_with_context(
            source,
            TextRange::new(state.cursor.offset, state.cursor_end),
            state.context_end,
        );
        Self {
            source,
            cursor,
            ids,
            state,
        }
    }

    fn suspend(mut self) -> ParserState {
        self.state.cursor = self.cursor.checkpoint();
        self.state.cursor_end = self.cursor.end();
        self.state.context_end = self.cursor.context_end();
        self.state
    }

    /// The continuation owns this policy until its speculative child completes,
    /// including across input and processing yields. Restore the returned policy
    /// before parsing the selected alternative.
    pub(crate) fn replace_consuming_recovery(&mut self, allowed: bool) -> bool {
        core::mem::replace(&mut self.state.allow_consuming_recovery, allowed)
    }

    pub(crate) fn consuming_recovery_allowed(&self) -> bool {
        self.state.allow_consuming_recovery
    }

    pub(crate) fn source(&self) -> &TextSnapshot {
        self.source
    }

    pub(crate) fn set_resource_rule(&mut self, rule: RuleId) {
        self.state.resource_rule = Some(rule);
    }

    pub(crate) fn cursor(&self) -> &Cursor<'a> {
        &self.cursor
    }

    /// Parse an embedded body in the same source and event stream. The body
    /// shares every resource budget with its enclosing document. Exhaustion
    /// still finalizes the enclosing parse range, including the owned closer.
    pub(crate) fn enter_cursor_scope(&mut self, end: TextSize) -> CursorScope {
        let outer = CursorScope {
            consume_end: self.cursor.end(),
            context_end: self.cursor.context_end(),
            cursor_frontier: self.state.cursor_frontier,
            context_frontier: self.state.context_frontier,
        };
        self.state.cursor_frontier = false;
        self.state.context_frontier = false;
        self.cursor =
            Cursor::for_range_with_context(self.source, TextRange::new(self.offset(), end), end);
        outer
    }

    pub(crate) fn leave_cursor_scope(&mut self, outer: CursorScope) {
        let checkpoint = self.cursor.checkpoint();
        let end = if outer.cursor_frontier {
            self.source.byte_len()
        } else {
            outer.consume_end
        };
        let context = if outer.context_frontier {
            self.source.byte_len()
        } else {
            outer.context_end
        };
        // Resource finalization may own a document remainder beyond an embedded
        // consume bound; restoration preserves that cursor exactly.
        self.cursor = Cursor::for_range_with_context(
            self.source,
            TextRange::new(TextSize::ZERO, end),
            context,
        );
        self.cursor.rewind(checkpoint);
        self.state.cursor_frontier = outer.cursor_frontier;
        self.state.context_frontier = outer.context_frontier;
    }

    pub(crate) fn config(&self) -> ParseConfig {
        self.state.config
    }

    pub(crate) fn offset(&self) -> TextSize {
        self.cursor.offset()
    }

    pub(crate) fn is_eof(&self) -> bool {
        self.cursor.is_eof()
    }

    pub(crate) fn is_halted(&self) -> bool {
        self.state.halted
    }

    pub(crate) fn halt(&mut self) {
        self.state.halted = true;
    }

    pub(crate) fn stats(&self) -> ParseStats {
        self.state.stats
    }

    pub(crate) fn stats_mut(&mut self) -> &mut ParseStats {
        &mut self.state.stats
    }

    pub(crate) fn start(&mut self) -> Marker {
        if self.state.halted || self.state.resource_finalizing {
            return Marker {
                position: usize::MAX,
            };
        }
        let open_after = self.state.open_markers.len().saturating_add(1);
        let position = self
            .emit(Event::Tombstone, open_after)
            .unwrap_or(usize::MAX);
        if position != usize::MAX {
            self.state.open_markers.push(position);
        }
        Marker { position }
    }

    pub(crate) fn complete_marker(
        &mut self,
        marker: Marker,
        kind: SyntaxKind,
        flags: NodeFlags,
    ) -> CompletedMarker {
        if marker.position == usize::MAX {
            return CompletedMarker {
                position: usize::MAX,
                kind,
            };
        }
        if self.state.halted && !self.state.resource_finalizing {
            self.consume_resource_remainder();
        }
        if self.state.open_markers.is_empty() {
            return CompletedMarker {
                position: usize::MAX,
                kind,
            };
        }
        assert_eq!(
            self.state.open_markers.last().copied(),
            Some(marker.position),
            "parser markers must complete in strict LIFO order"
        );
        if let Some(event) = self.state.events.get_mut(marker.position) {
            *event = Event::Start { kind, flags };
        }
        let open_after = self.state.open_markers.len().saturating_sub(1);
        let finish = if self.state.resource_finalizing {
            self.emit_emergency(Event::Finish)
        } else {
            self.emit(Event::Finish, open_after)
        };
        assert!(
            finish.is_some(),
            "accepted marker start must reserve capacity for its finish"
        );
        self.state.open_markers.pop();
        CompletedMarker {
            position: marker.position,
            kind,
        }
    }

    pub(crate) fn abandon_marker(&mut self, marker: Marker) {
        if marker.position == usize::MAX {
            return;
        }
        assert!(
            !self.state.resource_finalizing,
            "resource finalization cannot abandon an enclosing parser marker"
        );
        assert_eq!(
            self.state.open_markers.last().copied(),
            Some(marker.position),
            "parser markers must abandon in strict LIFO order"
        );
        self.state.open_markers.pop();
        if marker.position + 1 == self.state.events.len() {
            self.state.events.pop();
        } else if let Some(event) = self.state.events.get_mut(marker.position) {
            *event = Event::Tombstone;
        }
    }

    pub(crate) fn checkpoint(&self) -> ParserCheckpoint {
        ParserCheckpoint {
            cursor: self.cursor.checkpoint(),
            events: self.state.events.len(),
            diagnostics: self.state.diagnostics.len(),
            open_markers: self.state.open_markers.len(),
            covered_end: self.state.covered_end,
            rule_depth: self.state.rules.len(),
            nesting: self.state.nesting,
        }
    }

    pub(crate) fn rewind(&mut self, checkpoint: ParserCheckpoint) {
        // Resource finalization has assigned the remaining source to an ERROR
        // envelope. Rejected lookahead cannot discard it or refund parser work.
        if self.state.resource_finalizing {
            // The rejected candidate may have left provisional markers for its
            // transaction to discard. Keep finalized children and source, but
            // remove those unselected wrappers before its enclosing owner ends.
            while self.state.open_markers.len() > checkpoint.open_markers {
                let position = self
                    .state
                    .open_markers
                    .pop()
                    .expect("marker above checkpoint");
                self.state.events[position] = Event::Tombstone;
            }
            return;
        }
        self.cursor.rewind(checkpoint.cursor);
        self.state.events.truncate(checkpoint.events);
        self.state.diagnostics.truncate(checkpoint.diagnostics);
        self.state.open_markers.truncate(checkpoint.open_markers);
        self.state.covered_end = checkpoint.covered_end;
        self.state.rules.truncate(checkpoint.rule_depth);
        self.state.nesting = checkpoint.nesting;
    }

    pub(crate) fn cache_clean_subtree(
        &mut self,
        start: ParserCheckpoint,
        end: ParserCheckpoint,
    ) -> Option<CleanSubtree> {
        if start.cursor.offset >= end.cursor.offset
            || start.events >= end.events
            || start.diagnostics != end.diagnostics
            || start.open_markers != end.open_markers
            || start.rule_depth != end.rule_depth
            || start.nesting != end.nesting
        {
            return None;
        }
        let node = sink(
            self.state.events.get(start.events..end.events)?,
            self.source,
            self.ids,
        )
        .ok()?
        .root;
        (node.text_len == end.cursor.offset - start.cursor.offset).then_some(CleanSubtree {
            start: start.cursor.offset,
            end: end.cursor.offset,
            node,
        })
    }

    pub(crate) fn reuse_clean_subtree(&mut self, subtree: &CleanSubtree) -> bool {
        if self.offset() != subtree.start || self.state.halted || self.state.resource_finalizing {
            return false;
        }
        let open_after = self.state.open_markers.len();
        if self
            .emit(
                Event::Reuse {
                    node: subtree.node.clone(),
                },
                open_after,
            )
            .is_none()
        {
            return false;
        }
        self.cursor.rewind(CursorCheckpoint {
            offset: subtree.end,
        });
        self.state.covered_end = self.state.covered_end.max(subtree.end);
        self.state.stats.reused_node_count = self.state.stats.reused_node_count.saturating_add(1);
        true
    }

    pub(crate) fn with_rule<T>(
        &mut self,
        context: ParserContextId,
        canonical: Option<RuleId>,
        parse: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let depth = self.state.rules.len();
        self.state.rules.push(context, canonical);
        let result = parse(self);
        self.state.rules.truncate(depth);
        result
    }

    pub(crate) fn with_canonical_rule<T>(
        &mut self,
        rule: RuleId,
        parse: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let depth = self.state.rules.len();
        self.state.rules.push_canonical(rule);
        let result = parse(self);
        self.state.rules.truncate(depth);
        result
    }

    pub(crate) fn current_rule(&self) -> Option<RuleId> {
        self.state.rules.current_rule()
    }

    pub(crate) fn current_context(&self) -> Option<ParserContextId> {
        self.state.rules.current_context()
    }

    #[cfg(test)]
    pub(crate) fn rule_depth(&self) -> usize {
        self.state.rules.len()
    }

    pub(crate) fn bump_char_raw(&mut self) -> Option<(char, TextRange)> {
        if !self.charge() {
            return None;
        }
        self.cursor.bump_char()
    }

    pub(crate) fn bump_grapheme_raw(&mut self) -> Option<TextRange> {
        if !self.charge() {
            return None;
        }
        self.cursor.bump_grapheme()
    }

    pub(crate) fn bump_bytes_token(&mut self, count: u32, kind: SyntaxKind) -> Option<TextRange> {
        if !self.charge() {
            return None;
        }
        let range = self.cursor.bump_bytes(count)?;
        self.token(kind, range);
        Some(range)
    }

    #[cfg(test)]
    pub(crate) fn bump_char_token(&mut self, kind: SyntaxKind) -> Option<TextRange> {
        let (_, range) = self.bump_char_raw()?;
        self.token(kind, range);
        Some(range)
    }

    pub(crate) fn token(&mut self, kind: SyntaxKind, range: TextRange) {
        self.token_with_flags(kind, range, TokenFlags::NONE);
    }

    pub(crate) fn token_with_flags(
        &mut self,
        kind: SyntaxKind,
        range: TextRange,
        flags: TokenFlags,
    ) {
        let _ = self.emit(
            Event::Token { kind, range, flags },
            self.state.open_markers.len(),
        );
    }

    pub(crate) fn missing_token(&mut self, kind: SyntaxKind) {
        let _ = self.emit(
            Event::Token {
                kind,
                range: TextRange::empty(self.offset()),
                flags: TokenFlags::SYNTHETIC | TokenFlags::MISSING,
            },
            self.state.open_markers.len(),
        );
    }

    pub(crate) fn next_diagnostic_id(&mut self) -> crate::document::DiagnosticId {
        self.ids.diagnostic()
    }

    pub(crate) fn push_diagnostic(
        &mut self,
        diagnostic: Diagnostic,
        event: Option<usize>,
        relative: TextRange,
    ) {
        if self.state.diagnostics.len() >= self.state.config.limits.max_diagnostics as usize {
            self.state.stats.diagnostics_truncated = true;
            return;
        }
        self.state.diagnostics.push(PendingDiagnostic {
            diagnostic,
            event,
            relative,
        });
    }

    pub(crate) fn last_diagnostic_mut(&mut self) -> Option<&mut Diagnostic> {
        self.state
            .diagnostics
            .last_mut()
            .map(|pending| &mut pending.diagnostic)
    }

    pub(crate) fn consume_horizontal_space(&mut self) -> Option<TextRange> {
        let start = self.offset();
        while self
            .cursor
            .peek_char()
            .is_some_and(terminal::is_horizontal_space)
        {
            let _ = self.bump_char_raw()?;
        }
        if self.offset() == start {
            return None;
        }
        let range = TextRange::new(start, self.offset());
        self.token(SyntaxKind::Whitespace, range);
        Some(range)
    }

    pub(crate) fn consume_newline(&mut self) -> Option<TextRange> {
        let count = match (self.cursor.byte(), self.cursor.byte_at(1)) {
            (Some(b'\r'), Some(b'\n')) => 2,
            (Some(b'\r' | b'\n'), _) => 1,
            _ => return None,
        };
        self.bump_bytes_token(count, SyntaxKind::Newline)
    }

    pub(crate) fn consume_syntax_whitespace(&mut self) {
        loop {
            if self.consume_horizontal_space().is_some() {
                continue;
            }
            if self.consume_newline().is_some() {
                continue;
            }
            break;
        }
    }

    pub(crate) fn found_syntax(&self) -> FoundSyntax {
        match self.state.lexical_mode {
            LexicalMode::PrototypeDocument => {
                let character = self.cursor.context_peek_char();
                if character.is_none() {
                    return FoundSyntax {
                        kind: Some(SyntaxKind::Eof),
                        text: None,
                    };
                }
                FoundSyntax {
                    kind: character.map(terminal::token_kind_for_char),
                    text: character.map(|character| character.to_string()),
                }
            }
            LexicalMode::CanonicalGrammar => canonical::found::found_syntax(self, self.offset()),
            LexicalMode::CanonicalSourceFragment => {
                canonical::found::source_found_syntax(self, self.offset())
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn nesting(&self) -> u32 {
        self.state.nesting
    }

    pub(crate) fn push_nesting(&mut self) -> bool {
        if self.state.nesting >= self.state.config.limits.max_nesting {
            return false;
        }
        self.state.nesting += 1;
        true
    }

    pub(crate) fn pop_nesting(&mut self) {
        self.state.nesting = self.state.nesting.saturating_sub(1);
    }

    pub(crate) fn is_fence_start(&self) -> bool {
        mechdown::fence_delimiter(self.cursor()).is_some()
    }

    pub(crate) fn is_context_fence_start(&self) -> bool {
        mechdown::fence_delimiter_context(self.cursor.context_view()).is_some()
    }

    pub(crate) fn is_strong_document_boundary(&self) -> bool {
        let context = self.cursor.context_view();
        context.is_line_start()
            && (mechdown::is_ul_subtitle_context(context) || self.is_context_fence_start())
    }

    pub(crate) fn consume_resource_remainder(&mut self) {
        if self.state.resource_finalizing {
            return;
        }
        self.state.resource_finalizing = true;
        let mut range = TextRange::new(self.state.covered_end, self.state.parse_range.end);
        self.cursor.rewind(CursorCheckpoint { offset: range.end });

        if self.state.config.limits.max_events < MIN_PREFIX_PRESERVING_EVENTS {
            self.state.events.clear();
            self.state.open_markers.clear();
            self.state.covered_end = self.state.parse_range.start;
            range = self.state.parse_range;
        } else if !range.is_empty() {
            let required = self.state.open_markers.len().saturating_add(3);
            assert!(
                self.state.events.len().saturating_add(required)
                    <= self.state.config.limits.max_events as usize,
                "ordinary events must reserve the resource envelope and open-marker finishes"
            );
            let _ = self.emit_emergency(Event::Start {
                kind: SyntaxKind::Error,
                flags: NodeFlags::ERROR,
            });
            let _ = self.emit_emergency(Event::Token {
                kind: SyntaxKind::Unknown,
                range,
                flags: TokenFlags::ERROR,
            });
            let _ = self.emit_emergency(Event::Finish);
            self.state.covered_end = range.end;
        }
        if !self.state.resource_diagnostic_emitted {
            self.state.resource_diagnostic_emitted = true;
            let rule = self.current_rule().or(self.state.resource_rule);
            let context = rule.is_none().then(|| self.current_context()).flatten();
            let found = if self.state.lexical_mode == LexicalMode::PrototypeDocument {
                Some(FoundSyntax {
                    kind: Some(SyntaxKind::Unknown),
                    text: None,
                })
            } else {
                None
            };
            let diagnostic_index = self.state.diagnostics.len();
            let deferred = found.is_none();
            let diagnostic = Diagnostic {
                id: self.next_diagnostic_id(),
                code: DiagnosticCode::syntax("recovery-limit"),
                phase: DiagnosticPhase::Syntax,
                severity: Severity::Error,
                rule,
                context,
                primary: DiagnosticAnchor::Absolute {
                    revision: self.source.revision(),
                    range,
                },
                labels: Vec::new(),
                expected: Vec::new(),
                found,
                fixes: Vec::new(),
                related: Vec::new(),
                recovery: Some(RecoveryAction::ResourceLimit { range }),
                tags: DiagnosticTags::NONE,
                message: String::from("parser resource limit reached"),
            };
            self.push_diagnostic(diagnostic, None, range);
            if deferred && self.state.diagnostics.len() > diagnostic_index {
                self.state.resource_found = Some(resource_found::Continuation::new(
                    self.state.lexical_mode,
                    diagnostic_index,
                    range.start,
                    self.cursor.context_end(),
                ));
            }
        }
        self.state.halted = true;
    }

    fn emit(&mut self, event: Event, open_after: usize) -> Option<usize> {
        if self.state.halted || self.state.resource_finalizing {
            self.state.halted = true;
            return None;
        }
        let emergency = open_after.saturating_add(3);
        if self
            .state
            .events
            .len()
            .saturating_add(1)
            .saturating_add(emergency)
            > self.state.config.limits.max_events as usize
        {
            self.state.halted = true;
            return None;
        }
        let position = self.state.events.len();
        if let Event::Token { range, flags, .. } = &event
            && !flags.contains(TokenFlags::SYNTHETIC)
        {
            self.state.covered_end = self.state.covered_end.max(range.end);
        }
        self.state.events.push(event);
        Some(position)
    }

    fn emit_emergency(&mut self, event: Event) -> Option<usize> {
        if self.state.events.len() >= self.state.config.limits.max_events as usize {
            return None;
        }
        let position = self.state.events.len();
        self.state.events.push(event);
        Some(position)
    }

    fn charge(&mut self) -> bool {
        if self.state.halted || self.state.fuel == 0 {
            self.state.halted = true;
            return false;
        }
        self.state.fuel -= 1;
        self.state.stats.parser_steps = self.state.stats.parser_steps.saturating_add(1);
        true
    }

    fn finish(mut self) -> ParserOutput {
        if self.state.halted && !self.state.resource_finalizing {
            self.consume_resource_remainder();
        }
        loop {
            let mut allowance = u64::MAX;
            if self.advance_resource_found(&mut allowance) {
                break;
            }
        }
        assert_eq!(
            self.state.rules.len(),
            0,
            "parser rule stack must be empty after every parse"
        );
        assert!(
            self.state.open_markers.is_empty(),
            "parser marker stack must be empty after every parse"
        );
        assert!(
            self.state.events.len() <= self.state.config.limits.max_events as usize,
            "parser event budget must be a hard limit"
        );
        self.state.stats.events_emitted = self.state.events.len() as u64;
        self.state.stats.diagnostics_emitted = self.state.diagnostics.len() as u64;
        let state = self.suspend();
        ParserOutput {
            events: state.events,
            diagnostics: state.diagnostics,
            stats: state.stats,
        }
    }
}

pub fn parse_syntax(
    source: TextSnapshot,
    root: ParseRoot,
    implementation: ParserImplementation,
    config: ParseConfig,
) -> Result<SyntaxSnapshot, ParseRequestError> {
    let mut ids = IdGenerator::new();
    match (implementation, root) {
        (ParserImplementation::Prototype, ParseRoot::Document) => {
            Ok(parse_document_with_ids(source, config, &mut ids))
        }
        (ParserImplementation::Canonical, ParseRoot::Grammar) => {
            Ok(parse_canonical_grammar_with_ids(source, config, &mut ids))
        }
        (ParserImplementation::Canonical, ParseRoot::Document) => {
            Ok(parse_canonical_document_with_ids(source, config, &mut ids))
        }
        _ => Err(ParseRequestError::Unsupported {
            implementation,
            root,
        }),
    }
}

pub fn parse_document(source: TextSnapshot, config: ParseConfig) -> SyntaxSnapshot {
    parse_syntax(
        source,
        ParseRoot::Document,
        ParserImplementation::Prototype,
        config,
    )
    .expect("prototype document parsing is a supported configuration")
}

pub fn parse_canonical_grammar(source: TextSnapshot, config: ParseConfig) -> SyntaxSnapshot {
    parse_syntax(
        source,
        ParseRoot::Grammar,
        ParserImplementation::Canonical,
        config,
    )
    .expect("canonical grammar parsing is a supported configuration")
}

pub fn parse_canonical_document(source: TextSnapshot, config: ParseConfig) -> SyntaxSnapshot {
    parse_syntax(
        source,
        ParseRoot::Document,
        ParserImplementation::Canonical,
        config,
    )
    .expect("canonical document parsing is a supported configuration")
}

pub(crate) fn parse_document_with_ids(
    source: TextSnapshot,
    config: ParseConfig,
    ids: &mut IdGenerator,
) -> SyntaxSnapshot {
    let mut parser = Parser::new(&source, LexicalMode::PrototypeDocument, config, ids);
    document::parse_document_root(&mut parser);
    let output = parser.finish();
    finish_snapshot(source, output, ids, SyntaxKind::Document)
}

fn parse_canonical_grammar_with_ids(
    source: TextSnapshot,
    config: ParseConfig,
    ids: &mut IdGenerator,
) -> SyntaxSnapshot {
    let mut parser = Parser::new(&source, LexicalMode::CanonicalGrammar, config, ids);
    parser.set_resource_rule(rules::PARSE_GRAMMAR);
    canonical::roots::parse_grammar_root(&mut parser);
    let output = parser.finish();
    finish_snapshot(source, output, ids, SyntaxKind::GrammarDocument)
}

pub(crate) fn parse_canonical_document_with_ids(
    source: TextSnapshot,
    config: ParseConfig,
    ids: &mut IdGenerator,
) -> SyntaxSnapshot {
    let mut parser = Parser::new(&source, LexicalMode::CanonicalSourceFragment, config, ids);
    parser.set_resource_rule(rules::PARSE);
    canonical::document::parse_document_root(&mut parser);
    let output = parser.finish();
    finish_snapshot(source, output, ids, SyntaxKind::Document)
}

fn canonical_fragment_rule(kind: SyntaxKind) -> Option<RuleId> {
    match kind {
        SyntaxKind::Grammar => Some(rules::GRAMMAR),
        SyntaxKind::GrammarRule => Some(rules::GRAMMAR_RULE),
        SyntaxKind::GrammarExpression => Some(rules::GRAMMAR_EXPRESSION),
        SyntaxKind::GrammarTerm => Some(rules::GRAMMAR_TERM),
        SyntaxKind::GrammarFactor => Some(rules::GRAMMAR_FACTOR),
        SyntaxKind::GrammarTerminalToken => Some(rules::GRAMMAR_TERMINAL_TOKEN),
        _ => None,
    }
}

fn finish_snapshot(
    source: TextSnapshot,
    output: ParserOutput,
    ids: &mut IdGenerator,
    fallback_kind: SyntaxKind,
) -> SyntaxSnapshot {
    let sink_result = sink(&output.events, &source, ids)
        .unwrap_or_else(|_| fallback_tree(&source, ids, fallback_kind));

    let mut diagnostics = DiagnosticStore::new(source.revision());
    for mut pending in output.diagnostics {
        // Accepted append-only prefixes keep their byte ranges. Bind absolute
        // anchors to the exported revision here, not by revisiting every old
        // diagnostic each time input arrives. Edits invalidate live parse state.
        for anchor in core::iter::once(&mut pending.diagnostic.primary).chain(
            pending
                .diagnostic
                .labels
                .iter_mut()
                .map(|label| &mut label.anchor),
        ) {
            if let DiagnosticAnchor::Absolute { revision, .. } = anchor {
                *revision = source.revision();
            }
        }
        if let Some(event) = pending.event
            && let Some(node) = sink_result.event_nodes.get(&event)
        {
            pending.diagnostic.primary = DiagnosticAnchor::Element {
                element: crate::document::SyntaxElementId::Node(*node),
                relative: pending.relative,
            };
        }
        diagnostics.push(pending.diagnostic);
    }

    let mut snapshot = SyntaxSnapshot::new(source, sink_result.root, diagnostics);
    snapshot.stats = output.stats;
    snapshot.stats.new_node_count = snapshot.nodes.node_count() as u64;
    snapshot.restarts = build_restart_index(&snapshot);
    snapshot
}

fn fallback_tree(
    source: &TextSnapshot,
    ids: &mut IdGenerator,
    root_kind: SyntaxKind,
) -> SinkResult {
    let mut builder = GreenBuilder::new(ids);
    builder.start_node(root_kind);
    if !source.is_empty() {
        builder.start_node_with_flags(SyntaxKind::Error, NodeFlags::ERROR);
        for chunk in source.chunks() {
            let _ = builder.token_with_flags(SyntaxKind::Unknown, chunk, TokenFlags::ERROR);
        }
        let _ = builder.finish_node();
    }
    let _ = builder.finish_node();
    let root = builder.finish().unwrap_or_else(|_| {
        Arc::new(GreenNode {
            id: ids.node(),
            kind: root_kind,
            text_len: TextSize::ZERO,
            children: Arc::from([]),
            flags: NodeFlags::ERROR,
            structural_hash: 0,
        })
    });
    SinkResult {
        root,
        event_nodes: BTreeMap::new(),
    }
}

pub(crate) fn build_restart_index(snapshot: &SyntaxSnapshot) -> RestartIndex {
    let mut restarts = RestartIndex::default();
    for (node, record) in snapshot.nodes.nodes() {
        let mode = match record.kind {
            SyntaxKind::Paragraph | SyntaxKind::ParagraphElement => RestartMode::Paragraph,
            SyntaxKind::MechItem
            | SyntaxKind::VariableDefine
            | SyntaxKind::ParentheticalExpression => RestartMode::Mech,
            SyntaxKind::GenericFence => RestartMode::Fence,
            SyntaxKind::Document
            | SyntaxKind::Section
            | SyntaxKind::SectionElement
            | SyntaxKind::Subtitle
            | SyntaxKind::UlSubtitle => RestartMode::Document,
            SyntaxKind::GrammarDocument | SyntaxKind::Grammar | SyntaxKind::GrammarRule => {
                RestartMode::Grammar
            }
            _ => continue,
        };
        restarts.push(RestartEntry {
            node,
            range: record.range,
            mode,
            delimiter_depth: enclosing_delimiter_depth(snapshot, node),
            line_start: snapshot
                .source
                .line_index()
                .line_start(snapshot.source.line_index().line_of(record.range.start))
                == Some(record.range.start),
            indentation: leading_indentation(&snapshot.source, record.range),
        });
    }
    restarts
}

fn enclosing_delimiter_depth(snapshot: &SyntaxSnapshot, node: crate::document::NodeId) -> u32 {
    let mut depth = 0_u32;
    let mut current = snapshot.nodes.node(node).and_then(|record| record.parent);
    while let Some(parent) = current {
        let Some(record) = snapshot.nodes.node(parent) else {
            break;
        };
        if owns_delimiter(record.kind) {
            depth = depth.saturating_add(1);
        }
        current = record.parent;
    }
    depth
}

fn owns_delimiter(kind: SyntaxKind) -> bool {
    matches!(kind, SyntaxKind::ParentheticalExpression)
}

fn leading_indentation(source: &TextSnapshot, range: TextRange) -> u32 {
    let mut cursor = Cursor::for_range(source, range);
    let start = cursor.offset();
    while cursor
        .peek_char()
        .is_some_and(terminal::is_horizontal_space)
    {
        let _ = cursor.bump_char();
    }
    cursor.offset().0.saturating_sub(start.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{DocumentId, ExpectedSyntax, Revision};

    #[test]
    fn embedded_cursor_scopes_survive_append_and_restore_each_owning_frontier() {
        let source = TextSnapshot::new(DocumentId(826), Revision(0), "abcd").unwrap();
        let mut ids = IdGenerator::new();
        let mut parser = Parser::new(
            &source,
            LexicalMode::CanonicalSourceFragment,
            ParseConfig::default(),
            &mut ids,
        );
        let outer = parser.enter_cursor_scope(TextSize(3));
        let inner = parser.enter_cursor_scope(TextSize(2));
        parser.bump_bytes_token(1, SyntaxKind::Text).unwrap();
        let fuel = parser.state.fuel;
        let state = parser.suspend();
        let extended = source.append("ef").unwrap();
        let mut parser = Parser::resume(&extended, state, &mut ids);
        assert_eq!(parser.offset(), TextSize(1));
        assert_eq!(parser.cursor().end(), TextSize(2));
        parser.leave_cursor_scope(inner);
        assert_eq!(parser.cursor().end(), TextSize(3));
        assert_eq!(parser.cursor().context_end(), TextSize(3));
        assert!(!parser.state.cursor_frontier);
        parser.leave_cursor_scope(outer);
        assert_eq!(parser.cursor().end(), extended.byte_len());
        assert_eq!(parser.cursor().context_end(), extended.byte_len());
        assert_eq!(parser.offset(), TextSize(1));
        assert_eq!(parser.state.fuel, fuel);
        assert!(parser.state.cursor_frontier);
        assert!(parser.state.context_frontier);
    }

    #[test]
    fn cursor_scope_restore_keeps_document_resource_remainders_past_local_bounds() {
        let source = TextSnapshot::new(DocumentId(826), Revision(0), "abcdef").unwrap();
        let mut ids = IdGenerator::new();
        let mut parser = Parser::new(
            &source,
            LexicalMode::CanonicalSourceFragment,
            ParseConfig::default(),
            &mut ids,
        );
        let wrapper = parser.start();
        let outer = parser.enter_cursor_scope(TextSize(4));
        let inner = parser.enter_cursor_scope(TextSize(2));
        parser.halt();
        parser.consume_resource_remainder();
        assert_eq!(parser.offset(), source.byte_len());
        parser.leave_cursor_scope(inner);
        assert_eq!(parser.cursor().end(), TextSize(4));
        assert_eq!(parser.offset(), source.byte_len());
        parser.leave_cursor_scope(outer);
        assert_eq!(parser.cursor().end(), source.byte_len());
        assert_eq!(parser.offset(), source.byte_len());
        wrapper.complete(&mut parser, SyntaxKind::Document);
        let output = parser.finish();
        assert_eq!(output.diagnostics.len(), 1);
        assert_eq!(output.stats.source_bytes, 6);
    }

    #[test]
    fn suspended_parser_retains_live_markers_context_and_nonrefundable_work() {
        let source = TextSnapshot::new(DocumentId(826), Revision(0), "a").unwrap();
        let mut ids = IdGenerator::new();
        let mut parser = Parser::new(
            &source,
            LexicalMode::CanonicalSourceFragment,
            ParseConfig::default(),
            &mut ids,
        );
        let root = parser.start();
        parser.state.rules.push_canonical(rules::PARSE);
        let checkpoint = parser.checkpoint();
        parser.bump_bytes_token(1, SyntaxKind::Text).unwrap();
        let fuel = parser.state.fuel;
        let events = parser.state.events.as_ptr();
        let steps = parser.stats().parser_steps;
        let state = parser.suspend();
        let source = source.append("b").unwrap();
        let mut parser = Parser::resume(&source, state, &mut ids);
        assert_eq!(parser.state.events.as_ptr(), events);
        assert_eq!(parser.current_rule(), Some(rules::PARSE));
        assert_eq!(parser.state.open_markers.len(), 1);
        assert_eq!(parser.offset(), TextSize(1));
        assert_eq!(parser.cursor.end(), TextSize(2));
        assert_eq!(parser.state.fuel, fuel);
        assert_eq!(parser.stats().parser_steps, steps);
        parser.rewind(checkpoint);
        assert_eq!(parser.offset(), TextSize::ZERO);
        assert_eq!(
            parser.state.fuel, fuel,
            "rewind cannot refund work across a resume"
        );
        parser.bump_bytes_token(2, SyntaxKind::Text).unwrap();
        parser.state.rules.truncate(0);
        root.complete(&mut parser, SyntaxKind::Document);
        let output = parser.finish();
        let tree = event::sink(&output.events, &source, &mut ids).unwrap();
        assert_eq!(
            crate::document::reconstruct_source(&tree.root, &source).unwrap(),
            "ab"
        );
        assert_eq!(output.stats.source_bytes, 2);
        assert!(output.stats.parser_steps > steps);
    }

    #[test]
    fn resumed_diagnostics_keep_identity_and_bind_labels_to_the_export_revision() {
        let source = TextSnapshot::new(DocumentId(826), Revision(0), "a").unwrap();
        let mut ids = IdGenerator::new();
        let mut parser = Parser::new(
            &source,
            LexicalMode::CanonicalSourceFragment,
            ParseConfig::default(),
            &mut ids,
        );
        let root = parser.start();
        recovery::insert_missing(
            &mut parser,
            "syntax/test-missing",
            "expected closer",
            ExpectedSyntax::Token(SyntaxKind::RightParen),
            Some(SyntaxKind::RightParen),
        );
        let diagnostic = parser.last_diagnostic_mut().unwrap();
        let id = diagnostic.id;
        diagnostic.labels.push(crate::document::DiagnosticLabel {
            anchor: DiagnosticAnchor::Absolute {
                revision: source.revision(),
                range: source.full_range(),
            },
            message: String::from("retained prefix"),
        });
        parser.bump_bytes_token(1, SyntaxKind::Text).unwrap();
        let diagnostics = parser.state.diagnostics.as_ptr();
        let state = parser.suspend();
        let source = source.append("b").unwrap();
        let mut parser = Parser::resume(&source, state, &mut ids);
        assert_eq!(parser.state.diagnostics.as_ptr(), diagnostics);
        assert_eq!(parser.state.diagnostics[0].diagnostic.id, id);
        parser.bump_bytes_token(1, SyntaxKind::Text).unwrap();
        root.complete(&mut parser, SyntaxKind::Document);
        let snapshot = finish_snapshot(
            source.clone(),
            parser.finish(),
            &mut ids,
            SyntaxKind::Document,
        );
        let diagnostic = snapshot.diagnostics.iter().next().unwrap();
        assert_eq!(diagnostic.id, id);
        assert_eq!(
            diagnostic.labels[0]
                .anchor
                .resolve(snapshot.revision, &snapshot.nodes),
            Some(TextRange::new(TextSize::ZERO, TextSize(1)))
        );
        assert_eq!(
            crate::document::reconstruct_source(&snapshot.root, &snapshot.source).unwrap(),
            "ab"
        );
    }

    #[test]
    fn resuming_an_embedded_range_does_not_expand_a_fixed_eof_bound() {
        let source = TextSnapshot::new(DocumentId(826), Revision(0), "a").unwrap();
        let mut ids = IdGenerator::new();
        let parser = Parser::for_range(
            &source,
            source.full_range(),
            LexicalMode::CanonicalSourceFragment,
            None,
            0,
            ParseConfig::default(),
            &mut ids,
        );
        let state = parser.suspend();
        let source = source.append("b").unwrap();
        let parser = Parser::resume(&source, state, &mut ids);
        assert_eq!(parser.cursor.end(), TextSize(1));
        assert_eq!(parser.cursor.context_end(), TextSize(2));
        assert_eq!(parser.state.parse_range.end, TextSize(1));
    }

    fn classify(text: &str, mode: LexicalMode, resource_rule: Option<RuleId>) -> FoundSyntax {
        let source = TextSnapshot::new(DocumentId(1), Revision(0), text).unwrap();
        let mut ids = IdGenerator::new();
        let mut parser = Parser::new(&source, mode, ParseConfig::default(), &mut ids);
        if let Some(rule) = resource_rule {
            parser.set_resource_rule(rule);
        }
        parser.found_syntax()
    }

    #[test]
    fn lexical_mode_not_resource_attribution_selects_found_syntax() {
        for mode in [
            LexicalMode::PrototypeDocument,
            LexicalMode::CanonicalGrammar,
            LexicalMode::CanonicalSourceFragment,
        ] {
            assert_eq!(
                classify("@", mode, None),
                classify("@", mode, Some(rules::GRAMMAR)),
                "resource attribution changed {mode:?} classification"
            );
        }
    }

    #[test]
    #[should_panic(expected = "parser markers must complete in strict LIFO order")]
    fn marker_completion_rejects_non_lifo_order() {
        let source = TextSnapshot::new(DocumentId(1), Revision(0), "").unwrap();
        let mut ids = IdGenerator::new();
        let mut parser = Parser::new(
            &source,
            LexicalMode::PrototypeDocument,
            ParseConfig::default(),
            &mut ids,
        );
        let outer = parser.start();
        let _inner = parser.start();
        let _ = outer.complete(&mut parser, SyntaxKind::Document);
    }

    #[test]
    #[should_panic(expected = "parser markers must abandon in strict LIFO order")]
    fn marker_abandonment_rejects_non_lifo_order() {
        let source = TextSnapshot::new(DocumentId(1), Revision(0), "").unwrap();
        let mut ids = IdGenerator::new();
        let mut parser = Parser::new(
            &source,
            LexicalMode::PrototypeDocument,
            ParseConfig::default(),
            &mut ids,
        );
        let outer = parser.start();
        let _inner = parser.start();
        outer.abandon(&mut parser);
    }

    #[test]
    fn retained_nesting_enters_and_restores_depth() {
        let source = TextSnapshot::new(DocumentId(1), Revision(0), "").unwrap();
        let mut ids = IdGenerator::new();
        let mut parser = Parser::new(
            &source,
            LexicalMode::CanonicalSourceFragment,
            ParseConfig::default(),
            &mut ids,
        );
        let initial = parser.nesting();

        assert!(parser.push_nesting());
        assert_eq!(parser.nesting(), initial + 1);
        parser.pop_nesting();
        assert_eq!(parser.nesting(), initial);
    }

    #[test]
    fn retained_nesting_rejects_entry_at_limit() {
        let source = TextSnapshot::new(DocumentId(1), Revision(0), "").unwrap();
        let mut ids = IdGenerator::new();
        let config = ParseConfig {
            limits: ParseLimits {
                max_nesting: 0,
                ..ParseLimits::default()
            },
        };
        let mut parser = Parser::new(
            &source,
            LexicalMode::CanonicalSourceFragment,
            config,
            &mut ids,
        );
        let initial = parser.nesting();
        assert!(!parser.push_nesting());
        assert_eq!(parser.nesting(), initial);
    }

    #[test]
    fn retained_nesting_restores_depth_after_halt() {
        let source = TextSnapshot::new(DocumentId(1), Revision(0), "").unwrap();
        let mut ids = IdGenerator::new();
        let mut parser = Parser::new(
            &source,
            LexicalMode::CanonicalSourceFragment,
            ParseConfig::default(),
            &mut ids,
        );
        let initial = parser.nesting();

        assert!(parser.push_nesting());
        parser.halt();
        assert_eq!(parser.nesting(), initial + 1);
        parser.pop_nesting();
        assert!(parser.is_halted());
        assert_eq!(parser.nesting(), initial);
    }

    #[test]
    fn canonical_rule_scope_uses_rule_without_prototype_context() {
        let source = TextSnapshot::new(DocumentId(1), Revision(0), "").unwrap();
        let mut ids = IdGenerator::new();
        let mut parser = Parser::new(
            &source,
            LexicalMode::CanonicalGrammar,
            ParseConfig::default(),
            &mut ids,
        );
        let rule = rules::GRAMMAR;
        parser.with_canonical_rule(rule, |parser| {
            let _ = recovery::insert_missing(
                parser,
                "syntax/missing-grammar-rule",
                "expected a grammar rule",
                ExpectedSyntax::Production(String::from("grammar rule")),
                None,
            );
        });

        let output = parser.finish();
        assert_eq!(output.diagnostics.len(), 1);
        assert_eq!(output.diagnostics[0].diagnostic.rule, Some(rule));
        assert_eq!(output.diagnostics[0].diagnostic.context, None);
    }

    #[test]
    fn prototype_rule_scope_keeps_context_without_canonical_rule() {
        let source = TextSnapshot::new(DocumentId(1), Revision(0), "").unwrap();
        let mut ids = IdGenerator::new();
        let mut parser = Parser::new(
            &source,
            LexicalMode::PrototypeDocument,
            ParseConfig::default(),
            &mut ids,
        );
        let context = parser_context_id("prototype-test");
        parser.with_rule(context, None, |parser| {
            let _ = recovery::insert_missing(
                parser,
                "syntax/missing-test-token",
                "expected a test token",
                ExpectedSyntax::Production(String::from("test token")),
                None,
            );
        });

        let output = parser.finish();
        assert_eq!(output.diagnostics.len(), 1);
        assert_eq!(output.diagnostics[0].diagnostic.rule, None);
        assert_eq!(output.diagnostics[0].diagnostic.context, Some(context));
    }

    #[test]
    fn canonical_resource_diagnostic_keeps_rule_attribution() {
        let source = TextSnapshot::new(DocumentId(1), Revision(0), "remainder").unwrap();
        let mut ids = IdGenerator::new();
        let mut parser = Parser::new(
            &source,
            LexicalMode::CanonicalGrammar,
            ParseConfig::default(),
            &mut ids,
        );
        let rule = rules::PARSE_GRAMMAR;
        parser.with_canonical_rule(rule, |parser| {
            parser.halt();
            parser.consume_resource_remainder();
        });

        let output = parser.finish();
        assert_eq!(output.diagnostics.len(), 1);
        assert_eq!(output.diagnostics[0].diagnostic.rule, Some(rule));
        assert_eq!(output.diagnostics[0].diagnostic.context, None);
    }

    #[test]
    fn canonical_tiny_event_budgets_keep_root_rule_attribution() {
        for max_events in 0..=4 {
            let source = TextSnapshot::new(DocumentId(1), Revision(0), "x := \"a\";").unwrap();
            let config = ParseConfig {
                limits: ParseLimits {
                    max_events,
                    ..ParseLimits::default()
                },
            };
            let snapshot = parse_canonical_grammar(source, config);

            assert!(snapshot.stats.events_emitted <= u64::from(max_events));
            crate::document::validate_lossless(&snapshot.root, &snapshot.source).unwrap();
            assert!(!snapshot.diagnostics.is_empty());
            for diagnostic in snapshot.diagnostics.iter() {
                assert_eq!(diagnostic.rule, Some(rules::PARSE_GRAMMAR));
                assert_eq!(
                    diagnostic.rule.and_then(canonical_rule_name),
                    Some("parse-grammar")
                );
                assert_eq!(diagnostic.context, None);
            }
        }
    }

    #[test]
    fn failed_canonical_alternative_restores_rule_and_marker_depth() {
        let source = TextSnapshot::new(DocumentId(1), Revision(0), "x").unwrap();
        let mut ids = IdGenerator::new();
        let mut parser = Parser::new(
            &source,
            LexicalMode::CanonicalGrammar,
            ParseConfig::default(),
            &mut ids,
        );
        let checkpoint = parser.checkpoint();
        let matched = parser.with_canonical_rule(rules::GRAMMAR_FACTOR, |parser| {
            let factor = parser.start();
            let _ = parser.bump_char_token(SyntaxKind::Text);
            factor.complete(parser, SyntaxKind::ParagraphText);
            parser.rewind(checkpoint);
            false
        });

        assert!(!matched);
        assert_eq!(parser.offset(), TextSize::ZERO);
        assert_eq!(parser.rule_depth(), 0);
        let output = parser.finish();
        assert!(output.events.is_empty());
        assert!(output.diagnostics.is_empty());
    }
    #[test]
    fn speculative_rewinds_do_not_refund_recovery_work() {
        let source = TextSnapshot::new(DocumentId(1), Revision(0), "@]").unwrap();
        let mut ids = IdGenerator::new();
        let mut parser = Parser::new(
            &source,
            LexicalMode::CanonicalGrammar,
            ParseConfig {
                limits: ParseLimits {
                    max_recovery_bytes: 1,
                    ..ParseLimits::default()
                },
            },
            &mut ids,
        );
        let checkpoint = parser.checkpoint();
        recovery::abandon_to_restart(
            &mut parser,
            rules::EXPRESSION,
            &[']'],
            "test/recovery",
            "test",
        );
        assert_eq!(parser.stats().recovery_bytes, 1);
        parser.rewind(checkpoint);
        assert_eq!(parser.offset(), TextSize::ZERO);
        assert_eq!(parser.stats().recovery_bytes, 1);
        recovery::abandon_to_restart(
            &mut parser,
            rules::EXPRESSION,
            &[']'],
            "test/recovery",
            "test",
        );
        assert!(parser.is_halted());
        assert_eq!(parser.stats().recovery_bytes, 1);
        assert_eq!(parser.offset(), TextSize::ZERO);
    }
}
