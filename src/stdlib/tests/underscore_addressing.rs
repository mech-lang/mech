//! Sugar selects catalog/type metadata, then uses ordinary canonical calls.
#![cfg(all(feature = "full_compiler", feature = "map_ops"))]

use mech_core::{
    CanonicalFunctionSpecializer, FunctionCatalog, FunctionCatalogBuilder, FunctionExport,
    FunctionExposure, MResult, ReactiveInstanceId, SpecializationContext, SpecializationInvocation,
    SpecializedFunction, Value, ValueData,
};
use mech_engine::{
    CanonicalSourceFrontend, ProgramArtifact, SourceDocumentOutputKind, SourceSemanticError,
    resident::{ActivationFacts, activate},
};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
use std::sync::Arc;

fn syntax(source: &str) -> DocumentSyntax {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x554e4452), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    DocumentSyntax::cast(parsed.syntax()).unwrap()
}

fn compile(
    source: &str,
    catalog: Arc<FunctionCatalog>,
) -> Result<(ProgramArtifact, usize), SourceSemanticError> {
    let program =
        CanonicalSourceFrontend.compile_document_with_catalog(&syntax(source), catalog)?;
    let output = program
        .document_outputs()
        .iter()
        .rev()
        .find(|output| output.kind == SourceDocumentOutputKind::Program)
        .unwrap()
        .output as usize;
    Ok((program.compile_artifact().unwrap(), output))
}

fn execute(source: &str, catalog: Arc<FunctionCatalog>) -> (ProgramArtifact, Vec<Value>) {
    let (original, output) = compile(source, catalog.clone()).unwrap();
    let encoded = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&encoded).unwrap();
    let mut values = Vec::new();
    for (ordinal, artifact) in [&original, &decoded].into_iter().enumerate() {
        let mut instance = activate(
            ReactiveInstanceId::new(0x554e4452, ordinal as u32),
            artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        instance.turn(&[]).unwrap();
        values.push(instance.copied_output(output).unwrap());
    }
    (original, values)
}

fn operations(artifact: &ProgramArtifact) -> Vec<String> {
    artifact
        .nodes()
        .iter()
        .filter_map(|node| node.as_operation())
        .map(|op| op.operation.canonical_name())
        .collect()
}

#[test]
fn ordinary_catalog_projections_share_explicit_call_nodes_and_execution() {
    let catalog = mech_stdlib::source_catalog();
    for (prefix, sugar, explicit) in [
        (
            "+> map\ncounts := {\"b\": 2u64, \"a\": 1u64}\n",
            "counts_keys",
            "map/keys(counts)",
        ),
        (
            "+> map\ncounts := {\"b\": 2u64, \"a\": 1u64}\n",
            "counts_values",
            "map/values(counts)",
        ),
        (
            "+> sequence\ntext := \"αβ\"\n",
            "text_length",
            "sequence/length(text)",
        ),
        (
            "values := {1u64, 2u64}\n",
            "values_size",
            "set/size(values)",
        ),
    ] {
        let (sugar_artifact, sugar_values) =
            execute(&format!("{prefix}{sugar}\n"), catalog.clone());
        let (explicit_artifact, explicit_values) =
            execute(&format!("{prefix}{explicit}\n"), catalog.clone());
        assert_eq!(operations(&sugar_artifact), operations(&explicit_artifact));
        for (left, right) in sugar_values.iter().zip(&explicit_values) {
            assert!(
                left.snapshot_eq(sugar_artifact.schemas(), right, explicit_artifact.schemas(),)
                    .unwrap()
            );
        }
    }
}

#[test]
fn addressed_results_support_existing_selection_and_function_locals() {
    let catalog = mech_stdlib::source_catalog();
    for source in [
        "+> map\ncounts := {\"b\": 2u64, \"a\": 1u64}\ncounts_keys[2u64]\n",
        "+> map\npick(counts<{string:u64}>) => (out<string>) := out := counts_keys[2u64].\npick({\"b\": 2u64, \"a\": 1u64})\n",
    ] {
        let (artifact, values) = execute(source, catalog.clone());
        assert!(operations(&artifact).contains(&"map/keys".to_owned()));
        for value in values {
            assert!(matches!(value.data(), ValueData::String(text) if text.as_ref() == "b"));
        }
    }
}

