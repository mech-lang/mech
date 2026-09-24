use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use mech_syntax::document::parser::canonical::{
    CanonicalRuleOutcome, parse_canonical_phase_2i_rule_for_test,
};
use mech_syntax::document::parser::{canonical_rule_id, canonical_rule_name, rules};
use mech_syntax::document::{
    ArrayPatternSyntax, AstNode, DocumentId, ExpectedSyntax, FormulaSyntax, KindSyntax,
    KindValueSyntax, NodeFlags, ParseConfig, PatternArrayItemSyntax, RecursiveCoreSyntax, Revision,
    SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken, TextRange, TextSize, TextSnapshot,
    normalize_diagnostics, phase_2i_node_kind, reconstruct_source_range, validate_lossless_range,
};

#[derive(Debug)]
struct CertificationRow {
    name: String,
    accepted: String,
    rejected: String,
    recovery: String,
    emission_policy: String,
    syntax_kind: String,
    clean_tree_hash: u64,
    typed_access_hash: u64,
    recovery_snapshot_hash: u64,
    semantic_disposition: String,
    semantic_source: Option<String>,
    spec_location: String,
    conformance_cases: String,
    semantic_snapshot_hash: String,
    canonical_consumer: String,
    required_semantic_outcome: String,
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(0x2c8), Revision(5), text).unwrap()
}

fn certification_rows() -> Vec<CertificationRow> {
    let table = fs::read_to_string(
        repository_root().join("docs/design/grammar-audit/phase-2i-certification.tsv"),
    )
    .expect("read Phase 2I certification table");
    let mut lines = table.lines();
    assert_eq!(
        lines.next(),
        Some(
            "grammar-name\taccepted-source-json\trejected-source-json\trecovery-source-json\temission-policy\tsyntax-kind\tclean-tree-hash\ttyped-access-hash\trecovery-snapshot-hash\tsemantic-disposition\tsemantic-source-json\tspec-location\tconformance-cases\tsemantic-snapshot-hash\tcanonical-consumer\trequired-semantic-outcome"
        )
    );
    lines
        .map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            assert_eq!(fields.len(), 16, "invalid certification row: {line}");
            CertificationRow {
                name: fields[0].to_owned(),
                accepted: serde_json::from_str(fields[1]).expect("accepted source JSON"),
                rejected: serde_json::from_str(fields[2]).expect("rejected source JSON"),
                recovery: serde_json::from_str(fields[3]).expect("recovery source JSON"),
                emission_policy: fields[4].to_owned(),
                syntax_kind: fields[5].to_owned(),
                clean_tree_hash: fields[6].parse().expect("clean tree hash"),
                typed_access_hash: fields[7].parse().expect("typed access hash"),
                recovery_snapshot_hash: fields[8].parse().expect("recovery snapshot hash"),
                semantic_disposition: fields[9].to_owned(),
                semantic_source: (fields[10] != "none")
                    .then(|| serde_json::from_str(fields[10]).expect("semantic source JSON")),
                spec_location: fields[11].to_owned(),
                conformance_cases: fields[12].to_owned(),
                semantic_snapshot_hash: fields[13].to_owned(),
                canonical_consumer: fields[14].to_owned(),
                required_semantic_outcome: fields[15].to_owned(),
            }
        })
        .collect()
}

fn inventory_contracts() -> BTreeMap<String, (String, String, String, String)> {
    let productions =
        fs::read_to_string(repository_root().join("docs/design/grammar-audit/productions.tsv"))
            .unwrap();
    let productions = productions
        .lines()
        .skip(1)
        .map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            assert_eq!(fields.len(), 17);
            (
                fields[1].to_owned(),
                (fields[13].to_owned(), fields[14].to_owned()),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let schema = fs::read_to_string(
        repository_root().join("docs/design/grammar-audit/phase-2i-syntax-schema.tsv"),
    )
    .unwrap();
    schema
        .lines()
        .skip(1)
        .map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            assert_eq!(fields.len(), 6);
            let (spec, cases) = productions
                .get(fields[0])
                .unwrap_or_else(|| panic!("production inventory for {}", fields[0]));
            (
                fields[0].to_owned(),
                (
                    fields[2].to_owned(),
                    fields[3].to_owned(),
                    spec.clone(),
                    cases.clone(),
                ),
            )
        })
        .collect()
}

#[derive(Clone, Copy)]
struct StableHash(u64);

impl StableHash {
    const fn new() -> Self {
        Self(0xcbf29ce484222325)
    }

    fn field(&mut self, value: &str) {
        for byte in (value.len() as u64).to_le_bytes() {
            self.byte(byte);
        }
        for byte in value.bytes() {
            self.byte(byte);
        }
    }

    fn byte(&mut self, byte: u8) {
        self.0 ^= u64::from(byte);
        self.0 = self.0.wrapping_mul(0x100000001b3);
    }
}

fn find_kind(node: &SyntaxNode, kind: mech_syntax::document::SyntaxKind) -> Option<SyntaxNode> {
    if node.kind() == kind {
        return Some(node.clone());
    }
    node.children().find_map(|child| find_kind(&child, kind))
}

fn find_typed<N: AstNode>(node: &SyntaxNode) -> Option<N> {
    N::cast(node.clone()).or_else(|| node.children().find_map(|child| find_typed(&child)))
}

fn hash_node<N: AstNode>(hash: &mut StableHash, role: &str, value: Option<N>) {
    hash.field(role);
    match value {
        Some(value) => {
            hash.field(&value.syntax().text().expect("typed accessor source text"));
            hash.field("some");
            hash.field(value.syntax().kind().name());
            hash.field(&value.syntax().range().start.0.to_string());
            hash.field(&value.syntax().range().end.0.to_string());
            hash.field(&value.syntax().flags().0.to_string());
        }
        None => hash.field("none"),
    }
}

fn hash_nodes<N: AstNode>(hash: &mut StableHash, role: &str, values: Vec<N>) {
    hash.field(role);
    hash.field(&values.len().to_string());
    for (index, value) in values.into_iter().enumerate() {
        hash.field(&index.to_string());
        hash_node(hash, "item", Some(value));
    }
}

fn hash_token(hash: &mut StableHash, role: &str, value: Option<SyntaxToken>) {
    hash.field(role);
    match value {
        Some(value) => {
            hash.field("some");
            hash.field(value.kind().name());
            hash.field(&value.range().start.0.to_string());
            hash.field(&value.range().end.0.to_string());
            hash.field(&value.flags().0.to_string());
            hash.field(&value.text().expect("clean source token text"));
        }
        None => hash.field("none"),
    }
}

