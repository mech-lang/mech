#![cfg(feature = "full")]

use mech_core::*;
use nalgebra::RowDVector;

fn schema_shape(body: SchemaBody) -> (Schema, ShapeInstance) {
    let schema = SchemaDraft {
        dimension_parameters: Box::new([]),
        body,
    }
    .finalize()
    .unwrap();
    let shape = schema.instantiate_shape(Box::new([])).unwrap();
    (schema, shape)
}

fn f64_schema_shape() -> (Schema, ShapeInstance) {
    schema_shape(SchemaBody::FloatingPoint(FloatWidth::W64))
}

fn string_schema_shape() -> (Schema, ShapeInstance) {
    schema_shape(SchemaBody::String)
}

fn tuple_schema_shape() -> (Schema, ShapeInstance) {
    schema_shape(SchemaBody::Tuple(
        vec![SchemaBody::FloatingPoint(FloatWidth::W64)].into_boxed_slice(),
    ))
}

fn record_schema_shape() -> (Schema, ShapeInstance) {
    schema_shape(SchemaBody::Record(
        vec![SchemaField {
            name: "value".to_owned(),
            schema: SchemaBody::FloatingPoint(FloatWidth::W64),
        }]
        .into_boxed_slice(),
    ))
}

fn set_schema_shape() -> (Schema, ShapeInstance) {
    schema_shape(SchemaBody::Set {
        element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
        cardinality: DimensionExpr::Constant(1).into(),
    })
}

fn table_schema_shape() -> (Schema, ShapeInstance) {
    schema_shape(SchemaBody::Table {
        columns: vec![SchemaField {
            name: "value".to_owned(),
            schema: SchemaBody::FloatingPoint(FloatWidth::W64),
        }]
        .into_boxed_slice(),
        rows: DimensionExpr::Constant(1).into(),
    })
}

fn map_schema_shape() -> (Schema, ShapeInstance) {
    schema_shape(SchemaBody::Map {
        key: Box::new(SchemaBody::String),
        value: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
        cardinality: DimensionExpr::Constant(1).into(),
    })
}

fn matrix_schema_shape(rows: u64, columns: u64) -> (Schema, ShapeInstance) {
    schema_shape(SchemaBody::Matrix {
        element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
        dimensions: vec![
            DimensionExpr::Constant(rows),
            DimensionExpr::Constant(columns),
        ]
        .into_boxed_slice(),
    })
}

fn universal_storage() -> StorageCapabilityDescriptor {
    ValueCell::unit().storage_capabilities()
}

fn input_policy(access: AccessMode, delivery: DeliveryMode) -> InputPortPolicy {
    InputPortPolicy { access, delivery }
}

fn shape_contract() -> ShapeContractReference {
    ShapeContractReference {
        module_path: vec!["tests".to_string()].into_boxed_slice(),
        contract_name: "shape".to_string(),
    }
}

fn output_policy(
    access: AccessMode,
    delivery: DeliveryMode,
    construction: OutputConstruction,
    alias: AliasPolicy,
    change_detection: ChangeDetectionPolicy,
) -> OutputPortPolicy {
    OutputPortPolicy {
        access,
        delivery,
        construction,
        alias,
        change_detection,
    }
}

