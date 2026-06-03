use std::collections::BTreeMap;

use mech_core::*;

use super::{
    ConfigEvaluationBudgetExceeded, ConfigProfileOptions, ConfigProfileViolation,
    ExtractedConfigProgram, MissingConfigBinding,
};

#[derive(Clone, Debug, PartialEq)]
pub enum ConfigValue {
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
    List(Vec<ConfigValue>),
    Map(BTreeMap<String, ConfigValue>),
}

pub struct ConfigEvaluator {
    options: ConfigProfileOptions,
    steps: usize,
    collection_items: usize,
    string_bytes: usize,
    bindings: BTreeMap<String, ConfigValue>,
    functions: BTreeMap<String, FunctionDefine>,
}

impl ConfigEvaluator {
    pub fn new(options: ConfigProfileOptions) -> Self {
        Self {
            options,
            steps: 0,
            collection_items: 0,
            string_bytes: 0,
            bindings: BTreeMap::new(),
            functions: BTreeMap::new(),
        }
    }

    pub fn evaluate(mut self, program: &ExtractedConfigProgram) -> MResult<ConfigValue> {
        for (code, _) in &program.code {
            if let MechCode::FunctionDefine(function) = code {
                self.functions
                    .insert(function.name.to_string(), function.clone());
            }
        }
        for (code, _) in &program.code {
            match code {
                MechCode::Comment(_) | MechCode::FunctionDefine(_) => {}
                MechCode::Expression(expression) => {
                    let _ = self.eval_expression(expression, 0)?;
                }
                MechCode::Statement(Statement::VariableDefine(def)) => {
                    let value = self.eval_expression(&def.expression, 0)?;
                    self.bindings.insert(def.var.name.to_string(), value);
                }
                _ => {
                    return Err(ConfigProfileViolation::error(
                        "validated Mech config contains unsupported code",
                    ));
                }
            }
        }
        self.bindings
            .remove("config")
            .ok_or_else(|| MissingConfigBinding::error("final binding `config` is required"))
    }

    fn step(&mut self) -> MResult<()> {
        self.steps += 1;
        if self.steps > self.options.max_eval_steps {
            return Err(ConfigEvaluationBudgetExceeded::error(
                "maximum evaluation steps exceeded",
            ));
        }
        Ok(())
    }

    fn count_collection_item(&mut self) -> MResult<()> {
        self.collection_items += 1;
        if self.collection_items > self.options.max_collection_items {
            return Err(ConfigEvaluationBudgetExceeded::error(
                "maximum collection items exceeded",
            ));
        }
        Ok(())
    }

    fn count_string(&mut self, s: &str) -> MResult<()> {
        self.string_bytes += s.len();
        if self.string_bytes > self.options.max_string_bytes {
            return Err(ConfigEvaluationBudgetExceeded::error(
                "maximum string bytes exceeded",
            ));
        }
        Ok(())
    }

    fn eval_expression(&mut self, expression: &Expression, depth: usize) -> MResult<ConfigValue> {
        self.step()?;
        match expression {
            Expression::Literal(literal) => self.eval_literal(literal),
            Expression::Var(var) => self
                .bindings
                .get(&var.name.to_string())
                .cloned()
                .ok_or_else(|| {
                    ConfigProfileViolation::error(format!(
                        "unknown config binding `{}`",
                        var.name.to_string()
                    ))
                }),
            Expression::Structure(structure) => self.eval_structure(structure, depth),
            Expression::Formula(factor) => self.eval_factor(factor, depth),
            Expression::FunctionCall(call) => self.eval_function_call(call, depth),
            _ => Err(ConfigProfileViolation::error(
                "expression is not supported by Mech config evaluator v1",
            )),
        }
    }

    fn eval_literal(&mut self, literal: &Literal) -> MResult<ConfigValue> {
        match literal {
            Literal::Boolean(t) => Ok(ConfigValue::Bool(matches!(
                t.to_string().as_str(),
                "true" | "True" | "TRUE"
            ))),
            Literal::Empty(_) => Ok(ConfigValue::Null),
            Literal::String(s) => {
                let out = s.to_string();
                self.count_string(&out)?;
                Ok(ConfigValue::String(out))
            }
            Literal::Number(n) => self.eval_number(n),
            Literal::Atom(atom) => {
                let out = atom.name.to_string();
                self.count_string(&out)?;
                Ok(ConfigValue::String(out))
            }
            Literal::TypedLiteral((literal, _)) => self.eval_literal(literal),
            Literal::Kind(_) => Err(ConfigProfileViolation::error(
                "kind literals are not supported in Mech config values",
            )),
        }
    }

