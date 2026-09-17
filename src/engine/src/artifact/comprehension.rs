//! Lexical collection control. Qualifiers and patterns are executable artifact
//! data; source maps only describe where these declarations originated.

use mech_core::{ConstantId, OperationContractId, SchemaId};

use super::OperationReference;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComprehensionValue {
    Constant(ConstantId),
    /// Ordinal in the enclosing node's input bindings.
    Input(u16),
    /// Dense, single-writer lexical identity, visible after its defining step.
    Local(u32),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CollectionPattern<S = SchemaId, V = ComprehensionValue> {
    Wildcard,
    Bind {
        local: u32,
        schema: S,
    },
    /// A repeated binding is equality, including across generators (a join).
    Equal(V),
    Tuple(Box<[CollectionPattern<S, V>]>),
    Array {
        prefix: Box<[CollectionPattern<S, V>]>,
        /// None requires exact length; a wildcard accepts an ignored middle.
        rest: Option<Box<CollectionPattern<S, V>>>,
        suffix: Box<[CollectionPattern<S, V>]>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComprehensionOperation<C = OperationContractId, S = SchemaId, V = ComprehensionValue> {
    pub local: u32,
    pub operation: OperationReference,
    pub contract: C,
    pub inputs: Box<[V]>,
    pub schema: S,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ComprehensionStep<C = OperationContractId, S = SchemaId, V = ComprehensionValue> {
    Generator {
        source: V,
        pattern: CollectionPattern<S, V>,
    },
    Operation(ComprehensionOperation<C, S, V>),
    Filter(V),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComprehensionKind {
    Matrix,
    Set,
}

/// Steps execute in source order for every binding of preceding generators.
/// A failed pattern or filter skips the current binding, not the whole query.
/// Zero generators describe one lexical evaluation. The declared kind owns
/// collection construction; the output schema validates its yielded elements.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComprehensionDeclaration<C = OperationContractId, S = SchemaId, V = ComprehensionValue> {
    pub kind: ComprehensionKind,
    pub steps: Box<[ComprehensionStep<C, S, V>]>,
    pub yield_value: V,
}

impl<C> ComprehensionDeclaration<C> {
    pub(super) fn operations(&self) -> impl Iterator<Item = &ComprehensionOperation<C>> {
        self.steps.iter().filter_map(|step| match step {
            ComprehensionStep::Operation(operation) => Some(operation),
            _ => None,
        })
    }

    pub(super) fn map_contracts<D, E>(
        &self,
        mut map: impl FnMut(&ComprehensionOperation<C>) -> Result<D, E>,
    ) -> Result<ComprehensionDeclaration<D>, E> {
        Ok(ComprehensionDeclaration {
            kind: self.kind,
            steps: self
                .steps
                .iter()
                .map(|step| {
                    Ok(match step {
                        ComprehensionStep::Generator { source, pattern } => {
                            ComprehensionStep::Generator {
                                source: *source,
                                pattern: pattern.clone(),
                            }
                        }
                        ComprehensionStep::Filter(value) => ComprehensionStep::Filter(*value),
                        ComprehensionStep::Operation(operation) => {
                            ComprehensionStep::Operation(ComprehensionOperation {
                                local: operation.local,
                                operation: operation.operation.clone(),
                                contract: map(operation)?,
                                inputs: operation.inputs.clone(),
                                schema: operation.schema,
                            })
                        }
                    })
                })
                .collect::<Result<Box<[_]>, E>>()?,
            yield_value: self.yield_value,
        })
    }
}

/// Bounds apply before recursive pattern inspection, including hand-built
/// artifacts which do not pass through the bytecode decoder.
pub const MAX_COLLECTION_PATTERN_DEPTH: usize = 32;
pub const MAX_COLLECTION_GENERATORS: usize = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PatternMetrics {
    pub nodes: usize,
    pub bindings: usize,
    pub equalities: usize,
    /// Bindings/equalities whose candidate is the complete native dense lane
    /// or an array rest. Only these candidates require canonical snapshot
    /// finalization; scalar prefix/suffix leaves use direct resident lanes.
    pub dense_finalizations: usize,
    pub depth: usize,
}

pub(crate) fn pattern_metrics<S, V>(pattern: &CollectionPattern<S, V>) -> Option<PatternMetrics> {
    let mut pending = vec![(pattern, 1usize, true)];
    let mut count = 0usize;
    let mut bindings = 0usize;
    let mut equalities = 0usize;
    let mut dense_finalizations = 0usize;
    let mut max_depth = 0usize;
    while let Some((pattern, depth, dense_candidate)) = pending.pop() {
        count = count.checked_add(1)?;
        if depth > MAX_COLLECTION_PATTERN_DEPTH || count > super::MAX_CONTROL_OPERANDS {
            return None;
        }
        max_depth = max_depth.max(depth);
        if matches!(pattern, CollectionPattern::Bind { .. }) {
            bindings = bindings.checked_add(1)?;
        }
        if matches!(pattern, CollectionPattern::Equal(_)) {
            equalities = equalities.checked_add(1)?;
        }
        if dense_candidate
            && matches!(
                pattern,
                CollectionPattern::Bind { .. } | CollectionPattern::Equal(_)
            )
        {
            dense_finalizations = dense_finalizations.checked_add(1)?;
        }
        let children = match pattern {
            CollectionPattern::Tuple(items) => items.len(),
            CollectionPattern::Array {
                prefix,
                rest,
                suffix,
            } => prefix
                .len()
                .checked_add(suffix.len())?
                .checked_add(usize::from(rest.is_some()))?,
            _ => 0,
        };
        if pending.len().checked_add(children)? > super::MAX_CONTROL_OPERANDS {
            return None;
        }
        match pattern {
            CollectionPattern::Tuple(items) => {
                pending.extend(items.iter().map(|item| (item, depth + 1, false)));
            }
            CollectionPattern::Array {
                prefix,
                rest,
                suffix,
            } => {
                pending.extend(
                    prefix
                        .iter()
                        .chain(suffix.iter())
                        .map(|item| (item, depth + 1, false)),
                );
                pending.extend(rest.iter().map(|item| (item.as_ref(), depth + 1, true)));
            }
            _ => {}
        }
        if pending.len() > super::MAX_CONTROL_OPERANDS {
            return None;
        }
    }
    Some(PatternMetrics {
        nodes: count,
        bindings,
        equalities,
        dense_finalizations,
        depth: max_depth,
    })
}

pub(crate) fn pattern_counts<S, V>(pattern: &CollectionPattern<S, V>) -> Option<usize> {
    pattern_metrics(pattern).map(|metrics| metrics.nodes)
}

pub(super) fn value_schema(
    value: ComprehensionValue,
    constants: &mech_core::ConstantStore,
    inputs: &[SchemaId],
    locals: &[SchemaId],
) -> Option<SchemaId> {
    match value {
        ComprehensionValue::Constant(id) => constants.get(id).map(|value| value.schema()),
        ComprehensionValue::Input(ordinal) => inputs.get(ordinal as usize).copied(),
        ComprehensionValue::Local(local) => locals.get(local as usize).copied(),
    }
}

pub(super) fn pattern_locals(
    pattern: &CollectionPattern,
    locals: &mut Vec<SchemaId>,
) -> Option<()> {
    match pattern {
        CollectionPattern::Bind { local, schema } => {
            if *local as usize != locals.len() {
                return None;
            }
            locals.push(*schema);
        }
        CollectionPattern::Tuple(items) => {
            for item in items {
                pattern_locals(item, locals)?;
            }
        }
        CollectionPattern::Array {
            prefix,
            rest,
            suffix,
        } => {
            for item in prefix
                .iter()
                .chain(rest.iter().map(Box::as_ref))
                .chain(suffix.iter())
            {
                pattern_locals(item, locals)?;
            }
        }
        _ => {}
    }
    Some(())
}

pub(super) fn validate_comprehension(
    draft: &super::ProgramArtifactDraft,
    node: mech_core::NodeId,
    declaration: &ComprehensionDeclaration,
    inputs: &[SchemaId],
    output: SchemaId,
) -> Result<(), super::ArtifactBuildError> {
    use mech_core::{
        AccessMode, AliasPolicy, DeliveryMode, ExternalInteraction, OutputConstruction,
        ResolvedOperationContract, SchemaBody,
    };
    let invalid = |reason| super::ArtifactBuildError::InvalidControl { node, reason };
    let mut locals = Vec::new();
    if declaration
        .steps
        .iter()
        .filter(|step| matches!(step, ComprehensionStep::Generator { .. }))
        .count()
        > MAX_COLLECTION_GENERATORS
    {
        return Err(invalid("collection generator nesting limit"));
    }
    for step in &declaration.steps {
        match step {
            ComprehensionStep::Generator { source, pattern } => {
                let source = value_schema(*source, &draft.constants, inputs, &locals)
                    .and_then(|id| draft.schemas.get(id))
                    .ok_or_else(|| invalid("unknown generator source"))?;
                let element = match source.body() {
                    SchemaBody::Matrix { element, .. } | SchemaBody::Set { element, .. } => {
                        element.as_ref()
                    }
                    SchemaBody::Dynamic => source.body(),
                    _ => return Err(invalid("generator source is not a collection")),
                };
                pattern_counts(pattern)
                    .ok_or_else(|| invalid("collection pattern admission limit"))?;
                let element = component_schema(source, element)
                    .ok_or_else(|| invalid("invalid generator component schema"))?;
                validate_pattern(draft, pattern, &element, inputs, &mut locals)
                    .ok_or_else(|| invalid("invalid collection pattern or binding"))?;
            }
            ComprehensionStep::Filter(value) => {
                if !value_schema(*value, &draft.constants, inputs, &locals)
                    .and_then(|id| draft.schemas.get(id))
                    .is_some_and(|schema| matches!(schema.body(), SchemaBody::Bool))
                {
                    return Err(invalid("collection filter requires Bool"));
                }
            }
            ComprehensionStep::Operation(operation) => {
                if operation.local as usize != locals.len()
                    || draft.schemas.get(operation.schema).is_none()
                {
                    return Err(invalid("noncanonical collection local identity or schema"));
                }
                super::validation::validate_operation(&operation.operation)?;
                let Some(ResolvedOperationContract::Declared(contract)) =
                    draft.contracts.get(operation.contract)
                else {
                    return Err(invalid(
                        "collection operation requires an ordinary contract",
                    ));
                };
                if contract.interaction != ExternalInteraction::Pure
                    || contract.inputs.len() != operation.inputs.len()
                    || contract.outputs.len() != 1
                {
                    return Err(invalid(
                        "collection operations must be pure single-result calls",
                    ));
                }
                for (value, port) in operation.inputs.iter().zip(contract.inputs.iter()) {
                    if value_schema(*value, &draft.constants, inputs, &locals) != Some(port.schema)
                        || port.access != AccessMode::Read
                        || port.delivery != DeliveryMode::Signal
                    {
                        return Err(invalid("invalid collection operation input"));
                    }
                }
                let port = &contract.outputs[0];
                if port.schema != operation.schema
                    || port.alias != AliasPolicy::NoAlias
                    || port.delivery != DeliveryMode::Signal
                    || port.access != AccessMode::Write
                    || !matches!(
                        port.construction,
                        OutputConstruction::FullWrite { .. } | OutputConstruction::Build { .. }
                    )
                {
                    return Err(invalid("invalid collection operation output"));
                }
                locals.push(operation.schema);
            }
        }
    }
    let yielded = value_schema(declaration.yield_value, &draft.constants, inputs, &locals)
        .and_then(|id| draft.schemas.get(id))
        .ok_or_else(|| invalid("unknown collection yield"))?;
    let output = draft
        .schemas
        .get(output)
        .ok_or_else(|| invalid("unknown collection output"))?;
    let element = match output.body() {
        SchemaBody::Matrix {
            element,
            dimensions,
        } if dimensions.len() == 2
            && dimensions[0] == mech_core::DimensionExpr::Constant(1)
            && declaration.kind == ComprehensionKind::Matrix =>
        {
            element
        }
        SchemaBody::Set { element, .. } if declaration.kind == ComprehensionKind::Set => element,
        SchemaBody::Dynamic if matches!(yielded.body(), SchemaBody::Dynamic) => return Ok(()),
        _ => return Err(invalid("collection result requires a matrix or set schema")),
    };
    if yielded.body() != element.as_ref() || !yielded.dimension_parameters().is_empty() {
        return Err(invalid(
            "collection yield does not match its element schema",
        ));
    }
    Ok(())
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

pub(super) fn array_rest_schema(
    parent: &mech_core::Schema,
    element: &mech_core::SchemaBody,
) -> Option<mech_core::Schema> {
    let mut parameters = parent
        .dimension_parameters()
        .iter()
        .enumerate()
        .map(|(id, parameter)| {
            Some(mech_core::DimensionParameterDeclaration {
                id: mech_core::DimensionParameterId::new(u32::try_from(id).ok()?),
                origin: mech_core::DimensionParameterOrigin::Explicit,
                lifetime: parameter.lifetime(),
                lower_bound: parameter.lower_bound().clone(),
                upper_bound: parameter.upper_bound().cloned(),
            })
        })
        .collect::<Option<Vec<_>>>()?;
    let extent = mech_core::DimensionParameterId::new(u32::try_from(parameters.len()).ok()?);
    parameters.push(mech_core::DimensionParameterDeclaration {
        id: extent,
        origin: mech_core::DimensionParameterOrigin::Inferred,
        lifetime: mech_core::DimensionLifetime::Turn,
        lower_bound: mech_core::DimensionExpr::Constant(0),
        upper_bound: None,
    });
    mech_core::SchemaDraft {
        body: mech_core::SchemaBody::Matrix {
            element: Box::new(element.clone()),
            dimensions: vec![
                mech_core::DimensionExpr::Constant(1),
                mech_core::DimensionExpr::Parameter(extent),
            ]
            .into_boxed_slice(),
        },
        dimension_parameters: parameters.into_boxed_slice(),
    }
    .finalize()
    .ok()
}

fn validate_pattern(
    draft: &super::ProgramArtifactDraft,
    pattern: &CollectionPattern,
    expected: &mech_core::Schema,
    inputs: &[SchemaId],
    locals: &mut Vec<SchemaId>,
) -> Option<()> {
    use mech_core::SchemaBody;
    let compatible = |schema: &mech_core::Schema| {
        matches!(expected.body(), SchemaBody::Dynamic) || schema == expected
    };
    match pattern {
        CollectionPattern::Wildcard => {}
        CollectionPattern::Bind { local, schema } => {
            let definition = draft.schemas.get(*schema)?;
            if *local as usize != locals.len() || !compatible(definition) {
                return None;
            }
            locals.push(*schema);
        }
        CollectionPattern::Equal(value) => {
            let schema = value_schema(*value, &draft.constants, inputs, locals)?;
            if !compatible(draft.schemas.get(schema)?) {
                return None;
            }
        }
        CollectionPattern::Tuple(items) => {
            let fields = match expected.body() {
                SchemaBody::Tuple(fields) if fields.len() == items.len() => Some(fields),
                SchemaBody::Dynamic => None,
                _ => return None,
            };
            for (index, item) in items.iter().enumerate() {
                validate_pattern(
                    draft,
                    item,
                    &component_schema(
                        expected,
                        fields.map_or(&SchemaBody::Dynamic, |fields| &fields[index]),
                    )?,
                    inputs,
                    locals,
                )?;
            }
        }
        CollectionPattern::Array {
            prefix,
            rest,
            suffix,
        } => {
            let element = match expected.body() {
                SchemaBody::Matrix { element, .. } => element.as_ref(),
                SchemaBody::Dynamic => expected.body(),
                _ => return None,
            };
            for item in prefix.iter() {
                validate_pattern(
                    draft,
                    item,
                    &component_schema(expected, element)?,
                    inputs,
                    locals,
                )?;
            }
            if let Some(rest) = rest {
                validate_pattern(
                    draft,
                    rest,
                    &array_rest_schema(expected, element)?,
                    inputs,
                    locals,
                )?;
            }
            for item in suffix.iter() {
                validate_pattern(
                    draft,
                    item,
                    &component_schema(expected, element)?,
                    inputs,
                    locals,
                )?;
            }
        }
    }
    Some(())
}

impl<S, V> CollectionPattern<S, V> {
    #[cfg(feature = "source")]
    pub(crate) fn map<T, W>(
        &self,
        schema: &impl Fn(&S) -> T,
        value: &impl Fn(&V) -> W,
    ) -> CollectionPattern<T, W> {
        match self {
            Self::Wildcard => CollectionPattern::Wildcard,
            Self::Bind { local, schema: s } => CollectionPattern::Bind {
                local: *local,
                schema: schema(s),
            },
            Self::Equal(v) => CollectionPattern::Equal(value(v)),
            Self::Tuple(items) => {
                CollectionPattern::Tuple(items.iter().map(|item| item.map(schema, value)).collect())
            }
            Self::Array {
                prefix,
                rest,
                suffix,
            } => CollectionPattern::Array {
                prefix: prefix.iter().map(|item| item.map(schema, value)).collect(),
                rest: rest.as_ref().map(|item| Box::new(item.map(schema, value))),
                suffix: suffix.iter().map(|item| item.map(schema, value)).collect(),
            },
        }
    }

    pub(crate) fn bindings(&self, visit: &mut impl FnMut(u32, &S)) {
        match self {
            Self::Bind { local, schema } => visit(*local, schema),
            Self::Tuple(items) => {
                for item in items {
                    item.bindings(visit);
                }
            }
            Self::Array {
                prefix,
                rest,
                suffix,
            } => {
                for item in prefix
                    .iter()
                    .chain(rest.iter().map(Box::as_ref))
                    .chain(suffix.iter())
                {
                    item.bindings(visit);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod schema_tests {
    use super::*;
    use mech_core::{
        CardinalitySpec, ConstantStoreBuilder, DimensionExpr, DimensionLifetime,
        DimensionParameterDeclaration, DimensionParameterId, DimensionParameterOrigin, FloatWidth,
        NodeId, OperationContractTableBuilder, SchemaBody, SchemaDraft, SchemaTableBuilder,
        ValueDataDraft, ValueDraft,
    };

    #[test]
    fn dense_finalization_metrics_count_only_whole_or_rest_candidates() {
        let pattern = CollectionPattern::Array {
            prefix: vec![CollectionPattern::Bind {
                local: 0,
                schema: SchemaId::new(0),
            }]
            .into_boxed_slice(),
            rest: Some(Box::new(CollectionPattern::Bind {
                local: 1,
                schema: SchemaId::new(1),
            })),
            suffix: vec![CollectionPattern::Equal(ComprehensionValue::Constant(
                mech_core::ConstantId::new(0),
            ))]
            .into_boxed_slice(),
        };
        let metrics = pattern_metrics(&pattern).unwrap();
        assert_eq!(metrics.bindings, 2);
        assert_eq!(metrics.equalities, 1);
        assert_eq!(metrics.dense_finalizations, 1);
    }

    #[test]
    fn lexical_collection_pattern_schemas_preserve_component_bounds_and_lifetimes() {
        let matrix = SchemaBody::Matrix {
            element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
            dimensions: vec![
                DimensionExpr::Constant(1),
                DimensionExpr::Parameter(DimensionParameterId::new(0)),
            ]
            .into_boxed_slice(),
        };
        let schema = |body, lifetime, upper| {
            SchemaDraft {
                body,
                dimension_parameters: vec![DimensionParameterDeclaration {
                    id: DimensionParameterId::new(0),
                    origin: DimensionParameterOrigin::Explicit,
                    lifetime,
                    lower_bound: DimensionExpr::Constant(0),
                    upper_bound: Some(DimensionExpr::Constant(upper)),
                }]
                .into_boxed_slice(),
            }
            .finalize()
            .unwrap()
        };
        let expected = schema(matrix.clone(), DimensionLifetime::Activation, 10);
        let mut builder = SchemaTableBuilder::new();
        let correct = builder.insert(expected.clone()).unwrap();
        let wrong_bound = builder
            .insert(schema(matrix.clone(), DimensionLifetime::Activation, 20))
            .unwrap();
        let wrong_lifetime = builder
            .insert(schema(matrix.clone(), DimensionLifetime::Turn, 10))
            .unwrap();
        let input = builder
            .insert(schema(
                SchemaBody::Matrix {
                    element: Box::new(SchemaBody::Tuple(vec![matrix].into_boxed_slice())),
                    dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(2)]
                        .into_boxed_slice(),
                },
                DimensionLifetime::Activation,
                10,
            ))
            .unwrap();
        let boolean = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Bool,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let output = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Set {
                        element: Box::new(SchemaBody::Bool),
                        cardinality: CardinalitySpec::Dynamic { upper_bound: None },
                    },
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let built = builder.finish().unwrap();
        let (correct, wrong_bound, wrong_lifetime, input, boolean, output) = (
            built.resolve(correct).unwrap(),
            built.resolve(wrong_bound).unwrap(),
            built.resolve(wrong_lifetime).unwrap(),
            built.resolve(input).unwrap(),
            built.resolve(boolean).unwrap(),
            built.resolve(output).unwrap(),
        );
        let schemas = built.into_parts().0;
        let mut constants = ConstantStoreBuilder::new(&schemas);
        let value = constants
            .insert(
                ValueDraft {
                    schema: boolean,
                    shape_values: Box::new([]),
                    data: ValueDataDraft::Bool(true),
                }
                .finalize(&mech_core::snapshot::SnapshotValidationContext::new(
                    &schemas,
                ))
                .unwrap(),
            )
            .unwrap();
        let constants = constants.finish().unwrap();
        let value = constants.resolve(value).unwrap();
        let draft = crate::ProgramArtifactDraft {
            schemas,
            constants: constants.into_parts().0,
            contracts: OperationContractTableBuilder::new()
                .finish()
                .unwrap()
                .into_parts()
                .0,
            requirements: Default::default(),
            inputs: Box::new([]),
            slots: Box::new([]),
            nodes: Box::new([]),
            bindings: Box::new([]),
            outputs: Box::new([]),
            constraints: Box::new([]),
            compute_regions: Box::new([]),
        };
        for local_schema in [correct, wrong_bound, wrong_lifetime] {
            let control = ComprehensionDeclaration {
                kind: ComprehensionKind::Set,
                steps: vec![ComprehensionStep::Generator {
                    source: ComprehensionValue::Input(0),
                    pattern: CollectionPattern::Tuple(
                        vec![CollectionPattern::Bind {
                            local: 0,
                            schema: local_schema,
                        }]
                        .into_boxed_slice(),
                    ),
                }]
                .into_boxed_slice(),
                yield_value: ComprehensionValue::Constant(value),
            };
            let result = validate_comprehension(&draft, NodeId::new(0), &control, &[input], output);
            assert_eq!(
                result.is_ok(),
                local_schema == correct,
                "{local_schema:?}: {result:?}"
            );
        }
    }
}
