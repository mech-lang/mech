//! Private EnumPack retains the maintained logical output envelope even though
//! its payload remains in private lanes instead of a canonical copied tree.
use crate as engine;
use engine::native_artifact::{
    NativeBufferFrameField as Field, NativeBufferFrameLane as Lane,
    NativeBufferFrameOperation as Operation, NativeIterationAction as Action,
    NativeLibraryWorkBudget, NativeScalarKind, lower_native_program_artifact,
    prepare_native_program_library_call,
};
use mech_core::snapshot::{MapEntryDraft, SnapshotValidationContext};
use mech_core::{ManagedMemoryBudget, SchemaBody, Value, ValueDataDraft as Data, ValueDraft};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
use std::sync::Arc;

const SOURCE: &str = r#"
+> map
#Bounds(initial<{string:u64}>, limit<u64>) => <u64>
  | :Next(index<{string:u64}>, count<u64>)
  | :Done(index<{string:u64}>).
#Bounds(initial, limit) -> :Next(initial, 0u64)
  :Next(index, count)
    | count < limit -> :Next(index, count + 1u64)
    | * -> :Done(index)
  :Done(index) => index{"needle"}.
#Bounds(initial<{string:u64}>, limit<u64>)
"#;

fn sample(body: &SchemaBody, entries: usize, key_len: usize) -> Data {
    match body {
        SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64) => Data::U64(7),
        SchemaBody::Map { .. } => Data::Map(
            (0..entries)
                .map(|i| MapEntryDraft {
                    items: [
                        Data::String(format!("{i:04}{}", "x".repeat(key_len)).into()),
                        Data::U64(i as u64 + 11),
                    ]
                    .into(),
                })
                .collect(),
        ),
        SchemaBody::Tuple(items) => Data::Tuple(
            items
                .iter()
                .map(|item| sample(item, entries, key_len))
                .collect(),
        ),
        _ => panic!("unexpected test payload {body:?}"),
    }
}

fn admitted(
    artifact: &engine::ProgramArtifact,
    schema: mech_core::SchemaId,
    data: Data,
    memory: &ManagedMemoryBudget,
) -> Value {
    let value = ValueDraft {
        schema,
        shape_values: Box::new([]),
        data,
    }
    .finalize(&SnapshotValidationContext::new(artifact.schemas()))
    .unwrap();
    let mut reservation = memory
        .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
        .unwrap();
    value.into_memory_budget(&mut reservation).unwrap()
}

