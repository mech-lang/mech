//! Document statements lower through the same maintained operations and state
//! slots as expression compilation. Each mutable binding retains one writer.

use mech_syntax::document::{
    CanonicalOpAssign, CodeBlockSyntax, CodeFencePresentation, CodeFenceScope,
    EvalInlineMechCodeSyntax, ExportDeclarationSyntax, OpAssignSyntax, SliceRefSyntax,
    VariableAssignSyntax,
};

use super::*;

#[path = "document_assignment.rs"]
mod document_assignment;

enum DocumentUnit {
    Statement(SyntaxNode),
    Inline(EvalInlineMechCodeSyntax),
    Fence(CodeBlockSyntax, CodeFencePresentation, Vec<DocumentUnit>),
}

struct CompiledDocumentValue {
    value: PendingValue,
    syntax: SyntaxNode,
    program_visible: bool,
}

pub(super) fn compile_document(
    document: &DocumentSyntax,
) -> Result<CanonicalSourceProgram, SourceSemanticError> {
    let anchor = SourceSemanticAnchor::for_node(document.syntax());
    let mut units = Vec::new();
    let mut exports = Vec::new();
    collect_document_units(document.syntax(), &mut units, &mut exports)?;
    compile_collected_document(anchor, units, exports)
}

pub(super) fn compile_named_document_scope(
    document: &DocumentSyntax,
    name: &str,
) -> Result<CanonicalSourceProgram, SourceSemanticError> {
    compile_named_scope(document.syntax(), name)
}

fn compile_named_scope(
    root: &SyntaxNode,
    name: &str,
) -> Result<CanonicalSourceProgram, SourceSemanticError> {
    let anchor = SourceSemanticAnchor::for_node(root);
    let mut units = Vec::new();
    let mut exports = Vec::new();
    let mut pending = vec![root.clone()];
    while let Some(node) = pending.pop() {
        if matches!(
            node.kind(),
            SyntaxKind::MikaSection | SyntaxKind::InlineMechCode
        ) {
            continue;
        }
        if let Some(fence) = CodeBlockSyntax::cast(node.clone()) {
            if !matches!(fence.info().map(|info| info.scope), Some(CodeFenceScope::Named(scope)) if scope == name)
            {
                continue;
            }
            let presentation = fence_presentation(&fence)?;
            let body = fence.mech_code().ok_or_else(|| {
                internal(
                    SourceSemanticAnchor::for_node(fence.syntax()),
                    "executable fence has no canonical Mech body".to_owned(),
                )
            })?;
            let mut body_units = Vec::new();
            collect_document_units(body.syntax(), &mut body_units, &mut exports)?;
            units.push(DocumentUnit::Fence(fence, presentation, body_units));
            continue;
        }
        let children: Vec<_> = node.children().collect();
        pending.extend(children.into_iter().rev());
    }
    compile_collected_document(anchor, units, exports)
}

pub(super) fn compile_mika_section(
    section: &mech_syntax::document::MikaSectionSyntax,
    name: Option<&str>,
) -> Result<CanonicalSourceProgram, SourceSemanticError> {
    let anchor = SourceSemanticAnchor::for_node(section.syntax());
    let body = section
        .body()
        .ok_or_else(|| internal(anchor, "Mika section has no retained body".to_owned()))?;
    if let Some(name) = name {
        return compile_named_scope(body.syntax(), name);
    }
    let mut units = Vec::new();
    let mut exports = Vec::new();
    collect_document_units(body.syntax(), &mut units, &mut exports)?;
    compile_collected_document(anchor, units, exports)
}

fn compile_collected_document(
    anchor: SourceSemanticAnchor,
    units: Vec<DocumentUnit>,
    exports: Vec<ExportDeclarationSyntax>,
) -> Result<CanonicalSourceProgram, SourceSemanticError> {
    let mut builder = SemanticBuilder::new(anchor);
    let mut bindings = BTreeSet::new();
    declare_document_inputs(&mut builder, &units, &mut bindings)?;
    declare_document_inline_inputs(&mut builder, &units, &bindings)?;
    let mut presentation = Vec::new();
    let last = compile_document_units(&mut builder, units, &bindings, &mut presentation)?;
    let Some(last) = last else {
        return Err(SourceSemanticError {
            code: "source-semantics/empty-document",
            message: "canonical document contains no executable source unit".to_owned(),
            anchor,
        });
    };
    builder.publish("result", None, last.value, &last.syntax);
    let mut output_bindings = vec![SourceDocumentOutput {
        output: 0,
        kind: SourceDocumentOutputKind::Program,
        visible: last.program_visible,
    }];
    let mut document_exports = Vec::new();
    let mut exported_names = BTreeSet::new();
    for export in exports {
        let name_node = builder.required(export.name(), export.syntax(), "an exported name")?;
        let name = node_text(name_node.syntax())?;
        if !exported_names.insert(name.clone()) {
            return Err(SourceSemanticError {
                code: "source-semantics/duplicate-export",
                message: format!("document exports {name} more than once"),
                anchor: SourceSemanticAnchor::for_node(export.syntax()),
            });
        }
        let binding = builder
            .bindings
            .get(&name)
            .copied()
            .ok_or_else(|| SourceSemanticError {
                code: "source-semantics/unknown-export",
                message: format!("document exports undefined binding {name}"),
                anchor: SourceSemanticAnchor::for_node(export.syntax()),
            })?;
        let value = builder.read_document_binding(binding, export.syntax())?;
        let output = u32::try_from(builder.outputs.len()).map_err(|_| SourceSemanticError {
            code: "source-semantics/output-identity-exhausted",
            message: "canonical output count exceeds SourceProgram identity space".to_owned(),
            anchor: SourceSemanticAnchor::for_node(export.syntax()),
        })?;
        builder.publish(&name, None, value, name_node.syntax());
        document_exports.push(SourceDocumentExport { output, name });
    }
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
            visible: true,
        });
        builder.publish(&name, None, value, &owner);
    }
    builder.order_document_state_writers();
    let mut program = builder.finish()?;
    program.document_outputs = output_bindings.into_boxed_slice();
    program.document_exports = document_exports.into_boxed_slice();
    Ok(program)
}

