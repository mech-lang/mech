//! Selected collection-leaf metadata, never an executor or a complete program.
//! No live shape, binder, kernel, source parser or preflight is invoked here.
use super::{node_inputs, node_output_slot};
use crate::native_artifact::NativeArtifactLoweringError as Error;
pub use crate::resident::native_collection_semantics::NativeCollectionPrimitive;
use crate::{ArtifactSource, ExecutableNodeBody, OperationReference, ProgramArtifact};
use mech_core::{
    CellSlotId, FunctionCatalog, ImplementationMemoryClass, NodeId, OperationContractId,
    ProgramRevision, ResidentOperationKey, ResolvedOperationContract, Schema, SchemaBody, SchemaId,
};

/// This read-only record certifies the exact selected library implementation
/// and its symbolic canonical schemas. It is not permission to execute a call:
/// native consumers must still derive current NativeHost descriptors, validate
/// geometry, construct the exact BoundCall and admit storage/work before use.
#[derive(Clone, Debug)]
pub struct NativeCollectionOperationTemplate {
    revision: ProgramRevision,
    node: NodeId,
    target: CellSlotId,
    sources: Box<[ArtifactSource]>,
    leaf: SelectedCollectionLeaf,
}

#[derive(Clone, Debug)]
struct SelectedCollectionLeaf {
    selected_key: ResidentOperationKey,
    primitive: NativeCollectionPrimitive,
    inputs: Box<[Schema]>,
    output: Schema,
    contract: ResolvedOperationContract,
    implementation_memory: ImplementationMemoryClass,
}
impl NativeCollectionOperationTemplate {
    pub const fn program_revision(&self) -> ProgramRevision {
        self.revision
    }
    pub const fn node(&self) -> NodeId {
        self.node
    }
    pub const fn target(&self) -> CellSlotId {
        self.target
    }
    pub fn selected_key(&self) -> &ResidentOperationKey {
        &self.leaf.selected_key
    }
    pub const fn primitive(&self) -> NativeCollectionPrimitive {
        self.leaf.primitive
    }
    pub fn sources(&self) -> &[ArtifactSource] {
        &self.sources
    }
    pub fn inputs(&self) -> &[Schema] {
        &self.leaf.inputs
    }
    pub fn output(&self) -> &Schema {
        &self.leaf.output
    }
    pub fn contract(&self) -> &ResolvedOperationContract {
        &self.leaf.contract
    }
    /// Selected source-implementation metadata, not a native cost allowance.
    pub const fn implementation_memory(&self) -> ImplementationMemoryClass {
        self.leaf.implementation_memory
    }
}

/// Exact lexical operand identity, not a reconstructed root slot or a value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeCollectionLexicalSource {
    Control(crate::ControlValue),
    Comprehension(crate::ComprehensionValue),
}

/// A selected leaf inside the actual canonical Match/Activation, iteration or
/// comprehension tree. Its constructor searches that tree; callers cannot
/// substitute a fabricated block with a coincidentally equal block number.
#[derive(Clone, Debug)]
pub struct NativeCollectionLexicalTemplate {
    revision: ProgramRevision,
    owner: NodeId,
    block: crate::ControlBlockId,
    local: u32,
    sources: Box<[NativeCollectionLexicalSource]>,
    leaf: SelectedCollectionLeaf,
}
impl NativeCollectionLexicalTemplate {
    pub const fn program_revision(&self) -> ProgramRevision {
        self.revision
    }
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn block(&self) -> crate::ControlBlockId {
        self.block
    }
    pub const fn local(&self) -> u32 {
        self.local
    }
    pub fn sources(&self) -> &[NativeCollectionLexicalSource] {
        &self.sources
    }
    pub fn selected_key(&self) -> &ResidentOperationKey {
        &self.leaf.selected_key
    }
    pub const fn primitive(&self) -> NativeCollectionPrimitive {
        self.leaf.primitive
    }
    pub fn inputs(&self) -> &[Schema] {
        &self.leaf.inputs
    }
    pub fn output(&self) -> &Schema {
        &self.leaf.output
    }
    pub fn contract(&self) -> &ResolvedOperationContract {
        &self.leaf.contract
    }
    pub const fn implementation_memory(&self) -> ImplementationMemoryClass {
        self.leaf.implementation_memory
    }
}

