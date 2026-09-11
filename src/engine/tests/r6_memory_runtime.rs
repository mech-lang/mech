#![cfg(feature = "full_compiler")]

use mech_core::{
    AccessMode, AliasPolicy, AllocationPlan, AllocationRole, ArenaBackingKind, ArenaPlacement,
    ArenaPlan, BoundCall, CallMemoryPlanningRequest, CellSlotId, ChangeDetectionPolicy,
    CurrentMemoryFootprint, DeliveryMode, ExecutionTarget, ExternalInteraction,
    ImplementationMemoryClass, InputPortLayout, InputPortPolicy, MemoryArenaId, MemoryBudgetLimits,
    MemoryFootprintWitness, MemoryLifetime, MemoryObjectId, MemoryObjectOwner, MemorySpace, NodeId,
    OperationContractDeclaration, OutputConstruction, OutputPortPolicy, Ref, RegionAccessPlan,
    ResolvedOperationDescriptor, ResourceDemand, RuntimeFunctionId, ShapeRule, TargetMemoryProfile,
    ValueCell, physical_storage_descriptor, plan_call_memory,
};
use mech_engine::ArtifactSource;
use mech_engine::memory_planner::{
    ActivationMemoryFacts, ActivationValueFact, CallSiteMemoryTemplate, PlannedValueClass,
    ProgramMemoryPlan, ProgramMemoryPlanTemplate, ValueMemoryPlanTemplate,
    instantiate_program_memory_plan,
};
use mech_engine::memory_runtime::{ManagedProgramMemory, realize_resident_memory};
use nalgebra::DMatrix;

fn plan() -> ProgramMemoryPlan {
    ProgramMemoryPlan {
        values: Box::new([]),
        call_nodes: Box::new([]),
        calls: Box::new([]),
        allocations: vec![AllocationPlan {
            id: MemoryObjectId::new(0),
            owner: MemoryObjectOwner::NodeScratch {
                node: NodeId::new(0),
                ordinal: 0,
            },
            role: AllocationRole::Scratch,
            slot: None,
            space: MemorySpace::ResidentCpu,
            current_bytes: 16,
            capacity_bytes: 16,
            payload_block_capacity: 0,
            alignment: 8,
            lifetime: MemoryLifetime::Activation,
            placement: ArenaPlacement {
                arena: MemoryArenaId::new(0),
                offset: 0,
            },
            reuse_group: None,
        }]
        .into_boxed_slice(),
        arenas: vec![ArenaPlan {
            id: MemoryArenaId::new(0),
            space: MemorySpace::ResidentCpu,
            backing: ArenaBackingKind::ContiguousBytes,
            capacity_bytes: 16,
            alignment: 8,
            members: vec![MemoryObjectId::new(0)].into_boxed_slice(),
        }]
        .into_boxed_slice(),
        transfers: Box::new([]),
        budget_limits: MemoryBudgetLimits::default(),
        peak: ResourceDemand::default(),
        budget_violations: Box::new([]),
    }
}

fn assert_realized(memory: &ManagedProgramMemory) {
    assert_eq!(memory.realized().bindings().len(), 1);
    // The live ledger includes the 16-byte arena plus its preallocated
    // eight-byte initialization map. Initialization authority is owned and
    // reclaimed with the arena; it is not free side metadata.
    assert_eq!(memory.domain().ledger().committed_bytes, 24);
    let observations = memory.domain().allocation_observations();
    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].actual_block_bytes, 16);
}

#[test]
fn direct_and_resident_adapters_share_the_r5_realization_boundary() {
    let direct = ManagedProgramMemory::realize(&plan()).unwrap();
    assert_realized(&direct);
    direct.close().unwrap();

    let resident = realize_resident_memory(&plan()).unwrap();
    assert_realized(&resident);
    resident.close().unwrap();
}

#[test]
fn repeated_activation_and_close_reclaims_all_instance_storage() {
    for _ in 0..32 {
        let memory = ManagedProgramMemory::realize(&plan()).unwrap();
        assert_eq!(memory.domain().ledger().committed_bytes, 24);
        memory.close().unwrap();
    }
}

