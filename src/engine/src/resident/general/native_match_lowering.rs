//! Canonical scalar control -> immutable native CFG; metadata binding only.
//! Match and counted-loop views share selected witnesses, exact scalar types
//! and lexical block lowering. Neither path parses source or executes values.
use super::native_lowering::{
    port_layout, public_kind, public_op, scalar_type, validate_scalar_slot_initializers,
};
use super::{
    NodeClass, ResidentExternalAdmission, bind_operation_for_target, build_activation_schedule,
    classify_nodes, forwarded_output_source, node_inputs, node_output_slot,
};
use crate::native_artifact::{
    NativeArtifactLoweringError, NativeInput, NativeIterationAction, NativeIterationBlock,
    NativeIterationLoop, NativeIterationTerminator, NativeIterationValueScope, NativeMatchAction,
    NativeMatchBlock, NativeMatchConstant, NativeMatchInput, NativeMatchOrigin, NativeMatchOutput,
    NativeMatchTerminator, NativeMatchValue, NativeScalarKind, NativeScalarType, NativeScalarValue,
};
use crate::resident::native_semantics;
use crate::{
    ArtifactSource, ControlBlock, ControlOperationBody, ControlParameterSource, ControlValue,
    ExecutableNodeBody, MatchDeclaration, MatchPattern, ProgramArtifact,
};
use mech_core::{ConstantId, ExecutionTarget, FunctionCatalog, NodeId, ProgramRevision};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct NativeMatchArtifactPlan {
    revision: ProgramRevision,
    inputs: Box<[NativeMatchInput]>,
    constants: Box<[NativeMatchConstant]>,
    values: Box<[NativeScalarType]>,
    blocks: Box<[NativeMatchBlock]>,
    outputs: Box<[NativeMatchOutput]>,
    entry: u32,
}
impl NativeMatchArtifactPlan {
    pub const fn program_revision(&self) -> ProgramRevision {
        self.revision
    }
    pub fn inputs(&self) -> &[NativeMatchInput] {
        &self.inputs
    }
    pub fn constants(&self) -> &[NativeMatchConstant] {
        &self.constants
    }
    pub fn values(&self) -> &[NativeScalarType] {
        &self.values
    }
    pub fn blocks(&self) -> &[NativeMatchBlock] {
        &self.blocks
    }
    pub fn outputs(&self) -> &[NativeMatchOutput] {
        &self.outputs
    }
    pub const fn entry(&self) -> u32 {
        self.entry
    }
}

/// Private-constructor lowering of the ordinary canonical artifact into a
/// structured counted-loop CFG. This is not a second program/source format.
#[derive(Clone, Debug)]
pub struct NativeIterationArtifactPlan {
    revision: ProgramRevision,
    inputs: Box<[NativeMatchInput]>,
    constants: Box<[NativeMatchConstant]>,
    values: Box<[NativeScalarType]>,
    value_scopes: Box<[NativeIterationValueScope]>,
    blocks: Box<[NativeIterationBlock]>,
    iterations: Box<[NativeIterationLoop]>,
    outputs: Box<[NativeMatchOutput]>,
    entry: u32,
}
impl NativeIterationArtifactPlan {
    /// Rechecks bounded lexical ownership/current-visit definitions without
    /// executing an operation. The plan has no public constructor or mutator.
    pub fn validate_current_visits(&self) -> Result<(), &'static str> {
        crate::native_artifact::iteration_validation::validate(self)
    }
    pub const fn program_revision(&self) -> ProgramRevision {
        self.revision
    }
    pub fn inputs(&self) -> &[NativeMatchInput] {
        &self.inputs
    }
    pub fn constants(&self) -> &[NativeMatchConstant] {
        &self.constants
    }
    pub fn values(&self) -> &[NativeScalarType] {
        &self.values
    }
    pub fn value_scopes(&self) -> &[NativeIterationValueScope] {
        &self.value_scopes
    }
    pub fn blocks(&self) -> &[NativeIterationBlock] {
        &self.blocks
    }
    pub fn iterations(&self) -> &[NativeIterationLoop] {
        &self.iterations
    }
    pub fn outputs(&self) -> &[NativeMatchOutput] {
        &self.outputs
    }
    pub const fn entry(&self) -> u32 {
        self.entry
    }
}

