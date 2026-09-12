#![cfg(feature = "source")]

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use mech_engine::{CanonicalSourceFrontend, PHASE_2I_SEMANTIC_RULES, Phase2iSemanticDisposition};
use mech_syntax::document::parser::canonical::parse_canonical_phase_2i_rule_for_test;
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    AstNode, DocumentId, ExpressionSyntax, ParseConfig, Revision, SyntaxKind, SyntaxNode, TextSize,
    TextSnapshot, VariableDefineSyntax, phase_2i_node_kind,
};

struct CertificationContract {
    semantic_source: Option<String>,
    disposition: String,
    semantic_snapshot_hash: String,
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
            assert_eq!(fields.len(), 15);
            (
                fields[0].to_owned(),
                CertificationContract {
                    semantic_source: (fields[10] != "none")
                        .then(|| serde_json::from_str(fields[10]).expect("semantic source JSON")),
                    disposition: fields[9].to_owned(),
                    semantic_snapshot_hash: fields[13].to_owned(),
                },
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

fn semantic_snapshot_hash(compiled: &mech_engine::CanonicalSourceProgram) -> u64 {
    let mut hash = StableHash::new();
    hash.field(&format!("program:{:#?}", compiled.program()));
    hash.field(&format!("schemas:{:#?}", compiled.schemas()));
    hash.field(&format!("constants:{:#?}", compiled.constants()));
    hash.field(&format!("contracts:{:#?}", compiled.contracts()));
    hash.field(&format!("source-map:{:#?}", compiled.source_map()));
    match compiled.compile_artifact() {
        Ok(artifact) => {
            hash.field("artifact:ok");
            hash.field(&format!("revision:{:?}", artifact.revision()));
            hash.field(&format!("schemas:{:#?}", artifact.schemas()));
            hash.field(&format!("constants:{:#?}", artifact.constants()));
            hash.field(&format!("contracts:{:#?}", artifact.contracts()));
            hash.field(&format!("requirements:{:#?}", artifact.requirements()));
            hash.field(&format!("inputs:{:#?}", artifact.inputs()));
            hash.field(&format!("slots:{:#?}", artifact.slots()));
            hash.field(&format!(
                "slot-shape-hints:{:#?}",
                artifact
                    .slots()
                    .iter()
                    .map(|slot| (slot.slot, artifact.slot_shape_hint(slot.slot)))
                    .collect::<Vec<_>>()
            ));
            hash.field(&format!("nodes:{:#?}", artifact.nodes()));
            hash.field(&format!("bindings:{:#?}", artifact.bindings()));
            hash.field(&format!("outputs:{:#?}", artifact.outputs()));
            hash.field(&format!("constraints:{:#?}", artifact.constraints()));
            hash.field(&format!(
                "compute-regions:{:#?}",
                artifact.compute_regions()
            ));
        }
        Err(error) => hash.field(&format!("artifact:error:{error:?}")),
    }
    hash.0
}

#[test]
fn slice_semantic_evidence_reaches_the_select_all_operation() {
    let contracts = certification_contracts();
    let source = contracts["slice"]
        .semantic_source
        .as_deref()
        .expect("slice semantic source");
    let compiled = CanonicalSourceFrontend
        .compile_expression(&expression(source))
        .unwrap();
    assert!(
        compiled
            .source_map()
            .nodes
            .iter()
            .any(|node| node.operation == "source/select-all")
    );
}

#[test]
fn every_semantic_rule_has_specification_derived_program_evidence() {
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
            let compiled = CanonicalSourceFrontend
                .compile_definition(&definition)
                .unwrap_or_else(|error| {
                    panic!("{} on {semantic_source:?}: {error}", rule.grammar_name)
                });
            (syntax, compiled)
        } else {
            let expression = expression(semantic_source);
            let syntax = expression.syntax().clone();
            let compiled = CanonicalSourceFrontend
                .compile_expression(&expression)
                .unwrap_or_else(|error| {
                    panic!("{} on {semantic_source:?}: {error}", rule.grammar_name)
                });
            (syntax, compiled)
        };
        if let Some(kind) = phase_2i_node_kind(rule.grammar_name) {
            assert!(
                find(syntax.clone(), kind).is_some(),
                "{} semantic context does not contain {kind:?}",
                rule.grammar_name
            );
        }
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
        let actual = semantic_snapshot_hash(&compiled);
        let expected = contract
            .semantic_snapshot_hash
            .parse::<u64>()
            .expect("semantic snapshot hash");
        assert_eq!(
            actual, expected,
            "{} on {semantic_source:?}",
            rule.grammar_name
        );
    }
}
