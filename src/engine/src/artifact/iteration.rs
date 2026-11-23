//! Exact counted application of one typed, pure lexical block.

use mech_core::{
    DimensionExpr, DimensionLifetime, OperationContractId, Schema, SchemaBody, SchemaId,
};

use super::{ControlBlock, ControlCapture, ControlOperationBody};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IterationCount {
    Literal(u64),
    /// Numeric matrix checked as exact U64 counts. A singleton broadcasts; otherwise its actual
    /// shape must equal the seed matrix shape. Each body receives one element.
    Elements {
        input: u16,
    },
}

pub(crate) fn is_iteration_count_element(body: &SchemaBody) -> bool {
    matches!(
        body,
        SchemaBody::Index
            | SchemaBody::UnsignedInteger(_)
            | SchemaBody::SignedInteger(_)
            | SchemaBody::FloatingPoint(_)
    )
}

/// The seed is evaluated once; the body is validated once and executed `count`
/// times with a privately owned current carrier. The body's id is its sole
/// lexical scope identity. A count of zero never executes a body operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IterationDeclaration<C = OperationContractId> {
    pub count: IterationCount,
    /// Ordinal in the enclosing node's actual input bindings.
    pub seed: u16,
    pub captures: Box<[ControlCapture]>,
    /// The actual current shape is checked against this schema at every boundary.
    pub carrier: SchemaId,
    pub body: ControlBlock<C>,
}

fn scalar(body: &SchemaBody) -> bool {
    matches!(
        body,
        SchemaBody::Bool
            | SchemaBody::Index
            | SchemaBody::String
            | SchemaBody::UnsignedInteger(_)
            | SchemaBody::SignedInteger(_)
            | SchemaBody::FloatingPoint(_)
    )
}

/// This bounded iteration slice owns exact primitive scalar values and flat
/// rank-two primitive matrices. Dynamic/structured carriers are unsupported.
/// Turn axes vary only through the checked actual-value boundary.
pub(crate) fn is_iteration_carrier_schema(schema: &Schema) -> bool {
    match schema.body() {
        body if scalar(body) => schema.dimension_parameters().is_empty(),
        SchemaBody::Matrix {
            element,
            dimensions,
        } => {
            dimensions.len() == 2
                && scalar(element)
                && schema
                    .dimension_parameters()
                    .iter()
                    .all(|parameter| parameter.lifetime() == DimensionLifetime::Turn)
        }
        _ => false,
    }
}

/// Metadata compatibility is deliberately weaker than geometry equality.
/// Different Turn identities do not prove equal extents. Actual current
/// extents and every declared bound are checked before replacing a carrier.
pub(crate) fn iteration_carrier_compatible(actual: &Schema, carrier: &Schema) -> bool {
    if !is_iteration_carrier_schema(actual) || !is_iteration_carrier_schema(carrier) {
        return false;
    }
    match (actual.body(), carrier.body()) {
        (
            SchemaBody::Matrix {
                element: actual,
                dimensions: ad,
            },
            SchemaBody::Matrix {
                element: carrier,
                dimensions: cd,
            },
        ) => {
            actual == carrier
                && ad.iter().zip(cd).all(|(a, c)| match (a, c) {
                    (DimensionExpr::Constant(a), DimensionExpr::Constant(c)) => a == c,
                    _ => true,
                })
        }
        (actual, carrier) => actual == carrier,
    }
}

impl<C> IterationDeclaration<C> {
    pub(super) fn validate_depth(
        &self,
        node: mech_core::NodeId,
    ) -> Result<(), super::ArtifactBuildError> {
        super::control::validate_iteration_depth(self, node)
    }

    #[cfg(feature = "resident-artifact")]
    pub(crate) fn contains_suspend(&self) -> bool {
        self.body
            .operations
            .iter()
            .any(|operation| match &operation.body {
                ControlOperationBody::Assert => false,
                super::ControlOperationBody::Suspend => true,
                super::ControlOperationBody::Match(control) => control.contains_suspend(),
                super::ControlOperationBody::Comprehension(control) => control.contains_suspend(),
                super::ControlOperationBody::Iteration(control) => control.contains_suspend(),
                super::ControlOperationBody::Operation { .. }
                | super::ControlOperationBody::Recur(_)
                | super::ControlOperationBody::Publish => false,
            })
    }

