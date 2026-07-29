use alloc::collections::BTreeMap;

use super::edit::{TextRange, TextSize};
use super::flags::{NodeFlags, TokenFlags};
use super::green::{GreenElement, GreenNode};
use super::ids::{NodeId, SyntaxElementId, TokenId};
use super::kind::SyntaxKind;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NodeRecord {
  pub kind: SyntaxKind,
  pub range: TextRange,
  pub parent: Option<NodeId>,
  pub flags: NodeFlags,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TokenRecord {
  pub kind: SyntaxKind,
  pub range: TextRange,
  pub parent: NodeId,
  pub flags: TokenFlags,
}

#[derive(Clone, Debug, Default)]
pub struct NodeIndex {
  nodes: BTreeMap<NodeId, NodeRecord>,
  tokens: BTreeMap<TokenId, TokenRecord>,
}

impl NodeIndex {
  pub fn build(root: &GreenNode) -> Self {
    let mut index = Self::default();
    index.index_node(root, None, TextSize::ZERO);
    index
  }

  pub fn node(&self, id: NodeId) -> Option<&NodeRecord> {
    self.nodes.get(&id)
  }

  pub fn token(&self, id: TokenId) -> Option<&TokenRecord> {
    self.tokens.get(&id)
  }

  pub fn range(&self, id: SyntaxElementId) -> Option<TextRange> {
    match id {
      SyntaxElementId::Node(id) => self.node(id).map(|record| record.range),
      SyntaxElementId::Token(id) => self.token(id).map(|record| record.range),
    }
  }

  pub fn parent(&self, id: SyntaxElementId) -> Option<NodeId> {
    match id {
      SyntaxElementId::Node(id) => self.node(id).and_then(|record| record.parent),
      SyntaxElementId::Token(id) => self.token(id).map(|record| record.parent),
    }
  }

  pub fn node_count(&self) -> usize {
    self.nodes.len()
  }

  pub fn token_count(&self) -> usize {
    self.tokens.len()
  }

  pub fn contains_node(&self, id: NodeId) -> bool {
    self.nodes.contains_key(&id)
  }

  fn index_node(
    &mut self,
    node: &GreenNode,
    parent: Option<NodeId>,
    start: TextSize,
  ) {
    self.nodes.insert(
      node.id,
      NodeRecord {
        kind: node.kind,
        range: TextRange::at(start, node.text_len),
        parent,
        flags: node.flags,
      },
    );
    let mut offset = start;
    for child in node.children.iter() {
      match child {
        GreenElement::Node(child) => {
          self.index_node(child, Some(node.id), offset);
          offset += child.text_len;
        }
        GreenElement::Token(token) => {
          self.tokens.insert(
            token.id,
            TokenRecord {
              kind: token.kind,
              range: TextRange::at(offset, token.text_len),
              parent: node.id,
              flags: token.flags,
            },
          );
          offset += token.text_len;
        }
      }
    }
  }
}