impl NativeCollectionPrimitive {
    pub const fn name(self) -> &'static str {
        match self {
            #[cfg(feature = "pure_text")]
            Self::TextLines => "text/lines",
            #[cfg(feature = "pure_text")]
            Self::TextJoin => "text/join",
            #[cfg(feature = "pure_text")]
            Self::SequenceLength => "sequence/length",
            #[cfg(feature = "pure_text")]
            Self::NumberFormat => "number/format",
            #[cfg(feature = "map_ops")]
            Self::StringHistogram => "map/histogram",
            #[cfg(feature = "map_ops")]
            Self::StringMapKeys => "map/keys",
            #[cfg(feature = "map_ops")]
            Self::StringMapValues => "map/values",
        }
    }
    pub const fn arity(self) -> usize {
        match self {
            #[cfg(feature = "pure_text")]
            Self::TextJoin | Self::NumberFormat => 2,
            _ => 1,
        }
    }
}

fn unsupported(node: NodeId, feature: &'static str) -> Error {
    Error::UnsupportedNode { node, feature }
}
fn schema(artifact: &ProgramArtifact, id: SchemaId) -> Result<Schema, Error> {
    artifact
        .schemas()
        .get(id)
        .cloned()
        .ok_or(Error::UnsupportedSchema { schema: id })
}
fn source_schema_id(artifact: &ProgramArtifact, source: ArtifactSource) -> Result<SchemaId, Error> {
    match source {
        ArtifactSource::Slot(slot) => artifact
            .slots()
            .get(slot.get() as usize)
            .map(|slot| slot.schema),
        ArtifactSource::Constant(constant) => artifact
            .constants()
            .get(constant)
            .map(|value| value.schema()),
    }
    .ok_or(Error::InvalidBinding { node: None })
}
fn is_matrix(body: &SchemaBody) -> bool {
    matches!(body, SchemaBody::Matrix { dimensions, .. } if dimensions.len() == 2)
}
fn is_string_matrix(body: &SchemaBody, row: bool) -> bool {
    matches!(body, SchemaBody::Matrix { element, dimensions }
        if element.as_ref() == &SchemaBody::String && dimensions.len() == 2
        && (!row || dimensions[0] == mech_core::DimensionExpr::Constant(1)))
}
fn is_u64(body: &SchemaBody) -> bool {
    body == &SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64)
}
#[cfg(feature = "map_ops")]
fn is_count_map(body: &SchemaBody) -> bool {
    crate::intrinsics::map_ops::is_count_map(body)
}

fn validate_schemas(
    primitive: NativeCollectionPrimitive,
    inputs: &[Schema],
    output: &Schema,
) -> bool {
    if inputs.len() != primitive.arity() {
        return false;
    }
    let first = inputs[0].body();
    let output = output.body();
    match primitive {
        #[cfg(feature = "pure_text")]
        NativeCollectionPrimitive::TextLines => {
            first == &SchemaBody::String && is_string_matrix(output, true)
        }
        #[cfg(feature = "pure_text")]
        NativeCollectionPrimitive::TextJoin => {
            is_string_matrix(first, false)
                && inputs[1].body() == &SchemaBody::String
                && output == &SchemaBody::String
        }
        #[cfg(feature = "pure_text")]
        NativeCollectionPrimitive::SequenceLength => {
            (first == &SchemaBody::String || is_matrix(first)) && is_u64(output)
        }
        #[cfg(feature = "pure_text")]
        NativeCollectionPrimitive::NumberFormat => {
            crate::intrinsics::pure_text::format_schema_supported(first)
                && is_u64(inputs[1].body())
                && if is_matrix(first) {
                    is_string_matrix(output, false)
                } else {
                    output == &SchemaBody::String
                }
        }
        #[cfg(feature = "map_ops")]
        NativeCollectionPrimitive::StringHistogram => {
            is_string_matrix(first, false) && is_count_map(output)
        }
        #[cfg(feature = "map_ops")]
        NativeCollectionPrimitive::StringMapKeys => {
            is_count_map(first) && is_string_matrix(output, true)
        }
        #[cfg(feature = "map_ops")]
        NativeCollectionPrimitive::StringMapValues => {
            is_count_map(first)
                && matches!(output, SchemaBody::Matrix { element, dimensions }
                if is_u64(element) && dimensions.len() == 2 && dimensions[0] == mech_core::DimensionExpr::Constant(1))
        }
    }
}

