//! Canonical scalar control -> immutable native CFG; metadata binding only.
//! Match and counted-loop views share selected witnesses, exact scalar types
//! and lexical block lowering. Neither path parses source or executes values.
use super::native_lowering::{
    port_layout, public_kind, public_op, scalar_type, validate_slot_initializers,
};
use super::{
    NodeClass, ResidentExternalAdmission, bind_operation_for_target, build_activation_schedule,
    classify_nodes, forwarded_output_source, node_inputs, node_output_slot,
};
use crate::native_artifact::{
    NativeArtifactLoweringError, NativeControlLoop, NativeInput, NativeIterationAction,
    NativeIterationBlock, NativeIterationLoop, NativeIterationTerminator,
    NativeIterationValueScope, NativeMatchAction, NativeMatchBlock, NativeMatchConstant,
    NativeMatchInput, NativeMatchOrigin, NativeMatchOutput, NativeMatchTerminator,
    NativeMatchValue, NativeProgramArtifactPlan, NativeProgramConstant, NativeProgramInput,
    NativeProgramOutput, NativeProgramType, NativeScalarKind, NativeScalarType, NativeScalarValue,
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

fn program_type(
    artifact: &ProgramArtifact,
    id: mech_core::SchemaId,
    managed: bool,
) -> Result<NativeProgramType, Error> {
    let definition = artifact
        .schemas()
        .get(id)
        .cloned()
        .ok_or(Error::UnsupportedSchema { schema: id })?;
    let scalar = match scalar_type(artifact, id) {
        Ok(ty) => Some(ty),
        Err(_) if managed => None,
        Err(error) => return Err(error),
    };
    Ok(NativeProgramType {
        schema: id,
        definition,
        scalar,
    })
}

pub(crate) fn validate_library_types(
    template: &crate::native_artifact::NativeProgramLibraryTemplate,
    operands: &[NativeMatchValue],
    output: NativeMatchValue,
    values: &[NativeProgramType],
) -> Result<(), &'static str> {
    let bad = "library call differs from its selected canonical types";
    let mech_core::ResolvedOperationContract::Declared(contract) = template.contract() else {
        return Err(bad);
    };
    if operands.len() != template.inputs().len()
        || operands.len() != contract.inputs.len()
        || contract.outputs.len() != 1
    {
        return Err(bad);
    }
    for ((&value, schema), port) in operands.iter().zip(template.inputs()).zip(&contract.inputs) {
        let ty = values.get(value.get() as usize).ok_or(bad)?;
        if ty.schema() != port.schema || ty.definition() != schema {
            return Err(bad);
        }
    }
    let ty = values.get(output.get() as usize).ok_or(bad)?;
    if ty.schema() != contract.outputs[0].schema || ty.definition() != template.output() {
        return Err(bad);
    }
    Ok(())
}

fn scalar_constant(
    ty: &NativeScalarType,
    constant: &mech_core::Value,
) -> Result<NativeScalarValue, Error> {
    match (ty.kind(), constant.data()) {
        (NativeScalarKind::Bool, mech_core::ValueData::Bool(v)) => Ok(NativeScalarValue::Bool(*v)),
        (NativeScalarKind::U64, mech_core::ValueData::U64(v)) => Ok(NativeScalarValue::U64(*v)),
        (NativeScalarKind::I64, mech_core::ValueData::I64(v)) => Ok(NativeScalarValue::I64(*v)),
        (NativeScalarKind::F64, mech_core::ValueData::F64(v)) => Ok(NativeScalarValue::F64(*v)),
        _ => Err(Error::UnsupportedSchema {
            schema: constant.schema(),
        }),
    }
}

