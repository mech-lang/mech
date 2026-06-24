use mech_core::{
    ComprehensionQualifier, ContextBase, ContextCapabilityScope, Expression, Factor, FsmArm,
    FsmImplementation, FunctionDefine, MResult, MechCode, MechError, Pattern, Program,
    RangeExpression, SectionElement, SourceRange, Statement, Structure, Subscript, Term, Token,
    Transition,
};

use super::{
    classify_import_specifier, module_import_declarations, AddressTargetNameConflict,
    SourceAddressReference, SourceContextBase, SourceContextCapability,
    SourceContextCapabilityScope, SourceContextDeclaration, SourceExportDeclaration,
    SourceImportDeclaration,
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
                            index_mech_code_address_references(
                                &mut index,
                                &SourceScope::Program,
                                &mut order,
                                code,
                            );
                        }
                    }
                    SectionElement::FencedMechCode(fenced) => {
                        let scope = if fenced.config.namespace_str.is_empty() {
                            SourceScope::Program
                        } else {
                            let interpreter = SourceInterpreterId {
                                namespace: fenced.config.namespace,
                                namespace_str: fenced.config.namespace_str.clone(),
                            };
                            fenced_interpreters
                                .entry(interpreter.namespace_str.clone())
                                .or_insert_with(|| {
                                    index.address_target_interpreters.push(interpreter.clone());
                                    let scope = SourceScope::Interpreter(interpreter);
                                    index.push_scope(scope.clone());
                                    scope
                                })
                                .clone()
                        };

                        for (code, _) in &fenced.code {
                            index_mech_code_address_references(
                                &mut index, &scope, &mut order, code,
                            );
                        }
                    }
                    _ => {}
                }
            }
        }

        index
    }

    pub fn validate_address_targets(&self) -> MResult<()> {
        let mut targets: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();

        for interpreter in &self.address_target_interpreters {
            if let Some(first_kind) =
                targets.insert(interpreter.namespace_str.clone(), "interpreter".to_string())
            {
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
            if let Some(first_kind) =
                targets.insert(context.declaration.name.clone(), "context".to_string())
            {
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

fn index_mech_code_address_references(
    index: &mut SourceIndex,
    scope: &SourceScope,
    order: &mut usize,
    code: &MechCode,
) {
    match code {
        MechCode::Import(import) => {
            for declaration in module_import_declarations(import) {
                index.push_import(
                    scope.clone(),
                    *order,
                    declaration_range(import.tokens()),
                    declaration,
                );
                *order += 1;
            }
        }
        MechCode::Statement(statement) => {
            match statement {
                Statement::ImportDeclaration(import) => {
                    index.push_import(
                        scope.clone(),
                        *order,
                        declaration_range(import.tokens()),
                        classify_import_specifier(import.specifier.to_string()),
                    );
                    *order += 1;
                }
                Statement::ExportDeclaration(export) => {
                    index.push_export(
                        scope.clone(),
                        *order,
                        declaration_range(export.tokens()),
                        SourceExportDeclaration {
                            name: export.name.to_string(),
                        },
                    );
                    *order += 1;
                }
                Statement::ContextDeclaration(context) => {
                    index.push_context(
                        scope.clone(),
                        *order,
                        declaration_range(context.tokens()),
                        source_context_declaration(context),
                    );
                    *order += 1;
                }
                _ => {}
            }

            index_statement_address_references(index, scope, order, statement);
        }
        MechCode::Expression(expression) => {
            index_expression_address_references(index, scope, order, expression);
        }
        MechCode::FunctionDefine(function) => {
            index_function_define_address_references(index, scope, order, function);
        }
        MechCode::FsmImplementation(fsm) => {
            index_fsm_implementation_address_references(index, scope, order, fsm);
        }
        MechCode::FsmSpecification(_) | MechCode::Comment(_) | MechCode::Error(_, _) => {}
    }
}

fn index_function_define_address_references(
    index: &mut SourceIndex,
    scope: &SourceScope,
    order: &mut usize,
    function: &FunctionDefine,
) {
    for statement in &function.statements {
        index_statement_address_references(index, scope, order, statement);
    }

    for arm in &function.match_arms {
        index_pattern_address_references(index, scope, order, &arm.pattern);
        index_expression_address_references(index, scope, order, &arm.expression);
    }
}

fn index_fsm_implementation_address_references(
    index: &mut SourceIndex,
    scope: &SourceScope,
    order: &mut usize,
    fsm: &FsmImplementation,
) {
    index_pattern_address_references(index, scope, order, &fsm.start);

    for arm in &fsm.arms {
        match arm {
            FsmArm::Guard(pattern, guards) => {
                index_pattern_address_references(index, scope, order, pattern);
                for guard in guards {
                    index_pattern_address_references(index, scope, order, &guard.condition);
                    for transition in &guard.transitions {
                        index_transition_address_references(index, scope, order, transition);
                    }
                }
            }
            FsmArm::Transition(pattern, transitions) => {
                index_pattern_address_references(index, scope, order, pattern);
                for transition in transitions {
                    index_transition_address_references(index, scope, order, transition);
                }
            }
            FsmArm::Comment(_) => {}
        }
    }
}

fn index_transition_address_references(
    index: &mut SourceIndex,
    scope: &SourceScope,
    order: &mut usize,
    transition: &Transition,
) {
    match transition {
        Transition::Async(pattern) | Transition::Next(pattern) | Transition::Output(pattern) => {
            index_pattern_address_references(index, scope, order, pattern);
        }
        Transition::Statement(statement) => {
            index_statement_address_references(index, scope, order, statement);
        }
        Transition::CodeBlock(code_items) => {
            for (code, _) in code_items {
                index_mech_code_address_references(index, scope, order, code);
            }
        }
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
        Statement::ContextSend(send) => {
            index_expression_address_references(index, scope, order, &send.expression)
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
        Statement::InvariantDefine(invariant) => {
            index_expression_address_references(index, scope, order, &invariant.expression);
        }
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
                index_pattern_address_references(index, scope, order, &arm.pattern);
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
            if let Some(args) = &pipe.start.args {
                for (_, expression) in args {
                    index_expression_address_references(index, scope, order, expression);
                }
            }
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
                            index_mech_code_address_references(index, scope, order, code);
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
        Subscript::All | Subscript::Dot(_) | Subscript::DotInt(_) | Subscript::Swizzle(_) => {}
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

#[cfg(test)]
mod tests {
    use super::*;

    fn ident(name: &str) -> mech_core::Identifier {
        mech_core::Identifier {
            name: Token::new(
                mech_core::TokenKind::Identifier,
                SourceRange::default(),
                name.chars().collect(),
            ),
        }
    }

    fn addressed_var(target: &str, name: &str) -> Expression {
        Expression::Var(mech_core::Var {
            name: ident(name),
            context: Some(ident(target)),
            kind: None,
        })
    }

    fn program_with_code(code: MechCode) -> Program {
        Program {
            title: None,
            body: mech_core::Body {
                sections: vec![mech_core::Section {
                    subtitle: None,
                    elements: vec![SectionElement::MechCode(vec![(code, None)])],
                }],
            },
        }
    }

    #[test]
    fn source_index_treats_unqualified_fenced_mech_as_program_scope() {
        let tree =
            mech_syntax::parser::parse("```mech\n+> @env := cli/env\nhome := @env/HOME\n```\n")
                .unwrap();

        let index = SourceIndex::from_program(&tree);

        assert_eq!(index.imports.len(), 1);
        assert_eq!(index.imports[0].occurrence.scope, SourceScope::Program);
        assert!(index.interpreter_scopes().is_empty());
    }

    #[test]
    fn source_index_records_function_body_address_references() {
        let tree = program_with_code(MechCode::FunctionDefine(FunctionDefine {
            name: ident("lookup"),
            input: vec![],
            output: vec![],
            statements: vec![Statement::VariableDefine(mech_core::VariableDefine {
                mutable: false,
                var: mech_core::Var {
                    name: ident("result"),
                    context: None,
                    kind: None,
                },
                expression: addressed_var("missing", "HOME"),
            })],
            match_arms: vec![],
        }));

        let index = SourceIndex::from_program(&tree);
        assert!(
            index
                .program_address_references()
                .iter()
                .any(|reference| reference.target == "missing" && reference.name == "HOME"),
            "expected function body addressed read to be indexed"
        );
    }

    #[test]
    fn source_index_records_function_pattern_address_references() {
        let tree = program_with_code(MechCode::FunctionDefine(FunctionDefine {
            name: ident("pick"),
            input: vec![],
            output: vec![],
            statements: vec![],
            match_arms: vec![mech_core::FunctionMatchArm {
                pattern: Pattern::Expression(addressed_var("env", "SECRET")),
                expression: Expression::Literal(mech_core::Literal::Empty(Token::new(
                    mech_core::TokenKind::Empty,
                    SourceRange::default(),
                    vec!['_'],
                ))),
            }],
        }));

        let index = SourceIndex::from_program(&tree);
        assert!(
            index
                .program_address_references()
                .iter()
                .any(|reference| reference.target == "env" && reference.name == "SECRET"),
            "expected function pattern addressed read to be indexed"
        );
    }

    #[test]
    fn source_index_records_match_pattern_address_references() {
        let tree = mech_syntax::parser::parse(
            r#"
x := "secret"
result := x?
  | @env/SECRET => "matched"
  | * => "missed".
"#,
        )
        .unwrap();

        let index = SourceIndex::from_program(&tree);
        assert!(
            index
                .program_address_references()
                .iter()
                .any(|reference| reference.target == "env" && reference.name == "SECRET"),
            "expected match pattern addressed read to be indexed"
        );
    }

    #[test]
    fn source_index_records_fsm_arm_selector_address_references() {
        let tree = mech_syntax::parser::parse(
            r#"
#Pick(x<string>) => <string>
  ├ :PickState(x<string>)
  └ :Done(out<string>).

#Pick(x) -> :PickState("not-the-secret")
  :PickState(@env/STATE) -> :Done("matched")
  :PickState("not-the-secret") -> :Done("missed")
  :Done(out) => out.
"#,
        )
        .unwrap();

        let index = SourceIndex::from_program(&tree);
        assert!(
            index
                .program_address_references()
                .iter()
                .any(|reference| reference.target == "env" && reference.name == "STATE"),
            "expected FSM arm selector addressed read to be indexed"
        );
    }

    #[test]
    fn source_index_records_fsm_pipe_start_arg_address_references() {
        let tree = program_with_code(MechCode::Expression(Expression::FsmPipe(
            mech_core::FsmPipe {
                start: mech_core::FsmInstance {
                    name: ident("Pick"),
                    args: Some(vec![(None, addressed_var("missing", "x"))]),
                },
                transitions: vec![],
            },
        )));

        let index = SourceIndex::from_program(&tree);
        assert!(
            index
                .program_address_references()
                .iter()
                .any(|reference| reference.target == "missing" && reference.name == "x"),
            "expected FSM pipe start arg addressed read to be indexed"
        );
    }

    #[cfg(feature = "invariant_define")]
    #[test]
    fn source_index_records_invariant_address_references() {
        let tree = program_with_code(MechCode::Statement(Statement::InvariantDefine(
            mech_core::InvariantDefine {
                name: ident("check"),
                expression: addressed_var("missing", "x"),
            },
        )));

        let index = SourceIndex::from_program(&tree);
        assert!(
            index
                .program_address_references()
                .iter()
                .any(|reference| reference.target == "missing" && reference.name == "x"),
            "expected invariant addressed read to be indexed"
        );
    }
}
