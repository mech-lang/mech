#![cfg(all(feature = "full_compiler", feature = "native-artifact"))]
use mech_core::{
    BoundResidentKernel, FunctionCatalog, FunctionCatalogBuilder, NodeId, ResidentKernelBindError,
    ResidentKernelBindRequest,
};
use mech_engine::native_artifact::{
    NativeArtifactLoweringError, NativeCollectionLexicalSource, NativeCollectionPrimitive,
    bind_native_collection_operation, native_collection_lexical_template,
    native_collection_operation_template,
};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

const SOURCE: &str = r#"+> text
+> sequence
+> number
+> map
words := text/lines(input<string>)
joined := text/join(words, ",")
size := sequence/length(joined)
formatted := number/format(size, 0u64)
counts := map/histogram(words)
keys := map/keys(counts)
values := map/values(counts)
(formatted, keys, values)
"#;

#[test]
fn mixed_native_control_lowering_preserves_all_seven_library_families_and_scalar_work() {
    use mech_engine::native_artifact::{
        NativeIterationAction, NativeMatchAction, NativeScalarOp, lower_native_program_artifact,
    };
    let source = SOURCE.replace("(formatted, keys, values)\n", "more := size + 1u64\n");
    for artifact in artifacts(&source) {
        let plan =
            lower_native_program_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        plan.validate_current_visits().unwrap();
        assert_eq!(plan.program_revision(), artifact.revision());
        let mut names = Vec::new();
        let mut scalar_add = false;
        for block in plan.blocks() {
            match block.action() {
                NativeIterationAction::LibraryCall {
                    template,
                    operands,
                    output,
                } => {
                    assert_eq!(template.origin(), block.origin());
                    assert_eq!(template.program_revision(), artifact.revision());
                    assert!(template.root().is_some());
                    assert!(template.lexical().is_none());
                    assert_eq!(operands.len(), template.inputs().len());
                    assert_eq!(
                        plan.values()[output.get() as usize].definition(),
                        template.output()
                    );
                    names.push(template.primitive().name());
                }
                NativeIterationAction::Scalar(NativeMatchAction::Primitive {
                    op: NativeScalarOp::U64Add,
                    ..
                }) => scalar_add = true,
                _ => {}
            }
        }
        names.sort();
        assert_eq!(
            names,
            [
                "map/histogram",
                "map/keys",
                "map/values",
                "number/format",
                "sequence/length",
                "text/join",
                "text/lines"
            ]
        );
        assert!(scalar_add);
        assert!(plan.values().iter().any(|ty| ty.scalar().is_none()));
        assert!(
            mech_engine::native_artifact::lower_scalar_iteration_artifact(
                &artifact,
                &mech_stdlib::source_catalog()
            )
            .is_err()
        );
    }
}

#[test]
fn mixed_native_control_lowering_owns_managed_loop_carriers_and_lexical_leaf_calls() {
    use mech_engine::native_artifact::{
        NativeIterationAction, NativeIterationValueScope, lower_native_program_artifact,
    };
    let source = "+> sequence\n+> number\nstep(x<string>) => (next<string>) := next := (choose? | true => number/format(sequence/length(x), 0u64) | false => x).\nchoose := flag<bool>\nseed<string> |3> step\n";
    for artifact in artifacts(source) {
        let plan =
            lower_native_program_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        plan.validate_current_visits().unwrap();
        assert_eq!(plan.iterations().len(), 1);
        let iteration = &plan.iterations()[0];
        assert_eq!(iteration.count(), 3);
        assert!(
            plan.values()[iteration.carrier().get() as usize]
                .scalar()
                .is_none()
        );
        assert_eq!(
            plan.value_scopes()[iteration.carrier().get() as usize],
            NativeIterationValueScope::Carrier { iteration: 0 }
        );
        let calls = plan
            .blocks()
            .iter()
            .filter_map(|block| {
                let NativeIterationAction::LibraryCall { template, .. } = block.action() else {
                    return None;
                };
                assert_eq!(block.iteration(), Some(0));
                assert_eq!(template.origin(), block.origin());
                let lexical = template.lexical().expect("actual lexical canonical site");
                assert_eq!(Some(lexical.block().0), block.origin().block());
                assert_eq!(Some(lexical.local()), block.origin().local());
                assert!(template.root().is_none());
                Some(template.primitive().name())
            })
            .collect::<Vec<_>>();
        assert_eq!(calls, ["sequence/length", "number/format"]);
    }
}

