use alloc::sync::Arc;

use crate::document::{
  DiagnosticAnchor, DiagnosticStore, GreenBuilder, GreenNode, IdGenerator, NodeFlags, NodeIndex,
  ParseStats, SyntaxKind, SyntaxNode, TextRange, TextSize, TextSnapshot, TokenFlags,
};

pub use super::mechdown::FenceDelimiter;
use super::mechdown;
use super::recovery::Attempt;
use super::{Parser, document, mech, sink};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FragmentKind {
  Document,
  Section,
  SectionElement,
  Paragraph,
  MechItem,
  VariableDefine,
  Expression,
  ParentheticalTerm,
  CodeBlock,
}

impl FragmentKind {
  pub const fn syntax_kind(self) -> SyntaxKind {
    match self {
      Self::Document => SyntaxKind::Document,
      Self::Section => SyntaxKind::Section,
      Self::SectionElement => SyntaxKind::SectionElement,
      Self::Paragraph => SyntaxKind::Paragraph,
      Self::MechItem => SyntaxKind::MechItem,
      Self::VariableDefine => SyntaxKind::VariableDefine,
      Self::Expression => SyntaxKind::Expression,
      Self::ParentheticalTerm => SyntaxKind::ParentheticalExpression,
      Self::CodeBlock => SyntaxKind::GenericFence,
    }
  }

