//! Document statements lower through the same maintained operations and state
//! slots as expression compilation. Each mutable binding retains one writer.

use mech_syntax::document::{
    CanonicalOpAssign, CodeBlockSyntax, CodeFencePresentation, CodeFenceScope,
    EvalInlineMechCodeSyntax, ExportDeclarationSyntax, InvariantDefineSyntax, OpAssignSyntax,
    SliceRefSyntax, VariableAssignSyntax,
};

use super::*;

#[path = "document_assignment.rs"]
mod document_assignment;

#[path = "document_functions.rs"]
mod document_functions;
#[path = "document_imports.rs"]
mod document_imports;

enum DocumentUnit {
    Import(mech_syntax::document::ModuleImportSyntax),
    Function(SyntaxNode),
    Statement(SyntaxNode),
    ResourceSend(mech_syntax::document::ContextSendSyntax),
    Invariant(InvariantDefineSyntax),
    Inline(EvalInlineMechCodeSyntax),
    Fence(CodeBlockSyntax, CodeFencePresentation, Vec<DocumentUnit>),
}

struct CompiledDocumentValue {
    value: PendingValue,
    syntax: SyntaxNode,
    program_visible: bool,
}

struct DeferredInline {
    inline: EvalInlineMechCodeSyntax,
    captured: BTreeMap<String, PendingValue>,
    waiting: BTreeSet<String>,
}

pub(super) fn root_statement_nodes(
    document: &DocumentSyntax,
) -> Result<Vec<SyntaxNode>, SourceSemanticError> {
    let mut units = Vec::new();
    collect_document_units(document.syntax(), &mut units, &mut Vec::new())?;
    let mut statements = Vec::new();
    while let Some(unit) = units.pop() {
        match unit {
            DocumentUnit::Fence(_, _, children) => units.extend(children),
            DocumentUnit::Statement(node) => statements.push(node),
            _ => {}
        }
    }
    statements.sort_by_key(|node| node.range().start);
    Ok(statements)
}

pub(super) fn root_state_mutation_names(
    document: &DocumentSyntax,
) -> Result<BTreeSet<String>, SourceSemanticError> {
    let mut names = BTreeSet::new();
    for node in root_statement_nodes(document)? {
        let target = VariableAssignSyntax::cast(node.clone())
            .and_then(|assignment| assignment.target())
            .or_else(|| OpAssignSyntax::cast(node).and_then(|assignment| assignment.target()));
        if let Some(stem) = target.and_then(|target| target.stem()) {
            names.insert(node_text(stem.syntax())?);
        }
    }
    Ok(names)
}

pub(super) fn compile_document(
    document: &DocumentSyntax,
) -> Result<CanonicalSourceProgram, SourceSemanticError> {
    compile_document_with_options(
        document,
        None,
        BTreeMap::new(),
        false,
        BTreeMap::new(),
        &BTreeSet::new(),
        &BTreeSet::new(),
        &BTreeSet::new(),
    )
}

pub(super) fn compile_document_with_catalog(
    document: &DocumentSyntax,
    catalog: Arc<mech_core::FunctionCatalog>,
) -> Result<CanonicalSourceProgram, SourceSemanticError> {
    compile_document_with_options(
        document,
        Some(catalog),
        BTreeMap::new(),
        false,
        BTreeMap::new(),
        &BTreeSet::new(),
        &BTreeSet::new(),
        &BTreeSet::new(),
    )
}

pub(super) fn compile_interactive_document_with_catalog(
    document: &DocumentSyntax,
    catalog: Arc<mech_core::FunctionCatalog>,
) -> Result<CanonicalSourceProgram, SourceSemanticError> {
    compile_document_with_options(
        document,
        Some(catalog),
        BTreeMap::new(),
        true,
        BTreeMap::new(),
        &BTreeSet::new(),
        &BTreeSet::new(),
        &BTreeSet::new(),
    )
}

pub(super) fn compile_document_with_catalog_and_input_schemas(
    document: &DocumentSyntax,
    catalog: Arc<mech_core::FunctionCatalog>,
    input_schemas: BTreeMap<String, SchemaBody>,
) -> Result<CanonicalSourceProgram, SourceSemanticError> {
    compile_document_with_options(
        document,
        Some(catalog),
        input_schemas,
        false,
        BTreeMap::new(),
        &BTreeSet::new(),
        &BTreeSet::new(),
        &BTreeSet::new(),
    )
}

pub(super) fn compile_document_with_catalog_and_resources(
    document: &DocumentSyntax,
    catalog: Arc<mech_core::FunctionCatalog>,
    input_schemas: BTreeMap<String, SchemaBody>,
    resource_writes: BTreeMap<String, mech_core::ExecutionResourceRequest>,
    interactive: bool,
) -> Result<CanonicalSourceProgram, SourceSemanticError> {
    compile_document_with_options(
        document,
        Some(catalog),
        input_schemas,
        interactive,
        resource_writes,
        &BTreeSet::new(),
        &BTreeSet::new(),
        &BTreeSet::new(),
    )
}

