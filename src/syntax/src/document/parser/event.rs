use alloc::collections::BTreeMap;
use alloc::sync::Arc;

use crate::document::{
    BuildError, GreenNode, IdGenerator, NodeFlags, NodeId, SyntaxKind, TextRange, TextSnapshot,
    TokenFlags,
};

#[derive(Clone)]
pub enum Event {
    Start {
        identity: Option<NodeId>,
        cached: Option<super::tree_cache::CachedNode>,
        kind: SyntaxKind,
        flags: NodeFlags,
    },
    Token {
        kind: SyntaxKind,
        range: TextRange,
        flags: TokenFlags,
    },
    Reuse {
        node: Arc<GreenNode>,
    },
    Finish,
    Tombstone,
}

pub struct SinkResult {
    pub root: Arc<GreenNode>,
    pub event_nodes: BTreeMap<usize, NodeId>,
}

pub fn sink(
    events: &super::journal::Journal<Event>,
    source: &TextSnapshot,
    ids: &mut IdGenerator,
) -> Result<SinkResult, BuildError> {
    let mut sink = super::tree_cache::Sink::new(events.clone(), 0, events.len());
    loop {
        let mut allowance = u64::MAX;
        if sink.advance(source, events, ids, &mut allowance)? {
            return Ok(sink.finish());
        }
    }
}

// Debugging the grammar tape excludes materialization scheduling and session
// identities. Those have dedicated view/identity APIs and qualification.
impl core::fmt::Debug for Event {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Start { kind, flags, .. } => f
                .debug_struct("Start")
                .field("kind", kind)
                .field("flags", flags)
                .finish(),
            Self::Token { kind, range, flags } => f
                .debug_struct("Token")
                .field("kind", kind)
                .field("range", range)
                .field("flags", flags)
                .finish(),
            Self::Reuse { node } => f
                .debug_struct("Reuse")
                .field("kind", &node.kind)
                .field("flags", &node.flags)
                .field("text_len", &node.text_len)
                .field("structural_hash", &node.structural_hash)
                .finish(),
            Self::Finish => f.write_str("Finish"),
            Self::Tombstone => f.write_str("Tombstone"),
        }
    }
}