type Error = NativeArtifactLoweringError;
fn unsupported(node: NodeId, feature: &'static str) -> Error {
    Error::UnsupportedNode { node, feature }
}
fn origin(node: NodeId, block: Option<u32>, local: Option<u32>) -> NativeMatchOrigin {
    NativeMatchOrigin { node, block, local }
}

struct Builder<'a> {
    artifact: &'a ProgramArtifact,
    catalog: &'a FunctionCatalog,
    values: Vec<NativeScalarType>,
    constants: BTreeMap<ConstantId, NativeMatchValue>,
    blocks: Vec<NativeIterationBlock>,
    value_scopes: Vec<NativeIterationValueScope>,
    iterations: Vec<Option<NativeIterationLoop>>,
    current_iteration: Option<u32>,
    allow_iteration: bool,
}
impl Builder<'_> {
    fn value(&mut self, ty: NativeScalarType) -> Result<NativeMatchValue, Error> {
        // scalar_type validates exact Bool/U64/I64/F64 schema evidence. The
        // control transport, constants and literal emitter support all four;
        // primitives still require the actual selected private witness.
        let id =
            u32::try_from(self.values.len()).map_err(|_| Error::InvalidBinding { node: None })?;
        self.values.push(ty);
        self.value_scopes
            .push(NativeIterationValueScope::Immutable {
                iteration: self.current_iteration,
            });
        Ok(NativeMatchValue(id))
    }
    fn block(&mut self, at: NativeMatchOrigin, action: NativeMatchAction) -> Result<u32, Error> {
        let id = u32::try_from(self.blocks.len()).map_err(|_| Error::InvalidBinding {
            node: Some(at.node()),
        })?;
        self.blocks.push(NativeIterationBlock {
            origin: at,
            action: NativeIterationAction::Scalar(action),
            terminator: NativeMatchTerminator::Reject.into(),
            iteration: self.current_iteration,
        });
        Ok(id)
    }
    fn link(&mut self, from: u32, to: u32) {
        self.blocks[from as usize].terminator = NativeMatchTerminator::Jump(to).into();
    }
    fn ty(&self, value: NativeMatchValue) -> &NativeScalarType {
        &self.values[value.get() as usize]
    }
    fn primitive(
        &mut self,
        at: NativeMatchOrigin,
        reference: &crate::OperationReference,
        contract: mech_core::OperationContractId,
        arguments: &[NativeMatchValue],
        output: NativeMatchValue,
    ) -> Result<u32, Error> {
        let layouts = arguments
            .iter()
            .map(|&value| port_layout(self.ty(value)))
            .collect::<Vec<_>>();
        let (kernel, memory_plan) = bind_operation_for_target(
            self.artifact,
            self.catalog,
            at.node(),
            reference,
            contract,
            &layouts,
            port_layout(self.ty(output)),
            ExecutionTarget::Native,
        )?;
        let semantics = native_semantics::validated_scalar(&kernel)
            .ok_or(Error::MissingSemanticWitness { node: at.node() })?;
        if semantics.inputs().len() != arguments.len()
            || semantics
                .inputs()
                .iter()
                .zip(arguments)
                .any(|(&kind, &arg)| public_kind(kind) != self.ty(arg).kind())
            || public_kind(semantics.output()) != self.ty(output).kind()
        {
            return Err(Error::WitnessTypeMismatch { node: at.node() });
        }
        self.block(
            at,
            NativeMatchAction::Primitive {
                op: public_op(semantics.op()),
                operands: arguments.into(),
                output,
                memory_plan,
            },
        )
    }
    fn read(
        &self,
        value: ControlValue,
        block: &ControlBlock,
        parameters: &[NativeMatchValue],
        locals: &[NativeMatchValue],
        owner: NodeId,
    ) -> Result<NativeMatchValue, Error> {
        match value {
            ControlValue::Constant(id) => self.constants.get(&id).copied(),
            ControlValue::Parameter { block: id, ordinal } if id == block.id => {
                parameters.get(ordinal as usize).copied()
            }
            ControlValue::Local { block: id, node } if id == block.id => {
                locals.get(node as usize).copied()
            }
            _ => None,
        }
        .ok_or(Error::InvalidBinding { node: Some(owner) })
    }
    fn control_block(
        &mut self,
        owner: NodeId,
        declaration: &MatchDeclaration,
        block: &ControlBlock,
        arguments: &[NativeMatchValue],
        binding: Option<(u32, NativeMatchValue)>,
    ) -> Result<(u32, u32, NativeMatchValue), Error> {
        let scrutinee = *arguments
            .get(declaration.scrutinee as usize)
            .ok_or(Error::InvalidBinding { node: Some(owner) })?;
        let parameters = block
            .parameters
            .iter()
            .map(|parameter| {
                let value = match parameter.source {
                    ControlParameterSource::Scrutinee => Some(scrutinee),
                    ControlParameterSource::Capture(index) => declaration
                        .captures
                        .get(index as usize)
                        .and_then(|capture| arguments.get(capture.input as usize))
                        .copied(),
                    ControlParameterSource::PatternBinding(local) => binding
                        .filter(|(id, _)| *id == local)
                        .map(|(_, value)| value),
                    ControlParameterSource::IterationCarrier => {
                        return Err(unsupported(owner, "Iteration-carried control parameter"));
                    }
                }
                .ok_or(Error::InvalidBinding { node: Some(owner) })?;
                if self.ty(value).schema() != parameter.schema {
                    return Err(Error::InvalidBinding { node: Some(owner) });
                }
                Ok(value)
            })
            .collect::<Result<Vec<_>, Error>>()?;
        self.lower_block(owner, block, parameters)
    }
    fn lower_block(
        &mut self,
        owner: NodeId,
        block: &ControlBlock,
        parameters: Vec<NativeMatchValue>,
    ) -> Result<(u32, u32, NativeMatchValue), Error> {
        let at = origin(owner, Some(block.id.0), None);
        let entry = self.block(at, NativeMatchAction::None)?;
        let mut tail = entry;
        let mut locals = Vec::new();
        for operation in &block.operations {
            let at = origin(owner, Some(block.id.0), Some(operation.node));
            if operation.node as usize != locals.len() {
                return Err(Error::InvalidBinding { node: Some(owner) });
            }
            let args = operation
                .inputs
                .iter()
                .map(|&value| self.read(value, block, &parameters, &locals, owner))
                .collect::<Result<Vec<_>, _>>()?;
            let result = self.value(scalar_type(self.artifact, operation.schema)?)?;
            let (start, end) = match &operation.body {
                ControlOperationBody::Operation {
                    operation,
                    contract,
                } => {
                    let id = self.primitive(at, operation, *contract, &args, result)?;
                    (id, id)
                }
                ControlOperationBody::Match(nested) => self.matched(at, nested, &args, result)?,
                ControlOperationBody::Iteration(nested) if self.allow_iteration => {
                    self.iteration(at, nested, &args, result)?
                }
                _ => return Err(unsupported(owner, "non-Match scalar control operation")),
            };
            self.link(tail, start);
            tail = end;
            locals.push(result);
        }
        let yielded = self.read(block.yield_value, block, &parameters, &locals, owner)?;
        Ok((entry, tail, yielded))
    }
    fn iteration(
        &mut self,
        at: NativeMatchOrigin,
        declaration: &crate::IterationDeclaration,
        arguments: &[NativeMatchValue],
        output: NativeMatchValue,
    ) -> Result<(u32, u32), Error> {
        let owner = at.node();
        if !self.allow_iteration {
            return Err(unsupported(owner, "counted iteration in Match-only plan"));
        }
        let count = match declaration.count {
            crate::IterationCount::Literal(count) => count,
            crate::IterationCount::Elements { .. } => {
                return Err(unsupported(
                    owner,
                    "element-wise iteration requires native matrix transport",
                ));
            }
        };
        let seed = *arguments
            .get(declaration.seed as usize)
            .ok_or(Error::InvalidBinding { node: Some(owner) })?;
        let ty = scalar_type(self.artifact, declaration.carrier)?;
        if self.ty(seed) != &ty || self.ty(output) != &ty {
            return Err(Error::InvalidBinding { node: Some(owner) });
        }
        for capture in &declaration.captures {
            let value = arguments
                .get(capture.input as usize)
                .ok_or(Error::InvalidBinding { node: Some(owner) })?;
            if !capture.freeze_on_suspend || self.ty(*value).schema() != capture.schema {
                return Err(Error::InvalidBinding { node: Some(owner) });
            }
        }
        let id = u32::try_from(self.iterations.len())
            .map_err(|_| Error::InvalidBinding { node: Some(owner) })?;
        let parent = self.current_iteration;
        self.iterations.push(None);
        let initialize = self.block(at, NativeMatchAction::None)?;
        self.blocks[initialize as usize].action =
            NativeIterationAction::Initialize { iteration: id };
        self.current_iteration = Some(id);
        let body_result: Result<_, Error> = (|| {
            let carrier = self.value(ty)?;
            self.value_scopes[carrier.get() as usize] =
                NativeIterationValueScope::Carrier { iteration: id };
            let body_origin = origin(owner, Some(declaration.body.id.0), None);
            let header = self.block(body_origin, NativeMatchAction::None)?;
            let parameters = declaration
                .body
                .parameters
                .iter()
                .map(|parameter| {
                    let value = match parameter.source {
                        ControlParameterSource::IterationCarrier => Some(carrier),
                        ControlParameterSource::Capture(index) => declaration
                            .captures
                            .get(index as usize)
                            .and_then(|capture| arguments.get(capture.input as usize))
                            .copied(),
                        ControlParameterSource::Scrutinee
                        | ControlParameterSource::PatternBinding(_) => None,
                    }
                    .ok_or(Error::InvalidBinding { node: Some(owner) })?;
                    if self.ty(value).schema() != parameter.schema {
                        return Err(Error::InvalidBinding { node: Some(owner) });
                    }
                    Ok(value)
                })
                .collect::<Result<Vec<_>, Error>>()?;
            let (body, tail, next) = self.lower_block(owner, &declaration.body, parameters)?;
            if self.ty(next) != self.ty(carrier) {
                return Err(Error::InvalidBinding { node: Some(owner) });
            }
            let latch = self.block(body_origin, NativeMatchAction::None)?;
            self.blocks[latch as usize].action = NativeIterationAction::Latch { iteration: id };
            self.link(tail, latch);
            self.link(latch, header);
            Ok((carrier, header, body, next, latch))
        })();
        self.current_iteration = parent;
        let (carrier, header, body, next, latch) = body_result?;
        let exit = self.block(
            at,
            NativeMatchAction::Copy {
                source: carrier,
                output,
            },
        )?;
        self.blocks[header as usize].terminator = NativeIterationTerminator::Header {
            iteration: id,
            body,
            exit,
        };
        self.link(initialize, header);
        self.iterations[id as usize] = Some(NativeIterationLoop {
            origin: at,
            count,
            parent,
            seed,
            carrier,
            next,
            result: output,
            initialize,
            header,
            body,
            latch,
            exit,
        });
        Ok((initialize, exit))
    }

    fn matched(
        &mut self,
        at: NativeMatchOrigin,
        declaration: &MatchDeclaration,
        arguments: &[NativeMatchValue],
        output: NativeMatchValue,
    ) -> Result<(u32, u32), Error> {
        let owner = at.node();
        if declaration.partial {
            return Err(unsupported(owner, "partial or sticky Match"));
        }
        let scrutinee = *arguments
            .get(declaration.scrutinee as usize)
            .ok_or(Error::InvalidBinding { node: Some(owner) })?;
        for capture in &declaration.captures {
            let value = arguments
                .get(capture.input as usize)
                .ok_or(Error::InvalidBinding { node: Some(owner) })?;
            if self.ty(*value).schema() != capture.schema {
                return Err(Error::InvalidBinding { node: Some(owner) });
            }
        }
        let entry = self.block(at, NativeMatchAction::None)?;
        let join = self.block(at, NativeMatchAction::None)?;
        let mut test = entry;
        for arm in &declaration.arms {
            let next = self.block(at, NativeMatchAction::None)?;
            let mut binding = None;
            let literal = match &arm.pattern {
                MatchPattern::Wildcard | MatchPattern::Bind => None,
                MatchPattern::Literal(id) => Some(
                    *self
                        .constants
                        .get(id)
                        .ok_or(Error::InvalidBinding { node: Some(owner) })?,
                ),
                MatchPattern::Structural(crate::CollectionPattern::Wildcard) => None,
                MatchPattern::Structural(crate::CollectionPattern::Bind { local, schema }) => {
                    if *schema != self.ty(scrutinee).schema() {
                        return Err(Error::InvalidBinding { node: Some(owner) });
                    }
                    binding = Some((*local, scrutinee));
                    None
                }
                MatchPattern::Structural(crate::CollectionPattern::Equal(
                    crate::MatchPatternValue::Literal(id),
                )) => Some(
                    *self
                        .constants
                        .get(id)
                        .ok_or(Error::InvalidBinding { node: Some(owner) })?,
                ),
                _ => return Err(unsupported(owner, "aggregate or computed scalar pattern")),
            };
            let (body, body_tail, yielded) =
                self.control_block(owner, declaration, &arm.body, arguments, binding)?;
            if self.ty(yielded) != self.ty(output) {
                return Err(Error::InvalidBinding { node: Some(owner) });
            }
            let copy = self.block(
                origin(owner, Some(arm.body.id.0), None),
                NativeMatchAction::Copy {
                    source: yielded,
                    output,
                },
            )?;
            self.link(body_tail, copy);
            self.link(copy, join);
            let selected = if let Some(guard) = &arm.guard {
                let (start, tail, condition) =
                    self.control_block(owner, declaration, guard, arguments, binding)?;
                if self.ty(condition).kind() != NativeScalarKind::Bool {
                    return Err(Error::InvalidBinding { node: Some(owner) });
                }
                self.blocks[tail as usize].terminator = NativeMatchTerminator::Branch {
                    condition,
                    yes: body,
                    no: next,
                }
                .into();
                start
            } else {
                body
            };
            self.blocks[test as usize].terminator = (if let Some(peer) = literal {
                if self.ty(peer) != self.ty(scrutinee) {
                    return Err(Error::InvalidBinding { node: Some(owner) });
                }
                NativeMatchTerminator::LiteralBranch {
                    left: scrutinee,
                    right: peer,
                    yes: selected,
                    no: next,
                }
            } else {
                NativeMatchTerminator::Jump(selected)
            })
            .into();
            test = next;
        }
        // Keep a typed fail-closed terminal even though validated exhaustive
        // patterns make this path unreachable for admitted scalar inputs.
        self.blocks[test as usize].terminator = NativeMatchTerminator::Reject.into();
        Ok((entry, join))
    }
}