#[test]
fn mixed_native_control_lowering_never_uses_replacement_factories_as_authority() {
    use mech_engine::native_artifact::lower_native_program_artifact;
    let maintained = mech_stdlib::source_catalog();
    let original = maintained
        .resident_factory(&["sequence".into()], "length")
        .unwrap();
    let mut builder = FunctionCatalogBuilder::new();
    builder
        .insert_resident_factory_entry(original.clone().with_factory(replacement))
        .unwrap();
    let replaced = builder.build().unwrap();
    for artifact in artifacts("+> sequence\nsequence/length(input<string>)\n") {
        assert!(matches!(
            lower_native_program_artifact(&artifact, &replaced),
            Err(NativeArtifactLoweringError::MissingSemanticWitness { .. })
        ));
        lower_native_program_artifact(&artifact, &maintained)
            .unwrap()
            .validate_current_visits()
            .unwrap();
    }
    assert_eq!(BINDS.load(Ordering::SeqCst), 0);
}

fn artifacts(source: &str) -> [ProgramArtifact; 2] {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x434f4c4c), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let document = DocumentSyntax::cast(parsed.syntax()).unwrap();
    let compiled = CanonicalSourceFrontend
        .compile_document_with_catalog(&document, mech_stdlib::source_catalog())
        .unwrap_or_else(|error| panic!("{source}: {error:?}"));
    let original = compiled.compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    [original, decoded]
}

fn operations(artifact: &ProgramArtifact) -> Vec<(NodeId, String)> {
    artifact
        .nodes()
        .iter()
        .filter_map(|node| {
            let operation = node.as_operation()?;
            Some((
                node.node,
                format!(
                    "{}/{}",
                    operation.operation.module_path.join("/"),
                    operation.operation.operation_name
                ),
            ))
        })
        .collect()
}

#[test]
fn original_and_decoded_collection_templates_retain_actual_schemas_and_selected_implementations() {
    let catalog = mech_stdlib::source_catalog();
    let cloned = FunctionCatalogBuilder::from_catalog(&catalog)
        .build()
        .unwrap();
    let expected = [
        NativeCollectionPrimitive::TextLines,
        NativeCollectionPrimitive::TextJoin,
        NativeCollectionPrimitive::SequenceLength,
        NativeCollectionPrimitive::NumberFormat,
        NativeCollectionPrimitive::StringHistogram,
        NativeCollectionPrimitive::StringMapKeys,
        NativeCollectionPrimitive::StringMapValues,
    ];
    for artifact in artifacts(SOURCE) {
        let operations = operations(&artifact);
        for primitive in expected {
            let (node, _) = operations
                .iter()
                .find(|(_, name)| name == primitive.name())
                .unwrap();
            for catalog in [catalog.as_ref(), &cloned] {
                let template =
                    native_collection_operation_template(&artifact, *node, catalog).unwrap();
                assert_eq!(template.program_revision(), artifact.revision());
                assert_eq!(template.node(), *node);
                assert_eq!(template.primitive(), primitive);
                assert_eq!(template.sources().len(), primitive.arity());
                assert_eq!(template.inputs().len(), primitive.arity());
                assert_eq!(
                    template.selected_key().operation_name,
                    primitive.name().rsplit('/').next().unwrap()
                );
                assert_eq!(
                    template.output().key(),
                    artifact
                        .schemas()
                        .get(artifact.slots()[template.target().get() as usize].schema)
                        .unwrap()
                        .key()
                );
                assert_eq!(
                    template.contract(),
                    artifact
                        .contracts()
                        .get(
                            artifact.nodes()[node.get() as usize]
                                .as_operation()
                                .unwrap()
                                .contract
                        )
                        .unwrap()
                );
            }
        }
        let invalid = NodeId::new(u32::MAX);
        assert!(
            matches!(native_collection_operation_template(&artifact, invalid, &catalog), Err(NativeArtifactLoweringError::InvalidBinding { node: Some(node) }) if node == invalid)
        );
        // A local library proof cannot silently cover the enclosing aggregate
        // constructor or claim the complete program is executable natively.
        let (other, _) = operations
            .iter()
            .find(|(_, name)| !expected.iter().any(|primitive| primitive.name() == name))
            .unwrap();
        assert!(native_collection_operation_template(&artifact, *other, &catalog).is_err());
    }
}

