use alloc::collections::BTreeSet;
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::document::green::{child_text_len, hash_node, propagated_flags};
use crate::document::parser::{
  build_restart_index, parse_document, parse_document_with_ids,
};
use crate::document::{
  Diagnostic, DiagnosticAnchor, DiagnosticId, DiagnosticStore, DocumentId, GreenElement,
  GreenNode, IdGenerator, NodeFlags, NodeId, ParseConfig, RecoveryAction, Revision,
  SourceError, SyntaxElementId, SyntaxKind, SyntaxSnapshot, TextEdit, TextRange, TextSize,
  TextSnapshot,
};

use super::change_map::{Affinity, ChangeMap};
use super::restart::{
  ReparseRoot, parent_supported_root, select_reparse_root,
};
use super::{DiagnosticDelta, ReparseStats};

pub(crate) struct ReparseResult {
  pub snapshot: SyntaxSnapshot,
  pub reparsed_roots: Vec<NodeId>,
  pub reused_roots: Vec<NodeId>,
  pub diagnostics: DiagnosticDelta,
  pub stats: ReparseStats,
}

pub(crate) fn reparse(
  old: &SyntaxSnapshot,
  edits: &[TextEdit],
  config: ParseConfig,
  ids: &mut IdGenerator,
) -> Result<ReparseResult, SourceError> {
  let source = old.source.apply_edits(edits)?;
  let changes = ChangeMap::new(edits);
  let mut root = select_reparse_root(old, &changes);
  let mut attempted = 0_u64;

  loop {
    attempted = attempted.saturating_add(1);
    if let Some(result) =
      try_reparse_root(old, &source, &changes, root, config, ids)
    {
      return Ok(finish_result(
        old,
        result,
        root,
        changes.new_changed_range(),
        attempted,
      ));
    }
    let Some(parent) = parent_supported_root(old, root) else {
      let parsed = parse_document_with_ids(source.clone(), config, ids);
      return Ok(finish_full_fallback(
        old,
        parsed,
        changes.new_changed_range(),
        attempted,
      ));
    };
    root = parent;
  }
}

struct RootResult {
  snapshot: SyntaxSnapshot,
  fragment_stats: crate::document::ParseStats,
}

fn try_reparse_root(
  old: &SyntaxSnapshot,
  source: &TextSnapshot,
  changes: &ChangeMap,
  root: ReparseRoot,
  config: ParseConfig,
  ids: &mut IdGenerator,
) -> Option<RootResult> {
  if root.kind == SyntaxKind::Document {
    let snapshot = parse_document_with_ids(source.clone(), config, ids);
    let fragment_stats = snapshot.stats;
    return Some(RootResult {
      snapshot,
      fragment_stats,
    });
  }

  let mapped = changes.map_range(root.range);
  if mapped.end.0 > source.byte_len().0 || !mapped.contains_range(changes.new_changed_range()) {
    return None;
  }
  let fragment_text = source.text(mapped).ok()?;
  let fragment_source = TextSnapshot::new(
    source.document(),
    source.revision(),
    fragment_text.as_str(),
  )
  .ok()?;
  let fragment = parse_document_with_ids(fragment_source, config, ids);
  let (replacement, fragment_range) =
    find_node_with_range(&fragment.root, root.kind, TextSize::ZERO)?;
  if fragment_range != TextRange::new(TextSize::ZERO, mapped.len()) {
    return None;
  }
  if root.kind == SyntaxKind::Section
    && root.range.len().0 > 0
    && replacement.text_len.0 == 0
  {
    return None;
  }
  if root.kind == SyntaxKind::Section {
    let original = find_node_by_id(&old.root, root.node)?;
    if has_direct_child(original, SyntaxKind::UlSubtitle)
      && !has_direct_child(&replacement, SyntaxKind::UlSubtitle)
    {
      return None;
    }
  }
  if replacement
    .flags
    .intersects(NodeFlags::MISSING | NodeFlags::CONTAINS_MISSING)
    && mapped.end < source.byte_len()
    && !tail_starts_with_ul_subtitle(source, mapped.end, config)
  {
    return None;
  }

  let new_root = splice_node(&old.root, root.node, replacement, ids)?.0;
  if root.kind != SyntaxKind::Section
    && !section_envelope_matches(
      old,
      source,
      changes,
      root,
      &new_root,
      config,
    )
  {
    return None;
  }
  let mut snapshot = SyntaxSnapshot::new(
    source.clone(),
    new_root,
    DiagnosticStore::new(source.revision()),
  );
  snapshot.diagnostics = merge_diagnostics(
    old,
    &fragment,
    &snapshot,
    mapped.start,
    changes,
  );
  snapshot.restarts = build_restart_index(&snapshot);
  snapshot.stats = fragment.stats;
  Some(RootResult {
    snapshot,
    fragment_stats: fragment.stats,
  })
}

