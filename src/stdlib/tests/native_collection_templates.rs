#![cfg(all(feature = "full_compiler", feature = "native-artifact"))]
use mech_core::{
    BoundResidentKernel, FunctionCatalog, FunctionCatalogBuilder, NodeId, ResidentKernelBindError,
    ResidentKernelBindRequest,
};
use mech_engine::native_artifact::{
    NativeArtifactLoweringError, NativeCollectionLexicalSource, NativeCollectionPrimitive,
    native_collection_lexical_template, native_collection_operation_template,
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