fn solve_program(call_count: u32) -> ProgramMemoryPlan {
    let mut target = TargetMemoryProfile::current_resident_cpu().unwrap();
    target.limits.max_output_bytes = Some(2_000);
    let dimension = 205;
    let coefficients = ValueCell::from_exact_matrix_ref(
        Ref::new(DMatrix::<f64>::identity(dimension, dimension)),
        dimension,
        dimension,
    )
    .unwrap();
    let rhs = ValueCell::from_exact_matrix_ref(
        Ref::new(DMatrix::<f64>::from_element(dimension, 1, 1.0)),
        dimension,
        1,
    )
    .unwrap();
    let descriptors = [
        coefficients.resolved_descriptor().unwrap(),
        rhs.resolved_descriptor().unwrap(),
    ];
    let lifetime = MemoryLifetime::Turn {
        first: mech_core::MemoryPlanPoint::new(0),
        last: mech_core::MemoryPlanPoint::new(1),
    };
    let storages = [
        physical_storage_descriptor(coefficients.representation(), &target, lifetime),
        physical_storage_descriptor(rhs.representation(), &target, lifetime),
    ];
    let witnesses = [
        MemoryFootprintWitness::Known(CurrentMemoryFootprint {
            logical_elements: u64::try_from(dimension * dimension).unwrap(),
            ..CurrentMemoryFootprint::default()
        }),
        MemoryFootprintWitness::Known(CurrentMemoryFootprint {
            logical_elements: u64::try_from(dimension).unwrap(),
            ..CurrentMemoryFootprint::default()
        }),
    ];
    let operation = ResolvedOperationDescriptor::from_name(
        "test/r6-program-budget-scope",
        OperationContractDeclaration {
            inputs: InputPortLayout::Fixed(
                vec![
                    InputPortPolicy {
                        access: AccessMode::Read,
                        delivery: DeliveryMode::Signal,
                    };
                    2
                ]
                .into_boxed_slice(),
            ),
            outputs: vec![OutputPortPolicy {
                access: AccessMode::Write,
                delivery: DeliveryMode::Signal,
                construction: OutputConstruction::FullWrite {
                    shape: ShapeRule::Declared,
                },
                alias: AliasPolicy::NoAlias,
                change_detection: ChangeDetectionPolicy::AlwaysChanged,
            }]
            .into_boxed_slice(),
            interaction: ExternalInteraction::Pure,
        },
    )
    .unwrap();
    let bound = BoundCall::syntax_directed(
        operation,
        descriptors.to_vec().into_boxed_slice(),
        vec![descriptors[1].clone()].into_boxed_slice(),
        RuntimeFunctionId::from_name("R6ProgramBudgetScope"),
        ExecutionTarget::ResidentCpu,
    )
    .unwrap();
    let call = plan_call_memory(CallMemoryPlanningRequest {
        bound_call: &bound,
        input_storage: &storages,
        output_storage: &[storages[1].clone()],
        input_witnesses: &witnesses,
        output_witnesses: &[witnesses[1]],
        published_output_witnesses: &[witnesses[1]],
        implementation_memory: ImplementationMemoryClass::MatrixSolve,
        target: &target,
        regions: &[RegionAccessPlan::WholeValue],
    })
    .unwrap();
    assert!(call.demand.work.compute < target.limits.max_compute_work.unwrap());
    assert!(
        call.outputs[0].value.current_address_span_bytes < target.limits.max_output_bytes.unwrap()
    );

    let mut template = ProgramMemoryPlanTemplate::default();
    let mut facts = ActivationMemoryFacts::default();
    for raw in 0..call_count {
        let node = NodeId::new(raw);
        let slot = CellSlotId::new(raw);
        template.node_positions.insert(node, raw);
        template.values = template
            .values
            .iter()
            .cloned()
            .chain([ValueMemoryPlanTemplate {
                slot,
                descriptor: Some(descriptors[1].clone()),
                class: PlannedValueClass::PublishedOutput,
                producer: Some(node),
                last_consumer: None,
                alias_source: None,
            }])
            .collect();
        template.call_nodes = template.call_nodes.iter().copied().chain([node]).collect();
        template.call_sites = template
            .call_sites
            .iter()
            .cloned()
            .chain([CallSiteMemoryTemplate {
                node,
                input_sources: vec![
                    ArtifactSource::Constant(mech_core::ConstantId::new(0)),
                    ArtifactSource::Constant(mech_core::ConstantId::new(1)),
                ]
                .into_boxed_slice(),
                output_slots: vec![slot].into_boxed_slice(),
            }])
            .collect();
        template.calls = template
            .calls
            .iter()
            .cloned()
            .chain([call.clone()])
            .collect();
        facts.values.insert(
            slot,
            ActivationValueFact {
                descriptor: descriptors[1].clone(),
                storage: storages[1].clone(),
                witness: witnesses[1],
            },
        );
    }
    instantiate_program_memory_plan(&template, &target, &facts).unwrap()
}

