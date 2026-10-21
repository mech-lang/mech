//! Actual selected original/decoded producers, EOF geometry and ownership.
use super::*;
use crate::native_artifact::{
    NativeCollectionPrimitive, NativeProgramLibraryTemplate, native_collection_operation_template,
    prepare_native_program_library_call,
};
use mech_core::{FunctionCatalog, FunctionCatalogBuilder, ValueDataDraft, ValueDraft};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn variants() -> [crate::ProgramArtifact; 2] {
    let source = "lines := text/lines(input<string>)\ncount := sequence/length(lines)\nline := lines[position<u64>]\nselected := lines[positions<[u64]>]\ntext/join(selected, separator<string>)\n";
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x454f464c), Revision(0), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let artifact = crate::CanonicalSourceFrontend
        .compile_document(&DocumentSyntax::cast(parsed.syntax()).unwrap())
        .unwrap()
        .compile_artifact()
        .unwrap();
    let decoded = crate::decode_program_artifact_bytecode_v1(
        &crate::encode_program_artifact_bytecode_v1(&artifact).unwrap(),
    )
    .unwrap();
    [artifact, decoded]
}
fn catalog() -> FunctionCatalog {
    let mut builder = FunctionCatalogBuilder::new();
    crate::install_intrinsic_resident(&mut builder).unwrap();
    builder.build().unwrap()
}
fn prepared(
    artifact: &crate::ProgramArtifact,
    primitive: NativeCollectionPrimitive,
) -> NativePreparedLibraryCall {
    let catalog = catalog();
    let template = artifact
        .nodes()
        .iter()
        .find_map(|node| {
            native_collection_operation_template(artifact, node.node, &catalog)
                .ok()
                .filter(|template| template.primitive() == primitive)
        })
        .unwrap();
    prepare_native_program_library_call(
        artifact,
        &NativeProgramLibraryTemplate::from_root(template),
        &catalog,
    )
    .unwrap()
}
fn limits() -> MemoryBudgetLimits {
    let mut limits = mech_core::TargetMemoryProfile::current_resident_cpu()
        .unwrap()
        .limits;
    limits.max_output_elements = Some(4096);
    limits.max_output_bytes = Some(32 * 1024 * 1024);
    limits.max_temporary_bytes = Some(32 * 1024 * 1024);
    limits.max_compute_work = Some(1 << 40);
    limits
}
fn work(memory: &ManagedMemoryBudget) -> NativeLibraryWorkBudget {
    NativeLibraryWorkBudget::new(1 << 48, 1 << 32, memory).unwrap()
}
fn capture(
    call: &NativePreparedLibraryCall,
    text: &str,
    limits: MemoryBudgetLimits,
    work: &NativeLibraryWorkBudget,
) -> NativeTextLineChunks {
    let mut owner = call
        .prepare_text_line_chunks(text.len() as u64, 4, limits, work)
        .unwrap();
    for character in text.chars() {
        owner.push(character.encode_utf8(&mut [0; 4])).unwrap();
    }
    owner.finish().unwrap();
    owner
}

