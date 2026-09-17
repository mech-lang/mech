use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use mech_syntax::document::parser::{
    CANONICAL_PORT_COUNT, CANONICAL_PORTS, CANONICAL_RULE_COUNT, CANONICAL_RULES,
    LoweringPortStatus, NodePolicy, PortPhase, RuleFamily, SyntaxPortStatus, canonical_rule_id,
};

const EXPECTED_RULES: usize = 540;
const EXPECTED_PHASE_2A: usize = 167;

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fields(line: &str) -> Vec<&str> {
    line.split('\t').collect()
}

fn family_name(family: RuleFamily) -> &'static str {
    match family {
        RuleFamily::Activation => "activation",
        RuleFamily::Base => "base",
        RuleFamily::Expressions => "expressions",
        RuleFamily::Functions => "functions",
        RuleFamily::Grammar => "grammar",
        RuleFamily::Imports => "imports",
        RuleFamily::Literals => "literals",
        RuleFamily::Mechdown => "mechdown",
        RuleFamily::Mika => "mika",
        RuleFamily::Parser => "parser",
        RuleFamily::Patterns => "patterns",
        RuleFamily::Repl => "repl",
        RuleFamily::StateMachines => "state_machines",
        RuleFamily::Statements => "statements",
        RuleFamily::Structures => "structures",
    }
}

fn syntax_name(status: SyntaxPortStatus) -> &'static str {
    match status {
        SyntaxPortStatus::Unported => "unported",
        SyntaxPortStatus::SyntaxPorted => "syntax-ported",
        SyntaxPortStatus::ParityVerified => "parity-verified",
    }
}

fn lowering_name(status: LoweringPortStatus) -> &'static str {
    match status {
        LoweringPortStatus::NotApplicable => "not-applicable",
        LoweringPortStatus::Pending => "pending",
        LoweringPortStatus::ParityVerified => "parity-verified",
    }
}

fn policy_name(policy: NodePolicy) -> String {
    match policy {
        NodePolicy::Undecided => "undecided".to_owned(),
        NodePolicy::Token => "token".to_owned(),
        NodePolicy::Transparent => "transparent".to_owned(),
        NodePolicy::Node(kind) => format!("node:{kind:?}"),
        NodePolicy::Root(kind) => format!("root:{kind:?}"),
    }
}

fn phase_name(phase: Option<PortPhase>) -> &'static str {
    match phase {
        None => "",
        Some(PortPhase::Phase2A) => "2A",
    }
}

#[test]
fn checked_in_port_registry_exactly_matches_ports_tsv() {
    let ports = fs::read_to_string(repository_root().join("docs/design/grammar-audit/ports.tsv"))
        .expect("read ports.tsv");
    let mut lines = ports.lines();
    assert_eq!(
        lines.next(),
        Some(
            "grammar-name\tfamily\tsyntax-status\tlowering-status\t\
       node-policy\tphase\tnotes"
        )
    );
    let rows = lines.map(fields).collect::<Vec<_>>();
    assert_eq!(rows.len(), EXPECTED_RULES);
    assert_eq!(CANONICAL_PORT_COUNT, EXPECTED_RULES);
    assert_eq!(CANONICAL_PORTS.len(), EXPECTED_RULES);

    let mut previous = "";
    let mut names = BTreeSet::new();
    for (index, (row, generated)) in rows.iter().zip(CANONICAL_PORTS).enumerate() {
        assert_eq!(row.len(), 7, "invalid ports.tsv row {}", index + 2);
        assert!(row[0] > previous, "ports.tsv is not strictly ordered");
        previous = row[0];
        assert!(names.insert(row[0]), "duplicate port entry {}", row[0]);
        assert_eq!(generated.name, row[0]);
        assert_eq!(generated.rule, canonical_rule_id(row[0]).unwrap());
        assert_eq!(family_name(generated.family), row[1]);
        assert_eq!(syntax_name(generated.syntax), row[2]);
        assert_eq!(lowering_name(generated.lowering), row[3]);
        assert_eq!(policy_name(generated.node_policy), row[4]);
        assert_eq!(phase_name(generated.phase), row[5]);
        assert_eq!(generated.notes, row[6]);
    }

    let canonical = CANONICAL_RULES
        .iter()
        .map(|(name, _)| *name)
        .collect::<BTreeSet<_>>();
    assert_eq!(CANONICAL_RULE_COUNT, EXPECTED_RULES);
    assert_eq!(canonical.len(), EXPECTED_RULES);
    assert_eq!(names, canonical, "unknown or missing canonical port names");
}

