//! Metadata-only canonical scalar lowering. No value executor is called here.
use super::{
    NodeClass, ResidentActivationError, ResidentExternalAdmission, bind_operation_for_target,
    build_activation_schedule, classify_nodes, forwarded_output_source, node_inputs,
    node_output_slot,
};
use crate::native_artifact::{
    NativeConstant, NativeInput, NativeLet, NativeOutput, NativeScalarKind, NativeScalarOp,
    NativeScalarType, NativeScalarValue, NativeValueRef,
};
use crate::resident::native_semantics;
use crate::{
    ArtifactSource, ExecutableNodeBody, InitializerReference, ProducerReference, ProgramArtifact,
    SlotRole,
};
use mech_core::{
    ConstantId, ExecutionTarget, FunctionCatalog, IntegerWidth, NodeId, ProgramRevision,
    ResidentPortLayout, ResidentShape, ResidentValueKind, ResolvedValueDescriptor, SchemaBody,
    SchemaId,
};
use std::collections::BTreeMap;

/// A validated, immutable metadata plan. Only this module can construct one.
/// No executor, resident kernel, deserializer, or unchecked constructor is
/// exposed. Native code generation and invocation are separate work.
#[derive(Clone, Debug)]
pub struct NativeArtifactPlan {
    revision: ProgramRevision,
    inputs: Box<[NativeInput]>,
    constants: Box<[NativeConstant]>,
    lets: Box<[NativeLet]>,
    outputs: Box<[NativeOutput]>,
}
impl NativeArtifactPlan {
    pub const fn program_revision(&self) -> ProgramRevision {
        self.revision
    }
    pub fn inputs(&self) -> &[NativeInput] {
        &self.inputs
    }
    pub fn constants(&self) -> &[NativeConstant] {
        &self.constants
    }
    /// Complete executor schedule. Unused checked nodes remain in this list.
    pub fn lets(&self) -> &[NativeLet] {
        &self.lets
    }
    pub fn outputs(&self) -> &[NativeOutput] {
        &self.outputs
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativeArtifactLoweringError {
    UnsupportedProgram { feature: &'static str },
    UnsupportedNode { node: NodeId, feature: &'static str },
    UnsupportedSchema { schema: SchemaId },
    InvalidBinding { node: Option<NodeId> },
    MissingSemanticWitness { node: NodeId },
    WitnessTypeMismatch { node: NodeId },
    ResidentBinding { error: ResidentActivationError },
}
impl std::fmt::Display for NativeArtifactLoweringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "canonical native artifact lowering: {self:?}")
    }
}
impl std::error::Error for NativeArtifactLoweringError {}
impl From<ResidentActivationError> for NativeArtifactLoweringError {
    fn from(error: ResidentActivationError) -> Self {
        Self::ResidentBinding { error }
    }
}

/// Validates and translates the actual canonical artifact and the actual
/// selected factory's private scalar certificate. This function only binds
/// metadata; it never activates a program, observes an input, runs a kernel,
/// folds a computation, or reconstructs a program from source names.
///
/// The initial slice is stateless Turn-only Bool/U64. Computed Activation
/// nodes are rejected even when their result could have been constant-folded.
pub fn lower_pure_native_artifact(
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
) -> Result<NativeArtifactPlan, NativeArtifactLoweringError> {
    if !artifact.requirements().is_empty() {
        return Err(NativeArtifactLoweringError::UnsupportedProgram {
            feature: "resources",
        });
    }
    if !artifact.constraints().is_empty() {
        return Err(NativeArtifactLoweringError::UnsupportedProgram {
            feature: "constraints",
        });
    }
    if !artifact.compute_regions().is_empty() {
        return Err(NativeArtifactLoweringError::UnsupportedProgram {
            feature: "compute regions",
        });
    }
    for slot in artifact.slots() {
        if slot.role == SlotRole::State {
            return Err(NativeArtifactLoweringError::UnsupportedProgram {
                feature: "state or initializers",
            });
        }
        scalar_type(artifact, slot.schema)?;
        if let Some(initializer) = slot.initializer {
            // Canonical compilation materializes a direct constant result in
            // an Output alias with that same constant as its publication
            // initializer. This is immutable output metadata, not retained
            // State or an Activation computation. Admit only an exact alias
            // of the same validated constant, using ordinary forwarding.
            let same_constant_output = match (slot.role, slot.producer, initializer) {
                (
                    SlotRole::Output,
                    ProducerReference::Output { output, .. },
                    InitializerReference::Constant(constant),
                ) => {
                    let declaration_matches = artifact
                        .outputs()
                        .get(output.get() as usize)
                        .is_some_and(|declaration| {
                            declaration.output == output
                                && declaration.source == slot.slot
                                && declaration.schema == slot.schema
                        });
                    let source =
                        forwarded_output_source(artifact, ArtifactSource::Slot(slot.slot))?;
                    declaration_matches
                        && source == ArtifactSource::Constant(constant)
                        && artifact.constants().get(constant).is_some_and(|value| {
                            value.schema() == slot.schema
                                && artifact
                                    .schemas()
                                    .entry(slot.schema)
                                    .is_some_and(|schema| schema.key() == value.schema_key())
                                && value.shape().parameter_values().is_empty()
                        })
                }
                _ => false,
            };
            if !same_constant_output {
                return Err(NativeArtifactLoweringError::UnsupportedProgram {
                    feature: "state or initializers",
                });
            }
        }
    }
    for node in artifact.nodes() {
        let feature = match &node.body {
            ExecutableNodeBody::Operation(operation) => {
                if operation.requirement.is_some() {
                    Some("resource operation")
                } else {
                    None
                }
            }
            ExecutableNodeBody::Match(_) => Some("Match"),
            ExecutableNodeBody::Activation(_) => Some("Activation scope"),
            ExecutableNodeBody::Comprehension(_) => Some("comprehension"),
            ExecutableNodeBody::Fsm(_) => Some("FSM"),
        };
        if let Some(feature) = feature {
            return Err(NativeArtifactLoweringError::UnsupportedNode {
                node: node.node,
                feature,
            });
        }
    }
    let classes = classify_nodes(artifact, ResidentExternalAdmission::StructuralOnly)?;
    for node in artifact.nodes() {
        if classes[node.node.get() as usize] != NodeClass::Turn {
            let feature = match classes[node.node.get() as usize] {
                NodeClass::Activation => "Activation computation",
                NodeClass::Observation => "observation",
                NodeClass::External => "external effect",
                NodeClass::Turn => unreachable!(),
            };
            return Err(NativeArtifactLoweringError::UnsupportedNode {
                node: node.node,
                feature,
            });
        }
    }
    // This is the same dependency order used by the ordinary executor. It
    // includes every node rather than only nodes reachable from outputs.
    let schedule = build_activation_schedule(artifact, &classes)?;
    let mut values = BTreeMap::<ArtifactSource, (NativeValueRef, NativeScalarType)>::new();
    let mut inputs = Vec::with_capacity(artifact.inputs().len());
    for input in artifact.inputs() {
        let slot = &artifact.slots()[input.slot.get() as usize];
        if slot.role != SlotRole::Input
            || slot.producer != ProducerReference::Input(input.input)
            || slot.schema != input.schema
        {
            return Err(NativeArtifactLoweringError::InvalidBinding { node: None });
        }
        let ty = scalar_type(artifact, input.schema)?;
        if values
            .insert(
                ArtifactSource::Slot(input.slot),
                (NativeValueRef::Input(input.input), ty.clone()),
            )
            .is_some()
        {
            return Err(NativeArtifactLoweringError::InvalidBinding { node: None });
        }
        inputs.push(NativeInput {
            input: input.input,
            slot: input.slot,
            name: input.name.clone(),
            ty,
        });
    }
    let mut constants = Vec::with_capacity(artifact.constants().len());
    for index in 0..artifact.constants().len() {
        let id = ConstantId::new(
            u32::try_from(index)
                .map_err(|_| NativeArtifactLoweringError::InvalidBinding { node: None })?,
        );
        let value = artifact
            .constants()
            .get(id)
            .expect("validated constant store ordinal");
        let ty = scalar_type(artifact, value.schema())?;
        if value.schema_key() != ty.schema_key || !value.shape().parameter_values().is_empty() {
            return Err(NativeArtifactLoweringError::UnsupportedSchema {
                schema: value.schema(),
            });
        }
        let scalar = match (ty.kind, value.data()) {
            (NativeScalarKind::Bool, mech_core::ValueData::Bool(value)) => {
                NativeScalarValue::Bool(*value)
            }
            (NativeScalarKind::U64, mech_core::ValueData::U64(value)) => {
                NativeScalarValue::U64(*value)
            }
            _ => {
                return Err(NativeArtifactLoweringError::UnsupportedSchema {
                    schema: value.schema(),
                });
            }
        };
        values.insert(
            ArtifactSource::Constant(id),
            (NativeValueRef::Constant(id), ty.clone()),
        );
        constants.push(NativeConstant {
            constant: id,
            ty,
            value: scalar,
        });
    }
    let mut lets = Vec::with_capacity(schedule.nodes.len());
    for node_id in schedule.nodes.iter().copied() {
        let node = artifact.nodes()[node_id.get() as usize]
            .as_operation()
            .expect("checked ordinary node");
        let target = node_output_slot(artifact, node_id)?;
        let slot = &artifact.slots()[target.get() as usize];
        if slot.role != SlotRole::Derived
            || slot.producer
                != (ProducerReference::NodeOutput {
                    node: node_id,
                    output_ordinal: 0,
                })
        {
            return Err(NativeArtifactLoweringError::InvalidBinding {
                node: Some(node_id),
            });
        }
        let result_type = scalar_type(artifact, slot.schema)?;
        let sources = node_inputs(artifact, node_id)?;
        let args = sources
            .iter()
            .map(|source| resolved_value(artifact, &values, *source, Some(node_id)))
            .collect::<Result<Vec<_>, _>>()?;
        let layouts = args
            .iter()
            .map(|(_, ty)| port_layout(ty))
            .collect::<Vec<_>>();
        let (kernel, memory_plan) = bind_operation_for_target(
            artifact,
            catalog,
            node_id,
            node.operation,
            node.contract,
            &layouts,
            port_layout(&result_type),
            ExecutionTarget::Native,
        )?;
        let semantics = native_semantics::validated_scalar(&kernel)
            .ok_or(NativeArtifactLoweringError::MissingSemanticWitness { node: node_id })?;
        if semantics.inputs().len() != args.len()
            || semantics
                .inputs()
                .iter()
                .copied()
                .zip(&args)
                .any(|(kind, (_, ty))| public_kind(kind) != ty.kind)
            || public_kind(semantics.output()) != result_type.kind
        {
            return Err(NativeArtifactLoweringError::WitnessTypeMismatch { node: node_id });
        }
        let local = NativeValueRef::Local(node_id);
        if values
            .insert(ArtifactSource::Slot(target), (local, result_type.clone()))
            .is_some()
        {
            return Err(NativeArtifactLoweringError::InvalidBinding {
                node: Some(node_id),
            });
        }
        lets.push(NativeLet {
            node: node_id,
            target,
            op: public_op(semantics.op()),
            operands: args
                .into_iter()
                .map(|(value, _)| value)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            ty: result_type,
            memory_plan,
        });
        // The bound resident implementation is dropped, never retained as a
        // fallback or evaluated. Only its sealed semantic identity survives.
    }
    let mut outputs = Vec::with_capacity(artifact.outputs().len());
    for output in artifact.outputs() {
        let ty = scalar_type(artifact, output.schema)?;
        let (value, source_type) =
            resolved_value(artifact, &values, ArtifactSource::Slot(output.source), None)?;
        if source_type != ty {
            return Err(NativeArtifactLoweringError::InvalidBinding { node: None });
        }
        outputs.push(NativeOutput {
            output: output.output,
            slot: output.source,
            name: output.name.clone(),
            interactive_binding: output.interactive_binding.clone(),
            value,
            ty,
        });
    }
    Ok(NativeArtifactPlan {
        revision: artifact.revision(),
        inputs: inputs.into_boxed_slice(),
        constants: constants.into_boxed_slice(),
        lets: lets.into_boxed_slice(),
        outputs: outputs.into_boxed_slice(),
    })
}

fn scalar_type(
    artifact: &ProgramArtifact,
    id: SchemaId,
) -> Result<NativeScalarType, NativeArtifactLoweringError> {
    let entry = artifact
        .schemas()
        .entry(id)
        .ok_or(NativeArtifactLoweringError::UnsupportedSchema { schema: id })?;
    let schema = entry.schema();
    if !schema.dimension_parameters().is_empty() {
        return Err(NativeArtifactLoweringError::UnsupportedSchema { schema: id });
    }
    let kind = match schema.body() {
        SchemaBody::Bool => NativeScalarKind::Bool,
        SchemaBody::UnsignedInteger(IntegerWidth::W64) => NativeScalarKind::U64,
        _ => return Err(NativeArtifactLoweringError::UnsupportedSchema { schema: id }),
    };
    let shape = schema
        .instantiate_shape(Box::new([]))
        .map_err(|_| NativeArtifactLoweringError::UnsupportedSchema { schema: id })?;
    let descriptor = ResolvedValueDescriptor::from_schema(schema.clone(), shape)
        .map_err(|_| NativeArtifactLoweringError::UnsupportedSchema { schema: id })?;
    Ok(NativeScalarType {
        kind,
        schema: id,
        schema_key: entry.key(),
        descriptor,
    })
}

fn port_layout(ty: &NativeScalarType) -> ResidentPortLayout {
    ResidentPortLayout {
        schema_id: ty.schema,
        schema_key: ty.schema_key,
        kind: match ty.kind {
            NativeScalarKind::Bool => ResidentValueKind::Bool,
            NativeScalarKind::U64 => ResidentValueKind::Snapshot,
        },
        shape: ResidentShape::SCALAR,
        shape_instance: ty.descriptor.shape().clone(),
        activation_fixed_shape: true,
        resolved_selector: None,
    }
}

fn resolved_value(
    artifact: &ProgramArtifact,
    values: &BTreeMap<ArtifactSource, (NativeValueRef, NativeScalarType)>,
    source: ArtifactSource,
    node: Option<NodeId>,
) -> Result<(NativeValueRef, NativeScalarType), NativeArtifactLoweringError> {
    let source = forwarded_output_source(artifact, source)?;
    values
        .get(&source)
        .cloned()
        .ok_or(NativeArtifactLoweringError::InvalidBinding { node })
}

fn public_kind(kind: native_semantics::NativeScalarKind) -> NativeScalarKind {
    match kind {
        native_semantics::NativeScalarKind::Bool => NativeScalarKind::Bool,
        native_semantics::NativeScalarKind::U64 => NativeScalarKind::U64,
    }
}
fn public_op(op: native_semantics::NativeScalarOp) -> NativeScalarOp {
    use native_semantics::NativeScalarOp as Private;
    match op {
        Private::U64Add => NativeScalarOp::U64Add,
        Private::U64Sub => NativeScalarOp::U64Sub,
        Private::U64Mul => NativeScalarOp::U64Mul,
        Private::U64Div => NativeScalarOp::U64Div,
        Private::U64Rem => NativeScalarOp::U64Rem,
        Private::U64Eq => NativeScalarOp::U64Eq,
        Private::U64Ne => NativeScalarOp::U64Ne,
        Private::U64Lt => NativeScalarOp::U64Lt,
        Private::U64Le => NativeScalarOp::U64Le,
        Private::U64Gt => NativeScalarOp::U64Gt,
        Private::U64Ge => NativeScalarOp::U64Ge,
        Private::BoolAnd => NativeScalarOp::BoolAnd,
        Private::BoolOr => NativeScalarOp::BoolOr,
        Private::BoolXor => NativeScalarOp::BoolXor,
        Private::BoolNot => NativeScalarOp::BoolNot,
        Private::BoolEq => NativeScalarOp::BoolEq,
        Private::BoolNe => NativeScalarOp::BoolNe,
    }
}
