#![cfg(feature = "full")]

use mech_core::{
    snapshot::{
        EnumDraft, F64Bits, MapEntryDraft, NamedValueDraft, OptionDraft, ReifiedTypeDraft,
        SnapshotValidationContext, TableColumnDraft,
    },
    *,
};
use nalgebra::{DMatrix, DVector, RowDVector};
use std::rc::Rc;

#[derive(Debug)]
struct MemoryBoundaryCase {
    name: &'static str,
    schema: Schema,
    shape: ShapeInstance,
    storage: StorageCapabilityDescriptor,
    requirement: PortMemoryRequirement,
    expected: Result<(), PortStorageCompatibilityError>,
}

fn schema_shape(body: SchemaBody) -> (Schema, ShapeInstance) {
    parameterized_schema_shape(Vec::new(), body, Vec::new())
}

fn parameterized_schema_shape(
    parameters: Vec<DimensionParameterDeclaration>,
    body: SchemaBody,
    values: Vec<u64>,
) -> (Schema, ShapeInstance) {
    let schema = SchemaDraft {
        dimension_parameters: parameters.into_boxed_slice(),
        body,
    }
    .finalize()
    .unwrap();
    let shape = schema.instantiate_shape(values.into_boxed_slice()).unwrap();
    (schema, shape)
}

fn canonical_storage() -> StorageCapabilityDescriptor {
    ValueCell::unit().storage_capabilities()
}

fn derive_input_requirement(access: AccessMode, delivery: DeliveryMode) -> PortMemoryRequirement {
    PortMemoryRequirement::for_input(InputPortPolicy { access, delivery })
}

fn derive_output_requirement(
    construction: OutputConstruction,
    alias: AliasPolicy,
    change_detection: ChangeDetectionPolicy,
) -> PortMemoryRequirement {
    PortMemoryRequirement::for_output(&OutputPortPolicy {
        access: match construction {
            OutputConstruction::ReadModifyWrite { .. } => AccessMode::ReadWrite,
            _ => AccessMode::Write,
        },
        delivery: DeliveryMode::Signal,
        construction,
        alias,
        change_detection,
    })
    .unwrap()
}

fn full_output(alias: AliasPolicy) -> OutputPortPolicy {
    OutputPortPolicy {
        access: AccessMode::Write,
        delivery: DeliveryMode::Signal,
        construction: OutputConstruction::FullWrite {
            shape: ShapeRule::Declared,
        },
        alias,
        change_detection: ChangeDetectionPolicy::KernelReported,
    }
}

fn declaration(
    inputs: Vec<InputPortPolicy>,
    outputs: Vec<OutputPortPolicy>,
) -> OperationContractDeclaration {
    OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(inputs.into_boxed_slice()),
        outputs: outputs.into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    }
}

fn f64_body() -> SchemaBody {
    SchemaBody::FloatingPoint(FloatWidth::W64)
}

fn matrix_body(element: SchemaBody, rows: u64, columns: u64) -> SchemaBody {
    SchemaBody::Matrix {
        element: Box::new(element),
        dimensions: vec![
            DimensionExpr::Constant(rows),
            DimensionExpr::Constant(columns),
        ]
        .into_boxed_slice(),
    }
}

fn record_body() -> SchemaBody {
    SchemaBody::Record(
        vec![SchemaField {
            name: "value".into(),
            schema: f64_body(),
        }]
        .into_boxed_slice(),
    )
}

fn table_body() -> SchemaBody {
    SchemaBody::Table {
        columns: vec![SchemaField {
            name: "value".into(),
            schema: f64_body(),
        }]
        .into_boxed_slice(),
        rows: DimensionExpr::Constant(1).into(),
    }
}

fn set_body() -> SchemaBody {
    SchemaBody::Set {
        element: Box::new(f64_body()),
        cardinality: DimensionExpr::Constant(1).into(),
    }
}

fn map_body() -> SchemaBody {
    SchemaBody::Map {
        key: Box::new(SchemaBody::String),
        value: Box::new(f64_body()),
        cardinality: DimensionExpr::Constant(1).into(),
    }
}

fn semantic_error(
    schema: &Schema,
    shape: &ShapeInstance,
    required: AddressingRequirement,
) -> PortStorageCompatibilityError {
    PortStorageCompatibilityError::SemanticAddressingUnsupported {
        required,
        available: schema
            .resolved_type_memory_contract(shape)
            .unwrap()
            .addressing,
    }
}