/// Retained coordinator units awaiting schemas from the compiled compute interface.
/// The units and source anchors belong to the same snapshot as the compute program.
/// Completing this plan does not parse, repartition, or execute the source again.
pub struct CanonicalCoordinatorPlan {
    owner: DocumentScopeId,
    anchor: SourceSemanticAnchor,
    units: Vec<DocumentUnit>,
    exports: Vec<ExportDeclarationSyntax>,
    catalog: Arc<mech_core::FunctionCatalog>,
    input_schemas: BTreeMap<String, SchemaBody>,
    resource_writes: BTreeMap<String, mech_core::ExecutionResourceRequest>,
    resolved_source_modules: BTreeSet<String>,
}

impl CanonicalCoordinatorPlan {
    /// Complete coordinator lowering after the caller plans interface-dependent reads.
    pub fn compile(
        mut self,
        additional_input_schemas: BTreeMap<String, SchemaBody>,
    ) -> Result<CanonicalSourceProgram, SourceSemanticError> {
        self.input_schemas.extend(additional_input_schemas);
        compile_collected_document(
            self.owner,
            self.anchor,
            self.units,
            self.exports,
            Some(self.catalog),
            self.input_schemas,
            true,
            self.resource_writes,
            &BTreeSet::new(),
            &BTreeSet::new(),
            &self.resolved_source_modules,
        )
    }
}

