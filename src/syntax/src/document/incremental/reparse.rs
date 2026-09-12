use alloc::collections::{BTreeMap, BTreeSet, VecDeque};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::document::parser::{build_restart_index, parse_canonical_document_with_ids};
use crate::document::{
    Diagnostic, DiagnosticAnchor, DiagnosticId, DiagnosticStore, ExpectedSyntax, GreenElement,
    GreenNode, GreenToken, IdGenerator, NodeId, ParseConfig, RecoveryAction, Revision, SourceError,
    SyntaxElementId, SyntaxSnapshot, TextEdit, TextRange, TextSize,
};

use super::change_map::{Affinity, ChangeMap};
use super::restart::select_reparse_root;
use super::{DiagnosticDelta, ReparseStats};

pub(crate) struct ReparseResult {
    pub snapshot: SyntaxSnapshot,
    pub reparsed_roots: Vec<NodeId>,
    pub reused_roots: Vec<NodeId>,
    pub diagnostics: DiagnosticDelta,
    pub stats: ReparseStats,
}

/// Reuse is a post-parse identity operation: only the fresh canonical parse
/// decides syntax and diagnostics. No fragment or prototype parser participates.
pub(crate) fn reparse(
    old: &SyntaxSnapshot,
    edits: &[TextEdit],
    config: ParseConfig,
    ids: &mut IdGenerator,
) -> Result<ReparseResult, SourceError> {
    let source = old.source.apply_edits(edits)?;
    let changes = ChangeMap::new(edits);
    let root = select_reparse_root(old, &changes);
    let parsed = parse_canonical_document_with_ids(source, config, ids);
    let parse_stats = parsed.stats;
    let elements = [
        old.nodes.node_count(),
        old.nodes.token_count(),
        parsed.nodes.node_count(),
        parsed.nodes.token_count(),
    ]
    .into_iter()
    .fold(0_u64, |sum, count| sum.saturating_add(count as u64));
    let old_diagnostic_work = old
        .diagnostics
        .iter()
        .map(diagnostic_work)
        .fold(0_u64, u64::saturating_add);
    let fresh_diagnostic_work = parsed
        .diagnostics
        .iter()
        .map(diagnostic_work)
        .fold(0_u64, u64::saturating_add);
    let mut budget = ReconciliationBudget {
        // Reserve the mandatory snapshot-index, delta and fresh-diagnostic
        // passes before spending the remaining allowance on optional reuse.
        steps: elements
            .saturating_mul(2)
            .saturating_add(fresh_diagnostic_work.saturating_mul(3)),
        limit: elements
            .saturating_mul(8)
            .saturating_add(
                old_diagnostic_work
                    .saturating_add(fresh_diagnostic_work)
                    .saturating_mul(16),
            )
            .saturating_add(32),
    };
    let mut reuse = ReuseIndex::default();
    reuse.index_old(&old.root, TextSize::ZERO, &changes, &mut budget);
    let green = reuse.reconcile(&parsed.root, TextSize::ZERO, &mut budget);
    let mut snapshot =
        SyntaxSnapshot::new(parsed.source, green, DiagnosticStore::new(parsed.revision));
    snapshot.diagnostics = reconcile_diagnostics(
        old,
        &parsed.diagnostics,
        &snapshot,
        &changes,
        &reuse.identities,
        &mut budget,
    );
    snapshot.restarts = build_restart_index(&snapshot);
    snapshot.stats = parse_stats;
    let reused_roots = snapshot
        .nodes
        .nodes()
        .filter_map(|(id, _)| old.nodes.node(id).map(|_| id))
        .collect::<Vec<_>>();
    let new_count = snapshot.nodes.node_count() as u64 - reused_roots.len() as u64;
    let diagnostics = diagnostic_delta(old, &snapshot);
    snapshot.stats.reparse_root_count = 1;
    snapshot.stats.reused_node_count = reused_roots.len() as u64;
    snapshot.stats.new_node_count = new_count;
    let stats = ReparseStats {
        source_bytes: u64::from(snapshot.source.byte_len().0),
        parser_steps: parse_stats.parser_steps,
        events_emitted: parse_stats.events_emitted,
        fallback_parser_steps: parse_stats.parser_steps,
        fallback_events_emitted: parse_stats.events_emitted,
        total_parser_steps: parse_stats.parser_steps,
        total_events_emitted: parse_stats.events_emitted,
        diagnostics_emitted: snapshot.diagnostics.len() as u64,
        diagnostics_truncated: parse_stats.diagnostics_truncated,
        recovery_bytes: parse_stats.recovery_bytes,
        reconciliation_steps: budget.steps,
        reconciliation_limit: budget.limit,
        reparse_root_count: 1,
        reused_node_count: reused_roots.len() as u64,
        new_node_count: new_count,
        attempted_roots: 1,
        document_fallbacks: 1,
        ..ReparseStats::default()
    };
    Ok(ReparseResult {
        snapshot,
        reparsed_roots: alloc::vec![root.node],
        reused_roots,
        diagnostics,
        stats,
    })
}

