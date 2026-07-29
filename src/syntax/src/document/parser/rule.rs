use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::vec::Vec;

use crate::document::RuleId;

pub const fn rule_id(name: &str) -> RuleId {
  let bytes = name.as_bytes();
  let mut hash = 0x811c_9dc5_u32;
  let mut index = 0;
  while index < bytes.len() {
    hash ^= bytes[index] as u32;
    hash = hash.wrapping_mul(0x0100_0193);
    index += 1;
  }
  RuleId(hash)
}

#[derive(Clone, Debug, Default)]
pub struct RuleStack {
  rules: Vec<RuleId>,
}

impl RuleStack {
  pub fn push(&mut self, rule: RuleId) {
    self.rules.push(rule);
  }

  pub fn pop(&mut self) {
    self.rules.pop();
  }

  pub fn current(&self) -> Option<RuleId> {
    self.rules.last().copied()
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
  use super::*;

  #[test]
  fn canonical_specification_rule_hashes_do_not_collide() {
    let specification = include_str!("../../../../../docs/design/specification.mec");
    let mut names = BTreeSet::new();
    for line in specification.lines() {
      let Some((left, _)) = line.split_once(":=") else {
        continue;
      };
      let candidate = left.trim();
      if !candidate.is_empty()
        && candidate
          .chars()
          .all(|character| character.is_ascii_alphanumeric() || character == '-')
      {
        names.insert(String::from(candidate));
      }
    }
    assert!(names.len() > 100, "canonical grammar inventory was not found");

    let mut hashes = BTreeMap::new();
    for name in names {
      let id = rule_id(&name);
      if let Some(previous) = hashes.insert(id, name.clone()) {
        panic!("RuleId collision between {previous} and {name}: {id}");
      }
    }
  }
}
