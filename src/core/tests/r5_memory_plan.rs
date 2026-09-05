#![cfg(feature = "full")]

use std::collections::BTreeMap;

use mech_core::snapshot::{Complex64Bits, F64Bits};
use mech_core::{
    AccessMode, AliasDecision, AliasPolicy, BoundCall, CallMemoryPlan, CallMemoryPlanningRequest,
    CapacityAuthority, CardinalitySpec, ChangeDetectionPolicy, CurrentMemoryFootprint,
    DeliveryMode, DimensionExpr, DimensionLifetime, DimensionParameterDeclaration,
    DimensionParameterId, DimensionParameterOrigin, EffectContract, EffectDeliveryPolicy,
    ExecutionTarget, ExtentEvolution, ExternalInteraction, FunctionValueRepresentation,
    GrowthPolicy, IdempotencyRequirement, ImplementationMemoryClass, InputPortLayout,
    InputPortPolicy, MemoryFootprintWitness, MemoryLifetime, MemoryPlanError, MemoryTargetKind,
    OperationContractDeclaration, OutputConstruction, OutputPortPolicy, PlannedSlotKind, Ref,
    RegionAccessPlan, RegionPolicy, ResolvedOperationDescriptor, ResolvedValueDescriptor,
    RuntimeFunctionId, SchemaBody, SchemaDraft, ShapeContractReference, ShapeRule, SlotLayout,
    TargetMemoryProfile, TransactionRequirement, ValueCell, ValueDataDraft, ValueLayoutPlan,
    ValueLayoutPlanningRequest, derive_dimension_capacity, physical_storage_descriptor,
    plan_call_memory, plan_value_layout, replan_call_memory, resolve_deferred_call_demand,
};
use nalgebra::{DMatrix, DVector, RowDVector};

fn known(elements: u64, payload: u64, nodes: u64) -> MemoryFootprintWitness {
    MemoryFootprintWitness::Known(CurrentMemoryFootprint {
        logical_elements: elements,
        payload_bytes: payload,
        retained_nodes: nodes,
        ..CurrentMemoryFootprint::default()
    })
}

fn plan(
    cell: &ValueCell,
    target: &TargetMemoryProfile,
    witness: MemoryFootprintWitness,
) -> Result<ValueLayoutPlan, MemoryPlanError> {
    let descriptor = cell
        .resolved_descriptor()
        .expect("test value has a resolved descriptor");
    let storage =
        physical_storage_descriptor(cell.representation(), target, MemoryLifetime::Activation);
    plan_value_layout(ValueLayoutPlanningRequest {
        descriptor: &descriptor,
        storage: &storage,
        witness,
        target,
    })
}

#[test]
fn every_fixed_scalar_kind_has_the_exact_host_layout() {
    let target = TargetMemoryProfile::current_direct_host().unwrap();
    let layouts = &target.primitives;
    let expected = [
        (layouts.bool_slot, size_of::<bool>()),
        (layouts.u8_slot, size_of::<u8>()),
        (layouts.u16_slot, size_of::<u16>()),
        (layouts.u32_slot, size_of::<u32>()),
        (layouts.u64_slot, size_of::<u64>()),
        (layouts.u128_slot, size_of::<u128>()),
        (layouts.i8_slot, size_of::<i8>()),
        (layouts.i16_slot, size_of::<i16>()),
        (layouts.i32_slot, size_of::<i32>()),
        (layouts.i64_slot, size_of::<i64>()),
        (layouts.i128_slot, size_of::<i128>()),
        (layouts.f32_slot, size_of::<f32>()),
        (layouts.f64_slot, size_of::<f64>()),
        (layouts.id_slot, size_of::<u64>()),
        (layouts.index_slot, size_of::<usize>()),
        (layouts.atom_slot, size_of::<()>()),
    ];
    for (layout, bytes) in expected {
        assert_eq!(layout.bytes, bytes as u64);
        assert!(layout.alignment.is_power_of_two());
    }
    assert_eq!(target.kind, MemoryTargetKind::DirectHost);
    assert_eq!(layouts.c32_slot.bytes, 8);
    assert_eq!(layouts.c64_slot.bytes, 16);

    let complex = ValueCell::from_schema_data(
        SchemaBody::Complex(mech_core::FloatWidth::W64),
        ValueDataDraft::Complex64(Complex64Bits::new(
            F64Bits::from_f64(1.0),
            F64Bits::from_f64(2.0),
        )),
    )
    .unwrap();
    assert_eq!(
        plan(&complex, &target, known(1, 0, 0)).unwrap().slot.bytes,
        16
    );
}

#[test]
fn index_layout_is_target_local_and_host_atom_may_be_zero_sized() {
    let host = TargetMemoryProfile::current_direct_host().unwrap();
    let resident = TargetMemoryProfile::current_resident_cpu().unwrap();
    assert_eq!(host.primitives.index_slot.bytes, size_of::<usize>() as u64);
    assert_eq!(
        resident.primitives.index_slot.bytes,
        size_of::<u64>() as u64
    );

    assert_eq!(
        host.primitives.atom_slot,
        SlotLayout {
            bytes: 0,
            alignment: 1
        }
    );
}

