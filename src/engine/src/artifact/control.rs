//! Lexical, engine-owned executable control. Source text is never an operand.

use mech_core::{ConstantId, OperationContractId, SchemaId};

use super::OperationReference;

/// Dense preorder identity within one root control declaration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ControlBlockId(pub u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MatchPatternValue<C = ConstantId> {
    Literal(C),
    Binding(u32),
    /// Ordinal in the enclosing node's input bindings. Activation scopes use
    /// this for pattern expressions sampled on the triggering turn.
    Input(u16),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MatchPattern<C = ConstantId, S = SchemaId> {
    Literal(C),
    Wildcard,
    /// Makes the scrutinee available to explicitly declared block parameters.
    Bind,
    /// A schema-directed structural pattern. Bindings are dense within one arm;
    /// repeated names refer back to an earlier binding through `Equal`.
    Structural(super::CollectionPattern<S, MatchPatternValue<C>>),
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
    PatternBinding(u32),
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

/// Each local has one writer and one exactly typed owned result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControlOperation<C = OperationContractId> {
    pub node: u32,
    pub body: ControlOperationBody<C>,
    pub inputs: Box<[ControlValue]>,
    pub schema: SchemaId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ControlOperationBody<C = OperationContractId> {
    Operation {
        operation: OperationReference,
        contract: C,
    },
    Match(MatchDeclaration<C>),
    Comprehension(super::ComprehensionDeclaration<C>),
    /// Invoke the enclosing function match with the operation's single input.
    ///
    /// This is a lexical back-edge, not a graph edge and not a source-level
    /// callable lookup. Activation binds it to the enclosing match and the
    /// resident executor admits a bounded call frame before following it.
    Recur,
    /// Suspend the enclosing declared FSM with the operation's single state
    /// input. Suspension publishes no result in the current turn; resident
    /// execution resumes the same lexical match on a later turn.
    Suspend,
    /// Stage an FSM output while control continues to a later transition in
    /// the same arm. Publication commits atomically with any suspension.
    Publish,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControlBlock<C = OperationContractId> {
    pub id: ControlBlockId,
    pub parameters: Box<[ControlParameter]>,
    pub operations: Box<[ControlOperation<C>]>,
    pub yield_value: ControlValue,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControlMatchArm<C = OperationContractId> {
    pub pattern: MatchPattern,
    pub guard: Option<ControlBlock<C>>,
    pub body: ControlBlock<C>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchDeclaration<C = OperationContractId> {
    /// Ordinal in the enclosing node's input bindings; evaluated once.
    pub scrutinee: u16,
    /// Function dispatch may deliberately be partial and fails the call when
    /// no ordered arm matches. Ordinary match expressions remain exhaustive.
    pub partial: bool,
    pub captures: Box<[ControlCapture]>,
    pub arms: Box<[ControlMatchArm<C>]>,
}

fn component_schema(
    parent: &mech_core::Schema,
    body: &mech_core::SchemaBody,
) -> Option<mech_core::Schema> {
    mech_core::SchemaDraft {
        body: body.clone(),
        dimension_parameters: parent
            .dimension_parameters()
            .iter()
            .enumerate()
            .map(|(id, parameter)| mech_core::DimensionParameterDeclaration {
                id: mech_core::DimensionParameterId::new(id as u32),
                origin: mech_core::DimensionParameterOrigin::Explicit,
                lifetime: parameter.lifetime(),
                lower_bound: parameter.lower_bound().clone(),
                upper_bound: parameter.upper_bound().cloned(),
            })
            .collect(),
    }
    .finalize()
    .ok()
}

fn validate_structural_pattern(
    draft: &super::ProgramArtifactDraft,
    pattern: &super::CollectionPattern<SchemaId, MatchPatternValue>,
    expected: &mech_core::Schema,
    bindings: &mut Vec<SchemaId>,
    inputs: &[SchemaId],
) -> Option<()> {
    use mech_core::SchemaBody;
    let compatible = |schema: &mech_core::Schema| {
        matches!(expected.body(), SchemaBody::Dynamic) || schema == expected
    };
    match pattern {
        super::CollectionPattern::Wildcard => {}
        super::CollectionPattern::Bind { local, schema } => {
            let definition = draft.schemas.get(*schema)?;
            if *local as usize != bindings.len() || !compatible(definition) {
                return None;
            }
            bindings.push(*schema);
        }
        super::CollectionPattern::Equal(MatchPatternValue::Literal(constant)) => {
            let schema = draft
                .schemas
                .get(draft.constants.get(*constant)?.schema())?;
            if !compatible(schema) {
                return None;
            }
        }
        super::CollectionPattern::Equal(MatchPatternValue::Binding(local)) => {
            let schema = draft.schemas.get(*bindings.get(*local as usize)?)?;
            if !compatible(schema) {
                return None;
            }
        }
        super::CollectionPattern::Equal(MatchPatternValue::Input(input)) => {
            let schema = draft.schemas.get(*inputs.get(*input as usize)?)?;
            if !compatible(schema) {
                return None;
            }
        }
        super::CollectionPattern::Enum { ordinal, payload } => {
            let payload_schema = match expected.body() {
                SchemaBody::Enum { variants, .. } => {
                    variants.get(*ordinal as usize)?.payload.as_ref()
                }
                SchemaBody::Dynamic => None,
                _ => return None,
            };
            match (payload_schema, payload) {
                (Some(schema), Some(pattern)) => validate_structural_pattern(
                    draft,
                    pattern,
                    &component_schema(expected, schema)?,
                    bindings,
                    inputs,
                )?,
                (None, None) if matches!(expected.body(), SchemaBody::Enum { .. }) => {}
                (_, Some(pattern)) if matches!(expected.body(), SchemaBody::Dynamic) => {
                    validate_structural_pattern(
                        draft,
                        pattern,
                        &component_schema(expected, &SchemaBody::Dynamic)?,
                        bindings,
                        inputs,
                    )?;
                }
                _ => return None,
            }
        }
        super::CollectionPattern::Tuple(items) => {
            let fields = match expected.body() {
                SchemaBody::Tuple(fields) if fields.len() == items.len() => Some(fields),
                SchemaBody::Dynamic => None,
                _ => return None,
            };
            for (index, item) in items.iter().enumerate() {
                validate_structural_pattern(
                    draft,
                    item,
                    &component_schema(
                        expected,
                        fields.map_or(&SchemaBody::Dynamic, |fields| &fields[index]),
                    )?,
                    bindings,
                    inputs,
                )?;
            }
        }
        super::CollectionPattern::Array {
            prefix,
            rest,
            suffix,
        } => {
            let element = match expected.body() {
                SchemaBody::Matrix { element, .. } => element.as_ref(),
                SchemaBody::Dynamic => expected.body(),
                _ => return None,
            };
            for item in prefix {
                validate_structural_pattern(
                    draft,
                    item,
                    &component_schema(expected, element)?,
                    bindings,
                    inputs,
                )?;
            }
            if let Some(rest) = rest {
                validate_structural_pattern(
                    draft,
                    rest,
                    &super::comprehension::array_rest_schema(expected, element)?,
                    bindings,
                    inputs,
                )?;
            }
            for item in suffix {
                validate_structural_pattern(
                    draft,
                    item,
                    &component_schema(expected, element)?,
                    bindings,
                    inputs,
                )?;
            }
        }
    }
    Some(())
}

pub(super) fn validate_match(
    draft: &super::ProgramArtifactDraft,
    node: mech_core::NodeId,
    declaration: &MatchDeclaration,
    inputs: &[SchemaId],
    output: SchemaId,
) -> Result<(), super::ArtifactBuildError> {
    let input = *inputs.get(declaration.scrutinee as usize).ok_or(
        super::ArtifactBuildError::InvalidControl {
            node,
            reason: "unknown scrutinee input",
        },
    )?;
    validate_match_inner(
        draft,
        node,
        declaration,
        inputs,
        output,
        Some((input, output)),
        &mut 0,
    )
}

pub(super) fn validate_activation(
    draft: &super::ProgramArtifactDraft,
    node: mech_core::NodeId,
    declaration: &MatchDeclaration,
    inputs: &[SchemaId],
    output: SchemaId,
) -> Result<(), super::ArtifactBuildError> {
    validate_match_inner(draft, node, declaration, inputs, output, None, &mut 0)
}

fn constant_dimension(expression: &mech_core::DimensionExpr) -> Option<u64> {
    match expression {
        mech_core::DimensionExpr::Constant(value) => Some(*value),
        mech_core::DimensionExpr::Add(items) => items.iter().try_fold(0_u64, |total, item| {
            total.checked_add(constant_dimension(item)?)
        }),
        mech_core::DimensionExpr::Multiply(items) => items.iter().try_fold(1_u64, |total, item| {
            total.checked_mul(constant_dimension(item)?)
        }),
        mech_core::DimensionExpr::Hole
        | mech_core::DimensionExpr::Parameter(_)
        | mech_core::DimensionExpr::Min(_)
        | mech_core::DimensionExpr::Max(_) => None,
    }
}

fn structural_pattern_is_irrefutable(
    pattern: &super::CollectionPattern<SchemaId, MatchPatternValue>,
    expected: &mech_core::SchemaBody,
) -> bool {
    use mech_core::SchemaBody;

    match pattern {
        super::CollectionPattern::Wildcard | super::CollectionPattern::Bind { .. } => true,
        super::CollectionPattern::Equal(_) => false,
        super::CollectionPattern::Enum { ordinal, payload } => {
            let SchemaBody::Enum { variants, .. } = expected else {
                return false;
            };
            if variants.len() != 1 || *ordinal != 0 {
                return false;
            }
            match (&variants[0].payload, payload.as_deref()) {
                (None, None) => true,
                (Some(schema), Some(pattern)) => structural_pattern_is_irrefutable(pattern, schema),
                _ => false,
            }
        }
        super::CollectionPattern::Tuple(items) => {
            let SchemaBody::Tuple(fields) = expected else {
                return false;
            };
            fields.len() == items.len()
                && items
                    .iter()
                    .zip(fields)
                    .all(|(pattern, schema)| structural_pattern_is_irrefutable(pattern, schema))
        }
        super::CollectionPattern::Array {
            prefix,
            rest,
            suffix,
        } => {
            let SchemaBody::Matrix {
                element,
                dimensions,
            } = expected
            else {
                return false;
            };
            if !prefix
                .iter()
                .chain(suffix.iter())
                .all(|pattern| structural_pattern_is_irrefutable(pattern, element))
            {
                return false;
            }
            let Some(length) = dimensions.iter().try_fold(1_u64, |total, dimension| {
                total.checked_mul(constant_dimension(dimension)?)
            }) else {
                return false;
            };
            let fixed = (prefix.len() + suffix.len()) as u64;
            match rest.as_deref() {
                None => length == fixed,
                Some(super::CollectionPattern::Wildcard)
                | Some(super::CollectionPattern::Bind { .. }) => length >= fixed,
                Some(_) => false,
            }
        }
    }
}

pub(super) fn validate_match_inner(
    draft: &super::ProgramArtifactDraft,
    node: mech_core::NodeId,
    declaration: &MatchDeclaration,
    inputs: &[SchemaId],
    output: SchemaId,
    lexical_signature: Option<(SchemaId, SchemaId)>,
    next_block: &mut u32,
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
    let closed_value = |schema| {
        draft
            .schemas
            .get(schema)
            .is_some_and(is_control_value_schema)
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
    if !closed_value(output)
        || (!scalar(scrutinee)
            && declaration
                .arms
                .iter()
                .any(|arm| matches!(&arm.pattern, MatchPattern::Literal(_))))
    {
        return Err(invalid(
            "match requires a closed result and scalar literal-pattern scrutinee",
        ));
    }
    if !closed_value(scrutinee)
        && declaration.arms.iter().any(|arm| {
            matches!(
                &arm.pattern,
                MatchPattern::Bind | MatchPattern::Structural(_)
            )
        })
    {
        return Err(invalid("match bindings require a closed value schema"));
    }
    let mut used_inputs = std::collections::BTreeSet::from([declaration.scrutinee]);
    let mut captured_inputs = std::collections::BTreeSet::new();
    for capture in &declaration.captures {
        if inputs.get(capture.input as usize) != Some(&capture.schema)
            || !closed_value(capture.schema)
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
    let mut coverage = [false; 2];
    let mut enum_coverage = match draft.schemas.get(scrutinee).map(|schema| schema.body()) {
        Some(mech_core::SchemaBody::Enum { variants, .. }) => Some(vec![false; variants.len()]),
        _ => None,
    };
    for arm in &declaration.arms {
        let mut pattern_bindings = Vec::new();
        if let MatchPattern::Literal(constant) = &arm.pattern {
            let value = draft
                .constants
                .get(*constant)
                .ok_or_else(|| invalid("unknown literal pattern constant"))?;
            if value.schema() != scrutinee {
                return Err(invalid("literal pattern schema differs from scrutinee"));
            }
        }
        if let MatchPattern::Structural(pattern) = &arm.pattern {
            let schema = draft
                .schemas
                .get(scrutinee)
                .ok_or_else(|| invalid("unknown structural scrutinee schema"))?;
            validate_structural_pattern(draft, pattern, schema, &mut pattern_bindings, inputs)
                .ok_or_else(|| invalid("invalid structural match pattern"))?;
        }
        for (block, is_guard) in arm
            .guard
            .iter()
            .map(|block| (block, true))
            .chain(core::iter::once((&arm.body, false)))
        {
            if block.id.0 != *next_block {
                return Err(invalid("noncanonical block identity"));
            }
            *next_block = next_block
                .checked_add(1)
                .ok_or_else(|| invalid("block count overflow"))?;
            for parameter in &block.parameters {
                let expected = match parameter.source {
                    ControlParameterSource::Scrutinee
                        if matches!(&arm.pattern, MatchPattern::Bind) =>
                    {
                        scrutinee
                    }
                    ControlParameterSource::Scrutinee => {
                        return Err(invalid("unbound scrutinee parameter"));
                    }
                    ControlParameterSource::PatternBinding(local) => *pattern_bindings
                        .get(local as usize)
                        .ok_or_else(|| invalid("unknown pattern binding parameter"))?,
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
                if operation.node as usize != index || !closed_value(operation.schema) {
                    return Err(invalid(
                        "invalid local identity or non-closed operation result",
                    ));
                }
                let ControlOperationBody::Operation {
                    operation: reference,
                    contract: contract_id,
                } = &operation.body
                else {
                    let inputs = operation
                        .inputs
                        .iter()
                        .map(|value| value_schema(*value, index))
                        .collect::<Result<Vec<_>, _>>()?;
                    match &operation.body {
                        ControlOperationBody::Match(nested) => validate_match_inner(
                            draft,
                            node,
                            nested,
                            &inputs,
                            operation.schema,
                            lexical_signature,
                            next_block,
                        )?,
                        ControlOperationBody::Comprehension(nested) => {
                            super::comprehension::validate_comprehension_inner(
                                draft,
                                node,
                                nested,
                                &inputs,
                                operation.schema,
                                next_block,
                            )?
                        }
                        ControlOperationBody::Recur | ControlOperationBody::Suspend => {
                            let Some((lexical_input, lexical_output)) = lexical_signature else {
                                return Err(invalid(
                                    "recursive or suspended control requires an enclosing lexical signature",
                                ));
                            };
                            if inputs.as_slice() != [lexical_input]
                                || operation.schema != lexical_output
                            {
                                return Err(invalid(
                                    "recursive or suspended control must preserve the enclosing input and output schemas",
                                ));
                            }
                        }
                        ControlOperationBody::Publish => {
                            let Some((_, lexical_output)) = lexical_signature else {
                                return Err(invalid(
                                    "FSM publication requires an enclosing lexical signature",
                                ));
                            };
                            if inputs.as_slice() != [lexical_output]
                                || operation.schema != lexical_output
                            {
                                return Err(invalid(
                                    "FSM publication must preserve the enclosing output schema",
                                ));
                            }
                        }
                        ControlOperationBody::Operation { .. } => unreachable!(),
                    }
                    continue;
                };
                super::validation::validate_operation(reference)?;
                let Some(ResolvedOperationContract::Declared(contract)) =
                    draft.contracts.get(*contract_id)
                else {
                    return Err(super::ArtifactBuildError::UnknownOperationContract {
                        contract: *contract_id,
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
                    let live_pattern_parameter = matches!(
                        input,
                        ControlValue::Parameter { block: owner, ordinal }
                            if *owner == block.id
                                && block.parameters.get(*ordinal as usize).is_some_and(|parameter| {
                                    matches!(
                                        parameter.source,
                                        ControlParameterSource::PatternBinding(_)
                                    )
                                })
                    );
                    if port.access != AccessMode::Read
                        || port.delivery != DeliveryMode::Signal
                        || value_schema(*input, index)? != port.schema
                        || (!closed_value(port.schema) && !live_pattern_parameter)
                    {
                        return Err(invalid("block operation input contract mismatch"));
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
            match &arm.pattern {
                MatchPattern::Literal(constant) => {
                    match draft.constants.get(*constant).unwrap().data() {
                        mech_core::ValueData::Bool(value) => coverage[*value as usize] = true,
                        mech_core::ValueData::Enum(value) => {
                            if let Some(variants) = &mut enum_coverage
                                && let Some(covered) = variants.get_mut(value.ordinal() as usize)
                            {
                                *covered = true;
                            }
                        }
                        _ => {}
                    }
                }
                MatchPattern::Wildcard | MatchPattern::Bind => {
                    coverage = [true; 2];
                    if let Some(variants) = &mut enum_coverage {
                        variants.fill(true);
                    }
                }
                MatchPattern::Structural(pattern)
                    if structural_pattern_is_irrefutable(
                        pattern,
                        draft
                            .schemas
                            .get(scrutinee)
                            .expect("validated scrutinee schema")
                            .body(),
                    ) =>
                {
                    coverage = [true; 2];
                    if let Some(variants) = &mut enum_coverage {
                        variants.fill(true);
                    }
                }
                MatchPattern::Structural(super::CollectionPattern::Enum { ordinal, payload }) => {
                    if let Some(variants) = &mut enum_coverage
                        && let Some(covered) = variants.get_mut(*ordinal as usize)
                    {
                        let payload_irrefutable = match draft
                            .schemas
                            .get(scrutinee)
                            .expect("validated scrutinee schema")
                            .body()
                        {
                            SchemaBody::Enum {
                                variants: declarations,
                                ..
                            } => {
                                declarations.get(*ordinal as usize).is_some_and(|variant| {
                                    match (&variant.payload, payload.as_deref()) {
                                        (None, None) => true,
                                        (Some(schema), Some(pattern)) => {
                                            structural_pattern_is_irrefutable(pattern, schema)
                                        }
                                        _ => false,
                                    }
                                })
                            }
                            _ => false,
                        };
                        *covered |= payload_irrefutable;
                    }
                }
                MatchPattern::Structural(_) => {}
            }
        }
    }
    let exhaustive = enum_coverage
        .as_ref()
        .map_or(coverage == [true; 2], |variants| {
            variants.iter().all(|covered| *covered)
        });
    if !declaration.partial && !exhaustive {
        return Err(invalid("non-exhaustive match"));
    }
    Ok(())
}

/// Structural admission limits shared by source finalization and wire defaults.
pub const MAX_CONTROL_ARMS: usize = 4_096;
pub const MAX_CONTROL_BLOCKS: usize = 8_192;
pub const MAX_CONTROL_OPERATIONS: usize = 65_536;
pub const MAX_CONTROL_OPERANDS: usize = 262_144;
/// Scratch ordinals are encoded as `u16`; operations and pattern bindings
/// share that one namespace during resident activation.
pub const MAX_CONTROL_LOCALS: usize = u16::MAX as usize + 1;
/// Keeps validation, encoding and execution recursion within a bounded stack.
pub const MAX_CONTROL_DEPTH: usize = 8;

enum ControlRef<'a, C> {
    Match(&'a MatchDeclaration<C>),
    Comprehension(&'a super::ComprehensionDeclaration<C>),
}

fn control_counts<C>(root: ControlRef<'_, C>) -> Option<[usize; 5]> {
    let mut counts = [0usize; 5];
    let limits = [
        MAX_CONTROL_ARMS,
        MAX_CONTROL_BLOCKS,
        MAX_CONTROL_OPERATIONS,
        MAX_CONTROL_OPERANDS,
        MAX_CONTROL_LOCALS,
    ];
    let mut pending = vec![(root, 1usize)];
    while let Some((control, depth)) = pending.pop() {
        if depth > MAX_CONTROL_DEPTH {
            return None;
        }
        let mut add = |index: usize, amount: usize| -> Option<()> {
            counts[index] = counts[index].checked_add(amount)?;
            (counts[index] <= limits[index]).then_some(())
        };
        match control {
            ControlRef::Match(control) => {
                add(0, control.arms.len())?;
                add(3, control.captures.len())?;
                for arm in &control.arms {
                    if let MatchPattern::Structural(pattern) = &arm.pattern {
                        let metrics = super::comprehension::pattern_metrics(pattern)?;
                        add(3, metrics.nodes)?;
                        add(4, metrics.bindings)?;
                    }
                    for block in arm.guard.iter().chain(core::iter::once(&arm.body)) {
                        add(1, 1)?;
                        add(2, block.operations.len())?;
                        add(3, block.parameters.len())?;
                        add(4, block.operations.len())?;
                        for operation in &block.operations {
                            add(3, operation.inputs.len())?;
                            match &operation.body {
                                ControlOperationBody::Match(nested) => {
                                    pending.push((ControlRef::Match(nested), depth.checked_add(1)?))
                                }
                                ControlOperationBody::Comprehension(nested) => pending.push((
                                    ControlRef::Comprehension(nested),
                                    depth.checked_add(1)?,
                                )),
                                ControlOperationBody::Operation { .. }
                                | ControlOperationBody::Recur
                                | ControlOperationBody::Suspend
                                | ControlOperationBody::Publish => {}
                            }
                        }
                    }
                }
            }
            ControlRef::Comprehension(control) => {
                add(1, 1)?;
                add(2, control.steps.len())?;
                for step in &control.steps {
                    match step {
                        super::ComprehensionStep::Generator { pattern, .. } => {
                            let metrics = super::comprehension::pattern_metrics(pattern)?;
                            add(3, metrics.nodes)?;
                            add(4, metrics.bindings)?;
                        }
                        super::ComprehensionStep::Filter(_) => add(3, 1)?,
                        super::ComprehensionStep::Operation(operation) => {
                            add(3, operation.inputs.len())?;
                            add(4, 1)?;
                            match &operation.body {
                                ControlOperationBody::Match(nested) => {
                                    pending.push((ControlRef::Match(nested), depth.checked_add(1)?))
                                }
                                ControlOperationBody::Comprehension(nested) => pending.push((
                                    ControlRef::Comprehension(nested),
                                    depth.checked_add(1)?,
                                )),
                                ControlOperationBody::Operation { .. }
                                | ControlOperationBody::Recur
                                | ControlOperationBody::Suspend
                                | ControlOperationBody::Publish => {}
                            }
                        }
                    }
                }
            }
        }
    }
    Some(counts)
}

fn match_counts<C>(root: &MatchDeclaration<C>) -> Option<[usize; 5]> {
    control_counts(ControlRef::Match(root))
}

pub(super) fn validate_control_counts(
    draft: &super::ProgramArtifactDraft,
) -> Result<(), super::ArtifactBuildError> {
    let mut counts = [0usize; 4];
    for node in &draft.nodes {
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
        if let super::ExecutableNodeBody::Comprehension(control) = &node.body {
            let control_counts =
                control_counts(ControlRef::Comprehension(control)).ok_or_else(invalid)?;
            for (index, count) in control_counts[..4].iter().copied().enumerate() {
                add(index, count)?;
            }
            continue;
        }
        if let super::ExecutableNodeBody::Fsm(control) = &node.body {
            add(2, control.stages.len())?;
            add(3, control.arguments.len())?;
            for stage in &control.stages {
                let count = super::fsm::value_count(&stage.value).ok_or(
                    super::ArtifactBuildError::InvalidControl {
                        node: node.node,
                        reason: "FSM value admission limit",
                    },
                )?;
                add(3, count)?;
            }
            continue;
        }
        let control = match &node.body {
            super::ExecutableNodeBody::Match(control)
            | super::ExecutableNodeBody::Activation(control) => control,
            _ => continue,
        };
        let control_counts = match_counts(control).ok_or_else(invalid)?;
        for (index, count) in control_counts[..4].iter().copied().enumerate() {
            add(index, count)?;
        }
    }
    Ok(())
}

impl<C> MatchDeclaration<C> {
    #[cfg(feature = "resident-artifact")]
    pub(crate) fn contains_suspend(&self) -> bool {
        self.arms.iter().any(|arm| {
            arm.guard
                .iter()
                .chain(core::iter::once(&arm.body))
                .flat_map(|block| &block.operations)
                .any(|operation| match &operation.body {
                    ControlOperationBody::Suspend => true,
                    ControlOperationBody::Match(nested) => nested.contains_suspend(),
                    ControlOperationBody::Comprehension(_)
                    | ControlOperationBody::Operation { .. }
                    | ControlOperationBody::Recur
                    | ControlOperationBody::Publish => false,
                })
        })
    }

    pub(super) fn validate_depth(
        &self,
        node: mech_core::NodeId,
    ) -> Result<(), super::ArtifactBuildError> {
        let mut pending = vec![(self, 1)];
        while let Some((control, depth)) = pending.pop() {
            if depth > MAX_CONTROL_DEPTH {
                return Err(super::ArtifactBuildError::InvalidControl {
                    node,
                    reason: "control graph nesting limit",
                });
            }
            for block in control
                .arms
                .iter()
                .flat_map(|arm| arm.guard.iter().chain(core::iter::once(&arm.body)))
            {
                for operation in &block.operations {
                    if let ControlOperationBody::Match(nested) = &operation.body {
                        pending.push((nested, depth + 1));
                    }
                }
            }
        }
        Ok(())
    }

    pub(super) fn map_contracts<D, E>(
        &self,
        mut map: impl FnMut(&ControlBlock<C>, &ControlOperation<C>, &C) -> Result<D, E>,
    ) -> Result<MatchDeclaration<D>, E> {
        self.map_contracts_inner(&mut map)
    }

    pub(super) fn map_contracts_inner<D, E>(
        &self,
        map: &mut dyn FnMut(&ControlBlock<C>, &ControlOperation<C>, &C) -> Result<D, E>,
    ) -> Result<MatchDeclaration<D>, E> {
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
                            body: match &operation.body {
                                ControlOperationBody::Operation {
                                    operation: reference,
                                    contract,
                                } => ControlOperationBody::Operation {
                                    operation: reference.clone(),
                                    contract: map(block, operation, contract)?,
                                },
                                ControlOperationBody::Match(nested) => {
                                    ControlOperationBody::Match(nested.map_contracts_inner(map)?)
                                }
                                ControlOperationBody::Comprehension(nested) => {
                                    ControlOperationBody::Comprehension(
                                        nested.map_contracts_inner(&mut |_, contract| {
                                            map(block, operation, contract)
                                        })?,
                                    )
                                }
                                ControlOperationBody::Recur => ControlOperationBody::Recur,
                                ControlOperationBody::Suspend => ControlOperationBody::Suspend,
                                ControlOperationBody::Publish => ControlOperationBody::Publish,
                            },
                            inputs: operation.inputs.clone(),
                            schema: operation.schema,
                        })
                    })
                    .collect::<Result<Box<[_]>, E>>()?,
            })
        };
        Ok(MatchDeclaration {
            scrutinee: self.scrutinee,
            partial: self.partial,
            captures: self.captures.clone(),
            arms: self
                .arms
                .iter()
                .map(|arm| {
                    Ok(ControlMatchArm {
                        pattern: arm.pattern.clone(),
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

/// Control values share ordinary schema and construction authorities.
/// Per-turn variable shapes need explicit control shape bindings before admission.
pub(crate) fn is_control_value_schema(schema: &mech_core::Schema) -> bool {
    schema.dimension_parameters().is_empty()
        && !matches!(schema.body(), mech_core::SchemaBody::Dynamic)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combined_match_operations_and_bindings_share_the_u16_scratch_limit() {
        let pattern = super::super::CollectionPattern::Array {
            prefix: (0..=MAX_CONTROL_LOCALS)
                .map(|local| super::super::CollectionPattern::Bind {
                    local: u32::try_from(local).unwrap(),
                    schema: SchemaId::new(0),
                })
                .collect(),
            rest: None,
            suffix: Box::new([]),
        };
        let control = MatchDeclaration::<OperationContractId> {
            scrutinee: 0,
            partial: false,
            captures: Box::new([]),
            arms: vec![ControlMatchArm {
                pattern: MatchPattern::Structural(pattern),
                guard: None,
                body: ControlBlock {
                    id: ControlBlockId(0),
                    parameters: Box::new([]),
                    operations: Box::new([]),
                    yield_value: ControlValue::Constant(mech_core::ConstantId::new(0)),
                },
            }]
            .into_boxed_slice(),
        };
        assert_eq!(match_counts(&control), None);
    }

    #[test]
    fn overdepth_control_is_rejected_by_the_bounded_count_walk() {
        fn leaf() -> MatchDeclaration<OperationContractId> {
            MatchDeclaration {
                scrutinee: 0,
                partial: false,
                captures: Box::new([]),
                arms: vec![ControlMatchArm {
                    pattern: MatchPattern::Wildcard,
                    guard: None,
                    body: ControlBlock {
                        id: ControlBlockId(0),
                        parameters: Box::new([]),
                        operations: Box::new([]),
                        yield_value: ControlValue::Constant(mech_core::ConstantId::new(0)),
                    },
                }]
                .into_boxed_slice(),
            }
        }

        let mut control = leaf();
        for depth in 1..=MAX_CONTROL_DEPTH {
            control = MatchDeclaration {
                scrutinee: 0,
                partial: false,
                captures: Box::new([]),
                arms: vec![ControlMatchArm {
                    pattern: MatchPattern::Wildcard,
                    guard: None,
                    body: ControlBlock {
                        id: ControlBlockId(depth as u32),
                        parameters: Box::new([]),
                        operations: vec![ControlOperation {
                            node: 0,
                            body: ControlOperationBody::Match(control),
                            inputs: Box::new([]),
                            schema: SchemaId::new(0),
                        }]
                        .into_boxed_slice(),
                        yield_value: ControlValue::Constant(mech_core::ConstantId::new(0)),
                    },
                }]
                .into_boxed_slice(),
            };
        }
        assert_eq!(match_counts(&control), None);
    }
}
