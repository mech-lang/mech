//! Canonical scalar control -> immutable native CFG; metadata binding only.
//! Match and counted-loop views share selected witnesses, exact scalar types
//! and lexical block lowering. Neither path parses source or executes values.
use super::native_lowering::{port_layout, scalar_type, validate_slot_initializers};
use super::{
    NodeClass, ResidentExternalAdmission, bind_operation_for_target, build_activation_schedule,
    classify_nodes, forwarded_output_source, node_inputs, node_output_slot,
};
use crate::native_artifact::{
    NativeArtifactLoweringError, NativeControlLoop, NativeInput, NativeIterationAction,
    NativeIterationBlock, NativeIterationLoop, NativeIterationTerminator,
    NativeIterationValueScope, NativeMatchAction, NativeMatchBlock, NativeMatchConstant,
    NativeMatchInput, NativeMatchOrigin, NativeMatchOutput, NativeMatchTerminator,
    NativeMatchValue, NativeProgramArtifactPlan, NativeProgramConstant, NativeProgramEffect,
    NativeProgramInput, NativeProgramObservation, NativeProgramOutput, NativeProgramType,
    NativeScalarKind, NativeScalarType, NativeScalarValue,
};
use crate::resident::native_semantics;
use crate::{
    ArtifactSource, ControlBlock, ControlOperationBody, ControlParameterSource, ControlValue,
    ExecutableNodeBody, MatchDeclaration, MatchPattern, ProgramArtifact,
};
use mech_core::{ConstantId, ExecutionTarget, FunctionCatalog, NodeId, ProgramRevision};
use std::collections::BTreeMap;

#[path = "native_resource_lowering.rs"]
mod resource;

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

pub(super) type Error = NativeArtifactLoweringError;
pub(super) fn unsupported(node: NodeId, feature: &'static str) -> Error {
    Error::UnsupportedNode { node, feature }
}
pub(super) fn origin(node: NodeId, block: Option<u32>, local: Option<u32>) -> NativeMatchOrigin {
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
        (NativeScalarKind::U8, mech_core::ValueData::U8(v)) => Ok(NativeScalarValue::U8(*v)),
        (NativeScalarKind::U16, mech_core::ValueData::U16(v)) => Ok(NativeScalarValue::U16(*v)),
        (NativeScalarKind::U32, mech_core::ValueData::U32(v)) => Ok(NativeScalarValue::U32(*v)),

        (NativeScalarKind::U64, mech_core::ValueData::U64(v)) => Ok(NativeScalarValue::U64(*v)),
        (NativeScalarKind::I64, mech_core::ValueData::I64(v)) => Ok(NativeScalarValue::I64(*v)),
        (NativeScalarKind::F64, mech_core::ValueData::F64(v)) => Ok(NativeScalarValue::F64(*v)),
        _ => Err(Error::UnsupportedSchema {
            schema: constant.schema(),
        }),
    }
}

