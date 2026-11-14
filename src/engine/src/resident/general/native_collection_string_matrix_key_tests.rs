use super::*;
use crate::resident::general::native_collection_calls::{
    NativePreparedLibraryCallCache, prepare_native_program_library_call,
};
use crate::{CanonicalSourceFrontend, ProgramArtifact};
use mech_core::snapshot::SnapshotValidationContext;
use mech_core::{ManagedMemoryBudget, SchemaId, ValueDataDraft as Data, ValueDraft};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

pub(super) fn catalog() -> mech_core::FunctionCatalog {
    let mut b = mech_core::FunctionCatalogBuilder::new();
    crate::install_intrinsic_runtime(&mut b).unwrap();
    crate::install_intrinsic_compiler_runtime(&mut b).unwrap();
    crate::install_intrinsic_resident(&mut b).unwrap();
    crate::install_intrinsic_source(&mut b).unwrap();
    b.build().unwrap()
}
fn variants(shape: &str) -> [ProgramArtifact; 2] {
    let source = format!(
        "words := input<{shape}>\nkey := words[position<u64>]\nindex := dictionary<{{string:u64}}>\n(index{{key}}, key ∈ index)\n"
    );
    artifacts(&source)
}
pub(super) fn artifacts(source: &str) -> [ProgramArtifact; 2] {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x534d4b59), Revision(0), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let artifact = CanonicalSourceFrontend
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

#[test]
fn selected_string_projection_admits_the_complete_large_schema_owner() {
    let fields = (0..256)
        .map(|i| format!("field{i}<string>"))
        .collect::<Vec<_>>()
        .join(" ");
    let source = format!("words := input<[string]>\nwords[position<u64>]\npadding<{{{fields}}}>\n");
    for artifact in artifacts(&source) {
        assert!(artifact.schemas().clone_allocation_bound_bytes().unwrap() > 16_384);
        let calls = prepared(&artifact);
        let call = calls
            .iter()
            .find(|c| c.supports_string_matrix_element())
            .unwrap();
        let memory = ManagedMemoryBudget::new(64 << 20);
        let work = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &memory).unwrap();
        let source = input_source(&artifact, call, &["é", "last"], 1, 2, &memory);
        let before = (memory.used_bytes(), memory.retained_snapshot_nodes());
        for index in [1, 2] {
            let output = call
                .string_matrix_element(&source, index, limits(), &work, ResourceDemand::default())
                .unwrap()
                .unwrap();
            assert_eq!(string(&output), if index == 1 { "é" } else { "last" });
            drop(output);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                before
            );
        }
        drop((source, work));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
