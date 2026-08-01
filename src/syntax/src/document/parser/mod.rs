use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::document::{
  Diagnostic, DiagnosticAnchor, DiagnosticCode, DiagnosticPhase, DiagnosticStore, DiagnosticTags,
  DocumentId, ExpectedSyntax, FoundSyntax, GreenBuilder, GreenNode, IdGenerator, NodeFlags,
  ParseStats, ParserContextId, RecoveryAction, RestartEntry, RestartIndex, RestartMode, Revision,
  RuleId, Severity, SyntaxKind, SyntaxSnapshot, TextRange, TextSize, TextSnapshot, TokenFlags,
};

mod canonical_ports;
mod canonical_rules;
pub mod canonical;
pub mod checkpoint;
pub mod cursor;
pub mod document;
pub mod event;
pub mod fragment;
pub mod limits;
pub mod marker;
pub mod mech;
pub mod mechdown;
pub mod recovery;
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
  lexical_mode: LexicalMode,
  parse_range: TextRange,
  cursor: Cursor<'a>,
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
  resource_finalizing: bool,
  resource_rule: Option<RuleId>,
  ids: &'a mut IdGenerator,
  stats: ParseStats,
}

impl<'a> Parser<'a> {
  fn new(
    source: &'a TextSnapshot,
    lexical_mode: LexicalMode,
    config: ParseConfig,
    ids: &'a mut IdGenerator,
  ) -> Self {
    Self {
      source,
      lexical_mode,
      parse_range: source.full_range(),
      cursor: Cursor::new(source),
      events: Vec::new(),
      open_markers: Vec::new(),
      covered_end: TextSize::ZERO,
      diagnostics: Vec::new(),
      rules: RuleStack::default(),
      config,
      fuel: config.limits.fuel,
      nesting: 0,
      halted: false,
      resource_diagnostic_emitted: false,
      resource_finalizing: false,
      resource_rule: None,
      ids,
      stats: ParseStats {
        source_bytes: u64::from(source.byte_len().0),
        ..ParseStats::default()
      },
    }
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
    let mut parser = Self {
      source,
      lexical_mode,
      parse_range: range,
      cursor: Cursor::for_range(source, range),
      events: Vec::new(),
      open_markers: Vec::new(),
      covered_end: range.start,
      diagnostics: Vec::new(),
      rules: RuleStack::default(),
      config,
      fuel: config.limits.fuel,
      nesting: initial_nesting,
      halted: false,
      resource_diagnostic_emitted: false,
      resource_finalizing: false,
      resource_rule,
      ids,
      stats: ParseStats {
        source_bytes: u64::from(range.len().0),
        ..ParseStats::default()
      },
    };
    if source.validate_range(range).is_err() {
      parser.halted = true;
    }
    parser
  }

  pub(crate) fn source(&self) -> &TextSnapshot {
    self.source
  }

  pub(crate) fn set_resource_rule(&mut self, rule: RuleId) {
    self.resource_rule = Some(rule);
  }