struct ReconciliationBudget {
    steps: u64,
    limit: u64,
}
impl ReconciliationBudget {
    fn charge(&mut self, count: u64) -> bool {
        if count > self.limit.saturating_sub(self.steps) {
            return false;
        }
        self.steps += count;
        true
    }
}

type ElementKey = (u32, u32, u16, u16, u64);
fn node_key(node: &GreenNode, range: TextRange) -> ElementKey {
    (
        range.start.0,
        range.end.0,
        node.kind as u16,
        node.flags.0,
        node.structural_hash,
    )
}
fn token_key(token: &GreenToken, range: TextRange) -> ElementKey {
    (
        range.start.0,
        range.end.0,
        token.kind as u16,
        token.flags.0,
        token.text_hash,
    )
}

#[derive(Default)]
struct ReuseIndex {
    nodes: BTreeMap<ElementKey, VecDeque<Arc<GreenNode>>>,
    tokens: BTreeMap<ElementKey, VecDeque<GreenToken>>,
    identities: BTreeMap<SyntaxElementId, SyntaxElementId>,
    unavailable: BTreeSet<SyntaxElementId>,
}
impl ReuseIndex {
    fn index_old(
        &mut self,
        node: &Arc<GreenNode>,
        start: TextSize,
        changes: &ChangeMap,
        budget: &mut ReconciliationBudget,
    ) {
        if !budget.charge(1) {
            return;
        }
        if let Some(range) = changes.map_unchanged_range(TextRange::at(start, node.text_len)) {
            self.nodes
                .entry(node_key(node, range))
                .or_default()
                .push_back(Arc::clone(node));
        } else if node.text_len == TextSize::ZERO {
            self.unavailable.insert(SyntaxElementId::Node(node.id));
        }
        let mut offset = start;
        for child in node.children.iter() {
            match child {
                GreenElement::Node(node) => self.index_old(node, offset, changes, budget),
                GreenElement::Token(token) => {
                    if !budget.charge(1) {
                        return;
                    }
                    if let Some(range) =
                        changes.map_unchanged_range(TextRange::at(offset, token.text_len))
                    {
                        self.tokens
                            .entry(token_key(token, range))
                            .or_default()
                            .push_back(*token);
                    } else if token.text_len == TextSize::ZERO {
                        // A parent span may be unchanged while an edit touches
                        // its empty boundary child. Whole-tree reuse must not
                        // bypass that child's independent boundary exclusion.
                        self.unavailable.insert(SyntaxElementId::Token(token.id));
                    }
                }
            }
            offset += child.text_len();
        }
    }

    fn reconcile(
        &mut self,
        node: &Arc<GreenNode>,
        start: TextSize,
        budget: &mut ReconciliationBudget,
    ) -> Arc<GreenNode> {
        if !budget.charge(1) {
            return Arc::clone(node);
        }
        let key = node_key(node, TextRange::at(start, node.text_len));
        while let Some(old) = self.nodes.get_mut(&key).and_then(VecDeque::pop_front) {
            // Hashes only choose candidates. Compare every structural field;
            // source bytes are already proven unchanged by the ChangeMap.
            if let Some(count) = equal_tree(&old, node, &self.unavailable, budget)
                && budget.charge(count)
            {
                self.reuse_tree_ids(&old, node);
                return old;
            }
        }
        let mut children = Vec::with_capacity(node.children.len());
        let mut offset = start;
        for child in node.children.iter() {
            children.push(match child {
                GreenElement::Node(child) => {
                    GreenElement::Node(self.reconcile(child, offset, budget))
                }
                GreenElement::Token(token) => {
                    let mut selected = *token;
                    if budget.charge(1) {
                        let key = token_key(token, TextRange::at(offset, token.text_len));
                        while let Some(old) =
                            self.tokens.get_mut(&key).and_then(VecDeque::pop_front)
                        {
                            if !budget.charge(1) {
                                break;
                            }
                            if self.unavailable.insert(SyntaxElementId::Token(old.id)) {
                                self.identities.insert(
                                    SyntaxElementId::Token(token.id),
                                    SyntaxElementId::Token(old.id),
                                );
                                selected = old;
                                break;
                            }
                        }
                    }
                    GreenElement::Token(selected)
                }
            });
            offset += child.text_len();
        }
        Arc::new(GreenNode {
            children: children.into(),
            ..node.as_ref().clone()
        })
    }