fn prepared(artifact: &ProgramArtifact) -> Vec<NativePreparedLibraryCall> {
    let catalog = catalog();
    let plan = crate::native_artifact::lower_native_program_artifact(artifact, &catalog).unwrap();
    plan.blocks()
        .iter()
        .filter_map(|block| {
            let crate::native_artifact::NativeIterationAction::LibraryCall { template, .. } =
                block.action()
            else {
                return None;
            };
            Some(prepare_native_program_library_call(artifact, template, &catalog).unwrap())
        })
        .collect()
}
pub(super) fn limits() -> MemoryBudgetLimits {
    let mut l = mech_core::TargetMemoryProfile::current_resident_cpu()
        .unwrap()
        .limits;
    l.max_compute_work = Some(u64::MAX);
    l.max_comparison_work = Some(u64::MAX);
    l.max_temporary_bytes = Some(u64::MAX);
    l.max_retained_nodes = Some(u64::MAX);
    l.max_output_elements = Some(u64::MAX);
    l.max_output_bytes = Some(u64::MAX);
    l.max_cloned_bytes = Some(u64::MAX);
    l
}
pub(super) fn value(
    artifact: &ProgramArtifact,
    id: SchemaId,
    shape: Box<[u64]>,
    data: Data,
    memory: &ManagedMemoryBudget,
) -> Value {
    let value = ValueDraft {
        schema: id,
        shape_values: shape,
        data,
    }
    .finalize(&SnapshotValidationContext::new(artifact.schemas()))
    .unwrap();
    let mut claim = memory
        .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
        .unwrap();
    value.into_memory_budget(&mut claim).unwrap()
}
fn input_source(
    artifact: &ProgramArtifact,
    call: &NativePreparedLibraryCall,
    texts: &[&str],
    rows: u64,
    columns: u64,
    memory: &ManagedMemoryBudget,
) -> Value {
    let mech_core::ResolvedOperationContract::Declared(contract) = call.template.contract() else {
        panic!("declared")
    };
    let shape = mech_core::shape_for_resolved_extents(&call.template.inputs()[0], &[rows, columns])
        .unwrap();
    value(
        artifact,
        contract.inputs[0].schema,
        shape.parameter_values().to_vec().into(),
        Data::Matrix(
            texts
                .iter()
                .map(|s| Data::String((*s).into()))
                .collect::<Vec<_>>()
                .into(),
        ),
        memory,
    )
}
fn ordinary(
    artifact: &ProgramArtifact,
    call: &NativePreparedLibraryCall,
    source: &Value,
    index: u64,
    l: MemoryBudgetLimits,
    work: &NativeLibraryWorkBudget,
) -> Result<Value, InvokeError> {
    let mech_core::ResolvedOperationContract::Declared(contract) = call.template.contract() else {
        panic!("declared")
    };
    let selector = value(
        artifact,
        contract.inputs[1].schema,
        Box::new([]),
        Data::U64(index),
        &work.memory_budget(),
    );
    let mut cache = NativePreparedLibraryCallCache::default();
    let mut output = None;
    call.invoke_current_cached(
        &mut cache,
        &[Some(source.clone()), Some(selector)],
        &mut output,
        l,
        work,
    )?;
    Ok(output.unwrap())
}
fn string(value: &Value) -> &str {
    let ValueData::String(text) = value.data() else {
        panic!("String")
    };
    text
}

#[test]
fn whole_string_matrix_loan_uses_current_source_and_canonical_column_major_selection() {
    let text = ["", "é", "a\0b", "🧑‍🚀", "e\u{301}\r\n", "last"];
    for shape in ["[string]", "[string]:2,3"] {
        for artifact in variants(shape) {
            let calls = prepared(&artifact);
            let call = calls
                .iter()
                .find(|c| c.supports_string_matrix_key_loan())
                .unwrap();
            let consumer = calls
                .iter()
                .find(|c| c.template.primitive() == Primitive::MapContainsKey)
                .unwrap();
            let memory = ManagedMemoryBudget::new(64 << 20);
            let work = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &memory).unwrap();
            let rows = if shape == "[string]" { 1 } else { 2 };
            let columns = 6 / rows;
            let source = input_source(&artifact, call, &text, rows, columns, &memory);
            let alias = source.clone();
            let mut loan = call
                .try_string_matrix_key_loan(&source, limits(), &work, ResourceDemand::default())
                .unwrap()
                .unwrap();
            let view = loan.native_view(&source, &work).unwrap();
            assert_eq!((view.count, view.rows, view.columns), (6, rows, columns));
            assert_eq!(core::mem::size_of::<NativeStringMatrixKeyCell>(), 64);
            assert_eq!(view.cells, loan.cells.as_ptr() as u64);
            for index in 1..=6 {
                let old = ordinary(&artifact, call, &source, index, limits(), &work).unwrap();
                let selected = loan
                    .admit_element(&source, index, limits(), &work, ResourceDemand::default())
                    .unwrap();
                let bytes = memory.used_bytes();
                let nodes = memory.retained_snapshot_nodes();
                let got = loan
                    .with_element(consumer, &source, &selected, &work, |key| key.to_owned())
                    .unwrap();
                assert_eq!(got, string(&old));
                assert_eq!(memory.used_bytes(), bytes);
                assert_eq!(memory.retained_snapshot_nodes(), nodes);
                let cell = &loan.cells[selected.ordinal];
                // SAFETY: the whole loan owns its canonical source; this exact
                // current descriptor was emitted by its private constructor.
                let native = unsafe {
                    core::slice::from_raw_parts(cell.address as *const u8, cell.bytes as usize)
                };
                assert_eq!(native, got.as_bytes());
            }
            assert!(source.shares_frozen_storage(&alias));
            assert_eq!(
                string(&ordinary(&artifact, call, &alias, 2, limits(), &work).unwrap()),
                if rows == 1 { "é" } else { "🧑‍🚀" }
            );
            drop((loan, alias, source, work));
            assert_eq!(memory.used_bytes(), 0);
            assert_eq!(memory.retained_snapshot_nodes(), 0);
        }
    }
}