fn storage_error(error: StorageCompatibilityError) -> PortStorageCompatibilityError {
    PortStorageCompatibilityError::SchemaStorage(SchemaStorageCompatibilityError::Storage(error))
}

fn run(cases: Vec<MemoryBoundaryCase>) {
    for case in cases {
        assert_eq!(
            check_port_storage_compatibility(
                &case.schema,
                &case.shape,
                &case.requirement,
                &case.storage,
            ),
            case.expected,
            "memory-boundary case {:?}",
            case.name,
        );
    }
}

fn case(
    name: &'static str,
    body: SchemaBody,
    requirement: PortMemoryRequirement,
    storage: StorageCapabilityDescriptor,
    expected: Result<(), PortStorageCompatibilityError>,
) -> MemoryBoundaryCase {
    let (schema, shape) = schema_shape(body);
    MemoryBoundaryCase {
        name,
        schema,
        shape,
        storage,
        requirement,
        expected,
    }
}

#[rustfmt::skip]
fn canonical_cell(body: SchemaBody, data: ValueDataDraft) -> ValueCell {
    let schema = SchemaDraft { dimension_parameters: Box::new([]), body }.finalize().unwrap();
    let mut builder = SchemaTableBuilder::new();
    let handle = builder.insert(schema).unwrap();
    let build = builder.finish().unwrap();
    let schema = build.resolve(handle).unwrap();
    let (schemas, _) = build.into_parts();
    let schemas = Rc::new(schemas);
    let value = ValueDraft { schema, shape_values: Box::new([]), data }
        .finalize(&SnapshotValidationContext::new(&schemas)).unwrap();
    ValueCell::from_value(value, schemas).unwrap()
}

#[test]
#[rustfmt::skip]
fn canonical_storage_is_universal_mechanics_not_universal_semantics() {
    let storage = canonical_storage();
    let read = derive_input_requirement(AccessMode::Read, DeliveryMode::Signal);
    let enum_body = SchemaBody::Enum {
        key: NominalKey::from_bytes([7; 32]),
        variants: vec![
            EnumVariantSchema { name: "none".into(), payload: None },
            EnumVariantSchema { name: "some".into(), payload: Some(f64_body()) },
        ]
        .into_boxed_slice(),
    };
    let accepted = vec![
        ("bool", SchemaBody::Bool, ValueDataDraft::Bool(true)),
        ("f64", f64_body(), ValueDataDraft::F64(F64Bits::from_f64(1.0))),
        ("string", SchemaBody::String, ValueDataDraft::String("value".into())),
        ("option", SchemaBody::Option(Box::new(f64_body())), ValueDataDraft::Option(OptionDraft { present: true, value: Some(Box::new(ValueDataDraft::F64(F64Bits::from_f64(1.0)))) })),
        ("enum", enum_body.clone(), ValueDataDraft::Enum(EnumDraft { ordinal: 1, payload: Some(Box::new(ValueDataDraft::F64(F64Bits::from_f64(1.0)))) })),
        ("tuple", SchemaBody::Tuple(vec![f64_body(), SchemaBody::String].into_boxed_slice()), ValueDataDraft::Tuple(vec![ValueDataDraft::F64(F64Bits::from_f64(1.0)), ValueDataDraft::String("value".into())].into_boxed_slice())),
        ("record", record_body(), ValueDataDraft::Record(vec![NamedValueDraft { name: "value".into(), value: ValueDataDraft::F64(F64Bits::from_f64(1.0)) }].into_boxed_slice())),
        ("f64-matrix", matrix_body(f64_body(), 2, 3), ValueDataDraft::Matrix(vec![ValueDataDraft::F64(F64Bits::from_f64(1.0)); 6].into_boxed_slice())),
        ("string-matrix", matrix_body(SchemaBody::String, 1, 2), ValueDataDraft::Matrix(vec![ValueDataDraft::String("a".into()), ValueDataDraft::String("b".into())].into_boxed_slice())),
        ("table", table_body(), ValueDataDraft::Table(vec![TableColumnDraft { name: "value".into(), values: vec![ValueDataDraft::F64(F64Bits::from_f64(1.0))].into_boxed_slice() }].into_boxed_slice())),
        ("set", set_body(), ValueDataDraft::Set(vec![ValueDataDraft::F64(F64Bits::from_f64(1.0))].into_boxed_slice())),
        ("map", map_body(), ValueDataDraft::Map(vec![MapEntryDraft { items: vec![ValueDataDraft::String("key".into()), ValueDataDraft::F64(F64Bits::from_f64(1.0))].into_boxed_slice() }].into_boxed_slice())),
        ("dynamic", SchemaBody::Dynamic, ValueDataDraft::Dynamic(None)),
        ("reified-type", SchemaBody::ReifiedType, ValueDataDraft::Type(ReifiedTypeDraft::Schema(SchemaKey::from_bytes([9; 32])))),
    ];
    let mut cases = accepted.into_iter()
        .map(|(name, body, data)| {
            let actual_storage = canonical_cell(body.clone(), data).storage_capabilities();
            case(name, body, read.clone(), actual_storage, Ok(()))
        })
        .collect::<Vec<_>>();
    for (name, body, required) in [
        ("scalar-positional", f64_body(), AddressingRequirement::Positional { minimum_rank: 1 }),
        ("scalar-collection", f64_body(), AddressingRequirement::CollectionEntry),
        ("scalar-arbitrary", f64_body(), AddressingRequirement::ArbitraryRegion),
        ("record-positional", record_body(), AddressingRequirement::Positional { minimum_rank: 1 }),
        ("option-collection", SchemaBody::Option(Box::new(f64_body())), AddressingRequirement::CollectionEntry),
    ] {
        let (schema, shape) = schema_shape(body);
        cases.push(MemoryBoundaryCase {
            name,
            expected: Err(semantic_error(&schema, &shape, required)),
            schema,
            shape,
            storage: storage.clone(),
            requirement: PortMemoryRequirement { addressing: required, ..read.clone() },
        });
    }
    run(cases);
}

