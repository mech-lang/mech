use alloc::sync::Arc;

use crate::document::{
  DiagnosticAnchor, DiagnosticStore, GreenNode, IdGenerator, NodeIndex, ParseStats, RuleId,
  SyntaxKind, SyntaxNode, TextRange, TextSize, TextSnapshot,
};

use super::super::{ParseConfig, Parser, sink};
use super::base;

/// A narrow prefix snapshot used to exercise canonical lexical productions.
///
/// Phase 2A exposes whole grammar parsing as the supported canonical root. This
/// wrapper keeps the individual lexical ports testable without pretending that
/// any one token production is a document root.
#[derive(Clone, Debug)]
pub struct CanonicalRuleSnapshot {
  pub source: TextSnapshot,
  pub rule: RuleId,
  pub root: Arc<GreenNode>,
  pub diagnostics: DiagnosticStore,
  pub nodes: NodeIndex,
  pub stats: ParseStats,
  pub matched: bool,
  pub consumed: TextRange,
}

impl CanonicalRuleSnapshot {
  pub fn syntax(&self) -> SyntaxNode {
    SyntaxNode::new_root_at(
      self.root.clone(),
      self.source.clone(),
      self.consumed.start,
    )
  }

  pub fn is_strictly_clean(&self) -> bool {
    self.matched && self.diagnostics.is_empty()
  }
}

#[doc(hidden)]
pub fn canonical_base_rule_supported(rule: RuleId) -> bool {
  base::supports(rule)
}

/// Parses one canonical base production for deterministic contract tests.
///
/// `tag` is caller-parameterized and therefore uses
/// [`parse_canonical_tag_for_test`] instead.
#[doc(hidden)]
pub fn parse_canonical_base_rule_for_test(
  source: TextSnapshot,
  rule: RuleId,
  config: ParseConfig,
) -> Option<CanonicalRuleSnapshot> {
  if rule == super::super::rules::TAG || !base::supports(rule) {
    return None;
  }
  Some(parse_rule_prefix(source, rule, config, |parser| {
    base::parse_rule(parser, rule)
  }))
}

/// Parses the caller-supplied exact `tag` production for deterministic tests.
#[doc(hidden)]
pub fn parse_canonical_tag_for_test(
  source: TextSnapshot,
  literal: &str,
  config: ParseConfig,
) -> CanonicalRuleSnapshot {
  parse_rule_prefix(
    source,
    super::super::rules::TAG,
    config,
    |parser| base::parse_exact_tag(parser, literal, SyntaxKind::Text),
  )
}

fn parse_rule_prefix(
  source: TextSnapshot,
  rule: RuleId,
  config: ParseConfig,
  parse: impl FnOnce(&mut Parser<'_>) -> bool,
) -> CanonicalRuleSnapshot {
  let mut ids = IdGenerator::new();
  let mut parser = Parser::new(
    &source,
    super::super::LexicalMode::CanonicalGrammar,
    config,
    &mut ids,
  );
  let fragment = parser.start();
  let start = parser.offset();
  let matched = parse(&mut parser);
  let end = parser.offset();
  fragment.complete(&mut parser, SyntaxKind::CanonicalFragment);
  let output = parser.finish();
  let sink_result =
    sink(&output.events, &source, &mut ids).expect("canonical lexical events must form one root");

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
  let nodes = NodeIndex::build_at(&sink_result.root, consumed.start);
  let mut stats = output.stats;
  stats.new_node_count = nodes.node_count() as u64;
  CanonicalRuleSnapshot {
    source,
    rule,
    root: sink_result.root,
    diagnostics,
    nodes,
    stats,
    matched,
    consumed,
  }
}

#[cfg(test)]
mod tests {
  use crate::document::{DocumentId, Revision};

  use super::*;

  #[test]
  fn unsupported_and_parameterized_rules_are_distinct() {
    let source = TextSnapshot::new(DocumentId(1), Revision(0), "x").unwrap();
    assert!(
      parse_canonical_base_rule_for_test(
        source.clone(),
        super::super::super::rules::GRAMMAR,
        ParseConfig::default(),
      )
      .is_none()
    );
    assert!(
      parse_canonical_base_rule_for_test(
        source.clone(),
        super::super::super::rules::TAG,
        ParseConfig::default(),
      )
      .is_none()
    );
    let parsed = parse_canonical_tag_for_test(source, "x", ParseConfig::default());
    assert!(parsed.matched);
    assert_eq!(parsed.consumed, TextRange::new(TextSize::ZERO, TextSize(1)));
  }
}