fn validate_contract(
    node: NodeId,
    primitive: NativeCollectionPrimitive,
    input_ids: &[SchemaId],
    output_id: SchemaId,
    matrix_output: bool,
    actual: &ResolvedOperationContract,
) -> Result<(), Error> {
    let fail = || {
        unsupported(
            node,
            "collection leaf requires its exact maintained contract",
        )
    };
    let ResolvedOperationContract::Declared(actual) = actual else {
        return Err(fail());
    };
    let expected = mech_core::maintained_operation_contract(
        primitive.name(),
        primitive.arity(),
        matrix_output,
    )
    .ok_or_else(fail)?;
    let inputs = expected
        .inputs
        .resolve(primitive.arity())
        .map_err(|_| fail())?;
    let [output] = actual.outputs.as_ref() else {
        return Err(fail());
    };
    let [expected_output] = expected.outputs.as_ref() else {
        return Err(fail());
    };
    if input_ids.len() != primitive.arity()
        || actual.inputs.len() != inputs.len()
        || actual.interaction != expected.interaction
        || !actual.inputs.iter().zip(inputs.iter()).zip(input_ids).all(
            |((actual, expected), id)| {
                actual.schema == *id
                    && actual.access == expected.access
                    && actual.delivery == expected.delivery
            },
        )
        || output.schema != output_id
        || output.access != expected_output.access
        || output.delivery != expected_output.delivery
        || output.construction != expected_output.construction
        || output.alias != expected_output.alias
        || output.change_detection != expected_output.change_detection
    {
        return Err(fail());
    }
    Ok(())
}

fn select_leaf(
    artifact: &ProgramArtifact,
    node: NodeId,
    operation: &OperationReference,
    contract_id: OperationContractId,
    input_ids: &[SchemaId],
    output_id: SchemaId,
    catalog: &FunctionCatalog,
) -> Result<SelectedCollectionLeaf, Error> {
    let entry = catalog
        .resident_factory(&operation.module_path, &operation.operation_name)
        .ok_or(Error::MissingSemanticWitness { node })?;
    let primitive = crate::resident::native_collection_semantics::primitive(entry)
        .ok_or(Error::MissingSemanticWitness { node })?;
    let inputs = input_ids
        .iter()
        .map(|id| schema(artifact, *id))
        .collect::<Result<Box<[_]>, _>>()?;
    let output = schema(artifact, output_id)?;
    if !validate_schemas(primitive, &inputs, &output) {
        return Err(Error::WitnessTypeMismatch { node });
    }
    let contract = artifact
        .contracts()
        .get(contract_id)
        .ok_or(Error::InvalidBinding { node: Some(node) })?;
    validate_contract(
        node,
        primitive,
        input_ids,
        output_id,
        is_matrix(output.body()),
        contract,
    )?;
    Ok(SelectedCollectionLeaf {
        selected_key: entry.key().clone(),
        primitive,
        inputs,
        output,
        contract: contract.clone(),
        implementation_memory: entry.implementation_memory(),
    })
}

/// Inspect one actual root operation. This does not claim any other program
/// node is supported, scheduled or executable by a native backend.
pub fn native_collection_operation_template(
    artifact: &ProgramArtifact,
    node: NodeId,
    catalog: &FunctionCatalog,
) -> Result<NativeCollectionOperationTemplate, Error> {
    let declaration = artifact
        .nodes()
        .get(node.get() as usize)
        .filter(|declaration| declaration.node == node)
        .ok_or(Error::InvalidBinding { node: Some(node) })?;
    let ExecutableNodeBody::Operation(operation) = &declaration.body else {
        return Err(unsupported(
            node,
            "collection template requires an operation, not control",
        ));
    };
    if operation.requirement.is_some() {
        return Err(unsupported(
            node,
            "resource operation is not a pure collection leaf",
        ));
    }
    let sources = node_inputs(artifact, node)?;
    let input_ids = sources
        .iter()
        .map(|source| source_schema_id(artifact, *source))
        .collect::<Result<Vec<_>, _>>()?;
    let target = node_output_slot(artifact, node)?;
    let output_id = artifact
        .slots()
        .get(target.get() as usize)
        .ok_or(Error::InvalidBinding { node: Some(node) })?
        .schema;
    let leaf = select_leaf(
        artifact,
        node,
        &operation.operation,
        operation.contract,
        &input_ids,
        output_id,
        catalog,
    )?;
    Ok(NativeCollectionOperationTemplate {
        revision: artifact.revision(),
        node,
        target,
        sources: sources.into_boxed_slice(),
        leaf,
    })
}

