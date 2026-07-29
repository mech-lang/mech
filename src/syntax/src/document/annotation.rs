use alloc::collections::BTreeMap;

use super::ids::{DiagnosticId, NodeId, Revision};
use super::index::NodeIndex;

#[derive(Clone, Debug)]
pub struct NodeMap<T> {
  pub revision: Revision,
  values: BTreeMap<NodeId, T>,
}

impl<T> NodeMap<T> {
  pub fn new(revision: Revision) -> Self {
    Self {
      revision,
      values: BTreeMap::new(),
    }
  }

  pub fn insert(&mut self, node: NodeId, value: T) -> Option<T> {
    self.values.insert(node, value)
  }

  pub fn get(&self, node: NodeId) -> Option<&T> {
    self.values.get(&node)
  }

  pub fn get_mut(&mut self, node: NodeId) -> Option<&mut T> {
    self.values.get_mut(&node)
  }

  pub fn remove(&mut self, node: NodeId) -> Option<T> {
    self.values.remove(&node)
  }

  pub fn contains_key(&self, node: NodeId) -> bool {
    self.values.contains_key(&node)
  }

  pub fn len(&self) -> usize {
    self.values.len()
  }

  pub fn is_empty(&self) -> bool {
    self.values.is_empty()
  }

  pub fn iter(&self) -> impl Iterator<Item = (NodeId, &T)> {
    self.values.iter().map(|(id, value)| (*id, value))
  }

  pub fn retain_reused(mut self, revision: Revision, index: &NodeIndex) -> Self {
    self.values.retain(|id, _| index.contains_node(*id));
    self.revision = revision;
    self
  }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FactState<T> {
  Unknown,
  Partial(T),
  Complete(T),
  Invalid(DiagnosticId),
}