#[test]
fn actual_collection_calls_bind_native_host_handles_and_explicit_finite_policy() {
    let catalog = mech_stdlib::source_catalog();
    let limits = mech_core::TargetMemoryProfile::current_resident_cpu()
        .unwrap()
        .limits;
    for artifact in artifacts(SOURCE) {
        let mut bound = 0;
        for (node, _) in operations(&artifact) {
            let Ok(template) = native_collection_operation_template(&artifact, node, &catalog)
            else {
                continue;
            };
            let parameters = template
                .inputs()
                .iter()
                .map(|schema| vec![2; schema.dimension_parameters().len()])
                .collect::<Vec<_>>();
            let refs = parameters.iter().map(Vec::as_slice).collect::<Vec<_>>();
            let output_parameters = vec![2; template.output().dimension_parameters().len()];
            let call = bind_native_collection_operation(
                &artifact,
                node,
                &catalog,
                &refs,
                &output_parameters,
                limits,
            )
            .unwrap_or_else(|error| panic!("{}: {error:?}", template.primitive().name()));
            bound += 1;
            assert_eq!(call.template().node(), node);
            assert_eq!(call.memory_plan().bound_call, *call.bound_call());
            assert_eq!(
                call.memory_plan().target.kind,
                mech_core::MemoryTargetKind::NativeHost
            );
            assert_eq!(call.memory_plan().target.limits, limits);
            assert_eq!(call.inputs().len(), template.inputs().len());
            assert_eq!(call.output().schema_key, template.output().key());
            for port in call
                .memory_plan()
                .inputs
                .iter()
                .chain(&call.memory_plan().outputs)
            {
                assert_eq!(
                    port.value.storage.planned_slot(),
                    mech_core::PlannedSlotKind::CanonicalValueHandle
                );
                assert_eq!(
                    port.value.slot.bytes,
                    core::mem::size_of::<Option<mech_core::Value>>() as u64
                );
            }
            assert!(
                call.memory_plan()
                    .allocations
                    .iter()
                    .all(|allocation| allocation.space == mech_core::MemorySpace::Host)
            );
            assert!(!call.memory_plan().deferred_witnesses.is_empty());
            assert!(
                bind_native_collection_operation(
                    &artifact,
                    node,
                    &catalog,
                    &refs,
                    &output_parameters,
                    mech_core::MemoryBudgetLimits::default(),
                )
                .is_err()
            );
            let mut wrong_output = output_parameters.clone();
            wrong_output.push(1);
            assert!(
                bind_native_collection_operation(
                    &artifact,
                    node,
                    &catalog,
                    &refs,
                    &wrong_output,
                    limits,
                )
                .is_err()
            );
        }
        assert_eq!(bound, 7, "all seven actual maintained leaves must bind");
    }
}

fn input_schema(artifact: &ProgramArtifact, name: &str, ordinal: usize) -> mech_core::SchemaId {
    let (node, _) = operations(artifact)
        .into_iter()
        .find(|(_, actual)| actual == name)
        .unwrap();
    let operation = artifact.nodes()[node.get() as usize]
        .as_operation()
        .unwrap();
    let mech_core::ResolvedOperationContract::Declared(contract) =
        artifact.contracts().get(operation.contract).unwrap()
    else {
        panic!("declared call")
    };
    contract.inputs[ordinal].schema
}

fn scalar_value(
    artifact: &ProgramArtifact,
    schema: mech_core::SchemaId,
    data: mech_core::ValueDataDraft,
) -> mech_core::Value {
    mech_core::ValueDraft {
        schema,
        shape_values: Box::new([]),
        data,
    }
    .finalize(&mech_core::snapshot::SnapshotValidationContext::new(
        artifact.schemas(),
    ))
    .unwrap()
}

fn admit_value(
    value: mech_core::Value,
    memory: &mech_core::ManagedMemoryBudget,
) -> mech_core::Value {
    let mut reservation = memory
        .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
        .unwrap();
    value.into_memory_budget(&mut reservation).unwrap()
}

