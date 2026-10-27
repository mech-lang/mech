//! Lexical, engine-owned executable control. Source text is never an operand.

use mech_core::{ConstantId, OperationContractId, SchemaId};

use super::OperationReference;
use crate::structural_coverage::{
    StructuralCoveragePattern, StructuralCoverageSpace, StructuralPatternCoverage,
};

/// Dense preorder identity within one root control declaration.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
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
    /// Retain lexical arguments and derived captures across suspension.
    /// Direct external inputs remain live when the FSM resumes. Counted
    /// application requires this flag for its once-per-invocation snapshots.
    pub freeze_on_suspend: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControlParameterSource {
    Scrutinee,
    PatternBinding(u32),
    Capture(u16),
    /// Actual privately owned value at the current counted-application visit.
    IterationCarrier,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ControlParameter {
    pub source: ControlParameterSource,
    pub schema: SchemaId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
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
    Recur(u8),

    /// Check the single Bool operand at this exact lexical visit. False fails
    /// the candidate; true produces the same Bool. This is engine-owned
    /// control, not a document constraint or a selected library implementation.
    Assert,

    /// Suspend the enclosing declared FSM with the operation's single state
    /// input. Suspension publishes no result in the current turn; resident
    /// execution resumes the same lexical match on a later turn.
    Suspend,
    /// Stage an FSM output while control continues to a later transition in
    /// the same arm. Publication commits atomically with any suspension.
    Publish,
    Iteration(super::IterationDeclaration<C>),
}

pub(super) fn is_assertion_signature(
    schemas: &mech_core::SchemaTable,
    inputs: &[SchemaId],
    output: SchemaId,
) -> bool {
    inputs.len() == 1
        && inputs.iter().chain(core::iter::once(&output)).all(|id| {
            schemas
                .get(*id)
                .is_some_and(|schema| schema.body() == &mech_core::SchemaBody::Bool)
        })
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

pub(super) fn structurally_irrefutable_shape<S, V>(
    pattern: &super::CollectionPattern<S, V>,
) -> bool {
    match pattern {
        super::CollectionPattern::Wildcard | super::CollectionPattern::Bind { .. } => true,
        super::CollectionPattern::Tuple(items) => items.iter().all(structurally_irrefutable_shape),
        super::CollectionPattern::Array {
            prefix,
            rest: Some(rest),
            suffix,
        } if prefix.is_empty() && suffix.is_empty() => structurally_irrefutable_shape(rest),
        super::CollectionPattern::Equal(_)
        | super::CollectionPattern::Enum { .. }
        | super::CollectionPattern::Array { .. } => false,
    }
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
    allow_sampled_input: bool,
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
            if !allow_sampled_input {
                return None;
            }
            let schema = draft.schemas.get(*inputs.get(*input as usize)?)?;
            if !compatible(schema) {
                return None;
            }
        }
        super::CollectionPattern::Enum { ordinal, payload } => {
            if matches!(expected.body(), SchemaBody::Dynamic) {
                if let Some(pattern) = payload {
                    validate_structural_pattern(
                        draft,
                        pattern,
                        &component_schema(expected, &SchemaBody::Dynamic)?,
                        bindings,
                        inputs,
                        allow_sampled_input,
                    )?;
                }
                return Some(());
            }
            let enum_expected = match expected.body() {
                SchemaBody::Enum { .. } => expected.clone(),
                SchemaBody::Option(body) if matches!(body.as_ref(), SchemaBody::Enum { .. }) => {
                    component_schema(expected, body)?
                }
                _ => return None,
            };
            let SchemaBody::Enum { variants, .. } = enum_expected.body() else {
                return None;
            };
            let payload_schema = variants.get(*ordinal as usize)?.payload.as_ref();
            match (payload_schema, payload) {
                (Some(schema), Some(pattern)) => validate_structural_pattern(
                    draft,
                    pattern,
                    &component_schema(&enum_expected, schema)?,
                    bindings,
                    inputs,
                    allow_sampled_input,
                )?,
                (None, None) => {}
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
                    allow_sampled_input,
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
                    allow_sampled_input,
                )?;
            }
            if let Some(rest) = rest {
                validate_structural_pattern(
                    draft,
                    rest,
                    &super::comprehension::array_rest_schema(expected, element, None)?,
                    bindings,
                    inputs,
                    allow_sampled_input,
                )?;
            }
            for item in suffix {
                validate_structural_pattern(
                    draft,
                    item,
                    &component_schema(expected, element)?,
                    bindings,
                    inputs,
                    allow_sampled_input,
                )?;
            }
        }
    }
    Some(())
}

fn structurally_irrefutable_for_schema<V>(
    schemas: &mech_core::SchemaTable,
    pattern: &super::CollectionPattern<SchemaId, V>,
    expected: &mech_core::Schema,
) -> bool {
    match pattern {
        super::CollectionPattern::Wildcard => true,
        super::CollectionPattern::Bind { schema, .. } => schemas.get(*schema) == Some(expected),
        super::CollectionPattern::Tuple(items) => {
            let mech_core::SchemaBody::Tuple(fields) = expected.body() else {
                return false;
            };
            fields.len() == items.len()
                && items.iter().zip(fields).all(|(item, field)| {
                    component_schema(expected, field).is_some_and(|expected| {
                        structurally_irrefutable_for_schema(schemas, item, &expected)
                    })
                })
        }
        super::CollectionPattern::Array {
            prefix,
            rest: Some(rest),
            suffix,
        } => {
            let mech_core::SchemaBody::Matrix { element, .. } = expected.body() else {
                return false;
            };
            let Some(fixed) = prefix.len().checked_add(suffix.len()) else {
                return false;
            };
            let residual = super::comprehension::fixed_matrix_element_count(expected)
                .and_then(|count| count.checked_sub(fixed));
            let length_is_irrefutable = fixed == 0 || residual.is_some();
            length_is_irrefutable
                && component_schema(expected, element).is_some_and(|element| {
                    prefix
                        .iter()
                        .chain(suffix)
                        .all(|item| structurally_irrefutable_for_schema(schemas, item, &element))
                        && (super::comprehension::array_rest_schema(
                            &element,
                            element.body(),
                            residual,
                        )
                        .is_some_and(|expected| {
                            structurally_irrefutable_for_schema(schemas, rest, &expected)
                        }) || matches!(
                            rest.as_ref(),
                            super::CollectionPattern::Bind { schema, .. }
                            if super::comprehension::array_rest_schema(
                                &element,
                                element.body(),
                                None,
                            )
                            .as_ref()
                            .is_some_and(|expected| schemas.get(*schema) == Some(expected))
                        ))
                })
        }
        super::CollectionPattern::Array {
            prefix,
            rest: None,
            suffix,
        } => {
            let mech_core::SchemaBody::Matrix { element, .. } = expected.body() else {
                return false;
            };
            prefix.len().checked_add(suffix.len()).is_some_and(|count| {
                super::comprehension::fixed_matrix_element_count(expected) == Some(count)
            }) && component_schema(expected, element).is_some_and(|expected| {
                prefix
                    .iter()
                    .chain(suffix)
                    .all(|item| structurally_irrefutable_for_schema(schemas, item, &expected))
            })
        }
        super::CollectionPattern::Enum { ordinal, payload } => {
            let mech_core::SchemaBody::Enum { variants, .. } = expected.body() else {
                return false;
            };
            if variants.len() != 1 {
                return false;
            }
            let Some(variant) = variants.get(*ordinal as usize) else {
                return false;
            };
            match (&variant.payload, payload) {
                (None, None) => true,
                (Some(payload_schema), Some(pattern)) => component_schema(expected, payload_schema)
                    .is_some_and(|expected| {
                        structurally_irrefutable_for_schema(schemas, pattern, &expected)
                    }),
                _ => false,
            }
        }
        super::CollectionPattern::Equal(_) => false,
    }
}