#[test]
#[rustfmt::skip]
fn semantic_addressing_precedes_backing_addressing() {
    let storage = canonical_storage();
    let read = derive_input_requirement(AccessMode::Read, DeliveryMode::Signal);
    let mut cases = Vec::new();
    for (name, body, addressing) in [
        ("string-positional", SchemaBody::String, AddressingRequirement::Positional { minimum_rank: 1 }),
        ("string-arbitrary", SchemaBody::String, AddressingRequirement::ArbitraryRegion),
        ("matrix-positional-1", matrix_body(f64_body(), 2, 3), AddressingRequirement::Positional { minimum_rank: 1 }),
        ("matrix-positional-2", matrix_body(f64_body(), 2, 3), AddressingRequirement::Positional { minimum_rank: 2 }),
        ("matrix-arbitrary", matrix_body(f64_body(), 2, 3), AddressingRequirement::ArbitraryRegion),
    ] {
        let requirement = PortMemoryRequirement { addressing, ..read.clone() };
        cases.push(case(name, body, requirement, storage.clone(), Ok(())));
    }
    for (name, body) in [
        ("tuple-entry", SchemaBody::Tuple(vec![f64_body(), SchemaBody::String].into_boxed_slice())),
        ("record-entry", record_body()),
        ("table-entry", table_body()),
        ("set-entry", set_body()),
        ("map-entry", map_body()),
    ] {
        let requirement = PortMemoryRequirement {
            addressing: AddressingRequirement::CollectionEntry, ..read.clone()
        };
        cases.push(case(name, body, requirement, storage.clone(), Ok(())));
    }
    let mut no_arbitrary = storage.clone();
    no_arbitrary.addressing.arbitrary_regions = false;
    let arbitrary = PortMemoryRequirement {
        addressing: AddressingRequirement::ArbitraryRegion, ..read.clone()
    };
    cases.push(case("eligible-string-without-arbitrary-backing", SchemaBody::String,
        arbitrary, no_arbitrary, Err(PortStorageCompatibilityError::ArbitraryRegionUnsupported)));
    let mut no_whole = storage.clone();
    no_whole.addressing.whole_value = false;
    cases.push(case("eligible-f64-without-whole-backing", f64_body(), read.clone(),
        no_whole, Err(PortStorageCompatibilityError::WholeValueAddressingUnsupported)));
    let mut no_positional = storage;
    no_positional.addressing.positional = PositionalAddressingCapability::None;
    let positional = PortMemoryRequirement {
        addressing: AddressingRequirement::Positional { minimum_rank: 1 }, ..read
    };
    let (scalar_schema, scalar_shape) = schema_shape(f64_body());
    cases.push(MemoryBoundaryCase { name: "semantic-failure-precedes-deficient-backing",
        expected: Err(semantic_error(&scalar_schema, &scalar_shape, positional.addressing)),
        schema: scalar_schema, shape: scalar_shape, storage: no_positional.clone(), requirement: positional.clone() });
    cases.push(case("string-schema-storage-precedes-port-backing", SchemaBody::String,
        positional, no_positional,
        Err(storage_error(StorageCompatibilityError::PositionalAddressingUnsupported))));
    run(cases);
}