fn call_for_values(
    artifact: &ProgramArtifact,
    name: &str,
    inputs: &[Option<mech_core::Value>],
    output: &Option<mech_core::Value>,
    limits: mech_core::MemoryBudgetLimits,
) -> mech_engine::native_artifact::NativeCollectionCall {
    let (node, _) = operations(artifact)
        .into_iter()
        .find(|(_, actual)| actual == name)
        .unwrap();
    let catalog = mech_stdlib::source_catalog();
    let template = native_collection_operation_template(artifact, node, &catalog).unwrap();
    let parameters = inputs
        .iter()
        .map(|value| value.as_ref().unwrap().shape().parameter_values())
        .collect::<Vec<_>>();
    let output_parameters = output
        .as_ref()
        .map(|value| value.shape().parameter_values().to_vec())
        .unwrap_or_else(|| vec![0; template.output().dimension_parameters().len()]);
    bind_native_collection_operation(
        artifact,
        node,
        &catalog,
        &parameters,
        &output_parameters,
        limits,
    )
    .unwrap()
}

#[test]
fn native_leaf_invocation_runs_all_seven_real_factories_and_transfers_same_account_results() {
    use mech_core::{ValueData, ValueDataDraft, snapshot::SequenceView};
    use mech_engine::native_artifact::NativeLibraryWorkBudget;
    let limits = mech_core::TargetMemoryProfile::current_resident_cpu()
        .unwrap()
        .limits;
    for artifact in artifacts(SOURCE) {
        let memory = mech_core::ManagedMemoryBudget::new(256 * 1024 * 1024);
        {
            let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
            let text_id = input_schema(&artifact, "text/lines", 0);
            let text = admit_value(
                scalar_value(
                    &artifact,
                    text_id,
                    ValueDataDraft::String("one\ntwo\none".into()),
                ),
                &memory,
            );
            let separator = admit_value(
                scalar_value(
                    &artifact,
                    input_schema(&artifact, "text/join", 1),
                    ValueDataDraft::String(",".into()),
                ),
                &memory,
            );
            let precision = admit_value(
                scalar_value(
                    &artifact,
                    input_schema(&artifact, "number/format", 1),
                    ValueDataDraft::U64(0),
                ),
                &memory,
            );
            let invoke = |name: &str, inputs: &[Option<mech_core::Value>]| {
                let mut output = None;
                let call = call_for_values(&artifact, name, inputs, &output, limits);
                call.invoke(inputs, &mut output, &work)
                    .unwrap_or_else(|error| panic!("{name}: {error:?}"));
                assert!(
                    output
                        .as_ref()
                        .unwrap()
                        .memory_budget_retained_bytes(&memory)
                        .is_some()
                );
                output
            };
            let mut words = invoke("text/lines", &[Some(text.clone())]);
            let joined = invoke("text/join", &[words.clone(), Some(separator.clone())]);
            assert!(
                matches!(joined.as_ref().unwrap().data(), ValueData::String(value) if value.as_ref() == "one,two,one")
            );
            let length = invoke("sequence/length", &[joined.clone()]);
            assert!(matches!(
                length.as_ref().unwrap().data(),
                ValueData::U64(11)
            ));
            let formatted = invoke("number/format", &[length.clone(), Some(precision.clone())]);
            assert!(
                matches!(formatted.as_ref().unwrap().data(), ValueData::String(value) if value.as_ref() == "11")
            );
            let counts = invoke("map/histogram", &[words.clone()]);
            let keys = invoke("map/keys", &[counts.clone()]);
            let values = invoke("map/values", &[counts.clone()]);
            let ValueData::Matrix(matrix) = keys.as_ref().unwrap().data() else {
                panic!("keys matrix")
            };
            let SequenceView::String(keys) = matrix.elements() else {
                panic!("String keys")
            };
            assert_eq!(
                keys.iter().map(|value| value.as_ref()).collect::<Vec<_>>(),
                ["one", "two"]
            );
            let ValueData::Matrix(matrix) = values.as_ref().unwrap().data() else {
                panic!("count matrix")
            };
            let SequenceView::U64(values) = matrix.elements() else {
                panic!("exact counts")
            };
            assert_eq!(values, [2, 1]);

            // Reuse a live binding across a changed data-dependent shape.
            let args = [Some(text.clone())];
            let lines = call_for_values(&artifact, "text/lines", &args, &words, limits);
            let shorter = admit_value(
                scalar_value(&artifact, text_id, ValueDataDraft::String("x\ny".into())),
                &memory,
            );
            let before = work.used();
            lines.invoke(&[Some(shorter)], &mut words, &work).unwrap();
            let ValueData::Matrix(matrix) = words.as_ref().unwrap().data() else {
                panic!("words")
            };
            assert_eq!(matrix.elements().len(), 2);
            assert!(work.used().0 > before.0);
        }
        assert_eq!(
            memory.used_bytes(),
            0,
            "all payload and invocation owners release"
        );
    }
}