#[test]
fn eof_capture_matches_actual_snapshot_geometry_without_string_or_row_table() {
    for artifact in variants() {
        let call = prepared(&artifact, NativeCollectionPrimitive::TextLines);
        let length = prepared(&artifact, NativeCollectionPrimitive::SequenceLength);
        let element = prepared(&artifact, NativeCollectionPrimitive::MatrixScalarAccess);
        for text in ["", "\n", "a\r\n猫\ne\u{301}\nlast\r", "abc\nxyz\n"] {
            let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
            let work = work(&memory);
            let owner = capture(&call, text, limits(), &work);
            let matrix = call
                .text_line_matrix_from_capture(owner, limits(), &work, ResourceDemand::default())
                .unwrap();
            let mech_core::ResolvedOperationContract::Declared(contract) = call.template.contract()
            else {
                panic!("declared text/lines")
            };
            let value = ValueDraft {
                schema: contract.inputs[0].schema,
                shape_values: Box::new([]),
                data: ValueDataDraft::String(text.into()),
            }
            .finalize(&mech_core::snapshot::SnapshotValidationContext::new(
                artifact.schemas(),
            ))
            .unwrap();
            let mut reservation = memory
                .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
                .unwrap();
            let value = value.into_memory_budget(&mut reservation).unwrap();
            let snapshot = call
                .text_line_matrix(&value, limits(), &work, ResourceDemand::default())
                .unwrap();
            assert_eq!(matrix.shape(), snapshot.shape());
            assert_eq!(matrix.schema(), snapshot.schema());
            assert_eq!(matrix.schema_key(), snapshot.schema_key());
            assert_eq!(matrix.count(), text.lines().count());
            assert!(matrix.source().is_none());
            assert!(matrix.rows().is_none());
            assert!(matrix.captured_chunks().is_some());
            assert_eq!(
                length
                    .text_line_matrix_length(&matrix, limits(), &work, ResourceDemand::default())
                    .unwrap(),
                matrix.count() as u64
            );
            for (ordinal, expected) in text.lines().enumerate() {
                assert_eq!(matrix.line(ordinal), Some(expected));
                assert_eq!(
                    element
                        .text_line_matrix_element(
                            &matrix,
                            ordinal as u64 + 1,
                            limits(),
                            &work,
                            ResourceDemand::default()
                        )
                        .unwrap(),
                    expected
                );
            }
            assert!(
                matrix
                    .retained_bytes(&ManagedMemoryBudget::new(1 << 20))
                    .is_err()
            );
            drop((matrix, snapshot, value, reservation, work));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn retained_capture_keeps_full_eof_count_and_enforces_full_output_element_limit() {
    for artifact in variants() {
        let call = prepared(&artifact, NativeCollectionPrimitive::TextLines);
        for limit in [2, 5] {
            let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
            let work = work(&memory);
            let mut limits = limits();
            limits.max_output_elements = Some(limit);
            let mut owner = capture(&call, "discard\n猫\nA\nend\nsuffix", limits, &work);
            owner.retain_range(2, 4).unwrap();
            let result =
                call.text_line_matrix_from_capture(owner, limits, &work, ResourceDemand::default());
            if limit == 2 {
                assert!(matches!(
                    result,
                    Err(InvokeError::Kernel(ResidentKernelError::ResourceLimit {
                        dimension: Dimension::OutputElements,
                        required: 5,
                        limit: 2,
                        ..
                    }))
                ));
            } else {
                let matrix = result.unwrap();
                assert_eq!(matrix.count(), 5);
                assert_eq!(matrix.line(0), None);
                assert_eq!(matrix.line(1), Some("猫"));
                assert_eq!(matrix.line(2), Some("A"));
                assert_eq!(matrix.line(3), None);
            }
            drop(work);
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn unfinished_unicode_cell_uses_maintained_scalar_index_and_actual_window_owner() {
    for artifact in variants() {
        let producer = prepared(&artifact, NativeCollectionPrimitive::TextLines);
        let element = prepared(&artifact, NativeCollectionPrimitive::MatrixScalarAccess);
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let work = work(&memory);
        let mut owner = producer
            .prepare_text_line_chunks(64, 64, limits(), &work)
            .unwrap();
        owner.push("discard\n猫\npartial").unwrap();
        owner.retain_from(2).unwrap();
        let window = owner.borrow_window(2, 3).unwrap();
        assert!(window.final_facts().is_none());
        assert_eq!(
            element
                .text_line_window_element(&window, 2, limits(), &work, ResourceDemand::default())
                .unwrap(),
            "猫"
        );
        for index in [0, 3] {
            let actual = element
                .text_line_window_element(
                    &window,
                    index,
                    limits(),
                    &work,
                    ResourceDemand::default(),
                )
                .unwrap_err();
            let expected =
                crate::resident::numeric::selected_string_matrix::available_scalar_index(index, 2)
                    .unwrap_err();
            assert_eq!(
                format!("{actual:?}"),
                format!("{:?}", InvokeError::Kernel(expected))
            );
        }
        assert!(
            element
                .text_line_window_element(&window, 1, limits(), &work, ResourceDemand::default())
                .is_err()
        );
        let foreign = NativeLibraryWorkBudget::new(1 << 48, 1 << 32, &memory).unwrap();
        assert!(
            element
                .text_line_window_element(&window, 2, limits(), &foreign, ResourceDemand::default())
                .is_err()
        );
        drop(window);
        drop((owner, work, foreign));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn final_admission_rejects_unfinished_foreign_and_changed_scopes_without_refill() {
    let artifact = variants().into_iter().next().unwrap();
    let call = prepared(&artifact, NativeCollectionPrimitive::TextLines);
    let foreign = prepared(&artifact, NativeCollectionPrimitive::TextLines);
    let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
    let work = work(&memory);
    let unfinished = call
        .prepare_text_line_chunks(4, 4, limits(), &work)
        .unwrap();
    assert!(
        call.text_line_matrix_from_capture(unfinished, limits(), &work, ResourceDemand::default())
            .is_err()
    );
    let owner = capture(&call, "a\nb", limits(), &work);
    assert!(
        foreign
            .text_line_matrix_from_capture(owner, limits(), &work, ResourceDemand::default())
            .is_err()
    );
    let owner = capture(&call, "a\nb", limits(), &work);
    let other_work = NativeLibraryWorkBudget::new(1 << 48, 1 << 32, &memory).unwrap();
    assert!(
        call.text_line_matrix_from_capture(owner, limits(), &other_work, ResourceDemand::default())
            .is_err()
    );
    let owner = capture(&call, "a\nb", limits(), &work);
    let mut changed = limits();
    changed.max_compute_work = Some(changed.max_compute_work.unwrap() + 1);
    assert!(
        call.text_line_matrix_from_capture(owner, changed, &work, ResourceDemand::default())
            .is_err()
    );
    let start = work.used().0;
    let owner = capture(&call, "a\nb", limits(), &work);
    let matrix = call
        .text_line_matrix_from_capture(owner, limits(), &work, ResourceDemand::default())
        .unwrap();
    let exact = work.used().0 - start;
    drop(matrix);
    for cap in [exact - 1, exact] {
        let mut limited = limits();
        limited.max_compute_work = Some(cap);
        let owner = capture(&call, "a\nb", limited, &work);
        let result =
            call.text_line_matrix_from_capture(owner, limited, &work, ResourceDemand::default());
        if cap == exact {
            assert!(result.is_ok());
        } else {
            assert!(
                matches!(result, Err(InvokeError::Kernel(ResidentKernelError::ResourceLimit { dimension: Dimension::ComputeWork, required, limit, .. })) if required == exact && limit == cap)
            );
        }
    }
    drop((work, other_work));
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn capture_work_bound_covers_fragmented_loans_retention_and_final_admission() {
    let artifact = variants().into_iter().next().unwrap();
    let call = prepared(&artifact, NativeCollectionPrimitive::TextLines);
    assert!(call.text_line_capture_work_bound(254_166_745).unwrap() < 1 << 43);
    for text in [
        String::new(),
        "\n".repeat(1024),
        "x".repeat(1024),
        "猫\r\ne\u{301}\n".repeat(128),
    ] {
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let work = work(&memory);
        let start = work.used().0;
        let mut owner = call
            .prepare_text_line_chunks(text.len() as u64, 4, limits(), &work)
            .unwrap();
        let mut available = 0;
        for character in text.chars() {
            owner.push(character.encode_utf8(&mut [0; 4])).unwrap();
            let next = owner.available_lines().unwrap();
            let window = owner.borrow_window(available + 1, next + 1).unwrap();
            call.bind_text_line_window(&window, &work).unwrap();
            drop(window);
            owner.retain_from(1).unwrap();
            available = next;
        }
        let facts = owner.finish().unwrap();
        let window = owner
            .borrow_window(available + 1, facts.lines() + 1)
            .unwrap();
        call.bind_text_line_window(&window, &work).unwrap();
        drop(window);
        owner.retain_range(1, facts.lines() + 1).unwrap();
        let matrix = call
            .text_line_matrix_from_capture(owner, limits(), &work, ResourceDemand::default())
            .unwrap();
        assert!(
            work.used().0 - start
                <= call
                    .text_line_capture_work_bound(text.len() as u64)
                    .unwrap()
        );
        drop((matrix, work));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn consuming_join_releases_unique_chunks_and_preserves_shared_and_reordered_sources() {
    for artifact in variants() {
        let producer = prepared(&artifact, NativeCollectionPrimitive::TextLines);
        let selection = prepared(&artifact, NativeCollectionPrimitive::MatrixRangeAccess);
        let join = prepared(&artifact, NativeCollectionPrimitive::TextJoin);
        let mech_core::ResolvedOperationContract::Declared(select_contract) =
            selection.template.contract()
        else {
            panic!("declared selection")
        };
        let mech_core::ResolvedOperationContract::Declared(join_contract) =
            join.template.contract()
        else {
            panic!("declared join")
        };
        for (positions, shared) in [
            (vec![2, 3, 4], false),
            (vec![2, 3, 4], true),
            (vec![4, 2, 2], false),
            (vec![2, 4, 1, 3], false),
            (vec![4, 3, 2, 1, 1, 4], true),
            (vec![], false),
        ] {
            let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
            let work = work(&memory);
            let admit = |draft: ValueDraft| {
                let value = draft
                    .finalize(&mech_core::snapshot::SnapshotValidationContext::new(
                        artifact.schemas(),
                    ))
                    .unwrap();
                let mut reservation = memory
                    .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
                    .unwrap();
                value.into_memory_budget(&mut reservation).unwrap()
            };
            let selector_shape = mech_core::shape_for_resolved_extents(
                &selection.template.inputs()[1],
                &[1, positions.len() as u64],
            )
            .unwrap();
            let selector = admit(ValueDraft {
                schema: select_contract.inputs[1].schema,
                shape_values: selector_shape.parameter_values().into(),
                data: ValueDataDraft::Matrix(
                    positions
                        .iter()
                        .copied()
                        .map(ValueDataDraft::U64)
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                ),
            });
            let separator = admit(ValueDraft {
                schema: join_contract.inputs[1].schema,
                shape_values: Box::new([]),
                data: ValueDataDraft::String("|".into()),
            });
            let baseline = memory.used_bytes();
            let owner = capture(&producer, "discard\n猫\nA\nlast\n", limits(), &work);
            let matrix = std::rc::Rc::new(
                producer
                    .text_line_matrix_from_capture(
                        owner,
                        limits(),
                        &work,
                        ResourceDemand::default(),
                    )
                    .unwrap(),
            );
            let view = selection
                .selected_text_line_matrix(
                    &matrix,
                    &selector,
                    limits(),
                    &work,
                    ResourceDemand::default(),
                )
                .unwrap();
            let shared_owner = shared.then(|| matrix.clone());
            drop(matrix);
            let expected = positions
                .iter()
                .map(|index| ["discard", "猫", "A", "last"][*index as usize - 1])
                .collect::<Vec<_>>()
                .join("|");
            let mut target = String::new();
            target.try_reserve_exact(expected.len()).unwrap();
            let reservation = memory.reserve_capacity(expected.len() as u64).unwrap();
            work.charge(
                (expected.len() as u64 + positions.len() as u64 + 1) * 128,
                0,
            )
            .unwrap();
            join.selected_matrix_join_append_consuming(view, &separator, &mut target, &memory)
                .unwrap();
            assert_eq!(target, expected);
            if let Some(owner) = shared_owner {
                assert_eq!(owner.line(1), Some("猫"));
                assert!(owner.captured_chunks().unwrap().retained_capacity_bytes() > 0);
            } else {
                assert_eq!(memory.used_bytes(), baseline + reservation.capacity_bytes());
            }
            drop((selector, separator, reservation, work));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}
