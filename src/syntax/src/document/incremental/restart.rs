use crate::document::{NodeId, SyntaxKind, SyntaxSnapshot, TextRange};

use super::ChangeMap;

const SUPPORTED_ROOTS: &[SyntaxKind] = &[
  SyntaxKind::VariableDefine,
  SyntaxKind::MechItem,
  SyntaxKind::Paragraph,
  SyntaxKind::GenericFence,
  SyntaxKind::Subtitle,
  SyntaxKind::UlSubtitle,
  SyntaxKind::SectionElement,
  SyntaxKind::Section,
  SyntaxKind::Document,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReparseRoot {
  pub node: NodeId,
  pub kind: SyntaxKind,
  pub range: TextRange,
}

pub fn select_reparse_root(
  snapshot: &SyntaxSnapshot,
  changes: &ChangeMap,
) -> ReparseRoot {
  let changed = changes.old_changed_range();
  let mut selected = snapshot
    .restarts
    .iter()
    .filter_map(|entry| {
      let record = snapshot.nodes.node(entry.node)?;
      if !SUPPORTED_ROOTS.contains(&record.kind)
        || !contains_change(record.range, changed)
      {
        return None;
      }
      Some(ReparseRoot {
        node: entry.node,
        kind: record.kind,
        range: record.range,
      })
    })
    .min_by_key(|root| root.range.len().0)
    .unwrap_or_else(|| document_root(snapshot));

  let context_change = changes.changes_parser_context(|range| {
    snapshot.source.text(range).unwrap_or_default()
  });
  if changes.touches_boundary(selected.range) {
    selected = expand_boundary_root(snapshot, selected);
  }
  if context_change {
    selected = parent_supported_root(snapshot, selected)
      .unwrap_or_else(|| document_root(snapshot));
    if selected.kind == SyntaxKind::SectionElement {
      selected = parent_supported_root(snapshot, selected)
        .unwrap_or_else(|| document_root(snapshot));
    }
  }
  selected
}

fn expand_boundary_root(
  snapshot: &SyntaxSnapshot,
  root: ReparseRoot,
) -> ReparseRoot {
  let mut expanded = parent_supported_root(snapshot, root)
    .unwrap_or_else(|| document_root(snapshot));
  if expanded.kind == SyntaxKind::SectionElement {
    expanded = parent_supported_root(snapshot, expanded)
      .unwrap_or_else(|| document_root(snapshot));
  }
  expanded
}

pub fn parent_supported_root(
  snapshot: &SyntaxSnapshot,
  root: ReparseRoot,
) -> Option<ReparseRoot> {
  let mut parent = snapshot.nodes.node(root.node)?.parent;
  while let Some(node) = parent {
    let record = snapshot.nodes.node(node)?;
    if SUPPORTED_ROOTS.contains(&record.kind) {
      return Some(ReparseRoot {
        node,
        kind: record.kind,
        range: record.range,
      });
    }
    parent = record.parent;
  }
  None
}

fn contains_change(container: TextRange, changed: TextRange) -> bool {
  if changed.is_empty() {
    container.contains_inclusive(changed.start)
  } else {
    container.contains_range(changed)
  }
}

fn document_root(snapshot: &SyntaxSnapshot) -> ReparseRoot {
  ReparseRoot {
    node: snapshot.root.id,
    kind: SyntaxKind::Document,
    range: snapshot.source.full_range(),
  }
}