fn collect_document_units(
    node: &SyntaxNode,
    output: &mut Vec<DocumentUnit>,
    exports: &mut Vec<ExportDeclarationSyntax>,
) -> Result<(), SourceSemanticError> {
    if matches!(
        node.kind(),
        SyntaxKind::InlineMechCode | SyntaxKind::MikaSection
    ) {
        return Ok(());
    }
    if let Some(expression) = EvalInlineMechCodeSyntax::cast(node.clone()) {
        output.push(DocumentUnit::Inline(expression));
        return Ok(());
    }
    if let Some(fence) = CodeBlockSyntax::cast(node.clone()) {
        let Some(info) = fence.info() else {
            return Ok(());
        };
        if !matches!(info.scope, CodeFenceScope::Root) {
            return Ok(());
        }
        let presentation = fence_presentation(&fence)?;
        let Some(body) = fence.mech_code() else {
            return Err(internal(
                SourceSemanticAnchor::for_node(fence.syntax()),
                "executable fence has no canonical Mech body".to_owned(),
            ));
        };
        let mut units = Vec::new();
        collect_document_units(body.syntax(), &mut units, exports)?;
        output.push(DocumentUnit::Fence(fence, presentation, units));
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
    if let Some(export) = ExportDeclarationSyntax::cast(node.clone()) {
        exports.push(export);
        return Ok(());
    }
    // Resolver-owned declarations participate through the canonical source
    // index and runtime handoff; they do not emit engine operations themselves.
    if matches!(
        node.kind(),
        SyntaxKind::ContextDeclaration | SyntaxKind::ImportDeclaration | SyntaxKind::ModuleImport
    ) {
        return Ok(());
    }
    if matches!(
        node.kind(),
        SyntaxKind::ActivationScope
            | SyntaxKind::ContextSend
            | SyntaxKind::EnumDefine
            | SyntaxKind::Fsm
            | SyntaxKind::FsmDeclare
            | SyntaxKind::FsmImplementation
            // An expression owns a pipe's semantics. A bare pipe in a document
            // must not be traversed as unrelated child expressions.
            | SyntaxKind::FsmPipe
            | SyntaxKind::FsmSpecification
            | SyntaxKind::FunctionDefine
            | SyntaxKind::InvariantDefine
            | SyntaxKind::KindDefine
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
        collect_document_units(&child, output, exports)?;
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
            DocumentUnit::Inline(_) => {}
            DocumentUnit::Fence(_, _, units) => declare_document_inputs(builder, units, bindings)?,
        }
    }
    Ok(())
}

fn declare_document_inline_inputs(
    builder: &mut SemanticBuilder,
    units: &[DocumentUnit],
    bindings: &BTreeSet<String>,
) -> Result<(), SourceSemanticError> {
    for unit in units {
        match unit {
            DocumentUnit::Statement(_) => {}
            DocumentUnit::Inline(inline) => {
                builder.declare_input_annotations(inline.syntax(), bindings)?
            }
            DocumentUnit::Fence(_, _, units) => {
                declare_document_inline_inputs(builder, units, bindings)?
            }
        }
    }
    Ok(())
}

