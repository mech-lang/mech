use crate::capability::CapabilityRequest;
use crate::context::{
    RuntimeContext, RuntimeContextBase, RuntimeContextBinding, RuntimeContextCapabilityScope,
    RuntimeContextRegistry,
};
use crate::resolver::{SourceImportAlias, SourceScope};
use crate::runtime::external::{
    RuntimeResourceInitialValue, RuntimeResourceReadSpecializer, RuntimeResourceWriteSpecializer,
};
use crate::runtime::{MechRuntime, RuntimeExecutionMode, RuntimeInvalidOperationError};
use crate::{
    RuntimeCapabilityOperation, RuntimeResourceCapabilityDenied, RuntimeResourceKey,
    RuntimeResourceReadRequest, RuntimeResourceWriteIntent, RuntimeResourceWritePreflightRequest,
    SourceDeclaration, SourceIndex,
};
use mech_core::{
    ApplicationRequirement, ExecutionResourceRequest, MResult, MechError, MechErrorKind,
    MechSourceCode, ResourceDelivery, ResourceIntent, Value, hash_str,
};
use mech_engine::MechProgram;
use std::sync::Arc;

impl MechRuntime {
    fn is_manifest_context_import(import: &mech_core::ModuleImport) -> bool {
        matches!(import.alias, Some(mech_core::ModuleImportAlias::Context(_)))
    }

    pub(crate) fn context_declarations_from_index_scope(
        &self,
        index: &SourceIndex,
        scope: &SourceScope,
    ) -> MResult<Vec<crate::SourceContextDeclaration>> {
        let mut declarations = Vec::new();

        for declaration in &index.declarations {
            match declaration {
                SourceDeclaration::Context(context) if &context.occurrence.scope == scope => {
                    declarations.push(context.declaration.clone());
                }
                SourceDeclaration::Import(import) if &import.occurrence.scope == scope => {
                    let Some(SourceImportAlias::Context(alias)) = &import.declaration.alias else {
                        continue;
                    };
                    let module = import.declaration.module.as_deref().ok_or_else(|| {
                        MechError::new(
                            RuntimeInvalidOperationError {
                                operation: "materialize_direct_manifest_context_imports",
                                reason: format!(
                                    "context import `{}` is missing module metadata",
                                    import.declaration.specifier
                                ),
                            },
                            None,
                        )
                    })?;
                    let item = import.declaration.item.as_deref().ok_or_else(|| {
                        MechError::new(
                            RuntimeInvalidOperationError {
                                operation: "materialize_direct_manifest_context_imports",
                                reason: format!(
                                    "context import `{}` is missing item metadata",
                                    import.declaration.specifier
                                ),
                            },
                            None,
                        )
                    })?;
                    let target = format!("{module}/{item}");
                    if let Some(context) = self.host_interfaces.resolve_optional(&target)? {
                        declarations.push(crate::SourceContextDeclaration {
                            name: alias.clone(),
                            base: crate::SourceContextBase::ResourceUri(context.base_uri.clone()),
                            capabilities: context
                                .operations
                                .iter()
                                .map(|operation| crate::SourceContextCapability {
                                    operation: operation.clone(),
                                    scope: crate::SourceContextCapabilityScope::Wildcard,
                                })
                                .collect(),
                        });
                    } else {
                        let export = self.module_manifests.context_export(module, item)?;
                        declarations.push(crate::SourceContextDeclaration {
                            name: alias.clone(),
                            base: crate::SourceContextBase::ResourceUri(export.base_uri.clone()),
                            capabilities: export
                                .operations
                                .iter()
                                .map(|operation| crate::SourceContextCapability {
                                    operation: operation.clone(),
                                    scope: crate::SourceContextCapabilityScope::Wildcard,
                                })
                                .collect(),
                        });
                    }
                }
                _ => {}
            }
        }

        Ok(declarations)
    }

    pub(super) fn direct_context_registry_for_scope(
        &self,
        tree: &mech_core::Program,
        scope: &SourceScope,
    ) -> MResult<RuntimeContextRegistry> {
        let index = SourceIndex::from_program(tree);
        let declarations = self.context_declarations_from_index_scope(&index, scope)?;
        RuntimeContextRegistry::from_declarations(scope.clone(), &declarations)
    }

    fn resolve_context_resource_request(
        &self,
        binding: &RuntimeContextBinding,
        requested_path: &str,
    ) -> MResult<ResolvedContextResourceRequest> {
        let context_base_uri = runtime_context_base_uri(binding)
            .trim_end_matches('/')
            .to_string();
        let requested_path = requested_path.trim_matches('/').to_string();
        let candidate_uri = if requested_path.is_empty() {
            context_base_uri.clone()
        } else {
            format!("{}/{}", context_base_uri, requested_path)
        };

        let provider_base_uri = self
            .resources
            .provider_base_uri_for(&candidate_uri)?
            .unwrap_or_else(|| context_base_uri.clone());
        let provider_path = candidate_uri
            .strip_prefix(&provider_base_uri)
            .unwrap_or_default()
            .trim_matches('/')
            .to_string();

        Ok(ResolvedContextResourceRequest {
            provider_base_uri,
            provider_path,
            context_path: requested_path,
        })
    }

    fn register_context_resource_read(
        &mut self,
        context: &RuntimeContext,
        target: &mut RuntimeProgramTarget<'_>,
        binding: &RuntimeContextBinding,
        path: &str,
    ) -> MResult<mech_core::Expression> {
        let resolved = self.resolve_context_resource_request(binding, path)?;
        if !runtime_context_allows_read(binding, &resolved.context_path) {
            return Err(MechError::new(
                RuntimeResourceCapabilityDenied {
                    context_name: binding.name.clone(),
                    operation: "read".to_string(),
                    path: resolved.context_path,
                },
                None,
            ));
        }
        let key = RuntimeResourceKey::new(&resolved.provider_base_uri, &resolved.provider_path)?;
        let capability = CapabilityRequest::from_keys(
            &context.subject,
            RuntimeCapabilityOperation::Read.name(),
            key.capability_resource(),
        );
        self.check_capability_for_execution(context, &capability)?;

        let source = crate::RuntimeHostInputSource::new(
            resolved.provider_base_uri.clone(),
            resolved.provider_path.clone(),
        )?;
        let driven = self
            .input_drivers
            .iter()
            .try_fold(false, |driven, driver| {
                if driven {
                    return Ok(true);
                }
                crate::runtime::extension::invoke_extension_value(
                    "host input driver",
                    "drives",
                    || driver.drives(&source),
                )
            })?;
        let request = ExecutionResourceRequest {
            base_uri: resolved.provider_base_uri,
            path: resolved.provider_path,
            context_name: binding.name.clone(),
            operation: RuntimeCapabilityOperation::Read.name().to_string(),
            intent: ResourceIntent::Read,
            delivery: if driven {
                ResourceDelivery::Live
            } else {
                ResourceDelivery::Snapshot
            },
        };
        let runtime_request = RuntimeResourceReadRequest {
            base_uri: request.base_uri.clone(),
            path: request.path.clone(),
            context_name: request.context_name.clone(),
        };
        let initial = match self.execution_mode {
            RuntimeExecutionMode::Execute => {
                let staged = if let Some(transaction_id) = context.transaction {
                    let resource_identity = self
                        .resources
                        .staged_resource_identity_for(&request.base_uri)?;
                    self.active_execution_transaction(transaction_id)?
                        .effects
                        .staged_resource_value(&resource_identity, &request.path)
                } else {
                    None
                };
                let value = match staged {
                    Some(value) => value,
                    None => self.resources.read(runtime_request)?,
                };
                RuntimeResourceInitialValue::executed(&value)?
            }
            RuntimeExecutionMode::Plan => {
                let value = self.resources.plan_read(runtime_request)?;
                RuntimeResourceInitialValue::planned(&value)?
            }
        };
        let name = self
            .external_requirements
            .register(ApplicationRequirement::Resource(request.clone()))?;
        let interpreter_id = self.program_target_ref(target).interpreter().id;
        self.program_target_mut(target)
            .register_function_extension(
                name.clone(),
                Arc::new(RuntimeResourceReadSpecializer {
                    interpreter_id,
                    request,
                    initial,
                }),
            )?;

        Ok(mech_core::Expression::FunctionCall(
            mech_core::FunctionCall {
                name: identifier_from_str(&name),
                args: Vec::new(),
            },
        ))
    }

