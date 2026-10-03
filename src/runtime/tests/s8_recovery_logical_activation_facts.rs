//! Exact source/decoded acceptance for recovery finding G17's fixed-population
//! logical-mask activation facts.
#![cfg(all(feature = "full_source", feature = "resident-routing-source"))]

use mech_core::{
    DimensionExpr, FloatWidth, ReactiveInstanceId, ResidentValueRef, SchemaBody, Value,
};
use mech_engine::ProgramArtifact;
use mech_engine::resident::{
    ActivationFacts, CapturedSignalInput, ResidentActivationError, activate,
};
use mech_runtime::{
    ResidentDurabilityPolicy, RuntimeBuilder, RuntimeValueSnapshot, SourceDocument,
};
use mech_syntax::document::{ParseConfig, Revision};

#[test]
fn closed_boolean_matrix_literals_supply_selector_populations() {
    assert_closed_selector_source("x := [42 43]\np := 1 == 1\nmask := [p false]\nx[mask]\n");
}

fn assert_closed_selector_source(source: &str) {
    let artifact = compile(source);
    assert_closed_selector_artifact(source, &artifact);
}

fn assert_closed_selector_artifact(source: &str, artifact: &ProgramArtifact) {
    let decoded = roundtrip(artifact);
    for artifact in [artifact, &decoded] {
        let mut builder = mech_core::FunctionCatalogBuilder::new();
        mech_engine::install_intrinsic_resident(&mut builder).unwrap();
        let catalog = builder.build().unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(0x606, 0),
            artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap_or_else(|error| panic!("{source}: {error:?}"));
        instance.turn(&[]).unwrap();
        if artifact.outputs().len() > 1 {
            let compare = artifact
                .nodes()
                .iter()
                .find(|node| {
                    node.as_operation().is_some_and(|operation| {
                        operation.operation.canonical_name() == "compare/seq"
                    })
                })
                .unwrap();
            let mech_engine::BindingDeclaration::Input {
                source: mech_engine::ArtifactSource::Constant(expected),
                ..
            } = artifact.bindings()[compare.input_bindings.start as usize + 1]
            else {
                panic!("expected constant")
            };
            let actual_producer = instance.copied_output(0).unwrap();
            assert!(
                actual_producer
                    .snapshot_eq(
                        artifact.schemas(),
                        artifact.constants().get(expected).unwrap(),
                        artifact.schemas()
                    )
                    .unwrap(),
                "complete producer identity: {source}"
            );
        }
        let selected = instance
            .copied_output(artifact.outputs().len() - 1)
            .unwrap();
        assert_eq!(
            RuntimeValueSnapshot::from_value(selected.clone())
                .unwrap()
                .format_canonical_inline(),
            "[42]",
            "{source}"
        );
        assert_selected_identity(artifact, &selected, 1, 1);
    }
}

fn compile(source: &str) -> ProgramArtifact {
    compile_with_catalog(source, mech_stdlib::source_catalog())
}

fn compile_with_catalog(
    source: &str,
    catalog: std::sync::Arc<mech_core::FunctionCatalog>,
) -> ProgramArtifact {
    let document = SourceDocument::parse_resolved(
        "s8-recovery-logical-activation-facts.mec",
        Revision(0),
        source,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(
        document.is_strictly_clean(),
        "source did not parse cleanly: {:?}",
        document.snapshot().diagnostics.as_slice(),
    );
    RuntimeBuilder::new()
        .function_catalog(catalog)
        .build_compiler()
        .unwrap()
        .compile_document(&document)
        .unwrap_or_else(|error| panic!("failed to compile:\n{source}\n{error:?}"))
        .artifact()
        .clone()
}

fn roundtrip(artifact: &ProgramArtifact) -> ProgramArtifact {
    mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(artifact).unwrap(),
    )
    .unwrap()
}

fn output(instance: &mech_engine::resident::ReactiveInstance) -> String {
    RuntimeValueSnapshot::from_value(instance.copied_output(0).unwrap())
        .unwrap()
        .format_canonical_inline()
}

fn assert_selected_identity(artifact: &ProgramArtifact, value: &Value, rows: u64, columns: u64) {
    assert_eq!(value.schema(), artifact.outputs().last().unwrap().schema);
    let schema = artifact.schemas().get(value.schema()).unwrap();
    assert_eq!(value.schema_key(), schema.key());
    assert_eq!(
        schema.closed_body(value.shape()).unwrap(),
        SchemaBody::Matrix {
            element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
            dimensions: vec![
                DimensionExpr::Constant(rows),
                DimensionExpr::Constant(columns),
            ]
            .into_boxed_slice(),
        },
    );
}

#[test]
fn closed_boolean_matrix_literal_preserves_nonsquare_coordinate_order() {
    exact_closed_mask_identity(
        "x := [42 43 44 45 46 47]\np := 1 == 1\nq := 1 == 2\nmask := [q p q; p q q]\nx[mask]\n",
        "[43; 44]",
        (2, 1),
    );
}

#[test]
fn resolved_boolean_literal_contract_drives_direct_and_composed_selectors() {
    for selector in ["mask", "mask == true", "logic/and(mask,true)"] {
        let source = format!(
            "x := [42 43 44 45 46 47]\np := 1 == 1\nq := 1 == 2\nmask := [q p q; p q q]\nx[{selector}]\n"
        );
        let artifact = force_resolved_boolean_literal(&compile(&source));
        assert!(artifact.nodes().iter().any(|node| {
            node.as_operation()
                .is_some_and(|op| op.operation.canonical_name() == "matrix/literal")
        }));
        let decoded = roundtrip(&artifact);
        for artifact in [&artifact, &decoded] {
            let mut builder = mech_core::FunctionCatalogBuilder::new();
            mech_engine::install_intrinsic_resident(&mut builder).unwrap();
            let mut instance = activate(
                ReactiveInstanceId::new(606, 3),
                artifact,
                &builder.build().unwrap(),
                &ActivationFacts::default(),
            )
            .unwrap();
            instance.turn(&[]).unwrap();
            let actual = instance.copied_output(0).unwrap();
            assert_eq!(
                RuntimeValueSnapshot::from_value(actual.clone())
                    .unwrap()
                    .format_canonical_inline(),
                "[43; 44]"
            );
            assert_selected_identity(artifact, &actual, 2, 1);
        }
    }
}

