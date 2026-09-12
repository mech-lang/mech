use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const CONSUMER_HEADER: &str = "consumer-id\tpackage\tsource-path\tcaller-symbol\tcall-count\t\
feature-gate\tinput-policy\tfailure-policy\tcurrent-result-use\tcanonical-target\t\
cutover-action\tfixture-id\tnotes";
const FIXTURE_HEADER: &str = "fixture-id\tsource-path\tcanonical-root\tcanonical-outcome\t\
consumer-contracts\tnotes";
const EXPECTED_CONSUMERS: usize = 25;
const EXPECTED_PRODUCTION_CALLS: usize = 27;
const EXPECTED_FIXTURES: usize = 9;

#[derive(Debug)]
struct ConsumerRow {
    source_path: String,
    caller: String,
    calls: usize,
    feature_gate: String,
    input_policy: String,
    failure_policy: String,
    current_result_use: String,
    canonical_target: String,
    cutover_action: String,
    fixture: String,
    notes: String,
}

#[derive(Debug)]
struct FixtureRow {
    source_path: String,
    root: String,
    outcome: String,
    contracts: String,
    notes: String,
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fields(line: &str) -> Vec<&str> {
    line.split('\t').collect()
}

fn consumers() -> BTreeMap<String, ConsumerRow> {
    let path = "docs/design/grammar-audit/source-parser-consumers.tsv";
    let source = fs::read_to_string(repository_root().join(path)).expect("read consumer inventory");
    let mut lines = source.lines();
    assert_eq!(lines.next(), Some(CONSUMER_HEADER));
    let mut previous = String::new();
    let mut rows = BTreeMap::new();
    for (index, line) in lines.enumerate() {
        let row = fields(line);
        assert_eq!(row.len(), 13, "invalid consumer row {}", index + 2);
        assert!(row[0] > previous.as_str(), "consumer rows are not ordered");
        previous = row[0].to_owned();
        let calls = row[4]
            .parse::<usize>()
            .unwrap_or_else(|_| panic!("invalid call count on row {}", index + 2));
        assert!(calls > 0, "empty call count on row {}", index + 2);
        let value = ConsumerRow {
            source_path: row[2].to_owned(),
            caller: row[3].to_owned(),
            calls,
            feature_gate: row[5].to_owned(),
            input_policy: row[6].to_owned(),
            failure_policy: row[7].to_owned(),
            current_result_use: row[8].to_owned(),
            canonical_target: row[9].to_owned(),
            cutover_action: row[10].to_owned(),
            fixture: row[11].to_owned(),
            notes: row[12].to_owned(),
        };
        assert!(rows.insert(row[0].to_owned(), value).is_none());
    }
    assert_eq!(rows.len(), EXPECTED_CONSUMERS);
    rows
}

fn fixtures() -> BTreeMap<String, FixtureRow> {
    let path = "docs/design/grammar-audit/source-parser-consumer-fixtures.tsv";
    let source = fs::read_to_string(repository_root().join(path)).expect("read fixture inventory");
    let mut lines = source.lines();
    assert_eq!(lines.next(), Some(FIXTURE_HEADER));
    let mut previous = String::new();
    let mut rows = BTreeMap::new();
    for (index, line) in lines.enumerate() {
        let row = fields(line);
        assert_eq!(row.len(), 6, "invalid fixture row {}", index + 2);
        assert!(row[0] > previous.as_str(), "fixture rows are not ordered");
        previous = row[0].to_owned();
        let value = FixtureRow {
            source_path: row[1].to_owned(),
            root: row[2].to_owned(),
            outcome: row[3].to_owned(),
            contracts: row[4].to_owned(),
            notes: row[5].to_owned(),
        };
        assert!(rows.insert(row[0].to_owned(), value).is_none());
    }
    assert_eq!(rows.len(), EXPECTED_FIXTURES);
    rows
}

fn visit_rust_files(path: &Path, files: &mut Vec<PathBuf>) {
    for entry in
        fs::read_dir(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
    {
        let path = entry.expect("read source-tree entry").path();
        if path.is_dir() {
            if path.file_name().and_then(|name| name.to_str()) != Some("tests") {
                visit_rust_files(&path, files);
            }
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs")
            && path.file_name().and_then(|name| name.to_str()) != Some("tests.rs")
        {
            files.push(path);
        }
    }
}

fn production_prefix(source: &str) -> &str {
    ["\nmod tests {", "\nmod tests;"]
        .into_iter()
        .filter_map(|marker| source.find(marker))
        .min()
        .map_or(source, |end| &source[..end])
}

fn test_only_parse_calls(path: &str, source: &str) -> usize {
    match path {
        "src/engine/src/program/compiler_planning.rs" => {
            assert!(source.contains("#[cfg(test)]\n    pub(crate) fn plan_source_for_test"));
            1
        }
        "src/engine/src/structures.rs" => {
            assert!(source.contains("fn wildcard_table_column_uses_the_canonical_dynamic_schema"));
            1
        }
        _ => 0,
    }
}

fn discovered_production_calls() -> BTreeMap<String, usize> {
    let root = repository_root();
    let mut files = Vec::new();
    visit_rust_files(&root.join("src"), &mut files);
    let mut calls = BTreeMap::new();
    for path in files {
        let relative = path
            .strip_prefix(&root)
            .expect("source path is below repository root")
            .to_string_lossy()
            .replace('\\', "/");
        let source = fs::read_to_string(&path).expect("read Rust source");
        let prefix = production_prefix(&source);
        let count = prefix.matches("parser::parse(").count();
        let count = count
            .checked_sub(test_only_parse_calls(&relative, prefix))
            .expect("test-only parse count exceeds file count");
        if count > 0 {
            calls.insert(relative, count);
        }
    }
    calls
}

#[test]
fn production_parser_callers_are_exactly_inventoried() {
    let consumers = consumers();
    let mut expected = BTreeMap::new();
    let mut total = 0usize;
    for (id, row) in &consumers {
        let source = fs::read_to_string(repository_root().join(&row.source_path))
            .unwrap_or_else(|error| panic!("read consumer source for {id}: {error}"));
        assert!(
            source.contains(&format!("fn {}", row.caller)),
            "missing caller {} for {id}",
            row.caller
        );
        for (field, value) in [
            ("feature gate", &row.feature_gate),
            ("input policy", &row.input_policy),
            ("failure policy", &row.failure_policy),
            ("current result use", &row.current_result_use),
            ("canonical target", &row.canonical_target),
            ("cutover action", &row.cutover_action),
            ("notes", &row.notes),
        ] {
            assert!(!value.is_empty(), "empty {field} for {id}");
        }
        assert!(
            !row.canonical_target.contains("legacy")
                && !row.canonical_target.contains("compat")
                && !row.cutover_action.contains("legacy")
                && !row.cutover_action.contains("compat"),
            "noncanonical cutover target for {id}"
        );
        *expected.entry(row.source_path.clone()).or_insert(0usize) += row.calls;
        total += row.calls;
    }
    assert_eq!(total, EXPECTED_PRODUCTION_CALLS);
    assert_eq!(discovered_production_calls(), expected);
}

#[test]
fn every_consumer_names_a_frozen_fixture() {
    let fixtures = fixtures();
    let fixture_names = fixtures.keys().cloned().collect::<BTreeSet<_>>();
    for (id, row) in consumers() {
        assert!(
            fixture_names.contains(&row.fixture),
            "unknown fixture {} for {id}",
            row.fixture
        );
    }
}

#[test]
fn frozen_fixtures_define_canonical_document_expectations() {
    for (id, row) in fixtures() {
        assert_eq!(row.root, "document", "canonical root for {id}");
        assert!(
            matches!(row.outcome.as_str(), "accept" | "reject"),
            "canonical outcome for {id}"
        );
        assert!(
            !row.contracts.is_empty(),
            "empty consumer contracts for {id}"
        );
        assert!(!row.notes.is_empty(), "empty fixture note for {id}");
        assert!(
            repository_root().join(&row.source_path).is_file(),
            "missing canonical fixture {id}"
        );
    }
}