fn structural_coverage_space(expected: &mech_core::Schema) -> StructuralCoverageSpace {
    match expected.body() {
        mech_core::SchemaBody::Bool => StructuralCoverageSpace::Bool,
        mech_core::SchemaBody::Enum { variants, .. } => StructuralCoverageSpace::Enum(
            variants
                .iter()
                .map(|variant| {
                    variant.payload.as_ref().map(|payload| {
                        component_schema(expected, payload)
                            .map_or(StructuralCoverageSpace::Opaque, |payload| {
                                structural_coverage_space(&payload)
                            })
                    })
                })
                .collect(),
        ),
        mech_core::SchemaBody::Tuple(fields) => StructuralCoverageSpace::Tuple(
            fields
                .iter()
                .map(|field| {
                    component_schema(expected, field)
                        .map_or(StructuralCoverageSpace::Opaque, |field| {
                            structural_coverage_space(&field)
                        })
                })
                .collect(),
        ),
        mech_core::SchemaBody::Matrix { element, .. } => {
            super::comprehension::fixed_matrix_element_count(expected)
                .and_then(|count| {
                    component_schema(expected, element).map(|element| {
                        StructuralCoverageSpace::Repeated {
                            element: Box::new(structural_coverage_space(&element)),
                            count,
                        }
                    })
                })
                .unwrap_or(StructuralCoverageSpace::Opaque)
        }
        _ => StructuralCoverageSpace::Opaque,
    }
}

fn structural_coverage_pattern(
    pattern: &super::CollectionPattern<SchemaId, MatchPatternValue>,
    expected: &mech_core::Schema,
    draft: &super::ProgramArtifactDraft,
) -> StructuralCoveragePattern {
    if structurally_irrefutable_for_schema(&draft.schemas, pattern, expected) {
        return StructuralCoveragePattern::Wildcard;
    }
    match (pattern, expected.body()) {
        (
            super::CollectionPattern::Equal(MatchPatternValue::Literal(constant)),
            mech_core::SchemaBody::Bool,
        ) => match draft.constants.get(*constant).map(|value| value.data()) {
            Some(mech_core::ValueData::Bool(value)) => StructuralCoveragePattern::Bool(*value),
            _ => StructuralCoveragePattern::Never,
        },
        (super::CollectionPattern::Tuple(items), mech_core::SchemaBody::Tuple(fields))
            if items.len() == fields.len() =>
        {
            let Some(items) = items
                .iter()
                .zip(fields)
                .map(|(item, field)| {
                    let expected = component_schema(expected, field)?;
                    Some(structural_coverage_pattern(item, &expected, draft))
                })
                .collect::<Option<Vec<_>>>()
            else {
                return StructuralCoveragePattern::Never;
            };
            StructuralCoveragePattern::Tuple(items.into_boxed_slice())
        }
        (
            super::CollectionPattern::Array {
                prefix,
                rest,
                suffix,
            },
            mech_core::SchemaBody::Matrix { element, .. },
        ) => {
            let Some(count) = super::comprehension::fixed_matrix_element_count(expected) else {
                return StructuralCoveragePattern::Never;
            };
            let Some(fixed) = prefix.len().checked_add(suffix.len()) else {
                return StructuralCoveragePattern::Never;
            };
            let Some(residual) = count.checked_sub(fixed) else {
                return StructuralCoveragePattern::Never;
            };
            if rest.is_none() && residual != 0 {
                return StructuralCoveragePattern::Never;
            }
            let Some(element) = component_schema(expected, element) else {
                return StructuralCoveragePattern::Never;
            };
            let mut normalized_prefix = Vec::with_capacity(prefix.len());
            normalized_prefix.extend(
                prefix
                    .iter()
                    .map(|item| structural_coverage_pattern(item, &element, draft)),
            );
            let mut normalized_suffix = suffix
                .iter()
                .map(|item| structural_coverage_pattern(item, &element, draft))
                .collect::<Vec<_>>();
            let mut wildcard_count = 0;
            if let Some(rest) = rest {
                let Some(exact_rest) = super::comprehension::array_rest_schema(
                    &element,
                    element.body(),
                    Some(residual),
                ) else {
                    return StructuralCoveragePattern::Never;
                };
                let rest = if matches!(
                    rest.as_ref(),
                    super::CollectionPattern::Bind { schema, .. }
                        if super::comprehension::array_rest_schema(
                            &element,
                            element.body(),
                            None,
                        )
                        .as_ref()
                        .is_some_and(|expected| draft.schemas.get(*schema) == Some(expected))
                ) {
                    StructuralCoveragePattern::Wildcard
                } else {
                    structural_coverage_pattern(rest, &exact_rest, draft)
                };
                match rest {
                    StructuralCoveragePattern::Wildcard => wildcard_count = residual,
                    StructuralCoveragePattern::Sequence {
                        prefix,
                        wildcard_count: rest_wildcard_count,
                        suffix,
                    } if prefix
                        .len()
                        .checked_add(rest_wildcard_count)
                        .and_then(|length| length.checked_add(suffix.len()))
                        == Some(residual) =>
                    {
                        normalized_prefix.extend(prefix);
                        wildcard_count = rest_wildcard_count;
                        let mut combined_suffix = suffix.into_vec();
                        combined_suffix.append(&mut normalized_suffix);
                        normalized_suffix = combined_suffix;
                    }
                    _ => return StructuralCoveragePattern::Never,
                }
            }
            StructuralCoveragePattern::Sequence {
                prefix: normalized_prefix.into_boxed_slice(),
                wildcard_count,
                suffix: normalized_suffix.into_boxed_slice(),
            }
        }
        (
            super::CollectionPattern::Enum { ordinal, payload },
            mech_core::SchemaBody::Enum { variants, .. },
        ) => {
            let Some(variant) = variants.get(*ordinal as usize) else {
                return StructuralCoveragePattern::Never;
            };
            let payload = match (&variant.payload, payload.as_deref()) {
                (None, None) => None,
                (Some(payload_schema), Some(pattern)) => {
                    let Some(expected) = component_schema(expected, payload_schema) else {
                        return StructuralCoveragePattern::Never;
                    };
                    Some(Box::new(structural_coverage_pattern(
                        pattern, &expected, draft,
                    )))
                }
                _ => return StructuralCoveragePattern::Never,
            };
            StructuralCoveragePattern::Enum {
                ordinal: *ordinal,
                payload,
            }
        }
        _ => StructuralCoveragePattern::Never,
    }
}