// The authoring path normally emits concatenation. Replace only its final
// constructor with the maintained, pure scalar-input literal contract; keep
// the predicate producers and downstream consumers unchanged.
fn force_resolved_boolean_literal(artifact: &ProgramArtifact) -> ProgramArtifact {
    use mech_core::*;
    use mech_engine::*;
    let scalar_outputs = artifact
        .nodes()
        .iter()
        .filter_map(|node| {
            let operation = node.as_operation()?;
            if operation.operation.canonical_name() != "compare/eq" {
                return None;
            }
            let BindingDeclaration::Output { target, .. } =
                artifact.bindings()[node.output_bindings.start as usize]
            else {
                return None;
            };
            (artifact
                .schemas()
                .get(artifact.slots()[target.get() as usize].schema)
                .unwrap()
                .body()
                == &SchemaBody::Bool)
                .then_some(target)
        })
        .take(2)
        .collect::<Vec<_>>();
    assert_eq!(scalar_outputs.len(), 2);
    let constructor = artifact
        .nodes()
        .iter()
        .find(|node| {
            node.as_operation()
                .is_some_and(|operation| operation.operation.canonical_name() == "matrix/vertcat")
        })
        .unwrap();
    let BindingDeclaration::Output { target, .. } =
        artifact.bindings()[constructor.output_bindings.start as usize]
    else {
        panic!("matrix output")
    };
    let output_schema = artifact.slots()[target.get() as usize].schema;
    let scalar_schema = artifact.slots()[scalar_outputs[0].get() as usize].schema;
    let mut contracts = OperationContractTableBuilder::new();
    let old = (0..artifact.contracts().len())
        .map(|index| {
            contracts
                .insert(
                    artifact
                        .contracts()
                        .get(OperationContractId::new(index as u32))
                        .unwrap()
                        .clone(),
                )
                .unwrap()
        })
        .collect::<Vec<_>>();
    let literal = contracts
        .insert(ResolvedOperationContract::Declared(
            DeclaredOperationContract {
                inputs: vec![
                    ResolvedInputPort {
                        schema: scalar_schema,
                        access: AccessMode::Read,
                        delivery: DeliveryMode::Signal
                    };
                    6
                ]
                .into_boxed_slice(),
                outputs: vec![ResolvedOutputPort {
                    schema: output_schema,
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
        ))
        .unwrap();
    let build = contracts.finish().unwrap();
    let remapped = old
        .into_iter()
        .map(|id| build.resolve(id).unwrap())
        .collect::<Vec<_>>();
    let literal = build.resolve(literal).unwrap();
    let (contracts, _) = build.into_parts();
    let mut nodes = artifact.nodes().to_vec();
    let mut bindings = Vec::new();
    for node in &mut nodes {
        let inputs = if node.node == constructor.node {
            [
                scalar_outputs[1],
                scalar_outputs[0],
                scalar_outputs[1],
                scalar_outputs[0],
                scalar_outputs[1],
                scalar_outputs[1],
            ]
            .map(ArtifactSource::Slot)
            .to_vec()
        } else {
            artifact.bindings()
                [node.input_bindings.start as usize..node.input_bindings.end as usize]
                .iter()
                .map(|binding| match binding {
                    BindingDeclaration::Input { source, .. } => *source,
                    _ => panic!("input"),
                })
                .collect()
        };
        let start = bindings.len() as u32;
        for (ordinal, source) in inputs.into_iter().enumerate() {
            bindings.push(BindingDeclaration::Input {
                id: BindingId::new(bindings.len() as u32),
                node: node.node,
                port_ordinal: ordinal as u16,
                source,
            });
        }
        let end = bindings.len() as u32;
        for binding in &artifact.bindings()
            [node.output_bindings.start as usize..node.output_bindings.end as usize]
        {
            let BindingDeclaration::Output {
                port_ordinal,
                target,
                ..
            } = binding
            else {
                panic!("output")
            };
            bindings.push(BindingDeclaration::Output {
                id: BindingId::new(bindings.len() as u32),
                node: node.node,
                port_ordinal: *port_ordinal,
                target: *target,
            });
        }
        node.input_bindings = start..end;
        node.output_bindings = end..bindings.len() as u32;
        let ExecutableNodeBody::Operation(operation) = &mut node.body else {
            panic!("pure fixture")
        };
        operation.contract = remapped[operation.contract.get() as usize];
        if node.node == constructor.node {
            operation.operation = OperationReference {
                module_path: vec!["matrix".to_owned()].into_boxed_slice(),
                operation_name: "literal".to_owned(),
            };
            operation.contract = literal;
        }
    }
    ProgramArtifactDraft {
        schemas: artifact.schemas().clone(),
        constants: artifact.constants().clone(),
        contracts,
        requirements: artifact.requirements().clone(),
        inputs: artifact.inputs().to_vec().into_boxed_slice(),
        slots: artifact.slots().to_vec().into_boxed_slice(),
        nodes: nodes.into_boxed_slice(),
        bindings: bindings.into_boxed_slice(),
        outputs: artifact.outputs().to_vec().into_boxed_slice(),
        constraints: artifact.constraints().to_vec().into_boxed_slice(),
        compute_regions: artifact.compute_regions().to_vec().into_boxed_slice(),
    }
    .finalize()
    .unwrap()
}

#[test]
fn closed_ekf_predicates_supply_selector_populations() {
    for predicate in [
        "ekf/candidate-finite([1; 2; 3], [1 0 0; 0 2 0; 0 0 3])",
        "ekf/covariance-positive-diagonal([1 0 0; 0 2 0; 0 0 3])",
        "ekf/covariance-symmetric([1 0 0; 0 2 0; 0 0 3])",
    ] {
        let source = format!("+> ekf\nx := [42]\np := {predicate}\nmask := [p]\nx[mask]\n");
        let artifact = compile_with_catalog(
            &source,
            mech_engine::__resident::frozen_ekf_compiler_catalog().unwrap(),
        );
        assert_closed_selector_artifact(&source, &artifact);
    }
}

#[test]
fn all_closed_ekf_operations_match_resident_values_and_shapes() {
    let catalog = mech_engine::__resident::frozen_ekf_compiler_catalog().unwrap();
    let cases = [
        "trigonometric-state([1; 2; 0.3])",
        "motion-jacobian([1; 2; 0.3], [2; 0.5; 3; 4], [0.8; 0.6], 0.2)",
        "control-jacobian([0.8; 0.6], 0.2)",
        "predicted-state([1; 2; 0.3], [2; 0.5; 3; 4], [0.8; 0.6], 0.2)",
        "predicted-covariance([2 0.1 0.2; 0.1 3 0.3; 0.2 0.3 4], [1 0 0.2; 0 1 0.3; 0 0 1], [0.2 0; 0.3 0; 0 0.2], [0.4 0.1; 0.1 0.5])",
        "landmark-delta-and-range([1; 2; 0.3], [4; 6])",
        "predicted-measurement([1; 2; 0.3], [3; 4; 5])",
        "measurement-jacobian([3; 4; 5])",
        "innovation-covariance([2 0.1 0.2; 0.1 3 0.3; 0.2 0.3 4], [0.2 0.3 0.4; 0.5 0.6 0.7], [0.8 0.1; 0.1 0.9])",
        "solve-2x2([2 0.5; 0.25 3])",
        "kalman-gain([2 0.1 0.2; 0.1 3 0.3; 0.2 0.3 4], [0.2 0.3 0.4; 0.5 0.6 0.7], [0.8 0.1; 0.1 0.9])",
        "innovation([2; 0.5; 3; 4], [2.5; 0.2])",
        "corrected-state([1; 2; 0.3], [0.2 0.3; 0.4 0.5; 0.6 0.7], [0.1; 0.2])",
        "joseph-covariance-update([2 0.1 0.2; 0.1 3 0.3; 0.2 0.3 4], [0.2 0.3 0.4; 0.5 0.6 0.7], [0.2 0.3; 0.4 0.5; 0.6 0.7], [0.8 0.1; 0.1 0.9])",
        "covariance-symmetrization([2 0.1 0.2; 0.4 3 0.3; 0.5 0.6 4])",
        "candidate-finite([1; 2; 0.3], [2 0.1 0.2; 0.1 3 0.3; 0.2 0.3 4])",
        "covariance-positive-diagonal([2 0.1 0.2; 0.1 3 0.3; 0.2 0.3 4])",
        "covariance-symmetric([2 0.1 0.2; 0.1 3 0.3; 0.2 0.3 4])",
    ];
    assert_eq!(cases.len(), 18);
    for expression in cases {
        let source = format!("+> ekf\nekf/{expression}\n");
        let producer = compile_with_catalog(&source, catalog.clone());
        let expected = resident_producer_value(&producer);
        let artifact = closed_strict_selector_artifact_with_shape(
            &producer,
            expected.canonical_data_draft().unwrap(),
            Some(expected.shape().clone()),
        );
        assert_closed_selector_artifact(&source, &artifact);
    }
}

fn resident_producer_value(artifact: &ProgramArtifact) -> Value {
    let mut builder = mech_core::FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut builder).unwrap();
    let catalog = builder.build().unwrap();
    let mut instance = activate(
        ReactiveInstanceId::new(0x607, 0),
        artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .unwrap();
    instance.turn(&[]).unwrap();
    instance.copied_output(0).unwrap()
}

#[test]
fn closed_set_expansions_match_resident_identity_and_drive_selectors() {
    for source in [
        "set/powerset({1u8, 2u8})\n",
        "set/cartesian-product({1u8, 2u8}, {3u8, 4u8})\n",
        "set/powerset({\"short\", \"a longer key\"})\n",
        "set/cartesian-product({\"short\", \"a longer key\"}, {3u8, 4u8})\n",
    ] {
        let producer = compile_intrinsic_document(source);
        let expected = resident_producer_value(&producer);
        let artifact = closed_strict_selector_artifact_with_shape(
            &producer,
            expected.canonical_data_draft().unwrap(),
            Some(expected.shape().clone()),
        );
        assert_closed_selector_artifact(source, &artifact);
    }
}

#[test]
fn closed_named_access_matches_heterogeneous_resident_columns_and_fields() {
    for source in [
        "t := (|key<u8> flag<bool> label<string>|1u8 false \"short\"|2u8 true \"a longer label\"|)\nt.label\n",
        "t := (|key<u8> flag<bool> label<string>|1u8 false \"short\"|2u8 true \"a longer label\"|)\nt.flag\n",
        "r := {label: \"a longer label\", flag: true}\nr.label\n",
        "r := {label: \"a longer label\", flag: true}\nr.flag\n",
        "joined := table/left-outer-join((|key<u8> flag<bool>|1u8 false|2u8 true|), (|key<u8>|2u8|))\njoined.flag\n",
    ] {
        let producer = compile_intrinsic_document(source);
        assert!(producer.nodes().iter().any(|node| {
            node.as_operation()
                .is_some_and(|operation| operation.operation.canonical_name() == "access/column")
        }));
        let expected = resident_producer_value(&producer);
        let artifact = closed_strict_selector_artifact_with_shape(
            &producer,
            expected.canonical_data_draft().unwrap(),
            Some(expected.shape().clone()),
        );
        assert_closed_selector_artifact(source, &artifact);
    }
    for selector in ["t.flag", "t.flag == true", "logic/and(t.flag, true)"] {
        let source = format!(
            "t := (|key<u8> flag<bool>|1u8 false|2u8 true|)\nx := [41 42]\nx[{selector}]\n"
        );
        assert_closed_selector_source(&source);
    }
}

#[test]
fn closed_variable_width_aggregate_gathers_drive_strict_selectors() {
    for access in ["m[[2 2], :]", "m[:, [3 1]]", "m[[6 2]]", "m[[2 1], [3 1]]"] {
        let source = format!(
            "m := [(1u8, (\"one\", true)) (2u8, (\"a longer two\", false)) (3u8, (\"three\", true)); (4u8, (\"four\", false)) (5u8, (\"five\", true)) (6u8, (\"a longer six\", false))]\n{access}\n"
        );
        let producer = compile_intrinsic_document(&source);
        let expected = resident_producer_value(&producer);
        let artifact = closed_strict_selector_artifact_with_shape(
            &producer,
            expected.canonical_data_draft().unwrap(),
            Some(expected.shape().clone()),
        );
        assert_closed_selector_artifact(&source, &artifact);
    }
}

#[test]
fn closed_ekf_predicates_retain_false_and_tolerance_boundary_semantics() {
    let catalog = mech_engine::__resident::frozen_ekf_compiler_catalog().unwrap();
    for (predicate, expected) in [
        ("candidate-finite([1; 2; 3], [1 0 0; 0 2 0; 0 0 3])", true),
        (
            "candidate-finite([1; 2; 0 / 0], [1 0 0; 0 2 0; 0 0 3])",
            false,
        ),
        (
            "candidate-finite([1; 2; 3], [1 0 0; 0 2 0; 0 0 1 / 0])",
            false,
        ),
        ("covariance-positive-diagonal([1 0 0; 0 0 0; 0 0 3])", false),
        (
            "covariance-positive-diagonal([1 0 0; 0 -2 0; 0 0 3])",
            false,
        ),
        (
            "covariance-symmetric([1 0.0000000001 0; 0 2 0; 0 0 3])",
            true,
        ),
        (
            "covariance-symmetric([1 0.0000000001000001 0; 0 2 0; 0 0 3])",
            false,
        ),
    ] {
        let source = format!("+> ekf\nx := [42]\np := ekf/{predicate}\nmask := [p]\nx[mask]\n");
        let artifact = compile_with_catalog(&source, catalog.clone());
        for artifact in [&artifact, &roundtrip(&artifact)] {
            let actual = resident_producer_value(artifact);
            assert_eq!(
                RuntimeValueSnapshot::from_value(actual.clone())
                    .unwrap()
                    .format_canonical_inline(),
                if expected { "[42]" } else { "[]" },
                "{source}"
            );
            assert_selected_identity(artifact, &actual, u64::from(expected), 1);
        }
    }
}

#[test]
fn closed_registered_matrix_comprehension_constructor_supplies_masks() {
    for selector in ["mask", "mask == true", "logic/and(mask, true)"] {
        let source = format!("x := [42 43]\np := 1 == 1\nmask := [p false]\nx[{selector}]\n");
        let artifact = compile(&source);
        let mut nodes = artifact.nodes().to_vec();
        let mut replaced = 0;
        for node in &mut nodes {
            if let mech_engine::ExecutableNodeBody::Operation(operation) = &mut node.body
                && operation.operation.canonical_name() == "matrix/horzcat"
            {
                // These two registered constructors share the maintained
                // horizontal-output contract, including the resident binder.
                operation.operation.operation_name = "comprehension".to_owned();
                replaced += 1;
            }
        }
        assert!(replaced > 0);
        let artifact = mech_engine::ProgramArtifactDraft {
            schemas: artifact.schemas().clone(),
            constants: artifact.constants().clone(),
            contracts: artifact.contracts().clone(),
            requirements: artifact.requirements().clone(),
            inputs: artifact.inputs().to_vec().into_boxed_slice(),
            slots: artifact.slots().to_vec().into_boxed_slice(),
            nodes: nodes.into_boxed_slice(),
            bindings: artifact.bindings().to_vec().into_boxed_slice(),
            outputs: artifact.outputs().to_vec().into_boxed_slice(),
            constraints: artifact.constraints().to_vec().into_boxed_slice(),
            compute_regions: artifact.compute_regions().to_vec().into_boxed_slice(),
        }
        .finalize()
        .unwrap();
        assert_closed_selector_artifact(&source, &artifact);
    }
}

#[test]
fn closed_lexical_comprehensions_supply_direct_and_composed_masks() {
    for producer in [
        "[y > 1 | y <- [1 2 3]]",
        "[(y ? | 1 => false | * => true) | y <- [1 2 3]]",
        "[(y ? | item, item > 1 => true | * => false) | y <- [1 2 3]]",
        "[head > 1 | ([head | rest], flag) <- [([1 2 3], true) ([4 5 6], false) ([7 8 9], true)]]",
    ] {
        for consumer in ["mask", "mask == true", "logic/and(mask, true)"] {
            let source = format!("x := [42 43 44]\nmask := {producer}\nx[{consumer}]\n");
            let artifact = compile(&source);
            assert!(artifact.nodes().iter().any(|node| matches!(
                node.body,
                mech_engine::ExecutableNodeBody::Comprehension(_)
            )));
            exact_closed_mask_identity(&source, "[43; 44]", (2, 1));
        }
    }
}

#[test]
fn closed_table_join_family_supplies_selector_populations() {
    for (operation, expected) in [
        ("join", "(|key<u8>|2u8|)"),
        ("left-outer-join", "(|key<u8>|1u8|2u8|)"),
        ("right-outer-join", "(|key<u8>|2u8|3u8|)"),
        ("full-outer-join", "(|key<u8>|1u8|2u8|3u8|)"),
        ("left-semi-join", "(|key<u8>|2u8|)"),
        ("left-anti-join", "(|key<u8>|1u8|)"),
    ] {
        let source = format!("table/{operation}((|key<u8>|1u8|2u8|), (|key<u8>|2u8|3u8|))\n");
        use mech_syntax::document::{AstNode, DocumentSyntax};
        let document = SourceDocument::parse_resolved(
            "closed-table-join.mec",
            Revision(0),
            source.as_str(),
            ParseConfig::default(),
        )
        .unwrap();
        let root = DocumentSyntax::cast(document.snapshot().syntax()).unwrap();
        let producer = mech_engine::CanonicalSourceFrontend
            .compile_document(&root)
            .unwrap()
            .compile_artifact()
            .unwrap();
        let expected_rows = match operation {
            "join" | "left-semi-join" => vec![2],
            "left-anti-join" => vec![1],
            "left-outer-join" => vec![1, 2],
            "right-outer-join" => vec![2, 3],
            _ => vec![1, 2, 3],
        };
        let artifact = closed_strict_selector_artifact(
            &producer,
            mech_core::ValueDataDraft::Table(
                vec![mech_core::snapshot::TableColumnDraft {
                    name: "key".to_owned(),
                    values: expected_rows
                        .into_iter()
                        .map(mech_core::ValueDataDraft::U8)
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                }]
                .into_boxed_slice(),
            ),
        );
        assert_closed_selector_artifact(&source, &artifact);
        let _ = expected; // The explicit rows above also cover provider row order.
    }
}

#[test]
fn closed_table_joins_preserve_duplicates_empty_results_and_outer_options() {
    use mech_core::ValueDataDraft as D;
    let optional = |value: Option<u8>| {
        D::Option(mech_core::snapshot::OptionDraft {
            present: value.is_some(),
            value: value.map(|value| Box::new(D::U8(value))),
        })
    };
    for (mode, keys, left, right) in [
        (
            "join",
            vec![1, 1],
            vec![Some(10), Some(11)],
            vec![Some(30), Some(30)],
        ),
        (
            "left-outer-join",
            vec![1, 1, 2],
            vec![Some(10), Some(11), Some(20)],
            vec![Some(30), Some(30), None],
        ),
        (
            "right-outer-join",
            vec![1, 1, 3],
            vec![Some(10), Some(11), None],
            vec![Some(30), Some(30), Some(40)],
        ),
        (
            "full-outer-join",
            vec![1, 1, 2, 3],
            vec![Some(10), Some(11), Some(20), None],
            vec![Some(30), Some(30), None, Some(40)],
        ),
        (
            "left-semi-join",
            vec![1, 1],
            vec![Some(10), Some(11)],
            vec![],
        ),
        ("left-anti-join", vec![2], vec![Some(20)], vec![]),
    ] {
        let source = format!(
            "table/{mode}((|key<u8> l<u8>|1u8 10u8|1u8 11u8|2u8 20u8|), (|key<u8> r<u8>|1u8 30u8|3u8 40u8|))\n"
        );
        let producer = compile_intrinsic_document(&source);
        let column = |name: &str, values: Vec<D>| mech_core::snapshot::TableColumnDraft {
            name: name.to_owned(),
            values: values.into_boxed_slice(),
        };
        let left_optional = matches!(mode, "right-outer-join" | "full-outer-join");
        let right_optional = matches!(mode, "left-outer-join" | "full-outer-join");
        let mut columns = vec![
            column("key", keys.into_iter().map(D::U8).collect()),
            column(
                "l",
                left.into_iter()
                    .map(|value| {
                        if left_optional {
                            optional(value)
                        } else {
                            D::U8(value.unwrap())
                        }
                    })
                    .collect(),
            ),
        ];
        if !matches!(mode, "left-semi-join" | "left-anti-join") {
            columns.push(column(
                "r",
                right
                    .into_iter()
                    .map(|value| {
                        if right_optional {
                            optional(value)
                        } else {
                            D::U8(value.unwrap())
                        }
                    })
                    .collect(),
            ));
        }
        let artifact =
            closed_strict_selector_artifact(&producer, D::Table(columns.into_boxed_slice()));
        assert_closed_selector_artifact(&source, &artifact);
    }
    let source = "table/join((|key<u8>|1u8|), (|key<u8>|2u8|))\n";
    let producer = compile_intrinsic_document(source);
    let artifact = closed_strict_selector_artifact(
        &producer,
        D::Table(
            vec![mech_core::snapshot::TableColumnDraft {
                name: "key".to_owned(),
                values: Box::new([]),
            }]
            .into_boxed_slice(),
        ),
    );
    assert_closed_selector_artifact(source, &artifact);
}

fn compile_intrinsic_document(source: &str) -> ProgramArtifact {
    use mech_syntax::document::{AstNode, DocumentSyntax};
    let document = SourceDocument::parse_resolved(
        "closed-producer.mec",
        Revision(0),
        source,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(
        document.is_strictly_clean(),
        "{source}: {:?}",
        document.snapshot().diagnostics
    );
    let root = DocumentSyntax::cast(document.snapshot().syntax()).unwrap();
    mech_engine::CanonicalSourceFrontend
        .compile_document(&root)
        .unwrap()
        .compile_artifact()
        .unwrap()
}

// Source typing intentionally declines strict comparisons against a join's
// data-dependent row dimension. Exercise the supported resolved contract,
// including a downstream selector, without broadening that frontend policy.
fn closed_snapshot_copy_artifact(
    module: &str,
    name: &str,
    inputs: Vec<(SchemaBody, mech_core::ValueDataDraft)>,
    output_body: SchemaBody,
) -> ProgramArtifact {
    use mech_core::snapshot::SnapshotValidationContext;
    use mech_core::*;
    use mech_engine::*;
    let schema = |body| {
        SchemaDraft {
            dimension_parameters: Box::new([]),
            body,
        }
        .finalize()
        .unwrap()
    };
    let mut schemas = SchemaTableBuilder::new();
    let input_schemas = inputs
        .iter()
        .map(|(body, _)| schemas.insert(schema(body.clone())).unwrap())
        .collect::<Vec<_>>();
    let output = schemas.insert(schema(output_body)).unwrap();
    let build = schemas.finish().unwrap();
    let input_schemas = input_schemas
        .into_iter()
        .map(|id| build.resolve(id).unwrap())
        .collect::<Vec<_>>();
    let output = build.resolve(output).unwrap();
    let (schemas, _) = build.into_parts();
    let mut constants = ConstantStoreBuilder::new(&schemas);
    let values = inputs
        .into_iter()
        .zip(&input_schemas)
        .map(|((_, data), schema)| {
            constants
                .insert(
                    ValueDraft {
                        schema: *schema,
                        shape_values: Box::new([]),
                        data,
                    }
                    .finalize(&SnapshotValidationContext::new(&schemas))
                    .unwrap(),
                )
                .unwrap()
        })
        .collect::<Vec<_>>();
    let build = constants.finish().unwrap();
    let values = values
        .into_iter()
        .map(|id| build.resolve(id).unwrap())
        .collect::<Vec<_>>();
    let (constants, _) = build.into_parts();
    let mut contracts = OperationContractTableBuilder::new();
    let contract = contracts
        .insert(ResolvedOperationContract::Declared(
            DeclaredOperationContract {
                inputs: input_schemas
                    .into_iter()
                    .map(|schema| ResolvedInputPort {
                        schema,
                        access: AccessMode::Read,
                        delivery: DeliveryMode::Signal,
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
                outputs: vec![ResolvedOutputPort {
                    schema: output,
                    access: AccessMode::Write,
                    delivery: DeliveryMode::Signal,
                    construction: if matches!(name, "horzcat" | "vertcat") {
                        OutputConstruction::Build {
                            postcondition: mech_core::ShapeContractReference {
                                module_path: vec!["matrix".to_owned(), "concatenate".to_owned()]
                                    .into_boxed_slice(),
                                contract_name: if name == "horzcat" {
                                    "horizontal-output"
                                } else {
                                    "vertical-output"
                                }
                                .to_owned(),
                            },
                        }
                    } else {
                        OutputConstruction::FullWrite {
                            shape: if name == "transpose" {
                                ShapeRule::TransposeOf { input: 0 }
                            } else {
                                ShapeRule::Declared
                            },
                        }
                    },
                    alias: AliasPolicy::NoAlias,
                    change_detection: if name == "literal" {
                        ChangeDetectionPolicy::AlwaysChanged
                    } else {
                        ChangeDetectionPolicy::KernelReported
                    },
                }]
                .into_boxed_slice(),
                interaction: ExternalInteraction::Pure,
            },
        ))
        .unwrap();
    let build = contracts.finish().unwrap();
    let contract = build.resolve(contract).unwrap();
    let (contracts, _) = build.into_parts();
    let mut bindings = values
        .into_iter()
        .enumerate()
        .map(|(index, value)| BindingDeclaration::Input {
            id: BindingId::new(index as u32),
            node: NodeId::new(0),
            port_ordinal: index as u16,
            source: ArtifactSource::Constant(value),
        })
        .collect::<Vec<_>>();
    let input_end = bindings.len() as u32;
    bindings.push(BindingDeclaration::Output {
        id: BindingId::new(input_end),
        node: NodeId::new(0),
        port_ordinal: 0,
        target: CellSlotId::new(0),
    });
    ProgramArtifactDraft {
        schemas,
        constants,
        contracts,
        requirements: Default::default(),
        inputs: Box::new([]),
        slots: vec![
            SlotDeclaration {
                slot: CellSlotId::new(0),
                schema: output,
                role: SlotRole::Derived,
                producer: ProducerReference::NodeOutput {
                    node: NodeId::new(0),
                    output_ordinal: 0,
                },
                initializer: None,
            },
            SlotDeclaration {
                slot: CellSlotId::new(1),
                schema: output,
                role: SlotRole::Output,
                producer: ProducerReference::Output {
                    source: ArtifactSource::Slot(CellSlotId::new(0)),
                    output: mech_core::OutputId::new(0),
                },
                initializer: None,
            },
        ]
        .into_boxed_slice(),
        nodes: vec![NodeDeclaration {
            node: NodeId::new(0),
            body: mech_engine::ExecutableNodeBody::Operation(mech_engine::OperationNodeBody {
                operation: OperationReference {
                    module_path: vec![module.to_owned()].into_boxed_slice(),
                    operation_name: name.to_owned(),
                },
                contract,
                requirement: None,
            }),
            input_bindings: 0..input_end,
            output_bindings: input_end..input_end + 1,
        }]
        .into_boxed_slice(),
        bindings: bindings.into_boxed_slice(),
        outputs: vec![mech_engine::OutputDeclaration {
            output: mech_core::OutputId::new(0),
            name: "result".to_owned(),
            interactive_binding: None,
            source: CellSlotId::new(1),
            schema: output,
        }]
        .into_boxed_slice(),
        constraints: Box::new([]),
        compute_regions: Box::new([]),
    }
    .finalize()
    .unwrap()
}

fn closed_strict_selector_artifact(
    producer: &ProgramArtifact,
    expected: mech_core::ValueDataDraft,
) -> ProgramArtifact {
    closed_strict_selector_artifact_with_shape(producer, expected, None)
}

fn closed_strict_selector_artifact_with_shape(
    producer: &ProgramArtifact,
    expected: mech_core::ValueDataDraft,
    expected_shape: Option<mech_core::ShapeInstance>,
) -> ProgramArtifact {
    closed_strict_selector_artifact_with_comparison(producer, expected, expected_shape, None, "seq")
}

fn closed_strict_selector_artifact_with_comparison(
    producer: &ProgramArtifact,
    expected: mech_core::ValueDataDraft,
    expected_shape: Option<mech_core::ShapeInstance>,
    expected_body: Option<SchemaBody>,
    comparison: &str,
) -> ProgramArtifact {
    use mech_core::*;
    use mech_engine::{
        ArtifactSource, BindingDeclaration, ExecutableNodeBody, InitializerReference,
        NodeDeclaration, OperationNodeBody, OperationReference, OutputDeclaration,
        ProducerReference, ProgramArtifactDraft, SlotDeclaration, SlotRole,
    };
    let schema = |body, dimension_parameters| {
        SchemaDraft {
            body,
            dimension_parameters,
        }
        .finalize()
        .unwrap()
    };
    let mut additional = SchemaTableBuilder::new();
    let expected_key = expected_body.map(|body| schema(body, Box::new([])));
    if let Some(expected) = &expected_key {
        additional.insert(expected.clone()).unwrap();
    }
    let boolean_key = schema(SchemaBody::Bool, Box::new([]));
    additional.insert(boolean_key.clone()).unwrap();
    let matrix = |element, dimensions| SchemaBody::Matrix {
        element: Box::new(element),
        dimensions,
    };
    let mask_key = schema(
        matrix(
            SchemaBody::Bool,
            vec![DimensionExpr::Constant(1), DimensionExpr::Constant(1)].into_boxed_slice(),
        ),
        Box::new([]),
    );
    additional.insert(mask_key.clone()).unwrap();
    let numeric_key = schema(
        matrix(
            SchemaBody::FloatingPoint(FloatWidth::W64),
            vec![DimensionExpr::Constant(1), DimensionExpr::Constant(1)].into_boxed_slice(),
        ),
        Box::new([]),
    );
    additional.insert(numeric_key.clone()).unwrap();
    let selected_key = schema(
        matrix(
            SchemaBody::FloatingPoint(FloatWidth::W64),
            vec![
                DimensionExpr::Parameter(DimensionParameterId::new(0)),
                DimensionExpr::Constant(1),
            ]
            .into_boxed_slice(),
        ),
        vec![DimensionParameterDeclaration {
            id: DimensionParameterId::new(0),
            origin: DimensionParameterOrigin::Explicit,
            lifetime: DimensionLifetime::Activation,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: Some(DimensionExpr::Constant(1)),
        }]
        .into_boxed_slice(),
    );
    additional.insert(selected_key.clone()).unwrap();
    for entry in producer.schemas().entries() {
        additional.insert(entry.schema().clone()).unwrap();
    }
    let (schemas, _) = additional.finish().unwrap().into_parts();
    let remapped_schemas = producer
        .schemas()
        .entries()
        .map(|entry| schemas.find_by_key(entry.key()).unwrap())
        .collect::<Vec<_>>();
    let boolean = schemas.find_by_key(boolean_key.key()).unwrap();
    let mask = schemas.find_by_key(mask_key.key()).unwrap();
    let numeric = schemas.find_by_key(numeric_key.key()).unwrap();
    let selected = schemas.find_by_key(selected_key.key()).unwrap();
    let source_slot = producer.outputs()[0].source;
    let ProducerReference::Output {
        source: ArtifactSource::Slot(source_slot),
        ..
    } = producer.slots()[source_slot.get() as usize].producer
    else {
        panic!("closed producer output")
    };
    let source_schema =
        remapped_schemas[producer.slots()[source_slot.get() as usize].schema.get() as usize];
    let expected_schema = expected_key.as_ref().map_or(source_schema, |schema| {
        schemas.find_by_key(schema.key()).unwrap()
    });
    let expected_shape = expected_shape.unwrap_or_else(|| {
        shape_for_value_data(schemas.get(expected_schema).unwrap(), &expected, &[], None).unwrap()
    });
    let expected = ValueDraft {
        schema: expected_schema,
        shape_values: expected_shape
            .parameter_values()
            .to_vec()
            .into_boxed_slice(),
        data: expected,
    }
    .finalize(&mech_core::snapshot::SnapshotValidationContext::new(
        &schemas,
    ))
    .unwrap();
    let numeric_value = ValueDraft {
        schema: numeric,
        shape_values: Box::new([]),
        data: ValueDataDraft::Matrix(
            vec![ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_f64(
                42.0,
            ))]
            .into_boxed_slice(),
        ),
    }
    .finalize(&mech_core::snapshot::SnapshotValidationContext::new(
        &schemas,
    ))
    .unwrap();
    let mut constants = ConstantStoreBuilder::new(&schemas);
    let old_constants = (0..producer.constants().len())
        .map(|index| {
            let value = producer
                .constants()
                .get(ConstantId::new(index as u32))
                .unwrap();
            let remapped_value = ValueDraft {
                schema: remapped_schemas[value.schema().get() as usize],
                shape_values: value.shape().parameter_values().to_vec().into_boxed_slice(),
                data: value.canonical_data_draft().unwrap(),
            }
            .finalize(&mech_core::snapshot::SnapshotValidationContext::new(
                &schemas,
            ))
            .unwrap();
            constants.insert(remapped_value).unwrap()
        })
        .collect::<Vec<_>>();
    let expected = constants.insert(expected).unwrap();
    let numeric_value = constants.insert(numeric_value).unwrap();
    let build = constants.finish().unwrap();
    let remapped = old_constants
        .into_iter()
        .map(|handle| build.resolve(handle).unwrap())
        .collect::<Vec<_>>();
    let expected = build.resolve(expected).unwrap();
    let numeric_value = build.resolve(numeric_value).unwrap();
    let (constants, _) = build.into_parts();
    let contract = |input_schemas: &[SchemaId], output_schema, change| {
        ResolvedOperationContract::Declared(DeclaredOperationContract {
            inputs: input_schemas
                .iter()
                .map(|schema| ResolvedInputPort {
                    schema: *schema,
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            outputs: vec![ResolvedOutputPort {
                schema: output_schema,
                access: AccessMode::Write,
                delivery: DeliveryMode::Signal,
                construction: OutputConstruction::FullWrite {
                    shape: ShapeRule::Declared,
                },
                alias: AliasPolicy::NoAlias,
                change_detection: change,
            }]
            .into_boxed_slice(),
            interaction: ExternalInteraction::Pure,
        })
    };
    let mut contracts = OperationContractTableBuilder::new();
    let old_contracts = (0..producer.contracts().len())
        .map(|index| {
            let mut contract = producer
                .contracts()
                .get(OperationContractId::new(index as u32))
                .unwrap()
                .clone();
            let ResolvedOperationContract::Declared(declared) = &mut contract else {
                panic!("ordinary table fixture")
            };
            for input in &mut declared.inputs {
                input.schema = remapped_schemas[input.schema.get() as usize];
            }
            for output in &mut declared.outputs {
                output.schema = remapped_schemas[output.schema.get() as usize];
            }
            contracts.insert(contract).unwrap()
        })
        .collect::<Vec<_>>();
    let compare = contracts
        .insert(contract(
            &[source_schema, expected_schema],
            boolean,
            ChangeDetectionPolicy::ExactScalar,
        ))
        .unwrap();
    let literal = contracts
        .insert(contract(
            &[boolean],
            mask,
            ChangeDetectionPolicy::AlwaysChanged,
        ))
        .unwrap();
    let access = contracts
        .insert(contract(
            &[numeric, mask],
            selected,
            ChangeDetectionPolicy::KernelReported,
        ))
        .unwrap();
    let build = contracts.finish().unwrap();
    let remapped_contracts = old_contracts
        .into_iter()
        .map(|handle| build.resolve(handle).unwrap())
        .collect::<Vec<_>>();
    let new_contracts = [compare, literal, access].map(|handle| build.resolve(handle).unwrap());
    let (contracts, _) = build.into_parts();
    let mut slots = producer.slots().to_vec();
    for slot in &mut slots {
        slot.schema = remapped_schemas[slot.schema.get() as usize];
        if let Some(InitializerReference::Constant(id)) = &mut slot.initializer {
            *id = remapped[id.get() as usize];
        }
    }
    let mut bindings = producer.bindings().to_vec();
    for binding in &mut bindings {
        if let BindingDeclaration::Input {
            source: ArtifactSource::Constant(id),
            ..
        } = binding
        {
            *id = remapped[id.get() as usize];
        }
    }
    let mut nodes = producer.nodes().to_vec();
    for node in &mut nodes {
        if let ExecutableNodeBody::Operation(operation) = &mut node.body {
            operation.contract = remapped_contracts[operation.contract.get() as usize];
        }
    }
    let first_slot = slots.len() as u32;
    let sources = [
        vec![
            ArtifactSource::Slot(source_slot),
            ArtifactSource::Constant(expected),
        ],
        vec![ArtifactSource::Slot(CellSlotId::new(first_slot))],
        vec![
            ArtifactSource::Constant(numeric_value),
            ArtifactSource::Slot(CellSlotId::new(first_slot + 1)),
        ],
    ];
    for (index, ((module, name), schema)) in [
        ("compare", comparison),
        ("matrix", "literal"),
        ("access", "range"),
    ]
    .into_iter()
    .zip([boolean, mask, selected])
    .enumerate()
    {
        let node = NodeId::new(nodes.len() as u32);
        let slot = CellSlotId::new(first_slot + index as u32);
        slots.push(SlotDeclaration {
            slot,
            schema,
            role: SlotRole::Derived,
            producer: ProducerReference::NodeOutput {
                node,
                output_ordinal: 0,
            },
            initializer: None,
        });
        let start = bindings.len() as u32;
        for (ordinal, source) in sources[index].iter().copied().enumerate() {
            bindings.push(BindingDeclaration::Input {
                id: BindingId::new(bindings.len() as u32),
                node,
                port_ordinal: ordinal as u16,
                source,
            });
        }
        let end = bindings.len() as u32;
        bindings.push(BindingDeclaration::Output {
            id: BindingId::new(end),
            node,
            port_ordinal: 0,
            target: slot,
        });
        nodes.push(NodeDeclaration {
            node,
            body: ExecutableNodeBody::Operation(OperationNodeBody {
                operation: OperationReference {
                    module_path: vec![module.to_owned()].into_boxed_slice(),
                    operation_name: name.to_owned(),
                },
                contract: new_contracts[index],
                requirement: None,
            }),
            input_bindings: start..end,
            output_bindings: end..end + 1,
        });
    }
    let output_slot = CellSlotId::new(slots.len() as u32);
    let output_id = OutputId::new(producer.outputs().len() as u32);
    slots.push(SlotDeclaration {
        slot: output_slot,
        schema: selected,
        role: SlotRole::Output,
        producer: ProducerReference::Output {
            source: ArtifactSource::Slot(CellSlotId::new(first_slot + 2)),
            output: output_id,
        },
        initializer: None,
    });
    let mut outputs = producer.outputs().to_vec();
    for output in &mut outputs {
        output.schema = remapped_schemas[output.schema.get() as usize];
    }
    outputs.push(OutputDeclaration {
        output: output_id,
        name: "selected".to_owned(),
        interactive_binding: None,
        source: output_slot,
        schema: selected,
    });
    ProgramArtifactDraft {
        schemas,
        constants,
        contracts,
        requirements: producer.requirements().clone(),
        inputs: producer.inputs().to_vec().into_boxed_slice(),
        slots: slots.into_boxed_slice(),
        nodes: nodes.into_boxed_slice(),
        bindings: bindings.into_boxed_slice(),
        outputs: outputs.into_boxed_slice(),
        constraints: producer.constraints().to_vec().into_boxed_slice(),
        compute_regions: producer.compute_regions().to_vec().into_boxed_slice(),
    }
    .finalize()
    .unwrap()
}

fn exact_closed_mask(source: &str, expected: &str) {
    let artifact = compile(source);
    let decoded = roundtrip(&artifact);
    let catalog = mech_stdlib::source_catalog();
    for artifact in [&artifact, &decoded] {
        let mut instance = activate(
            ReactiveInstanceId::new(0x58c, 6),
            artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap_or_else(|error| panic!("failed to activate:\n{source}\n{error:?}"));
        for _ in 0..2 {
            instance.turn(&[]).unwrap();
            assert_eq!(output(&instance), expected);
        }
    }
}

fn exact_closed_mask_identity(source: &str, expected: &str, expected_shape: (u64, u64)) {
    let artifact = compile(source);
    let decoded = roundtrip(&artifact);
    let catalog = mech_stdlib::source_catalog();
    for artifact in [&artifact, &decoded] {
        let mut instance = activate(
            ReactiveInstanceId::new(0x58c, 9),
            artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap_or_else(|error| panic!("{source}: {error:?}"));
        instance.turn(&[]).unwrap();
        let value = instance.copied_output(0).unwrap();
        assert_eq!(
            RuntimeValueSnapshot::from_value(value.clone())
                .unwrap()
                .format_canonical_inline(),
            expected,
        );
        assert_selected_identity(artifact, &value, expected_shape.0, expected_shape.1);
    }
}

#[test]
fn closed_literal_and_computed_masks_publish_exact_source_and_decoded_shapes() {
    exact_closed_mask("x := [1 2 3]\nx[[false true true]]\n", "[2; 3]");
    exact_closed_mask("x := [1 2 3]\nmask := x > 1\nx[mask]\n", "[2; 3]");
    exact_closed_mask_identity("x := [1 2 3]\nmask := x > 1\nx[mask]\n", "[2; 3]", (2, 1));
    exact_closed_mask_identity("x := [1 2 3]\nmask := x == 2\nx[mask]\n", "[2]", (1, 1));
    exact_closed_mask_identity("x := [1 2 3]\nmask := x > 3\nx[mask]\n", "[]", (0, 1));
}

#[test]
fn production_source_and_bytecode_loads_publish_closed_mask_identity() {
    let source = "x := [1 2 3]\nmask := x > 1\nx[mask]\n";
    let artifact = compile(source);
    for bytecode in [false, true] {
        let mut runtime = RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_catalog())
            .build()
            .unwrap();
        let outcome = if bytecode {
            runtime.load_bytecode_program(
                &mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap(),
                ResidentDurabilityPolicy::Volatile,
            )
        } else {
            runtime.load_source_program(source, ResidentDurabilityPolicy::Volatile)
        }
        .unwrap();
        let initial = outcome.initial_value.to_value();
        assert_eq!(
            RuntimeValueSnapshot::from_value(initial.clone())
                .unwrap()
                .format_canonical_inline(),
            "[2; 3]",
        );
        assert_selected_identity(&artifact, &initial, 2, 1);
        runtime.step_active_program().unwrap();
        let value = runtime
            .output_value(artifact.outputs()[0].output)
            .unwrap()
            .unwrap()
            .to_value();
        assert_selected_identity(&artifact, &value, 2, 1);
    }
}

#[test]
fn closed_comparison_masks_share_broadcast_and_ordering_semantics() {
    for (comparison, expected) in [
        ("x > 1", "[2; 3]"),
        ("x >= 2", "[2; 3]"),
        ("x < 3", "[1; 2]"),
        ("x <= 2", "[1; 2]"),
        ("x == 2", "[2]"),
        ("x != 2", "[1; 3]"),
        ("1 < x", "[2; 3]"),
    ] {
        exact_closed_mask(
            &format!("x := [1 2 3]\nmask := {comparison}\nx[mask]\n"),
            expected,
        );
    }
    exact_closed_mask("x := [1 2; 3 4]\nmask := x >= 3\nx[mask]\n", "[3; 4]");
    exact_closed_mask(
        "x := [1 2 3; 4 5 6]\nmask := x > [0 2.5 10]\nx[mask]\n",
        "[1; 4; 5]",
    );
    exact_closed_mask(
        "x := [42 43]\np := 1 < 2\nmask := [p false]\nx[mask]\n",
        "[42]",
    );
    exact_closed_mask("x := [1 2 3]\nmask := logic/not(x > 1)\nx[mask]\n", "[1]");
}

#[test]
fn closed_arithmetic_comparison_operands_publish_exact_populations() {
    for (arithmetic, comparison, expected) in [
        ("x + 1", "y > 2", "[2; 3]"),
        ("x - 1", "y >= 1", "[2; 3]"),
        ("x * 2", "y > 2", "[2; 3]"),
        ("x / 2", "y >= 1", "[2; 3]"),
        ("x % 2", "y == 1", "[1; 3]"),
        ("x ^ 2", "y > 3", "[2; 3]"),
        ("-x", "y < -1", "[2; 3]"),
    ] {
        exact_closed_mask(
            &format!("x := [1 2 3]\ny := {arithmetic}\nmask := {comparison}\nx[mask]\n"),
            expected,
        );
    }
    exact_closed_mask(
        "x := [1 2 3; 4 5 6]\ny := x + [10 20 30]\nmask := y > 24\nx[mask]\n",
        "[5; 3; 6]",
    );
    exact_closed_mask(
        "x := [1 2 3; 4 5 6]\ny := x + [10; 20]\nmask := y > 22\nx[mask]\n",
        "[4; 5; 6]",
    );
    exact_closed_mask(
        "x := [42 43]\np := (1/2 ^ 2<i32>) == 1/4\nmask := [p false]\nx[mask]\n",
        "[42]",
    );
}

#[test]
fn closed_binary_logical_masks_publish_exact_populations() {
    exact_closed_mask(
        "x := [42 43]\np := logic/and(1 < 2, 2 < 3)\nmask := [p false]\nx[mask]\n",
        "[42]",
    );
    for (operation, expected) in [("and", "[2]"), ("or", "[1; 2; 3]"), ("xor", "[1; 3]")] {
        exact_closed_mask(
            &format!("x := [1 2 3]\nmask := logic/{operation}(x > 1, x < 3)\nx[mask]\n"),
            expected,
        );
    }
    exact_closed_mask(
        "x := [1 2 3]\na := x > 1\nmask := a == true\nx[mask]\n",
        "[2; 3]",
    );
    exact_closed_mask(
        "x := [1 2 3]\na := logic/and(x > 0, x < 3)\nmask := a == false\nx[mask]\n",
        "[3]",
    );
}

#[test]
fn closed_comparisons_compose_through_supported_producers() {
    exact_closed_mask(
        "x := [42 43]\np := 1 < 2\nt := (p, 7)\nq := t == (true, 7)\nmask := [q false]\nx[mask]\n",
        "[42]",
    );
    exact_closed_mask(
        "x := [42 43]\np := 1 < 2\ntext<string> := p\nq := text == \"true\"\nmask := [q false]\nx[mask]\n",
        "[42]",
    );
    exact_closed_mask(
        "x := [42 43]\npresent := :some(7)\nq := present == :some(7)\nmask := [q false]\nx[mask]\n",
        "[42]",
    );
    exact_closed_mask(
        "+> string\nx := [42 43]\njoined := string/concat(\"ab\", \"cd\")\nq := joined == \"abcd\"\nmask := [q false]\nx[mask]\n",
        "[42]",
    );
    exact_closed_mask(
        "x := [42 43]\nvalues := [7 8]\nfirst := values[1]\np := first == 7\nmask := [p false]\nx[mask]\n",
        "[42]",
    );
    exact_closed_mask(
        "x := [42 43]\nvalues := [1 2; 3 4]\nsecond := values[2]\np := second == 3\nmask := [p false]\nx[mask]\n",
        "[42]",
    );
    exact_closed_mask(
        "x := [42 43]\nvalues := [1 2; 3 4]\ncell := values[2,1]\np := cell == 3\nmask := [p false]\nx[mask]\n",
        "[42]",
    );
}

#[test]
fn closed_comparisons_compose_through_set_size_and_matrix_access() {
    exact_closed_mask(
        "x := [42 43]\ncount := set/size({1, 2})\nmask := [count == 2<u64> false]\nx[mask]\n",
        "[42]",
    );
    exact_closed_mask(
        "x := [42 43]\ncombined := set/union({1}, {2})\ncount := set/size(combined)\nmask := [count == 2<u64> false]\nx[mask]\n",
        "[42]",
    );
    exact_closed_mask(
        "x := [42 43]\nvalues := [10 20 30; 40 50 60]\nselected := values[:,2]\nmask := selected > 25\nx[mask]\n",
        "[43]",
    );
    exact_closed_mask(
        "x := [42 43 44]\nvalues := [10 20 30; 40 50 60]\nselected := values[2,:]\nmask := selected > 45\nx[mask]\n",
        "[43; 44]",
    );
    exact_closed_mask(
        "x := [42 43]\nvalues := [10 20 30; 40 50 60]\nselected := values[[6 2]]\nmask := selected > 50\nx[mask]\n",
        "[42]",
    );
    exact_closed_mask(
        "x := [42 43; 44 45]\nvalues := [10 20 30; 40 50 60]\nselected := values[[2 1],[3 1]]\nmask := selected > 35\nx[mask]\n",
        "[42; 43]",
    );
}

#[test]
fn closed_boolean_access_is_equivalent_across_selector_routes() {
    for mask in [
        "values[:]",
        "values[:] == true",
        "logic/and(values[:], true)",
    ] {
        exact_closed_mask(
            &format!(
                "x := [40 41 42 43 44 45]\nvalues := [false false false; true false true]\nmask := {mask}\nx[mask]\n"
            ),
            "[41; 45]",
        );
    }
}

#[test]
fn closed_comparisons_compose_through_matrix_concatenation() {
    exact_closed_mask(
        "x := [10 20 30 40]\njoined := matrix/horzcat([1 2], [3 4])\nmask := joined > 2\nx[mask]\n",
        "[30; 40]",
    );
    exact_closed_mask(
        "x := [10 20; 30 40]\njoined := matrix/vertcat([1 2], [3 4])\nmask := joined > 2\nx[mask]\n",
        "[30; 40]",
    );
}

#[test]
fn closed_comparisons_compose_through_matrix_products() {
    exact_closed_mask(
        "x := [42 43; 44 45]\nproduct := matrix/matmul([1 2; 3 4], [5 6; 7 8])\nmask := product > 20\nx[mask]\n",
        "[44; 43; 45]",
    );
    exact_closed_mask(
        "x := [42 43; 44 45]\nproduct := matrix/matmul([1f32 2f32; 3f32 4f32], [5f32 6f32; 7f32 8f32])\nmask := product > 20f32\nx[mask]\n",
        "[44; 43; 45]",
    );
    exact_closed_mask(
        "x := [42 43; 44 45]\nproduct := matrix/matmul([1u8 2u8; 3u8 4u8], [5u8 6u8; 7u8 8u8])\nmask := product > 20u8\nx[mask]\n",
        "[44; 43; 45]",
    );
    exact_closed_mask(
        "x := [42 43]\nproduct := matrix/dot([1 2], [3 4])\nmask := [product == 11 false]\nx[mask]\n",
        "[42]",
    );
    exact_closed_mask(
        "x := [42 43]\nproduct := matrix/dot([1f32 2f32], [3f32 4f32])\nmask := [product == 11f32 false]\nx[mask]\n",
        "[42]",
    );
    exact_closed_mask(
        "x := [42 43]\nproduct := matrix/dot([1u8 2u8], [3u8 4u8])\nmask := [product == 11u8 false]\nx[mask]\n",
        "[42]",
    );
    exact_closed_mask(
        "x := [42 43]\nproduct := matrix/dot([10000000000000000.0 1.0; -10000000000000000.0 1.0], [1.0 1.0; 1.0 1.0])\nmask := [product == 2.0 false]\nx[mask]\n",
        "[42]",
    );
}

#[test]
fn closed_comparisons_compose_through_matrix_solve() {
    exact_closed_mask(
        "x := [42 43]\nsolution := matrix/solve([1.0 2.0; 3.0 4.0], [5.0; 11.0])\nmask := solution > 1.5\nx[mask]\n",
        "[43]",
    );
    exact_closed_mask(
        "x := [42 43]\nsolution := matrix/solve([1f32 2f32; 3f32 4f32], [5f32; 11f32])\nmask := solution > 1.5<f32>\nx[mask]\n",
        "[43]",
    );
}

#[test]
fn closed_n_choose_k_accepts_a_closed_slot_backed_selection() {
    exact_closed_mask(
        "+> combinatorics\nx := [42 43]\nk := 1 + 1\ncombinations := combinatorics/n-choose-k([1 2 3 4], k)\np := combinations === combinations\nmask := [p false]\nx[mask]\n",
        "[42]",
    );
}

#[test]
fn closed_comparisons_compose_through_row_and_column_reductions() {
    exact_closed_mask(
        "+> stats\nx := [42 43 44]\nreduced := stats/sum/row([1 2 3; 4 5 6])\nmask := reduced > 6\nx[mask]\n",
        "[43; 44]",
    );
    exact_closed_mask(
        "+> stats\nx := [42 43]\nreduced := stats/sum/column([1 2 3; 4 5 6])\nmask := reduced > 10\nx[mask]\n",
        "[43]",
    );
    exact_closed_mask(
        "+> stats\nx := [42 43 44]\nreduced := stats/sum/row([1f32 2f32 3f32; 4f32 5f32 6f32])\nmask := reduced > 6f32\nx[mask]\n",
        "[43; 44]",
    );
    exact_closed_mask(
        "+> stats\nx := [42 43]\nreduced := stats/sum/column([1u8 2u8 3u8; 4u8 5u8 6u8])\nmask := reduced > 10u8\nx[mask]\n",
        "[43]",
    );
}

#[test]
fn closed_comparisons_compose_through_absolute_value() {
    exact_closed_mask(
        "+> math\nx := [42 43]\nmagnitude := math/abs([-2.0 3.0])\nmask := magnitude > 2.5\nx[mask]\n",
        "[43]",
    );
}

#[test]
fn closed_comparisons_compose_through_resident_unary_float_operations() {
    exact_closed_mask(
        "+> math\nx := [42 43]\nrounded := math/floor([1.5 3.5])\nmask := rounded > 2\nx[mask]\n",
        "[43]",
    );
    exact_closed_mask(
        "+> math\nx := [42 43]\nrounded := math/floor([1.5<f32> 3.5<f32>])\nmask := rounded > 2f32\nx[mask]\n",
        "[43]",
    );
    for operation in [
        "acos",
        "acosh",
        "acot",
        "acsc",
        "asec",
        "asin",
        "asinh",
        "atan",
        "atanh",
        "cbrt",
        "ceil",
        "cos",
        "cosh",
        "cot",
        "csc",
        "erf",
        "erfc",
        "floor",
        "lgamma",
        "log",
        "log10",
        "log1p",
        "log2",
        "rint",
        "round",
        "roundeven",
        "sec",
        "sin",
        "sinh",
        "sqrt",
        "tan",
        "tanh",
        "tgamma",
        "trunc",
    ] {
        exact_closed_mask(
            &format!(
                "+> math\nx := [42 43]\ny := math/{operation}([1.0 1.0])\nmask := y == y\nx[mask]\n"
            ),
            "[42; 43]",
        );
    }
    for operation in ["j0", "j1", "y0", "y1"] {
        exact_closed_mask(
            &format!(
                "+> math\nx := [42 43]\ny := math/bessel/{operation}([1.0 1.0])\nmask := y == y\nx[mask]\n"
            ),
            "[42; 43]",
        );
    }
}

#[test]
fn closed_comparisons_compose_through_resident_binary_float_operations() {
    for operation in [
        "atan2",
        "copysign",
        "fdim",
        "fmod",
        "nextafter",
        "remainder",
    ] {
        exact_closed_mask(
            &format!(
                "+> math\nx := [42 43]\ny := math/{operation}([2.0 2.0], [1.0 1.0])\nmask := y == y\nx[mask]\n"
            ),
            "[42; 43]",
        );
    }
    for operation in ["jn", "yn"] {
        exact_closed_mask(
            &format!(
                "+> math\nx := [42 43]\ny := math/bessel/{operation}([1.0 1.0], [2.0 2.0])\nmask := y == y\nx[mask]\n"
            ),
            "[42; 43]",
        );
    }
    exact_closed_mask(
        "+> math\nx := [42 43]\ny := math/fmod([5.3<f32> 5.3<f32>], [2f32 2f32])\nmask := y == y\nx[mask]\n",
        "[42; 43]",
    );
}

#[test]
fn dense_string_comparisons_require_runtime_scan_admission() {
    exact_closed_mask(
        "x := [42 43]\ntext := [\"yes\" \"no\"]\nmask := text == \"yes\"\nx[mask]\n",
        "[42]",
    );

    let payload = usize::try_from(mech_core::RESIDENT_MAX_COMPARISON_WORK).unwrap() + 1;
    let source = format!(
        "x := [42 43]\ntext := [\"{}\" \"short\"]\nmask := text == text\nx[mask]\n",
        "x".repeat(payload),
    );
    let artifact = compile(&source);
    let decoded = roundtrip(&artifact);
    let catalog = mech_stdlib::source_catalog();
    for artifact in [&artifact, &decoded] {
        match activate(
            ReactiveInstanceId::new(0x58c, 14),
            artifact,
            &catalog,
            &ActivationFacts::default(),
        ) {
            Err(ResidentActivationError::UnresolvedShape { .. }) => {}
            Err(error) => panic!("unexpected oversized String comparison error: {error:?}"),
            Ok(_) => panic!("oversized String comparison inferred a selector population"),
        }
    }
}

#[test]
fn snapshot_gather_preparation_cannot_publish_a_cross_schema_selector_fact() {
    let matrix = |first: usize, second: usize| {
        format!(
            "m := [(1u8, \"{}\") (2u8, \"{}\")]\n",
            "a".repeat(first),
            "b".repeat(second)
        )
    };
    let large_matrix = matrix(15_000, 25_000);
    let producer = compile(&format!("{large_matrix}m[[2]]\n"));
    assert!(
        producer.nodes().iter().any(|node| node
            .as_operation()
            .is_some_and(|operation| operation.operation.canonical_name() == "access/range")),
        "fixture must execute the snapshot gather"
    );
    // This is a supported binding, not an UnsupportedLayout reproduction.
    // The selected payload fits alone, but source and selection preparation
    // cannot share one 65,536-work allowance.
    let catalog = mech_stdlib::source_catalog();
    for artifact in [&producer, &roundtrip(&producer)] {
        let instance = activate(
            ReactiveInstanceId::new(0x606, 40),
            artifact,
            &catalog,
            &ActivationFacts::default(),
        );
        match instance {
            Ok(mut instance) => assert!(
                instance.turn(&[]).is_err(),
                "resident preparation must refuse the cumulative demand"
            ),
            Err(ResidentActivationError::ActivationKernelExecution {
                error: mech_core::ResidentKernelError::InvalidShape,
                ..
            }) => {}
            Err(error) => panic!("unsupported preparation fixture: {error:?}"),
        }
    }
    for (comparison, selected) in [("===", "[]"), ("!==", "[42]")] {
        let source = format!(
            "{large_matrix}g := m[[2]]\nother<[(u8,string)]:1,1> := [(3u8, \"short\")]\np := g {comparison} other\nx := [42]\nx[[p]]\n"
        );
        let artifact = compile(&source);
        let decoded = roundtrip(&artifact);
        for artifact in [&artifact, &decoded] {
            assert!(
                matches!(
                    activate(
                        ReactiveInstanceId::new(0x606, 41),
                        artifact,
                        &catalog,
                        &ActivationFacts::default()
                    ),
                    Err(ResidentActivationError::UnresolvedShape { .. })
                ),
                "cross-schema comparison must not bypass refused producer preparation"
            );
        }
        exact_closed_mask(
            &format!(
                "{}g := m[[2]]\nother<[(u8,string)]:1,1> := [(3u8, \"short\")]\np := g {comparison} other\nx := [42]\nx[[p]]\n",
                matrix(50, 70)
            ),
            selected,
        );
        for bytecode in [false, true] {
            let mut runtime = RuntimeBuilder::new()
                .function_catalog(catalog.clone())
                .build()
                .unwrap();
            let rejected = if bytecode {
                runtime.load_bytecode_program(
                    &mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap(),
                    ResidentDurabilityPolicy::Volatile,
                )
            } else {
                runtime.load_source_program(&source, ResidentDurabilityPolicy::Volatile)
            };
            let error = rejected
                .err()
                .expect("production loading must refuse unavailable producer facts");
            assert!(
                format!("{error:?}").contains("UnresolvedShape"),
                "wrong rejection boundary: {error:?}"
            );
            let retry_source = format!(
                "{}g := m[[2]]\nother<[(u8,string)]:1,1> := [(3u8, \"short\")]\np := g {comparison} other\nx := [42]\nx[[p]]\n",
                matrix(50, 70)
            );
            let outcome = if bytecode {
                runtime.load_bytecode_program(
                    &mech_engine::encode_program_artifact_bytecode_v1(&compile(&retry_source))
                        .unwrap(),
                    ResidentDurabilityPolicy::Volatile,
                )
            } else {
                runtime.load_source_program(&retry_source, ResidentDurabilityPolicy::Volatile)
            }
            .unwrap();
            assert_eq!(outcome.initial_value.format_canonical_inline(), selected);
            runtime.step_active_program().unwrap();
        }
    }
}

#[test]
fn genuine_snapshot_literal_and_available_source_gather_require_producer_admission() {
    use mech_core::{IntegerWidth, ValueDataDraft as D};
    let tuple = SchemaBody::Tuple(
        vec![
            SchemaBody::UnsignedInteger(IntegerWidth::W8),
            SchemaBody::String,
        ]
        .into_boxed_slice(),
    );
    let matrix = |rows| SchemaBody::Matrix {
        element: Box::new(tuple.clone()),
        dimensions: vec![DimensionExpr::Constant(rows), DimensionExpr::Constant(1)]
            .into_boxed_slice(),
    };
    let cell =
        |id, bytes| D::Tuple(vec![D::U8(id), D::String("x".repeat(bytes))].into_boxed_slice());
    let catalog = mech_stdlib::source_catalog();
    for operation in ["literal", "range"] {
        for large in [false, true] {
            let (module, inputs) = if operation == "literal" {
                (
                    "matrix",
                    vec![(tuple.clone(), cell(1, if large { 65_537 } else { 70 }))],
                )
            } else {
                (
                    "access",
                    vec![
                        (
                            matrix(2),
                            D::Matrix(
                                vec![
                                    cell(1, if large { 15_000 } else { 50 }),
                                    cell(2, if large { 25_000 } else { 70 }),
                                ]
                                .into_boxed_slice(),
                            ),
                        ),
                        (SchemaBody::Index, D::Index(2)),
                    ],
                )
            };
            let producer = closed_snapshot_copy_artifact(module, operation, inputs, matrix(1));
            assert_eq!(
                producer.nodes().len(),
                1,
                "producer preparation must be isolated from preceding constructors"
            );
            for comparison in ["seq", "sneq"] {
                let artifact = closed_strict_selector_artifact_with_comparison(
                    &producer,
                    D::Bool(false),
                    None,
                    Some(SchemaBody::Bool),
                    comparison,
                );
                let decoded = roundtrip(&artifact);
                for artifact in [&artifact, &decoded] {
                    let instance = activate(
                        ReactiveInstanceId::new(0x606, 43),
                        artifact,
                        &catalog,
                        &ActivationFacts::default(),
                    );
                    if large {
                        assert!(
                            matches!(
                                instance,
                                Err(ResidentActivationError::UnresolvedShape { .. })
                            ),
                            "refused {module}/{operation} must not supply {comparison} population"
                        );
                    } else {
                        let mut instance = instance.unwrap();
                        instance.turn(&[]).unwrap();
                        let value = instance.copied_output(1).unwrap();
                        assert_selected_identity(
                            artifact,
                            &value,
                            u64::from(comparison == "sneq"),
                            1,
                        );
                        assert_eq!(
                            RuntimeValueSnapshot::from_value(value)
                                .unwrap()
                                .format_canonical_inline(),
                            if comparison == "sneq" { "[42]" } else { "[]" }
                        );
                    }
                }
                let mut runtime = RuntimeBuilder::new()
                    .function_catalog(catalog.clone())
                    .build()
                    .unwrap();
                let outcome = runtime.load_bytecode_program(
                    &mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap(),
                    ResidentDurabilityPolicy::Volatile,
                );
                if large {
                    assert!(
                        format!("{:?}", outcome.err().expect("producer admission"))
                            .contains("UnresolvedShape")
                    );
                    runtime
                        .load_source_program("[42]\n", ResidentDurabilityPolicy::Volatile)
                        .unwrap();
                } else {
                    outcome.unwrap();
                }
                runtime.step_active_program().unwrap();
            }
        }
    }
}

#[test]
fn closed_comparisons_compose_through_all_range_modes() {
    for range in ["1..4", "1..=3", "1..2..6", "1..2..=5"] {
        exact_closed_mask(
            &format!("x := [10 20 30]\nr := {range}\nmask := r > 1\nx[mask]\n"),
            "[20; 30]",
        );
    }
}

#[test]
fn closed_whole_value_comparisons_have_scalar_populations() {
    exact_closed_mask(
        "x := [42 43]\np := [1 2] === [1 2]\nmask := [p false]\nx[mask]\n",
        "[42]",
    );
    exact_closed_mask(
        "x := [42 43]\np := [1 2] !== [1 3]\nmask := [p false]\nx[mask]\n",
        "[42]",
    );
    exact_closed_mask(
        "x := [42 43]\np := [:Point :Point] == [:Point :Point]\nmask := [p false]\nx[mask]\n",
        "[42]",
    );
    exact_closed_mask(
        "x := [42 43]\np := [-0.0] === [0.0]\nmask := [p false]\nx[mask]\n",
        "[42]",
    );
    exact_closed_mask(
        "x := [42 43]\np := (-0.0, 1) == (0.0, 1)\nmask := [p true]\nx[mask]\n",
        "[43]",
    );
    exact_closed_mask(
        "x := [42 43]\np := {a: 1, b: 2} == {a: 1, b: 2}\nmask := [p false]\nx[mask]\n",
        "[42]",
    );
    exact_closed_mask(
        "x := [42 43]\np := {1: 2, 3: 4} == {1: 2, 3: 4}\nmask := [p false]\nx[mask]\n",
        "[42]",
    );
    exact_closed_mask(
        "x := [42 43]\np := (|a<u8>|1u8|) == (|a<u8>|1u8|)\nmask := [p false]\nx[mask]\n",
        "[42]",
    );
    exact_closed_mask(
        "x := [42 43]\nfixed := [1 2]\nparameterized<[f64]:1,2> := [1 2]\np := fixed === parameterized\nmask := [p false]\nx[mask]\n",
        "[42]",
    );
}

#[test]
fn live_masks_still_require_one_explicit_fixed_population_fact() {
    let artifact = compile("x := [1 2 3]\nx[mask<[bool]:1,3>]\n");
    let decoded = roundtrip(&artifact);
    let catalog = mech_stdlib::source_catalog();
    for artifact in [&artifact, &decoded] {
        let selected = artifact
            .slots()
            .iter()
            .find(|slot| {
                artifact.schemas().get(slot.schema).is_some_and(|schema| {
                    matches!(schema.body(), SchemaBody::Matrix { .. })
                        && !schema.dimension_parameters().is_empty()
                })
            })
            .unwrap();
        assert!(matches!(
            activate(
                ReactiveInstanceId::new(0x58c, 7),
                artifact,
                &catalog,
                &ActivationFacts::default(),
            ),
            Err(ResidentActivationError::UnresolvedShape { slot }) if slot == selected.slot
        ));

        let mut facts = ActivationFacts::default();
        facts.slot_shapes.insert(
            selected.slot,
            artifact
                .schemas()
                .get(selected.schema)
                .unwrap()
                .instantiate_shape(Box::new([2]))
                .unwrap(),
        );
        let mut instance = activate(
            ReactiveInstanceId::new(0x58c, 8),
            artifact,
            &catalog,
            &facts,
        )
        .unwrap();
        let mask_slot = instance
            .plan
            .inputs
            .iter()
            .find(|input| {
                artifact.inputs().iter().any(|declaration| {
                    declaration.name == mech_engine::encode_source_input_name("mask")
                        && declaration.slot == input.artifact_slot
                })
            })
            .unwrap()
            .slot;
        for (mask, expected) in [([0_u8, 1, 1], "[2; 3]"), ([1, 0, 1], "[1; 3]")] {
            instance
                .turn(&[CapturedSignalInput {
                    slot: mask_slot,
                    value: ResidentValueRef::Bool(&mask),
                }])
                .unwrap();
            assert_eq!(output(&instance), expected);
        }
        let previous = output(&instance);
        for mask in [[1_u8, 0, 0], [1, 1, 1], [2, 0, 0]] {
            assert!(
                instance
                    .turn(&[CapturedSignalInput {
                        slot: mask_slot,
                        value: ResidentValueRef::Bool(&mask),
                    }])
                    .is_err()
            );
            assert_eq!(output(&instance), previous);
        }
        instance
            .turn(&[CapturedSignalInput {
                slot: mask_slot,
                value: ResidentValueRef::Bool(&[0, 1, 1]),
            }])
            .unwrap();
        assert_eq!(output(&instance), "[2; 3]");
    }
}
