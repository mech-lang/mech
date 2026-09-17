use alloc::vec::Vec;

use crate::document::{ParserContextId, RuleId};

include!(concat!(env!("OUT_DIR"), "/canonical_rules.rs"));

const fn stable_hash(name: &str) -> u32 {
  let bytes = name.as_bytes();
  let mut hash = 0x811c_9dc5_u32;
  let mut index = 0;
  while index < bytes.len() {
    hash ^= bytes[index] as u32;
    hash = hash.wrapping_mul(0x0100_0193);
    index += 1;
  }
  hash
}

pub const fn parser_context_id(name: &str) -> ParserContextId {
  ParserContextId(stable_hash(name))
}

pub fn canonical_rule_id(name: &str) -> Option<RuleId> {
  CANONICAL_RULES
    .binary_search_by_key(&name, |(candidate, _)| *candidate)
    .ok()
    .map(|index| CANONICAL_RULES[index].1)
}

pub fn canonical_rule_name(rule: RuleId) -> Option<&'static str> {
  CANONICAL_RULES
    .iter()
    .find_map(|(name, candidate)| (*candidate == rule).then_some(*name))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RuleFrame {
  context: ParserContextId,
  canonical: Option<RuleId>,
}

#[derive(Clone, Debug, Default)]
pub struct RuleStack {
  rules: Vec<RuleFrame>,
}

impl RuleStack {
  pub fn push(&mut self, context: ParserContextId, canonical: Option<RuleId>) {
    self.rules.push(RuleFrame { context, canonical });
  }

  pub fn current_rule(&self) -> Option<RuleId> {
    self.rules.iter().rev().find_map(|frame| frame.canonical)
  }

  pub fn current_context(&self) -> Option<ParserContextId> {
    self.rules.last().map(|frame| frame.context)
  }

  pub fn len(&self) -> usize {
    self.rules.len()
  }

  pub fn truncate(&mut self, len: usize) {
    self.rules.truncate(len);
  }
}

#[cfg(test)]
mod tests {
  use alloc::collections::{BTreeMap, BTreeSet};

  use super::*;

  #[test]
  fn generated_canonical_inventory_is_exact_and_collision_free() {
    assert_eq!(CANONICAL_RULE_COUNT, 540);
    assert_eq!(CANONICAL_RULES.len(), 540);

    let mut names = BTreeSet::new();
    let mut hashes = BTreeMap::new();
    for (name, id) in CANONICAL_RULES {
      assert!(names.insert(*name), "duplicate canonical rule {name}");
      if let Some(previous) = hashes.insert(*id, *name) {
        panic!("RuleId collision between {previous} and {name}: {id}");
      }
      assert_eq!(canonical_rule_id(name), Some(*id));
      assert_eq!(canonical_rule_name(*id), Some(*name));
    }
  }

  #[test]
  fn internal_contexts_are_not_canonical_rules() {
    let context = parser_context_id("prototype-additive-expression");
    assert!(canonical_rule_id("prototype-additive-expression").is_none());
    assert_ne!(context.0, 0);
  }
}
