#![cfg(all(feature = "full_compiler", feature = "native-artifact"))]
use mech_core::{FunctionCatalogBuilder, NodeId};
use mech_engine::native_artifact::{
    lower_native_program_artifact, native_collection_operation_template,
};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn artifacts(source: &str) -> [ProgramArtifact; 2] {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x4d415045), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let document = DocumentSyntax::cast(parsed.syntax()).unwrap();
    let source = CanonicalSourceFrontend
        .compile_document_with_catalog(&document, mech_stdlib::source_catalog())
        .unwrap();
    let original = source.compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    assert_eq!(
        bytes,
        mech_engine::encode_program_artifact_bytecode_v1(&decoded).unwrap()
    );
    [original, decoded]
}

const ENTRIES: &str = "+> map\nempty<{string:u64}> := map/new()\nnext := map/insert(empty, \"é\", 3u64)\n\"é\" ∈ next\n";

#[test]
fn native_map_entry_templates_keep_exact_identity_arity_and_private_factory_authority() {
    let catalog = mech_stdlib::source_catalog();
    let cloned = FunctionCatalogBuilder::from_catalog(&catalog)
        .build()
        .unwrap();
    for artifact in artifacts(ENTRIES) {
        for (index, node) in artifact.nodes().iter().enumerate() {
            let Some(op) = node.as_operation() else {
                continue;
            };
            let name = op.operation.canonical_name();
            let arity = match name.as_str() {
                "map/new" => 0,
                "map/insert" => 3,
                "map/contains-key" => 2,
                _ => continue,
            };
            for selected in [catalog.as_ref(), &cloned] {
                let template = native_collection_operation_template(
                    &artifact,
                    NodeId::new(index as u32),
                    selected,
                )
                .unwrap();
                assert_eq!(template.primitive().name(), name);
                assert_eq!(template.inputs().len(), arity);
                assert_eq!(template.primitive().arity(), arity);
                let entry = selected
                    .resident_factory(&op.operation.module_path, &op.operation.operation_name)
                    .unwrap();
                let mut changed = FunctionCatalogBuilder::new();
                changed
                    .insert_resident_factory_entry(
                        entry
                            .clone()
                            .with_implementation_memory(entry.implementation_memory()),
                    )
                    .unwrap();
                assert!(
                    native_collection_operation_template(
                        &artifact,
                        NodeId::new(index as u32),
                        &changed.build().unwrap()
                    )
                    .is_err()
                );
            }
        }
    }
}

#[test]
fn native_map_entry_program_lowering_keeps_library_operations_and_scalar_arithmetic() {
    use mech_engine::native_artifact::{NativeIterationAction, NativeMatchAction, NativeScalarOp};
    let source = ENTRIES.replace("3u64)", "(2u64 + 1u64))");
    for artifact in artifacts(&source) {
        let plan =
            lower_native_program_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        plan.validate_current_visits().unwrap();
        let mut operations = Vec::new();
        let mut scalar_add = false;
        for block in plan.blocks() {
            match block.action() {
                NativeIterationAction::LibraryCall { template, .. } => {
                    operations.push(template.primitive().name())
                }
                NativeIterationAction::Scalar(NativeMatchAction::Primitive {
                    op: NativeScalarOp::U64Add,
                    ..
                }) => scalar_add = true,
                _ => {}
            }
        }
        operations.sort();
        assert_eq!(operations, ["map/contains-key", "map/insert", "map/new"]);
        assert!(
            scalar_add,
            "entry arithmetic must remain a native scalar operation"
        );
    }
}

const COUNT: &str = r#"
+> map
increment(h<{string:u64}>, key<string>) => (out<{string:u64}>) :=
  previous := (key ∈ h)? | true => h{key} | false => 0u64.
  out := map/insert(h, key, previous + 1u64).
#Count(s<string>, width<u64>, n<u64>, initial<{string:u64}>) => <{string:u64}>
  | :Next(index<u64>, counts<{string:u64}>)
  | :Done(counts<{string:u64}>).
#Count(s, width, n, initial) -> :Next(0u64, initial)
  :Next(index, counts)
    | width == 0u64 -> :Done(counts)
    | index >= n -> :Done(counts)
    | width > n - index -> :Done(counts)
    | * -> :Next(index + 1u64, increment(counts, s[(index + 1u64)..=(index + width)]))
  :Done(counts) => counts.
empty<{string:u64}> := map/new()
"#;

#[test]
fn native_counting_state_machine_lowers_all_seven_widths() {
    for width in [1, 2, 3, 4, 6, 12, 18] {
        let source =
            format!("{COUNT}\n#Count(\"ACGTACGTNACGTTACGTACGTACGT\", {width}u64, 26u64, empty)\n");
        for artifact in artifacts(&source) {
            let plan = lower_native_program_artifact(&artifact, &mech_stdlib::source_catalog())
                .unwrap_or_else(|error| panic!("width {width}: {error:?}"));
            plan.validate_current_visits().unwrap();
        }
    }
}