fn full_output(alias: AliasPolicy) -> OutputPortPolicy {
    output_policy(
        AccessMode::Write,
        DeliveryMode::Signal,
        OutputConstruction::FullWrite {
            shape: ShapeRule::Declared,
        },
        alias,
        ChangeDetectionPolicy::KernelReported,
    )
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

fn requirement(
    access: AccessMode,
    ownership: OwnershipRequirement,
    construction: Option<OutputConstruction>,
    addressing: AddressingRequirement,
    publication: PublicationRequirement,
    change_detection: Option<ChangeDetectionPolicy>,
) -> PortMemoryRequirement {
    PortMemoryRequirement {
        access,
        delivery: DeliveryMode::Signal,
        ownership,
        construction,
        addressing,
        alias: None,
        publication,
        change_detection,
    }
}

fn read_requirement() -> PortMemoryRequirement {
    requirement(
        AccessMode::Read,
        OwnershipRequirement::SharedRead,
        None,
        AddressingRequirement::WholeValue,
        PublicationRequirement::None,
        None,
    )
}

fn write_requirement(construction: OutputConstruction) -> PortMemoryRequirement {
    requirement(
        AccessMode::Write,
        OwnershipRequirement::ExclusiveWrite,
        Some(construction),
        AddressingRequirement::WholeValue,
        PublicationRequirement::AtomicReplace,
        Some(ChangeDetectionPolicy::KernelReported),
    )
}

fn check(
    schema: &Schema,
    shape: &ShapeInstance,
    requirement: &PortMemoryRequirement,
    storage: &StorageCapabilityDescriptor,
) -> Result<(), PortStorageCompatibilityError> {
    check_port_storage_compatibility(schema, shape, requirement, storage)
}

fn semantic_addressing_error(
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

#[test]
fn input_derivation_preserves_access_delivery_and_ownership() {
    for (access, ownership) in [
        (AccessMode::Read, OwnershipRequirement::SharedRead),
        (AccessMode::Write, OwnershipRequirement::ExclusiveWrite),
        (AccessMode::ReadWrite, OwnershipRequirement::ExclusiveWrite),
        (AccessMode::Consume, OwnershipRequirement::OwnedValue),
    ] {
        for delivery in [
            DeliveryMode::Signal,
            DeliveryMode::Stream,
            DeliveryMode::Future,
        ] {
            assert_eq!(
                PortMemoryRequirement::for_input(input_policy(access, delivery)),
                PortMemoryRequirement {
                    access,
                    delivery,
                    ownership,
                    construction: None,
                    addressing: AddressingRequirement::WholeValue,
                    alias: None,
                    publication: PublicationRequirement::None,
                    change_detection: None,
                }
            );
        }
    }
}

#[test]
fn output_derivation_preserves_declared_policy_and_maps_every_construction() {
    let cases = [
        (
            OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            AddressingRequirement::WholeValue,
        ),
        (
            OutputConstruction::Replace {
                shape: ShapeRule::SameAsInput { input: 0 },
            },
            AddressingRequirement::WholeValue,
        ),
        (
            OutputConstruction::Build {
                postcondition: shape_contract(),
            },
            AddressingRequirement::WholeValue,
        ),
        (
            OutputConstruction::ReadModifyWrite {
                base_input: 0,
                regions: RegionPolicy::WholeValue,
            },
            AddressingRequirement::WholeValue,
        ),
        (
            OutputConstruction::ReadModifyWrite {
                base_input: 0,
                regions: RegionPolicy::IndexedAxis { axis: u16::MAX },
            },
            AddressingRequirement::Positional {
                minimum_rank: u16::MAX as u64 + 1,
            },
        ),
        (
            OutputConstruction::ReadModifyWrite {
                base_input: 0,
                regions: RegionPolicy::SingleElement,
            },
            AddressingRequirement::Positional { minimum_rank: 1 },
        ),
        (
            OutputConstruction::ReadModifyWrite {
                base_input: 0,
                regions: RegionPolicy::ContiguousRange,
            },
            AddressingRequirement::Positional { minimum_rank: 1 },
        ),
        (
            OutputConstruction::ReadModifyWrite {
                base_input: 0,
                regions: RegionPolicy::RectangularRegion,
            },
            AddressingRequirement::Positional { minimum_rank: 2 },
        ),
        (
            OutputConstruction::ReadModifyWrite {
                base_input: 0,
                regions: RegionPolicy::CollectionEntry,
            },
            AddressingRequirement::CollectionEntry,
        ),
        (
            OutputConstruction::ReadModifyWrite {
                base_input: 0,
                regions: RegionPolicy::Arbitrary,
            },
            AddressingRequirement::ArbitraryRegion,
        ),
    ];

    for (construction, addressing) in cases {
        for access in [
            AccessMode::Read,
            AccessMode::Write,
            AccessMode::ReadWrite,
            AccessMode::Consume,
        ] {
            for delivery in [
                DeliveryMode::Signal,
                DeliveryMode::Stream,
                DeliveryMode::Future,
            ] {
                let policy = output_policy(
                    access,
                    delivery,
                    construction.clone(),
                    AliasPolicy::MayAlias { input: 0 },
                    ChangeDetectionPolicy::SemanticHash,
                );
                let derived = PortMemoryRequirement::for_output(&policy).unwrap();
                assert_eq!(derived.access, access);
                assert_eq!(derived.delivery, delivery);
                assert_eq!(derived.construction, Some(construction.clone()));
                assert_eq!(derived.addressing, addressing);
                assert_eq!(derived.alias, Some(AliasPolicy::MayAlias { input: 0 }));
                assert_eq!(derived.publication, PublicationRequirement::AtomicReplace);
                assert_eq!(
                    derived.change_detection,
                    Some(ChangeDetectionPolicy::SemanticHash)
                );
            }
        }
    }

    for alias in [
        AliasPolicy::NoAlias,
        AliasPolicy::MayAlias { input: 0 },
        AliasPolicy::InPlaceRequired { input: 0 },
    ] {
        for change_detection in [
            ChangeDetectionPolicy::KernelReported,
            ChangeDetectionPolicy::ExactScalar,
            ChangeDetectionPolicy::SemanticHash,
            ChangeDetectionPolicy::AlwaysChanged,
        ] {
            let policy = output_policy(
                AccessMode::Write,
                DeliveryMode::Signal,
                OutputConstruction::FullWrite {
                    shape: ShapeRule::Declared,
                },
                alias,
                change_detection,
            );
            let derived = PortMemoryRequirement::for_output(&policy).unwrap();
            assert_eq!(derived.alias, Some(alias));
            assert_eq!(derived.change_detection, Some(change_detection));
        }
    }
}

#[test]
fn operation_derivation_uses_fixed_and_variadic_resolution_without_mutation() {
    let fixed = declaration(
        vec![input_policy(AccessMode::Read, DeliveryMode::Signal)],
        vec![],
    );
    let before = fixed.clone();
    assert_eq!(fixed.memory_requirements(1).unwrap().inputs.len(), 1);
    assert_eq!(fixed, before);
    assert_eq!(
        fixed.memory_requirements(0),
        Err(OperationContractError::PortCountMismatch {
            direction: PortDirection::Input,
            expected: 1,
            actual: 0,
        })
    );

    let variadic = OperationContractDeclaration {
        inputs: InputPortLayout::Variadic {
            prefix: vec![input_policy(AccessMode::Read, DeliveryMode::Signal)].into_boxed_slice(),
            repeated: input_policy(AccessMode::Consume, DeliveryMode::Future),
            min_repetitions: 2,
        },
        outputs: vec![full_output(AliasPolicy::NoAlias)].into_boxed_slice(),
        interaction: ExternalInteraction::Observation(ObservationContract {
            replay: ObservationReplayPolicy::CaptureAsInputFact,
        }),
    };
    let derived = variadic.memory_requirements(3).unwrap();
    assert_eq!(derived.inputs.len(), 3);
    assert_eq!(
        derived.inputs[0].ownership,
        OwnershipRequirement::SharedRead
    );
    assert_eq!(
        derived.inputs[1].ownership,
        OwnershipRequirement::OwnedValue
    );
    assert_eq!(derived.inputs[2].delivery, DeliveryMode::Future);
    assert_eq!(derived.outputs.len(), 1);
    assert_eq!(
        variadic.memory_requirements(2),
        Err(OperationContractError::VariadicInputCount {
            prefix: 1,
            minimum_repetitions: 2,
            actual: 2,
        })
    );

    for outputs in 0..=2 {
        assert_eq!(
            declaration(vec![], vec![full_output(AliasPolicy::NoAlias); outputs])
                .memory_requirements(0)
                .unwrap()
                .outputs
                .len(),
            outputs
        );
    }
}

#[test]
fn port_checker_rejects_schema_storage_mismatch_before_port_rules() {
    let (schema, shape) = f64_schema_shape();
    let storage = ValueCell::from_exact(true).unwrap().storage_capabilities();
    assert_eq!(
        check(&schema, &shape, &read_requirement(), &storage),
        Err(PortStorageCompatibilityError::SchemaStorage(
            SchemaStorageCompatibilityError::Storage(StorageCompatibilityError::ScalarKindMismatch)
        ))
    );
}

#[test]
fn port_access_and_ownership_capabilities_fail_independently() {
    let (schema, shape) = f64_schema_shape();
    let base = universal_storage();

    let mut storage = base.clone();
    storage.access.readable = false;
    assert_eq!(
        check(&schema, &shape, &read_requirement(), &storage),
        Err(PortStorageCompatibilityError::ReadUnsupported)
    );

    let write = requirement(
        AccessMode::Write,
        OwnershipRequirement::ExclusiveWrite,
        None,
        AddressingRequirement::WholeValue,
        PublicationRequirement::None,
        None,
    );
    storage = base.clone();
    storage.access.writable = false;
    assert_eq!(
        check(&schema, &shape, &write, &storage),
        Err(PortStorageCompatibilityError::WriteUnsupported)
    );

    let read_write = requirement(
        AccessMode::ReadWrite,
        OwnershipRequirement::ExclusiveWrite,
        None,
        AddressingRequirement::WholeValue,
        PublicationRequirement::None,
        None,
    );
    storage = base.clone();
    storage.access.readable = false;
    assert_eq!(
        check(&schema, &shape, &read_write, &storage),
        Err(PortStorageCompatibilityError::ReadUnsupported)
    );

    let consume = requirement(
        AccessMode::Consume,
        OwnershipRequirement::OwnedValue,
        None,
        AddressingRequirement::WholeValue,
        PublicationRequirement::None,
        None,
    );
    storage = base.clone();
    storage.access.readable = false;
    assert_eq!(
        check(&schema, &shape, &consume, &storage),
        Err(PortStorageCompatibilityError::ReadUnsupported)
    );

    storage = base.clone();
    storage.ownership.shared_read = false;
    assert_eq!(
        check(&schema, &shape, &read_requirement(), &storage),
        Err(PortStorageCompatibilityError::SharedReadUnsupported)
    );

    storage = base.clone();
    storage.ownership.exclusive_write = false;
    assert_eq!(
        check(&schema, &shape, &write, &storage),
        Err(PortStorageCompatibilityError::ExclusiveWriteUnsupported)
    );

    storage = base.clone();
    storage.ownership.owned_value = false;
    storage.ownership.detachable = false;
    assert_eq!(
        check(&schema, &shape, &consume, &storage),
        Err(PortStorageCompatibilityError::OwnedValueUnsupported)
    );
    storage.ownership.detachable = true;
    check(&schema, &shape, &consume, &storage).unwrap();
}

#[test]
fn construction_addressing_and_publication_capabilities_are_explicit() {
    let (scalar_schema, scalar_shape) = f64_schema_shape();
    let base = universal_storage();
    let constructions = [
        OutputConstruction::FullWrite {
            shape: ShapeRule::Declared,
        },
        OutputConstruction::Replace {
            shape: ShapeRule::Declared,
        },
        OutputConstruction::Build {
            postcondition: shape_contract(),
        },
        OutputConstruction::ReadModifyWrite {
            base_input: 0,
            regions: RegionPolicy::WholeValue,
        },
    ];
    for construction in constructions {
        let requirement = write_requirement(construction);
        let mut storage = base.clone();
        storage.access.replaceable = false;
        assert_eq!(
            check(&scalar_schema, &scalar_shape, &requirement, &storage),
            Err(PortStorageCompatibilityError::ReplaceUnsupported)
        );
        check(&scalar_schema, &scalar_shape, &requirement, &base).unwrap();
    }

    let (string_schema, string_shape) = string_schema_shape();
    let regional = requirement(
        AccessMode::ReadWrite,
        OwnershipRequirement::ExclusiveWrite,
        Some(OutputConstruction::ReadModifyWrite {
            base_input: 0,
            regions: RegionPolicy::SingleElement,
        }),
        AddressingRequirement::Positional { minimum_rank: 1 },
        PublicationRequirement::AtomicReplace,
        Some(ChangeDetectionPolicy::KernelReported),
    );
    let mut storage = base.clone();
    storage.access.region_mutable = false;
    assert_eq!(
        check(&string_schema, &string_shape, &regional, &storage),
        Err(PortStorageCompatibilityError::RegionMutationUnsupported)
    );
    check(&string_schema, &string_shape, &regional, &base).unwrap();

    let whole = read_requirement();
    storage = base.clone();
    storage.addressing.whole_value = false;
    assert_eq!(
        check(&scalar_schema, &scalar_shape, &whole, &storage),
        Err(PortStorageCompatibilityError::WholeValueAddressingUnsupported)
    );

    let positional_one = requirement(
        AccessMode::Read,
        OwnershipRequirement::SharedRead,
        None,
        AddressingRequirement::Positional { minimum_rank: 1 },
        PublicationRequirement::None,
        None,
    );
    storage = base.clone();
    storage.addressing.positional = PositionalAddressingCapability::None;
    assert_eq!(
        check(&string_schema, &string_shape, &positional_one, &storage),
        Err(PortStorageCompatibilityError::SchemaStorage(
            SchemaStorageCompatibilityError::Storage(
                StorageCompatibilityError::PositionalAddressingUnsupported
            )
        ))
    );
    storage.addressing.positional = PositionalAddressingCapability::Rank(1);
    check(&string_schema, &string_shape, &positional_one, &storage).unwrap();

    let (matrix_schema, matrix_shape) = matrix_schema_shape(2, 2);
    let positional_two = PortMemoryRequirement {
        addressing: AddressingRequirement::Positional { minimum_rank: 2 },
        ..positional_one.clone()
    };
    assert_eq!(
        check(&matrix_schema, &matrix_shape, &positional_two, &storage),
        Err(PortStorageCompatibilityError::SchemaStorage(
            SchemaStorageCompatibilityError::Storage(
                StorageCompatibilityError::PositionalAddressingUnsupported
            )
        ))
    );
    storage.addressing.positional = PositionalAddressingCapability::Rank(2);
    check(&matrix_schema, &matrix_shape, &positional_two, &storage).unwrap();
    storage.addressing.positional = PositionalAddressingCapability::AnyRank;
    check(&matrix_schema, &matrix_shape, &positional_two, &storage).unwrap();

    let arbitrary = PortMemoryRequirement {
        addressing: AddressingRequirement::ArbitraryRegion,
        ..positional_one
    };
    storage = base.clone();
    storage.addressing.arbitrary_regions = false;
    assert_eq!(
        check(&string_schema, &string_shape, &arbitrary, &storage),
        Err(PortStorageCompatibilityError::ArbitraryRegionUnsupported)
    );
    check(&string_schema, &string_shape, &arbitrary, &base).unwrap();

    let publication = write_requirement(OutputConstruction::FullWrite {
        shape: ShapeRule::Declared,
    });
    storage = base.clone();
    storage.publication.atomic_replace = false;
    assert_eq!(
        check(&scalar_schema, &scalar_shape, &publication, &storage),
        Err(PortStorageCompatibilityError::AtomicPublicationUnsupported)
    );
    storage = base.clone();
    storage.publication.preserves_previous_on_failure = false;
    assert_eq!(
        check(&scalar_schema, &scalar_shape, &publication, &storage),
        Err(PortStorageCompatibilityError::FailureAtomicityUnsupported)
    );
}

#[test]
fn collection_entry_accepts_positional_named_or_keyed_addressing() {
    let requirement = requirement(
        AccessMode::Read,
        OwnershipRequirement::SharedRead,
        None,
        AddressingRequirement::CollectionEntry,
        PublicationRequirement::None,
        None,
    );
    let mut empty = universal_storage();
    empty.addressing.positional = PositionalAddressingCapability::None;
    empty.addressing.named_members = false;
    empty.addressing.keyed_members = false;
    let (tuple_schema, tuple_shape) = tuple_schema_shape();
    assert_eq!(
        check(&tuple_schema, &tuple_shape, &requirement, &empty),
        Err(PortStorageCompatibilityError::SchemaStorage(
            SchemaStorageCompatibilityError::Storage(
                StorageCompatibilityError::PositionalAddressingUnsupported
            )
        ))
    );

    let mut positional = empty.clone();
    positional.addressing.positional = PositionalAddressingCapability::Rank(0);
    assert_eq!(
        check(&tuple_schema, &tuple_shape, &requirement, &positional),
        Err(PortStorageCompatibilityError::SchemaStorage(
            SchemaStorageCompatibilityError::Storage(
                StorageCompatibilityError::PositionalAddressingUnsupported
            )
        ))
    );
    positional.addressing.positional = PositionalAddressingCapability::Rank(1);
    check(&tuple_schema, &tuple_shape, &requirement, &positional).unwrap();
    positional.addressing.positional = PositionalAddressingCapability::AnyRank;
    check(&tuple_schema, &tuple_shape, &requirement, &positional).unwrap();

    let (record_schema, record_shape) = record_schema_shape();
    let mut named = empty.clone();
    named.addressing.named_members = true;
    check(&record_schema, &record_shape, &requirement, &named).unwrap();

    let (set_schema, set_shape) = set_schema_shape();
    let mut keyed = empty;
    keyed.addressing.keyed_members = true;
    check(&set_schema, &set_shape, &requirement, &keyed).unwrap();
}

#[test]
fn semantic_addressing_completes_the_type_port_storage_triangle() {
    let storage = universal_storage();
    let positional_one = requirement(
        AccessMode::Read,
        OwnershipRequirement::SharedRead,
        None,
        AddressingRequirement::Positional { minimum_rank: 1 },
        PublicationRequirement::None,
        None,
    );
    let positional_two = PortMemoryRequirement {
        addressing: AddressingRequirement::Positional { minimum_rank: 2 },
        ..positional_one.clone()
    };
    let collection = PortMemoryRequirement {
        addressing: AddressingRequirement::CollectionEntry,
        ..positional_one.clone()
    };
    let arbitrary = PortMemoryRequirement {
        addressing: AddressingRequirement::ArbitraryRegion,
        ..positional_one.clone()
    };

    let (string_schema, string_shape) = string_schema_shape();
    check(&string_schema, &string_shape, &positional_one, &storage).unwrap();
    check(&string_schema, &string_shape, &arbitrary, &storage).unwrap();

    let (matrix_schema, matrix_shape) = matrix_schema_shape(2, 3);
    check(&matrix_schema, &matrix_shape, &positional_two, &storage).unwrap();
    check(&matrix_schema, &matrix_shape, &arbitrary, &storage).unwrap();

    for (schema, shape) in [
        tuple_schema_shape(),
        record_schema_shape(),
        table_schema_shape(),
        set_schema_shape(),
        map_schema_shape(),
    ] {
        check(&schema, &shape, &collection, &storage).unwrap();
    }

    let (table_schema, table_shape) = table_schema_shape();
    check(&table_schema, &table_shape, &positional_two, &storage).unwrap();

    let (scalar_schema, scalar_shape) = f64_schema_shape();
    for required in [
        AddressingRequirement::Positional { minimum_rank: 1 },
        AddressingRequirement::CollectionEntry,
        AddressingRequirement::ArbitraryRegion,
    ] {
        let port = PortMemoryRequirement {
            addressing: required,
            ..read_requirement()
        };
        assert_eq!(
            check(&scalar_schema, &scalar_shape, &port, &storage),
            Err(semantic_addressing_error(
                &scalar_schema,
                &scalar_shape,
                required,
            ))
        );
    }

    let (record_schema, record_shape) = record_schema_shape();
    assert_eq!(
        check(&record_schema, &record_shape, &positional_one, &storage,),
        Err(semantic_addressing_error(
            &record_schema,
            &record_shape,
            positional_one.addressing,
        ))
    );

    let (dynamic_schema, dynamic_shape) = schema_shape(SchemaBody::Dynamic);
    assert_eq!(
        check(&dynamic_schema, &dynamic_shape, &arbitrary, &storage,),
        Err(semantic_addressing_error(
            &dynamic_schema,
            &dynamic_shape,
            arbitrary.addressing,
        ))
    );

    let (tuple_schema, tuple_shape) = tuple_schema_shape();
    let mut keyed_only = storage;
    keyed_only.addressing.positional = PositionalAddressingCapability::None;
    keyed_only.addressing.named_members = false;
    keyed_only.addressing.keyed_members = true;
    assert_eq!(
        check(&tuple_schema, &tuple_shape, &collection, &keyed_only),
        Err(PortStorageCompatibilityError::SchemaStorage(
            SchemaStorageCompatibilityError::Storage(
                StorageCompatibilityError::PositionalAddressingUnsupported
            )
        ))
    );
}

#[test]
fn change_detection_uses_semantic_topology_and_canonical_snapshots() {
    let (scalar_schema, scalar_shape) = f64_schema_shape();
    let (matrix_schema, matrix_shape) = matrix_schema_shape(2, 2);
    let mut storage = universal_storage();

    let semantic_hash = requirement(
        AccessMode::Write,
        OwnershipRequirement::ExclusiveWrite,
        None,
        AddressingRequirement::WholeValue,
        PublicationRequirement::None,
        Some(ChangeDetectionPolicy::SemanticHash),
    );
    storage.access.canonical_snapshot = false;
    assert_eq!(
        check(&scalar_schema, &scalar_shape, &semantic_hash, &storage),
        Err(PortStorageCompatibilityError::CanonicalSnapshotUnsupported)
    );
    check(
        &scalar_schema,
        &scalar_shape,
        &semantic_hash,
        &universal_storage(),
    )
    .unwrap();

    storage = universal_storage();
    let exact_scalar = PortMemoryRequirement {
        change_detection: Some(ChangeDetectionPolicy::ExactScalar),
        ..semantic_hash.clone()
    };
    assert_eq!(
        check(&matrix_schema, &matrix_shape, &exact_scalar, &storage),
        Err(PortStorageCompatibilityError::ExactScalarChangeDetectionRequiresScalar)
    );
    check(&scalar_schema, &scalar_shape, &exact_scalar, &storage).unwrap();
    check(
        &scalar_schema,
        &scalar_shape,
        &exact_scalar,
        &ValueCell::from_exact(1_f64).unwrap().storage_capabilities(),
    )
    .unwrap();

    for policy in [
        ChangeDetectionPolicy::KernelReported,
        ChangeDetectionPolicy::AlwaysChanged,
    ] {
        let no_extra_requirement = PortMemoryRequirement {
            change_detection: Some(policy),
            ..semantic_hash.clone()
        };
        storage.access.canonical_snapshot = false;
        check(
            &scalar_schema,
            &scalar_shape,
            &no_extra_requirement,
            &storage,
        )
        .unwrap();
    }

    for delivery in [
        DeliveryMode::Signal,
        DeliveryMode::Stream,
        DeliveryMode::Future,
    ] {
        let delivered = PortMemoryRequirement {
            delivery,
            ..read_requirement()
        };
        check(
            &scalar_schema,
            &scalar_shape,
            &delivered,
            &universal_storage(),
        )
        .unwrap();
    }
}

fn invocation_reason(
    invocation: &FunctionInvocation,
    declaration: &OperationContractDeclaration,
) -> FunctionMemoryContractViolationReason {
    invocation
        .check_operation_memory_contract(declaration)
        .unwrap_err()
        .kind_as::<FunctionMemoryContractViolation>()
        .expect("operation-memory errors remain structured")
        .reason
        .clone()
}

#[test]
fn invocation_validates_ports_and_current_output_bridge() {
    let input = ValueCell::from_exact(1_f64).unwrap();
    let output = ValueCell::from_exact(0_f64).unwrap();
    let invocation = FunctionInvocation::unary(output.clone(), input.clone());
    let one_output = declaration(
        vec![input_policy(AccessMode::Read, DeliveryMode::Signal)],
        vec![full_output(AliasPolicy::NoAlias)],
    );
    invocation
        .check_operation_memory_contract(&one_output)
        .unwrap();

    let zero_outputs = declaration(vec![], vec![]);
    FunctionInvocation::nullary(ValueCell::unit())
        .check_operation_memory_contract(&zero_outputs)
        .unwrap();
    assert_eq!(
        invocation_reason(
            &FunctionInvocation::nullary(ValueCell::from_exact(0_f64).unwrap()),
            &zero_outputs,
        ),
        FunctionMemoryContractViolationReason::ZeroOutputBridgeIsNotUnit
    );

    let multiple_outputs = declaration(
        vec![],
        vec![
            full_output(AliasPolicy::NoAlias),
            full_output(AliasPolicy::NoAlias),
        ],
    );
    assert_eq!(
        invocation_reason(
            &FunctionInvocation::nullary(ValueCell::from_exact(0_f64).unwrap()),
            &multiple_outputs,
        ),
        FunctionMemoryContractViolationReason::MultipleSemanticOutputsUnsupported { outputs: 2 }
    );

    let wrong_arity = declaration(
        vec![input_policy(AccessMode::Read, DeliveryMode::Signal)],
        vec![],
    );
    assert!(matches!(
        invocation_reason(
            &FunctionInvocation::nullary(ValueCell::unit()),
            &wrong_arity,
        ),
        FunctionMemoryContractViolationReason::OperationContractDerivation {
            error: OperationContractError::PortCountMismatch { .. }
        }
    ));

    assert!(input.snapshot_eq(&invocation.input_cells()[0]).unwrap());
    assert!(output.snapshot_eq(invocation.output_cell()).unwrap());
}

#[test]
fn invocation_dynamic_vector_ports_match_their_resizable_storage_axes() {
    let row = ValueCell::from_exact(RowDVector::<f64>::zeros(3)).unwrap();
    let scalar = ValueCell::from_exact(0_f64).unwrap();
    let declaration = declaration(
        vec![input_policy(AccessMode::Read, DeliveryMode::Signal)],
        vec![full_output(AliasPolicy::NoAlias)],
    );
    FunctionInvocation::unary(scalar.clone(), row.clone())
        .check_operation_memory_contract(&declaration)
        .unwrap();
    FunctionInvocation::unary(row, scalar)
        .check_operation_memory_contract(&declaration)
        .unwrap();
}

#[test]
fn invocation_alias_policies_use_physical_storage_groups() {
    let policy = |alias| {
        declaration(
            vec![
                input_policy(AccessMode::Read, DeliveryMode::Signal),
                input_policy(AccessMode::Read, DeliveryMode::Signal),
            ],
            vec![full_output(alias)],
        )
    };

    let first = ValueCell::from_exact(1_f64).unwrap();
    let second = ValueCell::from_exact(2_f64).unwrap();
    let independent = ValueCell::from_exact(0_f64).unwrap();
    FunctionInvocation::binary(independent.clone(), first.clone(), second.clone())
        .check_operation_memory_contract(&policy(AliasPolicy::NoAlias))
        .unwrap();
    assert_eq!(
        invocation_reason(
            &FunctionInvocation::binary(first.clone(), first.clone(), second.clone()),
            &policy(AliasPolicy::NoAlias),
        ),
        FunctionMemoryContractViolationReason::NoAliasViolation { input: 0 }
    );

    FunctionInvocation::binary(independent, first.clone(), second.clone())
        .check_operation_memory_contract(&policy(AliasPolicy::MayAlias { input: 0 }))
        .unwrap();
    FunctionInvocation::binary(first.clone(), first.clone(), second.clone())
        .check_operation_memory_contract(&policy(AliasPolicy::MayAlias { input: 0 }))
        .unwrap();

    let same_group = first.clone();
    FunctionInvocation::binary(same_group.clone(), first.clone(), same_group.clone())
        .check_operation_memory_contract(&policy(AliasPolicy::MayAlias { input: 0 }))
        .unwrap();

    assert_eq!(
        invocation_reason(
            &FunctionInvocation::binary(second.clone(), first.clone(), second.clone()),
            &policy(AliasPolicy::MayAlias { input: 0 }),
        ),
        FunctionMemoryContractViolationReason::MayAliasViolation {
            declared_input: 0,
            unrelated_input: 1,
        }
    );
    assert_eq!(
        invocation_reason(
            &FunctionInvocation::binary(
                ValueCell::from_exact(0_f64).unwrap(),
                first.clone(),
                second.clone(),
            ),
            &policy(AliasPolicy::MayAlias { input: 2 }),
        ),
        FunctionMemoryContractViolationReason::InvalidDeclaredAliasInput {
            input: 2,
            inputs: 2,
        }
    );

    FunctionInvocation::binary(first.clone(), first.clone(), second.clone())
        .check_operation_memory_contract(&policy(AliasPolicy::InPlaceRequired { input: 0 }))
        .unwrap();
    assert_eq!(
        invocation_reason(
            &FunctionInvocation::binary(ValueCell::from_exact(0_f64).unwrap(), first, second,),
            &policy(AliasPolicy::InPlaceRequired { input: 0 }),
        ),
        FunctionMemoryContractViolationReason::InPlaceRequiredViolation { input: 0 }
    );
}

#[test]
fn failed_shadow_checks_do_not_mutate_cells() {
    let input = ValueCell::from_exact(7_f64).unwrap();
    let output = ValueCell::from_exact(3_f64).unwrap();
    let before_input = input.detached_clone().unwrap();
    let before_output = output.detached_clone().unwrap();
    let before_input_shape = input.shape().clone();
    let before_output_shape = output.shape().clone();
    let invocation = FunctionInvocation::unary(output.clone(), input.clone());
    let declaration = declaration(
        vec![input_policy(AccessMode::Read, DeliveryMode::Signal)],
        vec![full_output(AliasPolicy::InPlaceRequired { input: 0 })],
    );

    assert!(
        invocation
            .check_operation_memory_contract(&declaration)
            .is_err()
    );
    assert!(input.snapshot_eq(&before_input).unwrap());
    assert!(output.snapshot_eq(&before_output).unwrap());
    assert_eq!(*input.shape(), before_input_shape);
    assert_eq!(*output.shape(), before_output_shape);
}

#[test]
fn r2c_types_are_derived_only_and_do_not_add_serialization() {
    let source = include_str!("../src/memory_contract/operation_requirement.rs");
    assert!(!source.contains("Serialize"));
    assert!(!source.contains("Deserialize"));

    let declaration = declaration(
        vec![input_policy(AccessMode::Read, DeliveryMode::Signal)],
        vec![full_output(AliasPolicy::NoAlias)],
    );
    let before = declaration.clone();
    let first = declaration.memory_requirements(1).unwrap();
    let second = declaration.memory_requirements(1).unwrap();
    assert_eq!(first, second);
    assert_eq!(declaration, before);

    let operation_encoding = include_str!("../src/operation_contract/encoding.rs");
    assert!(!operation_encoding.contains("OperationMemoryRequirements"));
    assert!(!operation_encoding.contains("PortMemoryRequirement"));
}