#[test]
fn target_addressability_is_checked_before_layout_publication() {
    let value = ValueCell::from_exact("five!".to_owned()).unwrap();
    let mut target = TargetMemoryProfile::current_direct_host().unwrap();
    target.maximum_addressable_bytes = 4;
    assert_eq!(
        plan(&value, &target, known(1, 5, 1)),
        Err(MemoryPlanError::TargetAddressOverflow)
    );

    target.maximum_addressable_bytes = u32::MAX as u64;
    assert!(plan(&value, &target, known(1, 5, 1)).is_ok());
    target.maximum_addressable_bytes = u64::MAX;
    assert!(plan(&value, &target, known(1, 5, 1)).is_ok());
}

#[test]
fn fixed_and_invariant_axis_matrices_have_deterministic_column_major_capacity() {
    let target = TargetMemoryProfile::current_direct_host().unwrap();
    let fixed = ValueCell::from_exact_matrix_ref(
        Ref::new(DMatrix::<f64>::from_vec(2, 3, vec![0.0; 6])),
        2,
        3,
    )
    .unwrap();
    let first = plan(&fixed, &target, known(6, 0, 0)).unwrap();
    let second = plan(&fixed, &target, known(6, 0, 0)).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.current_elements, 6);
    assert_eq!(first.capacity_bytes, 6 * size_of::<f64>() as u64);
    assert_eq!(first.strides_bytes.as_ref(), &[8, 16]);

    let row =
        ValueCell::from_exact_matrix_ref(Ref::new(RowDVector::<f64>::zeros(3)), 1, 3).unwrap();
    let column =
        ValueCell::from_exact_matrix_ref(Ref::new(DVector::<f64>::zeros(3)), 3, 1).unwrap();
    let row_plan = plan(&row, &target, known(3, 0, 0)).unwrap();
    let column_plan = plan(&column, &target, known(3, 0, 0)).unwrap();
    assert_eq!(
        row_plan.axes[0].capacity.authority,
        CapacityAuthority::ExactSemantic
    );
    assert_eq!(
        column_plan.axes[1].capacity.authority,
        CapacityAuthority::ExactSemantic
    );
    assert_eq!(row_plan.axes[1].evolution, ExtentEvolution::TurnUnbounded);
    assert_eq!(
        column_plan.axes[0].evolution,
        ExtentEvolution::TurnUnbounded
    );
}

#[test]
fn string_and_recursive_payloads_require_current_witnesses_and_replanning() {
    let target = TargetMemoryProfile::current_direct_host().unwrap();
    let text = ValueCell::from_exact("payload".to_owned()).unwrap();
    let text_plan = plan(&text, &target, known(1, 7, 1)).unwrap();
    assert_eq!(text_plan.slot, target.primitives.string_header);
    assert_eq!(text_plan.payload.current_bytes, 7);
    assert_eq!(text_plan.payload.growth, GrowthPolicy::ReplanBeforeGrowth);

    let aggregate = ValueCell::from_schema_data(
        SchemaBody::Tuple(vec![SchemaBody::String, SchemaBody::Index].into_boxed_slice()),
        ValueDataDraft::Tuple(
            vec![
                ValueDataDraft::String("payload".into()),
                ValueDataDraft::Index(1),
            ]
            .into_boxed_slice(),
        ),
    )
    .unwrap();
    let aggregate_plan = plan(&aggregate, &target, known(2, 7, 3)).unwrap();
    assert!(matches!(
        aggregate_plan.storage,
        mech_core::StorageLayoutClass::CanonicalSnapshot { .. }
    ));
    assert_eq!(aggregate_plan.payload.current_nodes, 3);
}

#[test]
fn canonical_physical_storage_uses_one_handle_for_a_dense_semantic_value() {
    let target = TargetMemoryProfile::current_resident_cpu().unwrap();
    let matrix = ValueCell::from_schema_data(
        SchemaBody::Matrix {
            element: Box::new(SchemaBody::SignedInteger(mech_core::IntegerWidth::W32)),
            dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(2)]
                .into_boxed_slice(),
        },
        ValueDataDraft::Matrix(
            vec![ValueDataDraft::I32(3), ValueDataDraft::I32(4)].into_boxed_slice(),
        ),
    )
    .unwrap();
    let descriptor = matrix.resolved_descriptor().unwrap();
    let canonical = ValueCell::from_schema_data(
        SchemaBody::Tuple(Box::new([])),
        ValueDataDraft::Tuple(Box::new([])),
    )
    .unwrap();
    let storage = physical_storage_descriptor(
        canonical.representation(),
        &target,
        MemoryLifetime::Activation,
    );
    let layout = plan_value_layout(ValueLayoutPlanningRequest {
        descriptor: &descriptor,
        storage: &storage,
        witness: known(2, 8, 3),
        target: &target,
    })
    .unwrap();

    assert!(matches!(
        layout.storage,
        mech_core::StorageLayoutClass::CanonicalSnapshot { .. }
    ));
    assert_eq!(layout.current_elements, 2);
    assert_eq!(
        layout.capacity_bytes,
        target.primitives.canonical_value_handle.bytes
    );
}