#[test]
fn realization_preserves_per_call_work_and_output_budget_scopes() {
    let single = solve_program(1);
    assert!(
        single.budget_violations.is_empty(),
        "single call exceeded a supposedly per-call limit: {:?}",
        single.budget_violations
    );
    ManagedProgramMemory::realize(&single)
        .unwrap()
        .close()
        .unwrap();

    let multiple = solve_program(2);
    assert!(
        multiple.budget_violations.is_empty(),
        "program planner changed a per-call limit into a program quota: {:?}",
        multiple.budget_violations
    );
    assert!(multiple.peak.work.compute > multiple.budget_limits.max_compute_work.unwrap());
    let output_bytes = multiple
        .values
        .iter()
        .filter(|value| value.class == PlannedValueClass::PublishedOutput)
        .map(|value| value.layout.current_address_span_bytes + value.layout.payload.current_bytes)
        .sum::<u64>();
    assert!(output_bytes > multiple.budget_limits.max_output_bytes.unwrap());
    ManagedProgramMemory::realize(&multiple)
        .unwrap()
        .close()
        .unwrap();
}

#[test]
fn ordinary_source_literals_enter_one_interpreter_memory_session() {
    use mech_core::{FunctionCatalogBuilder, NoMechExecutionServices};
    use mech_engine::{CompilerPlanningConfig, CompilerPlanningProgram};
    use std::sync::Arc;

    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_runtime(&mut catalog).unwrap();
    mech_engine::install_intrinsic_compiler_runtime(&mut catalog).unwrap();
    mech_engine::install_intrinsic_source(&mut catalog).unwrap();
    let mut program = CompilerPlanningProgram::with_function_catalog(
        CompilerPlanningConfig::default(),
        Arc::new(catalog.build().unwrap()),
    );
    let tree = mech_syntax::parser::parse(
        "number := 1.0\ntext := \"managed\"\nmatrix := [1.0 2.0; 3.0 4.0]\nnumber",
    )
    .unwrap();
    let mut services = NoMechExecutionServices;
    program
        .plan_tree_with_services(&tree, &mut services)
        .unwrap();

    let values = program
        .compiler_root_symbol_cells(&["number", "text", "matrix"])
        .unwrap();
    let owner = values[0].1.memory_domain().unwrap().id();
    assert!(
        values
            .iter()
            .all(|(_, value)| value.memory_domain().unwrap().id() == owner),
        "source cells escaped the interpreter session: {:?}",
        values
            .iter()
            .map(|(name, value)| (name, value.memory_domain().unwrap().id()))
            .collect::<Vec<_>>()
    );
}

mod managed_index_conversion {
    use mech_core::*;

    fn bind(source: ValueCell, output: ValueCell) -> SpecializedFunction {
        let mut builder = FunctionCatalogBuilder::new();
        mech_engine::install_intrinsic_runtime(&mut builder).unwrap();
        let catalog = builder.build().unwrap();
        let operation = OperationId::from_name("access/index");
        let runtime = RuntimeFunctionId::from_name("access/index");
        let entry = catalog.runtime_entry(runtime).unwrap();
        let contract = entry.operation_contract(operation).unwrap().clone();
        let parts = entry
            .bind_resolved_invocation(
                operation,
                ExecutionTarget::DirectRuntime,
                FunctionInvocation::unary(output, source),
            )
            .unwrap();
        SpecializedFunction::syntax_directed(
            parts,
            ResolvedOperationDescriptor::from_name("access/index", contract).unwrap(),
            runtime,
            ExecutionTarget::DirectRuntime,
            entry.implementation_memory_class(),
        )
        .unwrap()
    }

    fn indices(output: &ValueCell) -> Vec<u64> {
        let value = output.snapshot().unwrap();
        match value.data() {
            ValueData::Index(value) => vec![*value],
            ValueData::Matrix(matrix) => {
                let snapshot::SequenceView::Index(values) = matrix.elements() else {
                    panic!("expected Index matrix")
                };
                values.to_vec()
            }
            _ => panic!("expected Index value"),
        }
    }

