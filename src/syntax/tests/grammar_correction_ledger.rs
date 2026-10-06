use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

const CORRECTION_HEADER: &str = "id\tapplied-date\tstatus\taffected-rules\told-behavior\tnew-behavior\trationale\tconformance-cases";

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn canonical_grammar_rules(specification: &str) -> BTreeSet<&str> {
    let (_, after_opening) = specification
        .split_once("```ebnf:canonical\n")
        .expect("canonical grammar fence");
    let (grammar, _) = after_opening
        .split_once("```")
        .expect("canonical grammar fence is closed");

    grammar
        .lines()
        .filter_map(|line| line.split_once(":=").map(|(name, _)| name.trim()))
        .filter(|name| {
            !name.is_empty()
                && name
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte == b'-')
        })
        .collect()
}

#[test]
fn grammar_correction_ledger_is_well_formed_and_applied_to_the_canonical_grammar() {
    let root = repository_root();
    let corrections = fs::read_to_string(root.join("docs/design/grammar-audit/corrections.tsv"))
        .expect("read corrections.tsv");
    let mut lines = corrections.lines();
    assert_eq!(lines.next(), Some(CORRECTION_HEADER));

    let mut ids = BTreeSet::new();
    let mut applied = BTreeSet::new();
    for (index, line) in lines.enumerate() {
        assert!(
            !line.is_empty(),
            "blank correction row at line {}",
            index + 2
        );
        let fields = line.split('\t').collect::<Vec<_>>();
        assert_eq!(
            fields.len(),
            8,
            "invalid correction row at line {}",
            index + 2
        );
        assert!(
            ids.insert(fields[0]),
            "duplicate correction ID {:?}",
            fields[0]
        );
        assert!(
            matches!(fields[2], "proposed" | "applied" | "reverted"),
            "unknown correction status {:?}",
            fields[2]
        );
        assert!(
            !fields[3].is_empty()
                && !fields[4].is_empty()
                && !fields[5].is_empty()
                && !fields[6].is_empty()
                && !fields[7].is_empty(),
            "correction {:?} has an empty required field",
            fields[0]
        );
        if matches!(fields[0], "IMPORT-001" | "MATCH-001") {
            assert_eq!(fields[2], "applied");
            applied.insert(fields[0]);
        }
    }
    assert_eq!(applied, BTreeSet::from(["IMPORT-001", "MATCH-001"]));
    assert_eq!(ids, BTreeSet::from(["IMPORT-001", "MATCH-001"]));

    let specification = fs::read_to_string(root.join("docs/design/specification.mec"))
        .expect("read canonical specification");
    let rules = canonical_grammar_rules(&specification);
    assert!(rules.contains("import-sigil"));
    assert!(!rules.contains("module-import-sigil"));
    assert!(!rules.contains("module-import-end"));
}
