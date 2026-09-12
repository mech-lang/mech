//! Document statements lower through the same maintained operations and state
//! slots as expression compilation. Each mutable binding retains one writer.

use mech_syntax::document::{
    CanonicalOpAssign, CodeBlockSyntax, CodeFenceScope, EvalInlineMechCodeSyntax, OpAssignSyntax,
    SliceRefSyntax, VariableAssignSyntax,
};

use super::*;

enum DocumentUnit {
    Statement(SyntaxNode),
    Fence(CodeBlockSyntax, Vec<DocumentUnit>),
}

pub(super) fn compile_document(
    document: &DocumentSyntax,
) -> Result<CanonicalSourceProgram, SourceSemanticError> {
    let anchor = SourceSemanticAnchor::for_node(document.syntax());
    let mut units = Vec::new();
    let mut inline = Vec::new();
    collect_document_units(document.syntax(), &mut units, &mut inline)?;
    let mut builder = SemanticBuilder::new(anchor);
    let mut bindings = BTreeSet::new();
    declare_document_inputs(&mut builder, &units, &mut bindings)?;
    for expression in &inline {
        builder.declare_input_annotations(expression.syntax(), &bindings)?;
    }
    let mut presentation = Vec::new();
    let last = compile_document_units(&mut builder, units, &mut presentation)?;
    let mut last_inline = None;
    for inline in inline {
        let expression = builder.required(
            inline.expression(),
            inline.syntax(),
            "an evaluated inline expression",
        )?;
        let value = builder.expression(&expression)?.0;
        last_inline = Some((value, inline.syntax().clone()));
        presentation.push((
            SourceDocumentOutputKind::Inline,
            value,
            inline.syntax().clone(),
        ));
    }
    let Some((value, syntax)) = last.or(last_inline) else {
        return Err(SourceSemanticError {
            code: "source-semantics/empty-document",
            message: "canonical document contains no executable source unit".to_owned(),
            anchor,
        });
    };
    builder.publish("result", None, value, &syntax);
    let mut output_bindings = vec![SourceDocumentOutput {
        output: 0,
        kind: SourceDocumentOutputKind::Program,
    }];
    presentation.sort_by_key(|(_, _, owner)| owner.range().start);
    for (kind, value, owner) in presentation {
        let role = match kind {
            SourceDocumentOutputKind::Inline => "inline",
            SourceDocumentOutputKind::Fence => "fence",
            SourceDocumentOutputKind::Program => {
                unreachable!("program output is published separately")
            }
        };
        let name = format!("document:{role}:{}", owner.range().start.0);
        output_bindings.push(SourceDocumentOutput {
            output: builder.outputs.len() as u32,
            kind,
        });
        builder.publish(&name, None, value, &owner);
    }
    builder.order_document_state_writers();
    let mut program = builder.finish()?;
    program.document_outputs = output_bindings.into_boxed_slice();
    Ok(program)
}

fn collect_document_units(
    node: &SyntaxNode,
    output: &mut Vec<DocumentUnit>,
    inline: &mut Vec<EvalInlineMechCodeSyntax>,
) -> Result<(), SourceSemanticError> {
    if matches!(
        node.kind(),
        SyntaxKind::InlineMechCode | SyntaxKind::MikaSection
    ) {
        return Ok(());
    }
    if let Some(expression) = EvalInlineMechCodeSyntax::cast(node.clone()) {
        inline.push(expression);
        return Ok(());
    }
    if let Some(fence) = CodeBlockSyntax::cast(node.clone()) {
        let Some(info) = fence.info() else {
            return Ok(());
        };
        if let CodeFenceScope::UnsupportedInfo(info) = &info.scope {
            return Err(SourceSemanticError {
                code: "source-semantics/unsupported-fence-info",
                message: format!(
                    "Mech fence information {info:?} does not select a documented execution scope"
                ),
                anchor: SourceSemanticAnchor {
                    document: fence.syntax().source().document(),
                    revision: fence.syntax().source().revision(),
                    range: fence
                        .info_range()
                        .expect("classified fence has an information range"),
                },
            });
        }
        if !matches!(info.scope, CodeFenceScope::Root) {
            return Ok(());
        }
        if let Some(options) = fence.options() {
            return Err(SourceSemanticError {
                code: "source-semantics/unsupported-fence-options",
                message: "configured fence options need a typed document consumer".to_owned(),
                anchor: SourceSemanticAnchor::for_node(options.syntax()),
            });
        }
        let Some(body) = fence.mech_code() else {
            return Err(internal(
                SourceSemanticAnchor::for_node(fence.syntax()),
                "executable fence has no canonical Mech body".to_owned(),
            ));
        };
        let mut units = Vec::new();
        collect_document_units(body.syntax(), &mut units, inline)?;
        output.push(DocumentUnit::Fence(fence, units));
        return Ok(());
    }
    if matches!(
        node.kind(),
        SyntaxKind::VariableDefine
            | SyntaxKind::Expression
            | SyntaxKind::OpAssign
            | SyntaxKind::VariableAssign
    ) {
        output.push(DocumentUnit::Statement(node.clone()));
        return Ok(());
    }
    if matches!(
        node.kind(),
        SyntaxKind::ActivationScope
            | SyntaxKind::ContextDeclaration
            | SyntaxKind::ContextSend
            | SyntaxKind::EnumDefine
            | SyntaxKind::ExportDeclaration
            | SyntaxKind::Fsm
            | SyntaxKind::FsmDeclare
            | SyntaxKind::FsmImplementation
            | SyntaxKind::FsmSpecification
            | SyntaxKind::FunctionDefine
            | SyntaxKind::InvariantDefine
            | SyntaxKind::ImportDeclaration
            | SyntaxKind::KindDefine
            | SyntaxKind::ModuleImport
            | SyntaxKind::TupleDestructure
    ) {
        return Err(SourceSemanticError {
            code: "source-semantics/unsupported-document-unit",
            message: format!(
                "canonical document unit {:?} has no engine semantic implementation",
                node.kind()
            ),
            anchor: SourceSemanticAnchor::for_node(node),
        });
    }
    for child in node.children() {
        collect_document_units(&child, output, inline)?;
    }
    Ok(())
}