fn compile_document_units(
    builder: &mut SemanticBuilder,
    units: Vec<DocumentUnit>,
    local_bindings: &BTreeSet<String>,
    presentation: &mut Vec<(SourceDocumentOutputKind, PendingValue, SyntaxNode)>,
) -> Result<Option<CompiledDocumentValue>, SourceSemanticError> {
    let mut last = None;
    let mut deferred_inline = Vec::new();
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
                retain_later_document_value(
                    &mut last,
                    CompiledDocumentValue {
                        value: result.0,
                        syntax: result.1,
                        program_visible: true,
                    },
                );
                flush_deferred_inline(
                    builder,
                    local_bindings,
                    &mut deferred_inline,
                    presentation,
                    &mut last,
                )?;
            }
            DocumentUnit::Inline(inline) => {
                if inline_reads_unbound_local(builder, &inline, local_bindings)? {
                    deferred_inline.push(inline);
                } else {
                    retain_later_document_value(
                        &mut last,
                        compile_inline(builder, inline, presentation)?,
                    );
                }
            }
            DocumentUnit::Fence(fence, fence_presentation, units) => {
                if let Some(compiled) =
                    compile_document_units(builder, units, local_bindings, presentation)?
                {
                    let value = builder.read_document_binding(
                        PendingBinding::Value(compiled.value),
                        &compiled.syntax,
                    )?;
                    if fence_presentation.show_output {
                        presentation.push((
                            SourceDocumentOutputKind::Fence,
                            value,
                            fence.syntax().clone(),
                        ));
                    }
                    retain_later_document_value(
                        &mut last,
                        CompiledDocumentValue {
                            value,
                            syntax: compiled.syntax,
                            program_visible: false,
                        },
                    );
                }
                flush_deferred_inline(
                    builder,
                    local_bindings,
                    &mut deferred_inline,
                    presentation,
                    &mut last,
                )?;
            }
        }
    }
    for inline in deferred_inline {
        retain_later_document_value(&mut last, compile_inline(builder, inline, presentation)?);
    }
    Ok(last)
}

fn flush_deferred_inline(
    builder: &mut SemanticBuilder,
    local_bindings: &BTreeSet<String>,
    deferred: &mut Vec<EvalInlineMechCodeSyntax>,
    presentation: &mut Vec<(SourceDocumentOutputKind, PendingValue, SyntaxNode)>,
    last: &mut Option<CompiledDocumentValue>,
) -> Result<(), SourceSemanticError> {
    loop {
        let mut ready = None;
        for (index, inline) in deferred.iter().enumerate() {
            if !inline_reads_unbound_local(builder, inline, local_bindings)? {
                ready = Some(index);
                break;
            }
        }
        let Some(ready) = ready else { break };
        let inline = deferred.remove(ready);
        retain_later_document_value(last, compile_inline(builder, inline, presentation)?);
    }
    Ok(())
}

fn retain_later_document_value(
    last: &mut Option<CompiledDocumentValue>,
    candidate: CompiledDocumentValue,
) {
    if last
        .as_ref()
        .is_none_or(|current| current.syntax.range().start < candidate.syntax.range().start)
    {
        *last = Some(candidate);
    }
}

fn compile_inline(
    builder: &mut SemanticBuilder,
    inline: EvalInlineMechCodeSyntax,
    presentation: &mut Vec<(SourceDocumentOutputKind, PendingValue, SyntaxNode)>,
) -> Result<CompiledDocumentValue, SourceSemanticError> {
    let expression = builder.required(
        inline.expression(),
        inline.syntax(),
        "an evaluated inline expression",
    )?;
    let value = builder.expression(&expression)?.0;
    presentation.push((
        SourceDocumentOutputKind::Inline,
        value,
        inline.syntax().clone(),
    ));
    Ok(CompiledDocumentValue {
        value,
        syntax: inline.syntax().clone(),
        program_visible: false,
    })
}

fn inline_reads_unbound_local(
    builder: &SemanticBuilder,
    inline: &EvalInlineMechCodeSyntax,
    local_bindings: &BTreeSet<String>,
) -> Result<bool, SourceSemanticError> {
    let mut pending = vec![inline.syntax().clone()];
    while let Some(node) = pending.pop() {
        if let Some(variable) = VariableSyntax::cast(node.clone()) {
            let stem = builder.required(variable.stem(), variable.syntax(), "a variable stem")?;
            let name = node_text(stem.syntax())?;
            if local_bindings.contains(&name) && !builder.bindings.contains_key(&name) {
                return Ok(true);
            }
            continue;
        }
        pending.extend(node.children());
    }
    Ok(false)
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
        if let Some(subscripts) = target.subscripts() {
            value = self.document_selected_update(
                self.current_state_value(state),
                &subscripts.items(),
                value,
                operation,
                syntax,
                expression.syntax(),
            )?;
            let writer = self.states[state as usize].producer_node as usize;
            self.nodes[writer].inputs[0] = value;
            return Ok((value, syntax.clone()));
        }
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

fn fence_presentation(
    fence: &CodeBlockSyntax,
) -> Result<CodeFencePresentation, SourceSemanticError> {
    fence.presentation().ok_or_else(|| SourceSemanticError {
        code: "source-semantics/invalid-fence-options",
        message: "fence presentation settings require complete typed option values".to_owned(),
        anchor: SourceSemanticAnchor::for_node(fence.syntax()),
    })
}
