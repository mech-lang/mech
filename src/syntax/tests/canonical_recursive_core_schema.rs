use mech_syntax::document::SyntaxKind;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

const EXPECTED_SCHEMA_ROWS: usize = 80;
const SCHEMA_HEADER: &str =
    "grammar-name\tparser-module\temission-policy\tsyntax-kind\tkind-origin\tnotes";
const PHASE_HEADER: &str = "grammar-name\tfamily\tcomponent-id\tcomponent-size\t\
                            recursive-component\tsame-component-children\tclosure-children\t\
                            ported-external-children";
const PORTS_HEADER: &str = "grammar-name\tfamily\tsyntax-status\tlowering-status\t\
                            node-policy\tphase\tnotes";

const PHASE_2I_NEW_KINDS: &[SyntaxKind] = &[
    SyntaxKind::Literal,
    SyntaxKind::Kind,
    SyntaxKind::KindWithOption,
    SyntaxKind::KindKind,
    SyntaxKind::KindTable,
    SyntaxKind::KindSet,
    SyntaxKind::KindMap,
    SyntaxKind::KindRecord,
    SyntaxKind::KindMatrix,
    SyntaxKind::KindTuple,
    SyntaxKind::KindScalar,
    SyntaxKind::Variable,
    SyntaxKind::Slice,
    SyntaxKind::SubscriptList,
    SyntaxKind::BracketSubscript,
    SyntaxKind::BraceSubscript,
    SyntaxKind::FormulaSubscript,
    SyntaxKind::RangeSubscript,
    SyntaxKind::Structure,
    SyntaxKind::Matrix,
    SyntaxKind::MatrixRow,
    SyntaxKind::MatrixColumn,
    SyntaxKind::Table,
    SyntaxKind::FancyTable,
    SyntaxKind::FancyTableHeader,
    SyntaxKind::FancyTableRow,
    SyntaxKind::InlineTable,
    SyntaxKind::InlineTableHeader,
    SyntaxKind::InlineTableRow,
    SyntaxKind::RegularTable,
    SyntaxKind::TableHeader,
    SyntaxKind::TableRow,
    SyntaxKind::HeaderField,
    SyntaxKind::TableField,
    SyntaxKind::Map,
    SyntaxKind::MapEntry,
    SyntaxKind::Record,
    SyntaxKind::RecordBinding,
    SyntaxKind::Set,
    SyntaxKind::Tuple,
    SyntaxKind::TupleStruct,
    SyntaxKind::FunctionCall,
    SyntaxKind::ArgumentList,
    SyntaxKind::CallArgument,
    SyntaxKind::BoundCallArgument,
    SyntaxKind::Pattern,
    SyntaxKind::ArrayPattern,
    SyntaxKind::ArrayPatternElement,
    SyntaxKind::AtomStructPattern,
    SyntaxKind::TuplePattern,
    SyntaxKind::TupleStructPattern,
    SyntaxKind::ComprehensionQualifier,
    SyntaxKind::Generator,
    SyntaxKind::SetComprehension,
    SyntaxKind::MatrixComprehension,
    SyntaxKind::FsmPipe,
    SyntaxKind::FsmInstance,
    SyntaxKind::FsmArguments,
    SyntaxKind::FsmValue,
    SyntaxKind::FsmStateTransition,
    SyntaxKind::FsmAsyncTransition,
    SyntaxKind::FsmOutput,
    SyntaxKind::Factor,
    SyntaxKind::NegateFactor,
    SyntaxKind::NotFactor,
    SyntaxKind::RangeExpression,
    SyntaxKind::MatchArm,
    SyntaxKind::LogicExpression,
    SyntaxKind::ComparisonExpression,
    SyntaxKind::MultiplicativeExpression,
    SyntaxKind::PowerExpression,
    SyntaxKind::TableExpression,
    SyntaxKind::SetExpression,
];

#[derive(Debug)]
struct SchemaRow {
    module: String,
    policy: String,
    kind: String,
    origin: String,
}

