use super::*;
use crate::{
    CollectionPattern, ComprehensionDeclaration, ComprehensionOperation, ComprehensionStep,
    ComprehensionValue,
};

#[derive(Clone, Copy)]
pub(super) enum PendingCollectionValue {
    Constant(usize),
    Input(u16),
    Local(u32),
}

pub(super) type PendingComprehension =
    ComprehensionDeclaration<OperationContractDeclaration, SchemaDraft, PendingCollectionValue>;
pub(super) type SourcePattern = CollectionPattern<PendingValue, PendingValue>;

enum Qualifier {
    Generator {
        source: PendingValue,
        pattern: SourcePattern,
    },
    Filter(PendingValue),
}

fn unsupported(syntax: &SyntaxNode, message: &str) -> SourceSemanticError {
    SourceSemanticError {
        code: "source-semantics/unsupported-comprehension-control",
        message: message.to_owned(),
        anchor: SourceSemanticAnchor::for_node(syntax),
    }
}

fn canonical_component_schema_draft(
    parent: &SchemaDraft,
    body: &SchemaBody,
    syntax: &SyntaxNode,
) -> Result<SchemaDraft, SourceSemanticError> {
    let component = SchemaDraft {
        body: body.clone(),
        dimension_parameters: parent.dimension_parameters.clone(),
    }
    .finalize()
    .map_err(|error| {
        internal(
            SourceSemanticAnchor::for_node(syntax),
            format!("unable to canonicalize collection component schema: {error:?}"),
        )
    })?;
    Ok(SchemaDraft {
        body: component.body().clone(),
        dimension_parameters: component
            .dimension_parameters()
            .iter()
            .enumerate()
            .map(|(id, parameter)| DimensionParameterDeclaration {
                id: DimensionParameterId::new(id as u32),
                origin: DimensionParameterOrigin::Explicit,
                lifetime: parameter.lifetime(),
                lower_bound: parameter.lower_bound().clone(),
                upper_bound: parameter.upper_bound().cloned(),
            })
            .collect(),
    })
}

fn array_rest_schema(
    element: &SchemaDraft,
    syntax: &SyntaxNode,
) -> Result<SchemaDraft, SourceSemanticError> {
    let mut parameters = element.dimension_parameters.to_vec();
    let extent =
        DimensionParameterId::new(u32::try_from(parameters.len()).map_err(|_| {
            unsupported(syntax, "array-rest dimension identity space was exhausted")
        })?);
    parameters.push(DimensionParameterDeclaration {
        id: extent,
        origin: DimensionParameterOrigin::Inferred,
        lifetime: DimensionLifetime::Turn,
        lower_bound: DimensionExpr::Constant(0),
        upper_bound: None,
    });
    Ok(SchemaDraft {
        dimension_parameters: parameters.into_boxed_slice(),
        body: SchemaBody::Matrix {
            element: Box::new(element.body.clone()),
            dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Parameter(extent)]
                .into_boxed_slice(),
        },
    })
}

impl SemanticBuilder {
    pub(super) fn comprehension(
        &mut self,
        syntax: &SyntaxNode,
        result: Option<ExpressionSyntax>,
        qualifiers: Vec<mech_syntax::document::ComprehensionQualifierSyntax>,
        operation: &'static str,
    ) -> Result<PendingValue, SourceSemanticError> {
        let saved = self.bindings.clone();
        let saved_definitions = core::mem::take(&mut self.scope_definitions);
        let start = self.nodes.len();
        self.control_depth += 1;
        let compiled = self.collection_body(syntax, result, qualifiers, operation, start);
        self.control_depth -= 1;
        self.bindings = saved;
        self.scope_definitions = saved_definitions;
        compiled
    }

