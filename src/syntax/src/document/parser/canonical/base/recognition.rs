//! Bounded successful lexical recognition owned by one resumable parser.
//! Entries describe fresh flat node construction; they never retain syntax IDs.
use super::{Spec, TerminalSpacing, fixed_terminal_spec, specification};
use crate::document::parser::{
    Parser, checkpoint::ParserCheckpoint, event::Event, grapheme_scan::GraphemeScan,
};
use crate::document::{
    NodeFlags, NodeId, ParseStats, RuleId, SyntaxKind, TextRange, TextSize, TokenFlags,
};
use alloc::vec::Vec;
use smallvec::SmallVec;

const CAPACITY: usize = 1024;
const MAX_BYTES: u32 = 64;

#[derive(Clone, Copy, Eq, PartialEq)]
struct Key {
    rule: RuleId,
    start: TextSize,
    consume_end: TextSize,
    context_end: TextSize,
}
impl Key {
    fn new(p: &Parser<'_>, rule: RuleId) -> Self {
        Self {
            rule,
            start: p.offset(),
            consume_end: p.cursor().end(),
            context_end: p.cursor().context_end(),
        }
    }
    fn slot(self) -> usize {
        let hash = self.rule.0 ^ self.start.0.wrapping_mul(0x9e3779b9);
        ((hash ^ (hash >> 16)) as usize) & (CAPACITY - 1)
    }
}

#[derive(Clone, Copy)]
struct Peak {
    extra: usize,
    exact: bool,
}
impl Peak {
    fn observed(start: usize, before: usize, after: usize) -> Self {
        Self {
            extra: after - start,
            exact: after > before || before <= start,
        }
    }
    fn replay(self, start: usize, previous: usize) -> Option<usize> {
        let next = start.checked_add(self.extra)?;
        (self.exact || next <= previous).then_some(next.max(previous))
    }
}

struct Entry {
    key: Key,
    end: TextSize,
    kind: SyntaxKind,
    tokens: SmallVec<[SyntaxKind; 16]>,
    work: u64,
    charges: u64,
    rewinds: u64,
    rewound_bytes: u64,
    events: Peak,
    markers: Peak,
    rules: Peak,
    raw: TextRange,
    tail: GraphemeScan,
}

// Retain prediction and recognition state behind the existing storage pointer.
// Active continuations and the outer parser frame enums keep their old sizes.
pub(in super::super) struct Storage {
    plans: [Option<super::super::prediction::Plan>; super::NON_FIXED_RULES.len()],
    pub(super) memo: Cache,
}
impl Default for Storage {
    fn default() -> Self {
        Self {
            plans: [None; super::NON_FIXED_RULES.len()],
            memo: Cache::default(),
        }
    }
}
impl core::ops::Deref for Storage {
    type Target = [Option<super::super::prediction::Plan>; super::NON_FIXED_RULES.len()];
    fn deref(&self) -> &Self::Target {
        &self.plans
    }
}
impl core::ops::DerefMut for Storage {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.plans
    }
}

#[derive(Default)]
pub(super) struct Cache {
    plans: Vec<(RuleId, Option<SyntaxKind>)>,
    slots: Vec<Option<Entry>>,
    pub(super) capture: Option<Capture>,
    #[cfg(test)]
    pub(super) disabled: bool,
    #[cfg(test)]
    pub(super) hits: u64,
}
impl Cache {
    pub(super) fn kind(
        &mut self,
        rule: RuleId,
        kind: SyntaxKind,
        first: &[RuleId],
        rest: &[RuleId],
    ) -> Option<SyntaxKind> {
        #[cfg(test)]
        if self.disabled {
            return None;
        }
        if let Some((_, kind)) = self.plans.iter().find(|(known, _)| *known == rule) {
            return *kind;
        }
        // A future grammar containing nested nodes or cycles declines this
        // optimization. The production specification remains the authority.
        let accepted = first
            .iter()
            .chain(rest)
            .all(|&child| flat(child, 0))
            .then_some(kind);
        self.plans.push((rule, accepted));
        accepted
    }