fn dynamic_set_descriptor(
    upper_bound: Option<DimensionExpr>,
    _current: u64,
) -> ResolvedValueDescriptor {
    let schema = SchemaDraft {
        dimension_parameters: Box::new([]),
        body: SchemaBody::Set {
            element: Box::new(SchemaBody::Index),
            cardinality: CardinalitySpec::Dynamic { upper_bound },
        },
    }
    .finalize()
    .unwrap();
    let shape = schema.instantiate_shape(Box::new([])).unwrap();
    ResolvedValueDescriptor::from_schema(schema, shape).unwrap()
}

#[test]
fn dynamic_collection_capacity_distinguishes_bounded_and_unbounded_growth() {
    let target = TargetMemoryProfile::current_direct_host().unwrap();
    let bounded = dynamic_set_descriptor(Some(DimensionExpr::Constant(8)), 3);
    let unbounded = dynamic_set_descriptor(None, 3);
    let storage = physical_storage_descriptor(
        FunctionValueRepresentation::Set,
        &target,
        MemoryLifetime::Activation,
    );
    let bounded_plan = plan_value_layout(ValueLayoutPlanningRequest {
        descriptor: &bounded,
        storage: &storage,
        witness: known(3, 24, 4),
        target: &target,
    })
    .unwrap();
    assert_eq!(bounded_plan.capacity_elements.required, 8);
    assert_eq!(
        bounded_plan.capacity_elements.growth,
        GrowthPolicy::ReservedToBound
    );

    let unbounded_plan = plan_value_layout(ValueLayoutPlanningRequest {
        descriptor: &unbounded,
        storage: &storage,
        witness: known(3, 24, 4),
        target: &target,
    })
    .unwrap();
    assert_eq!(unbounded_plan.capacity_elements.required, 3);
    assert_eq!(
        unbounded_plan.capacity_elements.growth,
        GrowthPolicy::ReplanBeforeGrowth
    );

    assert!(matches!(
        plan_value_layout(ValueLayoutPlanningRequest {
            descriptor: &bounded,
            storage: &storage,
            witness: known(9, 72, 10),
            target: &target,
        }),
        Err(MemoryPlanError::DynamicCardinalityExceedsBound {
            current: 9,
            maximum: 8
        })
    ));
}

#[test]
fn compound_dimension_bounds_use_checked_add_multiply_min_and_max() {
    let parameter = DimensionParameterId::new(0);
    let schema = SchemaDraft {
        dimension_parameters: vec![DimensionParameterDeclaration {
            id: parameter,
            origin: DimensionParameterOrigin::Explicit,
            lifetime: DimensionLifetime::Turn,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: Some(DimensionExpr::Constant(5)),
        }]
        .into_boxed_slice(),
        body: SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: vec![
                DimensionExpr::Parameter(parameter),
                DimensionExpr::Constant(1),
            ]
            .into_boxed_slice(),
        },
    }
    .finalize()
    .unwrap();
    let shape = schema
        .instantiate_shape(vec![3].into_boxed_slice())
        .unwrap();
    let p = DimensionExpr::Parameter(parameter);
    let expressions = [
        (
            DimensionExpr::Add(vec![p.clone(), DimensionExpr::Constant(2)].into_boxed_slice()),
            5,
            7,
        ),
        (
            DimensionExpr::Multiply(vec![p.clone(), DimensionExpr::Constant(2)].into_boxed_slice()),
            6,
            10,
        ),
        (
            DimensionExpr::Min(vec![p.clone(), DimensionExpr::Constant(4)].into_boxed_slice()),
            3,
            4,
        ),
        (
            DimensionExpr::Max(vec![p, DimensionExpr::Constant(4)].into_boxed_slice()),
            4,
            5,
        ),
    ];
    for (expression, current, maximum) in expressions {
        let capacity = derive_dimension_capacity(&schema, &shape, &expression).unwrap();
        assert_eq!(
            (capacity.current, capacity.maximum),
            (current, Some(maximum))
        );
    }
}