#[test]
fn exact_underscore_names_and_unknown_suffixes_remain_ordinary_names() {
    let catalog = mech_stdlib::source_catalog();
    for source in [
        "+> map\ncounts := {\"a\": 1u64}\ncounts_keys := 7u64\ncounts_keys\n",
        "counts_keys(x<u64>) => (out<u64>) := out := x.\ncounts_keys(7u64)\n",
        "user_data := 7u64\nuser_data\n",
        "+> map\ncounts := {\"a\": 1u64}\n~counts_keys := 7u64\ncounts_keys\n",
    ] {
        let (artifact, values) = execute(source, catalog.clone());
        assert!(!operations(&artifact).contains(&"map/keys".to_owned()));
        for value in values {
            assert!(matches!(value.data(), ValueData::U64(7)));
        }
    }
    for source in [
        "+> map\ncounts := {\"a\": 1u64}\ncounts_keys<u64>\n",
        "unknown_suffix<u64>\n",
    ] {
        let (artifact, _) = compile(source, catalog.clone()).unwrap();
        assert_eq!(artifact.inputs().len(), 1);
        assert!(!operations(&artifact).contains(&"map/keys".to_owned()));
    }
}

struct SourceOnlyProbe;
impl CanonicalFunctionSpecializer for SourceOnlyProbe {
    fn specialize_invocation(
        &self,
        _: &SpecializationInvocation,
        _: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        panic!("canonical artifact calls do not enter the older source specializer");
    }
}

fn custom_catalog(names: &[(&str, &str)]) -> Arc<FunctionCatalog> {
    let maintained = mech_stdlib::source_catalog();
    let mut builder = FunctionCatalogBuilder::from_catalog(&maintained);
    for &(canonical, delegate) in names {
        let declaration = maintained
            .source_type_declaration(delegate)
            .unwrap()
            .clone();
        let contract = maintained
            .source_operation_contract(delegate, 1, false)
            .unwrap()
            .clone();
        let operation = builder
            .insert_canonical_specializer_with_contract(
                canonical,
                declaration,
                contract,
                Arc::new(SourceOnlyProbe),
            )
            .unwrap();
        let (module, item) = canonical.rsplit_once('/').unwrap();
        builder
            .insert_export(FunctionExport {
                operation,
                canonical_name: canonical.to_owned(),
                module: Some(module.to_owned()),
                item: Some(item.to_owned()),
                exposure: FunctionExposure::ModuleOnly,
            })
            .unwrap();
        let (delegate_module, delegate_item) = delegate.rsplit_once('/').unwrap();
        let factory = maintained
            .resident_factory(&[delegate_module.to_owned()], delegate_item)
            .unwrap();
        builder
            .insert_resident_factory(
                [module],
                item,
                factory.implementation_memory(),
                factory.factory(),
            )
            .unwrap();
    }
    Arc::new(builder.build().unwrap())
}

#[test]
fn generic_catalog_items_use_types_and_report_ambiguity() {
    // No builtin suffix registry knows "tokens". Both modules are visible;
    // ordinary type schemes, not module spelling, select the String one.
    let typed = custom_catalog(&[
        ("measure/tokens", "sequence/length"),
        ("flags/tokens", "logic/not"),
    ]);
    let (artifact, values) = execute(
        "+> measure\n+> flags\ntext := \"abc\"\ntext_tokens\n",
        typed,
    );
    assert!(operations(&artifact).contains(&"measure/tokens".to_owned()));
    for value in values {
        assert!(matches!(value.data(), ValueData::U64(3)));
    }

    let ambiguous = custom_catalog(&[
        ("measure/tokens", "sequence/length"),
        ("metrics/tokens", "sequence/length"),
    ]);
    let error = compile(
        "+> measure\n+> metrics\ntext := \"abc\"\ntext_tokens\n",
        ambiguous.clone(),
    )
    .unwrap_err();
    assert_eq!(error.code, "source-semantics/ambiguous-underscore-address");
    assert!(error.message.contains("measure/tokens(text)"));
    assert!(error.message.contains("metrics/tokens(text)"));

    let (artifact, values) = execute(
        "+> measure\n+> metrics\ntext := \"abc\"\ntext_tokens := 9u64\ntext_tokens\n",
        ambiguous,
    );
    assert!(
        !operations(&artifact)
            .iter()
            .any(|name| name.ends_with("/tokens"))
    );
    for value in values {
        assert!(matches!(value.data(), ValueData::U64(9)));
    }
}