#[test]
#[rustfmt::skip]
fn exact_backings_preserve_kind_extent_and_evolution() {
    let read = derive_input_requirement(AccessMode::Read, DeliveryMode::Signal);
    let exact_f64 = ValueCell::from_exact(1_f64).unwrap().storage_capabilities();
    let exact_bool = ValueCell::from_exact(true).unwrap().storage_capabilities();
    let f64_matrix = ValueCell::from_exact(DMatrix::<f64>::zeros(2, 3)).unwrap().storage_capabilities();
    let strings = DMatrix::<String>::from_vec(1, 2, vec!["a".into(), "b".into()]);
    let string_matrix = ValueCell::from_exact(strings).unwrap().storage_capabilities();
    let bool_matrix = ValueCell::from_exact(DMatrix::<bool>::from_element(2, 3, false)).unwrap().storage_capabilities();
    let mut wrong_extent = f64_matrix.clone();
    wrong_extent.extent = StorageExtentCapability::FixedDimensions(vec![3, 2].into_boxed_slice());
    run(vec![
        case("exact-f64", f64_body(), read.clone(), exact_f64.clone(), Ok(())),
        case("exact-bool-for-f64", f64_body(), read.clone(), exact_bool, Err(storage_error(StorageCompatibilityError::ScalarKindMismatch))),
        case("exact-f64-matrix", matrix_body(f64_body(), 2, 3), read.clone(), f64_matrix.clone(), Ok(())),
        case("exact-string-matrix", matrix_body(SchemaBody::String, 1, 2), read.clone(), string_matrix, Ok(())),
        case("exact-bool-matrix-for-f64", matrix_body(f64_body(), 2, 3), read.clone(), bool_matrix, Err(storage_error(StorageCompatibilityError::DenseElementMismatch))),
        case("transposed-fixed-extent", matrix_body(f64_body(), 2, 3), read.clone(), wrong_extent, Err(storage_error(StorageCompatibilityError::AxisMismatch))),
        case("dynamic-with-exact-f64", SchemaBody::Dynamic, read.clone(), exact_f64, Err(storage_error(StorageCompatibilityError::TopologyMismatch))),
        case("dynamic-with-canonical", SchemaBody::Dynamic, read.clone(), canonical_storage(), Ok(())),
    ]);
    for (name, lifetime, expected) in [
        ("turn-bounded-fixed-axis", DimensionLifetime::Turn, Err(StorageCompatibilityError::DynamicAxisUnsupported)),
        ("turn-unbounded-fixed-axis", DimensionLifetime::Turn, Err(StorageCompatibilityError::DynamicAxisUnsupported)),
        ("activation-fixed-axis", DimensionLifetime::Activation, Ok(())),
    ] {
        let upper = (name != "turn-unbounded-fixed-axis").then_some(8);
        let (schema, shape) = parameterized_schema_shape(
            vec![DimensionParameterDeclaration {
                id: DimensionParameterId::new(0),
                origin: DimensionParameterOrigin::Explicit,
                lifetime,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: upper.map(DimensionExpr::Constant),
            }],
            SchemaBody::Matrix {
                element: Box::new(f64_body()),
                dimensions: vec![DimensionExpr::Parameter(DimensionParameterId::new(0)),
                    DimensionExpr::Constant(3)].into_boxed_slice(),
            },
            vec![2],
        );
        let fixed = StorageCapabilityDescriptor {
            extent: StorageExtentCapability::FixedDimensions(vec![2, 3].into_boxed_slice()),
            ..f64_matrix.clone()
        };
        let result = check_schema_storage_compatibility(&schema, &shape, &fixed);
        let result = result.map_err(|error| match error {
            SchemaStorageCompatibilityError::Storage(error) => error,
            SchemaStorageCompatibilityError::Semantic(error) => panic!("{name}: invalid fixture: {error:?}"),
        });
        assert_eq!(result, expected, "parameterized boundary case {name}");
    }
}

