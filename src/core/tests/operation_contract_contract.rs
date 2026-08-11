use mech_core::{
    AccessMode, AliasPolicy, ChangeDetectionPolicy, DeclaredOperationContract, DeliveryMode,
    DimensionExpr, EffectContract, EffectDeliveryPolicy, EnumVariantSchema, ExternalInteraction,
    IdempotencyRequirement, InputPortLayout, InputPortPolicy, LegacyOpaqueOperationContract,
    NominalKey, ObservationContract, ObservationReplayPolicy, OperationContractDeclaration,
    OperationContractError, OperationContractId, OperationContractTable,
    OperationContractTableBuilder, OutputConstruction, OutputPortPolicy, PortDirection,
    RegionPolicy, ResolvedInputPort, ResolvedOperationContract, ResolvedOutputPort, SchemaBody,
    SchemaDraft, SchemaField, SchemaId, SchemaTableBuilder, ShapeContractReference, ShapeRule,
    TransactionalEffectProtocol, TransactionalExternalContract, validate_contract_schemas,
    validate_declaration, validate_resolved_contract, validate_signal_bindings,
};

fn declared(change_detection: ChangeDetectionPolicy) -> ResolvedOperationContract {
    ResolvedOperationContract::Declared(DeclaredOperationContract {
        inputs: vec![
            ResolvedInputPort {
                schema: SchemaId::new(3),
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            },
            ResolvedInputPort {
                schema: SchemaId::new(3),
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            },
        ]
        .into_boxed_slice(),
        outputs: vec![ResolvedOutputPort {
            schema: SchemaId::new(3),
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            alias: AliasPolicy::NoAlias,
            change_detection,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    })
}

#[test]
fn operation_contract_id_is_a_distinct_dense_u32_domain() {
    let id = OperationContractId::new(17);
    assert_eq!(id.get(), 17);
    assert_eq!(core::mem::size_of::<OperationContractId>(), 4);
}

#[test]
fn empty_operation_contract_table_is_canonical_and_round_trips() {
    let table = OperationContractTable::empty();
    let bytes = table.canonical_bytes().unwrap();

    assert!(table.is_empty());
    assert_eq!(bytes.as_ref(), &[0, 0, 0, 0]);
    assert_eq!(
        OperationContractTable::from_canonical_bytes(&bytes),
        Ok(table)
    );
}

fn one_input_one_output(
    input_schema: SchemaId,
    output_schema: SchemaId,
    alias: AliasPolicy,
    construction: OutputConstruction,
    interaction: ExternalInteraction,
) -> ResolvedOperationContract {
    ResolvedOperationContract::Declared(DeclaredOperationContract {
        inputs: vec![ResolvedInputPort {
            schema: input_schema,
            access: AccessMode::Read,
            delivery: DeliveryMode::Signal,
        }]
        .into_boxed_slice(),
        outputs: vec![ResolvedOutputPort {
            schema: output_schema,
            access: match &construction {
                OutputConstruction::ReadModifyWrite { .. } => AccessMode::ReadWrite,
                _ => AccessMode::Write,
            },
            delivery: DeliveryMode::Signal,
            construction,
            alias,
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction,
    })
}

fn output_policy() -> OutputPortPolicy {
    OutputPortPolicy {
        access: AccessMode::Write,
        delivery: DeliveryMode::Signal,
        construction: OutputConstruction::FullWrite {
            shape: ShapeRule::Declared,
        },
        alias: AliasPolicy::NoAlias,
        change_detection: ChangeDetectionPolicy::KernelReported,
    }
}

fn declaration_with_outputs(
    interaction: ExternalInteraction,
    output_count: usize,
) -> OperationContractDeclaration {
    OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![InputPortPolicy {
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            }]
            .into_boxed_slice(),
        ),
        outputs: vec![output_policy(); output_count].into_boxed_slice(),
        interaction,
    }
}

#[test]
fn insertion_order_does_not_change_contract_ids() {
    let exact = declared(ChangeDetectionPolicy::ExactScalar);
    let always = declared(ChangeDetectionPolicy::AlwaysChanged);

    let mut first = OperationContractTableBuilder::new();
    let first_exact = first.insert(exact.clone()).unwrap();
    let first_always = first.insert(always.clone()).unwrap();
    let first = first.finish().unwrap();

    let mut second = OperationContractTableBuilder::new();
    let second_always = second.insert(always).unwrap();
    let second_exact = second.insert(exact).unwrap();
    let second = second.finish().unwrap();

    assert_eq!(first.resolve(first_exact), second.resolve(second_exact));
    assert_eq!(first.resolve(first_always), second.resolve(second_always));
    assert_eq!(first.table, second.table);
}

#[test]
fn duplicate_contracts_deduplicate_and_distinct_contracts_do_not() {
    let exact = declared(ChangeDetectionPolicy::ExactScalar);
    let mut builder = OperationContractTableBuilder::new();
    let first = builder.insert(exact.clone()).unwrap();
    let duplicate = builder.insert(exact).unwrap();
    let distinct = builder
        .insert(declared(ChangeDetectionPolicy::KernelReported))
        .unwrap();
    let build = builder.finish().unwrap();

    assert_eq!(build.table.len(), 2);
    assert_eq!(build.resolve(first), build.resolve(duplicate));
    assert_ne!(build.resolve(first), build.resolve(distinct));
}

#[test]
fn canonical_bytes_round_trip_every_contract_family() {
    let contracts = [
        declared(ChangeDetectionPolicy::SemanticHash),
        ResolvedOperationContract::LegacyOpaque(LegacyOpaqueOperationContract {
            input_schemas: vec![SchemaId::new(0), SchemaId::new(2)].into_boxed_slice(),
            output_schemas: vec![SchemaId::new(1)].into_boxed_slice(),
        }),
    ];
    for contract in contracts {
        let bytes = contract.canonical_bytes().unwrap();
        assert_eq!(
            ResolvedOperationContract::from_canonical_bytes(&bytes).unwrap(),
            contract,
        );
    }
}

#[test]
fn canonical_bytes_round_trip_resident_region_policies() {
    for region in [
        RegionPolicy::WholeValue,
        RegionPolicy::IndexedAxis { axis: 0 },
        RegionPolicy::IndexedAxis { axis: 7 },
    ] {
        let contract = one_input_one_output(
            SchemaId::new(3),
            SchemaId::new(3),
            AliasPolicy::MayAlias { input: 0 },
            OutputConstruction::ReadModifyWrite {
                base_input: 0,
                regions: region,
            },
            ExternalInteraction::Pure,
        );
        let bytes = contract.canonical_bytes().unwrap();
        assert_eq!(
            ResolvedOperationContract::from_canonical_bytes(&bytes).unwrap(),
            contract,
        );
    }
}

#[test]
fn canonical_decoder_rejects_nested_counts_before_allocating() {
    let mut declared_bytes = declared(ChangeDetectionPolicy::ExactScalar)
        .canonical_bytes()
        .unwrap()
        .into_vec();
    declared_bytes[2..6].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(matches!(
        ResolvedOperationContract::from_canonical_bytes(&declared_bytes),
        Err(OperationContractError::InvalidCanonicalEncoding { .. })
    ));

    let mut output_count = declared(ChangeDetectionPolicy::ExactScalar)
        .canonical_bytes()
        .unwrap()
        .into_vec();
    output_count[18..22].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(matches!(
        ResolvedOperationContract::from_canonical_bytes(&output_count),
        Err(OperationContractError::InvalidCanonicalEncoding { .. })
    ));

    let legacy = ResolvedOperationContract::LegacyOpaque(LegacyOpaqueOperationContract {
        input_schemas: vec![SchemaId::new(0)].into_boxed_slice(),
        output_schemas: vec![SchemaId::new(1)].into_boxed_slice(),
    });
    let mut legacy_bytes = legacy.canonical_bytes().unwrap().into_vec();
    legacy_bytes[2..6].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(matches!(
        ResolvedOperationContract::from_canonical_bytes(&legacy_bytes),
        Err(OperationContractError::InvalidCanonicalEncoding { .. })
    ));

    let mut build = declared(ChangeDetectionPolicy::ExactScalar);
    let ResolvedOperationContract::Declared(contract) = &mut build else {
        unreachable!()
    };
    contract.outputs[0].construction = OutputConstruction::Build {
        postcondition: ShapeContractReference {
            module_path: vec!["matrix".to_owned()].into_boxed_slice(),
            contract_name: "selection-output".to_owned(),
        },
    };
    let mut build_bytes = build.canonical_bytes().unwrap().into_vec();
    let module_count_offset = 2 + 4 + (2 * 6) + 4 + 4 + 1 + 1 + 1;
    build_bytes[module_count_offset..module_count_offset + 4]
        .copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(matches!(
        ResolvedOperationContract::from_canonical_bytes(&build_bytes),
        Err(OperationContractError::InvalidCanonicalEncoding { .. })
    ));
}

#[test]
fn aliasing_requires_exact_input_and_output_schema_identity() {
    let input_schema = SchemaId::new(3);
    let other_schema = SchemaId::new(9);
    let contract = |output_schema, alias| {
        one_input_one_output(
            input_schema,
            output_schema,
            alias,
            OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            ExternalInteraction::Pure,
        )
    };

    assert!(validate_resolved_contract(&contract(other_schema, AliasPolicy::NoAlias)).is_ok());
    assert!(
        validate_resolved_contract(&contract(input_schema, AliasPolicy::MayAlias { input: 0 }))
            .is_ok()
    );
    assert_eq!(
        validate_resolved_contract(&contract(other_schema, AliasPolicy::MayAlias { input: 0 })),
        Err(OperationContractError::AliasSchemaMismatch {
            output: 0,
            input: 0,
            input_schema,
            output_schema: other_schema,
        })
    );
    assert!(
        validate_resolved_contract(&contract(
            input_schema,
            AliasPolicy::InPlaceRequired { input: 0 }
        ))
        .is_ok()
    );
    assert_eq!(
        validate_resolved_contract(&contract(
            other_schema,
            AliasPolicy::InPlaceRequired { input: 0 }
        )),
        Err(OperationContractError::AliasSchemaMismatch {
            output: 0,
            input: 0,
            input_schema,
            output_schema: other_schema,
        })
    );
    assert_eq!(
        validate_resolved_contract(&contract(input_schema, AliasPolicy::MayAlias { input: 7 })),
        Err(OperationContractError::InputOrdinalOutOfRange {
            field: "alias.input",
            input: 7,
            inputs: 1,
        })
    );
}

#[test]
fn canonical_decoder_rejects_alias_schema_mismatches() {
    for alias in [
        AliasPolicy::MayAlias { input: 0 },
        AliasPolicy::InPlaceRequired { input: 0 },
    ] {
        let contract = one_input_one_output(
            SchemaId::new(3),
            SchemaId::new(3),
            alias,
            OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            ExternalInteraction::Pure,
        );
        let mut bytes = contract.canonical_bytes().unwrap().into_vec();
        bytes[16..20].copy_from_slice(&9_u32.to_le_bytes());

        assert_eq!(
            ResolvedOperationContract::from_canonical_bytes(&bytes),
            Err(OperationContractError::AliasSchemaMismatch {
                output: 0,
                input: 0,
                input_schema: SchemaId::new(3),
                output_schema: SchemaId::new(9),
            })
        );
    }
}

#[test]
fn irreversible_effects_forbid_ordinary_outputs() {
    let effect = ExternalInteraction::Effect(EffectContract {
        delivery: EffectDeliveryPolicy::ProviderDefined,
        idempotency: IdempotencyRequirement::Optional,
    });
    let observation = ExternalInteraction::Observation(ObservationContract {
        replay: ObservationReplayPolicy::CaptureAsInputFact,
    });
    let transactional = ExternalInteraction::TransactionalExternal(TransactionalExternalContract {
        protocol: TransactionalEffectProtocol::PrepareCommit,
    });

    assert!(validate_declaration(&declaration_with_outputs(effect.clone(), 0)).is_ok());
    let mut invalid_effect_declaration = declaration_with_outputs(effect.clone(), 1);
    invalid_effect_declaration.outputs[0].access = AccessMode::Read;
    assert_eq!(
        validate_declaration(&invalid_effect_declaration),
        Err(OperationContractError::EffectOutputUnsupported { outputs: 1 })
    );

    let resolved = |interaction, output_count| {
        ResolvedOperationContract::Declared(DeclaredOperationContract {
            inputs: Box::new([]),
            outputs: vec![
                ResolvedOutputPort {
                    schema: SchemaId::new(3),
                    access: AccessMode::Write,
                    delivery: DeliveryMode::Signal,
                    construction: OutputConstruction::FullWrite {
                        shape: ShapeRule::Declared,
                    },
                    alias: AliasPolicy::NoAlias,
                    change_detection: ChangeDetectionPolicy::KernelReported,
                };
                output_count
            ]
            .into_boxed_slice(),
            interaction,
        })
    };
    assert!(validate_resolved_contract(&resolved(effect.clone(), 0)).is_ok());
    let mut invalid_effect = resolved(effect, 1);
    let ResolvedOperationContract::Declared(invalid_effect_contract) = &mut invalid_effect else {
        unreachable!()
    };
    invalid_effect_contract.outputs[0].access = AccessMode::Read;
    assert_eq!(
        validate_resolved_contract(&invalid_effect),
        Err(OperationContractError::EffectOutputUnsupported { outputs: 1 })
    );

    for interaction in [ExternalInteraction::Pure, observation, transactional] {
        assert!(validate_declaration(&declaration_with_outputs(interaction.clone(), 1)).is_ok());
        assert!(validate_resolved_contract(&resolved(interaction, 1)).is_ok());
    }
}

#[test]
fn canonical_decoder_rejects_effects_with_ordinary_outputs() {
    let contract = one_input_one_output(
        SchemaId::new(3),
        SchemaId::new(3),
        AliasPolicy::NoAlias,
        OutputConstruction::FullWrite {
            shape: ShapeRule::Declared,
        },
        ExternalInteraction::Pure,
    );
    let mut bytes = contract.canonical_bytes().unwrap().into_vec();
    assert_eq!(bytes.pop(), Some(0));
    bytes.extend_from_slice(&[2, 0, 0]);

    assert_eq!(
        ResolvedOperationContract::from_canonical_bytes(&bytes),
        Err(OperationContractError::EffectOutputUnsupported { outputs: 1 })
    );
}

fn build_contract(module_path: Box<[String]>, contract_name: &str) -> ResolvedOperationContract {
    one_input_one_output(
        SchemaId::new(3),
        SchemaId::new(3),
        AliasPolicy::NoAlias,
        OutputConstruction::Build {
            postcondition: ShapeContractReference {
                module_path,
                contract_name: contract_name.to_owned(),
            },
        },
        ExternalInteraction::Pure,
    )
}

#[test]
fn shape_contract_references_use_canonical_segments() {
    for (module_path, contract_name) in [
        (
            vec!["matrix".to_owned(), "selection".to_owned()],
            "logical-mask-output",
        ),
        (vec!["matrix".to_owned()], "logical mask"),
    ] {
        assert!(
            validate_resolved_contract(&build_contract(
                module_path.into_boxed_slice(),
                contract_name
            ))
            .is_ok()
        );
    }

    assert!(matches!(
        validate_resolved_contract(&build_contract(Box::new([]), "logical-mask-output")),
        Err(OperationContractError::InvalidShapeContractReference {
            field: "module_path",
            ..
        })
    ));

    for value in [
        "",
        " leading",
        "trailing ",
        ".",
        "..",
        "matrix/selection",
        "matrix\\selection",
        "nul\0value",
    ] {
        assert_eq!(
            validate_resolved_contract(&build_contract(
                vec![value.to_owned()].into_boxed_slice(),
                "logical-mask-output"
            )),
            Err(OperationContractError::InvalidShapeContractReference {
                field: "module_path",
                value: value.to_owned(),
            })
        );
        assert_eq!(
            validate_resolved_contract(&build_contract(
                vec!["matrix".to_owned()].into_boxed_slice(),
                value
            )),
            Err(OperationContractError::InvalidShapeContractReference {
                field: "contract_name",
                value: value.to_owned(),
            })
        );
    }
}

#[test]
fn canonical_decoder_rejects_noncanonical_shape_contract_segments() {
    let contract = build_contract(
        vec!["matrixx".to_owned()].into_boxed_slice(),
        "logical-mask-output",
    );
    let canonical = contract.canonical_bytes().unwrap();
    let start = canonical
        .windows(b"matrixx".len())
        .position(|window| window == b"matrixx")
        .unwrap();

    for replacement in [b"mat/ixx", b"mat\\ixx", b" matrix", b"matrix "] {
        let mut bytes = canonical.to_vec();
        bytes[start..start + replacement.len()].copy_from_slice(replacement);
        assert!(matches!(
            ResolvedOperationContract::from_canonical_bytes(&bytes),
            Err(OperationContractError::InvalidShapeContractReference { .. })
        ));
    }
}

fn matrix_schema(element: SchemaBody, rows: u64, columns: u64) -> mech_core::Schema {
    SchemaDraft {
        dimension_parameters: Box::new([]),
        body: SchemaBody::Matrix {
            element: Box::new(element),
            dimensions: vec![
                DimensionExpr::Constant(rows),
                DimensionExpr::Constant(columns),
            ]
            .into_boxed_slice(),
        },
    }
    .finalize()
    .unwrap()
}

#[test]
fn transpose_requires_reversed_dimensions_and_the_same_element_schema() {
    let mut builder = SchemaTableBuilder::new();
    let input = builder
        .insert(matrix_schema(SchemaBody::Bool, 2, 3))
        .unwrap();
    let valid_output = builder
        .insert(matrix_schema(SchemaBody::Bool, 3, 2))
        .unwrap();
    let wrong_dimensions = builder
        .insert(matrix_schema(SchemaBody::Bool, 2, 3))
        .unwrap();
    let wrong_element = builder
        .insert(matrix_schema(SchemaBody::Index, 3, 2))
        .unwrap();
    let build = builder.finish().unwrap();
    let input = build.resolve(input).unwrap();
    let valid_output = build.resolve(valid_output).unwrap();
    let wrong_dimensions = build.resolve(wrong_dimensions).unwrap();
    let wrong_element = build.resolve(wrong_element).unwrap();
    let (schemas, _) = build.into_parts();

    let contract_for = |output_schema| {
        ResolvedOperationContract::Declared(DeclaredOperationContract {
            inputs: vec![ResolvedInputPort {
                schema: input,
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            }]
            .into_boxed_slice(),
            outputs: vec![ResolvedOutputPort {
                schema: output_schema,
                access: AccessMode::Write,
                delivery: DeliveryMode::Signal,
                construction: OutputConstruction::FullWrite {
                    shape: ShapeRule::TransposeOf { input: 0 },
                },
                alias: AliasPolicy::NoAlias,
                change_detection: ChangeDetectionPolicy::KernelReported,
            }]
            .into_boxed_slice(),
            interaction: ExternalInteraction::Pure,
        })
    };

    assert!(validate_contract_schemas(&contract_for(valid_output), &schemas).is_ok());
    for output_schema in [wrong_dimensions, wrong_element] {
        assert_eq!(
            validate_contract_schemas(&contract_for(output_schema), &schemas),
            Err(OperationContractError::TransposeSchemaMismatch {
                input: 0,
                output: 0,
            })
        );
    }
}

#[test]
fn same_as_input_requires_equivalent_schema_shape() {
    let mut builder = SchemaTableBuilder::new();
    let input = builder
        .insert(matrix_schema(SchemaBody::Bool, 2, 3))
        .unwrap();
    let same_shape = builder
        .insert(matrix_schema(SchemaBody::Index, 2, 3))
        .unwrap();
    let wrong_shape = builder
        .insert(matrix_schema(SchemaBody::Bool, 7, 11))
        .unwrap();
    let scalar = builder
        .insert(
            SchemaDraft {
                dimension_parameters: Box::new([]),
                body: SchemaBody::Bool,
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let build = builder.finish().unwrap();
    let input = build.resolve(input).unwrap();
    let same_shape = build.resolve(same_shape).unwrap();
    let wrong_shape = build.resolve(wrong_shape).unwrap();
    let scalar = build.resolve(scalar).unwrap();
    let (schemas, _) = build.into_parts();

    let contract_for = |output_schema| {
        ResolvedOperationContract::Declared(DeclaredOperationContract {
            inputs: vec![ResolvedInputPort {
                schema: input,
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            }]
            .into_boxed_slice(),
            outputs: vec![ResolvedOutputPort {
                schema: output_schema,
                access: AccessMode::Write,
                delivery: DeliveryMode::Signal,
                construction: OutputConstruction::FullWrite {
                    shape: ShapeRule::SameAsInput { input: 0 },
                },
                alias: AliasPolicy::NoAlias,
                change_detection: ChangeDetectionPolicy::KernelReported,
            }]
            .into_boxed_slice(),
            interaction: ExternalInteraction::Pure,
        })
    };

    assert!(validate_contract_schemas(&contract_for(same_shape), &schemas).is_ok());
    for output_schema in [wrong_shape, scalar] {
        assert_eq!(
            validate_contract_schemas(&contract_for(output_schema), &schemas),
            Err(OperationContractError::SameShapeSchemaMismatch {
                input: 0,
                output: 0,
            })
        );
    }
}

#[test]
fn same_as_input_checks_enum_variant_and_payload_shapes() {
    let enum_schema = |key, payload| {
        SchemaDraft {
            dimension_parameters: Box::new([]),
            body: SchemaBody::Enum {
                key: NominalKey::from_bytes([key; 32]),
                variants: vec![EnumVariantSchema {
                    name: "Value".to_owned(),
                    payload,
                }]
                .into_boxed_slice(),
            },
        }
        .finalize()
        .unwrap()
    };
    let matrix_body = |element, rows, columns| SchemaBody::Matrix {
        element: Box::new(element),
        dimensions: vec![
            DimensionExpr::Constant(rows),
            DimensionExpr::Constant(columns),
        ]
        .into_boxed_slice(),
    };
    let mut builder = SchemaTableBuilder::new();
    let input = builder
        .insert(enum_schema(1, Some(matrix_body(SchemaBody::Bool, 2, 3))))
        .unwrap();
    let same_shape = builder
        .insert(enum_schema(2, Some(matrix_body(SchemaBody::Index, 2, 3))))
        .unwrap();
    let wrong_payload_shape = builder
        .insert(enum_schema(3, Some(matrix_body(SchemaBody::Bool, 7, 11))))
        .unwrap();
    let scalar = builder
        .insert(
            SchemaDraft {
                dimension_parameters: Box::new([]),
                body: SchemaBody::Bool,
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let build = builder.finish().unwrap();
    let input = build.resolve(input).unwrap();
    let same_shape = build.resolve(same_shape).unwrap();
    let wrong_payload_shape = build.resolve(wrong_payload_shape).unwrap();
    let scalar = build.resolve(scalar).unwrap();
    let (schemas, _) = build.into_parts();

    let contract_for = |output_schema| {
        ResolvedOperationContract::Declared(DeclaredOperationContract {
            inputs: vec![ResolvedInputPort {
                schema: input,
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            }]
            .into_boxed_slice(),
            outputs: vec![ResolvedOutputPort {
                schema: output_schema,
                access: AccessMode::Write,
                delivery: DeliveryMode::Signal,
                construction: OutputConstruction::FullWrite {
                    shape: ShapeRule::SameAsInput { input: 0 },
                },
                alias: AliasPolicy::NoAlias,
                change_detection: ChangeDetectionPolicy::KernelReported,
            }]
            .into_boxed_slice(),
            interaction: ExternalInteraction::Pure,
        })
    };

    assert!(validate_contract_schemas(&contract_for(same_shape), &schemas).is_ok());
    for output_schema in [wrong_payload_shape, scalar] {
        assert_eq!(
            validate_contract_schemas(&contract_for(output_schema), &schemas),
            Err(OperationContractError::SameShapeSchemaMismatch {
                input: 0,
                output: 0,
            })
        );
    }
}

#[test]
fn same_as_input_checks_nested_table_column_shapes() {
    let matrix_body = |element, rows, columns| SchemaBody::Matrix {
        element: Box::new(element),
        dimensions: vec![
            DimensionExpr::Constant(rows),
            DimensionExpr::Constant(columns),
        ]
        .into_boxed_slice(),
    };
    let table_schema = |column_schema| {
        SchemaDraft {
            dimension_parameters: Box::new([]),
            body: SchemaBody::Table {
                columns: vec![SchemaField {
                    name: "value".to_owned(),
                    schema: column_schema,
                }]
                .into_boxed_slice(),
                rows: DimensionExpr::Constant(4),
            },
        }
        .finalize()
        .unwrap()
    };
    let mut builder = SchemaTableBuilder::new();
    let input = builder
        .insert(table_schema(matrix_body(SchemaBody::Bool, 2, 3)))
        .unwrap();
    let same_shape = builder
        .insert(table_schema(matrix_body(SchemaBody::Index, 2, 3)))
        .unwrap();
    let wrong_column_shape = builder
        .insert(table_schema(matrix_body(SchemaBody::Bool, 7, 11)))
        .unwrap();
    let build = builder.finish().unwrap();
    let input = build.resolve(input).unwrap();
    let same_shape = build.resolve(same_shape).unwrap();
    let wrong_column_shape = build.resolve(wrong_column_shape).unwrap();
    let (schemas, _) = build.into_parts();

    let contract_for = |output_schema| {
        ResolvedOperationContract::Declared(DeclaredOperationContract {
            inputs: vec![ResolvedInputPort {
                schema: input,
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            }]
            .into_boxed_slice(),
            outputs: vec![ResolvedOutputPort {
                schema: output_schema,
                access: AccessMode::Write,
                delivery: DeliveryMode::Signal,
                construction: OutputConstruction::FullWrite {
                    shape: ShapeRule::SameAsInput { input: 0 },
                },
                alias: AliasPolicy::NoAlias,
                change_detection: ChangeDetectionPolicy::KernelReported,
            }]
            .into_boxed_slice(),
            interaction: ExternalInteraction::Pure,
        })
    };

    assert!(validate_contract_schemas(&contract_for(same_shape), &schemas).is_ok());
    assert_eq!(
        validate_contract_schemas(&contract_for(wrong_column_shape), &schemas),
        Err(OperationContractError::SameShapeSchemaMismatch {
            input: 0,
            output: 0,
        })
    );
}

#[test]
fn canonical_contract_table_bytes_round_trip_and_reject_reordering() {
    let mut builder = OperationContractTableBuilder::new();
    builder
        .insert(declared(ChangeDetectionPolicy::ExactScalar))
        .unwrap();
    builder
        .insert(declared(ChangeDetectionPolicy::AlwaysChanged))
        .unwrap();
    let table = builder.finish().unwrap().table;
    let bytes = table.canonical_bytes().unwrap();
    assert_eq!(
        OperationContractTable::from_canonical_bytes(&bytes).unwrap(),
        table
    );

    let first_len = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    let first_end = 8 + first_len;
    let second_len =
        u32::from_le_bytes(bytes[first_end..first_end + 4].try_into().unwrap()) as usize;
    let second_end = first_end + 4 + second_len;
    let mut reordered = Vec::from(&bytes[..4]);
    reordered.extend_from_slice(&bytes[first_end..second_end]);
    reordered.extend_from_slice(&bytes[4..first_end]);
    assert_eq!(
        OperationContractTable::from_canonical_bytes(&reordered),
        Err(OperationContractError::NonCanonicalContractOrder),
    );

    let mut impossible_count = vec![u8::MAX; 4];
    impossible_count.extend_from_slice(&bytes[4..]);
    assert!(matches!(
        OperationContractTable::from_canonical_bytes(&impossible_count),
        Err(OperationContractError::InvalidCanonicalEncoding { .. })
    ));
}

#[test]
fn invalid_access_direction_and_construction_are_rejected() {
    let mut contract = declared(ChangeDetectionPolicy::ExactScalar);
    let ResolvedOperationContract::Declared(contract) = &mut contract else {
        unreachable!()
    };
    contract.inputs[0].access = AccessMode::Write;
    assert_eq!(
        validate_resolved_contract(&ResolvedOperationContract::Declared(contract.clone())),
        Err(OperationContractError::InvalidAccessDirection {
            direction: PortDirection::Input,
            ordinal: 0,
            access: AccessMode::Write,
        }),
    );

    contract.inputs[0].access = AccessMode::Read;
    contract.outputs[0].access = AccessMode::ReadWrite;
    assert!(matches!(
        validate_resolved_contract(&ResolvedOperationContract::Declared(contract.clone())),
        Err(OperationContractError::InvalidConstructionAccess {
            output: 0,
            construction: "FullWrite",
            ..
        })
    ));
}

#[test]
fn read_modify_write_requires_the_base_schema_and_valid_input() {
    let mut contract = declared(ChangeDetectionPolicy::KernelReported);
    let ResolvedOperationContract::Declared(contract) = &mut contract else {
        unreachable!()
    };
    contract.outputs[0].access = AccessMode::ReadWrite;
    contract.outputs[0].construction = OutputConstruction::ReadModifyWrite {
        base_input: 1,
        regions: RegionPolicy::SingleElement,
    };
    contract.inputs[1].schema = SchemaId::new(9);
    contract.outputs[0].alias = AliasPolicy::MayAlias { input: 1 };
    assert!(matches!(
        validate_resolved_contract(&ResolvedOperationContract::Declared(contract.clone())),
        Err(OperationContractError::ReadModifyWriteSchemaMismatch {
            output: 0,
            base_input: 1,
            ..
        })
    ));

    contract.inputs[1].schema = SchemaId::new(3);
    contract.outputs[0].construction = OutputConstruction::ReadModifyWrite {
        base_input: 9,
        regions: RegionPolicy::SingleElement,
    };
    assert!(matches!(
        validate_resolved_contract(&ResolvedOperationContract::Declared(contract.clone())),
        Err(OperationContractError::InputOrdinalOutOfRange {
            field: "base_input",
            input: 9,
            ..
        })
    ));
}

#[test]
fn stream_and_future_types_exist_but_cannot_use_signal_bindings() {
    let mut stream = declared(ChangeDetectionPolicy::AlwaysChanged);
    let ResolvedOperationContract::Declared(contract) = &mut stream else {
        unreachable!()
    };
    contract.inputs[0].delivery = DeliveryMode::Stream;
    assert!(validate_resolved_contract(&stream).is_ok());
    assert!(matches!(
        validate_signal_bindings(&stream),
        Err(OperationContractError::UnsupportedSignalBinding {
            direction: PortDirection::Input,
            ordinal: 0,
            delivery: DeliveryMode::Stream,
        })
    ));
}
