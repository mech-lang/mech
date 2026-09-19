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
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytecode).unwrap();
    for artifact in [artifact, decoded] {
        let mut catalog = FunctionCatalogBuilder::new();
        mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
        let catalog = catalog.build().unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(0x570, 0),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap_or_else(|error| {
            panic!(
                "{source:?}: document activation: {error:?}; nodes: {:?}",
                compiled.source_map().nodes
            )
        });
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
            "~numbers := [1, 2; 3, 4]\nnumbers[:,:] = [5, 6; 7, 8]\nnumbers[2,2]\n",
            [8.0, 8.0],
        ),
        (
            "~numbers := [1, 2; 3, 4]\nnumbers[:,:] = 9\nnumbers[1,1] + numbers[2,2]\n",
            [18.0, 18.0],
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
fn whole_value_document_assignment_does_not_emit_a_discarded_selection() {
    for selection in [":", ":,:"] {
        let source =
            format!("~matrix := [1, 2; 3, 4]\nmatrix[{selection}] = [5, 6; 7, 8]\nmatrix\n");
        let compiled = compiled(&source);
        let operations = compiled
            .program()
            .nodes
            .iter()
            .map(|node| {
                node.operation()
                    .expect("ordinary source operation")
                    .canonical_name()
            })
            .collect::<Vec<_>>();
        assert!(
            operations
                .iter()
                .any(|operation| operation == "core/assign/whole-value"),
            "{source}"
        );
        assert!(
            !operations
                .iter()
                .any(|operation| operation == "access/range"),
            "{source}: {operations:?}"
        );
        compiled.compile_artifact().unwrap();
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
fn sequential_derived_updates_share_only_the_largest_rmw_backup_region() {
    use mech_core::{AllocationRole, MemoryLifetime, MemoryObjectOwner};

    let source = "~tick := 0\ntick += 1\n~small := [0, 0]\nsmall[1] = tick\nsmall[1] = 0\n~large := [0, 0, 0, 0]\nlarge[1] = tick\nlarge[1] = 0\nsmall[1] + large[1]\n";
    let artifact = compiled(&source).compile_artifact().unwrap();
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let mut instance = activate(
        ReactiveInstanceId::new(0x582, 0),
        &artifact,
        &catalog.build().unwrap(),
        &ActivationFacts::default(),
    )
    .unwrap();
    let backups = instance
        .plan
        .memory_plan
        .allocations
        .iter()
        .filter(|allocation| {
            allocation.role == AllocationRole::Scratch
                && matches!(allocation.owner, MemoryObjectOwner::TransactionStage { .. })
                && matches!(allocation.lifetime, MemoryLifetime::Turn { .. })
        })
        .collect::<Vec<_>>();
    assert!(backups.len() >= 4, "{:#?}", instance.plan.memory_plan);
    let group = backups[0].reuse_group.unwrap();
    let arena_id = backups[0].placement.arena;
    assert!(backups.iter().all(|backup| {
        backup.reuse_group == Some(group)
            && backup.placement.arena == arena_id
            && backup.placement.offset == 0
    }));
    let maximum = backups
        .iter()
        .map(|backup| backup.capacity_bytes)
        .max()
        .unwrap();
    let sum = backups
        .iter()
        .map(|backup| backup.capacity_bytes)
        .sum::<u64>();
    assert!(sum > maximum);
    let arena = instance
        .plan
        .memory_plan
        .arenas
        .iter()
        .find(|arena| arena.id == arena_id)
        .unwrap();
    assert_eq!(arena.capacity_bytes, maximum);
    instance.turn(&[]).unwrap();
    let output = instance.copied_output(0).unwrap();
    let ValueData::F64(value) = output.data() else {
        panic!("scalar result")
    };
    assert_eq!(value.to_f64(), 0.0);
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

#[test]
fn configured_fences_execute_updates_when_their_result_is_hidden() {
    for suffix in ["", ":worker"] {
        let source = format!(
            "```mech{suffix}{{output: false, color: red}}\n~counter := 0\ncounter += 1\ncounter\n```\n"
        );
        let document = document(&source);
        let compiled = if suffix.is_empty() {
            CanonicalSourceFrontend.compile_document(&document)
        } else {
            CanonicalSourceFrontend.compile_named_document_scope(&document, "worker")
        }
        .unwrap();
        assert_eq!(
            compiled.document_outputs().len(),
            1,
            "only the program result is published"
        );
        compiled_turns(
            compiled,
            &source,
            &[1.0, 2.0],
            mech_engine::SourceDocumentOutputKind::Program,
        );
    }
    let source = "```mech:hidden{output: true}\n~counter := 0\ncounter += 1\ncounter\n```\n";
    let compiled = compiled(source);
    assert_eq!(
        compiled.document_outputs().len(),
        1,
        "a hidden fence executes without a presentation output"
    );
    compiled_turns(
        compiled,
        source,
        &[1.0, 2.0],
        mech_engine::SourceDocumentOutputKind::Program,
    );
}

#[test]
fn colonless_named_fences_execute_and_share_state_with_colon_spelling() {
    for info in ["mechworker", "mecworker", "🤖worker", "mechmechmec🤖worker"] {
        let source = format!(
            "~counter := 100\ncounter\n\n```{info}\n~counter := 0\ncounter += 1\ncounter\n```\n\n```mech:worker\ncounter += 2\ncounter\n```\n"
        );
        turns(&source, &[100.0, 100.0]);
        let program = CanonicalSourceFrontend
            .compile_named_document_scope(&document(&source), "worker")
            .unwrap();
        compiled_turns(
            program,
            &source,
            &[3.0, 6.0],
            mech_engine::SourceDocumentOutputKind::Program,
        );
    }
}

#[test]
fn mika_bodies_execute_independently_of_parent_sibling_and_nested_state() {
    let source = "~counter := 100\ncounter += 10\ncounter\n\n~∘~⸢~counter := 0\ncounter += 1\ncounter\n\n╭◉╮⸢~counter := 20\ncounter += 2\ncounter\n⸥\n⸥\n\n~∘~⸢~counter := 40\ncounter += 4\ncounter\n⸥\n";
    let document = document(source);
    compiled_turns(
        CanonicalSourceFrontend.compile_document(&document).unwrap(),
        source,
        &[110.0, 120.0],
        mech_engine::SourceDocumentOutputKind::Program,
    );
    let scopes = document.mika_scopes();
    assert_eq!(scopes.len(), 3);
    for (scope, expected) in scopes.iter().zip([[1.0, 2.0], [22.0, 24.0], [44.0, 48.0]]) {
        let program = CanonicalSourceFrontend
            .compile_mika_section(&scope.section)
            .unwrap();
        compiled_turns(
            program,
            source,
            &expected,
            mech_engine::SourceDocumentOutputKind::Program,
        );
    }
}

#[test]
fn mika_named_fences_share_only_their_local_owner_and_keep_output_options() {
    let source = "```mech:worker\n~counter := 100\ncounter += 10\ncounter\n```\n\n~∘~⸢```mechworker{output: false}\n~counter := 0\ncounter += 1\ncounter\n```\n\n```mech:worker\ncounter += 2\ncounter\n```\n⸥\n";
    let document = document(source);
    let root = CanonicalSourceFrontend
        .compile_named_document_scope(&document, "worker")
        .unwrap();
    compiled_turns(
        root,
        source,
        &[110.0, 120.0],
        mech_engine::SourceDocumentOutputKind::Program,
    );
    let scopes = document.mika_scopes();
    assert_eq!(scopes.len(), 1);
    let local = CanonicalSourceFrontend
        .compile_named_mika_scope(&scopes[0].section, "worker")
        .unwrap();
    assert_eq!(local.document_outputs().len(), 2);
    compiled_turns(
        local,
        source,
        &[3.0, 6.0],
        mech_engine::SourceDocumentOutputKind::Program,
    );
}

#[test]
fn finalized_streams_execute_configured_root_and_normalized_named_scopes() {
    use mech_syntax::document::{DocumentStream, StreamProgress};
    for scope in ["", "worker"] {
        let source = format!(
            "```mech{scope}{{output: false, color: red}}\n~counter := 0\ncounter += 1\ncounter\n```\n"
        );
        let mut stream = DocumentStream::new(DocumentId(826), ParseConfig::default());
        for ch in source.chars() {
            let mut progress = stream.append(&ch.to_string(), 17).unwrap().progress;
            for _ in 0..100_000 {
                if progress != StreamProgress::NeedsProcessing {
                    break;
                }
                progress = stream.advance(17).progress;
            }
            assert_eq!(progress, StreamProgress::NeedInput);
        }
        let mut progress = stream.finish(17).progress;
        for _ in 0..100_000 {
            if progress != StreamProgress::NeedsProcessing {
                break;
            }
            progress = stream.advance(17).progress;
        }
        assert_eq!(progress, StreamProgress::Finished);
        let snapshot = stream.materialize().unwrap();
        assert!(snapshot.is_strictly_clean());
        let document = DocumentSyntax::cast(snapshot.syntax()).unwrap();
        let program = if scope.is_empty() {
            CanonicalSourceFrontend.compile_document(&document)
        } else {
            CanonicalSourceFrontend.compile_named_document_scope(&document, scope)
        }
        .unwrap();
        assert_eq!(program.document_outputs().len(), 1);
        compiled_turns(
            program,
            &source,
            &[1.0, 2.0],
            mech_engine::SourceDocumentOutputKind::Program,
        );
    }
}

fn streamed_document(source: &str) -> DocumentSyntax {
    use mech_syntax::document::{DocumentStream, StreamProgress};
    let mut stream = DocumentStream::new(DocumentId(826), ParseConfig::default());
    for ch in source.chars() {
        let mut progress = stream.append(&ch.to_string(), 19).unwrap().progress;
        while progress == StreamProgress::NeedsProcessing {
            progress = stream.advance(19).progress;
        }
        assert_eq!(progress, StreamProgress::NeedInput);
    }
    let mut progress = stream.finish(19).progress;
    while progress == StreamProgress::NeedsProcessing {
        progress = stream.advance(19).progress;
    }
    assert_eq!(progress, StreamProgress::Finished);
    let snapshot = stream.materialize().unwrap();
    assert!(snapshot.is_strictly_clean());
    DocumentSyntax::cast(snapshot.syntax()).unwrap()
}

#[test]
fn finalized_streams_mika_bodies_execute_independently_of_parent_sibling_and_nested_state() {
    let source = "~counter := 100\ncounter += 10\ncounter\n\n~∘~⸢~counter := 0\ncounter += 1\ncounter\n\n╭◉╮⸢~counter := 20\ncounter += 2\ncounter\n⸥\n⸥\n\n~∘~⸢~counter := 40\ncounter += 4\ncounter\n⸥\n";
    let document = streamed_document(source);
    compiled_turns(
        CanonicalSourceFrontend.compile_document(&document).unwrap(),
        source,
        &[110.0, 120.0],
        mech_engine::SourceDocumentOutputKind::Program,
    );
    let scopes = document.mika_scopes();
    assert_eq!(scopes.len(), 3);
    for (scope, expected) in scopes.iter().zip([[1.0, 2.0], [22.0, 24.0], [44.0, 48.0]]) {
        let program = CanonicalSourceFrontend
            .compile_mika_section(&scope.section)
            .unwrap();
        compiled_turns(
            program,
            source,
            &expected,
            mech_engine::SourceDocumentOutputKind::Program,
        );
    }
}

#[test]
fn finalized_streams_mika_named_fences_share_only_their_local_owner_and_keep_output_options() {
    let source = "```mech:worker\n~counter := 100\ncounter += 10\ncounter\n```\n\n~∘~⸢```mechworker{output: false}\n~counter := 0\ncounter += 1\ncounter\n```\n\n```mech:worker\ncounter += 2\ncounter\n```\n⸥\n";
    let document = streamed_document(source);
    let root = CanonicalSourceFrontend
        .compile_named_document_scope(&document, "worker")
        .unwrap();
    compiled_turns(
        root,
        source,
        &[110.0, 120.0],
        mech_engine::SourceDocumentOutputKind::Program,
    );
    let scopes = document.mika_scopes();
    assert_eq!(scopes.len(), 1);
    let local = CanonicalSourceFrontend
        .compile_named_mika_scope(&scopes[0].section, "worker")
        .unwrap();
    assert_eq!(local.document_outputs().len(), 2);
    compiled_turns(
        local,
        source,
        &[3.0, 6.0],
        mech_engine::SourceDocumentOutputKind::Program,
    );
}

#[test]
fn repeated_matrix_compound_selectors_accumulate_in_occurrence_order() {
    for (source, expected) in [
        (
            "~a := [10 20; 30 40]\na[[1 1],:] += [1 2; 3 4]\na[1,1]\n",
            vec![14.0, 18.0],
        ),
        (
            "~a := [10 20; 30 40]\na[:,[1 1]] += [1 2; 3 4]\na[2,1]\n",
            vec![37.0, 44.0],
        ),
        (
            "~a := [10 20; 30 40]\na[[1 1],[2 2]] += [1 2; 3 4]\na[1,2]\n",
            vec![30.0, 40.0],
        ),
        (
            "~a := [10 20; 30 40]\na[[1 1]] += [1; 3]\na[1,1]\n",
            vec![14.0, 18.0],
        ),
        (
            "~a := [10 20; 30 40]\na[[1 1],:] += 2\na[1,2]\n",
            vec![24.0, 28.0],
        ),
        (
            "~a := [60 60; 30 40]\na[[1 1],:] /= [2 2; 3 3]\na[1,1]\n",
            vec![10.0],
        ),
        (
            "~a := [10 20; 30 40]\na[[1 1],:] *= [2 2; 3 3]\na[1,1]\n",
            vec![60.0, 360.0],
        ),
        (
            "~a := [10 20; 30 40]\na[[1 1],:] -= [1 2; 3 4]\na[1,1]\n",
            vec![6.0, 2.0],
        ),
        (
            "~a := [2 2; 3 4]\na[[1 1],:] ^= [2 2; 3 3]\na[1,1]\n",
            vec![64.0],
        ),
    ] {
        turns(source, &expected);
    }
}

#[test]
fn repeated_compound_selections_preserve_exact_numeric_kinds() {
    for kind in ["i32", "u32", "f32"] {
        for selection in ["[1 1],:", ":,[1 1]", "[1 1],[2 2]", "[1 1]"] {
            let source = format!(
                "~a := [10<{kind}> 20<{kind}>; 30<{kind}> 40<{kind}>]\na[{selection}] += 2<{kind}>\ntotal := a[1,1] + a[1,2] + a[2,1] + a[2,2]\nanswer := total<f64>\nanswer\n"
            );
            let increment = if selection == "[1 1]" { 4.0 } else { 8.0 };
            turns(&source, &[100.0 + increment, 100.0 + 2.0 * increment]);
        }
    }
}

#[test]
fn repeated_compound_overflow_rejects_without_publishing_and_valid_retry_succeeds() {
    use mech_core::ResidentKernelError;
    use mech_core::snapshot::{SnapshotValidationContext, ValueDataDraft, ValueDraft};
    use mech_engine::resident::{CapturedValueInput, ResidentExecutionError};
    let source = "changes := signal<[i8]:2,2>\n~a := [120<i8> 0<i8>; 0<i8> 0<i8>]\na[[1 1],:] += changes\nselected := a[1,1]\nanswer := selected<f64>\nanswer\n";
    let artifact = compiled(source).compile_artifact().unwrap();
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    for artifact in [
        artifact,
        mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap(),
    ] {
        let mut instance = activate(
            ReactiveInstanceId::new(0x589, 0),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        let before = instance.published_state_hash();
        for (deltas, expected) in [([1i8, 0, 20, 0], None), ([1, 0, 2, 0], Some(123.0))] {
            let value = ValueDraft {
                schema: artifact.inputs()[0].schema,
                shape_values: Box::new([]),
                data: ValueDataDraft::Matrix(deltas.into_iter().map(ValueDataDraft::I8).collect()),
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let input = CapturedValueInput {
                slot: instance.plan.inputs[0].slot,
                value: &value,
            };
            let result = instance
                .prepare_turn_values(&[input])
                .and_then(|prepared| prepared.publish());
            if let Some(expected) = expected {
                result.unwrap();
                let value = instance.copied_output(0).unwrap();
                assert!(matches!(value.data(), ValueData::F64(bits) if bits.to_f64() == expected));
            } else {
                assert!(
                    matches!(
                        result,
                        Err(ResidentExecutionError::Kernel {
                            error: ResidentKernelError::Arithmetic,
                            ..
                        })
                    ),
                    "{result:?}"
                );
                assert_eq!(instance.published_state_hash(), before);
                assert_eq!(instance.published_epoch(), mech_core::InstanceEpoch::ZERO);
            }
        }
    }
}

#[test]
fn selected_compound_assignment_preserves_arithmetic_before_destination_conversion() {
    for (target, source) in [("a", "~a := -1<i32>"), ("a[[1]]", "~a := [-1<i32>]")] {
        let result = if target == "a" { "a" } else { "a[1]" };
        turns(
            &format!(
                "{source}\n{target} += 0.5\nselected := {result}\nanswer := selected<f64>\nanswer\n"
            ),
            &[0.0],
        );
    }
}

#[test]
fn ordered_retained_roots_link_live_exports_and_preserve_caller_output_order() {
    use mech_engine::{CanonicalOrderedDocument, CanonicalOrderedImport};
    use std::collections::{BTreeMap, BTreeSet};
    let root = |identity, source: &str| {
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x590 + identity as u64), Revision(1), source).unwrap(),
            ParseConfig::default(),
        );
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        CanonicalOrderedDocument {
            identity,
            document: DocumentSyntax::cast(parsed.syntax()).unwrap(),
            input_schemas: BTreeMap::new(),
            resource_writes: BTreeMap::new(),
            imports: BTreeMap::new(),
            resolved_modules: BTreeSet::new(),
        }
    };
    let dependency = root(1, "~counter := 0\ncounter += 1\n<+ counter\ncounter\n");
    let mut main = root(0, "answer := dep/counter + 1\nanswer\n");
    main.imports.insert(
        "dep/counter".to_owned(),
        CanonicalOrderedImport::RootExport {
            root: 1,
            name: "counter".to_owned(),
        },
    );
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let program = CanonicalSourceFrontend
        .compile_ordered_documents_with_catalog(
            &[dependency, main],
            std::sync::Arc::new(catalog.build().unwrap()),
        )
        .unwrap();
    assert_eq!(
        program
            .program()
            .outputs
            .iter()
            .map(|output| output.name.as_str())
            .collect::<Vec<_>>(),
        ["answer", "counter"]
    );
    assert_eq!(program.program().states.len(), 1);
    assert!(
        program
            .source_map()
            .nodes
            .iter()
            .any(|anchor| anchor.anchor.document == DocumentId(0x590))
    );
    assert!(
        program
            .source_map()
            .nodes
            .iter()
            .any(|anchor| anchor.anchor.document == DocumentId(0x591))
    );
    compiled_turns(
        program,
        "ordered live roots",
        &[2.0, 3.0, 4.0],
        mech_engine::SourceDocumentOutputKind::Program,
    );
}

#[test]
fn terminal_logical_updates_do_not_require_a_gather_population_at_activation() {
    for target in ["a[mask,:]", "a[:,mask]", "a[mask,mask]", "a[mask]"] {
        let mask = if target == "a[mask]" {
            "[1 2; 3 4] <= n"
        } else {
            "[1; 2] <= n"
        };
        for (operator, expected) in [("=", [10.0, 10.0, 10.0]), ("+=", [11.0, 21.0, 31.0])] {
            let source = format!(
                "~a := [1 2; 3 4]\n~n := 0\nn += 1\nmask := {mask}\n{target} {operator} 10\na[1,1]\n"
            );
            turns(&source, &expected);
        }
    }
}

#[test]
fn promoted_repeated_compound_selectors_accumulate_each_occurrence() {
    turns(
        "~a := [10<i32> 20<i32>; 30<i32> 40<i32>]\na[[1 1],:] += 2.5\nselected := a[1,1]\nanswer := selected<f64>\nanswer\n",
        &[14.0, 18.0],
    );
}

#[test]
fn nested_repeated_compound_selectors_accumulate_each_occurrence() {
    turns(
        "~a := [10 20; 30 40]\na[[1 1],:][:,1] += 2\na[1,1]\n",
        &[14.0, 18.0],
    );
}

#[test]
fn promoted_repeated_updates_use_canonical_conversion_after_each_operation() {
    for (kind, right) in [
        ("i8", "2.5"),
        ("i16", "2.5"),
        ("i32", "2.5"),
        ("u8", "2.5"),
        ("u16", "2.5"),
        ("u32", "2.5"),
        // The maintained Number contract intentionally has no lossless
        // i64/u64/i128/u128 + f64 promotion. Exercise their integer promotions.
        ("i64", "2<i128>"),
        ("i128", "2<i64>"),
        ("u64", "2<u128>"),
        ("u128", "2<u64>"),
    ] {
        let source = format!(
            "~a := [10<{kind}> 20<{kind}>; 30<{kind}> 40<{kind}>]\na[[1 1],:] += {right}\nselected := a[1,1]\nanswer := selected<f64>\nanswer\n"
        );
        turns(&source, &[14.0, 18.0]);
    }
    for (start, operation, right, expected) in [
        ("-1", "+=", "0.5", 0.0),
        ("2", "*=", "1.5", 4.0),
        ("60", "/=", "2.5", 9.0),
        ("10", "-=", "2.5", 4.0),
    ] {
        let source = format!(
            "~a := [{start}<i32> 20<i32>; 30<i32> 40<i32>]\na[[1 1],:] {operation} {right}\nselected := a[1,1]\nanswer := selected<f64>\nanswer\n"
        );
        turns(&source, &[expected]);
    }
}

#[test]
fn nested_compound_updates_keep_row_major_occurrence_order_and_promotions() {
    for (source, expected) in [
        (
            "~a := [10 20; 30 40]\na[[1 1],:][:,1] += [1;3]\na[1,1]\n",
            14.0,
        ),
        (
            "~a := [10 20; 30 40]\na[[1 1],[2 2]][:,:] += [1 2;3 4]\na[1,2]\n",
            30.0,
        ),
        (
            "~a := [60 20; 30 40]\na[[1 1],:][:,1] /= [2;3]\na[1,1]\n",
            10.0,
        ),
        (
            "~a := [10 20; 30 40]\na[[1 1],:][:,1][:] += [1;3]\na[1,1]\n",
            14.0,
        ),
        (
            "~a := [10<i32> 20<i32>; 30<i32> 40<i32>]\na[[1 1],:][:,1] += 2.5\nselected := a[1,1]\nanswer := selected<f64>\nanswer\n",
            14.0,
        ),
        (
            "~a := [10<i32> 20<i32>; 30<i32> 40<i32>]\na[[1 1],[2 2]][:,:] += [1<i32> 2<i32>;3<i32> 4<i32>]\nselected := a[1,2]\nanswer := selected<f64>\nanswer\n",
            30.0,
        ),
    ] {
        turns(source, &[expected]);
    }
}

#[test]
fn nested_promoted_failure_preserves_state_and_valid_retry_accumulates() {
    use mech_core::snapshot::{SnapshotValidationContext, ValueDataDraft, ValueDraft};
    use mech_engine::resident::CapturedValueInput;
    let source = "divisors := signal<[i16]:2,1>\n~a := [120<i8> 0<i8>;0<i8> 0<i8>]\na[[1 1],:][:,1] /= divisors\nselected := a[1,1]\nanswer := selected<f64>\nanswer\n";
    let artifact = compiled(source).compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    for artifact in [
        artifact,
        mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap(),
    ] {
        let mut catalog = FunctionCatalogBuilder::new();
        mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(0x595, 0),
            &artifact,
            &catalog.build().unwrap(),
            &ActivationFacts::default(),
        )
        .unwrap();
        let before = instance.published_state_hash();
        for (values, succeeds) in [([2i16, 0], false), ([2i16, 3], true)] {
            let value = ValueDraft {
                schema: artifact.inputs()[0].schema,
                shape_values: Box::new([]),
                data: ValueDataDraft::Matrix(values.into_iter().map(ValueDataDraft::I16).collect()),
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let input = CapturedValueInput {
                slot: instance.plan.inputs[0].slot,
                value: &value,
            };
            let result = instance
                .prepare_turn_values(&[input])
                .and_then(|prepared| prepared.publish());
            if succeeds {
                result.unwrap();
                assert!(
                    matches!(instance.copied_output(0).unwrap().data(), ValueData::F64(v) if v.to_f64() == 20.0)
                );
            } else {
                assert!(
                    matches!(
                        result,
                        Err(mech_engine::resident::ResidentExecutionError::Kernel {
                            error: mech_core::ResidentKernelError::Arithmetic,
                            ..
                        })
                    ),
                    "{result:?}"
                );
                assert_eq!(instance.published_state_hash(), before);
            }
        }
    }
}

#[test]
fn nested_selected_rational_power_keeps_its_integer_exponent_contract() {
    turns(
        "~a := [2<r64> 3<r64>;4<r64> 5<r64>]\na[[1 1],:][1,1] ^= 2<i32>\nselected := a[1,1]\nanswer := selected<f64>\nanswer\n",
        &[4.0, 16.0],
    );
}

#[test]
fn promoted_boolean_masks_preserve_canonical_destination_coordinates() {
    turns(
        "~a := [10<i32> 20<i32>;30<i32>40<i32>]\na[[false true;false false]] += 2.5\nselected := a[1,2]\nanswer := selected<f64>\nanswer\n",
        &[22.0, 24.0],
    );
}

#[test]
fn promoted_selected_updates_preserve_row_and_column_broadcasts() {
    for (selection, rhs, expected) in [
        (
            "a[[1 2],:]",
            "[1.5 2.5 3.5]",
            [[11, 22, 33, 41, 52, 63], [12, 24, 36, 42, 54, 66]],
        ),
        (
            "a[:,[1 2 3]]",
            "[1.5;2.5]",
            [[11, 21, 31, 42, 52, 62], [12, 22, 32, 44, 54, 64]],
        ),
        (
            "a[[1 1],:][:,[1 2 3]]",
            "[1.5 2.5 3.5]",
            [[12, 24, 36, 40, 50, 60], [14, 28, 42, 40, 50, 60]],
        ),
        (
            "a[[1 1],:][:,[1 2 3]]",
            "[1.5;2.5]",
            [[13, 23, 33, 40, 50, 60], [16, 26, 36, 40, 50, 60]],
        ),
        (
            "a[[1 1],:]",
            "[1<i64> 2<i64> 3<i64>]",
            [[12, 24, 36, 40, 50, 60], [14, 28, 42, 40, 50, 60]],
        ),
        (
            "a[[1 1],:][:,[1 2 3]]",
            "[1<i32>;2<i32>]",
            [[13, 23, 33, 40, 50, 60], [16, 26, 36, 40, 50, 60]],
        ),
    ] {
        let source = format!(
            "~a := [10<i32> 20<i32> 30<i32>;40<i32> 50<i32> 60<i32>]\n{selection} += {rhs}\na\n"
        );
        let artifact = compiled(&source).compile_artifact().unwrap();
        let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
        for artifact in [
            artifact,
            mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap(),
        ] {
            let mut catalog = FunctionCatalogBuilder::new();
            mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
            let mut instance = activate(
                ReactiveInstanceId::new(0x599, 0),
                &artifact,
                &catalog.build().unwrap(),
                &ActivationFacts::default(),
            )
            .unwrap();
            for expected in expected {
                instance.turn(&[]).unwrap();
                let output = instance.copied_output(0).unwrap();
                let ValueData::Matrix(matrix) = output.data() else {
                    panic!("expected complete matrix: {output:?}")
                };
                let mech_core::snapshot::SequenceView::I32(values) = matrix.elements() else {
                    panic!("expected i32 matrix: {output:?}")
                };
                assert_eq!(
                    values, &expected,
                    "complete source/decoded matrix for {source:?}"
                );
            }
        }
    }
}

#[test]
fn promoted_broadcast_failure_preserves_the_complete_state_before_retry() {
    use mech_core::snapshot::{SnapshotValidationContext, ValueDataDraft, ValueDraft};
    use mech_engine::resident::CapturedValueInput;
    let source = "divisors := signal<[i16]:1,3>\n~a := [120<i8> 120<i8> 120<i8>;60<i8> 60<i8> 60<i8>]\na[[1 1],:][:,[1 2 3]] /= divisors\na\n";
    let artifact = compiled(source).compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    for artifact in [
        artifact,
        mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap(),
    ] {
        let mut catalog = FunctionCatalogBuilder::new();
        mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(0x59a, 0),
            &artifact,
            &catalog.build().unwrap(),
            &ActivationFacts::default(),
        )
        .unwrap();
        let before = instance.published_state_hash();
        for (values, succeeds) in [([2i16, 0, 3], false), ([2i16, 3, 4], true)] {
            let value = ValueDraft {
                schema: artifact.inputs()[0].schema,
                shape_values: Box::new([]),
                data: ValueDataDraft::Matrix(values.into_iter().map(ValueDataDraft::I16).collect()),
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let input = CapturedValueInput {
                slot: instance.plan.inputs[0].slot,
                value: &value,
            };
            let result = instance
                .prepare_turn_values(&[input])
                .and_then(|prepared| prepared.publish());
            if succeeds {
                result.unwrap();
                let output = instance.copied_output(0).unwrap();
                let ValueData::Matrix(matrix) = output.data() else {
                    panic!("expected complete matrix: {output:?}")
                };
                let mech_core::snapshot::SequenceView::I8(values) = matrix.elements() else {
                    panic!("expected i8 matrix: {output:?}")
                };
                assert_eq!(values, &[30, 13, 7, 60, 60, 60]);
            } else {
                assert!(
                    matches!(
                        result,
                        Err(mech_engine::resident::ResidentExecutionError::Kernel {
                            error: mech_core::ResidentKernelError::Arithmetic,
                            ..
                        })
                    ),
                    "{result:?}"
                );
                assert_eq!(instance.published_state_hash(), before);
            }
        }
    }
}

#[test]
fn review_nested_composite_matrix_field_update() {
    turns(
        "~a := [{value: 1} {value: 4}]\na[1].value += 1\na[1].value + a[2].value\n",
        &[6.0, 7.0],
    );
}

fn matrix_turns(source: &str, expected: &[Vec<f64>]) {
    let artifact = compiled(source).compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    for artifact in [
        artifact,
        mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap(),
    ] {
        let mut catalog = FunctionCatalogBuilder::new();
        mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(0x59b, 0),
            &artifact,
            &catalog.build().unwrap(),
            &ActivationFacts::default(),
        )
        .unwrap_or_else(|error| panic!("{source:?}: {error:?}"));
        for expected in expected {
            instance.turn(&[]).unwrap();
            let output = instance.copied_output(0).unwrap();
            let ValueData::Matrix(matrix) = output.data() else {
                panic!("{output:?}")
            };
            let actual: Vec<f64> = match matrix.elements() {
                mech_core::snapshot::SequenceView::I32(values) => {
                    values.iter().map(|v| f64::from(*v)).collect()
                }
                mech_core::snapshot::SequenceView::F64(values) => {
                    values.iter().map(|v| v.to_f64()).collect()
                }
                _ => panic!("{output:?}"),
            };
            assert_eq!(
                &actual, expected,
                "complete source/decoded output for {source:?}"
            );
        }
    }
}

#[test]
fn turn_mask_broadcast_uses_live_population_and_preserves_complete_matrices() {
    for kind in ["i32", "f64"] {
        for (selection, rhs, thresholds, increments) in [
            (
                "a[mask,:]",
                "[1.5 2.5 3.5]",
                "[1;2]",
                [1.5, 2.5, 3.5, 1.5, 2.5, 3.5],
            ),
            (
                "a[:,mask]",
                "[1.5;2.5]",
                "[1 2 2]",
                [1.5, 1.5, 1.5, 2.5, 2.5, 2.5],
            ),
            (
                "a[mask,[1 2 3]]",
                "[1.5 2.5 3.5]",
                "[1;2]",
                [1.5, 2.5, 3.5, 1.5, 2.5, 3.5],
            ),
            (
                "a[mask,:][:,[1 2 3]]",
                "[1.5 2.5 3.5]",
                "[1;2]",
                [1.5, 2.5, 3.5, 1.5, 2.5, 3.5],
            ),
        ] {
            let source = format!(
                "~a := [10<{kind}> 20<{kind}> 30<{kind}>;40<{kind}> 50<{kind}> 60<{kind}>]\n~n := -1\nn += 1\nmask := {thresholds} <= n\n{selection} += {rhs}\na\n"
            );
            let initial = vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0];
            let mut next = initial.clone();
            let mut expected = vec![initial];
            for population in 1..=2 {
                for index in 0..6 {
                    let selected = if selection == "a[:,mask]" {
                        index % 3 == 0 || population == 2
                    } else {
                        index < 3 || population == 2
                    };
                    if selected {
                        next[index] += if kind == "i32" {
                            f64::trunc(increments[index])
                        } else {
                            increments[index]
                        };
                    }
                }
                expected.push(next.clone());
            }
            matrix_turns(&source, &expected);
        }
    }
}

#[test]
fn logical_selected_updates_route_full_base_rhs_by_destination_position() {
    for (masks, selection, expected) in [
        (
            "rmask := [true;false]",
            "a[rmask,:]",
            vec![11.0, 22.0, 33.0, 40.0, 50.0, 60.0],
        ),
        (
            "cmask := [true false true]",
            "a[:,cmask]",
            vec![11.0, 20.0, 33.0, 50.0, 50.0, 90.0],
        ),
        (
            "rmask := [true;false]\ncmask := [false true true]",
            "a[rmask,cmask]",
            vec![10.0, 22.0, 33.0, 40.0, 50.0, 60.0],
        ),
    ] {
        let source = format!(
            "~a := [10<i32> 20<i32> 30<i32>;40<i32> 50<i32> 60<i32>]\n{masks}\nrhs := [1.5 2.5 3.5;10.5 20.5 30.5]\n{selection} += rhs\na\n"
        );
        matrix_turns(&source, &[expected]);
    }
}

#[test]
fn nested_logical_routing_uses_immediate_view_coordinates() {
    for (selection, rhs, expected) in [
        (
            "a[[2 1],:][[true;false],:]",
            "[1.5 2.5;10.5 20.5]",
            vec![10.0, 20.0, 31.0, 42.0],
        ),
        (
            "a[:,[2 1]][:,[true false]]",
            "[1.5 2.5;10.5 20.5]",
            vec![10.0, 21.0, 30.0, 50.0],
        ),
        (
            "a[[4 1]][[true;false]]",
            "[1.5;10.5]",
            vec![10.0, 20.0, 30.0, 41.0],
        ),
    ] {
        let source = format!(
            "~a := [10<i32> 20<i32>;30<i32> 40<i32>]\nrhs := {rhs}\n{selection} += rhs\na\n"
        );
        matrix_turns(&source, &[expected]);
    }
    matrix_turns(
        "~a := [10<i32> 20<i32>;30<i32> 40<i32>;50<i32> 60<i32>]\n\
         rhs := [1.5 2.5;10.5 20.5]\n\
         a[[3 1],:][[true;false],:] += rhs\n\
         a\n",
        &[vec![10.0, 20.0, 30.0, 40.0, 51.0, 62.0]],
    );
}

#[test]
fn sparse_nested_update_does_not_materialize_base_sized_addresses() {
    // Keep the executable witness at the resident target's 65,536-element
    // output ceiling. The artifact assertions are the scale-independent proof
    // that no base-sized identity/address helper survives at larger sizes.
    const SIZE: usize = 256;
    let row = std::iter::repeat_n("1<f64>", SIZE)
        .collect::<Vec<_>>()
        .join(" ");
    let rows = std::iter::repeat_n("row", SIZE)
        .collect::<Vec<_>>()
        .join(";");
    let source = format!("row := [{row}]\n~a := [{rows}]\na[1,:][1] += 1\na[1,1]\n");
    let artifact = compiled(&source).compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    for artifact in [
        artifact,
        mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap(),
    ] {
        let operations = artifact
            .nodes()
            .iter()
            .filter_map(|node| node.as_operation())
            .map(|operation| {
                (
                    operation.operation.module_path.as_ref(),
                    operation.operation.operation_name.as_str(),
                )
            })
            .collect::<Vec<_>>();
        assert!(
            operations
                .iter()
                .any(|(path, name)| { *path == ["core", "assign", "nested"] && *name == "add" })
        );
        assert!(!operations.iter().any(|(path, name)| {
            *path == ["core", "assign"]
                && matches!(*name, "identity-indices" | "broadcast" | "selection-order")
        }));
        let mut catalog = FunctionCatalogBuilder::new();
        mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(0x59d, 0),
            &artifact,
            &catalog.build().unwrap(),
            &ActivationFacts::default(),
        )
        .unwrap();
        instance.turn(&[]).unwrap();
        assert!(
            matches!(instance.copied_output(0).unwrap().data(), ValueData::F64(value) if value.to_f64() == 2.0)
        );
    }
}

