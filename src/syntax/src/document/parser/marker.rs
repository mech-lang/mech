use crate::document::{NodeFlags, SyntaxKind};

use super::Parser;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Marker {
  pub(crate) position: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompletedMarker {
  pub(crate) position: usize,
  pub(crate) kind: SyntaxKind,
}

impl Marker {
  pub fn complete(
    self,
    parser: &mut Parser<'_>,
    kind: SyntaxKind,
  ) -> CompletedMarker {
    self.complete_with_flags(parser, kind, NodeFlags::NONE)
  }

  pub fn complete_with_flags(
    self,
    parser: &mut Parser<'_>,
    kind: SyntaxKind,
    flags: NodeFlags,
  ) -> CompletedMarker {
    parser.complete_marker(self, kind, flags)
  }

  pub fn abandon(self, parser: &mut Parser<'_>) {
    parser.abandon_marker(self);
  }
}

impl CompletedMarker {
  pub fn position(self) -> usize {
    self.position
  }

  pub fn kind(self) -> SyntaxKind {
    self.kind
  }
}