#[test]
fn every_known_base_split_is_considered_instead_of_guessing_the_longest() {
    let catalog = custom_catalog(&[
        ("measure/tokens", "sequence/length"),
        ("metrics/word_tokens", "sequence/length"),
    ]);
    let error = compile(
        "+> measure\n+> metrics\ntext := \"a\"\ntext_word := \"bb\"\ntext_word_tokens\n",
        catalog,
    )
    .unwrap_err();
    assert_eq!(error.code, "source-semantics/ambiguous-underscore-address");
    assert!(error.message.contains("measure/tokens(text_word)"));
    assert!(error.message.contains("metrics/word_tokens(text)"));
}

#[test]
fn addressing_preserves_import_visibility_and_requires_a_concrete_receiver() {
    for (source, expected) in [
        (
            "counts := {\"a\": 1u64}\ncounts_keys\n",
            "source-semantics/unavailable-underscore-address",
        ),
        (
            "+> map\ncounts := signal\ncounts_keys\n",
            "source-semantics/unresolved-underscore-address",
        ),
        (
            "+> map\ncounts := 1u64\ncounts_keys\n",
            "source-semantics/unavailable-underscore-address",
        ),
    ] {
        let error = compile(source, mech_stdlib::source_catalog()).unwrap_err();
        assert_eq!(error.code, expected, "{source}: {error:?}");
    }
}

#[test]
fn completion_and_underscore_selection_share_the_public_registered_member_query() {
    let catalog = custom_catalog(&[
        ("measure/tokens", "sequence/length"),
        ("flags/tokens", "logic/not"),
    ]);
    let mut scope = mech_engine::FunctionEnvironment::from_catalog_defaults(&catalog).unwrap();
    let string_type =
        mech_core::ResolvedType::from_schema_body(&mech_core::SchemaBody::String, &[]).unwrap();
    let query = || mech_engine::ReceiverMemberQuery {
        exact_name: Some("tokens"),
        ..Default::default()
    };
    assert!(
        scope
            .receiver_members(&catalog, Some(&string_type), query())
            .is_empty()
    );
    let measure = catalog.module_export("measure", "tokens").unwrap();
    scope.bind_catalog_export(measure, "m/tokens").unwrap();
    let members = scope.receiver_members(&catalog, Some(&string_type), query());
    assert_eq!(members.len(), 1);
    assert_eq!(members[0].canonical_name, "measure/tokens");
    assert_eq!(members[0].module.as_deref(), Some("measure"));
    assert_eq!(members[0].operation, measure.operation);
    assert_eq!(
        members[0]
            .visible_names
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["m/tokens"]
    );
    assert!(matches!(
        &members[0].compatibility,
        mech_engine::ReceiverMemberCompatibility::Available { .. }
    ));
    assert_eq!(
        members[0].declaration,
        *catalog.source_type_declaration("measure/tokens").unwrap()
    );

    let unresolved = scope.receiver_members(
        &catalog,
        None,
        mech_engine::ReceiverMemberQuery {
            exact_name: Some("tokens"),
            include_unavailable: true,
            ..Default::default()
        },
    );
    assert!(
        unresolved
            .iter()
            .any(|member| member.canonical_name == "measure/tokens"
                && matches!(
                    member.compatibility,
                    mech_engine::ReceiverMemberCompatibility::UnresolvedReceiver
                ))
    );
    let excluded = ["m"];
    assert!(
        scope
            .receiver_members(
                &catalog,
                Some(&string_type),
                mech_engine::ReceiverMemberQuery {
                    exact_name: Some("tokens"),
                    excluded_module_roots: &excluded,
                    ..Default::default()
                }
            )
            .is_empty()
    );

    // Merely remaining enabled does not preserve a replaced visible spelling.
    // Both the compiler and completion now see the same import-scope result.
    scope
        .bind_catalog_export(
            catalog.module_export("flags", "tokens").unwrap(),
            "m/tokens",
        )
        .unwrap();
    assert!(scope.operation_is_enabled(measure.operation));
    assert!(
        scope
            .receiver_members(&catalog, Some(&string_type), query())
            .is_empty()
    );
}

