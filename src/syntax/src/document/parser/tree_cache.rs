//! Retained canonical subtree materialization. A completed grammar marker owns
//! one immutable green node; ancestors reuse it instead of rebuilding children.
use super::*;
use crate::document::green::FNV_OFFSET;
use crate::document::{BuildError, GreenChildren, GreenElement, GreenToken, NodeId};
use alloc::collections::VecDeque;

#[derive(Clone, Debug)]
pub struct CachedNode {
    pub node: Arc<GreenNode>,
    pub range: TextRange,
    pub end_event: usize,
}
struct Request {
    start: usize,
    end: usize,
    offset: TextSize,
    identity: NodeId,
    retained: Option<Journal<Event>>,
}

const MAX_COPIED_EVENTS: usize = 16;

enum EventWindow<'a> {
    Journal(&'a Journal<Event>),
    Copied {
        start: usize,
        first: &'a [Event],
        second: &'a [Event],
    },
}

impl<'a> EventWindow<'a> {
    fn page_at(&self, index: usize) -> Option<(usize, &'a [Event])> {
        match *self {
            Self::Journal(events) => events.page_at(index),
            Self::Copied {
                start,
                first,
                second,
            } => {
                let split = start + first.len();
                if index < split {
                    Some((start, first))
                } else {
                    Some((split, second))
                }
            }
        }
    }
}

impl Request {
    fn events<'a>(&'a self, copied_events: &'a VecDeque<Event>) -> EventWindow<'a> {
        if let Some(retained) = &self.retained {
            return EventWindow::Journal(retained);
        }
        let count = self.end - self.start;
        let (first, second) = copied_events.as_slices();
        let split = first.len().min(count);
        EventWindow::Copied {
            start: self.start,
            first: &first[..split],
            second: &second[..count - split],
        }
    }
}
struct Frame {
    kind: SyntaxKind,
    flags: NodeFlags,
    identity: Option<NodeId>,
    start: usize,
    children: GreenChildren,
}
struct Token {
    kind: SyntaxKind,
    flags: TokenFlags,
    range: TextRange,
    at: TextSize,
    hash: u64,
}
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct CacheWork {
    pub events: u64,
    pub token_bytes: u64,
    pub child_storage_nodes: u64,
    pub nodes: u64,
    pub reused: u64,
}
impl CacheWork {
    pub(super) fn accumulate(&mut self, work: Self) {
        self.events += work.events;
        self.token_bytes += work.token_bytes;
        self.child_storage_nodes += work.child_storage_nodes;
        self.nodes += work.nodes;
        self.reused += work.reused;
    }
}
#[derive(Default)]
pub(crate) struct Sink {
    at: usize,
    end: usize,
    frames: Vec<Frame>,
    token: Option<Token>,
    root: Option<Arc<GreenNode>>,
    discarded_depth: usize,
    discarded_root: bool,
    event_nodes: Option<BTreeMap<usize, NodeId>>,
    pub work: CacheWork,
}
impl Sink {
    pub fn new(start: usize, end: usize, collect_event_nodes: bool) -> Self {
        Self {
            at: start,
            end,
            event_nodes: collect_event_nodes.then(BTreeMap::new),
            ..Self::default()
        }
    }

    fn begin_cache_request(&mut self, start: usize, end: usize) {
        // The previous request completed before the next one starts. Keep its
        // empty frame stack so materializing another node can reuse capacity.
        debug_assert!(self.frames.is_empty() && self.token.is_none() && self.root.is_none());
        debug_assert!(self.discarded_depth == 0 && self.event_nodes.is_none());
        self.at = start;
        self.end = end;
        self.discarded_root = false;
        self.work = CacheWork::default();
    }
    fn element(&mut self, element: GreenElement) -> Result<(), BuildError> {
        if let Some(frame) = self.frames.last_mut() {
            self.work.child_storage_nodes += frame.children.append(element);
        } else if let GreenElement::Node(node) = element {
            if self.root.replace(node).is_some() {
                return Err(BuildError::MultipleRoots(2));
            }
        } else {
            return Err(BuildError::NoOpenNode);
        }
        Ok(())
    }
    fn discard_element(&mut self, is_node: bool) -> Result<(), BuildError> {
        if self.discarded_depth == 0 {
            if !is_node {
                return Err(BuildError::NoOpenNode);
            }
            if core::mem::replace(&mut self.discarded_root, true) {
                return Err(BuildError::MultipleRoots(2));
            }
        }
        Ok(())
    }
    pub fn advance(
        &mut self,
        source: &TextSnapshot,
        events: &Journal<Event>,
        current: &Journal<Event>,
        ids: &mut IdGenerator,
        allowance: &mut u64,
    ) -> Result<bool, BuildError> {
        self.advance_mode::<true>(
            source,
            EventWindow::Journal(events),
            current,
            ids,
            allowance,
        )
    }