#[test]
fn native_leaf_errors_and_memory_denials_preserve_output_and_recover_on_the_same_account() {
    use mech_core::{MemoryRuntimeError, ValueDataDraft};
    use mech_engine::native_artifact::{
        NativeCollectionInvocationError as Error, NativeLibraryWorkBudget,
    };
    let limits = mech_core::TargetMemoryProfile::current_resident_cpu()
        .unwrap()
        .limits;
    for artifact in artifacts(SOURCE) {
        let memory = mech_core::ManagedMemoryBudget::new(256 * 1024 * 1024);
        let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
        let integer = input_schema(&artifact, "number/format", 1);
        let good = [
            Some(admit_value(
                scalar_value(&artifact, integer, ValueDataDraft::U64(42)),
                &memory,
            )),
            Some(admit_value(
                scalar_value(&artifact, integer, ValueDataDraft::U64(0)),
                &memory,
            )),
        ];
        let mut output = None;
        let call = call_for_values(&artifact, "number/format", &good, &output, limits);
        call.invoke(&good, &mut output, &work).unwrap();
        let prior = output.clone().unwrap();
        let bad_precision = [
            good[0].clone(),
            Some(admit_value(
                scalar_value(&artifact, integer, ValueDataDraft::U64(13)),
                &memory,
            )),
        ];
        assert!(call.invoke(&bad_precision, &mut output, &work).is_err());
        assert!(output.as_ref().unwrap().shares_frozen_storage(&prior));
        let before = memory.used_bytes();
        let blocker = memory
            .reserve_capacity(memory.limit_bytes() - before)
            .unwrap();
        assert!(matches!(
            call.invoke(&good, &mut output, &work),
            Err(Error::Memory(MemoryRuntimeError::BudgetExceeded { .. }))
        ));
        assert!(output.as_ref().unwrap().shares_frozen_storage(&prior));
        drop(blocker);
        assert_eq!(memory.used_bytes(), before);
        assert!(!call.invoke(&good, &mut output, &work).unwrap());
        // Successful unchanged execution follows the maintained leaf's normal
        // publication behavior: equality is semantic, not root-pointer identity.
        assert!(
            output
                .as_ref()
                .unwrap()
                .snapshot_eq(artifact.schemas(), &prior, artifact.schemas())
                .unwrap()
        );
        let prior = output.as_ref().unwrap().clone();
        let unadmitted = [
            Some(scalar_value(&artifact, integer, ValueDataDraft::U64(42))),
            good[1].clone(),
        ];
        assert_eq!(
            call.invoke(&unadmitted, &mut output, &work),
            Err(Error::UnadmittedValue)
        );
        assert!(output.as_ref().unwrap().shares_frozen_storage(&prior));
        let foreign = mech_core::ManagedMemoryBudget::new(1_000_000);
        let wrong_account = [
            Some(admit_value(
                scalar_value(&artifact, integer, ValueDataDraft::U64(42)),
                &foreign,
            )),
            good[1].clone(),
        ];
        assert_eq!(
            call.invoke(&wrong_account, &mut output, &work),
            Err(Error::UnadmittedValue)
        );
        assert!(output.as_ref().unwrap().shares_frozen_storage(&prior));
        assert!(!call.invoke(&good, &mut output, &work).unwrap());
    }
}

