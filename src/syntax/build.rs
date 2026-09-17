use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;

const EXPECTED_CANONICAL_RULES: usize = 540;
const INVENTORY: &str = "../../docs/design/grammar-audit/productions.tsv";

fn rule_hash(name: &str) -> u32 {
  let mut hash = 0x811c_9dc5_u32;
  for byte in name.bytes() {
    hash ^= u32::from(byte);
    hash = hash.wrapping_mul(0x0100_0193);
  }
  hash
}

fn main() {
  println!("cargo:rerun-if-changed={INVENTORY}");
  let inventory = fs::read_to_string(INVENTORY)
    .unwrap_or_else(|error| panic!("failed to read {INVENTORY}: {error}"));
  let mut lines = inventory.lines();
  let header = lines.next().expect("grammar inventory must have a header");
  let columns = header.split('\t').collect::<Vec<_>>();
  let grammar_name = columns
    .iter()
    .position(|column| *column == "grammar-name")
    .expect("grammar inventory must contain grammar-name");
  let specification = columns
    .iter()
    .position(|column| *column == "spec-location")
    .expect("grammar inventory must contain spec-location");

  let mut rules = BTreeMap::new();
  let mut hashes = BTreeMap::new();
  for (index, line) in lines.enumerate() {
    let fields = line.split('\t').collect::<Vec<_>>();
    assert_eq!(
      fields.len(),
      columns.len(),
      "invalid productions.tsv row {}",
      index + 2
    );
    if !fields[specification].starts_with("docs/design/specification.mec::") {
      continue;
    }
    let name = fields[grammar_name];
    let id = rule_hash(name);
    if let Some(previous) = rules.insert(name.to_owned(), id) {
      assert_eq!(previous, id, "canonical rule {name} has inconsistent IDs");
    }
    if let Some(previous) = hashes.insert(id, name.to_owned()) {
      assert_eq!(
        previous, name,
        "RuleId collision between {previous} and {name}: {id:08x}"
      );
    }
  }
  assert_eq!(
    rules.len(),
    EXPECTED_CANONICAL_RULES,
    "canonical grammar inventory must contain exactly {EXPECTED_CANONICAL_RULES} rules"
  );

  let mut generated = String::from(
    "pub const CANONICAL_RULE_COUNT: usize = 540;\n\
     pub static CANONICAL_RULES: &[(&str, RuleId)] = &[\n",
  );
  for (name, id) in rules {
    generated.push_str(&format!("  ({name:?}, RuleId(0x{id:08x})),\n"));
  }
  generated.push_str("];\n");

  let output = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR must be set"))
    .join("canonical_rules.rs");
  fs::write(&output, generated)
    .unwrap_or_else(|error| panic!("failed to write {}: {error}", output.display()));
}