fn scalar_plan(plan: NativeProgramArtifactPlan) -> Result<NativeIterationArtifactPlan, Error> {
    if !plan.resource_requirements.is_empty()
        || !plan.observations.is_empty()
        || !plan.effects.is_empty()
    {
        return Err(Error::UnsupportedProgram {
            feature: "resource ports in a scalar plan",
        });
    }
    if !plan.collections.is_empty() {
        return Err(Error::UnsupportedProgram {
            feature: "managed collection in scalar plan",
        });
    }
    for block in &plan.blocks {
        if matches!(
            block.action(),
            NativeIterationAction::LibraryCall { .. }
                | NativeIterationAction::Structural { .. }
                | NativeIterationAction::PrepareU64Range { .. }
                | NativeIterationAction::PrepareStringSpanSelector { .. }
                | NativeIterationAction::ProjectStringSpan { .. }
                | NativeIterationAction::ProjectStringMapEntry { .. }
                | NativeIterationAction::TextJoinFinish { .. }
                | NativeIterationAction::HistogramFinish { .. }
                | NativeIterationAction::HistogramQueryStep { .. }
        ) {
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
                NativeControlLoop::Tail(_) => Err(Error::UnsupportedProgram {
                    feature: "managed tail loop in scalar plan",
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
    /// A constant tuple selector becomes the same checked structural child
    /// operation used by pattern binding. Callers select it only after the
    /// actual maintained access factory and its exact contract were certified.
    #[cfg(any(
        feature = "pure_text",
        feature = "map_ops",
        feature = "sequence_buffers"
    ))]
    fn tuple_scalar_projection(
        &mut self,
        at: NativeMatchOrigin,
        arguments: &[NativeMatchValue],
        output: NativeMatchValue,
    ) -> Result<u32, Error> {
        use crate::native_artifact::{NativeStructuralObservation as O, NativeStructuralStep as S};
        let [source, selector] = arguments else {
            return Err(Error::InvalidBinding {
                node: Some(at.node()),
            });
        };
        let ordinal = self
            .constants
            .iter()
            .find_map(|(id, value)| (*value == *selector).then_some(*id))
            .and_then(|id| self.artifact.constants().get(id))
            .and_then(|value| mech_core::canonical_positional_ordinal(value.data()).ok())
            .and_then(|ordinal| ordinal.checked_sub(1))
            .and_then(|ordinal| u32::try_from(ordinal).ok())
            .ok_or_else(|| {
                unsupported(
                    at.node(),
                    "native tuple projection requires a valid constant ordinal",
                )
            })?;
        let mech_core::SchemaBody::Tuple(fields) = self.ty(*source).definition().body() else {
            return Err(Error::InvalidBinding {
                node: Some(at.node()),
            });
        };
        if !fields.get(ordinal as usize).is_some_and(|body| {
            crate::native_artifact::structural_values::carrier_component_matches(
                self.ty(output).definition().body(),
                body,
            )
        }) {
            return Err(Error::WitnessTypeMismatch { node: at.node() });
        }
        let id = self.block(at, NativeMatchAction::None)?;
        self.blocks[id as usize].action = NativeIterationAction::Structural {
            source: *source,
            path: Box::new([S::TupleItem { ordinal }]),
            observation: if self.ty(output).definition().body()
                == &mech_core::SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64)
            {
                O::U64
            } else {
                O::CanonicalValue
            },
            output,
        };
        Ok(id)
    }

    pub(super) fn primitive(
        &mut self,
        at: NativeMatchOrigin,
        reference: &crate::OperationReference,
        contract: mech_core::OperationContractId,
        arguments: &[NativeMatchValue],
        output: NativeMatchValue,
    ) -> Result<u32, Error> {
        #[cfg(any(
            feature = "pure_text",
            feature = "map_ops",
            feature = "sequence_buffers"
        ))]
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
            let unsigned_cast = arguments.len() == 1
                && self
                    .ty(output)
                    .scalar_kind()
                    .is_some_and(|kind| kind.unsigned_maximum().is_some())
                && self
                    .ty(arguments[0])
                    .scalar_kind()
                    .is_some_and(|kind| kind.unsigned_maximum().is_some())
                && self.ty(output).scalar_kind() != self.ty(arguments[0]).scalar_kind();
            match template {
                // The actual conversion factory supplied this private proof.
                // All word widths now fit scalar native storage, so the same
                // binder must additionally seal its precise checked scalar op.
                #[cfg(feature = "convert")]
                Ok(template) if template.primitive() == crate::native_artifact::NativeCollectionPrimitive::UnsignedWordConversion => {}
                Err(error) if unsigned_cast => return Err(error),
                Ok(template) => {
                    validate_library_types(&template, arguments, output, &self.values).map_err(
                        |_| Error::InvalidBinding {
                            node: Some(at.node()),
                        },
                    )?;
                    if template.primitive()
                        == crate::native_artifact::NativeCollectionPrimitive::TupleScalarProjection
                    {
                        return self.tuple_scalar_projection(at, arguments, output);
                    }
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
        let (kernel, _memory_plan) = bind_operation_for_target(
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
                .any(|(&kind, &arg)| Some(kind) != self.ty(arg).scalar_kind())
            || Some(semantics.output()) != self.ty(output).scalar_kind()
        {
            return Err(Error::WitnessTypeMismatch { node: at.node() });
        }
        self.block(
            at,
            NativeMatchAction::Primitive {
                op: semantics.op(),
                operands: arguments.into(),
                output,
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
    pub(super) fn control_block(
        &mut self,
        owner: NodeId,
        declaration: &MatchDeclaration,
        block: &ControlBlock,
        arguments: &[NativeMatchValue],
        bindings: &BTreeMap<u32, NativeMatchValue>,
    ) -> Result<(u32, u32, NativeMatchValue), Error> {
        let (entry, returned) =
            self.control_block_tail(owner, declaration, block, arguments, bindings, None)?;
        let (exit, value) =
            returned.ok_or_else(|| unsupported(owner, "non-returning native block"))?;
        Ok((entry, exit, value))
    }

    pub(super) fn assertion(
        &mut self,
        at: NativeMatchOrigin,
        arguments: &[NativeMatchValue],
        output: NativeMatchValue,
    ) -> Result<(u32, u32), Error> {
        let [condition] = arguments else {
            return Err(Error::InvalidBinding {
                node: Some(at.node()),
            });
        };
        if self.ty(*condition).scalar_kind() != Some(NativeScalarKind::Bool)
            || self.ty(output).scalar_kind() != Some(NativeScalarKind::Bool)
        {
            return Err(Error::InvalidBinding {
                node: Some(at.node()),
            });
        }
        let entry = self.block(at, NativeMatchAction::None)?;
        let returned = self.block(
            at,
            NativeMatchAction::Copy {
                source: *condition,
                output,
            },
        )?;
        let failed = self.block(at, NativeMatchAction::None)?;
        self.blocks[entry as usize].terminator = NativeMatchTerminator::Branch {
            condition: *condition,
            yes: returned,
            no: failed,
        }
        .into();
        self.blocks[failed as usize].terminator = NativeMatchTerminator::IntegrityFailure.into();
        Ok((entry, returned))
    }
    fn control_block_tail(
        &mut self,
        owner: NodeId,
        declaration: &MatchDeclaration,
        block: &ControlBlock,
        arguments: &[NativeMatchValue],
        bindings: &BTreeMap<u32, NativeMatchValue>,
        tail_target: Option<super::native_tail_lowering::TailTarget>,
    ) -> Result<(u32, Option<(u32, NativeMatchValue)>), Error> {
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
                    ControlParameterSource::PatternBinding(local) => bindings.get(&local).copied(),
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
        self.lower_block_tail(owner, block, parameters, tail_target)
    }
    pub(super) fn lower_block(
        &mut self,
        owner: NodeId,
        block: &ControlBlock,
        parameters: Vec<NativeMatchValue>,
    ) -> Result<(u32, u32, NativeMatchValue), Error> {
        let (entry, returned) = self.lower_block_tail(owner, block, parameters, None)?;
        let (exit, value) =
            returned.ok_or_else(|| unsupported(owner, "non-returning native block"))?;
        Ok((entry, exit, value))
    }
    fn lower_block_tail(
        &mut self,
        owner: NodeId,
        block: &ControlBlock,
        parameters: Vec<NativeMatchValue>,
        tail_target: Option<super::native_tail_lowering::TailTarget>,
    ) -> Result<(u32, Option<(u32, NativeMatchValue)>), Error> {
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
                ControlOperationBody::Assert => self.assertion(at, &args, result)?,
                ControlOperationBody::Operation {
                    operation,
                    contract,
                } => {
                    let id = self.primitive(at, operation, *contract, &args, result)?;
                    (id, id)
                }
                ControlOperationBody::Match(nested) => {
                    let selected_tail = tail_target.filter(|_| {
                        operation.node as usize + 1 == block.operations.len()
                            && block.yield_value
                                == ControlValue::Local {
                                    block: block.id,
                                    node: operation.node,
                                }
                            && !super::native_tail_lowering::has_own_recur(nested)
                    });
                    if let Some(target) = selected_tail {
                        let (start, returned) = self.matched_body_flow(
                            at,
                            nested,
                            &args,
                            result,
                            Some(target.nested()?),
                        )?;
                        let Some(end) = returned else {
                            // Every selected child arm jumps to the owning
                            // loop. There is no return value or continuation
                            // here; do not fabricate an unreachable copy.
                            self.link(tail, start);
                            return Ok((entry, None));
                        };
                        (start, end)
                    } else {
                        self.matched(at, nested, &args, result)?
                    }
                }
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
        Ok((entry, Some((tail, yielded))))
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
        if self.allow_collections && super::native_tail_lowering::has_own_recur(declaration) {
            return self.tail_match(at, declaration, arguments, output);
        }
        self.matched_body(at, declaration, arguments, output, None)
    }

    pub(super) fn matched_body(
        &mut self,
        at: NativeMatchOrigin,
        declaration: &MatchDeclaration,
        arguments: &[NativeMatchValue],
        output: NativeMatchValue,
        tail: Option<super::native_tail_lowering::TailTarget>,
    ) -> Result<(u32, u32), Error> {
        let (entry, returned) = self.matched_body_flow(at, declaration, arguments, output, tail)?;
        let exit = returned
            .ok_or_else(|| unsupported(at.node(), "native tail loop has no return path"))?;
        Ok((entry, exit))
    }
    fn matched_body_flow(
        &mut self,
        at: NativeMatchOrigin,
        declaration: &MatchDeclaration,
        arguments: &[NativeMatchValue],
        output: NativeMatchValue,
        tail: Option<super::native_tail_lowering::TailTarget>,
    ) -> Result<(u32, Option<u32>), Error> {
        let owner = at.node();
        // An unguarded binding/wildcard is exhaustive for the exact validated
        // scrutinee type. Such a partial declaration cannot read a previous
        // result, so it uses ordinary total CFG lowering. Genuinely partial
        // (sticky) Match still requires persistent-state support and rejects.
        let exhaustive = declaration.arms.iter().any(|arm| {
            arm.guard.is_none()
                && match &arm.pattern {
                    MatchPattern::Wildcard | MatchPattern::Bind => true,
                    MatchPattern::Structural(pattern) => {
                        super::native_tail_lowering::irrefutable_pattern(pattern)
                    }
                    _ => false,
                }
        });
        // Only the owning loop's root carries the separate enum-coverage proof
        // from tail_match. An outer target does not prove a child's partial
        // match total and cannot waive its ordinary coverage check.
        if declaration.partial && !exhaustive && !tail.is_some_and(|target| target.ancestor == 0) {
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
        let mut returns = false;
        let mut test = entry;
        for arm in &declaration.arms {
            let next = self.block(at, NativeMatchAction::None)?;
            let mut bindings = BTreeMap::new();
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
                    bindings.insert(*local, scrutinee);
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
                MatchPattern::Structural(pattern) if self.allow_collections => {
                    let selected =
                        self.structural_pattern(at, pattern, scrutinee, test, next, &mut bindings)?;
                    test = selected;
                    None
                }
                _ => return Err(unsupported(owner, "aggregate or computed scalar pattern")),
            };
            let body = if let Some(target) = tail
                && let Some(prefix) =
                    super::native_tail_lowering::tail_prefix(&arm.body, target.ancestor)?
            {
                let (body, end, state) =
                    self.control_block(owner, declaration, &prefix, arguments, &bindings)?;
                if self.ty(state) != self.ty(target.next) {
                    return Err(Error::InvalidBinding { node: Some(owner) });
                }
                let copy = self.block(
                    origin(owner, Some(arm.body.id.0), None),
                    NativeMatchAction::Copy {
                        source: state,
                        output: target.next,
                    },
                )?;
                self.link(end, copy);
                self.link(copy, target.latch);
                body
            } else {
                let (body, returned) = self.control_block_tail(
                    owner,
                    declaration,
                    &arm.body,
                    arguments,
                    &bindings,
                    tail,
                )?;
                if let Some((body_tail, yielded)) = returned {
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
                    returns = true;
                }
                body
            };
            let selected = if let Some(guard) = &arm.guard {
                let (start, tail, condition) =
                    self.control_block(owner, declaration, guard, arguments, &bindings)?;
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
            if let Some(peer) = literal
                && self.ty(peer).scalar().is_none()
            {
                self.managed_literal_branch(at, scrutinee, peer, test, selected, next)?;
            } else {
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
            }
            test = next;
        }
        // Keep a typed fail-closed terminal even though validated exhaustive
        // patterns make this path unreachable for admitted scalar inputs.
        self.blocks[test as usize].terminator = NativeMatchTerminator::Reject.into();
        if !returns {
            self.blocks[join as usize].terminator = NativeMatchTerminator::Reject.into();
        }
        Ok((entry, returns.then_some(join)))
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

// Keep the validator's reason at the public lowering boundary. These checks
// still reject the same plans; callers can now identify the rejecting stage.
fn checked_plan_step(stage: &'static str, result: Result<(), &'static str>) -> Result<(), Error> {
    result.map_err(|reason| Error::InvalidPlan { stage, reason })
}

fn lower_control_artifact(
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    allow_iteration: bool,
    allow_collections: bool,
) -> Result<NativeProgramArtifactPlan, Error> {
    if !allow_collections && !artifact.requirements().is_empty() {
        return Err(Error::UnsupportedProgram {
            feature: "resource ports in a scalar plan",
        });
    }
    if !artifact.constraints().is_empty() || !artifact.compute_regions().is_empty() {
        return Err(Error::UnsupportedProgram {
            feature: "constraints or compute regions",
        });
    }
    let resource_requirements = resource::requirements(artifact)?;
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
        if class != NodeClass::Turn
            && !(allow_collections
                && matches!(
                    class,
                    NodeClass::Activation | NodeClass::Observation | NodeClass::External
                ))
        {
            return Err(unsupported(
                node.node,
                "Activation computation or external node",
            ));
        }
    }
    let schedule = build_activation_schedule(artifact, &classes)?;
    // Identity follows the resident step table's original artifact-node
    // order, while transport/commit order below follows the actual schedule.
    let effect_ordinals = artifact
        .nodes()
        .iter()
        .filter(|node| classes[node.node.get() as usize] == NodeClass::External)
        .enumerate()
        .map(|(ordinal, node)| {
            u32::try_from(ordinal)
                .map(|ordinal| (node.node, ordinal))
                .map_err(|_| Error::InvalidBinding {
                    node: Some(node.node),
                })
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
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
    let mut observations = Vec::new();
    let mut effects = Vec::new();
    let activation_nodes = artifact
        .nodes()
        .iter()
        .filter(|node| {
            allow_collections && classes[node.node.get() as usize] == NodeClass::Activation
        })
        .map(|node| node.node)
        .collect::<Box<[_]>>();
    let mut activation_boundary = None;
    // The same resident schedule places the entire closed activation DAG
    // before Turn nodes, including independent nodes whose artifact ordinals
    // are interleaved. Do not fold/execute those values during compilation or
    // put their work outside the invocation's cumulative native admission.
    for node_id in schedule.nodes.iter().copied() {
        let node = &artifact.nodes()[node_id.get() as usize];
        if classes[node_id.get() as usize] == NodeClass::Observation {
            // This node remains a requirement-bearing observation in the
            // original artifact. The authorized host supplies its initial SSA
            // value; no pure leaf, fake InputId or generated callback replaces it.
            let site = resource::observation(artifact, node, &resource_requirements)?;
            let ty = builder.type_for_schema(site.schema)?;
            let value = builder.value(ty.clone())?;
            if sources
                .insert(ArtifactSource::Slot(site.slot), value)
                .is_some()
            {
                return Err(Error::InvalidBinding {
                    node: Some(node_id),
                });
            }
            observations.push(NativeProgramObservation {
                node: node_id,
                requirement: site.requirement,
                request: site.request,
                slot: site.slot,
                ty,
                value,
                ordinal: 0,
            });
            continue;
        }
        if allow_collections
            && activation_boundary.is_none()
            && classes[node_id.get() as usize] != NodeClass::Activation
        {
            let boundary = builder.block(root_origin, NativeMatchAction::None)?;
            builder.link(tail, boundary);
            tail = boundary;
            activation_boundary = Some(boundary);
        }
        let args = node_inputs(artifact, node_id)?
            .iter()
            .map(|&source| {
                let source = forwarded_output_source(artifact, source)?;
                sources.get(&source).copied().ok_or(Error::InvalidBinding {
                    node: Some(node_id),
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        if classes[node_id.get() as usize] == NodeClass::External {
            let site = resource::effect(artifact, node, &resource_requirements)?;
            let [value] = args.as_slice() else {
                return Err(Error::InvalidBinding {
                    node: Some(node_id),
                });
            };
            let ty = builder.type_for_schema(site.schema)?;
            if builder.ty(*value) != &ty {
                return Err(Error::InvalidBinding {
                    node: Some(node_id),
                });
            }
            effects.push(NativeProgramEffect {
                node: node_id,
                requirement: site.requirement,
                request: site.request,
                payload: site.payload,
                ty,
                value: *value,
                ordinal: *effect_ordinals.get(&node_id).ok_or(Error::InvalidBinding {
                    node: Some(node_id),
                })?,
            });
            // A Send has NO application result slot. Its immutable payload is
            // transported only after Complete, for the host's normal transaction.
            continue;
        }
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
    if allow_collections && activation_boundary.is_none() {
        let boundary = builder.block(root_origin, NativeMatchAction::None)?;
        builder.link(tail, boundary);
        tail = boundary;
        activation_boundary = Some(boundary);
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
    // Match the resident input-capture order, independently of source-node
    // order. SSA identity is unaffected by this transport-only ordering.
    observations.sort_by_key(|observation| observation.slot.get());
    for (ordinal, observation) in observations.iter_mut().enumerate() {
        observation.ordinal = u32::try_from(ordinal).map_err(|_| Error::InvalidBinding {
            node: Some(observation.node),
        })?;
    }
    let mut plan = NativeProgramArtifactPlan {
        revision: artifact.revision(),
        resource_requirements,
        observations: observations.into(),
        effects: effects.into(),
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
        activation_nodes,
        activation_boundary,
    };
    if allow_iteration {
        let checked = if allow_collections {
            plan.validate_current_visits()
        } else {
            // Frozen scalar views retain their original proof and acquire no
            // managed phase boundary or generated phase dispatch.
            crate::native_artifact::iteration_validation::validate(&plan)
        };
        checked_plan_step("current visits before lowering", checked)?;
    }
    if allow_collections {
        checked_plan_step(
            "range cursor lowering",
            crate::native_artifact::range_cursor::lower(&mut plan),
        )?;
        checked_plan_step(
            "text join sink lowering",
            crate::native_artifact::text_join_sink::lower(&mut plan),
        )?;
        checked_plan_step(
            "guarded histogram query lowering",
            crate::native_artifact::guarded_histogram_query::lower(&mut plan),
        )?;
        checked_plan_step(
            "histogram sink lowering",
            crate::native_artifact::histogram_sink::lower(&mut plan),
        )?;
        checked_plan_step(
            "string span projection lowering",
            crate::native_artifact::string_span_projection::lower(&mut plan),
        )?;
        checked_plan_step(
            "string element map projection lowering",
            crate::native_artifact::string_element_map_projection::lower(&mut plan),
        )?;
        checked_plan_step(
            "current visits after lowering",
            plan.validate_current_visits(),
        )?;
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
    fn plan_rejection_preserves_stage_and_validator_reason() {
        let (artifact, catalog) = artifact("input<u64>");
        let valid = lower_native_program_artifact(&artifact, &catalog).unwrap();
        checked_plan_step(
            "current visits before lowering",
            valid.validate_current_visits(),
        )
        .unwrap();
        let mut invalid = valid;
        invalid.entry = u32::MAX;
        let reason = invalid.validate_current_visits().unwrap_err();
        for stage in [
            "current visits before lowering",
            "current visits after lowering",
        ] {
            let error = checked_plan_step(stage, invalid.validate_current_visits()).unwrap_err();
            assert_eq!(error, Error::InvalidPlan { stage, reason });
            let message = error.to_string();
            assert!(message.contains(stage));
            assert!(message.contains(reason));
        }
    }

    #[test]
    fn visit_admission_distinguishes_invalid_metadata_from_analysis_limit() {
        let (artifact, catalog) = artifact("input<u64>");
        let valid = lower_native_program_artifact(&artifact, &catalog).unwrap();
        let mut invalid_entry = valid.clone();
        invalid_entry.entry = u32::MAX;
        assert_eq!(
            crate::native_artifact::iteration_validation::validate(&invalid_entry),
            Err("current-visit entry is outside the control blocks")
        );
        let mut missing_scope = valid.clone();
        missing_scope.value_scopes = Box::new([]);
        assert_eq!(
            crate::native_artifact::iteration_validation::validate(&missing_scope),
            Err("current-visit scope count differs from value count")
        );
        // The proof rejects the estimated storage before building its fact sets.
        // Keep the test's own metadata small while crossing the 64 MiB cap.
        let mut oversized = valid;
        oversized.blocks = vec![oversized.blocks[0].clone(); 4096].into();
        oversized.values = vec![oversized.values[0].clone(); 131072].into();
        oversized.value_scopes =
            vec![NativeIterationValueScope::Immutable { iteration: None }; 131072].into();
        let reason = "current-visit analysis storage exceeds the 64 MiB compiler limit";
        assert_eq!(
            crate::native_artifact::iteration_validation::validate(&oversized),
            Err(reason)
        );
        assert_eq!(
            checked_plan_step(
                "current visits before lowering",
                crate::native_artifact::iteration_validation::validate(&oversized)
            ),
            Err(Error::InvalidPlan {
                stage: "current visits before lowering",
                reason
            })
        );
    }

    #[cfg(feature = "map_ops")]
    #[test]
    fn large_mapless_program_has_no_map_write_uses() {
        let (artifact, catalog) = artifact("input<u64>");
        let mut valid = lower_native_program_artifact(&artifact, &catalog).unwrap();
        let boundary = valid.activation_boundary.unwrap() as usize;
        let NativeIterationTerminator::Scalar(NativeMatchTerminator::Jump(complete)) =
            valid.blocks[boundary].terminator
        else {
            panic!("boundary jump")
        };
        let mut blocks = valid.blocks.into_vec();
        let first = blocks.len() as u32;
        let mut empty = blocks[boundary].clone();
        empty.action = NativeIterationAction::Scalar(NativeMatchAction::None);
        for _ in 0..16385 {
            let next = blocks.len() as u32 + 1;
            empty.terminator = NativeMatchTerminator::Jump(next).into();
            blocks.push(empty.clone());
        }
        blocks.last_mut().unwrap().terminator = NativeMatchTerminator::Jump(complete).into();
        blocks[boundary].terminator = NativeMatchTerminator::Jump(first).into();
        valid.blocks = blocks.into();
        valid.validate_current_visits().unwrap();
        assert!(valid.map_write_uses().unwrap().is_empty());
        // An absent map write does not bypass validation of the underlying CFG.
        valid.entry = u32::MAX;
        assert!(valid.map_write_uses().is_err());
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
    #[cfg(all(
        feature = "semantic-compiler",
        feature = "resident-artifact",
        any(
            feature = "pure_text",
            feature = "map_ops",
            feature = "sequence_buffers"
        )
    ))]
    fn closed_tail_plans() -> Vec<NativeProgramArtifactPlan> {
        // Real canonical enum packing and structural pattern sites. Bool is
        // the actual result type, not an extra schema injected for this test.
        let source = "#Once(n<u64>) => <bool>\n  | :Step(value<u64>)\n  | :Done(value<u64>).\n#Once(n) -> :Step(n)\n  :Step(value) -> :Done(value)\n  :Done(value) => true.\n#Once(input<u64>)\n";
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x5441494c), Revision(1), source).unwrap(),
            ParseConfig::default(),
        );
        assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
        let mut builder = mech_core::FunctionCatalogBuilder::new();
        crate::install_intrinsic_runtime(&mut builder).unwrap();
        crate::install_intrinsic_compiler_runtime(&mut builder).unwrap();
        crate::install_intrinsic_resident(&mut builder).unwrap();
        crate::install_intrinsic_source(&mut builder).unwrap();
        let catalog = Arc::new(builder.build().unwrap());
        let original = crate::CanonicalSourceFrontend
            .compile_document_with_catalog(
                &DocumentSyntax::cast(parsed.syntax()).unwrap(),
                catalog.clone(),
            )
            .unwrap()
            .compile_artifact()
            .unwrap();
        let decoded = crate::decode_program_artifact_bytecode_v1(
            &crate::encode_program_artifact_bytecode_v1(&original).unwrap(),
        )
        .unwrap();
        [original, decoded]
            .iter()
            .map(|artifact| lower_native_program_artifact(artifact, &catalog).unwrap())
            .collect()
    }

    #[cfg(all(
        feature = "semantic-compiler",
        feature = "resident-artifact",
        any(
            feature = "pure_text",
            feature = "map_ops",
            feature = "sequence_buffers"
        )
    ))]
    #[test]
    fn tail_current_visit_proof_rejects_forged_roles_edges_and_stale_return_slots() {
        for valid in closed_tail_plans() {
            valid.validate_current_visits().unwrap();
            assert_eq!(valid.iterations().len(), 1);
            let NativeControlLoop::Tail(tail) = valid.iterations()[0].clone() else {
                panic!("canonical synchronous transition must lower to Tail")
            };
            let mut forged = Vec::new();
            let mut plan = valid.clone();
            plan.blocks[tail.header() as usize].terminator = NativeIterationTerminator::Header {
                iteration: 0,
                body: tail.body(),
                exit: tail.exit(),
            };
            forged.push(("invented counted header", plan));
            let mut plan = valid.clone();
            let NativeControlLoop::Tail(data) = &mut plan.iterations[0] else {
                unreachable!()
            };
            std::mem::swap(&mut data.header, &mut data.latch);
            forged.push(("reordered header/latch roles", plan));
            let mut plan = valid.clone();
            plan.blocks[tail.latch() as usize].terminator =
                NativeMatchTerminator::Jump(tail.body()).into();
            forged.push(("latch bypasses header", plan));
            let mut plan = valid.clone();
            plan.blocks[tail.latch() as usize].terminator =
                NativeMatchTerminator::Jump(tail.exit()).into();
            forged.push(("latch bypasses return definition", plan));
            let mut plan = valid.clone();
            plan.blocks[tail.finish() as usize].terminator =
                NativeMatchTerminator::Jump(tail.header()).into();
            forged.push(("return reenters header", plan));
            let mut plan = valid.clone();
            let NativeControlLoop::Tail(data) = &mut plan.iterations[0] else {
                unreachable!()
            };
            data.returned = data.result;
            forged.push(("previous outer result is not current return", plan));
            let mut plan = valid.clone();
            let NativeControlLoop::Tail(data) = &mut plan.iterations[0] else {
                unreachable!()
            };
            data.next = data.seed;
            forged.push(("initial seed is not current next state", plan));
            let mut plan = valid.clone();
            plan.value_scopes[tail.carrier().get() as usize] =
                NativeIterationValueScope::Immutable { iteration: Some(0) };
            forged.push(("carrier disguised as immutable local", plan));
            for (value, label) in [
                (tail.next(), "missing current next-state definition"),
                (tail.returned(), "missing current return definition"),
            ] {
                let mut plan = valid.clone();
                let block = plan
                    .blocks
                    .iter_mut()
                    .find(|block| {
                        matches!(block.action(), NativeIterationAction::Scalar(
                        NativeMatchAction::Copy { output, .. }) if *output == value)
                    })
                    .expect("real canonical terminal copy");
                block.action = NativeIterationAction::Scalar(NativeMatchAction::None);
                forged.push((label, plan));
            }
            for (label, plan) in forged {
                assert!(plan.validate_current_visits().is_err(), "{label}");
            }
        }
    }

    #[cfg(all(
        feature = "semantic-compiler",
        feature = "resident-artifact",
        any(
            feature = "pure_text",
            feature = "map_ops",
            feature = "sequence_buffers"
        )
    ))]
    #[test]
    fn tail_proof_keeps_nominal_enum_identity_and_validates_tag_and_projection_paths() {
        for valid in closed_tail_plans() {
            let NativeControlLoop::Tail(tail) = &valid.iterations()[0] else {
                unreachable!()
            };
            let mut foreign_nominal = valid.clone();
            let ty = &mut foreign_nominal.values[tail.next().get() as usize];
            let mech_core::SchemaBody::Enum { variants, .. } = ty.definition.body() else {
                panic!("nominal enum state")
            };
            ty.definition = mech_core::SchemaDraft {
                dimension_parameters: Box::new([]),
                body: mech_core::SchemaBody::Enum {
                    key: mech_core::NominalKey::from_bytes([0x7b; 32]),
                    variants: variants.clone(),
                },
            }
            .finalize()
            .unwrap();
            assert!(foreign_nominal.validate_current_visits().is_err());
            let mut bad_tag = valid.clone();
            let action = bad_tag
                .blocks
                .iter_mut()
                .find_map(|block| match &mut block.action {
                    NativeIterationAction::Structural { observation, .. }
                        if matches!(
                            observation,
                            crate::native_artifact::NativeStructuralObservation::EnumOrdinal { .. }
                        ) =>
                    {
                        Some(observation)
                    }
                    _ => None,
                })
                .expect("canonical enum tag observation");
            *action = crate::native_artifact::NativeStructuralObservation::EnumOrdinal {
                ordinal: u32::MAX,
            };
            assert!(bad_tag.validate_current_visits().is_err());
            // A descriptive canonical-child opcode cannot reinterpret an
            // enum tag predicate's Bool result as the enum payload.
            let mut forged_child = valid.clone();
            let observation = forged_child
                .blocks
                .iter_mut()
                .find_map(|block| match &mut block.action {
                    NativeIterationAction::Structural { observation, .. }
                        if matches!(
                            observation,
                            crate::native_artifact::NativeStructuralObservation::EnumOrdinal { .. }
                        ) =>
                    {
                        Some(observation)
                    }
                    _ => None,
                })
                .unwrap();
            *observation = crate::native_artifact::NativeStructuralObservation::CanonicalValue;
            assert!(forged_child.validate_current_visits().is_err());
            let mut bad_path = valid.clone();
            let path = bad_path
                .blocks
                .iter_mut()
                .find_map(|block| match &mut block.action {
                    NativeIterationAction::Structural {
                        path, observation, ..
                    } if matches!(
                        observation,
                        crate::native_artifact::NativeStructuralObservation::U64
                    ) =>
                    {
                        Some(path)
                    }
                    _ => None,
                })
                .expect("canonical U64 state binding");
            *path = vec![crate::native_artifact::NativeStructuralStep::EnumPayload {
                ordinal: u32::MAX,
            }]
            .into();
            assert!(bad_path.validate_current_visits().is_err());
        }
    }

    #[test]
    fn frozen_scalar_views_still_reject_closed_root_calls_and_loops() {
        // Root Activation owners have never belonged to the frozen scalar
        // views. Do not widen that scope merely to exercise the managed phase
        // proof; closed lexical work inside a Turn owner is covered by the
        // scalar JIT/Object integration suites instead.
        // A passthrough function is elided to its argument and creates no
        // Activation owner. Keep a genuine closed control node in this test.
        let choose = "choose(x<u64>) => (next<u64>) := next := (x? | 0u64 => 1u64 | * => x).\n";
        for (closed, supports_match) in [
            ("fixed := choose(7u64)\n", true),
            ("fixed := 7u64 |3> choose\n", false),
        ] {
            let source = format!("{choose}{closed}choose(seed<u64>)\n");
            let (artifact, catalog) = artifact(&source);
            let classes =
                classify_nodes(&artifact, ResidentExternalAdmission::StructuralOnly).unwrap();
            assert!(classes.iter().any(|class| *class == NodeClass::Activation));
            assert!(classes.iter().any(|class| *class == NodeClass::Turn));
            assert!(matches!(
                lower_scalar_iteration_artifact(&artifact, &catalog),
                Err(Error::UnsupportedNode {
                    feature: "Activation computation or external node",
                    ..
                })
            ));
            if supports_match {
                assert!(matches!(
                    lower_scalar_match_artifact(&artifact, &catalog),
                    Err(Error::UnsupportedNode {
                        feature: "Activation computation or external node",
                        ..
                    })
                ));
            }
        }
    }

    #[test]
    fn native_phase_proof_rejects_moved_checkpoints_and_fresh_reads_during_activation() {
        // Identity passthroughs are folded to their argument. A real Match
        // keeps the closed owner and fresh copy in the authoritative schedule.
        let source = "choose(x<u64>) => (out<u64>) := out := (x? | 0u64 => 1u64 | * => x).\nclosed := choose(7u64)\nfresh := choose(input<u64>)\nfresh\n";
        let (artifact, catalog) = artifact(source);
        let valid = lower_native_program_artifact(&artifact, &catalog).unwrap();
        valid.validate_current_visits().unwrap();
        assert!(valid.has_activation());
        let boundary = valid.activation_boundary().unwrap();
        assert_ne!(boundary, valid.entry());
        let mut missing = valid.clone();
        missing.activation_boundary = None;
        assert!(missing.validate_current_visits().is_err());
        let mut early = valid.clone();
        early.activation_boundary = Some(early.entry());
        assert!(early.validate_current_visits().is_err());
        let mut relabeled = valid.clone();
        relabeled.activation_nodes = Box::new([]);
        assert!(relabeled.validate_current_visits().is_err());
        let mut fresh_read = valid.clone();
        let input = fresh_read.inputs()[0].value();
        let owners = fresh_read.activation_nodes().to_vec();
        let action = fresh_read
            .blocks
            .iter_mut()
            .find_map(|block| {
                if !owners.contains(&block.origin().node()) {
                    return None;
                }
                match &mut block.action {
                    NativeIterationAction::Scalar(NativeMatchAction::Copy { source, .. }) => {
                        Some(source)
                    }
                    _ => None,
                }
            })
            .expect("closed canonical Match result copy");
        *action = input;
        assert!(fresh_read.validate_current_visits().is_err());
    }

    #[test]
    fn root_scope_cycle_is_rejected_before_once_storage_is_derived() {
        let source = "choose(x<u64>) => (out<u64>) := out := (x? | 0u64 => 1u64 | * => x).\nclosed := choose(7u64)\nfresh := choose(input<u64>)\nfresh\n";
        let (artifact, catalog) = artifact(source);
        let valid = lower_native_program_artifact(&artifact, &catalog).unwrap();
        valid.validate_current_visits().unwrap();
        let boundary = valid.activation_boundary().unwrap();
        assert!(valid.blocks()[boundary as usize].iteration().is_none());
        let mut cyclic = valid.clone();
        cyclic.blocks[boundary as usize].terminator =
            NativeIterationTerminator::Scalar(NativeMatchTerminator::Jump(boundary));
        // Exercise the root DAG proof itself, independently of phase checks.
        assert_eq!(
            crate::native_artifact::iteration_validation::validate(&cyclic),
            Err("invalid counted-loop ownership or current-visit definitions")
        );
        assert!(cyclic.validate_current_visits().is_err());
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
        generator.source =
            crate::native_artifact::NativeCollectionSource::Value(generator.element.unwrap());
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