#[test]
fn a_finite_min_turn_expression_is_bounded_even_with_an_unbounded_operand() {
    let parameter = DimensionParameterId::new(0);
    let schema = SchemaDraft {
        dimension_parameters: vec![DimensionParameterDeclaration {
            id: parameter,
            origin: DimensionParameterOrigin::Explicit,
            lifetime: DimensionLifetime::Turn,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: None,
        }]
        .into_boxed_slice(),
        body: SchemaBody::Matrix {
            element: Box::new(SchemaBody::Index),
            dimensions: vec![
                DimensionExpr::Parameter(parameter),
                DimensionExpr::Constant(1),
            ]
            .into_boxed_slice(),
        },
    }
    .finalize()
    .unwrap();
    let shape = schema
        .instantiate_shape(vec![3].into_boxed_slice())
        .unwrap();
    let capacity = derive_dimension_capacity(
        &schema,
        &shape,
        &DimensionExpr::Min(
            vec![
                DimensionExpr::Parameter(parameter),
                DimensionExpr::Constant(100),
            ]
            .into_boxed_slice(),
        ),
    )
    .unwrap();
    assert_eq!(capacity.maximum, Some(100));
    assert_eq!(capacity.evolution, ExtentEvolution::TurnBounded);
}

#[test]
fn target_policy_limits_do_not_rewrite_semantic_capacity() {
    let mut target = TargetMemoryProfile::current_direct_host().unwrap();
    target.limits.max_output_elements = Some(1);
    let value = ValueCell::from_exact(7_u64).unwrap();
    let layout = plan(&value, &target, known(1, 0, 0)).unwrap();
    assert_eq!(
        layout.capacity_elements.authority,
        CapacityAuthority::ExactSemantic
    );
    assert_eq!(layout.capacity_elements.maximum, Some(1));
    assert_eq!(layout.slot, target.primitives.u64_slot);
    assert!(matches!(
        layout.storage,
        mech_core::StorageLayoutClass::Scalar {
            slot: PlannedSlotKind::FixedScalar(_)
        }
    ));
}

fn scalar_call_plan(
    construction: OutputConstruction,
    alias: AliasPolicy,
    change_detection: ChangeDetectionPolicy,
    implementation_memory: ImplementationMemoryClass,
    lifetime: MemoryLifetime,
) -> Result<CallMemoryPlan, MemoryPlanError> {
    let input = ValueCell::from_exact(1.0_f64).unwrap();
    let output = ValueCell::from_exact(2.0_f64).unwrap();
    let input_descriptor = input.resolved_descriptor().unwrap();
    let output_descriptor = output.resolved_descriptor().unwrap();
    let operation = ResolvedOperationDescriptor::from_name(
        "test/r5-scalar-plan",
        OperationContractDeclaration {
            inputs: InputPortLayout::Fixed(
                vec![InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                }]
                .into_boxed_slice(),
            ),
            outputs: vec![OutputPortPolicy {
                access: AccessMode::Write,
                delivery: DeliveryMode::Signal,
                construction,
                alias,
                change_detection,
            }]
            .into_boxed_slice(),
            interaction: ExternalInteraction::Pure,
        },
    )
    .unwrap();
    let call = BoundCall::syntax_directed(
        operation,
        vec![input_descriptor.clone()].into_boxed_slice(),
        vec![output_descriptor.clone()].into_boxed_slice(),
        RuntimeFunctionId::from_name("R5ScalarPlan"),
        ExecutionTarget::DirectRuntime,
    )
    .unwrap();
    let target = TargetMemoryProfile::current_direct_host().unwrap();
    let input_storage = physical_storage_descriptor(input.representation(), &target, lifetime);
    let output_storage = physical_storage_descriptor(output.representation(), &target, lifetime);
    let witness = MemoryFootprintWitness::Known(CurrentMemoryFootprint {
        logical_elements: 1,
        fixed_bytes: 8,
        encoded_bytes: 8,
        retained_nodes: 1,
        schema_bytes: 1,
        shape_parameter_count: 0,
        ..CurrentMemoryFootprint::default()
    });
    plan_call_memory(CallMemoryPlanningRequest {
        bound_call: &call,
        input_storage: &[input_storage],
        output_storage: &[output_storage],
        input_witnesses: &[witness],
        output_witnesses: &[witness],
        implementation_memory,
        target: &target,
        regions: &[RegionAccessPlan::WholeValue],
    })
}

#[test]
fn call_plans_cover_full_replace_build_and_read_modify_write_publication() {
    let constructions = [
        OutputConstruction::FullWrite {
            shape: ShapeRule::Declared,
        },
        OutputConstruction::Replace {
            shape: ShapeRule::Declared,
        },
        OutputConstruction::Build {
            postcondition: ShapeContractReference {
                module_path: vec!["test".into()].into_boxed_slice(),
                contract_name: "r5-build".into(),
            },
        },
        OutputConstruction::ReadModifyWrite {
            base_input: 0,
            regions: RegionPolicy::WholeValue,
        },
    ];
    for construction in constructions {
        let read_modify_write = matches!(construction, OutputConstruction::ReadModifyWrite { .. });
        let plan = scalar_call_plan(
            construction,
            AliasPolicy::NoAlias,
            ChangeDetectionPolicy::AlwaysChanged,
            ImplementationMemoryClass::NoAdditionalScratch,
            MemoryLifetime::Activation,
        )
        .unwrap();
        assert_eq!(
            plan.aliases[0],
            AliasDecision::StageThenPublish { input: None }
        );
        assert!(matches!(
            plan.transactions[0],
            TransactionRequirement::StageAndSwap { .. }
        ));
        assert_eq!(plan.demand.cloned_bytes != 0, read_modify_write);
    }
}