    fn collection_body(
        &mut self,
        syntax: &SyntaxNode,
        result: Option<ExpressionSyntax>,
        qualifiers: Vec<mech_syntax::document::ComprehensionQualifierSyntax>,
        operation: &'static str,
        start: usize,
    ) -> Result<PendingValue, SourceSemanticError> {
        let mut events = Vec::new();
        let mut names = BTreeMap::new();
        for qualifier in qualifiers {
            match self.required(
                qualifier.value(),
                qualifier.syntax(),
                "a collection qualifier",
            )? {
                ComprehensionQualifierValueSyntax::Generator(generator) => {
                    let source = self.required(
                        generator.source(),
                        generator.syntax(),
                        "a generator source",
                    )?;
                    let source = self.expression(&source)?.0;
                    let schema = self.schema_draft_of(source)?;
                    let element = match &schema.body {
                        SchemaBody::Matrix { element, .. } | SchemaBody::Set { element, .. } => {
                            canonical_component_schema_draft(&schema, element, generator.syntax())?
                        }
                        SchemaBody::Dynamic => schema,
                        _ => {
                            return Err(unsupported(
                                generator.syntax(),
                                "a generator requires a matrix or set",
                            ));
                        }
                    };
                    let pattern = self.required(
                        generator.pattern(),
                        generator.syntax(),
                        "a generator pattern",
                    )?;
                    let before_bindings = self.nodes.len();
                    let pattern = self.collection_pattern(&pattern, &element, start, &mut names)?;
                    let mut pattern_values = Vec::new();
                    collect_pattern_values(&pattern, &mut pattern_values);
                    if pattern_values.iter().any(|value| matches!(value, PendingValue::Node(index)
                        if *index as usize >= before_bindings && !matches!(self.nodes[*index as usize].body, PendingNodeBody::CollectionBinding))) {
                        return Err(unsupported(generator.syntax(), "computed pattern expressions require an executable pattern evaluation block"));
                    }
                    events.push((before_bindings, Qualifier::Generator { source, pattern }));
                }
                ComprehensionQualifierValueSyntax::Definition(definition) => {
                    if definition.mutability_marker().is_some() {
                        return Err(unsupported(
                            definition.syntax(),
                            "collection definitions are immutable lexical values",
                        ));
                    }
                    let (value, _) = self.definition(&definition)?;
                    let variable = self.required(
                        definition.variable(),
                        definition.syntax(),
                        "a lexical definition",
                    )?;
                    let stem =
                        self.required(variable.stem(), variable.syntax(), "a lexical name")?;
                    names.insert(node_text(stem.syntax())?, value);
                }
                ComprehensionQualifierValueSyntax::Filter(filter) => {
                    let value = self.expression(&filter)?.0;
                    let value = self.require_boolean_operand(value, filter.syntax())?;
                    events.push((self.nodes.len(), Qualifier::Filter(value)));
                }
            }
        }
        let result = self.required(result, syntax, "a collection yield")?;
        let result = self.expression(&result)?.0;
        let element = self.schema_draft_of(result)?;
        // A dynamically shaped element needs its own nested shape witness;
        // it cannot borrow the result collection's cardinality parameter.
        if !element.dimension_parameters.is_empty() {
            return Err(unsupported(
                syntax,
                "collection elements require a closed shape",
            ));
        }
        let kind = if operation == "set/comprehension" {
            crate::ComprehensionKind::Set
        } else {
            crate::ComprehensionKind::Matrix
        };
        let output = if is_dynamic_schema_draft(&element) {
            element
        } else if kind == crate::ComprehensionKind::Set {
            if !is_dynamic_schema_draft(&element) {
                require_keyable_set_element(&element.body, &element.dimension_parameters, syntax)?;
            }
            SchemaDraft {
                dimension_parameters: Box::new([]),
                body: SchemaBody::Set {
                    element: Box::new(element.body),
                    cardinality: CardinalitySpec::Dynamic { upper_bound: None },
                },
            }
        } else {
            let extent = DimensionParameterId::new(0);
            SchemaDraft {
                dimension_parameters: vec![DimensionParameterDeclaration {
                    id: extent,
                    origin: DimensionParameterOrigin::Inferred,
                    lifetime: DimensionLifetime::Turn,
                    lower_bound: DimensionExpr::Constant(0),
                    upper_bound: None,
                }]
                .into_boxed_slice(),
                body: SchemaBody::Matrix {
                    element: Box::new(element.body),
                    dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Parameter(extent)]
                        .into_boxed_slice(),
                },
            }
        };
        let mut inputs = Vec::new();
        let mut capture =
            |value: PendingValue| -> Result<PendingCollectionValue, SourceSemanticError> {
                match value {
                    PendingValue::Constant(id) => Ok(PendingCollectionValue::Constant(id)),
                    PendingValue::Node(index) if index as usize >= start => {
                        Ok(PendingCollectionValue::Local(index - start as u32))
                    }
                    PendingValue::UnresolvedEmpty(anchor) => Err(unresolved_empty(anchor)),
                    _ => {
                        let ordinal = match inputs.iter().position(|existing| *existing == value) {
                            Some(ordinal) => ordinal,
                            None => {
                                inputs.push(value);
                                inputs.len() - 1
                            }
                        };
                        Ok(PendingCollectionValue::Input(
                            u16::try_from(ordinal)
                                .map_err(|_| unsupported(syntax, "too many collection captures"))?,
                        ))
                    }
                }
            };
        let mut converted_events = Vec::new();
        for (position, event) in events {
            let event = match event {
                Qualifier::Filter(value) => ComprehensionStep::Filter(capture(value)?),
                Qualifier::Generator { source, pattern } => {
                    // Resolve inferred binder schemas only after the entire
                    // lexical body has contributed its ordinary type constraints.
                    let mut values = Vec::new();
                    collect_pattern_values(&pattern, &mut values);
                    let converted = values
                        .iter()
                        .copied()
                        .map(&mut capture)
                        .collect::<Result<Vec<_>, _>>()?;
                    let pattern = pattern.map(
                        &|value| {
                            self.schema_draft_of(*value)
                                .expect("retained lexical binding")
                        },
                        &|value| {
                            converted[values
                                .iter()
                                .position(|candidate| candidate == value)
                                .unwrap()]
                        },
                    );
                    ComprehensionStep::Generator {
                        source: capture(source)?,
                        pattern,
                    }
                }
            };
            converted_events.push((position, event));
        }
        let yield_value = capture(result)?;
        let mut steps = Vec::new();
        let nodes = self.nodes.split_off(start);
        let mut events = converted_events.into_iter().peekable();
        for (offset, node) in nodes.into_iter().enumerate() {
            while events
                .peek()
                .is_some_and(|(position, _)| *position == start + offset)
            {
                steps.push(events.next().unwrap().1);
            }
            match node.body {
                PendingNodeBody::CollectionBinding => {}
                PendingNodeBody::Operation {
                    operation,
                    contract: Some(contract),
                    requirement: None,
                } if node.state.is_none()
                    && contract.interaction == mech_core::ExternalInteraction::Pure =>
                {
                    steps.push(ComprehensionStep::Operation(ComprehensionOperation {
                        local: offset as u32,
                        operation,
                        contract,
                        schema: node.schema,
                        inputs: node
                            .inputs
                            .into_iter()
                            .map(&mut capture)
                            .collect::<Result<Box<[_]>, _>>()?,
                    }));
                }
                _ => {
                    return Err(unsupported(
                        syntax,
                        "collection bodies require pure ordinary operations",
                    ));
                }
            }
        }
        steps.extend(events.map(|(_, event)| event));
        let node = self.nodes.len() as u32;
        self.nodes.push(PendingNode {
            body: PendingNodeBody::Comprehension(ComprehensionDeclaration {
                kind,
                steps: steps.into_boxed_slice(),
                yield_value,
            }),
            inferable_projection: false,
            inputs,
            schema: output,
            exposes_output: true,
            state: None,
            semantic: SourceSemanticNode {
                operation: operation.to_owned(),
                role: "comprehension",
                detail: None,
                anchor: SourceSemanticAnchor::for_node(syntax),
            },
        });
        Ok(PendingValue::Node(node))
    }

    pub(super) fn collection_pattern(
        &mut self,
        pattern: &PatternSyntax,
        expected: &SchemaDraft,
        start: usize,
        names: &mut BTreeMap<String, PendingValue>,
    ) -> Result<SourcePattern, SourceSemanticError> {
        Ok(
            match self.required(pattern.value(), pattern.syntax(), "a collection pattern")? {
                PatternValueSyntax::Wildcard(_) => CollectionPattern::Wildcard,
                PatternValueSyntax::Expression(expression) => {
                    if let Some(variable) = standalone_pattern_variable(&expression)
                        && let Some(VariableStemSyntax::Identifier(identifier)) = variable.stem()
                    {
                        let name = node_text(identifier.syntax())?;
                        if let Some(value) = names.get(&name).copied() {
                            if let Some(annotation) = variable.annotation() {
                                let mut annotation = annotation_schema_draft(&annotation)?;
                                let actual = self.schema_draft_of(value)?;
                                if !annotation.dimension_parameters.is_empty()
                                    && !is_dynamic_schema_draft(&actual)
                                {
                                    annotation = specialize_annotation_dimensions(
                                        &actual,
                                        &annotation,
                                        pattern.syntax(),
                                    )?;
                                }
                                self.conform_dynamic_to_schema(
                                    value,
                                    &annotation,
                                    pattern.syntax(),
                                )?;
                                if self.schema_draft_of(value)? != annotation {
                                    return Err(unsupported(
                                        pattern.syntax(),
                                        "repeated binding annotation differs from its established kind",
                                    ));
                                }
                            }
                            if !is_dynamic_schema_draft(expected) {
                                self.conform_dynamic_to_schema(value, expected, pattern.syntax())?;
                            }
                            let actual = self.schema_draft_of(value)?;
                            if !is_dynamic_schema_draft(expected) && actual != *expected {
                                return Err(unsupported(
                                    pattern.syntax(),
                                    "repeated binding differs from the collection element kind",
                                ));
                            }
                            CollectionPattern::Equal(value)
                        } else {
                            let mut schema = variable
                                .annotation()
                                .map(|annotation| annotation_schema_draft(&annotation))
                                .transpose()?
                                .unwrap_or_else(|| expected.clone());
                            if !schema.dimension_parameters.is_empty()
                                && !is_dynamic_schema_draft(expected)
                            {
                                schema = specialize_annotation_dimensions(
                                    expected,
                                    &schema,
                                    pattern.syntax(),
                                )?;
                            }
                            if !is_dynamic_schema_draft(expected) && schema != *expected {
                                return Err(unsupported(
                                    pattern.syntax(),
                                    "pattern annotation differs from the collection element",
                                ));
                            }
                            let node = self.nodes.len() as u32;
                            let value = PendingValue::Node(node);
                            self.nodes.push(PendingNode {
                                body: PendingNodeBody::CollectionBinding,
                                inferable_projection: variable.annotation().is_none(),
                                inputs: Vec::new(),
                                schema,
                                exposes_output: true,
                                state: None,
                                semantic: SourceSemanticNode {
                                    operation: String::new(),
                                    role: "collection-binding",
                                    detail: None,
                                    anchor: SourceSemanticAnchor::for_node(pattern.syntax()),
                                },
                            });
                            names.insert(name.clone(), value);
                            self.bindings.insert(name, PendingBinding::Value(value));
                            CollectionPattern::Bind {
                                local: node - start as u32,
                                schema: value,
                            }
                        }
                    } else {
                        CollectionPattern::Equal(self.expression(&expression)?.0)
                    }
                }
                PatternValueSyntax::Tuple(tuple) => self.collection_tuple_pattern(
                    tuple.syntax(),
                    tuple.items(),
                    expected,
                    start,
                    names,
                )?,
                PatternValueSyntax::AtomStruct(tuple) => {
                    let name = self.required(tuple.name(), tuple.syntax(), "a pattern tag")?;
                    self.collection_tag_pattern(
                        name.syntax(),
                        tuple.items(),
                        expected,
                        start,
                        names,
                    )?
                }
                PatternValueSyntax::TupleStruct(tuple) => {
                    let name = self.required(tuple.name(), tuple.syntax(), "a pattern tag")?;
                    self.collection_tag_pattern(
                        name.syntax(),
                        tuple.items(),
                        expected,
                        start,
                        names,
                    )?
                }
                PatternValueSyntax::Array(array) => {
                    let element = match &expected.body {
                        SchemaBody::Matrix { element, .. } => {
                            canonical_component_schema_draft(expected, element, pattern.syntax())?
                        }
                        SchemaBody::Dynamic => expected.clone(),
                        _ => {
                            return Err(unsupported(
                                pattern.syntax(),
                                "array pattern requires a matrix element",
                            ));
                        }
                    };
                    let mut prefix = Vec::new();
                    let mut suffix = Vec::new();
                    let mut rest = None;
                    let mut bind_rest = false;
                    let rest_schema = array_rest_schema(&element, array.syntax())?;
                    for item in array.elements() {
                        if item.spread().is_some() || item.rest().is_some() {
                            if rest.is_some() || bind_rest {
                                return Err(unsupported(
                                    item.syntax(),
                                    "an array pattern has one rest position",
                                ));
                            }
                            if let Some(pattern) = item.pattern() {
                                rest = Some(Box::new(self.collection_pattern(
                                    &pattern,
                                    &rest_schema,
                                    start,
                                    names,
                                )?));
                            } else if item.rest().is_some() {
                                bind_rest = true;
                            } else {
                                rest = Some(Box::new(CollectionPattern::Wildcard));
                            }
                        } else if let Some(pattern) = item.pattern() {
                            if bind_rest {
                                rest = Some(Box::new(self.collection_pattern(
                                    &pattern,
                                    &rest_schema,
                                    start,
                                    names,
                                )?));
                                bind_rest = false;
                            } else {
                                let pattern =
                                    self.collection_pattern(&pattern, &element, start, names)?;
                                if rest.is_some() {
                                    suffix.push(pattern);
                                } else {
                                    prefix.push(pattern);
                                }
                            }
                        }
                    }
                    if bind_rest {
                        return Err(unsupported(
                            array.syntax(),
                            "array rest marker requires a pattern",
                        ));
                    }
                    CollectionPattern::Array {
                        prefix: prefix.into_boxed_slice(),
                        rest,
                        suffix: suffix.into_boxed_slice(),
                    }
                }
            },
        )
    }

    fn collection_tuple_pattern(
        &mut self,
        syntax: &SyntaxNode,
        items: Vec<PatternSyntax>,
        expected: &SchemaDraft,
        start: usize,
        names: &mut BTreeMap<String, PendingValue>,
    ) -> Result<SourcePattern, SourceSemanticError> {
        let fields = match &expected.body {
            SchemaBody::Tuple(fields) if fields.len() == items.len() => Some(fields),
            SchemaBody::Dynamic => None,
            _ => {
                return Err(unsupported(
                    syntax,
                    "tuple pattern does not match the element schema",
                ));
            }
        };
        Ok(CollectionPattern::Tuple(
            items
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    let schema = match fields {
                        None => builtin_schema_draft(BuiltinSchema::Dynamic),
                        Some(fields) => canonical_component_schema_draft(
                            expected,
                            &fields[index],
                            item.syntax(),
                        )?,
                    };
                    self.collection_pattern(item, &schema, start, names)
                })
                .collect::<Result<Box<[_]>, _>>()?,
        ))
    }

    fn collection_tag_pattern(
        &mut self,
        name: &SyntaxNode,
        items: Vec<PatternSyntax>,
        expected: &SchemaDraft,
        start: usize,
        names: &mut BTreeMap<String, PendingValue>,
    ) -> Result<SourcePattern, SourceSemanticError> {
        let path = CanonicalNominalPath::new(
            node_text(name)?
                .split('/')
                .map(str::to_owned)
                .collect::<Vec<_>>(),
        )
        .map_err(|_| unsupported(name, "invalid collection pattern tag"))?;
        let tag = self.constant_exact(
            SchemaBody::Atom(NominalKey::from_path(NominalKind::Atom, &path)),
            ValueDataDraft::Atom,
        );
        let payload = match &expected.body {
            SchemaBody::Tuple(fields) if fields.len() == 2 => {
                canonical_component_schema_draft(expected, &fields[1], name)?
            }
            _ => builtin_schema_draft(BuiltinSchema::Dynamic),
        };
        Ok(CollectionPattern::Tuple(
            vec![
                CollectionPattern::Equal(tag),
                self.collection_tuple_pattern(name, items, &payload, start, names)?,
            ]
            .into_boxed_slice(),
        ))
    }
}

