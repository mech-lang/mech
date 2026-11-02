use crate::{SchemaBody, SchemaId, SchemaTable};

use super::{
    AccessMode, AliasPolicy, DeclaredOperationContract, DeliveryMode, ExternalInteraction,
    InputPortLayout, OperationContractDeclaration, OutputConstruction, ResolvedInputPort,
    ResolvedOperationContract, ShapeRule,
};

#[cfg(feature = "no_std")]
use alloc::string::String;
#[cfg(not(feature = "no_std"))]
use std::string::String;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PortDirection {
    Input,
    Output,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OperationContractError {
    IdentityExhausted {
        identity: &'static str,
    },
    InvalidContractHandle,
    NonCanonicalContractOrder,
    NonCanonicalContractBytes,
    InvalidCanonicalEncoding {
        reason: &'static str,
    },
    PortCountMismatch {
        direction: PortDirection,
        expected: u64,
        actual: u64,
    },
    VariadicInputCount {
        prefix: u64,
        minimum_repetitions: u32,
        actual: u64,
    },
    InvalidAccessDirection {
        direction: PortDirection,
        ordinal: u32,
        access: AccessMode,
    },
    InvalidConstructionAccess {
        output: u32,
        access: AccessMode,
        construction: &'static str,
    },
    InputOrdinalOutOfRange {
        field: &'static str,
        input: u16,
        inputs: u32,
    },
    ReadModifyWriteSchemaMismatch {
        output: u32,
        base_input: u16,
        input_schema: SchemaId,
        output_schema: SchemaId,
    },
    AliasSchemaMismatch {
        output: u32,
        input: u16,
        input_schema: SchemaId,
        output_schema: SchemaId,
    },
    EffectOutputUnsupported {
        outputs: u32,
    },
    UnknownSchema {
        schema: SchemaId,
    },
    MatrixShapeRuleRequiresMatrix {
        field: &'static str,
        input: u16,
    },
    TransposeSchemaMismatch {
        input: u16,
        output: u32,
    },
    SameShapeSchemaMismatch {
        input: u16,
        output: u32,
    },
    MatrixProductSchemaMismatch {
        lhs: u16,
        rhs: u16,
        output: u32,
    },
    InvalidShapeContractReference {
        field: &'static str,
        value: String,
    },
    UnsupportedSignalBinding {
        direction: PortDirection,
        ordinal: u32,
        delivery: DeliveryMode,
    },
}

pub fn validate_declaration(
    declaration: &OperationContractDeclaration,
) -> Result<(), OperationContractError> {
    validate_interaction_outputs(&declaration.interaction, declaration.outputs.len())?;
    match &declaration.inputs {
        InputPortLayout::Fixed(inputs) => {
            for (ordinal, input) in inputs.iter().enumerate() {
                validate_input_access(ordinal as u32, input.access)?;
            }
        }
        InputPortLayout::Variadic {
            prefix, repeated, ..
        } => {
            for (ordinal, input) in prefix.iter().enumerate() {
                validate_input_access(ordinal as u32, input.access)?;
            }
            validate_input_access(prefix.len() as u32, repeated.access)?;
        }
    }
    for (ordinal, output) in declaration.outputs.iter().enumerate() {
        validate_output_access(ordinal as u32, output.access)?;
        validate_construction_access(ordinal as u32, output.access, &output.construction)?;
        validate_shape_reference(&output.construction)?;
    }
    Ok(())
}

pub fn validate_resolved_contract(
    contract: &ResolvedOperationContract,
) -> Result<(), OperationContractError> {
    match contract {
        ResolvedOperationContract::Declared(contract) => validate_declared(contract),
    }
}

fn validate_declared(contract: &DeclaredOperationContract) -> Result<(), OperationContractError> {
    validate_interaction_outputs(&contract.interaction, contract.outputs.len())?;
    for (ordinal, input) in contract.inputs.iter().enumerate() {
        validate_input_access(ordinal as u32, input.access)?;
    }
    for (ordinal, output) in contract.outputs.iter().enumerate() {
        let ordinal = ordinal as u32;
        validate_output_access(ordinal, output.access)?;
        validate_construction_access(ordinal, output.access, &output.construction)?;
        validate_shape_reference(&output.construction)?;
        for (field, input) in referenced_inputs(&output.construction) {
            if input as usize >= contract.inputs.len() {
                return Err(OperationContractError::InputOrdinalOutOfRange {
                    field,
                    input,
                    inputs: contract.inputs.len() as u32,
                });
            }
        }
        if let OutputConstruction::ReadModifyWrite { base_input, .. } = output.construction {
            let input_schema = contract.inputs[base_input as usize].schema;
            if input_schema != output.schema {
                return Err(OperationContractError::ReadModifyWriteSchemaMismatch {
                    output: ordinal,
                    base_input,
                    input_schema,
                    output_schema: output.schema,
                });
            }
        }
        validate_alias_policy(ordinal, output.schema, output.alias, &contract.inputs)?;
    }
    Ok(())
}

fn validate_interaction_outputs(
    interaction: &ExternalInteraction,
    output_count: usize,
) -> Result<(), OperationContractError> {
    if matches!(interaction, ExternalInteraction::Effect(_)) && output_count != 0 {
        return Err(OperationContractError::EffectOutputUnsupported {
            outputs: u32::try_from(output_count).unwrap_or(u32::MAX),
        });
    }
    Ok(())
}

pub fn validate_contract_schemas(
    contract: &ResolvedOperationContract,
    schemas: &SchemaTable,
) -> Result<(), OperationContractError> {
    validate_resolved_contract(contract)?;
    let ResolvedOperationContract::Declared(contract) = contract;
    let inputs = contract
        .inputs
        .iter()
        .map(|port| port.schema)
        .collect::<Vec<_>>();
    let outputs = contract
        .outputs
        .iter()
        .map(|port| port.schema)
        .collect::<Vec<_>>();
    for schema in inputs.iter().chain(outputs.iter()).copied() {
        if schemas.get(schema).is_none() {
            return Err(OperationContractError::UnknownSchema { schema });
        }
    }
    for (output_ordinal, output) in contract.outputs.iter().enumerate() {
        match output.construction {
            OutputConstruction::FullWrite { shape } | OutputConstruction::Replace { shape } => {
                validate_shape_rule(shape, output_ordinal as u32, contract, schemas)?
            }
            OutputConstruction::ReadModifyWrite { .. } | OutputConstruction::Build { .. } => {}
        }
    }
    Ok(())
}

pub fn validate_signal_bindings(
    contract: &ResolvedOperationContract,
) -> Result<(), OperationContractError> {
    let ResolvedOperationContract::Declared(contract) = contract;
    for (ordinal, input) in contract.inputs.iter().enumerate() {
        if input.delivery != DeliveryMode::Signal {
            return Err(OperationContractError::UnsupportedSignalBinding {
                direction: PortDirection::Input,
                ordinal: ordinal as u32,
                delivery: input.delivery,
            });
        }
    }
    for (ordinal, output) in contract.outputs.iter().enumerate() {
        if output.delivery != DeliveryMode::Signal {
            return Err(OperationContractError::UnsupportedSignalBinding {
                direction: PortDirection::Output,
                ordinal: ordinal as u32,
                delivery: output.delivery,
            });
        }
    }
    Ok(())
}

fn validate_input_access(ordinal: u32, access: AccessMode) -> Result<(), OperationContractError> {
    if matches!(access, AccessMode::Read | AccessMode::Consume) {
        Ok(())
    } else {
        Err(OperationContractError::InvalidAccessDirection {
            direction: PortDirection::Input,
            ordinal,
            access,
        })
    }
}

fn validate_output_access(ordinal: u32, access: AccessMode) -> Result<(), OperationContractError> {
    if matches!(access, AccessMode::Write | AccessMode::ReadWrite) {
        Ok(())
    } else {
        Err(OperationContractError::InvalidAccessDirection {
            direction: PortDirection::Output,
            ordinal,
            access,
        })
    }
}

fn validate_construction_access(
    output: u32,
    access: AccessMode,
    construction: &OutputConstruction,
) -> Result<(), OperationContractError> {
    let (required, name) = match construction {
        OutputConstruction::ReadModifyWrite { .. } => (AccessMode::ReadWrite, "ReadModifyWrite"),
        OutputConstruction::FullWrite { .. } => (AccessMode::Write, "FullWrite"),
        OutputConstruction::Replace { .. } => (AccessMode::Write, "Replace"),
        OutputConstruction::Build { .. } => (AccessMode::Write, "Build"),
    };
    if access == required {
        Ok(())
    } else {
        Err(OperationContractError::InvalidConstructionAccess {
            output,
            access,
            construction: name,
        })
    }
}

fn validate_shape_reference(
    construction: &OutputConstruction,
) -> Result<(), OperationContractError> {
    let OutputConstruction::Build { postcondition } = construction else {
        return Ok(());
    };
    if postcondition.module_path.is_empty() {
        return Err(OperationContractError::InvalidShapeContractReference {
            field: "module_path",
            value: String::new(),
        });
    }
    for segment in &postcondition.module_path {
        validate_reference_name("module_path", segment)?;
    }
    validate_reference_name("contract_name", &postcondition.contract_name)
}

fn validate_reference_name(field: &'static str, value: &str) -> Result<(), OperationContractError> {
    if value.is_empty()
        || value.trim() != value
        || value == "."
        || value == ".."
        || value.contains(['\0', '/', '\\'])
    {
        return Err(OperationContractError::InvalidShapeContractReference {
            field,
            value: value.into(),
        });
    }
    Ok(())
}

fn referenced_inputs(construction: &OutputConstruction) -> Vec<(&'static str, u16)> {
    let mut inputs = Vec::new();
    match construction {
        OutputConstruction::FullWrite { shape } | OutputConstruction::Replace { shape } => {
            match shape {
                ShapeRule::Declared => {}
                ShapeRule::SameAsInput { input } => inputs.push(("shape.input", *input)),
                ShapeRule::TransposeOf { input } => inputs.push(("shape.input", *input)),
                ShapeRule::MatrixProduct { lhs, rhs } => {
                    inputs.push(("shape.lhs", *lhs));
                    inputs.push(("shape.rhs", *rhs));
                }
            }
        }
        OutputConstruction::ReadModifyWrite { base_input, .. } => {
            inputs.push(("base_input", *base_input));
        }
        OutputConstruction::Build { .. } => {}
    }
    inputs
}

fn validate_alias_policy(
    output: u32,
    output_schema: SchemaId,
    alias: AliasPolicy,
    inputs: &[ResolvedInputPort],
) -> Result<(), OperationContractError> {
    let input = match alias {
        AliasPolicy::NoAlias => return Ok(()),
        AliasPolicy::MayAlias { input } | AliasPolicy::InPlaceRequired { input } => input,
    };

    let input_schema = inputs.get(input as usize).map(|port| port.schema).ok_or(
        OperationContractError::InputOrdinalOutOfRange {
            field: "alias.input",
            input,
            inputs: inputs.len() as u32,
        },
    )?;

    if input_schema != output_schema {
        return Err(OperationContractError::AliasSchemaMismatch {
            output,
            input,
            input_schema,
            output_schema,
        });
    }

    Ok(())
}

fn validate_shape_rule(
    rule: ShapeRule,
    output: u32,
    contract: &DeclaredOperationContract,
    schemas: &SchemaTable,
) -> Result<(), OperationContractError> {
    match rule {
        ShapeRule::Declared => Ok(()),
        ShapeRule::SameAsInput { input } => {
            let input_schema = schemas.get(contract.inputs[input as usize].schema).ok_or(
                OperationContractError::UnknownSchema {
                    schema: contract.inputs[input as usize].schema,
                },
            )?;
            let output_schema = schemas
                .get(contract.outputs[output as usize].schema)
                .ok_or(OperationContractError::UnknownSchema {
                    schema: contract.outputs[output as usize].schema,
                })?;
            if !schema_bodies_have_same_shape(input_schema.body(), output_schema.body())
                && input_schema.dimension_parameters().is_empty()
                && output_schema.dimension_parameters().is_empty()
            {
                return Err(OperationContractError::SameShapeSchemaMismatch { input, output });
            }
            Ok(())
        }
        ShapeRule::TransposeOf { input } => {
            let input_schema = require_matrix("transpose.input", input, contract, schemas)?;
            let output_schema =
                require_output_matrix("transpose.output", output, contract, schemas)?;
            let SchemaBody::Matrix {
                element: input_element,
                dimensions: input_dimensions,
            } = input_schema.body()
            else {
                unreachable!();
            };
            let SchemaBody::Matrix {
                element: output_element,
                dimensions: output_dimensions,
            } = output_schema.body()
            else {
                unreachable!();
            };
            if input_dimensions.len() != 2
                || output_dimensions.len() != 2
                || input_element != output_element
                || !schema_local_dimensions_can_match(&output_dimensions[0], &input_dimensions[1])
                || !schema_local_dimensions_can_match(&output_dimensions[1], &input_dimensions[0])
            {
                return Err(OperationContractError::TransposeSchemaMismatch { input, output });
            }
            Ok(())
        }
        ShapeRule::MatrixProduct { lhs, rhs } => {
            let lhs_schema = require_matrix("matrix_product.lhs", lhs, contract, schemas)?;
            let rhs_schema = require_matrix("matrix_product.rhs", rhs, contract, schemas)?;
            let output_schema = schemas
                .get(contract.outputs[output as usize].schema)
                .ok_or(OperationContractError::UnknownSchema {
                    schema: contract.outputs[output as usize].schema,
                })?;
            let SchemaBody::Matrix {
                element: lhs_element,
                dimensions: lhs_dimensions,
            } = lhs_schema.body()
            else {
                unreachable!();
            };
            let SchemaBody::Matrix {
                element: rhs_element,
                dimensions: rhs_dimensions,
            } = rhs_schema.body()
            else {
                unreachable!();
            };
            let SchemaBody::Matrix {
                element: output_element,
                dimensions: output_dimensions,
            } = output_schema.body()
            else {
                return Err(OperationContractError::MatrixProductSchemaMismatch {
                    lhs,
                    rhs,
                    output,
                });
            };
            if lhs_dimensions.len() != 2
                || rhs_dimensions.len() != 2
                || output_dimensions.len() != 2
                || lhs_element != rhs_element
                || lhs_element != output_element
                || !schema_local_dimensions_can_match(&lhs_dimensions[1], &rhs_dimensions[0])
                || !schema_local_dimensions_can_match(&output_dimensions[0], &lhs_dimensions[0])
                || !schema_local_dimensions_can_match(&output_dimensions[1], &rhs_dimensions[1])
            {
                return Err(OperationContractError::MatrixProductSchemaMismatch {
                    lhs,
                    rhs,
                    output,
                });
            }
            Ok(())
        }
    }
}

/// Dimension-parameter identities are local to one canonical schema. A
/// parameter in an input schema therefore cannot be compared directly with a
/// parameter in an independently canonicalized output schema, even when the
/// operation shape rule relates their current extents. Closed dimensions must
/// still agree here; parameterized relations are checked against the concrete
/// `ShapeInstance`s when the operation is bound for execution.
fn schema_local_dimensions_can_match(
    left: &crate::DimensionExpr,
    right: &crate::DimensionExpr,
) -> bool {
    match (left, right) {
        (crate::DimensionExpr::Constant(left), crate::DimensionExpr::Constant(right)) => {
            left == right
        }
        _ => true,
    }
}

fn schema_bodies_have_same_shape(input: &SchemaBody, output: &SchemaBody) -> bool {
    match (input, output) {
        (
            SchemaBody::Matrix {
                dimensions: input_dimensions,
                ..
            },
            SchemaBody::Matrix {
                dimensions: output_dimensions,
                ..
            },
        ) => input_dimensions == output_dimensions,
        (
            SchemaBody::Table {
                columns: input_columns,
                rows: input_rows,
            },
            SchemaBody::Table {
                columns: output_columns,
                rows: output_rows,
            },
        ) => {
            input_rows == output_rows
                && input_columns.len() == output_columns.len()
                && input_columns
                    .iter()
                    .zip(output_columns.iter())
                    .all(|(input, output)| {
                        schema_bodies_have_same_shape(&input.schema, &output.schema)
                    })
        }
        (
            SchemaBody::Set {
                cardinality: input_cardinality,
                ..
            },
            SchemaBody::Set {
                cardinality: output_cardinality,
                ..
            },
        ) => input_cardinality == output_cardinality,
        (
            SchemaBody::Map {
                cardinality: input_cardinality,
                ..
            },
            SchemaBody::Map {
                cardinality: output_cardinality,
                ..
            },
        ) => input_cardinality == output_cardinality,
        (SchemaBody::Tuple(input), SchemaBody::Tuple(output)) => {
            input.len() == output.len()
                && input
                    .iter()
                    .zip(output.iter())
                    .all(|(input, output)| schema_bodies_have_same_shape(input, output))
        }
        (SchemaBody::Record(input), SchemaBody::Record(output)) => {
            input.len() == output.len()
                && input.iter().zip(output.iter()).all(|(input, output)| {
                    schema_bodies_have_same_shape(&input.schema, &output.schema)
                })
        }
        (
            SchemaBody::Enum {
                variants: input_variants,
                ..
            },
            SchemaBody::Enum {
                variants: output_variants,
                ..
            },
        ) => {
            input_variants.len() == output_variants.len()
                && input_variants
                    .iter()
                    .zip(output_variants.iter())
                    .all(|(input, output)| match (&input.payload, &output.payload) {
                        (None, None) => true,
                        (Some(input), Some(output)) => schema_bodies_have_same_shape(input, output),
                        (None, Some(_)) | (Some(_), None) => false,
                    })
        }
        (SchemaBody::Option(input), SchemaBody::Option(output)) => {
            schema_bodies_have_same_shape(input, output)
        }
        (
            SchemaBody::Matrix { .. }
            | SchemaBody::Table { .. }
            | SchemaBody::Set { .. }
            | SchemaBody::Map { .. }
            | SchemaBody::Tuple(_)
            | SchemaBody::Record(_)
            | SchemaBody::Enum { .. }
            | SchemaBody::Option(_),
            _,
        )
        | (
            _,
            SchemaBody::Matrix { .. }
            | SchemaBody::Table { .. }
            | SchemaBody::Set { .. }
            | SchemaBody::Map { .. }
            | SchemaBody::Tuple(_)
            | SchemaBody::Record(_)
            | SchemaBody::Enum { .. }
            | SchemaBody::Option(_),
        ) => false,
        _ => true,
    }
}

fn require_matrix<'a>(
    field: &'static str,
    input: u16,
    contract: &'a DeclaredOperationContract,
    schemas: &'a SchemaTable,
) -> Result<&'a crate::Schema, OperationContractError> {
    let schema_id = contract
        .inputs
        .get(input as usize)
        .map(|port| port.schema)
        .ok_or(OperationContractError::InputOrdinalOutOfRange {
            field,
            input,
            inputs: contract.inputs.len() as u32,
        })?;
    let schema = schemas
        .get(schema_id)
        .ok_or(OperationContractError::UnknownSchema { schema: schema_id })?;
    if !matches!(schema.body(), SchemaBody::Matrix { .. }) {
        return Err(OperationContractError::MatrixShapeRuleRequiresMatrix { field, input });
    }
    Ok(schema)
}

fn require_output_matrix<'a>(
    field: &'static str,
    output: u32,
    contract: &'a DeclaredOperationContract,
    schemas: &'a SchemaTable,
) -> Result<&'a crate::Schema, OperationContractError> {
    let schema_id = contract
        .outputs
        .get(output as usize)
        .map(|port| port.schema)
        .ok_or(OperationContractError::PortCountMismatch {
            direction: PortDirection::Output,
            expected: output as u64 + 1,
            actual: contract.outputs.len() as u64,
        })?;
    let schema = schemas
        .get(schema_id)
        .ok_or(OperationContractError::UnknownSchema { schema: schema_id })?;
    if !matches!(schema.body(), SchemaBody::Matrix { .. }) {
        return Err(OperationContractError::MatrixShapeRuleRequiresMatrix {
            field,
            input: output as u16,
        });
    }
    Ok(schema)
}

#[cfg(feature = "no_std")]
use alloc::vec::Vec;
#[cfg(not(feature = "no_std"))]
use std::vec::Vec;