    macro_rules! selector_family {
        ($name:ident, $type:ty, $first:expr, $second:expr) => {
            #[test]
            fn $name() {
                let session = MemoryDomain::new().unwrap();
                let source = ValueCell::from_exact_in(&session, $first as $type).unwrap();
                let output = ValueCell::from_exact_in(&session, 1_usize).unwrap();
                let function = bind(source.clone(), output.clone());
                function.instance().solve_result().unwrap();
                assert_eq!(indices(&output), vec![1]);
                source
                    .replace(
                        &ValueCell::from_exact($second as $type)
                            .unwrap()
                            .snapshot()
                            .unwrap(),
                    )
                    .unwrap();
                function.instance().solve_result().unwrap();
                assert_eq!(indices(&output), vec![2]);

                let source = ValueCell::from_exact_in(
                    &session,
                    nalgebra::DMatrix::from_row_slice(
                        2,
                        3,
                        &[
                            1 as $type, 6 as $type, 2 as $type, 5 as $type, 3 as $type, 4 as $type,
                        ],
                    ),
                )
                .unwrap();
                let output = ValueCell::from_exact_in(
                    &session,
                    nalgebra::DMatrix::from_element(6, 1, 1_usize),
                )
                .unwrap();
                let function = bind(source.clone(), output.clone());
                let output_alias = output.clone();
                function.instance().solve_result().unwrap();
                assert_eq!(indices(&output_alias), vec![1, 6, 2, 5, 3, 4]);
                for position in [0, 2, 5] {
                    let mut values = [7 as $type; 6];
                    values[position] = 0 as $type;
                    let version = output.published_version();
                    let replacement =
                        ValueCell::from_exact(nalgebra::DMatrix::from_row_slice(2, 3, &values));
                    if std::any::TypeId::of::<$type>() == std::any::TypeId::of::<usize>() {
                        // Index itself has no zero value. Canonical ingress must reject it
                        // before a kernel can observe the invalid logical selector.
                        assert_eq!(
                            replacement.unwrap_err().kind_name(),
                            "ValueCellSnapshotFailure"
                        );
                    } else {
                        source
                            .replace(&replacement.unwrap().snapshot().unwrap())
                            .unwrap();
                        assert_eq!(
                            function.instance().solve_result().unwrap_err().kind_name(),
                            "CannotConvertToType"
                        );
                    }
                    assert_eq!(indices(&output_alias), vec![1, 6, 2, 5, 3, 4]);
                    assert_eq!(output.published_version(), version);
                }
            }
        };
    }

    selector_family!(index_ports_are_managed, usize, 1, 2);
    selector_family!(u8_ports_are_managed, u8, 1, 2);
    selector_family!(u16_ports_are_managed, u16, 1, 2);
    selector_family!(u32_ports_are_managed, u32, 1, 2);
    selector_family!(u64_ports_are_managed, u64, 1, 2);
    selector_family!(u128_ports_are_managed, u128, 1, 2);
    selector_family!(i8_ports_are_managed, i8, 1, 2);
    selector_family!(i16_ports_are_managed, i16, 1, 2);
    selector_family!(i32_ports_are_managed, i32, 1, 2);
    selector_family!(i64_ports_are_managed, i64, 1, 2);
    selector_family!(i128_ports_are_managed, i128, 1, 2);
    selector_family!(f32_fractional_ports_are_managed, f32, 1.9, 2.5);
    selector_family!(f64_fractional_ports_are_managed, f64, 1.9, 2.5);
}

#[cfg(feature = "resident-artifact")]
mod resident_existing_value_budget_scope {
    use mech_core::snapshot::{F64Bits, SnapshotValidationContext};
    use mech_core::*;
    use mech_engine::__resident::{
        ActivationFacts, CapturedSignalInput, ReactiveInstance, ResidentActivationError,
        ResidentActivationOptions, ResidentValueBorrow, StateMigrationPolicy, activate,
        activate_with_options,
    };
    use mech_engine::{
        ArtifactBuildContext, OperationReference, ProgramArtifact, SourceInput, SourceNode,
        SourceNodeOutput, SourceOutput, SourceProgram, SourceValue,
        compile_source_program_with_contracts,
    };

    fn catalog() -> FunctionCatalog {
        let mut builder = FunctionCatalogBuilder::new();
        mech_engine::install_intrinsic_source(&mut builder).unwrap();
        mech_engine::install_intrinsic_resident(&mut builder).unwrap();
        builder.build().unwrap()
    }

