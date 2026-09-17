use alloc::collections::BTreeSet;
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::document::green::{child_text_len, hash_node, propagated_flags};
use crate::document::parser::{
  Cursor, FragmentKind, FragmentSnapshot, ParseContext, ParseMode, build_restart_index,
  parse_document_with_ids, parse_fragment,
};
use crate::document::{
  Diagnostic, DiagnosticAnchor, DiagnosticId, DiagnosticStore, GreenElement, GreenNode,
  IdGenerator, NodeFlags, NodeId, ParseConfig, ParseStats, RecoveryAction, Revision,
  SourceError, SyntaxElementId, SyntaxKind, SyntaxSnapshot, TextEdit, TextRange, TextSize,
  TextSnapshot, normalize_diagnostics_in_range,
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

#[derive(Clone, Copy, Debug, Default)]
struct ParserWork {
  parser_steps: u64,
  events_emitted: u64,
  recovery_bytes: u64,
}

impl ParserWork {
  fn add_stats(&mut self, stats: ParseStats) {
    self.parser_steps =
      self.parser_steps.saturating_add(stats.parser_steps);
    self.events_emitted =
      self.events_emitted.saturating_add(stats.events_emitted);
    self.recovery_bytes =
      self.recovery_bytes.saturating_add(stats.recovery_bytes);
  }

  fn add_work(&mut self, work: Self) {
    self.parser_steps =
      self.parser_steps.saturating_add(work.parser_steps);
    self.events_emitted =
      self.events_emitted.saturating_add(work.events_emitted);
    self.recovery_bytes =
      self.recovery_bytes.saturating_add(work.recovery_bytes);
  }
}

#[derive(Clone, Copy, Debug, Default)]
struct ReparseWork {
  fragment: ParserWork,
  validation: ParserWork,
  rejected: ParserWork,
  fallback: ParserWork,
}

impl ReparseWork {
  fn total(self) -> ParserWork {
    let mut total = self.fragment;
    total.add_work(self.validation);
    total.add_work(self.rejected);
    total.add_work(self.fallback);
    total
  }
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
  let mut work = ReparseWork::default();

  loop {
    attempted = attempted.saturating_add(1);
    if root.kind == SyntaxKind::Document {
      let parsed = parse_document_with_ids(source.clone(), config, ids);
      work.fallback.add_stats(parsed.stats);
      return Ok(finish_full_fallback(
        old,
        parsed,
        changes.new_changed_range(),
        attempted,
        work,
      ));
    }
    let attempt =
      try_reparse_root(old, &source, &changes, root, config, ids);
    if let Some(result) = attempt.result {
      work.fragment.add_stats(attempt.fragment_stats);
      work.validation.add_stats(attempt.validation_stats);
      return Ok(finish_result(
        old,
        result,
        root,
        changes.new_changed_range(),
        attempted,
        work,
      ));
    }
    let mut rejected = ParserWork::default();
    rejected.add_stats(attempt.fragment_stats);
    rejected.add_stats(attempt.validation_stats);
    work.rejected.add_work(rejected);
    let Some(parent) = parent_supported_root(old, root) else {
      let parsed = parse_document_with_ids(source.clone(), config, ids);
      work.fallback.add_stats(parsed.stats);
      return Ok(finish_full_fallback(
        old,
        parsed,
        changes.new_changed_range(),
        attempted,
        work,
      ));
    };
    root = parent;
  }
}

struct RootResult {
  snapshot: SyntaxSnapshot,
}

struct RootAttempt {
  result: Option<RootResult>,
  fragment_stats: ParseStats,
  validation_stats: ParseStats,
}

impl RootAttempt {
  fn rejected(
    fragment_stats: ParseStats,
    validation_stats: ParseStats,
  ) -> Self {
    Self {
      result: None,
      fragment_stats,
      validation_stats,
    }
  }
}

fn try_reparse_root(
  old: &SyntaxSnapshot,
  source: &TextSnapshot,
  changes: &ChangeMap,
  root: ReparseRoot,
  config: ParseConfig,
  ids: &mut IdGenerator,
) -> RootAttempt {
  let mut fragment_stats = ParseStats::default();
  let mut validation_stats = ParseStats::default();
  if old.stats.diagnostics_truncated {
    return RootAttempt::rejected(fragment_stats, validation_stats);
  }

  let mapped = changes.map_range(root.range);
  if mapped.end.0 > source.byte_len().0 || !mapped.contains_range(changes.new_changed_range()) {
    return RootAttempt::rejected(fragment_stats, validation_stats);
  }
  let Some(kind) = fragment_kind(root.kind) else {
    return RootAttempt::rejected(fragment_stats, validation_stats);
  };
  let context = fragment_context(old, source, root, mapped, kind);
  let fragment = parse_fragment(source, mapped, kind, context, config, ids);
  fragment_stats = fragment.stats;
  if !fragment.matched
    || !fragment.consumed_complete
    || fragment.stats.diagnostics_truncated
    || fragment.root.kind != root.kind
    || fragment.root.text_len != mapped.len()
  {
    return RootAttempt::rejected(fragment_stats, validation_stats);
  }
  let replacement = fragment.root.clone();
  if root.kind == SyntaxKind::Section
    && root.range.len().0 > 0
    && replacement.text_len.0 == 0
  {
    return RootAttempt::rejected(fragment_stats, validation_stats);
  }
  if root.kind == SyntaxKind::Section {
    let Some(original) = find_node_by_id(&old.root, root.node) else {
      return RootAttempt::rejected(fragment_stats, validation_stats);
    };
    if has_direct_child(original, SyntaxKind::UlSubtitle)
      && !has_direct_child(&replacement, SyntaxKind::UlSubtitle)
    {
      return RootAttempt::rejected(fragment_stats, validation_stats);
    }
    // A non-final section is followed by an underline-style heading.
    // Boundary edits can otherwise shift that reused heading into the middle
    // of a physical line while leaving its old subtree structurally intact.
    if root.range.end < old.source.byte_len()
      && !Cursor::for_range(
        source,
        TextRange::new(mapped.end, source.byte_len()),
      )
      .is_line_start()
    {
      return RootAttempt::rejected(fragment_stats, validation_stats);
    }
  }
  if mapped.end < source.byte_len()
    && fragment
      .diagnostics
      .iter()
      .any(|diagnostic| diagnostic.code.as_str() == "syntax/unclosed-fence")
  {
    return RootAttempt::rejected(fragment_stats, validation_stats);
  }
  if replacement
    .flags
    .intersects(NodeFlags::MISSING | NodeFlags::CONTAINS_MISSING)
    && mapped.end < source.byte_len()
    && !tail_starts_with_ul_subtitle(source, mapped.end, config)
  {
    return RootAttempt::rejected(fragment_stats, validation_stats);
  }

  let Some((new_root, _)) =
    splice_node(&old.root, root.node, replacement, ids)
  else {
    return RootAttempt::rejected(fragment_stats, validation_stats);
  };
  let mut snapshot = SyntaxSnapshot::new(
    source.clone(),
    new_root,
    DiagnosticStore::new(source.revision()),
  );
  snapshot.diagnostics = merge_diagnostics(
    old,
    &fragment,
    &snapshot,
    changes,
    root.range,
  );
  if snapshot.diagnostics.len() > config.limits.max_diagnostics as usize {
    return RootAttempt::rejected(fragment_stats, validation_stats);
  }
  if root.kind != SyntaxKind::Section {
    let validation = validate_section_envelope(
      old, source, changes, root, &snapshot, config, ids,
    );
    validation_stats = validation.stats;
    if !validation.matched {
      return RootAttempt::rejected(fragment_stats, validation_stats);
    }
  }
  snapshot.restarts = build_restart_index(&snapshot);
  snapshot.stats = fragment.stats;
  RootAttempt {
    result: Some(RootResult { snapshot }),
    fragment_stats,
    validation_stats,
  }
}

fn fragment_kind(kind: SyntaxKind) -> Option<FragmentKind> {
  match kind {
    SyntaxKind::Document => Some(FragmentKind::Document),
    SyntaxKind::Section => Some(FragmentKind::Section),
    SyntaxKind::SectionElement => Some(FragmentKind::SectionElement),
    SyntaxKind::Paragraph => Some(FragmentKind::Paragraph),
    SyntaxKind::MechItem => Some(FragmentKind::MechItem),
    SyntaxKind::VariableDefine => Some(FragmentKind::VariableDefine),
    SyntaxKind::Expression | SyntaxKind::AdditiveExpression => Some(FragmentKind::Expression),
    SyntaxKind::ParentheticalExpression => Some(FragmentKind::ParentheticalTerm),
    SyntaxKind::GenericFence => Some(FragmentKind::CodeBlock),
    SyntaxKind::Grammar => Some(FragmentKind::Grammar),
    SyntaxKind::GrammarRule => Some(FragmentKind::GrammarRule),
    SyntaxKind::GrammarExpression => Some(FragmentKind::GrammarExpression),
    SyntaxKind::GrammarTerm => Some(FragmentKind::GrammarTerm),
    SyntaxKind::GrammarFactor => Some(FragmentKind::GrammarFactor),
    SyntaxKind::GrammarTerminalToken => Some(FragmentKind::GrammarTerminalToken),
    _ => None,
  }
}

fn fragment_context(
  old: &SyntaxSnapshot,
  source: &TextSnapshot,
  root: ReparseRoot,
  mapped: TextRange,
  kind: FragmentKind,
) -> ParseContext {
  let restart = old.restarts.get(root.node);
  let mode = restart
    .map(|entry| match entry.mode {
      crate::document::RestartMode::Document => ParseMode::Document,
      crate::document::RestartMode::Paragraph => ParseMode::Paragraph,
      crate::document::RestartMode::Mech => ParseMode::Mech,
      crate::document::RestartMode::Fence => ParseMode::Fence,
      crate::document::RestartMode::Grammar => ParseMode::Grammar,
    })
    .unwrap_or_else(|| kind.mode());
  let enclosing_fence = (kind == FragmentKind::CodeBlock)
    .then(|| {
      let cursor = Cursor::for_range(source, mapped);
      crate::document::parser::mechdown::fence_delimiter(&cursor)
        .map(|start| start.delimiter)
    })
    .flatten();
  ParseContext {
    mode,
    delimiter_depth: restart
      .map(|entry| entry.delimiter_depth.min(u32::from(u16::MAX)) as u16)
      .unwrap_or(0),
    line_start: restart
      .map(|entry| entry.line_start)
      .unwrap_or_else(|| Cursor::for_range(source, mapped).is_line_start()),
    indentation: restart.map(|entry| entry.indentation).unwrap_or(0),
    enclosing_fence,
  }
}

fn finish_result(
  old: &SyntaxSnapshot,
  mut result: RootResult,
  root: ReparseRoot,
  _changed: TextRange,
  attempted: u64,
  work: ReparseWork,
) -> ReparseResult {
  let old_ids = collect_node_ids(&old.root);
  let new_ids = collect_node_ids(&result.snapshot.root);
  let reused = old_ids
    .intersection(&new_ids)
    .copied()
    .collect::<Vec<_>>();
  let new_count = new_ids.difference(&old_ids).count() as u64;
  let diagnostics = diagnostic_delta(old, &result.snapshot);
  let total = work.total();
  let stats = ReparseStats {
    source_bytes: u64::from(result.snapshot.source.byte_len().0),
    parser_steps: total.parser_steps,
    events_emitted: total.events_emitted,
    fragment_parser_steps: work.fragment.parser_steps,
    fragment_events_emitted: work.fragment.events_emitted,
    validation_parser_steps: work.validation.parser_steps,
    validation_events_emitted: work.validation.events_emitted,
    rejected_parser_steps: work.rejected.parser_steps,
    rejected_events_emitted: work.rejected.events_emitted,
    fallback_parser_steps: work.fallback.parser_steps,
    fallback_events_emitted: work.fallback.events_emitted,
    total_parser_steps: total.parser_steps,
    total_events_emitted: total.events_emitted,
    diagnostics_emitted: result.snapshot.diagnostics.len() as u64,
    diagnostics_truncated: result.snapshot.stats.diagnostics_truncated,
    recovery_bytes: total.recovery_bytes,
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
  work: ReparseWork,
) -> ReparseResult {
  let root = ReparseRoot {
    node: old.root.id,
    kind: SyntaxKind::Document,
    range: old.source.full_range(),
  };
  let result = RootResult { snapshot };
  let mut finished =
    finish_result(old, result, root, changed, attempted, work);
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
  _config: ParseConfig,
) -> bool {
  let tail = TextRange::new(start, source.byte_len());
  let cursor = Cursor::for_range(source, tail);
  crate::document::parser::mechdown::is_ul_subtitle(&cursor)
}

struct EnvelopeValidation {
  matched: bool,
  stats: ParseStats,
}

fn validate_section_envelope(
  old: &SyntaxSnapshot,
  source: &TextSnapshot,
  changes: &ChangeMap,
  root: ReparseRoot,
  candidate: &SyntaxSnapshot,
  config: ParseConfig,
  ids: &mut IdGenerator,
) -> EnvelopeValidation {
  let Some(section) =
    ancestor_record(old, root.node, SyntaxKind::Section)
  else {
    return EnvelopeValidation {
      matched: true,
      stats: ParseStats::default(),
    };
  };
  let mapped = changes.map_range(section.range);
  if mapped.end > source.byte_len() {
    return EnvelopeValidation {
      matched: false,
      stats: ParseStats::default(),
    };
  }
  let Some(actual) =
    find_node_at_range(&candidate.root, SyntaxKind::Section, mapped, TextSize::ZERO)
  else {
    return EnvelopeValidation {
      matched: false,
      stats: ParseStats::default(),
    };
  };
  let context = ParseContext {
    mode: ParseMode::Document,
    delimiter_depth: 0,
    line_start: Cursor::for_range(source, mapped).is_line_start(),
    indentation: 0,
    enclosing_fence: None,
  };
  let expected = parse_fragment(
    source,
    mapped,
    FragmentKind::Section,
    context,
    config,
    ids,
  );
  let stats = expected.stats;
  let tree_matches = expected.matched
    && expected.consumed_complete
    && !expected.stats.diagnostics_truncated
    && actual.structural_hash == expected.root.structural_hash
    && actual.flags == expected.root.flags;
  let diagnostics_match = normalize_diagnostics_in_range(
    &candidate.diagnostics,
    candidate.revision,
    &candidate.nodes,
    mapped,
  ) == normalize_diagnostics_in_range(
    &expected.diagnostics,
    expected.source.revision(),
    &expected.nodes,
    mapped,
  );
  EnvelopeValidation {
    matched: tree_matches && diagnostics_match,
    stats,
  }
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
  fragment: &FragmentSnapshot,
  new_snapshot: &SyntaxSnapshot,
  changes: &ChangeMap,
  replaced_old_range: TextRange,
) -> DiagnosticStore {
  let mut diagnostics = Vec::new();
  for diagnostic in old.diagnostics.iter().cloned() {
    if absolute_primary_belongs_to_replaced(
      &diagnostic,
      old,
      replaced_old_range,
    ) {
      continue;
    }
    let diagnostic =
      map_old_diagnostic(diagnostic, changes, new_snapshot.revision);
    if diagnostic
      .primary
      .resolve(new_snapshot.revision, &new_snapshot.nodes)
      .is_some()
    {
      diagnostics.push(diagnostic);
    }
  }
  for diagnostic in fragment.diagnostics.iter().cloned() {
    if diagnostic
      .primary
      .resolve(new_snapshot.revision, &new_snapshot.nodes)
      .is_some()
    {
      diagnostics.push(diagnostic);
    }
  }
  let retained_ids = diagnostics
    .iter()
    .map(|diagnostic| diagnostic.id)
    .collect::<BTreeSet<_>>();
  for diagnostic in &mut diagnostics {
    diagnostic
      .related
      .retain(|related| retained_ids.contains(related));
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

fn absolute_primary_belongs_to_replaced(
  diagnostic: &Diagnostic,
  old: &SyntaxSnapshot,
  replaced: TextRange,
) -> bool {
  let DiagnosticAnchor::Absolute { revision, range } = diagnostic.primary else {
    return false;
  };
  if revision != old.revision {
    return true;
  }
  ranges_touch(range, replaced)
}

fn ranges_touch(left: TextRange, right: TextRange) -> bool {
  if left.is_empty() {
    return right.contains_inclusive(left.start);
  }
  if right.is_empty() {
    return left.contains_inclusive(right.start);
  }
  left.start < right.end && right.start < left.end
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

#[cfg(test)]
mod tests {
  use alloc::string::String;

  use crate::document::{
    DiagnosticCode, DiagnosticPhase, DiagnosticTags, DocumentId, Revision, Severity,
  };

  use super::*;

  fn absolute_diagnostic(
    ids: &mut IdGenerator,
    revision: Revision,
    range: TextRange,
    name: &str,
  ) -> Diagnostic {
    Diagnostic {
      id: ids.diagnostic(),
      code: DiagnosticCode::syntax(name),
      phase: DiagnosticPhase::SyntaxValidation,
      severity: Severity::Warning,
      rule: None,
      context: None,
      primary: DiagnosticAnchor::Absolute { revision, range },
      labels: Vec::new(),
      expected: Vec::new(),
      found: None,
      fixes: Vec::new(),
      related: Vec::new(),
      recovery: None,
      tags: DiagnosticTags::NONE,
      message: String::from(name),
    }
  }

  #[test]
  fn unaffected_absolute_diagnostics_are_remapped_and_replaced_ones_are_dropped() {
    let text = "x := 1\n1. Later\n--------\nstable\n";
    let source = TextSnapshot::new(DocumentId(4), Revision(0), text).unwrap();
    let mut ids = IdGenerator::new();
    let mut old = parse_document_with_ids(source, ParseConfig::default(), &mut ids);
    let replaced = absolute_diagnostic(
      &mut ids,
      Revision(0),
      TextRange::new(TextSize(5), TextSize(6)),
      "inside-replaced-root",
    );
    let later_start = text.find("stable").unwrap() as u32;
    let retained = absolute_diagnostic(
      &mut ids,
      Revision(0),
      TextRange::new(TextSize(later_start), TextSize(later_start + 6)),
      "outside-replaced-root",
    );
    let retained_id = retained.id;
    old.diagnostics.push(replaced);
    old.diagnostics.push(retained);

    let result = reparse(
      &old,
      &[TextEdit::replace(
        TextRange::new(TextSize(5), TextSize(6)),
        "123",
      )],
      ParseConfig::default(),
      &mut ids,
    )
    .unwrap();

    assert!(
      result
        .snapshot
        .diagnostics
        .iter()
        .all(|diagnostic| diagnostic.code.as_str() != "syntax/inside-replaced-root")
    );
    let retained = result
      .snapshot
      .diagnostics
      .iter()
      .find(|diagnostic| diagnostic.id == retained_id)
      .expect("unaffected absolute diagnostic must be retained");
    assert_eq!(
      retained
        .primary
        .resolve(result.snapshot.revision, &result.snapshot.nodes),
      Some(TextRange::new(
        TextSize(later_start + 2),
        TextSize(later_start + 8),
      ))
    );
  }
}
