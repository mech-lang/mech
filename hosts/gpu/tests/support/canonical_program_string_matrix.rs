use mech_core::snapshot::*;
use mech_core::{DimensionExpr, IntegerWidth, SchemaBody, SchemaDraft, SchemaTableBuilder};
use std::sync::Arc;

use super::{artifacts, equal, inputs, limits, reference, reference_result};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::JitProgramArtifact;

#[test]
fn compact_text_lines_admits_real_workspace_and_preserves_denial_atomicity() {
    let source = "+> text\ntext/lines(input<string>)\n";
    // The old generic draft/finalizer contract alone reserves >4MiB for
    // these cells. The compact constructor and publication fit that cap.
    let text = format!("{}\n", "x".repeat(60)).repeat(10_000);
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let args = inputs(&artifact, &[("input", Data::String(text.clone()))], &memory);
        let ordinary = reference(&artifact, &args);
        let mut caps = limits();
        caps.library.max_temporary_bytes = Some(4 * 1024 * 1024);
        caps.library.max_comparison_work = Some(16 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut output, caps, &memory)
            .unwrap();
        equal(&artifact, &output, &ordinary);
        let prior = output
            .iter()
            .map(|v| v.clone().unwrap())
            .collect::<Vec<_>>();
        let retained = memory.used_bytes();
        for dimension in 0..5 {
            let mut denied = caps;
            match dimension {
                0 => denied.library.max_temporary_bytes = Some(0),
                1 => denied.library.max_output_bytes = Some(0),
                2 => denied.library.max_cloned_bytes = Some(0),
                3 => denied.library.max_retained_nodes = Some(0),
                _ => denied.library.max_compute_work = Some(0),
            }
            assert!(
                jit.evaluate_into(&args, &mut output, denied, &memory)
                    .is_err()
            );
            equal(&artifact, &output, &prior);
            assert_eq!(memory.used_bytes(), retained);
        }
        jit.evaluate_into(&args, &mut output, caps, &memory)
            .unwrap();
        equal(&artifact, &output, &ordinary);
        drop((args, output, prior));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn compact_string_matrix_slice_keeps_live_selectors_and_constrained_workspace() {
    let source = "+> text\nwords := text/lines(input<string>)\nwords[first<u64>..end<u64>]\n";
    // Keep the same4MiB cap, including its independently reserved binding
    // metadata. Five thousand cells fit compactly but not as generic drafts.
    let text = format!("{}\n", "é\0x".repeat(15)).repeat(5_000);
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let mut caps = limits();
        caps.library.max_temporary_bytes = Some(4 * 1024 * 1024);
        caps.library.max_comparison_work = Some(16 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (first, end) in [(1, 5_001), (2, 4_001), (1, 1), (5_000, 5_001)] {
            let args = inputs(
                &artifact,
                &[
                    ("input", Data::String(text.clone())),
                    ("first", Data::U64(first)),
                    ("end", Data::U64(end)),
                ],
                &memory,
            );
            let ordinary = reference_result(&artifact, &args);
            if first == end {
                // Preserve this existing selected range leaf's empty-range
                // denial before reaching matrix selection, not a new empty
                // Matrix value or partial publication from this optimization.
                assert!(ordinary.unwrap_err().contains("error: InvalidInput"));
                let prior = output
                    .iter()
                    .map(|v| v.clone().unwrap())
                    .collect::<Vec<_>>();
                let retained = memory.used_bytes();
                let error = jit
                    .evaluate_into(&args, &mut output, caps, &memory)
                    .unwrap_err();
                assert!(matches!(error,
                    mech_gpu::native_artifact::NativeProgramExecutionError::Library {
                        origin,
                        error: mech_engine::native_artifact::NativeCollectionInvocationError::Kernel(
                            mech_core::ResidentKernelError::InvalidInput),
                    } if origin.node().get() == 1));
                equal(&artifact, &output, &prior);
                assert_eq!(memory.used_bytes(), retained);
                continue;
            }
            let ordinary = ordinary
                .unwrap_or_else(|error| panic!("slice {first}..{end} resident result: {error}"));
            jit.evaluate_into(&args, &mut output, caps, &memory)
                .unwrap();
            equal(&artifact, &output, &ordinary);
        }
        let prior = output
            .iter()
            .map(|v| v.clone().unwrap())
            .collect::<Vec<_>>();
        let args = inputs(
            &artifact,
            &[
                ("input", Data::String(text.clone())),
                ("first", Data::U64(1)),
                ("end", Data::U64(5_002)),
            ],
            &memory,
        );
        let retained = memory.used_bytes();
        assert!(
            jit.evaluate_into(&args, &mut output, caps, &memory)
                .is_err()
        );
        equal(&artifact, &output, &prior);
        assert_eq!(memory.used_bytes(), retained);
        drop((args, output, prior));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn compact_string_memory_class_keeps_other_finalizer_and_publication_obligations() {
    use mech_core::{
        AllocationRole, CurrentMemoryFootprint, ImplementationMemoryClass as Class, PortDirection,
    };
    use mech_engine::native_artifact::{NativeIterationAction, bind_native_program_library_call};
    use std::collections::BTreeMap;
    // Synthetic footprint witnesses exercise declaration derivation only;
    // they are never used to authorize an executing kernel or a timing.
    for (source, compact) in [
        ("values := input<[string]>\nvalues[indices<[u64]>]\n", true),
        ("values := input<[u64]>\nvalues[indices<[u64]>]\n", false),
        ("values := input<string>\nvalues[indices<[u64]>]\n", false),
    ] {
        for artifact in artifacts(source).0 {
            let catalog = mech_stdlib::source_catalog();
            let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let template = jit
                .descriptor()
                .plan()
                .blocks()
                .iter()
                .find_map(|block| match block.action() {
                    NativeIterationAction::LibraryCall { template, .. } => Some(template),
                    _ => None,
                })
                .unwrap();
            let parameters = template
                .inputs()
                .iter()
                .map(|schema| vec![2; schema.dimension_parameters().len()])
                .collect::<Vec<_>>();
            let parameters = parameters.iter().map(Vec::as_slice).collect::<Vec<_>>();
            let output = vec![2; template.output().dimension_parameters().len()];
            let call = bind_native_program_library_call(
                &artifact,
                template,
                &catalog,
                &parameters,
                &output,
                limits().library,
            )
            .unwrap();
            let selected = call.memory_plan();
            assert_eq!(
                selected.implementation_memory,
                Class::PackedStringMatrixOrCanonicalFinalize
            );
            let mut facts = BTreeMap::new();
            for (direction, ports) in [
                (PortDirection::Input, &selected.inputs),
                (PortDirection::Output, &selected.outputs),
            ] {
                for (index, port) in ports.iter().enumerate() {
                    facts.insert(
                        (direction, index as u16),
                        CurrentMemoryFootprint {
                            logical_elements: match port.descriptor.schema().body() {
                                SchemaBody::Matrix { dimensions, .. } => dimensions
                                    .iter()
                                    .map(|dimension| {
                                        port.descriptor
                                            .shape()
                                            .resolve_dimension(dimension)
                                            .unwrap()
                                    })
                                    .product(),
                                _ => 1,
                            },
                            payload_bytes: 1024,
                            encoded_bytes: 512,
                            retained_nodes: 5,
                            schema_bytes: port.descriptor.schema().canonical_bytes().len() as u64,
                            shape_parameter_count: port.descriptor.shape().parameter_values().len()
                                as u64,
                            ..CurrentMemoryFootprint::default()
                        },
                    );
                }
            }
            let new = mech_core::resolve_deferred_call_memory(selected, &facts).unwrap();
            let mut ordinary = selected.clone();
            ordinary.implementation_memory = Class::CanonicalFinalize;
            let ordinary = mech_core::resolve_deferred_call_memory(&ordinary, &facts).unwrap();
            assert_eq!(new.transactions, ordinary.transactions);
            assert_eq!(new.demand.work, ordinary.demand.work);
            if compact {
                assert!(
                    !new.allocations
                        .iter()
                        .any(|a| a.role == AllocationRole::ConstructionWorkspace)
                );
                assert_eq!(
                    ordinary
                        .allocations
                        .iter()
                        .filter(|a| a.role == AllocationRole::ConstructionWorkspace)
                        .count(),
                    2
                );
            } else {
                assert_eq!(new.allocations, ordinary.allocations);
                assert_eq!(new.demand, ordinary.demand);
            }
        }
    }
}

#[test]
fn matrix_valued_aggregate_range_keeps_generic_workspace() {
    use mech_core::*;
    let matrix = |element, columns| SchemaBody::Matrix {
        element: Box::new(element),
        dimensions: Box::new([DimensionExpr::Constant(1), DimensionExpr::Constant(columns)]),
    };
    let strings = matrix(SchemaBody::String, 2);
    let mut builder = SchemaTableBuilder::new();
    let handles = [
        SchemaBody::Tuple(Box::new([strings.clone(), strings.clone()])),
        matrix(SchemaBody::UnsignedInteger(IntegerWidth::W64), 1),
        strings,
    ]
    .into_iter()
    .map(|body| {
        builder
            .insert(
                SchemaDraft {
                    body,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap()
    })
    .collect::<Vec<_>>();
    let built = builder.finish().unwrap();
    let ids = handles
        .into_iter()
        .map(|handle| built.resolve(handle).unwrap())
        .collect::<Vec<_>>();
    let (schemas, _) = built.into_parts();
    let layouts = ids
        .iter()
        .map(|id| ResidentPortLayout {
            schema_id: *id,
            schema_key: schemas.get(*id).unwrap().key(),
            kind: ResidentValueKind::Snapshot,
            shape: ResidentShape::SCALAR,
            shape_instance: schemas
                .get(*id)
                .unwrap()
                .instantiate_shape(Box::new([]))
                .unwrap(),
            activation_fixed_shape: true,
            resolved_selector: None,
        })
        .collect::<Vec<_>>();
    let contract = ResolvedOperationContract::Declared(DeclaredOperationContract {
        inputs: ids[..2]
            .iter()
            .map(|schema| ResolvedInputPort {
                schema: *schema,
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            })
            .collect(),
        outputs: Box::new([ResolvedOutputPort {
            schema: ids[2],
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]),
        interaction: ExternalInteraction::Pure,
    });
    let key =
        ResidentOperationKey::new(Box::new(["access".to_owned()]), "range".to_owned()).unwrap();
    let catalog = mech_stdlib::source_catalog();
    let factory = catalog
        .resident_factory(&key.module_path, &key.operation_name)
        .unwrap();
    assert_eq!(
        factory.implementation_memory(),
        ImplementationMemoryClass::PackedStringMatrixOrCanonicalFinalize
    );
    // The maintained binder accepts a homogeneous tuple of String matrices
    // and a singleton matrix selector, although current source syntax rejects
    // that selector kind for tuples. Test the real public binder boundary,
    // not an invented native template or an alternate source evaluator.
    let kernel = (factory.factory())(&ResidentKernelBindRequest {
        contract: &contract,
        schemas: &schemas,
        inputs: &layouts[..2],
        output: layouts[2].clone(),
    })
    .expect("actual registered aggregate range fallback must bind");
    assert!(kernel.snapshot_output().is_some());
    let descriptor = |index: usize| {
        ResolvedValueDescriptor::from_schema(
            schemas.get(ids[index]).unwrap().clone(),
            layouts[index].shape_instance.clone(),
        )
        .unwrap()
    };
    let call = BoundCall::artifact_operation(
        ResolvedOperationDescriptor::from_resolved_contract("access/range", &contract).unwrap(),
        Box::new([descriptor(0), descriptor(1)]),
        Box::new([descriptor(2)]),
        key,
    )
    .unwrap();
    let target = TargetMemoryProfile::current_resident_cpu().unwrap();
    let storage = physical_storage_descriptor(
        FunctionValueRepresentation::AnyValue,
        &target,
        MemoryLifetime::Turn {
            first: MemoryPlanPoint::new(0),
            last: MemoryPlanPoint::new(1),
        },
    );
    // Concrete synthetic planner witnesses only: no plan here is used to
    // authorize execution or to claim a detached performance qualification.
    let footprint = |count| {
        MemoryFootprintWitness::Known(CurrentMemoryFootprint {
            logical_elements: count,
            payload_bytes: 1024,
            encoded_bytes: 512,
            retained_nodes: 5,
            ..CurrentMemoryFootprint::default()
        })
    };
    let plan = |class| {
        plan_call_memory(CallMemoryPlanningRequest {
            bound_call: &call,
            input_storage: &[storage.clone(), storage.clone()],
            output_storage: &[storage.clone()],
            input_witnesses: &[footprint(1), footprint(1)],
            output_witnesses: &[footprint(2)],
            published_output_witnesses: &[footprint(2)],
            implementation_memory: class,
            target: &target,
            regions: &[RegionAccessPlan::WholeValue],
        })
        .unwrap()
    };
    let guarded = plan(ImplementationMemoryClass::PackedStringMatrixOrCanonicalFinalize);
    let ordinary = plan(ImplementationMemoryClass::CanonicalFinalize);
    assert_eq!(guarded.allocations, ordinary.allocations);
    assert_eq!(guarded.demand, ordinary.demand);
    assert_eq!(guarded.transactions, ordinary.transactions);
}

#[test]
fn compact_string_matrices_match_validated_drafts_and_move_the_payloads() {
    let mut builder = SchemaTableBuilder::new();
    let mut handles = Vec::new();
    for (element, rows, columns) in [
        (SchemaBody::String, 2, 2),
        (SchemaBody::String, 1, 4),
        (SchemaBody::String, 0, 4),
        (SchemaBody::UnsignedInteger(IntegerWidth::W64), 2, 2),
    ] {
        handles.push(
            builder
                .insert(
                    SchemaDraft {
                        body: SchemaBody::Matrix {
                            element: Box::new(element),
                            dimensions: vec![
                                DimensionExpr::Constant(rows),
                                DimensionExpr::Constant(columns),
                            ]
                            .into_boxed_slice(),
                        },
                        dimension_parameters: Box::new([]),
                    }
                    .finalize()
                    .unwrap(),
                )
                .unwrap(),
        );
    }
    let built = builder.finish().unwrap();
    let ids = handles
        .into_iter()
        .map(|h| built.resolve(h).unwrap())
        .collect::<Vec<_>>();
    let (schemas, _) = built.into_parts();
    let schemas = Arc::new(schemas);
    let shape = |id| {
        schemas
            .get(id)
            .unwrap()
            .instantiate_shape(Box::new([]))
            .unwrap()
    };
    for id in &ids[..3] {
        let text: &[&str] = if *id == ids[2] {
            &[]
        } else {
            &["", "\0\r\n", "é", "e\u{301}🧑‍🚀"]
        };
        let strings = text
            .iter()
            .map(|s| Box::<str>::from(*s))
            .collect::<Vec<_>>();
        let pointer = strings.get(2).map(|s| s.as_ptr());
        let value = StringMatrixSnapshotConstructor::bind(*id, shape(*id), Arc::clone(&schemas))
            .unwrap()
            .construct(strings.into_boxed_slice())
            .unwrap();
        let ordinary = ValueDraft {
            schema: *id,
            shape_values: Box::new([]),
            data: ValueDataDraft::Matrix(
                text.iter()
                    .map(|s| ValueDataDraft::String((*s).into()))
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
        }
        .finalize(&SnapshotValidationContext::with_shared_schemas(&schemas))
        .unwrap();
        assert_eq!(value.schema_key(), ordinary.schema_key());
        assert_eq!(value.shape(), ordinary.shape());
        assert!(value.snapshot_eq(&schemas, &ordinary, &schemas).unwrap());
        assert_eq!(
            value.canonical_data_draft().unwrap(),
            ordinary.canonical_data_draft().unwrap()
        );
        let ValueData::Matrix(matrix) = value.data() else {
            panic!("matrix")
        };
        let SequenceView::String(strings) = matrix.elements() else {
            panic!("packed strings")
        };
        assert_eq!(
            strings.get(2).map(|s| s.as_ptr()),
            pointer,
            "construction must transfer existing String payload owners"
        );
    }
    for count in [0, 3, 5] {
        let strings = (0..count)
            .map(|_| Box::<str>::from("cell"))
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let compact_error =
            StringMatrixSnapshotConstructor::bind(ids[0], shape(ids[0]), Arc::clone(&schemas))
                .unwrap()
                .construct(strings)
                .unwrap_err();
        let ordinary_error = ValueDraft {
            schema: ids[0],
            shape_values: Box::new([]),
            data: ValueDataDraft::Matrix(
                (0..count)
                    .map(|_| ValueDataDraft::String("cell".into()))
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
        }
        .finalize(&SnapshotValidationContext::with_shared_schemas(&schemas))
        .unwrap_err();
        assert_eq!(compact_error, ordinary_error);
        assert!(matches!(
            compact_error,
            SnapshotValueError::PayloadCardinalityMismatchV1 {
                path,
                expected: 4,
                actual,
            } if path == SnapshotPath::root() && actual == count as u64
        ));
    }
    assert!(
        StringMatrixSnapshotConstructor::bind(ids[3], shape(ids[3]), Arc::clone(&schemas)).is_err()
    );
}
