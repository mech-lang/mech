use mech_core::{
    ComprehensionQualifier, ContextBase, ContextCapabilityScope, Expression, Factor, MechCode,
    MResult, MechError, Pattern, Program, RangeExpression, SectionElement, SourceRange, Statement,
    Structure, Subscript, Term, Token,
};

use super::{
    classify_import_specifier, AddressTargetNameConflict, SourceAddressReference, SourceContextBase,
    SourceContextCapability, SourceContextCapabilityScope, SourceContextDeclaration,
    SourceExportDeclaration, SourceImportDeclaration,
};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SourceScope {
    Program,
    Interpreter(SourceInterpreterId),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SourceInterpreterId {
    pub namespace: u64,
    pub namespace_str: String,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceOccurrence {
    pub scope: SourceScope,
    pub order: usize,
    pub range: Option<SourceRange>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopedSourceImportDeclaration {
    pub occurrence: SourceOccurrence,
    pub declaration: SourceImportDeclaration,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopedSourceExportDeclaration {
    pub occurrence: SourceOccurrence,
    pub declaration: SourceExportDeclaration,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopedSourceContextDeclaration {
    pub occurrence: SourceOccurrence,
    pub declaration: SourceContextDeclaration,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopedSourceAddressReference {
    pub occurrence: SourceOccurrence,
    pub reference: SourceAddressReference,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceDeclaration {
    Import(ScopedSourceImportDeclaration),
    Export(ScopedSourceExportDeclaration),
    Context(ScopedSourceContextDeclaration),
    AddressReference(ScopedSourceAddressReference),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleScopeMetadata {
    pub scope: SourceScope,
    pub imports: Vec<SourceImportDeclaration>,
    pub exports: Vec<SourceExportDeclaration>,
    pub contexts: Vec<SourceContextDeclaration>,
    pub address_references: Vec<SourceAddressReference>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourceIndex {
    pub declarations: Vec<SourceDeclaration>,
    pub imports: Vec<ScopedSourceImportDeclaration>,
    pub exports: Vec<ScopedSourceExportDeclaration>,
    pub contexts: Vec<ScopedSourceContextDeclaration>,
    pub address_references: Vec<ScopedSourceAddressReference>,
    pub scopes: Vec<SourceScope>,
    pub address_target_interpreters: Vec<SourceInterpreterId>,
}

impl SourceIndex {
    pub fn from_program(program: &Program) -> Self {
        let mut index = Self::default();
        let mut order = 0usize;
        let mut fenced_interpreters = std::collections::HashMap::new();
        index.push_scope(SourceScope::Program);

        for section in &program.body.sections {
            for element in &section.elements {
                match element {
                    SectionElement::MechCode(code_items) => {
                        for (code, _) in code_items {
                            match code {
                                MechCode::Statement(statement) => {
                                    match statement {
                                        Statement::ImportDeclaration(import) => {
                                            index.push_import(
                                                SourceScope::Program,
                                                order,
                                                declaration_range(import.tokens()),
                                                classify_import_specifier(import.specifier.to_string()),
                                            );
                                            order += 1;
                                        }
                                        Statement::ExportDeclaration(export) => {
                                            index.push_export(
                                                SourceScope::Program,
                                                order,
                                                declaration_range(export.tokens()),
                                                SourceExportDeclaration {
                                                    name: export.name.to_string(),
                                                },
                                            );
                                            order += 1;
                                        }
                                        Statement::ContextDeclaration(context) => {
                                            index.push_context(
                                                SourceScope::Program,
                                                order,
                                                declaration_range(context.tokens()),
                                                source_context_declaration(context),
                                            );
                                            order += 1;
                                        }
                                        _ => {}
                                    }
                                    index_statement_address_references(
                                        &mut index,
                                        &SourceScope::Program,
                                        &mut order,
                                        statement,
                                    );
                                }
                                MechCode::Expression(expression) => index_expression_address_references(
                                    &mut index,
                                    &SourceScope::Program,
                                    &mut order,
                                    expression,
                                ),
                                _ => {}
                            }
                        }
                    }
                    SectionElement::FencedMechCode(fenced) => {
                        let interpreter = SourceInterpreterId {
                            namespace: fenced.config.namespace,
                            namespace_str: fenced.config.namespace_str.clone(),
                        };
                        let scope = fenced_interpreters
                            .entry(interpreter.namespace_str.clone())
                            .or_insert_with(|| {
                                index.address_target_interpreters.push(interpreter.clone());
                                let scope = SourceScope::Interpreter(interpreter);
                                index.push_scope(scope.clone());
                                scope
                            })
                            .clone();

                        // TODO: Interleaving imports/exports/statements exactly as written in fenced code
                        // requires parser ordering data; currently imports and exports preserve local vector order.
                        for import in &fenced.imports {
                            index.push_import(
                                scope.clone(),
                                order,
                                declaration_range(import.tokens()),
                                classify_import_specifier(import.specifier.to_string()),
                            );
                            order += 1;
                        }

                        for export in &fenced.exports {
                            index.push_export(
                                scope.clone(),
                                order,
                                declaration_range(export.tokens()),
                                SourceExportDeclaration {
                                    name: export.name.to_string(),
                                },
                            );
                            order += 1;
                        }

                        for (code, _) in &fenced.code {
                            match code {
                                MechCode::Statement(Statement::ContextDeclaration(context)) => {
                                    index.push_context(
                                        scope.clone(),
                                        order,
                                        declaration_range(context.tokens()),
                                        source_context_declaration(context),
                                    );
                                    order += 1;
                                }
                                MechCode::Statement(statement) => index_statement_address_references(
                                    &mut index,
                                    &scope,
                                    &mut order,
                                    statement,
                                ),
                                MechCode::Expression(expression) => index_expression_address_references(
                                    &mut index,
                                    &scope,
                                    &mut order,
                                    expression,
                                ),
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        index
    }

    pub fn validate_address_targets(&self) -> MResult<()> {
        let mut targets: std::collections::HashMap<String, String> = std::collections::HashMap::new();

        for interpreter in &self.address_target_interpreters {
            if let Some(first_kind) = targets.insert(interpreter.namespace_str.clone(), "interpreter".to_string()) {
                return Err(MechError::new(
                    AddressTargetNameConflict {
                        name: interpreter.namespace_str.clone(),
                        first_kind,
                        second_kind: "interpreter".to_string(),
                    },
                    None,
                ));
            }
        }

        for context in &self.contexts {
            if let Some(first_kind) = targets.insert(context.declaration.name.clone(), "context".to_string()) {
                return Err(MechError::new(
                    AddressTargetNameConflict {
                        name: context.declaration.name.clone(),
                        first_kind,
                        second_kind: "context".to_string(),
                    },
                    None,
                ));
            }
        }

        Ok(())
    }

    pub fn module_scopes(&self) -> Vec<ModuleScopeMetadata> {
        let mut scopes: Vec<SourceScope> = self.scopes.clone();

        for declaration in &self.declarations {
            let scope = match declaration {
                SourceDeclaration::Import(import) => &import.occurrence.scope,
                SourceDeclaration::Export(export) => &export.occurrence.scope,
                SourceDeclaration::Context(context) => &context.occurrence.scope,
                SourceDeclaration::AddressReference(reference) => &reference.occurrence.scope,
            };
            if !scopes.contains(scope) {
                scopes.push(scope.clone());
            }
        }

        scopes
            .into_iter()
            .map(|scope| ModuleScopeMetadata {
                imports: self.imports_for_scope(&scope),
                exports: self.exports_for_scope(&scope),
                contexts: self.contexts_for_scope(&scope),
                address_references: self.address_references_for_scope(&scope),
                scope,
            })
            .collect()
    }

    pub fn all_imports(&self) -> Vec<SourceImportDeclaration> {
        self.imports.iter().map(|x| x.declaration.clone()).collect()
    }
    pub fn all_exports(&self) -> Vec<SourceExportDeclaration> {
        self.exports.iter().map(|x| x.declaration.clone()).collect()
    }
    pub fn all_contexts(&self) -> Vec<SourceContextDeclaration> {
        self.contexts
            .iter()
            .map(|x| x.declaration.clone())
            .collect()
    }
    pub fn all_address_references(&self) -> Vec<SourceAddressReference> {
        self.address_references
            .iter()
            .map(|x| x.reference.clone())
            .collect()
    }

    pub fn program_imports(&self) -> Vec<SourceImportDeclaration> {
        self.imports_for_scope(&SourceScope::Program)
    }
    pub fn program_exports(&self) -> Vec<SourceExportDeclaration> {
        self.exports_for_scope(&SourceScope::Program)
    }
    pub fn program_contexts(&self) -> Vec<SourceContextDeclaration> {
        self.contexts_for_scope(&SourceScope::Program)
    }
    pub fn program_address_references(&self) -> Vec<SourceAddressReference> {
        self.address_references_for_scope(&SourceScope::Program)
    }

    pub fn imports_for_scope(&self, scope: &SourceScope) -> Vec<SourceImportDeclaration> {
        self.imports
            .iter()
            .filter(|x| &x.occurrence.scope == scope)
            .map(|x| x.declaration.clone())
            .collect()
    }

    pub fn exports_for_scope(&self, scope: &SourceScope) -> Vec<SourceExportDeclaration> {
        self.exports
            .iter()
            .filter(|x| &x.occurrence.scope == scope)
            .map(|x| x.declaration.clone())
            .collect()
    }

    pub fn contexts_for_scope(&self, scope: &SourceScope) -> Vec<SourceContextDeclaration> {
        self.contexts
            .iter()
            .filter(|x| &x.occurrence.scope == scope)
            .map(|x| x.declaration.clone())
            .collect()
    }

    pub fn address_references_for_scope(&self, scope: &SourceScope) -> Vec<SourceAddressReference> {
        self.address_references
            .iter()
            .filter(|x| &x.occurrence.scope == scope)
            .map(|x| x.reference.clone())
            .collect()
    }

    pub fn interpreter_scopes(&self) -> Vec<SourceInterpreterId> {
        let mut scopes = Vec::new();
        for scope in &self.scopes {
            if let SourceScope::Interpreter(interpreter) = scope {
                if !scopes.contains(interpreter) {
                    scopes.push(interpreter.clone());
                }
            }
        }
        for declaration in &self.declarations {
            let scope = match declaration {
                SourceDeclaration::Import(import) => &import.occurrence.scope,
                SourceDeclaration::Export(export) => &export.occurrence.scope,
                SourceDeclaration::Context(context) => &context.occurrence.scope,
                SourceDeclaration::AddressReference(reference) => &reference.occurrence.scope,
            };
            if let SourceScope::Interpreter(interpreter) = scope {
                if !scopes.contains(interpreter) {
                    scopes.push(interpreter.clone());
                }
            }
        }
        scopes
    }

    fn push_scope(&mut self, scope: SourceScope) {
        if !self.scopes.contains(&scope) {
            self.scopes.push(scope);
        }
    }

    fn push_import(
        &mut self,
        scope: SourceScope,
        order: usize,
        range: Option<SourceRange>,
        declaration: SourceImportDeclaration,
    ) {
        self.push_scope(scope.clone());
        let scoped = ScopedSourceImportDeclaration {
            occurrence: SourceOccurrence {
                scope,
                order,
                range,
            },
            declaration,
        };
        self.declarations
            .push(SourceDeclaration::Import(scoped.clone()));
        self.imports.push(scoped);
    }

    fn push_export(
        &mut self,
        scope: SourceScope,
        order: usize,
        range: Option<SourceRange>,
        declaration: SourceExportDeclaration,
    ) {
        self.push_scope(scope.clone());
        let scoped = ScopedSourceExportDeclaration {
            occurrence: SourceOccurrence {
                scope,
                order,
                range,
            },
            declaration,
        };
        self.declarations
            .push(SourceDeclaration::Export(scoped.clone()));
        self.exports.push(scoped);
    }

    fn push_context(
        &mut self,
        scope: SourceScope,
        order: usize,
        range: Option<SourceRange>,
        declaration: SourceContextDeclaration,
    ) {
        self.push_scope(scope.clone());
        let scoped = ScopedSourceContextDeclaration {
            occurrence: SourceOccurrence {
                scope,
                order,
                range,
            },
            declaration,
        };
        self.declarations
            .push(SourceDeclaration::Context(scoped.clone()));
        self.contexts.push(scoped);
    }

    fn push_address_reference(
        &mut self,
        scope: SourceScope,
        order: usize,
        range: Option<SourceRange>,
        reference: SourceAddressReference,
    ) {
        self.push_scope(scope.clone());
        let scoped = ScopedSourceAddressReference {
            occurrence: SourceOccurrence {
                scope,
                order,
                range,
            },
            reference,
        };
        self.declarations
            .push(SourceDeclaration::AddressReference(scoped.clone()));
        self.address_references.push(scoped);
    }
}

fn index_statement_address_references(
    index: &mut SourceIndex,
    scope: &SourceScope,
    order: &mut usize,
    statement: &Statement,
) {
    match statement {
        Statement::VariableDefine(def) => {
            index_expression_address_references(index, scope, order, &def.expression)
        }
        Statement::VariableAssign(assign) => {
            index_expression_address_references(index, scope, order, &assign.expression)
        }
        Statement::OpAssign(assign) => {
            index_expression_address_references(index, scope, order, &assign.expression)
        }
        Statement::TupleDestructure(destructure) => {
            index_expression_address_references(index, scope, order, &destructure.expression)
        }
        Statement::FsmDeclare(fsm) => index_expression_address_references(
            index,
            scope,
            order,
            &Expression::FsmPipe(fsm.pipe.clone()),
        ),
        Statement::ImportDeclaration(_)
        | Statement::ExportDeclaration(_)
        | Statement::ContextDeclaration(_)
        | Statement::EnumDefine(_)
        | Statement::KindDefine(_)
        | Statement::SplitTable
        | Statement::FlattenTable => {}
        #[cfg(feature = "invariant_define")]
        Statement::InvariantDefine(_) => {}
    }
}

fn index_expression_address_references(
    index: &mut SourceIndex,
    scope: &SourceScope,
    order: &mut usize,
    expression: &Expression,
) {
    match expression {
        Expression::Var(var) => {
            if let Some(target) = &var.context {
                index.push_address_reference(
                    scope.clone(),
                    *order,
                    declaration_range(var.tokens()),
                    SourceAddressReference {
                        name: var.name.to_string(),
                        target: target.to_string(),
                    },
                );
                *order += 1;
            }
        }
        Expression::Slice(slice) => {
            if let Some(target) = &slice.context {
                index.push_address_reference(
                    scope.clone(),
                    *order,
                    declaration_range(slice.tokens()),
                    SourceAddressReference {
                        name: slice.name.to_string(),
                        target: target.to_string(),
                    },
                );
                *order += 1;
            }
            for subscript in &slice.subscript {
                index_subscript_address_references(index, scope, order, subscript);
            }
        }
        Expression::Formula(factor) => {
            index_factor_address_references(index, scope, order, factor);
        }
        Expression::Structure(structure) => {
            index_structure_address_references(index, scope, order, structure);
        }
        Expression::FunctionCall(call) => {
            for (_, argument) in &call.args {
                index_expression_address_references(index, scope, order, argument);
            }
        }
        Expression::Match(match_expr) => {
            index_expression_address_references(index, scope, order, &match_expr.source);
            for arm in &match_expr.arms {
                if let Some(guard) = &arm.guard {
                    index_expression_address_references(index, scope, order, guard);
                }
                index_expression_address_references(index, scope, order, &arm.expression);
            }
        }
        Expression::Range(range) => {
            index_range_address_references(index, scope, order, range);
        }
        Expression::SetComprehension(comprehension) => {
            index_expression_address_references(index, scope, order, &comprehension.expression);
            for qualifier in &comprehension.qualifiers {
                index_comprehension_qualifier_address_references(index, scope, order, qualifier);
            }
        }
        Expression::MatrixComprehension(comprehension) => {
            index_expression_address_references(index, scope, order, &comprehension.expression);
            for qualifier in &comprehension.qualifiers {
                index_comprehension_qualifier_address_references(index, scope, order, qualifier);
            }
        }
        Expression::FsmPipe(pipe) => {
            for transition in &pipe.transitions {
                match transition {
                    mech_core::Transition::Async(pattern)
                    | mech_core::Transition::Next(pattern)
                    | mech_core::Transition::Output(pattern) => {
                        index_pattern_address_references(index, scope, order, pattern);
                    }
                    mech_core::Transition::Statement(statement) => {
                        index_statement_address_references(index, scope, order, statement);
                    }
                    mech_core::Transition::CodeBlock(code_items) => {
                        for (code, _) in code_items {
                            match code {
                                MechCode::Statement(statement) => {
                                    index_statement_address_references(index, scope, order, statement);
                                }
                                MechCode::Expression(expression) => {
                                    index_expression_address_references(index, scope, order, expression);
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
        Expression::Literal(_) => {}
    }
}

fn index_structure_address_references(
    index: &mut SourceIndex,
    scope: &SourceScope,
    order: &mut usize,
    structure: &Structure,
) {
    match structure {
        Structure::Map(map) => {
            for mapping in &map.elements {
                index_expression_address_references(index, scope, order, &mapping.key);
                index_expression_address_references(index, scope, order, &mapping.value);
            }
        }
        Structure::Matrix(matrix) => {
            for row in &matrix.rows {
                for column in &row.columns {
                    index_expression_address_references(index, scope, order, &column.element);
                }
            }
        }
        Structure::Record(record) => {
            for binding in &record.bindings {
                index_expression_address_references(index, scope, order, &binding.value);
            }
        }
        Structure::Set(set) => {
            for element in &set.elements {
                index_expression_address_references(index, scope, order, element);
            }
        }
        Structure::Table(table) => {
            for row in &table.rows {
                for column in &row.columns {
                    index_expression_address_references(index, scope, order, &column.element);
                }
            }
        }
        Structure::Tuple(tuple) => {
            for element in &tuple.elements {
                index_expression_address_references(index, scope, order, element);
            }
        }
        Structure::TupleStruct(tuple_struct) => {
            index_expression_address_references(index, scope, order, &tuple_struct.value);
        }
        Structure::Empty => {}
    }
}

fn index_subscript_address_references(
    index: &mut SourceIndex,
    scope: &SourceScope,
    order: &mut usize,
    subscript: &Subscript,
) {
    match subscript {
        Subscript::Brace(subscripts) | Subscript::Bracket(subscripts) => {
            for subscript in subscripts {
                index_subscript_address_references(index, scope, order, subscript);
            }
        }
        Subscript::Formula(factor) => index_factor_address_references(index, scope, order, factor),
        Subscript::Range(range) => index_range_address_references(index, scope, order, range),
        Subscript::All
        | Subscript::Dot(_)
        | Subscript::DotInt(_)
        | Subscript::Swizzle(_) => {}
    }
}

fn index_range_address_references(
    index: &mut SourceIndex,
    scope: &SourceScope,
    order: &mut usize,
    range: &RangeExpression,
) {
    index_factor_address_references(index, scope, order, &range.start);
    if let Some((_, increment)) = &range.increment {
        index_factor_address_references(index, scope, order, increment);
    }
    index_factor_address_references(index, scope, order, &range.terminal);
}

fn index_factor_address_references(
    index: &mut SourceIndex,
    scope: &SourceScope,
    order: &mut usize,
    factor: &Factor,
) {
    match factor {
        Factor::Expression(expression) => {
            index_expression_address_references(index, scope, order, expression);
        }
        Factor::Negate(factor)
        | Factor::Not(factor)
        | Factor::Parenthetical(factor)
        | Factor::Transpose(factor) => {
            index_factor_address_references(index, scope, order, factor);
        }
        Factor::Term(term) => index_term_address_references(index, scope, order, term),
    }
}

fn index_term_address_references(
    index: &mut SourceIndex,
    scope: &SourceScope,
    order: &mut usize,
    term: &Term,
) {
    index_factor_address_references(index, scope, order, &term.lhs);
    for (_, factor) in &term.rhs {
        index_factor_address_references(index, scope, order, factor);
    }
}

fn index_comprehension_qualifier_address_references(
    index: &mut SourceIndex,
    scope: &SourceScope,
    order: &mut usize,
    qualifier: &ComprehensionQualifier,
) {
    match qualifier {
        ComprehensionQualifier::Generator((pattern, expression)) => {
            index_pattern_address_references(index, scope, order, pattern);
            index_expression_address_references(index, scope, order, expression);
        }
        ComprehensionQualifier::Filter(expression) => {
            index_expression_address_references(index, scope, order, expression);
        }
        ComprehensionQualifier::Let(definition) => {
            index_expression_address_references(index, scope, order, &definition.expression);
        }
    }
}

fn index_pattern_address_references(
    index: &mut SourceIndex,
    scope: &SourceScope,
    order: &mut usize,
    pattern: &Pattern,
) {
    match pattern {
        Pattern::Expression(expression) => {
            index_expression_address_references(index, scope, order, expression);
        }
        Pattern::TupleStruct(tuple_struct) => {
            for element in &tuple_struct.patterns {
                index_pattern_address_references(index, scope, order, element);
            }
        }
        Pattern::Tuple(tuple) => {
            for element in &tuple.0 {
                index_pattern_address_references(index, scope, order, element);
            }
        }
        Pattern::Array(array) => {
            for pattern in &array.prefix {
                index_pattern_address_references(index, scope, order, pattern);
            }
            if let Some(spread) = &array.spread {
                if let Some(binding) = &spread.binding {
                    index_pattern_address_references(index, scope, order, binding);
                }
            }
            for pattern in &array.suffix {
                index_pattern_address_references(index, scope, order, pattern);
            }
        }
        Pattern::Wildcard => {}
    }
}

fn source_context_declaration(context: &mech_core::ContextDeclaration) -> SourceContextDeclaration {
    let base = match &context.base {
        ContextBase::ResourceUri(uri) => SourceContextBase::ResourceUri(uri.to_string()),
        ContextBase::Context(name) => SourceContextBase::Context(name.to_string()),
    };
    let capabilities = context
        .capabilities
        .iter()
        .map(|capability| {
            let scope = match &capability.scope {
                ContextCapabilityScope::Path(path) => {
                    SourceContextCapabilityScope::Path(path.to_string())
                }
                ContextCapabilityScope::Wildcard(_) => SourceContextCapabilityScope::Wildcard,
            };
            SourceContextCapability {
                operation: capability.operation.to_string(),
                scope,
            }
        })
        .collect();
    SourceContextDeclaration {
        name: context.name.to_string(),
        base,
        capabilities,
    }
}

fn declaration_range(mut tokens: Vec<Token>) -> Option<SourceRange> {
    Token::merge_tokens(&mut tokens).map(|token| token.src_range)
}