enum LexicalScope<'a> {
    Block(&'a crate::ControlBlock),
    Comprehension(&'a crate::ComprehensionDeclaration, Vec<SchemaId>),
}

fn push_match<'a>(declaration: &'a crate::MatchDeclaration, pending: &mut Vec<LexicalScope<'a>>) {
    for arm in declaration.arms.iter().rev() {
        pending.push(LexicalScope::Block(&arm.body));
        if let Some(guard) = &arm.guard {
            pending.push(LexicalScope::Block(guard));
        }
    }
}
fn push_children<'a>(
    body: &'a crate::ControlOperationBody,
    inputs: Vec<SchemaId>,
    pending: &mut Vec<LexicalScope<'a>>,
) {
    match body {
        crate::ControlOperationBody::Match(declaration) => push_match(declaration, pending),
        crate::ControlOperationBody::Iteration(declaration) => {
            pending.push(LexicalScope::Block(&declaration.body))
        }
        crate::ControlOperationBody::Comprehension(declaration) => {
            pending.push(LexicalScope::Comprehension(declaration, inputs))
        }
        // These are execution boundaries, not new lexical declarations. In
        // particular, metadata traversal must never follow a Recur back-edge.
        crate::ControlOperationBody::Operation { .. }
        | crate::ControlOperationBody::Recur(_)
        | crate::ControlOperationBody::Suspend
        | crate::ControlOperationBody::Publish => {}
    }
}

fn control_source_schema(
    artifact: &ProgramArtifact,
    owner: NodeId,
    block: &crate::ControlBlock,
    before: u32,
    value: crate::ControlValue,
) -> Result<SchemaId, Error> {
    let fail = || Error::InvalidBinding { node: Some(owner) };
    match value {
        crate::ControlValue::Constant(id) => {
            source_schema_id(artifact, ArtifactSource::Constant(id))
        }
        crate::ControlValue::Parameter {
            block: actual,
            ordinal,
        } if actual == block.id => block
            .parameters
            .get(usize::from(ordinal))
            .map(|parameter| parameter.schema)
            .ok_or_else(fail),
        crate::ControlValue::Local {
            block: actual,
            node,
        } if actual == block.id && node < before => block
            .operations
            .get(node as usize)
            .filter(|operation| operation.node == node)
            .map(|operation| operation.schema)
            .ok_or_else(fail),
        _ => Err(fail()),
    }
}

fn comprehension_source_schema(
    artifact: &ProgramArtifact,
    owner: NodeId,
    inputs: &[SchemaId],
    locals: &std::collections::BTreeMap<u32, SchemaId>,
    value: crate::ComprehensionValue,
) -> Result<SchemaId, Error> {
    let fail = || Error::InvalidBinding { node: Some(owner) };
    match value {
        crate::ComprehensionValue::Constant(id) => {
            source_schema_id(artifact, ArtifactSource::Constant(id))
        }
        crate::ComprehensionValue::Input(ordinal) => {
            inputs.get(usize::from(ordinal)).copied().ok_or_else(fail)
        }
        crate::ComprehensionValue::Local(local) => locals.get(&local).copied().ok_or_else(fail),
    }
}

fn bind_pattern_schemas(
    owner: NodeId,
    pattern: &crate::CollectionPattern,
    locals: &mut std::collections::BTreeMap<u32, SchemaId>,
) -> Result<(), Error> {
    let mut pending = vec![pattern];
    while let Some(pattern) = pending.pop() {
        match pattern {
            crate::CollectionPattern::Bind { local, schema } => {
                if locals.insert(*local, *schema).is_some() {
                    return Err(Error::InvalidBinding { node: Some(owner) });
                }
            }
            crate::CollectionPattern::Enum { payload, .. } => {
                if let Some(payload) = payload {
                    pending.push(payload);
                }
            }
            crate::CollectionPattern::Tuple(items) => pending.extend(items.iter()),
            crate::CollectionPattern::Array {
                prefix,
                rest,
                suffix,
            } => {
                pending.extend(prefix.iter());
                if let Some(rest) = rest {
                    pending.push(rest);
                }
                pending.extend(suffix.iter());
            }
            crate::CollectionPattern::Wildcard | crate::CollectionPattern::Equal(_) => {}
        }
    }
    Ok(())
}

fn lexical_template(
    artifact: &ProgramArtifact,
    owner: NodeId,
    block: crate::ControlBlockId,
    local: u32,
    sources: Box<[NativeCollectionLexicalSource]>,
    input_ids: &[SchemaId],
    output_id: SchemaId,
    body: &crate::ControlOperationBody,
    catalog: &FunctionCatalog,
) -> Result<NativeCollectionLexicalTemplate, Error> {
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
    let leaf = select_leaf(
        artifact, owner, operation, *contract, input_ids, output_id, catalog,
    )?;
    Ok(NativeCollectionLexicalTemplate {
        revision: artifact.revision(),
        owner,
        block,
        local,
        sources,
        leaf,
    })
}

