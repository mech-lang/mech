use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use mech_syntax::document::RuleId;
use mech_syntax::document::parser::{
  CANONICAL_RULE_COUNT, CANONICAL_RULES,
};

const EXPECTED_RULES: usize = 540;
const REGENERATE: &str =
  "python3 scripts/generate-canonical-rule-registry.py";

fn repository_root() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn stable_hash(name: &str) -> RuleId {
  let mut hash = 0x811c_9dc5_u32;
  for byte in name.bytes() {
    hash ^= u32::from(byte);
    hash = hash.wrapping_mul(0x0100_0193);
  }
  RuleId(hash)
}

#[test]
fn checked_in_registry_exactly_matches_phase_0_inventory() {
  let inventory = fs::read_to_string(
    repository_root().join("docs/design/grammar-audit/productions.tsv"),
  )
  .expect("read Phase 0 productions.tsv");
  let mut lines = inventory.lines();
  let header = lines.next().expect("productions.tsv header");
  let columns = header.split('\t').collect::<Vec<_>>();
  let grammar_name = columns
    .iter()
    .position(|column| *column == "grammar-name")
    .expect("grammar-name column");
  let specification = columns
    .iter()
    .position(|column| *column == "spec-location")
    .expect("spec-location column");

  let mut canonical_rows = 0_usize;
  let mut names = BTreeSet::new();
  for (index, line) in lines.enumerate() {
    let fields = line.split('\t').collect::<Vec<_>>();
    assert_eq!(
      fields.len(),
      columns.len(),
      "invalid productions.tsv row {}",
      index + 2
    );
    if fields[specification].starts_with("docs/design/specification.mec::") {
      canonical_rows += 1;
      assert!(
        names.insert(fields[grammar_name].to_owned()),
        "duplicate canonical row for {:?}",
        fields[grammar_name]
      );
    }
  }
  assert_eq!(canonical_rows, EXPECTED_RULES);
  assert_eq!(names.len(), EXPECTED_RULES);

  let expected = names
    .iter()
    .map(|name| (name.as_str(), stable_hash(name)))
    .collect::<Vec<_>>();
  assert_eq!(CANONICAL_RULE_COUNT, EXPECTED_RULES);
  assert_eq!(
    CANONICAL_RULES,
    expected.as_slice(),
    "checked-in RuleId registry differs from Phase 0; run `{REGENERATE}`"
  );

  let mut hashes = BTreeMap::new();
  for (name, rule) in CANONICAL_RULES {
    if let Some(previous) = hashes.insert(*rule, *name) {
      panic!(
        "RuleId collision between {previous} and {name}: {rule}; \
         run `{REGENERATE}`"
      );
    }
  }
}
