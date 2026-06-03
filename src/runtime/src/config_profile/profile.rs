use std::collections::{BTreeMap, BTreeSet};

use mech_core::*;

use super::{
    ConfigEffectfulFunctionNotAllowed, ConfigProfileOptions, ConfigProfileViolation,
    ConfigRecursionNotAllowed, ConfigUnknownFunction, ExtractedConfigProgram,
};

pub struct ConfigProfileValidator {
    #[allow(dead_code)]
    options: ConfigProfileOptions,
}

impl ConfigProfileValidator {
    pub fn new(options: ConfigProfileOptions) -> Self {
        Self { options }
    }

    pub fn validate(&self, program: &ExtractedConfigProgram) -> MResult<()> {
        let mut functions: BTreeMap<String, &FunctionDefine> = BTreeMap::new();
        let mut top_level_calls = BTreeSet::new();

        for (code, _) in &program.code {
            self.validate_code(code)?;
            self.collect_code_functions_and_calls(code, &mut functions, &mut top_level_calls)?;
        }

        let function_names: BTreeSet<String> = functions.keys().cloned().collect();
        let mut graph: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for (name, function) in &functions {
            let mut calls = BTreeSet::new();
            for statement in &function.statements {
                collect_statement_calls(statement, &mut calls);
            }
            for arm in &function.match_arms {
                collect_expression_calls(&arm.expression, &mut calls);
            }
            for call in &calls {
                validate_call_name(call, &function_names)?;
            }
            graph.insert(
                name.clone(),
                calls
                    .into_iter()
                    .filter(|call| function_names.contains(call))
                    .collect(),
            );
        }
        for call in &top_level_calls {
            validate_call_name(call, &function_names)?;
        }
        reject_recursion(&graph)
    }

    fn validate_code(&self, code: &MechCode) -> MResult<()> {
        match code {
            MechCode::Comment(_) => Ok(()),
            MechCode::Expression(expression) => self.validate_expression(expression),
            MechCode::FunctionDefine(function) => self.validate_function(function),
            MechCode::Statement(statement) => self.validate_statement(statement),
            MechCode::FsmSpecification(_) | MechCode::FsmImplementation(_) => Err(
                ConfigProfileViolation::error("state machines are not allowed in Mech config"),
            ),
            MechCode::Error(_, _) => Err(ConfigProfileViolation::error(
                "parser error nodes are not allowed in Mech config",
            )),
        }
    }

    fn validate_function(&self, function: &FunctionDefine) -> MResult<()> {
        for statement in &function.statements {
            self.validate_statement(statement)?;
        }
        for arm in &function.match_arms {
            self.validate_expression(&arm.expression)?;
        }
        Ok(())
    }

    fn validate_statement(&self, statement: &Statement) -> MResult<()> {
        match statement {
            Statement::VariableDefine(def) => {
                if def.mutable {
                    return Err(ConfigProfileViolation::error(
                        "mutable bindings are not allowed in Mech config",
                    ));
                }
                if def.var.context.is_some() {
                    return Err(ConfigProfileViolation::error(
                        "addressed resources are not allowed in Mech config",
                    ));
                }
                self.validate_expression(&def.expression)
            }
            Statement::KindDefine(_) => Err(ConfigProfileViolation::error(
                "kind definitions are not allowed in Mech config v1",
            )),
            Statement::EnumDefine(_) => Err(ConfigProfileViolation::error(
                "enum definitions are not allowed in Mech config v1",
            )),
            Statement::ImportDeclaration(_) => Err(ConfigProfileViolation::error(
                "imports are not allowed in Mech config",
            )),
            Statement::ExportDeclaration(_) => Err(ConfigProfileViolation::error(
                "exports are not allowed in Mech config",
            )),
            Statement::ContextDeclaration(_) => Err(ConfigProfileViolation::error(
                "context declarations are not allowed in Mech config",
            )),
            Statement::FsmDeclare(_) => Err(ConfigProfileViolation::error(
                "state machines are not allowed in Mech config",
            )),
            Statement::OpAssign(_) => Err(ConfigProfileViolation::error(
                "op assignment is not allowed in Mech config",
            )),
            Statement::VariableAssign(_) => Err(ConfigProfileViolation::error(
                "assignment is not allowed in Mech config",
            )),
            #[cfg(feature = "invariant_define")]
            Statement::InvariantDefine(_) => Err(ConfigProfileViolation::error(
                "invariants are not allowed in Mech config",
            )),
            Statement::TupleDestructure(_) => Err(ConfigProfileViolation::error(
                "tuple destructuring is not allowed in Mech config",
            )),
            Statement::SplitTable | Statement::FlattenTable => Err(ConfigProfileViolation::error(
                "table mutation transforms are not allowed in Mech config",
            )),
        }
    }