    pub(super) fn replay(
        &mut self,
        p: &mut Parser<'_>,
        rule: RuleId,
        rule_depth: usize,
        allowance: u64,
    ) -> Option<(u64, TextRange)> {
        let key = Key::new(p, rule);
        let entry = self.slots.get(key.slot())?.as_ref()?;
        if entry.key != key
            || p.is_halted()
            || p.state.resource_finalizing
            || entry.work > allowance
            || entry.charges > p.state.fuel
            || p.state
                .grapheme_tail
                .as_ref()
                .is_some_and(|tail| tail.resumes_at(key.start, Some(key.context_end)))
        {
            return None;
        }
        let events = entry
            .events
            .replay(p.state.events.len(), p.state.peak_events)?;
        let markers = entry
            .markers
            .replay(p.state.open_markers.len(), p.state.peak_open_markers)?;
        let rules = entry.rules.replay(rule_depth, p.state.rules.peak_depth())?;
        // Reserve every speculative event/marker from the original run, all
        // enclosing finishes, and the unchanged resource error envelope.
        let reserve = p
            .state
            .events
            .len()
            .checked_add(entry.events.extra)?
            .checked_add(p.state.open_markers.len())?
            .checked_add(entry.markers.extra)?
            .checked_add(3)?;
        if reserve > p.state.config.limits.max_events as usize {
            return None;
        }
        // All admission checks precede mutation. The cold run starts its first
        // raw scanner afresh at this key, making the final Unicode tail exact.
        p.observe_definition_raw_origin(Some(key.context_end));
        let start = p.state.events.len();
        let identity = p.ids.node();
        let node = core::iter::once(Event::Start {
            identity: Some(identity),
            cached: None,
            kind: entry.kind,
            flags: NodeFlags::NONE,
        });
        let tokens = entry.tokens.iter().enumerate().map(|(index, &kind)| {
            let at = key.start + TextSize(index as u32);
            Event::Token {
                kind,
                range: TextRange::new(at, at + TextSize(1)),
                flags: TokenFlags::NONE,
            }
        });
        p.state
            .events
            .extend(node.chain(tokens).chain(core::iter::once(Event::Finish)));
        p.state.covered_end = p.state.covered_end.max(entry.end);
        let range = p.cursor.bump_bytes((entry.end - key.start).0);
        debug_assert_eq!(range, Some(TextRange::new(key.start, entry.end)));
        p.state
            .tree_cache
            .enqueue(&p.state.events, start, key.start, identity);
        p.state.fuel -= entry.charges;
        p.state.stats.parser_steps = p.state.stats.parser_steps.saturating_add(entry.charges);
        p.state.rewinds += entry.rewinds;
        p.state.rewound_bytes += entry.rewound_bytes;
        p.state.peak_events = events;
        p.state.peak_open_markers = markers;
        p.state.rules.note_peak(rules);
        p.state.grapheme_tail = Some(entry.tail.clone());
        #[cfg(test)]
        {
            self.hits += 1;
        }
        Some((entry.work, entry.raw))
    }
}

fn flat(rule: RuleId, depth: usize) -> bool {
    if depth >= super::NON_FIXED_RULES.len() {
        return false;
    }
    if let Some(spec) = fixed_terminal_spec(rule) {
        return spec.spacing == TerminalSpacing::Exact
            || flat(super::rules::WHITESPACE0, depth + 1);
    }
    let all = |rules: &[RuleId]| rules.iter().all(|&rule| flat(rule, depth + 1));
    match specification(rule) {
        Some(Spec::Alias(rule)) => flat(rule, depth + 1),
        Some(Spec::Choice(rules) | Spec::Sequence(rules)) => all(rules),
        Some(Spec::Repeat { choices, .. }) => all(choices),
        Some(Spec::Unless { excluded, rule }) => all(excluded) && flat(rule, depth + 1),
        Some(Spec::Classified(..) | Spec::Raw(..) | Spec::Never) => true,
        Some(Spec::NewlineIndent) => {
            flat(super::rules::NEW_LINE, depth + 1) && flat(super::rules::SPACE_TAB, depth + 1)
        }
        Some(Spec::RepeatedNode { .. } | Spec::Escape) | None => false,
    }
}