    fn reuse_tree_ids(&mut self, old: &GreenNode, fresh: &GreenNode) {
        self.unavailable.insert(SyntaxElementId::Node(old.id));
        self.identities.insert(
            SyntaxElementId::Node(fresh.id),
            SyntaxElementId::Node(old.id),
        );
        for (old, fresh) in old.children.iter().zip(fresh.children.iter()) {
            match (old, fresh) {
                (GreenElement::Node(old), GreenElement::Node(fresh)) => {
                    self.reuse_tree_ids(old, fresh)
                }
                (GreenElement::Token(old), GreenElement::Token(fresh)) => {
                    self.unavailable.insert(SyntaxElementId::Token(old.id));
                    self.identities.insert(
                        SyntaxElementId::Token(fresh.id),
                        SyntaxElementId::Token(old.id),
                    );
                }
                _ => unreachable!("structural equality checked before reuse"),
            }
        }
    }
}

fn equal_tree(
    old: &GreenNode,
    fresh: &GreenNode,
    unavailable: &BTreeSet<SyntaxElementId>,
    budget: &mut ReconciliationBudget,
) -> Option<u64> {
    if !budget.charge(1)
        || unavailable.contains(&SyntaxElementId::Node(old.id))
        || old.kind != fresh.kind
        || old.flags != fresh.flags
        || old.text_len != fresh.text_len
        || old.children.len() != fresh.children.len()
    {
        return None;
    }
    let mut count = 1;
    for (old, fresh) in old.children.iter().zip(fresh.children.iter()) {
        count += match (old, fresh) {
            (GreenElement::Node(old), GreenElement::Node(fresh)) => {
                equal_tree(old, fresh, unavailable, budget)?
            }
            (GreenElement::Token(old), GreenElement::Token(fresh)) => {
                if !budget.charge(1)
                    || unavailable.contains(&SyntaxElementId::Token(old.id))
                    || old.kind != fresh.kind
                    || old.flags != fresh.flags
                    || old.text_len != fresh.text_len
                    || old.text_hash != fresh.text_hash
                {
                    return None;
                }
                1
            }
            _ => return None,
        };
    }
    Some(count)
}

/// Count variable-length metadata as well as entries. Candidate comparisons
/// must not hide repeated scans of long diagnostic messages or fixes.
fn diagnostic_work(diagnostic: &Diagnostic) -> u64 {
    let mut work = 1_u64;
    let mut add = |amount: usize| {
        work = work.saturating_add(amount as u64);
    };
    let syntax_work = |syntax: &ExpectedSyntax| match syntax {
        ExpectedSyntax::Token(_) => 1,
        ExpectedSyntax::Production(name) => 1 + name.len(),
    };
    add(diagnostic.code.0.len());
    add(diagnostic.message.len());
    for label in &diagnostic.labels {
        add(1 + label.message.len());
    }
    for syntax in &diagnostic.expected {
        add(syntax_work(syntax));
    }
    if let Some(found) = &diagnostic.found {
        add(1 + found.text.as_ref().map_or(0, String::len));
    }
    for fix in &diagnostic.fixes {
        add(1 + fix.title.len());
        for edit in &fix.edits {
            add(1 + edit.insert.len());
        }
    }
    add(diagnostic.related.len());
    if let Some(RecoveryAction::Insert { syntax, .. }) = &diagnostic.recovery {
        add(syntax_work(syntax));
    }
    work
}

fn remap_anchor(
    anchor: &mut DiagnosticAnchor,
    identities: &BTreeMap<SyntaxElementId, SyntaxElementId>,
) {
    if let DiagnosticAnchor::Element { element, .. } = anchor
        && let Some(reused) = identities.get(element)
    {
        *element = *reused;
    }
}

