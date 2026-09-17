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

pub(crate) struct Parser<'a> {
  source: &'a TextSnapshot,
  parse_range: TextRange,
  cursor: Cursor<'a>,
  events: Vec<Event>,
  diagnostics: Vec<PendingDiagnostic>,
  rules: RuleStack,
  config: ParseConfig,
  fuel: u64,
  nesting: u32,
  halted: bool,
  resource_diagnostic_emitted: bool,
  resource_finalized: bool,
  resource_root_kind: SyntaxKind,
  ids: &'a mut IdGenerator,
  stats: ParseStats,
}

impl<'a> Parser<'a> {
  fn new(
    source: &'a TextSnapshot,
    config: ParseConfig,
    ids: &'a mut IdGenerator,
  ) -> Self {
    Self {
      source,
      parse_range: source.full_range(),
      cursor: Cursor::new(source),
      events: Vec::new(),
      diagnostics: Vec::new(),
      rules: RuleStack::default(),
      config,
      fuel: config.limits.fuel,
      nesting: 0,
      halted: false,
      resource_diagnostic_emitted: false,
      resource_finalized: false,
      resource_root_kind: SyntaxKind::Document,
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
    root_kind: SyntaxKind,
    config: ParseConfig,
    ids: &'a mut IdGenerator,
  ) -> Self {
    let mut parser = Self {
      source,
      parse_range: range,
      cursor: Cursor::for_range(source, range),
      events: Vec::new(),
      diagnostics: Vec::new(),
      rules: RuleStack::default(),
      config,
      fuel: config.limits.fuel,
      nesting: 0,
      halted: false,
      resource_diagnostic_emitted: false,
      resource_finalized: false,
      resource_root_kind: root_kind,
      ids,
      stats: ParseStats {
        source_bytes: u64::from(range.len().0),
        ..ParseStats::default()
      },
    };
    if source.text(range).is_err() {
      parser.halted = true;
    }
    parser
  }

  pub(crate) fn source(&self) -> &TextSnapshot {
    self.source
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
    let position = self.emit(Event::Tombstone).unwrap_or(usize::MAX);
    Marker { position }
  }

  pub(crate) fn complete_marker(
    &mut self,
    marker: Marker,
    kind: SyntaxKind,
    flags: NodeFlags,
  ) -> CompletedMarker {
    if self.resource_finalized || marker.position == usize::MAX {
      return CompletedMarker {
        position: usize::MAX,
        kind,
      };
    }
    if let Some(event) = self.events.get_mut(marker.position) {
      *event = Event::Start { kind, flags };
    }
    if self.emit(Event::Finish).is_none()
      && let Some(event) = self.events.get_mut(marker.position)
    {
      *event = Event::Tombstone;
    }
    CompletedMarker {
      position: marker.position,
      kind,
    }
  }

  pub(crate) fn abandon_marker(&mut self, marker: Marker) {
    if self.resource_finalized || marker.position == usize::MAX {
      return;
    }
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
      rule_depth: self.rules.len(),
      nesting: self.nesting,
    }
  }

  pub(crate) fn rewind(&mut self, checkpoint: ParserCheckpoint) {
    self.cursor.rewind(checkpoint.cursor);
    self.events.truncate(checkpoint.events);
    self.diagnostics.truncate(checkpoint.diagnostics);
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
    let _ = self.emit(Event::Token { kind, range, flags });
  }