fn finish_result(
  old: &SyntaxSnapshot,
  mut result: RootResult,
  root: ReparseRoot,
  _changed: TextRange,
  attempted: u64,
) -> ReparseResult {
  let old_ids = collect_node_ids(&old.root);
  let new_ids = collect_node_ids(&result.snapshot.root);
  let reused = old_ids
    .intersection(&new_ids)
    .copied()
    .collect::<Vec<_>>();
  let new_count = new_ids.difference(&old_ids).count() as u64;
  let diagnostics = diagnostic_delta(old, &result.snapshot);
  let stats = ReparseStats {
    source_bytes: u64::from(result.snapshot.source.byte_len().0),
    parser_steps: result.fragment_stats.parser_steps,
    events_emitted: result.fragment_stats.events_emitted,
    diagnostics_emitted: result.snapshot.diagnostics.len() as u64,
    recovery_bytes: result.fragment_stats.recovery_bytes,
    reparse_root_count: 1,
    reused_node_count: reused.len() as u64,
    new_node_count: new_count,
    attempted_roots: attempted,
    document_fallbacks: u64::from(root.kind == SyntaxKind::Document),
  };
  result.snapshot.stats.reparse_root_count = 1;
  result.snapshot.stats.source_bytes =
    u64::from(result.snapshot.source.byte_len().0);
  result.snapshot.stats.diagnostics_emitted =
    result.snapshot.diagnostics.len() as u64;
  result.snapshot.stats.reused_node_count = reused.len() as u64;
  result.snapshot.stats.new_node_count = new_count;
  ReparseResult {
    snapshot: result.snapshot,
    reparsed_roots: alloc::vec![root.node],
    reused_roots: reused,
    diagnostics,
    stats,
  }
}

fn finish_full_fallback(
  old: &SyntaxSnapshot,
  snapshot: SyntaxSnapshot,
  changed: TextRange,
  attempted: u64,
) -> ReparseResult {
  let root = ReparseRoot {
    node: old.root.id,
    kind: SyntaxKind::Document,
    range: old.source.full_range(),
  };
  let result = RootResult {
    fragment_stats: snapshot.stats,
    snapshot,
  };
  let mut finished = finish_result(old, result, root, changed, attempted);
  finished.stats.document_fallbacks = 1;
  finished
}

fn splice_node(
  node: &Arc<GreenNode>,
  target: NodeId,
  replacement: Arc<GreenNode>,
  ids: &mut IdGenerator,
) -> Option<(Arc<GreenNode>, bool)> {
  if node.id == target {
    return Some((replacement, true));
  }
  let mut changed = false;
  let mut children = Vec::with_capacity(node.children.len());
  for child in node.children.iter() {
    match child {
      GreenElement::Node(child_node) => {
        if let Some((new_child, child_changed)) =
          splice_node(child_node, target, replacement.clone(), ids)
        {
          changed |= child_changed;
          children.push(GreenElement::Node(new_child));
        } else {
          children.push(GreenElement::Node(child_node.clone()));
        }
      }
      GreenElement::Token(token) => children.push(GreenElement::Token(*token)),
    }
  }
  if !changed {
    return None;
  }
  let explicit = NodeFlags(
    node.flags.0
      & (NodeFlags::ERROR.0
        | NodeFlags::MISSING.0
        | NodeFlags::REPARSE_ROOT.0),
  );
  let flags = propagated_flags(node.kind, explicit, &children);
  let rebuilt = Arc::new(GreenNode {
    id: ids.node(),
    kind: node.kind,
    text_len: child_text_len(&children),
    structural_hash: hash_node(node.kind, &children),
    children: children.into(),
    flags,
  });
  Some((rebuilt, true))
}

fn find_node_with_range(
  node: &Arc<GreenNode>,
  kind: SyntaxKind,
  start: TextSize,
) -> Option<(Arc<GreenNode>, TextRange)> {
  if node.kind == kind {
    return Some((node.clone(), TextRange::at(start, node.text_len)));
  }
  let mut offset = start;
  for child in node.children.iter() {
    if let GreenElement::Node(child) = child {
      if let Some(found) = find_node_with_range(child, kind, offset) {
        return Some(found);
      }
    }
    offset += child.text_len();
  }
  None
}