pub(super) struct Capture {
    key: Key,
    kind: SyntaxKind,
    pub(super) depth: usize,
    checkpoint: ParserCheckpoint,
    work: u64,
    fuel: u64,
    stats: ParseStats,
    rewinds: u64,
    rewound_bytes: u64,
    cached_replay_bytes: u64,
    next_node: u64,
    next_token: u64,
    next_diagnostic: u64,
    peak_events: usize,
    peak_markers: usize,
    peak_rules: usize,
    first_raw_fresh: Option<bool>,
}
impl Capture {
    pub(super) fn new(
        p: &Parser<'_>,
        rule: RuleId,
        kind: SyntaxKind,
        depth: usize,
        checkpoint: ParserCheckpoint,
        work: u64,
    ) -> Self {
        Self {
            key: Key::new(p, rule),
            kind,
            depth,
            checkpoint,
            work,
            fuel: p.state.fuel,
            stats: p.state.stats,
            rewinds: p.state.rewinds,
            rewound_bytes: p.state.rewound_bytes,
            cached_replay_bytes: p.state.cached_replay_bytes,
            next_node: p.ids.next_node(),
            next_token: p.ids.next_token(),
            next_diagnostic: p.ids.next_diagnostic(),
            peak_events: p.state.peak_events,
            peak_markers: p.state.peak_open_markers,
            peak_rules: p.state.rules.peak_depth(),
            first_raw_fresh: None,
        }
    }
    pub(super) fn raw(&mut self, p: &Parser<'_>) {
        if self.first_raw_fresh.is_none() {
            self.first_raw_fresh = Some(
                p.offset() == self.key.start
                    && !p.state.grapheme_tail.as_ref().is_some_and(|tail| {
                        tail.resumes_at(self.key.start, Some(self.key.context_end))
                    }),
            );
        }
    }
    pub(super) fn complete(
        self,
        cache: &mut Cache,
        p: &Parser<'_>,
        success: bool,
        work: u64,
        raw: Option<TextRange>,
    ) {
        let start = self.checkpoint;
        let end = p.offset();
        if !success
            || p.is_halted()
            || p.state.resource_finalizing
            || self.first_raw_fresh != Some(true)
            || Key::new(p, self.key.rule).consume_end != self.key.consume_end
            || p.cursor().context_end() != self.key.context_end
            || end <= self.key.start
            || (end - self.key.start).0 > MAX_BYTES
            || p.state.diagnostics.len() != start.diagnostics
            || p.state.open_markers.len() != start.open_markers
            || p.state.rules.len() != start.rule_depth
            || p.state.nesting != start.nesting
            || self.next_node.checked_add(1) != Some(p.ids.next_node())
            || self.next_token != p.ids.next_token()
            || self.next_diagnostic != p.ids.next_diagnostic()
            || p.state.cached_replay_bytes != self.cached_replay_bytes
        {
            return;
        }
        let charges = self.fuel - p.state.fuel;
        let mut stats = self.stats;
        stats.parser_steps = stats.parser_steps.saturating_add(charges);
        if stats != p.state.stats {
            return;
        }
        let Some(raw) = raw.filter(|range| range.start >= self.key.start && range.end <= end)
        else {
            return;
        };
        let Some(tail) = p.state.grapheme_tail.as_ref() else {
            return;
        };
        let count = (end - self.key.start).0 as usize;
        if p.state.events.len() - start.events != count + 2 {
            return;
        }
        let mut tokens = SmallVec::new();
        for (index, event) in p
            .state
            .events
            .iter_range(start.events..p.state.events.len())
            .enumerate()
        {
            match (index, event) {
                (
                    0,
                    Event::Start {
                        kind,
                        flags,
                        identity: Some(identity),
                        cached: None,
                    },
                ) if *kind == self.kind
                    && *flags == NodeFlags::NONE
                    && *identity == NodeId(self.next_node) => {}
                (index, Event::Finish) if index == count + 1 => {}
                (index, Event::Token { kind, range, flags })
                    if index > 0
                        && index <= count
                        && *flags == TokenFlags::NONE
                        && range.start == self.key.start + TextSize(index as u32 - 1)
                        && range.len() == TextSize(1)
                        && p.source.byte_at(range.start).is_some_and(|b| b.is_ascii()) =>
                {
                    tokens.push(*kind);
                }
                _ => return,
            }
        }
        if tokens.len() != count {
            return;
        }
        let entry = Entry {
            key: self.key,
            end,
            kind: self.kind,
            tokens,
            work: work - self.work,
            charges,
            rewinds: p.state.rewinds - self.rewinds,
            rewound_bytes: p.state.rewound_bytes - self.rewound_bytes,
            events: Peak::observed(start.events, self.peak_events, p.state.peak_events),
            markers: Peak::observed(
                start.open_markers,
                self.peak_markers,
                p.state.peak_open_markers,
            ),
            rules: Peak::observed(
                start.rule_depth,
                self.peak_rules,
                p.state.rules.peak_depth(),
            ),
            raw,
            tail: tail.clone(),
        };
        if cache.slots.is_empty() {
            cache.slots.resize_with(CAPACITY, || None);
        }
        cache.slots[self.key.slot()] = Some(entry);
    }
}

