//! Shared hidden support for direct canonical source-rule contract tests.
//!
//! These helpers deliberately parse physical source fragments. They are not
//! public parser roots and must not be used by production document parsing.

use alloc::sync::Arc;

use crate::document::{
    DiagnosticAnchor, DiagnosticStore, GreenNode, IdGenerator, NodeFlags, NodeIndex, ParseStats,
    RuleId, SyntaxKind, SyntaxNode, TextRange, TextSnapshot,
};

use super::super::{LexicalMode, ParseConfig, Parser, sink};
use super::combinator::Attempt;

/// A narrow prefix snapshot used to exercise canonical source productions.
#[derive(Clone, Debug)]
pub struct CanonicalSourceRuleSnapshot {
    pub source: TextSnapshot,
    pub rule: RuleId,
    pub root: Arc<GreenNode>,
    pub diagnostics: DiagnosticStore,
    pub nodes: NodeIndex,
    pub stats: ParseStats,
    pub matched: bool,
    pub consumed: TextRange,
}

impl CanonicalSourceRuleSnapshot {
    pub fn syntax(&self) -> SyntaxNode {
        SyntaxNode::new_root_at(
            self.root.clone(),
            self.source.clone(),
            self.consumed.start,
        )
    }

    pub fn is_strictly_clean(&self) -> bool {
        self.matched
            && self.diagnostics.is_empty()
            && !self.root.flags.intersects(
                NodeFlags::ERROR
                    | NodeFlags::MISSING
                    | NodeFlags::CONTAINS_ERROR
                    | NodeFlags::CONTAINS_MISSING,
            )
    }
}

/// Parse a single canonical production as a deterministic source prefix.
///
/// This remains internal test support: callers select one already-closed
/// production and receive its lossless fragment snapshot. Physical source is
/// intentionally preserved, including whitespace for diagnostic reporting.
pub(crate) fn parse_source_rule_prefix(
    source: TextSnapshot,
    rule: RuleId,
    config: ParseConfig,
    parse: impl FnOnce(&mut Parser<'_>) -> Attempt,
) -> CanonicalSourceRuleSnapshot {
    let mut ids = IdGenerator::new();
    let mut parser = Parser::new(
        &source,
        LexicalMode::CanonicalSourceFragment,
        config,
        &mut ids,
    );
    parser.set_resource_rule(rule);
    let fragment = parser.start();
    let start = parser.offset();
    let matched = parse(&mut parser).accepted();
    let end = parser.offset();
    fragment.complete(&mut parser, SyntaxKind::CanonicalFragment);
    let output = parser.finish();
    let sink_result = sink(&output.events, &source, &mut ids)
        .expect("canonical source-rule events must form one root");

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
    CanonicalSourceRuleSnapshot {
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