fn cover_structural_pattern(
    coverage: &mut StructuralPatternCoverage,
    pattern: &super::CollectionPattern<SchemaId, MatchPatternValue>,
    expected: &mech_core::Schema,
    draft: &super::ProgramArtifactDraft,
) {
    coverage.cover(structural_coverage_pattern(pattern, expected, draft));
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
        false,
        &mut 0,
        &[],
        false,
        false,
        true,
    )
}

pub(super) fn validate_activation(
    draft: &super::ProgramArtifactDraft,
    node: mech_core::NodeId,
    declaration: &MatchDeclaration,
    inputs: &[SchemaId],
    output: SchemaId,
) -> Result<(), super::ArtifactBuildError> {
    validate_match_inner(
        draft,
        node,
        declaration,
        inputs,
        output,
        None,
        true,
        &mut 0,
        &[],
        false,
        false,
        false,
    )
}

pub(super) fn validate_match_inner(
    draft: &super::ProgramArtifactDraft,
    node: mech_core::NodeId,
    declaration: &MatchDeclaration,
    inputs: &[SchemaId],
    output: SchemaId,
    lexical_signature: Option<(SchemaId, SchemaId)>,
    allow_sampled_pattern: bool,
    next_block: &mut u32,
    enclosing_matches: &[(SchemaId, SchemaId, bool, bool, bool)],
    inside_comprehension: bool,
    enclosing_guard: bool,
    recursive_target_eligible: bool,
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
            .is_some_and(is_control_literal_schema)
    };
    let closed_value = |schema| {
        draft
            .schemas
            .get(schema)
            .is_some_and(is_control_value_schema)
    };
    let result_value = |schema| {
        draft
            .schemas
            .get(schema)
            .is_some_and(is_control_result_schema)
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
    let mut match_schemas = enclosing_matches.to_vec();
    match_schemas.push((
        scrutinee,
        output,
        declaration
            .arms
            .iter()
            .any(|arm| matches!(&arm.pattern, MatchPattern::Bind)),
        declaration
            .captures
            .iter()
            .any(|capture| capture.input == declaration.scrutinee),
        recursive_target_eligible,
    ));
    if !result_value(output)
        || (!scalar(scrutinee)
            && declaration
                .arms
                .iter()
                .any(|arm| matches!(&arm.pattern, MatchPattern::Literal(_))))
    {
        return Err(invalid(
            "match requires a supported result and scalar literal-pattern scrutinee",
        ));
    }
    if !draft
        .schemas
        .get(scrutinee)
        .is_some_and(is_control_pattern_schema)
        && declaration.arms.iter().any(|arm| {
            matches!(
                &arm.pattern,
                MatchPattern::Bind | MatchPattern::Structural(_)
            )
        })
    {
        return Err(invalid("match bindings require a supported value schema"));
    }
    let mut used_inputs = std::collections::BTreeSet::from([declaration.scrutinee]);
    let mut captured_inputs = std::collections::BTreeSet::new();
    for capture in &declaration.captures {
        if inputs.get(capture.input as usize) != Some(&capture.schema)
            || !draft
                .schemas
                .get(capture.schema)
                .is_some_and(is_control_capture_schema)
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
    let scrutinee_schema = draft
        .schemas
        .get(scrutinee)
        .ok_or_else(|| invalid("unknown match scrutinee schema"))?;
    let mut coverage = StructuralPatternCoverage::new(structural_coverage_space(scrutinee_schema));
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
            validate_structural_pattern(
                draft,
                pattern,
                schema,
                &mut pattern_bindings,
                inputs,
                allow_sampled_pattern,
            )
            .ok_or_else(|| invalid("invalid structural match pattern"))?;
        }
        for (block, is_guard) in arm
            .guard
            .iter()
            .map(|block| (block, true))
            .chain(core::iter::once((&arm.body, false)))
        {
            let guarded = enclosing_guard || is_guard;
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
                    ControlParameterSource::IterationCarrier => {
                        return Err(invalid("iteration carrier is not a match parameter"));
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
                        let producer = &block.operations[local as usize];
                        if matches!(producer.body, ControlOperationBody::Publish) {
                            return Err(invalid("publication locals cannot be referenced"));
                        }
                        Ok(producer.schema)
                    }
                    _ => Err(invalid("cross-block or non-dominating local reference")),
                }
            };
            for (index, operation) in block.operations.iter().enumerate() {
                // Pure intermediate operations may preserve turn-shaped
                // dimensions. The block yield is checked against the closed
                // match result after all local operations are validated.
                let local_schema = draft
                    .schemas
                    .get(operation.schema)
                    .ok_or_else(|| invalid("unknown block local schema"))?;
                if operation.node as usize != index
                    || (matches!(local_schema.body(), SchemaBody::Dynamic)
                        && !matches!(&operation.body, ControlOperationBody::Comprehension(_)))
                {
                    return Err(invalid(
                        "invalid local identity or dynamic operation result",
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
                            false,
                            next_block,
                            &match_schemas,
                            inside_comprehension,
                            guarded,
                            true,
                        )?,
                        ControlOperationBody::Comprehension(nested) => {
                            super::comprehension::validate_comprehension_inner(
                                draft,
                                node,
                                nested,
                                &inputs,
                                operation.schema,
                                next_block,
                                guarded,
                            )?
                        }
                        ControlOperationBody::Iteration(nested) => {
                            super::iteration::validate_iteration_inner(
                                draft,
                                node,
                                nested,
                                &inputs,
                                operation.schema,
                                next_block,
                                guarded,
                            )?;
                        }
                        ControlOperationBody::Assert => {
                            if !is_assertion_signature(&draft.schemas, &inputs, operation.schema) {
                                return Err(invalid(
                                    "lexical assertion requires one Bool input and Bool result",
                                ));
                            }
                        }
                        ControlOperationBody::Recur(ancestor) => {
                            let target = match_schemas
                                .len()
                                .checked_sub(usize::from(*ancestor) + 1)
                                .and_then(|index| match_schemas.get(index))
                                .ok_or_else(|| invalid("unknown recursive lexical target"))?;
                            if !target.4 {
                                return Err(invalid(
                                    "recursive target cannot be an activation scope",
                                ));
                            }
                            if inputs.as_slice() != [target.0] || operation.schema != target.1 {
                                return Err(invalid(
                                    "recursive call must preserve its lexical target input and output schemas",
                                ));
                            }
                            if target.2 {
                                return Err(invalid(
                                    "recursive target cannot use a direct bind pattern",
                                ));
                            }
                            if target.3 {
                                return Err(invalid(
                                    "recursive target cannot capture its own scrutinee",
                                ));
                            }
                        }
                        ControlOperationBody::Suspend => {
                            if !enclosing_matches.is_empty()
                                || guarded
                                || inside_comprehension
                                || index + 1 != block.operations.len()
                                || block.yield_value
                                    != (ControlValue::Local {
                                        block: block.id,
                                        node: index as u32,
                                    })
                            {
                                return Err(invalid(
                                    "suspension must be the terminal FSM body yield",
                                ));
                            }
                            let Some((lexical_input, lexical_output)) = lexical_signature else {
                                return Err(invalid(
                                    "suspended control requires an enclosing lexical signature",
                                ));
                            };
                            if inputs.as_slice() != [lexical_input]
                                || operation.schema != lexical_output
                            {
                                return Err(invalid(
                                    "suspended control must preserve the enclosing input and output schemas",
                                ));
                            }
                        }
                        ControlOperationBody::Publish => {
                            if guarded || inside_comprehension {
                                return Err(invalid(
                                    "FSM publication cannot execute inside a guard or comprehension",
                                ));
                            }
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
                    let live_turn_shaped_capture = matches!(
                        input,
                        ControlValue::Parameter { block: owner, ordinal }
                            if *owner == block.id
                                && block.parameters.get(*ordinal as usize).is_some_and(|parameter| {
                                    matches!(parameter.source, ControlParameterSource::Capture(_))
                                        && draft.schemas.get(parameter.schema)
                                            .is_some_and(is_control_capture_schema)
                                })
                    );
                    let live_turn_shaped_local = matches!(
                        input,
                        ControlValue::Local { block: owner, node: local }
                            if *owner == block.id
                                && block.operations.get(*local as usize).is_some_and(|producer| {
                                    !closed_value(producer.schema)
                                })
                    );
                    if port.access != AccessMode::Read
                        || port.delivery != DeliveryMode::Signal
                        || value_schema(*input, index)? != port.schema
                        || (!closed_value(port.schema)
                            && !live_pattern_parameter
                            && !live_turn_shaped_capture
                            && !live_turn_shaped_local)
                    {
                        return Err(invalid("block operation input contract mismatch"));
                    }
                }
                let port = &contract.outputs[0];
                // A branch owns every local result. RMW is still a pure value
                // operation: execution seeds a fresh result from its explicit
                // base, then passes only the remaining operands to the kernel.
                let construction_supported = match port.construction {
                    OutputConstruction::FullWrite { .. }
                    | OutputConstruction::Build { .. }
                    | OutputConstruction::Replace { .. } => {
                        port.access == AccessMode::Write && port.alias == AliasPolicy::NoAlias
                    }
                    OutputConstruction::ReadModifyWrite { base_input, .. } => {
                        port.access == AccessMode::ReadWrite
                            && port.alias == (AliasPolicy::MayAlias { input: base_input })
                            && contract
                                .inputs
                                .get(usize::from(base_input))
                                .is_some_and(|base| base.schema == operation.schema)
                    }
                };
                if port.schema != operation.schema
                    || port.delivery != DeliveryMode::Signal
                    || !construction_supported
                {
                    return Err(invalid(
                        "block operation must construct its exact owned result",
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
                        mech_core::ValueData::Bool(value) => coverage.cover_bool(*value),
                        mech_core::ValueData::Enum(value) => {
                            if value.payload().is_none() {
                                coverage.cover(StructuralCoveragePattern::Enum {
                                    ordinal: value.ordinal(),
                                    payload: None,
                                });
                            }
                        }
                        _ => {}
                    }
                }
                MatchPattern::Wildcard | MatchPattern::Bind => {
                    coverage.cover_all();
                }
                MatchPattern::Structural(pattern) => {
                    cover_structural_pattern(&mut coverage, pattern, scrutinee_schema, draft);
                }
            }
        }
    }
    if !declaration.partial && !coverage.is_complete() {
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
// Ordinary function frames and their selected match expressions share this
// lexical nesting bound. Keep a finite stack limit without excluding a small
// chain of recursive helpers; total declarations/operations remain separately
// bounded by MAX_CONTROL_BLOCKS and the artifact admission limits.
pub const MAX_CONTROL_DEPTH: usize = 32;

enum ControlRef<'a, C> {
    Match(&'a MatchDeclaration<C>),
    Comprehension(&'a super::ComprehensionDeclaration<C>),
    Iteration(&'a super::IterationDeclaration<C>),
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
                                ControlOperationBody::Iteration(nested) => pending
                                    .push((ControlRef::Iteration(nested), depth.checked_add(1)?)),
                                ControlOperationBody::Operation { .. }
                                | ControlOperationBody::Recur(_)
                                | ControlOperationBody::Suspend
                                | ControlOperationBody::Assert
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
                                ControlOperationBody::Iteration(nested) => pending
                                    .push((ControlRef::Iteration(nested), depth.checked_add(1)?)),
                                ControlOperationBody::Operation { .. }
                                | ControlOperationBody::Recur(_)
                                | ControlOperationBody::Suspend
                                | ControlOperationBody::Assert
                                | ControlOperationBody::Publish => {}
                            }
                        }
                    }
                }
            }
            ControlRef::Iteration(control) => {
                add(1, 1)?;
                add(2, control.body.operations.len())?;
                add(3, control.captures.len())?;
                add(3, control.body.parameters.len())?;
                // One real scratch carrier shares the u16 local namespace.
                add(4, 1)?;
                add(4, control.body.operations.len())?;
                for operation in &control.body.operations {
                    add(3, operation.inputs.len())?;
                    match &operation.body {
                        ControlOperationBody::Match(nested) => {
                            pending.push((ControlRef::Match(nested), depth.checked_add(1)?))
                        }
                        ControlOperationBody::Comprehension(nested) => {
                            pending.push((ControlRef::Comprehension(nested), depth.checked_add(1)?))
                        }
                        ControlOperationBody::Iteration(nested) => {
                            pending.push((ControlRef::Iteration(nested), depth.checked_add(1)?))
                        }
                        ControlOperationBody::Operation { .. }
                        | ControlOperationBody::Recur(_)
                        | ControlOperationBody::Suspend
                        | ControlOperationBody::Assert
                        | ControlOperationBody::Publish => {}
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

fn validate_control_depth<C>(
    root: ControlRef<'_, C>,
    node: mech_core::NodeId,
) -> Result<(), super::ArtifactBuildError> {
    let invalid = || super::ArtifactBuildError::InvalidControl {
        node,
        reason: "control graph nesting limit",
    };
    let mut pending = vec![(root, 1usize)];
    while let Some((control, depth)) = pending.pop() {
        if depth > MAX_CONTROL_DEPTH {
            return Err(invalid());
        }
        let nested_depth = depth.checked_add(1).ok_or_else(invalid)?;
        match control {
            ControlRef::Match(control) => {
                for operation in control
                    .arms
                    .iter()
                    .flat_map(|arm| arm.guard.iter().chain(core::iter::once(&arm.body)))
                    .flat_map(|block| block.operations.iter())
                {
                    match &operation.body {
                        ControlOperationBody::Match(nested) => {
                            pending.push((ControlRef::Match(nested), nested_depth));
                        }
                        ControlOperationBody::Comprehension(nested) => {
                            pending.push((ControlRef::Comprehension(nested), nested_depth));
                        }
                        ControlOperationBody::Iteration(nested) => {
                            pending.push((ControlRef::Iteration(nested), nested_depth));
                        }
                        ControlOperationBody::Operation { .. }
                        | ControlOperationBody::Recur(_)
                        | ControlOperationBody::Suspend
                        | ControlOperationBody::Assert
                        | ControlOperationBody::Publish => {}
                    }
                }
            }
            ControlRef::Comprehension(control) => {
                for operation in control.steps.iter().filter_map(|step| match step {
                    super::ComprehensionStep::Operation(operation) => Some(operation),
                    super::ComprehensionStep::Generator { .. }
                    | super::ComprehensionStep::Filter(_) => None,
                }) {
                    match &operation.body {
                        ControlOperationBody::Match(nested) => {
                            pending.push((ControlRef::Match(nested), nested_depth));
                        }
                        ControlOperationBody::Comprehension(nested) => {
                            pending.push((ControlRef::Comprehension(nested), nested_depth));
                        }
                        ControlOperationBody::Iteration(nested) => {
                            pending.push((ControlRef::Iteration(nested), nested_depth));
                        }
                        ControlOperationBody::Operation { .. }
                        | ControlOperationBody::Recur(_)
                        | ControlOperationBody::Suspend
                        | ControlOperationBody::Assert
                        | ControlOperationBody::Publish => {}
                    }
                }
            }
            ControlRef::Iteration(control) => {
                for operation in &control.body.operations {
                    match &operation.body {
                        ControlOperationBody::Match(nested) => {
                            pending.push((ControlRef::Match(nested), nested_depth))
                        }
                        ControlOperationBody::Comprehension(nested) => {
                            pending.push((ControlRef::Comprehension(nested), nested_depth))
                        }
                        ControlOperationBody::Iteration(nested) => {
                            pending.push((ControlRef::Iteration(nested), nested_depth))
                        }
                        ControlOperationBody::Operation { .. }
                        | ControlOperationBody::Recur(_)
                        | ControlOperationBody::Suspend
                        | ControlOperationBody::Assert
                        | ControlOperationBody::Publish => {}
                    }
                }
            }
        }
    }
    Ok(())
}

pub(super) fn validate_iteration_depth<C>(
    control: &super::IterationDeclaration<C>,
    node: mech_core::NodeId,
) -> Result<(), super::ArtifactBuildError> {
    validate_control_depth(ControlRef::Iteration(control), node)
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
        if let super::ExecutableNodeBody::Iteration(control) = &node.body {
            let nested = control_counts(ControlRef::Iteration(control)).ok_or_else(invalid)?;
            for (index, count) in nested[..4].iter().copied().enumerate() {
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
                    ControlOperationBody::Comprehension(nested) => nested.contains_suspend(),
                    ControlOperationBody::Iteration(nested) => nested.contains_suspend(),
                    ControlOperationBody::Operation { .. }
                    | ControlOperationBody::Recur(_)
                    | ControlOperationBody::Assert
                    | ControlOperationBody::Publish => false,
                })
        })
    }

    pub(super) fn validate_depth(
        &self,
        node: mech_core::NodeId,
    ) -> Result<(), super::ArtifactBuildError> {
        validate_control_depth(ControlRef::Match(self), node)
    }

    pub(super) fn map_contracts<D, E>(
        &self,
        mut map: impl FnMut(&ControlBlock<C>, &ControlOperation<C>, &C) -> Result<D, E>,
    ) -> Result<MatchDeclaration<D>, E> {
        self.map_contracts_inner(&mut |block, operation, _, contract| {
            map(block, operation, contract)
        })
    }

    pub(super) fn map_contracts_inner<D, E>(
        &self,
        map: &mut dyn FnMut(
            &ControlBlock<C>,
            &ControlOperation<C>,
            &super::OperationReference,
            &C,
        ) -> Result<D, E>,
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
                                    contract: map(block, operation, reference, contract)?,
                                },
                                ControlOperationBody::Match(nested) => {
                                    ControlOperationBody::Match(nested.map_contracts_inner(map)?)
                                }
                                ControlOperationBody::Comprehension(nested) => {
                                    ControlOperationBody::Comprehension(
                                        nested.map_contracts_inner(
                                            &mut |reference, contract| {
                                                map(block, operation, reference, contract)
                                            },
                                        )?,
                                    )
                                }
                                ControlOperationBody::Iteration(nested) => {
                                    ControlOperationBody::Iteration(
                                        nested.map_contracts_inner(map)?,
                                    )
                                }
                                ControlOperationBody::Recur(ancestor) => {
                                    ControlOperationBody::Recur(*ancestor)
                                }
                                ControlOperationBody::Assert => ControlOperationBody::Assert,

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

impl<C> super::ComprehensionDeclaration<C> {
    #[cfg(feature = "resident-artifact")]
    pub(crate) fn contains_suspend(&self) -> bool {
        self.steps.iter().any(|step| {
            let super::ComprehensionStep::Operation(operation) = step else {
                return false;
            };
            match &operation.body {
                ControlOperationBody::Suspend => true,
                ControlOperationBody::Match(nested) => nested.contains_suspend(),
                ControlOperationBody::Comprehension(nested) => nested.contains_suspend(),
                ControlOperationBody::Iteration(nested) => nested.contains_suspend(),
                ControlOperationBody::Operation { .. }
                | ControlOperationBody::Recur(_)
                | ControlOperationBody::Assert
                | ControlOperationBody::Publish => false,
            }
        })
    }

    pub(super) fn validate_depth(
        &self,
        node: mech_core::NodeId,
    ) -> Result<(), super::ArtifactBuildError> {
        validate_control_depth(ControlRef::Comprehension(self), node)
    }
}

pub(crate) fn is_control_scalar_schema(schema: &mech_core::Schema) -> bool {
    use mech_core::SchemaBody;
    schema.dimension_parameters().is_empty()
        && (is_builtin_control_scalar_body(schema.body())
            || matches!(schema.body(), SchemaBody::Option(payload) if is_optional_control_scalar_body(payload)))
}

/// Literal patterns include exact raw UTF-8 Strings without broadening the
/// fixed numeric/Boolean scalar execution class used by ordinary kernels.
pub(crate) fn is_control_literal_schema(schema: &mech_core::Schema) -> bool {
    is_control_scalar_schema(schema)
        || (schema.dimension_parameters().is_empty()
            && matches!(schema.body(), mech_core::SchemaBody::String))
}

fn is_builtin_control_scalar_body(body: &mech_core::SchemaBody) -> bool {
    use mech_core::SchemaBody;
    matches!(
        body,
        SchemaBody::Bool
            | SchemaBody::Index
            | SchemaBody::IntegerInterval(_)
            | SchemaBody::SignedInteger(_)
            | SchemaBody::UnsignedInteger(_)
            | SchemaBody::FloatingPoint(_)
            | SchemaBody::Complex(_)
            | SchemaBody::Rational64
    )
}

fn is_optional_control_scalar_body(body: &mech_core::SchemaBody) -> bool {
    use mech_core::SchemaBody;
    is_builtin_control_scalar_body(body)
        || matches!(body, SchemaBody::Atom(_))
        || matches!(
            body,
            SchemaBody::Enum { variants, .. }
                if variants.iter().all(|variant| variant.payload.as_ref().is_none_or(|payload| {
                    is_optional_control_scalar_body(payload)
                }))
        )
        || matches!(body, SchemaBody::Option(payload) if is_optional_control_scalar_body(payload))
}

/// Control values share ordinary schema and construction authorities.
/// Per-turn variable shapes need explicit control shape bindings before admission.
pub(crate) fn is_control_value_schema(schema: &mech_core::Schema) -> bool {
    schema.dimension_parameters().is_empty()
        && !matches!(schema.body(), mech_core::SchemaBody::Dynamic)
}

// Fixed record/tuple elements carry no child-container dimensions. A matrix
// of these values uses the same turn-local geometry as an unsigned buffer.
pub(crate) fn is_control_fixed_aggregate_element(body: &mech_core::SchemaBody) -> bool {
    use mech_core::SchemaBody as B;
    fn field(body: &B) -> bool {
        matches!(
            body,
            B::Bool
                | B::Index
                | B::Id
                | B::Atom(_)
                | B::String
                | B::UnsignedInteger(_)
                | B::SignedInteger(_)
                | B::FloatingPoint(_)
        ) || is_control_fixed_aggregate_element(body)
    }
    match body {
        B::Record(fields) => fields.iter().all(|entry| field(&entry.schema)),
        B::Tuple(fields) => fields.iter().all(field),
        _ => false,
    }
}

/// A turn-shaped unsigned or fixed-aggregate matrix carries its live dimensions
/// in a canonical snapshot. This is structural-pattern authority; closed-value,
/// literal and capture eligibility remain separate predicates.
pub(crate) fn is_control_pattern_schema(schema: &mech_core::Schema) -> bool {
    fn carrier(body: &mech_core::SchemaBody) -> Option<bool> {
        use mech_core::SchemaBody as B;
        match body {
            B::Matrix {
                element,
                dimensions,
            } if dimensions.len() == 2
                && (matches!(element.as_ref(), B::UnsignedInteger(_))
                    || is_control_fixed_aggregate_element(element)) =>
            {
                Some(true)
            }
            B::Tuple(fields) => fields.iter().try_fold(false, |matrix, body| {
                carrier(body).map(|child| matrix || child)
            }),
            B::Bool
            | B::Index
            | B::Id
            | B::Atom(_)
            | B::String
            | B::UnsignedInteger(_)
            | B::SignedInteger(_)
            | B::FloatingPoint(_) => Some(false),
            _ => None,
        }
    }
    is_control_value_schema(schema)
        || (schema
            .dimension_parameters()
            .iter()
            .all(|parameter| parameter.lifetime() == mech_core::DimensionLifetime::Turn)
            && carrier(schema.body()) == Some(true))
}

/// Selected control results may retain per-turn Matrix geometry in an
/// immutable canonical snapshot. This is a result-only authority: patterns,
/// scrutinees, lexical captures and state carriers keep their own gates.
pub(crate) fn is_control_result_schema(schema: &mech_core::Schema) -> bool {
    use mech_core::{DimensionLifetime, SchemaBody};
    fn matrix_result(body: &SchemaBody) -> Option<bool> {
        fn children<'a>(mut bodies: impl Iterator<Item = &'a SchemaBody>) -> Option<bool> {
            bodies.try_fold(false, |matrix, body| {
                matrix_result(body).map(|child| matrix || child)
            })
        }
        match body {
            SchemaBody::Matrix {
                element,
                dimensions,
            } if dimensions.len() == 2 => matrix_result(element).map(|_| true),
            SchemaBody::Tuple(fields) => children(fields.iter()),
            SchemaBody::Record(fields) => children(fields.iter().map(|field| &field.schema)),
            SchemaBody::Option(body) => matrix_result(body),
            SchemaBody::Enum { variants, .. } => children(
                variants
                    .iter()
                    .filter_map(|variant| variant.payload.as_ref()),
            ),
            SchemaBody::Bool
            | SchemaBody::UnsignedInteger(_)
            | SchemaBody::SignedInteger(_)
            | SchemaBody::IntegerInterval(_)
            | SchemaBody::FloatingPoint(_)
            | SchemaBody::Complex(_)
            | SchemaBody::Rational64
            | SchemaBody::String
            | SchemaBody::Id
            | SchemaBody::Index
            | SchemaBody::Atom(_) => Some(false),
            _ => None,
        }
    }
    is_control_value_schema(schema)
        || (schema
            .dimension_parameters()
            .iter()
            .all(|parameter| parameter.lifetime() == DimensionLifetime::Turn)
            && matrix_result(schema.body()) == Some(true))
}