#[test]
#[rustfmt::skip]
fn port_capability_failures_remain_structured() {
    let f64 = f64_body();
    let string = SchemaBody::String;
    let matrix = matrix_body(f64_body(), 2, 3);
    let canonical = canonical_storage();
    let read = derive_input_requirement(AccessMode::Read, DeliveryMode::Signal);
    let write = derive_input_requirement(AccessMode::Write, DeliveryMode::Signal);
    let consume = derive_input_requirement(AccessMode::Consume, DeliveryMode::Signal);
    let full = derive_output_requirement(
        OutputConstruction::FullWrite {
            shape: ShapeRule::Declared,
        },
        AliasPolicy::NoAlias,
        ChangeDetectionPolicy::KernelReported,
    );
    let regional = derive_output_requirement(
        OutputConstruction::ReadModifyWrite {
            base_input: 0,
            regions: RegionPolicy::SingleElement,
        },
        AliasPolicy::MayAlias { input: 0 },
        ChangeDetectionPolicy::KernelReported,
    );
    let arbitrary = derive_output_requirement(
        OutputConstruction::ReadModifyWrite {
            base_input: 0,
            regions: RegionPolicy::Arbitrary,
        },
        AliasPolicy::MayAlias { input: 0 },
        ChangeDetectionPolicy::KernelReported,
    );
    let semantic_hash = derive_output_requirement(
        OutputConstruction::FullWrite {
            shape: ShapeRule::Declared,
        },
        AliasPolicy::NoAlias,
        ChangeDetectionPolicy::SemanticHash,
    );
    let exact_scalar = derive_output_requirement(
        OutputConstruction::FullWrite {
            shape: ShapeRule::Declared,
        },
        AliasPolicy::NoAlias,
        ChangeDetectionPolicy::ExactScalar,
    );
    let mut cases = Vec::new();
    type Specification = (
        &'static str,
        SchemaBody,
        PortMemoryRequirement,
        fn(&mut StorageCapabilityDescriptor),
        PortStorageCompatibilityError,
    );
    let specifications: [Specification; 12] = [
        ("read", f64.clone(), read.clone(), |s| s.access.readable = false, PortStorageCompatibilityError::ReadUnsupported),
        ("write", f64.clone(), full.clone(), |s| s.access.writable = false, PortStorageCompatibilityError::WriteUnsupported),
        ("shared-read", f64.clone(), read.clone(), |s| s.ownership.shared_read = false, PortStorageCompatibilityError::SharedReadUnsupported),
        ("exclusive-write", f64.clone(), write, |s| s.ownership.exclusive_write = false, PortStorageCompatibilityError::ExclusiveWriteUnsupported),
        ("replace", f64.clone(), full.clone(), |s| s.access.replaceable = false, PortStorageCompatibilityError::ReplaceUnsupported),
        ("region-mutation", string.clone(), regional, |s| s.access.region_mutable = false, PortStorageCompatibilityError::RegionMutationUnsupported),
        ("whole-value", f64.clone(), read, |s| s.addressing.whole_value = false, PortStorageCompatibilityError::WholeValueAddressingUnsupported),
        ("arbitrary-region", string, arbitrary, |s| s.addressing.arbitrary_regions = false, PortStorageCompatibilityError::ArbitraryRegionUnsupported),
        ("atomic-publication", f64.clone(), full.clone(), |s| s.publication.atomic_replace = false, PortStorageCompatibilityError::AtomicPublicationUnsupported),
        ("failure-atomicity", f64.clone(), full, |s| s.publication.preserves_previous_on_failure = false, PortStorageCompatibilityError::FailureAtomicityUnsupported),
        ("canonical-snapshot", f64.clone(), semantic_hash, |s| s.access.canonical_snapshot = false, PortStorageCompatibilityError::CanonicalSnapshotUnsupported),
        ("exact-scalar-matrix", matrix, exact_scalar, |_| {}, PortStorageCompatibilityError::ExactScalarChangeDetectionRequiresScalar),
    ];
    for (name, body, requirement, mutate, error) in specifications {
        let mut storage = canonical.clone();
        mutate(&mut storage);
        cases.push(case(name, body, requirement, storage, Err(error)));
    }
    let mut no_owned = canonical;
    no_owned.ownership.owned_value = false;
    no_owned.ownership.detachable = false;
    cases.push(case(
        "owned-value",
        f64,
        consume,
        no_owned,
        Err(PortStorageCompatibilityError::OwnedValueUnsupported),
    ));
    run(cases);
}

