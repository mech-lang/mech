use alloc::sync::Arc;
use alloc::vec::Vec;
use core::fmt;

use super::edit::TextSize;
use super::flags::{NodeFlags, TokenFlags};
use super::green::{
  GreenElement, GreenNode, GreenToken, child_text_len, hash_node, propagated_flags, text_hash,
};
use super::ids::IdGenerator;
use super::kind::SyntaxKind;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BuildError {
  NoOpenNode,
  UnclosedNodes(usize),
  MultipleRoots(usize),
  TokenKindExpected(SyntaxKind),
  TextTooLarge,
}

impl fmt::Display for BuildError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::NoOpenNode => f.write_str("tree builder has no open node"),
      Self::UnclosedNodes(count) => write!(f, "tree builder has {count} unclosed nodes"),
      Self::MultipleRoots(count) => write!(f, "tree builder produced {count} roots"),
      Self::TokenKindExpected(kind) => write!(f, "{kind:?} is not a token kind"),
      Self::TextTooLarge => f.write_str("token text exceeds the 32-bit text range"),
    }
  }
}

struct Frame {
  kind: SyntaxKind,
  flags: NodeFlags,
  children: Vec<GreenElement>,
}

pub struct GreenBuilder<'a> {
  ids: &'a mut IdGenerator,
  frames: Vec<Frame>,
  roots: Vec<Arc<GreenNode>>,
}

impl<'a> GreenBuilder<'a> {
  pub fn new(ids: &'a mut IdGenerator) -> Self {
    Self {
      ids,
      frames: Vec::new(),
      roots: Vec::new(),
    }
  }

  pub fn start_node(&mut self, kind: SyntaxKind) {
    self.start_node_with_flags(kind, NodeFlags::NONE);
  }

  pub fn start_node_with_flags(&mut self, kind: SyntaxKind, flags: NodeFlags) {
    self.frames.push(Frame {
      kind,
      flags,
      children: Vec::new(),
    });
  }

  pub fn token(&mut self, kind: SyntaxKind, text: &str) -> Result<(), BuildError> {
    self.token_with_flags(kind, text, TokenFlags::NONE)
  }

  pub fn token_with_flags(
    &mut self,
    kind: SyntaxKind,
    text: &str,
    flags: TokenFlags,
  ) -> Result<(), BuildError> {
    if !kind.is_token() {
      return Err(BuildError::TokenKindExpected(kind));
    }
    let text_len = TextSize::checked_from_usize(text.len()).map_err(|_| BuildError::TextTooLarge)?;
    let id = self.ids.token();
    self.push_element(GreenElement::Token(GreenToken {
      id,
      kind,
      text_len,
      flags,
      text_hash: text_hash(text),
    }))
  }

  pub fn missing_token(&mut self, expected: SyntaxKind) -> Result<(), BuildError> {
    if !expected.is_token() {
      return Err(BuildError::TokenKindExpected(expected));
    }
    let id = self.ids.token();
    self.push_element(GreenElement::Token(GreenToken {
      id,
      kind: expected,
      text_len: TextSize::ZERO,
      flags: TokenFlags::SYNTHETIC | TokenFlags::MISSING,
      text_hash: text_hash(""),
    }))
  }

  pub fn reuse_node(&mut self, node: Arc<GreenNode>) -> Result<(), BuildError> {
    self.push_element(GreenElement::Node(node))
  }

  pub fn finish_node(&mut self) -> Result<Arc<GreenNode>, BuildError> {
    let frame = self.frames.pop().ok_or(BuildError::NoOpenNode)?;
    let flags = propagated_flags(frame.kind, frame.flags, &frame.children);
    let node = Arc::new(GreenNode {
      id: self.ids.node(),
      kind: frame.kind,
      text_len: child_text_len(&frame.children),
      structural_hash: hash_node(frame.kind, &frame.children),
      children: frame.children.into(),
      flags,
    });
    if let Some(parent) = self.frames.last_mut() {
      parent.children.push(GreenElement::Node(node.clone()));
    } else {
      self.roots.push(node.clone());
    }
    Ok(node)
  }

  pub fn finish(self) -> Result<Arc<GreenNode>, BuildError> {
    if !self.frames.is_empty() {
      return Err(BuildError::UnclosedNodes(self.frames.len()));
    }
    if self.roots.len() != 1 {
      return Err(BuildError::MultipleRoots(self.roots.len()));
    }
    Ok(self.roots.into_iter().next().expect("one root was checked"))
  }

  fn push_element(&mut self, element: GreenElement) -> Result<(), BuildError> {
    let frame = self.frames.last_mut().ok_or(BuildError::NoOpenNode)?;
    frame.children.push(element);
    Ok(())
  }
}