#[test]
fn resolved_external_contract_replans_every_derived_call_field() {
    let initial = scalar_call_plan(
        OutputConstruction::FullWrite {
            shape: ShapeRule::Declared,
        },
        AliasPolicy::NoAlias,
        ChangeDetectionPolicy::AlwaysChanged,
        ImplementationMemoryClass::NoAdditionalScratch,
        MemoryLifetime::Activation,
    )
    .unwrap();
    let mut rebound = initial.bound_call.clone();
    rebound
        .resolve_operation_contract(&OperationContractDeclaration {
            inputs: InputPortLayout::Fixed(
                vec![InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                }]
                .into_boxed_slice(),
            ),
            outputs: vec![OutputPortPolicy {
                access: AccessMode::ReadWrite,
                delivery: DeliveryMode::Signal,
                construction: OutputConstruction::ReadModifyWrite {
                    base_input: 0,
                    regions: RegionPolicy::WholeValue,
                },
                alias: AliasPolicy::MayAlias { input: 0 },
                change_detection: ChangeDetectionPolicy::SemanticHash,
            }]
            .into_boxed_slice(),
            interaction: ExternalInteraction::Pure,
        })
        .unwrap();

    // Provider-deferred contracts can expose fewer provisional memory ports
    // than the already-resolved semantic call. Replanning must retain the
    // requested region authority independently from those provisional ports.
    let mut provisional = initial.clone();
    provisional.outputs = Box::new([]);
    let replanned = replan_call_memory(&provisional, &rebound).unwrap();
    assert_eq!(replanned.bound_call, rebound);
    assert_ne!(replanned.aliases, initial.aliases);
    assert_ne!(replanned.demand, initial.demand);
    assert_eq!(replanned.input_storage, initial.input_storage);
    assert_eq!(replanned.output_storage, initial.output_storage);
    assert_eq!(replanned.output_regions, initial.output_regions);
    assert_eq!(replanned.target, initial.target);
}

#[test]
fn alias_plans_choose_safe_reuse_staging_and_required_undo() {
    let construction = || OutputConstruction::ReadModifyWrite {
        base_input: 0,
        regions: RegionPolicy::WholeValue,
    };
    let reusable = scalar_call_plan(
        construction(),
        AliasPolicy::MayAlias { input: 0 },
        ChangeDetectionPolicy::ExactScalar,
        ImplementationMemoryClass::NoAdditionalScratch,
        MemoryLifetime::Turn {
            first: mech_core::MemoryPlanPoint::new(0),
            last: mech_core::MemoryPlanPoint::new(1),
        },
    )
    .unwrap();
    assert_eq!(reusable.aliases[0], AliasDecision::ReuseInput { input: 0 });

    let staged = scalar_call_plan(
        construction(),
        AliasPolicy::MayAlias { input: 0 },
        ChangeDetectionPolicy::ExactScalar,
        ImplementationMemoryClass::NoAdditionalScratch,
        MemoryLifetime::Activation,
    )
    .unwrap();
    assert_eq!(
        staged.aliases[0],
        AliasDecision::StageThenPublish { input: Some(0) }
    );

    let in_place = scalar_call_plan(
        construction(),
        AliasPolicy::InPlaceRequired { input: 0 },
        ChangeDetectionPolicy::ExactScalar,
        ImplementationMemoryClass::NoAdditionalScratch,
        MemoryLifetime::Activation,
    )
    .unwrap();
    assert_eq!(
        in_place.aliases[0],
        AliasDecision::InPlaceRequired { input: 0 }
    );
    assert!(matches!(
        in_place.transactions[0],
        TransactionRequirement::UndoSnapshot { .. }
    ));
}