fn scalar_plan(plan: NativeProgramArtifactPlan) -> Result<NativeIterationArtifactPlan, Error> {
    if !plan.collections.is_empty() {
        return Err(Error::UnsupportedProgram {
            feature: "managed collection in scalar plan",
        });
    }
    for block in &plan.blocks {
        if matches!(block.action(), NativeIterationAction::LibraryCall { .. }) {
            return Err(unsupported(
                block.origin().node(),
                "managed library call in scalar plan",
            ));
        }
    }
    let values = plan
        .values
        .into_vec()
        .into_iter()
        .map(NativeProgramType::into_scalar)
        .collect::<Result<Box<[_]>, _>>()?;
    let constants = plan
        .constants
        .into_vec()
        .into_iter()
        .map(|constant| {
            let ty = values
                .get(constant.value.get() as usize)
                .ok_or(Error::InvalidBinding { node: None })?;
            Ok(NativeMatchConstant {
                value: constant.value,
                scalar: scalar_constant(ty, &constant.constant)?,
            })
        })
        .collect::<Result<Box<[_]>, Error>>()?;
    let inputs = plan
        .inputs
        .into_vec()
        .into_iter()
        .map(|input| {
            Ok(NativeMatchInput {
                declaration: NativeInput {
                    input: input.input,
                    slot: input.slot,
                    name: input.name,
                    ty: input.ty.into_scalar()?,
                },
                value: input.value,
            })
        })
        .collect::<Result<Box<[_]>, Error>>()?;
    let outputs = plan
        .outputs
        .into_vec()
        .into_iter()
        .map(|output| {
            Ok(NativeMatchOutput {
                output: output.output,
                slot: output.slot,
                name: output.name,
                interactive_binding: output.interactive_binding,
                value: output.value,
                ty: output.ty.into_scalar()?,
            })
        })
        .collect::<Result<Box<[_]>, Error>>()?;
    Ok(NativeIterationArtifactPlan {
        revision: plan.revision,
        values,
        constants,
        inputs,
        outputs,
        value_scopes: plan.value_scopes,
        blocks: plan.blocks,
        iterations: plan
            .iterations
            .into_vec()
            .into_iter()
            .map(|item| match item {
                NativeControlLoop::Iteration(item) => Ok(item),
                NativeControlLoop::Collection(_) => Err(Error::UnsupportedProgram {
                    feature: "managed collection loop in scalar plan",
                }),
            })
            .collect::<Result<Box<[_]>, _>>()?,
        entry: plan.entry,
    })
}

