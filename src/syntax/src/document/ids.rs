use core::fmt;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

macro_rules! id_type {
  ($name:ident) => {
    #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
    pub struct $name(pub u64);

    impl fmt::Display for $name {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
      }
    }
  };
}

id_type!(DocumentId);
id_type!(Revision);
id_type!(NodeId);
id_type!(TokenId);
id_type!(DiagnosticId);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct RuleId(pub u32);

impl fmt::Display for RuleId {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{:08x}", self.0)
  }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub enum SyntaxElementId {
  Node(NodeId),
  Token(TokenId),
}

#[derive(Clone, Debug)]
pub struct IdGenerator {
  next_node: u64,
  next_token: u64,
  next_diagnostic: u64,
}

impl Default for IdGenerator {
  fn default() -> Self {
    Self::new()
  }
}

impl IdGenerator {
  pub const fn new() -> Self {
    Self {
      next_node: 1,
      next_token: 1,
      next_diagnostic: 1,
    }
  }

  pub const fn with_next(next_node: u64, next_token: u64, next_diagnostic: u64) -> Self {
    Self {
      next_node,
      next_token,
      next_diagnostic,
    }
  }

  pub fn node(&mut self) -> NodeId {
    let id = NodeId(self.next_node);
    self.next_node = self.next_node.saturating_add(1);
    id
  }

  pub fn token(&mut self) -> TokenId {
    let id = TokenId(self.next_token);
    self.next_token = self.next_token.saturating_add(1);
    id
  }

  pub fn diagnostic(&mut self) -> DiagnosticId {
    let id = DiagnosticId(self.next_diagnostic);
    self.next_diagnostic = self.next_diagnostic.saturating_add(1);
    id
  }

  pub const fn next_node(&self) -> u64 {
    self.next_node
  }

  pub const fn next_token(&self) -> u64 {
    self.next_token
  }

  pub const fn next_diagnostic(&self) -> u64 {
    self.next_diagnostic
  }
}