#[test]
fn change_detection_and_closed_implementation_classes_contribute_exact_work() {
    for (policy, has_comparison) in [
        (ChangeDetectionPolicy::KernelReported, false),
        (ChangeDetectionPolicy::AlwaysChanged, false),
        (ChangeDetectionPolicy::ExactScalar, true),
        (ChangeDetectionPolicy::SemanticHash, true),
    ] {
        let plan = scalar_call_plan(
            OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            AliasPolicy::NoAlias,
            policy,
            ImplementationMemoryClass::NoAdditionalScratch,
            MemoryLifetime::Activation,
        )
        .unwrap();
        assert_eq!(plan.demand.work.comparison != 0, has_comparison);
    }

    let clone = scalar_call_plan(
        OutputConstruction::FullWrite {
            shape: ShapeRule::Declared,
        },
        AliasPolicy::NoAlias,
        ChangeDetectionPolicy::AlwaysChanged,
        ImplementationMemoryClass::CloneInput { input: 0 },
        MemoryLifetime::Activation,
    )
    .unwrap();
    assert_eq!(clone.demand.cloned_bytes, 8);

    for class in [
        ImplementationMemoryClass::CanonicalFinalize,
        ImplementationMemoryClass::CanonicalSortUnique,
    ] {
        let plan = scalar_call_plan(
            OutputConstruction::Build {
                postcondition: ShapeContractReference {
                    module_path: Box::new([]),
                    contract_name: "canonical".into(),
                },
            },
            AliasPolicy::NoAlias,
            ChangeDetectionPolicy::SemanticHash,
            class,
            MemoryLifetime::Activation,
        )
        .unwrap();
        assert_ne!(plan.demand.work.canonicalization, 0);
        if class == ImplementationMemoryClass::CanonicalSortUnique {
            assert_ne!(plan.demand.work.comparison, 0);
        }
    }
}

#[test]
fn deferred_footprints_rederive_clone_hash_and_canonical_demand() {
    let input = ValueCell::from_exact("input".to_owned()).unwrap();
    let output = ValueCell::from_exact("output".to_owned()).unwrap();
    let input_descriptor = input.resolved_descriptor().unwrap();
    let output_descriptor = output.resolved_descriptor().unwrap();
    let operation = ResolvedOperationDescriptor::from_name(
        "test/r5-deferred-canonical",
        OperationContractDeclaration {
            inputs: InputPortLayout::Fixed(
                vec![InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                }]
                .into_boxed_slice(),
            ),
            outputs: vec![OutputPortPolicy {
                access: AccessMode::Write,
                delivery: DeliveryMode::Signal,
                construction: OutputConstruction::Build {
                    postcondition: ShapeContractReference {
                        module_path: Box::new([]),
                        contract_name: "canonical".into(),
                    },
                },
                alias: AliasPolicy::NoAlias,
                change_detection: ChangeDetectionPolicy::SemanticHash,
            }]
            .into_boxed_slice(),
            interaction: ExternalInteraction::Pure,
        },
    )
    .unwrap();
    let call = BoundCall::syntax_directed(
        operation,
        vec![input_descriptor.clone()].into_boxed_slice(),
        vec![output_descriptor.clone()].into_boxed_slice(),
        RuntimeFunctionId::from_name("R5DeferredCanonical"),
        ExecutionTarget::ResidentCpu,
    )
    .unwrap();
    let target = TargetMemoryProfile::current_resident_cpu().unwrap();
    let lifetime = MemoryLifetime::Turn {
        first: mech_core::MemoryPlanPoint::new(0),
        last: mech_core::MemoryPlanPoint::new(1),
    };
    let input_storage = physical_storage_descriptor(input.representation(), &target, lifetime);
    let output_storage = physical_storage_descriptor(output.representation(), &target, lifetime);
    let deferred = MemoryFootprintWitness::Deferred(mech_core::MemoryWitnessStage::Turn);
    let plan = plan_call_memory(CallMemoryPlanningRequest {
        bound_call: &call,
        input_storage: &[input_storage],
        output_storage: &[output_storage],
        input_witnesses: &[deferred],
        output_witnesses: &[deferred],
        implementation_memory: ImplementationMemoryClass::CanonicalSortUnique,
        target: &target,
        regions: &[RegionAccessPlan::WholeValue],
    })
    .unwrap();
    let resolved = BTreeMap::from([
        (
            (mech_core::PortDirection::Input, 0),
            CurrentMemoryFootprint {
                logical_elements: 1,
                payload_bytes: 11,
                encoded_bytes: 17,
                retained_nodes: 3,
                schema_bytes: 5,
                shape_parameter_count: 2,
                ..CurrentMemoryFootprint::default()
            },
        ),
        (
            (mech_core::PortDirection::Output, 0),
            CurrentMemoryFootprint {
                logical_elements: 1,
                payload_bytes: 13,
                encoded_bytes: 19,
                retained_nodes: 4,
                schema_bytes: 7,
                shape_parameter_count: 3,
                ..CurrentMemoryFootprint::default()
            },
        ),
    ]);
    let demand = resolve_deferred_call_demand(&plan, &resolved).unwrap();
    assert!(demand.turn_peak_bytes > plan.demand.turn_peak_bytes);
    assert!(demand.transaction_peak_bytes > plan.demand.transaction_peak_bytes);
    assert!(demand.retained_nodes > plan.demand.retained_nodes);
    assert!(demand.work.comparison > plan.demand.work.comparison);
    assert!(demand.work.canonicalization > plan.demand.work.canonicalization);

    let clone_plan = plan_call_memory(CallMemoryPlanningRequest {
        bound_call: &call,
        input_storage: &[physical_storage_descriptor(
            input.representation(),
            &target,
            lifetime,
        )],
        output_storage: &[physical_storage_descriptor(
            output.representation(),
            &target,
            lifetime,
        )],
        input_witnesses: &[deferred],
        output_witnesses: &[deferred],
        implementation_memory: ImplementationMemoryClass::CloneInput { input: 0 },
        target: &target,
        regions: &[RegionAccessPlan::WholeValue],
    })
    .unwrap();
    let clone_demand = resolve_deferred_call_demand(&clone_plan, &resolved).unwrap();
    assert_eq!(
        clone_demand.cloned_bytes - clone_plan.demand.cloned_bytes,
        11
    );
}