    fn advance_mode<const MATERIALIZE: bool>(
        &mut self,
        source: &TextSnapshot,
        events: EventWindow<'_>,
        current: &Journal<Event>,
        ids: &mut IdGenerator,
        allowance: &mut u64,
    ) -> Result<bool, BuildError> {
        let refresh_child_cache =
            !matches!(events, EventWindow::Journal(journal) if core::ptr::eq(journal, current));
        let (mut page_start, mut page): (usize, &[Event]) = (0, &[]);
        loop {
            if self.at >= self.end && self.token.is_none() {
                let depth = if MATERIALIZE {
                    self.frames.len()
                } else {
                    self.discarded_depth
                };
                if depth != 0 {
                    return Err(BuildError::UnclosedNodes(depth));
                }
                let has_root = if MATERIALIZE {
                    self.root.is_some()
                } else {
                    self.discarded_root
                };
                if !has_root {
                    return Err(BuildError::MultipleRoots(0));
                }
                return Ok(true);
            }
            if *allowance == 0 {
                return Ok(false);
            }
            *allowance -= 1;
            if let Some(mut token) = self.token.take() {
                if token.at < token.range.end {
                    // The first byte was charged by the loop. Stop at the
                    // same byte even when a budget ends within a UTF-8 scalar.
                    let count = if MATERIALIZE {
                        let chunk = source.chunk_at(token.at).ok_or(BuildError::TextTooLarge)?;
                        let available =
                            (token.range.end.min(chunk.range.end) - token.at).to_usize();
                        let count = (available as u64).min(*allowance + 1) as usize;
                        let start = (token.at - chunk.range.start).to_usize();
                        for &byte in &chunk.text.as_bytes()[start..start + count] {
                            token.hash =
                                (token.hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
                        }
                        count
                    } else {
                        u64::from((token.range.end - token.at).0).min(*allowance + 1) as usize
                    };
                    token.at += TextSize(count as u32);
                    *allowance -= count as u64 - 1;
                    self.work.token_bytes += count as u64;
                    self.token = Some(token);
                } else if MATERIALIZE {
                    self.element(GreenElement::Token(GreenToken {
                        id: ids.token(),
                        kind: token.kind,
                        text_len: token.range.len(),
                        flags: token.flags,
                        text_hash: token.hash,
                    }))?;
                } else {
                    ids.token();
                    self.discard_element(false)?;
                }
                continue;
            }
            let index = self.at;
            if index < page_start || index - page_start >= page.len() {
                (page_start, page) = events.page_at(index).ok_or(BuildError::NoOpenNode)?;
            }
            let event = page
                .get(index - page_start)
                .ok_or(BuildError::NoOpenNode)?
                .clone();
            self.at += 1;
            self.work.events += 1;
            match event {
                Event::Start {
                    kind,
                    flags,
                    identity,
                    cached,
                } => {
                    // A queued parent may predate a child's cache publication.
                    // Its immutable marker identity proves whether the current
                    // journal still describes that exact completed child.
                    let cached = cached.or_else(|| {
                        if identity.is_none() || !refresh_child_cache {
                            return None;
                        }
                        match current.get(index) {
                            Some(Event::Start {
                                identity: now,
                                cached,
                                ..
                            }) if *now == identity => cached.clone(),
                            _ => None,
                        }
                    });
                    if let Some(cached) = cached.filter(|cached| cached.end_event <= self.end) {
                        self.at = cached.end_event;
                        if let Some(nodes) = &mut self.event_nodes {
                            nodes.insert(index, cached.node.id);
                        }
                        self.work.reused += 1;
                        if MATERIALIZE {
                            self.element(GreenElement::Node(cached.node))?;
                        } else {
                            self.discard_element(true)?;
                        }
                    } else if MATERIALIZE {
                        self.frames.push(Frame {
                            kind,
                            flags,
                            identity,
                            start: index,
                            children: GreenChildren::for_kind(kind),
                        });
                    } else {
                        self.discarded_depth += 1;
                        // A discarded request has no observable intermediate identities.
                        // Its anonymous nodes still consume the original ID sequence.
                        if identity.is_none() {
                            ids.node();
                        }
                    }
                }
                Event::Token { kind, range, flags } => {
                    if !kind.is_token() {
                        return Err(BuildError::TokenKindExpected(kind));
                    }
                    source
                        .validate_range(range)
                        .map_err(|_| BuildError::TextTooLarge)?;
                    self.token = Some(Token {
                        kind,
                        flags,
                        range,
                        at: range.start,
                        hash: FNV_OFFSET,
                    });
                }
                Event::Reuse { node } => {
                    self.work.reused += 1;
                    if MATERIALIZE {
                        self.element(GreenElement::Node(node))?;
                    } else {
                        self.discard_element(true)?;
                    }
                }
                Event::Finish => {
                    if !MATERIALIZE {
                        self.discarded_depth = self
                            .discarded_depth
                            .checked_sub(1)
                            .ok_or(BuildError::NoOpenNode)?;
                        self.work.nodes += 1;
                        self.discard_element(true)?;
                        continue;
                    }
                    let frame = self.frames.pop().ok_or(BuildError::NoOpenNode)?;
                    let node = Arc::new(GreenNode {
                        id: frame.identity.unwrap_or_else(|| ids.node()),
                        kind: frame.kind,
                        text_len: frame.children.text_len(),
                        structural_hash: frame.children.hash(frame.kind),
                        flags: frame.children.flags(frame.kind, frame.flags),
                        children: frame.children,
                    });
                    self.work.nodes += 1;
                    if let Some(nodes) = &mut self.event_nodes {
                        nodes.insert(frame.start, node.id);
                    }
                    self.element(GreenElement::Node(node))?;
                }
                Event::Tombstone => {}
            }
        }
    }
    pub fn finish(self) -> SinkResult {
        SinkResult {
            root: self.root.expect("completed sink"),
            event_nodes: self.event_nodes.unwrap_or_default(),
        }
    }
}
#[derive(Default)]
pub(crate) struct TreeCache {
    queue: VecDeque<Request>,
    active: Option<Request>,
    sink: Sink,
    // Small one-shot requests own only their immutable event range. Share
    // storage in request order so none of them forces journal page copies.
    copied_events: VecDeque<Event>,
    pub work: CacheWork,
    pub peak_pending: usize,
    pub one_shot: bool,
}
impl TreeCache {
    pub fn enqueue(
        &mut self,
        events: &Journal<Event>,
        start: usize,
        offset: TextSize,
        identity: NodeId,
    ) {
        let end = events.len();
        let retained = if self.one_shot && end - start <= MAX_COPIED_EVENTS {
            let mut cursor = start;
            while cursor < end {
                let (base, page) = events.page_at(cursor).expect("queued marker page");
                let through = (base + page.len()).min(end);
                self.copied_events
                    .extend(page[cursor - base..through - base].iter().cloned());
                cursor = through;
            }
            None
        } else {
            Some(events.clone())
        };
        self.queue.push_back(Request {
            start,
            end,
            offset,
            identity,
            retained,
        });
        self.peak_pending = self
            .peak_pending
            .max(self.queue.len() + usize::from(self.active.is_some()));
    }
    pub fn total_work(&self) -> CacheWork {
        let mut work = self.work;
        if self.active.is_some() {
            work.accumulate(self.sink.work);
        }
        work
    }
    pub fn pending(&self) -> bool {
        self.active.is_some() || !self.queue.is_empty()
    }