  pub(crate) fn missing_token(&mut self, kind: SyntaxKind) {
    let _ = self.emit(Event::Token {
      kind,
      range: TextRange::empty(self.offset()),
      flags: TokenFlags::SYNTHETIC | TokenFlags::MISSING,
    });
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
    if self.is_eof() {
      return FoundSyntax {
        kind: Some(SyntaxKind::Eof),
        text: None,
      };
    }
    let character = self.cursor.peek_char();
    FoundSyntax {
      kind: character.map(terminal::token_kind_for_char),
      text: character.map(|character| character.to_string()),
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

  pub(crate) fn is_fence_start(&self) -> bool {
    mechdown::fence_delimiter(self.cursor()).is_some()
  }

  pub(crate) fn is_strong_document_boundary(&self) -> bool {
    self.cursor.is_line_start()
      && (mechdown::is_ul_subtitle(self.cursor()) || self.is_fence_start())
  }

  pub(crate) fn consume_resource_remainder(&mut self) {
    if self.resource_finalized {
      return;
    }
    self.resource_finalized = true;
    let start = self.offset();
    let range = TextRange::new(start, self.cursor.end());
    self.cursor.rewind(CursorCheckpoint { offset: range.end });
    self.events.clear();
    if self.config.limits.max_events >= 5 {
      let full = self.parse_range;
      let _ = self.emit_resource(Event::Start {
        kind: self.resource_root_kind,
        flags: NodeFlags::ERROR | NodeFlags::CONTAINS_ERROR,
      });
      let _ = self.emit_resource(Event::Start {
        kind: SyntaxKind::Error,
        flags: NodeFlags::ERROR,
      });
      let _ = self.emit_resource(Event::Token {
        kind: SyntaxKind::Unknown,
        range: full,
        flags: TokenFlags::ERROR,
      });
      let _ = self.emit_resource(Event::Finish);
      let _ = self.emit_resource(Event::Finish);
    }
    if !self.resource_diagnostic_emitted {
      self.resource_diagnostic_emitted = true;
      let diagnostic = Diagnostic {
        id: self.next_diagnostic_id(),
        code: DiagnosticCode::syntax("recovery-limit"),
        phase: DiagnosticPhase::Syntax,
        severity: Severity::Error,
        rule: None,
        context: self.current_context(),
        primary: DiagnosticAnchor::Absolute {
          revision: self.source.revision(),
          range,
        },
        labels: Vec::new(),
        expected: Vec::new(),
        found: Some(FoundSyntax {
          kind: Some(SyntaxKind::Unknown),
          text: None,
        }),
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

  fn emit(&mut self, event: Event) -> Option<usize> {
    if self.resource_finalized
      || self.events.len() >= self.config.limits.max_events as usize
    {
      self.halted = true;
      return None;
    }
    let position = self.events.len();
    self.events.push(event);
    Some(position)
  }

  fn emit_resource(&mut self, event: Event) -> Option<usize> {
    if self.events.len() >= self.config.limits.max_events as usize {
      return None;
    }
    let position = self.events.len();
    self.events.push(event);
    Some(position)
  }

  fn charge(&mut self) -> bool {
    if self.fuel == 0 {
      self.halted = true;
      return false;
    }
    self.fuel -= 1;
    self.stats.parser_steps = self.stats.parser_steps.saturating_add(1);
    true
  }

  fn finish(mut self) -> ParserOutput {
    assert_eq!(
      self.rules.len(),
      0,
      "parser rule stack must be empty after every parse"
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

pub fn parse_document(source: TextSnapshot, config: ParseConfig) -> SyntaxSnapshot {
  let mut ids = IdGenerator::new();
  parse_document_with_ids(source, config, &mut ids)
}

pub(crate) fn parse_document_with_ids(
  source: TextSnapshot,
  config: ParseConfig,
  ids: &mut IdGenerator,
) -> SyntaxSnapshot {
  let mut parser = Parser::new(&source, config, ids);
  document::parse_document_root(&mut parser);
  let output = parser.finish();
  let sink_result = sink(&output.events, &source, ids)
    .unwrap_or_else(|_| fallback_tree(&source, ids));

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

fn fallback_tree(source: &TextSnapshot, ids: &mut IdGenerator) -> SinkResult {
  let mut builder = GreenBuilder::new(ids);
  builder.start_node(SyntaxKind::Document);
  if !source.is_empty() {
    builder.start_node_with_flags(SyntaxKind::Error, NodeFlags::ERROR);
    let text = source.to_contiguous_string();
    let _ = builder.token_with_flags(
      SyntaxKind::Unknown,
      &text,
      TokenFlags::ERROR,
    );
    let _ = builder.finish_node();
  }
  let _ = builder.finish_node();
  let root = builder.finish().unwrap_or_else(|_| {
    Arc::new(GreenNode {
      id: ids.node(),
      kind: SyntaxKind::Document,
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
      _ => continue,
    };
    restarts.push(RestartEntry {
      node,
      range: record.range,
      mode,
      delimiter_depth: if record.kind == SyntaxKind::ParentheticalExpression {
        1
      } else {
        0
      },
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

fn leading_indentation(source: &TextSnapshot, range: TextRange) -> u32 {
  let mut cursor = Cursor::for_range(source, range);
  let start = cursor.offset();
  while cursor.peek_char().is_some_and(terminal::is_horizontal_space) {
    let _ = cursor.bump_char();
  }
  cursor.offset().0.saturating_sub(start.0)
}