    fn validate_expression(&self, expression: &Expression) -> MResult<()> {
        match expression {
            Expression::Literal(_) => Ok(()),
            Expression::Var(var) => {
                if var.context.is_some() {
                    Err(ConfigProfileViolation::error(
                        "addressed resources are not allowed in Mech config",
                    ))
                } else {
                    Ok(())
                }
            }
            Expression::Structure(structure) => self.validate_structure(structure),
            Expression::FunctionCall(call) => {
                if is_effectful_function_name(&call.name.to_string()) {
                    return Err(ConfigEffectfulFunctionNotAllowed::error(format!(
                        "{} is not allowed in Mech config",
                        call.name.to_string()
                    )));
                }
                for (_, arg) in &call.args {
                    self.validate_expression(arg)?;
                }
                Ok(())
            }
            Expression::Formula(factor) => self.validate_factor(factor),
            Expression::Match(match_expr) => {
                self.validate_expression(&match_expr.source)?;
                for arm in &match_expr.arms {
                    if let Some(guard) = &arm.guard {
                        self.validate_expression(guard)?;
                    }
                    self.validate_expression(&arm.expression)?;
                }
                Ok(())
            }
            Expression::Slice(slice) => {
                if slice.context.is_some() {
                    return Err(ConfigProfileViolation::error(
                        "addressed resources are not allowed in Mech config",
                    ));
                }
                for sub in &slice.subscript {
                    self.validate_subscript(sub)?;
                }
                Ok(())
            }
            Expression::Range(range) => {
                self.validate_factor(&range.start)?;
                if let Some((_, inc)) = &range.increment {
                    self.validate_factor(inc)?;
                }
                self.validate_factor(&range.terminal)
            }
            Expression::FsmPipe(_) => Err(ConfigProfileViolation::error(
                "state machines are not allowed in Mech config",
            )),
            Expression::SetComprehension(_) | Expression::MatrixComprehension(_) => Err(
                ConfigProfileViolation::error("comprehensions are not allowed in Mech config"),
            ),
        }
    }

    fn validate_structure(&self, structure: &Structure) -> MResult<()> {
        match structure {
            Structure::Empty => Ok(()),
            Structure::Map(map) => map.elements.iter().try_for_each(|m| {
                self.validate_expression(&m.key)?;
                self.validate_expression(&m.value)
            }),
            Structure::Record(record) => record
                .bindings
                .iter()
                .try_for_each(|b| self.validate_expression(&b.value)),
            Structure::Set(set) => set
                .elements
                .iter()
                .try_for_each(|e| self.validate_expression(e)),
            Structure::Tuple(tuple) => tuple
                .elements
                .iter()
                .try_for_each(|e| self.validate_expression(e)),
            Structure::Matrix(matrix) => matrix.rows.iter().try_for_each(|r| {
                r.columns
                    .iter()
                    .try_for_each(|c| self.validate_expression(&c.element))
            }),
            Structure::Table(_) | Structure::TupleStruct(_) => Err(ConfigProfileViolation::error(
                "this structure is not allowed in Mech config v1",
            )),
        }
    }

    fn validate_factor(&self, factor: &Factor) -> MResult<()> {
        match factor {
            Factor::Expression(expression) => self.validate_expression(expression),
            Factor::Negate(inner)
            | Factor::Not(inner)
            | Factor::Parenthetical(inner)
            | Factor::Transpose(inner) => self.validate_factor(inner),
            Factor::Term(term) => {
                self.validate_factor(&term.lhs)?;
                term.rhs
                    .iter()
                    .try_for_each(|(_, rhs)| self.validate_factor(rhs))
            }
        }
    }

    fn validate_subscript(&self, subscript: &Subscript) -> MResult<()> {
        match subscript {
            Subscript::Formula(f) => self.validate_factor(f),
            Subscript::Range(r) => {
                self.validate_factor(&r.start)?;
                if let Some((_, inc)) = &r.increment {
                    self.validate_factor(inc)?;
                }
                self.validate_factor(&r.terminal)
            }
            Subscript::Brace(items) | Subscript::Bracket(items) => {
                items.iter().try_for_each(|s| self.validate_subscript(s))
            }
            _ => Ok(()),
        }
    }