fn typed_access_hash(rule_name: &str, node: &SyntaxNode) -> u64 {
    let mut hash = StableHash::new();
    hash.field("canonical-typed-access-v2");
    hash.field(rule_name);
    if rule_name == "formula" {
        let view = find_typed::<FormulaSyntax>(node).expect("transparent formula typed view");
        hash_node(&mut hash, "formula", Some(view));
        return hash.0;
    }
    if rule_name == "pattern-array-item" {
        let view = find_typed::<PatternArrayItemSyntax>(node)
            .expect("transparent pattern-array-item typed view");
        hash_node(&mut hash, "pattern-array-item", Some(view.clone()));
        hash_node(&mut hash, "value", view.value());
        return hash.0;
    }
    let kind = phase_2i_node_kind(rule_name).expect("node-valued certification rule");
    let syntax = find_kind(node, kind).expect("certified typed node");
    let view = RecursiveCoreSyntax::cast(syntax).expect("closed recursive typed view");
    hash.field(match &view {
        RecursiveCoreSyntax::ArgumentList(_) => "ArgumentList",
        RecursiveCoreSyntax::RecordBinding(_) => "RecordBinding",
        RecursiveCoreSyntax::BraceSubscript(_) => "BraceSubscript",
        RecursiveCoreSyntax::BracketSubscript(_) => "BracketSubscript",
        RecursiveCoreSyntax::CallArgument(_) => "CallArgument",
        RecursiveCoreSyntax::BoundCallArgument(_) => "BoundCallArgument",
        RecursiveCoreSyntax::ComprehensionQualifier(_) => "ComprehensionQualifier",
        RecursiveCoreSyntax::Expression(_) => "Expression",
        RecursiveCoreSyntax::Factor(_) => "Factor",
        RecursiveCoreSyntax::FancyTable(_) => "FancyTable",
        RecursiveCoreSyntax::FancyTableHeader(_) => "FancyTableHeader",
        RecursiveCoreSyntax::TableField(_) => "TableField",
        RecursiveCoreSyntax::FormulaSubscript(_) => "FormulaSubscript",
        RecursiveCoreSyntax::FsmArguments(_) => "FsmArguments",
        RecursiveCoreSyntax::FsmAsyncTransition(_) => "FsmAsyncTransition",
        RecursiveCoreSyntax::FsmInstance(_) => "FsmInstance",
        RecursiveCoreSyntax::FsmOutput(_) => "FsmOutput",
        RecursiveCoreSyntax::FsmPipe(_) => "FsmPipe",
        RecursiveCoreSyntax::FsmStateTransition(_) => "FsmStateTransition",
        RecursiveCoreSyntax::FsmValue(_) => "FsmValue",
        RecursiveCoreSyntax::FunctionCall(_) => "FunctionCall",
        RecursiveCoreSyntax::Generator(_) => "Generator",
        RecursiveCoreSyntax::HeaderField(_) => "HeaderField",
        RecursiveCoreSyntax::InlineTable(_) => "InlineTable",
        RecursiveCoreSyntax::InlineTableHeader(_) => "InlineTableHeader",
        RecursiveCoreSyntax::InlineTableRow(_) => "InlineTableRow",
        RecursiveCoreSyntax::Kind(_) => "Kind",
        RecursiveCoreSyntax::KindAnnotation(_) => "KindAnnotation",
        RecursiveCoreSyntax::KindKind(_) => "KindKind",
        RecursiveCoreSyntax::KindMap(_) => "KindMap",
        RecursiveCoreSyntax::KindMatrix(_) => "KindMatrix",
        RecursiveCoreSyntax::KindRecord(_) => "KindRecord",
        RecursiveCoreSyntax::KindScalar(_) => "KindScalar",
        RecursiveCoreSyntax::KindSet(_) => "KindSet",
        RecursiveCoreSyntax::TableKind(_) => "TableKind",
        RecursiveCoreSyntax::KindTuple(_) => "KindTuple",
        RecursiveCoreSyntax::KindWithOption(_) => "KindWithOption",
        RecursiveCoreSyntax::LogicExpression(_) => "LogicExpression",
        RecursiveCoreSyntax::ComparisonExpression(_) => "ComparisonExpression",
        RecursiveCoreSyntax::AdditiveExpression(_) => "AdditiveExpression",
        RecursiveCoreSyntax::MultiplicativeExpression(_) => "MultiplicativeExpression",
        RecursiveCoreSyntax::PowerExpression(_) => "PowerExpression",
        RecursiveCoreSyntax::TableExpression(_) => "TableExpression",
        RecursiveCoreSyntax::SetExpression(_) => "SetExpression",
        RecursiveCoreSyntax::Literal(_) => "Literal",
        RecursiveCoreSyntax::Map(_) => "Map",
        RecursiveCoreSyntax::MapEntry(_) => "MapEntry",
        RecursiveCoreSyntax::MatchArm(_) => "MatchArm",
        RecursiveCoreSyntax::Matrix(_) => "Matrix",
        RecursiveCoreSyntax::MatrixColumn(_) => "MatrixColumn",
        RecursiveCoreSyntax::MatrixComprehension(_) => "MatrixComprehension",
        RecursiveCoreSyntax::MatrixRow(_) => "MatrixRow",
        RecursiveCoreSyntax::NegateFactor(_) => "NegateFactor",
        RecursiveCoreSyntax::NotFactor(_) => "NotFactor",
        RecursiveCoreSyntax::ParentheticalExpression(_) => "ParentheticalExpression",
        RecursiveCoreSyntax::Pattern(_) => "Pattern",
        RecursiveCoreSyntax::ArrayPattern(_) => "ArrayPattern",
        RecursiveCoreSyntax::ArrayPatternElement(_) => "ArrayPatternElement",
        RecursiveCoreSyntax::AtomStructPattern(_) => "AtomStructPattern",
        RecursiveCoreSyntax::TuplePattern(_) => "TuplePattern",
        RecursiveCoreSyntax::TupleStructPattern(_) => "TupleStructPattern",
        RecursiveCoreSyntax::RangeExpression(_) => "RangeExpression",
        RecursiveCoreSyntax::RangeSubscript(_) => "RangeSubscript",
        RecursiveCoreSyntax::Record(_) => "Record",
        RecursiveCoreSyntax::RegularTable(_) => "RegularTable",
        RecursiveCoreSyntax::Set(_) => "Set",
        RecursiveCoreSyntax::SetComprehension(_) => "SetComprehension",
        RecursiveCoreSyntax::Slice(_) => "Slice",
        RecursiveCoreSyntax::Structure(_) => "Structure",
        RecursiveCoreSyntax::SubscriptList(_) => "SubscriptList",
        RecursiveCoreSyntax::Table(_) => "Table",
        RecursiveCoreSyntax::TableHeader(_) => "TableHeader",
        RecursiveCoreSyntax::TableRow(_) => "TableRow",
        RecursiveCoreSyntax::FancyTableRow(_) => "FancyTableRow",
        RecursiveCoreSyntax::Tuple(_) => "Tuple",
        RecursiveCoreSyntax::TupleStruct(_) => "TupleStruct",
        RecursiveCoreSyntax::Variable(_) => "Variable",
        RecursiveCoreSyntax::VariableDefine(_) => "VariableDefine",
    });
    macro_rules! node {
        ($role:literal, $value:expr) => {
            hash_node(&mut hash, $role, $value)
        };
    }
    macro_rules! nodes {
        ($role:literal, $value:expr) => {
            hash_nodes(&mut hash, $role, $value)
        };
    }
    macro_rules! token {
        ($role:literal, $value:expr) => {
            hash_token(&mut hash, $role, $value)
        };
    }
    match view {
        RecursiveCoreSyntax::ArgumentList(view) => {
            token!("opening-parenthesis", view.opening_parenthesis());
            nodes!("arguments", view.arguments());
            token!("closing-parenthesis", view.closing_parenthesis());
        }
        RecursiveCoreSyntax::RecordBinding(view) => {
            node!("name", view.name());
            node!("annotation", view.annotation());
            token!("colon", view.colon());
            node!("value", view.value());
        }
        RecursiveCoreSyntax::BraceSubscript(view) => {
            token!("opening-delimiter", view.opening_delimiter());
            nodes!("values", view.values());
            token!("closing-delimiter", view.closing_delimiter());
        }
        RecursiveCoreSyntax::BracketSubscript(view) => {
            token!("opening-delimiter", view.opening_delimiter());
            nodes!("values", view.values());
            token!("closing-delimiter", view.closing_delimiter());
        }
        RecursiveCoreSyntax::CallArgument(view) => node!("value", view.value()),
        RecursiveCoreSyntax::BoundCallArgument(view) => {
            node!("name", view.name());
            token!("colon", view.colon());
            node!("value", view.value());
        }
        RecursiveCoreSyntax::ComprehensionQualifier(view) => node!("value", view.value()),
        RecursiveCoreSyntax::Expression(view) => {
            node!("body", view.body());
            nodes!("match-arms", view.match_arms());
        }
        RecursiveCoreSyntax::Factor(view) => {
            node!("value", view.value());
            token!("transpose", view.transpose());
        }
        RecursiveCoreSyntax::FancyTable(view) => {
            node!("header", view.header());
            nodes!("rows", view.rows());
        }
        RecursiveCoreSyntax::FancyTableHeader(view) => nodes!("fields", view.fields()),
        RecursiveCoreSyntax::TableField(view) => {
            node!("name", view.name());
            node!("annotation", view.annotation());
        }
        RecursiveCoreSyntax::FormulaSubscript(view) => node!("formula", view.formula()),
        RecursiveCoreSyntax::FsmArguments(view) => {
            token!("opening-parenthesis", view.opening_parenthesis());
            nodes!("arguments", view.arguments());
            token!("closing-parenthesis", view.closing_parenthesis());
        }
        RecursiveCoreSyntax::FsmAsyncTransition(view) => {
            token!("operator", view.operator());
            node!("value", view.value());
        }
        RecursiveCoreSyntax::FsmInstance(view) => {
            token!("hash", view.hash());
            node!("name", view.name());
            node!("arguments", view.arguments());
        }
        RecursiveCoreSyntax::FsmOutput(view) => {
            token!("operator", view.operator());
            node!("value", view.value());
        }
        RecursiveCoreSyntax::FsmPipe(view) => {
            node!("instance", view.instance());
            nodes!("stages", view.stages());
        }
        RecursiveCoreSyntax::FsmStateTransition(view) => {
            token!("operator", view.operator());
            node!("value", view.value());
        }
        RecursiveCoreSyntax::FsmValue(view) => node!("pattern", view.pattern()),
        RecursiveCoreSyntax::FunctionCall(view) => {
            node!("function", view.function());
            node!("arguments", view.arguments());
        }
        RecursiveCoreSyntax::Generator(view) => {
            node!("pattern", view.pattern());
            token!("arrow", view.arrow());
            node!("source", view.source());
        }
        RecursiveCoreSyntax::HeaderField(view) => {
            node!("name", view.name());
            node!("annotation", view.annotation());
        }
        RecursiveCoreSyntax::InlineTable(view) => {
            node!("header", view.header());
            nodes!("rows", view.rows());
        }
        RecursiveCoreSyntax::InlineTableHeader(view) => nodes!("fields", view.fields()),
        RecursiveCoreSyntax::InlineTableRow(view) => nodes!("cells", view.cells()),
        RecursiveCoreSyntax::Kind(view) => node!("value", view.value()),
        RecursiveCoreSyntax::KindAnnotation(view) => {
            token!("opening-angle", view.opening_angle());
            node!("kind", view.kind());
            token!("closing-angle", view.closing_angle());
        }
        RecursiveCoreSyntax::KindKind(view) => {
            token!("opening-angle", view.opening_angle());
            node!("kind", view.kind());
            token!("closing-angle", view.closing_angle());
        }
        RecursiveCoreSyntax::KindMap(view) => {
            token!("opening-brace", view.opening_brace());
            node!("key", view.key());
            token!("colon", view.colon());
            node!("value", view.value());
            token!("closing-brace", view.closing_brace());
        }
        RecursiveCoreSyntax::KindMatrix(view) => {
            token!("opening-bracket", view.opening_bracket());
            node!("element", view.element());
            nodes!("dimensions", view.dimensions());
            token!("closing-bracket", view.closing_bracket());
        }
        RecursiveCoreSyntax::KindRecord(view) => {
            token!("opening-brace", view.opening_brace());
            nodes!("fields", view.fields());
            nodes!("field-kinds", view.field_kinds());
            token!("closing-brace", view.closing_brace());
        }
        RecursiveCoreSyntax::KindScalar(view) => {
            node!("name", view.name());
            node!("constraint", view.constraint());
            node!("recovered-expression", view.recovered_expression());
        }
        RecursiveCoreSyntax::KindSet(view) => {
            token!("opening-brace", view.opening_brace());
            node!("element", view.element());
            node!("literal-constraint", view.literal_constraint());
            token!("closing-brace", view.closing_brace());
        }
        RecursiveCoreSyntax::TableKind(view) => {
            token!("opening-bar", view.opening_bar());
            nodes!("field-names", view.field_names());
            nodes!("field-kinds", view.field_kinds());
            token!("closing-bar", view.closing_bar());
            node!("constraint", view.constraint());
        }
        RecursiveCoreSyntax::KindTuple(view) => {
            token!("opening-parenthesis", view.opening_parenthesis());
            nodes!("items", view.items());
            token!("closing-parenthesis", view.closing_parenthesis());
        }
        RecursiveCoreSyntax::KindWithOption(view) => {
            node!("kind", view.kind());
            token!("question-mark", view.question_mark());
        }
        RecursiveCoreSyntax::LogicExpression(view) => {
            nodes!("operands", view.operands());
            nodes!("operators", view.operators());
        }
        RecursiveCoreSyntax::ComparisonExpression(view) => {
            nodes!("operands", view.operands());
            nodes!("operators", view.operators());
        }
        RecursiveCoreSyntax::AdditiveExpression(view) => {
            nodes!("operands", view.operands());
            nodes!("operators", view.operators());
        }
        RecursiveCoreSyntax::MultiplicativeExpression(view) => {
            nodes!("operands", view.operands());
            nodes!("operators", view.operators());
        }
        RecursiveCoreSyntax::PowerExpression(view) => {
            nodes!("operands", view.operands());
            nodes!("operators", view.operators());
        }
        RecursiveCoreSyntax::TableExpression(view) => {
            nodes!("operands", view.operands());
            nodes!("operators", view.operators());
        }
        RecursiveCoreSyntax::SetExpression(view) => {
            nodes!("operands", view.operands());
            nodes!("operators", view.operators());
        }
        RecursiveCoreSyntax::Literal(view) => {
            node!("value", view.value());
            token!("true-token", view.true_token());
            token!("false-token", view.false_token());
            node!("annotation", view.annotation());
        }
        RecursiveCoreSyntax::Map(view) => {
            token!("opening-brace", view.opening_brace());
            nodes!("entries", view.entries());
            token!("closing-brace", view.closing_brace());
        }
        RecursiveCoreSyntax::MapEntry(view) => {
            node!("key", view.key());
            token!("colon", view.colon());
            node!("value", view.value());
        }
        RecursiveCoreSyntax::MatchArm(view) => {
            node!("pattern", view.pattern());
            nodes!("expressions", view.expressions());
            node!("guard", view.guard());
            token!("output-operator", view.output_operator());
            node!("value", view.value());
        }
        RecursiveCoreSyntax::Matrix(view) => {
            token!("opening-delimiter", view.opening_delimiter());
            nodes!("rows", view.rows());
            token!("closing-delimiter", view.closing_delimiter());
        }
        RecursiveCoreSyntax::MatrixColumn(view) => node!("value", view.value()),
        RecursiveCoreSyntax::MatrixComprehension(view) => {
            token!("opening-delimiter", view.opening_delimiter());
            node!("value", view.value());
            token!("bar", view.bar());
            nodes!("qualifiers", view.qualifiers());
            token!("closing-delimiter", view.closing_delimiter());
        }
        RecursiveCoreSyntax::MatrixRow(view) => nodes!("columns", view.columns()),
        RecursiveCoreSyntax::NegateFactor(view) => {
            token!("dash", view.dash());
            node!("operand", view.operand());
        }
        RecursiveCoreSyntax::NotFactor(view) => {
            node!("operator", view.operator());
            node!("operand", view.operand());
        }
        RecursiveCoreSyntax::ParentheticalExpression(view) => {
            token!("opening-parenthesis", view.opening_parenthesis());
            node!("expression", view.expression());
            token!("closing-parenthesis", view.closing_parenthesis());
        }
        RecursiveCoreSyntax::Pattern(view) => node!("value", view.value()),
        RecursiveCoreSyntax::ArrayPattern(view) => {
            token!("opening-bracket", view.opening_bracket());
            nodes!("elements", view.elements());
            token!("closing-bracket", view.closing_bracket());
        }
        RecursiveCoreSyntax::ArrayPatternElement(view) => {
            node!("pattern", view.pattern());
            token!("spread", view.spread());
            token!("rest", view.rest());
        }
        RecursiveCoreSyntax::AtomStructPattern(view) => {
            token!("prefix", view.prefix());
            node!("name", view.name());
            token!("opening-parenthesis", view.opening_parenthesis());
            nodes!("items", view.items());
            token!("closing-parenthesis", view.closing_parenthesis());
        }
        RecursiveCoreSyntax::TuplePattern(view) => {
            token!("opening-parenthesis", view.opening_parenthesis());
            nodes!("items", view.items());
            token!("closing-parenthesis", view.closing_parenthesis());
        }
        RecursiveCoreSyntax::TupleStructPattern(view) => {
            token!("prefix", view.prefix());
            node!("name", view.name());
            token!("opening-parenthesis", view.opening_parenthesis());
            nodes!("items", view.items());
            token!("closing-parenthesis", view.closing_parenthesis());
        }
        RecursiveCoreSyntax::RangeExpression(view) => {
            nodes!("bounds", view.bounds());
            nodes!("operators", view.operators());
        }
        RecursiveCoreSyntax::RangeSubscript(view) => {
            node!("range", view.range());
            node!("recovered-expression", view.recovered_expression());
        }
        RecursiveCoreSyntax::Record(view) => {
            token!("opening-delimiter", view.opening_delimiter());
            nodes!("bindings", view.bindings());
            token!("closing-delimiter", view.closing_delimiter());
        }
        RecursiveCoreSyntax::RegularTable(view) => {
            node!("header", view.header());
            nodes!("rows", view.rows());
        }
        RecursiveCoreSyntax::Set(view) => {
            token!("opening-brace", view.opening_brace());
            nodes!("items", view.items());
            token!("closing-brace", view.closing_brace());
        }
        RecursiveCoreSyntax::SetComprehension(view) => {
            token!("opening-delimiter", view.opening_delimiter());
            node!("value", view.value());
            token!("bar", view.bar());
            nodes!("qualifiers", view.qualifiers());
            token!("closing-delimiter", view.closing_delimiter());
        }
        RecursiveCoreSyntax::Slice(view) => {
            node!("stem", view.stem());
            node!("subscripts", view.subscripts());
        }
        RecursiveCoreSyntax::Structure(view) => node!("value", view.value()),
        RecursiveCoreSyntax::SubscriptList(view) => nodes!("items", view.items()),
        RecursiveCoreSyntax::Table(view) => node!("value", view.value()),
        RecursiveCoreSyntax::TableHeader(view) => nodes!("fields", view.fields()),
        RecursiveCoreSyntax::TableRow(view) => nodes!("cells", view.cells()),
        RecursiveCoreSyntax::FancyTableRow(view) => nodes!("cells", view.cells()),
        RecursiveCoreSyntax::Tuple(view) => {
            token!("opening-parenthesis", view.opening_parenthesis());
            nodes!("items", view.items());
            token!("closing-parenthesis", view.closing_parenthesis());
        }
        RecursiveCoreSyntax::TupleStruct(view) => {
            token!("prefix", view.prefix());
            node!("name", view.name());
            token!("opening-parenthesis", view.opening_parenthesis());
            node!("value", view.value());
            token!("closing-parenthesis", view.closing_parenthesis());
        }
        RecursiveCoreSyntax::Variable(view) => {
            node!("stem", view.stem());
            node!("annotation", view.annotation());
        }
        RecursiveCoreSyntax::VariableDefine(view) => {
            token!("mutability-marker", view.mutability_marker());
            node!("variable", view.variable());
            token!("define-operator", view.define_operator());
            node!("value", view.value());
        }
    }
    hash.0
}