fn find_node_by_id(
  node: &Arc<GreenNode>,
  id: NodeId,
) -> Option<&Arc<GreenNode>> {
  if node.id == id {
    return Some(node);
  }
  node.children.iter().find_map(|child| match child {
    GreenElement::Node(child) => find_node_by_id(child, id),
    GreenElement::Token(_) => None,
  })
}

fn has_direct_child(node: &GreenNode, kind: SyntaxKind) -> bool {
  node.children.iter().any(|child| {
    matches!(child, GreenElement::Node(child) if child.kind == kind)
  })
}

fn tail_starts_with_ul_subtitle(
  source: &TextSnapshot,
  start: TextSize,
  config: ParseConfig,
) -> bool {
  let tail = TextRange::new(start, source.byte_len());
  let Ok(text) = source.text(tail) else {
    return false;
  };
  let Ok(tail_source) = TextSnapshot::new(
    source.document(),
    source.revision(),
    text.as_str(),
  ) else {
    return false;
  };
  let parsed = parse_document(tail_source, config);
  parsed
    .root
    .children
    .iter()
    .find_map(|child| match child {
      GreenElement::Node(child) if child.kind == SyntaxKind::Body => {
        Some(child)
      }
      _ => None,
    })
    .and_then(|body| {
      body.children.iter().find_map(|child| match child {
        GreenElement::Node(child) if child.kind == SyntaxKind::Section => {
          Some(child)
        }
        _ => None,
      })
    })
    .is_some_and(|section| {
      has_direct_child(section, SyntaxKind::UlSubtitle)
    })
}

fn section_envelope_matches(
  old: &SyntaxSnapshot,
  source: &TextSnapshot,
  changes: &ChangeMap,
  root: ReparseRoot,
  new_root: &Arc<GreenNode>,
  config: ParseConfig,
) -> bool {
  let Some(section) =
    ancestor_record(old, root.node, SyntaxKind::Section)
  else {
    return true;
  };
  let mapped = changes.map_range(section.range);
  if mapped.end > source.byte_len() {
    return false;
  }
  let Some(actual) =
    find_node_at_range(new_root, SyntaxKind::Section, mapped, TextSize::ZERO)
  else {
    return false;
  };
  let Ok(text) = source.text(mapped) else {
    return false;
  };
  let Ok(fragment_source) = TextSnapshot::new(
    source.document(),
    source.revision(),
    text.as_str(),
  ) else {
    return false;
  };
  let expected_snapshot = parse_document(fragment_source, config);
  let expected_range = TextRange::at(TextSize::ZERO, mapped.len());
  let Some((expected, range)) = find_node_with_range(
    &expected_snapshot.root,
    SyntaxKind::Section,
    TextSize::ZERO,
  ) else {
    return false;
  };
  range == expected_range
    && actual.structural_hash == expected.structural_hash
    && actual.flags == expected.flags
}

fn ancestor_record<'a>(
  snapshot: &'a SyntaxSnapshot,
  node: NodeId,
  kind: SyntaxKind,
) -> Option<&'a crate::document::NodeRecord> {
  let mut current = Some(node);
  while let Some(node) = current {
    let record = snapshot.nodes.node(node)?;
    if record.kind == kind {
      return Some(record);
    }
    current = record.parent;
  }
  None
}

fn find_node_at_range(
  node: &Arc<GreenNode>,
  kind: SyntaxKind,
  target: TextRange,
  start: TextSize,
) -> Option<Arc<GreenNode>> {
  let range = TextRange::at(start, node.text_len);
  if node.kind == kind && range == target {
    return Some(node.clone());
  }
  if !range.contains_range(target) {
    return None;
  }
  let mut offset = start;
  for child in node.children.iter() {
    if let GreenElement::Node(child) = child {
      if let Some(found) =
        find_node_at_range(child, kind, target, offset)
      {
        return Some(found);
      }
    }
    offset += child.text_len();
  }
  None
}