    fn matrix_schema(element: SchemaBody, elements: usize, column: bool) -> Schema {
        SchemaDraft {
            dimension_parameters: Box::new([]),
            body: SchemaBody::Matrix {
                element: Box::new(element),
                dimensions: if column {
                    vec![
                        DimensionExpr::Constant(elements as u64),
                        DimensionExpr::Constant(1),
                    ]
                } else {
                    vec![
                        DimensionExpr::Constant(1),
                        DimensionExpr::Constant(elements as u64),
                    ]
                }
                .into_boxed_slice(),
            },
        }
        .finalize()
        .unwrap()
    }

    // Compile an ordinary semantic artifact using the installed access contract
    // and Resident binder. The selector remains a signal even for constants,
    // so both cases cross activation and the real turn/publication boundary.
    fn access_artifact(
        catalog: &FunctionCatalog,
        values: &[f64],
        constant: bool,
        gather: bool,
    ) -> (ProgramArtifact, MemoryObjectOwner) {
        let mut schemas = SchemaTableBuilder::new();
        let f64_body = SchemaBody::FloatingPoint(FloatWidth::W64);
        let source_schema = schemas
            .insert(matrix_schema(f64_body.clone(), values.len(), false))
            .unwrap();
        let output_schema = schemas
            .insert(if gather {
                matrix_schema(f64_body, values.len(), true)
            } else {
                SchemaDraft {
                    dimension_parameters: Box::new([]),
                    body: f64_body,
                }
                .finalize()
                .unwrap()
            })
            .unwrap();
        let selector_schema = schemas
            .insert(if gather {
                matrix_schema(SchemaBody::Index, values.len(), true)
            } else {
                SchemaDraft {
                    dimension_parameters: Box::new([]),
                    body: SchemaBody::Index,
                }
                .finalize()
                .unwrap()
            })
            .unwrap();
        let build = schemas.finish().unwrap();
        let source_schema = build.resolve(source_schema).unwrap();
        let output_schema = build.resolve(output_schema).unwrap();
        let selector_schema = build.resolve(selector_schema).unwrap();
        let (schemas, _) = build.into_parts();

        let mut constants = ConstantStoreBuilder::new(&schemas);
        let source_constant = constant.then(|| {
            constants
                .insert(
                    ValueDraft {
                        schema: source_schema,
                        shape_values: Box::new([]),
                        data: ValueDataDraft::Matrix(
                            values
                                .iter()
                                .map(|value| ValueDataDraft::F64(F64Bits::from_f64(*value)))
                                .collect::<Vec<_>>()
                                .into_boxed_slice(),
                        ),
                    }
                    .finalize(&SnapshotValidationContext::new(&schemas))
                    .unwrap(),
                )
                .unwrap()
        });
        let build = constants.finish().unwrap();
        let source_constant = source_constant.map(|handle| build.resolve(handle).unwrap());
        let (constants, _) = build.into_parts();
        let mut inputs = Vec::new();
        let (source, owner) = if let Some(constant) = source_constant {
            (
                SourceValue::Constant(constant),
                MemoryObjectOwner::Constant(constant),
            )
        } else {
            inputs.push(SourceInput {
                name: "source".to_owned(),
                schema: source_schema,
            });
            (
                SourceValue::Input(0),
                MemoryObjectOwner::Slot(CellSlotId::new(0)),
            )
        };
        let selector = SourceValue::Input(inputs.len() as u32);
        inputs.push(SourceInput {
            name: "selector".to_owned(),
            schema: selector_schema,
        });
        let name = if gather { "range" } else { "scalar" };
        let graph = SourceProgram {
            inputs: inputs.into_boxed_slice(),
            nodes: vec![SourceNode {
                operation: OperationReference {
                    module_path: vec!["access".to_owned()].into_boxed_slice(),
                    operation_name: name.to_owned(),
                },
                requirement: None,
                inputs: vec![source, selector].into_boxed_slice(),
                outputs: vec![SourceNodeOutput::Derived {
                    schema: output_schema,
                }]
                .into_boxed_slice(),
            }]
            .into_boxed_slice(),
            outputs: vec![SourceOutput {
                name: "result".to_owned(),
                interactive_symbol: None,
                source: SourceValue::NodeOutput {
                    node: 0,
                    output_ordinal: 0,
                },
                schema: output_schema,
            }]
            .into_boxed_slice(),
            ..SourceProgram::default()
        };
        let contract = &catalog
            .operation_specializer(OperationId::from_name(&format!("access/{name}")))
            .unwrap()
            .operation
            .contract;
        let artifact = compile_source_program_with_contracts(
            &graph,
            &mut ArtifactBuildContext::new(&schemas, &constants),
            &[contract],
        )
        .unwrap();
        (artifact, owner)
    }

