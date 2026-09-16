#![cfg(feature = "source")]

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use mech_engine::{
    CanonicalSourceFrontend, CanonicalSourceProgram, CardinalitySpec, DimensionExpr, FloatWidth,
    IntegerWidth, PHASE_2I_SEMANTIC_RULES, Phase2iSemanticDisposition, ProgramArtifact, SchemaBody,
    SourceNodeOutput, SourceSemanticAnchor, SourceValue, canonical_application_requirement_bytes,
    encode_program_artifact_bytecode_v1,
};
use mech_syntax::document::parser::canonical::parse_canonical_phase_2i_rule_for_test;
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    AstNode, DocumentId, ExpressionSyntax, FunctionCallSyntax, ParseConfig, Revision, SyntaxKind,
    SyntaxNode, TextSize, TextSnapshot, VariableDefineSyntax, phase_2i_node_kind,
};

struct CertificationContract {
    semantic_source: Option<String>,
    disposition: String,
    semantic_snapshot_hash: String,
    required_outcome: String,
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn find(node: SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if node.kind() == kind {
        return Some(node);
    }
    node.children().find_map(|child| find(child, kind))
}

fn expression(source: &str) -> ExpressionSyntax {
    let parsed = parse_canonical_phase_2i_rule_for_test(
        TextSnapshot::new(DocumentId(0x549), Revision(7), source).unwrap(),
        rules::EXPRESSION,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(parsed.is_strictly_clean(), "{source:?}");
    assert_eq!(
        parsed.consumed.end,
        TextSize(source.len() as u32),
        "{source:?}"
    );
    find(parsed.syntax(), SyntaxKind::Expression)
        .and_then(ExpressionSyntax::cast)
        .expect("canonical Expression")
}

fn variable_definition(source: &str) -> VariableDefineSyntax {
    let parsed = parse_canonical_phase_2i_rule_for_test(
        TextSnapshot::new(DocumentId(0x549), Revision(7), source).unwrap(),
        rules::VARIABLE_DEFINE,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(parsed.is_strictly_clean(), "{source:?}");
    assert_eq!(
        parsed.consumed.end,
        TextSize(source.len() as u32),
        "{source:?}"
    );
    find(parsed.syntax(), SyntaxKind::VariableDefine)
        .and_then(VariableDefineSyntax::cast)
        .expect("canonical VariableDefine")
}

fn certification_contracts() -> BTreeMap<String, CertificationContract> {
    let table = fs::read_to_string(
        repository_root().join("docs/design/grammar-audit/phase-2i-certification.tsv"),
    )
    .unwrap();
    table
        .lines()
        .skip(1)
        .map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            assert_eq!(fields.len(), 16);
            (
                fields[0].to_owned(),
                CertificationContract {
                    semantic_source: (fields[10] != "none")
                        .then(|| serde_json::from_str(fields[10]).expect("semantic source JSON")),
                    disposition: fields[9].to_owned(),
                    semantic_snapshot_hash: fields[13].to_owned(),
                    required_outcome: fields[15].to_owned(),
                },
            )
        })
        .collect()
}

fn contains_named_test(source: &str, name: &str) -> bool {
    let bytes = source.as_bytes();
    let mut code = Vec::with_capacity(bytes.len());
    let mut cursor = 0;
    while cursor < bytes.len() {
        if bytes.get(cursor..cursor + 2) == Some(b"//") {
            code.push(b' ');
            cursor += 2;
            while cursor < bytes.len() && bytes[cursor] != b'\n' {
                cursor += 1;
            }
            continue;
        }
        if bytes.get(cursor..cursor + 2) == Some(b"/*") {
            code.push(b' ');
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
                    cursor += 1;
                }
            }
            continue;
        }
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
                cursor = quote + 1;
                while cursor < bytes.len() {
                    if bytes[cursor] == b'"'
                        && bytes.get(cursor + 1..cursor + 1 + hashes)
                            == Some(&bytes[raw_start + 1..quote])
                    {
                        cursor += 1 + hashes;
                        break;
                    }
                    cursor += 1;
                }
                code.push(b' ');
                continue;
            }
        }
        let quote = if bytes[cursor] == b'"' {
            Some(cursor)
        } else if bytes[cursor] == b'b' && bytes.get(cursor + 1) == Some(&b'"') {
            Some(cursor + 1)
        } else {
            None
        };
        if let Some(quote) = quote {
            cursor = quote + 1;
            while cursor < bytes.len() {
                if bytes[cursor] == b'\\' {
                    cursor = (cursor + 2).min(bytes.len());
                } else {
                    let end = bytes[cursor] == b'"';
                    cursor += 1;
                    if end {
                        break;
                    }
                }
            }
            code.push(b' ');
            continue;
        }
        code.push(bytes[cursor]);
        cursor += 1;
    }
    let compact = String::from_utf8(code)
        .expect("masking preserves UTF-8")
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    compact.contains(&format!("#[test]fn{name}("))
}