#[test]
fn phase_2a_is_the_exact_mechanically_closed_167_rule_set() {
    let ported = CANONICAL_PORTS
        .iter()
        .filter(|port| port.syntax != SyntaxPortStatus::Unported)
        .collect::<Vec<_>>();
    let phase_2a = CANONICAL_PORTS
        .iter()
        .filter(|port| port.phase == Some(PortPhase::Phase2A))
        .collect::<Vec<_>>();
    let parity = CANONICAL_PORTS
        .iter()
        .filter(|port| port.syntax == SyntaxPortStatus::ParityVerified)
        .count();
    assert_eq!(ported.len(), EXPECTED_PHASE_2A);
    assert_eq!(phase_2a.len(), EXPECTED_PHASE_2A);
    assert_eq!(parity, EXPECTED_PHASE_2A);
    assert!(
        ported
            .iter()
            .all(|port| port.phase == Some(PortPhase::Phase2A))
    );

    let names = ported.iter().map(|port| port.name).collect::<BTreeSet<_>>();

    let inventory =
        fs::read_to_string(repository_root().join("docs/design/grammar-audit/productions.tsv"))
            .expect("read productions.tsv");
    let mut lines = inventory.lines();
    let header = fields(lines.next().expect("productions.tsv header"));
    let grammar_name = header
        .iter()
        .position(|column| *column == "grammar-name")
        .unwrap();
    let module = header
        .iter()
        .position(|column| *column == "module")
        .unwrap();
    let specification = header
        .iter()
        .position(|column| *column == "spec-location")
        .unwrap();
    let explicit = BTreeSet::from([
        "left-angle",
        "right-angle",
        "box-drawing-char",
        "box-drawing-emoji",
        "tag",
        "parse-grammar",
    ]);
    let expected = lines
        .map(fields)
        .filter(|row| {
            row[specification].starts_with("docs/design/specification.mec::")
                && (matches!(row[module], "base" | "grammar")
                    || explicit.contains(row[grammar_name]))
        })
        .map(|row| row[grammar_name].to_owned())
        .collect::<BTreeSet<_>>();
    assert_eq!(expected.len(), EXPECTED_PHASE_2A);
    assert_eq!(
        names,
        expected.iter().map(String::as_str).collect(),
        "Phase 2A port set differs from the mechanical selection formula"
    );

    for dependency in ["left-angle", "right-angle"] {
        assert!(
            names.contains(dependency),
            "grouping-symbol hidden dependency {dependency} is unported"
        );
    }
    assert!(
        names.contains("box-drawing-emoji"),
        "forbidden-emoji hidden dependency box-drawing-emoji is unported"
    );
}

#[test]
fn every_ported_rule_has_closed_declared_children_and_conformance_evidence() {
    let inventory =
        fs::read_to_string(repository_root().join("docs/design/grammar-audit/productions.tsv"))
            .expect("read productions.tsv");
    let mut lines = inventory.lines();
    let header = fields(lines.next().expect("productions.tsv header"));
    let grammar_name = header
        .iter()
        .position(|column| *column == "grammar-name")
        .unwrap();
    let child_rules = header
        .iter()
        .position(|column| *column == "child-rules")
        .unwrap();
    let specification = header
        .iter()
        .position(|column| *column == "spec-location")
        .unwrap();
    let conformance = header
        .iter()
        .position(|column| *column == "conformance-cases")
        .unwrap();

    let mut rows = BTreeMap::new();
    for (index, line) in lines.enumerate() {
        let row = fields(line);
        assert_eq!(
            row.len(),
            header.len(),
            "invalid inventory row {}",
            index + 2
        );
        if row[specification].starts_with("docs/design/specification.mec::") {
            assert!(rows.insert(row[grammar_name], row).is_none());
        }
    }
    assert_eq!(rows.len(), EXPECTED_RULES);

    let cases =
        fs::read_to_string(repository_root().join("src/syntax/tests/fixtures/grammar/cases.tsv"))
            .expect("read cases.tsv");
    let mut case_lines = cases.lines();
    let case_header = fields(case_lines.next().expect("cases.tsv header"));
    let case_id = case_header
        .iter()
        .position(|column| *column == "id")
        .unwrap();
    let case_ids = case_lines
        .map(fields)
        .map(|row| row[case_id])
        .collect::<BTreeSet<_>>();

    let ported = CANONICAL_PORTS
        .iter()
        .filter(|port| port.syntax != SyntaxPortStatus::Unported)
        .map(|port| port.name)
        .collect::<BTreeSet<_>>();
    for name in &ported {
        let row = &rows[name];
        assert!(
            !row[conformance].is_empty() && row[conformance] != "none",
            "{name} has no canonical conformance evidence"
        );
        for conformance_id in row[conformance].split(',') {
            assert!(
                case_ids.contains(conformance_id),
                "{name} references unknown conformance case {conformance_id}"
            );
        }
        for child in row[child_rules].split(',') {
            if child.is_empty() || child == "none" {
                continue;
            }
            // These two rows record nom::sequence::tuple, not the canonical
            // structures.tuple production.
            if child == "tuple" && matches!(*name, "grammar-range" | "grammar-terminal-token") {
                continue;
            }
            if rows.contains_key(child) {
                assert!(
                    ported.contains(child),
                    "{name} has unported canonical child {child}"
                );
            }
        }
    }
}

