#![cfg(all(feature = "source_default", feature = "resident-artifact"))]

use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId, ValueData};
use mech_engine::resident::{ActivationFacts, activate};
use mech_engine::{CanonicalSourceFrontend, CanonicalSourceProgram, SourceNodeOutput};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document, reconstruct_source,
};

fn document(source: &str) -> DocumentSyntax {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x570), Revision(7), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.diagnostics.is_empty(),
        "{source:?}: {:?}",
        parsed.diagnostics
    );
    assert_eq!(
        reconstruct_source(&parsed.root, &parsed.source).unwrap(),
        source
    );
    let document = DocumentSyntax::cast(parsed.syntax()).unwrap();
    assert!(!document.syntax().flags().intersects(
        mech_syntax::document::NodeFlags::ERROR | mech_syntax::document::NodeFlags::CONTAINS_ERROR
    ));
    document
}

fn compiled(source: &str) -> CanonicalSourceProgram {
    CanonicalSourceFrontend
        .compile_document(&document(source))
        .unwrap_or_else(|error| panic!("{source:?}: {error}"))
}

fn turns(source: &str, expected: &[f64]) {
    turns_for_output(
        source,
        expected,
        mech_engine::SourceDocumentOutputKind::Program,
    );
}

fn turns_for_output(source: &str, expected: &[f64], kind: mech_engine::SourceDocumentOutputKind) {
    let compiled = compiled(source);
    assert!(compiled.program().inputs.is_empty());
    for state in 0..compiled.program().states.len() as u32 {
        assert_eq!(
            compiled
                .program()
                .nodes
                .iter()
                .flat_map(|node| node.outputs.iter())
                .filter(|output| **output == SourceNodeOutput::State(state))
                .count(),
            1,
            "each state retains one writer"
        );
    }
    let artifact = compiled
        .compile_artifact()
        .expect("document must construct a canonical artifact");
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    let mut instance = activate(
        ReactiveInstanceId::new(0x570, 0),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .expect("document must activate with the maintained resident catalog");
    let output = compiled
        .document_outputs()
        .iter()
        .find(|binding| binding.kind == kind)
        .unwrap()
        .output as usize;
    for expected in expected {
        instance
            .turn(&[])
            .expect("state update must execute and publish");
        let output = instance.copied_output(output).unwrap();
        let ValueData::F64(actual) = output.data() else {
            panic!("expected a scalar f64 result: {output:?}")
        };
        assert_eq!(actual.to_f64(), *expected, "{source:?}");
    }
}

#[test]
fn interactive_fixture_executes_and_retains_state_across_turns() {
    let source = include_str!("../../../tests/fixtures/syntax-source-boundary/interactive.mec");
    assert_eq!(source, "~answer := 0\nanswer += 1\nanswer\n");
    turns(source, &[1.0, 2.0]);
}

#[test]
fn discarded_candidate_does_not_advance_document_state() {
    let compiled = compiled("~answer := 0\nanswer += 1\nanswer\n");
    let artifact = compiled.compile_artifact().unwrap();
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    let mut instance = activate(
        ReactiveInstanceId::new(0x571, 0),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .unwrap();
    {
        let candidate = instance.prepare_turn(&[]).unwrap();
        let output = candidate.copied_output(0).unwrap();
        let ValueData::F64(value) = output.data() else {
            panic!("scalar candidate")
        };
        assert_eq!(value.to_f64(), 1.0);
    }
    for expected in [1.0, 2.0] {
        instance.turn(&[]).unwrap();
        let output = instance.copied_output(0).unwrap();
        let ValueData::F64(value) = output.data() else {
            panic!("scalar published output")
        };
        assert_eq!(value.to_f64(), expected);
    }
}

#[test]
fn sequential_updates_read_the_preceding_candidate_and_keep_one_writer() {
    turns(
        "~answer := 0\nanswer += 1\nanswer *= 2\nanswer\n",
        &[2.0, 6.0],
    );
    turns(
        "~answer := 1\nanswer += 1\nanswer += answer\nanswer\n",
        &[4.0, 10.0],
    );
    turns(
        "~answer := 0\nanswer = 3\nanswer += 1\nanswer\n",
        &[4.0, 4.0],
    );
    turns(
        "~left := 0\n~right := 1\nleft += right\nright += left\nright\n",
        &[2.0, 5.0],
    );
}

#[test]
fn immutable_reads_retain_their_source_order_value() {
    turns(
        "~answer := 0\nbefore := answer\nanswer += 1\nbefore\n",
        &[0.0, 1.0],
    );
    turns(
        "~answer := 0\nanswer += 1\nafter := answer\nanswer += 1\nafter\n",
        &[1.0, 3.0],
    );
}

#[test]
fn each_assignment_operator_uses_maintained_arithmetic() {
    for (source, expected) in [
        ("~answer := 8\nanswer -= 2\nanswer\n", [6.0, 4.0]),
        ("~answer := 8\nanswer /= 2\nanswer\n", [4.0, 2.0]),
        ("~answer := 2\nanswer ^= 2\nanswer\n", [4.0, 16.0]),
    ] {
        turns(source, &expected);
    }
}

#[test]
fn assignment_errors_are_anchored_to_the_target_or_value() {
    for (source, code, anchor_text) in [
        (
            "answer += 1\n",
            "source-semantics/unknown-assignment-target",
            "answer",
        ),
        (
            "answer := 0\nanswer += 1\n",
            "source-semantics/immutable-assignment-target",
            "answer",
        ),
        (
            "~answer := 0\ncopy := answer\ncopy += 1\n",
            "source-semantics/immutable-assignment-target",
            "copy",
        ),
        (
            "~answer := 0\nanswer = true\n",
            "source-semantics/incompatible-assignment-kind",
            "true",
        ),
        (
            "~answer := 0\nanswer[1] += 1\n",
            "source-semantics/unsupported-assignment-target",
            "[1]",
        ),
    ] {
        let error = CanonicalSourceFrontend
            .compile_document(&document(source))
            .err()
            .expect("invalid assignment must fail");
        assert_eq!(error.code, code, "{source:?}");
        assert_eq!(error.anchor.document, DocumentId(0x570));
        assert_eq!(error.anchor.revision, Revision(7));
        assert_eq!(
            &source[error.anchor.range.start.0 as usize..error.anchor.range.end.0 as usize],
            anchor_text
        );
    }
}

#[test]
fn document_display_and_child_scopes_do_not_execute_updates() {
    turns(
        "~answer := 0\n\nDisplayed {{answer += 100}}.\n\nanswer += 1\nanswer\n",
        &[1.0, 2.0],
    );
    turns(
        "~answer := 0\n~∘~⸢answer += 100\n⸥\nanswer += 1\nanswer\n",
        &[1.0, 2.0],
    );
    turns_for_output(
        "~answer := 0\nanswer += 1\n\nEvaluated {answer + 10}.\n",
        &[11.0, 12.0],
        mech_engine::SourceDocumentOutputKind::Inline,
    );
}