#[test]
fn declared_requirements_preserve_delivery_without_target_policy() {
    let (schema, shape) = schema_shape(f64_body());
    let storage = ValueCell::from_exact(1_f64).unwrap().storage_capabilities();
    for delivery in [
        DeliveryMode::Signal,
        DeliveryMode::Stream,
        DeliveryMode::Future,
    ] {
        let derived = derive_input_requirement(AccessMode::Read, delivery);
        assert_eq!(
            derived.delivery, delivery,
            "delivery {delivery:?} was not retained"
        );
        assert_eq!(
            check_port_storage_compatibility(&schema, &shape, &derived, &storage),
            Ok(()),
            "generic compatibility rejected delivery {delivery:?}",
        );
    }

    let variadic = OperationContractDeclaration {
        inputs: InputPortLayout::Variadic {
            prefix: vec![InputPortPolicy {
                access: AccessMode::Read,
                delivery: DeliveryMode::Stream,
            }]
            .into_boxed_slice(),
            repeated: InputPortPolicy {
                access: AccessMode::Consume,
                delivery: DeliveryMode::Future,
            },
            min_repetitions: 2,
        },
        outputs: Box::new([]),
        interaction: ExternalInteraction::Pure,
    };
    let before = variadic.clone();
    let policies = variadic.inputs.resolve(3).unwrap();
    let first = variadic.memory_requirements(3).unwrap();
    let second = variadic.memory_requirements(3).unwrap();
    assert_eq!(
        policies.len(),
        3,
        "variadic resolver did not determine port count"
    );
    assert_eq!(first, second, "variadic derivation was not deterministic");
    assert_eq!(variadic, before, "variadic derivation mutated declaration");
    for (index, (policy, requirement)) in policies.iter().zip(first.inputs.iter()).enumerate() {
        assert_eq!(requirement.access, policy.access, "variadic access {index}");
        assert_eq!(
            requirement.delivery, policy.delivery,
            "variadic delivery {index}"
        );
    }
}

fn invocation_reason(result: MResult<()>) -> FunctionMemoryContractViolationReason {
    result
        .unwrap_err()
        .kind_as::<FunctionMemoryContractViolation>()
        .unwrap()
        .reason
        .clone()
}

fn assert_shadow_check(
    name: &str,
    invocation: &FunctionInvocation,
    declaration: &OperationContractDeclaration,
    expected: Result<(), FunctionMemoryContractViolationReason>,
) {
    let input_snapshots = invocation
        .input_cells()
        .iter()
        .map(ValueCell::snapshot)
        .collect::<MResult<Vec<_>>>()
        .unwrap();
    let output_snapshot = invocation.output_cell().snapshot().unwrap();
    let input_shapes = invocation
        .input_cells()
        .iter()
        .map(|cell| cell.shape().clone())
        .collect::<Vec<_>>();
    let output_shape = invocation.output_cell().shape().clone();
    let result = invocation.check_operation_memory_contract(declaration);
    match expected {
        Ok(()) => assert!(result.is_ok(), "{name}: {result:?}"),
        Err(expected) => assert_eq!(invocation_reason(result), expected, "{name}"),
    }
    for (index, ((cell, snapshot), shape)) in invocation
        .input_cells()
        .iter()
        .zip(input_snapshots.iter())
        .zip(input_shapes.iter())
        .enumerate()
    {
        let after = cell.snapshot().unwrap();
        assert!(
            after
                .snapshot_eq(
                    &after.schemas().unwrap(),
                    snapshot,
                    &snapshot.schemas().unwrap(),
                )
                .unwrap(),
            "{name}: input {index} snapshot changed",
        );
        assert_eq!(*cell.shape(), *shape, "{name}: input {index} shape changed");
    }
    let after = invocation.output_cell().snapshot().unwrap();
    assert!(
        after
            .snapshot_eq(
                &after.schemas().unwrap(),
                &output_snapshot,
                &output_snapshot.schemas().unwrap(),
            )
            .unwrap(),
        "{name}: output snapshot changed",
    );
    assert_eq!(
        *invocation.output_cell().shape(),
        output_shape,
        "{name}: output shape changed",
    );
}

