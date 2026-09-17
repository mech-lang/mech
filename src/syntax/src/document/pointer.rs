use super::ids::{DocumentId, NodeId};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct SyntaxPtr {
  pub document: DocumentId,
  pub node: NodeId,
}