#[test]
fn phase_2a_node_and_lowering_policies_are_exact() {
    let structural = BTreeMap::from([
        ("digit-sequence", "node:DigitSequence"),
        ("escaped-char", "node:EscapedCharacter"),
        ("identifier", "node:Identifier"),
        ("identifier-path-segment", "node:IdentifierPathSegment"),
        ("grammar", "node:Grammar"),
        ("grammar-definition", "node:GrammarDefinition"),
        ("grammar-expression", "node:GrammarExpression"),
        ("grammar-factor", "node:GrammarFactor"),
        ("grammar-group", "node:GrammarGroup"),
        ("grammar-identifier", "node:GrammarIdentifier"),
        ("grammar-list", "node:GrammarList"),
        ("grammar-not", "node:GrammarNot"),
        ("grammar-optional", "node:GrammarOptional"),
        ("grammar-peek", "node:GrammarPeek"),
        ("grammar-range", "node:GrammarRange"),
        ("grammar-repeat0", "node:GrammarRepeat0"),
        ("grammar-repeat1", "node:GrammarRepeat1"),
        ("grammar-rule", "node:GrammarRule"),
        ("grammar-term", "node:GrammarTerm"),
        ("grammar-terminal", "node:GrammarTerminal"),
        ("grammar-terminal-token", "node:GrammarTerminalToken"),
        ("parse-grammar", "root:GrammarDocument"),
    ]);
    let transparent = BTreeSet::from([
        "enum-separator",
        "list-separator",
        "newline-indent",
        "space-tab0",
        "space-tab1",
        "whitespace0",
        "whitespace1",
        "ws0e",
        "ws1e",
    ]);

    let mut counts = BTreeMap::new();
    for port in CANONICAL_PORTS {
        *counts
            .entry(policy_name(port.node_policy))
            .or_insert(0_usize) += 1;
        if port.phase != Some(PortPhase::Phase2A) {
            assert_eq!(port.node_policy, NodePolicy::Undecided);
            assert_eq!(port.lowering, LoweringPortStatus::Pending);
            continue;
        }
        if let Some(expected) = structural.get(port.name) {
            assert_eq!(policy_name(port.node_policy), *expected);
            assert_eq!(port.lowering, LoweringPortStatus::ParityVerified);
        } else if transparent.contains(port.name) {
            assert_eq!(port.node_policy, NodePolicy::Transparent);
            assert_eq!(port.lowering, LoweringPortStatus::NotApplicable);
        } else {
            assert_eq!(port.node_policy, NodePolicy::Token);
            assert_eq!(port.lowering, LoweringPortStatus::NotApplicable);
        }
    }

    assert_eq!(structural.len(), 22);
    assert_eq!(transparent.len(), 9);
    assert_eq!(counts["undecided"], 373);
    assert_eq!(counts["token"], 136);
    assert_eq!(counts["transparent"], 9);
    assert_eq!(
        counts
            .iter()
            .filter(|(policy, _)| policy.starts_with("node:"))
            .map(|(_, count)| count)
            .sum::<usize>(),
        21
    );
    assert_eq!(
        counts
            .iter()
            .filter(|(policy, _)| policy.starts_with("root:"))
            .map(|(_, count)| count)
            .sum::<usize>(),
        1
    );
}
