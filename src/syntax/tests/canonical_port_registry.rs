use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use mech_syntax::document::parser::{
    CANONICAL_PORT_COUNT, CANONICAL_PORTS, CANONICAL_RULE_COUNT, CANONICAL_RULES, NodePolicy,
    PortPhase, RuleFamily, SemanticPortStatus, SyntaxPortStatus, canonical_rule_id,
};

const EXPECTED_RULES: usize = 539;
const EXPECTED_PHASE_2A: usize = 167;
const EXPECTED_PHASE_2B: usize = 13;
const EXPECTED_PHASE_2C: usize = 30;
const EXPECTED_PHASE_2D: usize = 53;
const EXPECTED_PHASE_2E: usize = 19;
const EXPECTED_PHASE_2F: usize = 21;
const EXPECTED_PHASE_2G: usize = 15;
const EXPECTED_PHASE_2H: usize = 10;
const EXPECTED_PHASE_2I: usize = 80;
const EXPECTED_S7: usize = 112;
const EXPECTED_CERTIFIED: usize = 520;
const EXPECTED_UNPORTED: usize = 19;

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
        SyntaxPortStatus::Certified => "certified",
    }
}

fn semantic_name(status: SemanticPortStatus) -> &'static str {
    match status {
        SemanticPortStatus::SyntaxOnly => "syntax-only",
        SemanticPortStatus::Pending => "pending",
        SemanticPortStatus::Certified => "certified",
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
        Some(PortPhase::Phase2H) => "2H",
        Some(PortPhase::Phase2I) => "2I",
        Some(PortPhase::S7) => "S7",
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
            "grammar-name\tfamily\tsyntax-status\tsemantic-status\t\
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
        assert_eq!(semantic_name(generated.semantic), row[3]);
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
            .all(|port| port.syntax == SyntaxPortStatus::Certified)
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
fn phase_2a_node_and_semantic_policies_are_exact() {
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
            assert_eq!(port.semantic, SemanticPortStatus::Certified);
        } else if transparent.contains(port.name) {
            assert_eq!(port.node_policy, NodePolicy::Transparent);
            assert_eq!(port.semantic, SemanticPortStatus::SyntaxOnly);
        } else {
            assert_eq!(port.node_policy, NodePolicy::Token);
            assert_eq!(port.semantic, SemanticPortStatus::SyntaxOnly);
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
        assert_eq!(port.semantic, SemanticPortStatus::Pending);
    }
}