fn collect_pattern_values(pattern: &SourcePattern, values: &mut Vec<PendingValue>) {
    match pattern {
        CollectionPattern::Equal(value) => values.push(*value),
        CollectionPattern::Tuple(items) => {
            for item in items {
                collect_pattern_values(item, values);
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
                collect_pattern_values(item, values);
            }
        }
        _ => {}
    }
}

pub(super) fn resolve_comprehension(
    control: &PendingComprehension,
    schemas: &SchemaTable,
    constants: &[mech_core::ConstantId],
) -> ComprehensionDeclaration<OperationContractDeclaration> {
    let schema = |draft: &SchemaDraft| {
        schemas
            .find_by_key(
                draft
                    .clone()
                    .finalize()
                    .expect("validated collection schema")
                    .key(),
            )
            .expect("retained collection schema")
    };
    let value = |value: &PendingCollectionValue| match *value {
        PendingCollectionValue::Constant(index) => ComprehensionValue::Constant(constants[index]),
        PendingCollectionValue::Input(ordinal) => ComprehensionValue::Input(ordinal),
        PendingCollectionValue::Local(local) => ComprehensionValue::Local(local),
    };
    ComprehensionDeclaration {
        kind: control.kind,
        steps: control
            .steps
            .iter()
            .map(|step| match step {
                ComprehensionStep::Generator { source, pattern } => ComprehensionStep::Generator {
                    source: value(source),
                    pattern: pattern.map(&schema, &value),
                },
                ComprehensionStep::Filter(filter) => ComprehensionStep::Filter(value(filter)),
                ComprehensionStep::Operation(operation) => {
                    ComprehensionStep::Operation(ComprehensionOperation {
                        local: operation.local,
                        operation: operation.operation.clone(),
                        contract: operation.contract.clone(),
                        inputs: operation.inputs.iter().map(value).collect(),
                        schema: schema(&operation.schema),
                    })
                }
            })
            .collect(),
        yield_value: value(&control.yield_value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_syntax::document::parser::{canonical::parse_canonical_phase_2i_rule_for_test, rules};
    use mech_syntax::document::{ParseConfig, TextSnapshot};

    fn expression(source: &str) -> ExpressionSyntax {
        fn find(node: SyntaxNode) -> Option<ExpressionSyntax> {
            ExpressionSyntax::cast(node.clone()).or_else(|| node.children().find_map(find))
        }
        let parsed = parse_canonical_phase_2i_rule_for_test(
            TextSnapshot::new(DocumentId(822), Revision(70), source).unwrap(),
            rules::EXPRESSION,
            ParseConfig::default(),
        )
        .unwrap();
        assert!(
            parsed.is_strictly_clean(),
            "{source}: {:?}",
            parsed.diagnostics
        );
        assert_eq!(parsed.consumed.end.0 as usize, source.len(), "{source}");
        find(parsed.syntax()).unwrap()
    }

    fn compile(source: &str) -> CanonicalSourceProgram {
        CanonicalSourceFrontend
            .compile_expression(&expression(source))
            .unwrap_or_else(|error| panic!("{source}: {error:?}"))
    }

    #[test]
    fn lexical_collection_checks_repeated_annotations_and_pattern_scope() {
        for source in [
            "[x | x <- xs, x <- [1 2]]",
            "[x | x <- xs, x<f64> <- [1 2]]",
        ] {
            compile(source).compile_artifact().unwrap();
        }
        for source in [
            "[x | x <- [1 2], x<bool> <- [true false]]",
            "[1 | 1 + 1 <- [2 3]]",
        ] {
            let error = CanonicalSourceFrontend
                .compile_expression(&expression(source))
                .err()
                .expect("invalid or not yet executable pattern must fail at its source boundary");
            assert_eq!(
                error.code, "source-semantics/unsupported-comprehension-control",
                "{source}: {error:?}"
            );
        }
    }

    #[test]
    fn lexical_collection_artifacts_own_generators_patterns_filters_and_yields() {
        for source in [
            "[x | x <- xs]",
            "{x | x <- xs}",
            "[head | [head, ..., tail] <- xs]",
            "[x | [x] <- xs]",
            "[head | [head | tail] <- xs]",
            "[x | (x,y) <- xs]",
            "[x | :some(x) <- xs]",
            "[x | `some(x) <- xs]",
            "[item + 1 | item <- [1 2 3], item > 1]",
            "{item + 1 | item <- {1,2,3}, item > 1}",
            "[(x,y) | x <- [1 2], x <- [2 3], y <- [4 5]]",
        ] {
            let mut compiled = compile(source);
            let artifact = compiled
                .compile_artifact()
                .unwrap_or_else(|error| panic!("{source}: {error:?}"));
            assert!(
                artifact
                    .nodes()
                    .iter()
                    .any(|node| matches!(node.body, crate::ExecutableNodeBody::Comprehension(_)))
            );
            let bytes = crate::encode_program_artifact_bytecode_v1(&artifact).unwrap();
            let decoded = crate::decode_program_artifact_bytecode_v1(&bytes).unwrap();
            assert_eq!(artifact.revision(), decoded.revision(), "{source}");
            compiled.source_map = SourceSemanticMap::default();
            assert_eq!(
                bytes,
                crate::encode_program_artifact_bytecode_v1(&compiled.compile_artifact().unwrap())
                    .unwrap(),
                "diagnostics cannot own execution: {source}"
            );
        }
    }
    #[test]
    fn lexical_collection_aggregate_step_limits_match_default_decode() {
        let artifact = compile("[1 | x <- signal<[f64]:1,1>, true]")
            .compile_artifact()
            .unwrap();
        for count in [
            crate::MAX_CONTROL_OPERATIONS,
            crate::MAX_CONTROL_OPERATIONS + 1,
        ] {
            let mut draft = crate::ProgramArtifactDraft {
                schemas: artifact.schemas().clone(),
                constants: artifact.constants().clone(),
                contracts: artifact.contracts().clone(),
                requirements: artifact.requirements().clone(),
                inputs: artifact.inputs().into(),
                slots: artifact.slots().into(),
                nodes: artifact.nodes().into(),
                bindings: artifact.bindings().into(),
                outputs: artifact.outputs().into(),
                constraints: artifact.constraints().into(),
                compute_regions: artifact.compute_regions().into(),
            };
            let crate::ExecutableNodeBody::Comprehension(control) = &mut draft.nodes[0].body else {
                panic!("collection")
            };
            let mut steps = control.steps.to_vec();
            steps.resize(count, steps.last().unwrap().clone());
            control.steps = steps.into_boxed_slice();
            if count > crate::MAX_CONTROL_OPERATIONS {
                assert!(matches!(
                    draft.finalize(),
                    Err(crate::ArtifactBuildError::InvalidControl {
                        reason: "control graph admission limit",
                        ..
                    })
                ));
            } else {
                let artifact = draft.finalize().unwrap();
                let bytes = crate::encode_program_artifact_bytecode_v1(&artifact).unwrap();
                assert_eq!(
                    artifact.revision(),
                    crate::decode_program_artifact_bytecode_v1(&bytes)
                        .unwrap()
                        .revision()
                );
            }
        }
    }

    #[test]
    fn lexical_collection_rejects_malformed_executable_declarations() {
        let artifact = compile("[x + 1 | x <- signal<[f64]:2,3>, x > 1]")
            .compile_artifact()
            .unwrap();
        for mutation in 0..10 {
            let mut draft = crate::ProgramArtifactDraft {
                schemas: artifact.schemas().clone(),
                constants: artifact.constants().clone(),
                contracts: artifact.contracts().clone(),
                requirements: artifact.requirements().clone(),
                inputs: artifact.inputs().into(),
                slots: artifact.slots().into(),
                nodes: artifact.nodes().into(),
                bindings: artifact.bindings().into(),
                outputs: artifact.outputs().into(),
                constraints: artifact.constraints().into(),
                compute_regions: artifact.compute_regions().into(),
            };
            let control = draft
                .nodes
                .iter_mut()
                .find_map(|node| match &mut node.body {
                    crate::ExecutableNodeBody::Comprehension(control) => Some(control),
                    _ => None,
                })
                .unwrap();
            match mutation {
                0 => control.yield_value = crate::ComprehensionValue::Local(u32::MAX),
                1 => {
                    let crate::ComprehensionStep::Generator { source, .. } = &mut control.steps[0]
                    else {
                        panic!("fixture generator")
                    };
                    *source = control.yield_value;
                }
                2 => {
                    let filter = control
                        .steps
                        .iter_mut()
                        .find(|step| matches!(step, crate::ComprehensionStep::Filter(_)))
                        .unwrap();
                    *filter = crate::ComprehensionStep::Filter(crate::ComprehensionValue::Local(0));
                }
                3 => {
                    let crate::ComprehensionStep::Generator { source, .. } = &mut control.steps[0]
                    else {
                        panic!("fixture generator")
                    };
                    *source = crate::ComprehensionValue::Input(u16::MAX);
                }
                4 => {
                    let crate::ComprehensionStep::Generator { pattern, .. } = &mut control.steps[0]
                    else {
                        panic!("fixture generator")
                    };
                    let crate::CollectionPattern::Bind { local, .. } = pattern else {
                        panic!("fixture binder")
                    };
                    *local = 1;
                }
                5 => control.kind = crate::ComprehensionKind::Set,
                6 => {
                    control.steps =
                        vec![control.steps[0].clone(); crate::MAX_COLLECTION_GENERATORS + 1]
                            .into_boxed_slice()
                }
                7 => {
                    let crate::ComprehensionStep::Generator { pattern, .. } = &mut control.steps[0]
                    else {
                        panic!("fixture generator")
                    };
                    for _ in 0..crate::MAX_COLLECTION_PATTERN_DEPTH {
                        *pattern = crate::CollectionPattern::Tuple(
                            vec![pattern.clone()].into_boxed_slice(),
                        );
                    }
                }
                8 => {
                    let operation = control
                        .steps
                        .iter_mut()
                        .find_map(|step| match step {
                            crate::ComprehensionStep::Operation(operation) => Some(operation),
                            _ => None,
                        })
                        .unwrap();
                    operation.contract = mech_core::OperationContractId::new(u32::MAX);
                }
                9 => {
                    // A comprehension constructs one row even when its generator
                    // is a multi-row matrix. Reject a hand-built two-row result.
                    let source_schema = draft
                        .slots
                        .iter()
                        .find(|slot| slot.role == crate::SlotRole::Input)
                        .unwrap()
                        .schema;
                    for slot in draft.slots.iter_mut().filter(|slot| {
                        matches!(
                            slot.role,
                            crate::SlotRole::Derived | crate::SlotRole::Output
                        )
                    }) {
                        slot.schema = source_schema;
                    }
                }
                _ => unreachable!(),
            }
            assert!(
                matches!(
                    draft.finalize(),
                    Err(crate::ArtifactBuildError::InvalidControl { .. })
                ),
                "mutation {mutation}"
            );
        }
    }

    #[test]
    fn lexical_collection_codec_enforces_pattern_population_and_exact_depth() {
        let artifact = compile("[x | x <- xs]").compile_artifact().unwrap();
        let draft = |nodes| crate::ProgramArtifactDraft {
            schemas: artifact.schemas().clone(),
            constants: artifact.constants().clone(),
            contracts: artifact.contracts().clone(),
            requirements: artifact.requirements().clone(),
            inputs: artifact.inputs().into(),
            slots: artifact.slots().into(),
            nodes,
            bindings: artifact.bindings().into(),
            outputs: artifact.outputs().into(),
            constraints: artifact.constraints().into(),
            compute_regions: artifact.compute_regions().into(),
        };
        for depth in [1, crate::MAX_COLLECTION_PATTERN_DEPTH] {
            let mut nodes = artifact.nodes().to_vec().into_boxed_slice();
            let crate::ExecutableNodeBody::Comprehension(control) = &mut nodes[0].body else {
                panic!("collection")
            };
            let crate::ComprehensionStep::Generator { pattern, .. } = &mut control.steps[0] else {
                panic!("generator")
            };
            for _ in 1..depth {
                *pattern =
                    crate::CollectionPattern::Tuple(vec![pattern.clone()].into_boxed_slice());
            }
            let artifact = draft(nodes).finalize().unwrap();
            let sections = crate::encode_program_artifact_sections(&artifact).unwrap();
            let exact = crate::ArtifactDecodeLimits {
                max_control_operations: 1,
                max_control_operands: depth,
                ..crate::ArtifactDecodeLimits::default()
            };
            assert_eq!(
                crate::decode_program_artifact_sections_with_limits(&sections, exact)
                    .unwrap()
                    .revision(),
                artifact.revision()
            );
            let short = crate::ArtifactDecodeLimits {
                max_control_operands: depth - 1,
                ..exact
            };
            assert!(crate::decode_program_artifact_sections_with_limits(&sections, short).is_err());
        }
    }

    #[cfg(feature = "resident-artifact")]
    #[test]
    fn lexical_collection_executes_live_generators_filters_joins_and_normalization() {
        use crate::resident::{ActivationFacts, CapturedSignalInput, activate};
        use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef};
        let mut catalog = FunctionCatalogBuilder::new();
        crate::install_intrinsic_resident(&mut catalog).unwrap();
        let catalog = catalog.build().unwrap();
        let data = |set, values: &[f64]| {
            let values = values
                .iter()
                .map(|number| ValueDataDraft::F64(F64Bits::from_f64(*number)))
                .collect();
            if set {
                ValueDataDraft::Set(values)
            } else {
                ValueDataDraft::Matrix(values)
            }
        };
        for (source, inputs, expected) in [
            (
                "[x | x := 1, x <- [1 2]]",
                vec![vec![], vec![]],
                vec![data(false, &[1.0]); 2],
            ),
            (
                "[x | x := signal<f64>, x <- [1 2]]",
                vec![vec![1.0], vec![2.0], vec![3.0]],
                vec![data(false, &[1.0]), data(false, &[2.0]), data(false, &[])],
            ),
            (
                "[item + 1 | item <- [1 2 3], item > 1]",
                vec![vec![], vec![]],
                vec![data(false, &[3.0, 4.0]); 2],
            ),
            (
                "{item + 1 | item <- {1,2,3}, item > 1}",
                vec![vec![], vec![]],
                vec![data(true, &[3.0, 4.0]); 2],
            ),
            (
                "[item + 1 | item <- signal<[f64]:1,3>, item > 1]",
                vec![
                    vec![1.0, 2.0, 3.0],
                    vec![5.0, 0.0, -1.0],
                    vec![0.0, 0.0, 0.0],
                ],
                vec![
                    data(false, &[3.0, 4.0]),
                    data(false, &[6.0]),
                    data(false, &[]),
                ],
            ),
            (
                "[x + y | x <- [1 2], x <- [2 3], y <- signal<[f64]:1,2>]",
                vec![vec![10.0, 20.0], vec![20.0, 40.0]],
                vec![data(false, &[12.0, 22.0]), data(false, &[22.0, 42.0])],
            ),
            (
                "{x | x <- signal<[f64]:1,3>}",
                vec![
                    vec![3.0, 3.0, 1.0],
                    vec![5.0, 5.0, 5.0],
                    vec![0.0, -0.0, f64::NAN],
                ],
                vec![
                    data(true, &[1.0, 3.0]),
                    data(true, &[5.0]),
                    data(true, &[0.0, f64::NAN]),
                ],
            ),
            (
                "[x | x <- signal<[f64]:2,3>]",
                vec![vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0]],
                vec![data(false, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0])],
            ),
            (
                "[1 | 2 <- signal<[f64]:1,3>]",
                vec![vec![2.0, 0.0, 2.0], vec![0.0, 0.0, 0.0]],
                vec![data(false, &[1.0, 1.0]), data(false, &[])],
            ),
            (
                "[7 | * <- signal<[f64]:1,2>]",
                vec![vec![1.0, 2.0]],
                vec![data(false, &[7.0, 7.0])],
            ),
            (
                "[y | x <- signal<[f64]:1,2>, y := x + 1]",
                vec![vec![1.0, 2.0]],
                vec![data(false, &[2.0, 3.0])],
            ),
        ] {
            let artifact = compile(source).compile_artifact().unwrap();
            let decoded = crate::decode_program_artifact_bytecode_v1(
                &crate::encode_program_artifact_bytecode_v1(&artifact).unwrap(),
            )
            .unwrap();
            let mut instance = activate(
                ReactiveInstanceId::new(822, 71),
                &decoded,
                &catalog,
                &ActivationFacts::default(),
            )
            .unwrap_or_else(|error| panic!("{source}: {error:?}"));
            for (input, expected) in inputs.iter().zip(expected) {
                let inputs = instance
                    .plan
                    .inputs
                    .iter()
                    .map(|input_slot| CapturedSignalInput {
                        slot: input_slot.slot,
                        value: ResidentValueRef::F64(input),
                    })
                    .collect::<Vec<_>>();
                instance
                    .turn(&inputs)
                    .unwrap_or_else(|error| panic!("{source}: {error:?}"));
                assert_eq!(
                    instance
                        .copied_output(0)
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    expected,
                    "{source}"
                );
            }
        }
    }

    #[cfg(feature = "resident-artifact")]
    #[test]
    fn lexical_collection_set_sorting_admits_comparison_work_before_publication() {
        use crate::resident::{ActivationFacts, CapturedSignalInput, activate};
        use mech_core::{ReactiveInstanceId, ResidentValueRef};
        let artifact = compile("{x | x <- signal<[f64]:1,8192>, x > 0}")
            .compile_artifact()
            .unwrap();
        let mut catalog = mech_core::FunctionCatalogBuilder::new();
        crate::install_intrinsic_resident(&mut catalog).unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(822, 76),
            &artifact,
            &catalog.build().unwrap(),
            &ActivationFacts::default(),
        )
        .unwrap();
        let slot = instance.plan.inputs[0].slot;
        let mut values = vec![0.0; 8192];
        instance
            .turn(&[CapturedSignalInput {
                slot,
                value: ResidentValueRef::F64(&values),
            }])
            .unwrap();
        let before = instance
            .copied_output(0)
            .unwrap()
            .canonical_data_draft()
            .unwrap();
        let epoch = instance.published_epoch();
        for (index, value) in values.iter_mut().enumerate() {
            *value = (8192 - index) as f64;
        }
        assert!(
            instance
                .turn(&[CapturedSignalInput {
                    slot,
                    value: ResidentValueRef::F64(&values)
                }])
                .is_err()
        );
        assert_eq!(instance.published_epoch(), epoch);
        assert_eq!(
            instance
                .copied_output(0)
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            before
        );
        values.fill(0.0);
        values[0] = 2.0;
        values[1] = 1.0;
        instance
            .turn(&[CapturedSignalInput {
                slot,
                value: ResidentValueRef::F64(&values),
            }])
            .unwrap();
        assert_eq!(
            instance
                .copied_output(0)
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            ValueDataDraft::Set(
                vec![
                    ValueDataDraft::F64(F64Bits::from_f64(1.0)),
                    ValueDataDraft::F64(F64Bits::from_f64(2.0))
                ]
                .into_boxed_slice()
            )
        );
    }

    #[cfg(feature = "resident-artifact")]
    #[test]
    fn lexical_collection_projects_borrowed_structures_and_repeated_bindings() {
        use crate::resident::{ActivationFacts, CapturedSignalInput, activate};
        use mech_core::{ReactiveInstanceId, ResidentValueRef};
        let f = |value| ValueDataDraft::F64(F64Bits::from_f64(value));
        let tuple = |a, b| ValueDataDraft::Tuple(vec![f(a), f(b)].into_boxed_slice());
        let nested = |a, b, c| ValueDataDraft::Tuple(vec![tuple(a, b), f(c)].into_boxed_slice());
        let tagged = |a, b| {
            ValueDataDraft::Tuple(vec![ValueDataDraft::Atom, tuple(a, b)].into_boxed_slice())
        };
        let row =
            |values: &[f64]| ValueDataDraft::Matrix(values.iter().map(|value| f(*value)).collect());
        let mut catalog = mech_core::FunctionCatalogBuilder::new();
        crate::install_intrinsic_resident(&mut catalog).unwrap();
        let catalog = catalog.build().unwrap();
        for (source, turns) in [
            (
                "[x + y | (x,y) <- signal<[(f64,f64)]:1,2>]",
                vec![
                    (vec![tuple(1.0, 2.0), tuple(3.0, 4.0)], vec![3.0, 7.0]),
                    (vec![tuple(5.0, 6.0), tuple(7.0, 8.0)], vec![11.0, 15.0]),
                ],
            ),
            (
                "[x | (x,x) <- signal<[(f64,f64)]:1,3>]",
                vec![
                    (
                        vec![tuple(1.0, 1.0), tuple(2.0, 3.0), tuple(4.0, 4.0)],
                        vec![1.0, 4.0],
                    ),
                    (
                        vec![tuple(6.0, 6.0), tuple(7.0, 8.0), tuple(9.0, 10.0)],
                        vec![6.0],
                    ),
                ],
            ),
            (
                "[head + tail | [head, ..., tail] <- signal<[[f64]:1,3]:1,2>]",
                vec![
                    (
                        vec![row(&[1.0, 2.0, 3.0]), row(&[4.0, 5.0, 6.0])],
                        vec![4.0, 10.0],
                    ),
                    (
                        vec![row(&[7.0, 8.0, 9.0]), row(&[10.0, 11.0, 12.0])],
                        vec![16.0, 22.0],
                    ),
                ],
            ),
            (
                "[rest[1] + rest[2] | [head | rest] <- signal<[[f64]:1,3]:1,2>]",
                vec![
                    (
                        vec![row(&[1.0, 2.0, 3.0]), row(&[4.0, 5.0, 6.0])],
                        vec![5.0, 11.0],
                    ),
                    (
                        vec![row(&[7.0, 8.0, 9.0]), row(&[10.0, 11.0, 12.0])],
                        vec![17.0, 23.0],
                    ),
                ],
            ),
            (
                "[x + y + z | ((x,y),z) <- signal<[((f64,f64),f64)]:1,2>]",
                vec![
                    (
                        vec![nested(1.0, 2.0, 3.0), nested(4.0, 5.0, 6.0)],
                        vec![6.0, 15.0],
                    ),
                    (
                        vec![nested(7.0, 8.0, 9.0), nested(10.0, 11.0, 12.0)],
                        vec![24.0, 33.0],
                    ),
                ],
            ),
            (
                "[x + y | :Point(x,y) <- signal<[(:Point,(f64,f64))]:1,2>]",
                vec![
                    (vec![tagged(1.0, 2.0), tagged(3.0, 4.0)], vec![3.0, 7.0]),
                    (vec![tagged(5.0, 6.0), tagged(7.0, 8.0)], vec![11.0, 15.0]),
                ],
            ),
            (
                "[x | `Point(x,x) <- signal<[(:Point,(f64,f64))]:1,2>]",
                vec![
                    (vec![tagged(1.0, 1.0), tagged(2.0, 3.0)], vec![1.0]),
                    (vec![tagged(7.0, 8.0), tagged(10.0, 10.0)], vec![10.0]),
                ],
            ),
            (
                "[x + y | (x,y) <- signal<{(f64,f64)}>]",
                vec![
                    (vec![tuple(3.0, 4.0), tuple(1.0, 2.0)], vec![3.0, 7.0]),
                    (vec![tuple(7.0, 8.0), tuple(5.0, 6.0)], vec![11.0, 15.0]),
                ],
            ),
            (
                "[1 | :Point <- signal<[:Point]:1,2>]",
                vec![(
                    vec![ValueDataDraft::Atom, ValueDataDraft::Atom],
                    vec![1.0, 1.0],
                )],
            ),
            (
                "[x | [x,x] <- signal<[[f64]:1,3]:1,2>]",
                vec![(vec![row(&[1.0, 1.0, 1.0]), row(&[2.0, 2.0, 2.0])], vec![])],
            ),
            (
                "[x | [x,x] <- signal<[[f64]:1,2]:1,2>]",
                vec![
                    (vec![row(&[1.0, 1.0]), row(&[2.0, 3.0])], vec![1.0]),
                    (vec![row(&[7.0, 8.0]), row(&[10.0, 10.0])], vec![10.0]),
                ],
            ),
        ] {
            let original = compile(source).compile_artifact().unwrap();
            let artifact = crate::decode_program_artifact_bytecode_v1(
                &crate::encode_program_artifact_bytecode_v1(&original).unwrap(),
            )
            .unwrap();
            let mut instance = activate(
                ReactiveInstanceId::new(822, 75),
                &artifact,
                &catalog,
                &ActivationFacts::default(),
            )
            .unwrap_or_else(|error| panic!("{source}: {error:?}"));
            let slot = instance.plan.inputs[0].slot;
            for (items, expected) in turns {
                let input = ValueDraft {
                    schema: artifact.slots()[slot.get() as usize].schema,
                    shape_values: Box::new([]),
                    data: if source.contains("signal<{") {
                        ValueDataDraft::Set(items.into_boxed_slice())
                    } else {
                        ValueDataDraft::Matrix(items.into_boxed_slice())
                    },
                }
                .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                .unwrap();
                instance
                    .turn(&[CapturedSignalInput {
                        slot,
                        value: ResidentValueRef::Snapshot(&[Some(input)]),
                    }])
                    .unwrap_or_else(|error| panic!("{source}: {error:?}"));
                assert_eq!(
                    instance
                        .copied_output(0)
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    row(&expected),
                    "{source}"
                );
            }
        }
    }

    #[cfg(feature = "resident-artifact")]
    #[test]
    fn lexical_collection_admission_failure_preserves_publication_and_recovers() {
        use crate::resident::{ActivationFacts, CapturedSignalInput, activate};
        use mech_core::{ReactiveInstanceId, ResidentValueRef};
        let artifact = compile("[x | x <- signal<[f64]:1,1024>, x > 0, y <- signal<[f64]:1,1024>]")
            .compile_artifact()
            .unwrap();
        let mut catalog = mech_core::FunctionCatalogBuilder::new();
        crate::install_intrinsic_resident(&mut catalog).unwrap();
        let catalog = catalog.build().unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(822, 72),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        let slot = instance.plan.inputs[0].slot;
        let zeros = vec![0.0; 1024];
        instance
            .turn(&[CapturedSignalInput {
                slot,
                value: ResidentValueRef::F64(&zeros),
            }])
            .unwrap();
        let before = instance.copied_output(0).unwrap();
        let epoch = instance.published_epoch();
        let large = vec![1.0; 1024];
        assert!(
            instance
                .turn(&[CapturedSignalInput {
                    slot,
                    value: ResidentValueRef::F64(&large)
                }])
                .is_err()
        );
        assert_eq!(instance.published_epoch(), epoch);
        assert_eq!(
            instance
                .copied_output(0)
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            before.canonical_data_draft().unwrap()
        );
        let mut small = zeros;
        small[0] = 1.0;
        instance
            .turn(&[CapturedSignalInput {
                slot,
                value: ResidentValueRef::F64(&small),
            }])
            .unwrap();
        let mech_core::ValueDataDraft::Matrix(values) = instance
            .copied_output(0)
            .unwrap()
            .canonical_data_draft()
            .unwrap()
        else {
            panic!("matrix output")
        };
        assert_eq!(values.len(), 1024);
        assert!(values.iter().all(|value| *value
            == mech_core::ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_f64(1.0))));
    }
    #[cfg(feature = "resident-artifact")]
    #[test]
    fn lexical_collection_accumulates_inner_kernel_work_without_output_growth() {
        use crate::resident::{ActivationFacts, CapturedSignalInput, activate};
        use mech_core::{ReactiveInstanceId, ResidentValueRef};
        let artifact = compile("[x | x <- signal<[f64]:1,300>, x > 0, total := stats/sum/column(data<[f64]:1,65536>), total[1,1] > 0]").compile_artifact().unwrap();
        let mut catalog = mech_core::FunctionCatalogBuilder::new();
        crate::install_intrinsic_resident(&mut catalog).unwrap();
        let catalog = catalog.build().unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(822, 73),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        let slot = |name: &str| {
            let input = artifact
                .inputs()
                .iter()
                .find(|input| input.name == crate::encode_source_input_name(name))
                .unwrap();
            instance
                .plan
                .inputs
                .iter()
                .find(|slot| slot.artifact_slot == input.slot)
                .unwrap()
                .slot
        };
        let signal = slot("signal");
        let data = slot("data");
        let source = vec![0.0; 65536];
        let empty = vec![0.0; 300];
        let turns = |signal_values| {
            [
                CapturedSignalInput {
                    slot: signal,
                    value: ResidentValueRef::F64(signal_values),
                },
                CapturedSignalInput {
                    slot: data,
                    value: ResidentValueRef::F64(&source),
                },
            ]
        };
        instance.turn(&turns(&empty)).unwrap();
        let before = instance
            .copied_output(0)
            .unwrap()
            .canonical_data_draft()
            .unwrap();
        let epoch = instance.published_epoch();
        let large = vec![1.0; 300];
        assert!(
            instance.turn(&turns(&large)).is_err(),
            "individually admissible reductions must share the collection work allowance"
        );
        assert_eq!(instance.published_epoch(), epoch);
        assert_eq!(
            instance
                .copied_output(0)
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            before
        );
        let mut small = vec![0.0; 300];
        small[0] = 1.0;
        instance.turn(&turns(&small)).unwrap();
        assert_eq!(
            instance
                .copied_output(0)
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            before
        );
    }
}
