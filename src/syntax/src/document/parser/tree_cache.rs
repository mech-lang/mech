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
    events: Journal<Event>,
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
pub(crate) struct Sink {
    at: usize,
    end: usize,
    frames: Vec<Frame>,
    token: Option<Token>,
    root: Option<Arc<GreenNode>>,
    event_nodes: Option<BTreeMap<usize, NodeId>>,
    pub work: CacheWork,
}
impl Sink {
    pub fn new(start: usize, end: usize, collect_event_nodes: bool) -> Self {
        Self {
            at: start,
            end,
            frames: Vec::new(),
            token: None,
            root: None,
            event_nodes: collect_event_nodes.then(BTreeMap::new),
            work: CacheWork::default(),
        }
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
    pub fn advance(
        &mut self,
        source: &TextSnapshot,
        events: &Journal<Event>,
        current: &Journal<Event>,
        ids: &mut IdGenerator,
        allowance: &mut u64,
    ) -> Result<bool, BuildError> {
        let (mut page_start, mut page): (usize, &[Event]) = (0, &[]);
        loop {
            if self.at >= self.end && self.token.is_none() {
                if !self.frames.is_empty() {
                    return Err(BuildError::UnclosedNodes(self.frames.len()));
                }
                if self.root.is_none() {
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
                    let chunk = source.chunk_at(token.at).ok_or(BuildError::TextTooLarge)?;
                    let available = (token.range.end.min(chunk.range.end) - token.at).to_usize();
                    // The first byte was charged by the loop. Stop at the
                    // same byte even when a budget ends within a UTF-8 scalar.
                    let count = (available as u64).min(*allowance + 1) as usize;
                    let start = (token.at - chunk.range.start).to_usize();
                    for &byte in &chunk.text.as_bytes()[start..start + count] {
                        token.hash =
                            (token.hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
                    }
                    token.at += TextSize(count as u32);
                    *allowance -= count as u64 - 1;
                    self.work.token_bytes += count as u64;
                    self.token = Some(token);
                } else {
                    self.element(GreenElement::Token(GreenToken {
                        id: ids.token(),
                        kind: token.kind,
                        text_len: token.range.len(),
                        flags: token.flags,
                        text_hash: token.hash,
                    }))?;
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
                        if identity.is_none() || core::ptr::eq(events, current) {
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
                        self.element(GreenElement::Node(cached.node))?;
                    } else {
                        self.frames.push(Frame {
                            kind,
                            flags,
                            identity,
                            start: index,
                            children: GreenChildren::for_kind(kind),
                        });
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
                    self.element(GreenElement::Node(node))?;
                }
                Event::Finish => {
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
    active: Option<(Request, Sink)>,
    pub work: CacheWork,
    pub peak_pending: usize,
}
impl TreeCache {
    pub fn enqueue(
        &mut self,
        events: &Journal<Event>,
        start: usize,
        offset: TextSize,
        identity: NodeId,
    ) {
        self.queue.push_back(Request {
            start,
            end: events.len(),
            offset,
            identity,
            events: events.clone(),
        });
        self.peak_pending = self
            .peak_pending
            .max(self.queue.len() + usize::from(self.active.is_some()));
    }
    pub fn total_work(&self) -> CacheWork {
        let mut work = self.work;
        if let Some((_, sink)) = &self.active {
            work.events += sink.work.events;
            work.token_bytes += sink.work.token_bytes;
            work.child_storage_nodes += sink.work.child_storage_nodes;
            work.nodes += sink.work.nodes;
            work.reused += sink.work.reused;
        }
        work
    }
    pub fn pending(&self) -> bool {
        self.active.is_some() || !self.queue.is_empty()
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
                // A child owner can reject an alternative before yielding to
                // this cache. Its retired marker must never be materialized.
                if request.end > current.len()
                    || !matches!(current.get(request.start),
                        Some(Event::Start { identity: Some(identity), .. })
                            if *identity == request.identity)
                {
                    continue;
                }
                let sink = Sink::new(request.start, request.end, false);
                self.active = Some((request, sink));
            }
            let (request, sink) = self.active.as_mut().expect("active cache request");
            if !sink
                .advance(source, &request.events, current, ids, allowance)
                .expect("completed canonical marker must materialize")
            {
                return false;
            }
            let cached = CachedNode {
                node: sink.root.as_ref().expect("materialized root").clone(),
                range: TextRange::at(request.offset, sink.root.as_ref().expect("root").text_len),
                end_event: request.end,
            };
            let (request, sink) = self.active.take().expect("completed cache request");
            let start = request.start;
            let expected_identity = request.identity;
            let work = sink.work;
            // Release the request's snapshot before publishing the new cache
            // entry, so the completed request cannot force path copying.
            drop(request);
            drop(sink);
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
            self.work.events += work.events;
            self.work.token_bytes += work.token_bytes;
            self.work.child_storage_nodes += work.child_storage_nodes;
            self.work.nodes += work.nodes;
            self.work.reused += work.reused;
        }
    }
}
impl Parser<'_> {
    pub(crate) fn advance_tree_cache(&mut self, allowance: &mut u64) -> bool {
        let mut cache = core::mem::take(&mut self.state.tree_cache);
        let complete = cache.advance(self.source, &mut self.state.events, self.ids, allowance);
        self.state.tree_cache = cache;
        complete
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{DocumentId, Revision};

    #[test]
    fn rejected_marker_requests_are_discarded_before_materialization() {
        let source = TextSnapshot::new(DocumentId(829), Revision(0), "a").unwrap();
        let mut ids = IdGenerator::new();
        let mut parser = Parser::new(
            &source,
            LexicalMode::CanonicalSourceFragment,
            ParseConfig::default(),
            &mut ids,
        );
        let checkpoint = parser.checkpoint();
        let rejected = parser.start();
        parser.bump_bytes_token(1, SyntaxKind::Alpha).unwrap();
        rejected.complete(&mut parser, SyntaxKind::Identifier);
        parser.rewind(checkpoint);
        let retained = parser.start();
        parser.bump_bytes_token(1, SyntaxKind::Alpha).unwrap();
        retained.complete(&mut parser, SyntaxKind::Identifier);
        let mut allowance = u64::MAX;
        assert!(parser.advance_tree_cache(&mut allowance));
        assert_eq!(parser.state.tree_cache.total_work().nodes, 1);
        let Event::Start {
            cached: Some(cached),
            ..
        } = &parser.state.events[0]
        else {
            panic!("the retained marker must be available to streaming views");
        };
        crate::document::validate_lossless(&cached.node, &source).unwrap();
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