fn range_evidence(range: Option<TextRange>) -> serde_json::Value {
    serde_json::json!(range.map(|range| [range.start.0, range.end.0]))
}

fn expected_evidence(expected: &mech_syntax::document::ExpectedSyntax) -> serde_json::Value {
    match expected {
        ExpectedSyntax::Token(kind) => serde_json::json!(["token", kind.name()]),
        ExpectedSyntax::Production(name) => serde_json::json!(["production", name]),
    }
}

fn diagnostic_evidence(
    diagnostic: &mech_syntax::document::NormalizedDiagnostic,
) -> serde_json::Value {
    use mech_syntax::document::{DiagnosticPhase, FixApplicability, RecoveryAction, Severity};
    let phase = match diagnostic.phase {
        DiagnosticPhase::Syntax => "syntax",
        DiagnosticPhase::SyntaxValidation => "syntax-validation",
        DiagnosticPhase::Lowering => "lowering",
        DiagnosticPhase::Kind => "kind",
        DiagnosticPhase::Dimension => "dimension",
        DiagnosticPhase::Effect => "effect",
        DiagnosticPhase::Coeffect => "coeffect",
        DiagnosticPhase::Refinement => "refinement",
        DiagnosticPhase::Liveness => "liveness",
        DiagnosticPhase::Document => "document",
        DiagnosticPhase::Runtime => "runtime",
    };
    let severity = match diagnostic.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Information => "information",
        Severity::Hint => "hint",
    };
    let recovery = diagnostic.recovery.as_ref().map(|action| match action {
        RecoveryAction::Insert { syntax, at } => {
            serde_json::json!(["insert", expected_evidence(syntax), at.0])
        }
        RecoveryAction::Skip { range } => serde_json::json!(["skip", range_evidence(Some(*range))]),
        RecoveryAction::Abandon { rule, at } => serde_json::json!(["abandon", rule.0, at.0]),
        RecoveryAction::ResourceLimit { range } => {
            serde_json::json!(["resource-limit", range_evidence(Some(*range))])
        }
    });
    let fixes = diagnostic
        .fixes
        .iter()
        .map(|fix| {
            let applicability = match fix.applicability {
                FixApplicability::MachineApplicable => "machine-applicable",
                FixApplicability::MaybeIncorrect => "maybe-incorrect",
                FixApplicability::HasPlaceholders => "has-placeholders",
            };
            serde_json::json!([
                applicability,
                fix.edits
                    .iter()
                    .map(|edit| serde_json::json!([range_evidence(Some(edit.delete)), edit.insert]))
                    .collect::<Vec<_>>()
            ])
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "code": diagnostic.code.as_str(), "phase": phase, "severity": severity,
        "rule": diagnostic.rule.map(|rule| rule.0), "context": diagnostic.context.map(|context| context.0),
        "primary": range_evidence(diagnostic.primary),
        "expected": diagnostic.expected.iter().map(expected_evidence).collect::<Vec<_>>(),
        "found": diagnostic.found.as_ref().map(|found| serde_json::json!([found.kind.map(SyntaxKind::name), found.text])),
        "related": diagnostic.related, "recovery": recovery, "tags": diagnostic.tags.0,
        "labels": diagnostic.labels.iter().map(|label| range_evidence(label.range)).collect::<Vec<_>>(),
        "fixes": fixes,
    })
}