    pub(super) fn map_contracts<D, E>(
        &self,
        mut map: impl FnMut(&ControlBlock<C>, &super::ControlOperation<C>, &C) -> Result<D, E>,
    ) -> Result<IterationDeclaration<D>, E> {
        self.map_contracts_inner(&mut |block, operation, _, contract| {
            map(block, operation, contract)
        })
    }

    pub(super) fn map_contracts_inner<D, E>(
        &self,
        map: &mut dyn FnMut(
            &ControlBlock<C>,
            &super::ControlOperation<C>,
            &super::OperationReference,
            &C,
        ) -> Result<D, E>,
    ) -> Result<IterationDeclaration<D>, E> {
        let block = &self.body;
        Ok(IterationDeclaration {
            count: self.count,
            seed: self.seed,
            captures: self.captures.clone(),
            carrier: self.carrier,
            body: ControlBlock {
                id: block.id,
                parameters: block.parameters.clone(),
                yield_value: block.yield_value,
                operations: block
                    .operations
                    .iter()
                    .map(|operation| {
                        Ok(super::ControlOperation {
                            node: operation.node,
                            inputs: operation.inputs.clone(),
                            schema: operation.schema,
                            body: match &operation.body {
                                ControlOperationBody::Assert => ControlOperationBody::Assert,
                                super::ControlOperationBody::Operation {
                                    operation: reference,
                                    contract,
                                } => super::ControlOperationBody::Operation {
                                    operation: reference.clone(),
                                    contract: map(block, operation, reference, contract)?,
                                },
                                super::ControlOperationBody::Match(control) => {
                                    super::ControlOperationBody::Match(
                                        control.map_contracts_inner(map)?,
                                    )
                                }
                                super::ControlOperationBody::Comprehension(control) => {
                                    super::ControlOperationBody::Comprehension(
                                        control.map_contracts_inner(
                                            &mut |reference, contract| {
                                                map(block, operation, reference, contract)
                                            },
                                        )?,
                                    )
                                }
                                super::ControlOperationBody::Iteration(control) => {
                                    super::ControlOperationBody::Iteration(
                                        control.map_contracts_inner(map)?,
                                    )
                                }
                                super::ControlOperationBody::Recur(ancestor) => {
                                    super::ControlOperationBody::Recur(*ancestor)
                                }
                                super::ControlOperationBody::Suspend => {
                                    super::ControlOperationBody::Suspend
                                }
                                super::ControlOperationBody::Publish => {
                                    super::ControlOperationBody::Publish
                                }
                            },
                        })
                    })
                    .collect::<Result<Box<[_]>, E>>()?,
            },
        })
    }
}

pub(super) fn validate_iteration(
    draft: &super::ProgramArtifactDraft,
    node: mech_core::NodeId,
    declaration: &IterationDeclaration,
    inputs: &[SchemaId],
    output: SchemaId,
) -> Result<(), super::ArtifactBuildError> {
    declaration.validate_depth(node)?;
    validate_iteration_inner(draft, node, declaration, inputs, output, &mut 0, false)
}