#[test]
fn native_leaf_call_policy_cannot_be_lifted_by_a_larger_invocation_account() {
    use mech_core::{MemoryBudgetDimension, ValueDataDraft};
    use mech_engine::native_artifact::{
        NativeCollectionInvocationError as Error, NativeLibraryWorkBudget,
    };
    for artifact in artifacts(SOURCE) {
        let memory = mech_core::ManagedMemoryBudget::new(256 * 1024 * 1024);
        let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
        let text = admit_value(
            scalar_value(
                &artifact,
                input_schema(&artifact, "text/lines", 0),
                ValueDataDraft::String("one\ntwo".into()),
            ),
            &memory,
        );
        let args = [Some(text)];
        let mut output = None;
        let mut limits = mech_core::TargetMemoryProfile::current_resident_cpu()
            .unwrap()
            .limits;
        limits.max_compute_work = Some(1);
        let call = call_for_values(&artifact, "text/lines", &args, &output, limits);
        let before = memory.used_bytes();
        assert!(matches!(
            call.invoke(&args, &mut output, &work),
            Err(Error::Kernel(
                mech_core::ResidentKernelError::ResourceLimit {
                    dimension: MemoryBudgetDimension::ComputeWork,
                    ..
                }
            ))
        ));
        assert!(output.is_none());
        assert_eq!(memory.used_bytes(), before);
        assert_eq!(work.used(), (0, 0));
        limits.max_compute_work = Some(16_777_216);
        for temporary in [1_024, 65_536] {
            limits.max_temporary_bytes = Some(temporary);
            let call = call_for_values(&artifact, "text/lines", &args, &output, limits);
            // The per-call policy must reject before any private owner is
            // allocated, even when its larger invocation account is full.
            let blocker = memory
                .reserve_capacity(memory.limit_bytes() - before)
                .unwrap();
            assert!(
                matches!(call.invoke(&args, &mut output, &work), Err(Error::Plan(mech_core::MemoryPlanError::TargetLimitExceeded { violation })) if violation.dimension == MemoryBudgetDimension::TemporaryBytes)
            );
            assert!(output.is_none());
            drop(blocker);
            assert_eq!(memory.used_bytes(), before);
        }
    }
}

#[test]
fn native_leaf_invocation_requires_the_actual_current_shape_parameters() {
    use mech_core::ValueDataDraft;
    use mech_engine::native_artifact::{
        NativeCollectionInvocationError as Error, NativeLibraryWorkBudget,
    };
    let limits = mech_core::TargetMemoryProfile::current_resident_cpu()
        .unwrap()
        .limits;
    for artifact in artifacts(SOURCE) {
        let memory = mech_core::ManagedMemoryBudget::new(256 * 1024 * 1024);
        let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
        let text = admit_value(
            scalar_value(
                &artifact,
                input_schema(&artifact, "text/lines", 0),
                ValueDataDraft::String("one\ntwo\none".into()),
            ),
            &memory,
        );
        let separator = admit_value(
            scalar_value(
                &artifact,
                input_schema(&artifact, "text/join", 1),
                ValueDataDraft::String(",".into()),
            ),
            &memory,
        );
        let args = [Some(text)];
        let mut words = None;
        call_for_values(&artifact, "text/lines", &args, &words, limits)
            .invoke(&args, &mut words, &work)
            .unwrap();
        let mut stale = words.as_ref().unwrap().shape().parameter_values().to_vec();
        assert!(!stale.is_empty());
        stale[0] += 1;
        let catalog = mech_stdlib::source_catalog();
        let node = |name: &str| {
            operations(&artifact)
                .into_iter()
                .find(|(_, actual)| actual == name)
                .unwrap()
                .0
        };
        let joined_args = [words.clone(), Some(separator)];
        let wrong_input = bind_native_collection_operation(
            &artifact,
            node("text/join"),
            &catalog,
            &[&stale, &[]],
            &[],
            limits,
        )
        .unwrap();
        let mut joined = None;
        let before = memory.used_bytes();
        assert_eq!(
            wrong_input.invoke(&joined_args, &mut joined, &work),
            Err(Error::InvalidInput)
        );
        assert!(joined.is_none());
        assert_eq!(memory.used_bytes(), before);
        call_for_values(&artifact, "text/join", &joined_args, &joined, limits)
            .invoke(&joined_args, &mut joined, &work)
            .unwrap();

        let wrong_publication = bind_native_collection_operation(
            &artifact,
            node("text/lines"),
            &catalog,
            &[&[]],
            &stale,
            limits,
        )
        .unwrap();
        let prior = words.as_ref().unwrap().clone();
        let before = memory.used_bytes();
        assert_eq!(
            wrong_publication.invoke(&args, &mut words, &work),
            Err(Error::InvalidInput)
        );
        assert!(words.as_ref().unwrap().shares_frozen_storage(&prior));
        assert_eq!(memory.used_bytes(), before);
        call_for_values(&artifact, "text/lines", &args, &words, limits)
            .invoke(&args, &mut words, &work)
            .unwrap();
    }
}

static BINDS: AtomicUsize = AtomicUsize::new(0);
fn replacement(
    _: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    BINDS.fetch_add(1, Ordering::SeqCst);
    Err(ResidentKernelBindError::UnsupportedLayout)
}

