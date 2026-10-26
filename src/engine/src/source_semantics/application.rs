//! Ordinary application lowers already-produced lexical values through the same
//! declared callable resolver as a normal source call. Counted stages own a
//! canonical iteration body; neither route reparses source or dispatches names
//! during execution.
use super::*;
use mech_syntax::document::{ApplicationCountError, ApplicationExpressionSyntax};

enum ApplicationCallable {
    Document(String),
    Catalog {
        name: String,
        declaration: mech_core::FunctionTypeDeclaration,
    },
}

enum ApplicationCount {
    Literal(u64),
    Elements(PendingValue),
}

pub(super) struct PendingIteration {
    pub(super) count: crate::IterationCount,
    pub(super) captures: Vec<(u16, SchemaDraft, bool)>,
    pub(super) carrier: SchemaDraft,
    pub(super) body: PendingControlBlock,
}

impl PendingIteration {
    pub(super) fn visit_schemas(&self, visit: &mut impl FnMut(&SchemaDraft)) {
        visit(&self.carrier);
        for (_, schema, _) in &self.captures {
            visit(schema);
        }
        for (_, schema) in &self.body.parameters {
            visit(schema);
        }
        for operation in &self.body.operations {
            visit(&operation.schema);
            match &operation.body {
                PendingControlOperationBody::Match(nested) => nested.visit_schemas(visit),
                PendingControlOperationBody::Comprehension(nested) => nested.visit_schemas(visit),
                PendingControlOperationBody::Iteration(nested) => nested.visit_schemas(visit),
                PendingControlOperationBody::Operation { .. }
                | PendingControlOperationBody::Recur(_)
                | PendingControlOperationBody::Suspend
                | PendingControlOperationBody::Publish => {}
            }
        }
    }
}

fn generalized_carrier(schema: &SchemaDraft) -> SchemaDraft {
    let SchemaBody::Matrix { element, .. } = &schema.body else {
        return schema.clone();
    };
    SchemaDraft {
        body: SchemaBody::Matrix {
            element: element.clone(),
            dimensions: Box::new([
                DimensionExpr::Parameter(DimensionParameterId::new(0)),
                DimensionExpr::Parameter(DimensionParameterId::new(1)),
            ]),
        },
        dimension_parameters: (0..2)
            .map(|ordinal| DimensionParameterDeclaration {
                id: DimensionParameterId::new(ordinal),
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Turn,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: None,
            })
            .collect(),
    }
}

impl SemanticBuilder {
    fn application_callable(
        &self,
        name: &str,
        syntax: &SyntaxNode,
    ) -> Result<ApplicationCallable, SourceSemanticError> {
        if self.resolved_source_modules.iter().any(|module| {
            name.strip_prefix(module)
                .is_some_and(|suffix| suffix.starts_with('/'))
        }) {
            return Err(SourceSemanticError {
                code: "source-semantics/source-module-value-not-callable",
                message: format!(
                    "{name} belongs to a resolved source module, not a catalog function"
                ),
                anchor: SourceSemanticAnchor::for_node(syntax),
            });
        }
        if self.local_functions.contains_key(name) {
            Ok(ApplicationCallable::Document(name.to_owned()))
        } else {
            let (name, declaration) = self.named_call_declaration(name, syntax)?;
            Ok(ApplicationCallable::Catalog { name, declaration })
        }
    }

    fn apply_callable(
        &mut self,
        callable: ApplicationCallable,
        input: PendingValue,
        syntax: &SyntaxNode,
    ) -> Result<PendingValue, SourceSemanticError> {
        match callable {
            ApplicationCallable::Document(name) => {
                self.inline_document_function(&name, vec![input], &[String::new()], syntax)
            }
            ApplicationCallable::Catalog { name, declaration } => {
                let (inputs, output) =
                    self.resolve_declared_call(&name, vec![input], syntax, declaration)?;
                Ok(self.emit_with_schema_draft(&name, inputs, output, syntax, "application", None))
            }
        }
    }