    fn eval_number(&self, number: &Number) -> MResult<ConfigValue> {
        match number {
            Number::Real(real) => {
                let text = real.to_string();
                if matches!(real, RealNumber::Float(_) | RealNumber::Scientific(_)) {
                    text.parse::<f64>().map(ConfigValue::Float).map_err(|_| {
                        ConfigProfileViolation::error(format!("invalid config float `{text}`"))
                    })
                } else {
                    let clean = text.trim_start_matches("0d");
                    clean.parse::<i64>().map(ConfigValue::Integer).map_err(|_| {
                        ConfigProfileViolation::error(format!("invalid config integer `{text}`"))
                    })
                }
            }
            Number::Complex(_) => Err(ConfigProfileViolation::error(
                "complex numbers are not supported in Mech config",
            )),
        }
    }

    fn eval_structure(&mut self, structure: &Structure, depth: usize) -> MResult<ConfigValue> {
        match structure {
            Structure::Empty => Ok(ConfigValue::Map(BTreeMap::new())),
            Structure::Record(record) => {
                let mut out = BTreeMap::new();
                for binding in &record.bindings {
                    self.count_collection_item()?;
                    out.insert(
                        binding.name.to_string(),
                        self.eval_expression(&binding.value, depth)?,
                    );
                }
                Ok(ConfigValue::Map(out))
            }
            Structure::Map(map) => {
                let mut out = BTreeMap::new();
                for mapping in &map.elements {
                    self.count_collection_item()?;
                    let key = self.eval_expression(&mapping.key, depth)?;
                    let key = match key {
                        ConfigValue::String(s) => s,
                        ConfigValue::Integer(i) => i.to_string(),
                        _ => {
                            return Err(ConfigProfileViolation::error(
                                "config map keys must be strings, atoms, identifiers, or integers",
                            ));
                        }
                    };
                    out.insert(key, self.eval_expression(&mapping.value, depth)?);
                }
                Ok(ConfigValue::Map(out))
            }
            Structure::Matrix(matrix) => {
                let mut out = Vec::new();
                for row in &matrix.rows {
                    for column in &row.columns {
                        self.count_collection_item()?;
                        out.push(self.eval_expression(&column.element, depth)?);
                    }
                }
                Ok(ConfigValue::List(out))
            }
            Structure::Set(set) => {
                let mut out = Vec::new();
                for element in &set.elements {
                    self.count_collection_item()?;
                    out.push(self.eval_expression(element, depth)?);
                }
                Ok(ConfigValue::List(out))
            }
            Structure::Tuple(tuple) => {
                let mut out = Vec::new();
                for element in &tuple.elements {
                    self.count_collection_item()?;
                    out.push(self.eval_expression(element, depth)?);
                }
                Ok(ConfigValue::List(out))
            }
            _ => Err(ConfigProfileViolation::error(
                "structure is not supported by Mech config evaluator v1",
            )),
        }
    }

    fn eval_factor(&mut self, factor: &Factor, depth: usize) -> MResult<ConfigValue> {
        match factor {
            Factor::Expression(expression) => self.eval_expression(expression, depth),
            Factor::Parenthetical(inner) => self.eval_factor(inner, depth),
            Factor::Negate(inner) => match self.eval_factor(inner, depth)? {
                ConfigValue::Integer(i) => Ok(ConfigValue::Integer(-i)),
                ConfigValue::Float(f) => Ok(ConfigValue::Float(-f)),
                _ => Err(ConfigProfileViolation::error(
                    "cannot negate non-number in Mech config",
                )),
            },
            Factor::Not(inner) => match self.eval_factor(inner, depth)? {
                ConfigValue::Bool(b) => Ok(ConfigValue::Bool(!b)),
                _ => Err(ConfigProfileViolation::error(
                    "cannot apply not to non-bool in Mech config",
                )),
            },
            Factor::Term(term) => self.eval_term(term, depth),
            Factor::Transpose(inner) => self.eval_factor(inner, depth),
        }
    }