#[test]
fn shadow_invocation_validation_is_complete_and_pure() {
    let input = ValueCell::from_exact(1_f64).unwrap();
    let other = ValueCell::from_exact(2_f64).unwrap();
    let independent = ValueCell::from_exact(0_f64).unwrap();
    let input_policy = InputPortPolicy {
        access: AccessMode::Read,
        delivery: DeliveryMode::Signal,
    };
    let contract = |alias| declaration(vec![input_policy], vec![full_output(alias)]);
    assert_shadow_check(
        "valid-input-output",
        &FunctionInvocation::unary(independent.clone(), input.clone()),
        &contract(AliasPolicy::NoAlias),
        Ok(()),
    );
    assert_shadow_check(
        "zero-output-unit",
        &FunctionInvocation::nullary(ValueCell::unit()),
        &declaration(vec![], vec![]),
        Ok(()),
    );
    assert_shadow_check(
        "zero-output-non-unit",
        &FunctionInvocation::nullary(independent.clone()),
        &declaration(vec![], vec![]),
        Err(FunctionMemoryContractViolationReason::ZeroOutputBridgeIsNotUnit),
    );
    assert_shadow_check(
        "multiple-outputs",
        &FunctionInvocation::nullary(independent.clone()),
        &declaration(vec![], vec![full_output(AliasPolicy::NoAlias); 2]),
        Err(
            FunctionMemoryContractViolationReason::MultipleSemanticOutputsUnsupported {
                outputs: 2,
            },
        ),
    );
    assert_shadow_check(
        "no-alias-independent",
        &FunctionInvocation::unary(independent.clone(), input.clone()),
        &contract(AliasPolicy::NoAlias),
        Ok(()),
    );
    assert_shadow_check(
        "no-alias-shared",
        &FunctionInvocation::unary(input.clone(), input.clone()),
        &contract(AliasPolicy::NoAlias),
        Err(FunctionMemoryContractViolationReason::NoAliasViolation { input: 0 }),
    );
    assert_shadow_check(
        "may-alias-independent",
        &FunctionInvocation::unary(independent.clone(), input.clone()),
        &contract(AliasPolicy::MayAlias { input: 0 }),
        Ok(()),
    );
    assert_shadow_check(
        "may-alias-designated",
        &FunctionInvocation::unary(input.clone(), input.clone()),
        &contract(AliasPolicy::MayAlias { input: 0 }),
        Ok(()),
    );
    let binary_contract = |alias| declaration(vec![input_policy; 2], vec![full_output(alias)]);
    assert_shadow_check(
        "may-alias-unrelated",
        &FunctionInvocation::binary(other.clone(), input.clone(), other.clone()),
        &binary_contract(AliasPolicy::MayAlias { input: 0 }),
        Err(FunctionMemoryContractViolationReason::MayAliasViolation {
            declared_input: 0,
            unrelated_input: 1,
        }),
    );
    assert_shadow_check(
        "in-place-designated",
        &FunctionInvocation::unary(input.clone(), input.clone()),
        &contract(AliasPolicy::InPlaceRequired { input: 0 }),
        Ok(()),
    );
    assert_shadow_check(
        "in-place-independent",
        &FunctionInvocation::unary(independent, input),
        &contract(AliasPolicy::InPlaceRequired { input: 0 }),
        Err(FunctionMemoryContractViolationReason::InPlaceRequiredViolation { input: 0 }),
    );
}

fn assert_identity_pair(name: &str, left: &ValueCell, right: &ValueCell) {
    assert_eq!(
        left.same_cell(right),
        left.same_storage(right),
        "{name}: same_cell compatibility spelling drifted",
    );
}

#[test]
fn logical_value_cell_and_storage_identity_are_distinct() {
    let original = ValueCell::from_exact(1_f64).unwrap();
    let cloned = original.clone();
    assert!(
        original.same_logical_cell(&cloned),
        "ValueCell::clone logical identity"
    );
    assert!(
        original.same_storage(&cloned),
        "ValueCell::clone storage identity"
    );
    assert_identity_pair("ValueCell::clone", &original, &cloned);

    let detached = original.detached_clone().unwrap();
    assert!(
        !original.same_logical_cell(&detached),
        "detached logical identity"
    );
    assert!(
        !original.same_storage(&detached),
        "detached storage identity"
    );
    assert!(
        original.snapshot_eq(&detached).unwrap(),
        "detached logical value"
    );
    assert_identity_pair("detached", &original, &detached);

    let independent = ValueCell::from_exact(1_f64).unwrap();
    assert!(
        !original.same_logical_cell(&independent),
        "independent logical identity"
    );
    assert!(
        !original.same_storage(&independent),
        "independent storage identity"
    );
    assert!(
        original.snapshot_eq(&independent).unwrap(),
        "independent logical value"
    );
    assert_identity_pair("independent", &original, &independent);

    let canonical = canonical_cell(f64_body(), ValueDataDraft::F64(F64Bits::from_f64(1.0)));
    assert!(
        original.snapshot_eq(&canonical).unwrap(),
        "exact/canonical logical value"
    );
    assert!(
        !original.same_storage(&canonical),
        "exact/canonical storage identity"
    );
    assert_identity_pair("exact/canonical", &original, &canonical);
}

