//! Local function calls expand into the same canonical operation graph as their caller.

use super::comprehension::{
    PendingCollectionValue, PendingComprehensionOperation, PendingComprehensionStep,
};
use super::*;
use mech_syntax::document::FunctionCallSyntax;

enum DocumentFunctionBody {
    Statements(SyntaxNode),
    Patterns(SyntaxNode),
}

enum PatternLiftOutput {
    Set { upper_bound: Option<DimensionExpr> },
    Matrix { source_schema: SchemaDraft },
}

struct PatternLift {
    id: crate::ControlBlockId,
    start: usize,
    source: PendingValue,
    element: SchemaDraft,
    output: PatternLiftOutput,
}

fn calls_function(node: &SyntaxNode, name: &str) -> Result<bool, SourceSemanticError> {
    if let Some(call) = FunctionCallSyntax::cast(node.clone())
        && let Some(function) = call.function()
        && node_text(function.syntax())? == name
    {
        return Ok(true);
    }
    for child in node.children() {
        if calls_function(&child, name)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn references_variable(node: &SyntaxNode, name: &str) -> Result<bool, SourceSemanticError> {
    if let Some(variable) = VariableSyntax::cast(node.clone())
        && let Some(VariableStemSyntax::Identifier(identifier)) = variable.stem()
        && node_text(identifier.syntax())? == name
    {
        return Ok(true);
    }
    for child in node.children() {
        if references_variable(&child, name)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn lift_element_conforms(element: &SchemaBody, parameter: &SchemaDraft) -> bool {
    if element == &parameter.body {
        return true;
    }
    if matches!(
        parameter.body,
        SchemaBody::Matrix { .. } | SchemaBody::Set { .. } | SchemaBody::Dynamic
    ) {
        return false;
    }
    if schema_annotation_accepts(element, &parameter.body) {
        return true;
    }
    let Ok(source) = ResolvedType::from_schema_body(element, &[]) else {
        return false;
    };
    let Ok(target) = ResolvedType::from_schema_body(&parameter.body, &[]) else {
        return false;
    };
    plan_explicit_cast(&source, &target).is_ok()
}

fn function_parameter(
    builder: &SemanticBuilder,
    node: &SyntaxNode,
) -> Result<(String, SchemaDraft), SourceSemanticError> {
    let name = node
        .children()
        .find(|child| child.kind() == SyntaxKind::Identifier)
        .ok_or_else(|| {
            internal(
                SourceSemanticAnchor::for_node(node),
                "function parameter has no name".to_owned(),
            )
        })?;
    let annotation = node
        .children()
        .find_map(KindAnnotationSyntax::cast)
        .ok_or_else(|| {
            internal(
                SourceSemanticAnchor::for_node(node),
                "function parameter has no kind".to_owned(),
            )
        })?;
    Ok((
        node_text(&name)?,
        builder.annotation_schema_draft(&annotation)?,
    ))
}

impl SemanticBuilder {
    pub(super) fn register_document_functions(
        &mut self,
        units: &[DocumentUnit],
    ) -> Result<(), SourceSemanticError> {
        for unit in units {
            match unit {
                DocumentUnit::Function(function) => {
                    let name = function
                        .children()
                        .find(|child| child.kind() == SyntaxKind::Identifier)
                        .ok_or_else(|| {
                            internal(
                                SourceSemanticAnchor::for_node(function),
                                "function has no name".to_owned(),
                            )
                        })?;
                    let name = node_text(&name)?;
                    if self
                        .local_functions
                        .insert(name.clone(), function.clone())
                        .is_some()
                    {
                        return Err(SourceSemanticError {
                            code: "source-semantics/duplicate-function",
                            message: format!("function {name} is defined more than once"),
                            anchor: SourceSemanticAnchor::for_node(function),
                        });
                    }
                }
                DocumentUnit::Fence(_, _, units) => self.register_document_functions(units)?,
                _ => {}
            }
        }
        Ok(())
    }

    pub(in super::super) fn require_function_local_binding(
        &self,
        name: &str,
        syntax: &SyntaxNode,
    ) -> Result<(), SourceSemanticError> {
        if name.starts_with('@') && self.input_schema_overrides.contains_key(name) {
            return Ok(());
        }
        if let Some(function) = self.active_functions.last() {
            return Err(SourceSemanticError {
                code: "source-semantics/unbound-function-input",
                message: format!("function {function} references undeclared local {name}"),
                anchor: SourceSemanticAnchor::for_node(syntax),
            });
        }
        Ok(())
    }

    pub(in super::super) fn inline_document_function(
        &mut self,
        name: &str,
        inputs: Vec<PendingValue>,
        names: &[String],
        call: &SyntaxNode,
    ) -> Result<PendingValue, SourceSemanticError> {
        let error = |code, message: String| SourceSemanticError {
            code,
            message,
            anchor: SourceSemanticAnchor::for_node(call),
        };
        let function = self.local_functions[name].clone();
        let parameters = function
            .children()
            .filter(|child| child.kind() == SyntaxKind::FunctionArg)
            .map(|node| function_parameter(self, &node))
            .collect::<Result<Vec<_>, _>>()?;
        let mut selected = vec![None; parameters.len()];
        for (input, supplied) in inputs.into_iter().zip(names) {
            let ordinal = if supplied.is_empty() {
                selected.iter().position(Option::is_none)
            } else {
                parameters.iter().position(|(name, _)| name == supplied)
            }
            .ok_or_else(|| {
                error(
                    "source-semantics/invalid-call-argument",
                    format!("function {name} has no available parameter {supplied}"),
                )
            })?;
            if selected[ordinal].replace(input).is_some() {
                return Err(error(
                    "source-semantics/duplicate-call-argument",
                    format!("parameter {} is supplied twice", parameters[ordinal].0),
                ));
            }
        }
        let selected = selected
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| {
                error(
                    "source-semantics/missing-call-argument",
                    format!("function {name} requires all declared arguments"),
                )
            })?;
        let body = function
            .children()
            .find_map(|child| match child.kind() {
                SyntaxKind::FunctionDefineStatements => {
                    Some(DocumentFunctionBody::Statements(child))
                }
                SyntaxKind::FunctionDefineMatchArms => Some(DocumentFunctionBody::Patterns(child)),
                _ => None,
            })
            .ok_or_else(|| {
                error(
                    "source-semantics/unsupported-function-body",
                    "canonical function has no supported body".to_owned(),
                )
            })?;
        if self.active_functions.iter().any(|active| active == name) {
            if self.comprehension_depth != 0 {
                return Err(error(
                    "source-semantics/recursive-comprehension",
                    format!("recursive function {name} cannot be called inside a comprehension"),
                ));
            }
            if self
                .active_functions
                .last()
                .is_none_or(|active| active != name)
            {
                return Err(error(
                    "source-semantics/mutual-recursion",
                    format!("function {name} forms an unsupported mutual recursion cycle"),
                ));
            }
            if !matches!(body, DocumentFunctionBody::Patterns(_)) {
                return Err(error(
                    "source-semantics/recursive-statement-function",
                    format!("function {name} requires a pattern body for canonical recursion"),
                ));
            }
            let (expected, root_depth) = self
                .active_recursive_outputs
                .iter()
                .rev()
                .find_map(|(active, output, root_depth)| {
                    (active == name).then_some((output.clone(), *root_depth))
                })
                .ok_or_else(|| {
                    error(
                        "source-semantics/recursive-function",
                        format!("function {name} has no active callable frame"),
                    )
                })?;
            let ancestor = self
                .match_depth
                .checked_sub(root_depth)
                .and_then(|depth| u8::try_from(depth).ok())
                .ok_or_else(|| {
                    error(
                        "source-semantics/recursive-function",
                        format!("function {name} has no enclosing lexical match"),
                    )
                })?;
            let arguments = parameters
                .iter()
                .zip(selected)
                .map(|((_, schema), input)| {
                    self.conform_schema_draft(
                        input,
                        schema,
                        call,
                        "source-semantics/incompatible-function-argument",
                        "recursive argument does not satisfy its declared kind",
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            let argument = if let [argument] = arguments.as_slice() {
                *argument
            } else {
                let mut dimensions = Vec::new();
                let elements = arguments
                    .iter()
                    .map(|argument| {
                        embed_schema_draft(
                            &self.schema_draft_of(*argument)?,
                            &mut dimensions,
                            SourceSemanticAnchor::for_node(call),
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                self.emit_with_schema_draft(
                    "core/composite-pack",
                    arguments,
                    SchemaDraft {
                        dimension_parameters: dimensions.into_boxed_slice(),
                        body: SchemaBody::Tuple(elements.into_boxed_slice()),
                    },
                    call,
                    "function-arguments",
                    Some(name.to_owned()),
                )
            };
            let index = self.nodes.len() as u32;
            self.nodes.push(PendingNode {
                body: PendingNodeBody::RecursiveCall(ancestor),
                inferable_projection: false,
                inputs: vec![argument],
                schema: expected,
                exposes_output: true,
                state: None,
                semantic: SourceSemanticNode {
                    operation: format!("function/{name}/recur"),
                    role: "recursive-call",
                    detail: Some(name.to_owned()),
                    anchor: SourceSemanticAnchor::for_node(call),
                },
            });
            return Ok(PendingValue::Node(index));
        }
        if self.active_functions.len() >= crate::MAX_CONTROL_DEPTH as usize {
            return Err(error(
                "source-semantics/function-expansion-depth",
                format!("function {name} exceeds the lexical expansion depth"),
            ));
        }
        let mut local_bindings = BTreeMap::new();
        let mut parameter_names = BTreeSet::new();
        let mut arguments = Vec::new();
        let mut lifted_collection = None;
        for ((parameter, schema), input) in parameters.iter().zip(selected) {
            if !parameter_names.insert(parameter.clone()) {
                return Err(error(
                    "source-semantics/duplicate-function-parameter",
                    format!("parameter {parameter} is declared twice"),
                ));
            }
            let actual = self.schema_draft_of(input)?;
            let matrix_lift = parameters.len() == 1
                && matches!(&body, DocumentFunctionBody::Patterns(_))
                && schema.dimension_parameters.is_empty()
                && matches!(
                    &actual.body,
                    SchemaBody::Matrix { element, .. }
                        if lift_element_conforms(element, schema)
                );
            let set_lift = parameters.len() == 1
                && matches!(&body, DocumentFunctionBody::Patterns(_))
                && schema.dimension_parameters.is_empty()
                && matches!(
                    &actual.body,
                    SchemaBody::Set { element, .. }
                        if lift_element_conforms(element, schema)
                );
            let value = if matrix_lift {
                let SchemaBody::Matrix { element, .. } = &actual.body else {
                    unreachable!("matrix lifting retains its source schema")
                };
                let source_element = SchemaDraft {
                    dimension_parameters: Box::new([]),
                    body: element.as_ref().clone(),
                };
                let (lift, value) = self.begin_pattern_lift(
                    input,
                    &source_element,
                    PatternLiftOutput::Matrix {
                        source_schema: actual,
                    },
                    call,
                )?;
                lifted_collection = Some(lift);
                self.conform_schema_draft(
                    value,
                    schema,
                    call,
                    "source-semantics/incompatible-function-argument",
                    "function argument does not satisfy its declared kind",
                )?
            } else if set_lift {
                let SchemaBody::Set {
                    element,
                    cardinality,
                } = &actual.body
                else {
                    unreachable!("set lifting retains its source schema")
                };
                let source_element = SchemaDraft {
                    dimension_parameters: Box::new([]),
                    body: element.as_ref().clone(),
                };
                let upper_bound = match cardinality {
                    CardinalitySpec::Exact(value) => Some(value.clone()),
                    CardinalitySpec::Dynamic { upper_bound } => upper_bound.clone(),
                };
                let upper_bound = if actual.dimension_parameters.is_empty() {
                    upper_bound
                } else {
                    match upper_bound {
                        Some(DimensionExpr::Constant(value)) => {
                            Some(DimensionExpr::Constant(value))
                        }
                        _ => None,
                    }
                };
                let (lift, value) = self.begin_pattern_lift(
                    input,
                    &source_element,
                    PatternLiftOutput::Set { upper_bound },
                    call,
                )?;
                lifted_collection = Some(lift);
                self.conform_schema_draft(
                    value,
                    schema,
                    call,
                    "source-semantics/incompatible-function-argument",
                    "function argument does not satisfy its declared kind",
                )?
            } else {
                self.conform_schema_draft(
                    input,
                    schema,
                    call,
                    "source-semantics/incompatible-function-argument",
                    "function argument does not satisfy its declared kind",
                )?
            };
            local_bindings.insert(parameter.clone(), PendingBinding::Value(value));
            arguments.push(value);
        }
        let caller_bindings = std::mem::replace(&mut self.bindings, local_bindings);
        let caller_definitions = std::mem::replace(&mut self.scope_definitions, parameter_names);
        let caller_external = std::mem::take(&mut self.external_definitions);
        self.active_functions.push(name.to_owned());
        let result = (|| match body {
            DocumentFunctionBody::Statements(body) => {
                self.inline_statement_function_body(name, &body, call, &error)
            }
            DocumentFunctionBody::Patterns(body) => {
                self.inline_pattern_function_body(name, &body, &arguments, call)
            }
        })();
        let result = if let Some(lift) = lifted_collection {
            self.control_depth -= 1;
            result.and_then(|result| self.finish_pattern_lift(lift, result, name, call))
        } else {
            result
        };
        self.active_functions.pop();
        self.bindings = caller_bindings;
        self.scope_definitions = caller_definitions;
        self.external_definitions = caller_external;
        result
    }

    fn begin_pattern_lift(
        &mut self,
        source: PendingValue,
        element: &SchemaDraft,
        output: PatternLiftOutput,
        call: &SyntaxNode,
    ) -> Result<(PatternLift, PendingValue), SourceSemanticError> {
        if !element.dimension_parameters.is_empty() {
            return Err(SourceSemanticError {
                code: "source-semantics/incompatible-function-argument",
                message: "collection-lifted function input requires a closed element kind"
                    .to_owned(),
                anchor: SourceSemanticAnchor::for_node(call),
            });
        }
        if self.control_depth == 0 {
            self.next_control_block = 0;
        }
        let id = self.next_control_block;
        self.next_control_block = id
            .checked_add(1)
            .filter(|id| *id as usize <= crate::MAX_CONTROL_BLOCKS)
            .ok_or_else(|| SourceSemanticError {
                code: "source-semantics/unsupported-function-body",
                message: "collection-lifted function exhausted control block identities".to_owned(),
                anchor: SourceSemanticAnchor::for_node(call),
            })?;
        self.control_depth += 1;
        let start = self.nodes.len();
        let binding = PendingValue::Node(u32::try_from(start).map_err(|_| {
            internal(
                SourceSemanticAnchor::for_node(call),
                "collection-lifted function exhausted node identities".to_owned(),
            )
        })?);
        self.nodes.push(PendingNode {
            body: PendingNodeBody::CollectionBinding,
            inferable_projection: false,
            inputs: Vec::new(),
            schema: element.clone(),
            exposes_output: true,
            state: None,
            semantic: SourceSemanticNode {
                operation: String::new(),
                role: "collection-binding",
                detail: Some("collection-lifted function element".to_owned()),
                anchor: SourceSemanticAnchor::for_node(call),
            },
        });
        Ok((
            PatternLift {
                id: crate::ControlBlockId(id),
                start,
                source,
                element: element.clone(),
                output,
            },
            binding,
        ))
    }

    fn finish_pattern_lift(
        &mut self,
        lift: PatternLift,
        result: PendingValue,
        name: &str,
        call: &SyntaxNode,
    ) -> Result<PendingValue, SourceSemanticError> {
        let output_element = self.schema_draft_of(result)?;
        if !output_element.dimension_parameters.is_empty() {
            return Err(SourceSemanticError {
                code: "source-semantics/incompatible-function-output",
                message: "collection-lifted function output requires a closed element kind"
                    .to_owned(),
                anchor: SourceSemanticAnchor::for_node(call),
            });
        }
        if matches!(lift.output, PatternLiftOutput::Set { .. }) {
            require_keyable_set_element(&output_element.body, &[], call)?;
            if !matches!(
                &output_element.body,
                SchemaBody::Bool
                    | SchemaBody::Index
                    | SchemaBody::FloatingPoint(mech_core::FloatWidth::W64)
            ) {
                return Err(SourceSemanticError {
                    code: "source-semantics/unsupported-lifted-set-result-kind",
                    message: "lifted set results require Bool, Index, or f64 elements".to_owned(),
                    anchor: SourceSemanticAnchor::for_node(call),
                });
            }
        }

        let mut inputs = Vec::new();
        let mut capture =
            |value: PendingValue| -> Result<PendingCollectionValue, SourceSemanticError> {
                match value {
                    PendingValue::Constant(id) => Ok(PendingCollectionValue::Constant(id)),
                    PendingValue::Node(index) if index as usize >= lift.start => {
                        Ok(PendingCollectionValue::Local(index - lift.start as u32))
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
                            u16::try_from(ordinal).map_err(|_| SourceSemanticError {
                                code: "source-semantics/unsupported-function-body",
                                message: "collection-lifted function has too many captures"
                                    .to_owned(),
                                anchor: SourceSemanticAnchor::for_node(call),
                            })?,
                        ))
                    }
                }
            };
        let source = capture(lift.source)?;
        let yield_value = capture(result)?;
        let mut steps = vec![PendingComprehensionStep::Generator {
            source,
            pattern: crate::CollectionPattern::Bind {
                local: 0,
                schema: lift.element,
            },
        }];
        for (offset, node) in self.nodes.split_off(lift.start).into_iter().enumerate() {
            let body = match node.body {
                PendingNodeBody::CollectionBinding => continue,
                PendingNodeBody::Operation {
                    operation,
                    contract: Some(contract),
                    requirement: None,
                } if node.state.is_none()
                    && contract.interaction == mech_core::ExternalInteraction::Pure =>
                {
                    PendingControlOperationBody::Operation {
                        operation,
                        contract,
                    }
                }
                PendingNodeBody::Match(control) if node.state.is_none() => {
                    PendingControlOperationBody::Match(control)
                }
                PendingNodeBody::Comprehension(control) if node.state.is_none() => {
                    PendingControlOperationBody::Comprehension(control)
                }
                _ => {
                    return Err(SourceSemanticError {
                        code: "source-semantics/unsupported-function-body",
                        message: "collection-lifted functions require pure element operations"
                            .to_owned(),
                        anchor: SourceSemanticAnchor::for_node(call),
                    });
                }
            };
            steps.push(PendingComprehensionStep::Operation(
                PendingComprehensionOperation {
                    local: offset as u32,
                    body,
                    inputs: node
                        .inputs
                        .into_iter()
                        .map(&mut capture)
                        .collect::<Result<Box<[_]>, _>>()?,
                    schema: node.schema,
                },
            ));
        }
        let (kind, operation, schema) = match lift.output {
            PatternLiftOutput::Set { upper_bound } => (
                crate::ComprehensionKind::Set,
                "set/comprehension",
                SchemaDraft {
                    dimension_parameters: Box::new([]),
                    body: SchemaBody::Set {
                        element: Box::new(output_element.body),
                        cardinality: CardinalitySpec::Dynamic { upper_bound },
                    },
                },
            ),
            PatternLiftOutput::Matrix { source_schema } => {
                let SchemaBody::Matrix { dimensions, .. } = source_schema.body else {
                    unreachable!("matrix lifting retains its source schema")
                };
                (
                    crate::ComprehensionKind::MatrixPreserveShape,
                    "matrix/comprehension",
                    SchemaDraft {
                        dimension_parameters: source_schema.dimension_parameters,
                        body: SchemaBody::Matrix {
                            element: Box::new(output_element.body),
                            dimensions,
                        },
                    },
                )
            }
        };
        let node = self.nodes.len() as u32;
        self.nodes.push(PendingNode {
            body: PendingNodeBody::Comprehension(PendingComprehension {
                id: lift.id,
                kind,
                steps: steps.into_boxed_slice(),
                yield_value,
            }),
            inferable_projection: false,
            inputs,
            schema,
            exposes_output: true,
            state: None,
            semantic: SourceSemanticNode {
                operation: operation.to_owned(),
                role: "function-lift",
                detail: Some(name.to_owned()),
                anchor: SourceSemanticAnchor::for_node(call),
            },
        });
        Ok(PendingValue::Node(node))
    }

    fn inline_statement_function_body(
        &mut self,
        name: &str,
        body: &SyntaxNode,
        call: &SyntaxNode,
        error: &impl Fn(&'static str, String) -> SourceSemanticError,
    ) -> Result<PendingValue, SourceSemanticError> {
        let outputs = body
            .children()
            .find(|child| {
                matches!(
                    child.kind(),
                    SyntaxKind::FunctionOutArg | SyntaxKind::FunctionOutArgs
                )
            })
            .ok_or_else(|| {
                error(
                    "source-semantics/missing-function-output",
                    format!("function {name} has no output declaration"),
                )
            })?;
        let outputs = outputs
            .children()
            .filter(|child| child.kind() == SyntaxKind::FunctionArg)
            .map(|node| function_parameter(self, &node))
            .collect::<Result<Vec<_>, _>>()?;
        let mut units = Vec::new();
        let mut exports = Vec::new();
        for statement in body
            .children()
            .filter(|child| child.kind() == SyntaxKind::Statement)
        {
            collect_document_units(&statement, &mut units, &mut exports)?;
        }
        let mut bindings = self.bindings.keys().cloned().collect();
        declare_document_inputs(self, &units, &mut bindings)?;
        compile_document_units(self, units, &bindings, &mut Vec::new(), false)?;
        let mut values = Vec::new();
        let mut output_names = BTreeSet::new();
        for (output, schema) in outputs {
            if !output_names.insert(output.clone()) {
                return Err(error(
                    "source-semantics/duplicate-function-output",
                    format!("output {output} is declared twice"),
                ));
            }
            if !self.scope_definitions.contains(&output) {
                return Err(error(
                    "source-semantics/undefined-function-output",
                    format!("function {name} does not define output {output}"),
                ));
            }
            let binding = self.bindings.get(&output).copied().ok_or_else(|| {
                error(
                    "source-semantics/undefined-function-output",
                    format!("function {name} does not define output {output}"),
                )
            })?;
            let value = self.read_document_binding(binding, call)?;
            values.push(self.conform_schema_draft(
                value,
                &schema,
                call,
                "source-semantics/incompatible-function-output",
                "function output does not satisfy its declared kind",
            )?);
        }
        if let [value] = values.as_slice() {
            return Ok(*value);
        }
        let mut dimensions = Vec::new();
        let schemas = values
            .iter()
            .map(|value| {
                embed_schema_draft(
                    &self.schema_draft_of(*value)?,
                    &mut dimensions,
                    SourceSemanticAnchor::for_node(call),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(self.emit_with_schema_draft(
            "core/composite-pack",
            values,
            SchemaDraft {
                dimension_parameters: dimensions.into_boxed_slice(),
                body: SchemaBody::Tuple(schemas.into_boxed_slice()),
            },
            call,
            "function-outputs",
            Some(name.to_owned()),
        ))
    }

    fn inline_pattern_function_body(
        &mut self,
        name: &str,
        body: &SyntaxNode,
        arguments: &[PendingValue],
        call: &SyntaxNode,
    ) -> Result<PendingValue, SourceSemanticError> {
        let output = body
            .children()
            .find_map(KindAnnotationSyntax::cast)
            .ok_or_else(|| SourceSemanticError {
                code: "source-semantics/missing-function-output",
                message: format!("function {name} has no output kind"),
                anchor: SourceSemanticAnchor::for_node(body),
            })?;
        let expected = self.annotation_schema_draft(&output)?;
        let scrutinee = if let [argument] = arguments {
            *argument
        } else {
            let mut dimensions = Vec::new();
            let elements = arguments
                .iter()
                .map(|argument| {
                    embed_schema_draft(
                        &self.schema_draft_of(*argument)?,
                        &mut dimensions,
                        SourceSemanticAnchor::for_node(call),
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            self.emit_with_schema_draft(
                "core/composite-pack",
                arguments.to_vec(),
                SchemaDraft {
                    dimension_parameters: dimensions.into_boxed_slice(),
                    body: SchemaBody::Tuple(elements.into_boxed_slice()),
                },
                call,
                "function-arguments",
                Some(name.to_owned()),
            )
        };
        let arms = body
            .children()
            .filter(|child| child.kind() == SyntaxKind::FunctionMatchArm)
            .map(|arm| {
                let pattern = arm.children().find_map(PatternSyntax::cast);
                let value = arm
                    .children()
                    .find_map(ExpressionSyntax::cast)
                    .ok_or_else(|| SourceSemanticError {
                        code: "source-semantics/missing-function-arm-value",
                        message: format!("function {name} has an arm without a value"),
                        anchor: SourceSemanticAnchor::for_node(&arm),
                    })?;
                Ok(SourceMatchArm {
                    pattern,
                    guard: None,
                    value,
                    syntax: arm,
                })
            })
            .collect::<Result<Vec<_>, SourceSemanticError>>()?;
        if arms
            .iter()
            .map(|arm| calls_function(arm.value.syntax(), name))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .any(|recursive| recursive)
        {
            for arm in &arms {
                let mut bindings = Vec::new();
                if let Some(pattern) = &arm.pattern {
                    collect_pattern_bindings(pattern, &mut bindings)?;
                }
                for parameter in &self.scope_definitions {
                    if !bindings.iter().any(|binding| &binding.name == parameter)
                        && references_variable(arm.value.syntax(), parameter)?
                    {
                        return Err(SourceSemanticError {
                            code: "source-semantics/recursive-function-capture",
                            message: format!(
                                "recursive function {name} must bind parameter {parameter} in every arm that reads it"
                            ),
                            anchor: SourceSemanticAnchor::for_node(&arm.syntax),
                        });
                    }
                }
            }
        }
        let enum_input = matches!(
            self.schema_draft_of(scrutinee)?.body,
            SchemaBody::Enum { .. }
        );
        self.active_recursive_outputs.push((
            name.to_owned(),
            expected.clone(),
            self.match_depth + 1,
        ));
        let result =
            self.lower_match_expression(scrutinee, &arms, body, !enum_input, Some(&expected));
        self.active_recursive_outputs.pop();
        let result = result?;
        self.conform_schema_draft(
            result,
            &expected,
            call,
            "source-semantics/incompatible-function-output",
            "function output does not satisfy its declared kind",
        )
    }
}
