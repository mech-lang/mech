//! Scoped recipes for selected structural alternatives.
use super::{Parser, RuleId};
use crate::document::parser::canonical::prediction::MIN_ANALYSIS_ALLOWANCE;
use crate::document::parser::{
    DefinitionRawOrigin, Event, ParserCheckpoint,
    cursor::CursorCheckpoint,
    grapheme_scan::GraphemeScan,
    tree_cache::{CacheWork, CachedNode},
};
use crate::document::{
    GreenChildren, GreenElement, GreenNode, GreenToken, IdGenerator, NodeId, ParseStats,
    SyntaxKind, TextRange, TextSize, TokenId,
};
use alloc::{boxed::Box, sync::Arc, vec, vec::Vec};

const APPLY: bool = true;
const MAX_EVENTS: usize = 24_000;
const MAX_BYTES: u32 = 8_192;
const MAX_IDS: u64 = 65_536;

#[derive(Clone, Copy)]
pub(super) struct Position {
    pub poll: u64,
    pub steps: u64,
    pub child_work: u64,
    pub allowance: u64,
    pub input_final: bool,
    pub delimiter: Option<RuleId>,
}

struct Origin {
    checkpoint: ParserCheckpoint,
    position: Position,
    kind: SyntaxKind,
    parent_raw: DefinitionRawOrigin,
    recovery_allowed: bool,
    stats: ParseStats,
    ids: IdGenerator,
    fuel: u64,
    rewinds: u64,
    rewound_bytes: u64,
    cache_work: CacheWork,
}

pub(super) struct Recording(Phase);

enum Phase {
    Trial(Origin),
    Selected(Summary),
}

struct Summary {
    origin: Origin,
    end: ParserCheckpoint,
    steps: u64,
    child_work: u64,
    charges: u64,
    rewinds: u64,
    rewound_bytes: u64,
    cache_work: CacheWork,
    nodes: u64,
    tokens: u64,
    raw: RawTransfer,
    events: Vec<Event>,
}

enum RawTransfer {
    Unchanged,
    Fresh {
        at: TextSize,
        final_end: Option<TextSize>,
        tail: Option<GraphemeScan>,
    },
}

fn sealed(parser: &Parser<'_>, at: Position) -> bool {
    let state = &parser.state;
    let end = parser.source().byte_len();
    at.input_final
        && at.poll != u64::MAX
        && !parser.is_halted()
        && !state.resource_finalizing
        && state.resource_found.is_none()
        && state.tree_cache.one_shot
        && !state.tree_cache.pending()
        && state.diagnostics.is_empty()
        && !state.stats.diagnostics_truncated
        && parser.cursor().end() == end
        && parser.cursor().context_end() == end
        && parser.source().contiguous_without_lookup_work().is_some()
}

// Both polls must give all of the replay's original work enough headroom
// for the coarse stale-cache bound and the terminal prediction threshold.
fn remaining_headroom(parser: &Parser<'_>) -> Option<u64> {
    let max_events = u64::from(parser.config().limits.max_events);
    let max_token_work = u64::from(parser.source().byte_len().0) + 2;
    max_events
        .checked_mul(max_token_work)?
        .checked_add(MIN_ANALYSIS_ALLOWANCE)
}

impl Recording {
    pub(super) fn begin(
        parser: &mut Parser<'_>,
        position: Position,
        kind: SyntaxKind,
    ) -> Option<Box<Self>> {
        // A nested trial may borrow the observer after its parent has
        // recorded the first raw access. Restore that witness on completion.
        if matches!(parser.state.definition_raw, DefinitionRawOrigin::Waiting)
            || !sealed(parser, position)
            || position.allowance < remaining_headroom(parser)?
        {
            return None;
        }
        let state = &mut parser.state;
        let parent_raw =
            core::mem::replace(&mut state.definition_raw, DefinitionRawOrigin::Waiting);
        Some(Box::new(Self(Phase::Trial(Origin {
            checkpoint: parser.checkpoint(),
            position,
            kind,
            parent_raw,
            recovery_allowed: parser.consuming_recovery_allowed(),
            stats: parser.state.stats,
            ids: parser.ids.clone(),
            fuel: parser.state.fuel,
            rewinds: parser.state.rewinds,
            rewound_bytes: parser.state.rewound_bytes,
            cache_work: parser.state.tree_cache.work,
        }))))
    }

    pub(super) fn observe(
        self: Box<Self>,
        parser: &mut Parser<'_>,
        position: Position,
        winner: bool,
    ) -> Option<Box<Self>> {
        match self.0 {
            Phase::Trial(origin) => {
                let raw = core::mem::replace(&mut parser.state.definition_raw, origin.parent_raw);
                if !winner {
                    return None;
                }
                origin
                    .finish(parser, position, raw)
                    .map(|summary| Box::new(Self(Phase::Selected(summary))))
            }
            Phase::Selected(_) if winner => None,
            Phase::Selected(_) => Some(self),
        }
    }