#[cfg(test)]
mod tests {
    use super::super::continuation::{Continuation, Progress};
    use super::*;
    use crate::document::parser::{Cursor, LexicalMode};
    use crate::document::{
        DocumentId, GreenElement, GreenNode, IdGenerator, ParseConfig, Revision,
    };
    use alloc::{format, string::String};

    #[derive(Debug, Eq, PartialEq)]
    struct Outcome {
        results: Vec<(bool, TextSize, u64)>,
        trace: String,
        tree: String,
        stats: ParseStats,
        ids: (u64, u64, u64),
    }
    fn tree(node: &GreenNode, text: &mut String) {
        text.push_str(&format!(
            "N{}:{:?}:{}:{};",
            node.id.0, node.kind, node.text_len.0, node.structural_hash
        ));
        for child in node.children.iter() {
            match child {
                GreenElement::Node(n) => tree(n, text),
                GreenElement::Token(t) => text.push_str(&format!(
                    "T{}:{:?}:{}:{};",
                    t.id.0, t.kind, t.text_len.0, t.text_hash
                )),
            }
        }
        text.push(')');
    }
    fn run(
        rule: RuleId,
        text: &str,
        views: &[(u32, u32)],
        quantum: u64,
        fuel: u64,
        events: u32,
        disabled: bool,
    ) -> (Outcome, u64) {
        let source =
            crate::document::TextSnapshot::new(DocumentId(514), Revision(1), text).unwrap();
        let mut ids = IdGenerator::new();
        let mut config = ParseConfig::default();
        config.limits.fuel = fuel;
        config.limits.max_events = events;
        let mut p = Parser::new(
            &source,
            LexicalMode::CanonicalSourceFragment,
            config,
            &mut ids,
        );
        p.state
            .base_storage
            .prediction
            .get_or_insert_with(|| alloc::boxed::Box::new(Storage::default()))
            .memo
            .disabled = disabled;
        let wrapper = p.start();
        let checkpoint = p.checkpoint();
        let mut results = Vec::new();
        let mut trace = String::new();
        for (index, &(consume, context)) in views.iter().enumerate() {
            if index > 0 {
                p.rewind(checkpoint);
            }
            p.cursor = Cursor::for_range_with_context(
                &source,
                TextRange::new(TextSize::ZERO, TextSize(consume)),
                TextSize(context),
            );
            let mut c = Continuation::new(rule);
            let result = loop {
                let mut allowance = quantum;
                match c.advance(&mut p, true, &mut allowance) {
                    Progress::Complete(result) => break result,
                    Progress::NeedsProcessing => {}
                    Progress::NeedInput | Progress::Limited => {
                        panic!("sealed finite lexical parse")
                    }
                }
            };
            results.push((result, p.offset(), c.work));
            trace.push_str(&format!(
                "{},{},{},{},{},{},{:?};",
                p.state.fuel,
                p.state.rewinds,
                p.state.rewound_bytes,
                p.state.peak_events,
                p.state.peak_open_markers,
                p.state.rules.peak_depth(),
                p.state.grapheme_tail
            ));
            if p.is_halted() {
                break;
            }
        }
        let hits = p
            .state
            .base_storage
            .prediction
            .as_ref()
            .map_or(0, |state| state.memo.hits);
        wrapper.complete(&mut p, SyntaxKind::Document);
        let output = p.finish();
        let parsed =
            crate::document::parser::event::sink(&output.events, &source, &mut ids).unwrap();
        let mut nodes = String::new();
        tree(&parsed.root, &mut nodes);
        (
            Outcome {
                results,
                trace,
                tree: nodes,
                stats: output.stats,
                ids: (ids.next_node(), ids.next_token(), ids.next_diagnostic()),
            },
            hits,
        )
    }
    #[test]
    fn replay_assigns_fresh_identities_and_keeps_unicode_state_and_work() {
        let text = "alpha3_z-then+";
        let views = vec![(text.len() as u32, text.len() as u32); 3];
        let (candidate, hits) = run(
            super::super::rules::IDENTIFIER,
            text,
            &views,
            u64::MAX,
            100000,
            1000,
            false,
        );
        let (reference, _) = run(
            super::super::rules::IDENTIFIER,
            text,
            &views,
            u64::MAX,
            100000,
            1000,
            true,
        );
        assert!(hits >= 2, "positive replay must actually execute");
        assert_eq!(candidate, reference);
    }
    #[test]
    fn repeated_lexical_replay_preserves_limits_view_bounds_and_fallbacks() {
        for rule in [
            super::super::rules::IDENTIFIER,
            super::super::rules::DIGIT_SEQUENCE,
            super::super::rules::IDENTIFIER_PATH_SEGMENT,
        ] {
            for text in [
                "abc",
                "abc_3-z:",
                "x_",
                "7",
                "123_456",
                "1_",
                "A1_2+",
                "caf\u{e9}",
                "a\u{301}",
                "a\u{1f980}",
                "\u{1f642}z",
                "",
                "1_\u{1f980}",
            ] {
                let n = text.len() as u32;
                let mut views = vec![(n, n); 3];
                if text.is_char_boundary(1) && n >= 1 {
                    views.extend_from_slice(&[(1, n), (n, n), (1, 1), (n, n)]);
                }
                for quantum in [1, 17, 4096, u64::MAX] {
                    for fuel in [0, 1, 2, 5, 32, 100000] {
                        for events in [7, 12, 64, 1000] {
                            let (candidate, _) =
                                run(rule, text, &views, quantum, fuel, events, false);
                            let (reference, _) =
                                run(rule, text, &views, quantum, fuel, events, true);
                            assert_eq!(
                                candidate, reference,
                                "rule {rule:?} text {text:?} quantum {quantum} fuel {fuel} events {events}"
                            );
                        }
                    }
                }
            }
        }
    }
}
