use alloc::sync::Arc;

use crate::document::{
    DiagnosticAnchor, DiagnosticStore, GreenBuilder, GreenNode, IdGenerator, NodeFlags, NodeIndex,
    ParseStats, SyntaxKind, SyntaxNode, TextRange, TextSize, TextSnapshot, TokenFlags,
};

use super::{LexicalMode, Parser, canonical_fragment_rule, sink};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GrammarFragmentKind {
    Grammar,
    GrammarRule,
    GrammarExpression,
    GrammarTerm,
    GrammarFactor,
    GrammarTerminalToken,
}

impl GrammarFragmentKind {
    pub const fn syntax_kind(self) -> SyntaxKind {
        match self {
            Self::Grammar => SyntaxKind::Grammar,
            Self::GrammarRule => SyntaxKind::GrammarRule,
            Self::GrammarExpression => SyntaxKind::GrammarExpression,
            Self::GrammarTerm => SyntaxKind::GrammarTerm,
            Self::GrammarFactor => SyntaxKind::GrammarFactor,
            Self::GrammarTerminalToken => SyntaxKind::GrammarTerminalToken,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GrammarFragmentContext {
    pub delimiter_depth: u16,
}

#[derive(Clone, Debug)]
pub struct GrammarFragmentSnapshot {
    pub source: TextSnapshot,
    pub range: TextRange,
    pub kind: GrammarFragmentKind,
    pub context: GrammarFragmentContext,
    pub root: Arc<GreenNode>,
    pub diagnostics: DiagnosticStore,
    pub nodes: NodeIndex,
    pub stats: ParseStats,
    pub matched: bool,
    pub consumed: TextRange,
    pub consumed_complete: bool,
}

impl GrammarFragmentSnapshot {
    pub fn syntax(&self) -> SyntaxNode {
        SyntaxNode::new_root_at(self.root.clone(), self.source.clone(), self.range.start)
    }
}

/// Parse one of the six canonical grammar roots over a bounded retained range.
pub fn parse_canonical_grammar_fragment(
    source: &TextSnapshot,
    range: TextRange,
    kind: GrammarFragmentKind,
    context: GrammarFragmentContext,
    config: super::ParseConfig,
    ids: &mut IdGenerator,
) -> GrammarFragmentSnapshot {
    // Reject invalid public byte bounds before a canonical classifier can inspect
    // a scalar at an endpoint inside UTF-8. Such a range has no parseable text.
    if source.validate_range(range).is_err() {
        let fallback = fallback_fragment(source, range, kind.syntax_kind(), ids);
        let nodes = NodeIndex::build_at(&fallback.root, range.start);
        return GrammarFragmentSnapshot {
            source: source.clone(),
            range,
            kind,
            context,
            root: fallback.root,
            diagnostics: DiagnosticStore::new(source.revision()),
            nodes,
            stats: ParseStats::default(),
            matched: false,
            consumed: TextRange::empty(range.start),
            consumed_complete: false,
        };
    }
    let resource_rule = canonical_fragment_rule(kind.syntax_kind());
    let mut parser = Parser::for_range(
        source,
        range,
        LexicalMode::CanonicalGrammar,
        resource_rule,
        u32::from(context.delimiter_depth),
        config,
        ids,
    );
    let start = parser.offset();
    let matched = super::canonical::roots::parse_grammar_fragment(&mut parser, kind.syntax_kind());
    let end = parser.offset();
    let halted = parser.is_halted();
    let output = parser.finish();
    let sink_result = sink(&output.events, source, ids)
        .ok()
        .filter(|result| result.root.kind == kind.syntax_kind())
        .unwrap_or_else(|| fallback_fragment(source, range, kind.syntax_kind(), ids));

    let nodes = NodeIndex::build_at(&sink_result.root, range.start);
    let mut diagnostics = DiagnosticStore::new(source.revision());
    for mut pending in output.diagnostics.iter().cloned() {
        if let Some(event) = pending.event
            && let Some(node) = output
                .events
                .get(event)
                .and_then(|event| match event {
                    super::Event::Start { identity, .. } => identity.as_ref(),
                    _ => None,
                })
                .or_else(|| sink_result.event_nodes.get(&event))
        {
            pending.diagnostic.primary = DiagnosticAnchor::Element {
                element: crate::document::SyntaxElementId::Node(*node),
                relative: pending.relative,
            };
        }
        diagnostics.push(pending.diagnostic);
    }
    let consumed = TextRange::new(start, end);
    GrammarFragmentSnapshot {
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

fn fallback_fragment(
    source: &TextSnapshot,
    range: TextRange,
    kind: SyntaxKind,
    ids: &mut IdGenerator,
) -> super::SinkResult {
    let mut builder = GreenBuilder::new(ids);
    builder.start_node_with_flags(kind, NodeFlags::ERROR | NodeFlags::CONTAINS_ERROR);
    // Parser::for_range deliberately turns invalid public ranges into a halted
    // parse. Do not try to slice that same invalid range while constructing the
    // failed snapshot: an endpoint inside a scalar is not a legal str index.
    if !range.is_empty() && source.validate_range(range).is_ok() {
        builder.start_node_with_flags(SyntaxKind::Error, NodeFlags::ERROR);
        source.for_each_slice(range, |text| {
            let _ = builder.token_with_flags(SyntaxKind::Unknown, text, TokenFlags::ERROR);
        });
        let _ = builder.finish_node();
    }
    let _ = builder.finish_node();
    let root = builder.finish().unwrap_or_else(|_| {
        Arc::new(GreenNode {
            id: ids.node(),
            kind,
            text_len: TextSize::ZERO,
            children: Default::default(),
            flags: NodeFlags::ERROR,
            structural_hash: 0,
        })
    });
    super::SinkResult {
        root,
        event_nodes: Default::default(),
    }
}