#[test]
fn logical_string_output_and_clone_caps_match_maintained_scalar_kernel_exactly() {
    for artifact in variants("[string]") {
        let calls = prepared(&artifact);
        let call = calls
            .iter()
            .find(|c| c.supports_string_matrix_key_loan())
            .unwrap();
        let memory = ManagedMemoryBudget::new(64 << 20);
        let work = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &memory).unwrap();
        let text = "é🧑‍🚀\0e\u{301}\r\n".repeat(129);
        let source = input_source(&artifact, call, &[&text], 1, 1, &memory);
        let mut loan = call
            .try_string_matrix_key_loan(&source, limits(), &work, ResourceDemand::default())
            .unwrap()
            .unwrap();
        let cell = loan.cells[0];
        let output = ordinary(&artifact, call, &source, 1, limits(), &work).unwrap();
        let mut meter = ResidentBudgetMeter::for_closed_analysis(u64::MAX, u64::MAX);
        let expected = budget::measure_borrowed_canonical_value_footprint(
            &mut meter,
            &output,
            artifact.schemas(),
        )
        .unwrap();
        assert_eq!(cell.logical_output_bytes, expected.retained_bytes);
        for (dimension, exact) in [
            ("output", cell.logical_output_bytes),
            ("clone", cell.logical_cloned_bytes),
        ] {
            let mut l = limits();
            if dimension == "output" {
                l.max_output_bytes = Some(exact)
            } else {
                l.max_cloned_bytes = Some(exact)
            }
            ordinary(&artifact, call, &source, 1, l, &work).unwrap();
            loan.admit_element(&source, 1, l, &work, ResourceDemand::default())
                .unwrap();
            if dimension == "output" {
                l.max_output_bytes = Some(exact - 1)
            } else {
                l.max_cloned_bytes = Some(exact - 1)
            }
            let old = ordinary(&artifact, call, &source, 1, l, &work).unwrap_err();
            let new = loan
                .admit_element(&source, 1, l, &work, ResourceDemand::default())
                .unwrap_err();
            let old = format!("{old:?}");
            let new = format!("{new:?}");
            let expected = if dimension == "output" {
                "OutputBytes"
            } else {
                "ClonedBytes"
            };
            assert!(old.contains(expected), "{dimension} ordinary={old}");
            assert!(new.contains(expected), "{dimension} native={new}");
        }
        let mut l = limits();
        l.max_output_elements = Some(0);
        for index in [0, 1, 2, u64::MAX] {
            let old = ordinary(&artifact, call, &source, index, l, &work).unwrap_err();
            let new = loan
                .admit_element(&source, index, l, &work, ResourceDemand::default())
                .unwrap_err();
            assert!(
                format!("{old:?}").contains("OutputElements"),
                "ordinary {old:?}"
            );
            assert!(
                format!("{new:?}").contains("OutputElements"),
                "native {new:?}"
            );
        }
        for index in [0, 2, u64::MAX] {
            let old = ordinary(&artifact, call, &source, index, limits(), &work).unwrap_err();
            let new = loan
                .admit_element(&source, index, limits(), &work, ResourceDemand::default())
                .unwrap_err();
            assert_eq!(
                format!("{new:?}"),
                format!("{old:?}"),
                "same maintained bounds error {index}"
            );
        }
        drop((output, loan, source, work));
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
}