    pub(super) fn application(
        &mut self,
        application: &ApplicationExpressionSyntax,
    ) -> Result<PendingValue, SourceSemanticError> {
        let stages = application.stages();
        let mut resolved = Vec::with_capacity(stages.len());
        for stage in &stages {
            let function = self.required(stage.function(), stage.syntax(), "a unary callable")?;
            let name = node_text(function.syntax())?;
            let callable = self.application_callable(&name, function.syntax())?;
            let count = if stage.count_expression().is_some() {
                None
            } else {
                stage.count().map_err(|error| SourceSemanticError {
                    code: match error {
                        ApplicationCountError::Invalid => {
                            "source-semantics/invalid-application-count"
                        }
                        ApplicationCountError::Overflow => {
                            "source-semantics/application-count-overflow"
                        }
                    },
                    message: "counted application requires an exact nonnegative decimal u64 count"
                        .to_owned(),
                    anchor: SourceSemanticAnchor::for_node(stage.syntax()),
                })?
            };
            resolved.push((callable, count, stage));
        }
        let seed = self.required(
            application.seed(),
            application.syntax(),
            "an application seed",
        )?;
        // The seed is lowered once. A concrete common numeric argument context
        // follows normal call contextual typing; explicit literal suffixes and
        // annotations keep their existing authority.
        let seed_context = match resolved.first() {
            Some((ApplicationCallable::Catalog { declaration, .. }, _, stage)) => {
                declared_call_argument_contexts(
                    declaration,
                    &[String::new()],
                    SourceSemanticAnchor::for_node(stage.syntax()),
                )
                .into_iter()
                .next()
                .flatten()
            }
            _ => None,
        };
        let mut value = self.expression_body_with_expected(
            &seed,
            seed_context.as_ref().map(ExpectedSchema::CallArgument),
        )?;
        for (callable, count, stage) in resolved {
            let syntax = stage.syntax();
            let count = if let Some(expression) = stage.count_expression() {
                let count = self.expression_body_with_expected(&expression, None)?;
                let schema = self.schema_draft_of(count)?;
                if !matches!(&schema.body, SchemaBody::Matrix { element, dimensions }
                    if crate::is_iteration_count_element(element) && dimensions.len() == 2)
                {
                    return Err(SourceSemanticError {
                        code: "source-semantics/invalid-application-count",
                        message:
                            "bracketed application counts require numeric elements exactly representable as nonnegative U64 counts"
                                .to_owned(),
                        anchor: SourceSemanticAnchor::for_node(syntax),
                    });
                }
                Some(ApplicationCount::Elements(count))
            } else {
                count.map(ApplicationCount::Literal)
            };
            value = match count {
                None => self.apply_callable(callable, value, syntax)?,
                Some(count) => self.counted_application(callable, value, count, syntax)?,
            };
        }
        Ok(value)
    }
}

