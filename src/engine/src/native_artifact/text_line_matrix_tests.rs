use super::*;
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
use std::sync::Arc;

fn plans(body: &str) -> [Plan; 2] {
    let source = format!("+> text\n+> sequence\nlines := text/lines(input<string>)\n{body}");
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x4c494e45), Revision(1), source.as_str()).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    let mut builder = mech_core::FunctionCatalogBuilder::new();
    crate::install_intrinsic_runtime(&mut builder).unwrap();
    crate::install_intrinsic_compiler_runtime(&mut builder).unwrap();
    crate::install_intrinsic_resident(&mut builder).unwrap();
    crate::install_intrinsic_source(&mut builder).unwrap();
    let catalog = Arc::new(builder.build().unwrap());
    let original = crate::CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            catalog.clone(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap();
    let decoded = crate::decode_program_artifact_bytecode_v1(
        &crate::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    [original, decoded]
        .map(|artifact| super::super::lower_native_program_artifact(&artifact, &catalog).unwrap())
}

#[test]
fn original_and_decoded_private_lengths_and_selected_joins_are_proved() {
    for source in [
        "sequence/length(lines)\n",
        "text/join(lines[indices<[u64]>], \"|\")\n",
        "n := sequence/length(lines)\ntext/join(lines[1u64..=n], \"\")\n",
    ] {
        for plan in plans(source) {
            let regions = plan.text_line_matrix_regions();
            assert_eq!(regions.len(), 1, "{source}");
            let region = &regions[0];
            assert_ne!(region.source(), region.matrix());
            assert!(matches!(plan.blocks()[region.producer() as usize].action(),
                Action::LibraryCall { template, operands, output }
                if template.primitive() == Primitive::TextLines
                    && operands.as_ref() == [region.source()] && *output == region.matrix()));
            assert!(!region.lengths().is_empty() || !region.selections().is_empty());
        }
    }
}

#[test]
fn unproved_consumers_cannot_hide_an_escaping_or_ordinary_matrix() {
    for source in [
        "lines\n",
        "text/join(lines, \"\")\n",
        "selected := lines[indices<[u64]>]\nselected\n",
        "sequence/length(lines)\nlines\n",
        "[line | line <- lines]\n",
        "lines[1u64]\n",
    ] {
        for plan in plans(source) {
            assert!(plan.text_line_matrix_regions().is_empty(), "{source}");
        }
    }
}

#[test]
fn root_native_fsm_reads_keep_their_genuine_borrowed_string_proof() {
    let body = "#Scan(lines<[string]>, n<u64>) => <u64>\n  | :Step(i<u64>, found<u64>)\n  | :Done(found<u64>).\n#Scan(lines, n) -> :Step(1u64, 0u64)\n  :Step(i, found)\n    | i > n -> :Done(found)\n    | lines[i] == \"stop\" -> :Step(i + 1u64, found + 1u64)\n    | * -> :Step(i + 1u64, found)\n  :Done(found) => found.\n#Scan(lines, sequence/length(lines))\n";
    for plan in plans(body) {
        let regions = plan.text_line_matrix_regions();
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].borrowed_regions(), [0]);
        assert_eq!(regions[0].lengths().len(), 1);
        let mut escaping = plan.clone();
        let source = body.replace("lines[i] == \"stop\"", "lines[i] + \"!\" == \"stop!\"");
        for ordinary in plans(&source) {
            assert!(ordinary.text_line_matrix_regions().is_empty());
        }
        let region = &regions[0];
        let output = escaping.outputs.last_mut().unwrap();
        output.value = region.matrix();
        output.ty = escaping.values[region.matrix().get() as usize].clone();
        assert!(escaping.text_line_matrix_regions().is_empty());
    }
}

#[test]
fn producer_and_consumer_mutations_cannot_reuse_a_stale_region() {
    for plan in plans("text/join(lines[indices<[u64]>], \"\")\n") {
        let region = plan.text_line_matrix_regions().pop().unwrap();
        let mut omitted = plan.clone();
        omitted.blocks[region.producer() as usize].action =
            Action::Scalar(super::super::NativeMatchAction::None);
        assert!(omitted.text_line_matrix_regions().is_empty());
        let mut barrier = plan.clone();
        barrier.activation_boundary = Some(region.producer());
        assert!(barrier.text_line_matrix_regions().is_empty());
        let mut consumer = plan.clone();
        let Action::LibraryCall { operands, .. } =
            &mut consumer.blocks[region.selections()[0] as usize].action
        else {
            unreachable!()
        };
        operands[0] = region.source();
        assert!(consumer.text_line_matrix_regions().is_empty());
    }
}