fn reconcile_diagnostics(
    old: &SyntaxSnapshot,
    fresh: &DiagnosticStore,
    snapshot: &SyntaxSnapshot,
    changes: &ChangeMap,
    identities: &BTreeMap<SyntaxElementId, SyntaxElementId>,
    budget: &mut ReconciliationBudget,
) -> DiagnosticStore {
    let mut candidates = BTreeMap::<(u32, u32, String), VecDeque<Diagnostic>>::new();
    for diagnostic in old.diagnostics.iter() {
        if !budget.charge(diagnostic_work(diagnostic)) {
            break;
        }
        let Some(range) = diagnostic.primary.resolve(old.revision, &old.nodes) else {
            continue;
        };
        if changes.map_unchanged_range(range).is_none() {
            continue;
        }
        let mapped = map_old_diagnostic(diagnostic.clone(), changes, snapshot.revision);
        let Some(range) = mapped.primary.resolve(snapshot.revision, &snapshot.nodes) else {
            continue;
        };
        candidates
            .entry((range.start.0, range.end.0, mapped.code.0.clone()))
            .or_default()
            .push_back(mapped);
    }
    let mut result = fresh.iter().cloned().collect::<Vec<_>>();
    let fresh_ids = result.iter().map(|d| d.id).collect::<Vec<_>>();
    let mut retained = BTreeMap::<DiagnosticId, DiagnosticId>::new();
    let mut original = Vec::with_capacity(result.len());
    for diagnostic in &mut result {
        remap_anchor(&mut diagnostic.primary, identities);
        for label in &mut diagnostic.labels {
            remap_anchor(&mut label.anchor, identities);
        }
        let mut matched = None;
        if let Some(range) = diagnostic
            .primary
            .resolve(snapshot.revision, &snapshot.nodes)
        {
            let key = (range.start.0, range.end.0, diagnostic.code.0.clone());
            if let Some(candidates) = candidates.get_mut(&key) {
                for index in 0..candidates.len() {
                    let candidate = &candidates[index];
                    if !budget.charge(
                        1 + diagnostic_work(candidate)
                            .saturating_mul(2)
                            .saturating_add(diagnostic_work(diagnostic)),
                    ) {
                        break;
                    }
                    let mut comparison = candidate.clone();
                    comparison.id = diagnostic.id;
                    comparison.related = diagnostic.related.clone();
                    if comparison == *diagnostic {
                        // Removing a deque entry moves the shorter adjacent
                        // side. Account for that movement before accepting it.
                        if !budget.charge(1 + index.min(candidates.len() - index - 1) as u64) {
                            break;
                        }
                        let candidate = candidates.remove(index).unwrap();
                        retained.insert(diagnostic.id, candidate.id);
                        matched = Some(candidate);
                        break;
                    }
                    // A mismatch may belong to a later fresh finding with the
                    // same range and code. Keep it available for that finding.
                }
            }
        }
        original.push(matched);
    }
    // A retained diagnostic cannot keep its identity while changing its related
    // edges. Remove mismatches and their dependants in one bounded worklist.
    let mut dependants = BTreeMap::<DiagnosticId, Vec<usize>>::new();
    for (index, diagnostic) in result.iter().enumerate() {
        for related in &diagnostic.related {
            dependants.entry(*related).or_default().push(index);
        }
    }
    let mut pending = VecDeque::new();
    for (index, candidate) in original.iter().enumerate() {
        let Some(candidate) = candidate else {
            continue;
        };
        if !budget.charge(1 + result[index].related.len() as u64) {
            retained.clear();
            break;
        }
        if !result[index]
            .related
            .iter()
            .map(|id| retained.get(id).copied().unwrap_or(*id))
            .eq(candidate.related.iter().copied())
        {
            pending.push_back(index);
        }
    }
    while let Some(index) = pending.pop_front() {
        if retained.remove(&fresh_ids[index]).is_none() {
            continue;
        }
        // Each diagnostic is removed once, and each incoming edge is visited
        // once. Losing an old related identity invalidates its dependants too.
        if let Some(dependants) = dependants.get(&fresh_ids[index]) {
            if !budget.charge(1 + dependants.len() as u64) {
                retained.clear();
                break;
            }
            pending.extend(dependants.iter().copied());
        }
    }
    let mut store = DiagnosticStore::new(snapshot.revision);
    for mut diagnostic in result {
        diagnostic.id = retained
            .get(&diagnostic.id)
            .copied()
            .unwrap_or(diagnostic.id);
        for related in &mut diagnostic.related {
            *related = retained.get(related).copied().unwrap_or(*related);
        }
        store.push(diagnostic);
    }
    store
}