#[test]
fn nested_snapshot_helpers_report_signed_zero_representation_changes() {
    use mech_core::snapshot::{F32Bits, SnapshotValidationContext, ValueDataDraft, ValueDraft};
    use mech_engine::resident::CapturedValueInput;

    let source = "delta := signal<[f32]:1,1>\n~a := [-1<f32> 2<f32>;-1<f32> 2<f32>]\na[[1 2],:][:,1] *= delta\nselected := a[1,1]\nanswer := selected<f64>\n1 / answer\n";
    let artifact = compiled(source).compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    for artifact in [
        artifact,
        mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap(),
    ] {
        let mut catalog = FunctionCatalogBuilder::new();
        mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(0x59e, 0),
            &artifact,
            &catalog.build().unwrap(),
            &ActivationFacts::default(),
        )
        .unwrap();
        for (delta, expected) in [(0.0_f32, f64::NEG_INFINITY), (-0.0, f64::INFINITY)] {
            let value = ValueDraft {
                schema: artifact.inputs()[0].schema,
                shape_values: Box::new([]),
                data: ValueDataDraft::Matrix(
                    vec![ValueDataDraft::F32(F32Bits::from_f32(delta))].into_boxed_slice(),
                ),
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let input = CapturedValueInput {
                slot: instance.plan.inputs[0].slot,
                value: &value,
            };
            instance
                .prepare_turn_values(&[input])
                .unwrap()
                .publish()
                .unwrap();
            let output = instance.copied_output(0).unwrap();
            let ValueData::F64(actual) = output.data() else {
                panic!("expected f64 output: {output:?}")
            };
            assert_eq!(
                actual.to_f64().to_bits(),
                expected.to_bits(),
                "delta {delta:?}"
            );
        }
    }
}