fn declare_document_inputs(
    builder: &mut SemanticBuilder,
    units: &[DocumentUnit],
    bindings: &mut BTreeSet<String>,
) -> Result<(), SourceSemanticError> {
    for unit in units {
        match unit {
            DocumentUnit::Statement(unit) => {
                builder.declare_unit_input_annotations(unit, bindings)?
            }
            DocumentUnit::Fence(_, units) => declare_document_inputs(builder, units, bindings)?,
        }
    }
    Ok(())
}

fn compile_document_units(
    builder: &mut SemanticBuilder,
    units: Vec<DocumentUnit>,
    presentation: &mut Vec<(SourceDocumentOutputKind, PendingValue, SyntaxNode)>,
) -> Result<Option<(PendingValue, SyntaxNode)>, SourceSemanticError> {
    let mut last = None;
    for unit in units {
        match unit {
            DocumentUnit::Statement(unit) => {
                let result = match unit.kind() {
                    SyntaxKind::VariableDefine => {
                        builder.definition(&VariableDefineSyntax::cast(unit).unwrap())?
                    }
                    SyntaxKind::Expression => {
                        builder.expression(&ExpressionSyntax::cast(unit).unwrap())?
                    }
                    SyntaxKind::OpAssign | SyntaxKind::VariableAssign => {
                        builder.document_assignment(&unit)?
                    }
                    _ => unreachable!("document statement selection is closed"),
                };
                result.0.resolved()?;
                last = Some(result);
            }
            DocumentUnit::Fence(fence, units) => {
                if let Some((value, syntax)) = compile_document_units(builder, units, presentation)?
                {
                    let value =
                        builder.read_document_binding(PendingBinding::Value(value), &syntax)?;
                    presentation.push((
                        SourceDocumentOutputKind::Fence,
                        value,
                        fence.syntax().clone(),
                    ));
                    last = Some((value, syntax));
                }
            }
        }
    }
    Ok(last)
}

impl SemanticBuilder {
    /// A state reference in a statement reads the latest candidate produced by
    /// preceding statements. The writer's original self input denotes the
    /// committed value from the previous turn.
    pub(super) fn read_document_binding(
        &mut self,
        binding: PendingBinding,
        syntax: &SyntaxNode,
    ) -> Result<PendingValue, SourceSemanticError> {
        let value = match binding {
            PendingBinding::Value(value) => value,
            PendingBinding::MutableState(state) => self.current_state_value(state),
        };
        if matches!(value, PendingValue::State(_)) {
            // A derived read preserves the value's source-order version even
            // when it is exported after the final state writer publishes.
            Ok(self.emit_with_schema_draft(
                "core/assign",
                vec![value],
                self.schema_draft_of(value)?,
                syntax,
                "state-read",
                None,
            ))
        } else {
            Ok(value)
        }
    }