fn map_old_diagnostic(
    mut diagnostic: Diagnostic,
    changes: &ChangeMap,
    revision: Revision,
) -> Diagnostic {
    let map = |anchor: &mut DiagnosticAnchor| {
        if let DiagnosticAnchor::Absolute {
            revision: rev,
            range,
        } = anchor
        {
            *rev = revision;
            *range = changes.map_range(*range);
        }
    };
    map(&mut diagnostic.primary);
    for label in &mut diagnostic.labels {
        map(&mut label.anchor);
    }
    for fix in &mut diagnostic.fixes {
        for edit in &mut fix.edits {
            edit.delete = changes.map_range(edit.delete);
        }
    }
    if let Some(recovery) = &mut diagnostic.recovery {
        match recovery {
            RecoveryAction::Insert { at, .. } | RecoveryAction::Abandon { at, .. } => {
                *at = changes.map_offset(*at, Affinity::After)
            }
            RecoveryAction::Skip { range } | RecoveryAction::ResourceLimit { range } => {
                *range = changes.map_range(*range)
            }
        }
    }
    diagnostic
}

fn diagnostic_delta(old: &SyntaxSnapshot, new: &SyntaxSnapshot) -> DiagnosticDelta {
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
    use super::*;
    use crate::document::{GreenBuilder, SyntaxKind};

    fn tree(ids: &mut IdGenerator, token_kind: SyntaxKind, missing: bool) -> Arc<GreenNode> {
        let mut builder = GreenBuilder::new(ids);
        builder.start_node(SyntaxKind::Paragraph);
        builder.token(token_kind, "x").unwrap();
        if missing {
            builder.missing_token(SyntaxKind::RightParen).unwrap();
        }
        builder.finish_node().unwrap();
        builder.finish().unwrap()
    }

    #[test]
    fn a_structural_hash_collision_cannot_reuse_a_different_tree() {
        let mut ids = IdGenerator::new();
        let old = tree(&mut ids, SyntaxKind::IdentifierToken, false);
        let fresh = tree(&mut ids, SyntaxKind::Text, false);
        let mut fresh = fresh.as_ref().clone();
        fresh.structural_hash = old.structural_hash;
        let fresh = Arc::new(fresh);
        let mut index = ReuseIndex::default();
        let mut budget = ReconciliationBudget {
            steps: 0,
            limit: 100,
        };
        index.index_old(&old, TextSize::ZERO, &ChangeMap::new(&[]), &mut budget);
        let result = index.reconcile(&fresh, TextSize::ZERO, &mut budget);
        assert_eq!(result.id, fresh.id);
        assert!(!Arc::ptr_eq(&result, &old));
        assert!(index.identities.is_empty());
    }

    #[test]
    fn budget_exhaustion_keeps_the_fresh_canonical_tree() {
        let mut ids = IdGenerator::new();
        let old = tree(&mut ids, SyntaxKind::IdentifierToken, false);
        let fresh = tree(&mut ids, SyntaxKind::IdentifierToken, false);
        let mut index = ReuseIndex::default();
        index.index_old(
            &old,
            TextSize::ZERO,
            &ChangeMap::new(&[]),
            &mut ReconciliationBudget {
                steps: 0,
                limit: 100,
            },
        );
        let mut budget = ReconciliationBudget { steps: 0, limit: 0 };
        let result = index.reconcile(&fresh, TextSize::ZERO, &mut budget);
        assert!(Arc::ptr_eq(&result, &fresh));
        assert!(index.identities.is_empty());
        assert_eq!(budget.steps, 0);
    }

    #[test]
    fn parent_reuse_cannot_retain_a_missing_token_at_an_edited_boundary() {
        let mut ids = IdGenerator::new();
        let old = tree(&mut ids, SyntaxKind::IdentifierToken, true);
        let fresh = tree(&mut ids, SyntaxKind::IdentifierToken, true);
        let mut index = ReuseIndex::default();
        let mut budget = ReconciliationBudget {
            steps: 0,
            limit: 100,
        };
        index.index_old(
            &old,
            TextSize::ZERO,
            &ChangeMap::new(&[TextEdit::insert(TextSize(1), "after")]),
            &mut budget,
        );
        let result = index.reconcile(&fresh, TextSize::ZERO, &mut budget);
        assert_eq!(result.id, fresh.id);
        let GreenElement::Token(missing) = result.children[1] else {
            panic!("missing token");
        };
        let GreenElement::Token(fresh_missing) = fresh.children[1] else {
            panic!("fresh missing token");
        };
        assert_eq!(missing.id, fresh_missing.id);
        assert!(!Arc::ptr_eq(&result, &old));
    }

    #[test]
    fn mapped_absolute_diagnostics_preserve_complete_findings_and_related_identity() {
        use crate::document::{
            DiagnosticFix, DiagnosticLabel, DiagnosticPhase, DiagnosticTags, DocumentId,
            FixApplicability, Revision, Severity, TextSnapshot,
        };
        let config = ParseConfig::default();
        let old_source = TextSnapshot::new(DocumentId(1), Revision(0), "prefix\nx +\n").unwrap();
        let mut old = crate::document::parse_canonical_document(old_source, config);
        let edits = [TextEdit::insert(TextSize(0), "💡 ")];
        let fresh = crate::document::parse_canonical_document(
            old.source.apply_edits(&edits).unwrap(),
            config,
        );
        let finding =
            |id: u64, revision: Revision, shift: u32, name: &str, related: Vec<DiagnosticId>| {
                let anchor = DiagnosticAnchor::Absolute {
                    revision,
                    range: TextRange::new(TextSize(7 + shift), TextSize(8 + shift)),
                };
                Diagnostic {
                    id: DiagnosticId(id),
                    code: "syntax/independent-owner-test".into(),
                    phase: DiagnosticPhase::Syntax,
                    severity: Severity::Error,
                    rule: None,
                    context: None,
                    primary: anchor.clone(),
                    labels: alloc::vec![DiagnosticLabel {
                        anchor,
                        message: "label".into()
                    }],
                    expected: alloc::vec![ExpectedSyntax::Production("expression".into())],
                    found: None,
                    fixes: alloc::vec![DiagnosticFix {
                        title: "complete operand".into(),
                        applicability: FixApplicability::MaybeIncorrect,
                        edits: alloc::vec![TextEdit::replace(
                            TextRange::new(TextSize(9 + shift), TextSize(10 + shift)),
                            "1"
                        )],
                    }],
                    related,
                    recovery: Some(RecoveryAction::Skip {
                        range: TextRange::new(TextSize(9 + shift), TextSize(10 + shift)),
                    }),
                    tags: DiagnosticTags::NONE,
                    message: name.into(),
                }
            };
        old.diagnostics = DiagnosticStore::new(old.revision);
        let mut fresh_diagnostics = DiagnosticStore::new(fresh.revision);
        for (index, name) in ["first", "second", "unrelated"].into_iter().enumerate() {
            let old_related = if index < 2 {
                alloc::vec![DiagnosticId(11 - index as u64)]
            } else {
                Vec::new()
            };
            let fresh_related = if index < 2 {
                alloc::vec![DiagnosticId(21 - index as u64)]
            } else {
                Vec::new()
            };
            old.diagnostics.push(finding(
                10 + index as u64,
                old.revision,
                0,
                name,
                old_related,
            ));
            fresh_diagnostics.push(finding(
                20 + index as u64,
                fresh.revision,
                5,
                name,
                fresh_related,
            ));
        }
        let reconcile = |store: &DiagnosticStore| {
            reconcile_diagnostics(
                &old,
                store,
                &fresh,
                &ChangeMap::new(&edits),
                &BTreeMap::new(),
                &mut ReconciliationBudget {
                    steps: 0,
                    limit: 10_000,
                },
            )
        };
        let retained = reconcile(&fresh_diagnostics);
        assert_eq!(
            retained.iter().map(|d| d.id.0).collect::<Vec<_>>(),
            [10, 11, 12]
        );
        assert_eq!(retained.iter().next().unwrap().related, [DiagnosticId(11)]);
        assert_eq!(
            normalize(&retained, &fresh),
            normalize(&fresh_diagnostics, &fresh)
        );
        let mut changed = fresh_diagnostics.iter().cloned().collect::<Vec<_>>();
        changed[1].message = "changed semantic finding".into();
        let mut store = DiagnosticStore::new(fresh.revision);
        for diagnostic in changed {
            store.push(diagnostic);
        }
        let result = reconcile(&store);
        assert_eq!(
            result.iter().map(|d| d.id.0).collect::<Vec<_>>(),
            [20, 21, 12]
        );
        assert_eq!(normalize(&result, &fresh), normalize(&store, &fresh));
    }

    fn normalize(
        store: &DiagnosticStore,
        snapshot: &SyntaxSnapshot,
    ) -> Vec<crate::document::NormalizedDiagnostic> {
        crate::document::normalize_diagnostics(store, snapshot.revision, &snapshot.nodes)
    }
}
