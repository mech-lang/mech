use alloc::sync::Arc;
use alloc::vec::Vec;
use core::fmt;

use super::edit::{TextRange, TextSize};
use super::flags::{NodeFlags, TokenFlags};
use super::ids::{NodeId, TokenId};
use super::kind::SyntaxKind;
use super::source::TextSnapshot;

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

#[derive(Clone, Debug)]
pub struct GreenNode {
  pub id: NodeId,
  pub kind: SyntaxKind,
  pub text_len: TextSize,
  pub children: Arc<[GreenElement]>,
  pub flags: NodeFlags,
  pub structural_hash: u64,
}

#[derive(Clone, Debug)]
pub enum GreenElement {
  Node(Arc<GreenNode>),
  Token(GreenToken),
}

impl GreenElement {
  pub fn text_len(&self) -> TextSize {
    match self {
      Self::Node(node) => node.text_len,
      Self::Token(token) => token.text_len,
    }
  }

  pub fn kind(&self) -> SyntaxKind {
    match self {
      Self::Node(node) => node.kind,
      Self::Token(token) => token.kind,
    }
  }
}

#[derive(Clone, Copy, Debug)]
pub struct GreenToken {
  pub id: TokenId,
  pub kind: SyntaxKind,
  pub text_len: TextSize,
  pub flags: TokenFlags,
  pub text_hash: u64,
}

impl GreenToken {
  pub fn is_synthetic(self) -> bool {
    self.flags.contains(TokenFlags::SYNTHETIC)
  }
}

pub fn text_hash(text: &str) -> u64 {
  let mut hash = FNV_OFFSET;
  for byte in text.as_bytes() {
    hash ^= u64::from(*byte);
    hash = hash.wrapping_mul(FNV_PRIME);
  }
  hash
}

pub(crate) fn hash_node(kind: SyntaxKind, children: &[GreenElement]) -> u64 {
  let mut hash = hash_u64(FNV_OFFSET, kind as u64);
  for child in children {
    hash = hash_u64(hash, child.kind() as u64);
    hash = hash_u64(hash, u64::from(child.text_len().0));
    hash = hash_u64(
      hash,
      match child {
        GreenElement::Node(node) => node.structural_hash,
        GreenElement::Token(token) => token.text_hash,
      },
    );
  }
  hash
}

fn hash_u64(mut hash: u64, value: u64) -> u64 {
  for byte in value.to_le_bytes() {
    hash ^= u64::from(byte);
    hash = hash.wrapping_mul(FNV_PRIME);
  }
  hash
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TreeInvariantError {
  RootLength {
    tree: TextSize,
    source: TextSize,
  },
  SyntheticTokenHasWidth {
    token: TokenId,
    len: TextSize,
  },
  TokenOutsideSource {
    token: TokenId,
    range: TextRange,
  },
  TokenTextHash {
    token: TokenId,
    range: TextRange,
  },
  NonTokenKind {
    token: TokenId,
    kind: SyntaxKind,
  },
}

impl fmt::Display for TreeInvariantError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::RootLength { tree, source } => {
        write!(f, "tree length {} differs from source length {}", tree.0, source.0)
      }
      Self::SyntheticTokenHasWidth { token, len } => {
        write!(f, "synthetic token {} has width {}", token.0, len.0)
      }
      Self::TokenOutsideSource { token, range } => write!(
        f,
        "token {} range {}..{} is outside the source",
        token.0, range.start.0, range.end.0
      ),
      Self::TokenTextHash { token, range } => write!(
        f,
        "token {} does not match source bytes {}..{}",
        token.0, range.start.0, range.end.0
      ),
      Self::NonTokenKind { token, kind } => {
        write!(f, "token {} uses node kind {kind:?}", token.0)
      }
    }
  }
}

pub fn validate_lossless(
  root: &GreenNode,
  source: &TextSnapshot,
) -> Result<(), TreeInvariantError> {
  if root.text_len != source.byte_len() {
    return Err(TreeInvariantError::RootLength {
      tree: root.text_len,
      source: source.byte_len(),
    });
  }
  let mut offset = TextSize::ZERO;
  validate_node(root, source, &mut offset)?;
  if offset != source.byte_len() {
    return Err(TreeInvariantError::RootLength {
      tree: offset,
      source: source.byte_len(),
    });
  }
  Ok(())
}