fn canonical_tree_evidence(node: &SyntaxNode) -> serde_json::Value {
    let children = node
        .children_with_tokens()
        .into_iter()
        .map(|child| match child {
            SyntaxElement::Node(node) => canonical_tree_evidence(&node),
            SyntaxElement::Token(token) => serde_json::json!([
                "token",
                token.kind().name(),
                range_evidence(Some(token.range())),
                token.flags().0,
                token.text().unwrap()
            ]),
        })
        .collect::<Vec<_>>();
    serde_json::json!([
        "node",
        node.kind().name(),
        range_evidence(Some(node.range())),
        node.flags().0,
        children
    ])
}

fn clean_tree_hash(node: &SyntaxNode) -> u64 {
    let mut hash = StableHash::new();
    hash.field("canonical-clean-tree-v2");
    hash.field(&serde_json::to_string(&canonical_tree_evidence(node)).unwrap());
    hash.0
}

fn recovery_snapshot_hash(
    parsed: &mech_syntax::document::parser::canonical::CanonicalSourceRuleSnapshot,
) -> u64 {
    let mut hash = StableHash::new();
    hash.field("canonical-recovery-v3");
    hash.field(&serde_json::to_string(&canonical_tree_evidence(&parsed.syntax())).unwrap());
    let diagnostics =
        normalize_diagnostics(&parsed.diagnostics, parsed.source.revision(), &parsed.nodes);
    hash.field(
        &serde_json::to_string(
            &diagnostics
                .iter()
                .map(diagnostic_evidence)
                .collect::<Vec<_>>(),
        )
        .unwrap(),
    );
    hash.0
}

#[test]
fn recovery_evidence_uses_normalized_identity_and_preserves_structured_changes() {
    use mech_syntax::document::{
        ExpectedSyntax, FixApplicability, FoundSyntax, NormalizedDiagnosticFix,
        NormalizedDiagnosticLabel, RecoveryAction, TextEdit,
    };
    let parse = |document, revision| {
        parse_canonical_phase_2i_rule_for_test(
            TextSnapshot::new(DocumentId(document), Revision(revision), "[1").unwrap(),
            rules::MATRIX,
            ParseConfig::default(),
        )
        .unwrap()
    };
    let first = parse(1, 2);
    let second = parse(99, 100);
    assert_eq!(
        recovery_snapshot_hash(&first),
        recovery_snapshot_hash(&second)
    );
    let mut raw_wording = mech_syntax::document::DiagnosticStore::new(first.source.revision());
    for (index, diagnostic) in first.diagnostics.iter().cloned().enumerate() {
        let mut diagnostic = diagnostic;
        if index == 0 {
            diagnostic.message = "equivalent presentation wording".into();
        }
        raw_wording.push(diagnostic);
    }
    let original_normalized =
        normalize_diagnostics(&first.diagnostics, first.source.revision(), &first.nodes);
    let wording_normalized =
        normalize_diagnostics(&raw_wording, first.source.revision(), &first.nodes);
    assert_eq!(
        original_normalized
            .iter()
            .map(diagnostic_evidence)
            .collect::<Vec<_>>(),
        wording_normalized
            .iter()
            .map(diagnostic_evidence)
            .collect::<Vec<_>>(),
        "main diagnostic wording is presentation, not recovery evidence"
    );
    let mut diagnostic =
        normalize_diagnostics(&first.diagnostics, first.source.revision(), &first.nodes).remove(0);
    diagnostic.expected = vec![
        ExpectedSyntax::Token(SyntaxKind::RightBracket),
        ExpectedSyntax::Production("expression".into()),
    ];
    diagnostic.found = Some(FoundSyntax {
        kind: Some(SyntaxKind::LeftBracket),
        text: Some("[".into()),
    });
    diagnostic.labels = vec![NormalizedDiagnosticLabel {
        range: Some(TextRange::empty(TextSize(2))),
        message: "closing delimiter".into(),
    }];
    diagnostic.fixes = vec![NormalizedDiagnosticFix {
        title: "close matrix".into(),
        applicability: FixApplicability::MachineApplicable,
        edits: vec![TextEdit::insert(TextSize(2), "]")],
    }];
    diagnostic.recovery = Some(RecoveryAction::Insert {
        syntax: ExpectedSyntax::Token(SyntaxKind::RightBracket),
        at: TextSize(2),
    });
    let evidence = diagnostic_evidence(&diagnostic);
    assert_eq!(
        evidence["expected"][0],
        serde_json::json!(["token", "RightBracket"])
    );
    assert_eq!(evidence["fixes"][0][0], "machine-applicable");
    let mut wording_changed = diagnostic.clone();
    wording_changed.labels[0].message = "same label, different words".into();
    wording_changed.fixes[0].title = "same edit, different title".into();
    assert_eq!(
        evidence,
        diagnostic_evidence(&wording_changed),
        "label messages and fix titles are presentation-only"
    );
    for change in 0..10 {
        let mut changed = diagnostic.clone();
        match change {
            0 => changed.code = "syntax/different".into(),
            1 => changed.primary = Some(TextRange::empty(TextSize(1))),
            2 => changed.expected.reverse(),
            3 => changed.found.as_mut().unwrap().text = Some("]".into()),
            4 => changed.related.push(0),
            5 => {
                changed.recovery = Some(RecoveryAction::Skip {
                    range: TextRange::new(TextSize(0), TextSize(1)),
                })
            }
            6 => changed.fixes[0].edits[0].insert = ")".into(),
            7 => changed.labels[0].range = None,
            8 => changed.fixes[0].edits[0].delete = TextRange::new(TextSize(1), TextSize(2)),
            9 => changed.fixes[0].applicability = FixApplicability::MaybeIncorrect,
            _ => unreachable!(),
        }
        assert_ne!(evidence, diagnostic_evidence(&changed), "change {change}");
    }
}

