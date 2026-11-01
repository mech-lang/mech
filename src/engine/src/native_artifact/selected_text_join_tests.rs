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

#[test]
fn sole_use_unit_range_join_proof_keeps_original_and_decoded_three_visits() {
    for (source, inclusive) in [
        ("text/join(lines[begin<u64>..stop<u64>], \"\")\n", false),
        ("text/join(lines[begin<u64>..=stop<u64>], \"|\")\n", true),
        (
            "join(lines<[string]>, first<u64>, last<u64>, sep<string>) => (out<string>) := out := text/join(lines[first..last], sep).\njoin(input<[string]>, begin<u64>, stop<u64>, \"|\")\n",
            false,
        ),
    ] {
        for plan in plans(source) {
            let regions = plan.selected_text_joins();
            assert_eq!(regions.len(), 1, "{source}");
            let region = &regions[0];
            let range = region.range().expect("anonymous sole-use unit range");
            assert_eq!(range.inclusive(), inclusive);
            assert_ne!(range.producer(), region.selection());
            assert_ne!(region.selection(), region.consumer());
            assert!(matches!(plan.blocks()[range.producer() as usize].action(),
                Action::LibraryCall { operands, output, template }
                    if operands.as_ref() == [range.start(), range.end()]
                        && *output == region.selector()
                        && (template.primitive() == Primitive::RangeInclusive) == inclusive));
            let selection = plan.blocks()[region.selection() as usize].action();
            assert!(matches!(selection, Action::LibraryCall { operands, .. }
                if operands.as_ref() == [region.source(), region.selector()]));
            assert!(matches!(plan.blocks()[region.consumer() as usize].action(),
                Action::LibraryCall { operands, .. }
                    if operands.as_ref() == [region.selected(), region.separator()]));
            plan.validate_current_visits().unwrap();
        }
    }
}

#[test]
fn unsupported_ranges_preserve_the_ordinary_selected_join_proof() {
    for source in [
        "text/join(lines[indices], \"\")\n",
        "selector := begin<u64>..stop<u64>\ntext/join(lines[selector], \"\")\nselector\n",
        "selector := begin<u64>..stop<u64>\ntext/join(lines[selector], \"\") + text/join(lines[selector], \"|\")\n",
        "text/join(lines[begin<u64>..2u64..stop<u64>], \"\")\n",
    ] {
        for plan in plans(source) {
            let regions = plan.selected_text_joins();
            assert!(
                !regions.is_empty(),
                "ordinary selected joins remain: {source}"
            );
            assert!(
                regions.iter().all(|region| region.range().is_none()),
                "{source}"
            );
        }
    }
}

#[test]
fn range_join_proof_rejects_barriers_other_reads_and_wrong_schema_or_scope() {
    for plan in plans("text/join(lines[begin<u64>..stop<u64>], \"\")\n") {
        let regions = plan.selected_text_joins();
        let region = &regions[0];
        let range = region.range().unwrap();
        let selection = region.selection() as usize;
        let Action::LibraryCall { template, .. } = plan.blocks()[selection].action() else {
            unreachable!()
        };
        let uses = reads(&plan, &semantic_actions(&plan).unwrap()).unwrap();
        let candidate = |changed: &Plan, reads: &[Vec<Read>]| {
            selected_range(changed, selection, region.selector(), template, reads)
        };
        for barrier in [range.producer(), region.selection()] {
            let mut changed = plan.clone();
            changed.activation_boundary = Some(barrier);
            assert!(candidate(&changed, &uses).is_none());
            // A real LibraryCall is not a valid activation checkpoint. The
            // complete plan must fail closed before deriving either proof;
            // valid unsupported ranges retain their ordinary proof above.
            assert!(changed.validate_current_visits().is_err());
            assert!(changed.selected_text_joins().is_empty());
        }
        for read in [
            Read::Other,
            Read::Terminator(range.producer()),
            Read::Action(region.consumer()),
        ] {
            let mut extra = uses.clone();
            extra[region.selector().get() as usize].push(read);
            assert!(candidate(&plan, &extra).is_none());
        }
        let mut wrong_schema = plan.clone();
        wrong_schema.values[range.start().get() as usize] =
            plan.values()[region.source().get() as usize].clone();
        assert!(candidate(&wrong_schema, &uses).is_none());
        let mut wrong_selector = plan.clone();
        wrong_selector.values[region.selector().get() as usize] =
            plan.values()[region.source().get() as usize].clone();
        assert!(candidate(&wrong_selector, &uses).is_none());
        let mut wrong_scope = plan.clone();
        wrong_scope.blocks[selection].iteration = Some(u32::MAX);
        assert!(candidate(&wrong_scope, &uses).is_none());
        let mut branch = plan.clone();
        branch.blocks[range.producer() as usize].terminator =
            Terminator::Scalar(Term::LiteralBranch {
                left: range.start(),
                right: range.end(),
                yes: region.selection(),
                no: region.selection(),
            });
        assert!(candidate(&branch, &uses).is_none());
        let mut intervening = plan.clone();
        let mut blocks = intervening.blocks.to_vec();
        let inserted = blocks.len() as u32;
        let mut block = blocks[region.consumer() as usize].clone();
        block.terminator = Terminator::Scalar(Term::Jump(region.selection()));
        blocks.push(block);
        blocks[range.producer() as usize].terminator = Terminator::Scalar(Term::Jump(inserted));
        intervening.blocks = blocks.into_boxed_slice();
        assert!(
            candidate(&intervening, &uses).is_none(),
            "fallible join may not intervene"
        );
        intervening.blocks[inserted as usize].action = Action::Scalar(Scalar::None);
        assert!(
            candidate(&intervening, &uses).is_some(),
            "inert same-scope jumps are allowed"
        );
    }
}
