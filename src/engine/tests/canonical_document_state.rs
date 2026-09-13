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
    compiled_turns(compiled(source), source, expected, kind);
}

fn compiled_turns(
    compiled: CanonicalSourceProgram,
    source: &str,
    expected: &[f64],
    kind: mech_engine::SourceDocumentOutputKind,
) {
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
    let bytecode = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    let artifact = mech_engine::decode_program_artifact_bytecode_v1(&bytecode).unwrap();
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    let mut instance = activate(
        ReactiveInstanceId::new(0x570, 0),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .unwrap_or_else(|error| panic!("{source:?}: document activation: {error:?}"));
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
fn indexed_document_updates_use_canonical_assignment_and_preserve_state_order() {
    for (source, expected) in [
        (
            "~numbers := [1, 2, 3, 4, 5]\nnumbers[2] = 10\nnumbers[2]\n",
            [10.0, 10.0],
        ),
        (
            "~numbers := [1, 2, 3]\nnumbers[2] += 1\nnumbers[2]\n",
            [3.0, 4.0],
        ),
        (
            "~numbers := [1, 2, 3]\nnumbers[2] = 10\nbefore := numbers[2]\nnumbers[2] += 2\nbefore + numbers[2]\n",
            [22.0, 22.0],
        ),
        (
            "~numbers := [1, 2, 3]\nnumbers[1..=2] = 10\nnumbers[1] + numbers[2] + numbers[3]\n",
            [23.0, 23.0],
        ),
        (
            "~numbers := [1, 2; 3, 4]\nnumbers[:] = [5, 6; 7, 8]\nnumbers[2,2]\n",
            [8.0, 8.0],
        ),
        (
            "~numbers := [1, 2; 3, 4]\nnumbers[:] = 9\nnumbers[1,1] + numbers[2,2]\n",
            [18.0, 18.0],
        ),
        (
            "~numbers := [1, 2; 3, 4]\nnumbers[2,1] = 10\nnumbers[2,1]\n",
            [10.0, 10.0],
        ),
        (
            "~record := {value: 1}\nrecord.value += 1\nrecord.value\n",
            [2.0, 3.0],
        ),
        (
            "~record := {values: [1, 2]}\nrecord.values[2] += 1\nrecord.values[2]\n",
            [3.0, 4.0],
        ),
        (
            "~numbers := [1, 2]\nnumbers[1] += 1\nnumbers[2] = 2\nnumbers[1] + numbers[2]\n",
            [4.0, 5.0],
        ),
        ("~pair := (1, 2)\npair.2 += 1\npair.2\n", [3.0, 4.0]),
        (
            "~record := {a: 1, b: 2}\nrecord.b,a = (3, 4)\nrecord.a + record.b\n",
            [7.0, 7.0],
        ),
        (
            "~numbers := [1, 2; 3, 4]\nnumbers[2,:] = 10\nnumbers[2,1] + numbers[2,2]\n",
            [20.0, 20.0],
        ),
        (
            "~numbers := [1, 2; 3, 4]\nnumbers[:,2] += 1\nnumbers[1,2] + numbers[2,2]\n",
            [8.0, 10.0],
        ),
    ] {
        turns(source, &expected);
    }
}

#[test]
fn document_bindings_preserve_empty_errors_and_contextual_optional_state() {
    for source in [
        "x := _\nx\n",
        "~x := _\nx\n",
        "~x := 1\nx = _\nx\n",
        "_\n1\n",
        "```mech\n_\n1\n```\n",
    ] {
        let error = CanonicalSourceFrontend
            .compile_document(&document(source))
            .err()
            .expect("an unresolved binding must not become an executable value");
        assert_eq!(error.code, "source-semantics/unresolved-empty-expression");
        assert_eq!(error.anchor.document, DocumentId(0x570));
        assert_eq!(error.anchor.revision, Revision(7));
        assert_eq!(
            &source[error.anchor.range.start.0 as usize..error.anchor.range.end.0 as usize],
            "_",
        );
    }
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for source in ["~x<u8?> := _\nx\n", "~x<u8?> := 1u8\nx = _\nx\n"] {
        let program = compiled(source);
        assert!(program.program().inputs.is_empty());
        let artifact = program.compile_artifact().unwrap();
        let encoded = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(&encoded).unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(0x578, 0),
            &decoded,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        for _ in 0..2 {
            instance.turn(&[]).unwrap();
            assert!(
                matches!(
                    instance.copied_output(0).unwrap().data(),
                    ValueData::Option(None)
                ),
                "{source}"
            );
        }
    }
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
fn indexed_candidates_abort_without_mutating_published_state() {
    for failing in [false, true] {
        let source = if failing {
            "~numbers := [1, 2]\nnumbers[2] += 1\nnumbers[0] = 9\nnumbers[2]\n"
        } else {
            "~numbers := [1, 2]\nnumbers[2] += 1\nnumbers[2]\n"
        };
        let compiled = compiled(source);
        let bytes =
            mech_engine::encode_program_artifact_bytecode_v1(&compiled.compile_artifact().unwrap())
                .unwrap();
        let artifact = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
        let mut catalog = FunctionCatalogBuilder::new();
        mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
        let catalog = catalog.build().unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(0x572, 0),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        let before = instance.published_state_hash();
        if failing {
            assert!(instance.turn(&[]).is_err());
            assert_eq!(instance.published_state_hash(), before);
            assert!(instance.turn_without_summary(&[]).is_err());
            assert_eq!(instance.published_state_hash(), before);
        } else {
            {
                let candidate = instance.prepare_turn(&[]).unwrap();
                let output = candidate.copied_output(0).unwrap();
                let ValueData::F64(value) = output.data() else {
                    panic!("scalar candidate")
                };
                assert_eq!(value.to_f64(), 3.0);
            }
            assert_eq!(instance.published_state_hash(), before);
            instance.turn_without_summary(&[]).unwrap();
            let output = instance.copied_output(0).unwrap();
            let ValueData::F64(value) = output.data() else {
                panic!("scalar output")
            };
            assert_eq!(value.to_f64(), 3.0);
        }
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

#[test]
fn comprehension_reads_the_source_order_state_candidate_after_writer_reordering() {
    let compiled = compiled("~answer := 0\nanswer += 1\n[answer + x | x <- [1 2]]\n");
    let bytes =
        mech_engine::encode_program_artifact_bytecode_v1(&compiled.compile_artifact().unwrap())
            .unwrap();
    let artifact = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let mut instance = activate(
        ReactiveInstanceId::new(0x571, 0),
        &artifact,
        &catalog.build().unwrap(),
        &ActivationFacts::default(),
    )
    .unwrap();
    let output = compiled
        .document_outputs()
        .iter()
        .find(|binding| binding.kind == mech_engine::SourceDocumentOutputKind::Program)
        .unwrap()
        .output as usize;
    for expected in [[2.0, 3.0], [3.0, 4.0]] {
        instance.turn(&[]).unwrap();
        assert_eq!(
            instance
                .copied_output(output)
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            mech_core::ValueDataDraft::Matrix(
                expected
                    .into_iter()
                    .map(|value| mech_core::ValueDataDraft::F64(
                        mech_core::snapshot::F64Bits::from_f64(value)
                    ))
                    .collect()
            )
        );
    }
}

#[test]
fn named_document_scopes_execute_separately_and_share_repeated_fences() {
    let source = "~counter := 100\ncounter += 10\ncounter\n\n```mech:left\n~counter := 0\ncounter += 1\ncounter\n```\n\n```mech:right\n~counter := 10\ncounter += 5\ncounter\n```\n\n```mech:left\ncounter += 2\ncounter\n```\n";
    turns(source, &[110.0, 120.0]);
    for (name, expected, fences) in [("left", [3.0, 6.0], 2), ("right", [15.0, 20.0], 1)] {
        let program = CanonicalSourceFrontend
            .compile_named_document_scope(&document(source), name)
            .unwrap();
        assert_eq!(program.document_outputs().len(), fences + 1);
        assert_eq!(program.program().states.len(), 1);
        compiled_turns(
            program,
            source,
            &expected,
            mech_engine::SourceDocumentOutputKind::Program,
        );
    }
}

#[test]
fn absent_named_scopes_never_fall_back_to_the_root_program() {
    let source = "answer := 42\n\n```mech:disabled\nanswer := 99\n```\n";
    for name in ["missing", "disabled", "hidden", ""] {
        let error = CanonicalSourceFrontend
            .compile_named_document_scope(&document(source), name)
            .err()
            .expect("missing scope has no executable units");
        assert_eq!(error.code, "source-semantics/empty-document");
    }
}

#[test]
fn derived_updates_compare_final_output_to_the_value_before_seeding() {
    use mech_core::{
        BoundResidentKernel, ChangeDetectionPolicy, OutputConstruction, ResidentKernelError,
        ResidentKernelInputs, ResidentValueMut,
    };
    use mech_engine::resident::ActivatedTurnStep;

    fn leave_seed_unchanged(
        _: &BoundResidentKernel,
        _: &dyn ResidentKernelInputs,
        _: ResidentValueMut<'_>,
    ) -> Result<bool, ResidentKernelError> {
        Ok(false)
    }

    for (exact_scalar, source) in [
        (
            false,
            "~tick := 0\ntick += 1\n~values := [0]\nvalues[1] = tick\nvalues[1] = 0\nvalues[1]\n",
        ),
        (
            false,
            "~tick := 0\ntick += 1\n~values := {value: 0}\nvalues.value = tick\nvalues.value = 0\nvalues.value\n",
        ),
        (
            true,
            "~values := [0]\nvalues[1] += 1\nvalues[1] += 1\nvalues[1]\n",
        ),
    ] {
        let artifact = compiled(source).compile_artifact().unwrap();
        let mut catalog = FunctionCatalogBuilder::new();
        mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
        let catalog = catalog.build().unwrap();
        let mut instances = (0..2)
            .map(|id| {
                activate(
                    ReactiveInstanceId::new(0x580, id),
                    &artifact,
                    &catalog,
                    &ActivationFacts::default(),
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        let mut summaries = Vec::new();
        for (forced, instance) in instances.iter_mut().enumerate() {
            instance.turn(&[]).unwrap();
            let update = instance
                .plan
                .steps
                .iter()
                .enumerate()
                .filter_map(|(index, step)| match step {
                    ActivatedTurnStep::Kernel(node)
                        if artifact.slots()[node.write.slot.get() as usize].role
                            == mech_engine::SlotRole::Derived
                            && matches!(
                                node.construction,
                                OutputConstruction::ReadModifyWrite { .. }
                            ) =>
                    {
                        Some(index)
                    }
                    _ => None,
                })
                .last()
                .unwrap();
            if exact_scalar {
                instance.plan.replace_kernel_for_test(
                    update,
                    BoundResidentKernel::new(leave_seed_unchanged, Box::new([])),
                );
            }
            instance.plan.set_change_detection_for_test(
                update,
                if forced == 1 {
                    ChangeDetectionPolicy::AlwaysChanged
                } else if exact_scalar {
                    ChangeDetectionPolicy::ExactScalar
                } else {
                    ChangeDetectionPolicy::KernelReported
                },
            );
            summaries.push(instance.turn(&[]).unwrap());
            let output = instance.copied_output(0).unwrap();
            let ValueData::F64(value) = output.data() else {
                panic!("scalar result")
            };
            assert_eq!(value.to_f64(), if exact_scalar { 3.0 } else { 0.0 });
        }
        if !exact_scalar {
            assert!(
                summaries[0].dirty_nodes < summaries[1].dirty_nodes,
                "an unchanged final update must not dirty its consumers"
            );
        }
    }
}

#[test]
fn derived_snapshot_updates_release_budgeted_prior_outputs_on_abort_and_drop() {
    use mech_engine::resident::{ResidentActivationOptions, activate_with_options};
    let artifact =
        compiled("~record := {values: [1, 2]}\nrecord.values[2] += 1\nrecord.values[2]\n")
            .compile_artifact()
            .unwrap();
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    let budget = mech_core::ManagedMemoryBudget::new(1 << 24);
    let mut instance = activate_with_options(
        ReactiveInstanceId::new(0x581, 0),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
        ResidentActivationOptions {
            memory_budget: Some(budget.clone()),
            ..Default::default()
        },
    )
    .unwrap();
    let before = instance.published_state_hash();
    for _ in 0..3 {
        drop(instance.prepare_turn(&[]).unwrap());
        assert_eq!(instance.published_state_hash(), before);
    }
    for expected in [3.0, 4.0, 5.0] {
        instance.turn_without_summary(&[]).unwrap();
        let output = instance.copied_output(0).unwrap();
        let ValueData::F64(value) = output.data() else {
            panic!("scalar result")
        };
        assert_eq!(value.to_f64(), expected);
    }
    drop(instance);
    assert_eq!(budget.used_bytes(), 0);
}