pub fn reconstruct_source(
  root: &GreenNode,
  source: &TextSnapshot,
) -> Result<alloc::string::String, TreeInvariantError> {
  validate_lossless(root, source)?;
  let mut output = alloc::string::String::with_capacity(source.byte_len().to_usize());
  let mut offset = TextSize::ZERO;
  collect_text(root, source, &mut offset, &mut output);
  Ok(output)
}

fn validate_node(
  node: &GreenNode,
  source: &TextSnapshot,
  offset: &mut TextSize,
) -> Result<(), TreeInvariantError> {
  for child in node.children.iter() {
    match child {
      GreenElement::Node(node) => validate_node(node, source, offset)?,
      GreenElement::Token(token) => {
        if !token.kind.is_token() {
          return Err(TreeInvariantError::NonTokenKind {
            token: token.id,
            kind: token.kind,
          });
        }
        if token.is_synthetic() {
          if token.text_len.0 != 0 {
            return Err(TreeInvariantError::SyntheticTokenHasWidth {
              token: token.id,
              len: token.text_len,
            });
          }
          continue;
        }
        let range = TextRange::at(*offset, token.text_len);
        if range.end.0 > source.byte_len().0 {
          return Err(TreeInvariantError::TokenOutsideSource {
            token: token.id,
            range,
          });
        }
        let text = source
          .text(range)
          .map_err(|_| TreeInvariantError::TokenOutsideSource {
            token: token.id,
            range,
          })?;
        if text_hash(&text) != token.text_hash {
          return Err(TreeInvariantError::TokenTextHash {
            token: token.id,
            range,
          });
        }
        *offset += token.text_len;
      }
    }
  }
  Ok(())
}

fn collect_text(
  node: &GreenNode,
  source: &TextSnapshot,
  offset: &mut TextSize,
  output: &mut alloc::string::String,
) {
  for child in node.children.iter() {
    match child {
      GreenElement::Node(node) => collect_text(node, source, offset, output),
      GreenElement::Token(token) => {
        if !token.is_synthetic() {
          let range = TextRange::at(*offset, token.text_len);
          source.for_each_slice(range, |slice| output.push_str(slice));
          *offset += token.text_len;
        }
      }
    }
  }
}

pub(crate) fn propagated_flags(
  kind: SyntaxKind,
  explicit: NodeFlags,
  children: &[GreenElement],
) -> NodeFlags {
  let mut flags = explicit;
  if kind == SyntaxKind::Error {
    flags |= NodeFlags::ERROR;
  }
  if kind == SyntaxKind::Missing {
    flags |= NodeFlags::MISSING;
  }
  for child in children {
    match child {
      GreenElement::Node(node) => {
        if node.flags.intersects(NodeFlags::ERROR | NodeFlags::CONTAINS_ERROR) {
          flags |= NodeFlags::CONTAINS_ERROR;
        }
        if node
          .flags
          .intersects(NodeFlags::MISSING | NodeFlags::CONTAINS_MISSING)
        {
          flags |= NodeFlags::CONTAINS_MISSING;
        }
      }
      GreenElement::Token(token) => {
        if token.flags.contains(TokenFlags::ERROR) {
          flags |= NodeFlags::CONTAINS_ERROR;
        }
        if token.flags.contains(TokenFlags::MISSING) {
          flags |= NodeFlags::CONTAINS_MISSING;
        }
      }
    }
  }
  flags
}

pub(crate) fn child_text_len(children: &[GreenElement]) -> TextSize {
  children
    .iter()
    .fold(TextSize::ZERO, |total, child| total + child.text_len())
}

pub(crate) fn collect_tokens<'a>(node: &'a GreenNode, output: &mut Vec<&'a GreenToken>) {
  for child in node.children.iter() {
    match child {
      GreenElement::Node(node) => collect_tokens(node, output),
      GreenElement::Token(token) => output.push(token),
    }
  }
}