#[test]
fn native_map_leaf_calls_publish_canonical_values_and_preserve_output_on_denial() {
    use mech_core::{
        ManagedMemoryBudget, ResolvedOperationContract, ValueData, ValueDataDraft, ValueDraft,
    };
    use mech_engine::native_artifact::{
        NativeIterationAction, NativeLibraryWorkBudget, prepare_native_program_library_call,
    };
    for artifact in artifacts(&format!("{ENTRIES}pair := (7u64, next)\n")) {
        let catalog = mech_stdlib::source_catalog();
        let plan = lower_native_program_artifact(&artifact, &catalog).unwrap();
        let selected = |name: &str| {
            plan.blocks()
                .iter()
                .find_map(|block| match block.action() {
                    NativeIterationAction::LibraryCall { template, .. }
                        if template.primitive().name() == name =>
                    {
                        Some(template)
                    }
                    _ => None,
                })
                .unwrap()
        };
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
        let mut limits = mech_core::TargetMemoryProfile::current_resident_cpu()
            .unwrap()
            .limits;
        limits.max_compute_work = Some(128 * 1024 * 1024);
        limits.max_temporary_bytes = Some(128 * 1024 * 1024);
        let new =
            prepare_native_program_library_call(&artifact, selected("map/new"), &catalog).unwrap();
        let mut empty = None;
        new.invoke_current(&[], &mut empty, limits, &work).unwrap();
        assert!(
            matches!(empty.as_ref().unwrap().data(), ValueData::Map(entries) if entries.entries().is_empty())
        );
        let insert_template = selected("map/insert");
        let ResolvedOperationContract::Declared(contract) = insert_template.contract() else {
            panic!("declared map insertion")
        };
        let admitted = |schema, data| {
            let value = ValueDraft {
                schema,
                shape_values: Box::new([]),
                data,
            }
            .finalize(&mech_core::snapshot::SnapshotValidationContext::new(
                artifact.schemas(),
            ))
            .unwrap();
            let mut reservation = memory
                .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
                .unwrap();
            value.into_memory_budget(&mut reservation).unwrap()
        };
        let key = admitted(
            contract.inputs[1].schema,
            ValueDataDraft::String("é".into()),
        );
        let count = admitted(contract.inputs[2].schema, ValueDataDraft::U64(3));
        let insert =
            prepare_native_program_library_call(&artifact, insert_template, &catalog).unwrap();
        let mut next = None;
        insert
            .invoke_current(
                &[empty.clone(), Some(key.clone()), Some(count)],
                &mut next,
                limits,
                &work,
            )
            .unwrap();
        assert!(
            matches!(empty.as_ref().unwrap().data(), ValueData::Map(entries) if entries.entries().is_empty())
        );
        assert!(
            matches!(next.as_ref().unwrap().data(), ValueData::Map(entries) if entries.entries().len() == 1)
        );
        let pack_template = selected("core/composite-pack");
        let ResolvedOperationContract::Declared(pack_contract) = pack_template.contract() else {
            panic!("declared composite")
        };
        let ordinal = admitted(pack_contract.inputs[0].schema, ValueDataDraft::U64(7));
        let pack = prepare_native_program_library_call(&artifact, pack_template, &catalog).unwrap();
        let mut pair = None;
        pack.invoke_current(
            &[Some(ordinal.clone()), next.clone()],
            &mut pair,
            limits,
            &work,
        )
        .unwrap();
        assert!(
            matches!(pair.as_ref().unwrap().data(), ValueData::Tuple(items)
            if matches!(items.as_ref(), [ValueData::U64(7), ValueData::Map(entries)] if entries.entries().len() == 1))
        );
        let prior_pair = pair.as_ref().unwrap().clone();
        let mut denied_pack = limits;
        denied_pack.max_temporary_bytes = Some(0);
        assert!(
            pack.invoke_current(
                &[Some(ordinal), next.clone()],
                &mut pair,
                denied_pack,
                &work
            )
            .is_err()
        );
        assert!(pair.as_ref().unwrap().shares_frozen_storage(&prior_pair));
        let contains =
            prepare_native_program_library_call(&artifact, selected("map/contains-key"), &catalog)
                .unwrap();
        let mut output = None;
        contains
            .invoke_current(
                &[next.clone(), Some(key.clone())],
                &mut output,
                limits,
                &work,
            )
            .unwrap();
        assert!(matches!(
            output.as_ref().unwrap().data(),
            ValueData::Bool(true)
        ));
        contains
            .invoke_current(&[empty, Some(key.clone())], &mut output, limits, &work)
            .unwrap();
        assert!(matches!(
            output.as_ref().unwrap().data(),
            ValueData::Bool(false)
        ));
        let prior = output.as_ref().unwrap().clone();
        let mut denied = limits;
        denied.max_temporary_bytes = Some(0);
        assert!(
            contains
                .invoke_current(
                    &[next.clone(), Some(key.clone())],
                    &mut output,
                    denied,
                    &work
                )
                .is_err()
        );
        assert!(output.as_ref().unwrap().shares_frozen_storage(&prior));
        contains
            .invoke_current(&[next, Some(key)], &mut output, limits, &work)
            .unwrap();
        assert!(matches!(
            output.as_ref().unwrap().data(),
            ValueData::Bool(true)
        ));
    }
}