#[test]
fn selected_string_projection_matches_canonical_values_bounds_and_logical_caps() {
    let text = ["", "é", "a\0b", "🧑‍🚀", "e\u{301}\r\n", "last"];
    for shape in ["[string]", "[string]:2,3"] {
        for artifact in variants(shape) {
            let calls = prepared(&artifact);
            let call = calls
                .iter()
                .find(|c| c.supports_string_matrix_element())
                .unwrap();
            let memory = ManagedMemoryBudget::new(64 << 20);
            let work = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &memory).unwrap();
            let rows = if shape == "[string]" { 1 } else { 2 };
            let source = input_source(&artifact, call, &text, rows, 6 / rows, &memory);
            let baseline_bytes = memory.used_bytes();
            let baseline_nodes = memory.retained_snapshot_nodes();
            let loan = call
                .try_string_matrix_key_loan(&source, limits(), &work, ResourceDemand::default())
                .unwrap()
                .unwrap();
            for index in 1..=6 {
                let old = ordinary(&artifact, call, &source, index, limits(), &work).unwrap();
                let new = call
                    .string_matrix_element(
                        &source,
                        index,
                        limits(),
                        &work,
                        ResourceDemand::default(),
                    )
                    .unwrap()
                    .unwrap();
                assert_eq!(string(&old), string(&new));
                assert_eq!(old.schema_key(), new.schema_key());
                assert_eq!(old.shape(), new.shape());
                let ordinal = if rows == 1 {
                    index as usize - 1
                } else {
                    ((index - 1) % rows * (6 / rows) + (index - 1) / rows) as usize
                };
                let cell = loan.cells[ordinal];
                for (dimension, exact) in [
                    ("OutputBytes", cell.logical_output_bytes),
                    ("ClonedBytes", cell.logical_cloned_bytes),
                ] {
                    let mut l = limits();
                    if dimension == "OutputBytes" {
                        l.max_output_bytes = Some(exact);
                    } else {
                        l.max_cloned_bytes = Some(exact);
                    }
                    call.string_matrix_element(&source, index, l, &work, ResourceDemand::default())
                        .unwrap()
                        .unwrap();
                    if dimension == "OutputBytes" {
                        l.max_output_bytes = Some(exact - 1);
                    } else {
                        l.max_cloned_bytes = Some(exact - 1);
                    }
                    let before = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let old_error =
                        ordinary(&artifact, call, &source, index, l, &work).unwrap_err();
                    let new_error = call
                        .string_matrix_element(&source, index, l, &work, ResourceDemand::default())
                        .unwrap_err();
                    assert!(
                        format!("{old_error:?}").contains(dimension),
                        "{old_error:?}"
                    );
                    assert!(
                        format!("{new_error:?}").contains(dimension),
                        "{new_error:?}"
                    );
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        before
                    );
                }
            }
            for index in [0, 7, u64::MAX] {
                let old = ordinary(&artifact, call, &source, index, limits(), &work).unwrap_err();
                let new = call
                    .string_matrix_element(
                        &source,
                        index,
                        limits(),
                        &work,
                        ResourceDemand::default(),
                    )
                    .unwrap_err();
                assert_eq!(format!("{old:?}"), format!("{new:?}"));
            }
            let mut l = limits();
            l.max_output_elements = Some(0);
            for index in [0, 1, 7, u64::MAX] {
                let error = call
                    .string_matrix_element(&source, index, l, &work, ResourceDemand::default())
                    .unwrap_err();
                assert!(format!("{error:?}").contains("OutputElements"), "{error:?}");
            }
            for l in [
                MemoryBudgetLimits {
                    max_compute_work: Some(0),
                    ..limits()
                },
                MemoryBudgetLimits {
                    max_temporary_bytes: Some(0),
                    ..limits()
                },
                MemoryBudgetLimits {
                    max_retained_nodes: Some(0),
                    ..limits()
                },
            ] {
                let before = (memory.used_bytes(), memory.retained_snapshot_nodes());
                assert!(
                    call.string_matrix_element(&source, 1, l, &work, ResourceDemand::default(),)
                        .is_err()
                );
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    before
                );
            }
            let foreign_memory = ManagedMemoryBudget::new(64 << 20);
            let foreign_work =
                NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &foreign_memory).unwrap();
            assert!(matches!(
                call.string_matrix_element(
                    &source,
                    1,
                    limits(),
                    &foreign_work,
                    ResourceDemand::default(),
                ),
                Err(InvokeError::UnadmittedValue)
            ));
            drop(loan);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (baseline_bytes, baseline_nodes)
            );
            drop((source, work));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn loan_rejects_stale_epoch_replaced_source_foreign_account_and_wrong_consumer() {
    for artifact in variants("[string]") {
        let calls = prepared(&artifact);
        let call = calls
            .iter()
            .find(|c| c.supports_string_matrix_key_loan())
            .unwrap();
        let consumer = calls
            .iter()
            .find(|c| c.template.primitive() == Primitive::MapKeyAccess)
            .unwrap();
        let memory = ManagedMemoryBudget::new(64 << 20);
        let work = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &memory).unwrap();
        let source = input_source(&artifact, call, &["old", "new"], 1, 2, &memory);
        let replacement = input_source(&artifact, call, &["old", "new"], 1, 2, &memory);
        let mut loan = call
            .try_string_matrix_key_loan(&source, limits(), &work, ResourceDemand::default())
            .unwrap()
            .unwrap();
        let first = loan
            .admit_element(&source, 1, limits(), &work, ResourceDemand::default())
            .unwrap();
        assert_eq!(
            loan.with_element(consumer, &source, &first, &work, |s| s.to_owned())
                .unwrap(),
            "old"
        );
        // Core's exact-metadata rebind is a clone with the same pinned shape.
        // It is accepted; merely equal shape content is not identity authority.
        let shape_copy = source.shape().clone();
        assert_eq!(source.shape(), &shape_copy);
        assert!(!same_shape_identity(&shape_copy, loan.owner.shape()));
        let foreign_schema_owner = source
            .rebind_with_context(
                source.schema(),
                &shape_copy,
                &SnapshotValidationContext::new(artifact.schemas()),
            )
            .unwrap();
        assert!(foreign_schema_owner.shares_frozen_storage(&source));
        assert!(same_shape_identity(
            foreign_schema_owner.shape(),
            source.shape()
        ));
        assert!(
            loan.native_view(&foreign_schema_owner, &work).is_err(),
            "a new unadmitted schema-owner wrapper grants no authority"
        );
        drop(foreign_schema_owner);
        let shared_schemas = source.schemas().unwrap();
        let rebound = source
            .rebind_with_context(
                source.schema(),
                &shape_copy,
                &SnapshotValidationContext::with_shared_schemas(&shared_schemas),
            )
            .unwrap();
        assert!(rebound.shares_frozen_storage(&source));
        assert!(same_shape_identity(rebound.shape(), source.shape()));
        assert!(loan.native_view(&rebound, &work).is_ok());
        drop(rebound);
        assert!(loan.native_view(&replacement, &work).is_err());
        assert!(
            loan.with_element(consumer, &replacement, &first, &work, |_| ())
                .is_err()
        );
        let foreign = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &memory).unwrap();
        assert!(loan.native_view(&source, &foreign).is_err());
        assert!(
            loan.with_element(call, &source, &first, &work, |_| ())
                .is_err()
        );
        let second = loan
            .admit_element(&source, 2, limits(), &work, ResourceDemand::default())
            .unwrap();
        assert!(
            loan.with_element(consumer, &source, &first, &work, |_| ())
                .is_err()
        );
        assert_eq!(
            loan.with_element(consumer, &source, &second, &work, |s| s.to_owned())
                .unwrap(),
            "new"
        );
        assert!(
            loan.admit_element(&source, 0, limits(), &work, ResourceDemand::default())
                .is_err()
        );
        assert!(
            loan.with_element(consumer, &source, &second, &work, |_| ())
                .is_err(),
            "failure invalidates old producer ticket"
        );
        let third = loan
            .admit_element(&source, 1, limits(), &work, ResourceDemand::default())
            .unwrap();
        let other = call
            .try_string_matrix_key_loan(&source, limits(), &work, ResourceDemand::default())
            .unwrap()
            .unwrap();
        assert_ne!(loan.epoch, other.epoch);
        assert!(
            other
                .with_element(consumer, &source, &third, &work, |_| ())
                .is_err()
        );
        loan.serial = u64::MAX;
        let last = NativeStringMatrixKeyOrdinal {
            loan: loan.epoch,
            serial: u64::MAX,
            ordinal: 0,
        };
        assert!(
            loan.admit_element(&source, 1, limits(), &work, ResourceDemand::default())
                .is_err()
        );
        assert!(
            loan.with_element(consumer, &source, &last, &work, |_| ())
                .is_err()
        );
        assert!(
            loan.native_view(&source, &work).is_err(),
            "ticket exhaustion poisons authority instead of wrapping"
        );
        drop((other, loan, foreign, replacement, source, work));
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
}

