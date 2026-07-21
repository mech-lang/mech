use mech_core::*;

use super::{
    ConfigExpr, ConfigFunction, ConfigItem, ConfigLet, ConfigProfileViolation, ConfigProgram,
    ExtractedConfigProgram,
};

pub(super) struct ConfigCompiler;

impl ConfigCompiler {
    pub(super) fn new() -> Self {
        Self
    }

    pub(super) fn compile(&self, extracted: &ExtractedConfigProgram) -> MResult<ConfigProgram> {
        let mut items = Vec::new();
        for (code, _) in &extracted.code {
            match code {
                MechCode::ActivationScope(_) => return Err(ConfigProfileViolation::error("ActivationScopeBytecodeUnsupported")),
                MechCode::Comment(_) => {}
                MechCode::Expression(expr) => {
                    items.push(ConfigItem::Expr(self.compile_expr(expr)?))
                }
                MechCode::FunctionDefine(function) => {
                    items.push(ConfigItem::Function(self.compile_function(function)?));
                }
                MechCode::Statement(statement) => items.push(self.compile_statement(statement)?),
                MechCode::Import(_) => {
                    return Err(ConfigProfileViolation::error(
                        "module imports are not allowed in Mech config",
                    ));
                }
                MechCode::FsmSpecification(_) | MechCode::FsmImplementation(_) => {
                    return Err(ConfigProfileViolation::error(
                        "state machines are not allowed in Mech config",
                    ));
                }
                MechCode::Error(_, _) => {
                    return Err(ConfigProfileViolation::error(
                        "parser error nodes are not allowed in Mech config",
                    ));
                }
            }
        }
        Ok(ConfigProgram { items })
    }

    fn compile_statement(&self, statement: &Statement) -> MResult<ConfigItem> {
        match statement {
            Statement::VariableDefine(def) => self.compile_let(def).map(ConfigItem::Let),
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
            Statement::ContextSend(_) => Err(ConfigProfileViolation::error(
                "context sends are not allowed in Mech config",
            )),
            #[cfg(feature = "invariant_define")]
            Statement::InvariantDefine(_) => Err(ConfigProfileViolation::error(
                "invariants are not allowed in Mech config",
            )),
            Statement::TupleDestructure(_) => Err(ConfigProfileViolation::error(
                "tuple destructuring is not allowed in Mech config",
            )),
            Statement::KindDefine(_) => Err(ConfigProfileViolation::error(
                "kind definitions are not allowed in Mech config v1",
            )),
            Statement::EnumDefine(_) => Err(ConfigProfileViolation::error(
                "enum definitions are not allowed in Mech config v1",
            )),
            Statement::SplitTable | Statement::FlattenTable => Err(ConfigProfileViolation::error(
                "table mutation transforms are not allowed in Mech config",
            )),
        }
    }

    fn compile_let(&self, def: &VariableDefine) -> MResult<ConfigLet> {
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
        Ok(ConfigLet {
            name: def.var.name.to_string(),
            expr: self.compile_expr(&def.expression)?,
        })
    }

    fn compile_function(&self, function: &FunctionDefine) -> MResult<ConfigFunction> {
        if !function.statements.is_empty()
            || function.match_arms.len() != 1
            || !matches!(function.match_arms[0].pattern, Pattern::Wildcard)
        {
            return Err(ConfigProfileViolation::error(
                "pattern-dispatched config helper functions are not supported in config v1",
            ));
        }
        Ok(ConfigFunction {
            name: function.name.to_string(),
            params: function
                .input
                .iter()
                .map(|arg| arg.name.to_string())
                .collect(),
            body: self.compile_expr(&function.match_arms[0].expression)?,
        })
    }

    fn compile_expr(&self, expr: &Expression) -> MResult<ConfigExpr> {
        match expr {
            Expression::Literal(literal) => self.compile_literal(literal),
            Expression::Var(var) => {
                if var.context.is_some() {
                    return Err(ConfigProfileViolation::error(
                        "addressed resources are not allowed in Mech config",
                    ));
                }
                Ok(ConfigExpr::Var(var.name.to_string()))
            }
            Expression::Structure(structure) => self.compile_structure(structure),
            Expression::FunctionCall(call) => {
                if call.args.iter().any(|(name, _)| name.is_some()) {
                    return Err(ConfigProfileViolation::error(
                        "named function-call arguments are not supported in Mech config v1",
                    ));
                }
                Ok(ConfigExpr::Call {
                    name: call.name.to_string(),
                    args: call
                        .args
                        .iter()
                        .map(|(_, arg)| self.compile_expr(arg))
                        .collect::<MResult<Vec<_>>>()?,
                })
            }
            Expression::Formula(factor) => self.compile_factor(factor),
            Expression::Match(_) => Err(ConfigProfileViolation::error(
                "match expressions are not supported in Mech config v1",
            )),
            Expression::Slice(slice) => {
                if slice.context.is_some() {
                    return Err(ConfigProfileViolation::error(
                        "addressed resources are not allowed in Mech config",
                    ));
                }
                Err(ConfigProfileViolation::error(
                    "slice expressions are not supported in Mech config v1",
                ))
            }
            Expression::Range(_) => Err(ConfigProfileViolation::error(
                "range expressions are not supported in Mech config v1",
            )),
            Expression::FsmPipe(_) => Err(ConfigProfileViolation::error(
                "state machines are not allowed in Mech config",
            )),
            Expression::SetComprehension(_) | Expression::MatrixComprehension(_) => Err(
                ConfigProfileViolation::error("comprehensions are not allowed in Mech config"),
            ),
        }
    }