#[test]
fn native_structural_projection_requires_owned_inputs_and_funds_denial_and_retry() {
    use mech_core::{ManagedMemoryBudget, SchemaBody, ValueData, ValueDataDraft, ValueDraft};
    use mech_engine::native_artifact::{
        NativeIterationAction, NativeLibraryWorkBudget, NativeStructuralObservation,
        native_project_structural_value,
    };
    fn data(body: &SchemaBody) -> ValueDataDraft {
        match body {
            SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64) => ValueDataDraft::U64(0),
            SchemaBody::Tuple(items) => ValueDataDraft::Tuple(items.iter().map(data).collect()),
            SchemaBody::Enum { variants, .. } => {
                ValueDataDraft::Enum(mech_core::snapshot::EnumDraft {
                    ordinal: 0,
                    payload: variants[0]
                        .payload
                        .as_ref()
                        .map(|body| Box::new(data(body))),
                })
            }
            SchemaBody::Map { .. } => ValueDataDraft::Map(
                vec![mech_core::snapshot::MapEntryDraft {
                    items: vec![
                        ValueDataDraft::String("é\0x".into()),
                        ValueDataDraft::U64(17),
                    ]
                    .into_boxed_slice(),
                }]
                .into_boxed_slice(),
            ),
            _ => panic!("unexpected carrier {body:?}"),
        }
    }
    for artifact in artifacts(&format!("{COUNT}\n#Count(\"ACGT\", 1u64, 4u64, empty)\n")) {
        let plan =
            lower_native_program_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let (site, source) = plan
            .blocks()
            .iter()
            .enumerate()
            .find_map(|(site, block)| match block.action() {
                NativeIterationAction::Structural {
                    source,
                    output,
                    observation: NativeStructuralObservation::CanonicalValue,
                    ..
                } if matches!(
                    plan.values()[output.get() as usize].definition().body(),
                    SchemaBody::Map { .. }
                ) =>
                {
                    Some((site, *source))
                }
                _ => None,
            })
            .unwrap();
        let ty = &plan.values()[source.get() as usize];
        let schemas = std::sync::Arc::new(artifact.schemas().clone());
        let memory = ManagedMemoryBudget::new(16 * 1024 * 1024);
        let foreign = ManagedMemoryBudget::new(16 * 1024 * 1024);
        let input = |budget: &ManagedMemoryBudget| {
            let value = ValueDraft {
                schema: ty.schema(),
                shape_values: Box::new([]),
                data: data(ty.definition().body()),
            }
            .finalize(
                &mech_core::snapshot::SnapshotValidationContext::with_shared_schemas(&schemas),
            )
            .unwrap();
            let mut reservation = budget
                .reserve_capacity(value.memory_budget_admission_bytes(budget).unwrap())
                .unwrap();
            value.into_memory_budget(&mut reservation).unwrap()
        };
        let source = input(&memory);
        let foreign_source = input(&foreign);
        let work = NativeLibraryWorkBudget::new(100_000_000, 10_000_000, &memory).unwrap();
        let mut limits = mech_core::TargetMemoryProfile::current_resident_cpu()
            .unwrap()
            .limits;
        limits.max_compute_work = Some(10_000_000);
        limits.max_comparison_work = Some(1_000_000);
        limits.max_temporary_bytes = Some(8 * 1024 * 1024);
        assert!(
            native_project_structural_value(
                &plan,
                site,
                &schemas,
                &foreign_source,
                None,
                limits,
                &work
            )
            .is_err()
        );
        assert_eq!(work.used(), (0, 0));
        let output =
            native_project_structural_value(&plan, site, &schemas, &source, None, limits, &work)
                .unwrap();
        let ValueData::Map(map) = output.data() else {
            panic!("map")
        };
        assert!(
            matches!((map.entries()[0].key().data(), map.entries()[0].value()), (ValueData::String(key), ValueData::U64(17)) if key.as_ref() == "é\0x")
        );
        let retained = memory.used_bytes();
        let original = source.frozen_storage_identity();
        for tight in [0, 1] {
            let mut denied = limits;
            denied.max_output_bytes = Some(tight);
            assert!(
                native_project_structural_value(
                    &plan,
                    site,
                    &schemas,
                    &source,
                    Some(&output),
                    denied,
                    &work
                )
                .is_err()
            );
            assert_eq!(memory.used_bytes(), retained);
            assert!(original.matches(&source));
        }
        let retry = native_project_structural_value(
            &plan,
            site,
            &schemas,
            &source,
            Some(&output),
            limits,
            &work,
        )
        .unwrap();
        assert_eq!(
            retry.canonical_snapshot_bytes(&schemas).unwrap(),
            output.canonical_snapshot_bytes(&schemas).unwrap()
        );
        assert!(work.used().0 > 0 && work.used().1 > 0);
    }
}