#[test]
fn review_asymmetric_rectangle_admission() {
    let row = std::iter::repeat_n("10<i32>", 1000)
        .collect::<Vec<_>>()
        .join(" ");
    let columns = (1..=1000)
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    turns(
        &format!(
            "~a := [{row}]\na[[1],[{columns}]] += 2.5\nselected := a[1,1000]\nanswer := selected<f64>\nanswer\n"
        ),
        &[12.0, 14.0],
    );
}

#[test]
fn selected_f64_matrix_updates_propagate_signed_zero_changes() {
    use mech_core::snapshot::{SnapshotValidationContext, ValueDataDraft, ValueDraft};
    use mech_engine::resident::CapturedValueInput;

    fn checked_update(
        kernel: &mech_core::BoundResidentKernel,
        inputs: &dyn mech_core::ResidentKernelInputs,
        output: mech_core::ResidentValueMut<'_>,
    ) -> Result<bool, mech_core::ResidentKernelError> {
        struct Inputs<'a>(&'a dyn mech_core::ResidentKernelInputs);
        impl mech_core::ResidentKernelInputs for Inputs<'_> {
            fn len(&self) -> usize {
                self.0.len()
            }
            fn get(&self, index: usize) -> Option<mech_core::ResidentValueRef<'_>> {
                self.0.get(index)
            }
        }
        let mech_core::ResidentValueMut::F64(target) = output else {
            panic!("expected dense F64 selected update")
        };
        let before: Vec<_> = target.iter().map(|x| x.to_bits()).collect();
        let changed = kernel
            .retained_state::<mech_core::BoundResidentKernel>()
            .unwrap()
            .execute(&Inputs(inputs), mech_core::ResidentValueMut::F64(target))?;
        assert_eq!(
            changed,
            before
                .iter()
                .zip(target.iter())
                .any(|(old, new)| *old != new.to_bits()),
            "kernel must report representation changes"
        );
        Ok(changed)
    }

    for selection in ["a[1,:]", "a[:,1]", "a[[1],[1 2]]"] {
        let source = format!(
            "delta := signal<f64>\n~a := [-0.0 -0.0;-0.0 -0.0]\n{selection} += [delta]\n1 / a[1,1]\n"
        );
        let artifact = compiled(&source).compile_artifact().unwrap();
        let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
        for artifact in [
            artifact,
            mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap(),
        ] {
            let mut catalog = FunctionCatalogBuilder::new();
            mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
            let mut instance = activate(
                ReactiveInstanceId::new(0x59c, 0),
                &artifact,
                &catalog.build().unwrap(),
                &ActivationFacts::default(),
            )
            .unwrap();
            let updates: Vec<_> = instance
                .plan
                .steps
                .iter()
                .enumerate()
                .filter_map(|(index, step)| match step {
                    mech_engine::resident::ActivatedTurnStep::Kernel(node)
                        if matches!(
                            node.construction,
                            mech_core::OutputConstruction::ReadModifyWrite { .. }
                        ) =>
                    {
                        Some((index, node.kernel.clone()))
                    }
                    _ => None,
                })
                .collect();
            assert!(!updates.is_empty());
            for (index, kernel) in updates {
                instance.plan.replace_kernel_for_test(
                    index,
                    mech_core::BoundResidentKernel::new(checked_update, Box::new([]))
                        .with_retained_state(std::sync::Arc::new(kernel)),
                );
            }
            for (delta, expected) in [
                (-0.0, f64::NEG_INFINITY),
                (0.0, f64::INFINITY),
                (-0.0, f64::INFINITY),
            ] {
                let value = ValueDraft {
                    schema: artifact.inputs()[0].schema,
                    shape_values: Box::new([]),
                    data: ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_f64(delta)),
                }
                .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                .unwrap();
                let input = CapturedValueInput {
                    slot: instance.plan.inputs[0].slot,
                    value: &value,
                };
                instance
                    .prepare_turn_values(&[input])
                    .unwrap()
                    .publish()
                    .unwrap();
                let output = instance.copied_output(0).unwrap();
                let ValueData::F64(actual) = output.data() else {
                    panic!("{output:?}")
                };
                assert_eq!(
                    actual.to_f64().to_bits(),
                    expected.to_bits(),
                    "{selection}: delta {delta:?}"
                );
            }
        }
    }
}