/// Lower canonical Match control using the same scalar schemas and selected
/// semantic witnesses as straight-line lowering. No source parser, value
/// executor, resident activation or older source-native lowering is called.
pub fn lower_scalar_match_artifact(
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
) -> Result<NativeMatchArtifactPlan, Error> {
    let lowered = lower_scalar_control_artifact(artifact, catalog, false)?;
    let blocks = lowered
        .blocks
        .into_vec()
        .into_iter()
        .map(|block| {
            let (
                NativeIterationAction::Scalar(action),
                NativeIterationTerminator::Scalar(terminator),
            ) = (block.action, block.terminator)
            else {
                return Err(Error::InvalidBinding {
                    node: Some(block.origin.node()),
                });
            };
            if block.iteration.is_some() {
                return Err(Error::InvalidBinding {
                    node: Some(block.origin.node()),
                });
            }
            Ok(NativeMatchBlock {
                origin: block.origin,
                action,
                terminator,
            })
        })
        .collect::<Result<Box<[_]>, Error>>()?;
    if !lowered.iterations.is_empty() {
        return Err(Error::InvalidBinding { node: None });
    }
    Ok(NativeMatchArtifactPlan {
        revision: lowered.revision,
        inputs: lowered.inputs,
        constants: lowered.constants,
        values: lowered.values,
        blocks,
        outputs: lowered.outputs,
        entry: lowered.entry,
    })
}