pub(super) fn validate_iteration_inner(
    draft: &super::ProgramArtifactDraft,
    node: mech_core::NodeId,
    declaration: &IterationDeclaration,
    inputs: &[SchemaId],
    output: SchemaId,
    next_block: &mut u32,
    enclosing_guard: bool,
) -> Result<(), super::ArtifactBuildError> {
    use super::{ControlOperationBody, ControlParameterSource, ControlValue};
    use mech_core::{
        AccessMode, AliasPolicy, DeliveryMode, ExternalInteraction, OutputConstruction,
        ResolvedOperationContract,
    };
    let invalid = |reason| super::ArtifactBuildError::InvalidControl { node, reason };
    let carrier = draft
        .schemas
        .get(declaration.carrier)
        .ok_or_else(|| invalid("unknown iteration carrier schema"))?;
    let seed = *inputs
        .get(declaration.seed as usize)
        .ok_or_else(|| invalid("unknown iteration seed input"))?;
    let seed_schema = draft
        .schemas
        .get(seed)
        .ok_or_else(|| invalid("unknown iteration seed schema"))?;
    let output_schema = draft
        .schemas
        .get(output)
        .ok_or_else(|| invalid("unknown iteration output schema"))?;
    let compatible = match declaration.count {
        IterationCount::Literal(_) => {
            output == declaration.carrier && iteration_carrier_compatible(seed_schema, carrier)
        }
        IterationCount::Elements { input } => {
            let count = inputs
                .get(input as usize)
                .and_then(|id| draft.schemas.get(*id))
                .ok_or_else(|| invalid("unknown iteration count input"))?;
            if !matches!(count.body(), SchemaBody::Matrix { element, dimensions }
                if is_iteration_count_element(element) && dimensions.len() == 2)
                || !is_iteration_carrier_schema(count)
                || !carrier.dimension_parameters().is_empty()
                || !scalar(carrier.body())
            {
                return Err(invalid(
                    "element counts require a numeric matrix and scalar body",
                ));
            }
            match seed_schema.body() {
                SchemaBody::Matrix { element, .. } => {
                    **element == *carrier.body()
                        && iteration_carrier_compatible(seed_schema, output_schema)
                }
                _ => {
                    output == declaration.carrier
                        && iteration_carrier_compatible(seed_schema, carrier)
                }
            }
        }
    };
    if !is_iteration_carrier_schema(carrier) || !compatible {
        return Err(invalid(
            "iteration seed/output must preserve the supported carrier kind",
        ));
    }
    let mut used_inputs = std::collections::BTreeSet::from([declaration.seed]);
    if let IterationCount::Elements { input } = declaration.count {
        used_inputs.insert(input);
    }
    let mut captures = std::collections::BTreeSet::new();
    for capture in &declaration.captures {
        if inputs.get(capture.input as usize) != Some(&capture.schema)
            || !capture.freeze_on_suspend
            || !draft
                .schemas
                .get(capture.schema)
                .is_some_and(super::is_control_capture_schema)
            || !captures.insert(capture.input)
        {
            return Err(invalid(
                "invalid, non-frozen or duplicate iteration capture",
            ));
        }
        used_inputs.insert(capture.input);
    }
    if used_inputs.len() != inputs.len() || declaration.captures.len() > u16::MAX as usize {
        return Err(invalid(
            "every iteration input must have an explicit seed/count/capture role",
        ));
    }
    let block = &declaration.body;
    if block.id.0 != *next_block {
        return Err(invalid("noncanonical iteration block identity"));
    }
    *next_block = next_block
        .checked_add(1)
        .ok_or_else(|| invalid("iteration block count overflow"))?;
    if block.parameters.len() != declaration.captures.len() + 1 {
        return Err(invalid(
            "iteration parameters must be carrier followed by captures",
        ));
    }
    for (ordinal, parameter) in block.parameters.iter().enumerate() {
        let expected = if ordinal == 0 {
            if parameter.source != ControlParameterSource::IterationCarrier {
                return Err(invalid("iteration carrier parameter must be first"));
            }
            declaration.carrier
        } else {
            let index = ordinal - 1;
            if parameter.source != ControlParameterSource::Capture(index as u16) {
                return Err(invalid("noncanonical iteration capture parameter"));
            }
            declaration.captures[index].schema
        };
        if parameter.schema != expected {
            return Err(invalid("iteration parameter schema mismatch"));
        }
    }
    // Count zero never evaluates the body, but still validates every stored
    // operation, including unused/dead branches and nested controls.
    fn pure<C>(body: &ControlOperationBody<C>) -> bool {
        match body {
            ControlOperationBody::Assert => true,
            ControlOperationBody::Operation { .. } => true,
            ControlOperationBody::Iteration(control) => control
                .body
                .operations
                .iter()
                .all(|operation| pure(&operation.body)),
            ControlOperationBody::Match(control) => control
                .arms
                .iter()
                .flat_map(|arm| arm.guard.iter().chain(core::iter::once(&arm.body)))
                .flat_map(|block| block.operations.iter())
                .all(|operation| pure(&operation.body)),
            ControlOperationBody::Comprehension(control) => {
                control.operations().all(|operation| pure(&operation.body))
            }
            ControlOperationBody::Recur(_)
            | ControlOperationBody::Suspend
            | ControlOperationBody::Publish => false,
        }
    }
    if !block
        .operations
        .iter()
        .all(|operation| pure(&operation.body))
    {
        return Err(invalid(
            "iteration cannot contain recursion, suspension or publication",
        ));
    }
    let value_schema =
        |value: ControlValue, before: usize| -> Result<SchemaId, super::ArtifactBuildError> {
            match value {
                ControlValue::Constant(id) => draft
                    .constants
                    .get(id)
                    .map(|value| value.schema())
                    .ok_or_else(|| invalid("unknown iteration constant")),
                ControlValue::Parameter {
                    block: owner,
                    ordinal,
                } if owner == block.id => block
                    .parameters
                    .get(ordinal as usize)
                    .map(|parameter| parameter.schema)
                    .ok_or_else(|| invalid("unknown iteration parameter")),
                ControlValue::Local {
                    block: owner,
                    node: local,
                } if owner == block.id && (local as usize) < before => block
                    .operations
                    .get(local as usize)
                    .map(|operation| operation.schema)
                    .ok_or_else(|| invalid("unknown iteration local")),
                _ => Err(invalid("cross-block or non-dominating iteration reference")),
            }
        };
    for (index, operation) in block.operations.iter().enumerate() {
        if operation.node as usize != index || draft.schemas.get(operation.schema).is_none() {
            return Err(invalid("noncanonical iteration local identity/schema"));
        }
        let operation_inputs = operation
            .inputs
            .iter()
            .map(|value| value_schema(*value, index))
            .collect::<Result<Vec<_>, _>>()?;
        match &operation.body {
            ControlOperationBody::Assert => {
                if !super::control::is_assertion_signature(
                    &draft.schemas,
                    &operation_inputs,
                    operation.schema,
                ) {
                    return Err(invalid(
                        "lexical assertion requires one Bool input and Bool result",
                    ));
                }
            }
            ControlOperationBody::Operation {
                operation: reference,
                contract: id,
            } => {
                super::validation::validate_operation(reference)?;
                let Some(ResolvedOperationContract::Declared(contract)) = draft.contracts.get(*id)
                else {
                    return Err(invalid("iteration operation requires an ordinary contract"));
                };
                if contract.interaction != ExternalInteraction::Pure
                    || contract.inputs.len() != operation.inputs.len()
                    || contract.outputs.len() != 1
                {
                    return Err(invalid(
                        "iteration calls must be pure single-result operations",
                    ));
                }
                for (schema, port) in operation_inputs.iter().zip(contract.inputs.iter()) {
                    if *schema != port.schema
                        || port.access != AccessMode::Read
                        || port.delivery != DeliveryMode::Signal
                    {
                        return Err(invalid("iteration operation input contract mismatch"));
                    }
                }
                let port = &contract.outputs[0];
                if port.schema != operation.schema
                    || port.access != AccessMode::Write
                    || port.delivery != DeliveryMode::Signal
                    || port.alias != AliasPolicy::NoAlias
                    || !matches!(
                        port.construction,
                        OutputConstruction::FullWrite { .. }
                            | OutputConstruction::Build { .. }
                            | OutputConstruction::Replace { .. }
                    )
                {
                    return Err(invalid(
                        "iteration operation must fully write its exact owned result",
                    ));
                }
            }
            ControlOperationBody::Match(control) => super::control::validate_match_inner(
                draft,
                node,
                control,
                &operation_inputs,
                operation.schema,
                None,
                false,
                next_block,
                &[],
                false,
                enclosing_guard,
                false,
            )?,
            ControlOperationBody::Comprehension(control) => {
                super::comprehension::validate_comprehension_inner(
                    draft,
                    node,
                    control,
                    &operation_inputs,
                    operation.schema,
                    next_block,
                    enclosing_guard,
                )?
            }
            ControlOperationBody::Iteration(control) => validate_iteration_inner(
                draft,
                node,
                control,
                &operation_inputs,
                operation.schema,
                next_block,
                enclosing_guard,
            )?,
            ControlOperationBody::Recur(_)
            | ControlOperationBody::Suspend
            | ControlOperationBody::Publish => return Err(invalid("forbidden iteration control")),
        }
    }
    let yielded = value_schema(block.yield_value, block.operations.len())?;
    if !draft
        .schemas
        .get(yielded)
        .is_some_and(|schema| iteration_carrier_compatible(schema, carrier))
    {
        return Err(invalid(
            "iteration yield changes carrier kind or fixed geometry",
        ));
    }
    Ok(())
}
