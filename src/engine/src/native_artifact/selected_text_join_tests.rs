use super::*;
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
use std::sync::Arc;

fn plans(source: &str) -> [Plan; 2] {
    let source = format!("+> text\nlines := input<[string]>\nindices := select<[u64]>\n{source}");
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x53454c4a), Revision(1), source.as_str()).unwrap(),
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
fn selection_join_proof_recognizes_original_and_decoded_root_and_lexical_calls() {
    for source in [
        "text/join(lines[indices], separator<string>)\n",
        "first := begin<u64>\nend := stop<u64>\ntext/join(lines[first..end], \"\")\n",
        "join(lines<[string]>, indices<[u64]>, sep<string>) => (out<string>) := out := text/join(lines[indices], sep).\njoin(input<[string]>, select<[u64]>, \"|\")\n",
    ] {
        for plan in plans(source) {
            let regions = plan.selected_text_joins();
            assert_eq!(regions.len(), 1, "{source}");
            let region = &regions[0];
            assert_ne!(region.source(), region.selected());
            assert_ne!(region.selected(), region.output());
            assert!(
                plan.outputs()
                    .iter()
                    .all(|output| output.value() != region.selected())
            );
            assert!(
                matches!(plan.blocks()[region.selection() as usize].action(),
                Action::LibraryCall { operands, output, .. }
                if operands.as_ref() == [region.source(), region.selector()]
                    && *output == region.selected())
            );
            assert!(matches!(plan.blocks()[region.consumer() as usize].action(),
                Action::LibraryCall { operands, output, .. }
                if operands.as_ref() == [region.selected(), region.separator()]
                    && *output == region.output()));
        }
    }
}

#[test]
fn selection_join_proof_declines_escapes_multiple_uses_and_late_separator() {
    for source in [
        "selected := lines[indices]\ntext/join(selected, \"\")\nselected\n",
        "selected := lines[indices]\ntext/join(selected, \"\") + text/join(selected, \"-\")\n",
        "part := text<string>\nselected := lines[indices]\nseparator := part[1u64]\ntext/join(selected, separator)\n",
        "text/join(lines, \"\")\n",
        "floats := fractional<[f64]>\ntext/join(lines[floats], \"\")\n",
    ] {
        for plan in plans(source) {
            assert!(plan.selected_text_joins().is_empty(), "{source}");
        }
    }
}

#[test]
fn selection_join_proof_rederives_lifecycle_action_and_semantic_uses() {
    for plan in plans("text/join(lines[indices], \"\")\n") {
        let regions = plan.selected_text_joins();
        let region = &regions[0];
        let mut barrier = plan.clone();
        barrier.activation_boundary = Some(region.consumer());
        assert!(barrier.selected_text_joins().is_empty());
        let mut omitted = plan.clone();
        omitted.blocks[region.selection() as usize].action = Action::Scalar(Scalar::None);
        assert!(omitted.selected_text_joins().is_empty());
        let mut wrong_read = plan.clone();
        let Action::LibraryCall { operands, .. } =
            &mut wrong_read.blocks[region.consumer() as usize].action
        else {
            unreachable!()
        };
        operands[0] = region.source();
        assert!(wrong_read.selected_text_joins().is_empty());
        let mut observable = plan.clone();
        let output = observable.outputs.last_mut().unwrap();
        output.value = region.selected();
        output.ty = observable.values[region.selected().get() as usize].clone();
        assert!(observable.selected_text_joins().is_empty());
    }
}