#[test]
fn unit_external_effect_has_no_storage_transaction() {
    let operation = ResolvedOperationDescriptor::from_name(
        "test/r5-unit-effect",
        OperationContractDeclaration {
            inputs: InputPortLayout::Fixed(Box::new([])),
            outputs: Box::new([]),
            interaction: ExternalInteraction::Effect(EffectContract {
                delivery: EffectDeliveryPolicy::AtMostOnce,
                idempotency: IdempotencyRequirement::NotRequired,
            }),
        },
    )
    .unwrap();
    let call = BoundCall::syntax_directed(
        operation,
        Box::new([]),
        Box::new([]),
        RuntimeFunctionId::from_name("R5UnitEffect"),
        ExecutionTarget::DirectRuntime,
    )
    .unwrap();
    let target = TargetMemoryProfile::current_direct_host().unwrap();
    let plan = plan_call_memory(CallMemoryPlanningRequest {
        bound_call: &call,
        input_storage: &[],
        output_storage: &[],
        input_witnesses: &[],
        output_witnesses: &[],
        implementation_memory: ImplementationMemoryClass::NoAdditionalScratch,
        target: &target,
        regions: &[],
    })
    .unwrap();
    assert!(plan.allocations.is_empty());
    assert!(plan.transactions.is_empty());
    assert_eq!(plan.demand, mech_core::ResourceDemand::default());
}