#[test]
fn enum_logical_output_limit_matches_selected_constructor_for_map_and_private_tuple() {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x454e554d), Revision(0), SOURCE).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let mut builder = mech_core::FunctionCatalogBuilder::new();
    engine::install_intrinsic_runtime(&mut builder).unwrap();
    engine::install_intrinsic_compiler_runtime(&mut builder).unwrap();
    engine::install_intrinsic_resident(&mut builder).unwrap();
    engine::install_intrinsic_source(&mut builder).unwrap();
    let catalog = Arc::new(builder.build().unwrap());
    let original = engine::CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            catalog.clone(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap();
    let decoded = engine::decode_program_artifact_bytecode_v1(
        &engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    for artifact in [original, decoded] {
        let plan = lower_native_program_artifact(&artifact, &catalog).unwrap();
        let frames = plan.facts().buffer_frames();
        let mut kinds = std::collections::BTreeSet::new();
        for (site, block) in plan.blocks().iter().enumerate() {
            let Some(action) = frames.action(site) else {
                continue;
            };
            let Operation::EnumPack {
                payload, ordinal, ..
            } = action.operation()
            else {
                continue;
            };
            let Action::LibraryCall {
                template, operands, ..
            } = block.action()
            else {
                panic!()
            };
            let mut call =
                prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
            call.prepare_buffer_frame_composition(&frames, action.operation(), operands)
                .unwrap();
            let mech_core::ResolvedOperationContract::Declared(contract) = template.contract()
            else {
                panic!()
            };
            for (entries, key_len) in [(0, 0), (1, 1), (4, 3), (4, 129)] {
                let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
                let canonical_payload = admitted(
                    &artifact,
                    contract.inputs[1].schema,
                    sample(template.inputs()[1].body(), entries, key_len),
                    &memory,
                );
                let selector = admitted(
                    &artifact,
                    contract.inputs[0].schema,
                    Data::Index(u64::from(*ordinal) + 1),
                    &memory,
                );
                let virtual_payload = frames.fields(*payload);
                kinds.insert(virtual_payload.is_some());
                let mut leaves = Vec::<(Field, Option<Value>, u64)>::new();
                fn collect(
                    fields: &[Field],
                    frames: &engine::native_artifact::NativeBufferFrames,
                    artifact: &engine::ProgramArtifact,
                    memory: &ManagedMemoryBudget,
                    leaves: &mut Vec<(Field, Option<Value>, u64)>,
                    entries: usize,
                    key_len: usize,
                ) {
                    for field in fields {
                        match field.lane() {
                            Lane::Composite { .. } => collect(
                                frames.children(*field).unwrap(),
                                frames,
                                artifact,
                                memory,
                                leaves,
                                entries,
                                key_len,
                            ),
                            Lane::Scalar {
                                kind: NativeScalarKind::U64,
                                ..
                            } => leaves.push((*field, None, 7)),
                            Lane::Managed { .. } => leaves.push((
                                *field,
                                Some(admitted(
                                    artifact,
                                    field.schema(),
                                    sample(
                                        artifact.schemas().get(field.schema()).unwrap().body(),
                                        entries,
                                        key_len,
                                    ),
                                    memory,
                                )),
                                0,
                            )),
                            other => panic!("unexpected test lane {other:?}"),
                        }
                    }
                }
                if let Some(fields) = virtual_payload {
                    collect(
                        fields,
                        &frames,
                        &artifact,
                        &memory,
                        &mut leaves,
                        entries,
                        key_len,
                    );
                }
                let ordinary = [Some(selector.clone()), Some(canonical_payload.clone())];
                let private = [
                    Some(selector.clone()),
                    virtual_payload.is_none().then(|| canonical_payload.clone()),
                ];
                // This is the maintained enum_pack output_bytes formula: root
                // wrapper + enum slot + child clone slot + child logical footprint.
                let mut data_bytes = 0u64;
                mech_core::snapshot::visit_canonical_data_work(
                    template.inputs()[1].body(),
                    canonical_payload.data(),
                    |chunk| -> Result<(), ()> {
                        data_bytes = data_bytes.checked_add(chunk.retained_bytes).unwrap();
                        Ok(())
                    },
                )
                .unwrap();
                let measured_payload = core::mem::size_of::<Value>() as u64 + data_bytes;
                let bound =
                    mech_core::snapshot::CompositeSnapshotConstructor::value_container_bytes(0)
                        .unwrap() as u64
                        + 2 * core::mem::size_of::<mech_core::ValueData>() as u64
                        + measured_payload;
                println!(
                    "enum-site={site} entries={entries} key_len={key_len} private={} sizeof-value={} sizeof-data={} raw-payload={} visitor-payload={} expected-bound={bound}",
                    virtual_payload.is_some(),
                    core::mem::size_of::<Value>(),
                    core::mem::size_of::<mech_core::ValueData>(),
                    canonical_payload
                        .retained_footprint(artifact.schemas())
                        .unwrap()
                        .retained_bytes,
                    measured_payload
                );
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                for (limit, accepts) in [(bound - 1, false), (bound, true), (bound + 1, true)] {
                    let mut limits = mech_core::TargetMemoryProfile::current_resident_cpu()
                        .unwrap()
                        .limits;
                    limits.max_compute_work = Some(u64::MAX);
                    limits.max_comparison_work = Some(u64::MAX);
                    limits.max_output_bytes = Some(limit);
                    limits.max_output_elements = Some(u64::MAX);
                    limits.max_temporary_bytes = Some(256 * 1024 * 1024);
                    let work = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &memory).unwrap();
                    let mut output = None;
                    let canonical = call.invoke_current(&ordinary, &mut output, limits, &work);
                    let owner_work =
                        NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &memory).unwrap();
                    let owner = call.admit_map_owner_constructor(
                        entries as u64,
                        (entries * (4 + key_len)) as u64,
                        limits,
                        &owner_work,
                        mech_core::ResourceDemand::default(),
                    );
                    assert_eq!(
                        owner.is_ok(),
                        canonical.is_ok(),
                        "owner constructor differs from maintained EnumPack: owner={owner:?}, ordinary={canonical:?}"
                    );
                    if let (Err(owner), Err(canonical)) = (&owner, &canonical) {
                        assert_eq!(
                            owner, canonical,
                            "EnumPack must preserve its maintained denial"
                        );
                    }
                    drop(owner_work);
                    assert_eq!(
                        canonical.is_ok(),
                        accepts,
                        "canonical enum site {site} limit {limit}: {canonical:?}"
                    );
                    drop(output);
                    drop(work);
                    let work = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &memory).unwrap();
                    let result = call.admit_buffer_frame_composition(
                        &private,
                        &[0, 0],
                        |field| {
                            leaves
                                .iter()
                                .find(|(f, _, _)| {
                                    f.schema() == field.schema() && f.lane() == field.lane()
                                })
                                .map(|(_, value, word)| (value.as_ref(), *word))
                        },
                        None,
                        limits,
                        &work,
                        Default::default(),
                    );
                    assert_eq!(
                        result.is_ok(),
                        accepts,
                        "private enum site {site} limit {limit}: {result:?}"
                    );
                    drop(work);
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                }
                drop(leaves);
                drop(private);
                drop(ordinary);
                drop(selector);
                drop(canonical_payload);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
        assert_eq!(kinds, [false, true].into_iter().collect());
    }
}

