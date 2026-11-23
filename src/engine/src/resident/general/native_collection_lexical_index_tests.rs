use super::*;
use crate::{
    CollectionPattern, ComprehensionDeclaration, ComprehensionKind, ComprehensionOperation,
    ComprehensionStep, ComprehensionValue, ControlBlock, ControlBlockId, ControlMatchArm,
    ControlOperation, ControlOperationBody, ControlParameter, ControlParameterSource, ControlValue,
    MatchDeclaration, MatchPattern,
};

fn empty_artifact() -> ProgramArtifact {
    let schemas = mech_core::SchemaTableBuilder::new()
        .finish()
        .unwrap()
        .into_parts()
        .0;
    let constants = mech_core::ConstantStoreBuilder::new(&schemas)
        .finish()
        .unwrap()
        .into_parts()
        .0;
    crate::ProgramArtifactDraft {
        schemas,
        constants,
        contracts: mech_core::OperationContractTable::empty(),
        requirements: crate::ApplicationRequirementTable::empty(),
        inputs: Box::new([]),
        slots: Box::new([]),
        nodes: Box::new([]),
        bindings: Box::new([]),
        outputs: Box::new([]),
        constraints: Box::new([]),
        compute_regions: Box::new([]),
    }
    .finalize()
    .unwrap()
}

fn pure() -> ControlOperationBody {
    ControlOperationBody::Operation {
        operation: OperationReference {
            module_path: Box::new(["test".into()]),
            operation_name: "identity".into(),
        },
        contract: OperationContractId::new(0),
    }
}
fn parameter(block: u32, ordinal: u16) -> ControlValue {
    ControlValue::Parameter {
        block: ControlBlockId(block),
        ordinal,
    }
}
fn block(id: u32, operations: Vec<ControlOperation>) -> ControlBlock {
    ControlBlock {
        id: ControlBlockId(id),
        parameters: vec![
            ControlParameter {
                source: ControlParameterSource::Capture(0),
                schema: SchemaId::new(11),
            },
            ControlParameter {
                source: ControlParameterSource::Capture(1),
                schema: SchemaId::new(12),
            },
        ]
        .into(),
        operations: operations.into(),
        yield_value: parameter(id, 0),
    }
}
fn op(node: u32, body: ControlOperationBody, input: ControlValue) -> ControlOperation {
    ControlOperation {
        node,
        body,
        inputs: Box::new([input]),
        schema: SchemaId::new(20 + node),
    }
}
fn child(id: u32, malformed: bool) -> ControlBlock {
    block(
        id,
        vec![
            op(0, pure(), parameter(id, 0)),
            op(
                1,
                pure(),
                ControlValue::Local {
                    block: ControlBlockId(id),
                    node: if malformed { 1 } else { 0 },
                },
            ),
        ],
    )
}
fn comprehension(id: u32, duplicate: bool) -> ComprehensionDeclaration {
    ComprehensionDeclaration {
        id: ControlBlockId(id),
        kind: ComprehensionKind::Matrix,
        steps: vec![
            ComprehensionStep::Generator {
                source: ComprehensionValue::Input(0),
                pattern: CollectionPattern::Bind {
                    local: 0,
                    schema: SchemaId::new(30),
                },
            },
            ComprehensionStep::Operation(ComprehensionOperation {
                local: if duplicate { 0 } else { 1 },
                body: pure(),
                inputs: Box::new([ComprehensionValue::Local(0)]),
                schema: SchemaId::new(31),
            }),
            ComprehensionStep::Filter(ComprehensionValue::Local(u32::MAX)),
            ComprehensionStep::Operation(ComprehensionOperation {
                local: 2,
                body: ControlOperationBody::Recur(0),
                inputs: Box::new([ComprehensionValue::Input(0)]),
                schema: SchemaId::new(32),
            }),
        ]
        .into(),
        yield_value: ComprehensionValue::Local(2),
    }
}
fn indexed<'a>(
    artifact: &ProgramArtifact,
    owner: NodeId,
    pending: Vec<LexicalScope<'a>>,
) -> NativeCollectionLexicalIndex {
    let mut index = NativeCollectionLexicalIndex::default();
    index.terminal_error = index.collect(artifact, owner, pending).err();
    index
}

