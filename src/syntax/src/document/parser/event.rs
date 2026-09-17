use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::document::{
  BuildError, GreenBuilder, GreenNode, IdGenerator, NodeFlags, NodeId, SyntaxKind, TextRange,
  TextSnapshot, TokenFlags,
};

#[derive(Clone, Debug)]
pub enum Event {
  Start {
    kind: SyntaxKind,
    flags: NodeFlags,
  },
  Token {
    kind: SyntaxKind,
    range: TextRange,
    flags: TokenFlags,
  },
  Finish,
  Tombstone,
}

pub struct SinkResult {
  pub root: Arc<GreenNode>,
  pub event_nodes: BTreeMap<usize, NodeId>,
}

pub fn sink(
  events: &[Event],
  source: &TextSnapshot,
  ids: &mut IdGenerator,
) -> Result<SinkResult, BuildError> {
  let mut builder = GreenBuilder::new(ids);
  let mut starts = Vec::new();
  let mut event_nodes = BTreeMap::new();
  for (index, event) in events.iter().enumerate() {
    match event {
      Event::Start { kind, flags } => {
        builder.start_node_with_flags(*kind, *flags);
        starts.push(index);
      }
      Event::Token { kind, range, flags } => {
        let text = source
          .text(*range)
          .map_err(|_| BuildError::TextTooLarge)?;
        builder.token_with_flags(*kind, &text, *flags)?;
      }
      Event::Finish => {
        let start = starts.pop().ok_or(BuildError::NoOpenNode)?;
        let node = builder.finish_node()?;
        event_nodes.insert(start, node.id);
      }
      Event::Tombstone => {}
    }
  }
  Ok(SinkResult {
    root: builder.finish()?,
    event_nodes,
  })
}
