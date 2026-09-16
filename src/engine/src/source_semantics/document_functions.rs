//! Local function calls expand into the same canonical operation graph as their caller.

use super::*;

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
        if self.active_functions.iter().any(|active| active == name)
            || self.active_functions.len() >= crate::MAX_CONTROL_DEPTH as usize
        {
            return Err(error(
                "source-semantics/recursive-function",
                format!("function {name} cannot be finitely inlined"),
            ));
        }
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
            .find(|child| child.kind() == SyntaxKind::FunctionDefineStatements)
            .ok_or_else(|| {
                error(
                    "source-semantics/unsupported-function-body",
                    "canonical function inlining requires a statement body".to_owned(),
                )
            })?;
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
        let mut local_bindings = BTreeMap::new();
        let mut parameter_names = BTreeSet::new();
        for ((parameter, schema), input) in parameters.iter().zip(selected) {
            if !parameter_names.insert(parameter.clone()) {
                return Err(error(
                    "source-semantics/duplicate-function-parameter",
                    format!("parameter {parameter} is declared twice"),
                ));
            }
            let value = self.conform_schema_draft(
                input,
                schema,
                call,
                "source-semantics/incompatible-function-argument",
                "function argument does not satisfy its declared kind",
            )?;
            local_bindings.insert(parameter.clone(), PendingBinding::Value(value));
        }
        let caller_bindings = std::mem::replace(&mut self.bindings, local_bindings);
        let caller_definitions = std::mem::replace(&mut self.scope_definitions, parameter_names);
        let caller_external = std::mem::take(&mut self.external_definitions);
        self.active_functions.push(name.to_owned());
        let result = (|| {
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
        })();
        self.active_functions.pop();
        self.bindings = caller_bindings;
        self.scope_definitions = caller_definitions;
        self.external_definitions = caller_external;
        result
    }
}