fn assert_certified_inventory<'a>(
    names: impl IntoIterator<Item = &'a str>,
    contracts: &BTreeMap<String, (String, String, String, String)>,
) {
    let names = names.into_iter().collect::<Vec<_>>();
    let unique = names.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(names.len(), unique.len(), "duplicate certification rule");
    let expected = contracts
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        unique, expected,
        "certification must cover the exact inventory"
    );
}

#[test]
fn certification_inventory_rejects_duplicate_missing_and_unknown_rules() {
    let rows = certification_rows();
    let contracts = inventory_contracts();
    let names = rows.iter().map(|row| row.name.as_str()).collect::<Vec<_>>();
    assert_certified_inventory(names.iter().copied(), &contracts);
    for case in 0..3 {
        let mut changed = names.clone();
        match case {
            0 => changed[0] = changed[1],
            1 => {
                changed.pop();
            }
            2 => changed[0] = "not-an-inventory-rule",
            _ => unreachable!(),
        }
        assert!(
            std::panic::catch_unwind(|| assert_certified_inventory(changed, &contracts)).is_err()
        );
    }
}

#[test]
fn clean_tree_evidence_covers_flags_and_ignores_snapshot_identity() {
    use mech_syntax::document::{GreenElement, GreenNode, TokenFlags};
    use std::sync::Arc;
    fn change_token(node: &GreenNode) -> GreenNode {
        let mut changed = node.clone();
        let mut children = changed.children.to_vec();
        match &mut children[0] {
            GreenElement::Node(child) => *child = Arc::new(change_token(child)),
            GreenElement::Token(token) => token.flags.0 ^= TokenFlags::TRIVIA.0,
        }
        changed.children = children.into();
        changed
    }
    let parse = |document, revision| {
        parse_canonical_phase_2i_rule_for_test(
            TextSnapshot::new(DocumentId(document), Revision(revision), "[1 2]").unwrap(),
            rules::MATRIX,
            ParseConfig::default(),
        )
        .unwrap()
    };
    let parsed = parse(1, 2);
    let baseline = clean_tree_hash(&parsed.syntax());
    assert_eq!(baseline, clean_tree_hash(&parse(9, 10).syntax()));
    for flag in [NodeFlags::REPARSE_ROOT, NodeFlags::PROVISIONAL] {
        let mut changed = (*parsed.root).clone();
        changed.flags.0 ^= flag.0;
        assert_eq!(changed.structural_hash, parsed.root.structural_hash);
        assert_ne!(
            baseline,
            clean_tree_hash(&SyntaxNode::new_root(
                Arc::new(changed),
                parsed.source.clone()
            ))
        );
    }
    let changed = change_token(&parsed.root);
    assert_eq!(changed.structural_hash, parsed.root.structural_hash);
    assert_ne!(
        baseline,
        clean_tree_hash(&SyntaxNode::new_root(
            Arc::new(changed),
            parsed.source.clone()
        ))
    );
}

#[test]
fn certification_table_executes_every_direct_accept_reject_and_recovery_case() {
    let rows = certification_rows();
    let contracts = inventory_contracts();
    assert_eq!(contracts.len(), 80);
    assert_certified_inventory(rows.iter().map(|row| row.name.as_str()), &contracts);
    let mut stale_hashes = Vec::new();
    for row in rows {
        let contract = contracts
            .get(&row.name)
            .unwrap_or_else(|| panic!("Phase 2I schema row for {}", row.name));
        assert_eq!(&row.emission_policy, &contract.0, "{}", row.name);
        assert_eq!(&row.syntax_kind, &contract.1, "{}", row.name);
        assert_eq!(&row.spec_location, &contract.2, "{}", row.name);
        assert_eq!(&row.conformance_cases, &contract.3, "{}", row.name);
        let rule = canonical_rule_id(&row.name).expect("registered canonical rule");
        assert_eq!(canonical_rule_name(rule), Some(row.name.as_str()));

        let accepted = parse_canonical_phase_2i_rule_for_test(
            source(&row.accepted),
            rule,
            ParseConfig::default(),
        )
        .expect("Phase 2I direct dispatcher");
        assert_eq!(
            accepted.outcome,
            CanonicalRuleOutcome::Matched,
            "{}",
            row.name
        );
        assert!(accepted.is_strictly_clean(), "{}", row.name);
        assert_eq!(
            accepted.consumed,
            TextRange::new(TextSize::ZERO, TextSize(row.accepted.len() as u32)),
            "{}",
            row.name
        );
        let clean_hash = clean_tree_hash(&accepted.syntax());
        if clean_hash != row.clean_tree_hash {
            stale_hashes.push(format!(
                "{}\tclean-tree\t{}\t{}",
                row.name, row.clean_tree_hash, clean_hash
            ));
        }
        validate_lossless_range(&accepted.root, &accepted.source, accepted.consumed).unwrap();
        assert_eq!(
            reconstruct_source_range(&accepted.root, &accepted.source, accepted.consumed).unwrap(),
            row.accepted,
            "{}",
            row.name
        );
        let typed_hash = typed_access_hash(&row.name, &accepted.syntax());
        if typed_hash != row.typed_access_hash {
            stale_hashes.push(format!(
                "{}\ttyped-access\t{}\t{}",
                row.name, row.typed_access_hash, typed_hash
            ));
        }

        assert!(!row.rejected.is_empty(), "{}", row.name);
        assert_ne!(row.rejected, row.accepted, "{}", row.name);
        assert_ne!(row.rejected, row.recovery, "{}", row.name);
        assert!(
            row.rejected.ends_with(&row.accepted),
            "{} rejection must prefix its current accepted fixture",
            row.name
        );
        let rejected = parse_canonical_phase_2i_rule_for_test(
            source(&row.rejected),
            rule,
            ParseConfig::default(),
        )
        .expect("Phase 2I direct dispatcher");
        assert_eq!(
            rejected.outcome,
            CanonicalRuleOutcome::NoMatch,
            "{}",
            row.name
        );
        assert_eq!(rejected.consumed, TextRange::empty(TextSize::ZERO));
        assert!(rejected.diagnostics.is_empty(), "{}", row.name);

        let recovered = parse_canonical_phase_2i_rule_for_test(
            source(&row.recovery),
            rule,
            ParseConfig::default(),
        )
        .expect("Phase 2I direct dispatcher");
        assert_eq!(
            recovered.outcome,
            CanonicalRuleOutcome::Committed,
            "{}",
            row.name
        );
        assert_eq!(
            recovered.consumed,
            TextRange::new(TextSize::ZERO, TextSize(row.recovery.len() as u32)),
            "{}",
            row.name
        );
        assert!(!recovered.diagnostics.is_empty(), "{}", row.name);
        assert!(recovered.root.flags.intersects(
            NodeFlags::ERROR
                | NodeFlags::MISSING
                | NodeFlags::CONTAINS_ERROR
                | NodeFlags::CONTAINS_MISSING
        ));
        validate_lossless_range(&recovered.root, &recovered.source, recovered.consumed).unwrap();
        assert_eq!(
            reconstruct_source_range(&recovered.root, &recovered.source, recovered.consumed)
                .unwrap(),
            row.recovery,
            "{}",
            row.name
        );
        let recovery_hash = recovery_snapshot_hash(&recovered);
        if recovery_hash != row.recovery_snapshot_hash {
            stale_hashes.push(format!(
                "{}\trecovery\t{}\t{}",
                row.name, row.recovery_snapshot_hash, recovery_hash
            ));
        }

        assert!(matches!(
            row.emission_policy.as_str(),
            "node" | "conditional-node" | "transparent"
        ));
        assert!(matches!(
            row.semantic_disposition.as_str(),
            "executable" | "structural" | "compile-time"
        ));
        match row.semantic_disposition.as_str() {
            "executable" => {
                assert_eq!(row.canonical_consumer, "engine/source-semantics");
                assert!(row.semantic_source.is_some());
            }
            "structural" => {
                assert!(row.canonical_consumer.starts_with("syntax/typed-"));
                assert!(row.semantic_source.is_none());
                assert_eq!(row.semantic_snapshot_hash, "none");
            }
            "compile-time" => {
                assert_eq!(
                    row.canonical_consumer,
                    "engine/source-semantics/compile-time"
                );
                assert!(row.semantic_source.is_some());
            }
            _ => unreachable!("closed semantic disposition"),
        }
        if let Some(code) = row
            .required_semantic_outcome
            .strip_prefix("expected-user-error:")
        {
            assert!(row.semantic_source.is_some());
            assert!(code.starts_with("source-semantics/"));
            assert_eq!(row.semantic_snapshot_hash, "none");
        } else {
            assert_eq!(
                row.required_semantic_outcome, row.semantic_disposition,
                "{}",
                row.name
            );
            if row.semantic_disposition != "structural" {
                assert!(row.semantic_snapshot_hash.parse::<u64>().is_ok());
            }
        }
    }
    assert!(
        stale_hashes.is_empty(),
        "syntax certification snapshots changed:\n{}",
        stale_hashes.join("\n")
    );
}