    pub(super) fn apply(
        self: Box<Self>,
        parser: &mut Parser<'_>,
        at: Position,
    ) -> Option<(u64, u64)> {
        let Phase::Selected(summary) = self.0 else {
            unreachable!("definition trial must complete before applying a selected replay");
        };
        summary.apply(parser, at)
    }
}

impl Origin {
    fn finish(
        self,
        parser: &Parser<'_>,
        at: Position,
        raw: DefinitionRawOrigin,
    ) -> Option<Summary> {
        let state = &parser.state;
        let end = parser.checkpoint();
        let start = self.checkpoint;
        let offset = start.cursor.offset;
        let events = end.events.checked_sub(start.events)?;
        // Completed one-shot statements can contain several nested blocks.
        // Bound their transcript separately from single-definition choices.
        let scale = if self.kind == SyntaxKind::Statement {
            4
        } else {
            1
        };
        if !sealed(parser, at)
            || at.poll != self.position.poll
            || at.delimiter != self.position.delimiter
            || end.cursor.offset <= offset
            || (end.cursor.offset - offset).0 > MAX_BYTES * scale as u32
            || events == 0
            || events > MAX_EVENTS * scale
            || end.open_markers != start.open_markers
            || end.rule_depth != start.rule_depth
            || end.nesting != start.nesting
            || parser.consuming_recovery_allowed() != self.recovery_allowed
            || self.ids.next_diagnostic() == u64::MAX
            || parser.ids.next_diagnostic() != self.ids.next_diagnostic()
        {
            return None;
        }
        let charges = self.fuel.checked_sub(state.fuel)?;
        let expected_stats = ParseStats {
            parser_steps: self.stats.parser_steps.saturating_add(charges),
            ..self.stats
        };
        if expected_stats != state.stats || !event_headroom(parser) {
            return None;
        }
        // The enclosing BestChoice frame was charged after the trial.
        let steps = at.steps.checked_sub(self.position.steps)?.checked_sub(1)?;
        let child_work = at.child_work.checked_sub(self.position.child_work)?;
        let work = steps.checked_add(child_work)?;
        if self.position.allowance < work.checked_add(remaining_headroom(parser)?)? {
            return None;
        }
        let nodes = parser.ids.next_node().checked_sub(self.ids.next_node())?;
        let tokens = parser.ids.next_token().checked_sub(self.ids.next_token())?;
        if nodes == 0
            || nodes > MAX_IDS * scale as u64
            || tokens > MAX_IDS * scale as u64
            || parser.ids.next_node() == u64::MAX
            || parser.ids.next_token() == u64::MAX
        {
            return None;
        }
        let raw = match raw {
            DefinitionRawOrigin::Waiting => RawTransfer::Unchanged,
            DefinitionRawOrigin::Fresh { at, final_end } => RawTransfer::Fresh {
                at,
                final_end,
                tail: state.grapheme_tail.clone(),
            },
            DefinitionRawOrigin::Off | DefinitionRawOrigin::Inherited => return None,
        };
        let range = TextRange::new(offset, end.cursor.offset);
        let Event::Start {
            kind,
            cached: Some(root),
            ..
        } = state.events.get(start.events)?
        else {
            return None;
        };
        if *kind != self.kind || root.end_event != end.events || root.range != range {
            return None;
        }
        let cache = state.tree_cache.work;
        Some(Summary {
            end,
            steps,
            child_work,
            charges,
            rewinds: state.rewinds - self.rewinds,
            rewound_bytes: state.rewound_bytes - self.rewound_bytes,
            cache_work: CacheWork {
                events: cache.events - self.cache_work.events,
                token_bytes: cache.token_bytes - self.cache_work.token_bytes,
                child_storage_nodes: cache.child_storage_nodes
                    - self.cache_work.child_storage_nodes,
                nodes: cache.nodes - self.cache_work.nodes,
                reused: cache.reused - self.cache_work.reused,
            },
            origin: self,
            nodes,
            tokens,
            raw,
            events: state
                .events
                .iter_range(start.events..end.events)
                .cloned()
                .collect(),
        })
    }
}

// The trial already reached every transient event/marker peak from the same
// checkpoint. Their sum bounds simultaneous events, finishes and recovery.
fn event_headroom(parser: &Parser<'_>) -> bool {
    let state = &parser.state;
    state
        .peak_events
        .saturating_add(state.peak_open_markers)
        .saturating_add(3)
        <= state.config.limits.max_events as usize
}