    fn turn(
        instance: &mut ReactiveInstance,
        source: Option<&[f64]>,
        selector: u64,
    ) -> Result<(), mech_engine::__resident::ResidentExecutionError> {
        let selector = [selector];
        let mut inputs = Vec::new();
        if let Some(source) = source {
            inputs.push(CapturedSignalInput {
                slot: instance.plan.inputs[0].slot,
                value: ResidentValueRef::F64(source),
            });
        }
        inputs.push(CapturedSignalInput {
            slot: instance.plan.inputs.last().unwrap().slot,
            value: ResidentValueRef::Index(&selector),
        });
        instance.turn_without_summary(&inputs)
    }

    fn assert_scalar(instance: &ReactiveInstance, expected: f64) {
        let ResidentValueBorrow::F64 { shape, values } = instance.output_borrow(0).unwrap() else {
            panic!("scalar access must publish an F64 result");
        };
        assert_eq!(shape, ResidentShape::SCALAR);
        assert_eq!(values, &[expected]);
    }

    fn activate_budgeted(
        artifact: &ProgramArtifact,
        catalog: &FunctionCatalog,
        budget: &ManagedMemoryBudget,
    ) -> Result<ReactiveInstance, ResidentActivationError> {
        activate_with_options(
            ReactiveInstanceId::new(1, 0),
            artifact,
            catalog,
            &ActivationFacts::default(),
            ResidentActivationOptions {
                memory_budget: Some(budget.clone()),
                ..ResidentActivationOptions::default()
            },
        )
    }

    #[test]
    fn configured_resident_budget_is_aggregate_and_preserves_failed_reactivation() {
        let catalog = catalog();
        let values = [3.0, 7.0];
        let (artifact, _) = access_artifact(&catalog, &values, false, false);
        let larger_values = vec![11.0; 1024];
        let (larger, _) = access_artifact(&catalog, &larger_values, false, false);
        let probe = ManagedMemoryBudget::new(u64::MAX);
        let instance = activate_budgeted(&artifact, &catalog, &probe).unwrap();
        let small_bytes = probe.used_bytes();
        assert!(small_bytes > 0);
        drop(instance);
        assert_eq!(probe.used_bytes(), 0);
        let instance = activate_budgeted(&larger, &catalog, &probe).unwrap();
        let large_bytes = probe.used_bytes();
        assert!(large_bytes > small_bytes);
        drop(instance);
        assert_eq!(probe.used_bytes(), 0);

        let one_under = ManagedMemoryBudget::new(small_bytes - 1);
        assert!(matches!(
            activate_budgeted(&artifact, &catalog, &one_under),
            Err(ResidentActivationError::MemoryRuntime {
                error: MemoryRuntimeError::BudgetExceeded { .. }
            })
        ));
        assert_eq!(one_under.used_bytes(), 0);

        // A complete legal program fits exactly, but another legal program
        // sharing its configured account may not exceed the aggregate limit.
        let exact = ManagedMemoryBudget::new(small_bytes);
        let mut instance = activate_budgeted(&artifact, &catalog, &exact).unwrap();
        assert_eq!(exact.used_bytes(), small_bytes);
        turn(&mut instance, Some(&values), 2).unwrap();
        assert_scalar(&instance, 7.0);
        assert!(matches!(
            activate_budgeted(&artifact, &catalog, &exact),
            Err(ResidentActivationError::MemoryRuntime {
                error: MemoryRuntimeError::BudgetExceeded { .. }
            })
        ));
        assert_eq!(exact.used_bytes(), small_bytes);
        assert_scalar(&instance, 7.0);
        drop(instance);
        assert_eq!(exact.used_bytes(), 0);

        // The replacement fits on its own. It still must include the old
        // instance retained for rollback until replacement publication.
        let coexist = ManagedMemoryBudget::new(small_bytes + large_bytes - 1);
        let mut instance = activate_budgeted(&artifact, &catalog, &coexist).unwrap();
        turn(&mut instance, Some(&values), 2).unwrap();
        let epoch = instance.published_epoch();
        let hash = instance.published_state_hash();
        let generation = instance.plan.plan_generation;
        assert!(matches!(
            instance.reactivate(
                &larger,
                &catalog,
                &ActivationFacts::default(),
                StateMigrationPolicy::PreserveCompatibleResetIncompatible,
            ),
            Err(ResidentActivationError::MemoryRuntime {
                error: MemoryRuntimeError::BudgetExceeded { .. }
            })
        ));
        assert_eq!(coexist.used_bytes(), small_bytes);
        assert_eq!(instance.published_epoch(), epoch);
        assert_eq!(instance.published_state_hash(), hash);
        assert_eq!(instance.plan.plan_generation, generation);
        assert_scalar(&instance, 7.0);
        turn(&mut instance, Some(&values), 1).unwrap();
        assert_scalar(&instance, 3.0);
        drop(instance);
        assert_eq!(coexist.used_bytes(), 0);
        let mut replacement = activate_budgeted(&larger, &catalog, &coexist).unwrap();
        turn(&mut replacement, Some(&larger_values), 1).unwrap();
        assert_scalar(&replacement, 11.0);
        drop(replacement);
        assert_eq!(coexist.used_bytes(), 0);
    }

