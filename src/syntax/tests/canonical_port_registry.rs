use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use mech_syntax::document::parser::{
    CANONICAL_PORT_COUNT, CANONICAL_PORTS, CANONICAL_RULE_COUNT, CANONICAL_RULES,
    LoweringPortStatus, NodePolicy, PortPhase, RuleFamily, SyntaxPortStatus, canonical_rule_id,
};

const EXPECTED_RULES: usize = 539;
const EXPECTED_PHASE_2A: usize = 167;
const EXPECTED_PHASE_2B: usize = 13;
const EXPECTED_PHASE_2C: usize = 30;
const EXPECTED_PHASE_2D: usize = 53;
const EXPECTED_PHASE_2E: usize = 19;
const EXPECTED_PHASE_2F: usize = 21;
const EXPECTED_PHASE_2G: usize = 15;
const EXPECTED_PORTED: usize = 318;
const EXPECTED_UNPORTED: usize = 221;

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
        Some(PortPhase::Phase2B) => "2B",
        Some(PortPhase::Phase2C) => "2C",
        Some(PortPhase::Phase2D) => "2D",
        Some(PortPhase::Phase2E) => "2E",
        Some(PortPhase::Phase2F) => "2F",
        Some(PortPhase::Phase2G) => "2G",
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
    let phase_2a = CANONICAL_PORTS
        .iter()
        .filter(|port| port.phase == Some(PortPhase::Phase2A))
        .collect::<Vec<_>>();
    assert_eq!(phase_2a.len(), EXPECTED_PHASE_2A);
    assert!(
        phase_2a
            .iter()
            .all(|port| port.syntax == SyntaxPortStatus::ParityVerified)
    );

    let names = phase_2a
        .iter()
        .map(|port| port.name)
        .collect::<BTreeSet<_>>();

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
fn every_phase_2a_rule_has_closed_declared_children_and_conformance_evidence() {
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

    let phase_2a = CANONICAL_PORTS
        .iter()
        .filter(|port| port.phase == Some(PortPhase::Phase2A))
        .map(|port| port.name)
        .collect::<BTreeSet<_>>();
    for name in &phase_2a {
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
                    phase_2a.contains(child),
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
    for port in CANONICAL_PORTS
        .iter()
        .filter(|port| port.phase == Some(PortPhase::Phase2A))
    {
        *counts
            .entry(policy_name(port.node_policy))
            .or_insert(0_usize) += 1;
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

    for port in CANONICAL_PORTS.iter().filter(|port| port.phase.is_none()) {
        assert_eq!(port.syntax, SyntaxPortStatus::Unported);
        assert_eq!(port.node_policy, NodePolicy::Undecided);
        assert_eq!(port.lowering, LoweringPortStatus::Pending);
    }
}

#[test]
fn phase_2b_registry_accounting_and_policies_are_exact() {
    let expected = BTreeMap::from([
        (
            "blank-line",
            (LoweringPortStatus::NotApplicable, "node:BlankLine"),
        ),
        (
            "codeblock-sigil",
            (LoweringPortStatus::NotApplicable, "transparent"),
        ),
        (
            "comment",
            (LoweringPortStatus::Pending, "node:Comment"),
        ),
        (
            "comment-sigil",
            (LoweringPortStatus::NotApplicable, "transparent"),
        ),
        (
            "equation",
            (LoweringPortStatus::ParityVerified, "node:Equation"),
        ),
        (
            "footnote-reference",
            (
                LoweringPortStatus::ParityVerified,
                "node:FootnoteReference",
            ),
        ),
        (
            "inline-code",
            (LoweringPortStatus::ParityVerified, "node:InlineCode"),
        ),
        (
            "inline-equation",
            (
                LoweringPortStatus::ParityVerified,
                "node:InlineEquation",
            ),
        ),
        (
            "paragraph-text",
            (LoweringPortStatus::ParityVerified, "node:ParagraphText"),
        ),
        (
            "raw-hyperlink",
            (LoweringPortStatus::ParityVerified, "node:RawHyperlink"),
        ),
        (
            "reference",
            (LoweringPortStatus::ParityVerified, "node:Reference"),
        ),
        (
            "section-reference",
            (
                LoweringPortStatus::ParityVerified,
                "node:SectionReference",
            ),
        ),
        (
            "thematic-break",
            (LoweringPortStatus::ParityVerified, "node:ThematicBreak"),
        ),
    ]);
    assert_eq!(expected.len(), EXPECTED_PHASE_2B);

    let phase_2b = CANONICAL_PORTS
        .iter()
        .filter(|port| port.phase == Some(PortPhase::Phase2B))
        .collect::<Vec<_>>();
    assert_eq!(phase_2b.len(), EXPECTED_PHASE_2B);
    for port in phase_2b {
        let (lowering, policy) = expected
            .get(port.name)
            .unwrap_or_else(|| panic!("unexpected Phase 2B rule {}", port.name));
        assert_eq!(port.syntax, SyntaxPortStatus::ParityVerified);
        assert_eq!(port.lowering, *lowering);
        assert_eq!(policy_name(port.node_policy), *policy);
    }

    let ported = CANONICAL_PORTS
        .iter()
        .filter(|port| port.syntax != SyntaxPortStatus::Unported)
        .collect::<Vec<_>>();
    assert_eq!(ported.len(), EXPECTED_PORTED);
    assert_eq!(CANONICAL_PORTS.len() - ported.len(), EXPECTED_UNPORTED);
    assert_eq!(
        CANONICAL_PORTS
            .iter()
            .filter(|port| port.syntax == SyntaxPortStatus::ParityVerified)
            .count(),
        313
    );
    assert_eq!(
        CANONICAL_PORTS
            .iter()
            .filter(|port| port.syntax == SyntaxPortStatus::SyntaxPorted)
            .count(),
        5
    );
    assert_eq!(
        ported
            .iter()
            .filter(|port| port.phase == Some(PortPhase::Phase2A))
            .count(),
        EXPECTED_PHASE_2A
    );
    assert_eq!(
        ported
            .iter()
            .filter(|port| port.phase == Some(PortPhase::Phase2B))
            .count(),
        EXPECTED_PHASE_2B
    );
    assert_eq!(
        ported
            .iter()
            .filter(|port| port.phase == Some(PortPhase::Phase2C))
            .count(),
        EXPECTED_PHASE_2C
    );
    assert_eq!(
        ported
            .iter()
            .filter(|port| port.phase == Some(PortPhase::Phase2D))
            .count(),
        EXPECTED_PHASE_2D
    );
    assert_eq!(
        ported
            .iter()
            .filter(|port| port.phase == Some(PortPhase::Phase2E))
            .count(),
        EXPECTED_PHASE_2E
    );
    assert_eq!(
        ported
            .iter()
            .filter(|port| port.phase == Some(PortPhase::Phase2F))
            .count(),
        EXPECTED_PHASE_2F
    );
    assert_eq!(
        ported
            .iter()
            .filter(|port| port.phase == Some(PortPhase::Phase2G))
            .count(),
        EXPECTED_PHASE_2G
    );
    assert_eq!(
        ported
            .iter()
            .filter(|port| port.lowering == LoweringPortStatus::ParityVerified)
            .count(),
        149
    );
    assert_eq!(
        ported
            .iter()
            .filter(|port| port.lowering == LoweringPortStatus::Pending)
            .count(),
        6
    );
    assert_eq!(
        ported
            .iter()
            .filter(|port| port.lowering == LoweringPortStatus::NotApplicable)
            .count(),
        163
    );
}

#[test]
fn phase_2c_registry_accounting_and_policies_are_exact() {
    let node_policies = [
        ("empty", "EmptyLiteral"),
        ("atom", "AtomLiteral"),
        ("string", "StringLiteral"),
        ("utf8-string", "Utf8String"),
        ("raw-string", "RawString"),
        ("number", "Number"),
        ("complex-number", "ComplexNumber"),
        ("real-number", "RealNumber"),
        ("untyped-real-number", "UntypedRealNumber"),
        ("rational-literal", "RationalLiteral"),
        ("scientific-literal", "ScientificLiteral"),
        ("float-decimal-start", "FloatDecimalStart"),
        ("float-full", "FloatFull"),
        ("float-literal", "FloatLiteral"),
        ("integer-literal", "IntegerLiteral"),
        ("typed-integer", "TypedInteger"),
        ("untyped-integer", "UntypedInteger"),
        ("decimal-literal", "DecimalLiteral"),
        ("hexadecimal-literal", "HexadecimalLiteral"),
        ("octal-literal", "OctalLiteral"),
        ("binary-literal", "BinaryLiteral"),
        ("context-address-path", "ContextAddressPath"),
        ("prefixed-context-path", "PrefixedContextPath"),
        ("kind-any", "KindAny"),
        ("kind-empty", "KindEmpty"),
        ("kind-atom", "KindAtom"),
    ];
    let token_rules = [
        "boolean",
        "true-literal",
        "false-literal",
        "context-address-path-token",
    ];
    let exceptions = BTreeSet::from([
        "number",
        "complex-number",
        "real-number",
        "untyped-real-number",
        "scientific-literal",
    ]);
    let expected_names = node_policies
        .iter()
        .map(|(name, _)| *name)
        .chain(token_rules)
        .collect::<BTreeSet<_>>();
    assert_eq!(expected_names.len(), EXPECTED_PHASE_2C);
    assert_eq!(node_policies.len(), 26);
    assert_eq!(token_rules.len(), 4);
    assert_eq!(exceptions.len(), 5);

    let phase_2c = CANONICAL_PORTS
        .iter()
        .filter(|port| port.phase == Some(PortPhase::Phase2C))
        .collect::<Vec<_>>();
    assert_eq!(phase_2c.len(), EXPECTED_PHASE_2C);
    assert_eq!(
        phase_2c.iter().map(|port| port.name).collect::<BTreeSet<_>>(),
        expected_names
    );

    for port in phase_2c {
        if let Some((_, kind)) = node_policies.iter().find(|(name, _)| *name == port.name) {
            assert_eq!(policy_name(port.node_policy), format!("node:{kind}"));
            if exceptions.contains(port.name) {
                assert_eq!(port.syntax, SyntaxPortStatus::SyntaxPorted, "{}", port.name);
                assert_eq!(port.lowering, LoweringPortStatus::Pending, "{}", port.name);
            } else {
                assert_eq!(
                    port.syntax,
                    SyntaxPortStatus::ParityVerified,
                    "{}",
                    port.name
                );
                assert_eq!(
                    port.lowering,
                    LoweringPortStatus::ParityVerified,
                    "{}",
                    port.name
                );
            }
        } else {
            assert!(token_rules.contains(&port.name), "{}", port.name);
            assert_eq!(port.syntax, SyntaxPortStatus::ParityVerified, "{}", port.name);
            assert_eq!(port.lowering, LoweringPortStatus::NotApplicable, "{}", port.name);
            assert_eq!(port.node_policy, NodePolicy::Token, "{}", port.name);
        }
    }

    let syntax_ported = CANONICAL_PORTS
        .iter()
        .filter(|port| {
            port.phase == Some(PortPhase::Phase2C)
                && port.syntax == SyntaxPortStatus::SyntaxPorted
        })
        .map(|port| port.name)
        .collect::<BTreeSet<_>>();
    assert_eq!(syntax_ported, exceptions);
    assert_eq!(
        CANONICAL_PORTS
            .iter()
            .filter(|port| {
                port.phase == Some(PortPhase::Phase2C)
                    && port.syntax == SyntaxPortStatus::ParityVerified
            })
            .count(),
        25
    );
}

#[test]
fn phase_2d_registry_accounting_and_policies_are_exact() {
    let node_policies = [
        ("add-sub-operator", "AddSubOperator"),
        ("mul-div-operator", "MulDivOperator"),
        ("power-operator", "PowerOperator"),
        ("matrix-operator", "MatrixOperator"),
        ("range-operator", "RangeOperator"),
        ("comparison-operator", "ComparisonOperator"),
        ("logic-operator", "LogicOperator"),
        ("table-operator", "TableOperator"),
        ("set-operator", "SetOperator"),
        ("add", "AddOperation"),
        ("subtract", "SubtractOperation"),
        ("raw-subtract", "RawSubtractOperation"),
        ("spaced-subtract", "SpacedSubtractOperation"),
        ("multiply", "MultiplyOperation"),
        ("divide", "DivideOperation"),
        ("modulus", "ModulusOperation"),
        ("power", "PowerOperation"),
        ("matrix-multiply", "MatrixMultiplyOperation"),
        ("matrix-solve", "MatrixSolveOperation"),
        ("dot-product", "DotProductOperation"),
        ("cross-product", "CrossProductOperation"),
        ("range-inclusive", "RangeInclusiveOperation"),
        ("range-exclusive", "RangeExclusiveOperation"),
        ("not-equal", "NotEqualOperation"),
        ("equal-to", "EqualToOperation"),
        ("strict-not-equal", "StrictNotEqualOperation"),
        ("strict-equal", "StrictEqualOperation"),
        ("greater-than", "GreaterThanOperation"),
        ("less-than", "LessThanOperation"),
        ("greater-than-equal", "GreaterThanEqualOperation"),
        ("less-than-equal", "LessThanEqualOperation"),
        ("or", "OrOperation"),
        ("and", "AndOperation"),
        ("not", "NotOperation"),
        ("xor", "XorOperation"),
        ("join", "JoinOperation"),
        ("left-join", "LeftJoinOperation"),
        ("right-join", "RightJoinOperation"),
        ("full-join", "FullJoinOperation"),
        ("left-semi-join", "LeftSemiJoinOperation"),
        ("left-anti-join", "LeftAntiJoinOperation"),
        ("union-op", "UnionOperation"),
        ("intersection", "IntersectionOperation"),
        ("difference", "DifferenceOperation"),
        ("complement", "ComplementOperation"),
        ("subset", "SubsetOperation"),
        ("superset", "SupersetOperation"),
        ("proper-subset", "ProperSubsetOperation"),
        ("proper-superset", "ProperSupersetOperation"),
        ("element-of", "ElementOfOperation"),
        ("not-element-of", "NotElementOfOperation"),
        ("symmetric-difference", "SymmetricDifferenceOperation"),
    ];
    let expected_names = node_policies
        .iter()
        .map(|(name, _)| *name)
        .chain(["transpose"])
        .collect::<BTreeSet<_>>();
    assert_eq!(node_policies.len(), 52);
    assert_eq!(expected_names.len(), EXPECTED_PHASE_2D);

    let phase_2d = CANONICAL_PORTS
        .iter()
        .filter(|port| port.phase == Some(PortPhase::Phase2D))
        .collect::<Vec<_>>();
    assert_eq!(phase_2d.len(), EXPECTED_PHASE_2D);
    assert_eq!(
        phase_2d.iter().map(|port| port.name).collect::<BTreeSet<_>>(),
        expected_names
    );

    for port in phase_2d {
        assert_eq!(port.syntax, SyntaxPortStatus::ParityVerified, "{}", port.name);
        if let Some((_, kind)) = node_policies.iter().find(|(name, _)| *name == port.name) {
            assert_eq!(policy_name(port.node_policy), format!("node:{kind}"));
            assert_eq!(port.lowering, LoweringPortStatus::ParityVerified);
        } else {
            assert_eq!(port.name, "transpose");
            assert_eq!(port.node_policy, NodePolicy::Transparent);
            assert_eq!(port.lowering, LoweringPortStatus::NotApplicable);
        }
    }

    assert_eq!(
        CANONICAL_PORTS
            .iter()
            .filter(|port| {
                port.phase == Some(PortPhase::Phase2D)
                    && port.lowering == LoweringPortStatus::ParityVerified
            })
            .count(),
        52
    );
    assert_eq!(
        CANONICAL_PORTS
            .iter()
            .filter(|port| {
                port.phase == Some(PortPhase::Phase2D)
                    && port.lowering == LoweringPortStatus::Pending
            })
            .count(),
        0
    );
    assert_eq!(
        CANONICAL_PORTS
            .iter()
            .filter(|port| {
                port.phase == Some(PortPhase::Phase2D)
                    && port.lowering == LoweringPortStatus::NotApplicable
            })
            .count(),
        1
    );
}

#[test]
fn phase_2e_registry_accounting_and_policies_are_exact() {
    let node_policies = [
        ("aliased-item-import", "AliasedItemImport"),
        ("context-import-alias-segment", "ContextImportAliasSegment"),
        ("import-group-item", "ImportGroupItem"),
        ("import-group-items", "ImportGroupItems"),
        ("module-import", "ModuleImport"),
        ("module-import-alias", "ModuleImportAlias"),
        ("module-import-alias-path", "ModuleImportAliasPath"),
        ("module-import-alias-segment", "ModuleImportAliasSegment"),
        ("module-import-context-alias", "ModuleImportContextAlias"),
        (
            "module-import-intrinsic-segment",
            "ModuleImportIntrinsicSegment",
        ),
        ("module-import-name-segment", "ModuleImportNameSegment"),
        ("module-import-path", "ModuleImportPath"),
        ("module-import-path-segment", "ModuleImportPathSegment"),
        ("module-import-value-alias", "ModuleImportValueAlias"),
        ("module-only-import", "ModuleOnlyImport"),
        ("module-root", "ModuleRoot"),
        ("module-suffix-import", "ModuleSuffixImport"),
    ];
    let transparent = BTreeSet::from(["import-alias-operator", "import-group-separator"]);
    let expected_names = node_policies
        .iter()
        .map(|(name, _)| *name)
        .chain(transparent.iter().copied())
        .collect::<BTreeSet<_>>();
    assert_eq!(node_policies.len(), 17);
    assert_eq!(transparent.len(), 2);
    assert_eq!(expected_names.len(), EXPECTED_PHASE_2E);

    let phase_2e = CANONICAL_PORTS
        .iter()
        .filter(|port| port.phase == Some(PortPhase::Phase2E))
        .collect::<Vec<_>>();
    assert_eq!(phase_2e.len(), EXPECTED_PHASE_2E);
    assert_eq!(
        phase_2e
            .iter()
            .map(|port| port.name)
            .collect::<BTreeSet<_>>(),
        expected_names
    );

    for port in &phase_2e {
        assert_eq!(
            port.syntax,
            SyntaxPortStatus::ParityVerified,
            "{}",
            port.name
        );
        if let Some((_, kind)) = node_policies.iter().find(|(name, _)| *name == port.name) {
            assert_eq!(policy_name(port.node_policy), format!("node:{kind}"));
            assert_eq!(port.lowering, LoweringPortStatus::ParityVerified);
        } else {
            assert!(transparent.contains(port.name), "{}", port.name);
            assert_eq!(port.node_policy, NodePolicy::Transparent);
            assert_eq!(port.lowering, LoweringPortStatus::NotApplicable);
        }
    }

    assert_eq!(
        phase_2e
            .iter()
            .filter(|port| port.lowering == LoweringPortStatus::ParityVerified)
            .count(),
        17
    );
    assert_eq!(
        phase_2e
            .iter()
            .filter(|port| port.lowering == LoweringPortStatus::Pending)
            .count(),
        0
    );
    assert_eq!(
        phase_2e
            .iter()
            .filter(|port| port.lowering == LoweringPortStatus::NotApplicable)
            .count(),
        2
    );

    let import_sigil = CANONICAL_PORTS
        .iter()
        .find(|port| port.name == "import-sigil")
        .expect("import-sigil port");
    assert_eq!(import_sigil.phase, Some(PortPhase::Phase2A));
    assert_eq!(import_sigil.syntax, SyntaxPortStatus::ParityVerified);
    assert_eq!(import_sigil.lowering, LoweringPortStatus::NotApplicable);
    assert_eq!(import_sigil.node_policy, NodePolicy::Token);
    assert!(
        CANONICAL_PORTS
            .iter()
            .all(|port| port.name != "module-import-sigil" && port.name != "module-import-end")
    );

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
    for row in lines.map(fields) {
        if !expected_names.contains(row[grammar_name]) {
            continue;
        }
        for child in row[child_rules].split(',') {
            if child.is_empty() || child == "none" {
                continue;
            }
            let child_port = CANONICAL_PORTS
                .iter()
                .find(|port| port.name == child)
                .unwrap_or_else(|| {
                    panic!("{} has unknown canonical child {child}", row[grammar_name])
                });
            assert_ne!(
                child_port.syntax,
                SyntaxPortStatus::Unported,
                "{} has unported canonical child {child}",
                row[grammar_name]
            );
        }
    }
}

#[test]
fn phase_2f_registry_accounting_and_policies_are_exact() {
    let node_policies = [
        ("source-import-tail", "SourceImportTail"),
        ("source-path-component", "SourcePathComponent"),
        ("source-mec-path", "SourceMecPath"),
        (
            "relative-source-import-specifier",
            "RelativeSourceImportSpecifier",
        ),
        (
            "absolute-source-import-specifier",
            "AbsoluteSourceImportSpecifier",
        ),
        ("bare-source-import-specifier", "BareSourceImportSpecifier"),
        ("source-import-uri-scheme", "SourceImportUriScheme"),
        ("uri-source-import-specifier", "UriSourceImportSpecifier"),
        ("source-import-specifier", "SourceImportSpecifier"),
        ("import-declaration", "ImportDeclaration"),
        ("export-declaration", "ExportDeclaration"),
        ("context-declaration", "ContextDeclaration"),
        ("context-base-context", "ContextBaseContext"),
        ("context-base-resource-uri", "ContextBaseResourceUri"),
        (
            "context-capability-declaration",
            "ContextCapabilityDeclaration",
        ),
        ("context-capability-path", "ContextCapabilityPath"),
        ("context-capability-scope", "ContextCapabilityScope"),
    ];
    let tokens = BTreeSet::from([
        "source-path-component-token",
        "uri-scheme-part",
        "context-capability-path-token",
    ]);
    let transparent = BTreeSet::from(["source-mec-path-wildcard-suffix"]);
    let expected_names = node_policies
        .iter()
        .map(|(name, _)| *name)
        .chain(tokens.iter().copied())
        .chain(transparent.iter().copied())
        .collect::<BTreeSet<_>>();
    assert_eq!(node_policies.len(), 17);
    assert_eq!(tokens.len(), 3);
    assert_eq!(transparent.len(), 1);
    assert_eq!(expected_names.len(), EXPECTED_PHASE_2F);

    let phase_2f = CANONICAL_PORTS
        .iter()
        .filter(|port| port.phase == Some(PortPhase::Phase2F))
        .collect::<Vec<_>>();
    assert_eq!(phase_2f.len(), EXPECTED_PHASE_2F);
    assert_eq!(
        phase_2f
            .iter()
            .map(|port| port.name)
            .collect::<BTreeSet<_>>(),
        expected_names
    );
    for port in &phase_2f {
        assert_eq!(port.syntax, SyntaxPortStatus::ParityVerified, "{}", port.name);
        if let Some((_, kind)) = node_policies.iter().find(|(name, _)| *name == port.name) {
            assert_eq!(policy_name(port.node_policy), format!("node:{kind}"));
            assert_eq!(port.lowering, LoweringPortStatus::ParityVerified);
        } else if tokens.contains(port.name) {
            assert_eq!(port.node_policy, NodePolicy::Token);
            assert_eq!(port.lowering, LoweringPortStatus::NotApplicable);
        } else {
            assert!(transparent.contains(port.name), "{}", port.name);
            assert_eq!(port.node_policy, NodePolicy::Transparent);
            assert_eq!(port.lowering, LoweringPortStatus::NotApplicable);
        }
    }
    assert_eq!(
        phase_2f
            .iter()
            .filter(|port| port.lowering == LoweringPortStatus::ParityVerified)
            .count(),
        17
    );

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
    for row in lines.map(fields) {
        if !expected_names.contains(row[grammar_name]) {
            continue;
        }
        for child in row[child_rules].split(',') {
            if child.is_empty() || child == "none" {
                continue;
            }
            let child_port = CANONICAL_PORTS
                .iter()
                .find(|port| port.name == child)
                .unwrap_or_else(|| panic!("{} has unknown canonical child {child}", row[grammar_name]));
            assert_ne!(
                child_port.syntax,
                SyntaxPortStatus::Unported,
                "{} has unported canonical child {child}",
                row[grammar_name]
            );
        }
    }
}

#[test]
fn phase_2g_registry_accounting_and_policies_are_exact() {
    let node_policies = [
        ("select-all", "SelectAllSubscript"),
        ("swizzle-subscript", "SwizzleSubscript"),
        ("dot-subscript", "DotSubscript"),
        ("dot-subscript-int", "DotSubscriptInt"),
        ("wildcard", "WildcardPattern"),
        ("op-assign-operator", "OpAssignOperator"),
        ("add-assign-operator", "AddAssignOperation"),
        ("sub-assign-operator", "SubAssignOperation"),
        ("mul-assign-operator", "MulAssignOperation"),
        ("div-assign-operator", "DivAssignOperation"),
        ("exp-assign-operator", "ExpAssignOperation"),
    ];
    let transparent = BTreeSet::from([
        "statement-separator",
        "spread-operator",
        "send-operator",
        "guard-operator",
    ]);
    let expected_names = node_policies
        .iter()
        .map(|(name, _)| *name)
        .chain(transparent.iter().copied())
        .collect::<BTreeSet<_>>();
    assert_eq!(node_policies.len(), 11);
    assert_eq!(transparent.len(), 4);
    assert_eq!(expected_names.len(), EXPECTED_PHASE_2G);

    let phase_2g = CANONICAL_PORTS
        .iter()
        .filter(|port| port.phase == Some(PortPhase::Phase2G))
        .collect::<Vec<_>>();
    assert_eq!(phase_2g.len(), EXPECTED_PHASE_2G);
    assert_eq!(
        phase_2g
            .iter()
            .map(|port| port.name)
            .collect::<BTreeSet<_>>(),
        expected_names,
    );
    for port in phase_2g {
        assert_eq!(port.syntax, SyntaxPortStatus::ParityVerified, "{}", port.name);
        if let Some((_, kind)) = node_policies.iter().find(|(name, _)| *name == port.name) {
            assert_eq!(policy_name(port.node_policy), format!("node:{kind}"));
            assert_eq!(port.lowering, LoweringPortStatus::ParityVerified);
        } else {
            assert!(transparent.contains(port.name), "{}", port.name);
            assert_eq!(port.node_policy, NodePolicy::Transparent);
            assert_eq!(port.lowering, LoweringPortStatus::NotApplicable);
        }
    }
}

#[test]
fn phase_2c_closed_dependencies_are_all_already_ported() {
    let dependencies: &[(&str, &[&str])] = &[
        ("empty", &["underscore"]),
        ("atom", &["colon", "identifier"]),
        ("string", &["raw-string", "utf8-string"]),
        ("utf8-string", &["quote", "text", "new-line"]),
        ("raw-string", &["quote", "raw-text", "new-line"]),
        ("boolean", &["true-literal", "false-literal"]),
        ("true-literal", &["english-true-literal", "check-mark"]),
        ("false-literal", &["english-false-literal", "cross"]),
        ("number", &["complex-number", "real-number"]),
        ("complex-number", &["untyped-real-number", "tag"]),
        (
            "real-number",
            &[
                "dash",
                "hexadecimal-literal",
                "decimal-literal",
                "octal-literal",
                "binary-literal",
                "scientific-literal",
                "rational-literal",
                "float-literal",
                "integer-literal",
            ],
        ),
        (
            "untyped-real-number",
            &[
                "dash",
                "hexadecimal-literal",
                "decimal-literal",
                "octal-literal",
                "binary-literal",
                "scientific-literal",
                "rational-literal",
                "float-literal",
                "untyped-integer",
            ],
        ),
        ("rational-literal", &["integer-literal", "slash"]),
        (
            "scientific-literal",
            &["float-literal", "integer-literal", "tag"],
        ),
        ("float-decimal-start", &["period", "digit-sequence"]),
        ("float-full", &["digit-sequence", "period"]),
        (
            "float-literal",
            &["float-decimal-start", "float-full"],
        ),
        ("integer-literal", &["typed-integer", "untyped-integer"]),
        ("typed-integer", &["digit-sequence", "identifier"]),
        ("untyped-integer", &["digit-sequence"]),
        ("decimal-literal", &["tag", "digit-sequence"]),
        (
            "hexadecimal-literal",
            &["tag", "digit-token", "underscore", "alpha-token"],
        ),
        ("octal-literal", &["tag", "digit-sequence"]),
        ("binary-literal", &["tag", "digit-sequence"]),
        (
            "context-address-path-token",
            &[
                "alpha-token",
                "digit-token",
                "dash",
                "slash",
                "underscore",
                "period",
            ],
        ),
        (
            "context-address-path",
            &["context-address-path-token"],
        ),
        (
            "prefixed-context-path",
            &[
                "at",
                "identifier-path-segment",
                "slash",
                "context-address-path",
            ],
        ),
        ("kind-any", &["asterisk"]),
        ("kind-empty", &["underscore"]),
        ("kind-atom", &["colon", "identifier"]),
    ];
    assert_eq!(dependencies.len(), EXPECTED_PHASE_2C);

    for (name, children) in dependencies {
        let parent = CANONICAL_PORTS
            .iter()
            .find(|port| port.name == *name)
            .unwrap_or_else(|| panic!("missing canonical port entry {name}"));
        assert_eq!(parent.phase, Some(PortPhase::Phase2C), "{name}");
        for child in *children {
            let child_port = CANONICAL_PORTS
                .iter()
                .find(|port| port.name == *child)
                .unwrap_or_else(|| panic!("{name} has unknown canonical child {child}"));
            assert_ne!(
                child_port.syntax,
                SyntaxPortStatus::Unported,
                "{name} has unported canonical child {child}"
            );
        }
    }
}

#[test]
fn phase_2b_parent_and_rich_document_rules_remain_unported() {
    for name in [
        "inline-paragraph",
        "paragraph-element",
        "paragraph",
        "paragraph-newline",
        "title",
        "title-front-matter",
        "subtitle",
        "ul-subtitle",
        "code-block",
        "section-element",
        "section",
        "body",
        "program",
        "parse-mech",
        "parse",
    ] {
        let port = CANONICAL_PORTS
            .iter()
            .find(|port| port.name == name)
            .unwrap_or_else(|| panic!("missing canonical port entry {name}"));
        assert_eq!(port.syntax, SyntaxPortStatus::Unported, "{name}");
        assert_eq!(port.phase, None, "{name}");
    }
}

#[test]
fn phase_2c_recursive_parent_rules_remain_unported() {
    for name in [
        "literal",
        "var",
        "kind",
        "kind-annotation",
        "kind-with-option",
        "kind-kind",
        "kind-table",
        "kind-set",
        "kind-map",
        "kind-record",
        "kind-matrix",
        "kind-tuple",
        "kind-scalar",
        "range-expression",
        "formula",
        "factor",
        "expression",
    ] {
        let port = CANONICAL_PORTS
            .iter()
            .find(|port| port.name == name)
            .unwrap_or_else(|| panic!("missing canonical port entry {name}"));
        assert_eq!(port.syntax, SyntaxPortStatus::Unported, "{name}");
        assert_eq!(port.phase, None, "{name}");
        assert_eq!(port.node_policy, NodePolicy::Undecided, "{name}");
        assert_eq!(port.lowering, LoweringPortStatus::Pending, "{name}");
    }
}

#[test]
fn phase_2d_expression_and_related_parent_rules_remain_unported() {
    let deferred = [
        "expression",
        "match-expression",
        "match-arm",
        "formula",
        "l1",
        "l2",
        "l3",
        "l4",
        "l5",
        "l6",
        "l7",
        "factor",
        "parenthetical-term",
        "negate-factor",
        "not-factor",
        "range-expression",
        "literal",
        "kind",
        "kind-annotation",
        "kind-with-option",
        "var",
        "structure",
        "matrix",
        "table",
        "tuple",
        "tuple-struct",
        "record",
        "map",
        "set",
        "empty-map",
        "empty-set",
        "function-call",
        "argument-list",
        "call-arg",
        "call-arg-with-binding",
        "set-comprehension",
        "matrix-comprehension",
        "comprehension-qualifier",
        "generator",
        "pattern",
        "pattern-array",
        "pattern-tuple",
        "pattern-atom-struct",
        "pattern-tuple-struct",
        "subscript",
        "slice",
        "slice-ref",
        "formula-subscript",
        "range-subscript",
        "fsm-pipe",
        "fsm-instance",
    ];
    assert_eq!(deferred.len(), 51);

    for name in deferred {
        let port = CANONICAL_PORTS
            .iter()
            .find(|port| port.name == name)
            .unwrap_or_else(|| panic!("missing canonical port entry {name}"));
        assert_eq!(port.syntax, SyntaxPortStatus::Unported, "{name}");
        assert_eq!(port.phase, None, "{name}");
        assert_eq!(port.node_policy, NodePolicy::Undecided, "{name}");
        assert_eq!(port.lowering, LoweringPortStatus::Pending, "{name}");
    }
}

#[test]
fn phase_2g_parent_rules_remain_unported() {
    for name in [
        "subscript",
        "bracket-subscript",
        "brace-subscript",
        "formula-subscript",
        "range-subscript",
        "slice",
        "slice-ref",
        "pattern",
        "pattern-array",
        "pattern-array-item",
        "pattern-array-token",
        "pattern-tuple",
        "pattern-tuple-struct",
        "pattern-atom-struct",
        "context-send",
        "op-assign",
        "variable-assign",
        "variable-define",
        "tuple-destructure",
        "statement",
        "match-arm",
        "match-expression",
        "fsm-guard",
        "fsm-state-definition",
        "fsm-transition",
        "activation-arm",
        "formula",
        "factor",
        "expression",
    ] {
        let port = CANONICAL_PORTS
            .iter()
            .find(|port| port.name == name)
            .unwrap_or_else(|| panic!("missing canonical port entry {name}"));
        assert_eq!(port.syntax, SyntaxPortStatus::Unported, "{name}");
        assert_eq!(port.phase, None, "{name}");
        assert_eq!(port.node_policy, NodePolicy::Undecided, "{name}");
        assert_eq!(port.lowering, LoweringPortStatus::Pending, "{name}");
    }
}