#[test]
fn completion_keeps_extra_arguments_multi_outputs_and_effect_contracts_descriptive() {
    use mech_core::{
        AccessMode, AliasPolicy, BuiltinScalarKind, ChangeDetectionPolicy, DeliveryMode,
        EffectContract, EffectDeliveryPolicy, ExternalInteraction, FunctionTypeDeclaration,
        IdempotencyRequirement, InputKindScheme, InputPortLayout, InputPortPolicy, KindScheme,
        OperationContractDeclaration, OutputConstruction, OutputPortPolicy, ShapeRule,
    };
    let maintained = mech_stdlib::source_catalog();
    let mut builder = FunctionCatalogBuilder::from_catalog(&maintained);
    for (item, inputs, outputs, interaction) in [
        ("extra", 2usize, 1usize, ExternalInteraction::Pure),
        ("multiple", 1, 2, ExternalInteraction::Pure),
        (
            "effect",
            1,
            0,
            ExternalInteraction::Effect(EffectContract {
                delivery: EffectDeliveryPolicy::AtMostOnce,
                idempotency: IdempotencyRequirement::NotRequired,
            }),
        ),
    ] {
        let name = format!("fixture/{item}");
        let scheme = KindScheme::new(
            Box::new([]),
            Box::new([]),
            InputKindScheme::Fixed(
                vec![BuiltinScalarKind::String.kind_expr(); inputs].into_boxed_slice(),
            ),
            vec![BuiltinScalarKind::String.kind_expr(); outputs].into_boxed_slice(),
            Box::new([]),
        )
        .unwrap();
        let mut declaration = FunctionTypeDeclaration::from_schemes(vec![scheme]);
        if item == "extra" {
            declaration.parameter_names =
                Some(vec!["receiver".into(), "argument".into()].into_boxed_slice());
        }
        let contract = OperationContractDeclaration {
            inputs: InputPortLayout::Fixed(
                vec![
                    InputPortPolicy {
                        access: AccessMode::Read,
                        delivery: DeliveryMode::Signal,
                    };
                    inputs
                ]
                .into_boxed_slice(),
            ),
            outputs: vec![
                OutputPortPolicy {
                    access: AccessMode::Write,
                    delivery: DeliveryMode::Signal,
                    construction: OutputConstruction::FullWrite {
                        shape: ShapeRule::Declared
                    },
                    alias: AliasPolicy::NoAlias,
                    change_detection: ChangeDetectionPolicy::ExactScalar,
                };
                outputs
            ]
            .into_boxed_slice(),
            interaction,
        };
        let operation = builder
            .insert_canonical_specializer_with_contract(
                &name,
                declaration,
                contract,
                Arc::new(SourceOnlyProbe),
            )
            .unwrap();
        builder
            .insert_export(FunctionExport {
                operation,
                canonical_name: name,
                module: Some("fixture".into()),
                item: Some(item.into()),
                exposure: FunctionExposure::ModuleOnly,
            })
            .unwrap();
    }
    let catalog = builder.build().unwrap();
    let mut scope = mech_engine::FunctionEnvironment::from_catalog_defaults(&catalog).unwrap();
    for export in catalog.module_exports("fixture") {
        scope
            .bind_catalog_export(export, &export.canonical_name)
            .unwrap();
    }
    let string =
        mech_core::ResolvedType::from_schema_body(&mech_core::SchemaBody::String, &[]).unwrap();
    let members = scope.receiver_members(
        &catalog,
        Some(&string),
        mech_engine::ReceiverMemberQuery {
            role: mech_engine::ReceiverMemberRole::Completion,
            ..Default::default()
        },
    );
    for item in ["extra", "multiple", "effect"] {
        let member = members
            .iter()
            .find(|member| member.canonical_name == format!("fixture/{item}"))
            .unwrap();
        assert!(matches!(&member.compatibility,
            mech_engine::ReceiverMemberCompatibility::PrefixCompatible { candidate_ids }
                if candidate_ids.as_ref() == [0]));
        assert_eq!(member.contracts.len(), 1);
        assert_eq!(
            member.contracts.as_ref(),
            catalog
                .specializer(member.operation)
                .unwrap()
                .operation_contracts(),
        );
        assert!(
            scope
                .receiver_members(
                    &catalog,
                    Some(&string),
                    mech_engine::ReceiverMemberQuery {
                        exact_name: Some(item),
                        include_unavailable: true,
                        ..Default::default()
                    },
                )
                .is_empty(),
            "unary read sugar must not enable fixture/{item}"
        );
    }
    let number =
        mech_core::ResolvedType::new(BuiltinScalarKind::U64.kind_expr(), Box::new([])).unwrap();
    assert!(
        scope
            .receiver_members(
                &catalog,
                Some(&number),
                mech_engine::ReceiverMemberQuery {
                    role: mech_engine::ReceiverMemberRole::Completion,
                    exact_name: Some("extra"),
                    ..Default::default()
                },
            )
            .is_empty(),
        "registered String receiver must reject U64"
    );
    let extra = members
        .iter()
        .find(|member| member.name == "extra")
        .unwrap();
    assert_eq!(
        extra.declaration.parameter_names.as_deref().unwrap(),
        ["receiver", "argument"],
    );
    let effect = members
        .iter()
        .find(|member| member.name == "effect")
        .unwrap();
    assert!(matches!(
        &effect.contracts[0].interaction,
        ExternalInteraction::Effect(_)
    ));
    assert!(effect.contracts[0].outputs.is_empty());
}