#[test]
fn earlier_site_keeps_original_result_before_later_metadata_failure() {
    let artifact = empty_artifact();
    let owner = NodeId::new(4);
    let declaration = block(
        1,
        vec![
            op(0, pure(), parameter(1, 0)),
            op(
                1,
                pure(),
                ControlValue::Local {
                    block: ControlBlockId(1),
                    node: 1,
                },
            ),
        ],
    );
    let index = indexed(&artifact, owner, vec![LexicalScope::Block(&declaration)]);
    let first = index.site(owner, ControlBlockId(1), 0).unwrap();
    assert_eq!(&*first.inputs, &[SchemaId::new(11)]);
    assert!(
        matches!(index.site(owner, ControlBlockId(1), 1), Err(Error::InvalidBinding { node: Some(n) }) if n == owner)
    );
    assert!(index.site(owner, ControlBlockId(99), 0).is_err());
}

#[test]
fn current_comprehension_site_precedes_duplicate_local_error() {
    let artifact = empty_artifact();
    let owner = NodeId::new(4);
    let declaration = comprehension(2, true);
    let index = indexed(
        &artifact,
        owner,
        vec![LexicalScope::Comprehension(
            &declaration,
            vec![SchemaId::new(40)],
        )],
    );
    assert_eq!(
        &*index.site(owner, ControlBlockId(2), 0).unwrap().inputs,
        &[SchemaId::new(30)]
    );
    assert!(index.site(owner, ControlBlockId(2), 2).is_err());
}

#[test]
fn first_duplicate_site_and_non_library_error_are_retained() {
    let artifact = empty_artifact();
    let owner = NodeId::new(4);
    let first = block(
        1,
        vec![op(0, ControlOperationBody::Recur(0), parameter(1, 0))],
    );
    let later = child(1, false);
    let index = indexed(
        &artifact,
        owner,
        vec![LexicalScope::Block(&later), LexicalScope::Block(&first)],
    );
    assert_eq!(
        index.site(owner, ControlBlockId(1), 0),
        Err(unsupported(
            owner,
            "lexical collection site is not a pure library operation"
        ))
    );
}

#[test]
fn index_matches_original_ordered_walker_for_mixed_and_malformed_scopes() {
    let artifact = empty_artifact();
    let owner = NodeId::new(4);
    let mut comparisons = 0;
    let mut successful = 0;
    for seed in 0u32..4096 {
        let mut operations = Vec::new();
        for node in 0u32..3 {
            let input = match (seed >> (node * 2)) & 3 {
                0 => parameter(1, 0),
                1 => parameter(1, 3),
                2 => ControlValue::Local {
                    block: ControlBlockId(1),
                    node: node.saturating_sub(1),
                },
                _ => ControlValue::Local {
                    block: ControlBlockId(9),
                    node: 0,
                },
            };
            let body = if seed & (1 << (node + 8)) != 0 {
                pure()
            } else {
                match node {
                    0 => ControlOperationBody::Iteration(crate::IterationDeclaration {
                        count: crate::IterationCount::Literal(0),
                        seed: 0,
                        captures: Box::new([]),
                        carrier: SchemaId::new(11),
                        body: child(if seed & 128 == 0 { 2 } else { 1 }, seed & 64 != 0),
                    }),
                    1 => ControlOperationBody::Match(MatchDeclaration {
                        scrutinee: 0,
                        partial: true,
                        captures: Box::new([]),
                        arms: vec![ControlMatchArm {
                            pattern: MatchPattern::Wildcard,
                            guard: Some(child(3, seed & 64 != 0)),
                            body: child(4, false),
                        }]
                        .into(),
                    }),
                    _ => ControlOperationBody::Comprehension(comprehension(5, seed & 128 != 0)),
                }
            };
            operations.push(op(node, body, input));
        }
        let declaration = block(1, operations);
        let separate = comprehension(6, seed & 2048 != 0);
        let pending = || {
            vec![
                LexicalScope::Comprehension(&separate, vec![SchemaId::new(50)]),
                LexicalScope::Block(&declaration),
            ]
        };
        let index = indexed(&artifact, owner, pending());
        for block in [1, 2, 3, 4, 5, 6, 99] {
            for local in 0..4 {
                let actual = index.site(owner, ControlBlockId(block), local).cloned();
                let expected =
                    reference_lookup(&artifact, owner, ControlBlockId(block), local, pending());
                assert_eq!(
                    actual, expected,
                    "seed={seed}, block={block}, local={local}"
                );
                successful += usize::from(actual.is_ok());
                comparisons += 1;
            }
        }
    }
    assert_eq!(comparisons, 114688);
    assert!(successful > 1000);
}