#[test]
fn factory_replacements_names_costs_and_public_opcodes_do_not_grant_collection_authority() {
    #[derive(Debug)]
    struct ForgedOpcode(NativeCollectionPrimitive);
    let maintained = mech_stdlib::source_catalog();
    let original = maintained
        .resident_factory(&["sequence".into()], "length")
        .unwrap();
    let forged = Arc::new(ForgedOpcode(NativeCollectionPrimitive::SequenceLength));
    assert_eq!(forged.0, NativeCollectionPrimitive::SequenceLength);
    let variants = [
        original.clone().with_factory(replacement),
        original.clone().with_factory(original.factory()),
        original.clone().with_key(original.key().clone()),
        original
            .clone()
            .with_implementation_memory(original.implementation_memory()),
        original.clone().with_lowering_witness(forged),
    ];
    for artifact in artifacts(SOURCE) {
        let node = operations(&artifact)
            .into_iter()
            .find(|(_, name)| name == "sequence/length")
            .unwrap()
            .0;
        for changed in &variants {
            let mut builder = FunctionCatalogBuilder::new();
            builder
                .insert_resident_factory_entry(changed.clone())
                .unwrap();
            let catalog: FunctionCatalog = builder.build().unwrap();
            assert!(
                matches!(native_collection_operation_template(&artifact, node, &catalog), Err(NativeArtifactLoweringError::MissingSemanticWitness { node: actual }) if node == actual)
            );
            assert!(matches!(
                bind_native_collection_operation(
                    &artifact,
                    node,
                    &catalog,
                    &[&[]],
                    &[],
                    mech_core::TargetMemoryProfile::current_resident_cpu().unwrap().limits,
                ),
                Err(NativeArtifactLoweringError::MissingSemanticWitness { node: actual }) if node == actual
            ));
        }
    }
    assert_eq!(
        BINDS.load(Ordering::SeqCst),
        0,
        "metadata lowering must not bind or execute replacement code"
    );
}

struct Site {
    owner: NodeId,
    block: mech_engine::ControlBlockId,
    local: u32,
    name: String,
    sources: Box<[NativeCollectionLexicalSource]>,
}
fn lexical_sites(artifact: &ProgramArtifact) -> Vec<Site> {
    use mech_engine::{ControlOperationBody as Body, ExecutableNodeBody as Root};
    fn operation_name(operation: &mech_engine::OperationReference) -> String {
        format!(
            "{}/{}",
            operation.module_path.join("/"),
            operation.operation_name
        )
    }
    fn body(owner: NodeId, declaration: &Body, sites: &mut Vec<Site>) {
        match declaration {
            Body::Match(declaration) => matched(owner, declaration, sites),
            Body::Iteration(declaration) => block(owner, &declaration.body, sites),
            Body::Comprehension(declaration) => comprehension(owner, declaration, sites),
            _ => {}
        }
    }
    fn block(owner: NodeId, declaration: &mech_engine::ControlBlock, sites: &mut Vec<Site>) {
        for operation in &declaration.operations {
            if let Body::Operation {
                operation: reference,
                ..
            } = &operation.body
            {
                sites.push(Site {
                    owner,
                    block: declaration.id,
                    local: operation.node,
                    name: operation_name(reference),
                    sources: operation
                        .inputs
                        .iter()
                        .copied()
                        .map(NativeCollectionLexicalSource::Control)
                        .collect(),
                });
            }
            body(owner, &operation.body, sites);
        }
    }
    fn matched(owner: NodeId, declaration: &mech_engine::MatchDeclaration, sites: &mut Vec<Site>) {
        for arm in &declaration.arms {
            if let Some(guard) = &arm.guard {
                block(owner, guard, sites);
            }
            block(owner, &arm.body, sites);
        }
    }
    fn comprehension(
        owner: NodeId,
        declaration: &mech_engine::ComprehensionDeclaration,
        sites: &mut Vec<Site>,
    ) {
        for step in &declaration.steps {
            if let mech_engine::ComprehensionStep::Operation(operation) = step {
                if let Body::Operation {
                    operation: reference,
                    ..
                } = &operation.body
                {
                    sites.push(Site {
                        owner,
                        block: declaration.id,
                        local: operation.local,
                        name: operation_name(reference),
                        sources: operation
                            .inputs
                            .iter()
                            .copied()
                            .map(NativeCollectionLexicalSource::Comprehension)
                            .collect(),
                    });
                }
                body(owner, &operation.body, sites);
            }
        }
    }
    let mut sites = Vec::new();
    for node in artifact.nodes() {
        match &node.body {
            Root::Match(declaration) | Root::Activation(declaration) => {
                matched(node.node, declaration, &mut sites)
            }
            Root::Iteration(declaration) => block(node.node, &declaration.body, &mut sites),
            Root::Comprehension(declaration) => comprehension(node.node, declaration, &mut sites),
            _ => {}
        }
    }
    sites
}