#[test]
fn certification_variants_cover_empty_kinds_and_array_rest_accessors() {
    for (text, expected) in [("*", SyntaxKind::KindAny), ("_", SyntaxKind::KindEmpty)] {
        let parsed = parse_canonical_phase_2i_rule_for_test(
            source(text),
            rules::KIND,
            ParseConfig::default(),
        )
        .unwrap();
        assert!(parsed.is_strictly_clean(), "{text:?}");
        let kind = find_typed::<KindSyntax>(&parsed.syntax()).expect("typed kind evidence");
        assert!(
            matches!(
                kind.value(),
                Some(KindValueSyntax::Any(_)) if expected == SyntaxKind::KindAny
            ) || matches!(
                kind.value(),
                Some(KindValueSyntax::Empty(_)) if expected == SyntaxKind::KindEmpty
            )
        );
    }

    let parsed = parse_canonical_phase_2i_rule_for_test(
        source("[head | tail]"),
        rules::PATTERN_ARRAY,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(parsed.is_strictly_clean());
    let array = find_typed::<ArrayPatternSyntax>(&parsed.syntax()).expect("typed array pattern");
    let elements = array.elements();
    assert_eq!(
        elements
            .iter()
            .filter(|element| element.pattern().is_some())
            .count(),
        2
    );
    assert_eq!(
        elements
            .iter()
            .filter(|element| element.rest().is_some())
            .count(),
        1
    );

    let parsed = parse_canonical_phase_2i_rule_for_test(
        source("[head, ..., tail]"),
        rules::PATTERN_ARRAY,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(parsed.is_strictly_clean());
    let array = find_typed::<ArrayPatternSyntax>(&parsed.syntax()).expect("typed array pattern");
    assert_eq!(
        array
            .elements()
            .iter()
            .filter(|element| element.spread().is_some())
            .count(),
        1
    );
}

#[test]
fn match_001_is_a_positive_canonical_conformance_case() {
    for text in ["x? | * => 1", "x ? | * => 1", "x\t?\n| * => 1"] {
        let parsed = parse_canonical_phase_2i_rule_for_test(
            source(text),
            rules::EXPRESSION,
            ParseConfig::default(),
        )
        .unwrap();
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::Matched, "{text:?}");
        assert!(parsed.is_strictly_clean(), "{text:?}");
    }
    let corrections =
        fs::read_to_string(repository_root().join("docs/design/grammar-audit/corrections.tsv"))
            .unwrap();
    let row = corrections
        .lines()
        .find(|line| line.starts_with("MATCH-001\t"))
        .expect("MATCH-001 correction");
    assert!(row.contains("\tapplied\t"));
    for case in [
        "MATCH-001-ADJACENT",
        "MATCH-001-SPACED",
        "MATCH-001-MULTILINE",
    ] {
        assert!(row.contains(case));
    }
}

#[test]
fn certification_evidence_uses_only_canonical_authorities() {
    for relative in [
        "src/syntax/tests/canonical_phase_2i_rule_surface.rs",
        "src/syntax/tests/canonical_phase_2i_recovery.rs",
        "src/syntax/tests/canonical_phase_2i_typed_views.rs",
        "src/syntax/tests/canonical_phase_2i_ambiguity.rs",
        "src/syntax/tests/canonical_phase_2i_complexity.rs",
        "src/syntax/tests/canonical_phase_2i_piece_backed.rs",
        "src/syntax/tests/canonical_phase_2i_resource_limits.rs",
        "src/syntax/tests/canonical_phase_2i_certification.rs",
        "src/engine/tests/canonical_phase_2i_semantic_certification.rs",
        "src/engine/tests/canonical_source_completion.rs",
        "src/engine/tests/canonical_source_semantics.rs",
    ] {
        let path = repository_root().join(relative);
        let evidence = fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!(
                "unable to read certification evidence {}: {error}",
                path.display()
            )
        });
        assert_canonical_only(&path, &evidence);
    }
}

fn strip_rust_comments(source: &str) -> String {
    mask_rust_source(source, true)
}

fn rust_character_end(source: &str, start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut end = start + 1;
    if bytes.get(end) == Some(&b'\\') {
        end += 1;
        match bytes.get(end)? {
            b'u' if bytes.get(end + 1) == Some(&b'{') => {
                end += 2;
                while bytes
                    .get(end)
                    .is_some_and(|byte| byte.is_ascii_hexdigit() || *byte == b'_')
                {
                    end += 1;
                }
                if bytes.get(end) != Some(&b'}') {
                    return None;
                }
                end += 1;
            }
            b'x' => end += 3,
            _ => end += 1,
        }
    } else {
        let character = source.get(end..)?.chars().next()?;
        if matches!(character, '\'' | '\n' | '\r') {
            return None;
        }
        end += character.len_utf8();
    }
    (bytes.get(end) == Some(&b'\'')).then_some(end + 1)
}

fn mask_rust_source(source: &str, preserve_literals: bool) -> String {
    let bytes = source.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut cursor = 0;
    while cursor < bytes.len() {
        let raw_start = if bytes[cursor] == b'r' {
            Some(cursor)
        } else if bytes[cursor] == b'b' && bytes.get(cursor + 1) == Some(&b'r') {
            Some(cursor + 1)
        } else {
            None
        };
        if let Some(raw_start) = raw_start {
            let mut quote = raw_start + 1;
            while bytes.get(quote) == Some(&b'#') {
                quote += 1;
            }
            if bytes.get(quote) == Some(&b'"') {
                let hashes = quote - raw_start - 1;
                let mut end = quote + 1;
                while end < bytes.len() {
                    if bytes[end] == b'"'
                        && bytes.get(end + 1..end + 1 + hashes)
                            == Some(&bytes[raw_start + 1..quote])
                    {
                        end += 1 + hashes;
                        if preserve_literals {
                            output.extend_from_slice(&bytes[cursor..end]);
                        } else {
                            output.push(b' ');
                        }
                        cursor = end;
                        break;
                    }
                    end += 1;
                }
                if cursor == end {
                    continue;
                }
            }
        }
        if bytes[cursor] == b'\''
            && let Some(end) = rust_character_end(source, cursor)
        {
            if preserve_literals {
                output.extend_from_slice(&bytes[cursor..end]);
            } else {
                output.push(b' ');
            }
            cursor = end;
            continue;
        }
        if bytes[cursor] == b'"' {
            let start = cursor;
            cursor += 1;
            while cursor < bytes.len() {
                if bytes[cursor] == b'\\' {
                    cursor = (cursor + 2).min(bytes.len());
                } else {
                    let quote = bytes[cursor] == b'"';
                    cursor += 1;
                    if quote {
                        break;
                    }
                }
            }
            if preserve_literals {
                output.extend_from_slice(&bytes[start..cursor]);
            } else {
                output.push(b' ');
            }
            continue;
        }
        if bytes.get(cursor..cursor + 2) == Some(b"//") {
            output.push(b' ');
            cursor += 2;
            while cursor < bytes.len() && bytes[cursor] != b'\n' {
                cursor += 1;
            }
            continue;
        }
        if bytes.get(cursor..cursor + 2) == Some(b"/*") {
            output.push(b' ');
            cursor += 2;
            let mut depth = 1usize;
            while cursor < bytes.len() && depth != 0 {
                if bytes.get(cursor..cursor + 2) == Some(b"/*") {
                    depth += 1;
                    cursor += 2;
                } else if bytes.get(cursor..cursor + 2) == Some(b"*/") {
                    depth -= 1;
                    cursor += 2;
                } else {
                    if bytes[cursor] == b'\n' {
                        output.push(b'\n');
                    }
                    cursor += 1;
                }
            }
            continue;
        }
        output.push(bytes[cursor]);
        cursor += 1;
    }
    String::from_utf8(output).expect("comment stripping preserves UTF-8 source bytes")
}