#[test]
fn canonical_item_suffix_and_imported_call_alias_are_distinct_metadata() {
    let catalog = mech_stdlib::source_catalog();
    let mut scope = mech_engine::FunctionEnvironment::from_catalog_defaults(&catalog).unwrap();
    let export = catalog.module_export("sequence", "length").unwrap();
    scope.bind_catalog_export(export, "len").unwrap();
    let string =
        mech_core::ResolvedType::from_schema_body(&mech_core::SchemaBody::String, &[]).unwrap();
    let members = scope.receiver_members(
        &catalog,
        Some(&string),
        mech_engine::ReceiverMemberQuery {
            exact_name: Some("length"),
            ..Default::default()
        },
    );
    let member = members
        .iter()
        .find(|member| member.operation == export.operation)
        .unwrap();
    assert!(member.visible_names.iter().any(|name| name == "len"));
    assert_eq!(member.name, "length");
    assert!(
        scope
            .receiver_members(
                &catalog,
                Some(&string),
                mech_engine::ReceiverMemberQuery {
                    exact_name: Some("len"),
                    include_unavailable: true,
                    ..Default::default()
                },
            )
            .is_empty()
    );
    let (artifact, values) = execute(
        "+> len := sequence/length\ntext := \"abc\"\ntext_length\n",
        catalog,
    );
    assert!(operations(&artifact).contains(&"sequence/length".into()));
    assert!(
        values
            .iter()
            .all(|value| matches!(value.data(), ValueData::U64(3)))
    );
}