  pub(crate) fn cursor(&self) -> &Cursor<'a> {
    &self.cursor
  }

  pub(crate) fn config(&self) -> ParseConfig {
    self.config
  }

  pub(crate) fn offset(&self) -> TextSize {
    self.cursor.offset()
  }

  pub(crate) fn is_eof(&self) -> bool {
    self.cursor.is_eof()
  }

  pub(crate) fn is_halted(&self) -> bool {
    self.halted
  }

  pub(crate) fn halt(&mut self) {
    self.halted = true;
  }

  pub(crate) fn stats(&self) -> ParseStats {
    self.stats
  }

  pub(crate) fn stats_mut(&mut self) -> &mut ParseStats {
    &mut self.stats
  }

  pub(crate) fn start(&mut self) -> Marker {
    if self.halted || self.resource_finalizing {
      return Marker {
        position: usize::MAX,
      };
    }
    let open_after = self.open_markers.len().saturating_add(1);
    let position = self
      .emit(Event::Tombstone, open_after)
      .unwrap_or(usize::MAX);
    if position != usize::MAX {
      self.open_markers.push(position);
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
    if self.halted && !self.resource_finalizing {
      self.consume_resource_remainder();
    }
    if self.open_markers.is_empty() {
      return CompletedMarker {
        position: usize::MAX,
        kind,
      };
    }
    assert_eq!(
      self.open_markers.last().copied(),
      Some(marker.position),
      "parser markers must complete in strict LIFO order"
    );
    if let Some(event) = self.events.get_mut(marker.position) {
      *event = Event::Start { kind, flags };
    }
    let open_after = self.open_markers.len().saturating_sub(1);
    let finish = if self.resource_finalizing {
      self.emit_emergency(Event::Finish)
    } else {
      self.emit(Event::Finish, open_after)
    };
    assert!(
      finish.is_some(),
      "accepted marker start must reserve capacity for its finish"
    );
    self.open_markers.pop();
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
      !self.resource_finalizing,
      "resource finalization cannot abandon an enclosing parser marker"
    );
    assert_eq!(
      self.open_markers.last().copied(),
      Some(marker.position),
      "parser markers must abandon in strict LIFO order"
    );
    self.open_markers.pop();
    if marker.position + 1 == self.events.len() {
      self.events.pop();
    } else if let Some(event) = self.events.get_mut(marker.position) {
      *event = Event::Tombstone;
    }
  }

  pub(crate) fn checkpoint(&self) -> ParserCheckpoint {
    ParserCheckpoint {
      cursor: self.cursor.checkpoint(),
      events: self.events.len(),
      diagnostics: self.diagnostics.len(),
      open_markers: self.open_markers.len(),
      covered_end: self.covered_end,
      rule_depth: self.rules.len(),
      nesting: self.nesting,
    }
  }

  pub(crate) fn rewind(&mut self, checkpoint: ParserCheckpoint) {
    self.cursor.rewind(checkpoint.cursor);
    self.events.truncate(checkpoint.events);
    self.diagnostics.truncate(checkpoint.diagnostics);
    self.open_markers.truncate(checkpoint.open_markers);
    self.covered_end = checkpoint.covered_end;
    self.rules.truncate(checkpoint.rule_depth);
    self.nesting = checkpoint.nesting;
  }

  pub(crate) fn with_rule<T>(
    &mut self,
    context: ParserContextId,
    canonical: Option<RuleId>,
    parse: impl FnOnce(&mut Self) -> T,
  ) -> T {
    let depth = self.rules.len();
    self.rules.push(context, canonical);
    let result = parse(self);
    self.rules.truncate(depth);
    result
  }

  pub(crate) fn with_canonical_rule<T>(
    &mut self,
    rule: RuleId,
    parse: impl FnOnce(&mut Self) -> T,
  ) -> T {
    let depth = self.rules.len();
    self.rules.push_canonical(rule);
    let result = parse(self);
    self.rules.truncate(depth);
    result
  }

  pub(crate) fn current_rule(&self) -> Option<RuleId> {
    self.rules.current_rule()
  }

  pub(crate) fn current_context(&self) -> Option<ParserContextId> {
    self.rules.current_context()
  }

  pub(crate) fn rule_depth(&self) -> usize {
    self.rules.len()
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

  pub(crate) fn bump_bytes_token(
    &mut self,
    count: u32,
    kind: SyntaxKind,
  ) -> Option<TextRange> {
    if !self.charge() {
      return None;
    }
    let range = self.cursor.bump_bytes(count)?;
    self.token(kind, range);
    Some(range)
  }

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
      self.open_markers.len(),
    );
  }

  pub(crate) fn missing_token(&mut self, kind: SyntaxKind) {
    let _ = self.emit(
      Event::Token {
        kind,
        range: TextRange::empty(self.offset()),
        flags: TokenFlags::SYNTHETIC | TokenFlags::MISSING,
      },
      self.open_markers.len(),
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
    if self.diagnostics.len() >= self.config.limits.max_diagnostics as usize {
      self.stats.diagnostics_truncated = true;
      return;
    }
    self.diagnostics.push(PendingDiagnostic {
      diagnostic,
      event,
      relative,
    });
  }

  pub(crate) fn last_diagnostic_mut(&mut self) -> Option<&mut Diagnostic> {
    self
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
    match self.lexical_mode {
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

  pub(crate) fn nesting(&self) -> u32 {
    self.nesting
  }

  pub(crate) fn push_nesting(&mut self) -> bool {
    if self.nesting >= self.config.limits.max_nesting {
      return false;
    }
    self.nesting += 1;
    true
  }

  pub(crate) fn pop_nesting(&mut self) {
    self.nesting = self.nesting.saturating_sub(1);
  }

  pub(crate) fn with_nesting<T>(
    &mut self,
    parse: impl FnOnce(&mut Self) -> T,
  ) -> Option<T> {
    if !self.push_nesting() {
      return None;
    }

    let result = parse(self);
    self.pop_nesting();
    Some(result)
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
    if self.resource_finalizing {
      return;
    }
    self.resource_finalizing = true;
    let mut range = TextRange::new(self.covered_end, self.parse_range.end);
    self.cursor.rewind(CursorCheckpoint { offset: range.end });

    if self.config.limits.max_events < MIN_PREFIX_PRESERVING_EVENTS {
      self.events.clear();
      self.open_markers.clear();
      self.covered_end = self.parse_range.start;
      range = self.parse_range;
    } else if !range.is_empty() {
      let required = self.open_markers.len().saturating_add(3);
      assert!(
        self.events.len().saturating_add(required)
          <= self.config.limits.max_events as usize,
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
      self.covered_end = range.end;
    }
    if !self.resource_diagnostic_emitted {
      self.resource_diagnostic_emitted = true;
      let rule = self.current_rule().or(self.resource_rule);
      let context = rule.is_none().then(|| self.current_context()).flatten();
      let found = match self.lexical_mode {
        LexicalMode::PrototypeDocument => FoundSyntax {
          kind: Some(SyntaxKind::Unknown),
          text: None,
        },
        LexicalMode::CanonicalGrammar => canonical::found::found_syntax(self, range.start),
        LexicalMode::CanonicalSourceFragment => {
          canonical::found::source_found_syntax(self, range.start)
        }
      };
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
        found: Some(found),
        fixes: Vec::new(),
        related: Vec::new(),
        recovery: Some(RecoveryAction::ResourceLimit { range }),
        tags: DiagnosticTags::NONE,
        message: String::from("parser resource limit reached"),
      };
      self.push_diagnostic(
        diagnostic,
        None,
        range,
      );
    }
    self.halted = true;
  }

  fn emit(&mut self, event: Event, open_after: usize) -> Option<usize> {
    if self.halted || self.resource_finalizing {
      self.halted = true;
      return None;
    }
    let emergency = open_after.saturating_add(3);
    if self
      .events
      .len()
      .saturating_add(1)
      .saturating_add(emergency)
      > self.config.limits.max_events as usize
    {
      self.halted = true;
      return None;
    }
    let position = self.events.len();
    if let Event::Token { range, flags, .. } = &event
      && !flags.contains(TokenFlags::SYNTHETIC)
    {
      self.covered_end = self.covered_end.max(range.end);
    }
    self.events.push(event);
    Some(position)
  }

  fn emit_emergency(&mut self, event: Event) -> Option<usize> {
    if self.events.len() >= self.config.limits.max_events as usize {
      return None;
    }
    let position = self.events.len();
    self.events.push(event);
    Some(position)
  }

  fn charge(&mut self) -> bool {
    if self.halted || self.fuel == 0 {
      self.halted = true;
      return false;
    }
    self.fuel -= 1;
    self.stats.parser_steps = self.stats.parser_steps.saturating_add(1);
    true
  }

  fn finish(mut self) -> ParserOutput {
    if self.halted && !self.resource_finalizing {
      self.consume_resource_remainder();
    }
    assert_eq!(
      self.rules.len(),
      0,
      "parser rule stack must be empty after every parse"
    );
    assert!(
      self.open_markers.is_empty(),
      "parser marker stack must be empty after every parse"
    );
    assert!(
      self.events.len() <= self.config.limits.max_events as usize,
      "parser event budget must be a hard limit"
    );
    self.stats.events_emitted = self.events.len() as u64;
    self.stats.diagnostics_emitted = self.diagnostics.len() as u64;
    ParserOutput {
      events: self.events,
      diagnostics: self.diagnostics,
      stats: self.stats,
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

pub fn parse_canonical_grammar(
  source: TextSnapshot,
  config: ParseConfig,
) -> SyntaxSnapshot {
  parse_syntax(
    source,
    ParseRoot::Grammar,
    ParserImplementation::Canonical,
    config,
  )
  .expect("canonical grammar parsing is a supported configuration")
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

  let mut snapshot =
    SyntaxSnapshot::new(source, sink_result.root, diagnostics);
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

fn enclosing_delimiter_depth(
  snapshot: &SyntaxSnapshot,
  node: crate::document::NodeId,
) -> u32 {
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
  while cursor.peek_char().is_some_and(terminal::is_horizontal_space) {
    let _ = cursor.bump_char();
  }
  cursor.offset().0.saturating_sub(start.0)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn classify(
    text: &str,
    mode: LexicalMode,
    resource_rule: Option<RuleId>,
  ) -> FoundSyntax {
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
    let source =
      TextSnapshot::new(DocumentId(1), Revision(0), "").unwrap();
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
    let source =
      TextSnapshot::new(DocumentId(1), Revision(0), "").unwrap();
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
  fn scoped_nesting_returns_result_and_restores_depth() {
    let source =
      TextSnapshot::new(DocumentId(1), Revision(0), "").unwrap();
    let mut ids = IdGenerator::new();
    let mut parser = Parser::new(
      &source,
      LexicalMode::CanonicalSourceFragment,
      ParseConfig::default(),
      &mut ids,
    );
    let initial = parser.nesting();

    let result = parser.with_nesting(|parser| {
      assert_eq!(parser.nesting(), initial + 1);
      42_u32
    });

    assert_eq!(result, Some(42));
    assert_eq!(parser.nesting(), initial);
  }

  #[test]
  fn scoped_nesting_does_not_call_closure_at_limit() {
    let source =
      TextSnapshot::new(DocumentId(1), Revision(0), "").unwrap();
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
    let mut called = false;

    let result = parser.with_nesting(|_| {
      called = true;
    });

    assert_eq!(result, None);
    assert!(!called);
    assert_eq!(parser.nesting(), initial);
  }

  #[test]
  fn scoped_nesting_restores_depth_when_closure_halts_parser() {
    let source =
      TextSnapshot::new(DocumentId(1), Revision(0), "").unwrap();
    let mut ids = IdGenerator::new();
    let mut parser = Parser::new(
      &source,
      LexicalMode::CanonicalSourceFragment,
      ParseConfig::default(),
      &mut ids,
    );
    let initial = parser.nesting();

    let result = parser.with_nesting(|parser| {
      parser.halt();
      assert_eq!(parser.nesting(), initial + 1);
      "halted"
    });

    assert_eq!(result, Some("halted"));
    assert!(parser.is_halted());
    assert_eq!(parser.nesting(), initial);
  }

  #[test]
  fn canonical_rule_scope_uses_rule_without_prototype_context() {
    let source =
      TextSnapshot::new(DocumentId(1), Revision(0), "").unwrap();
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
    let source =
      TextSnapshot::new(DocumentId(1), Revision(0), "").unwrap();
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
    assert_eq!(
      output.diagnostics[0].diagnostic.context,
      Some(context)
    );
  }

  #[test]
  fn canonical_resource_diagnostic_keeps_rule_attribution() {
    let source =
      TextSnapshot::new(DocumentId(1), Revision(0), "remainder").unwrap();
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
      let source =
        TextSnapshot::new(DocumentId(1), Revision(0), "x := \"a\";").unwrap();
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
    let source =
      TextSnapshot::new(DocumentId(1), Revision(0), "x").unwrap();
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
}