#[test]
fn matrix_solve_and_indexed_mutation_have_explicit_scratch_and_regions() {
    let coefficients = ValueCell::from_exact_matrix_ref(
        Ref::new(DMatrix::<f64>::from_vec(2, 2, vec![2.0, 0.0, 0.0, 2.0])),
        2,
        2,
    )
    .unwrap();
    let rhs = ValueCell::from_exact_matrix_ref(
        Ref::new(DMatrix::<f64>::from_vec(2, 1, vec![2.0, 4.0])),
        2,
        1,
    )
    .unwrap();
    let output = ValueCell::from_exact_matrix_ref(
        Ref::new(DMatrix::<f64>::from_vec(2, 1, vec![0.0, 0.0])),
        2,
        1,
    )
    .unwrap();
    let inputs = [
        coefficients.resolved_descriptor().unwrap(),
        rhs.resolved_descriptor().unwrap(),
    ];
    let outputs = [output.resolved_descriptor().unwrap()];
    let operation = ResolvedOperationDescriptor::from_name(
        "test/r5-matrix-solve",
        OperationContractDeclaration {
            inputs: InputPortLayout::Fixed(
                vec![
                    InputPortPolicy {
                        access: AccessMode::Read,
                        delivery: DeliveryMode::Signal,
                    },
                    InputPortPolicy {
                        access: AccessMode::Read,
                        delivery: DeliveryMode::Signal,
                    },
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
                change_detection: ChangeDetectionPolicy::SemanticHash,
            }]
            .into_boxed_slice(),
            interaction: ExternalInteraction::Pure,
        },
    )
    .unwrap();
    let call = BoundCall::syntax_directed(
        operation,
        inputs.clone().into(),
        outputs.clone().into(),
        RuntimeFunctionId::from_name("R5MatrixSolve"),
        ExecutionTarget::DirectRuntime,
    )
    .unwrap();
    let target = TargetMemoryProfile::current_direct_host().unwrap();
    let lifetime = MemoryLifetime::Activation;
    let input_storage = [
        physical_storage_descriptor(coefficients.representation(), &target, lifetime),
        physical_storage_descriptor(rhs.representation(), &target, lifetime),
    ];
    let output_storage = [physical_storage_descriptor(
        output.representation(),
        &target,
        lifetime,
    )];
    let footprints = [known(4, 0, 0), known(2, 0, 0)];
    let plan = plan_call_memory(CallMemoryPlanningRequest {
        bound_call: &call,
        input_storage: &input_storage,
        output_storage: &output_storage,
        input_witnesses: &footprints,
        output_witnesses: &[known(2, 0, 0)],
        implementation_memory: ImplementationMemoryClass::MatrixSolve,
        target: &target,
        regions: &[RegionAccessPlan::WholeValue],
    })
    .unwrap();
    assert_eq!(
        plan.implementation_memory,
        ImplementationMemoryClass::MatrixSolve
    );
    assert_ne!(plan.demand.work.compute, 0);
    assert_ne!(plan.demand.cloned_bytes, 0);
    let scratch = plan
        .allocations
        .iter()
        .filter(|a| matches!(a.owner, mech_core::MemoryObjectOwner::NodeScratch { .. }))
        .collect::<Vec<_>>();
    assert_eq!(scratch.len(), 3);
    assert_eq!(
        scratch.iter().map(|a| a.capacity_bytes).collect::<Vec<_>>(),
        vec![32, 16, 16]
    );
    assert_eq!(scratch[2].role, mech_core::AllocationRole::OrderedIndex);
    for pair in scratch.windows(2) {
        assert!(pair[0].placement.offset + pair[0].capacity_bytes <= pair[1].placement.offset);
    }
    assert_eq!(
        plan.demand.turn_peak_bytes,
        plan.demand.transaction_peak_bytes + 64
    );

    let indexed_operation = ResolvedOperationDescriptor::from_name(
        "test/r5-indexed-mutation",
        OperationContractDeclaration {
            inputs: InputPortLayout::Fixed(
                vec![InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                }]
                .into_boxed_slice(),
            ),
            outputs: vec![OutputPortPolicy {
                access: AccessMode::ReadWrite,
                delivery: DeliveryMode::Signal,
                construction: OutputConstruction::ReadModifyWrite {
                    base_input: 0,
                    regions: RegionPolicy::SingleElement,
                },
                alias: AliasPolicy::MayAlias { input: 0 },
                change_detection: ChangeDetectionPolicy::SemanticHash,
            }]
            .into_boxed_slice(),
            interaction: ExternalInteraction::Pure,
        },
    )
    .unwrap();
    let indexed_call = BoundCall::syntax_directed(
        indexed_operation,
        vec![inputs[1].clone()].into_boxed_slice(),
        vec![outputs[0].clone()].into_boxed_slice(),
        RuntimeFunctionId::from_name("R5IndexedMutation"),
        ExecutionTarget::DirectRuntime,
    )
    .unwrap();
    let indexed = plan_call_memory(CallMemoryPlanningRequest {
        bound_call: &indexed_call,
        input_storage: &input_storage[1..],
        output_storage: &output_storage,
        input_witnesses: &[known(2, 0, 0)],
        output_witnesses: &[known(2, 0, 0)],
        implementation_memory: ImplementationMemoryClass::NoAdditionalScratch,
        target: &target,
        regions: &[RegionAccessPlan::Gather {
            selected_elements: 1,
            index_bytes: 8,
        }],
    })
    .unwrap();
    assert!(matches!(
        indexed.outputs[0].region,
        RegionAccessPlan::Gather {
            selected_elements: 1,
            index_bytes: 8
        }
    ));
    assert_ne!(indexed.demand.cloned_bytes, 0);
}

#[test]
fn implementation_scratch_records_are_the_source_of_temporary_byte_demand() {
    for (class, count) in [
        (ImplementationMemoryClass::CloneInput { input: 0 }, 1),
        (ImplementationMemoryClass::CanonicalFinalize, 2),
        (ImplementationMemoryClass::CanonicalSortUnique, 3),
    ] {
        let plan = scalar_call_plan(
            OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            AliasPolicy::NoAlias,
            ChangeDetectionPolicy::AlwaysChanged,
            class,
            MemoryLifetime::Activation,
        )
        .unwrap();
        let scratch = plan
            .allocations
            .iter()
            .filter(|a| matches!(a.owner, mech_core::MemoryObjectOwner::NodeScratch { .. }))
            .collect::<Vec<_>>();
        assert_eq!(scratch.len(), count);
        let bytes: u64 = scratch.iter().map(|a| a.capacity_bytes).sum();
        assert_eq!(
            plan.demand.turn_peak_bytes,
            plan.demand.transaction_peak_bytes + bytes
        );
        assert!(
            scratch
                .iter()
                .all(|a| matches!(a.lifetime, MemoryLifetime::Turn { .. }))
        );
    }
}

#[test]
fn publication_comparison_keeps_old_and_candidate_sizes_distinct() {
    let old = CurrentMemoryFootprint {
        encoded_bytes: 1000,
        schema_bytes: 10,
        shape_parameter_count: 2,
        ..CurrentMemoryFootprint::default()
    };
    let next = CurrentMemoryFootprint {
        encoded_bytes: 5,
        schema_bytes: 10,
        shape_parameter_count: 2,
        ..CurrentMemoryFootprint::default()
    };
    assert_eq!(
        mech_core::publication_comparison_work(old, next).unwrap(),
        1041
    );
    assert_eq!(
        mech_core::publication_comparison_work(next, old).unwrap(),
        1041
    );
}