/// Lexical matrices and records of matrices retain their turn-local geometry
/// in a canonical snapshot. This does not widen pattern or state eligibility.
pub(crate) fn is_control_capture_schema(schema: &mech_core::Schema) -> bool {
    use mech_core::{DimensionLifetime, SchemaBody};
    fn carrier(body: &SchemaBody) -> Option<bool> {
        match body {
            SchemaBody::Matrix {
                element,
                dimensions,
            } if dimensions.len() == 2
                && (is_control_fixed_aggregate_element(element)
                    || matches!(
                        element.as_ref(),
                        SchemaBody::Bool
                            | SchemaBody::Index
                            | SchemaBody::String
                            | SchemaBody::UnsignedInteger(_)
                            | SchemaBody::SignedInteger(_)
                            | SchemaBody::FloatingPoint(_)
                    )) =>
            {
                Some(true)
            }
            SchemaBody::Record(fields) => fields.iter().try_fold(false, |matrix, field| {
                carrier(&field.schema).map(|child| matrix || child)
            }),
            body if is_builtin_control_scalar_body(body)
                || matches!(
                    body,
                    SchemaBody::String | SchemaBody::Id | SchemaBody::Atom(_)
                ) =>
            {
                Some(false)
            }
            _ => None,
        }
    }
    is_control_value_schema(schema)
        || (carrier(schema.body()) == Some(true)
            && schema
                .dimension_parameters()
                .iter()
                .all(|parameter| parameter.lifetime() == DimensionLifetime::Turn))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_core::{DimensionExpr, FloatWidth, SchemaBody, SchemaDraft, SchemaTableBuilder};

    #[test]
    fn turn_record_matrix_carriers_reject_child_containers_and_other_lifetimes() {
        use mech_core::{
            DimensionLifetime, DimensionParameterDeclaration, DimensionParameterId,
            DimensionParameterOrigin, SchemaField,
        };
        let record = |field| {
            SchemaBody::Record(
                vec![SchemaField {
                    name: "value".into(),
                    schema: field,
                }]
                .into_boxed_slice(),
            )
        };
        let schema = |element, lifetime| {
            SchemaDraft {
                body: SchemaBody::Matrix {
                    element: Box::new(element),
                    dimensions: vec![
                        DimensionExpr::Constant(1),
                        DimensionExpr::Parameter(DimensionParameterId::new(0)),
                    ]
                    .into_boxed_slice(),
                },
                dimension_parameters: vec![DimensionParameterDeclaration {
                    id: DimensionParameterId::new(0),
                    origin: DimensionParameterOrigin::Explicit,
                    lifetime,
                    lower_bound: DimensionExpr::Constant(0),
                    upper_bound: None,
                }]
                .into_boxed_slice(),
            }
            .finalize()
            .unwrap()
        };
        let fixed = record(SchemaBody::Tuple(
            vec![
                SchemaBody::Bool,
                SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W32),
            ]
            .into_boxed_slice(),
        ));
        for lifetime in [DimensionLifetime::Turn, DimensionLifetime::Activation] {
            let value = schema(fixed.clone(), lifetime);
            assert_eq!(
                is_control_pattern_schema(&value),
                lifetime == DimensionLifetime::Turn
            );
            assert_eq!(
                is_control_capture_schema(&value),
                lifetime == DimensionLifetime::Turn
            );
            assert!(!is_control_value_schema(&value));
        }
        let child = record(SchemaBody::Matrix {
            element: Box::new(SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64)),
            dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(2)]
                .into_boxed_slice(),
        });
        let value = schema(child, DimensionLifetime::Turn);
        assert!(!is_control_pattern_schema(&value));
        assert!(!is_control_capture_schema(&value));
    }

    #[test]
    fn turn_matrix_capture_eligibility_does_not_widen_closed_values() {
        use mech_core::{
            DimensionLifetime, DimensionParameterDeclaration, DimensionParameterId,
            DimensionParameterOrigin,
        };
        let schema = |element, rank, lifetime| {
            SchemaDraft {
                body: SchemaBody::Matrix {
                    element: Box::new(element),
                    dimensions: (0..rank)
                        .map(|axis| {
                            if axis == 0 {
                                DimensionExpr::Parameter(DimensionParameterId::new(0))
                            } else {
                                DimensionExpr::Constant(1)
                            }
                        })
                        .collect(),
                },
                dimension_parameters: vec![DimensionParameterDeclaration {
                    id: DimensionParameterId::new(0),
                    origin: DimensionParameterOrigin::Explicit,
                    lifetime,
                    lower_bound: DimensionExpr::Constant(0),
                    upper_bound: None,
                }]
                .into_boxed_slice(),
            }
            .finalize()
            .unwrap()
        };
        for element in [
            SchemaBody::String,
            SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64),
            SchemaBody::FloatingPoint(FloatWidth::W64),
        ] {
            let turn = schema(element.clone(), 2, DimensionLifetime::Turn);
            assert!(is_control_capture_schema(&turn));
            assert!(!is_control_value_schema(&turn));
            assert!(!is_control_capture_schema(&schema(
                element.clone(),
                2,
                DimensionLifetime::Activation,
            )));
            assert!(!is_control_capture_schema(&schema(
                element,
                1,
                DimensionLifetime::Turn,
            )));
        }
        assert!(!is_control_capture_schema(&schema(
            SchemaBody::Dynamic,
            2,
            DimensionLifetime::Turn,
        )));
    }

    #[test]
    fn turn_matrix_result_eligibility_is_separate_from_patterns_and_captures() {
        use mech_core::{
            DimensionLifetime, DimensionParameterDeclaration, DimensionParameterId,
            DimensionParameterOrigin, SchemaField,
        };
        let matrix = SchemaBody::Matrix {
            element: Box::new(SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64)),
            dimensions: vec![
                DimensionExpr::Constant(1),
                DimensionExpr::Parameter(DimensionParameterId::new(0)),
            ]
            .into_boxed_slice(),
        };
        let schema = |body, lifetime| {
            SchemaDraft {
                body,
                dimension_parameters: vec![DimensionParameterDeclaration {
                    id: DimensionParameterId::new(0),
                    origin: DimensionParameterOrigin::Explicit,
                    lifetime,
                    lower_bound: DimensionExpr::Constant(0),
                    upper_bound: Some(DimensionExpr::Constant(8)),
                }]
                .into_boxed_slice(),
            }
            .finalize()
            .unwrap()
        };
        for body in [
            matrix.clone(),
            SchemaBody::Tuple(vec![matrix.clone(), SchemaBody::Bool].into_boxed_slice()),
            SchemaBody::Record(
                vec![SchemaField {
                    name: "items".into(),
                    schema: matrix.clone(),
                }]
                .into_boxed_slice(),
            ),
            SchemaBody::Option(Box::new(matrix.clone())),
            SchemaBody::Enum {
                key: mech_core::NominalKey::from_bytes([0x43; 32]),
                variants: vec![
                    mech_core::EnumVariantSchema {
                        name: "Empty".into(),
                        payload: None,
                    },
                    mech_core::EnumVariantSchema {
                        name: "Items".into(),
                        payload: Some(matrix.clone()),
                    },
                ]
                .into_boxed_slice(),
            },
        ] {
            let turn = schema(body.clone(), DimensionLifetime::Turn);
            assert!(is_control_result_schema(&turn));
            assert!(!is_control_value_schema(&turn));
            assert!(!is_control_literal_schema(&turn));
            assert_eq!(
                is_control_capture_schema(&turn),
                matches!(body, SchemaBody::Matrix { .. } | SchemaBody::Record(_))
            );
            let activation = schema(body, DimensionLifetime::Activation);
            assert!(!is_control_result_schema(&activation));
            assert!(!is_control_capture_schema(&activation));
        }
        let bounded = schema(matrix.clone(), DimensionLifetime::Turn);
        assert!(mech_core::shape_for_resolved_extents(&bounded, &[1, 8]).is_ok());
        assert!(mech_core::shape_for_resolved_extents(&bounded, &[1, 9]).is_err());
        let mixed = SchemaDraft {
            body: SchemaBody::Matrix {
                element: Box::new(SchemaBody::Bool),
                dimensions: vec![
                    DimensionExpr::Parameter(DimensionParameterId::new(0)),
                    DimensionExpr::Parameter(DimensionParameterId::new(1)),
                ]
                .into_boxed_slice(),
            },
            dimension_parameters: [DimensionLifetime::Activation, DimensionLifetime::Turn]
                .into_iter()
                .enumerate()
                .map(|(index, lifetime)| DimensionParameterDeclaration {
                    id: DimensionParameterId::new(index as u32),
                    origin: DimensionParameterOrigin::Explicit,
                    lifetime,
                    lower_bound: DimensionExpr::Constant(0),
                    upper_bound: None,
                })
                .collect(),
        }
        .finalize()
        .unwrap();
        assert!(!is_control_result_schema(&mixed));
        let map = SchemaBody::Map {
            key: Box::new(SchemaBody::String),
            value: Box::new(matrix.clone()),
            cardinality: mech_core::ExtentSpec::Exact(DimensionExpr::Constant(0)),
        };
        assert!(!is_control_result_schema(&schema(
            map,
            DimensionLifetime::Turn
        )));
        let dynamic =
            SchemaBody::Tuple(vec![matrix.clone(), SchemaBody::Dynamic].into_boxed_slice());
        assert!(!is_control_result_schema(&schema(
            dynamic,
            DimensionLifetime::Turn
        )));
        let non_rank_two = SchemaBody::Matrix {
            element: Box::new(SchemaBody::Bool),
            dimensions: vec![DimensionExpr::Parameter(DimensionParameterId::new(0))]
                .into_boxed_slice(),
        };
        assert!(!is_control_result_schema(&schema(
            non_rank_two,
            DimensionLifetime::Turn
        )));
        // This result policy does not retroactively restrict legacy closed values.
        let closed = SchemaDraft {
            body: SchemaBody::Bool,
            dimension_parameters: Box::new([]),
        }
        .finalize()
        .unwrap();
        assert!(is_control_value_schema(&closed));
        assert!(is_control_result_schema(&closed));
    }

    #[test]
    fn fixed_array_patterns_are_irrefutable_only_when_their_lengths_fit() {
        let mut builder = SchemaTableBuilder::new();
        let matrix = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Matrix {
                        element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                        dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(2)]
                            .into_boxed_slice(),
                    },
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let built = builder.finish().unwrap();
        let matrix = built.resolve(matrix).unwrap();
        let schemas = built.table;
        let expected = schemas.get(matrix).unwrap();
        let wildcard = || super::super::CollectionPattern::Wildcard;

        let exact: super::super::CollectionPattern<SchemaId, MatchPatternValue> =
            super::super::CollectionPattern::Array {
                prefix: vec![wildcard(), wildcard()].into_boxed_slice(),
                rest: None,
                suffix: Box::new([]),
            };
        assert!(structurally_irrefutable_for_schema(
            &schemas, &exact, expected
        ));

        let short: super::super::CollectionPattern<SchemaId, MatchPatternValue> =
            super::super::CollectionPattern::Array {
                prefix: vec![wildcard()].into_boxed_slice(),
                rest: None,
                suffix: Box::new([]),
            };
        assert!(!structurally_irrefutable_for_schema(
            &schemas, &short, expected
        ));

        let rest: super::super::CollectionPattern<SchemaId, MatchPatternValue> =
            super::super::CollectionPattern::Array {
                prefix: vec![wildcard()].into_boxed_slice(),
                rest: Some(Box::new(wildcard())),
                suffix: Box::new([]),
            };
        assert!(structurally_irrefutable_for_schema(
            &schemas, &rest, expected
        ));

        let oversized_rest: super::super::CollectionPattern<SchemaId, MatchPatternValue> =
            super::super::CollectionPattern::Array {
                prefix: vec![wildcard(), wildcard(), wildcard()].into_boxed_slice(),
                rest: Some(Box::new(wildcard())),
                suffix: Box::new([]),
            };
        assert!(!structurally_irrefutable_for_schema(
            &schemas,
            &oversized_rest,
            expected
        ));
    }

    #[test]
    fn narrowed_dynamic_bindings_are_refutable_for_enum_coverage() {
        let mut builder = SchemaTableBuilder::new();
        let dynamic = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Dynamic,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let f64 = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::FloatingPoint(FloatWidth::W64),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let built = builder.finish().unwrap();
        let dynamic = built.resolve(dynamic).unwrap();
        let f64 = built.resolve(f64).unwrap();
        let schemas = built.table;
        let expected = schemas.get(dynamic).unwrap();

        let narrowed: super::super::CollectionPattern<SchemaId, MatchPatternValue> =
            super::super::CollectionPattern::Bind {
                local: 0,
                schema: f64,
            };
        assert!(!structurally_irrefutable_for_schema(
            &schemas, &narrowed, expected
        ));

        let exact: super::super::CollectionPattern<SchemaId, MatchPatternValue> =
            super::super::CollectionPattern::Bind {
                local: 0,
                schema: dynamic,
            };
        assert!(structurally_irrefutable_for_schema(
            &schemas, &exact, expected
        ));
    }

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