const LEXICAL_CASES: &[&str] = &[
    "+> sequence\n(signal<string>? | value => sequence/length(value))\n",
    "+> sequence\n+> text\n[sequence/length(word) | word <- text/lines(signal<string>)]\n",
    "+> sequence\n+> text\n(flag<bool>? | true => [sequence/length(word) | word <- text/lines(signal<string>)] | false => [sequence/length(word) | word <- text/lines(signal<string>)])\n",
    "+> sequence\n+> number\nstep(x<string>) => (next<string>) := next := number/format(sequence/length(x), 0u64).\nseed<string> |2> step\n",
];

#[test]
fn collection_templates_resolve_genuine_match_comprehension_nested_and_iteration_sites() {
    let catalog = mech_stdlib::source_catalog();
    let mut control_operands = false;
    let mut comprehension_operands = false;
    for &source in LEXICAL_CASES {
        for artifact in artifacts(source) {
            let sites = lexical_sites(&artifact);
            let lengths = sites
                .iter()
                .filter(|site| site.name == "sequence/length")
                .collect::<Vec<_>>();
            assert!(
                !lengths.is_empty(),
                "{source}: expected an actual lexical library site"
            );
            for site in lengths {
                let template = native_collection_lexical_template(
                    &artifact, site.owner, site.block, site.local, &catalog,
                )
                .unwrap_or_else(|error| panic!("{source}: {error:?}"));
                assert_eq!(template.program_revision(), artifact.revision());
                assert_eq!(template.owner(), site.owner);
                assert_eq!(template.block(), site.block);
                assert_eq!(template.local(), site.local);
                assert_eq!(
                    template.primitive(),
                    NativeCollectionPrimitive::SequenceLength
                );
                assert_eq!(template.sources(), site.sources.as_ref());
                assert_eq!(template.inputs()[0].body(), &mech_core::SchemaBody::String);
                assert_eq!(
                    template.output().body(),
                    &mech_core::SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64)
                );
                for source in template.sources() {
                    control_operands |= matches!(source, NativeCollectionLexicalSource::Control(_));
                    comprehension_operands |=
                        matches!(source, NativeCollectionLexicalSource::Comprehension(_));
                }
                assert!(
                    native_collection_lexical_template(
                        &artifact,
                        site.owner,
                        mech_engine::ControlBlockId(u32::MAX),
                        site.local,
                        &catalog
                    )
                    .is_err()
                );
                assert!(
                    native_collection_lexical_template(
                        &artifact,
                        site.owner,
                        site.block,
                        u32::MAX,
                        &catalog
                    )
                    .is_err()
                );
                assert!(
                    native_collection_lexical_template(
                        &artifact,
                        NodeId::new(u32::MAX),
                        site.block,
                        site.local,
                        &catalog
                    )
                    .is_err()
                );
            }
        }
    }
    assert!(control_operands && comprehension_operands);
}

#[test]
fn lexical_collection_templates_also_reject_replaced_factories_without_binding_them() {
    let maintained = mech_stdlib::source_catalog();
    let original = maintained
        .resident_factory(&["sequence".into()], "length")
        .unwrap();
    let mut builder = FunctionCatalogBuilder::new();
    builder
        .insert_resident_factory_entry(original.clone().with_factory(replacement))
        .unwrap();
    let catalog = builder.build().unwrap();
    for artifact in artifacts(LEXICAL_CASES[2]) {
        let sites = lexical_sites(&artifact);
        let site = sites
            .iter()
            .find(|site| site.name == "sequence/length")
            .unwrap();
        assert!(
            matches!(native_collection_lexical_template(&artifact, site.owner, site.block, site.local, &catalog),
            Err(NativeArtifactLoweringError::MissingSemanticWitness { node }) if node == site.owner)
        );
    }
    assert_eq!(BINDS.load(Ordering::SeqCst), 0);
}