#[test]
fn exact_new_policy_work_and_capacity_are_funded_without_refund_or_per_element_owner() {
    for artifact in variants("[string]") {
        let calls = prepared(&artifact);
        let call = calls
            .iter()
            .find(|c| c.supports_string_matrix_key_loan())
            .unwrap();
        let consumer = calls
            .iter()
            .find(|c| c.template.primitive() == Primitive::MapContainsKey)
            .unwrap();
        let memory = ManagedMemoryBudget::new(64 << 20);
        let work = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &memory).unwrap();
        let source = input_source(&artifact, call, &["old"], 1, 1, &memory);
        let mut loan = call
            .try_string_matrix_key_loan(&source, limits(), &work, ResourceDemand::default())
            .unwrap()
            .unwrap();
        let cell = loan.cells[0];
        let exact = loan.selector_compute_work + cell.selection_compute_work;
        let mut l = limits();
        l.max_compute_work = Some(exact);
        let before = work.used();
        let ticket = loan
            .admit_element(&source, 1, l, &work, ResourceDemand::default())
            .unwrap();
        assert_eq!(work.used().0 - before.0, exact);
        let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
        for _ in 0..1000 {
            assert!(
                !loan
                    .with_element(consumer, &source, &ticket, &work, |s| s.is_empty())
                    .unwrap()
            );
        }
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            retained,
            "borrowing creates no per-key Value/owner"
        );
        l.max_compute_work = Some(exact - 1);
        let before = work.used();
        assert!(
            loan.admit_element(&source, 1, l, &work, ResourceDemand::default())
                .is_err()
        );
        assert!(work.used().0 >= before.0);
        assert!(work.used().0 - before.0 < exact);
        let before = work.used();
        loan.admit_element(&source, 1, limits(), &work, ResourceDemand::default())
            .unwrap();
        assert_eq!(
            work.used().0 - before.0,
            exact,
            "retry has fresh debit, no refunded prefix"
        );
        let required_bytes = source.memory_budget_retained_bytes(&memory).unwrap()
            + loan.metadata_bytes
            + cell.logical_temporary_bytes;
        let required_nodes = source.memory_budget_retained_nodes(&memory).unwrap()
            + loan.metadata_nodes
            + cell.logical_extra_nodes;
        for (dimension, required) in [("bytes", required_bytes), ("nodes", required_nodes)] {
            let mut l = limits();
            if dimension == "bytes" {
                l.max_temporary_bytes = Some(required)
            } else {
                l.max_retained_nodes = Some(required)
            }
            loan.admit_element(&source, 1, l, &work, ResourceDemand::default())
                .unwrap();
            if dimension == "bytes" {
                l.max_temporary_bytes = Some(required - 1)
            } else {
                l.max_retained_nodes = Some(required - 1)
            }
            assert!(
                loan.admit_element(&source, 1, l, &work, ResourceDemand::default())
                    .is_err(),
                "{dimension} one short"
            );
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                retained
            );
        }
        let mut l = limits();
        l.max_comparison_work = Some(0);
        let mut prior = ResourceDemand::default();
        prior.work.comparison = 1;
        assert!(
            loan.admit_element(&source, 1, l, &work, prior).is_err(),
            "prior comparison demand is not waived"
        );
        let mut l = limits();
        l.max_cloned_bytes = Some(cell.logical_cloned_bytes);
        let prior = ResourceDemand {
            cloned_bytes: 1,
            ..ResourceDemand::default()
        };
        assert!(
            loan.admit_element(&source, 1, l, &work, prior).is_err(),
            "caller clone demand remains in the selected sink"
        );
        let mut l = limits();
        l.max_output_elements = Some(1);
        let prior = ResourceDemand {
            output_elements: 1,
            ..ResourceDemand::default()
        };
        assert!(
            loan.admit_element(&source, 0, l, &work, prior).is_err(),
            "combined output population is checked before bounds"
        );
        drop((loan, source, work));
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
}