    fn release(&mut self, request: Request) {
        if request.retained.is_none() {
            self.copied_events.drain(..request.end - request.start);
        }
    }

    pub fn advance(
        &mut self,
        source: &TextSnapshot,
        current: &mut Journal<Event>,
        ids: &mut IdGenerator,
        allowance: &mut u64,
    ) -> bool {
        loop {
            if !self.pending() {
                return true;
            }
            if *allowance == 0 {
                return false;
            }
            if self.active.is_none() {
                *allowance -= 1;
                let request = self.queue.pop_front().expect("pending cache request");
                // Cache publication needs the root, while the final sink
                // collects the event map used to anchor diagnostics.
                self.sink.begin_cache_request(request.start, request.end);
                if self.one_shot
                    && !matches!(current.get(request.start), Some(Event::Start { identity: Some(id), .. }) if *id == request.identity)
                    // Bound potential work before starting. Each event costs one unit.
                    // A token can add the byte length of the source and one finish unit.
                    && ((request.end - request.start) as u64)
                        .checked_mul(u64::from(source.byte_len().0) + 2)
                        .is_some_and(|bound| bound <= *allowance)
                {
                    // The marker cannot receive the result. Visit the same events and
                    // consume the same work and IDs without allocating a discarded tree.
                    let complete = self
                        .sink
                        .advance_mode::<false>(
                            source,
                            request.events(&self.copied_events),
                            current,
                            ids,
                            allowance,
                        )
                        .expect("completed canonical marker must materialize");
                    assert!(complete, "bounded cache accounting must complete");
                    self.work.accumulate(self.sink.work);
                    self.release(request);
                    continue;
                }
                self.active = Some(request);
            }
            let request = self.active.as_ref().expect("active cache request");
            if !self
                .sink
                .advance_mode::<true>(
                    source,
                    request.events(&self.copied_events),
                    current,
                    ids,
                    allowance,
                )
                .expect("completed canonical marker must materialize")
            {
                return false;
            }
            let node = self.sink.root.take().expect("materialized root");
            let cached = CachedNode {
                range: TextRange::at(request.offset, node.text_len),
                node,
                end_event: request.end,
            };
            let request = self.active.take().expect("completed cache request");
            let start = request.start;
            let expected_identity = request.identity;
            // Release the request's snapshot before publishing the new cache
            // entry, so the completed request cannot force path copying.
            self.release(request);
            if let Some(Event::Start {
                identity: Some(identity),
                cached: slot,
                ..
            }) = current.get_mut(start)
            {
                if *identity == expected_identity {
                    *slot = Some(cached);
                }
            }
            self.work.accumulate(self.sink.work);
        }
    }
}
impl Parser<'_> {
    pub(crate) fn advance_tree_cache(&mut self, allowance: &mut u64) -> bool {
        self.state
            .tree_cache
            .advance(self.source, &mut self.state.events, self.ids, allowance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{DocumentId, Revision};

    #[test]
    fn stale_requests_keep_later_identities_and_finite_work() {
        for padding in [0, 65] {
            assert_stale_requests_keep_grants(padding);
        }
    }

    fn assert_stale_requests_keep_grants(padding: usize) {
        let source = TextSnapshot::new(DocumentId(2), Revision(1), "aé")
            .unwrap()
            .append("βz")
            .unwrap();
        let mut events = Journal::default();
        // Start just before the 32-event page boundary. Both storage modes
        // must keep their event ranges when the live page is rewritten.
        let start = 31;
        events.extend((0..start).map(|_| Event::Tombstone));
        for identity in [Some(NodeId(20)), None] {
            events.push(Event::Start {
                kind: SyntaxKind::Document,
                flags: NodeFlags::NONE,
                identity,
                cached: None,
            });
        }
        for _ in 0..padding {
            events.push(Event::Tombstone);
        }
        events.push(Event::Token {
            kind: SyntaxKind::Unknown,
            range: source.full_range(),
            flags: TokenFlags::NONE,
        });
        events.push(Event::Finish);
        events.push(Event::Finish);

        let run = |discard, grant| {
            let mut cache = TreeCache {
                one_shot: discard,
                ..TreeCache::default()
            };
            let mut current = events.clone();
            cache.enqueue(&current, start, TextSize(0), NodeId(20));
            let Event::Start { identity, .. } = &mut current[start] else {
                panic!("root marker")
            };
            *identity = Some(NodeId(21));
            cache.enqueue(&current, start, TextSize(0), NodeId(21));
            let mut ids = IdGenerator::with_next(40, 100, 1);
            let mut charges = Vec::new();
            loop {
                let mut remaining = grant;
                let done = cache.advance(&source, &mut current, &mut ids, &mut remaining);
                charges.push(grant - remaining);
                if done {
                    break;
                }
            }
            let Event::Start {
                cached: Some(cached),
                ..
            } = &current[start]
            else {
                panic!("published root")
            };
            (
                charges,
                ids.next_node(),
                ids.next_token(),
                format!("{:?}", cached.node),
                cache.work,
            )
        };
        for grant in [1, 2, 37, u64::MAX] {
            let (old_grants, old_node, old_token, old_tree, old_work) = run(false, grant);
            let (new_grants, new_node, new_token, new_tree, new_work) = run(true, grant);
            assert_eq!(
                (old_grants, old_node, old_token, old_tree),
                (new_grants, new_node, new_token, new_tree)
            );
            assert_eq!((new_node, new_token), (42, 102));
            assert_eq!(old_work.events, new_work.events);
            assert_eq!(old_work.token_bytes, new_work.token_bytes);
            assert_eq!(old_work.nodes, new_work.nodes);
            assert_eq!(old_work.reused, new_work.reused);
            assert!(new_work.child_storage_nodes <= old_work.child_storage_nodes);
        }
    }

    #[test]
    fn batches_token_bytes_without_changing_partial_progress_or_hashes() {
        let first = TextSnapshot::new(DocumentId(1), Revision(1), "aé").unwrap();
        let source = first.append("βz").unwrap();
        assert_eq!(source.piece_count(), 2);
        let mut events = Journal::default();
        events.push(Event::Start {
            kind: SyntaxKind::Document,
            flags: NodeFlags::NONE,
            identity: Some(NodeId(20)),
            cached: None,
        });
        // Traverse two journal pages, then hash a token across both source
        // pieces. Small work grants stop inside both multibyte scalars.
        for _ in 0..65 {
            events.push(Event::Tombstone);
        }
        events.push(Event::Token {
            kind: SyntaxKind::Unknown,
            range: source.full_range(),
            flags: TokenFlags::NONE,
        });
        events.push(Event::Finish);
        let mut baseline_ids = IdGenerator::new();
        let mut baseline = Sink::new(0, events.len(), true);
        let mut unlimited = u64::MAX;
        assert!(
            baseline
                .advance(&source, &events, &events, &mut baseline_ids, &mut unlimited)
                .unwrap()
        );
        let reference = baseline.finish();
        let expected_work = events.len() as u64 + u64::from(source.byte_len().0) + 1;
        assert_eq!(u64::MAX - unlimited, expected_work);

        for quantum in [1, 2, 4, 7] {
            let mut ids = IdGenerator::new();
            let mut sink = Sink::new(0, events.len(), true);
            let mut charged = 0;
            loop {
                let mut allowance = quantum;
                let complete = sink
                    .advance(&source, &events, &events, &mut ids, &mut allowance)
                    .unwrap();
                charged += quantum - allowance;
                assert!(charged <= expected_work);
                if complete {
                    break;
                }
                assert_eq!(allowance, 0);
            }
            assert_eq!(sink.work.token_bytes, u64::from(source.byte_len().0));
            assert_eq!(sink.work.events, events.len() as u64);
            assert_eq!(charged, expected_work);
            let resumed = sink.finish();
            assert_eq!(resumed.event_nodes, reference.event_nodes);
            assert_eq!(resumed.root.id, reference.root.id);
            assert_eq!(resumed.root.structural_hash, reference.root.structural_hash);
            assert_eq!(resumed.root.text_len, source.byte_len());
            let GreenElement::Token(token) = resumed.root.children.iter().next().unwrap() else {
                panic!("the full source must be represented by one token")
            };
            assert_eq!(token.text_hash, crate::document::green::text_hash("aéβz"));
        }
    }
}