fn assert_allowed_qualified_syntax_paths(path: &Path, source: &str) {
    let code = mask_rust_source(source, false).replace("r#", "");
    let bytes = code.as_bytes();
    let mut tokens = Vec::new();
    let mut cursor = 0;
    while cursor < bytes.len() {
        let start = cursor;
        if bytes[cursor].is_ascii_alphabetic() || bytes[cursor] == b'_' {
            cursor += 1;
            while cursor < bytes.len()
                && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_')
            {
                cursor += 1;
            }
            tokens.push(&code[start..cursor]);
        } else if bytes.get(cursor..cursor + 2) == Some(b"::") {
            tokens.push("::");
            cursor += 2;
        } else {
            if !bytes[cursor].is_ascii_whitespace() {
                tokens.push(if bytes[cursor].is_ascii() {
                    &code[cursor..cursor + 1]
                } else {
                    "#"
                });
            }
            cursor += 1;
        }
    }
    for (index, token) in tokens.iter().enumerate() {
        if *token == "include" && tokens.get(index + 1) == Some(&"!") {
            panic!(
                "{} includes transitive certification evidence",
                path.display()
            );
        }
        if *token == "mod"
            && tokens.get(index + 1).is_some_and(|name| {
                name.bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
            })
            && tokens.get(index + 2) == Some(&";")
        {
            panic!(
                "{} declares an unscanned certification evidence module",
                path.display()
            );
        }
    }
    for (index, token) in tokens.iter().enumerate() {
        if *token == "use" {
            let end = tokens[index..]
                .iter()
                .position(|token| *token == ";")
                .map(|offset| index + offset + 1)
                .expect("terminated Rust import");
            let declaration = tokens[index..end].join("");
            if declaration.contains("mech_syntax") || declaration.contains("mech_core") {
                assert_allowed_mech_import(path, &declaration);
            }
        }
        if *token == "extern" && tokens.get(index + 1) == Some(&"crate") {
            assert!(
                !matches!(tokens.get(index + 2), Some(&"mech_syntax" | &"mech_core")),
                "extern crate aliases are outside canonical evidence"
            );
        }
    }
    for index in 0..tokens.len().saturating_sub(1) {
        if tokens[index] != "mech_syntax" || tokens[index + 1] != "::" {
            continue;
        }
        let mut parts = Vec::new();
        let mut next = index + 2;
        while next < tokens.len()
            && tokens[next]
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            parts.push(tokens[next]);
            if tokens.get(next + 1) != Some(&"::") {
                break;
            }
            next += 2;
        }
        let (module, item) = match parts.as_slice() {
            ["document"] | ["document", "parser"] | ["document", "parser", "canonical"] => continue,
            ["document", "parser", "canonical", item, ..] => ("document::parser::canonical", *item),
            ["document", "parser", item, ..] => ("document::parser", *item),
            ["document", item, ..] => ("document", *item),
            _ => panic!(
                "{} uses a noncanonical qualified syntax path: {}",
                path.display(),
                parts.join("::")
            ),
        };
        // Imports and qualified references share exactly one item allowlist.
        assert_allowed_mech_import(path, &format!("usemech_syntax::{module}::{{{item}}};"));
    }
}

fn assert_canonical_only(path: &Path, evidence: &str) {
    assert_allowed_qualified_syntax_paths(path, evidence);
    let uncommented = strip_rust_comments(evidence);
    let evidence = uncommented.as_str();
    let compact = evidence
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>()
        .replace("r#", "");
    let normalized = compact.replace(['{', '}'], "");
    for forbidden in [
        concat!("::", "lower"),
        concat!("usemech_", "core"),
        concat!("externcratemech_", "core"),
        concat!("mech_core::", "Program"),
    ] {
        if behavioral_evidence(path) && forbidden == concat!("usemech_", "core") {
            continue; // The import parser above checks this file's exact runtime-type allowance.
        }
        assert!(
            !compact.contains(forbidden) && !normalized.contains(forbidden),
            "{} imports forbidden certification authority {forbidden}",
            path.display()
        );
    }
    assert!(!compact.contains(concat!("lower/", "legacy")));
}

fn behavioral_evidence(path: &Path) -> bool {
    path.ends_with("src/engine/tests/canonical_source_completion.rs")
        || path.ends_with("src/engine/tests/canonical_source_semantics.rs")
}

fn assert_allowed_mech_import(path: &Path, declaration: &str) {
    let declaration = declaration
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    let declaration = strip_visibility_prefix(&declaration).to_owned();
    let declaration = declaration
        .strip_prefix("use::")
        .map(|path| format!("use{path}"))
        .unwrap_or(declaration);
    if behavioral_evidence(path) && declaration.contains("mech_core") {
        let inventory: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/architecture/canonical-evidence-imports.json"
        )))
        .expect("canonical evidence import inventory is valid JSON");
        let file = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("behavioral evidence has a file name");
        let allowed = if let Some(items) = declaration
            .strip_prefix(concat!("usemech_", "core::{"))
            .and_then(|items| items.strip_suffix("};"))
        {
            items
                .split(',')
                .filter(|item| !item.is_empty())
                .all(|item| {
                    inventory[file]["root"].as_array().is_some_and(|allowed| {
                        allowed.iter().any(|allowed| allowed.as_str() == Some(item))
                    })
                })
        } else if let Some(items) = declaration
            .strip_prefix(concat!("usemech_", "core::snapshot::{"))
            .and_then(|items| items.strip_suffix("};"))
        {
            items
                .split(',')
                .filter(|item| !item.is_empty())
                .all(|item| {
                    inventory[file]["snapshot"]
                        .as_array()
                        .is_some_and(|allowed| {
                            allowed.iter().any(|allowed| allowed.as_str() == Some(item))
                        })
                })
        } else {
            false
        };
        assert!(
            allowed,
            "{} imports outside the behavioral runtime-type allowance: {declaration}",
            path.display()
        );
        return;
    }
    assert!(
        !declaration.contains("mech_core"),
        "{} imports forbidden certification authority mech_core",
        path.display()
    );

    let allowed = if let Some(items) = declaration
        .strip_prefix("usemech_syntax::document::parser::canonical::{")
        .and_then(|items| items.strip_suffix("};"))
    {
        let allowed = [
            "CanonicalRuleOutcome",
            "CanonicalSourceRuleSnapshot",
            "parse_canonical_phase_2c_rule_for_test",
            "parse_canonical_phase_2i_rule_for_test",
        ];
        items
            .split(',')
            .filter(|item| !item.is_empty())
            .all(|item| allowed.contains(&item))
    } else if let Some(item) = declaration
        .strip_prefix("usemech_syntax::document::parser::canonical::")
        .and_then(|item| item.strip_suffix(';'))
    {
        matches!(
            item,
            "parse_canonical_phase_2c_rule_for_test" | "parse_canonical_phase_2i_rule_for_test"
        )
    } else if let Some(items) = declaration
        .strip_prefix("usemech_syntax::document::parser::{")
        .and_then(|items| items.strip_suffix("};"))
    {
        let allowed = [
            "canonical_rule_id",
            "canonical_rule_name",
            "rules",
            "MIN_PREFIX_PRESERVING_EVENTS",
        ];
        items
            .split(',')
            .filter(|item| !item.is_empty())
            .all(|item| allowed.contains(&item))
    } else if declaration == "usemech_syntax::document::parser::rules;" {
        true
    } else if let Some(items) = declaration
        .strip_prefix("usemech_syntax::document::{")
        .and_then(|items| items.strip_suffix("};"))
    {
        let allowed = [
            "DiagnosticPhase",
            "DiagnosticStore",
            "FixApplicability",
            "FoundSyntax",
            "NormalizedDiagnostic",
            "NormalizedDiagnosticFix",
            "NormalizedDiagnosticLabel",
            "Severity",
            "SyntaxElement",
            "TextEdit",
            "ArgumentListSyntax",
            "ArrayPatternSyntax",
            "AstNode",
            "DocumentId",
            "DocumentSyntax",
            "ExpectedSyntax",
            "ExpressionSyntax",
            "FactorSyntax",
            "FactorValueSyntax",
            "FormulaSyntax",
            "FunctionCallSyntax",
            "GreenChildren",
            "GreenElement",
            "GreenNode",
            "GreenToken",
            "KindScalarSyntax",
            "KindSyntax",
            "KindValueSyntax",
            "LiteralSyntax",
            "LiteralValueSyntax",
            "MapSyntax",
            "MatchArmSyntax",
            "MatrixSyntax",
            "NodeFlags",
            "NodeId",
            "ParentheticalExpressionSyntax",
            "ParseConfig",
            "ParseLimits",
            "PatternArrayItemSyntax",
            "RangeExpressionSyntax",
            "RangeSubscriptSyntax",
            "RecordSyntax",
            "RecoveryAction",
            "RecursiveCoreSyntax",
            "RecursiveSyntaxNode",
            "Revision",
            "RuleId",
            "StructureSyntax",
            "StructureValueSyntax",
            "SubscriptItemSyntax",
            "SyntaxKind",
            "SyntaxNode",
            "SyntaxToken",
            "TableKindSyntax",
            "TextRange",
            "TextSize",
            "TextSnapshot",
            "TokenFlags",
            "TokenId",
            "VariableDefineSyntax",
            "compact_debug_tree",
            "normalize_diagnostics",
            "phase_2i_node_kind",
            "parse_canonical_document",
            "reconstruct_source_range",
            "text_hash",
            "validate_lossless_range",
        ];
        items
            .split(',')
            .filter(|item| !item.is_empty())
            .all(|item| allowed.contains(&item))
    } else {
        false
    };
    assert!(
        allowed,
        "{} imports outside the enumerated canonical surface: {declaration}",
        path.display()
    );
}

