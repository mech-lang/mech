//! Declared named outputs contribute lexical context before literal lowering.
#![cfg(feature = "full_compiler")]

use mech_core::{ReactiveInstanceId, SchemaBody, Value, ValueDataDraft, snapshot::OptionDraft};
use mech_engine::resident::{ActivationFacts, activate};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact, SourceDocumentOutputKind};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn syntax(source: &str) -> DocumentSyntax {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x464f5554), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    DocumentSyntax::cast(parsed.syntax()).unwrap()
}

fn artifacts(source: &str) -> ([ProgramArtifact; 2], usize) {
    let product = CanonicalSourceFrontend
        .compile_document_with_catalog(&syntax(source), mech_stdlib::source_catalog())
        .unwrap_or_else(|error| panic!("{source}: {error:?}"));
    let output = product
        .document_outputs()
        .iter()
        .rev()
        .find(|output| output.kind == SourceDocumentOutputKind::Program)
        .unwrap()
        .output as usize;
    let original = product.compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    assert_eq!(original.revision(), decoded.revision());
    assert_eq!(
        bytes,
        mech_engine::encode_program_artifact_bytecode_v1(&decoded).unwrap()
    );
    ([original, decoded], output)
}

fn evaluate_twice(source: &str, mut check: impl FnMut(&Value)) {
    let (artifacts, output) = artifacts(source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        assert!(artifact.inputs().is_empty());
        let mut instance = activate(
            ReactiveInstanceId::new(0x464f5554, ordinal as u32),
            artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
        )
        .unwrap();
        for _ in 0..2 {
            instance
                .prepare_turn_values(&[])
                .unwrap()
                .publish()
                .unwrap();
            check(&instance.copied_output(output).unwrap());
        }
    }
}

fn none() -> ValueDataDraft {
    ValueDataDraft::Option(OptionDraft {
        present: false,
        value: None,
    })
}

#[test]
fn named_optional_matrix_output_lowers_absence_from_its_real_header_kind() {
    // This is the exact named-output absence form that was rejected before
    // body definitions received the header's expected kind.
    evaluate_twice(
        "absent() => (result<[u64]?>) := result := _.\nabsent()\n",
        |value| {
            assert_eq!(value.canonical_data_draft().unwrap(), none());
            let owner = value.schemas().unwrap();
            let SchemaBody::Option(payload) = owner.get(value.schema()).unwrap().body() else {
                panic!("Option result")
            };
            let SchemaBody::Matrix { dimensions, .. } = payload.as_ref() else {
                panic!("Matrix payload kind")
            };
            assert_eq!(
                dimensions
                    .iter()
                    .map(|dimension| value.shape().resolve_dimension(dimension).unwrap())
                    .collect::<Vec<_>>(),
                [0, 0]
            );
        },
    );
}

#[test]
fn nested_optional_literals_receive_their_declared_component_kinds() {
    // Tuple members use ordinary kinds; aliases retain each optional component
    // without requiring an unsupported inline optional-member grammar.
    let source = "<maybe-number> := <u64?>\n<maybe-row> := <[u64]?>\nnested() => (result<(maybe-number,(maybe-row,bool))>) := result := (_,(_,true)).\nnested()\n";
    evaluate_twice(source, |value| {
        assert_eq!(
            value.canonical_data_draft().unwrap(),
            ValueDataDraft::Tuple(
                vec![
                    none(),
                    ValueDataDraft::Tuple(
                        vec![none(), ValueDataDraft::Bool(true)].into_boxed_slice()
                    ),
                ]
                .into_boxed_slice()
            )
        );
        assert_eq!(value.shape().parameter_values(), [0, 0]);
    });
    // A type alias keeps the two Option wrappers distinct. Absence belongs to
    // the complete declared output, rather than an invented present payload.
    evaluate_twice(
        "<maybe> := <u64?>\nnested() => (result<maybe?>) := result := _.\nnested()\n",
        |value| {
            let owner = value.schemas().unwrap();
            assert!(
                matches!(owner.get(value.schema()).unwrap().body(), SchemaBody::Option(inner) if matches!(inner.as_ref(), SchemaBody::Option(_)))
            );
            assert_eq!(value.canonical_data_draft().unwrap(), none());
        },
    );
}

#[test]
fn each_named_output_and_each_nested_function_owns_its_context() {
    evaluate_twice(
        "pair() => (left<u64?>,right<bool>) := left := _\nright := true.\npair()\n",
        |value| {
            assert_eq!(
                value.canonical_data_draft().unwrap(),
                ValueDataDraft::Tuple(vec![none(), ValueDataDraft::Bool(true)].into_boxed_slice())
            );
        },
    );
    evaluate_twice(
        "inner() => (result<string>) := result := \"inner\".\nouter() => (result<u64?>) := unused := inner()\nresult := _.\nouter()\n",
        |value| {
            assert_eq!(value.canonical_data_draft().unwrap(), none());
        },
    );
}

#[test]
fn explicit_body_annotation_precedes_context_and_final_header_conformance_remains_checked() {
    // The bare decimal must first use the explicit body f64 annotation, then
    // the ordinary checked final conversion to the declared u64 result.
    evaluate_twice(
        "narrow() => (result<u64>) := result<f64> := 1.5.\nnarrow()\n",
        |value| {
            assert_eq!(
                value.canonical_data_draft().unwrap(),
                ValueDataDraft::U64(1)
            );
        },
    );
    for source in [
        "wrong() => (result<bool>) := result<u64> := 1u64.\nwrong()\n",
        "wrong() => (result<u64>) := result := \"wrong\".\nwrong()\n",
        "wrong() => (result<bool>) := result := 1u64.\nwrong()\n",
    ] {
        let error = match CanonicalSourceFrontend
            .compile_document_with_catalog(&syntax(source), mech_stdlib::source_catalog())
        {
            Ok(_) => panic!("incompatible named output accepted: {source}"),
            Err(error) => error,
        };
        assert!(
            matches!(
                error.code,
                "source-semantics/incompatible-definition-kind"
                    | "source-semantics/incompatible-function-output"
            ),
            "{error:?}"
        );
    }
}

#[test]
fn unrelated_local_and_nested_helper_definitions_do_not_inherit_an_output_kind() {
    for source in [
        "absent() => (result<u64?>) := scratch := _\nresult := _.\nabsent()\n",
        // The helper's local `result` is not its declared output. It cannot
        // inherit outer's optional result context while outer is being lowered.
        "helper() => (other<u64>) := result := _\nother := 1u64.\nouter() => (result<u64?>) := result := helper().\nouter()\n",
    ] {
        let error = match CanonicalSourceFrontend
            .compile_document_with_catalog(&syntax(source), mech_stdlib::source_catalog())
        {
            Ok(_) => panic!("uncontextualized absence accepted: {source}"),
            Err(error) => error,
        };
        assert_eq!(error.code, "source-semantics/unresolved-empty-expression");
    }
}
