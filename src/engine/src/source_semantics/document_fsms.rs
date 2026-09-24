//! Declared finite-state machines lower into the canonical lexical control graph.
//!
//! The document declaration owns nominal state typing. Synchronous transitions
//! reuse the resident match/recursion executor; suspension extends this same
//! representation rather than invoking the source interpreter.

use super::*;
use mech_syntax::document::{
    FsmArmBodySyntax, FsmBodyTransitionSyntax, FsmValueSyntax, IdentifierSyntax,
};

#[derive(Clone)]
struct DeclaredFsmParameter {
    name: String,
    schema: SchemaDraft,
}

#[derive(Clone)]
struct DeclaredFsmState {
    ordinal: u32,
    fields: Box<[SchemaBody]>,
}

#[derive(Clone)]
pub(in crate::source_semantics) struct DeclaredFsm {
    name: String,
    parameters: Vec<DeclaredFsmParameter>,
    implementation_parameters: Vec<String>,
    output: SchemaDraft,
    state_schema: SchemaDraft,
    states: BTreeMap<String, DeclaredFsmState>,
    implementation: mech_syntax::document::FsmImplementationSyntax,
}

#[derive(Clone)]
struct FsmControlArm {
    pattern: PatternSyntax,
    guard: Option<ExpressionSyntax>,
    transitions: Vec<FsmBodyTransitionSyntax>,
    syntax: SyntaxNode,
}

fn fsm_payload_irrefutable<S, V>(pattern: &crate::CollectionPattern<S, V>) -> bool {
    match pattern {
        crate::CollectionPattern::Wildcard | crate::CollectionPattern::Bind { .. } => true,
        crate::CollectionPattern::Tuple(items) => items.iter().all(fsm_payload_irrefutable),
        crate::CollectionPattern::Array {
            prefix,
            rest: Some(rest),
            suffix,
        } if prefix.is_empty() && suffix.is_empty() => fsm_payload_irrefutable(rest),
        crate::CollectionPattern::Equal(_)
        | crate::CollectionPattern::Enum { .. }
        | crate::CollectionPattern::Array { .. } => false,
    }
}

fn variable_name(
    variable: &VariableSyntax,
    role: &'static str,
) -> Result<String, SourceSemanticError> {
    let Some(VariableStemSyntax::Identifier(identifier)) = variable.stem() else {
        return Err(SourceSemanticError {
            code: "source-semantics/invalid-fsm-declaration",
            message: format!("{role} requires a lexical identifier"),
            anchor: SourceSemanticAnchor::for_node(variable.syntax()),
        });
    };
    node_text(identifier.syntax())
}

fn standalone_atom(expression: &ExpressionSyntax) -> Option<IdentifierSyntax> {
    fn find(node: &SyntaxNode, range: TextRange) -> Option<IdentifierSyntax> {
        if let Some(atom) = mech_syntax::document::AtomLiteralSyntax::cast(node.clone())
            && node.range() == range
        {
            return atom.name();
        }
        node.children().find_map(|child| find(&child, range))
    }
    find(expression.syntax(), expression.syntax().range())
}