fn strip_visibility_prefix(declaration: &str) -> &str {
    let Some(rest) = declaration.strip_prefix("pub") else {
        return declaration;
    };
    if rest.starts_with("use") {
        return rest;
    }
    let Some(rest) = rest.strip_prefix('(') else {
        return declaration;
    };
    let Some(end) = rest.find(')') else {
        return declaration;
    };
    &rest[end + 1..]
}

#[test]
fn canonical_authority_gate_rejects_glob_and_alias_routes() {
    for evidence in [
        concat!("use mech_syntax::document::", "lower::*;"),
        concat!("use ::mech_syntax::document::", "*;"),
        concat!("use mech_syntax::document::{", "lower::*,", "};"),
        concat!(
            "use {::mech_syntax as syntax_alias};",
            "syntax_alias::parse(\"1\");"
        ),
        concat!("use mech_", "core::*;"),
        concat!("use mech_", "core::{Program};"),
        concat!(
            "use mech_syntax::document::",
            "lower as syntax_lower;",
            "syntax_lower::",
            "lower_legacy_grammar();"
        ),
        concat!("mech_syntax/* detached path comment */::", "parse(\"1\");"),
        concat!("mech_syntax// detached path comment\n::", "parse(\"1\");"),
        concat!("mech_syntax::r#", "parse(\"1\");"),
        concat!("mech_syntax::document::", "parse_document(source, config);"),
        concat!(
            "::mech_syntax::document::",
            "parse_syntax(source, root, config);"
        ),
        concat!(
            "mech_syntax::document::parser::",
            "parse_document(source, config);"
        ),
        concat!(
            "mech_syntax::document/* path gap */::parser::r#",
            "parse_syntax(source, root, config);"
        ),
        concat!(
            "mech_syntax::document::",
            "parse_fragment(source, FragmentKind::Expression, config);"
        ),
        concat!(
            "mech_syntax::document::parser::",
            "parse_fragment(source, FragmentKind::VariableDefine, config);"
        ),
        concat!(
            "mech_syntax::document::parser::fragment::",
            "parse_fragment(source, FragmentKind::Expression, config);"
        ),
        concat!(
            "mech_syntax::document::",
            "DocumentSession::new(source, config);"
        ),
        concat!(
            "mech_syntax::document::",
            "incremental::DocumentSession::new_with_document(document, source, config);"
        ),
        concat!(
            "mech_syntax::document::",
            "incremental::session::DocumentSession::new(source, config);"
        ),
        concat!(
            "mech_syntax::document/* path gap */::r#incremental::reparse::",
            "reparse(snapshot, edits, config, ids);"
        ),
        concat!(
            "fn bypass() { use mech_syntax as syntax; ",
            "syntax::expression(input); }"
        ),
        concat!(
            "fn bypass() { use {::mech_syntax as syntax}; ",
            "syntax::expression(input); }"
        ),
        concat!("let quote = '\"'; mech_syntax::", "expression(input);"),
        concat!("let slash = '/'; mech_syntax::", "expression(input);"),
        concat!("mech_syntax::expressions::", "expression(input);"),
        concat!("mech_syntax::", "expression(input);"),
        concat!("mech_syntax::", "ParseString::new(&graphemes);"),
        concat!("mech_syntax::structures::", "matrix(input);"),
        concat!(
            "return mech_syntax /* gap */ :: expressions :: ",
            "expression(input);"
        ),
        concat!(
            "mech_syntax::document::parser::mech::",
            "parse_expression(context);"
        ),
        "pub use mech_syntax::document::*; lower_legacy_grammar();",
        "pub(crate) use mech_syntax::document::*; lower_legacy_grammar();",
        "mod helper;",
        "#[path = \"alternate.rs\"] mod helper;",
        "include!(\"generated_evidence.rs\");",
    ] {
        assert!(
            std::panic::catch_unwind(|| assert_canonical_only(Path::new("fixture.rs"), evidence))
                .is_err(),
            "authority route was accepted: {evidence}"
        );
    }

    assert_canonical_only(
        Path::new("fixture.rs"),
        "use ::mech_syntax::document::{AstNode};",
    );
}

#[test]
fn typed_access_evidence_ignores_snapshot_identity_but_retains_accessor_text() {
    let parse = |text, document, revision| {
        parse_canonical_phase_2i_rule_for_test(
            TextSnapshot::new(DocumentId(document), Revision(revision), text).unwrap(),
            canonical_rule_id("var").unwrap(),
            ParseConfig::default(),
        )
        .unwrap()
    };
    // Distinct Rust wrapper types expose the same canonical accessor result.
    #[derive(Clone)]
    struct Accessor<const LOCATION: usize>(SyntaxNode);
    impl<const LOCATION: usize> AstNode for Accessor<LOCATION> {
        fn can_cast(_: mech_syntax::document::SyntaxKind) -> bool {
            true
        }
        fn cast(syntax: SyntaxNode) -> Option<Self> {
            Some(Self(syntax))
        }
        fn syntax(&self) -> &SyntaxNode {
            &self.0
        }
    }
    let first = parse("alpha", 1, 1);
    let mut original = StableHash::new();
    let mut relocated = StableHash::new();
    hash_node(&mut original, "value", Some(Accessor::<1>(first.syntax())));
    hash_node(&mut relocated, "value", Some(Accessor::<2>(first.syntax())));
    assert_eq!(original.0, relocated.0);
    let same = parse("alpha", 99, 500);
    let changed = parse("omega", 1, 1);
    assert_eq!(
        typed_access_hash("var", &first.syntax()),
        typed_access_hash("var", &same.syntax())
    );
    assert_ne!(
        typed_access_hash("var", &first.syntax()),
        typed_access_hash("var", &changed.syntax())
    );
}

#[test]
fn behavioral_authority_allowance_excludes_parser_routes_and_unrelated_core_types() {
    let path = Path::new("src/engine/tests/canonical_source_completion.rs");
    assert_canonical_only(
        path,
        concat!(
            "use mech_",
            "core::{FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef, ValueDataDraft as Data};"
        ),
    );
    for evidence in [
        "use mech_syntax::document::*;",
        concat!("use mech_syntax::document::", "lower::*;"),
        concat!("use mech_", "core::{Program};"),
        concat!("use mech_", "core::*;"),
        concat!("use mech_", "core::{FunctionCatalogBuilder, Program};"),
        concat!("use mech_", "core::snapshot::{Value};"),
    ] {
        assert!(
            std::panic::catch_unwind(|| assert_canonical_only(path, evidence)).is_err(),
            "{evidence}"
        );
    }

    let semantic_path = Path::new("src/engine/tests/canonical_source_semantics.rs");
    assert_canonical_only(
        semantic_path,
        "use mech_syntax::document::{DocumentSyntax, parse_canonical_document};",
    );
    assert_canonical_only(
        semantic_path,
        concat!(
            "use mech_",
            "core::{CanonicalNominalPath, ChangeDetectionPolicy, FunctionCatalogBuilder, IntegerWidth, KindExpr, ManagedMemoryBudget, OutputConstruction, ReactiveInstanceId, ResidentValueRef, SchemaBody, ShapeRule, ValueData, ValueDataDraft};"
        ),
    );
    assert_canonical_only(
        semantic_path,
        concat!(
            "use mech_",
            "core::snapshot::{ReifiedKind, ReifiedTypeDraft};"
        ),
    );
    for evidence in [
        concat!("use mech_", "core::{Program};"),
        concat!("use mech_", "core::*;"),
        concat!("use mech_", "core::{SchemaBody, Program};"),
        concat!("use mech_", "core::snapshot::{Value};"),
    ] {
        assert!(
            std::panic::catch_unwind(|| assert_canonical_only(semantic_path, evidence)).is_err(),
            "{evidence}"
        );
    }
}