    fn eval_term(&mut self, term: &Term, depth: usize) -> MResult<ConfigValue> {
        let mut value = self.eval_factor(&term.lhs, depth)?;
        for (op, rhs) in &term.rhs {
            let rhs = self.eval_factor(rhs, depth)?;
            value = match (op, value, rhs) {
                (
                    FormulaOperator::AddSub(AddSubOp::Add),
                    ConfigValue::Integer(a),
                    ConfigValue::Integer(b),
                ) => ConfigValue::Integer(a + b),
                (
                    FormulaOperator::AddSub(AddSubOp::Add),
                    ConfigValue::String(a),
                    ConfigValue::String(b),
                ) => {
                    let s = format!("{a}{b}");
                    self.count_string(&s)?;
                    ConfigValue::String(s)
                }
                (
                    FormulaOperator::AddSub(AddSubOp::Sub),
                    ConfigValue::Integer(a),
                    ConfigValue::Integer(b),
                ) => ConfigValue::Integer(a - b),
                _ => {
                    return Err(ConfigProfileViolation::error(
                        "only simple pure arithmetic/string formulas are supported in Mech config v1",
                    ));
                }
            };
        }
        Ok(value)
    }

    fn eval_function_call(&mut self, call: &FunctionCall, depth: usize) -> MResult<ConfigValue> {
        if depth >= self.options.max_function_depth {
            return Err(ConfigEvaluationBudgetExceeded::error(
                "maximum function depth exceeded",
            ));
        }
        let name = call.name.to_string();
        if matches!(name.as_str(), "join-path" | "path-join") {
            let mut parts = Vec::new();
            for (_, arg) in &call.args {
                match self.eval_expression(arg, depth + 1)? {
                    ConfigValue::String(s) => parts.push(s),
                    _ => {
                        return Err(ConfigProfileViolation::error(
                            "path join arguments must be strings",
                        ));
                    }
                }
            }
            let out = parts.join("/");
            self.count_string(&out)?;
            return Ok(ConfigValue::String(out));
        }
        if matches!(name.as_str(), "str" | "string") {
            if call.args.len() != 1 {
                return Err(ConfigProfileViolation::error("string expects one argument"));
            }
            let value = self.eval_expression(&call.args[0].1, depth + 1)?;
            let out = match value {
                ConfigValue::String(s) => s,
                ConfigValue::Integer(i) => i.to_string(),
                ConfigValue::Bool(b) => b.to_string(),
                ConfigValue::Float(f) => f.to_string(),
                _ => {
                    return Err(ConfigProfileViolation::error(
                        "cannot convert config value to string",
                    ));
                }
            };
            self.count_string(&out)?;
            return Ok(ConfigValue::String(out));
        }
        let function = self.functions.get(&name).cloned().ok_or_else(|| {
            ConfigProfileViolation::error(format!("unknown config function `{name}`"))
        })?;
        let saved = self.bindings.clone();
        for (idx, arg) in function.input.iter().enumerate() {
            let Some((_, expr)) = call.args.get(idx) else {
                return Err(ConfigProfileViolation::error(format!(
                    "function `{name}` missing argument `{}`",
                    arg.name.to_string()
                )));
            };
            let value = self.eval_expression(expr, depth + 1)?;
            self.bindings.insert(arg.name.to_string(), value);
        }
        for statement in &function.statements {
            if let Statement::VariableDefine(def) = statement {
                let value = self.eval_expression(&def.expression, depth + 1)?;
                self.bindings.insert(def.var.name.to_string(), value);
            }
        }
        let result = if let Some(arm) = function.match_arms.first() {
            self.eval_expression(&arm.expression, depth + 1)?
        } else if let Some(output) = function.output.first() {
            self.bindings
                .get(&output.name.to_string())
                .cloned()
                .ok_or_else(|| {
                    ConfigProfileViolation::error(format!(
                        "function `{name}` did not produce `{}`",
                        output.name.to_string()
                    ))
                })?
        } else {
            ConfigValue::Null
        };
        self.bindings = saved;
        Ok(result)
    }
}