fn merge_diagnostics(
  old: &SyntaxSnapshot,
  fragment: &SyntaxSnapshot,
  new_snapshot: &SyntaxSnapshot,
  fragment_offset: TextSize,
  changes: &ChangeMap,
) -> DiagnosticStore {
  let mut diagnostics = Vec::new();
  for diagnostic in old.diagnostics.iter().cloned() {
    let diagnostic =
      map_old_diagnostic(diagnostic, changes, new_snapshot.revision);
    if matches!(diagnostic.primary, DiagnosticAnchor::Element { .. })
      && diagnostic
        .primary
        .resolve(new_snapshot.revision, &new_snapshot.nodes)
        .is_some()
    {
      diagnostics.push(diagnostic);
    }
  }
  for diagnostic in fragment.diagnostics.iter().cloned() {
    let shifted = shift_diagnostic(diagnostic, fragment_offset);
    if shifted
      .primary
      .resolve(new_snapshot.revision, &new_snapshot.nodes)
      .is_some()
    {
      diagnostics.push(shifted);
    }
  }
  diagnostics.sort_by_key(|diagnostic| {
    diagnostic
      .primary
      .resolve(new_snapshot.revision, &new_snapshot.nodes)
      .map(|range| (range.start.0, diagnostic.code.0.clone()))
      .unwrap_or((u32::MAX, diagnostic.code.0.clone()))
  });
  let mut store = DiagnosticStore::new(new_snapshot.revision);
  for diagnostic in diagnostics {
    store.push(diagnostic);
  }
  store
}

fn map_old_diagnostic(
  mut diagnostic: Diagnostic,
  changes: &ChangeMap,
  revision: Revision,
) -> Diagnostic {
  map_old_anchor(&mut diagnostic.primary, changes, revision);
  for label in &mut diagnostic.labels {
    map_old_anchor(&mut label.anchor, changes, revision);
  }
  for fix in &mut diagnostic.fixes {
    for edit in &mut fix.edits {
      edit.delete = changes.map_range(edit.delete);
    }
  }
  if let Some(recovery) = &mut diagnostic.recovery {
    match recovery {
      RecoveryAction::Insert { at, .. }
      | RecoveryAction::Abandon { at, .. } => {
        *at = changes.map_offset(*at, Affinity::After);
      }
      RecoveryAction::Skip { range }
      | RecoveryAction::ResourceLimit { range } => {
        *range = changes.map_range(*range);
      }
    }
  }
  diagnostic
}

fn map_old_anchor(
  anchor: &mut DiagnosticAnchor,
  changes: &ChangeMap,
  revision: Revision,
) {
  if let DiagnosticAnchor::Absolute {
    revision: anchor_revision,
    range,
  } = anchor
  {
    *anchor_revision = revision;
    *range = changes.map_range(*range);
  }
}

fn shift_diagnostic(mut diagnostic: Diagnostic, offset: TextSize) -> Diagnostic {
  shift_anchor(&mut diagnostic.primary, offset);
  for label in &mut diagnostic.labels {
    shift_anchor(&mut label.anchor, offset);
  }
  for fix in &mut diagnostic.fixes {
    for edit in &mut fix.edits {
      edit.delete.start += offset;
      edit.delete.end += offset;
    }
  }
  if let Some(recovery) = &mut diagnostic.recovery {
    match recovery {
      RecoveryAction::Insert { at, .. }
      | RecoveryAction::Abandon { at, .. } => *at += offset,
      RecoveryAction::Skip { range }
      | RecoveryAction::ResourceLimit { range } => {
        range.start += offset;
        range.end += offset;
      }
    }
  }
  diagnostic
}

fn shift_anchor(anchor: &mut DiagnosticAnchor, offset: TextSize) {
  if let DiagnosticAnchor::Absolute { range, .. } = anchor {
    range.start += offset;
    range.end += offset;
  }
}

fn collect_node_ids(root: &GreenNode) -> BTreeSet<NodeId> {
  let mut ids = BTreeSet::new();
  collect_ids(root, &mut ids);
  ids
}

fn collect_ids(node: &GreenNode, ids: &mut BTreeSet<NodeId>) {
  ids.insert(node.id);
  for child in node.children.iter() {
    if let GreenElement::Node(child) = child {
      collect_ids(child, ids);
    }
  }
}

fn diagnostic_delta(
  old: &SyntaxSnapshot,
  new: &SyntaxSnapshot,
) -> DiagnosticDelta {
  let old_ids = old
    .diagnostics
    .iter()
    .map(|diagnostic| diagnostic.id)
    .collect::<BTreeSet<DiagnosticId>>();
  let new_ids = new
    .diagnostics
    .iter()
    .map(|diagnostic| diagnostic.id)
    .collect::<BTreeSet<DiagnosticId>>();
  DiagnosticDelta {
    added: new_ids.difference(&old_ids).copied().collect(),
    removed: old_ids.difference(&new_ids).copied().collect(),
    retained: old_ids.intersection(&new_ids).copied().collect(),
  }
}
