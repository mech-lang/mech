//! A recursive or checked function has one irrefutable argument arm.
//! Its locals remain ordinary operations; recursive calls target that arm's
//! lexical frame, just as they do in a source pattern-body function.
use super::*;

impl SemanticBuilder {
    pub(super) fn inline_recursive_statement_function_body(
        &mut self,
        name: &str,
        body: &SyntaxNode,
        parameters: &[(String, SchemaDraft)],
        arguments: &[PendingValue],
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
            .map(|node| function_parameter(self, &node).map(|(_, schema)| schema))
            .collect::<Result<Vec<_>, _>>()?;
        let expected = if let [schema] = outputs.as_slice() {
            schema.clone()
        } else {
            let mut dimensions = Vec::new();
            let fields = outputs
                .iter()
                .map(|schema| {
                    embed_schema_draft(
                        schema,
                        &mut dimensions,
                        SourceSemanticAnchor::for_node(call),
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            SchemaDraft {
                dimension_parameters: dimensions.into_boxed_slice(),
                body: SchemaBody::Tuple(fields.into_boxed_slice()),
            }
        };
        self.inline_function_body_frame(
            name,
            body,
            parameters,
            arguments,
            call,
            expected,
            error,
            |builder| builder.inline_statement_function_body(name, body, call, error),
        )
    }

    pub(super) fn inline_function_body_frame(
        &mut self,
        name: &str,
        body: &SyntaxNode,
        parameters: &[(String, SchemaDraft)],
        arguments: &[PendingValue],
        call: &SyntaxNode,
        expected: SchemaDraft,
        error: &impl Fn(&'static str, String) -> SourceSemanticError,
        compile: impl FnOnce(&mut Self) -> Result<PendingValue, SourceSemanticError>,
    ) -> Result<PendingValue, SourceSemanticError> {
        let argument_schema = if let [argument] = arguments {
            self.schema_draft_of(*argument)?
        } else {
            let mut dimensions = Vec::new();
            let fields = arguments
                .iter()
                .map(|value| {
                    function_argument_shape::embed(
                        &self.schema_draft_of(*value)?,
                        &mut dimensions,
                        SourceSemanticAnchor::for_node(call),
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            SchemaDraft {
                dimension_parameters: dimensions.into_boxed_slice(),
                body: SchemaBody::Tuple(fields.into_boxed_slice()),
            }
        };
        let recursive = calls_function(body, name)?;
        let supported_carrier = argument_schema
            .clone()
            .finalize()
            .is_ok_and(|schema| crate::is_control_pattern_schema(&schema));
        if recursive && !supported_carrier {
            return Err(error(
                "source-semantics/unsupported-recursive-argument",
                format!("function {name} requires supported canonical argument bindings"),
            ));
        }
        // A nonrecursive checked call needs lexical ordering, not recursive
        // pattern bindings. Capture its arguments directly so the frame retains
        // their immutable snapshots instead of copying their payloads into a
        // composite and projecting them back out. Recursive calls still need
        // the structural carrier that supplies each fresh invocation's bindings.
        let capture_arguments = !recursive;
        let scrutinee = if capture_arguments {
            self.constant(BuiltinSchema::Bool, ValueDataDraft::Bool(true))
        } else if let [argument] = arguments {
            *argument
        } else {
            self.emit_with_schema_draft(
                "core/composite-pack",
                arguments.to_vec(),
                argument_schema,
                call,
                "function-arguments",
                Some(name.to_owned()),
            )
        };

        let saved_bindings = self.bindings.clone();
        let saved_definitions = self.scope_definitions.clone();
        let saved_next = self.next_control_block;
        if self.control_depth == 0 {
            self.next_control_block = 0;
        }
        let binding_start = self.nodes.len();
        let mut bindings = BTreeMap::new();
        let mut fields = Vec::new();
        if !capture_arguments {
            for (local, ((parameter, _), argument)) in parameters.iter().zip(arguments).enumerate()
            {
                let schema = self.schema_draft_of(*argument)?;
                let index = u32::try_from(self.nodes.len()).map_err(|_| {
                    error(
                        "source-semantics/unsupported-function-body",
                        "too many function bindings".to_owned(),
                    )
                })?;
                let local = u32::try_from(local).map_err(|_| {
                    error(
                        "source-semantics/unsupported-function-body",
                        "too many function parameters".to_owned(),
                    )
                })?;
                self.nodes.push(PendingNode {
                    body: PendingNodeBody::CollectionBinding,
                    inferable_projection: false,
                    inputs: Vec::new(),
                    schema: schema.clone(),
                    exposes_output: true,
                    state: None,
                    semantic: SourceSemanticNode {
                        operation: String::new(),
                        role: "function-parameter",
                        detail: Some(parameter.clone()),
                        anchor: SourceSemanticAnchor::for_node(body),
                    },
                });
                self.bindings.insert(
                    parameter.clone(),
                    PendingBinding::Value(PendingValue::Node(index)),
                );
                bindings.insert(index, local);
                fields.push(crate::CollectionPattern::Bind { local, schema });
            }
        }
        let pattern = if capture_arguments {
            crate::MatchPattern::Wildcard
        } else {
            crate::MatchPattern::Structural(if fields.len() == 1 {
                fields.pop().unwrap()
            } else {
                crate::CollectionPattern::Tuple(fields.into_boxed_slice())
            })
        };
        let mut inputs = vec![scrutinee];
        let mut captures = Vec::new();
        if capture_arguments {
            // Retain every argument dependency, even an unused parameter. Its
            // evaluation must precede this invocation's checks and later work.
            for argument in arguments {
                let input = match inputs.iter().position(|value| value == argument) {
                    Some(input) => input,
                    None => {
                        inputs.push(*argument);
                        inputs.len() - 1
                    }
                };
                let input = u16::try_from(input).map_err(|_| {
                    error(
                        "source-semantics/unsupported-function-body",
                        "too many function captures".to_owned(),
                    )
                })?;
                if !captures.iter().any(|(existing, _, _)| *existing == input) {
                    captures.push((
                        input,
                        self.schema_draft_of(*argument)?,
                        !matches!(argument, PendingValue::Input(_)),
                    ));
                }
            }
        }

        self.match_depth += 1;
        self.active_recursive_outputs
            .push((name.to_owned(), expected, self.match_depth));
        let result = self.control_block_with(
            body,
            &pattern,
            &bindings,
            scrutinee,
            &mut inputs,
            &mut captures,
            compile,
        );
        self.active_recursive_outputs.pop();
        self.match_depth -= 1;
        self.nodes.truncate(binding_start);
        self.bindings = saved_bindings;
        self.scope_definitions = saved_definitions;
        if self.control_depth == 0 {
            self.next_control_block = saved_next;
        }
        let (block, schema) = result?;
        let index = self.nodes.len() as u32;
        self.nodes.push(PendingNode {
            body: PendingNodeBody::Match(PendingMatch {
                partial: false,
                captures,
                arms: vec![PendingMatchArm {
                    pattern,
                    guard: None,
                    body: block,
                }],
            }),
            inferable_projection: false,
            inputs,
            schema,
            exposes_output: true,
            state: None,
            semantic: SourceSemanticNode {
                operation: "match".to_owned(),
                role: "function-frame",
                detail: Some(name.to_owned()),
                anchor: SourceSemanticAnchor::for_node(call),
            },
        });
        Ok(PendingValue::Node(index))
    }
}