#[derive(Debug)]
struct PortRow {
    syntax: String,
    lowering: String,
    policy: String,
    phase: String,
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fields(line: &str) -> Vec<&str> {
    line.split('\t').collect()
}

fn names(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn schema() -> BTreeMap<String, SchemaRow> {
    let source = fs::read_to_string(
        repository_root().join("docs/design/grammar-audit/phase-2i-syntax-schema.tsv"),
    )
    .expect("read phase-2i-syntax-schema.tsv");
    let mut lines = source.lines();
    assert_eq!(lines.next(), Some(SCHEMA_HEADER));
    let mut previous = String::new();
    let mut rows = BTreeMap::new();
    for (index, line) in lines.enumerate() {
        assert!(!line.is_empty(), "blank schema row {}", index + 2);
        let row = fields(line);
        assert_eq!(row.len(), 6, "invalid schema row {}", index + 2);
        assert!(row[0] > previous.as_str(), "schema rows are not ordered");
        assert!(!row[5].is_empty(), "empty schema note on row {}", index + 2);
        previous = row[0].to_owned();
        let value = SchemaRow {
            module: row[1].to_owned(),
            policy: row[2].to_owned(),
            kind: row[3].to_owned(),
            origin: row[4].to_owned(),
        };
        assert!(rows.insert(row[0].to_owned(), value).is_none());
    }
    assert_eq!(rows.len(), EXPECTED_SCHEMA_ROWS);
    rows
}

fn phase_names() -> BTreeSet<String> {
    let source = fs::read_to_string(
        repository_root().join("docs/design/grammar-audit/phase-2i-recursive-core.tsv"),
    )
    .expect("read phase-2i-recursive-core.tsv");
    let mut lines = source.lines();
    assert_eq!(lines.next(), Some(PHASE_HEADER));
    let mut previous = String::new();
    let mut result = BTreeSet::new();
    for (index, line) in lines.enumerate() {
        let row = fields(line);
        assert_eq!(row.len(), 8, "invalid Phase 2I row {}", index + 2);
        assert!(row[0] > previous.as_str(), "Phase 2I rows are not ordered");
        previous = row[0].to_owned();
        assert!(result.insert(row[0].to_owned()));
    }
    assert_eq!(result.len(), EXPECTED_SCHEMA_ROWS);
    result
}

fn ports() -> BTreeMap<String, PortRow> {
    let source =
        fs::read_to_string(repository_root().join("docs/design/grammar-audit/ports.tsv"))
            .expect("read ports.tsv");
    let mut lines = source.lines();
    assert_eq!(lines.next(), Some(PORTS_HEADER));
    let mut rows = BTreeMap::new();
    for (index, line) in lines.enumerate() {
        let row = fields(line);
        assert_eq!(row.len(), 7, "invalid ports row {}", index + 2);
        assert!(
            rows.insert(
                row[0].to_owned(),
                PortRow {
                    syntax: row[2].to_owned(),
                    lowering: row[3].to_owned(),
                    policy: row[4].to_owned(),
                    phase: row[5].to_owned(),
                },
            )
            .is_none()
        );
    }
    rows
}

#[test]
fn schema_inventory_and_categorical_contracts_are_exact() {
    let schema = schema();
    assert_eq!(schema.keys().cloned().collect::<BTreeSet<_>>(), phase_names());

    let allowed_modules = names(&[
        "kinds",
        "literals",
        "variables",
        "subscripts",
        "structures",
        "calls",
        "patterns",
        "comprehensions",
        "fsm",
        "precedence",
        "expressions",
    ]);
    let allowed_policies = names(&["node", "conditional-node", "transparent"]);
    let allowed_origins = names(&["new", "existing", "none"]);
    let mut policy_totals = BTreeMap::new();
    let mut origin_totals = BTreeMap::new();
    let mut module_totals = BTreeMap::new();
    let mut emitted_kinds = BTreeSet::new();

    for (name, row) in &schema {
        assert!(allowed_modules.contains(&row.module), "module for {name}");
        assert!(allowed_policies.contains(&row.policy), "policy for {name}");
        assert!(allowed_origins.contains(&row.origin), "origin for {name}");
        for value in [&row.module, &row.policy, &row.kind, &row.origin] {
            assert!(
                !matches!(value.as_str(), "token" | "root" | "undecided"),
                "reserved schema value for {name}: {value}"
            );
        }
        *policy_totals.entry(row.policy.as_str()).or_insert(0usize) += 1;
        *origin_totals.entry(row.origin.as_str()).or_insert(0usize) += 1;
        *module_totals.entry(row.module.as_str()).or_insert(0usize) += 1;

        match row.policy.as_str() {
            "node" | "conditional-node" => {
                assert_ne!(row.kind, "none", "missing syntax kind for {name}");
                assert!(
                    matches!(row.origin.as_str(), "new" | "existing"),
                    "invalid kind origin for {name}"
                );
                assert!(
                    emitted_kinds.insert(row.kind.as_str()),
                    "duplicate syntax kind {}",
                    row.kind
                );
            }
            "transparent" => {
                assert_eq!(row.kind, "none", "transparent kind for {name}");
                assert_eq!(row.origin, "none", "transparent origin for {name}");
            }
            _ => unreachable!(),
        }
    }

    assert_eq!(
        policy_totals,
        BTreeMap::from([("conditional-node", 7), ("node", 71), ("transparent", 2)])
    );
    assert_eq!(
        origin_totals,
        BTreeMap::from([("existing", 5), ("new", 73), ("none", 2)])
    );
    assert_eq!(
        module_totals,
        BTreeMap::from([
            ("calls", 4),
            ("comprehensions", 4),
            ("expressions", 2),
            ("fsm", 7),
            ("kinds", 11),
            ("literals", 1),
            ("patterns", 7),
            ("precedence", 13),
            ("structures", 23),
            ("subscripts", 6),
            ("variables", 2),
        ])
    );
}

#[test]
fn transparent_conditional_and_reused_mappings_are_exact() {
    let schema = schema();
    let transparent = schema
        .iter()
        .filter(|(_, row)| row.policy == "transparent")
        .map(|(name, _)| name.clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(transparent, names(&["formula", "pattern-array-item"]));

    let conditional = BTreeMap::from([
        ("l1", "LogicExpression"),
        ("l2", "ComparisonExpression"),
        ("l3", "AdditiveExpression"),
        ("l4", "MultiplicativeExpression"),
        ("l5", "PowerExpression"),
        ("l6", "TableExpression"),
        ("l7", "SetExpression"),
    ]);
    let actual_conditional = schema
        .iter()
        .filter(|(_, row)| row.policy == "conditional-node")
        .map(|(name, row)| (name.as_str(), row.kind.as_str()))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(actual_conditional, conditional);

    let reused = BTreeMap::from([
        ("expression", "Expression"),
        ("kind-annotation", "KindAnnotation"),
        ("l3", "AdditiveExpression"),
        ("parenthetical-term", "ParentheticalExpression"),
        ("variable-define", "VariableDefine"),
    ]);
    let actual_reused = schema
        .iter()
        .filter(|(_, row)| row.origin == "existing")
        .map(|(name, row)| (name.as_str(), row.kind.as_str()))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(actual_reused, reused);
}

#[test]
fn concrete_role_mappings_are_exact() {
    let schema = schema();
    for (name, kind) in [
        ("binding", "RecordBinding"),
        ("field", "TableField"),
        ("mapping", "MapEntry"),
        ("subscript", "SubscriptList"),
        ("table-row2", "FancyTableRow"),
        ("pattern-array", "ArrayPattern"),
        ("pattern-array-token", "ArrayPatternElement"),
        ("pattern-atom-struct", "AtomStructPattern"),
        ("pattern-tuple", "TuplePattern"),
        ("pattern-tuple-struct", "TupleStructPattern"),
        ("var", "Variable"),
        ("fsm-args", "FsmArguments"),
    ] {
        assert_eq!(schema[name].kind, kind, "concrete role for {name}");
    }
}

#[test]
fn new_kinds_match_the_exact_append_only_schema() {
    assert_eq!(PHASE_2I_NEW_KINDS.len(), 73);
    let expected = PHASE_2I_NEW_KINDS
        .iter()
        .enumerate()
        .map(|(offset, kind)| {
            assert_eq!(*kind as u16, 268 + offset as u16);
            assert!(!kind.is_token());
            format!("{kind:?}")
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(SyntaxKind::SetExpression as u16, 340);

    let actual = schema()
        .values()
        .filter(|row| row.origin == "new")
        .map(|row| row.kind.clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(actual, expected);
}

#[test]
fn recursive_core_port_status_remains_inactive() {
    let schema = schema();
    let ports = ports();
    for name in schema.keys() {
        let port = &ports[name];
        assert_eq!(port.syntax, "unported", "syntax status for {name}");
        assert_eq!(port.lowering, "pending", "lowering status for {name}");
        assert_eq!(port.policy, "undecided", "node policy for {name}");
        assert!(port.phase.is_empty(), "phase for {name}");
    }
}