  pub const fn mode(self) -> ParseMode {
    match self {
      Self::Document | Self::Section | Self::SectionElement => ParseMode::Document,
      Self::Paragraph => ParseMode::Paragraph,
      Self::MechItem
      | Self::VariableDefine
      | Self::Expression
      | Self::ParentheticalTerm => ParseMode::Mech,
      Self::CodeBlock => ParseMode::Fence,
    }
  }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParseMode {
  Document,
  Paragraph,
  Mech,
  Fence,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParseContext {
  pub mode: ParseMode,
  pub delimiter_depth: u16,
  pub line_start: bool,
  pub indentation: u32,
  pub enclosing_fence: Option<FenceDelimiter>,
}

impl ParseContext {
  pub const fn for_kind(kind: FragmentKind) -> Self {
    Self {
      mode: kind.mode(),
      delimiter_depth: 0,
      line_start: matches!(
        kind,
        FragmentKind::Document
          | FragmentKind::Section
          | FragmentKind::SectionElement
          | FragmentKind::Paragraph
          | FragmentKind::CodeBlock
      ),
      indentation: 0,
      enclosing_fence: None,
    }
  }
}

#[derive(Clone, Debug)]
pub struct FragmentSnapshot {
  pub source: TextSnapshot,
  pub range: TextRange,
  pub kind: FragmentKind,
  pub context: ParseContext,
  pub root: Arc<GreenNode>,
  pub diagnostics: DiagnosticStore,
  pub nodes: NodeIndex,
  pub stats: ParseStats,
  pub matched: bool,
  pub consumed: TextRange,
  pub consumed_complete: bool,
}

impl FragmentSnapshot {
  pub fn syntax(&self) -> SyntaxNode {
    SyntaxNode::new_root_at(
      self.root.clone(),
      self.source.clone(),
      self.range.start,
    )
  }
}

pub fn parse_fragment(
  source: &TextSnapshot,
  range: TextRange,
  kind: FragmentKind,
  context: ParseContext,
  config: super::ParseConfig,
  ids: &mut IdGenerator,
) -> FragmentSnapshot {
  let mut parser = Parser::for_range(
    source,
    range,
    kind.syntax_kind(),
    u32::from(context.delimiter_depth),
    config,
    ids,
  );
  let context_matches = context_matches(&parser, kind, context);
  let start = parser.offset();
  let matched = context_matches && parse_requested(&mut parser, kind);
  let end = parser.offset();
  let halted = parser.is_halted();
  let output = parser.finish();
  let sink_result = sink(&output.events, source, ids)
    .ok()
    .filter(|result| result.root.kind == kind.syntax_kind())
    .unwrap_or_else(|| fallback_fragment(source, range, kind.syntax_kind(), ids));

  let nodes = NodeIndex::build_at(&sink_result.root, range.start);
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
  let consumed = TextRange::new(start, end);
  FragmentSnapshot {
    source: source.clone(),
    range,
    kind,
    context,
    root: sink_result.root,
    diagnostics,
    nodes,
    stats: output.stats,
    matched,
    consumed,
    consumed_complete: matched && !halted && consumed == range,
  }
}

fn context_matches(
  parser: &Parser<'_>,
  kind: FragmentKind,
  context: ParseContext,
) -> bool {
  if context.mode != kind.mode() {
    return false;
  }
  if matches!(
    kind,
    FragmentKind::Document
      | FragmentKind::Section
      | FragmentKind::SectionElement
      | FragmentKind::Paragraph
      | FragmentKind::CodeBlock
  ) && parser.cursor().is_line_start() != context.line_start
  {
    return false;
  }
  if kind == FragmentKind::CodeBlock {
    let Some(found) = mechdown::fence_delimiter(parser.cursor()) else {
      return false;
    };
    if found.indentation_bytes != context.indentation {
      return false;
    }
    if context
      .enclosing_fence
      .is_some_and(|delimiter| delimiter != found.delimiter)
    {
      return false;
    }
  }
  true
}

fn parse_requested(parser: &mut Parser<'_>, kind: FragmentKind) -> bool {
  match kind {
    FragmentKind::Document => {
      document::parse_document_root(parser);
      true
    }
    FragmentKind::Section => {
      document::parse_section(parser);
      true
    }
    FragmentKind::SectionElement => {
      matches!(
        document::parse_section_element(parser),
        Attempt::Match(()) | Attempt::CommittedFailure(_)
      )
    }
    FragmentKind::Paragraph => {
      if parser.is_eof() {
        return false;
      }
      mechdown::parse_paragraph(parser);
      true
    }
    FragmentKind::MechItem => {
      matches!(
        mech::parse_mech_item(parser),
        Attempt::Match(()) | Attempt::CommittedFailure(_)
      )
    }
    FragmentKind::VariableDefine => {
      matches!(
        mech::parse_variable_definition(parser),
        Attempt::Match(()) | Attempt::CommittedFailure(_)
      )
    }
    FragmentKind::Expression => {
      matches!(
        mech::parse_expression(parser),
        Attempt::Match(()) | Attempt::CommittedFailure(_)
      )
    }
    FragmentKind::ParentheticalTerm => {
      if !parser.cursor().starts_with("(") {
        return false;
      }
      matches!(
        mech::parse_parenthetical(parser),
        Attempt::Match(()) | Attempt::CommittedFailure(_)
      )
    }
    FragmentKind::CodeBlock => {
      if mechdown::fence_delimiter(parser.cursor()).is_none() {
        return false;
      }
      mechdown::parse_generic_fence(parser);
      true
    }
  }
}

fn fallback_fragment(
  source: &TextSnapshot,
  range: TextRange,
  kind: SyntaxKind,
  ids: &mut IdGenerator,
) -> super::SinkResult {
  let mut builder = GreenBuilder::new(ids);
  builder.start_node_with_flags(kind, NodeFlags::ERROR | NodeFlags::CONTAINS_ERROR);
  if !range.is_empty() {
    builder.start_node_with_flags(SyntaxKind::Error, NodeFlags::ERROR);
    if let Ok(text) = source.text(range) {
      let _ = builder.token_with_flags(SyntaxKind::Unknown, &text, TokenFlags::ERROR);
    }
    let _ = builder.finish_node();
  }
  let _ = builder.finish_node();
  let root = builder.finish().unwrap_or_else(|_| {
    Arc::new(GreenNode {
      id: ids.node(),
      kind,
      text_len: TextSize::ZERO,
      children: Arc::from([]),
      flags: NodeFlags::ERROR,
      structural_hash: 0,
    })
  });
  super::SinkResult {
    root,
    event_nodes: Default::default(),
  }
}
