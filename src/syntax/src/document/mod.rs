#![forbid(unsafe_code)]

//! Experimental, lossless document syntax infrastructure.
//!
//! Production parsing still uses the legacy Nom parser and existing `Program`
//! AST. Nothing in this module is called by the public legacy `parse` path.

extern crate alloc;

pub mod annotation;
pub mod builder;
pub mod diagnostic;
pub mod edit;
pub mod flags;
pub mod green;
pub mod ids;
pub mod index;
pub mod kind;
pub mod line_index;
pub mod pointer;
pub mod red;
pub mod source;

use alloc::sync::Arc;
use alloc::vec::Vec;

pub use annotation::*;
pub use builder::*;
pub use diagnostic::*;
pub use edit::*;
pub use flags::*;
pub use green::*;
pub use ids::*;
pub use index::*;
pub use kind::*;
pub use line_index::*;
pub use pointer::*;
pub use red::*;
pub use source::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RestartMode {
  Document,
  Paragraph,
  Mech,
  Fence,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RestartEntry {
  pub node: NodeId,
  pub range: TextRange,
  pub mode: RestartMode,
  pub delimiter_depth: u32,
  pub line_start: bool,
  pub indentation: u32,
}

#[derive(Clone, Debug, Default)]
pub struct RestartIndex {
  entries: Vec<RestartEntry>,
}

impl RestartIndex {
  pub fn push(&mut self, entry: RestartEntry) {
    self.entries.push(entry);
  }

  pub fn iter(&self) -> impl Iterator<Item = &RestartEntry> {
    self.entries.iter()
  }

  pub fn as_slice(&self) -> &[RestartEntry] {
    &self.entries
  }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ParseStats {
  pub source_bytes: u64,
  pub parser_steps: u64,
  pub events_emitted: u64,
  pub diagnostics_emitted: u64,
  pub recovery_bytes: u64,
  pub reparse_root_count: u64,
  pub reused_node_count: u64,
  pub new_node_count: u64,
}

#[derive(Clone, Debug)]
pub struct SyntaxSnapshot {
  pub document: DocumentId,
  pub revision: Revision,
  pub source: TextSnapshot,
  pub root: Arc<GreenNode>,
  pub diagnostics: DiagnosticStore,
  pub nodes: NodeIndex,
  pub restarts: RestartIndex,
  pub stats: ParseStats,
}

impl SyntaxSnapshot {
  pub fn new(
    source: TextSnapshot,
    root: Arc<GreenNode>,
    diagnostics: DiagnosticStore,
  ) -> Self {
    let document = source.document();
    let revision = source.revision();
    let nodes = NodeIndex::build(&root);
    Self {
      document,
      revision,
      source,
      root,
      diagnostics,
      nodes,
      restarts: RestartIndex::default(),
      stats: ParseStats::default(),
    }
  }

  pub fn syntax(&self) -> SyntaxNode {
    SyntaxNode::new_root(self.root.clone(), self.source.clone())
  }

  pub fn is_strictly_clean(&self) -> bool {
    self.diagnostics.is_empty()
      && !self
        .root
        .flags
        .intersects(NodeFlags::CONTAINS_ERROR | NodeFlags::CONTAINS_MISSING)
  }
}