/// Select a leaf at its genuine lexical site. This statically walks the
/// immutable declaration tree; it never executes a guard, generator, pattern,
/// loop, recursive call, binder or kernel. Full native CFG/current-visit and
/// NativeHost storage/work proof remain mandatory at later compiler stages.
pub fn native_collection_lexical_template(
    artifact: &ProgramArtifact,
    owner: NodeId,
    block_id: crate::ControlBlockId,
    local: u32,
    catalog: &FunctionCatalog,
) -> Result<NativeCollectionLexicalTemplate, Error> {
    let fail = || Error::InvalidBinding { node: Some(owner) };
    let root = artifact
        .nodes()
        .get(owner.get() as usize)
        .filter(|root| root.node == owner)
        .ok_or_else(fail)?;
    let mut pending = Vec::new();
    match &root.body {
        ExecutableNodeBody::Match(declaration) | ExecutableNodeBody::Activation(declaration) => {
            push_match(declaration, &mut pending)
        }
        ExecutableNodeBody::Iteration(declaration) => {
            pending.push(LexicalScope::Block(&declaration.body))
        }
        ExecutableNodeBody::Comprehension(declaration) => {
            let inputs = node_inputs(artifact, owner)?
                .iter()
                .map(|source| source_schema_id(artifact, *source))
                .collect::<Result<Vec<_>, _>>()?;
            pending.push(LexicalScope::Comprehension(declaration, inputs));
        }
        _ => {
            return Err(unsupported(
                owner,
                "root has no lexical collection declaration tree",
            ));
        }
    }
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
                        return lexical_template(
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
                                return lexical_template(
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

#[cfg(all(test, feature = "pure_text"))]
mod tests {
    use super::*;
    use mech_core::{
        AccessMode, AliasPolicy, ChangeDetectionPolicy, DeclaredOperationContract, DeliveryMode,
        ExternalInteraction, OutputConstruction, ResolvedInputPort, ResolvedOutputPort, ShapeRule,
    };

    #[test]
    fn collection_templates_require_exact_ports_effects_shapes_alias_and_change_contracts() {
        let input = SchemaId::new(0);
        let output = SchemaId::new(1);
        let node = NodeId::new(7);
        let original = ResolvedOperationContract::Declared(DeclaredOperationContract {
            inputs: Box::new([ResolvedInputPort {
                schema: input,
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            }]),
            outputs: Box::new([ResolvedOutputPort {
                schema: output,
                access: AccessMode::Write,
                delivery: DeliveryMode::Signal,
                construction: OutputConstruction::FullWrite {
                    shape: ShapeRule::Declared,
                },
                alias: AliasPolicy::NoAlias,
                change_detection: ChangeDetectionPolicy::ExactScalar,
            }]),
            interaction: ExternalInteraction::Pure,
        });
        validate_contract(
            node,
            NativeCollectionPrimitive::SequenceLength,
            &[input],
            output,
            false,
            &original,
        )
        .unwrap();
        for mutation in 0..13 {
            let mut changed = original.clone();
            let ResolvedOperationContract::Declared(contract) = &mut changed else {
                unreachable!()
            };
            match mutation {
                0 => contract.inputs[0].schema = output,
                1 => contract.inputs[0].access = AccessMode::ReadWrite,
                2 => contract.outputs[0].schema = input,
                3 => contract.outputs[0].alias = AliasPolicy::MayAlias { input: 0 },
                4 => contract.outputs[0].change_detection = ChangeDetectionPolicy::KernelReported,
                5 => {
                    contract.outputs[0].construction = OutputConstruction::FullWrite {
                        shape: ShapeRule::SameAsInput { input: 0 },
                    }
                }
                6 => contract.inputs[0].delivery = DeliveryMode::Stream,
                7 => contract.outputs[0].delivery = DeliveryMode::Future,
                8 => {
                    contract.interaction =
                        ExternalInteraction::Observation(mech_core::ObservationContract {
                            replay: mech_core::ObservationReplayPolicy::CaptureAsInputFact,
                        })
                }
                9 => {
                    contract.interaction = ExternalInteraction::Effect(mech_core::EffectContract {
                        delivery: mech_core::EffectDeliveryPolicy::AtMostOnce,
                        idempotency: mech_core::IdempotencyRequirement::NotRequired,
                    })
                }
                10 => contract.outputs[0].access = AccessMode::ReadWrite,
                11 => {
                    contract.outputs =
                        Box::new([contract.outputs[0].clone(), contract.outputs[0].clone()])
                }
                12 => {
                    contract.interaction = ExternalInteraction::TransactionalExternal(
                        mech_core::TransactionalExternalContract {
                            protocol: mech_core::TransactionalEffectProtocol::PrepareCommit,
                        },
                    )
                }
                _ => unreachable!(),
            }
            assert!(
                validate_contract(
                    node,
                    NativeCollectionPrimitive::SequenceLength,
                    &[input],
                    output,
                    false,
                    &changed
                )
                .is_err()
            );
        }
        assert!(
            validate_contract(
                node,
                NativeCollectionPrimitive::SequenceLength,
                &[],
                output,
                false,
                &original
            )
            .is_err()
        );
    }

    #[test]
    fn collection_templates_check_schema_relations_without_live_values_or_layout_guesses() {
        let schema = |body| {
            mech_core::SchemaDraft {
                dimension_parameters: Box::new([]),
                body,
            }
            .finalize()
            .unwrap()
        };
        let u64_body = SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64);
        let text = schema(SchemaBody::String);
        let integer = schema(u64_body.clone());
        let boolean = schema(SchemaBody::Bool);
        let matrix = |element, rows| {
            schema(SchemaBody::Matrix {
                element: Box::new(element),
                dimensions: Box::new([
                    mech_core::DimensionExpr::Constant(rows),
                    mech_core::DimensionExpr::Constant(2),
                ]),
            })
        };
        let strings = matrix(SchemaBody::String, 1);
        let integers = matrix(u64_body.clone(), 1);
        let two_rows = matrix(SchemaBody::String, 2);
        use NativeCollectionPrimitive::*;
        assert!(validate_schemas(TextLines, &[text.clone()], &strings));
        assert!(!validate_schemas(TextLines, &[text.clone()], &two_rows));
        assert!(!validate_schemas(TextLines, &[integer.clone()], &strings));
        assert!(validate_schemas(
            TextJoin,
            &[strings.clone(), text.clone()],
            &text
        ));
        assert!(!validate_schemas(
            TextJoin,
            &[strings.clone(), integer.clone()],
            &text
        ));
        assert!(!validate_schemas(
            TextJoin,
            &[integers.clone(), text.clone()],
            &text
        ));
        assert!(validate_schemas(SequenceLength, &[text.clone()], &integer));
        assert!(validate_schemas(
            SequenceLength,
            &[integers.clone()],
            &integer
        ));
        assert!(!validate_schemas(
            SequenceLength,
            &[integer.clone()],
            &integer
        ));
        assert!(!validate_schemas(SequenceLength, &[text.clone()], &boolean));
        assert!(validate_schemas(
            NumberFormat,
            &[integer.clone(), integer.clone()],
            &text
        ));
        assert!(validate_schemas(
            NumberFormat,
            &[integers.clone(), integer.clone()],
            &strings
        ));
        assert!(!validate_schemas(
            NumberFormat,
            &[integers.clone(), integer.clone()],
            &text
        ));
        assert!(!validate_schemas(
            NumberFormat,
            &[integer.clone(), boolean],
            &text
        ));
        assert!(!validate_schemas(
            NumberFormat,
            &[text.clone(), integer],
            &text
        ));
        #[cfg(feature = "map_ops")]
        {
            let count_map = schema(SchemaBody::Map {
                key: Box::new(SchemaBody::String),
                value: Box::new(u64_body),
                cardinality: mech_core::CardinalitySpec::Dynamic { upper_bound: None },
            });
            assert!(validate_schemas(
                StringHistogram,
                &[strings.clone()],
                &count_map
            ));
            assert!(!validate_schemas(
                StringHistogram,
                &[integers.clone()],
                &count_map
            ));
            assert!(validate_schemas(
                StringMapKeys,
                &[count_map.clone()],
                &strings
            ));
            assert!(!validate_schemas(
                StringMapKeys,
                &[count_map.clone()],
                &integers
            ));
            assert!(!validate_schemas(
                StringMapKeys,
                &[count_map.clone()],
                &two_rows
            ));
            assert!(validate_schemas(StringMapValues, &[count_map], &integers));
        }
    }
}