#[test]
fn semantic_completion_record_keeps_artifact_evidence_from_opening_s6() {
    let table = fs::read_to_string(
        repository_root().join("docs/design/grammar-audit/phase-2i-semantic-completion.tsv"),
    )
    .unwrap();
    let mut lines = table.lines();
    assert_eq!(
        lines.next(),
        Some("capability\tgrammar-name\tresult\ttarget\towner\trequired-for-s6\tevidence")
    );
    let rows = lines
        .map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            assert_eq!(fields.len(), 7, "{line}");
            assert!(matches!(
                fields[2],
                "artifact-ready"
                    | "behavior-demonstrated"
                    | "intentionally-unavailable"
                    | "unfinished-implementation"
            ));
            assert!(matches!(fields[5], "true" | "false"));
            assert!(!fields[3].is_empty(), "completion target must be named");
            let (evidence_path, evidence_symbol) = fields[6]
                .split_once("::")
                .map_or((fields[6], None), |(path, symbol)| (path, Some(symbol)));
            let evidence_source = fs::read_to_string(repository_root().join(evidence_path))
                .unwrap_or_else(|error| {
                    panic!("missing completion evidence {evidence_path}: {error}")
                });
            assert!(
                evidence_symbol.is_none_or(|symbol| contains_named_test(&evidence_source, symbol)),
                "missing completion evidence test {}",
                fields[6]
            );
            fields
        })
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 6);
    let by_capability = rows
        .iter()
        .map(|fields| (fields[0], fields))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        by_capability.len(),
        rows.len(),
        "duplicate completion capability"
    );
    assert_eq!(by_capability["fsm-artifact"][2], "artifact-ready");
    assert_eq!(by_capability["fsm-artifact"][5], "false");
    assert_eq!(by_capability["fsm-runtime"][2], "intentionally-unavailable");
    assert_eq!(by_capability["fsm-runtime"][3], "resident-artifact");
    assert_eq!(by_capability["fsm-runtime"][5], "true");
    let s6_ready = rows
        .iter()
        .filter(|fields| fields[5] == "true")
        .all(|fields| fields[2] == "behavior-demonstrated");
    assert!(
        !s6_ready,
        "artifact readiness cannot satisfy the S6 semantic-completion gate"
    );
    assert_eq!(
        rows.iter()
            .filter(|fields| fields[5] == "true" && fields[2] != "behavior-demonstrated")
            .count(),
        1
    );
    assert!(!contains_named_test(
        "// #[test] fn stale() {}\nconst TEXT: &str = \"#[test] fn stale() {}\";",
        "stale"
    ));
}

#[derive(Clone, Copy)]
struct StableHash(u64);

impl StableHash {
    const fn new() -> Self {
        Self(0xcbf29ce484222325)
    }

    fn field(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }

    fn bytes(&mut self, value: &[u8]) {
        self.raw(&(value.len() as u64).to_le_bytes());
        self.raw(value);
    }

