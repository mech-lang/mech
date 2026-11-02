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
    events: Journal<Event>,
    at: usize,
    end: usize,
    frames: Vec<Frame>,
    token: Option<Token>,
    root: Option<Arc<GreenNode>>,
    event_nodes: BTreeMap<usize, NodeId>,
    pub work: CacheWork,
}
impl Sink {
    pub fn new(events: Journal<Event>, start: usize, end: usize) -> Self {
        Self {
            events,
            at: start,
            end,
            frames: Vec::new(),
            token: None,
            root: None,
            event_nodes: BTreeMap::new(),
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
        current: &Journal<Event>,
        ids: &mut IdGenerator,
        allowance: &mut u64,
    ) -> Result<bool, BuildError> {
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
                    let byte = source.byte_at(token.at).ok_or(BuildError::TextTooLarge)?;
                    token.hash = (token.hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
                    token.at += TextSize(1);
                    self.work.token_bytes += 1;
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
            let event = self
                .events
                .get(index)
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
                    let cached = cached.or_else(|| match current.get(index) {
                        Some(Event::Start {
                            identity: now,
                            cached,
                            ..
                        }) if identity.is_some() && *now == identity => cached.clone(),
                        _ => None,
                    });
                    if let Some(cached) = cached.filter(|cached| cached.end_event <= self.end) {
                        self.at = cached.end_event;
                        self.event_nodes.insert(index, cached.node.id);
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
                    self.event_nodes.insert(frame.start, node.id);
                    self.element(GreenElement::Node(node))?;
                }
                Event::Tombstone => {}
            }
        }
    }
    pub fn finish(self) -> SinkResult {
        SinkResult {
            root: self.root.expect("completed sink"),
            event_nodes: self.event_nodes,
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
                let sink = Sink::new(request.events.clone(), request.start, request.end);
                self.active = Some((request, sink));
            }
            let (request, sink) = self.active.as_mut().expect("active cache request");
            if !sink
                .advance(source, current, ids, allowance)
                .expect("completed canonical marker must materialize")
            {
                return false;
            }
            let cached = CachedNode {
                node: sink.root.as_ref().expect("materialized root").clone(),
                range: TextRange::at(request.offset, sink.root.as_ref().expect("root").text_len),
                end_event: request.end,
            };
            if let Some(Event::Start {
                identity: Some(identity),
                cached: slot,
                ..
            }) = current.get_mut(request.start)
            {
                if *identity == request.identity {
                    *slot = Some(cached);
                }
            }
            let (_, sink) = self.active.take().expect("completed cache request");
            self.work.events += sink.work.events;
            self.work.token_bytes += sink.work.token_bytes;
            self.work.child_storage_nodes += sink.work.child_storage_nodes;
            self.work.nodes += sink.work.nodes;
            self.work.reused += sink.work.reused;
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