/// Metadata-only loop foundation. Exact scalar carriers include Bool, U64,
/// I64 and F64; element-count matrices, collections, lifecycle and resources
/// still require their own canonical native transport/lowering. This does not
/// claim full-program AOT support and never substitutes a resident evaluator.
pub fn lower_scalar_iteration_artifact(
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
) -> Result<NativeIterationArtifactPlan, Error> {
    lower_scalar_control_artifact(artifact, catalog, true)
}

fn lower_scalar_control_artifact(
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    allow_iteration: bool,
) -> Result<NativeIterationArtifactPlan, Error> {
    if !artifact.requirements().is_empty()
        || !artifact.constraints().is_empty()
        || !artifact.compute_regions().is_empty()
    {
        return Err(Error::UnsupportedProgram {
            feature: "requirements, constraints or compute regions",
        });
    }
    validate_scalar_slot_initializers(artifact)?;
    for node in artifact.nodes() {
        if !matches!(
            &node.body,
            ExecutableNodeBody::Operation(_) | ExecutableNodeBody::Match(_)
        ) && !(allow_iteration && matches!(&node.body, ExecutableNodeBody::Iteration(_)))
        {
            return Err(unsupported(node.node, "non-Match control or lifecycle"));
        }
    }
    let classes = classify_nodes(artifact, ResidentExternalAdmission::StructuralOnly)?;
    for node in artifact.nodes() {
        if classes[node.node.get() as usize] != NodeClass::Turn {
            return Err(unsupported(
                node.node,
                "Activation computation or external node",
            ));
        }
    }
    let schedule = build_activation_schedule(artifact, &classes)?;
    let mut builder = Builder {
        artifact,
        catalog,
        values: Vec::new(),
        constants: BTreeMap::new(),
        blocks: Vec::new(),
        value_scopes: Vec::new(),
        iterations: Vec::new(),
        current_iteration: None,
        allow_iteration,
    };
    let mut sources = BTreeMap::new();
    let mut inputs = Vec::new();
    for input in artifact.inputs() {
        let ty = scalar_type(artifact, input.schema)?;
        let value = builder.value(ty.clone())?;
        sources.insert(ArtifactSource::Slot(input.slot), value);
        inputs.push(NativeMatchInput {
            declaration: NativeInput {
                input: input.input,
                slot: input.slot,
                name: input.name.clone(),
                ty,
            },
            value,
        });
    }
    let mut constants = Vec::new();
    for ordinal in 0..artifact.constants().len() {
        let id = ConstantId::new(
            u32::try_from(ordinal).map_err(|_| Error::InvalidBinding { node: None })?,
        );
        let constant = artifact
            .constants()
            .get(id)
            .ok_or(Error::InvalidBinding { node: None })?;
        let ty = scalar_type(artifact, constant.schema())?;
        let scalar = match (ty.kind(), constant.data()) {
            (NativeScalarKind::Bool, mech_core::ValueData::Bool(value)) => {
                NativeScalarValue::Bool(*value)
            }
            (NativeScalarKind::U64, mech_core::ValueData::U64(value)) => {
                NativeScalarValue::U64(*value)
            }
            (NativeScalarKind::I64, mech_core::ValueData::I64(value)) => {
                NativeScalarValue::I64(*value)
            }
            (NativeScalarKind::F64, mech_core::ValueData::F64(value)) => {
                NativeScalarValue::F64(*value)
            }
            _ => {
                return Err(Error::UnsupportedSchema {
                    schema: constant.schema(),
                });
            }
        };
        if constant.schema_key() != ty.schema_key()
            || !constant.shape().parameter_values().is_empty()
        {
            return Err(Error::UnsupportedSchema {
                schema: constant.schema(),
            });
        }
        let value = builder.value(ty)?;
        builder.constants.insert(id, value);
        sources.insert(ArtifactSource::Constant(id), value);
        constants.push(NativeMatchConstant { value, scalar });
    }
    let root_origin = origin(NodeId::new(0), None, None);
    let entry = builder.block(root_origin, NativeMatchAction::None)?;
    let mut tail = entry;
    for node_id in schedule.nodes.iter().copied() {
        let node = &artifact.nodes()[node_id.get() as usize];
        let args = node_inputs(artifact, node_id)?
            .iter()
            .map(|&source| {
                let source = forwarded_output_source(artifact, source)?;
                sources.get(&source).copied().ok_or(Error::InvalidBinding {
                    node: Some(node_id),
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let slot = node_output_slot(artifact, node_id)?;
        let output = builder.value(scalar_type(
            artifact,
            artifact.slots()[slot.get() as usize].schema,
        )?)?;
        let at = origin(node_id, None, None);
        let (start, end) = match &node.body {
            ExecutableNodeBody::Operation(operation) => {
                let id = builder.primitive(
                    at,
                    &operation.operation,
                    operation.contract,
                    &args,
                    output,
                )?;
                (id, id)
            }
            ExecutableNodeBody::Match(control) => builder.matched(at, control, &args, output)?,
            ExecutableNodeBody::Iteration(control) if allow_iteration => {
                builder.iteration(at, control, &args, output)?
            }
            _ => unreachable!("node class checked before construction"),
        };
        builder.link(tail, start);
        tail = end;
        if sources.insert(ArtifactSource::Slot(slot), output).is_some() {
            return Err(Error::InvalidBinding {
                node: Some(node_id),
            });
        }
    }
    let completion = builder.block(root_origin, NativeMatchAction::None)?;
    builder.link(tail, completion);
    builder.blocks[completion as usize].terminator = NativeMatchTerminator::Complete.into();
    let outputs = artifact
        .outputs()
        .iter()
        .map(|output| {
            let source = forwarded_output_source(artifact, ArtifactSource::Slot(output.source))?;
            let value = *sources
                .get(&source)
                .ok_or(Error::InvalidBinding { node: None })?;
            let ty = scalar_type(artifact, output.schema)?;
            if builder.ty(value) != &ty {
                return Err(Error::InvalidBinding { node: None });
            }
            Ok(NativeMatchOutput {
                output: output.output,
                slot: output.source,
                name: output.name.clone(),
                interactive_binding: output.interactive_binding.clone(),
                value,
                ty,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let plan = NativeIterationArtifactPlan {
        revision: artifact.revision(),
        inputs: inputs.into(),
        constants: constants.into(),
        values: builder.values.into(),
        blocks: builder.blocks.into(),
        value_scopes: builder.value_scopes.into(),
        iterations: builder
            .iterations
            .into_iter()
            .map(|item| item.ok_or(Error::InvalidBinding { node: None }))
            .collect::<Result<Box<[_]>, _>>()?,
        outputs: outputs.into(),
        entry,
    };
    if allow_iteration {
        crate::native_artifact::iteration_validation::validate(&plan)
            .map_err(|_| Error::InvalidBinding { node: None })?;
    }
    Ok(plan)
}

#[cfg(all(test, feature = "source", feature = "artifact-codec"))]
mod iteration_proof_tests {
    use super::*;
    use mech_syntax::document::{
        AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
        parse_canonical_document,
    };
    use std::sync::Arc;
    fn plan(source: &str) -> NativeIterationArtifactPlan {
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x49545052), Revision(1), source).unwrap(),
            ParseConfig::default(),
        );
        assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
        // Identity requires no operation factory. A replacement with a cost
        // tag cannot become semantic authority through this test catalog.
        let catalog = Arc::new(mech_core::FunctionCatalogBuilder::new().build().unwrap());
        let artifact = crate::CanonicalSourceFrontend
            .compile_document_with_catalog(
                &DocumentSyntax::cast(parsed.syntax()).unwrap(),
                catalog.clone(),
            )
            .unwrap()
            .compile_artifact()
            .unwrap();
        lower_scalar_iteration_artifact(&artifact, &catalog).unwrap()
    }
    #[test]
    fn iteration_current_visit_proof_accepts_nested_identity_and_zero_count() {
        for count in [0, 1, 25, u64::MAX] {
            let source = format!(
                "identity(x<u64>) => (next<u64>) := next := x.\nouter(x<u64>) => (next<u64>) := next := x |3> identity.\nseed<u64> |{count}> outer\n"
            );
            let plan = plan(&source);
            assert_eq!(plan.iterations.len(), 2);
            plan.validate_current_visits().unwrap();
        }
    }
    #[test]
    fn iteration_current_visit_proof_rejects_later_result_and_unowned_backedge() {
        let source = "identity(x<u64>) => (next<u64>) := next := x.\nseed<u64> |25> identity\n";
        let valid = plan(source);
        let mut previous_visit = valid.clone();
        // This physical word exists and is zero-initialized, but has no
        // definition in the current body invocation. It must not be readable.
        previous_visit.iterations[0].next = previous_visit.iterations[0].result;
        assert!(previous_visit.validate_current_visits().is_err());
        let mut skipped_header = valid.clone();
        let iteration = &valid.iterations[0];
        skipped_header.blocks[iteration.latch() as usize].terminator =
            NativeIterationTerminator::Scalar(NativeMatchTerminator::Jump(iteration.body()));
        assert!(skipped_header.validate_current_visits().is_err());
        let mut forged_scope = valid.clone();
        forged_scope.value_scopes[iteration.carrier().get() as usize] =
            NativeIterationValueScope::Immutable { iteration: Some(0) };
        assert!(forged_scope.validate_current_visits().is_err());
    }
}