#[cfg(all(feature = "source", feature = "artifact-codec"))]
#[test]
fn cache_follows_immutable_artifact_clones_and_is_absent_from_bytecode() {
    use mech_syntax::document::{
        AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
        parse_canonical_document,
    };
    let parsed = parse_canonical_document(
        TextSnapshot::new(
            DocumentId(19),
            Revision(1),
            "pick(x<u64>) => <u64>\n  | x => x.\npick(seed<u64>)\n",
        )
        .unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean());
    let catalog = Arc::new(mech_core::FunctionCatalogBuilder::new().build().unwrap());
    let artifact = crate::CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            catalog.clone(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap();
    let before = crate::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    assert!(artifact.collection_lexical_cache().owners.get().is_none());
    for node in artifact.nodes() {
        let _ = native_collection_lexical_template(
            &artifact,
            node.node,
            ControlBlockId(u32::MAX),
            0,
            &catalog,
        );
    }
    let owners = artifact.collection_lexical_cache().owners.get().unwrap();
    assert!(owners.iter().any(|owner| owner.get().is_some()));
    let cloned = artifact.clone();
    assert!(Arc::ptr_eq(
        owners,
        cloned.collection_lexical_cache().owners.get().unwrap()
    ));
    assert_eq!(
        before,
        crate::encode_program_artifact_bytecode_v1(&artifact).unwrap()
    );
    let decoded = crate::decode_program_artifact_bytecode_v1(&before).unwrap();
    assert!(decoded.collection_lexical_cache().owners.get().is_none());
    assert_eq!(
        before,
        crate::encode_program_artifact_bytecode_v1(&decoded).unwrap()
    );
}

// Original ordered query walker, retained as an independent differential oracle.
// Only the final factory selection is replaced by an owned descriptive record.
fn reference_site(
    _artifact: &ProgramArtifact,
    owner: NodeId,
    _block: crate::ControlBlockId,
    _local: u32,
    sources: Box<[NativeCollectionLexicalSource]>,
    input_ids: &[SchemaId],
    output_id: SchemaId,
    body: &crate::ControlOperationBody,
    _catalog: (),
) -> Result<Site, Error> {
    let crate::ControlOperationBody::Operation {
        operation,
        contract,
    } = body
    else {
        return Err(unsupported(
            owner,
            "lexical collection site is not a pure library operation",
        ));
    };
    Ok(Site {
        operation: operation.clone(),
        contract: *contract,
        inputs: input_ids.into(),
        output: output_id,
        sources,
    })
}
fn reference_lookup<'a>(
    artifact: &ProgramArtifact,
    owner: NodeId,
    block_id: crate::ControlBlockId,
    local: u32,
    mut pending: Vec<LexicalScope<'a>>,
) -> Result<Site, Error> {
    let fail = || Error::InvalidBinding { node: Some(owner) };
    let catalog = ();
    while let Some(scope) = pending.pop() {
        match scope {
            LexicalScope::Block(block) => {
                for operation in &block.operations {
                    let inputs = operation
                        .inputs
                        .iter()
                        .map(|value| {
                            control_source_schema(artifact, owner, block, operation.node, *value)
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    if block.id == block_id && operation.node == local {
                        return reference_site(
                            artifact,
                            owner,
                            block_id,
                            local,
                            operation
                                .inputs
                                .iter()
                                .copied()
                                .map(NativeCollectionLexicalSource::Control)
                                .collect(),
                            &inputs,
                            operation.schema,
                            &operation.body,
                            catalog,
                        );
                    }
                    push_children(&operation.body, inputs, &mut pending);
                }
            }
            LexicalScope::Comprehension(declaration, inputs) => {
                let mut locals = std::collections::BTreeMap::new();
                for step in &declaration.steps {
                    match step {
                        crate::ComprehensionStep::Generator { pattern, .. } => {
                            bind_pattern_schemas(owner, pattern, &mut locals)?
                        }
                        crate::ComprehensionStep::Operation(operation) => {
                            let actual_inputs = operation
                                .inputs
                                .iter()
                                .map(|value| {
                                    comprehension_source_schema(
                                        artifact, owner, &inputs, &locals, *value,
                                    )
                                })
                                .collect::<Result<Vec<_>, _>>()?;
                            if declaration.id == block_id && operation.local == local {
                                return reference_site(
                                    artifact,
                                    owner,
                                    block_id,
                                    local,
                                    operation
                                        .inputs
                                        .iter()
                                        .copied()
                                        .map(NativeCollectionLexicalSource::Comprehension)
                                        .collect(),
                                    &actual_inputs,
                                    operation.schema,
                                    &operation.body,
                                    catalog,
                                );
                            }
                            if locals.insert(operation.local, operation.schema).is_some() {
                                return Err(fail());
                            }
                            push_children(&operation.body, actual_inputs, &mut pending);
                        }
                        crate::ComprehensionStep::Filter(_) => {}
                    }
                }
            }
        }
    }
    Err(fail())
}