pub(super) struct Builder<'a> {
    pub(super) artifact: &'a ProgramArtifact,
    pub(super) catalog: &'a FunctionCatalog,
    pub(super) values: Vec<NativeProgramType>,
    pub(super) constants: BTreeMap<ConstantId, NativeMatchValue>,
    pub(super) blocks: Vec<NativeIterationBlock>,
    pub(super) value_scopes: Vec<NativeIterationValueScope>,
    pub(super) iterations: Vec<Option<NativeControlLoop>>,
    pub(super) collections: Vec<Option<crate::native_artifact::NativeProgramCollection>>,
    pub(super) current_iteration: Option<u32>,
    pub(super) current_collection: Option<u32>,
    pub(super) loop_depth: usize,
    pub(super) allow_iteration: bool,
    pub(super) allow_collections: bool,
}
impl Builder<'_> {
    pub(super) fn value(&mut self, ty: NativeProgramType) -> Result<NativeMatchValue, Error> {
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
    pub(super) fn block(
        &mut self,
        at: NativeMatchOrigin,
        action: NativeMatchAction,
    ) -> Result<u32, Error> {
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
    pub(super) fn link(&mut self, from: u32, to: u32) {
        self.blocks[from as usize].terminator = NativeMatchTerminator::Jump(to).into();
    }
    pub(super) fn ty(&self, value: NativeMatchValue) -> &NativeProgramType {
        &self.values[value.get() as usize]
    }
    pub(super) fn type_for_schema(
        &self,
        id: mech_core::SchemaId,
    ) -> Result<NativeProgramType, Error> {
        program_type(self.artifact, id, self.allow_collections)
    }
    pub(super) fn primitive(
        &mut self,
        at: NativeMatchOrigin,
        reference: &crate::OperationReference,
        contract: mech_core::OperationContractId,
        arguments: &[NativeMatchValue],
        output: NativeMatchValue,
    ) -> Result<u32, Error> {
        #[cfg(any(feature = "pure_text", feature = "map_ops"))]
        if self.allow_collections {
            use crate::native_artifact::{
                NativeProgramLibraryTemplate, native_collection_lexical_template,
                native_collection_operation_template,
            };
            let template = match (at.block(), at.local()) {
                (Some(block), Some(local)) => native_collection_lexical_template(
                    self.artifact,
                    at.node(),
                    crate::ControlBlockId(block),
                    local,
                    self.catalog,
                )
                .map(NativeProgramLibraryTemplate::from_lexical),
                (None, None) => {
                    native_collection_operation_template(self.artifact, at.node(), self.catalog)
                        .map(NativeProgramLibraryTemplate::from_root)
                }
                _ => {
                    return Err(Error::InvalidBinding {
                        node: Some(at.node()),
                    });
                }
            };
            match template {
                Ok(template) => {
                    validate_library_types(&template, arguments, output, &self.values).map_err(
                        |_| Error::InvalidBinding {
                            node: Some(at.node()),
                        },
                    )?;
                    let id = self.block(at, NativeMatchAction::None)?;
                    self.blocks[id as usize].action = NativeIterationAction::LibraryCall {
                        template,
                        operands: arguments.into(),
                        output,
                    };
                    return Ok(id);
                }
                Err(error)
                    if self.ty(output).scalar().is_none()
                        || arguments
                            .iter()
                            .any(|&value| self.ty(value).scalar().is_none()) =>
                {
                    return Err(error);
                }
                Err(_) => {} // Scalar calls still require their actual scalar factory witness below.
            }
        }
        let layouts = arguments
            .iter()
            .map(|&value| {
                self.ty(value)
                    .scalar()
                    .map(port_layout)
                    .ok_or(Error::UnsupportedSchema {
                        schema: self.ty(value).schema(),
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let (kernel, memory_plan) = bind_operation_for_target(
            self.artifact,
            self.catalog,
            at.node(),
            reference,
            contract,
            &layouts,
            port_layout(self.ty(output).scalar().ok_or(Error::UnsupportedSchema {
                schema: self.ty(output).schema(),
            })?),
            ExecutionTarget::Native,
        )?;
        let semantics = native_semantics::validated_scalar(&kernel)
            .ok_or(Error::MissingSemanticWitness { node: at.node() })?;
        if semantics.inputs().len() != arguments.len()
            || semantics
                .inputs()
                .iter()
                .zip(arguments)
                .any(|(&kind, &arg)| Some(public_kind(kind)) != self.ty(arg).scalar_kind())
            || Some(public_kind(semantics.output())) != self.ty(output).scalar_kind()
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
    pub(super) fn lower_block(
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
            let result = self.value(self.type_for_schema(operation.schema)?)?;
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
                ControlOperationBody::Comprehension(nested) if self.allow_collections => {
                    self.comprehension(at, nested, &args, result)?
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
    pub(super) fn iteration(
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
        if self.loop_depth > crate::MAX_CONTROL_DEPTH {
            return Err(unsupported(
                owner,
                "native loop nesting exceeds control depth",
            ));
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
        let ty = self.type_for_schema(declaration.carrier)?;
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
        self.loop_depth += 1;
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
        self.loop_depth -= 1;
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
        self.iterations[id as usize] = Some(NativeControlLoop::Iteration(NativeIterationLoop {
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
        }));
        Ok((initialize, exit))
    }

    pub(super) fn matched(
        &mut self,
        at: NativeMatchOrigin,
        declaration: &MatchDeclaration,
        arguments: &[NativeMatchValue],
        output: NativeMatchValue,
    ) -> Result<(u32, u32), Error> {
        let owner = at.node();
        // An unguarded binding/wildcard is exhaustive for the exact validated
        // scrutinee type. Such a partial declaration cannot read a previous
        // result, so it uses ordinary total CFG lowering. Genuinely partial
        // (sticky) Match still requires persistent-state support and rejects.
        let exhaustive = declaration.arms.iter().any(|arm| {
            arm.guard.is_none()
                && matches!(
                    &arm.pattern,
                    MatchPattern::Wildcard
                        | MatchPattern::Bind
                        | MatchPattern::Structural(crate::CollectionPattern::Wildcard)
                        | MatchPattern::Structural(crate::CollectionPattern::Bind { .. })
                )
        });
        if declaration.partial && !exhaustive {
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
                if self.ty(condition).scalar_kind() != Some(NativeScalarKind::Bool) {
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
                if self.ty(peer).scalar().is_none() {
                    return Err(unsupported(
                        owner,
                        "managed literal equality requires native comparison",
                    ));
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
    scalar_plan(lower_control_artifact(
        artifact,
        catalog,
        allow_iteration,
        false,
    )?)
}

/// Lower through the very same canonical Match/loop builder as scalar plans.
/// Library actions retain actual selected root/lexical provenance. A fresh
/// invocation executes closed pure activation computations before its turn,
/// in the canonical activation schedule. This is not a retained reactive
/// instance: state, triggered activation scopes and external lifecycle remain
/// separate lowering work. Selection never binds geometry or executes values.
pub fn lower_native_program_artifact(
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
) -> Result<NativeProgramArtifactPlan, Error> {
    lower_control_artifact(artifact, catalog, true, true)
}

fn lower_control_artifact(
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    allow_iteration: bool,
    allow_collections: bool,
) -> Result<NativeProgramArtifactPlan, Error> {
    if !artifact.requirements().is_empty()
        || !artifact.constraints().is_empty()
        || !artifact.compute_regions().is_empty()
    {
        return Err(Error::UnsupportedProgram {
            feature: "requirements, constraints or compute regions",
        });
    }
    validate_slot_initializers(artifact, allow_collections, |id| {
        program_type(artifact, id, allow_collections).map(|_| ())
    })?;
    for node in artifact.nodes() {
        if !matches!(
            &node.body,
            ExecutableNodeBody::Operation(_) | ExecutableNodeBody::Match(_)
        ) && !(allow_iteration && matches!(&node.body, ExecutableNodeBody::Iteration(_)))
            && !(allow_collections && matches!(&node.body, ExecutableNodeBody::Comprehension(_)))
        {
            return Err(unsupported(node.node, "non-Match control or lifecycle"));
        }
    }
    let classes = classify_nodes(artifact, ResidentExternalAdmission::StructuralOnly)?;
    for node in artifact.nodes() {
        let class = classes[node.node.get() as usize];
        if class != NodeClass::Turn && !(allow_collections && class == NodeClass::Activation) {
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
        collections: Vec::new(),
        current_iteration: None,
        current_collection: None,
        loop_depth: 0,
        allow_iteration,
        allow_collections,
    };
    let mut sources = BTreeMap::new();
    let mut inputs = Vec::new();
    for input in artifact.inputs() {
        let ty = builder.type_for_schema(input.schema)?;
        let value = builder.value(ty.clone())?;
        sources.insert(ArtifactSource::Slot(input.slot), value);
        inputs.push(NativeProgramInput {
            input: input.input,
            slot: input.slot,
            name: input.name.clone(),
            ty,
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
        let ty = builder.type_for_schema(constant.schema())?;
        if constant.schema_key() != ty.schema_key()
            || (!allow_collections && !constant.shape().parameter_values().is_empty())
            || constant.validate_against(artifact.schemas()).is_err()
        {
            return Err(Error::UnsupportedSchema {
                schema: constant.schema(),
            });
        }
        let value = builder.value(ty)?;
        builder.constants.insert(id, value);
        sources.insert(ArtifactSource::Constant(id), value);
        constants.push(NativeProgramConstant {
            value,
            constant: constant.clone(),
        });
    }
    let root_origin = origin(NodeId::new(0), None, None);
    let entry = builder.block(root_origin, NativeMatchAction::None)?;
    let mut tail = entry;
    // The same resident schedule places the entire closed activation DAG
    // before Turn nodes, including independent nodes whose artifact ordinals
    // are interleaved. Do not fold/execute those values during compilation or
    // put their work outside the invocation's cumulative native admission.
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
        let output = builder
            .value(builder.type_for_schema(artifact.slots()[slot.get() as usize].schema)?)?;
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
            ExecutableNodeBody::Comprehension(control) if allow_collections => {
                builder.comprehension(at, control, &args, output)?
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
            let ty = builder.type_for_schema(output.schema)?;
            if builder.ty(value) != &ty {
                return Err(Error::InvalidBinding { node: None });
            }
            Ok(NativeProgramOutput {
                output: output.output,
                slot: output.source,
                name: output.name.clone(),
                interactive_binding: output.interactive_binding.clone(),
                value,
                ty,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let plan = NativeProgramArtifactPlan {
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
        collections: builder
            .collections
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
    fn artifact(source: &str) -> (crate::ProgramArtifact, Arc<mech_core::FunctionCatalog>) {
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
        (artifact, catalog)
    }
    fn plan(source: &str) -> NativeIterationArtifactPlan {
        let (artifact, catalog) = artifact(source);
        lower_scalar_iteration_artifact(&artifact, &catalog).unwrap()
    }
    #[test]
    fn mixed_current_visit_proof_rejects_stale_managed_carriers_and_cross_scope_edges() {
        for count in [0, 25] {
            let source = format!(
                "identity(x<string>) => (next<string>) := next := x.\nouter(x<string>) => (next<string>) := next := x |3> identity.\nseed<string> |{count}> outer\n"
            );
            let (artifact, catalog) = artifact(&source);
            let valid = lower_native_program_artifact(&artifact, &catalog).unwrap();
            valid.validate_current_visits().unwrap();
            assert!(
                valid.values()[valid.iterations()[0]
                    .as_iteration()
                    .unwrap()
                    .carrier()
                    .get() as usize]
                    .scalar()
                    .is_none()
            );
            let mut previous = valid.clone();
            let NativeControlLoop::Iteration(previous_loop) = &mut previous.iterations[1] else {
                panic!("literal loop")
            };
            previous_loop.next = previous_loop.result;
            assert!(previous.validate_current_visits().is_err());
            let mut cross_scope = valid.clone();
            let child = valid.iterations()[1].as_iteration().unwrap();
            cross_scope.blocks[child.latch() as usize].terminator =
                NativeIterationTerminator::Scalar(NativeMatchTerminator::Jump(child.body()));
            assert!(cross_scope.validate_current_visits().is_err());
            let mut forged_scope = valid.clone();
            forged_scope.value_scopes[child.carrier().get() as usize] =
                NativeIterationValueScope::Immutable { iteration: Some(1) };
            assert!(forged_scope.validate_current_visits().is_err());
        }
    }
    #[test]
    fn exhaustive_partial_function_matches_lower_but_genuinely_partial_matches_reject() {
        let (total, catalog) = artifact("pick(x<u64>) => <u64>\n  | x => x.\npick(seed<u64>)\n");
        lower_native_program_artifact(&total, &catalog)
            .unwrap()
            .validate_current_visits()
            .unwrap();
        let (partial, catalog) =
            artifact("pick(x<u64>) => <u64>\n  | 0u64 => 1u64.\npick(seed<u64>)\n");
        assert!(matches!(
            lower_native_program_artifact(&partial, &catalog),
            Err(Error::UnsupportedNode {
                feature: "partial or sticky Match",
                ..
            })
        ));
    }

    #[test]
    fn collection_current_visit_proof_rejects_uninitialized_elements_and_stale_outputs() {
        let (artifact, catalog) = artifact("[x | x <- values<[u64]>, y <- values]\n");
        let valid = lower_native_program_artifact(&artifact, &catalog).unwrap();
        valid.validate_current_visits().unwrap();
        assert_eq!(valid.collections().len(), 1);
        assert_eq!(valid.iterations().len(), 2);
        let mut stale_output = valid.clone();
        let output = stale_output.collections[0].output;
        let push = stale_output.collections[0].push as usize;
        stale_output.blocks[push].action = NativeIterationAction::CollectionPush {
            collection: 0,
            value: output,
        };
        assert!(stale_output.validate_current_visits().is_err());
        let mut self_source = valid.clone();
        let NativeControlLoop::Collection(generator) = &mut self_source.iterations[1] else {
            panic!("collection loop")
        };
        generator.source = generator.element.unwrap();
        assert!(self_source.validate_current_visits().is_err());
        let mut skipped_element = valid.clone();
        let NativeControlLoop::Collection(generator) = &skipped_element.iterations[1] else {
            panic!("collection loop")
        };
        skipped_element.blocks[generator.body as usize].action =
            NativeIterationAction::Scalar(NativeMatchAction::None);
        assert!(skipped_element.validate_current_visits().is_err());
        let mut cyclic_builder = valid.clone();
        cyclic_builder.collections[0].parent_collection = Some(0);
        assert!(cyclic_builder.validate_current_visits().is_err());
        let mut premature_finish = valid.clone();
        premature_finish.collections[0].finish = premature_finish.collections[0].begin;
        assert!(premature_finish.validate_current_visits().is_err());
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