pub(super) fn prepare_mixed_document_with_catalog_and_resources(
    document: &DocumentSyntax,
    catalog: Arc<mech_core::FunctionCatalog>,
    input_schemas: BTreeMap<String, SchemaBody>,
    resource_writes: BTreeMap<String, mech_core::ExecutionResourceRequest>,
    external_inputs: &BTreeSet<String>,
    retained_outputs: &BTreeSet<String>,
    resolved_source_modules: &BTreeSet<String>,
) -> Result<CanonicalMixedSourcePreparation, SourceSemanticError> {
    let anchor = SourceSemanticAnchor::for_node(document.syntax());
    let sections = document
        .body()
        .map(|body| body.sections())
        .unwrap_or_default();
    let mut regions = Vec::new();
    for (index, section) in sections.iter().enumerate() {
        if let Some((name, placement)) = mixed_section_identity(section)? {
            regions.push((index, name, placement));
        }
    }
    if regions.len() != 1 {
        return Err(SourceSemanticError {
            code: "source-semantics/mixed-region-count",
            message: format!(
                "v0.4 mixed programs require exactly one executable compute region, found {}",
                regions.len(),
            ),
            anchor,
        });
    }
    let (region_index, region_name, placement) = regions.pop().expect("one region checked");

    let mut coordinator_units = Vec::new();
    let mut coordinator_exports = Vec::new();
    if let Some(title) = document.title() {
        collect_document_units(
            title.syntax(),
            &mut coordinator_units,
            &mut coordinator_exports,
        )?;
    }
    for (index, section) in sections.iter().enumerate() {
        if index != region_index {
            collect_document_units(
                section.syntax(),
                &mut coordinator_units,
                &mut coordinator_exports,
            )?;
        }
    }
    let root_imports = coordinator_units
        .iter()
        .filter_map(|unit| match unit {
            DocumentUnit::Import(import) => Some(import.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let coordinator = CanonicalCoordinatorPlan {
        owner: document.scope_id(),
        anchor,
        units: coordinator_units,
        exports: coordinator_exports,
        catalog: Arc::clone(&catalog),
        input_schemas: input_schemas.clone(),
        resource_writes,
        resolved_source_modules: resolved_source_modules.clone(),
    };

    let region = &sections[region_index];
    let mut compute_units = root_imports
        .iter()
        .cloned()
        .map(DocumentUnit::Import)
        .collect();
    let mut compute_exports = Vec::new();
    collect_document_units(region.syntax(), &mut compute_units, &mut compute_exports)?;
    let compute = compile_collected_document(
        document.scope_id(),
        anchor,
        compute_units,
        compute_exports,
        Some(Arc::clone(&catalog)),
        input_schemas.clone(),
        false,
        BTreeMap::new(),
        external_inputs,
        retained_outputs,
        resolved_source_modules,
    )?
    .with_compute_region(region_name.clone(), placement)?;

    let mut initializer_units = root_imports.into_iter().map(DocumentUnit::Import).collect();
    let mut initializer_exports = Vec::new();
    collect_document_units(
        region.syntax(),
        &mut initializer_units,
        &mut initializer_exports,
    )?;
    let compute_initializers = compile_collected_document(
        document.scope_id(),
        anchor,
        initializer_units,
        initializer_exports,
        Some(catalog),
        input_schemas,
        false,
        BTreeMap::new(),
        &BTreeSet::new(),
        external_inputs,
        resolved_source_modules,
    )?
    .retain_static_outputs(external_inputs)?;

    Ok(CanonicalMixedSourcePreparation {
        region_name,
        placement,
        coordinator,
        compute,
        compute_initializers,
    })
}

pub(super) fn compile_document_with_options(
    document: &DocumentSyntax,
    catalog: Option<Arc<mech_core::FunctionCatalog>>,
    input_schemas: BTreeMap<String, SchemaBody>,
    interactive: bool,
    resource_writes: BTreeMap<String, mech_core::ExecutionResourceRequest>,
    external_definitions: &BTreeSet<String>,
    published_bindings: &BTreeSet<String>,
    resolved_source_modules: &BTreeSet<String>,
) -> Result<CanonicalSourceProgram, SourceSemanticError> {
    let anchor = SourceSemanticAnchor::for_node(document.syntax());
    let mut units = Vec::new();
    let mut exports = Vec::new();
    collect_document_units(document.syntax(), &mut units, &mut exports)?;
    compile_collected_document(
        document.scope_id(),
        anchor,
        units,
        exports,
        catalog,
        input_schemas,
        interactive,
        resource_writes,
        external_definitions,
        published_bindings,
        resolved_source_modules,
    )
}

pub(super) fn compile_named_document_scope(
    document: &DocumentSyntax,
    name: &str,
) -> Result<CanonicalSourceProgram, SourceSemanticError> {
    compile_named_scope(document.syntax(), document.scope_id(), name)
}

fn compile_named_scope(
    root: &SyntaxNode,
    owner: DocumentScopeId,
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
    compile_collected_document(
        owner,
        anchor,
        units,
        exports,
        None,
        BTreeMap::new(),
        false,
        BTreeMap::new(),
        &BTreeSet::new(),
        &BTreeSet::new(),
        &BTreeSet::new(),
    )
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
        return compile_named_scope(body.syntax(), section.scope_id(), name);
    }
    let mut units = Vec::new();
    let mut exports = Vec::new();
    collect_document_units(body.syntax(), &mut units, &mut exports)?;
    compile_collected_document(
        section.scope_id(),
        anchor,
        units,
        exports,
        None,
        BTreeMap::new(),
        false,
        BTreeMap::new(),
        &BTreeSet::new(),
        &BTreeSet::new(),
        &BTreeSet::new(),
    )
}

fn compile_collected_document(
    owner: DocumentScopeId,
    anchor: SourceSemanticAnchor,
    units: Vec<DocumentUnit>,
    exports: Vec<ExportDeclarationSyntax>,
    catalog: Option<Arc<mech_core::FunctionCatalog>>,
    input_schemas: BTreeMap<String, SchemaBody>,
    interactive: bool,
    resource_writes: BTreeMap<String, mech_core::ExecutionResourceRequest>,
    external_definitions: &BTreeSet<String>,
    published_bindings: &BTreeSet<String>,
    resolved_source_modules: &BTreeSet<String>,
) -> Result<CanonicalSourceProgram, SourceSemanticError> {
    let mut builder = match catalog {
        Some(catalog) if !input_schemas.is_empty() => {
            SemanticBuilder::with_function_catalog_and_input_schemas(
                anchor,
                catalog,
                input_schemas,
            )?
        }
        Some(catalog) => SemanticBuilder::with_function_catalog(anchor, catalog)?,
        None => SemanticBuilder::new(anchor),
    };
    builder.resource_writes = resource_writes;
    builder.external_definitions = external_definitions.clone();
    builder.resolved_source_modules = resolved_source_modules.clone();
    builder.register_document_functions(&units)?;
    builder.register_document_imports(&units, resolved_source_modules)?;
    let mut bindings = BTreeSet::new();
    if interactive {
        bindings.insert("ans".to_owned());
    }
    declare_document_inputs(&mut builder, &units, &mut bindings)?;
    declare_document_inline_inputs(&mut builder, &units, &bindings)?;
    let mut presentation = Vec::new();
    let last = compile_document_units(
        &mut builder,
        units,
        &bindings,
        &mut presentation,
        interactive,
    )?;
    let Some(last) = last else {
        return Err(SourceSemanticError {
            code: "source-semantics/empty-document",
            message: "canonical document contains no executable source unit".to_owned(),
            anchor,
        });
    };
    // Constraint query outputs are separate from the implicit document result.
    let constraint_outputs = std::mem::take(&mut builder.outputs);
    builder.publish("result", None, last.value, &last.syntax);
    if interactive {
        builder.outputs.extend(constraint_outputs);
    }
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
    for output_name in published_bindings {
        if builder
            .outputs
            .iter()
            .any(|output| output.name == *output_name)
        {
            continue;
        }
        let decoded = crate::decode_interactive_symbol_output_name(output_name);
        let name = decoded.as_ref().unwrap_or(output_name);
        let value = if let Some(binding) = builder.bindings.get(name).copied() {
            builder.read_document_binding(binding, &last.syntax)?
        } else if let Some(input) = builder.input_by_name.get(name) {
            PendingValue::Input(*input)
        } else if let Some(schema) = builder.input_schema_overrides.get(name).cloned() {
            let ordinal = u32::try_from(builder.inputs.len())
                .map_err(|_| internal(anchor, "input identity exhausted".to_owned()))?;
            builder.inputs.push(PendingInput {
                name: name.clone(),
                schema,
                anchor,
            });
            builder.input_by_name.insert(name.clone(), ordinal);
            PendingValue::Input(ordinal)
        } else {
            return Err(SourceSemanticError {
                code: "source-semantics/unknown-published-binding",
                message: format!("document does not define requested output {name}"),
                anchor,
            });
        };
        builder.publish(output_name, None, value, &last.syntax);
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
    if interactive {
        let bindings = builder
            .bindings
            .iter()
            .map(|(name, binding)| (name.clone(), *binding))
            .collect::<Vec<_>>();
        for (name, binding) in bindings {
            // Interactive identity names the retained cell, not its transient
            // next-value calculation. Replacement migrates that same cell.
            let value = match binding {
                PendingBinding::MutableState(state) => PendingValue::State(state),
                PendingBinding::Value(_) => builder.read_document_binding(binding, &last.syntax)?,
            };
            builder.publish(
                &crate::encode_interactive_symbol_output_name(&name),
                Some(name),
                value,
                &last.syntax,
            );
        }
    }
    builder.order_document_state_writers();
    let mut program = builder.finish()?;
    program.document_owner = Some(owner);
    program.document_outputs = output_bindings.into_boxed_slice();
    program.document_exports = document_exports.into_boxed_slice();
    Ok(program)
}

fn mixed_section_identity(
    section: &mech_syntax::document::SectionSyntax,
) -> Result<Option<(String, mech_core::ComputePlacement)>, SourceSemanticError> {
    let Some(subtitle) = section.subtitle() else {
        return Ok(None);
    };
    let text = subtitle.syntax().text().map_err(|_| {
        internal(
            SourceSemanticAnchor::for_node(subtitle.syntax()),
            "compute section subtitle is outside retained source".to_owned(),
        )
    })?;
    let heading = text.lines().next().unwrap_or_default().trim();
    let mut name_parts = Vec::new();
    let mut selected = None;
    for part in heading.split_whitespace() {
        let Some(annotation) = part.strip_prefix('@') else {
            if selected.is_none() {
                name_parts.push(part);
                continue;
            }
            return Err(SourceSemanticError {
                code: "source-semantics/section-annotation-order",
                message: "section text cannot follow a placement annotation".to_owned(),
                anchor: SourceSemanticAnchor::for_node(subtitle.syntax()),
            });
        };
        if annotation.contains(['(', ')', ',']) {
            return Err(SourceSemanticError {
                code: "source-semantics/section-placement-arguments",
                message: format!(
                    "section placement annotation @{annotation} does not accept arguments"
                ),
                anchor: SourceSemanticAnchor::for_node(subtitle.syntax()),
            });
        }
        let placement = match annotation {
            "compute" => mech_core::ComputePlacement::Compute,
            "cpu" => mech_core::ComputePlacement::Cpu,
            "gpu" => mech_core::ComputePlacement::Gpu,
            other => {
                return Err(SourceSemanticError {
                    code: "source-semantics/unsupported-section-annotation",
                    message: format!("section annotation @{other} is not supported"),
                    anchor: SourceSemanticAnchor::for_node(subtitle.syntax()),
                });
            }
        };
        if selected.replace(placement).is_some() {
            return Err(SourceSemanticError {
                code: "source-semantics/duplicate-section-placement",
                message: "section has more than one placement annotation".to_owned(),
                anchor: SourceSemanticAnchor::for_node(subtitle.syntax()),
            });
        }
    }
    let Some(placement) = selected else {
        return Ok(None);
    };
    let name = name_parts.join(" ");
    if name.is_empty() {
        return Err(SourceSemanticError {
            code: "source-semantics/empty-compute-region-name",
            message: "the executable compute region must have a nonempty section name".to_owned(),
            anchor: SourceSemanticAnchor::for_node(subtitle.syntax()),
        });
    }
    Ok(Some((name, placement)))
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
    if node.kind() == SyntaxKind::FunctionDefine {
        output.push(DocumentUnit::Function(node.clone()));
        return Ok(());
    }
    if let Some(send) = mech_syntax::document::ContextSendSyntax::cast(node.clone()) {
        output.push(DocumentUnit::ResourceSend(send));
        return Ok(());
    }
    if let Some(invariant) = InvariantDefineSyntax::cast(node.clone()) {
        output.push(DocumentUnit::Invariant(invariant));
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
            | SyntaxKind::TupleDestructure
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
    if let Some(import) = mech_syntax::document::ModuleImportSyntax::cast(node.clone()) {
        output.push(DocumentUnit::Import(import));
        return Ok(());
    }
    // Resolver-owned declarations participate through the canonical source
    // index and runtime handoff; they do not emit engine operations themselves.
    if matches!(
        node.kind(),
        SyntaxKind::ContextDeclaration | SyntaxKind::ImportDeclaration
    ) {
        return Ok(());
    }
    if matches!(
        node.kind(),
        SyntaxKind::ActivationScope
            | SyntaxKind::EnumDefine
            | SyntaxKind::Fsm
            | SyntaxKind::FsmDeclare
            | SyntaxKind::FsmImplementation
            // An expression owns a pipe's semantics. A bare pipe in a document
            // must not be traversed as unrelated child expressions.
            | SyntaxKind::FsmPipe
            | SyntaxKind::FsmSpecification
            | SyntaxKind::KindDefine
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
            DocumentUnit::Function(_) | DocumentUnit::Import(_) => {}
            DocumentUnit::Statement(unit) => {
                builder.declare_unit_input_annotations(unit, bindings)?
            }
            DocumentUnit::ResourceSend(send) => {
                let expression =
                    builder.required(send.expression(), send.syntax(), "a resource-send value")?;
                builder.declare_input_annotations(expression.syntax(), bindings)?;
            }
            DocumentUnit::Invariant(invariant) => {
                builder.declare_input_annotations(invariant.syntax(), bindings)?
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
            DocumentUnit::Statement(_) | DocumentUnit::Function(_) | DocumentUnit::Import(_) => {}
            DocumentUnit::ResourceSend(_) => {}
            DocumentUnit::Invariant(_) => {}
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
    interactive: bool,
) -> Result<Option<CompiledDocumentValue>, SourceSemanticError> {
    let mut deferred_inline = Vec::new();
    let mut last = compile_document_units_inner(
        builder,
        units,
        local_bindings,
        presentation,
        &mut deferred_inline,
        interactive,
    )?;
    refresh_deferred_inline(builder, &mut deferred_inline, presentation, &mut last)?;
    if let Some(deferred) = deferred_inline.first() {
        return Err(internal(
            SourceSemanticAnchor::for_node(deferred.inline.syntax()),
            "a deferred inline expression never received its local definition".to_owned(),
        ));
    }
    Ok(last)
}

fn compile_document_units_inner(
    builder: &mut SemanticBuilder,
    units: Vec<DocumentUnit>,
    local_bindings: &BTreeSet<String>,
    presentation: &mut Vec<(SourceDocumentOutputKind, PendingValue, SyntaxNode)>,
    deferred_inline: &mut Vec<DeferredInline>,
    interactive: bool,
) -> Result<Option<CompiledDocumentValue>, SourceSemanticError> {
    let mut last = None;
    for unit in units {
        match unit {
            DocumentUnit::Function(_) | DocumentUnit::Import(_) => {}
            DocumentUnit::Statement(unit) => {
                let result = match unit.kind() {
                    SyntaxKind::VariableDefine => {
                        builder.definition(&VariableDefineSyntax::cast(unit).unwrap())?
                    }
                    SyntaxKind::Expression => {
                        builder.expression(&ExpressionSyntax::cast(unit).unwrap())?
                    }
                    SyntaxKind::TupleDestructure => builder.document_tuple_destructure(&unit)?,
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
                refresh_deferred_inline(builder, deferred_inline, presentation, &mut last)?;
            }
            DocumentUnit::ResourceSend(send) => {
                let value = builder.emit_resource_send(&send)?;
                if last.is_none() {
                    last = Some(CompiledDocumentValue {
                        value,
                        syntax: send.syntax().clone(),
                        program_visible: false,
                    });
                }
                refresh_deferred_inline(builder, deferred_inline, presentation, &mut last)?;
            }
            DocumentUnit::Invariant(invariant) => {
                let name = invariant
                    .syntax()
                    .children()
                    .find(|child| child.kind() == SyntaxKind::Identifier)
                    .ok_or_else(|| {
                        internal(
                            SourceSemanticAnchor::for_node(invariant.syntax()),
                            "invariant definition has no name".to_owned(),
                        )
                    })?;
                let expression = invariant
                    .syntax()
                    .children()
                    .find_map(ExpressionSyntax::cast)
                    .ok_or_else(|| {
                        internal(
                            SourceSemanticAnchor::for_node(invariant.syntax()),
                            "invariant definition has no expression".to_owned(),
                        )
                    })?;
                let (value, _) = builder.expression(&expression)?;
                value.resolved()?;
                if builder.schema_of(value)? != Some(BuiltinSchema::Bool) {
                    return Err(SourceSemanticError {
                        code: "source-semantics/non-boolean-invariant",
                        message: "an invariant expression must resolve to bool".to_owned(),
                        anchor: SourceSemanticAnchor::for_node(expression.syntax()),
                    });
                }
                let name = format!("{}!", node_text(&name)?);
                builder.publish(&name, None, value, invariant.syntax());
                builder.constraints.push(PendingConstraint { name, value });
                refresh_deferred_inline(builder, deferred_inline, presentation, &mut last)?;
            }
            DocumentUnit::Inline(inline) => {
                if let Some(deferred) = defer_inline(builder, &inline, local_bindings)? {
                    deferred_inline.push(deferred);
                } else {
                    retain_later_document_value(
                        &mut last,
                        compile_inline(builder, inline, presentation)?,
                    );
                }
            }
            DocumentUnit::Fence(fence, fence_presentation, units) => {
                if let Some(compiled) = compile_document_units_inner(
                    builder,
                    units,
                    local_bindings,
                    presentation,
                    deferred_inline,
                    interactive,
                )? {
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
                refresh_deferred_inline(builder, deferred_inline, presentation, &mut last)?;
            }
        }
        // `ans` is a source-level alias of the preceding interactive value,
        // not a live input or additional recurrence cell. Selection literals
        // use this same sequential rule as ordinary submitted expressions.
        if interactive && let Some(value) = &last {
            builder
                .bindings
                .insert("ans".to_owned(), PendingBinding::Value(value.value));
        }
    }
    Ok(last)
}

fn defer_inline(
    builder: &mut SemanticBuilder,
    inline: &EvalInlineMechCodeSyntax,
    local_bindings: &BTreeSet<String>,
) -> Result<Option<DeferredInline>, SourceSemanticError> {
    let references = inline_local_references(builder, inline, local_bindings)?;
    let waiting = references
        .iter()
        .filter(|name| !builder.bindings.contains_key(*name))
        .cloned()
        .collect::<BTreeSet<_>>();
    if waiting.is_empty() {
        return Ok(None);
    }
    let mut captured = BTreeMap::new();
    for name in references.difference(&waiting) {
        let binding = builder.bindings.get(name).copied().ok_or_else(|| {
            internal(
                SourceSemanticAnchor::for_node(inline.syntax()),
                format!("local inline binding {name} disappeared during capture"),
            )
        })?;
        let value = builder.read_document_binding(binding, inline.syntax())?;
        captured.insert(name.clone(), value);
    }
    Ok(Some(DeferredInline {
        inline: inline.clone(),
        captured,
        waiting,
    }))
}

fn refresh_deferred_inline(
    builder: &mut SemanticBuilder,
    deferred: &mut Vec<DeferredInline>,
    presentation: &mut Vec<(SourceDocumentOutputKind, PendingValue, SyntaxNode)>,
    last: &mut Option<CompiledDocumentValue>,
) -> Result<(), SourceSemanticError> {
    for deferred in deferred.iter_mut() {
        let available = deferred
            .waiting
            .iter()
            .filter(|name| builder.bindings.contains_key(*name))
            .cloned()
            .collect::<Vec<_>>();
        for name in available {
            let binding = builder.bindings.get(&name).copied().ok_or_else(|| {
                internal(
                    SourceSemanticAnchor::for_node(deferred.inline.syntax()),
                    format!("local inline binding {name} disappeared during capture"),
                )
            })?;
            let value = builder.read_document_binding(binding, deferred.inline.syntax())?;
            deferred.captured.insert(name.clone(), value);
            deferred.waiting.remove(&name);
        }
    }
    loop {
        let ready = deferred
            .iter()
            .position(|deferred| deferred.waiting.is_empty());
        let Some(ready) = ready else { break };
        let deferred = deferred.remove(ready);
        retain_later_document_value(
            last,
            compile_deferred_inline(builder, deferred, presentation)?,
        );
    }
    Ok(())
}

fn compile_deferred_inline(
    builder: &mut SemanticBuilder,
    deferred: DeferredInline,
    presentation: &mut Vec<(SourceDocumentOutputKind, PendingValue, SyntaxNode)>,
) -> Result<CompiledDocumentValue, SourceSemanticError> {
    for name in deferred.captured.keys() {
        if !builder.bindings.contains_key(name) {
            return Err(internal(
                SourceSemanticAnchor::for_node(deferred.inline.syntax()),
                format!("captured inline binding {name} has no current definition"),
            ));
        }
    }
    let mut previous = Vec::new();
    for (name, value) in &deferred.captured {
        let binding = builder
            .bindings
            .insert(name.clone(), PendingBinding::Value(*value))
            .expect("captured bindings were validated before replacement");
        previous.push((name.clone(), binding));
    }
    let compiled = compile_inline(builder, deferred.inline, presentation);
    for (name, binding) in previous {
        builder.bindings.insert(name, binding);
    }
    compiled
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

fn inline_local_references(
    builder: &SemanticBuilder,
    inline: &EvalInlineMechCodeSyntax,
    local_bindings: &BTreeSet<String>,
) -> Result<BTreeSet<String>, SourceSemanticError> {
    let mut references = BTreeSet::new();
    let mut pending = vec![inline.syntax().clone()];
    while let Some(node) = pending.pop() {
        if let Some(variable) = VariableSyntax::cast(node.clone()) {
            let stem = builder.required(variable.stem(), variable.syntax(), "a variable stem")?;
            let name = node_text(stem.syntax())?;
            if local_bindings.contains(&name) {
                references.insert(name);
            }
            continue;
        }
        if let Some(slice) = SliceSyntax::cast(node.clone()) {
            let stem = builder.required(slice.stem(), slice.syntax(), "a slice stem")?;
            let name = node_text(stem.syntax())?;
            if local_bindings.contains(&name) {
                references.insert(name);
            }
            if let Some(subscripts) = slice.subscripts() {
                pending.push(subscripts.syntax().clone());
            }
            continue;
        }
        pending.extend(node.children());
    }
    Ok(references)
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
        for constraint in &mut self.constraints {
            remap(&mut constraint.value);
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
        if let Some(SliceStemSyntax::Context(path)) = target.stem() {
            if operation.is_some() || target.subscripts().is_some() {
                return Err(SourceSemanticError {
                    code: "source-semantics/unsupported-resource-assignment",
                    message: "resource assignment requires a direct addressed target".to_owned(),
                    anchor: SourceSemanticAnchor::for_node(syntax),
                });
            }
            let context = self.required(path.context(), path.syntax(), "a resource context")?;
            let address = self.required(path.address(), path.syntax(), "a resource path")?;
            let key = format!(
                "=@{}/{}",
                node_text(context.syntax())?,
                node_text(address.syntax())?
            );
            let value = self.emit_resource_write(&key, &expression, syntax, "resource/assign")?;
            return Ok((value, syntax.clone()));
        }
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

impl SemanticBuilder {
    fn document_tuple_destructure(
        &mut self,
        syntax: &SyntaxNode,
    ) -> Result<(PendingValue, SyntaxNode), SourceSemanticError> {
        let destructure =
            mech_syntax::document::TupleDestructureSyntax::cast(syntax.clone()).unwrap();
        let expression = self.required(destructure.value(), syntax, "a destructuring value")?;
        let value = self.expression(&expression)?.0;
        let schema = self.schema_draft_of(value)?;
        let SchemaBody::Tuple(items) = &schema.body else {
            return Err(SourceSemanticError {
                code: "source-semantics/destructure-requires-tuple",
                message: "tuple destructuring requires a tuple value".to_owned(),
                anchor: SourceSemanticAnchor::for_node(expression.syntax()),
            });
        };
        let names = destructure.names();
        if names.len() > items.len() {
            return Err(SourceSemanticError {
                code: "source-semantics/destructure-arity",
                message: "tuple destructuring has more names than tuple elements".to_owned(),
                anchor: SourceSemanticAnchor::for_node(syntax),
            });
        }
        let mut declared = BTreeSet::new();
        for name in &names {
            let text = node_text(name.syntax())?;
            if self.scope_definitions.contains(&text) || !declared.insert(text.clone()) {
                return Err(SourceSemanticError {
                    code: "source-semantics/variable-already-defined",
                    message: format!("variable {text} is already defined"),
                    anchor: SourceSemanticAnchor::for_node(name.syntax()),
                });
            }
        }
        for (ordinal, name) in names.iter().enumerate() {
            let text = node_text(name.syntax())?;
            let selector = self.constant_exact(
                SchemaBody::Index,
                ValueDataDraft::Index((ordinal + 1) as u64),
            );
            let mut selected = self.select_values(value, vec![Some(selector)], name.syntax())?;
            if self.external_definitions.contains(&text) {
                let schema = self.schema_draft_of(selected)?;
                let index = u32::try_from(self.inputs.len()).map_err(|_| {
                    internal(
                        SourceSemanticAnchor::for_node(syntax),
                        "input identity exhausted".to_owned(),
                    )
                })?;
                self.input_by_name.insert(text.clone(), index);
                self.inputs.push(PendingInput {
                    name: text.clone(),
                    schema,
                    anchor: SourceSemanticAnchor::for_node(name.syntax()),
                });
                selected = PendingValue::Input(index);
            }
            self.scope_definitions.insert(text.clone());
            self.bindings.insert(text, PendingBinding::Value(selected));
        }
        Ok((value, syntax.clone()))
    }
}

pub(super) fn compile_ordered_documents(
    documents: &[CanonicalOrderedDocument],
    catalog: Arc<mech_core::FunctionCatalog>,
) -> Result<CanonicalSourceProgram, SourceSemanticError> {
    let first = documents.first().ok_or_else(|| {
        internal(
            SourceSemanticAnchor {
                document: DocumentId(0),
                revision: Revision(0),
                range: TextRange::empty(mech_syntax::document::TextSize::ZERO),
            },
            "ordered compilation requires at least one retained root".to_owned(),
        )
    })?;
    let anchor = SourceSemanticAnchor::for_node(first.document.syntax());
    let mut builder = SemanticBuilder::with_function_catalog(anchor, catalog)?;
    let mut exports_by_root = BTreeMap::<usize, BTreeMap<String, PendingBinding>>::new();
    let mut constants = BTreeMap::new();
    let mut results = BTreeMap::new();
    let mut presentation = Vec::new();
    for root in documents {
        let anchor = SourceSemanticAnchor::for_node(root.document.syntax());
        if results.contains_key(&root.identity) {
            return Err(internal(
                anchor,
                "ordered root identity is repeated".to_owned(),
            ));
        }
        // Each retained root owns its callable imports and local definitions.
        // Shared graph values do not grant another root's function visibility.
        builder.function_environment = Some(
            crate::FunctionEnvironment::from_catalog_defaults(
                builder
                    .function_catalog
                    .as_ref()
                    .expect("ordered roots have a catalog"),
            )
            .map_err(|error| internal(anchor, error.display_message()))?,
        );
        builder.function_imports.clear();
        builder.local_functions.clear();
        builder.resource_writes = root.resource_writes.clone();
        builder.resolved_source_modules = root.resolved_modules.clone();
        builder.input_schema_overrides = root
            .input_schemas
            .iter()
            .map(|(name, body)| {
                (
                    name.clone(),
                    SchemaDraft {
                        body: body.clone(),
                        dimension_parameters: Box::new([]),
                    },
                )
            })
            .collect();
        for (name, imported) in &root.imports {
            let binding = match imported {
                CanonicalOrderedImport::RootExport { root, name: export } => *exports_by_root
                    .get(root)
                    .and_then(|exports| exports.get(export))
                    .ok_or_else(|| {
                        internal(
                            anchor,
                            format!("linked root {root} has no preceding export {export}"),
                        )
                    })?,
                CanonicalOrderedImport::Value(value) => {
                    let schemas = value
                        .schemas()
                        .ok_or_else(|| internal(anchor, "import has no schema owner".to_owned()))?;
                    let schema = schemas.get(value.schema()).ok_or_else(|| {
                        internal(anchor, "import schema is unavailable".to_owned())
                    })?;
                    let input_name = format!("root:{}/import:{name}", root.identity);
                    let ordinal = builder.inputs.len() as u32;
                    builder.inputs.push(PendingInput {
                        name: input_name.clone(),
                        schema: SchemaDraft {
                            body: schema
                                .closed_body(value.shape())
                                .map_err(|error| internal(anchor, format!("{error:?}")))?,
                            dimension_parameters: Box::new([]),
                        },
                        anchor,
                    });
                    constants.insert(input_name, value.clone());
                    PendingBinding::Value(PendingValue::Input(ordinal))
                }
            };
            builder.bindings.insert(name.clone(), binding);
        }
        let mut units = Vec::new();
        let mut exports = Vec::new();
        collect_document_units(root.document.syntax(), &mut units, &mut exports)?;
        builder.register_document_functions(&units)?;
        builder.register_document_imports(&units, &root.resolved_modules)?;
        let mut bindings = builder.bindings.keys().cloned().collect();
        declare_document_inputs(&mut builder, &units, &mut bindings)?;
        declare_document_inline_inputs(&mut builder, &units, &bindings)?;
        let last =
            compile_document_units(&mut builder, units, &bindings, &mut presentation, false)?
                .ok_or_else(|| {
                    internal(
                        anchor,
                        "ordered root has no executable source unit".to_owned(),
                    )
                })?;
        // Keep the source binding's name where the last expression names it;
        // otherwise each root has its own unambiguous result identity.
        let text = node_text(&last.syntax)?;
        let definition_name = VariableDefineSyntax::cast(last.syntax.clone())
            .and_then(|definition| definition.variable())
            .and_then(|variable| variable.stem())
            .map(|stem| node_text(stem.syntax()))
            .transpose()?;
        let name = if let Some(name) = definition_name {
            name
        } else if builder.bindings.contains_key(text.trim()) {
            text.trim().to_owned()
        } else {
            format!("root:{}:result", root.identity)
        };
        results.insert(root.identity, (name, last));
        let mut root_exports = BTreeMap::new();
        for export in exports {
            let name = builder.required(export.name(), export.syntax(), "an exported name")?;
            let name = node_text(name.syntax())?;
            let binding = builder.bindings.get(&name).copied().ok_or_else(|| {
                internal(
                    anchor,
                    format!("ordered root exports undefined binding {name}"),
                )
            })?;
            if root_exports.insert(name.clone(), binding).is_some() {
                return Err(internal(
                    anchor,
                    format!("ordered root exports {name} more than once"),
                ));
            }
        }
        exports_by_root.insert(root.identity, root_exports);
        // Resource spellings are local to a document. Their graph input names
        // carry the root identity before the next root declares its contexts.
        for name in root.input_schemas.keys() {
            if let Some(input) = builder.input_by_name.remove(name) {
                builder.inputs[input as usize].name = format!("root:{}/{name}", root.identity);
            }
        }
    }
    // Constraints remain constraints; requested roots and visible document
    // slots alone become ordinary outputs.
    builder.outputs.clear();
    let mut output_bindings = Vec::new();
    for (_, (name, last)) in results {
        output_bindings.push(SourceDocumentOutput {
            output: builder.outputs.len() as u32,
            kind: SourceDocumentOutputKind::Program,
            visible: last.program_visible,
        });
        builder.publish(&name, None, last.value, &last.syntax);
    }
    for (kind, value, owner) in presentation {
        let role = match kind {
            SourceDocumentOutputKind::Inline => "inline",
            SourceDocumentOutputKind::Fence => "fence",
            SourceDocumentOutputKind::Program => unreachable!(),
        };
        let anchor = SourceSemanticAnchor::for_node(&owner);
        let name = format!(
            "document:{}:{role}:{}",
            anchor.document.0,
            owner.range().start.0
        );
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
    let constants = program
        .program
        .inputs
        .iter()
        .enumerate()
        .filter_map(|(ordinal, input)| {
            constants
                .get(&input.name)
                .map(|value| (ordinal as u32, value.clone()))
        })
        .collect::<Vec<_>>();
    program.bind_input_constants(&constants)
}
