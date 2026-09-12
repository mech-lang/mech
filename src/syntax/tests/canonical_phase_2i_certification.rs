use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use mech_syntax::document::parser::canonical::{
    CanonicalRuleOutcome, parse_canonical_phase_2i_rule_for_test,
};
use mech_syntax::document::parser::{canonical_rule_id, canonical_rule_name, rules};
use mech_syntax::document::{
    ArrayPatternSyntax, AstNode, DocumentId, FormulaSyntax, KindSyntax, KindValueSyntax, NodeFlags,
    ParseConfig, PatternArrayItemSyntax, RecursiveCoreSyntax, Revision, SyntaxKind, SyntaxNode,
    SyntaxToken, TextRange, TextSize, TextSnapshot, compact_debug_tree, normalize_diagnostics,
    phase_2i_node_kind, reconstruct_source_range, validate_lossless_range,
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
            "grammar-name\taccepted-source-json\trejected-source-json\trecovery-source-json\temission-policy\tsyntax-kind\tclean-tree-hash\ttyped-access-hash\trecovery-snapshot-hash\tsemantic-disposition\tsemantic-source-json\tspec-location\tconformance-cases\tsemantic-snapshot-hash\tcanonical-consumer"
        )
    );
    lines
        .map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            assert_eq!(fields.len(), 15, "invalid certification row: {line}");
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

fn hash_node<N: AstNode + std::fmt::Debug>(hash: &mut StableHash, role: &str, value: Option<N>) {
    hash.field(role);
    hash.field(std::any::type_name::<N>());
    match value {
        Some(value) => {
            hash.field(&format!("typed:{value:?}"));
            hash.field(&format!(
                "some:{:?}:{}:{}:{}",
                value.syntax().kind(),
                value.syntax().range().start.0,
                value.syntax().range().end.0,
                value.syntax().flags().0,
            ));
        }
        None => hash.field("none"),
    }
}

fn hash_nodes<N: AstNode + std::fmt::Debug>(hash: &mut StableHash, role: &str, values: Vec<N>) {
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
        Some(value) => hash.field(&format!(
            "some:{:?}:{}:{}:{}:{}",
            value.kind(),
            value.range().start.0,
            value.range().end.0,
            value.flags().0,
            value.text().expect("clean source token text"),
        )),
        None => hash.field("none"),
    }
}

fn typed_access_hash(rule_name: &str, node: &SyntaxNode) -> u64 {
    let mut hash = StableHash::new();
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
        RecursiveCoreSyntax::RangeSubscript(view) => node!("range", view.range()),
        RecursiveCoreSyntax::Record(view) => nodes!("bindings", view.bindings()),
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

fn recovery_snapshot_hash(
    parsed: &mech_syntax::document::parser::canonical::CanonicalSourceRuleSnapshot,
) -> u64 {
    let mut hash = StableHash::new();
    hash.field(&compact_debug_tree(&parsed.syntax()));
    for diagnostic in
        normalize_diagnostics(&parsed.diagnostics, parsed.source.revision(), &parsed.nodes)
    {
        hash.field(&format!(
            "diagnostic:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}",
            diagnostic.code,
            diagnostic.phase,
            diagnostic.severity,
            diagnostic.rule,
            diagnostic.context,
            diagnostic.primary,
            diagnostic.expected,
            diagnostic.found,
            diagnostic.related,
            diagnostic.recovery,
            diagnostic.tags,
        ));
        for label in diagnostic.labels {
            hash.field(&format!("label:{:?}", label.range));
        }
        for fix in diagnostic.fixes {
            hash.field(&format!("fix:{:?}:{:?}", fix.applicability, fix.edits));
        }
    }
    hash.0
}

#[test]
fn certification_table_executes_every_direct_accept_reject_and_recovery_case() {
    let rows = certification_rows();
    let contracts = inventory_contracts();
    assert_eq!(rows.len(), 80);
    assert_eq!(contracts.len(), rows.len());
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
        assert_eq!(
            accepted.root.structural_hash, row.clean_tree_hash,
            "{}",
            row.name
        );
        validate_lossless_range(&accepted.root, &accepted.source, accepted.consumed).unwrap();
        assert_eq!(
            reconstruct_source_range(&accepted.root, &accepted.source, accepted.consumed).unwrap(),
            row.accepted,
            "{}",
            row.name
        );
        let typed_hash = typed_access_hash(&row.name, &accepted.syntax());
        assert_eq!(typed_hash, row.typed_access_hash, "{}", row.name);

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
        assert_eq!(recovery_hash, row.recovery_snapshot_hash, "{}", row.name);

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
                assert!(row.semantic_snapshot_hash.parse::<u64>().is_ok());
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
                assert!(row.semantic_snapshot_hash.parse::<u64>().is_ok());
            }
            _ => unreachable!("closed semantic disposition"),
        }
    }
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

fn assert_canonical_only(path: &Path, evidence: &str) {
    let mut declaration = None::<String>;
    for line in evidence.lines().map(str::trim) {
        let compact_line = line
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>();
        if declaration.is_none() && compact_line.starts_with("use") {
            declaration = Some(String::new());
        }
        if let Some(current) = declaration.as_mut() {
            current.push_str(line);
            if line.ends_with(';') {
                let completed = declaration.take().expect("active import declaration");
                if completed.contains("mech_syntax") || completed.contains(concat!("mech_", "core"))
                {
                    assert_allowed_mech_import(path, &completed);
                }
            }
        }
    }
    assert!(
        declaration.is_none(),
        "unterminated import in {}",
        path.display()
    );

    let compact = evidence
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    let normalized = compact.replace(['{', '}'], "");
    for forbidden in [
        concat!("mech_syntax::", "parser"),
        concat!("mech_syntax::", "parse"),
        concat!("usemech_syntax", "as"),
        concat!("mech_syntax::", "{"),
        concat!("mech_syntax::", "*"),
        concat!("externcratemech_", "syntax"),
        concat!("mech_syntax::document::", "lower"),
        concat!("::", "lower"),
        concat!("document::", "lower::", "legacy"),
        concat!("usemech_", "core"),
        concat!("externcratemech_", "core"),
        concat!("mech_core::", "Program"),
    ] {
        assert!(
            !compact.contains(forbidden) && !normalized.contains(forbidden),
            "{} imports forbidden certification authority {forbidden}",
            path.display()
        );
    }
    assert!(!compact.contains(concat!("lower/", "legacy")));
}

fn assert_allowed_mech_import(path: &Path, declaration: &str) {
    let declaration = declaration
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    let declaration = declaration
        .strip_prefix("use::")
        .map(|path| format!("use{path}"))
        .unwrap_or(declaration);
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
        let allowed = ["canonical_rule_id", "canonical_rule_name", "rules"];
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
            "ArgumentListSyntax",
            "ArrayPatternSyntax",
            "AstNode",
            "DocumentId",
            "ExpectedSyntax",
            "ExpressionSyntax",
            "FactorSyntax",
            "FactorValueSyntax",
            "FormulaSyntax",
            "GreenNode",
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
            "RecoveryAction",
            "RecursiveCoreSyntax",
            "RecursiveSyntaxNode",
            "Revision",
            "RuleId",
            "StructureSyntax",
            "StructureValueSyntax",
            "SyntaxKind",
            "SyntaxNode",
            "SyntaxToken",
            "TableKindSyntax",
            "TextRange",
            "TextSize",
            "TextSnapshot",
            "TokenFlags",
            "VariableDefineSyntax",
            "compact_debug_tree",
            "normalize_diagnostics",
            "phase_2i_node_kind",
            "reconstruct_source_range",
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