    fn compile_literal(&self, literal: &Literal) -> MResult<ConfigExpr> {
        match literal {
            Literal::Atom(atom) => Ok(ConfigExpr::Atom(atom.name.to_string())),
            Literal::Boolean(token) => Ok(ConfigExpr::Bool(matches!(
                token.to_string().as_str(),
                "true" | "True" | "TRUE" | "✓"
            ))),
            Literal::Empty(_) => Ok(ConfigExpr::Null),
            Literal::Number(number) => self.compile_number(number),
            Literal::String(string) => Ok(ConfigExpr::String(string.to_string())),
            Literal::TypedLiteral((literal, _)) => self.compile_literal(literal),
            Literal::Kind(_) => Err(ConfigProfileViolation::error(
                "kind literals are not supported in Mech config values",
            )),
        }
    }

    fn compile_number(&self, number: &Number) -> MResult<ConfigExpr> {
        match number {
            Number::Real(real) => {
                let text = real.to_string();
                if matches!(real, RealNumber::Float(_) | RealNumber::Scientific(_)) {
                    text.parse::<f64>().map(ConfigExpr::Float).map_err(|_| {
                        ConfigProfileViolation::error(format!("invalid config float `{text}`"))
                    })
                } else {
                    let clean = text.trim_start_matches("0d");
                    clean.parse::<i64>().map(ConfigExpr::Integer).map_err(|_| {
                        ConfigProfileViolation::error(format!("invalid config integer `{text}`"))
                    })
                }
            }
            Number::Complex(_) => Err(ConfigProfileViolation::error(
                "complex numbers are not supported in Mech config",
            )),
        }
    }

    fn compile_structure(&self, structure: &Structure) -> MResult<ConfigExpr> {
        match structure {
            Structure::Empty => Ok(ConfigExpr::Map(Vec::new())),
            Structure::Record(record) => record
                .bindings
                .iter()
                .map(|binding| Ok((binding.name.to_string(), self.compile_expr(&binding.value)?)))
                .collect::<MResult<Vec<_>>>()
                .map(ConfigExpr::Map),
            Structure::Map(map) => map
                .elements
                .iter()
                .map(|mapping| {
                    Ok((
                        self.compile_map_key(&mapping.key)?,
                        self.compile_expr(&mapping.value)?,
                    ))
                })
                .collect::<MResult<Vec<_>>>()
                .map(ConfigExpr::Map),
            Structure::Set(set) => set
                .elements
                .iter()
                .map(|expr| self.compile_expr(expr))
                .collect::<MResult<Vec<_>>>()
                .map(ConfigExpr::List),
            Structure::Tuple(tuple) => tuple
                .elements
                .iter()
                .map(|expr| self.compile_expr(expr))
                .collect::<MResult<Vec<_>>>()
                .map(ConfigExpr::List),
            Structure::Matrix(matrix) => matrix
                .rows
                .iter()
                .flat_map(|row| row.columns.iter())
                .map(|column| self.compile_expr(&column.element))
                .collect::<MResult<Vec<_>>>()
                .map(ConfigExpr::List),
            Structure::Table(_) | Structure::TupleStruct(_) => Err(ConfigProfileViolation::error(
                "this structure is not allowed in Mech config v1",
            )),
        }
    }

    fn compile_map_key(&self, expr: &Expression) -> MResult<String> {
        match self.compile_expr(expr)? {
            ConfigExpr::String(key) | ConfigExpr::Atom(key) => Ok(key),
            ConfigExpr::Integer(key) => Ok(key.to_string()),
            _ => Err(ConfigProfileViolation::error(
                "config map keys must be literal strings, atoms, or integers",
            )),
        }
    }

    fn compile_factor(&self, factor: &Factor) -> MResult<ConfigExpr> {
        match factor {
            Factor::Expression(expr) => self.compile_expr(expr),
            Factor::Negate(inner) => Ok(ConfigExpr::Negate(Box::new(self.compile_factor(inner)?))),
            Factor::Not(inner) => Ok(ConfigExpr::Not(Box::new(self.compile_factor(inner)?))),
            Factor::Parenthetical(inner) => self.compile_factor(inner),
            Factor::Term(term) => self.compile_term(term),
            Factor::Transpose(_) => Err(ConfigProfileViolation::error(
                "transpose expressions are not supported in Mech config v1",
            )),
        }
    }

    fn compile_term(&self, term: &Term) -> MResult<ConfigExpr> {
        let mut expr = self.compile_factor(&term.lhs)?;
        for (op, rhs) in &term.rhs {
            let rhs = self.compile_factor(rhs)?;
            expr = match op {
                FormulaOperator::AddSub(AddSubOp::Add) => {
                    ConfigExpr::Add(Box::new(expr), Box::new(rhs))
                }
                FormulaOperator::AddSub(AddSubOp::Sub) => {
                    ConfigExpr::Sub(Box::new(expr), Box::new(rhs))
                }
                _ => {
                    return Err(ConfigProfileViolation::error(
                        "only add and subtract formulas are supported in Mech config v1",
                    ));
                }
            };
        }
        Ok(expr)
    }
}
