//! Current snapshot tapes and genuine dense resident output carriers.
use super::*;

fn variants(source: &str) -> (Arc<FunctionCatalog>, [crate::ProgramArtifact; 2]) {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x545045), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    let mut builder = FunctionCatalogBuilder::new();
    crate::install_intrinsic_resident(&mut builder).unwrap();
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
    (catalog, [original, decoded])
}
fn input(
    artifact: &crate::ProgramArtifact,
    data: ValueDataDraft,
    extents: &[u64],
    memory: &ManagedMemoryBudget,
) -> Value {
    assert_eq!(artifact.inputs().len(), 1);
    let input = &artifact.inputs()[0];
    let shape =
        shape_for_resolved_extents(artifact.schemas().get(input.schema).unwrap(), extents).unwrap();
    let value = ValueDraft {
        schema: input.schema,
        shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
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
fn fixed_matrix_source_pays_positive_target_tape_before_exact_output_boundary() {
    for values in [vec![7u8], vec![7u8, 8, 9], (0u8..128).collect::<Vec<_>>()] {
        let source = format!(
            "unique<{{u8}}> := values<[u8]:1,{}>\nunique\n",
            values.len()
        );
        let (catalog, artifacts) = variants(&source);
        for artifact in artifacts {
            let plan = lower_native_program_artifact(&artifact, &catalog).unwrap();
            let template = plan
                .blocks()
                .iter()
                .find_map(|block| match block.action() {
                    NativeIterationAction::LibraryCall { template, .. }
                        if template.primitive()
                            == NativeCollectionPrimitive::MatrixToSetConversion =>
                    {
                        Some(template)
                    }
                    _ => None,
                })
                .unwrap();
            assert_eq!(template.inputs()[0].dimension_parameters().len(), 0);
            assert_eq!(template.output().dimension_parameters().len(), 1);
            let prepared =
                prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let value = input(
                &artifact,
                ValueDataDraft::Matrix(values.iter().copied().map(ValueDataDraft::U8).collect()),
                &[1, values.len() as u64],
                &memory,
            );
            let source_bytes = value
                .retained_footprint(artifact.schemas())
                .unwrap()
                .retained_bytes;
            let input = vec![Some(value)];
            let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
            let mut output = None;
            reset();
            prepared
                .invoke_current(&input, &mut output, limits(), &work)
                .unwrap();
            let estimate = seen(0, false).bytes;
            let actual = output
                .as_ref()
                .unwrap()
                .retained_footprint(artifact.schemas())
                .unwrap()
                .retained_bytes;
            assert!(
                estimate >= actual,
                "actual {actual} exceeds pre-admitted {estimate}"
            );
            assert_eq!(
                estimate,
                source_bytes.max((values.len() as u64 + 1) * 32)
                    + values.len() as u64
                        * (size_of::<ValueData>() as u64 - size_of::<u8>() as u64)
                    + size_of::<u64>() as u64
                    + values.len() as u64 * size_of::<CanonicalKeyValue>() as u64
            );
            let prior_fits = values.len() == 128;
            assert_eq!(
                estimate - actual,
                ((values.len() as u64 + 1) * 32).saturating_sub(source_bytes),
                "only genuine existing flat slack remains above actual retention"
            );
            if prior_fits {
                let carrier = TargetMemoryProfile::current_native_host()
                    .unwrap()
                    .primitives
                    .canonical_value_handle
                    .bytes;
                assert!(
                    actual.checked_add(carrier).unwrap() <= estimate - 1,
                    "real prior plus actual native snapshot carrier must fit one-short control"
                );
            }
            // Small cases start unpublished: n1 has E==actual and cannot
            // truthfully hold that same output under the one-short policy.
            let prior = output.as_ref().unwrap().clone();
            let mut denied_output = if prior_fits { output.clone() } else { None };
            let before = memory.used_bytes();
            let mut short = limits();
            short.max_output_bytes = Some(estimate - 1);
            reset();
            let error = prepared
                .invoke_current(&input, &mut denied_output, short, &work)
                .unwrap_err();
            assert_eq!(seen(0, false).bytes, estimate);
            assert!(
                matches!(error, NativeCollectionInvocationError::Kernel(
                ResidentKernelError::ResourceLimit { dimension: MemoryBudgetDimension::OutputBytes,
                    required, limit, .. }) if required == estimate && limit == estimate - 1),
                "{error:?}"
            );
            if prior_fits {
                assert!(
                    denied_output
                        .as_ref()
                        .unwrap()
                        .shares_frozen_storage(&prior)
                );
            } else {
                assert!(denied_output.is_none());
            }
            assert!(output.as_ref().unwrap().shares_frozen_storage(&prior));
            assert_eq!(memory.used_bytes(), before);
            drop(denied_output);
            // Exact E must use the SAME publication state as the one-short
            // attempt. A Some publication seeds an independent carrier+old
            // payload floor; it is not part of the fresh None output estimate.
            let mut exact_output = if prior_fits { output.clone() } else { None };
            let mut exact = limits();
            exact.max_output_bytes = Some(estimate);
            reset();
            prepared
                .invoke_current(&input, &mut exact_output, exact, &work)
                .unwrap();
            assert_eq!(seen(0, false).bytes, estimate);
            assert!(
                exact_output
                    .as_ref()
                    .unwrap()
                    .snapshot_eq(artifact.schemas(), &prior, artifact.schemas())
                    .unwrap()
            );
            assert!(output.as_ref().unwrap().shares_frozen_storage(&prior));
            drop(exact_output);
            reset();
            prepared
                .invoke_current(&input, &mut output, limits(), &work)
                .unwrap();
            assert_eq!(seen(0, false).bytes, estimate);
            drop((input, output, prior, work));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn actual_dense_resident_input_funds_its_owned_value_and_shape_carrier() {
    use crate::resident::{
        ActivationFacts, CapturedValueInput, ResidentActivationOptions, activate_with_options,
    };
    // Long distinct Strings force real payload+carrier bytes above 32*D,
    // so the old flat slack cannot hide an unfunded String-header credit.
    let texts = ["é".repeat(64), "é".repeat(64)];
    let string_bytes = texts.iter().map(|s| s.len() as u64).sum::<u64>()
        + texts.len() as u64 * size_of::<String>() as u64;
    let cases = [
        (
            "unique<{f64}> := values<[f64]:1,1>\nunique\n",
            vec![ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_f64(
                7.0,
            ))],
            16,
            size_of::<f64>() as u64,
            false,
        ),
        (
            "unique<{string}> := values<[string]:1,2>\nunique\n",
            texts.into_iter().map(ValueDataDraft::String).collect(),
            string_bytes,
            size_of::<String>() as u64,
            true,
        ),
    ];
    for (source, values, source_bytes, source_width, exact_footprint) in cases {
        let n = values.len() as u64;
        let (catalog, artifacts) = variants(source);
        for artifact in artifacts {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let input = input(
                &artifact,
                ValueDataDraft::Matrix(values.clone().into_boxed_slice()),
                &[1, n],
                &memory,
            );
            let mut instance = activate_with_options(
                ReactiveInstanceId::new(0x545045, 0),
                &artifact,
                &catalog,
                &ActivationFacts::default(),
                ResidentActivationOptions {
                    memory_budget: Some(memory.clone()),
                    ..ResidentActivationOptions::default()
                },
            )
            .unwrap();
            assert_eq!(instance.plan.inputs.len(), 1);
            {
                let captured = [CapturedValueInput {
                    slot: instance.plan.inputs[0].slot,
                    value: &input,
                }];
                reset();
                instance
                    .prepare_turn_values(&captured)
                    .unwrap()
                    .publish()
                    .unwrap();
            }
            let estimate = seen(0, true).bytes;
            let output = instance.copied_output(0).unwrap();
            let actual = output
                .retained_footprint(artifact.schemas())
                .unwrap()
                .retained_bytes;
            assert!(
                estimate >= actual,
                "dense source has no owned Value/tape baseline: {estimate} < {actual}"
            );
            let flat = source_bytes.max((n + 1) * 32);
            let header_gap = (size_of::<ValueData>() as u64).saturating_sub(flat - source_bytes);
            assert_eq!(
                estimate,
                flat + size_of::<Value>() as u64
                    + size_of::<u64>() as u64
                    + header_gap
                    + n * (size_of::<ValueData>() as u64).saturating_sub(source_width)
                    + n * size_of::<CanonicalKeyValue>() as u64
            );
            if exact_footprint {
                assert!(source_bytes > (n + 1) * 32);
                assert_eq!(
                    estimate, actual,
                    "dense String carrier and key data must both be funded"
                );
            }
            drop((output, instance, input));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}
