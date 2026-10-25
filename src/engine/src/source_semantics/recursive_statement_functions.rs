//! A recursive statement function has one irrefutable canonical argument arm.
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
        let scrutinee = if let [argument] = arguments {
            *argument
        } else {
            let mut dimensions = Vec::new();
            let fields = arguments
                .iter()
                .map(|value| {
                    embed_schema_draft(
                        &self.schema_draft_of(*value)?,
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
                    body: SchemaBody::Tuple(fields.into_boxed_slice()),
                },
                call,
                "function-arguments",
                Some(name.to_owned()),
            )
        };
        if !self
            .schema_draft_of(scrutinee)?
            .finalize()
            .is_ok_and(|schema| crate::is_control_pattern_schema(&schema))
        {
            return Err(error(
                "source-semantics/unsupported-recursive-argument",
                format!("function {name} requires supported canonical argument bindings"),
            ));
        }

        let saved_bindings = self.bindings.clone();
        let saved_definitions = self.scope_definitions.clone();
        let saved_next = self.next_control_block;
        if self.control_depth == 0 {
            self.next_control_block = 0;
        }
        let binding_start = self.nodes.len();
        let constraint_start = self.constraints.len();
        let mut bindings = BTreeMap::new();
        let mut fields = Vec::new();
        for (local, ((parameter, _), argument)) in parameters.iter().zip(arguments).enumerate() {
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
        let pattern = crate::MatchPattern::Structural(if fields.len() == 1 {
            fields.pop().unwrap()
        } else {
            crate::CollectionPattern::Tuple(fields.into_boxed_slice())
        });
        let mut inputs = vec![scrutinee];
        let mut captures = Vec::new();
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
            |builder| builder.inline_statement_function_body(name, body, call, error),
        );
        self.active_recursive_outputs.pop();
        self.match_depth -= 1;
        self.nodes.truncate(binding_start);
        self.bindings = saved_bindings;
        self.scope_definitions = saved_definitions;
        if self.control_depth == 0 {
            self.next_control_block = saved_next;
        }
        // A root integrity declaration cannot refer to a visit-local result.
        // Keep this unsupported until constraints have a lexical operation form.
        if self.constraints.len() != constraint_start {
            self.constraints.truncate(constraint_start);
            return Err(error(
                "source-semantics/recursive-function-invariant",
                "recursive statement invariants require lexical constraint lowering".to_owned(),
            ));
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