#[test]
fn owner_composite_caps_match_the_maintained_constructor() {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x4d415043), Revision(0), SOURCE).unwrap(),
        ParseConfig::default(),
    );
    let mut builder = mech_core::FunctionCatalogBuilder::new();
    engine::install_intrinsic_runtime(&mut builder).unwrap();
    engine::install_intrinsic_compiler_runtime(&mut builder).unwrap();
    engine::install_intrinsic_resident(&mut builder).unwrap();
    engine::install_intrinsic_source(&mut builder).unwrap();
    let catalog = Arc::new(builder.build().unwrap());
    let original = engine::CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            catalog.clone(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap();
    let decoded = engine::decode_program_artifact_bytecode_v1(
        &engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    let mut checked = 0;
    for artifact in [original, decoded] {
        let plan = lower_native_program_artifact(&artifact, &catalog).unwrap();
        for block in plan.blocks() {
            let Action::LibraryCall { template, .. } = block.action() else {
                continue;
            };
            if !matches!(
                template.primitive(),
                engine::native_artifact::NativeCollectionPrimitive::CompositePack(2)
            ) || !matches!(template.inputs()[0].body(), SchemaBody::Map { .. })
                || template.inputs()[1].body()
                    != &SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64)
            {
                continue;
            }
            let call = prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
            let mech_core::ResolvedOperationContract::Declared(contract) = template.contract()
            else {
                panic!()
            };
            for (entries, key_len) in [(0, 0), (1, 1), (4, 129)] {
                let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
                let values = template
                    .inputs()
                    .iter()
                    .zip(&contract.inputs)
                    .map(|(schema, port)| {
                        Some(admitted(
                            &artifact,
                            port.schema,
                            sample(schema.body(), entries, key_len),
                            &memory,
                        ))
                    })
                    .collect::<Vec<_>>();
                let child_bytes = values
                    .iter()
                    .map(|value| {
                        let value = value.as_ref().unwrap();
                        let mut bytes = core::mem::size_of::<Value>() as u64;
                        mech_core::snapshot::visit_canonical_data_work(
                            artifact.schemas().get(value.schema()).unwrap().body(),
                            value.data(),
                            |chunk| -> Result<(), ()> {
                                bytes += chunk.retained_bytes;
                                Ok(())
                            },
                        )
                        .unwrap();
                        bytes + core::mem::size_of::<mech_core::ValueData>() as u64
                    })
                    .sum::<u64>();
                let output_bytes = child_bytes
                    + mech_core::snapshot::CompositeSnapshotConstructor::value_container_bytes(0)
                        .unwrap() as u64;
                let normalization =
                    mech_core::snapshot::CompositeSnapshotConstructor::binding_cost_for_inputs(
                        contract.outputs[0].schema,
                        contract.inputs.len(),
                        |i| Some(contract.inputs[i].schema),
                        artifact.schemas(),
                    )
                    .unwrap();
                for kind in ["elements", "output", "clone"] {
                    let bound = match kind {
                        "elements" => 2,
                        "output" => output_bytes,
                        _ => child_bytes + normalization.temporary_bytes,
                    };
                    for (limit, accepts) in [(bound - 1, false), (bound, true)] {
                        let mut limits = mech_core::TargetMemoryProfile::current_resident_cpu()
                            .unwrap()
                            .limits;
                        limits.max_compute_work = Some(u64::MAX);
                        limits.max_comparison_work = Some(u64::MAX);
                        limits.max_temporary_bytes = Some(256 * 1024 * 1024);
                        limits.max_output_bytes = Some(u64::MAX);
                        limits.max_cloned_bytes = Some(u64::MAX);
                        limits.max_output_elements = Some(u64::MAX);
                        match kind {
                            "elements" => limits.max_output_elements = Some(limit),
                            "output" => limits.max_output_bytes = Some(limit),
                            _ => limits.max_cloned_bytes = Some(limit),
                        };
                        let work =
                            NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &memory).unwrap();
                        let mut output = None;
                        let canonical = call.invoke_current(&values, &mut output, limits, &work);
                        let owner = call.admit_map_owner_constructor(
                            entries as u64,
                            (entries * (4 + key_len)) as u64,
                            limits,
                            &work,
                            mech_core::ResourceDemand::default(),
                        );
                        assert_eq!(
                            canonical.is_ok(),
                            accepts,
                            "canonical {kind} bound={bound} limit={limit}: {canonical:?}"
                        );
                        assert_eq!(
                            owner.is_ok(),
                            canonical.is_ok(),
                            "{kind}: owner={owner:?} canonical={canonical:?}"
                        );
                        if let (Err(owner), Err(canonical)) = (&owner, &canonical) {
                            assert_eq!(
                                owner, canonical,
                                "{kind} must retain the maintained error variant/order"
                            );
                        }
                        checked += 1;
                    }
                }
            }
        }
    }
    assert!(checked >= 36);
}
