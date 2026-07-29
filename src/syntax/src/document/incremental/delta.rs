use alloc::vec::Vec;

use crate::document::{DiagnosticId, NodeId, Revision, TextRange};

use super::ReparseStats;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DiagnosticDelta {
  pub added: Vec<DiagnosticId>,
  pub removed: Vec<DiagnosticId>,
  pub retained: Vec<DiagnosticId>,
}

#[derive(Clone, Debug)]
pub struct DocumentUpdate {
  pub old_revision: Revision,
  pub new_revision: Revision,
  pub changed_range: TextRange,
  pub reparsed_roots: Vec<NodeId>,
  pub reused_roots: Vec<NodeId>,
  pub diagnostics: DiagnosticDelta,
  pub stats: ReparseStats,
}