#[test]
fn inferred_vector_fixed_axes_are_authoritative_after_r4() {
    for (name, cell) in [
        (
            "RowDVector fixes its first axis",
            ValueCell::from_exact(RowDVector::<f64>::zeros(3)).unwrap(),
        ),
        (
            "DVector fixes its second axis",
            ValueCell::from_exact(DVector::<f64>::zeros(3)).unwrap(),
        ),
    ] {
        assert!(cell.validate_storage_contract().is_ok(), "{name}");
    }
    assert!(
        ValueCell::from_exact(DMatrix::<f64>::zeros(2, 3))
            .unwrap()
            .validate_storage_contract()
            .is_ok(),
        "DMatrix retains two dynamically resizable axes",
    );
}

fn declaration_encoding(
    declaration: &OperationContractDeclaration,
    input_count: usize,
) -> Box<[u8]> {
    let inputs = declaration
        .inputs
        .resolve(input_count)
        .unwrap()
        .iter()
        .map(|policy| ResolvedInputPort {
            schema: SchemaId::new(0),
            access: policy.access,
            delivery: policy.delivery,
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let outputs = declaration
        .outputs
        .iter()
        .map(|policy| ResolvedOutputPort {
            schema: SchemaId::new(0),
            access: policy.access,
            delivery: policy.delivery,
            construction: policy.construction.clone(),
            alias: policy.alias,
            change_detection: policy.change_detection,
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();
    ResolvedOperationContract::Declared(DeclaredOperationContract {
        inputs,
        outputs,
        interaction: declaration.interaction.clone(),
    })
    .canonical_bytes()
    .unwrap()
}

#[test]
fn r2_analysis_is_deterministic_non_mutating_and_non_serialized() {
    for (name, body) in [
        ("scalar", f64_body()),
        ("matrix", matrix_body(f64_body(), 2, 3)),
        ("record", record_body()),
        ("set", set_body()),
        ("map", map_body()),
    ] {
        let (schema, shape) = schema_shape(body);
        let bytes = schema.canonical_bytes();
        let key = schema.key();
        let first = schema.type_memory_contract().unwrap();
        let resolved = schema.resolved_type_memory_contract(&shape).unwrap();
        assert_eq!(
            first,
            schema.type_memory_contract().unwrap(),
            "{name}: type derivation"
        );
        assert_eq!(
            resolved,
            schema.resolved_type_memory_contract(&shape).unwrap(),
            "{name}: resolution"
        );
        assert_eq!(
            check_schema_storage_compatibility(&schema, &shape, &canonical_storage()),
            Ok(()),
            "{name}: schema/storage"
        );
        assert_eq!(
            check_port_storage_compatibility(
                &schema,
                &shape,
                &derive_input_requirement(AccessMode::Read, DeliveryMode::Signal),
                &canonical_storage()
            ),
            Ok(()),
            "{name}: port/storage"
        );
        assert_eq!(
            schema.canonical_bytes(),
            bytes,
            "{name}: schema bytes changed"
        );
        assert_eq!(schema.key(), key, "{name}: schema key changed");
    }

    let declaration = declaration(
        vec![InputPortPolicy {
            access: AccessMode::Read,
            delivery: DeliveryMode::Stream,
        }],
        vec![full_output(AliasPolicy::NoAlias)],
    );
    let before = declaration.clone();
    let bytes = declaration_encoding(&declaration, 1);
    let first = declaration.memory_requirements(1).unwrap();
    let second = declaration.memory_requirements(1).unwrap();
    assert_eq!(
        first, second,
        "operation requirements were not deterministic"
    );
    assert_eq!(declaration, before, "operation declaration was mutated");
    assert_eq!(
        declaration_encoding(&declaration, 1),
        bytes,
        "operation declaration encoding changed"
    );
    for (name, source) in [
        (
            "type",
            include_str!("../src/memory_contract/type_contract.rs"),
        ),
        (
            "storage",
            include_str!("../src/memory_contract/storage_capability.rs"),
        ),
        (
            "operation",
            include_str!("../src/memory_contract/operation_requirement.rs"),
        ),
    ] {
        assert!(
            !source.contains("Serialize"),
            "{name}: R2 metadata became serialized"
        );
        assert!(
            !source.contains("Deserialize"),
            "{name}: R2 metadata became deserialized"
        );
    }
}