#[test]
fn receiver_member_scope_excludes_exact_callable_shadows_not_other_aliases() {
    let catalog = custom_catalog(&[("measure/tokens", "sequence/length")]);
    let mut scope = mech_engine::FunctionEnvironment::from_catalog_defaults(&catalog).unwrap();
    let export = catalog.module_export("measure", "tokens").unwrap();
    scope.bind_catalog_export(export, "m/tokens").unwrap();
    scope.bind_catalog_export(export, "len").unwrap();
    let string =
        mech_core::ResolvedType::from_schema_body(&mech_core::SchemaBody::String, &[]).unwrap();
    for role in [
        mech_engine::ReceiverMemberRole::UnaryPureProjection,
        mech_engine::ReceiverMemberRole::Completion,
    ] {
        let members = scope.receiver_members(
            &catalog,
            Some(&string),
            mech_engine::ReceiverMemberQuery {
                role,
                exact_name: Some("tokens"),
                excluded_names: &["m/tokens"],
                ..Default::default()
            },
        );
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].visible_names.as_ref(), ["len".to_owned()]);
        // An exact callable called `m` does not shadow `m/tokens`.
        let members = scope.receiver_members(
            &catalog,
            Some(&string),
            mech_engine::ReceiverMemberQuery {
                role,
                exact_name: Some("tokens"),
                excluded_names: &["m", "len"],
                ..Default::default()
            },
        );
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].visible_names.as_ref(), ["m/tokens".to_owned()]);
        let hidden = scope.receiver_members(
            &catalog,
            Some(&string),
            mech_engine::ReceiverMemberQuery {
                role,
                exact_name: Some("tokens"),
                excluded_module_roots: &["m"],
                excluded_names: &["len"],
                include_unavailable: true,
            },
        );
        assert_eq!(hidden.len(), 1);
        assert!(hidden[0].visible_names.is_empty());
        assert!(matches!(
            &hidden[0].compatibility,
            mech_engine::ReceiverMemberCompatibility::NotVisible
        ));
        assert!(scope.operation_is_enabled(export.operation));
    }
}

#[test]
fn underscore_projection_respects_ordinary_exact_document_function_precedence() {
    let catalog = mech_stdlib::source_catalog();
    let local = "set/size(x<u64>) => (out<u64>) := out := 99u64.\n";
    // The canonical Identifier grammar permits slash in a document function
    // name. Ordinary calls select that exact lexical function over a prelude.
    let (artifact, values) = execute(&format!("{local}set/size(7u64)\n"), catalog.clone());
    assert!(!operations(&artifact).contains(&"set/size".to_owned()));
    assert!(
        values
            .iter()
            .all(|value| matches!(value.data(), ValueData::U64(99)))
    );
    let error = compile(
        &format!("{local}values := {{1u64, 2u64}}\nvalues_size\n"),
        catalog.clone(),
    )
    .unwrap_err();
    assert_eq!(
        error.code,
        "source-semantics/unavailable-underscore-address"
    );
    // Exact callable shadowing is not a namespace-root exclusion.
    let (artifact, values) = execute(
        "set(x<u64>) => (out<u64>) := out := 99u64.\nvalues := {1u64, 2u64}\nvalues_size\n",
        catalog,
    );
    assert!(operations(&artifact).contains(&"set/size".to_owned()));
    assert!(
        values
            .iter()
            .all(|value| matches!(value.data(), ValueData::U64(2)))
    );
}

#[test]
fn planning_source_module_roots_shadow_prelude_projection_spellings() {
    use std::collections::{BTreeMap, BTreeSet};
    let catalog = mech_stdlib::source_catalog();
    let roots = BTreeSet::from(["set".to_owned()]);
    for (expression, code) in [
        (
            "values_size",
            "source-semantics/unavailable-underscore-address",
        ),
        (
            "set/size(values)",
            "source-semantics/source-module-value-not-callable",
        ),
    ] {
        let source = format!("values := {{1u64, 2u64}}\n{expression}\n");
        let error = CanonicalSourceFrontend
            .compile_document_with_planning_contract(
                &syntax(&source),
                catalog.clone(),
                BTreeMap::new(),
                BTreeMap::new(),
                &BTreeSet::new(),
                &BTreeSet::new(),
                &roots,
            )
            .err()
            .expect("resolved source module must shadow the prelude spelling");
        assert_eq!(error.code, code);
    }
}