impl Summary {
    fn apply(self, parser: &mut Parser<'_>, at: Position) -> Option<(u64, u64)> {
        if matches!(parser.state.definition_raw, DefinitionRawOrigin::Waiting)
            || !sealed(parser, at)
            || self.origin.position.poll != at.poll
            || self.origin.position.delimiter != at.delimiter
            || self.origin.checkpoint != parser.checkpoint()
            || self.origin.recovery_allowed != parser.consuming_recovery_allowed()
            || self.origin.stats.recovery_bytes != parser.state.stats.recovery_bytes
            || !event_headroom(parser)
            || self.charges > parser.state.fuel
            || at.allowance
                < self
                    .steps
                    .checked_add(self.child_work)?
                    .checked_add(remaining_headroom(parser)?)?
        {
            return None;
        }
        if let RawTransfer::Fresh { at, final_end, .. } = &self.raw
            && parser
                .state
                .grapheme_tail
                .as_ref()
                .is_some_and(|tail| tail.resumes_at(*at, *final_end))
        {
            return None;
        }
        let next_nodes = parser.ids.next_node().checked_add(self.nodes)?;
        let next_tokens = parser.ids.next_token().checked_add(self.tokens)?;
        let rewinds = parser.state.rewinds.checked_add(self.rewinds)?;
        let rewound_bytes = parser.state.rewound_bytes.checked_add(self.rewound_bytes)?;
        if !APPLY {
            core::hint::black_box(self);
            return None;
        }
        let events = self.fresh_events(parser.ids)?;

        // The trial's transients already raised the historical peaks. No cache
        // request survives the branch. Install precisely its live publications
        // and consume the allocation ordinals, including those of stale work.
        parser.state.events.extend(events.into_iter().rev());
        parser.cursor.rewind(CursorCheckpoint {
            offset: self.end.cursor.offset,
        });
        parser.state.covered_end = self.end.covered_end;
        parser.state.rewinds = rewinds;
        parser.state.rewound_bytes = rewound_bytes;
        parser.state.fuel -= self.charges;
        parser.state.stats.parser_steps =
            parser.state.stats.parser_steps.saturating_add(self.charges);
        parser.state.tree_cache.work.accumulate(self.cache_work);
        *parser.ids = IdGenerator::with_next(next_nodes, next_tokens, parser.ids.next_diagnostic());
        if let RawTransfer::Fresh { tail, .. } = self.raw {
            parser.state.grapheme_tail = tail;
        }
        Some((self.steps, self.child_work))
    }

    fn fresh_events(&self, ids: &IdGenerator) -> Option<Vec<Event>> {
        let mut mapped = vec![None; self.nodes as usize];
        let mut events = Vec::with_capacity(self.events.len());
        for event in self.events.iter().rev() {
            let event = match event {
                Event::Start {
                    identity: Some(identity),
                    cached: Some(cached),
                    kind,
                    flags,
                } => {
                    let old = &cached.node;
                    if old.id != *identity {
                        return None;
                    }
                    let before_nodes = self.origin.ids.next_node();
                    let before_tokens = self.origin.ids.next_token();
                    let node_offset = ordinal(old.id.0, before_nodes, self.nodes)? as usize;
                    let mut children = GreenChildren::for_kind(old.kind);
                    for child in old.children.iter() {
                        let element = match child {
                            GreenElement::Node(node) => {
                                let index = ordinal(node.id.0, before_nodes, self.nodes)? as usize;
                                let (original, fresh): &(&Arc<GreenNode>, Arc<GreenNode>) =
                                    mapped.get(index)?.as_ref()?;
                                if !Arc::ptr_eq(original, node) {
                                    return None;
                                }
                                GreenElement::Node(fresh.clone())
                            }
                            GreenElement::Token(token) => {
                                let token_offset = ordinal(token.id.0, before_tokens, self.tokens)?;
                                GreenElement::Token(GreenToken {
                                    id: TokenId(ids.next_token() + token_offset),
                                    ..*token
                                })
                            }
                        };
                        children.append(element);
                    }
                    let id = NodeId(ids.next_node() + node_offset as u64);
                    let fresh = Arc::new(GreenNode {
                        id,
                        children,
                        kind: old.kind,
                        text_len: old.text_len,
                        structural_hash: old.structural_hash,
                        flags: old.flags,
                    });
                    if mapped[node_offset].replace((old, fresh.clone())).is_some() {
                        return None;
                    }
                    Event::Start {
                        identity: Some(id),
                        cached: Some(CachedNode {
                            node: fresh,
                            range: cached.range,
                            end_event: cached.end_event,
                        }),
                        kind: *kind,
                        flags: *flags,
                    }
                }
                Event::Start { .. } | Event::Reuse { .. } => return None,
                Event::Token { .. } | Event::Finish | Event::Tombstone => event.clone(),
            };
            events.push(event);
        }
        Some(events)
    }
}

fn ordinal(id: u64, first: u64, count: u64) -> Option<u64> {
    let offset = id.checked_sub(first)?;
    (offset < count).then_some(offset)
}
