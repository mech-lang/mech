use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

const CORRECTION_HEADER: &str = "id\tapplied-date\tstatus\taffected-rules\told-behavior\tnew-behavior\trationale\tconformance-cases";

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn canonical_grammar(specification: &str) -> &str {
    let (_, after_opening) = specification
        .split_once("```ebnf:canonical\n")
        .expect("canonical grammar fence");
    let (grammar, _) = after_opening
        .split_once("```")
        .expect("canonical grammar fence is closed");

    grammar
}

fn canonical_grammar_rules(specification: &str) -> BTreeSet<&str> {
    canonical_grammar(specification)
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

fn canonical_rule_body(specification: &str, rule: &str) -> Option<String> {
    let grammar = canonical_grammar(specification);
    if grammar
        .lines()
        .filter(|line| {
            line.split_once(":=")
                .is_some_and(|(name, _)| name.trim() == rule)
        })
        .count()
        != 1
    {
        return None;
    }
    let mut lines = grammar.lines();
    let first = lines.by_ref().find_map(|line| {
        let (name, body) = line.split_once(":=")?;
        (name.trim() == rule).then_some(body)
    })?;
    Some(
        std::iter::once(first)
            .chain(lines)
            .flat_map(str::chars)
            .take_while(|character| *character != ';')
            .filter(|character| !character.is_whitespace())
            .collect(),
    )
}

fn has_count_choice_and_stage(specification: &str) -> bool {
    canonical_rule_body(specification, "application-operator").as_deref()
        == Some("bar,(*digit-token|matrix),right-angle")
        && canonical_rule_body(specification, "application-stage").as_deref()
            == Some("whitespace0,application-operator,whitespace0,identifier")
}

#[test]
fn count_correction_rejects_lost_grouped_choice_and_stage_link() {
    let specification = |operator, stage| {
        format!(
            "```ebnf:canonical\napplication-operator := {operator};\napplication-stage := {stage};\n```\n"
        )
    };
    let operator = "bar, (*digit-token | matrix), right-angle";
    let stage = "whitespace0, application-operator, whitespace0, identifier";
    assert!(has_count_choice_and_stage(&specification(operator, stage)));
    assert!(has_count_choice_and_stage(&specification(
        "bar,\n (*digit-token\n | matrix),\n right-angle",
        stage,
    )));
    for broken in [
        "bar, digit-token, matrix, right-angle",
        "bar, *digit-token, right-angle",
        "bar, matrix, right-angle",
        "bar, *digit-token | matrix, right-angle",
    ] {
        assert!(!has_count_choice_and_stage(&specification(broken, stage)));
    }
    for broken in [
        "whitespace0, identifier",
        "whitespace0, matrix, whitespace0, identifier",
    ] {
        assert!(!has_count_choice_and_stage(&specification(
            operator, broken
        )));
    }
    let duplicate =
        specification(operator, stage).replace("```\n", "application-stage := identifier;\n```\n");
    assert!(!has_count_choice_and_stage(&duplicate));
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
        if matches!(fields[0], "IMPORT-001" | "MATCH-001" | "COUNT-001") {
            assert_eq!(fields[2], "applied");
            applied.insert(fields[0]);
        }
    }
    let expected = BTreeSet::from(["IMPORT-001", "MATCH-001", "COUNT-001"]);
    assert_eq!(applied, expected);
    assert_eq!(ids, expected);

    let specification = fs::read_to_string(root.join("docs/design/specification.mec"))
        .expect("read canonical specification");
    let rules = canonical_grammar_rules(&specification);
    assert!(rules.contains("import-sigil"));
    assert!(!rules.contains("module-import-sigil"));
    assert!(!rules.contains("module-import-end"));
    assert!(rules.contains("application-operator"));
    assert!(rules.contains("application-stage"));
    assert!(has_count_choice_and_stage(&specification));
}