impl SemanticBuilder {
    pub(super) fn register_document_fsms(
        &mut self,
        units: &[DocumentUnit],
        nominal_namespace: &[String],
    ) -> Result<(), SourceSemanticError> {
        let mut specifications = BTreeMap::new();
        let mut implementations = BTreeMap::new();
        fn collect(
            units: &[DocumentUnit],
            specifications: &mut BTreeMap<String, mech_syntax::document::FsmSpecificationSyntax>,
            implementations: &mut BTreeMap<String, mech_syntax::document::FsmImplementationSyntax>,
        ) -> Result<(), SourceSemanticError> {
            for unit in units {
                match unit {
                    DocumentUnit::FsmSpecification(specification) => {
                        let name = specification.name().ok_or_else(|| {
                            internal(
                                SourceSemanticAnchor::for_node(specification.syntax()),
                                "FSM specification has no name".to_owned(),
                            )
                        })?;
                        let name = node_text(name.syntax())?;
                        if specifications
                            .insert(name.clone(), specification.clone())
                            .is_some()
                        {
                            return Err(SourceSemanticError {
                                code: "source-semantics/duplicate-fsm-specification",
                                message: format!("FSM {name} is specified more than once"),
                                anchor: SourceSemanticAnchor::for_node(specification.syntax()),
                            });
                        }
                    }
                    DocumentUnit::FsmImplementation(implementation) => {
                        let name = implementation.name().ok_or_else(|| {
                            internal(
                                SourceSemanticAnchor::for_node(implementation.syntax()),
                                "FSM implementation has no name".to_owned(),
                            )
                        })?;
                        let name = node_text(name.syntax())?;
                        if implementations
                            .insert(name.clone(), implementation.clone())
                            .is_some()
                        {
                            return Err(SourceSemanticError {
                                code: "source-semantics/duplicate-fsm-implementation",
                                message: format!("FSM {name} is implemented more than once"),
                                anchor: SourceSemanticAnchor::for_node(implementation.syntax()),
                            });
                        }
                    }
                    DocumentUnit::Fence(_, _, nested) => {
                        collect(nested, specifications, implementations)?
                    }
                    _ => {}
                }
            }
            Ok(())
        }
        collect(units, &mut specifications, &mut implementations)?;

        for (name, specification) in specifications {
            let implementation =
                implementations
                    .remove(&name)
                    .ok_or_else(|| SourceSemanticError {
                        code: "source-semantics/missing-fsm-implementation",
                        message: format!("FSM {name} has no implementation"),
                        anchor: SourceSemanticAnchor::for_node(specification.syntax()),
                    })?;
            let mut parameter_names = BTreeSet::new();
            let parameters = specification
                .inputs()
                .into_iter()
                .map(|variable| {
                    let name = variable_name(&variable, "FSM input")?;
                    if !parameter_names.insert(name.clone()) {
                        return Err(SourceSemanticError {
                            code: "source-semantics/duplicate-fsm-parameter",
                            message: format!("FSM parameter {name} is declared more than once"),
                            anchor: SourceSemanticAnchor::for_node(variable.syntax()),
                        });
                    }
                    let annotation = variable.annotation().ok_or_else(|| SourceSemanticError {
                        code: "source-semantics/untyped-fsm-parameter",
                        message: format!("FSM parameter {name} requires a kind"),
                        anchor: SourceSemanticAnchor::for_node(variable.syntax()),
                    })?;
                    Ok(DeclaredFsmParameter {
                        name,
                        schema: self.annotation_schema_draft(&annotation)?,
                    })
                })
                .collect::<Result<Vec<_>, SourceSemanticError>>()?;
            let implementation_parameters = implementation
                .inputs()
                .into_iter()
                .map(|variable| variable_name(&variable, "FSM implementation input"))
                .collect::<Result<Vec<_>, _>>()?;
            let mut implementation_names = BTreeSet::new();
            if let Some(duplicate) = implementation_parameters
                .iter()
                .find(|name| !implementation_names.insert((*name).clone()))
            {
                return Err(SourceSemanticError {
                    code: "source-semantics/duplicate-fsm-implementation-parameter",
                    message: format!(
                        "FSM implementation parameter {duplicate} is declared more than once"
                    ),
                    anchor: SourceSemanticAnchor::for_node(implementation.syntax()),
                });
            }
            if implementation_parameters.len() != parameters.len() {
                return Err(SourceSemanticError {
                    code: "source-semantics/fsm-parameter-count-mismatch",
                    message: format!(
                        "FSM {name} implementation declares {} inputs but its specification declares {}",
                        implementation_parameters.len(),
                        parameters.len()
                    ),
                    anchor: SourceSemanticAnchor::for_node(implementation.syntax()),
                });
            }
            let output = specification.output().ok_or_else(|| SourceSemanticError {
                code: "source-semantics/missing-fsm-output-kind",
                message: format!("FSM {name} requires an output kind"),
                anchor: SourceSemanticAnchor::for_node(specification.syntax()),
            })?;
            let output = self.annotation_schema_draft(&output)?;
            let path = CanonicalNominalPath::new(
                nominal_namespace
                    .iter()
                    .cloned()
                    .chain(["fsm".to_owned(), name.clone()])
                    .collect::<Vec<_>>(),
            )
            .map_err(|error| {
                internal(
                    SourceSemanticAnchor::for_node(specification.syntax()),
                    format!("invalid FSM nominal path: {error:?}"),
                )
            })?;
            let mut states = BTreeMap::new();
            let mut variants = Vec::new();
            for (ordinal, state) in specification.states().into_iter().enumerate() {
                let state_name = state.name().ok_or_else(|| {
                    internal(
                        SourceSemanticAnchor::for_node(state.syntax()),
                        "FSM state has no name".to_owned(),
                    )
                })?;
                let state_name = node_text(state_name.syntax())?;
                let fields = state
                    .variables()
                    .into_iter()
                    .map(|variable| {
                        let annotation =
                            variable.annotation().ok_or_else(|| SourceSemanticError {
                                code: "source-semantics/untyped-fsm-state",
                                message: format!("FSM state {state_name} requires typed payloads"),
                                anchor: SourceSemanticAnchor::for_node(variable.syntax()),
                            })?;
                        let schema = self.annotation_schema_draft(&annotation)?;
                        if !schema.dimension_parameters.is_empty() {
                            return Err(SourceSemanticError {
                                code: "source-semantics/fsm-state-requires-closed-kind",
                                message: format!(
                                    "FSM state {state_name} requires closed payload kinds"
                                ),
                                anchor: SourceSemanticAnchor::for_node(variable.syntax()),
                            });
                        }
                        Ok(schema.body)
                    })
                    .collect::<Result<Vec<_>, SourceSemanticError>>()?;
                let payload = match fields.as_slice() {
                    [] => None,
                    [field] => Some(field.clone()),
                    _ => Some(SchemaBody::Tuple(fields.clone().into_boxed_slice())),
                };
                let ordinal = u32::try_from(ordinal).map_err(|_| SourceSemanticError {
                    code: "source-semantics/fsm-state-identity-exhausted",
                    message: format!("FSM {name} has too many states"),
                    anchor: SourceSemanticAnchor::for_node(state.syntax()),
                })?;
                if states
                    .insert(
                        state_name.clone(),
                        DeclaredFsmState {
                            ordinal,
                            fields: fields.into_boxed_slice(),
                        },
                    )
                    .is_some()
                {
                    return Err(SourceSemanticError {
                        code: "source-semantics/duplicate-fsm-state",
                        message: format!("FSM state {state_name} is declared more than once"),
                        anchor: SourceSemanticAnchor::for_node(state.syntax()),
                    });
                }
                variants.push(mech_core::EnumVariantSchema {
                    name: state_name,
                    payload,
                });
            }
            let state_schema = SchemaDraft {
                body: SchemaBody::Enum {
                    key: NominalKey::from_path(NominalKind::Enum, &path),
                    variants: variants.into_boxed_slice(),
                },
                dimension_parameters: Box::new([]),
            };
            self.local_fsms.insert(
                name.clone(),
                DeclaredFsm {
                    name,
                    parameters,
                    implementation_parameters,
                    output,
                    state_schema,
                    states,
                    implementation,
                },
            );
        }
        if let Some((name, implementation)) = implementations.into_iter().next() {
            return Err(SourceSemanticError {
                code: "source-semantics/missing-fsm-specification",
                message: format!("FSM {name} has no specification"),
                anchor: SourceSemanticAnchor::for_node(implementation.syntax()),
            });
        }
        Ok(())
    }