    #[test]
    fn large_inputs_and_constants_execute_small_results_without_output_quotas() {
        let catalog = catalog();
        let limit = TargetMemoryProfile::current_resident_cpu()
            .unwrap()
            .limits
            .max_output_elements
            .unwrap();
        assert_eq!(limit, 65_536, "do not relax the published-output quota");
        for constant in [false, true] {
            for elements in [limit as usize, limit as usize + 1] {
                let values = (0..elements)
                    .map(|index| index as f64 + 0.25)
                    .collect::<Vec<_>>();
                let (artifact, owner) = access_artifact(&catalog, &values, constant, false);
                let mut instance = activate(
                    ReactiveInstanceId::new(1, 0),
                    &artifact,
                    &catalog,
                    &ActivationFacts::default(),
                )
                .unwrap_or_else(|error| {
                    panic!("constant={constant}, elements={elements}: {error:?}")
                });

                // Activation also audits each realized lane against this plan;
                // admitting existing input storage must not remove its bytes.
                let plan = &instance.plan.memory_plan;
                let source = plan
                    .allocations
                    .iter()
                    .find(|allocation| {
                        allocation.owner == owner && allocation.role == AllocationRole::FixedStorage
                    })
                    .unwrap();
                assert_eq!(source.current_bytes, elements as u64 * 8);
                assert!(source.capacity_bytes >= source.current_bytes);
                assert!(plan.budget_violations.is_empty());
                if constant {
                    assert_eq!(source.lifetime, MemoryLifetime::Program);
                    assert!(plan.peak.persistent_bytes >= source.current_bytes);
                    assert_eq!(instance.activation.f64_storage(), values);
                } else {
                    assert_eq!(source.lifetime, MemoryLifetime::Activation);
                    assert!(plan.peak.activation_bytes >= source.current_bytes);
                }

                let supplied = (!constant).then_some(values.as_slice());
                turn(&mut instance, supplied, elements as u64).unwrap();
                assert_scalar(&instance, values[elements - 1]);
                let epoch = instance.published_epoch();
                let hash = instance.published_state_hash();
                assert!(turn(&mut instance, supplied, elements as u64 + 1).is_err());
                assert_eq!(instance.published_epoch(), epoch);
                assert_eq!(instance.published_state_hash(), hash);
                assert_scalar(&instance, values[elements - 1]);
                turn(&mut instance, supplied, 1).unwrap();
                assert_scalar(&instance, values[0]);
            }
        }
    }

    #[test]
    fn producing_an_oversized_result_still_fails_resident_activation() {
        let catalog = catalog();
        let values = vec![1.25; 65_537];
        let (artifact, _) = access_artifact(&catalog, &values, true, true);
        let error = activate(
            ReactiveInstanceId::new(1, 0),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap_err();
        let ResidentActivationError::ResidentMemoryPlanRejected {
            error: MemoryPlanError::TargetLimitExceeded { violation },
        } = error
        else {
            panic!("oversized output must fail its planned output quota, not binding: {error:?}");
        };
        assert_eq!(violation.dimension, MemoryBudgetDimension::OutputElements);
        assert_eq!(violation.required, 65_537);
        assert_eq!(violation.limit, 65_536);
    }
}
