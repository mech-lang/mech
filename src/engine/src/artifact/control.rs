//! Lexical, engine-owned executable control. Source text is never an operand.

use mech_core::{ConstantId, OperationContractId, SchemaId};

use super::OperationReference;

/// Dense identity within one match declaration. Blocks cannot reference blocks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ControlBlockId(pub u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BooleanPattern {
    Literal(bool),
    Wildcard,
    /// Makes the scrutinee available to explicitly declared block parameters.
    Bind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ControlCapture {
    /// Ordinal in the enclosing node's input bindings.
    pub input: u16,
    pub schema: SchemaId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControlParameterSource {
    Scrutinee,
    Capture(u16),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ControlParameter {
    pub source: ControlParameterSource,
    pub schema: SchemaId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControlValue {
    Constant(ConstantId),
    Parameter { block: ControlBlockId, ordinal: u16 },
    Local { block: ControlBlockId, node: u32 },
}

/// Each local has one writer and one exactly typed scalar result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControlOperation<C = OperationContractId> {
    pub node: u32,
    pub operation: OperationReference,
    pub contract: C,
    pub inputs: Box<[ControlValue]>,
    pub schema: SchemaId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControlBlock<C = OperationContractId> {
    pub id: ControlBlockId,
    pub parameters: Box<[ControlParameter]>,
    pub operations: Box<[ControlOperation<C>]>,
    pub yield_value: ControlValue,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BooleanMatchArm<C = OperationContractId> {
    pub pattern: BooleanPattern,
    pub guard: Option<ControlBlock<C>>,
    pub body: ControlBlock<C>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BooleanMatchDeclaration<C = OperationContractId> {
    /// Ordinal in the enclosing node's input bindings; evaluated once.
    pub scrutinee: u16,
    pub captures: Box<[ControlCapture]>,
    pub arms: Box<[BooleanMatchArm<C>]>,
}

pub(super) fn validate_match(
    draft: &super::ProgramArtifactDraft,
    node: mech_core::NodeId,
    declaration: &BooleanMatchDeclaration,
    inputs: &[SchemaId],
    output: SchemaId,
) -> Result<(), super::ArtifactBuildError> {
    use mech_core::{
        AccessMode, AliasPolicy, DeliveryMode, ExternalInteraction, OutputConstruction,
        ResolvedOperationContract, SchemaBody,
    };
    let invalid = |reason| super::ArtifactBuildError::InvalidControl { node, reason };
    let scalar = |schema| {
        draft
            .schemas
            .get(schema)
            .is_some_and(is_control_scalar_schema)
    };
    let boolean = |schema| {
        draft
            .schemas
            .get(schema)
            .is_some_and(|schema| matches!(schema.body(), SchemaBody::Bool))
    };
    let scrutinee = *inputs
        .get(declaration.scrutinee as usize)
        .ok_or_else(|| invalid("unknown scrutinee input"))?;
    if !boolean(scrutinee) || !scalar(output) {
        return Err(invalid(
            "Boolean match requires Boolean scrutinee and exact scalar result",
        ));
    }
    let mut used_inputs = std::collections::BTreeSet::from([declaration.scrutinee]);
    let mut captured_inputs = std::collections::BTreeSet::new();
    for capture in &declaration.captures {
        if inputs.get(capture.input as usize) != Some(&capture.schema)
            || !scalar(capture.schema)
            || !captured_inputs.insert(capture.input)
        {
            return Err(invalid("invalid or duplicate capture"));
        }
        used_inputs.insert(capture.input);
    }
    if used_inputs.len() != inputs.len() {
        return Err(invalid(
            "every enclosing input must have an explicit control role",
        ));
    }
    let mut next_block = 0u32;
    let mut coverage = [false; 2];
    for arm in &declaration.arms {
        for (block, is_guard) in arm
            .guard
            .iter()
            .map(|block| (block, true))
            .chain(core::iter::once((&arm.body, false)))
        {
            if block.id.0 != next_block {
                return Err(invalid("noncanonical block identity"));
            }
            next_block = next_block
                .checked_add(1)
                .ok_or_else(|| invalid("block count overflow"))?;
            for parameter in &block.parameters {
                let expected = match parameter.source {
                    ControlParameterSource::Scrutinee if arm.pattern == BooleanPattern::Bind => {
                        scrutinee
                    }
                    ControlParameterSource::Scrutinee => {
                        return Err(invalid("unbound scrutinee parameter"));
                    }
                    ControlParameterSource::Capture(index) => {
                        declaration
                            .captures
                            .get(index as usize)
                            .ok_or_else(|| invalid("unknown capture parameter"))?
                            .schema
                    }
                };
                if parameter.schema != expected {
                    return Err(invalid("parameter schema mismatch"));
                }
            }
            let value_schema = |value: ControlValue,
                                before: usize|
             -> Result<SchemaId, super::ArtifactBuildError> {
                match value {
                    ControlValue::Constant(id) => draft
                        .constants
                        .get(id)
                        .map(|value| value.schema())
                        .ok_or_else(|| invalid("unknown block constant")),
                    ControlValue::Parameter {
                        block: owner,
                        ordinal,
                    } if owner == block.id => block
                        .parameters
                        .get(ordinal as usize)
                        .map(|parameter| parameter.schema)
                        .ok_or_else(|| invalid("unknown block parameter")),
                    ControlValue::Local {
                        block: owner,
                        node: local,
                    } if owner == block.id && (local as usize) < before => {
                        Ok(block.operations[local as usize].schema)
                    }
                    _ => Err(invalid("cross-block or non-dominating local reference")),
                }
            };
            for (index, operation) in block.operations.iter().enumerate() {
                if operation.node as usize != index || !scalar(operation.schema) {
                    return Err(invalid(
                        "invalid local identity or non-scalar operation result",
                    ));
                }
                super::validation::validate_operation(&operation.operation)?;
                let Some(ResolvedOperationContract::Declared(contract)) =
                    draft.contracts.get(operation.contract)
                else {
                    return Err(super::ArtifactBuildError::UnknownOperationContract {
                        contract: operation.contract,
                    });
                };
                if contract.interaction != ExternalInteraction::Pure
                    || contract.inputs.len() != operation.inputs.len()
                    || contract.outputs.len() != 1
                {
                    return Err(invalid(
                        "blocks require pure ordinary single-result operations",
                    ));
                }
                for (input, port) in operation.inputs.iter().zip(&contract.inputs) {
                    if port.access != AccessMode::Read
                        || port.delivery != DeliveryMode::Signal
                        || value_schema(*input, index)? != port.schema
                        || !scalar(port.schema)
                    {
                        return Err(invalid("block operation input contract mismatch"));
                    }
                }
                let port = &contract.outputs[0];
                if port.schema != operation.schema
                    || port.access != AccessMode::Write
                    || port.delivery != DeliveryMode::Signal
                    || port.alias != AliasPolicy::NoAlias
                    || !matches!(port.construction, OutputConstruction::FullWrite { .. })
                {
                    return Err(invalid(
                        "block operation must fully write its exact owned result",
                    ));
                }
            }
            let yielded = value_schema(block.yield_value, block.operations.len())?;
            if (is_guard && !boolean(yielded)) || (!is_guard && yielded != output) {
                return Err(invalid("guard or arm yield schema mismatch"));
            }
        }
        if arm.guard.is_none() {
            match arm.pattern {
                BooleanPattern::Literal(value) => coverage[value as usize] = true,
                BooleanPattern::Wildcard | BooleanPattern::Bind => coverage = [true; 2],
            }
        }
    }
    if coverage != [true; 2] {
        return Err(invalid("non-exhaustive Boolean match"));
    }
    Ok(())
}

/// Structural admission limits shared by source finalization and wire defaults.
pub const MAX_CONTROL_ARMS: usize = 4_096;
pub const MAX_CONTROL_BLOCKS: usize = 8_192;
pub const MAX_CONTROL_OPERATIONS: usize = 65_536;
pub const MAX_CONTROL_OPERANDS: usize = 262_144;

pub(super) fn validate_control_counts(
    draft: &super::ProgramArtifactDraft,
) -> Result<(), super::ArtifactBuildError> {
    let mut counts = [0usize; 4];
    for node in &draft.nodes {
        let super::ExecutableNodeBody::BooleanMatch(control) = &node.body else {
            continue;
        };
        let invalid = || super::ArtifactBuildError::InvalidControl {
            node: node.node,
            reason: "control graph admission limit",
        };
        let mut add = |index: usize, amount: usize| -> Result<(), super::ArtifactBuildError> {
            counts[index] = counts[index].checked_add(amount).ok_or_else(invalid)?;
            if counts[index]
                > [
                    MAX_CONTROL_ARMS,
                    MAX_CONTROL_BLOCKS,
                    MAX_CONTROL_OPERATIONS,
                    MAX_CONTROL_OPERANDS,
                ][index]
            {
                return Err(invalid());
            }
            Ok(())
        };
        add(0, control.arms.len())?;
        add(3, control.captures.len())?;
        for block in control
            .arms
            .iter()
            .flat_map(|arm| arm.guard.iter().chain(core::iter::once(&arm.body)))
        {
            add(1, 1)?;
            add(2, block.operations.len())?;
            add(3, block.parameters.len())?;
            for operation in &block.operations {
                add(3, operation.inputs.len())?;
            }
        }
    }
    Ok(())
}

impl<C> BooleanMatchDeclaration<C> {
    pub(super) fn map_contracts<D, E>(
        &self,
        mut map: impl FnMut(&ControlBlock<C>, &ControlOperation<C>) -> Result<D, E>,
    ) -> Result<BooleanMatchDeclaration<D>, E> {
        let mut block = |block: &ControlBlock<C>| -> Result<ControlBlock<D>, E> {
            Ok(ControlBlock {
                id: block.id,
                parameters: block.parameters.clone(),
                yield_value: block.yield_value,
                operations: block
                    .operations
                    .iter()
                    .map(|operation| {
                        Ok(ControlOperation {
                            node: operation.node,
                            operation: operation.operation.clone(),
                            inputs: operation.inputs.clone(),
                            schema: operation.schema,
                            contract: map(block, operation)?,
                        })
                    })
                    .collect::<Result<Box<[_]>, E>>()?,
            })
        };
        Ok(BooleanMatchDeclaration {
            scrutinee: self.scrutinee,
            captures: self.captures.clone(),
            arms: self
                .arms
                .iter()
                .map(|arm| {
                    Ok(BooleanMatchArm {
                        pattern: arm.pattern,
                        guard: arm.guard.as_ref().map(&mut block).transpose()?,
                        body: block(&arm.body)?,
                    })
                })
                .collect::<Result<Box<[_]>, E>>()?,
        })
    }
}

pub(crate) fn is_control_scalar_schema(schema: &mech_core::Schema) -> bool {
    use mech_core::SchemaBody;
    schema.dimension_parameters().is_empty()
        && matches!(
            schema.body(),
            SchemaBody::Bool
                | SchemaBody::Index
                | SchemaBody::SignedInteger(_)
                | SchemaBody::UnsignedInteger(_)
                | SchemaBody::FloatingPoint(_)
                | SchemaBody::Complex(_)
                | SchemaBody::Rational64
        )
}