    pub(in crate::source_semantics) fn inline_document_fsm(
        &mut self,
        pipe: &FsmPipeSyntax,
    ) -> Result<Option<PendingValue>, SourceSemanticError> {
        let instance = self.required(pipe.instance(), pipe.syntax(), "an FSM instance")?;
        let name = self.required(instance.name(), instance.syntax(), "an FSM name")?;
        let name = node_text(name.syntax())?;
        let Some(machine) = self.local_fsms.get(&name).cloned() else {
            if pipe.stages().is_empty() {
                return Err(SourceSemanticError {
                    code: "source-semantics/unknown-fsm",
                    message: format!("FSM {name} has no retained declaration"),
                    anchor: SourceSemanticAnchor::for_node(instance.syntax()),
                });
            }
            return Ok(None);
        };
        if self.active_fsms.iter().any(|active| active == &name) {
            return Err(SourceSemanticError {
                code: "source-semantics/recursive-fsm-invocation",
                message: format!("FSM {name} recursively invokes an active FSM declaration"),
                anchor: SourceSemanticAnchor::for_node(pipe.syntax()),
            });
        }
        if !pipe.stages().is_empty() {
            return Err(SourceSemanticError {
                code: "source-semantics/declared-fsm-pipe-stages",
                message: "a declared FSM invocation cannot append an undeclared pipe".to_owned(),
                anchor: SourceSemanticAnchor::for_node(pipe.syntax()),
            });
        }
        if self.control_depth > 0
            && self.fsm_control_arms(&machine)?.iter().any(|arm| {
                arm.transitions
                    .iter()
                    .any(|transition| matches!(transition, FsmBodyTransitionSyntax::Async(_)))
            })
        {
            return Err(SourceSemanticError {
                code: "source-semantics/unsupported-nested-fsm-suspension",
                message:
                    "an FSM with a suspended transition cannot run inside another FSM control block"
                        .to_owned(),
                anchor: SourceSemanticAnchor::for_node(pipe.syntax()),
            });
        }
        let mut selected = vec![None; machine.parameters.len()];
        if let Some(arguments) = instance.arguments() {
            for argument in arguments.arguments() {
                let anchor = SourceSemanticAnchor::for_node(argument.syntax());
                let (supplied_name, expression) = match argument {
                    AnyCallArgumentSyntax::Positional(argument) => (
                        None,
                        self.required(argument.value(), argument.syntax(), "an FSM argument")?,
                    ),
                    AnyCallArgumentSyntax::Bound(argument) => {
                        let supplied = self.required(
                            argument.name(),
                            argument.syntax(),
                            "an FSM argument name",
                        )?;
                        (
                            Some(node_text(supplied.syntax())?),
                            self.required(argument.value(), argument.syntax(), "an FSM argument")?,
                        )
                    }
                };
                let ordinal = supplied_name.as_ref().map_or_else(
                    || selected.iter().position(Option::is_none),
                    |supplied| {
                        machine
                            .parameters
                            .iter()
                            .position(|parameter| &parameter.name == supplied)
                    },
                );
                let Some(ordinal) = ordinal else {
                    return Err(SourceSemanticError {
                        code: "source-semantics/unknown-fsm-argument",
                        message: supplied_name.map_or_else(
                            || format!("FSM {} received too many arguments", machine.name),
                            |name| format!("FSM {} has no parameter {name}", machine.name),
                        ),
                        anchor,
                    });
                };
                if selected[ordinal].is_some() {
                    return Err(SourceSemanticError {
                        code: "source-semantics/duplicate-fsm-argument",
                        message: format!(
                            "FSM parameter {} is supplied more than once",
                            machine.parameters[ordinal].name
                        ),
                        anchor,
                    });
                }
                let value = self.expression(&expression)?.0;
                selected[ordinal] = Some(self.conform_schema_draft(
                    value,
                    &machine.parameters[ordinal].schema,
                    expression.syntax(),
                    "source-semantics/incompatible-fsm-argument",
                    "FSM argument does not satisfy its declared kind",
                )?);
            }
        }
        let selected = selected
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| SourceSemanticError {
                code: "source-semantics/missing-fsm-argument",
                message: format!("FSM {} requires all declared arguments", machine.name),
                anchor: SourceSemanticAnchor::for_node(instance.syntax()),
            })?;
        let saved_bindings = self.bindings.clone();
        for (parameter, value) in machine.implementation_parameters.iter().zip(selected) {
            // Invocation arguments are lexical values. A direct read of the
            // same external input inside the FSM remains live on resume.
            let value = match value {
                PendingValue::Input(index) => PendingValue::LexicalInput(index),
                value => value,
            };
            self.bindings
                .insert(parameter.clone(), PendingBinding::Value(value));
        }
        self.active_fsms.push(name);
        let result = (|| {
            let start = machine.implementation.start().ok_or_else(|| {
                internal(
                    SourceSemanticAnchor::for_node(machine.implementation.syntax()),
                    format!("FSM {} implementation has no start state", machine.name),
                )
            })?;
            let start = self.fsm_declared_state_value(&machine, &start)?;
            self.lower_declared_fsm_match(&machine, start, pipe.syntax())
        })();
        self.active_fsms.pop();
        self.bindings = saved_bindings;
        result.map(Some)
    }

    fn lower_declared_fsm_value(
        &mut self,
        pattern: &PatternSyntax,
        expected: &SchemaDraft,
        incompatible_code: &'static str,
        incompatible_message: &'static str,
    ) -> Result<PendingValue, SourceSemanticError> {
        let invalid = |message: String| SourceSemanticError {
            code: incompatible_code,
            message,
            anchor: SourceSemanticAnchor::for_node(pattern.syntax()),
        };
        let body = self.required(pattern.value(), pattern.syntax(), "an FSM value")?;
        match body {
            PatternValueSyntax::Expression(expression) => {
                let value = self.expression(&expression)?.0;
                self.conform_schema_draft(
                    value,
                    expected,
                    expression.syntax(),
                    incompatible_code,
                    incompatible_message,
                )
            }
            PatternValueSyntax::Tuple(tuple) => {
                let items = tuple.items();
                if let [item] = items.as_slice() {
                    return self.lower_declared_fsm_value(
                        item,
                        expected,
                        incompatible_code,
                        incompatible_message,
                    );
                }
                let SchemaBody::Tuple(fields) = &expected.body else {
                    return Err(invalid(
                        "FSM tuple value does not satisfy its declared kind".to_owned(),
                    ));
                };
                if fields.len() != items.len() {
                    return Err(invalid(format!(
                        "FSM tuple value requires {} items, received {}",
                        fields.len(),
                        items.len()
                    )));
                }
                let values = items
                    .iter()
                    .zip(fields)
                    .map(|(item, field)| {
                        self.lower_declared_fsm_value(
                            item,
                            &schema_component(expected, field),
                            incompatible_code,
                            incompatible_message,
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(self.emit_with_schema_draft(
                    "core/composite-pack",
                    values,
                    expected.clone(),
                    pattern.syntax(),
                    "fsm-structured-value",
                    None,
                ))
            }
            PatternValueSyntax::Array(array) => {
                let SchemaBody::Matrix { element, .. } = &expected.body else {
                    return Err(invalid(
                        "FSM array value does not satisfy its declared kind".to_owned(),
                    ));
                };
                let element = schema_component(expected, element);
                let values = array
                    .elements()
                    .iter()
                    .map(|item| {
                        if item.spread().is_some() || item.rest().is_some() {
                            return Err(invalid(
                                "FSM array values cannot contain a spread or rest item".to_owned(),
                            ));
                        }
                        let item =
                            self.required(item.pattern(), item.syntax(), "an FSM array value")?;
                        self.lower_declared_fsm_value(
                            &item,
                            &element,
                            incompatible_code,
                            incompatible_message,
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let Some((inputs, output)) =
                    self.resolve_maintained_call("matrix/horzcat", values, pattern.syntax())?
                else {
                    return Err(invalid(
                        "FSM array value has no maintained matrix construction".to_owned(),
                    ));
                };
                let value = self.emit_with_schema_draft(
                    "matrix/horzcat",
                    inputs,
                    output,
                    pattern.syntax(),
                    "fsm-array-value",
                    None,
                );
                self.conform_schema_draft(
                    value,
                    expected,
                    pattern.syntax(),
                    incompatible_code,
                    incompatible_message,
                )
            }
            PatternValueSyntax::AtomStruct(value) => {
                let name = self.required(value.name(), value.syntax(), "an FSM value name")?;
                self.lower_declared_fsm_structured_value(
                    node_text(name.syntax())?,
                    value.items(),
                    pattern.syntax(),
                    expected,
                    incompatible_code,
                    incompatible_message,
                )
            }
            PatternValueSyntax::TupleStruct(value) => {
                let name = self.required(value.name(), value.syntax(), "an FSM value name")?;
                self.lower_declared_fsm_structured_value(
                    node_text(name.syntax())?,
                    value.items(),
                    pattern.syntax(),
                    expected,
                    incompatible_code,
                    incompatible_message,
                )
            }
            PatternValueSyntax::Wildcard(_) => {
                Err(invalid("wildcards cannot construct FSM values".to_owned()))
            }
        }
    }

    fn lower_declared_fsm_structured_value(
        &mut self,
        name: String,
        items: Vec<PatternSyntax>,
        syntax: &SyntaxNode,
        expected: &SchemaDraft,
        incompatible_code: &'static str,
        incompatible_message: &'static str,
    ) -> Result<PendingValue, SourceSemanticError> {
        let invalid = |message: String| SourceSemanticError {
            code: incompatible_code,
            message,
            anchor: SourceSemanticAnchor::for_node(syntax),
        };
        let SchemaBody::Enum { variants, .. } = &expected.body else {
            return Err(invalid(
                "FSM structured value does not satisfy its declared kind".to_owned(),
            ));
        };
        let (ordinal, variant) = variants
            .iter()
            .enumerate()
            .find(|(_, variant)| variant.name == name)
            .ok_or_else(|| invalid(format!("declared kind has no variant {name}")))?;
        let payload = match (&variant.payload, items.as_slice()) {
            (None, []) => None,
            (Some(payload), [item]) => Some(self.lower_declared_fsm_value(
                item,
                &schema_component(expected, payload),
                incompatible_code,
                incompatible_message,
            )?),
            (Some(payload @ SchemaBody::Tuple(fields)), items) if fields.len() == items.len() => {
                let payload_schema = schema_component(expected, payload);
                let values = items
                    .iter()
                    .zip(fields)
                    .map(|(item, field)| {
                        self.lower_declared_fsm_value(
                            item,
                            &schema_component(&payload_schema, field),
                            incompatible_code,
                            incompatible_message,
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Some(self.emit_with_schema_draft(
                    "core/composite-pack",
                    values,
                    payload_schema,
                    syntax,
                    "fsm-structured-payload",
                    Some(name.clone()),
                ))
            }
            (None, _) => {
                return Err(invalid(format!(
                    "FSM variant {name} does not accept a payload"
                )));
            }
            (Some(_), _) => {
                return Err(invalid(format!(
                    "FSM variant {name} received an incompatible payload"
                )));
            }
        };
        let ordinal = u32::try_from(ordinal)
            .map_err(|_| invalid("FSM variant identity is exhausted".to_owned()))?;
        let Some(payload) = payload else {
            return Ok(self.constant_draft(
                expected.clone(),
                ValueDataDraft::Enum(EnumDraft {
                    ordinal,
                    payload: None,
                }),
            ));
        };
        if let PendingValue::Constant(payload) = payload {
            return Ok(self.constant_draft(
                expected.clone(),
                ValueDataDraft::Enum(EnumDraft {
                    ordinal,
                    payload: Some(Box::new(self.constants[payload].data.clone())),
                }),
            ));
        }
        let ordinal = self.constant(
            BuiltinSchema::Index,
            ValueDataDraft::Index(u64::from(ordinal) + 1),
        );
        Ok(self.emit_with_schema_draft(
            "core/enum-pack",
            vec![ordinal, payload],
            expected.clone(),
            syntax,
            "fsm-structured-value",
            Some(name),
        ))
    }

    fn fsm_declared_state_value(
        &mut self,
        machine: &DeclaredFsm,
        value: &FsmValueSyntax,
    ) -> Result<PendingValue, SourceSemanticError> {
        let pattern = self.required(value.pattern(), value.syntax(), "an FSM state value")?;
        let body = self.required(pattern.value(), pattern.syntax(), "an FSM state value")?;
        let (name, items) = match body {
            PatternValueSyntax::AtomStruct(value) => (
                self.required(value.name(), value.syntax(), "an FSM state name")?,
                value.items(),
            ),
            PatternValueSyntax::TupleStruct(value) => (
                self.required(value.name(), value.syntax(), "an FSM state name")?,
                value.items(),
            ),
            PatternValueSyntax::Expression(expression) => {
                let name = standalone_atom(&expression).ok_or_else(|| SourceSemanticError {
                    code: "source-semantics/invalid-fsm-state-value",
                    message: "FSM transitions require a declared state constructor".to_owned(),
                    anchor: SourceSemanticAnchor::for_node(expression.syntax()),
                })?;
                (name, Vec::new())
            }
            _ => {
                return Err(SourceSemanticError {
                    code: "source-semantics/invalid-fsm-state-value",
                    message: "FSM transitions require a declared state constructor".to_owned(),
                    anchor: SourceSemanticAnchor::for_node(pattern.syntax()),
                });
            }
        };
        let name = node_text(name.syntax())?;
        let state = machine
            .states
            .get(&name)
            .ok_or_else(|| SourceSemanticError {
                code: "source-semantics/unknown-fsm-state",
                message: format!("FSM {} has no state {name}", machine.name),
                anchor: SourceSemanticAnchor::for_node(pattern.syntax()),
            })?;
        let payload = match (state.fields.as_ref(), items.as_slice()) {
            ([], []) => None,
            ([field], [item]) => {
                let expected = SchemaDraft {
                    body: field.clone(),
                    dimension_parameters: Box::new([]),
                };
                Some(self.lower_declared_fsm_value(
                    item,
                    &expected,
                    "source-semantics/incompatible-fsm-state-payload",
                    "FSM state payload does not satisfy its declared kind",
                )?)
            }
            (fields, items) if fields.len() > 1 => {
                if fields.len() != items.len() {
                    return Err(SourceSemanticError {
                        code: "source-semantics/invalid-fsm-state-payload",
                        message: format!(
                            "FSM state {name} requires {} payloads, received {}",
                            fields.len(),
                            items.len()
                        ),
                        anchor: SourceSemanticAnchor::for_node(pattern.syntax()),
                    });
                }
                let values = items
                    .iter()
                    .zip(fields)
                    .map(|(item, field)| {
                        self.lower_declared_fsm_value(
                            item,
                            &SchemaDraft {
                                body: field.clone(),
                                dimension_parameters: Box::new([]),
                            },
                            "source-semantics/incompatible-fsm-state-payload",
                            "FSM state payload does not satisfy its declared kind",
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let expected = SchemaDraft {
                    body: SchemaBody::Tuple(fields.to_vec().into_boxed_slice()),
                    dimension_parameters: Box::new([]),
                };
                Some(self.emit_with_schema_draft(
                    "core/composite-pack",
                    values,
                    expected,
                    pattern.syntax(),
                    "fsm-state-payload",
                    Some(name.clone()),
                ))
            }
            _ => {
                return Err(SourceSemanticError {
                    code: "source-semantics/invalid-fsm-state-payload",
                    message: format!(
                        "FSM state {name} requires {} payloads, received {}",
                        state.fields.len(),
                        items.len()
                    ),
                    anchor: SourceSemanticAnchor::for_node(pattern.syntax()),
                });
            }
        };
        if let Some(PendingValue::Constant(payload)) = payload {
            return Ok(self.constant_draft(
                machine.state_schema.clone(),
                ValueDataDraft::Enum(EnumDraft {
                    ordinal: state.ordinal,
                    payload: Some(Box::new(self.constants[payload].data.clone())),
                }),
            ));
        }
        if payload.is_none() {
            return Ok(self.constant_draft(
                machine.state_schema.clone(),
                ValueDataDraft::Enum(EnumDraft {
                    ordinal: state.ordinal,
                    payload: None,
                }),
            ));
        }
        let ordinal = self.constant(
            BuiltinSchema::Index,
            ValueDataDraft::Index(u64::from(state.ordinal) + 1),
        );
        Ok(self.emit_with_schema_draft(
            "core/enum-pack",
            vec![ordinal, payload.unwrap()],
            machine.state_schema.clone(),
            pattern.syntax(),
            "fsm-state",
            Some(name),
        ))
    }

    fn fsm_output_value(
        &mut self,
        machine: &DeclaredFsm,
        value: &FsmValueSyntax,
    ) -> Result<PendingValue, SourceSemanticError> {
        let pattern = self.required(value.pattern(), value.syntax(), "an FSM output value")?;
        self.lower_declared_fsm_value(
            &pattern,
            &machine.output,
            "source-semantics/incompatible-fsm-output",
            "FSM output does not satisfy its declared kind",
        )
    }

    fn fsm_control_arms(
        &self,
        machine: &DeclaredFsm,
    ) -> Result<Vec<FsmControlArm>, SourceSemanticError> {
        let mut output = Vec::new();
        for arm in machine.implementation.arms() {
            match arm.body() {
                Some(FsmArmBodySyntax::Transition(transition)) => {
                    let pattern = transition.pattern().ok_or_else(|| {
                        internal(
                            SourceSemanticAnchor::for_node(transition.syntax()),
                            "FSM transition has no state pattern".to_owned(),
                        )
                    })?;
                    output.push(FsmControlArm {
                        pattern,
                        guard: None,
                        transitions: transition.transitions(),
                        syntax: transition.syntax().clone(),
                    });
                }
                Some(FsmArmBodySyntax::Guard(guarded)) => {
                    let pattern = guarded.pattern().ok_or_else(|| {
                        internal(
                            SourceSemanticAnchor::for_node(guarded.syntax()),
                            "FSM guarded transition has no state pattern".to_owned(),
                        )
                    })?;
                    for guard in guarded.guards() {
                        let condition = guard.condition().ok_or_else(|| {
                            internal(
                                SourceSemanticAnchor::for_node(guard.syntax()),
                                "FSM guard has no condition".to_owned(),
                            )
                        })?;
                        let condition = match self.required(
                            condition.value(),
                            condition.syntax(),
                            "an FSM guard condition",
                        )? {
                            PatternValueSyntax::Wildcard(_) => None,
                            PatternValueSyntax::Expression(expression) => Some(expression),
                            _ => {
                                return Err(SourceSemanticError {
                                    code: "source-semantics/invalid-fsm-guard",
                                    message: "FSM guards require Boolean expressions or a wildcard"
                                        .to_owned(),
                                    anchor: SourceSemanticAnchor::for_node(condition.syntax()),
                                });
                            }
                        };
                        output.push(FsmControlArm {
                            pattern: pattern.clone(),
                            guard: condition,
                            transitions: guard.transitions(),
                            syntax: guard.syntax().clone(),
                        });
                    }
                }
                Some(FsmArmBodySyntax::Comment(_)) => {}
                None => {
                    return Err(internal(
                        SourceSemanticAnchor::for_node(arm.syntax()),
                        "FSM arm has no selected body".to_owned(),
                    ));
                }
            }
        }
        Ok(output)
    }

    fn fsm_state_pattern(
        &mut self,
        machine: &DeclaredFsm,
        pattern: &PatternSyntax,
        binding_start: usize,
        names: &mut BTreeMap<String, PendingValue>,
    ) -> Result<comprehension::SourcePattern, SourceSemanticError> {
        let body = self.required(pattern.value(), pattern.syntax(), "an FSM state pattern")?;
        let (name, items) = match body {
            PatternValueSyntax::AtomStruct(value) => (
                self.required(value.name(), value.syntax(), "an FSM state name")?,
                value.items(),
            ),
            PatternValueSyntax::TupleStruct(value) => (
                self.required(value.name(), value.syntax(), "an FSM state name")?,
                value.items(),
            ),
            PatternValueSyntax::Expression(expression) => {
                let name = standalone_atom(&expression).ok_or_else(|| SourceSemanticError {
                    code: "source-semantics/invalid-fsm-state-pattern",
                    message: "FSM arms require a declared state pattern".to_owned(),
                    anchor: SourceSemanticAnchor::for_node(expression.syntax()),
                })?;
                (name, Vec::new())
            }
            _ => {
                return Err(SourceSemanticError {
                    code: "source-semantics/invalid-fsm-state-pattern",
                    message: "FSM arms require a declared state pattern".to_owned(),
                    anchor: SourceSemanticAnchor::for_node(pattern.syntax()),
                });
            }
        };
        let name = node_text(name.syntax())?;
        let state = machine
            .states
            .get(&name)
            .ok_or_else(|| SourceSemanticError {
                code: "source-semantics/unknown-fsm-state",
                message: format!("FSM {} has no state {name}", machine.name),
                anchor: SourceSemanticAnchor::for_node(pattern.syntax()),
            })?;
        if state.fields.len() != items.len() {
            return Err(SourceSemanticError {
                code: "source-semantics/invalid-fsm-state-pattern",
                message: format!(
                    "FSM state {name} requires {} payload patterns, received {}",
                    state.fields.len(),
                    items.len()
                ),
                anchor: SourceSemanticAnchor::for_node(pattern.syntax()),
            });
        }
        let payloads = items
            .iter()
            .zip(&state.fields)
            .map(|(item, field)| {
                self.collection_pattern(
                    item,
                    &SchemaDraft {
                        body: field.clone(),
                        dimension_parameters: Box::new([]),
                    },
                    binding_start,
                    names,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let payload = match payloads.as_slice() {
            [] => None,
            [payload] => Some(Box::new(payload.clone())),
            _ => Some(Box::new(crate::CollectionPattern::Tuple(
                payloads.into_boxed_slice(),
            ))),
        };
        Ok(crate::CollectionPattern::Enum {
            ordinal: state.ordinal,
            payload,
        })
    }

    fn lower_declared_fsm_match(
        &mut self,
        machine: &DeclaredFsm,
        scrutinee: PendingValue,
        syntax: &SyntaxNode,
    ) -> Result<PendingValue, SourceSemanticError> {
        let arms = self.fsm_control_arms(machine)?;
        let mut inputs = vec![scrutinee];
        let mut captures = Vec::new();
        let mut lowered = Vec::new();
        let mut coverage = vec![false; machine.states.len()];
        let mut zero_guard_partitions = BTreeMap::<(u32, usize), u8>::new();
        if self.control_depth == 0 {
            self.next_control_block = 0;
        }
        for arm in arms {
            let saved = self.bindings.clone();
            let saved_scope_definitions = self.scope_definitions.clone();
            let binding_start = self.nodes.len();
            let lowered_arm = (|| -> Result<PendingMatchArm, SourceSemanticError> {
                let mut names = BTreeMap::new();
                let source =
                    self.fsm_state_pattern(machine, &arm.pattern, binding_start, &mut names)?;
                if self.nodes[binding_start..]
                    .iter()
                    .any(|node| !matches!(node.body, PendingNodeBody::CollectionBinding))
                {
                    return Err(SourceSemanticError {
                        code: "source-semantics/invalid-fsm-state-pattern",
                        message: "FSM state patterns cannot execute operations".to_owned(),
                        anchor: SourceSemanticAnchor::for_node(arm.pattern.syntax()),
                    });
                }
                let mut pattern_bindings = BTreeMap::new();
                source.bindings(&mut |local, value| {
                    let PendingValue::Node(index) = value else {
                        unreachable!("FSM pattern bindings are lexical nodes")
                    };
                    pattern_bindings.insert(*index, local);
                });
                let pattern =
                    crate::MatchPattern::Structural(self.resolve_structural_match_pattern(
                        &source,
                        binding_start,
                        false,
                        &mut inputs,
                        &mut captures,
                    )?);
                let guard = if let Some(condition) = &arm.guard {
                    let (block, schema) = self.control_block(
                        condition,
                        &pattern,
                        &pattern_bindings,
                        scrutinee,
                        &mut inputs,
                        &mut captures,
                        None,
                    )?;
                    if schema.body != SchemaBody::Bool {
                        return Err(SourceSemanticError {
                            code: "source-semantics/non-boolean-fsm-guard",
                            message: "FSM guard requires Boolean schema".to_owned(),
                            anchor: SourceSemanticAnchor::for_node(condition.syntax()),
                        });
                    }
                    Some(block)
                } else {
                    None
                };
                let transitions = arm.transitions.clone();
                let (body, schema) = self.control_block_with(
                    &arm.syntax,
                    &pattern,
                    &pattern_bindings,
                    scrutinee,
                    &mut inputs,
                    &mut captures,
                    |builder| {
                        builder.lower_synchronous_fsm_transitions(
                            machine,
                            &transitions,
                            &arm.syntax,
                        )
                    },
                )?;
                if schema != machine.output {
                    return Err(SourceSemanticError {
                        code: "source-semantics/incompatible-fsm-output",
                        message: "FSM transition does not produce its declared output kind"
                            .to_owned(),
                        anchor: SourceSemanticAnchor::for_node(&arm.syntax),
                    });
                }
                Ok(PendingMatchArm {
                    pattern,
                    guard,
                    body,
                })
            })();
            self.bindings = saved;
            self.scope_definitions = saved_scope_definitions;
            self.nodes.truncate(binding_start);
            let lowered_arm = lowered_arm?;
            if let crate::MatchPattern::Structural(crate::CollectionPattern::Enum {
                ordinal,
                payload,
            }) = &lowered_arm.pattern
                && payload.as_deref().is_none_or(fsm_payload_irrefutable)
            {
                if arm.guard.is_none() {
                    coverage[*ordinal as usize] = true;
                } else if let Some(state) = machine
                    .states
                    .values()
                    .find(|state| state.ordinal == *ordinal)
                {
                    let pattern_text = node_text(arm.pattern.syntax())?;
                    let binders = pattern_text
                        .split_once('(')
                        .and_then(|(_, payload)| payload.trim().strip_suffix(')'))
                        .map(|payload| payload.split(',').map(str::trim).collect::<Vec<_>>());
                    if let (Some(binders), Some(guard)) = (binders, arm.guard.as_ref())
                        && binders.len() == state.fields.len()
                    {
                        let text = node_text(guard.syntax())?
                            .chars()
                            .filter(|ch| !ch.is_whitespace())
                            .collect::<String>();
                        for (field_ordinal, (binder, field)) in
                            binders.into_iter().zip(&state.fields).enumerate()
                        {
                            if !matches!(field, SchemaBody::UnsignedInteger(IntegerWidth::W64))
                                || binder.is_empty()
                                || !binder
                                    .chars()
                                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
                            {
                                continue;
                            }
                            let witness = if text == format!("{binder}>0u64") {
                                1
                            } else if text == format!("{binder}==0u64") {
                                2
                            } else {
                                0
                            };
                            *zero_guard_partitions
                                .entry((*ordinal, field_ordinal))
                                .or_default() |= witness;
                        }
                    }
                }
            }
            lowered.push(lowered_arm);
        }
        for ((ordinal, _), partition) in zero_guard_partitions {
            if partition == 3 {
                coverage[ordinal as usize] = true;
            }
        }
        if coverage.iter().any(|covered| !covered) {
            return Err(SourceSemanticError {
                code: "source-semantics/non-exhaustive-fsm",
                message: format!("FSM {} does not handle every declared state", machine.name),
                anchor: SourceSemanticAnchor::for_node(machine.implementation.syntax()),
            });
        }
        let index = self.nodes.len() as u32;
        self.nodes.push(PendingNode {
            body: PendingNodeBody::Match(PendingMatch {
                // Guard predicates can be jointly exhaustive even though the
                // generic enum-pattern validator cannot prove that relation.
                partial: true,
                captures,
                arms: lowered,
            }),
            inferable_projection: false,
            inputs,
            schema: machine.output.clone(),
            exposes_output: true,
            state: None,
            semantic: SourceSemanticNode {
                operation: format!("fsm/{}", machine.name),
                role: "fsm",
                detail: Some(machine.name.clone()),
                anchor: SourceSemanticAnchor::for_node(syntax),
            },
        });
        Ok(PendingValue::Node(index))
    }

    fn lower_synchronous_fsm_transitions(
        &mut self,
        machine: &DeclaredFsm,
        transitions: &[FsmBodyTransitionSyntax],
        syntax: &SyntaxNode,
    ) -> Result<PendingValue, SourceSemanticError> {
        let mut result = None;
        for (transition_index, transition) in transitions.iter().enumerate() {
            match transition {
                FsmBodyTransitionSyntax::Statement(transition) => {
                    if result.is_some() {
                        return Err(SourceSemanticError {
                            code: "source-semantics/unreachable-fsm-transition",
                            message: "FSM state/output transition must terminate its arm"
                                .to_owned(),
                            anchor: SourceSemanticAnchor::for_node(syntax),
                        });
                    }
                    let statement = self.required(
                        transition.statement(),
                        transition.syntax(),
                        "an FSM transition statement",
                    )?;
                    let item = self.required(
                        statement.syntax().children().next(),
                        statement.syntax(),
                        "an FSM transition statement body",
                    )?;
                    self.lower_fsm_code_item(&item)?;
                    continue;
                }
                FsmBodyTransitionSyntax::Block(transition) => {
                    if result.is_some() {
                        return Err(SourceSemanticError {
                            code: "source-semantics/unreachable-fsm-transition",
                            message: "FSM state/output transition must terminate its arm"
                                .to_owned(),
                            anchor: SourceSemanticAnchor::for_node(syntax),
                        });
                    }
                    for item in transition.items() {
                        let value = self.required(
                            item.value(),
                            item.syntax(),
                            "an FSM transition block item",
                        )?;
                        self.lower_fsm_code_item(&value)?;
                    }
                    continue;
                }
                FsmBodyTransitionSyntax::Output(transition)
                    if transitions[transition_index + 1..].iter().any(|later| {
                        matches!(
                            later,
                            FsmBodyTransitionSyntax::State(_)
                                | FsmBodyTransitionSyntax::Async(_)
                                | FsmBodyTransitionSyntax::Output(_)
                        )
                    }) =>
                {
                    if result.is_some() {
                        return Err(SourceSemanticError {
                            code: "source-semantics/unreachable-fsm-transition",
                            message: "FSM state/async transition must terminate its arm".to_owned(),
                            anchor: SourceSemanticAnchor::for_node(syntax),
                        });
                    }
                    let value =
                        self.required(transition.value(), transition.syntax(), "an FSM output")?;
                    let value = self.fsm_output_value(machine, &value)?;
                    self.nodes.push(PendingNode {
                        body: PendingNodeBody::Publish,
                        inferable_projection: false,
                        inputs: vec![value],
                        schema: machine.output.clone(),
                        exposes_output: false,
                        state: None,
                        semantic: SourceSemanticNode {
                            operation: format!("fsm/{}/publish", machine.name),
                            role: "fsm-output",
                            detail: Some(machine.name.clone()),
                            anchor: SourceSemanticAnchor::for_node(transition.syntax()),
                        },
                    });
                    continue;
                }
                _ if result.is_some() => {
                    return Err(SourceSemanticError {
                        code: "source-semantics/unreachable-fsm-transition",
                        message: "FSM state/output transition must terminate its arm".to_owned(),
                        anchor: SourceSemanticAnchor::for_node(syntax),
                    });
                }
                _ => {}
            }
            result = Some(match transition {
                FsmBodyTransitionSyntax::State(transition) => {
                    let value = self.required(
                        transition.value(),
                        transition.syntax(),
                        "an FSM next state",
                    )?;
                    let state = self.fsm_declared_state_value(machine, &value)?;
                    let index = self.nodes.len() as u32;
                    self.nodes.push(PendingNode {
                        body: PendingNodeBody::RecursiveCall(0),
                        inferable_projection: false,
                        inputs: vec![state],
                        schema: machine.output.clone(),
                        exposes_output: true,
                        state: None,
                        semantic: SourceSemanticNode {
                            operation: format!("fsm/{}/transition", machine.name),
                            role: "fsm-transition",
                            detail: Some(machine.name.clone()),
                            anchor: SourceSemanticAnchor::for_node(transition.syntax()),
                        },
                    });
                    PendingValue::Node(index)
                }
                FsmBodyTransitionSyntax::Output(transition) => {
                    let value =
                        self.required(transition.value(), transition.syntax(), "an FSM output")?;
                    self.fsm_output_value(machine, &value)?
                }
                FsmBodyTransitionSyntax::Async(transition) => {
                    let value = self.required(
                        transition.value(),
                        transition.syntax(),
                        "an FSM suspended state",
                    )?;
                    let state = self.fsm_declared_state_value(machine, &value)?;
                    let index = self.nodes.len() as u32;
                    self.nodes.push(PendingNode {
                        body: PendingNodeBody::Suspend,
                        inferable_projection: false,
                        inputs: vec![state],
                        schema: machine.output.clone(),
                        exposes_output: false,
                        state: None,
                        semantic: SourceSemanticNode {
                            operation: format!("fsm/{}/suspend", machine.name),
                            role: "fsm-suspend",
                            detail: Some(machine.name.clone()),
                            anchor: SourceSemanticAnchor::for_node(transition.syntax()),
                        },
                    });
                    PendingValue::Node(index)
                }
                FsmBodyTransitionSyntax::Statement(_) | FsmBodyTransitionSyntax::Block(_) => {
                    unreachable!("sequential FSM code was lowered before terminal selection")
                }
            });
        }
        result.ok_or_else(|| SourceSemanticError {
            code: "source-semantics/missing-fsm-transition-result",
            message: "FSM arm requires a state or output transition".to_owned(),
            anchor: SourceSemanticAnchor::for_node(syntax),
        })
    }

    fn lower_fsm_code_item(
        &mut self,
        item: &SyntaxNode,
    ) -> Result<PendingValue, SourceSemanticError> {
        let value = match item.kind() {
            mech_syntax::document::SyntaxKind::Statement => {
                let child = self.required(
                    item.children().next(),
                    item,
                    "an FSM transition statement body",
                )?;
                return self.lower_fsm_code_item(&child);
            }
            mech_syntax::document::SyntaxKind::VariableDefine => {
                self.definition(
                    &mech_syntax::document::VariableDefineSyntax::cast(item.clone()).unwrap(),
                )?
                .0
            }
            mech_syntax::document::SyntaxKind::Expression => {
                self.expression(
                    &mech_syntax::document::ExpressionSyntax::cast(item.clone()).unwrap(),
                )?
                .0
            }
            mech_syntax::document::SyntaxKind::OpAssign
            | mech_syntax::document::SyntaxKind::VariableAssign => {
                self.document_assignment(item)?.0
            }
            _ => {
                return Err(SourceSemanticError {
                    code: "source-semantics/unsupported-fsm-transition-statement",
                    message: "FSM transition code requires a definition, assignment, or expression"
                        .to_owned(),
                    anchor: SourceSemanticAnchor::for_node(item),
                });
            }
        };
        value.resolved()
    }
}