#[test]
fn optional_metadata_headroom_declines_atomically_and_empty_source_has_original_bounds() {
    for artifact in variants("[string]") {
        let calls = prepared(&artifact);
        let call = calls
            .iter()
            .find(|c| c.supports_string_matrix_key_loan())
            .unwrap();
        let memory = ManagedMemoryBudget::new(64 << 20);
        let work = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &memory).unwrap();
        let source = input_source(&artifact, call, &["old", "é"], 1, 2, &memory);
        let before = (
            memory.used_bytes(),
            memory.retained_snapshot_nodes(),
            work.used(),
        );
        let mut l = limits();
        l.max_temporary_bytes = Some(source.memory_budget_retained_bytes(&memory).unwrap());
        assert!(
            call.try_string_matrix_key_loan(&source, l, &work, ResourceDemand::default())
                .unwrap()
                .is_none()
        );
        assert_eq!(
            (
                memory.used_bytes(),
                memory.retained_snapshot_nodes(),
                work.used()
            ),
            before
        );
        let ballast = memory
            .reserve_capacity(memory.limit_bytes() - memory.used_bytes())
            .unwrap();
        assert!(
            call.try_string_matrix_key_loan(&source, limits(), &work, ResourceDemand::default())
                .unwrap()
                .is_none()
        );
        drop(ballast);
        assert_eq!(
            (
                memory.used_bytes(),
                memory.retained_snapshot_nodes(),
                work.used()
            ),
            before
        );
        let empty = input_source(&artifact, call, &[], 1, 0, &memory);
        let mut loan = call
            .try_string_matrix_key_loan(&empty, limits(), &work, ResourceDemand::default())
            .unwrap()
            .unwrap();
        assert_eq!(loan.native_view(&empty, &work).unwrap().count, 0);
        let old = ordinary(&artifact, call, &empty, 1, limits(), &work).unwrap_err();
        let new = loan
            .admit_element(&empty, 1, limits(), &work, ResourceDemand::default())
            .unwrap_err();
        assert_eq!(format!("{new:?}"), format!("{old:?}"));
        drop((loan, empty, source, work));
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
}