    fn u32(&mut self, value: u32) {
        self.raw(&value.to_le_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.raw(&value.to_le_bytes());
    }

    fn usize(&mut self, value: usize) {
        self.raw(&(value as u64).to_le_bytes());
    }

    fn raw(&mut self, value: &[u8]) {
        for byte in value {
            self.byte(*byte);
        }
    }

    fn byte(&mut self, byte: u8) {
        self.0 ^= u64::from(byte);
        self.0 = self.0.wrapping_mul(0x100000001b3);
    }
}

fn hash_source_value(hash: &mut StableHash, value: SourceValue) {
    match value {
        SourceValue::Constant(id) => {
            hash.field("constant");
            hash.u32(id.get());
        }
        SourceValue::Input(id) => {
            hash.field("input");
            hash.u32(id);
        }
        SourceValue::State(id) => {
            hash.field("state");
            hash.u32(id);
        }
        SourceValue::NodeOutput {
            node,
            output_ordinal,
        } => {
            hash.field("node-output");
            hash.u32(node);
            hash.u32(u32::from(output_ordinal));
        }
    }
}

fn hash_anchor(hash: &mut StableHash, anchor: SourceSemanticAnchor) {
    hash.u64(anchor.document.0);
    hash.u64(anchor.revision.0);
    hash.u32(anchor.range.start.0);
    hash.u32(anchor.range.end.0);
}

fn hash_optional_u32(hash: &mut StableHash, value: Option<u32>) {
    match value {
        Some(value) => {
            hash.field("some");
            hash.u32(value);
        }
        None => hash.field("none"),
    }
}

fn hash_slot_shapes(
    hash: &mut StableHash,
    shapes: &[(mech_core::CellSlotId, &mech_core::ShapeInstance)],
) {
    hash.field("artifact-slot-shape-hints-v1");
    hash.usize(shapes.len());
    for (slot, shape) in shapes {
        hash.u32(slot.get());
        hash.bytes(&shape.canonical_bytes());
    }
}

fn semantic_snapshot_hash(compiled: &CanonicalSourceProgram, artifact: &ProgramArtifact) -> u64 {
    let mut hash = StableHash::new();
    let program = compiled.program();
    hash.field("canonical-source-program-v2");
    hash.usize(program.requirements.len());
    for (_, requirement) in program.requirements.iter() {
        hash.bytes(
            &canonical_application_requirement_bytes(requirement)
                .expect("canonical application requirement"),
        );
    }
    hash.usize(program.inputs.len());
    for input in &program.inputs {
        hash.field(&input.name);
        hash.u32(input.schema.get());
    }
    hash.usize(program.states.len());
    for state in &program.states {
        hash.u32(state.schema.get());
        match state.initializer {
            Some(value) => {
                hash.field("initializer");
                hash_source_value(&mut hash, value);
            }
            None => hash.field("no-initializer"),
        }
        hash.u32(state.producer_node);
        hash.u32(u32::from(state.producer_output_ordinal));
    }
    hash.usize(program.nodes.len());
    for node in &program.nodes {
        match &node.body {
            mech_engine::SourceNodeBody::Operation {
                operation,
                requirement,
            } => {
                hash.usize(operation.module_path.len());
                for segment in &operation.module_path {
                    hash.field(segment);
                }
                hash.field(&operation.operation_name);
                hash_optional_u32(&mut hash, requirement.map(|id| id.get()));
            }
            mech_engine::SourceNodeBody::Comprehension(_) => {
                // Ordered qualifiers, patterns, and yields are owned by the
                // typed artifact bytecode included below.
                hash.field("Comprehension");
            }
            mech_engine::SourceNodeBody::Match(_) => {
                // The complete typed control body is sealed by the artifact
                // bytecode below, including captures, guards, operations and yields.
                hash.field("Match");
            }
            mech_engine::SourceNodeBody::Activation(_) => {
                // The complete typed activation body is sealed by the artifact
                // bytecode below, including patterns, captures and register writes.
                hash.field("Activation");
            }
            mech_engine::SourceNodeBody::Fsm(_) => {
                // Machine identity, named arguments, stage kinds, and complete
                // typed values are sealed by artifact bytecode below.
                hash.field("Fsm");
            }
        }
        hash.usize(node.inputs.len());
        for input in &node.inputs {
            hash_source_value(&mut hash, *input);
        }
        hash.usize(node.outputs.len());
        for output in &node.outputs {
            match output {
                SourceNodeOutput::State(id) => {
                    hash.field("state");
                    hash.u32(*id);
                }
                SourceNodeOutput::Derived { schema } => {
                    hash.field("derived");
                    hash.u32(schema.get());
                }
            }
        }
    }
    hash.usize(program.outputs.len());
    for output in &program.outputs {
        hash.field(&output.name);
        match &output.interactive_symbol {
            Some(symbol) => {
                hash.field("interactive");
                hash.field(symbol);
            }
            None => hash.field("ordinary"),
        }
        hash_source_value(&mut hash, output.source);
        hash.u32(output.schema.get());
    }
    hash.usize(program.constraints.len());
    for constraint in &program.constraints {
        hash.field(&constraint.name);
        hash.field(&constraint.operation.canonical_name());
        hash.usize(constraint.inputs.len());
        for input in &constraint.inputs {
            hash_source_value(&mut hash, *input);
        }
    }
    hash.usize(compiled.schemas().len());
    for entry in compiled.schemas().entries() {
        hash.bytes(entry.key().as_bytes());
        hash.bytes(entry.canonical_bytes());
    }
    hash.usize(compiled.constants().len());
    for raw in 0..compiled.constants().len() {
        let entry = compiled
            .constants()
            .entry(mech_engine::ConstantId::new(raw as u32))
            .expect("dense constant store");
        hash.bytes(entry.hash().as_bytes());
    }
    hash.usize(compiled.contracts().len());
    for contract in compiled.contracts() {
        match contract {
            Some(contract) => {
                hash.field("contract");
                hash.bytes(&serde_json::to_vec(contract).expect("serialized operation contract"));
            }
            None => hash.field("unresolved"),
        }
    }
    let source_map = compiled.source_map();
    hash.usize(source_map.inputs.len());
    for anchor in &source_map.inputs {
        hash_anchor(&mut hash, *anchor);
    }
    hash.usize(source_map.nodes.len());
    for node in &source_map.nodes {
        hash.field(&node.operation);
        hash.field(node.role);
        match &node.detail {
            Some(detail) => {
                hash.field("detail");
                hash.field(detail);
            }
            None => hash.field("no-detail"),
        }
        hash_anchor(&mut hash, node.anchor);
    }
    hash.usize(source_map.patterns.len());
    for pattern in &source_map.patterns {
        hash.field(&pattern.source);
        hash.usize(pattern.bindings.len());
        for binding in &pattern.bindings {
            hash.field(binding);
        }
        hash_anchor(&mut hash, pattern.anchor);
    }
    hash.usize(source_map.outputs.len());
    for anchor in &source_map.outputs {
        hash_anchor(&mut hash, *anchor);
    }
    hash.field("artifact-bytecode-v1");
    hash.bytes(
        &encode_program_artifact_bytecode_v1(artifact).expect("canonical artifact bytecode v1"),
    );
    let slot_shape_hints = artifact
        .slots()
        .iter()
        .filter_map(|slot| {
            artifact
                .slot_shape_hint(slot.slot)
                .map(|shape| (slot.slot, shape))
        })
        .collect::<Vec<_>>();
    hash_slot_shapes(&mut hash, &slot_shape_hints);
    hash.0
}

#[test]
fn semantic_evidence_distinguishes_non_wire_shape_values_and_slot_ownership() {
    let parameter = mech_core::DimensionParameterId::new(0);
    let schema = mech_core::SchemaDraft {
        dimension_parameters: vec![mech_core::DimensionParameterDeclaration {
            id: parameter,
            origin: mech_core::DimensionParameterOrigin::Explicit,
            lifetime: mech_core::DimensionLifetime::Activation,
            lower_bound: DimensionExpr::Constant(1),
            upper_bound: Some(DimensionExpr::Constant(3)),
        }]
        .into_boxed_slice(),
        body: SchemaBody::Matrix {
            element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
            dimensions: vec![DimensionExpr::Parameter(parameter)].into_boxed_slice(),
        },
    }
    .finalize()
    .unwrap();
    assert_eq!(schema.dimension_parameters().len(), 1);
    let two = schema.instantiate_shape(Box::new([2])).unwrap();
    let three = schema.instantiate_shape(Box::new([3])).unwrap();
    let snapshot = |shapes: &[(mech_core::CellSlotId, &mech_core::ShapeInstance)]| {
        let mut hash = StableHash::new();
        hash_slot_shapes(&mut hash, shapes);
        hash.0
    };
    let slot = mech_core::CellSlotId::new(0);
    let original = snapshot(&[(slot, &two)]);
    assert_ne!(original, snapshot(&[]));
    assert_ne!(original, snapshot(&[(slot, &three)]));
    assert_ne!(original, snapshot(&[(mech_core::CellSlotId::new(1), &two)]));
}

#[test]
fn slice_semantic_evidence_preserves_linear_gathers_and_whole_identity() {
    let contracts = certification_contracts();
    let source = contracts["slice"]
        .semantic_source
        .as_deref()
        .expect("slice semantic source");
    let compiled = CanonicalSourceFrontend
        .compile_expression(&expression(source))
        .unwrap();
    assert_eq!(compiled.program().inputs.len(), 1);
    assert_eq!(compiled.program().inputs[0].name, "x");
    let operations = compiled
        .source_map()
        .nodes
        .iter()
        .filter(|node| node.operation.starts_with("access/"))
        .map(|node| node.operation.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        operations,
        ["access/scalar", "access/range", "access/range"]
    );
    let tuple = compiled.program().nodes.last().expect("tuple result");
    assert_eq!(
        tuple.operation().unwrap().canonical_name(),
        "core/composite-pack"
    );
    assert_eq!(tuple.inputs.len(), 3);
    let SourceValue::NodeOutput {
        node,
        output_ordinal: 0,
    } = tuple.inputs[2]
    else {
        panic!("one-axis all must retain its gather output")
    };
    let gather = &compiled.program().nodes[node as usize];
    assert_eq!(gather.operation().unwrap().canonical_name(), "access/range");
    assert_eq!(gather.inputs.as_ref(), &[SourceValue::Input(0)]);
    compiled.compile_artifact().unwrap();

    let linear = CanonicalSourceFrontend
        .compile_expression(&expression("(x<[f64]:2,3>, x[:][:])"))
        .unwrap();
    assert_eq!(
        linear
            .program()
            .nodes
            .iter()
            .filter(|node| node
                .operation()
                .is_some_and(|operation| operation.canonical_name() == "access/range"))
            .count(),
        2
    );
    let linear_artifact = linear.compile_artifact().unwrap();
    let SchemaBody::Tuple(items) = linear_artifact
        .schemas()
        .get(linear_artifact.outputs()[0].schema)
        .unwrap()
        .body()
    else {
        panic!()
    };
    assert!(
        matches!(&items[1], SchemaBody::Matrix { dimensions, .. } if dimensions.as_ref() == [DimensionExpr::Constant(6), DimensionExpr::Constant(1)])
    );

    let identity = CanonicalSourceFrontend
        .compile_expression(&expression("(x<[f64]:2,3>, x[:,:][:,:])"))
        .unwrap();
    assert_eq!(identity.program().nodes.len(), 1);
    assert_eq!(
        identity.program().nodes[0]
            .operation()
            .unwrap()
            .canonical_name(),
        "core/composite-pack"
    );
    assert_eq!(
        identity.program().nodes[0].inputs.as_ref(),
        &[SourceValue::Input(0), SourceValue::Input(0)]
    );
    let artifact = identity.compile_artifact().unwrap();
    let SchemaBody::Tuple(items) = artifact
        .schemas()
        .get(artifact.outputs()[0].schema)
        .unwrap()
        .body()
    else {
        panic!()
    };
    assert!(
        matches!(&items[1], SchemaBody::Matrix { dimensions, .. } if dimensions.as_ref() == [DimensionExpr::Constant(2), DimensionExpr::Constant(3)])
    );
}

#[test]
fn compound_kind_evidence_retains_each_resolved_schema() {
    let schema = |source: &str| {
        let compiled = CanonicalSourceFrontend
            .compile_expression(&expression(source))
            .unwrap_or_else(|error| panic!("{source:?}: {error}"));
        compiled
            .schemas()
            .get(compiled.program().inputs[0].schema)
            .expect("input schema")
            .body()
            .clone()
    };

    assert!(matches!(
        schema("x<{u8:f64}>"),
        SchemaBody::Map { key, value, cardinality }
            if matches!(key.as_ref(), SchemaBody::UnsignedInteger(IntegerWidth::W8))
                && matches!(value.as_ref(), SchemaBody::FloatingPoint(FloatWidth::W64))
                && cardinality == CardinalitySpec::Dynamic { upper_bound: None }
    ));
    assert!(matches!(
        schema("x<[u8]:1,2>"),
        SchemaBody::Matrix { element, dimensions }
            if matches!(element.as_ref(), SchemaBody::UnsignedInteger(IntegerWidth::W8))
                && dimensions.as_ref()
                    == [DimensionExpr::Constant(1), DimensionExpr::Constant(2)]
    ));
    assert!(matches!(
        schema("x<{a<u8>}>"),
        SchemaBody::Record(fields)
            if fields.len() == 1
                && fields[0].name == "a"
                && matches!(fields[0].schema, SchemaBody::UnsignedInteger(IntegerWidth::W8))
    ));
    assert!(matches!(
        schema("x<{u8}:10>"),
        SchemaBody::Set { element, cardinality }
            if matches!(element.as_ref(), SchemaBody::UnsignedInteger(IntegerWidth::W8))
                && cardinality == CardinalitySpec::Exact(DimensionExpr::Constant(10))
    ));
    assert!(matches!(
        schema("x<|a<u8>|:10>"),
        SchemaBody::Table { columns, rows }
            if columns.len() == 1
                && columns[0].name == "a"
                && matches!(columns[0].schema, SchemaBody::UnsignedInteger(IntegerWidth::W8))
                && rows == CardinalitySpec::Exact(DimensionExpr::Constant(10))
    ));
    assert!(matches!(
        schema("x<(u8,f64)>"),
        SchemaBody::Tuple(items)
            if matches!(items[0], SchemaBody::UnsignedInteger(IntegerWidth::W8))
                && matches!(items[1], SchemaBody::FloatingPoint(FloatWidth::W64))
    ));
}

#[test]
fn every_semantic_rule_meets_its_required_witness_outcome() {
    let contracts = certification_contracts();
    let certified = PHASE_2I_SEMANTIC_RULES
        .iter()
        .filter(|rule| rule.disposition != Phase2iSemanticDisposition::Structural)
        .collect::<Vec<_>>();
    assert_eq!(certified.len(), 53);

    for rule in PHASE_2I_SEMANTIC_RULES {
        let expected = match rule.disposition {
            Phase2iSemanticDisposition::Executable => "executable",
            Phase2iSemanticDisposition::Structural => "structural",
            Phase2iSemanticDisposition::CompileTime => "compile-time",
        };
        assert_eq!(
            contracts
                .get(rule.grammar_name)
                .map(|contract| contract.disposition.as_str()),
            Some(expected)
        );
    }

    let mut stale_hashes = Vec::new();
    let mut unfinished_witnesses = Vec::new();
    for rule in certified {
        let contract = contracts
            .get(rule.grammar_name)
            .unwrap_or_else(|| panic!("missing certification row for {}", rule.grammar_name));
        let semantic_source = contract
            .semantic_source
            .as_deref()
            .unwrap_or_else(|| panic!("missing semantic context for {}", rule.grammar_name));
        let (syntax, compiled) = if rule.grammar_name == "variable-define" {
            let definition = variable_definition(semantic_source);
            let syntax = definition.syntax().clone();
            let compiled = CanonicalSourceFrontend.compile_definition(&definition);
            (syntax, compiled)
        } else {
            let expression = expression(semantic_source);
            let syntax = expression.syntax().clone();
            let compiled = CanonicalSourceFrontend.compile_expression(&expression);
            (syntax, compiled)
        };
        if let Some(kind) = phase_2i_node_kind(rule.grammar_name) {
            assert!(
                find(syntax.clone(), kind).is_some(),
                "{} semantic context does not contain {kind:?}",
                rule.grammar_name
            );
        }
        if let Some(expected_code) = contract
            .required_outcome
            .strip_prefix("expected-user-error:")
        {
            let error = match compiled {
                Err(error) => error,
                Ok(_) => {
                    unfinished_witnesses.push(format!(
                        "{} on {semantic_source:?} requires user error {expected_code}, but produced a program",
                        rule.grammar_name,
                    ));
                    continue;
                }
            };
            assert_eq!(
                error.code, expected_code,
                "{} on {semantic_source:?}",
                rule.grammar_name
            );
            assert_eq!(error.anchor.document, syntax.source().document());
            assert_eq!(error.anchor.revision, syntax.source().revision());
            let expected_range = if expected_code == "source-semantics/unknown-function" {
                find(syntax.clone(), SyntaxKind::FunctionCall)
                    .and_then(FunctionCallSyntax::cast)
                    .and_then(|call| call.function())
                    .expect("unknown-call witness has a function identifier")
                    .syntax()
                    .range()
            } else {
                syntax.range()
            };
            assert_eq!(error.anchor.range, expected_range, "{}", rule.grammar_name);
            assert_eq!(contract.semantic_snapshot_hash, "none");
            continue;
        }
        assert_eq!(
            contract.required_outcome, contract.disposition,
            "{}",
            rule.grammar_name
        );
        let compiled = match compiled {
            Ok(compiled) => compiled,
            Err(error) => {
                unfinished_witnesses.push(format!(
                    "{} requires {} on {semantic_source:?}; unfinished source lowering: {error}",
                    rule.grammar_name, contract.required_outcome,
                ));
                continue;
            }
        };
        assert_eq!(compiled.program().outputs.len(), 1, "{}", rule.grammar_name);
        assert_eq!(
            compiled.program().nodes.len(),
            compiled.contracts().len(),
            "{}",
            rule.grammar_name
        );
        assert_eq!(
            compiled.source_map().outputs[0].range,
            syntax.range(),
            "{}",
            rule.grammar_name
        );
        let artifact = match compiled.compile_artifact() {
            Ok(artifact) => artifact,
            Err(error) => {
                unfinished_witnesses.push(format!(
                    "{} on {semantic_source:?}: unfinished artifact lowering: {error:?}",
                    rule.grammar_name,
                ));
                continue;
            }
        };
        let actual = semantic_snapshot_hash(&compiled, &artifact);
        let expected = contract
            .semantic_snapshot_hash
            .parse::<u64>()
            .expect("semantic snapshot hash");
        if actual != expected {
            stale_hashes.push(format!("{}\t{}\t{}", rule.grammar_name, expected, actual));
        }
    }
    assert!(
        unfinished_witnesses.is_empty() && stale_hashes.is_empty(),
        "semantic completion requires artifact-ready witnesses and current fingerprints; unfinished lowering cannot be certified:\n{}\nsemantic snapshot hashes changed:\n{}",
        unfinished_witnesses.join("\n"),
        stale_hashes.join("\n"),
    );
}

#[test]
fn contextual_empty_structure_errors_keep_their_exact_syntax_anchors() {
    let audit = repository_root().join("docs/design/grammar-audit");
    let matrix = fs::read_to_string(audit.join("phase-2i-certification.tsv")).unwrap();
    let structure = matrix
        .lines()
        .find(|line| line.starts_with("structure\t"))
        .unwrap();
    let linked_cases = structure
        .split('\t')
        .nth(12)
        .unwrap()
        .split(',')
        .collect::<Vec<_>>();
    let errors = fs::read_to_string(audit.join("phase-2i-semantic-errors.tsv")).unwrap();
    let mut lines = errors.lines();
    assert_eq!(
        lines.next(),
        Some("case-id\tgrammar-name\tsyntax-kind\tsource-json\terror-code")
    );
    let mut witnessed = Vec::new();
    for line in lines {
        let fields = line.split('\t').collect::<Vec<_>>();
        assert_eq!(fields.len(), 5);
        assert!(
            linked_cases.contains(&fields[0]),
            "unlinked conformance case {}",
            fields[0]
        );
        assert_eq!(fields[1], "structure");
        let kind = match fields[2] {
            "EmptySet" => SyntaxKind::EmptySet,
            "EmptyMap" => SyntaxKind::EmptyMap,
            other => panic!("unexpected empty-structure owner {other}"),
        };
        let source: String = serde_json::from_str(fields[3]).unwrap();
        let syntax = expression(&source);
        let owner =
            find(syntax.syntax().clone(), kind).expect("context contains the required empty form");
        assert!(
            owner.range().start.0 > 0,
            "witness must test an inner source range"
        );
        let error = CanonicalSourceFrontend
            .compile_expression(&syntax)
            .err()
            .expect("untyped empty structure requires a source error");
        assert_eq!(error.code, fields[4], "{}", fields[0]);
        assert_eq!(error.anchor.document, DocumentId(0x549));
        assert_eq!(error.anchor.revision, Revision(7));
        assert_eq!(error.anchor.range, owner.range(), "{}", fields[0]);
        witnessed.push(fields[0]);
    }
    witnessed.sort_unstable();
    assert_eq!(witnessed, ["STRUCT-EMPTY-MAP", "STRUCT-EMPTY-SET"]);
}