    fn register_context_resource_write(
        &mut self,
        target: &mut RuntimeProgramTarget<'_>,
        binding: &RuntimeContextBinding,
        path: &str,
        expression: mech_core::Expression,
        intent: RuntimeResourceWriteIntent,
    ) -> MResult<mech_core::Expression> {
        let resolved = self.resolve_context_resource_request(binding, path)?;
        let operation = context_write_operation(binding, intent, &resolved.context_path)?;
        if !runtime_context_allows_operation(binding, operation.name(), &resolved.context_path) {
            return Err(MechError::new(
                RuntimeResourceCapabilityDenied {
                    context_name: binding.name.clone(),
                    operation: operation.name().to_string(),
                    path: resolved.context_path,
                },
                None,
            ));
        }
        let request = ExecutionResourceRequest {
            base_uri: resolved.provider_base_uri,
            path: resolved.provider_path,
            context_name: binding.name.clone(),
            operation: operation.name().to_string(),
            intent: match intent {
                RuntimeResourceWriteIntent::Assign => ResourceIntent::Assign,
                RuntimeResourceWriteIntent::Send => ResourceIntent::Send,
            },
            delivery: ResourceDelivery::Snapshot,
        };
        let name = self
            .external_requirements
            .register(ApplicationRequirement::Resource(request.clone()))?;
        self.program_target_mut(target)
            .register_function_extension(
                name.clone(),
                Arc::new(RuntimeResourceWriteSpecializer { request }),
            )?;
        Ok(mech_core::Expression::FunctionCall(
            mech_core::FunctionCall {
                name: identifier_from_str(&name),
                args: vec![(None, expression)],
            },
        ))
    }
    pub(super) fn program_target_ref<'a>(
        &'a self,
        target: &'a RuntimeProgramTarget<'_>,
    ) -> &'a MechProgram {
        match target {
            RuntimeProgramTarget::Retained => &self.program,
            RuntimeProgramTarget::Isolated(program) => program,
        }
    }

    fn program_target_mut<'a>(
        &'a mut self,
        target: &'a mut RuntimeProgramTarget<'_>,
    ) -> &'a mut MechProgram {
        match target {
            RuntimeProgramTarget::Retained => &mut self.program,
            RuntimeProgramTarget::Isolated(program) => program,
        }
    }

    pub(super) fn execute_program_target_tree(
        &mut self,
        context: &mut RuntimeContext,
        target: &mut RuntimeProgramTarget<'_>,
        tree: &mech_core::Program,
    ) -> MResult<Value> {
        match target {
            RuntimeProgramTarget::Retained => self
                .with_retained_program_execution_session(context, |program, services| {
                    program.run_tree_with_services(tree, services)
                }),
            RuntimeProgramTarget::Isolated(program) => self
                .with_isolated_program_execution_session(context, program, |program, services| {
                    program.run_tree_with_services(tree, services)
                }),
        }
    }

    pub(super) fn execute_program_target_source(
        &mut self,
        context: &mut RuntimeContext,
        target: &mut RuntimeProgramTarget<'_>,
        source: &MechSourceCode,
    ) -> MResult<Value> {
        match target {
            RuntimeProgramTarget::Retained => self
                .with_retained_program_execution_session(context, |program, services| {
                    program.run_source_with_services(source, services)
                }),
            RuntimeProgramTarget::Isolated(program) => self
                .with_isolated_program_execution_session(context, program, |program, services| {
                    program.run_source_with_services(source, services)
                }),
        }
    }

    fn resolve_context_reads_in_expression(
        &mut self,
        context: &RuntimeContext,
        target: &mut RuntimeProgramTarget<'_>,
        registry: &RuntimeContextRegistry,
        expression: &mech_core::Expression,
    ) -> MResult<mech_core::Expression> {
        match expression {
            mech_core::Expression::Var(var) => {
                let Some(var_context) = &var.context else {
                    return Ok(expression.clone());
                };
                let context_target = var_context.to_string();
                let Some(binding) = registry.get(&context_target) else {
                    return Ok(expression.clone());
                };
                let path = var.name.to_string();
                self.register_context_resource_read(context, target, binding, &path)
            }
            mech_core::Expression::Formula(factor) => Ok(mech_core::Expression::Formula(
                self.resolve_context_reads_in_factor(context, target, registry, factor)?,
            )),
            mech_core::Expression::FunctionCall(call) => {
                let args = call
                    .args
                    .iter()
                    .map(|(name, expression)| {
                        Ok((
                            name.clone(),
                            self.resolve_context_reads_in_expression(
                                context, target, registry, expression,
                            )?,
                        ))
                    })
                    .collect::<MResult<Vec<_>>>()?;
                Ok(mech_core::Expression::FunctionCall(
                    mech_core::FunctionCall {
                        name: call.name.clone(),
                        args,
                    },
                ))
            }
            mech_core::Expression::FsmPipe(pipe) => Ok(mech_core::Expression::FsmPipe(
                self.resolve_context_reads_in_fsm_pipe(context, target, registry, pipe)?,
            )),
            mech_core::Expression::Literal(_) => Ok(expression.clone()),
            mech_core::Expression::Match(match_expression) => {
                let mut match_expression = match_expression.as_ref().clone();
                match_expression.source = self.resolve_context_reads_in_expression(
                    context,
                    target,
                    registry,
                    &match_expression.source,
                )?;
                match_expression.arms = match_expression
                    .arms
                    .iter()
                    .map(|arm| {
                        Ok(mech_core::MatchArm {
                            pattern: self.resolve_context_reads_in_match_pattern(
                                context,
                                target,
                                registry,
                                &arm.pattern,
                            )?,
                            guard: arm
                                .guard
                                .as_ref()
                                .map(|guard| {
                                    self.resolve_context_reads_in_expression(
                                        context, target, registry, guard,
                                    )
                                })
                                .transpose()?,
                            expression: self.resolve_context_reads_in_expression(
                                context,
                                target,
                                registry,
                                &arm.expression,
                            )?,
                        })
                    })
                    .collect::<MResult<Vec<_>>>()?;
                Ok(mech_core::Expression::Match(Box::new(match_expression)))
            }
            mech_core::Expression::Range(range) => {
                let mut range = range.as_ref().clone();
                range.start =
                    self.resolve_context_reads_in_factor(context, target, registry, &range.start)?;
                range.increment = match &range.increment {
                    Some((operator, increment)) => Some((
                        operator.clone(),
                        self.resolve_context_reads_in_factor(context, target, registry, increment)?,
                    )),
                    None => None,
                };
                range.terminal = self.resolve_context_reads_in_factor(
                    context,
                    target,
                    registry,
                    &range.terminal,
                )?;
                Ok(mech_core::Expression::Range(Box::new(range)))
            }
            mech_core::Expression::Slice(slice) => {
                if let Some(context_name) = &slice.context {
                    let context_name = context_name.to_string();
                    if registry.contains(&context_name) {
                        return Err(MechError::new(
                            RuntimeInvalidOperationError {
                                operation: "context_read",
                                reason: "context-addressed slices are not supported".to_string(),
                            },
                            None,
                        ));
                    }
                    return Ok(expression.clone());
                }
                Ok(mech_core::Expression::Slice(
                    self.resolve_context_reads_in_slice(context, target, registry, slice)?,
                ))
            }
            mech_core::Expression::Structure(structure) => Ok(mech_core::Expression::Structure(
                self.resolve_context_reads_in_structure(context, target, registry, structure)?,
            )),
            mech_core::Expression::SetComprehension(comprehension) => {
                let mut comprehension = comprehension.as_ref().clone();
                comprehension.expression = self.resolve_context_reads_in_expression(
                    context,
                    target,
                    registry,
                    &comprehension.expression,
                )?;
                comprehension.qualifiers = comprehension
                    .qualifiers
                    .iter()
                    .map(|qualifier| {
                        self.resolve_context_reads_in_comprehension_qualifier(
                            context, target, registry, qualifier,
                        )
                    })
                    .collect::<MResult<Vec<_>>>()?;
                Ok(mech_core::Expression::SetComprehension(Box::new(
                    comprehension,
                )))
            }
            mech_core::Expression::MatrixComprehension(comprehension) => {
                let mut comprehension = comprehension.as_ref().clone();
                comprehension.expression = self.resolve_context_reads_in_expression(
                    context,
                    target,
                    registry,
                    &comprehension.expression,
                )?;
                comprehension.qualifiers = comprehension
                    .qualifiers
                    .iter()
                    .map(|qualifier| {
                        self.resolve_context_reads_in_comprehension_qualifier(
                            context, target, registry, qualifier,
                        )
                    })
                    .collect::<MResult<Vec<_>>>()?;
                Ok(mech_core::Expression::MatrixComprehension(Box::new(
                    comprehension,
                )))
            }
        }
    }

    fn resolve_context_reads_in_factor(
        &mut self,
        context: &RuntimeContext,
        target: &mut RuntimeProgramTarget<'_>,
        registry: &RuntimeContextRegistry,
        factor: &mech_core::Factor,
    ) -> MResult<mech_core::Factor> {
        match factor {
            mech_core::Factor::Expression(expression) => {
                Ok(mech_core::Factor::Expression(Box::new(
                    self.resolve_context_reads_in_expression(
                        context, target, registry, expression,
                    )?,
                )))
            }
            mech_core::Factor::Negate(factor) => Ok(mech_core::Factor::Negate(Box::new(
                self.resolve_context_reads_in_factor(context, target, registry, factor)?,
            ))),
            mech_core::Factor::Not(factor) => Ok(mech_core::Factor::Not(Box::new(
                self.resolve_context_reads_in_factor(context, target, registry, factor)?,
            ))),
            mech_core::Factor::Parenthetical(factor) => Ok(mech_core::Factor::Parenthetical(
                Box::new(self.resolve_context_reads_in_factor(context, target, registry, factor)?),
            )),
            mech_core::Factor::Transpose(factor) => Ok(mech_core::Factor::Transpose(Box::new(
                self.resolve_context_reads_in_factor(context, target, registry, factor)?,
            ))),
            mech_core::Factor::Term(term) => {
                let rhs = term
                    .rhs
                    .iter()
                    .map(|(operator, factor)| {
                        Ok((
                            operator.clone(),
                            self.resolve_context_reads_in_factor(
                                context, target, registry, factor,
                            )?,
                        ))
                    })
                    .collect::<MResult<Vec<_>>>()?;
                Ok(mech_core::Factor::Term(Box::new(mech_core::Term {
                    lhs: self
                        .resolve_context_reads_in_factor(context, target, registry, &term.lhs)?,
                    rhs,
                })))
            }
        }
    }

    fn resolve_context_reads_in_slice(
        &mut self,
        context: &RuntimeContext,
        target: &mut RuntimeProgramTarget<'_>,
        registry: &RuntimeContextRegistry,
        slice: &mech_core::Slice,
    ) -> MResult<mech_core::Slice> {
        Ok(mech_core::Slice {
            name: slice.name.clone(),
            context: slice.context.clone(),
            subscript: slice
                .subscript
                .iter()
                .map(|subscript| {
                    self.resolve_context_reads_in_subscript(context, target, registry, subscript)
                })
                .collect::<MResult<Vec<_>>>()?,
        })
    }

    fn resolve_context_reads_in_slice_ref(
        &mut self,
        context: &RuntimeContext,
        target: &mut RuntimeProgramTarget<'_>,
        registry: &RuntimeContextRegistry,
        slice_ref: &mech_core::SliceRef,
    ) -> MResult<mech_core::SliceRef> {
        let mut slice_ref = slice_ref.clone();
        if let Some(subscripts) = &slice_ref.subscript {
            slice_ref.subscript = Some(
                subscripts
                    .iter()
                    .map(|subscript| {
                        self.resolve_context_reads_in_subscript(
                            context, target, registry, subscript,
                        )
                    })
                    .collect::<MResult<Vec<_>>>()?,
            );
        }
        Ok(slice_ref)
    }

    fn resolve_context_reads_in_subscript(
        &mut self,
        context: &RuntimeContext,
        target: &mut RuntimeProgramTarget<'_>,
        registry: &RuntimeContextRegistry,
        subscript: &mech_core::Subscript,
    ) -> MResult<mech_core::Subscript> {
        match subscript {
            mech_core::Subscript::Brace(subscripts) => Ok(mech_core::Subscript::Brace(
                subscripts
                    .iter()
                    .map(|subscript| {
                        self.resolve_context_reads_in_subscript(
                            context, target, registry, subscript,
                        )
                    })
                    .collect::<MResult<Vec<_>>>()?,
            )),
            mech_core::Subscript::Bracket(subscripts) => Ok(mech_core::Subscript::Bracket(
                subscripts
                    .iter()
                    .map(|subscript| {
                        self.resolve_context_reads_in_subscript(
                            context, target, registry, subscript,
                        )
                    })
                    .collect::<MResult<Vec<_>>>()?,
            )),
            mech_core::Subscript::Formula(factor) => Ok(mech_core::Subscript::Formula(
                self.resolve_context_reads_in_factor(context, target, registry, factor)?,
            )),
            mech_core::Subscript::Range(range) => {
                let mut range = range.clone();
                range.start =
                    self.resolve_context_reads_in_factor(context, target, registry, &range.start)?;
                range.increment = match &range.increment {
                    Some((operator, increment)) => Some((
                        operator.clone(),
                        self.resolve_context_reads_in_factor(context, target, registry, increment)?,
                    )),
                    None => None,
                };
                range.terminal = self.resolve_context_reads_in_factor(
                    context,
                    target,
                    registry,
                    &range.terminal,
                )?;
                Ok(mech_core::Subscript::Range(range))
            }
            _ => Ok(subscript.clone()),
        }
    }

    fn resolve_context_reads_in_structure(
        &mut self,
        context: &RuntimeContext,
        target: &mut RuntimeProgramTarget<'_>,
        registry: &RuntimeContextRegistry,
        structure: &mech_core::Structure,
    ) -> MResult<mech_core::Structure> {
        match structure {
            mech_core::Structure::Empty => Ok(mech_core::Structure::Empty),
            mech_core::Structure::Map(map) => Ok(mech_core::Structure::Map(mech_core::Map {
                elements: map
                    .elements
                    .iter()
                    .map(|mapping| {
                        Ok(mech_core::Mapping {
                            key: self.resolve_context_reads_in_expression(
                                context,
                                target,
                                registry,
                                &mapping.key,
                            )?,
                            value: self.resolve_context_reads_in_expression(
                                context,
                                target,
                                registry,
                                &mapping.value,
                            )?,
                        })
                    })
                    .collect::<MResult<Vec<_>>>()?,
            })),
            mech_core::Structure::Matrix(matrix) => {
                Ok(mech_core::Structure::Matrix(mech_core::nodes::Matrix {
                    rows: matrix
                        .rows
                        .iter()
                        .map(|row| {
                            Ok(mech_core::MatrixRow {
                                columns: row
                                    .columns
                                    .iter()
                                    .map(|column| {
                                        Ok(mech_core::MatrixColumn {
                                            element: self.resolve_context_reads_in_expression(
                                                context,
                                                target,
                                                registry,
                                                &column.element,
                                            )?,
                                        })
                                    })
                                    .collect::<MResult<Vec<_>>>()?,
                            })
                        })
                        .collect::<MResult<Vec<_>>>()?,
                }))
            }
            mech_core::Structure::Record(record) => {
                Ok(mech_core::Structure::Record(mech_core::Record {
                    bindings: record
                        .bindings
                        .iter()
                        .map(|binding| {
                            Ok(mech_core::Binding {
                                name: binding.name.clone(),
                                kind: binding.kind.clone(),
                                value: self.resolve_context_reads_in_expression(
                                    context,
                                    target,
                                    registry,
                                    &binding.value,
                                )?,
                            })
                        })
                        .collect::<MResult<Vec<_>>>()?,
                }))
            }
            mech_core::Structure::Set(set) => Ok(mech_core::Structure::Set(mech_core::Set {
                elements: set
                    .elements
                    .iter()
                    .map(|expression| {
                        self.resolve_context_reads_in_expression(
                            context, target, registry, expression,
                        )
                    })
                    .collect::<MResult<Vec<_>>>()?,
            })),
            mech_core::Structure::Table(table) => {
                Ok(mech_core::Structure::Table(mech_core::Table {
                    header: table.header.clone(),
                    rows: table
                        .rows
                        .iter()
                        .map(|row| {
                            Ok(mech_core::TableRow {
                                columns: row
                                    .columns
                                    .iter()
                                    .map(|column| {
                                        Ok(mech_core::TableColumn {
                                            element: self.resolve_context_reads_in_expression(
                                                context,
                                                target,
                                                registry,
                                                &column.element,
                                            )?,
                                        })
                                    })
                                    .collect::<MResult<Vec<_>>>()?,
                            })
                        })
                        .collect::<MResult<Vec<_>>>()?,
                }))
            }
            mech_core::Structure::Tuple(tuple) => {
                Ok(mech_core::Structure::Tuple(mech_core::Tuple {
                    elements: tuple
                        .elements
                        .iter()
                        .map(|expression| {
                            self.resolve_context_reads_in_expression(
                                context, target, registry, expression,
                            )
                        })
                        .collect::<MResult<Vec<_>>>()?,
                }))
            }
            mech_core::Structure::TupleStruct(tuple_struct) => {
                Ok(mech_core::Structure::TupleStruct(mech_core::TupleStruct {
                    name: tuple_struct.name.clone(),
                    value: Box::new(self.resolve_context_reads_in_expression(
                        context,
                        target,
                        registry,
                        &tuple_struct.value,
                    )?),
                }))
            }
        }
    }

    fn resolve_context_reads_in_comprehension_qualifier(
        &mut self,
        context: &RuntimeContext,
        target: &mut RuntimeProgramTarget<'_>,
        registry: &RuntimeContextRegistry,
        qualifier: &mech_core::ComprehensionQualifier,
    ) -> MResult<mech_core::ComprehensionQualifier> {
        match qualifier {
            mech_core::ComprehensionQualifier::Generator((pattern, expression)) => {
                Ok(mech_core::ComprehensionQualifier::Generator((
                    self.resolve_context_reads_in_match_pattern(
                        context, target, registry, pattern,
                    )?,
                    self.resolve_context_reads_in_expression(
                        context, target, registry, expression,
                    )?,
                )))
            }
            mech_core::ComprehensionQualifier::Filter(expression) => {
                Ok(mech_core::ComprehensionQualifier::Filter(
                    self.resolve_context_reads_in_expression(
                        context, target, registry, expression,
                    )?,
                ))
            }
            mech_core::ComprehensionQualifier::Let(var_def) => {
                let mut var_def = var_def.clone();
                var_def.expression = self.resolve_context_reads_in_expression(
                    context,
                    target,
                    registry,
                    &var_def.expression,
                )?;
                Ok(mech_core::ComprehensionQualifier::Let(var_def))
            }
        }
    }

    pub(super) fn flush_direct_execution(
        &mut self,
        context: &mut RuntimeContext,
        target: &mut RuntimeProgramTarget<'_>,
        pending: &mut Vec<mech_core::SectionElement>,
        result: &mut Value,
    ) -> MResult<()> {
        if pending.is_empty() {
            return Ok(());
        }
        let tree = mech_core::Program {
            title: None,
            body: mech_core::Body {
                sections: vec![mech_core::Section {
                    subtitle: None,
                    elements: std::mem::take(pending),
                }],
            },
        };
        *result = self.execute_program_target_tree(context, target, &tree)?;
        Ok(())
    }

    pub(super) fn executable_fence_for_scope(
        fenced: &mech_core::FencedMechCode,
        scope: &SourceScope,
    ) -> bool {
        match scope {
            SourceScope::Program => fenced.config.namespace_str.is_empty(),
            SourceScope::Interpreter(interpreter) => {
                fenced.config.namespace_str == interpreter.namespace_str
            }
        }
    }

    fn resolve_context_reads_in_pattern(
        &mut self,
        context: &RuntimeContext,
        target: &mut RuntimeProgramTarget<'_>,
        registry: &RuntimeContextRegistry,
        pattern: &mech_core::Pattern,
    ) -> MResult<mech_core::Pattern> {
        match pattern {
            mech_core::Pattern::Expression(expression) => Ok(mech_core::Pattern::Expression(
                self.resolve_context_reads_in_expression(context, target, registry, expression)?,
            )),
            mech_core::Pattern::TupleStruct(tuple_struct) => Ok(mech_core::Pattern::TupleStruct(
                mech_core::PatternTupleStruct {
                    name: tuple_struct.name.clone(),
                    patterns: tuple_struct
                        .patterns
                        .iter()
                        .map(|pattern| {
                            self.resolve_context_reads_in_pattern(
                                context, target, registry, pattern,
                            )
                        })
                        .collect::<MResult<Vec<_>>>()?,
                },
            )),
            mech_core::Pattern::Tuple(tuple) => {
                Ok(mech_core::Pattern::Tuple(mech_core::PatternTuple(
                    tuple
                        .0
                        .iter()
                        .map(|pattern| {
                            self.resolve_context_reads_in_pattern(
                                context, target, registry, pattern,
                            )
                        })
                        .collect::<MResult<Vec<_>>>()?,
                )))
            }
            mech_core::Pattern::Array(array) => {
                let spread = if let Some(spread) = &array.spread {
                    Some(mech_core::PatternArraySpread {
                        kind: spread.kind.clone(),
                        binding: spread
                            .binding
                            .as_ref()
                            .map(|binding| {
                                self.resolve_context_reads_in_pattern(
                                    context, target, registry, binding,
                                )
                                .map(Box::new)
                            })
                            .transpose()?,
                    })
                } else {
                    None
                };
                Ok(mech_core::Pattern::Array(mech_core::PatternArray {
                    prefix: array
                        .prefix
                        .iter()
                        .map(|pattern| {
                            self.resolve_context_reads_in_pattern(
                                context, target, registry, pattern,
                            )
                        })
                        .collect::<MResult<Vec<_>>>()?,
                    spread,
                    suffix: array
                        .suffix
                        .iter()
                        .map(|pattern| {
                            self.resolve_context_reads_in_pattern(
                                context, target, registry, pattern,
                            )
                        })
                        .collect::<MResult<Vec<_>>>()?,
                }))
            }
            mech_core::Pattern::Wildcard => Ok(mech_core::Pattern::Wildcard),
        }
    }

    fn resolve_context_reads_in_match_pattern(
        &mut self,
        context: &RuntimeContext,
        target: &mut RuntimeProgramTarget<'_>,
        registry: &RuntimeContextRegistry,
        pattern: &mech_core::Pattern,
    ) -> MResult<mech_core::Pattern> {
        match pattern {
            mech_core::Pattern::Expression(expression) => Ok(mech_core::Pattern::Expression(
                self.resolve_context_reads_in_match_pattern_expression(
                    context, target, registry, expression,
                )?,
            )),
            mech_core::Pattern::TupleStruct(tuple_struct) => Ok(mech_core::Pattern::TupleStruct(
                mech_core::PatternTupleStruct {
                    name: tuple_struct.name.clone(),
                    patterns: tuple_struct
                        .patterns
                        .iter()
                        .map(|pattern| {
                            self.resolve_context_reads_in_match_pattern(
                                context, target, registry, pattern,
                            )
                        })
                        .collect::<MResult<Vec<_>>>()?,
                },
            )),
            mech_core::Pattern::Tuple(tuple) => {
                if tuple.0.len() == 1 {
                    let pattern = self.resolve_context_reads_in_match_pattern(
                        context,
                        target,
                        registry,
                        &tuple.0[0],
                    )?;
                    if pattern != tuple.0[0] {
                        return Ok(pattern);
                    }
                }
                Ok(mech_core::Pattern::Tuple(mech_core::PatternTuple(
                    tuple
                        .0
                        .iter()
                        .map(|pattern| {
                            self.resolve_context_reads_in_match_pattern(
                                context, target, registry, pattern,
                            )
                        })
                        .collect::<MResult<Vec<_>>>()?,
                )))
            }
            mech_core::Pattern::Array(array) => {
                let spread = if let Some(spread) = &array.spread {
                    Some(mech_core::PatternArraySpread {
                        kind: spread.kind.clone(),
                        binding: spread
                            .binding
                            .as_ref()
                            .map(|binding| {
                                self.resolve_context_reads_in_match_pattern(
                                    context, target, registry, binding,
                                )
                                .map(Box::new)
                            })
                            .transpose()?,
                    })
                } else {
                    None
                };
                Ok(mech_core::Pattern::Array(mech_core::PatternArray {
                    prefix: array
                        .prefix
                        .iter()
                        .map(|pattern| {
                            self.resolve_context_reads_in_match_pattern(
                                context, target, registry, pattern,
                            )
                        })
                        .collect::<MResult<Vec<_>>>()?,
                    spread,
                    suffix: array
                        .suffix
                        .iter()
                        .map(|pattern| {
                            self.resolve_context_reads_in_match_pattern(
                                context, target, registry, pattern,
                            )
                        })
                        .collect::<MResult<Vec<_>>>()?,
                }))
            }
            mech_core::Pattern::Wildcard => Ok(mech_core::Pattern::Wildcard),
        }
    }

    fn resolve_context_reads_in_match_pattern_expression(
        &mut self,
        context: &RuntimeContext,
        target: &mut RuntimeProgramTarget<'_>,
        registry: &RuntimeContextRegistry,
        expression: &mech_core::Expression,
    ) -> MResult<mech_core::Expression> {
        if let Some(expression) =
            self.resolve_interpreter_address_pattern_expression(target, registry, expression)?
        {
            return Ok(expression);
        }
        self.resolve_context_reads_in_expression(context, target, registry, expression)
    }

    fn resolve_interpreter_address_pattern_expression(
        &mut self,
        target: &mut RuntimeProgramTarget<'_>,
        registry: &RuntimeContextRegistry,
        expression: &mech_core::Expression,
    ) -> MResult<Option<mech_core::Expression>> {
        match expression {
            mech_core::Expression::Var(var) => {
                let Some(var_context) = &var.context else {
                    return Ok(None);
                };
                let address_target = var_context.to_string();
                if registry.get(&address_target).is_some() {
                    return Ok(None);
                }

                let address = format!("@{address_target}/{}", var.name.to_string());
                let value = {
                    let program = self.program_target_ref(target);
                    let symbols = program.interpreter().symbols();
                    let symbols = symbols.borrow();
                    symbols
                        .get(hash_str(&address))
                        .map(|value_ref| resolve_runtime_value(value_ref.borrow().clone()))
                };
                match value {
                    Some(value) => self.resolved_pattern_value_expression(value).map(Some),
                    None => Ok(None),
                }
            }
            mech_core::Expression::Formula(factor) => {
                self.resolve_interpreter_address_pattern_factor(target, registry, factor)
            }
            _ => Ok(None),
        }
    }

    fn resolved_pattern_value_expression(&self, value: Value) -> MResult<mech_core::Expression> {
        let value = resolve_runtime_value(value);
        match value {
            #[cfg(feature = "string")]
            Value::String(value) => {
                let text = value.borrow().clone();
                Ok(mech_core::Expression::Literal(mech_core::Literal::String(
                    mech_core::MechString {
                        text: mech_core::Token::new(
                            mech_core::TokenKind::String,
                            mech_core::SourceRange::default(),
                            text.chars().collect(),
                        ),
                    },
                )))
            }
            #[cfg(feature = "bool")]
            Value::Bool(value) => {
                let flag = *value.borrow();
                Ok(mech_core::Expression::Literal(mech_core::Literal::Boolean(
                    mech_core::Token::new(
                        if flag {
                            mech_core::TokenKind::True
                        } else {
                            mech_core::TokenKind::False
                        },
                        mech_core::SourceRange::default(),
                        if flag {
                            "true".chars().collect()
                        } else {
                            "false".chars().collect()
                        },
                    ),
                )))
            }
            Value::Empty => Ok(mech_core::Expression::Literal(mech_core::Literal::Empty(
                mech_core::Token::new(
                    mech_core::TokenKind::Empty,
                    mech_core::SourceRange::default(),
                    vec!['_'],
                ),
            ))),
            other => Err(MechError::new(
                RuntimeInvalidOperationError {
                    operation: "interpreter_address_pattern",
                    reason: format!(
                        "interpreter-address patterns currently support string, bool, and empty values; got {other:?}",
                    ),
                },
                None,
            )),
        }
    }

    fn resolve_interpreter_address_pattern_factor(
        &mut self,
        target: &mut RuntimeProgramTarget<'_>,
        registry: &RuntimeContextRegistry,
        factor: &mech_core::Factor,
    ) -> MResult<Option<mech_core::Expression>> {
        match factor {
            mech_core::Factor::Expression(expression) => {
                self.resolve_interpreter_address_pattern_expression(target, registry, expression)
            }
            mech_core::Factor::Parenthetical(inner) => {
                self.resolve_interpreter_address_pattern_factor(target, registry, inner)
            }
            mech_core::Factor::Term(term) if term.rhs.is_empty() => {
                self.resolve_interpreter_address_pattern_factor(target, registry, &term.lhs)
            }
            _ => Ok(None),
        }
    }

    fn resolve_context_reads_in_transition(
        &mut self,
        context: &RuntimeContext,
        target: &mut RuntimeProgramTarget<'_>,
        registry: &RuntimeContextRegistry,
        transition: &mech_core::Transition,
    ) -> MResult<mech_core::Transition> {
        match transition {
            mech_core::Transition::Async(pattern) => Ok(mech_core::Transition::Async(
                self.resolve_context_reads_in_pattern(context, target, registry, pattern)?,
            )),
            mech_core::Transition::CodeBlock(code_items) => Ok(mech_core::Transition::CodeBlock(
                code_items
                    .iter()
                    .map(|(code, comment)| {
                        Ok((
                            self.resolve_context_reads_in_mech_code(
                                context, target, registry, code,
                            )?,
                            comment.clone(),
                        ))
                    })
                    .collect::<MResult<Vec<_>>>()?,
            )),
            mech_core::Transition::Next(pattern) => Ok(mech_core::Transition::Next(
                self.resolve_context_reads_in_pattern(context, target, registry, pattern)?,
            )),
            mech_core::Transition::Output(pattern) => Ok(mech_core::Transition::Output(
                self.resolve_context_reads_in_pattern(context, target, registry, pattern)?,
            )),
            mech_core::Transition::Statement(statement) => Ok(mech_core::Transition::Statement(
                self.resolve_context_reads_in_statement(context, target, registry, statement)?,
            )),
        }
    }

    fn resolve_context_reads_in_fsm_pipe(
        &mut self,
        context: &RuntimeContext,
        target: &mut RuntimeProgramTarget<'_>,
        registry: &RuntimeContextRegistry,
        pipe: &mech_core::FsmPipe,
    ) -> MResult<mech_core::FsmPipe> {
        let mut pipe = pipe.clone();

        if let Some(args) = &pipe.start.args {
            pipe.start.args = Some(
                args.iter()
                    .map(|(name, expression)| {
                        Ok((
                            name.clone(),
                            self.resolve_context_reads_in_expression(
                                context, target, registry, expression,
                            )?,
                        ))
                    })
                    .collect::<MResult<Vec<_>>>()?,
            );
        }

        pipe.transitions = pipe
            .transitions
            .iter()
            .map(|transition| {
                self.resolve_context_reads_in_transition(context, target, registry, transition)
            })
            .collect::<MResult<Vec<_>>>()?;

        Ok(pipe)
    }

    fn resolve_context_reads_in_fsm_implementation(
        &mut self,
        context: &RuntimeContext,
        target: &mut RuntimeProgramTarget<'_>,
        registry: &RuntimeContextRegistry,
        fsm: &mech_core::FsmImplementation,
    ) -> MResult<mech_core::FsmImplementation> {
        let arms = fsm
            .arms
            .iter()
            .map(|arm| match arm {
                mech_core::FsmArm::Guard(pattern, guards) => Ok(mech_core::FsmArm::Guard(
                    self.resolve_context_reads_in_match_pattern(
                        context, target, registry, pattern,
                    )?,
                    guards
                        .iter()
                        .map(|guard| {
                            Ok(mech_core::Guard {
                                condition: self.resolve_context_reads_in_match_pattern(
                                    context,
                                    target,
                                    registry,
                                    &guard.condition,
                                )?,
                                transitions: guard
                                    .transitions
                                    .iter()
                                    .map(|transition| {
                                        self.resolve_context_reads_in_transition(
                                            context, target, registry, transition,
                                        )
                                    })
                                    .collect::<MResult<Vec<_>>>()?,
                            })
                        })
                        .collect::<MResult<Vec<_>>>()?,
                )),
                mech_core::FsmArm::Transition(pattern, transitions) => {
                    Ok(mech_core::FsmArm::Transition(
                        self.resolve_context_reads_in_match_pattern(
                            context, target, registry, pattern,
                        )?,
                        transitions
                            .iter()
                            .map(|transition| {
                                self.resolve_context_reads_in_transition(
                                    context, target, registry, transition,
                                )
                            })
                            .collect::<MResult<Vec<_>>>()?,
                    ))
                }
                mech_core::FsmArm::Comment(comment) => {
                    Ok(mech_core::FsmArm::Comment(comment.clone()))
                }
            })
            .collect::<MResult<Vec<_>>>()?;

        Ok(mech_core::FsmImplementation {
            name: fsm.name.clone(),
            input: fsm.input.clone(),
            start: self.resolve_context_reads_in_pattern(context, target, registry, &fsm.start)?,
            arms,
        })
    }

    fn resolve_context_reads_in_function_define(
        &mut self,
        context: &RuntimeContext,
        target: &mut RuntimeProgramTarget<'_>,
        registry: &RuntimeContextRegistry,
        function: &mech_core::FunctionDefine,
    ) -> MResult<mech_core::FunctionDefine> {
        Ok(mech_core::FunctionDefine {
            name: function.name.clone(),
            input: function.input.clone(),
            output: function.output.clone(),
            statements: function.statements.clone(),
            match_arms: function
                .match_arms
                .iter()
                .map(|arm| {
                    Ok(mech_core::FunctionMatchArm {
                        pattern: self.resolve_context_reads_in_match_pattern(
                            context,
                            target,
                            registry,
                            &arm.pattern,
                        )?,
                        expression: arm.expression.clone(),
                    })
                })
                .collect::<MResult<Vec<_>>>()?,
        })
    }

    fn resolve_context_reads_in_mech_code(
        &mut self,
        context: &RuntimeContext,
        target: &mut RuntimeProgramTarget<'_>,
        registry: &RuntimeContextRegistry,
        code: &mech_core::MechCode,
    ) -> MResult<mech_core::MechCode> {
        match code {
            mech_core::MechCode::ActivationScope(scope) => {
                let mut scope = scope.clone();
                scope.trigger = self.resolve_context_reads_in_expression(
                    context,
                    target,
                    registry,
                    &scope.trigger,
                )?;
                scope.body = match scope.body {
                    mech_core::ActivationBody::Block(body) => mech_core::ActivationBody::Block(
                        body.iter()
                            .map(|(code, comment)| {
                                Ok((
                                    self.resolve_context_reads_in_mech_code(
                                        context, target, registry, code,
                                    )?,
                                    comment.clone(),
                                ))
                            })
                            .collect::<MResult<_>>()?,
                    ),
                    mech_core::ActivationBody::PatternArms(arms) => {
                        mech_core::ActivationBody::PatternArms(
                            arms.iter()
                                .map(|arm| {
                                    let body = match &arm.body {
                                        mech_core::ActivationArmBody::Block(body) => {
                                            mech_core::ActivationArmBody::Block(
                                                body.iter()
                                                    .map(|(code, comment)| {
                                                        Ok((
                                                            self.resolve_context_reads_in_mech_code(
                                                                context, target, registry, code,
                                                            )?,
                                                            comment.clone(),
                                                        ))
                                                    })
                                                    .collect::<MResult<_>>()?,
                                            )
                                        }
                                        mech_core::ActivationArmBody::Expression(expression) => {
                                            mech_core::ActivationArmBody::Expression(
                                                self.resolve_context_reads_in_expression(
                                                    context, target, registry, expression,
                                                )?,
                                            )
                                        }
                                    };
                                    Ok(mech_core::ActivationArm {
                                        pattern: self.resolve_context_reads_in_match_pattern(
                                            context,
                                            target,
                                            registry,
                                            &arm.pattern,
                                        )?,
                                        guard: arm
                                            .guard
                                            .as_ref()
                                            .map(|guard| {
                                                self.resolve_context_reads_in_expression(
                                                    context, target, registry, guard,
                                                )
                                            })
                                            .transpose()?,
                                        body,
                                    })
                                })
                                .collect::<MResult<_>>()?,
                        )
                    }
                };
                Ok(mech_core::MechCode::ActivationScope(scope))
            }
            mech_core::MechCode::Statement(mech_core::Statement::ContextSend(send)) => {
                let context_name = send.target.context.as_ref().ok_or_else(|| {
                    MechError::new(
                        RuntimeAddressedAssignmentUnsupported {
                            target: send.target.name.to_string(),
                        },
                        None,
                    )
                })?;
                let context_name = context_name.to_string();
                let binding = registry.get(&context_name).ok_or_else(|| {
                    MechError::new(
                        RuntimeAddressedAssignmentUnsupported {
                            target: context_name.clone(),
                        },
                        None,
                    )
                })?;
                let expression = self.resolve_context_reads_in_expression(
                    context,
                    target,
                    registry,
                    &send.expression,
                )?;
                Ok(mech_core::MechCode::Expression(
                    self.register_context_resource_write(
                        target,
                        binding,
                        &send.target.name.to_string(),
                        expression,
                        RuntimeResourceWriteIntent::Send,
                    )?,
                ))
            }
            mech_core::MechCode::Statement(mech_core::Statement::VariableAssign(assign))
                if assign.target.context.is_some() =>
            {
                let context_name = assign.target.context.as_ref().unwrap().to_string();
                let binding = registry.get(&context_name).ok_or_else(|| {
                    MechError::new(
                        RuntimeAddressedAssignmentUnsupported {
                            target: context_name.clone(),
                        },
                        None,
                    )
                })?;
                let expression = self.resolve_context_reads_in_expression(
                    context,
                    target,
                    registry,
                    &assign.expression,
                )?;
                Ok(mech_core::MechCode::Expression(
                    self.register_context_resource_write(
                        target,
                        binding,
                        &assign.target.name.to_string(),
                        expression,
                        RuntimeResourceWriteIntent::Assign,
                    )?,
                ))
            }
            mech_core::MechCode::Statement(statement) => Ok(mech_core::MechCode::Statement(
                self.resolve_context_reads_in_statement(context, target, registry, statement)?,
            )),
            mech_core::MechCode::Expression(expression) => Ok(mech_core::MechCode::Expression(
                self.resolve_context_reads_in_expression(context, target, registry, expression)?,
            )),
            mech_core::MechCode::FsmImplementation(fsm) => {
                Ok(mech_core::MechCode::FsmImplementation(
                    self.resolve_context_reads_in_fsm_implementation(
                        context, target, registry, fsm,
                    )?,
                ))
            }
            mech_core::MechCode::FsmSpecification(spec) => {
                Ok(mech_core::MechCode::FsmSpecification(spec.clone()))
            }
            mech_core::MechCode::FunctionDefine(function) => {
                Ok(mech_core::MechCode::FunctionDefine(
                    self.resolve_context_reads_in_function_define(
                        context, target, registry, function,
                    )?,
                ))
            }
            mech_core::MechCode::Import(_)
            | mech_core::MechCode::Comment(_)
            | mech_core::MechCode::Error(_, _) => Ok(code.clone()),
        }
    }

    fn resolve_context_reads_in_statement(
        &mut self,
        context: &RuntimeContext,
        target: &mut RuntimeProgramTarget<'_>,
        registry: &RuntimeContextRegistry,
        statement: &mech_core::Statement,
    ) -> MResult<mech_core::Statement> {
        match statement {
            mech_core::Statement::VariableDefine(var_def) => {
                let mut var_def = var_def.clone();
                var_def.expression = self.resolve_context_reads_in_expression(
                    context,
                    target,
                    registry,
                    &var_def.expression,
                )?;
                Ok(mech_core::Statement::VariableDefine(var_def))
            }
            mech_core::Statement::VariableAssign(assign) => {
                let mut assign = assign.clone();
                assign.target = self.resolve_context_reads_in_slice_ref(
                    context,
                    target,
                    registry,
                    &assign.target,
                )?;
                assign.expression = self.resolve_context_reads_in_expression(
                    context,
                    target,
                    registry,
                    &assign.expression,
                )?;
                Ok(mech_core::Statement::VariableAssign(assign))
            }
            mech_core::Statement::OpAssign(op_assign) => {
                let mut op_assign = op_assign.clone();
                op_assign.target = self.resolve_context_reads_in_slice_ref(
                    context,
                    target,
                    registry,
                    &op_assign.target,
                )?;
                op_assign.expression = self.resolve_context_reads_in_expression(
                    context,
                    target,
                    registry,
                    &op_assign.expression,
                )?;
                Ok(mech_core::Statement::OpAssign(op_assign))
            }
            mech_core::Statement::TupleDestructure(tuple_destructure) => {
                let mut tuple_destructure = tuple_destructure.clone();
                tuple_destructure.expression = self.resolve_context_reads_in_expression(
                    context,
                    target,
                    registry,
                    &tuple_destructure.expression,
                )?;
                Ok(mech_core::Statement::TupleDestructure(tuple_destructure))
            }
            #[cfg(feature = "invariant_define")]
            mech_core::Statement::InvariantDefine(invariant) => {
                let mut invariant = invariant.clone();
                invariant.expression = self.resolve_context_reads_in_expression(
                    context,
                    target,
                    registry,
                    &invariant.expression,
                )?;
                Ok(mech_core::Statement::InvariantDefine(invariant))
            }
            mech_core::Statement::FsmDeclare(fsm) => {
                let mut fsm = fsm.clone();
                fsm.pipe =
                    self.resolve_context_reads_in_fsm_pipe(context, target, registry, &fsm.pipe)?;
                Ok(mech_core::Statement::FsmDeclare(fsm))
            }
            _ => Ok(statement.clone()),
        }
    }

    pub(super) fn push_direct_code(
        &mut self,
        context: &mut RuntimeContext,
        target: &mut RuntimeProgramTarget<'_>,
        registry: &RuntimeContextRegistry,
        pending: &mut Vec<mech_core::SectionElement>,
        pending_codes: &mut Vec<(mech_core::MechCode, Option<mech_core::Comment>)>,
        result: &mut Value,
        skip_non_context_imports: bool,
        code: &mech_core::MechCode,
        comment: &Option<mech_core::Comment>,
    ) -> MResult<()> {
        match code {
            mech_core::MechCode::Import(import) if Self::is_manifest_context_import(import) => {
                Ok(())
            }
            mech_core::MechCode::Import(_) if skip_non_context_imports => Ok(()),
            mech_core::MechCode::Statement(mech_core::Statement::ImportDeclaration(_))
            | mech_core::MechCode::Statement(mech_core::Statement::ContextDeclaration(_)) => Ok(()),
            mech_core::MechCode::Statement(mech_core::Statement::ExportDeclaration(export)) => {
                if !pending_codes.is_empty() {
                    pending.push(mech_core::SectionElement::MechCode(std::mem::take(
                        pending_codes,
                    )));
                }
                self.flush_direct_execution(context, target, pending, result)?;
                let id = hash_str(&export.name.to_string());
                if let Some(value) = self
                    .program_target_ref(target)
                    .interpreter()
                    .symbols()
                    .borrow()
                    .get(id)
                {
                    *result = resolve_runtime_value(value.borrow().clone());
                } else {
                    *result = Value::Empty;
                }
                Ok(())
            }
            mech_core::MechCode::ActivationScope(scope) => {
                if !pending_codes.is_empty() {
                    pending.push(mech_core::SectionElement::MechCode(std::mem::take(
                        pending_codes,
                    )));
                }
                self.flush_direct_execution(context, target, pending, result)?;
                let lowered = self.resolve_context_reads_in_mech_code(
                    context,
                    target,
                    registry,
                    &mech_core::MechCode::ActivationScope(scope.clone()),
                )?;
                let tree = single_code_program(lowered, comment.clone());
                self.execute_program_target_tree(context, target, &tree)?;
                Ok(())
            }
            mech_core::MechCode::Statement(mech_core::Statement::ContextSend(_))
            | mech_core::MechCode::Statement(mech_core::Statement::VariableAssign(
                mech_core::VariableAssign {
                    target:
                        mech_core::SliceRef {
                            context: Some(_), ..
                        },
                    ..
                },
            )) => {
                if !pending_codes.is_empty() {
                    pending.push(mech_core::SectionElement::MechCode(std::mem::take(
                        pending_codes,
                    )));
                }
                self.flush_direct_execution(context, target, pending, result)?;
                let lowered =
                    self.resolve_context_reads_in_mech_code(context, target, registry, code)?;
                let tree = single_code_program(lowered, comment.clone());
                *result = self.execute_program_target_tree(context, target, &tree)?;
                Ok(())
            }
            mech_core::MechCode::Statement(mech_core::Statement::VariableDefine(var_def)) => {
                if let Some(context_name) = &var_def.var.context {
                    let context_target = context_name.to_string();
                    if let Some(binding) = registry.get(&context_target).cloned() {
                        if !pending_codes.is_empty() {
                            pending.push(mech_core::SectionElement::MechCode(std::mem::take(
                                pending_codes,
                            )));
                        }
                        self.flush_direct_execution(context, target, pending, result)?;
                        return Err(MechError::new(
                            RuntimeInvalidOperationError {
                                operation: "direct_context_define",
                                reason: format!(
                                    "context-addressed path `@{}/{}` cannot be defined with `:=`; use `=` for context writes",
                                    binding.name,
                                    var_def.var.name.to_string()
                                ),
                            },
                            None,
                        ));
                    }
                }
                let code = self.resolve_context_reads_in_mech_code(
                    context,
                    target,
                    registry,
                    &mech_core::MechCode::Statement(mech_core::Statement::VariableDefine(
                        var_def.clone(),
                    )),
                )?;
                pending_codes.push((code, comment.clone()));
                Ok(())
            }
            _ => {
                let code =
                    self.resolve_context_reads_in_mech_code(context, target, registry, code)?;
                pending_codes.push((code, comment.clone()));
                Ok(())
            }
        }
    }

    pub(super) fn preflight_context_capabilities(
        &self,
        context: &RuntimeContext,
        tree: &mech_core::Program,
        scope: &SourceScope,
    ) -> MResult<()> {
        let registry = self.direct_context_registry_for_scope(tree, scope)?;
        self.preflight_context_capabilities_with_registry(
            context,
            &registry,
            tree,
            scope,
            AddressedReadPreflight::RequireContextBinding,
        )
    }

    pub(super) fn preflight_context_capabilities_with_registry(
        &self,
        context: &RuntimeContext,
        registry: &RuntimeContextRegistry,
        tree: &mech_core::Program,
        scope: &SourceScope,
        addressed_read_preflight: AddressedReadPreflight,
    ) -> MResult<()> {
        for section in &tree.body.sections {
            for element in &section.elements {
                match element {
                    mech_core::SectionElement::MechCode(codes) => {
                        for (code, _) in codes {
                            self.preflight_code_context_capabilities(
                                context,
                                registry,
                                code,
                                DirectContextEffectPlacement::TopLevel,
                                addressed_read_preflight,
                            )?;
                        }
                    }
                    mech_core::SectionElement::FencedMechCode(fenced)
                        if Self::executable_fence_for_scope(fenced, scope) =>
                    {
                        for (code, _) in &fenced.code {
                            self.preflight_code_context_capabilities(
                                context,
                                registry,
                                code,
                                DirectContextEffectPlacement::TopLevel,
                                addressed_read_preflight,
                            )?;
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn preflight_code_context_capabilities(
        &self,
        context: &RuntimeContext,
        registry: &RuntimeContextRegistry,
        code: &mech_core::MechCode,
        placement: DirectContextEffectPlacement,
        addressed_read_preflight: AddressedReadPreflight,
    ) -> MResult<()> {
        match code {
            mech_core::MechCode::ActivationScope(scope) => {
                self.preflight_expression_context_reads(
                    context,
                    registry,
                    &scope.trigger,
                    addressed_read_preflight,
                )?;
                match &scope.body {
                    mech_core::ActivationBody::Block(body) => {
                        for (body_code, _) in body {
                            self.preflight_code_context_capabilities(
                                context,
                                registry,
                                body_code,
                                DirectContextEffectPlacement::ActivationScope,
                                addressed_read_preflight,
                            )?;
                        }
                    }
                    mech_core::ActivationBody::PatternArms(arms) => {
                        for arm in arms {
                            self.preflight_pattern_context_reads(
                                context,
                                registry,
                                &arm.pattern,
                                addressed_read_preflight,
                            )?;
                            if let Some(guard) = &arm.guard {
                                self.preflight_expression_context_reads(
                                    context,
                                    registry,
                                    guard,
                                    addressed_read_preflight,
                                )?;
                            }
                            match &arm.body {
                                mech_core::ActivationArmBody::Block(body) => {
                                    for (body_code, _) in body {
                                        self.preflight_code_context_capabilities(
                                            context,
                                            registry,
                                            body_code,
                                            DirectContextEffectPlacement::ActivationScope,
                                            addressed_read_preflight,
                                        )?;
                                    }
                                }
                                mech_core::ActivationArmBody::Expression(expression) => {
                                    self.preflight_expression_context_reads(
                                        context,
                                        registry,
                                        expression,
                                        addressed_read_preflight,
                                    )?;
                                }
                            }
                        }
                    }
                }
                Ok(())
            }
            mech_core::MechCode::Statement(statement) => self
                .preflight_statement_context_capabilities(
                    context,
                    registry,
                    statement,
                    placement,
                    addressed_read_preflight,
                ),
            mech_core::MechCode::Expression(expression) => self.preflight_expression_context_reads(
                context,
                registry,
                expression,
                addressed_read_preflight,
            ),
            mech_core::MechCode::FsmImplementation(fsm) => self
                .preflight_fsm_implementation_context_capabilities(
                    context,
                    registry,
                    fsm,
                    addressed_read_preflight,
                ),
            mech_core::MechCode::FunctionDefine(function) => self
                .preflight_function_define_context_capabilities(
                    context,
                    registry,
                    function,
                    addressed_read_preflight,
                ),
            mech_core::MechCode::Import(_)
            | mech_core::MechCode::Comment(_)
            | mech_core::MechCode::FsmSpecification(_)
            | mech_core::MechCode::Error(_, _) => Ok(()),
        }
    }

    fn preflight_function_define_context_capabilities(
        &self,
        context: &RuntimeContext,
        registry: &RuntimeContextRegistry,
        function: &mech_core::FunctionDefine,
        addressed_read_preflight: AddressedReadPreflight,
    ) -> MResult<()> {
        for statement in &function.statements {
            self.reject_runtime_context_reads_in_statement(registry, statement)?;
        }
        for arm in &function.match_arms {
            self.reject_runtime_context_reads_in_pattern(registry, &arm.pattern)?;
            self.reject_runtime_context_reads_in_expression(registry, &arm.expression)?;
        }

        for statement in &function.statements {
            self.preflight_statement_context_capabilities(
                context,
                registry,
                statement,
                DirectContextEffectPlacement::FunctionBody,
                addressed_read_preflight,
            )?;
        }
        for arm in &function.match_arms {
            self.preflight_pattern_context_reads(
                context,
                registry,
                &arm.pattern,
                addressed_read_preflight,
            )?;
            self.preflight_expression_context_reads(
                context,
                registry,
                &arm.expression,
                addressed_read_preflight,
            )?;
        }
        Ok(())
    }

    pub(super) fn reject_function_context_read(
        &self,
        context_name: &mech_core::Identifier,
        path: &mech_core::Identifier,
    ) -> MResult<()> {
        Err(MechError::new(
            RuntimeInvalidOperationError {
                operation: "direct_context_read_placement",
                reason: format!(
                    "context read from `{}` is not supported inside function definitions yet; read at module top level and pass the value as an argument",
                    direct_context_target(&context_name.to_string(), &path.to_string()),
                ),
            },
            None,
        ))
    }

    pub(super) fn preflight_transition_context_capabilities(
        &self,
        context: &RuntimeContext,
        registry: &RuntimeContextRegistry,
        transition: &mech_core::Transition,
        addressed_read_preflight: AddressedReadPreflight,
    ) -> MResult<()> {
        match transition {
            mech_core::Transition::Async(pattern)
            | mech_core::Transition::Next(pattern)
            | mech_core::Transition::Output(pattern) => self.preflight_pattern_context_reads(
                context,
                registry,
                pattern,
                addressed_read_preflight,
            ),
            mech_core::Transition::CodeBlock(code_items) => {
                for (code, _) in code_items {
                    self.preflight_code_context_capabilities(
                        context,
                        registry,
                        code,
                        DirectContextEffectPlacement::FsmTransition,
                        addressed_read_preflight,
                    )?;
                }
                Ok(())
            }
            mech_core::Transition::Statement(statement) => self
                .preflight_statement_context_capabilities(
                    context,
                    registry,
                    statement,
                    DirectContextEffectPlacement::FsmTransition,
                    addressed_read_preflight,
                ),
        }
    }

    pub(super) fn preflight_pattern_context_reads(
        &self,
        context: &RuntimeContext,
        registry: &RuntimeContextRegistry,
        pattern: &mech_core::Pattern,
        addressed_read_preflight: AddressedReadPreflight,
    ) -> MResult<()> {
        match pattern {
            mech_core::Pattern::Expression(expression) => self.preflight_expression_context_reads(
                context,
                registry,
                expression,
                addressed_read_preflight,
            ),
            mech_core::Pattern::TupleStruct(tuple_struct) => {
                for pattern in &tuple_struct.patterns {
                    self.preflight_pattern_context_reads(
                        context,
                        registry,
                        pattern,
                        addressed_read_preflight,
                    )?;
                }
                Ok(())
            }
            mech_core::Pattern::Tuple(tuple) => {
                for pattern in &tuple.0 {
                    self.preflight_pattern_context_reads(
                        context,
                        registry,
                        pattern,
                        addressed_read_preflight,
                    )?;
                }
                Ok(())
            }
            mech_core::Pattern::Array(array) => {
                for pattern in &array.prefix {
                    self.preflight_pattern_context_reads(
                        context,
                        registry,
                        pattern,
                        addressed_read_preflight,
                    )?;
                }
                if let Some(spread) = &array.spread {
                    if let Some(binding) = &spread.binding {
                        self.preflight_pattern_context_reads(
                            context,
                            registry,
                            binding,
                            addressed_read_preflight,
                        )?;
                    }
                }
                for pattern in &array.suffix {
                    self.preflight_pattern_context_reads(
                        context,
                        registry,
                        pattern,
                        addressed_read_preflight,
                    )?;
                }
                Ok(())
            }
            mech_core::Pattern::Wildcard => Ok(()),
        }
    }

    fn preflight_statement_context_capabilities(
        &self,
        context: &RuntimeContext,
        registry: &RuntimeContextRegistry,
        statement: &mech_core::Statement,
        placement: DirectContextEffectPlacement,
        addressed_read_preflight: AddressedReadPreflight,
    ) -> MResult<()> {
        match statement {
            mech_core::Statement::VariableDefine(var_def) => {
                if let Some(context_name) = &var_def.var.context {
                    return Err(MechError::new(
                        RuntimeInvalidOperationError {
                            operation: "direct_context_define",
                            reason: format!(
                                "context-addressed path `{}` cannot be defined with `:=`; use `=` or `<-`",
                                direct_context_target(
                                    &context_name.to_string(),
                                    &var_def.var.name.to_string()
                                ),
                            ),
                        },
                        None,
                    ));
                }
                self.preflight_expression_context_reads(
                    context,
                    registry,
                    &var_def.expression,
                    addressed_read_preflight,
                )
            }
            mech_core::Statement::VariableAssign(assign) => {
                self.preflight_slice_ref_subscript_context_reads(
                    context,
                    registry,
                    &assign.target,
                    addressed_read_preflight,
                )?;
                if let Some(context_name) = &assign.target.context {
                    let context_name = context_name.to_string();
                    let path = assign.target.name.to_string();
                    self.reject_direct_context_effect_placement(
                        "assignment",
                        &context_name,
                        &path,
                        placement,
                    )?;
                    self.preflight_context_access(
                        context,
                        registry,
                        &context_name,
                        &path,
                        RuntimeCapabilityOperation::Write,
                        true,
                        Some(provider_write_preflight_owner(
                            placement,
                            RuntimeResourceWriteIntent::Assign,
                        )),
                    )?;
                }
                self.preflight_expression_context_reads(
                    context,
                    registry,
                    &assign.expression,
                    addressed_read_preflight,
                )?;
                Ok(())
            }
            mech_core::Statement::ContextSend(send) => {
                let Some(context_name) = &send.target.context else {
                    return Err(MechError::new(
                        RuntimeInvalidOperationError {
                            operation: "direct_context_send",
                            reason: format!(
                                "send target `{}` is not a context path",
                                send.target.name.to_string()
                            ),
                        },
                        None,
                    ));
                };

                let context_name = context_name.to_string();
                let path = send.target.name.to_string();

                self.reject_direct_context_effect_placement(
                    "send",
                    &context_name,
                    &path,
                    placement,
                )?;

                self.preflight_context_send_access(
                    context,
                    registry,
                    &context_name,
                    &path,
                    placement,
                )?;

                self.preflight_expression_context_reads(
                    context,
                    registry,
                    &send.expression,
                    addressed_read_preflight,
                )?;
                Ok(())
            }
            mech_core::Statement::OpAssign(op_assign) => {
                self.preflight_slice_ref_subscript_context_reads(
                    context,
                    registry,
                    &op_assign.target,
                    addressed_read_preflight,
                )?;
                if op_assign.target.context.is_some() {
                    return Err(MechError::new(
                        RuntimeInvalidOperationError {
                            operation: "direct_context_op_assign",
                            reason: "context op-assignment is not supported; use `=` or `<-`"
                                .to_string(),
                        },
                        None,
                    ));
                }
                self.preflight_expression_context_reads(
                    context,
                    registry,
                    &op_assign.expression,
                    addressed_read_preflight,
                )
            }
            mech_core::Statement::TupleDestructure(tuple_destructure) => self
                .preflight_expression_context_reads(
                    context,
                    registry,
                    &tuple_destructure.expression,
                    addressed_read_preflight,
                ),
            #[cfg(feature = "invariant_define")]
            mech_core::Statement::InvariantDefine(invariant) => self
                .preflight_expression_context_reads(
                    context,
                    registry,
                    &invariant.expression,
                    addressed_read_preflight,
                ),
            mech_core::Statement::FsmDeclare(fsm) => self.preflight_fsm_pipe_context_capabilities(
                context,
                registry,
                &fsm.pipe,
                addressed_read_preflight,
            ),
            _ => Ok(()),
        }
    }

    fn preflight_fsm_pipe_context_capabilities(
        &self,
        context: &RuntimeContext,
        registry: &RuntimeContextRegistry,
        pipe: &mech_core::FsmPipe,
        addressed_read_preflight: AddressedReadPreflight,
    ) -> MResult<()> {
        if let Some(args) = &pipe.start.args {
            for (_, expression) in args {
                self.preflight_expression_context_reads(
                    context,
                    registry,
                    expression,
                    addressed_read_preflight,
                )?;
            }
        }

        for transition in &pipe.transitions {
            self.preflight_transition_context_capabilities(
                context,
                registry,
                transition,
                addressed_read_preflight,
            )?;
        }

        Ok(())
    }

    fn preflight_expression_context_reads(
        &self,
        context: &RuntimeContext,
        registry: &RuntimeContextRegistry,
        expression: &mech_core::Expression,
        addressed_read_preflight: AddressedReadPreflight,
    ) -> MResult<()> {
        match expression {
            mech_core::Expression::Var(var) => {
                if let Some(context_name) = &var.context {
                    let context_name = context_name.to_string();
                    if registry.contains(&context_name)
                        || addressed_read_preflight.requires_context_binding()
                    {
                        self.preflight_context_access(
                            context,
                            registry,
                            &context_name,
                            &var.name.to_string(),
                            RuntimeCapabilityOperation::Read,
                            true,
                            None,
                        )?;
                    }
                }
            }
            mech_core::Expression::Formula(factor) => {
                self.preflight_factor_context_reads(
                    context,
                    registry,
                    factor,
                    addressed_read_preflight,
                )?;
            }
            mech_core::Expression::FunctionCall(call) => {
                for (_, expression) in &call.args {
                    self.preflight_expression_context_reads(
                        context,
                        registry,
                        expression,
                        addressed_read_preflight,
                    )?;
                }
            }
            mech_core::Expression::FsmPipe(pipe) => {
                self.preflight_fsm_pipe_context_capabilities(
                    context,
                    registry,
                    pipe,
                    addressed_read_preflight,
                )?;
            }
            mech_core::Expression::Literal(_) => {}
            mech_core::Expression::Range(range) => {
                self.preflight_factor_context_reads(
                    context,
                    registry,
                    &range.start,
                    addressed_read_preflight,
                )?;
                if let Some((_, increment)) = &range.increment {
                    self.preflight_factor_context_reads(
                        context,
                        registry,
                        increment,
                        addressed_read_preflight,
                    )?;
                }
                self.preflight_factor_context_reads(
                    context,
                    registry,
                    &range.terminal,
                    addressed_read_preflight,
                )?;
            }
            mech_core::Expression::Structure(structure) => {
                self.preflight_structure_context_reads(
                    context,
                    registry,
                    structure,
                    addressed_read_preflight,
                )?;
            }
            mech_core::Expression::Match(match_expression) => {
                self.preflight_expression_context_reads(
                    context,
                    registry,
                    &match_expression.source,
                    addressed_read_preflight,
                )?;
                for arm in &match_expression.arms {
                    self.preflight_pattern_context_reads(
                        context,
                        registry,
                        &arm.pattern,
                        addressed_read_preflight,
                    )?;
                    if let Some(guard) = &arm.guard {
                        self.preflight_expression_context_reads(
                            context,
                            registry,
                            guard,
                            addressed_read_preflight,
                        )?;
                    }
                    self.preflight_expression_context_reads(
                        context,
                        registry,
                        &arm.expression,
                        addressed_read_preflight,
                    )?;
                }
            }
            mech_core::Expression::Slice(slice) => {
                if let Some(context_name) = &slice.context {
                    let context_name = context_name.to_string();
                    if registry.contains(&context_name) {
                        return Err(MechError::new(
                            RuntimeInvalidOperationError {
                                operation: "context_read",
                                reason: "context-addressed slices are not supported".to_string(),
                            },
                            None,
                        ));
                    }
                    if addressed_read_preflight.requires_context_binding() {
                        return Err(undeclared_direct_context_target_error(&context_name));
                    }
                }
                self.preflight_slice_context_reads(
                    context,
                    registry,
                    slice,
                    addressed_read_preflight,
                )?;
            }
            mech_core::Expression::SetComprehension(comprehension) => {
                self.preflight_expression_context_reads(
                    context,
                    registry,
                    &comprehension.expression,
                    addressed_read_preflight,
                )?;
                for qualifier in &comprehension.qualifiers {
                    self.preflight_comprehension_qualifier_context_reads(
                        context,
                        registry,
                        qualifier,
                        addressed_read_preflight,
                    )?;
                }
            }
            mech_core::Expression::MatrixComprehension(comprehension) => {
                self.preflight_expression_context_reads(
                    context,
                    registry,
                    &comprehension.expression,
                    addressed_read_preflight,
                )?;
                for qualifier in &comprehension.qualifiers {
                    self.preflight_comprehension_qualifier_context_reads(
                        context,
                        registry,
                        qualifier,
                        addressed_read_preflight,
                    )?;
                }
            }
        }
        Ok(())
    }

    fn preflight_factor_context_reads(
        &self,
        context: &RuntimeContext,
        registry: &RuntimeContextRegistry,
        factor: &mech_core::Factor,
        addressed_read_preflight: AddressedReadPreflight,
    ) -> MResult<()> {
        match factor {
            mech_core::Factor::Expression(expression) => self.preflight_expression_context_reads(
                context,
                registry,
                expression,
                addressed_read_preflight,
            ),
            mech_core::Factor::Negate(factor)
            | mech_core::Factor::Not(factor)
            | mech_core::Factor::Parenthetical(factor)
            | mech_core::Factor::Transpose(factor) => self.preflight_factor_context_reads(
                context,
                registry,
                factor,
                addressed_read_preflight,
            ),
            mech_core::Factor::Term(term) => {
                self.preflight_factor_context_reads(
                    context,
                    registry,
                    &term.lhs,
                    addressed_read_preflight,
                )?;
                for (_, factor) in &term.rhs {
                    self.preflight_factor_context_reads(
                        context,
                        registry,
                        factor,
                        addressed_read_preflight,
                    )?;
                }
                Ok(())
            }
        }
    }

    fn preflight_structure_context_reads(
        &self,
        context: &RuntimeContext,
        registry: &RuntimeContextRegistry,
        structure: &mech_core::Structure,
        addressed_read_preflight: AddressedReadPreflight,
    ) -> MResult<()> {
        match structure {
            mech_core::Structure::Map(map) => {
                for mapping in &map.elements {
                    self.preflight_expression_context_reads(
                        context,
                        registry,
                        &mapping.key,
                        addressed_read_preflight,
                    )?;
                    self.preflight_expression_context_reads(
                        context,
                        registry,
                        &mapping.value,
                        addressed_read_preflight,
                    )?;
                }
            }
            mech_core::Structure::Set(set) => {
                for expression in &set.elements {
                    self.preflight_expression_context_reads(
                        context,
                        registry,
                        expression,
                        addressed_read_preflight,
                    )?;
                }
            }
            mech_core::Structure::Matrix(matrix) => {
                for row in &matrix.rows {
                    for column in &row.columns {
                        self.preflight_expression_context_reads(
                            context,
                            registry,
                            &column.element,
                            addressed_read_preflight,
                        )?;
                    }
                }
            }
            mech_core::Structure::Record(record) => {
                for binding in &record.bindings {
                    self.preflight_expression_context_reads(
                        context,
                        registry,
                        &binding.value,
                        addressed_read_preflight,
                    )?;
                }
            }
            mech_core::Structure::Table(table) => {
                for row in &table.rows {
                    for column in &row.columns {
                        self.preflight_expression_context_reads(
                            context,
                            registry,
                            &column.element,
                            addressed_read_preflight,
                        )?;
                    }
                }
            }
            mech_core::Structure::Tuple(tuple) => {
                for expression in &tuple.elements {
                    self.preflight_expression_context_reads(
                        context,
                        registry,
                        expression,
                        addressed_read_preflight,
                    )?;
                }
            }
            mech_core::Structure::TupleStruct(tuple_struct) => {
                self.preflight_expression_context_reads(
                    context,
                    registry,
                    &tuple_struct.value,
                    addressed_read_preflight,
                )?;
            }
            _ => {}
        }
        Ok(())
    }

    fn preflight_slice_context_reads(
        &self,
        context: &RuntimeContext,
        registry: &RuntimeContextRegistry,
        slice: &mech_core::Slice,
        addressed_read_preflight: AddressedReadPreflight,
    ) -> MResult<()> {
        for subscript in &slice.subscript {
            self.preflight_subscript_context_reads(
                context,
                registry,
                subscript,
                addressed_read_preflight,
            )?;
        }
        Ok(())
    }

    fn preflight_slice_ref_subscript_context_reads(
        &self,
        context: &RuntimeContext,
        registry: &RuntimeContextRegistry,
        target: &mech_core::SliceRef,
        addressed_read_preflight: AddressedReadPreflight,
    ) -> MResult<()> {
        if let Some(subscripts) = &target.subscript {
            for subscript in subscripts {
                self.preflight_subscript_context_reads(
                    context,
                    registry,
                    subscript,
                    addressed_read_preflight,
                )?;
            }
        }
        Ok(())
    }

    fn preflight_subscript_context_reads(
        &self,
        context: &RuntimeContext,
        registry: &RuntimeContextRegistry,
        subscript: &mech_core::Subscript,
        addressed_read_preflight: AddressedReadPreflight,
    ) -> MResult<()> {
        match subscript {
            mech_core::Subscript::Brace(subscripts) | mech_core::Subscript::Bracket(subscripts) => {
                for subscript in subscripts {
                    self.preflight_subscript_context_reads(
                        context,
                        registry,
                        subscript,
                        addressed_read_preflight,
                    )?;
                }
            }
            mech_core::Subscript::Formula(factor) => {
                self.preflight_factor_context_reads(
                    context,
                    registry,
                    factor,
                    addressed_read_preflight,
                )?;
            }
            mech_core::Subscript::Range(range) => {
                self.preflight_factor_context_reads(
                    context,
                    registry,
                    &range.start,
                    addressed_read_preflight,
                )?;
                if let Some((_, increment)) = &range.increment {
                    self.preflight_factor_context_reads(
                        context,
                        registry,
                        increment,
                        addressed_read_preflight,
                    )?;
                }
                self.preflight_factor_context_reads(
                    context,
                    registry,
                    &range.terminal,
                    addressed_read_preflight,
                )?;
            }
            _ => {}
        }
        Ok(())
    }

    fn preflight_comprehension_qualifier_context_reads(
        &self,
        context: &RuntimeContext,
        registry: &RuntimeContextRegistry,
        qualifier: &mech_core::ComprehensionQualifier,
        addressed_read_preflight: AddressedReadPreflight,
    ) -> MResult<()> {
        match qualifier {
            mech_core::ComprehensionQualifier::Generator((pattern, expression)) => {
                self.preflight_pattern_context_reads(
                    context,
                    registry,
                    pattern,
                    addressed_read_preflight,
                )?;
                self.preflight_expression_context_reads(
                    context,
                    registry,
                    expression,
                    addressed_read_preflight,
                )
            }
            mech_core::ComprehensionQualifier::Filter(expression) => self
                .preflight_expression_context_reads(
                    context,
                    registry,
                    expression,
                    addressed_read_preflight,
                ),
            mech_core::ComprehensionQualifier::Let(var_def) => self
                .preflight_expression_context_reads(
                    context,
                    registry,
                    &var_def.expression,
                    addressed_read_preflight,
                ),
        }
    }

    fn reject_direct_context_effect_placement(
        &self,
        effect: &str,
        context_name: &str,
        path: &str,
        placement: DirectContextEffectPlacement,
    ) -> MResult<()> {
        let allowed = match effect {
            "assignment" => matches!(placement, DirectContextEffectPlacement::TopLevel),
            "send" => matches!(
                placement,
                DirectContextEffectPlacement::TopLevel
                    | DirectContextEffectPlacement::ActivationScope
            ),
            _ => false,
        };
        if allowed {
            return Ok(());
        }

        Err(MechError::new(
            RuntimeInvalidOperationError {
                operation: "direct_context_effect_placement",
                reason: format!(
                    "context {effect} to `{}` is only supported at module top level, not inside {}",
                    direct_context_target(context_name, path),
                    placement.description(),
                ),
            },
            None,
        ))
    }

    fn preflight_context_send_access(
        &self,
        context: &RuntimeContext,
        registry: &RuntimeContextRegistry,
        context_name: &str,
        path: &str,
        placement: DirectContextEffectPlacement,
    ) -> MResult<()> {
        let Some(binding) = registry.get(context_name) else {
            return Err(undeclared_direct_context_target_error(context_name));
        };
        let resolved = self.resolve_context_resource_request(binding, path)?;
        let operation = context_send_operation(binding, &resolved.context_path)?;
        self.preflight_context_access(
            context,
            registry,
            context_name,
            path,
            operation,
            true,
            Some(provider_write_preflight_owner(
                placement,
                RuntimeResourceWriteIntent::Send,
            )),
        )
    }

    fn preflight_context_access(
        &self,
        context: &RuntimeContext,
        registry: &RuntimeContextRegistry,
        context_name: &str,
        path: &str,
        operation: RuntimeCapabilityOperation,
        require_context_binding: bool,
        write_preflight_owner: Option<ProviderWritePreflightOwner>,
    ) -> MResult<()> {
        let Some(binding) = registry.get(context_name) else {
            if require_context_binding {
                return Err(undeclared_direct_context_target_error(context_name));
            }
            return Ok(());
        };
        let resolved = self.resolve_context_resource_request(binding, path)?;
        let context_allowed = match operation {
            RuntimeCapabilityOperation::Read => {
                runtime_context_allows_read(binding, &resolved.context_path)
            }
            RuntimeCapabilityOperation::Write => {
                runtime_context_allows_write(binding, &resolved.context_path)
            }
            _ => {
                runtime_context_allows_operation(binding, operation.name(), &resolved.context_path)
            }
        };
        if !context_allowed {
            return Err(MechError::new(
                RuntimeResourceCapabilityDenied {
                    context_name: binding.name.clone(),
                    operation: match operation {
                        RuntimeCapabilityOperation::Read => "read".to_string(),
                        RuntimeCapabilityOperation::Write => "write".to_string(),
                        other => format!("{other:?}"),
                    },
                    path: resolved.context_path,
                },
                None,
            ));
        }
        let key = RuntimeResourceKey::new(&resolved.provider_base_uri, &resolved.provider_path)?;
        let request = CapabilityRequest::from_keys(
            &context.subject,
            operation.name(),
            key.capability_resource(),
        );
        self.preview_capability_for_execution(context, &request)?;

        if let Some(ProviderWritePreflightOwner::ContextPreflight(intent)) = write_preflight_owner {
            self.resources
                .preflight_write(RuntimeResourceWritePreflightRequest {
                    base_uri: resolved.provider_base_uri,
                    path: resolved.provider_path,
                    context_name: binding.name.clone(),
                    operation: operation.clone(),
                    intent,
                })?;
        }

        Ok(())
    }

    fn reject_runtime_context_reads_in_statement(
        &self,
        registry: &RuntimeContextRegistry,
        statement: &mech_core::Statement,
    ) -> MResult<()> {
        match statement {
            mech_core::Statement::VariableDefine(var_def) => {
                self.reject_runtime_context_reads_in_expression(registry, &var_def.expression)
            }
            mech_core::Statement::VariableAssign(assign) => {
                self.reject_runtime_context_reads_in_slice_ref(registry, &assign.target)?;
                self.reject_runtime_context_reads_in_expression(registry, &assign.expression)
            }
            mech_core::Statement::OpAssign(op_assign) => {
                self.reject_runtime_context_reads_in_slice_ref(registry, &op_assign.target)?;
                self.reject_runtime_context_reads_in_expression(registry, &op_assign.expression)
            }
            mech_core::Statement::ContextSend(send) => {
                self.reject_runtime_context_reads_in_expression(registry, &send.expression)
            }
            mech_core::Statement::TupleDestructure(tuple_destructure) => self
                .reject_runtime_context_reads_in_expression(
                    registry,
                    &tuple_destructure.expression,
                ),
            mech_core::Statement::FsmDeclare(fsm) => {
                if let Some(args) = &fsm.pipe.start.args {
                    for (_, expression) in args {
                        self.reject_runtime_context_reads_in_expression(registry, expression)?;
                    }
                }
                for transition in &fsm.pipe.transitions {
                    self.reject_runtime_context_reads_in_transition(registry, transition)?;
                }
                Ok(())
            }
            #[cfg(feature = "invariant_define")]
            mech_core::Statement::InvariantDefine(invariant) => {
                self.reject_runtime_context_reads_in_expression(registry, &invariant.expression)
            }
            _ => Ok(()),
        }
    }

    fn reject_runtime_context_reads_in_transition(
        &self,
        registry: &RuntimeContextRegistry,
        transition: &mech_core::Transition,
    ) -> MResult<()> {
        match transition {
            mech_core::Transition::Async(pattern)
            | mech_core::Transition::Next(pattern)
            | mech_core::Transition::Output(pattern) => {
                self.reject_runtime_context_reads_in_pattern(registry, pattern)
            }
            mech_core::Transition::CodeBlock(code_items) => {
                for (code, _) in code_items {
                    if let mech_core::MechCode::Statement(statement) = code {
                        self.reject_runtime_context_reads_in_statement(registry, statement)?;
                    } else if let mech_core::MechCode::Expression(expression) = code {
                        self.reject_runtime_context_reads_in_expression(registry, expression)?;
                    }
                }
                Ok(())
            }
            mech_core::Transition::Statement(statement) => {
                self.reject_runtime_context_reads_in_statement(registry, statement)
            }
        }
    }

    fn reject_runtime_context_reads_in_pattern(
        &self,
        registry: &RuntimeContextRegistry,
        pattern: &mech_core::Pattern,
    ) -> MResult<()> {
        match pattern {
            mech_core::Pattern::Expression(expression) => {
                self.reject_runtime_context_reads_in_expression(registry, expression)
            }
            mech_core::Pattern::TupleStruct(tuple_struct) => {
                for pattern in &tuple_struct.patterns {
                    self.reject_runtime_context_reads_in_pattern(registry, pattern)?;
                }
                Ok(())
            }
            mech_core::Pattern::Tuple(tuple) => {
                for pattern in &tuple.0 {
                    self.reject_runtime_context_reads_in_pattern(registry, pattern)?;
                }
                Ok(())
            }
            mech_core::Pattern::Array(array) => {
                for pattern in &array.prefix {
                    self.reject_runtime_context_reads_in_pattern(registry, pattern)?;
                }
                if let Some(spread) = &array.spread {
                    if let Some(binding) = &spread.binding {
                        self.reject_runtime_context_reads_in_pattern(registry, binding)?;
                    }
                }
                for pattern in &array.suffix {
                    self.reject_runtime_context_reads_in_pattern(registry, pattern)?;
                }
                Ok(())
            }
            mech_core::Pattern::Wildcard => Ok(()),
        }
    }

    fn reject_runtime_context_reads_in_expression(
        &self,
        registry: &RuntimeContextRegistry,
        expression: &mech_core::Expression,
    ) -> MResult<()> {
        match expression {
            mech_core::Expression::Var(var) => {
                if let Some(context_name) = &var.context {
                    if registry.contains(&context_name.to_string()) {
                        self.reject_function_context_read(context_name, &var.name)?;
                    }
                }
                Ok(())
            }
            mech_core::Expression::Formula(factor) => {
                self.reject_runtime_context_reads_in_factor(registry, factor)
            }
            mech_core::Expression::FunctionCall(call) => {
                for (_, expression) in &call.args {
                    self.reject_runtime_context_reads_in_expression(registry, expression)?;
                }
                Ok(())
            }
            mech_core::Expression::FsmPipe(pipe) => {
                if let Some(args) = &pipe.start.args {
                    for (_, expression) in args {
                        self.reject_runtime_context_reads_in_expression(registry, expression)?;
                    }
                }
                for transition in &pipe.transitions {
                    self.reject_runtime_context_reads_in_transition(registry, transition)?;
                }
                Ok(())
            }
            mech_core::Expression::Literal(_) => Ok(()),
            mech_core::Expression::Range(range) => {
                self.reject_runtime_context_reads_in_factor(registry, &range.start)?;
                if let Some((_, increment)) = &range.increment {
                    self.reject_runtime_context_reads_in_factor(registry, increment)?;
                }
                self.reject_runtime_context_reads_in_factor(registry, &range.terminal)
            }
            mech_core::Expression::Structure(structure) => {
                self.reject_runtime_context_reads_in_structure(registry, structure)
            }
            mech_core::Expression::Match(match_expression) => {
                self.reject_runtime_context_reads_in_expression(
                    registry,
                    &match_expression.source,
                )?;
                for arm in &match_expression.arms {
                    self.reject_runtime_context_reads_in_pattern(registry, &arm.pattern)?;
                    if let Some(guard) = &arm.guard {
                        self.reject_runtime_context_reads_in_expression(registry, guard)?;
                    }
                    self.reject_runtime_context_reads_in_expression(registry, &arm.expression)?;
                }
                Ok(())
            }
            mech_core::Expression::Slice(slice) => {
                self.reject_runtime_context_reads_in_slice(registry, slice)
            }
            mech_core::Expression::SetComprehension(comprehension) => {
                self.reject_runtime_context_reads_in_expression(
                    registry,
                    &comprehension.expression,
                )?;
                for qualifier in &comprehension.qualifiers {
                    self.reject_runtime_context_reads_in_comprehension_qualifier(
                        registry, qualifier,
                    )?;
                }
                Ok(())
            }
            mech_core::Expression::MatrixComprehension(comprehension) => {
                self.reject_runtime_context_reads_in_expression(
                    registry,
                    &comprehension.expression,
                )?;
                for qualifier in &comprehension.qualifiers {
                    self.reject_runtime_context_reads_in_comprehension_qualifier(
                        registry, qualifier,
                    )?;
                }
                Ok(())
            }
        }
    }

    fn reject_runtime_context_reads_in_factor(
        &self,
        registry: &RuntimeContextRegistry,
        factor: &mech_core::Factor,
    ) -> MResult<()> {
        match factor {
            mech_core::Factor::Expression(expression) => {
                self.reject_runtime_context_reads_in_expression(registry, expression)
            }
            mech_core::Factor::Negate(factor)
            | mech_core::Factor::Not(factor)
            | mech_core::Factor::Parenthetical(factor)
            | mech_core::Factor::Transpose(factor) => {
                self.reject_runtime_context_reads_in_factor(registry, factor)
            }
            mech_core::Factor::Term(term) => {
                self.reject_runtime_context_reads_in_factor(registry, &term.lhs)?;
                for (_, factor) in &term.rhs {
                    self.reject_runtime_context_reads_in_factor(registry, factor)?;
                }
                Ok(())
            }
        }
    }

    fn reject_runtime_context_reads_in_slice(
        &self,
        registry: &RuntimeContextRegistry,
        slice: &mech_core::Slice,
    ) -> MResult<()> {
        if let Some(context_name) = &slice.context {
            if registry.contains(&context_name.to_string()) {
                self.reject_function_context_read(context_name, &slice.name)?;
            }
        }
        for subscript in &slice.subscript {
            self.reject_runtime_context_reads_in_subscript(registry, subscript)?;
        }
        Ok(())
    }

    fn reject_runtime_context_reads_in_slice_ref(
        &self,
        registry: &RuntimeContextRegistry,
        target: &mech_core::SliceRef,
    ) -> MResult<()> {
        if let Some(subscripts) = &target.subscript {
            for subscript in subscripts {
                self.reject_runtime_context_reads_in_subscript(registry, subscript)?;
            }
        }
        Ok(())
    }

    fn reject_runtime_context_reads_in_subscript(
        &self,
        registry: &RuntimeContextRegistry,
        subscript: &mech_core::Subscript,
    ) -> MResult<()> {
        match subscript {
            mech_core::Subscript::Brace(subscripts) | mech_core::Subscript::Bracket(subscripts) => {
                for subscript in subscripts {
                    self.reject_runtime_context_reads_in_subscript(registry, subscript)?;
                }
                Ok(())
            }
            mech_core::Subscript::Formula(factor) => {
                self.reject_runtime_context_reads_in_factor(registry, factor)
            }
            mech_core::Subscript::Range(range) => {
                self.reject_runtime_context_reads_in_factor(registry, &range.start)?;
                if let Some((_, increment)) = &range.increment {
                    self.reject_runtime_context_reads_in_factor(registry, increment)?;
                }
                self.reject_runtime_context_reads_in_factor(registry, &range.terminal)
            }
            _ => Ok(()),
        }
    }

    fn reject_runtime_context_reads_in_structure(
        &self,
        registry: &RuntimeContextRegistry,
        structure: &mech_core::Structure,
    ) -> MResult<()> {
        match structure {
            mech_core::Structure::Map(map) => {
                for mapping in &map.elements {
                    self.reject_runtime_context_reads_in_expression(registry, &mapping.key)?;
                    self.reject_runtime_context_reads_in_expression(registry, &mapping.value)?;
                }
            }
            mech_core::Structure::Set(set) => {
                for expression in &set.elements {
                    self.reject_runtime_context_reads_in_expression(registry, expression)?;
                }
            }
            mech_core::Structure::Matrix(matrix) => {
                for row in &matrix.rows {
                    for column in &row.columns {
                        self.reject_runtime_context_reads_in_expression(registry, &column.element)?;
                    }
                }
            }
            mech_core::Structure::Record(record) => {
                for binding in &record.bindings {
                    self.reject_runtime_context_reads_in_expression(registry, &binding.value)?;
                }
            }
            mech_core::Structure::Table(table) => {
                for row in &table.rows {
                    for column in &row.columns {
                        self.reject_runtime_context_reads_in_expression(registry, &column.element)?;
                    }
                }
            }
            mech_core::Structure::Tuple(tuple) => {
                for expression in &tuple.elements {
                    self.reject_runtime_context_reads_in_expression(registry, expression)?;
                }
            }
            mech_core::Structure::TupleStruct(tuple_struct) => {
                self.reject_runtime_context_reads_in_expression(registry, &tuple_struct.value)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn reject_runtime_context_reads_in_comprehension_qualifier(
        &self,
        registry: &RuntimeContextRegistry,
        qualifier: &mech_core::ComprehensionQualifier,
    ) -> MResult<()> {
        match qualifier {
            mech_core::ComprehensionQualifier::Generator((pattern, expression)) => {
                self.reject_runtime_context_reads_in_pattern(registry, pattern)?;
                self.reject_runtime_context_reads_in_expression(registry, expression)
            }
            mech_core::ComprehensionQualifier::Filter(expression) => {
                self.reject_runtime_context_reads_in_expression(registry, expression)
            }
            mech_core::ComprehensionQualifier::Let(var_def) => {
                self.reject_runtime_context_reads_in_expression(registry, &var_def.expression)
            }
        }
    }

    fn preflight_fsm_implementation_context_capabilities(
        &self,
        context: &RuntimeContext,
        registry: &RuntimeContextRegistry,
        fsm: &mech_core::FsmImplementation,
        addressed_read_preflight: AddressedReadPreflight,
    ) -> MResult<()> {
        self.preflight_pattern_context_reads(
            context,
            registry,
            &fsm.start,
            addressed_read_preflight,
        )?;
        for arm in &fsm.arms {
            match arm {
                mech_core::FsmArm::Guard(pattern, guards) => {
                    self.preflight_pattern_context_reads(
                        context,
                        registry,
                        pattern,
                        addressed_read_preflight,
                    )?;
                    for guard in guards {
                        self.preflight_pattern_context_reads(
                            context,
                            registry,
                            &guard.condition,
                            addressed_read_preflight,
                        )?;
                        for transition in &guard.transitions {
                            self.preflight_transition_context_capabilities(
                                context,
                                registry,
                                transition,
                                addressed_read_preflight,
                            )?;
                        }
                    }
                }
                mech_core::FsmArm::Transition(pattern, transitions) => {
                    self.preflight_pattern_context_reads(
                        context,
                        registry,
                        pattern,
                        addressed_read_preflight,
                    )?;
                    for transition in transitions {
                        self.preflight_transition_context_capabilities(
                            context,
                            registry,
                            transition,
                            addressed_read_preflight,
                        )?;
                    }
                }
                mech_core::FsmArm::Comment(_) => {}
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DirectContextEffectPlacement {
    TopLevel,
    ActivationScope,
    FunctionBody,
    FsmTransition,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProviderWritePreflightOwner {
    ExternalFunctionInitialization,
    ContextPreflight(RuntimeResourceWriteIntent),
}

fn provider_write_preflight_owner(
    placement: DirectContextEffectPlacement,
    intent: RuntimeResourceWriteIntent,
) -> ProviderWritePreflightOwner {
    match placement {
        // Activation registration deliberately skips external-function
        // initialization. Its provider validation therefore remains owned by
        // the source preflight pass.
        DirectContextEffectPlacement::ActivationScope => {
            ProviderWritePreflightOwner::ContextPreflight(intent)
        }
        // Top-level external functions initialize during the source turn. In a
        // planning runtime that initialization performs the one provider
        // preflight; in an executing runtime it prepares/stages the effect.
        DirectContextEffectPlacement::TopLevel => {
            ProviderWritePreflightOwner::ExternalFunctionInitialization
        }
        // These placements are rejected before provider preflight ownership is
        // consulted.
        DirectContextEffectPlacement::FunctionBody
        | DirectContextEffectPlacement::FsmTransition => {
            ProviderWritePreflightOwner::ExternalFunctionInitialization
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AddressedReadPreflight {
    RequireContextBinding,
    AllowModuleAddressTargets,
}

impl AddressedReadPreflight {
    fn requires_context_binding(self) -> bool {
        matches!(self, AddressedReadPreflight::RequireContextBinding)
    }
}

impl DirectContextEffectPlacement {
    fn description(self) -> &'static str {
        match self {
            DirectContextEffectPlacement::TopLevel => "module top level",
            DirectContextEffectPlacement::ActivationScope => "an activation scope",
            DirectContextEffectPlacement::FunctionBody => "a function body",
            DirectContextEffectPlacement::FsmTransition => "an FSM transition",
        }
    }
}

pub(super) enum RuntimeProgramTarget<'a> {
    Retained,
    Isolated(&'a mut MechProgram),
}

#[derive(Debug, Clone)]
pub struct RuntimeAddressedAssignmentUnsupported {
    pub target: String,
}

impl MechErrorKind for RuntimeAddressedAssignmentUnsupported {
    fn name(&self) -> &str {
        "RuntimeAddressedAssignmentUnsupported"
    }

    fn message(&self) -> String {
        format!(
            "addressed assignment is not supported for `{}`",
            self.target
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ResolvedContextResourceRequest {
    provider_base_uri: String,
    provider_path: String,
    context_path: String,
}

pub(super) fn identifier_from_str(name: &str) -> mech_core::Identifier {
    mech_core::Identifier {
        name: mech_core::Token::new(
            mech_core::TokenKind::Identifier,
            mech_core::SourceRange::default(),
            name.chars().collect(),
        ),
    }
}

fn runtime_context_base_uri(binding: &RuntimeContextBinding) -> String {
    match &binding.base {
        RuntimeContextBase::ResourceUri(uri) => uri.clone(),
    }
}

fn runtime_context_allows_operation(
    binding: &RuntimeContextBinding,
    operation: &str,
    path: &str,
) -> bool {
    binding.capabilities.iter().any(|capability| {
        if capability.operation != operation {
            return false;
        }

        match &capability.scope {
            RuntimeContextCapabilityScope::Wildcard => true,
            RuntimeContextCapabilityScope::Path(exact) => {
                if exact == path {
                    return true;
                }
                if let Some(prefix) = exact.strip_suffix("/*") {
                    let required_prefix = format!("{}/", prefix);
                    return path.starts_with(&required_prefix);
                }
                false
            }
        }
    })
}

fn runtime_context_exposes_operation(binding: &RuntimeContextBinding, operation: &str) -> bool {
    binding
        .capabilities
        .iter()
        .any(|capability| capability.operation == operation)
}

fn runtime_context_allows_read(binding: &RuntimeContextBinding, path: &str) -> bool {
    runtime_context_allows_operation(binding, "read", path)
}

fn runtime_context_allows_write(binding: &RuntimeContextBinding, path: &str) -> bool {
    runtime_context_allows_operation(binding, "write", path)
}

fn first_context_send_segment(path: &str) -> MResult<&str> {
    path.split('/')
        .next()
        .filter(|segment| !segment.is_empty())
        .ok_or_else(|| {
            MechError::new(
                RuntimeInvalidOperationError {
                    operation: "context_send",
                    reason: "context send target path must start with an operation name"
                        .to_string(),
                },
                None,
            )
        })
}

fn reserved_context_send_operation_error(operation: &str, path: &str) -> MechError {
    MechError::new(
        RuntimeInvalidOperationError {
            operation: "context_send",
            reason: format!(
                "context send target path `{path}` starts with reserved operation `{operation}`; use assignment for writes or a custom send operation"
            ),
        },
        None,
    )
}

fn context_send_operation(
    binding: &RuntimeContextBinding,
    path: &str,
) -> MResult<RuntimeCapabilityOperation> {
    let candidate = first_context_send_segment(path)?;
    if candidate == "read" {
        return Err(reserved_context_send_operation_error(candidate, path));
    }
    if candidate == "write" {
        if runtime_context_allows_write(binding, path) {
            return Ok(RuntimeCapabilityOperation::Write);
        }
        return Err(reserved_context_send_operation_error(candidate, path));
    }
    if runtime_context_allows_operation(binding, candidate, path) {
        return RuntimeCapabilityOperation::from_name(candidate.to_string());
    }
    if runtime_context_exposes_operation(binding, candidate) {
        return RuntimeCapabilityOperation::from_name(candidate.to_string());
    }
    if runtime_context_allows_write(binding, path) {
        return Ok(RuntimeCapabilityOperation::Write);
    }
    RuntimeCapabilityOperation::from_name(candidate.to_string())
}

fn context_write_operation(
    binding: &RuntimeContextBinding,
    intent: RuntimeResourceWriteIntent,
    path: &str,
) -> MResult<RuntimeCapabilityOperation> {
    match intent {
        RuntimeResourceWriteIntent::Assign => Ok(RuntimeCapabilityOperation::Write),
        RuntimeResourceWriteIntent::Send => context_send_operation(binding, path),
    }
}

fn direct_context_target(context_name: &str, path: &str) -> String {
    format!("@{}/{}", context_name, path)
}

fn undeclared_direct_context_target_error(context_name: &str) -> MechError {
    MechError::new(
        RuntimeInvalidOperationError {
            operation: "direct_context_target",
            reason: format!("context target `@{context_name}` is not declared or imported"),
        },
        None,
    )
}

pub(super) fn single_code_program(
    code: mech_core::MechCode,
    comment: Option<mech_core::Comment>,
) -> mech_core::Program {
    mech_core::Program {
        title: None,
        body: mech_core::Body {
            sections: vec![mech_core::Section {
                subtitle: None,
                elements: vec![mech_core::SectionElement::MechCode(vec![(code, comment)])],
            }],
        },
    }
}

pub(super) fn resolve_runtime_value(value: Value) -> Value {
    match value {
        Value::MutableReference(value) => value.borrow().clone(),
        other => other,
    }
}