    fn collect_code_functions_and_calls<'a>(
        &self,
        code: &'a MechCode,
        functions: &mut BTreeMap<String, &'a FunctionDefine>,
        top_calls: &mut BTreeSet<String>,
    ) -> MResult<()> {
        match code {
            MechCode::FunctionDefine(function) => {
                let name = function.name.to_string();
                if functions.insert(name.clone(), function).is_some() {
                    return Err(ConfigProfileViolation::error(format!(
                        "function `{name}` is defined more than once"
                    )));
                }
            }
            MechCode::Expression(expression) => collect_expression_calls(expression, top_calls),
            MechCode::Statement(Statement::VariableDefine(def)) => {
                collect_expression_calls(&def.expression, top_calls)
            }
            _ => {}
        }
        Ok(())
    }
}

fn collect_statement_calls(statement: &Statement, calls: &mut BTreeSet<String>) {
    if let Statement::VariableDefine(def) = statement {
        collect_expression_calls(&def.expression, calls);
    }
}

fn collect_expression_calls(expression: &Expression, calls: &mut BTreeSet<String>) {
    match expression {
        Expression::FunctionCall(call) => {
            calls.insert(call.name.to_string());
            for (_, arg) in &call.args {
                collect_expression_calls(arg, calls);
            }
        }
        Expression::Formula(f) => collect_factor_calls(f, calls),
        Expression::Structure(s) => match s {
            Structure::Map(m) => {
                for e in &m.elements {
                    collect_expression_calls(&e.key, calls);
                    collect_expression_calls(&e.value, calls);
                }
            }
            Structure::Record(r) => {
                for b in &r.bindings {
                    collect_expression_calls(&b.value, calls);
                }
            }
            Structure::Set(s) => {
                for e in &s.elements {
                    collect_expression_calls(e, calls);
                }
            }
            Structure::Tuple(t) => {
                for e in &t.elements {
                    collect_expression_calls(e, calls);
                }
            }
            Structure::Matrix(m) => {
                for r in &m.rows {
                    for c in &r.columns {
                        collect_expression_calls(&c.element, calls);
                    }
                }
            }
            _ => {}
        },
        Expression::Match(m) => {
            collect_expression_calls(&m.source, calls);
            for a in &m.arms {
                if let Some(g) = &a.guard {
                    collect_expression_calls(g, calls);
                }
                collect_expression_calls(&a.expression, calls);
            }
        }
        Expression::Range(r) => {
            collect_factor_calls(&r.start, calls);
            if let Some((_, inc)) = &r.increment {
                collect_factor_calls(inc, calls);
            }
            collect_factor_calls(&r.terminal, calls);
        }
        _ => {}
    }
}

fn collect_factor_calls(factor: &Factor, calls: &mut BTreeSet<String>) {
    match factor {
        Factor::Expression(e) => collect_expression_calls(e, calls),
        Factor::Negate(f) | Factor::Not(f) | Factor::Parenthetical(f) | Factor::Transpose(f) => {
            collect_factor_calls(f, calls)
        }
        Factor::Term(t) => {
            collect_factor_calls(&t.lhs, calls);
            for (_, rhs) in &t.rhs {
                collect_factor_calls(rhs, calls);
            }
        }
    }
}

fn is_effectful_function_name(name: &str) -> bool {
    matches!(
        name,
        "read"
            | "write"
            | "open"
            | "fetch"
            | "http"
            | "spawn"
            | "sleep"
            | "now"
            | "time"
            | "random"
            | "rand"
    )
}

fn is_pure_whitelist(name: &str) -> bool {
    matches!(name, "join-path" | "path-join" | "str" | "string")
}

fn validate_call_name(name: &str, user_functions: &BTreeSet<String>) -> MResult<()> {
    if user_functions.contains(name) || is_pure_whitelist(name) {
        return Ok(());
    }
    if is_effectful_function_name(name) {
        return Err(ConfigEffectfulFunctionNotAllowed::error(format!(
            "{name} is not allowed in Mech config"
        )));
    }
    Err(ConfigUnknownFunction::error(format!(
        "{name} is not a known pure Mech config function"
    )))
}

fn reject_recursion(graph: &BTreeMap<String, BTreeSet<String>>) -> MResult<()> {
    fn visit(
        node: &str,
        graph: &BTreeMap<String, BTreeSet<String>>,
        visiting: &mut BTreeSet<String>,
        visited: &mut BTreeSet<String>,
    ) -> MResult<()> {
        if visiting.contains(node) {
            return Err(ConfigRecursionNotAllowed::error(format!(
                "function `{node}` is recursive"
            )));
        }
        if visited.contains(node) {
            return Ok(());
        }
        visiting.insert(node.to_string());
        if let Some(edges) = graph.get(node) {
            for edge in edges {
                visit(edge, graph, visiting, visited)?;
            }
        }
        visiting.remove(node);
        visited.insert(node.to_string());
        Ok(())
    }
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for node in graph.keys() {
        visit(node, graph, &mut visiting, &mut visited)?;
    }
    Ok(())
}