    /// Artifact state reads before their writer refer to the previous turn.
    /// Derived candidates already encode statement order, so publication can
    /// occur after all candidates without introducing another state writer.
    pub(super) fn order_document_state_writers(&mut self) {
        let mut order = (0..self.nodes.len()).collect::<Vec<_>>();
        order.sort_by_key(|index| self.nodes[*index].state.is_some());
        let mut indices = vec![0; order.len()];
        for (new, old) in order.iter().enumerate() {
            indices[*old] = new as u32;
        }
        let remap = |value: &mut PendingValue| {
            if let PendingValue::Node(node) = value {
                *node = indices[*node as usize];
            }
        };
        let mut nodes = core::mem::take(&mut self.nodes)
            .into_iter()
            .map(Some)
            .collect::<Vec<_>>();
        self.nodes = order
            .into_iter()
            .map(|old| {
                let mut node = nodes[old].take().unwrap();
                for input in &mut node.inputs {
                    remap(input);
                }
                node
            })
            .collect();
        for state in &mut self.states {
            state.producer_node = indices[state.producer_node as usize];
            remap(&mut state.initializer);
        }
        for output in &mut self.outputs {
            remap(&mut output.source);
        }
        for binding in self.bindings.values_mut() {
            if let PendingBinding::Value(value) = binding {
                remap(value);
            }
        }
        for arm in &mut self.match_arms {
            arm.node = indices[arm.node as usize];
        }
        for qualifier in &mut self.comprehension_qualifiers {
            qualifier.node = indices[qualifier.node as usize];
        }
    }

    fn current_state_value(&self, state: u32) -> PendingValue {
        let writer = self.states[state as usize].producer_node as usize;
        self.nodes[writer].inputs[0]
    }

    pub(super) fn document_assignment(
        &mut self,
        syntax: &SyntaxNode,
    ) -> Result<(PendingValue, SyntaxNode), SourceSemanticError> {
        let (target, expression, operation) = match syntax.kind() {
            SyntaxKind::OpAssign => {
                let assignment = OpAssignSyntax::cast(syntax.clone()).unwrap();
                let operator =
                    self.required(assignment.operator(), syntax, "an assignment operator")?;
                let selected = self.required(
                    operator.selected(),
                    operator.syntax(),
                    "an assignment primitive",
                )?;
                let operation = match self.required(
                    selected.semantic(),
                    selected.syntax(),
                    "assignment semantics",
                )? {
                    CanonicalOpAssign::Add => "math/add",
                    CanonicalOpAssign::Sub => "math/sub",
                    CanonicalOpAssign::Mul => "math/mul",
                    CanonicalOpAssign::Div => "math/div",
                    CanonicalOpAssign::Exp => "math/pow",
                };
                (
                    self.required(assignment.target(), syntax, "an assignment target")?,
                    self.required(assignment.value(), syntax, "an assignment value")?,
                    Some(operation),
                )
            }
            SyntaxKind::VariableAssign => {
                let assignment = VariableAssignSyntax::cast(syntax.clone()).unwrap();
                (
                    self.required(assignment.target(), syntax, "an assignment target")?,
                    self.required(assignment.value(), syntax, "an assignment value")?,
                    None,
                )
            }
            _ => unreachable!("the document collector selects assignment statements"),
        };
        let state = self.assignment_state(&target)?;
        let expected = self.schema_draft_of(PendingValue::State(state))?;
        let mut value = self.expression(&expression)?.0;
        if let Some(operation) = operation {
            let current = self.current_state_value(state);
            let Some((inputs, schema)) =
                self.resolve_maintained_call(operation, vec![current, value], syntax)?
            else {
                return Err(internal(
                    SourceSemanticAnchor::for_node(syntax),
                    format!("assignment operation {operation} has no maintained type declaration"),
                ));
            };
            value = self.emit_with_schema_draft(
                operation,
                inputs,
                schema,
                syntax,
                "state-update",
                None,
            );
        }
        value = self.conform_schema_draft(
            value,
            &expected,
            expression.syntax(),
            "source-semantics/incompatible-assignment-kind",
            "assignment value does not satisfy the mutable binding's schema",
        )?;
        let writer = self.states[state as usize].producer_node as usize;
        self.nodes[writer].inputs[0] = value;
        Ok((value, syntax.clone()))
    }

    fn assignment_state(&self, target: &SliceRefSyntax) -> Result<u32, SourceSemanticError> {
        if let Some(subscripts) = target.subscripts() {
            return Err(SourceSemanticError {
                code: "source-semantics/unsupported-assignment-target",
                message: "indexed assignment requires a maintained update operation".to_owned(),
                anchor: SourceSemanticAnchor::for_node(subscripts.syntax()),
            });
        }
        let stem = self.required(target.stem(), target.syntax(), "an assignment target stem")?;
        let name = node_text(stem.syntax())?;
        match self.bindings.get(&name) {
            Some(PendingBinding::MutableState(state)) => Ok(*state),
            Some(_) => Err(SourceSemanticError {
                code: "source-semantics/immutable-assignment-target",
                message: format!("assignment target {name} is immutable"),
                anchor: SourceSemanticAnchor::for_node(stem.syntax()),
            }),
            None => Err(SourceSemanticError {
                code: "source-semantics/unknown-assignment-target",
                message: format!("assignment target {name} has no preceding mutable definition"),
                anchor: SourceSemanticAnchor::for_node(stem.syntax()),
            }),
        }
    }
}