#[test]
fn setup_exact_finite_limits_reject_before_authority_and_keep_paid_prefix_on_retry() {
    for artifact in variants("[string]") {
        let calls = prepared(&artifact);
        let call = calls
            .iter()
            .find(|c| c.supports_string_matrix_key_loan())
            .unwrap();
        let consumer = calls
            .iter()
            .find(|c| c.template.primitive() == Primitive::MapContainsKey)
            .unwrap();
        let memory = ManagedMemoryBudget::new(64 << 20);
        let work = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &memory).unwrap();
        let source = input_source(&artifact, call, &["old"], 1, 1, &memory);
        let baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
        let before = work.used();
        let loan = call
            .try_string_matrix_key_loan(&source, limits(), &work, ResourceDemand::default())
            .unwrap()
            .unwrap();
        let setup_work = work.used().0 - before.0;
        let metadata = (loan.metadata_bytes, loan.metadata_nodes);
        drop(loan);
        assert!(setup_work > 0);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            baseline
        );
        let mut l = limits();
        l.max_compute_work = Some(setup_work);
        let before = work.used();
        let exact = call
            .try_string_matrix_key_loan(&source, l, &work, ResourceDemand::default())
            .unwrap()
            .unwrap();
        assert_eq!(work.used().0 - before.0, setup_work);
        drop(exact);
        l.max_compute_work = Some(setup_work - 1);
        let before = work.used();
        assert!(
            call.try_string_matrix_key_loan(&source, l, &work, ResourceDemand::default())
                .is_err()
        );
        assert!(work.used().0 >= before.0);
        assert!(work.used().0 - before.0 < setup_work);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            baseline
        );
        for dimension in ["bytes", "nodes"] {
            let mut l = limits();
            let required = if dimension == "bytes" {
                source.memory_budget_retained_bytes(&memory).unwrap() + metadata.0
            } else {
                source.memory_budget_retained_nodes(&memory).unwrap() + metadata.1
            };
            if dimension == "bytes" {
                l.max_temporary_bytes = Some(required)
            } else {
                l.max_retained_nodes = Some(required)
            }
            let loan = call
                .try_string_matrix_key_loan(&source, l, &work, ResourceDemand::default())
                .unwrap()
                .unwrap();
            drop(loan);
            if dimension == "bytes" {
                l.max_temporary_bytes = Some(required - 1)
            } else {
                l.max_retained_nodes = Some(required - 1)
            }
            let before = work.used();
            assert!(
                call.try_string_matrix_key_loan(&source, l, &work, ResourceDemand::default())
                    .unwrap()
                    .is_none()
            );
            assert_eq!(work.used(), before);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                baseline
            );
        }
        assert!(!consumer.supports_string_matrix_key_loan());
        assert!(
            consumer
                .try_string_matrix_key_loan(&source, limits(), &work, ResourceDemand::default())
                .unwrap()
                .is_none()
        );
        let mut recovered = call
            .try_string_matrix_key_loan(&source, limits(), &work, ResourceDemand::default())
            .unwrap()
            .unwrap();
        let selected = recovered
            .admit_element(&source, 1, limits(), &work, ResourceDemand::default())
            .unwrap();
        assert_eq!(
            recovered
                .with_element(consumer, &source, &selected, &work, |s| s.to_owned())
                .unwrap(),
            "old"
        );
        drop((recovered, source, work));
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
    let exhausted = AtomicU64::new(u64::MAX);
    assert_eq!(next_epoch(&exhausted), None);
    assert_eq!(exhausted.load(Ordering::Relaxed), u64::MAX);
    let last = AtomicU64::new(u64::MAX - 1);
    assert_eq!(next_epoch(&last), Some(u64::MAX - 1));
    assert_eq!(next_epoch(&last), None);
    assert!(
        allocate_cells(usize::MAX).is_none(),
        "unrepresentable fixed table never allocates"
    );
}