#[test]
fn phase_2b_registry_accounting_and_policies_are_exact() {
    let expected = BTreeMap::from([
        (
            "blank-line",
            (SemanticPortStatus::SyntaxOnly, "node:BlankLine"),
        ),
        (
            "codeblock-sigil",
            (SemanticPortStatus::SyntaxOnly, "transparent"),
        ),
        ("comment", (SemanticPortStatus::SyntaxOnly, "node:Comment")),
        (
            "comment-sigil",
            (SemanticPortStatus::SyntaxOnly, "transparent"),
        ),
        ("equation", (SemanticPortStatus::Certified, "node:Equation")),
        (
            "footnote-reference",
            (SemanticPortStatus::Certified, "node:FootnoteReference"),
        ),
        (
            "inline-code",
            (SemanticPortStatus::Certified, "node:InlineCode"),
        ),
        (
            "inline-equation",
            (SemanticPortStatus::Certified, "node:InlineEquation"),
        ),
        (
            "paragraph-text",
            (SemanticPortStatus::Certified, "node:ParagraphText"),
        ),
        (
            "raw-hyperlink",
            (SemanticPortStatus::Certified, "node:RawHyperlink"),
        ),
        (
            "reference",
            (SemanticPortStatus::Certified, "node:Reference"),
        ),
        (
            "section-reference",
            (SemanticPortStatus::Certified, "node:SectionReference"),
        ),
        (
            "thematic-break",
            (SemanticPortStatus::Certified, "node:ThematicBreak"),
        ),
    ]);
    assert_eq!(expected.len(), EXPECTED_PHASE_2B);

    let phase_2b = CANONICAL_PORTS
        .iter()
        .filter(|port| port.phase == Some(PortPhase::Phase2B))
        .collect::<Vec<_>>();
    assert_eq!(phase_2b.len(), EXPECTED_PHASE_2B);
    for port in phase_2b {
        let (semantic, policy) = expected
            .get(port.name)
            .unwrap_or_else(|| panic!("unexpected Phase 2B rule {}", port.name));
        assert_eq!(port.syntax, SyntaxPortStatus::Certified);
        assert_eq!(port.semantic, *semantic);
        assert_eq!(policy_name(port.node_policy), *policy);
    }

    let certified = CANONICAL_PORTS
        .iter()
        .filter(|port| port.syntax != SyntaxPortStatus::Unported)
        .collect::<Vec<_>>();
    assert_eq!(certified.len(), EXPECTED_CERTIFIED);
    assert_eq!(CANONICAL_PORTS.len() - certified.len(), EXPECTED_UNPORTED);
    assert_eq!(
        CANONICAL_PORTS
            .iter()
            .filter(|port| port.syntax == SyntaxPortStatus::Certified)
            .count(),
        EXPECTED_CERTIFIED
    );
    assert_eq!(
        certified
            .iter()
            .filter(|port| port.phase == Some(PortPhase::Phase2A))
            .count(),
        EXPECTED_PHASE_2A
    );
    assert_eq!(
        certified
            .iter()
            .filter(|port| port.phase == Some(PortPhase::Phase2B))
            .count(),
        EXPECTED_PHASE_2B
    );
    assert_eq!(
        certified
            .iter()
            .filter(|port| port.phase == Some(PortPhase::Phase2C))
            .count(),
        EXPECTED_PHASE_2C
    );
    assert_eq!(
        certified
            .iter()
            .filter(|port| port.phase == Some(PortPhase::Phase2D))
            .count(),
        EXPECTED_PHASE_2D
    );
    assert_eq!(
        certified
            .iter()
            .filter(|port| port.phase == Some(PortPhase::Phase2E))
            .count(),
        EXPECTED_PHASE_2E
    );
    assert_eq!(
        certified
            .iter()
            .filter(|port| port.phase == Some(PortPhase::Phase2F))
            .count(),
        EXPECTED_PHASE_2F
    );
    assert_eq!(
        certified
            .iter()
            .filter(|port| port.phase == Some(PortPhase::Phase2G))
            .count(),
        EXPECTED_PHASE_2G
    );
    assert_eq!(
        certified
            .iter()
            .filter(|port| port.phase == Some(PortPhase::Phase2H))
            .count(),
        EXPECTED_PHASE_2H
    );
    assert_eq!(
        certified
            .iter()
            .filter(|port| port.phase == Some(PortPhase::Phase2I))
            .count(),
        EXPECTED_PHASE_2I
    );
    assert_eq!(
        certified
            .iter()
            .filter(|port| port.phase == Some(PortPhase::S7))
            .count(),
        EXPECTED_S7
    );
    assert_eq!(
        CANONICAL_PORTS
            .iter()
            .filter(|port| port.semantic == SemanticPortStatus::Certified)
            .count(),
        238
    );
    assert_eq!(
        CANONICAL_PORTS
            .iter()
            .filter(|port| port.semantic == SemanticPortStatus::Pending)
            .count(),
        EXPECTED_UNPORTED
    );
    assert_eq!(
        CANONICAL_PORTS
            .iter()
            .filter(|port| port.semantic == SemanticPortStatus::SyntaxOnly)
            .count(),
        282
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
    let expected_names = node_policies
        .iter()
        .map(|(name, _)| *name)
        .chain(token_rules)
        .collect::<BTreeSet<_>>();
    assert_eq!(expected_names.len(), EXPECTED_PHASE_2C);
    assert_eq!(node_policies.len(), 26);
    assert_eq!(token_rules.len(), 4);

    let phase_2c = CANONICAL_PORTS
        .iter()
        .filter(|port| port.phase == Some(PortPhase::Phase2C))
        .collect::<Vec<_>>();
    assert_eq!(phase_2c.len(), EXPECTED_PHASE_2C);
    assert_eq!(
        phase_2c
            .iter()
            .map(|port| port.name)
            .collect::<BTreeSet<_>>(),
        expected_names
    );

    for port in phase_2c {
        if let Some((_, kind)) = node_policies.iter().find(|(name, _)| *name == port.name) {
            assert_eq!(policy_name(port.node_policy), format!("node:{kind}"));
            assert_eq!(port.syntax, SyntaxPortStatus::Certified, "{}", port.name);
            assert_eq!(
                port.semantic,
                SemanticPortStatus::Certified,
                "{}",
                port.name
            );
        } else {
            assert!(token_rules.contains(&port.name), "{}", port.name);
            assert_eq!(port.syntax, SyntaxPortStatus::Certified, "{}", port.name);
            assert_eq!(
                port.semantic,
                SemanticPortStatus::SyntaxOnly,
                "{}",
                port.name
            );
            assert_eq!(port.node_policy, NodePolicy::Token, "{}", port.name);
        }
    }

    assert_eq!(
        CANONICAL_PORTS
            .iter()
            .filter(|port| {
                port.phase == Some(PortPhase::Phase2C) && port.syntax == SyntaxPortStatus::Certified
            })
            .count(),
        EXPECTED_PHASE_2C
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
        phase_2d
            .iter()
            .map(|port| port.name)
            .collect::<BTreeSet<_>>(),
        expected_names
    );

    for port in phase_2d {
        assert_eq!(port.syntax, SyntaxPortStatus::Certified, "{}", port.name);
        if let Some((_, kind)) = node_policies.iter().find(|(name, _)| *name == port.name) {
            assert_eq!(policy_name(port.node_policy), format!("node:{kind}"));
            assert_eq!(port.semantic, SemanticPortStatus::Certified);
        } else {
            assert_eq!(port.name, "transpose");
            assert_eq!(port.node_policy, NodePolicy::Transparent);
            assert_eq!(port.semantic, SemanticPortStatus::SyntaxOnly);
        }
    }

    assert_eq!(
        CANONICAL_PORTS
            .iter()
            .filter(|port| {
                port.phase == Some(PortPhase::Phase2D)
                    && port.semantic == SemanticPortStatus::Certified
            })
            .count(),
        52
    );
    assert_eq!(
        CANONICAL_PORTS
            .iter()
            .filter(|port| {
                port.phase == Some(PortPhase::Phase2D)
                    && port.semantic == SemanticPortStatus::Pending
            })
            .count(),
        0
    );
    assert_eq!(
        CANONICAL_PORTS
            .iter()
            .filter(|port| {
                port.phase == Some(PortPhase::Phase2D)
                    && port.semantic == SemanticPortStatus::SyntaxOnly
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
        assert_eq!(port.syntax, SyntaxPortStatus::Certified, "{}", port.name);
        if let Some((_, kind)) = node_policies.iter().find(|(name, _)| *name == port.name) {
            assert_eq!(policy_name(port.node_policy), format!("node:{kind}"));
            assert_eq!(port.semantic, SemanticPortStatus::Certified);
        } else {
            assert!(transparent.contains(port.name), "{}", port.name);
            assert_eq!(port.node_policy, NodePolicy::Transparent);
            assert_eq!(port.semantic, SemanticPortStatus::SyntaxOnly);
        }
    }

    assert_eq!(
        phase_2e
            .iter()
            .filter(|port| port.semantic == SemanticPortStatus::Certified)
            .count(),
        17
    );
    assert_eq!(
        phase_2e
            .iter()
            .filter(|port| port.semantic == SemanticPortStatus::Pending)
            .count(),
        0
    );
    assert_eq!(
        phase_2e
            .iter()
            .filter(|port| port.semantic == SemanticPortStatus::SyntaxOnly)
            .count(),
        2
    );

    let import_sigil = CANONICAL_PORTS
        .iter()
        .find(|port| port.name == "import-sigil")
        .expect("import-sigil port");
    assert_eq!(import_sigil.phase, Some(PortPhase::Phase2A));
    assert_eq!(import_sigil.syntax, SyntaxPortStatus::Certified);
    assert_eq!(import_sigil.semantic, SemanticPortStatus::SyntaxOnly);
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
        assert_eq!(port.syntax, SyntaxPortStatus::Certified, "{}", port.name);
        if let Some((_, kind)) = node_policies.iter().find(|(name, _)| *name == port.name) {
            assert_eq!(policy_name(port.node_policy), format!("node:{kind}"));
            assert_eq!(port.semantic, SemanticPortStatus::Certified);
        } else if tokens.contains(port.name) {
            assert_eq!(port.node_policy, NodePolicy::Token);
            assert_eq!(port.semantic, SemanticPortStatus::SyntaxOnly);
        } else {
            assert!(transparent.contains(port.name), "{}", port.name);
            assert_eq!(port.node_policy, NodePolicy::Transparent);
            assert_eq!(port.semantic, SemanticPortStatus::SyntaxOnly);
        }
    }
    assert_eq!(
        phase_2f
            .iter()
            .filter(|port| port.semantic == SemanticPortStatus::Certified)
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
        assert_eq!(port.syntax, SyntaxPortStatus::Certified, "{}", port.name);
        if let Some((_, kind)) = node_policies.iter().find(|(name, _)| *name == port.name) {
            assert_eq!(policy_name(port.node_policy), format!("node:{kind}"));
            assert_eq!(port.semantic, SemanticPortStatus::Certified);
        } else {
            assert!(transparent.contains(port.name), "{}", port.name);
            assert_eq!(port.node_policy, NodePolicy::Transparent);
            assert_eq!(port.semantic, SemanticPortStatus::SyntaxOnly);
        }
    }
}

#[test]
fn phase_2h_registry_accounting_and_policies_are_exact() {
    let node_policies = [
        ("row-separator", "TableRowSeparator"),
        ("empty-map", "EmptyMap"),
        ("empty-set", "EmptySet"),
    ];
    let token = BTreeSet::from([
        "matrix-start",
        "matrix-end",
        "table-start",
        "table-end",
        "table-separator",
        "table-horz",
    ]);
    let transparent = BTreeSet::from(["table-top"]);
    let expected_names = node_policies
        .iter()
        .map(|(name, _)| *name)
        .chain(token.iter().copied())
        .chain(transparent.iter().copied())
        .collect::<BTreeSet<_>>();
    assert_eq!(node_policies.len(), 3);
    assert_eq!(token.len(), 6);
    assert_eq!(transparent.len(), 1);
    assert_eq!(expected_names.len(), EXPECTED_PHASE_2H);

    let phase_2h = CANONICAL_PORTS
        .iter()
        .filter(|port| port.phase == Some(PortPhase::Phase2H))
        .collect::<Vec<_>>();
    assert_eq!(phase_2h.len(), EXPECTED_PHASE_2H);
    assert_eq!(
        phase_2h
            .iter()
            .map(|port| port.name)
            .collect::<BTreeSet<_>>(),
        expected_names,
    );
    for port in phase_2h {
        assert_eq!(port.syntax, SyntaxPortStatus::Certified, "{}", port.name);
        if let Some((_, kind)) = node_policies.iter().find(|(name, _)| *name == port.name) {
            assert_eq!(policy_name(port.node_policy), format!("node:{kind}"));
            assert_eq!(port.semantic, SemanticPortStatus::Certified);
        } else if token.contains(port.name) {
            assert_eq!(port.node_policy, NodePolicy::Token);
            assert_eq!(port.semantic, SemanticPortStatus::SyntaxOnly);
        } else {
            assert!(transparent.contains(port.name), "{}", port.name);
            assert_eq!(port.node_policy, NodePolicy::Transparent);
            assert_eq!(port.semantic, SemanticPortStatus::SyntaxOnly);
        }
    }
}

#[test]
fn phase_2i_registry_matches_the_certified_recursive_core() {
    let audit_root = repository_root().join("docs/design/grammar-audit");
    let schema_source = fs::read_to_string(audit_root.join("phase-2i-syntax-schema.tsv"))
        .expect("read phase-2i-syntax-schema.tsv");
    let mut schema_lines = schema_source.lines();
    assert_eq!(
        schema_lines.next(),
        Some("grammar-name\tparser-module\temission-policy\tsyntax-kind\tkind-origin\tnotes")
    );
    let schema = schema_lines
        .map(fields)
        .map(|row| {
            assert_eq!(row.len(), 6);
            (row[0], (row[2], row[3]))
        })
        .collect::<BTreeMap<_, _>>();

    let certification_source = fs::read_to_string(audit_root.join("phase-2i-certification.tsv"))
        .expect("read phase-2i-certification.tsv");
    let mut certification_lines = certification_source.lines();
    let certification_header = fields(
        certification_lines
            .next()
            .expect("phase-2i-certification.tsv header"),
    );
    let disposition = certification_header
        .iter()
        .position(|column| *column == "semantic-disposition")
        .unwrap();
    let certification = certification_lines
        .map(fields)
        .map(|row| (row[0], row[disposition]))
        .collect::<BTreeMap<_, _>>();

    let phase_source = fs::read_to_string(audit_root.join("phase-2i-recursive-core.tsv"))
        .expect("read phase-2i-recursive-core.tsv");
    let phase_names = phase_source
        .lines()
        .skip(1)
        .map(fields)
        .map(|row| row[0])
        .collect::<BTreeSet<_>>();
    let ports = CANONICAL_PORTS
        .iter()
        .filter(|port| port.phase == Some(PortPhase::Phase2I))
        .collect::<Vec<_>>();
    let port_names = ports.iter().map(|port| port.name).collect::<BTreeSet<_>>();

    assert_eq!(ports.len(), EXPECTED_PHASE_2I);
    assert_eq!(schema.len(), EXPECTED_PHASE_2I);
    assert_eq!(certification.len(), EXPECTED_PHASE_2I);
    assert_eq!(port_names, phase_names);
    assert_eq!(port_names, schema.keys().copied().collect());
    assert_eq!(port_names, certification.keys().copied().collect());

    for port in ports {
        let (emission, kind) = schema[port.name];
        assert!(matches!(
            emission,
            "node" | "conditional-node" | "transparent"
        ));
        assert_eq!(port.syntax, SyntaxPortStatus::Certified);
        assert_eq!(port.semantic, SemanticPortStatus::Certified);
        if emission == "transparent" {
            assert_eq!(port.node_policy, NodePolicy::Transparent);
        } else {
            assert_eq!(policy_name(port.node_policy), format!("node:{kind}"));
        }
        let disposition = certification[port.name];
        assert!(matches!(
            disposition,
            "executable" | "structural" | "compile-time"
        ));
        assert_eq!(
            port.notes,
            format!("Phase 2I canonical recursive core; semantic disposition: {disposition}.")
        );
    }
}

#[test]
fn phase_2c_closed_dependencies_are_all_certified() {
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
        ("float-literal", &["float-decimal-start", "float-full"]),
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
        ("context-address-path", &["context-address-path-token"]),
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
fn s7_activates_the_rich_document_parent_closure() {
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
        assert_eq!(port.syntax, SyntaxPortStatus::Certified, "{name}");
        assert_eq!(port.phase, Some(PortPhase::S7), "{name}");
    }
}

#[test]
fn s7_activates_the_remaining_document_boundaries() {
    let remaining = [
        "slice-ref",
        "context-send",
        "op-assign",
        "variable-assign",
        "tuple-destructure",
        "statement",
        "fsm-guard",
        "fsm-state-definition",
        "fsm-transition",
        "activation-arm",
    ];
    assert_eq!(remaining.len(), 10);
    for name in remaining {
        let port = CANONICAL_PORTS
            .iter()
            .find(|port| port.name == name)
            .unwrap_or_else(|| panic!("missing canonical port entry {name}"));
        assert_eq!(port.syntax, SyntaxPortStatus::Certified, "{name}");
        assert_eq!(port.phase, Some(PortPhase::S7), "{name}");
        assert_ne!(port.node_policy, NodePolicy::Undecided, "{name}");
        assert_ne!(port.semantic, SemanticPortStatus::Pending, "{name}");
    }
}

#[test]
fn s7_does_not_activate_rules_outside_document_reachability() {
    for name in ["match-expression", "table-column"] {
        let port = CANONICAL_PORTS
            .iter()
            .find(|port| port.name == name)
            .unwrap_or_else(|| panic!("missing canonical port entry {name}"));
        assert_eq!(port.syntax, SyntaxPortStatus::Unported, "{name}");
        assert_eq!(port.semantic, SemanticPortStatus::Pending, "{name}");
        assert_eq!(port.node_policy, NodePolicy::Undecided, "{name}");
        assert_eq!(port.phase, None, "{name}");
    }
}