impl SemanticBuilder {
    fn counted_application(
        &mut self,
        callable: ApplicationCallable,
        seed: PendingValue,
        count: ApplicationCount,
        syntax: &SyntaxNode,
    ) -> Result<PendingValue, SourceSemanticError> {
        let error = || {
            SourceSemanticError {
            code: "source-semantics/unsupported-iteration-body",
            message: "counted application requires a pure scalar or flat matrix carrier and immutable captures".to_owned(),
            anchor: SourceSemanticAnchor::for_node(syntax),
        }
        };
        if self.control_depth >= crate::MAX_CONTROL_DEPTH {
            return Err(error());
        }
        let seed_schema = self.schema_draft_of(seed)?;
        let elementwise = matches!(count, ApplicationCount::Elements(_));
        let carrier = if elementwise {
            match &seed_schema.body {
                SchemaBody::Matrix { element, .. } => SchemaDraft {
                    body: *element.clone(),
                    dimension_parameters: Box::new([]),
                },
                _ => seed_schema.clone(),
            }
        } else {
            match &callable {
                ApplicationCallable::Document(name) => {
                    let parameter = self.unary_document_parameter_schema(name, syntax)?;
                    let seed_type = seed_schema.clone().finalize().map_err(|_| error())?;
                    let parameter_type = parameter.clone().finalize().map_err(|_| error())?;
                    if crate::iteration_carrier_compatible(&seed_type, &parameter_type) {
                        parameter
                    } else {
                        // A normal scalar-pattern function may lift over a matrix.
                        // Its actual selected body still decides whether that call
                        // is valid; the loop never invents a lifting implementation.
                        generalized_carrier(&seed_schema)
                    }
                }
                ApplicationCallable::Catalog { .. } => generalized_carrier(&seed_schema),
            }
        };
        let seed_type = seed_schema.clone().finalize().map_err(|_| error())?;
        let carrier_type = carrier.clone().finalize().map_err(|_| error())?;
        if !crate::is_iteration_carrier_schema(&carrier_type)
            || (!elementwise && !crate::iteration_carrier_compatible(&seed_type, &carrier_type))
            || (elementwise && !crate::is_iteration_carrier_schema(&seed_type))
        {
            return Err(error());
        }
        let saved_next = self.next_control_block;
        if self.control_depth == 0 {
            self.next_control_block = 0;
        }
        let id = crate::ControlBlockId(self.next_control_block);
        self.next_control_block = self
            .next_control_block
            .checked_add(1)
            .filter(|value| *value as usize <= crate::MAX_CONTROL_BLOCKS)
            .ok_or_else(error)?;
        let start = self.nodes.len();
        let carrier_value = PendingValue::Node(u32::try_from(start).map_err(|_| error())?);
        self.nodes.push(PendingNode {
            body: PendingNodeBody::CollectionBinding,
            inferable_projection: false,
            inputs: Vec::new(),
            schema: carrier.clone(),
            exposes_output: false,
            state: None,
            semantic: SourceSemanticNode {
                operation: String::new(),
                role: "iteration-carrier",
                detail: None,
                anchor: SourceSemanticAnchor::for_node(syntax),
            },
        });
        self.control_depth += 1;
        let result = self
            .apply_callable(callable, carrier_value, syntax)
            .and_then(|result| self.schema_draft_of(result).map(|schema| (result, schema)));
        self.control_depth -= 1;
        let nodes = self.nodes.split_off(start);
        if self.control_depth == 0 {
            self.next_control_block = saved_next;
        }
        let (result, result_schema) = result?;
        let result_type = result_schema.finalize().map_err(|_| error())?;
        if !crate::iteration_carrier_compatible(&result_type, &carrier_type) {
            return Err(error());
        }
        let local_ids = nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| !matches!(node.body, PendingNodeBody::CollectionBinding))
            .enumerate()
            .map(|(local, (offset, _))| {
                Ok((
                    u32::try_from(start + offset).map_err(|_| error())?,
                    u32::try_from(local).map_err(|_| error())?,
                ))
            })
            .collect::<Result<BTreeMap<_, _>, SourceSemanticError>>()?;
        let mut inputs = vec![seed];
        let count = match count {
            ApplicationCount::Literal(count) => crate::IterationCount::Literal(count),
            ApplicationCount::Elements(value) => {
                let input = if value == seed {
                    0
                } else {
                    inputs.push(value);
                    1
                };
                crate::IterationCount::Elements { input }
            }
        };
        let mut captures = Vec::<(u16, SchemaDraft, bool)>::new();
        let mut parameters = vec![(
            crate::ControlParameterSource::IterationCarrier,
            carrier.clone(),
        )];
        let mut parameter_values = vec![carrier_value];
        let mut resolve =
            |value: PendingValue| -> Result<PendingControlValue, SourceSemanticError> {
                if let Some(ordinal) = parameter_values
                    .iter()
                    .position(|existing| *existing == value)
                {
                    return Ok(PendingControlValue::Parameter(
                        u16::try_from(ordinal).map_err(|_| error())?,
                    ));
                }
                match value {
                    PendingValue::Constant(index) => Ok(PendingControlValue::Constant(index)),
                    PendingValue::Node(index) if index as usize >= start => local_ids
                        .get(&index)
                        .copied()
                        .map(PendingControlValue::Local)
                        .ok_or_else(error),
                    PendingValue::State(_) | PendingValue::UnresolvedEmpty(_) => Err(error()),
                    _ => {
                        let schema = self.schema_draft_of(value)?;
                        if !schema
                            .clone()
                            .finalize()
                            .is_ok_and(|schema| crate::is_control_capture_schema(&schema))
                        {
                            return Err(error());
                        }
                        let input = match inputs.iter().position(|existing| *existing == value) {
                            Some(ordinal) => ordinal,
                            None => {
                                inputs.push(value);
                                inputs.len() - 1
                            }
                        };
                        let input = u16::try_from(input).map_err(|_| error())?;
                        let capture = u16::try_from(captures.len()).map_err(|_| error())?;
                        // A counted invocation retains every lexical capture
                        // once, including a document signal input.
                        captures.push((input, schema.clone(), true));
                        let ordinal = u16::try_from(parameters.len()).map_err(|_| error())?;
                        parameters.push((crate::ControlParameterSource::Capture(capture), schema));
                        parameter_values.push(value);
                        Ok(PendingControlValue::Parameter(ordinal))
                    }
                }
            };
        let mut operations = Vec::new();
        for node in nodes.into_iter().skip(1) {
            if node.state.is_some() {
                return Err(error());
            }
            let body = match node.body {
                PendingNodeBody::Operation {
                    operation,
                    contract: Some(contract),
                    requirement: None,
                } if contract.interaction == mech_core::ExternalInteraction::Pure => {
                    PendingControlOperationBody::Operation {
                        operation,
                        contract,
                    }
                }
                PendingNodeBody::Match(control) => PendingControlOperationBody::Match(control),
                PendingNodeBody::Comprehension(control) => {
                    PendingControlOperationBody::Comprehension(control)
                }
                PendingNodeBody::Iteration(control) => {
                    PendingControlOperationBody::Iteration(control)
                }
                _ => return Err(error()),
            };
            operations.push(PendingControlOperation {
                body,
                inputs: node
                    .inputs
                    .into_iter()
                    .map(&mut resolve)
                    .collect::<Result<Vec<_>, _>>()?,
                schema: node.schema,
            });
        }
        let yield_value = resolve(result)?;
        let body = PendingControlBlock {
            id,
            parameters,
            operations,
            yield_value,
        };
        let index = u32::try_from(self.nodes.len()).map_err(|_| error())?;
        self.nodes.push(PendingNode {
            body: PendingNodeBody::Iteration(PendingIteration {
                count,
                captures,
                carrier: carrier.clone(),
                body,
            }),
            inferable_projection: false,
            inputs,
            schema: if elementwise { seed_schema } else { carrier },
            exposes_output: true,
            state: None,
            semantic: SourceSemanticNode {
                operation: "iteration".to_owned(),
                role: "iteration",
                detail: Some(match count {
                    crate::IterationCount::Literal(count) => count.to_string(),
                    crate::IterationCount::Elements { input } => {
                        format!("element counts input {input}")
                    }
                }),
                anchor: SourceSemanticAnchor::for_node(syntax),
            },
        });
        Ok(PendingValue::Node(index))
    }
}

pub(super) fn resolve_iteration(
    iteration: &PendingIteration,
    schemas: &SchemaTable,
    constants: &[mech_core::ConstantId],
) -> crate::IterationDeclaration<OperationContractDeclaration> {
    let schema = |draft: &SchemaDraft| {
        schemas
            .find_by_key(
                draft
                    .clone()
                    .finalize()
                    .expect("validated iteration schema")
                    .key(),
            )
            .expect("retained iteration schema")
    };
    crate::IterationDeclaration {
        count: iteration.count,
        seed: 0,
        captures: iteration
            .captures
            .iter()
            .map(|(input, draft, freeze_on_suspend)| crate::ControlCapture {
                input: *input,
                schema: schema(draft),
                freeze_on_suspend: *freeze_on_suspend,
            })
            .collect(),
        carrier: schema(&iteration.carrier),
        body: resolve_pending_control_block(&iteration.body, schemas, constants),
    }
}