#[test]
fn query_and_ordinary_call_reject_merged_overloads_with_conflicting_output_rules() {
    use mech_core::{
        BuiltinScalarKind, FunctionTypeDeclaration, InputKindScheme, KindParameter,
        KindParameterId, KindScheme, ResolvedOutputSchemaRule, SchemaBody,
    };
    let maintained = mech_stdlib::source_catalog();
    let make_scheme = |parameters| {
        KindScheme::new(
            parameters,
            Box::new([]),
            InputKindScheme::Fixed(vec![BuiltinScalarKind::String.kind_expr()].into_boxed_slice()),
            vec![BuiltinScalarKind::U64.kind_expr()].into_boxed_slice(),
            Box::new([]),
        )
        .unwrap()
    };
    // Distinct declarations, equal score/result/conversions: the core solver
    // merges both candidate IDs. An unused quantified parameter is legal.
    let mut declaration = FunctionTypeDeclaration::from_schemes(vec![
        make_scheme(Box::new([])),
        make_scheme(
            vec![KindParameter {
                id: KindParameterId::new(0),
                upper_bound: None,
            }]
            .into_boxed_slice(),
        ),
    ]);
    declaration.overloads[1].output_schema_rules[0] =
        ResolvedOutputSchemaRule::Declared(BuiltinScalarKind::U64.schema_body());
    let mut builder = FunctionCatalogBuilder::from_catalog(&maintained);
    let operation = builder
        .insert_canonical_specializer_with_contract(
            "fixture/conflict",
            declaration,
            maintained
                .source_operation_contract("sequence/length", 1, false)
                .unwrap()
                .clone(),
            Arc::new(SourceOnlyProbe),
        )
        .unwrap();
    builder
        .insert_export(FunctionExport {
            operation,
            canonical_name: "fixture/conflict".into(),
            module: Some("fixture".into()),
            item: Some("conflict".into()),
            exposure: FunctionExposure::ModuleOnly,
        })
        .unwrap();
    let catalog = Arc::new(builder.build().unwrap());
    let mut scope = mech_engine::FunctionEnvironment::from_catalog_defaults(&catalog).unwrap();
    scope
        .bind_catalog_export(
            catalog.module_export("fixture", "conflict").unwrap(),
            "fixture/conflict",
        )
        .unwrap();
    let receiver = mech_core::ResolvedType::from_schema_body(&SchemaBody::String, &[]).unwrap();
    let declaration = catalog.source_type_declaration("fixture/conflict").unwrap();
    let candidates = declaration
        .overloads
        .iter()
        .map(|overload| mech_core::TypeOverloadCandidate {
            id: u64::from(overload.id),
            scheme: &overload.scheme,
        })
        .collect::<Vec<_>>();
    let resolution = mech_core::resolve_type_overloads(
        mech_core::TypeConstraintOrigin::new("fixture/conflict", None),
        &candidates,
        core::slice::from_ref(&receiver),
        None,
    )
    .unwrap();
    assert_eq!(resolution.candidate_ids.as_ref(), [0, 1]);
    let members = scope.receiver_members(
        &catalog,
        Some(&receiver),
        mech_engine::ReceiverMemberQuery {
            exact_name: Some("conflict"),
            include_unavailable: true,
            ..Default::default()
        },
    );
    assert_eq!(members.len(), 1);
    assert!(matches!(
        &members[0].compatibility,
        mech_engine::ReceiverMemberCompatibility::Incompatible
    ));
    let error = compile(
        "+> fixture\ntext := \"abc\"\nfixture/conflict(text)\n",
        catalog.clone(),
    )
    .unwrap_err();
    assert!(
        error
            .message
            .contains("selected conflicting output-schema rules")
    );
    let error = compile("+> fixture\ntext := \"abc\"\ntext_conflict\n", catalog).unwrap_err();
    assert_eq!(
        error.code,
        "source-semantics/unavailable-underscore-address"
    );
}
