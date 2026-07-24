use std::collections::BTreeMap;

use mech_core::MResult;

use super::{
    ConfigEvaluationBudgetExceeded, ConfigExpr, ConfigFunction, ConfigItem, ConfigProfileOptions,
    ConfigProfileViolation, ConfigProgram, MissingConfigBinding,
};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

pub(super) struct ConfigEvaluator {
    options: ConfigProfileOptions,
    steps: usize,
    collection_items: usize,
    string_bytes: usize,
    bindings: BTreeMap<String, ConfigValue>,
    functions: BTreeMap<String, ConfigFunction>,
}

impl ConfigEvaluator {
    pub(super) fn new(options: ConfigProfileOptions) -> Self {
        Self {
            options,
            steps: 0,
            collection_items: 0,
            string_bytes: 0,
            bindings: BTreeMap::new(),
            functions: BTreeMap::new(),
        }
    }

    pub(super) fn evaluate(mut self, program: &ConfigProgram) -> MResult<ConfigValue> {
        for item in &program.items {
            if let ConfigItem::Function(function) = item {
                self.functions
                    .insert(function.name.clone(), function.clone());
            }
        }

        for item in &program.items {
            match item {
                ConfigItem::Function(_) => {}
                ConfigItem::Expr(expr) => {
                    let _ = self.eval_expr(expr, 0)?;
                }
                ConfigItem::Let(binding) => {
                    let value = self.eval_expr(&binding.expr, 0)?;
                    self.bindings.insert(binding.name.clone(), value);
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

    fn eval_expr(&mut self, expr: &ConfigExpr, depth: usize) -> MResult<ConfigValue> {
        self.step()?;
        match expr {
            ConfigExpr::Null => Ok(ConfigValue::Null),
            ConfigExpr::Bool(value) => Ok(ConfigValue::Bool(*value)),
            ConfigExpr::Integer(value) => Ok(ConfigValue::Integer(*value)),
            ConfigExpr::Float(value) => Ok(ConfigValue::Float(*value)),
            ConfigExpr::String(value) => {
                self.count_string(value)?;
                Ok(ConfigValue::String(value.clone()))
            }
            ConfigExpr::Atom(value) => {
                self.count_string(value)?;
                Ok(ConfigValue::String(value.clone()))
            }
            ConfigExpr::List(items) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    self.count_collection_item()?;
                    out.push(self.eval_expr(item, depth)?);
                }
                Ok(ConfigValue::List(out))
            }
            ConfigExpr::Map(entries) => {
                let mut out = BTreeMap::new();
                for (key, value) in entries {
                    self.count_collection_item()?;
                    self.count_string(key)?;
                    out.insert(key.clone(), self.eval_expr(value, depth)?);
                }
                Ok(ConfigValue::Map(out))
            }
            ConfigExpr::Var(name) => self.bindings.get(name).cloned().ok_or_else(|| {
                ConfigProfileViolation::error(format!("unknown config binding `{name}`"))
            }),
            ConfigExpr::Call { name, args } => self.eval_call(name, args, depth),
            ConfigExpr::Add(lhs, rhs) => {
                let lhs = self.eval_expr(lhs, depth)?;
                let rhs = self.eval_expr(rhs, depth)?;
                self.add(lhs, rhs)
            }
            ConfigExpr::Sub(lhs, rhs) => {
                let lhs = self.eval_expr(lhs, depth)?;
                let rhs = self.eval_expr(rhs, depth)?;
                self.sub(lhs, rhs)
            }
            ConfigExpr::Negate(inner) => match self.eval_expr(inner, depth)? {
                ConfigValue::Integer(value) => Ok(ConfigValue::Integer(-value)),
                ConfigValue::Float(value) => Ok(ConfigValue::Float(-value)),
                _ => Err(ConfigProfileViolation::error(
                    "cannot negate non-number in Mech config",
                )),
            },
            ConfigExpr::Not(inner) => match self.eval_expr(inner, depth)? {
                ConfigValue::Bool(value) => Ok(ConfigValue::Bool(!value)),
                _ => Err(ConfigProfileViolation::error(
                    "cannot apply not to non-bool in Mech config",
                )),
            },
        }
    }

    fn add(&mut self, lhs: ConfigValue, rhs: ConfigValue) -> MResult<ConfigValue> {
        match (lhs, rhs) {
            (ConfigValue::Integer(lhs), ConfigValue::Integer(rhs)) => {
                Ok(ConfigValue::Integer(lhs + rhs))
            }
            (ConfigValue::Float(lhs), ConfigValue::Float(rhs)) => Ok(ConfigValue::Float(lhs + rhs)),
            (ConfigValue::Integer(lhs), ConfigValue::Float(rhs)) => {
                Ok(ConfigValue::Float(lhs as f64 + rhs))
            }
            (ConfigValue::Float(lhs), ConfigValue::Integer(rhs)) => {
                Ok(ConfigValue::Float(lhs + rhs as f64))
            }
            (ConfigValue::String(lhs), ConfigValue::String(rhs)) => {
                let out = format!("{lhs}{rhs}");
                self.count_string(&out)?;
                Ok(ConfigValue::String(out))
            }
            _ => Err(ConfigProfileViolation::error(
                "cannot add these config value types",
            )),
        }
    }

    fn sub(&self, lhs: ConfigValue, rhs: ConfigValue) -> MResult<ConfigValue> {
        match (lhs, rhs) {
            (ConfigValue::Integer(lhs), ConfigValue::Integer(rhs)) => {
                Ok(ConfigValue::Integer(lhs - rhs))
            }
            (ConfigValue::Float(lhs), ConfigValue::Float(rhs)) => Ok(ConfigValue::Float(lhs - rhs)),
            (ConfigValue::Integer(lhs), ConfigValue::Float(rhs)) => {
                Ok(ConfigValue::Float(lhs as f64 - rhs))
            }
            (ConfigValue::Float(lhs), ConfigValue::Integer(rhs)) => {
                Ok(ConfigValue::Float(lhs - rhs as f64))
            }
            _ => Err(ConfigProfileViolation::error(
                "cannot subtract these config value types",
            )),
        }
    }

    fn eval_call(&mut self, name: &str, args: &[ConfigExpr], depth: usize) -> MResult<ConfigValue> {
        if depth >= self.options.max_function_depth {
            return Err(ConfigEvaluationBudgetExceeded::error(
                "maximum function depth exceeded",
            ));
        }

        match name {
            "join-path" | "path-join" => self.eval_join_path(args, depth),
            "str" | "string" => self.eval_string(args, depth),
            _ => self.eval_user_function(name, args, depth),
        }
    }

    fn eval_join_path(&mut self, args: &[ConfigExpr], depth: usize) -> MResult<ConfigValue> {
        if args.is_empty() {
            return Err(ConfigProfileViolation::error(
                "wrong arity for builtin `join-path`: expected at least 1 got 0",
            ));
        }
        let mut parts = Vec::new();
        for arg in args {
            match self.eval_expr(arg, depth + 1)? {
                ConfigValue::String(value) => parts.push(value),
                _ => {
                    return Err(ConfigProfileViolation::error(
                        "path join arguments must be strings",
                    ));
                }
            }
        }
        let out = parts.join("/");
        self.count_string(&out)?;
        Ok(ConfigValue::String(out))
    }

    fn eval_string(&mut self, args: &[ConfigExpr], depth: usize) -> MResult<ConfigValue> {
        if args.len() != 1 {
            return Err(ConfigProfileViolation::error(format!(
                "wrong arity for builtin `string`: expected 1 got {}",
                args.len()
            )));
        }
        let arg = &args[0];
        let out = match self.eval_expr(arg, depth + 1)? {
            ConfigValue::String(value) => value,
            ConfigValue::Integer(value) => value.to_string(),
            ConfigValue::Bool(value) => value.to_string(),
            ConfigValue::Float(value) => value.to_string(),
            _ => {
                return Err(ConfigProfileViolation::error(
                    "cannot convert config value to string",
                ));
            }
        };
        self.count_string(&out)?;
        Ok(ConfigValue::String(out))
    }

    fn eval_user_function(
        &mut self,
        name: &str,
        args: &[ConfigExpr],
        depth: usize,
    ) -> MResult<ConfigValue> {
        let function = self.functions.get(name).cloned().ok_or_else(|| {
            ConfigProfileViolation::error(format!("unknown config function `{name}`"))
        })?;
        if function.params.len() != args.len() {
            return Err(ConfigProfileViolation::error(format!(
                "wrong arity for function `{name}`: expected {} got {}",
                function.params.len(),
                args.len()
            )));
        }

        let saved = self.bindings.clone();
        for (param, arg) in function.params.iter().zip(args) {
            let value = self.eval_expr(arg, depth + 1)?;
            self.bindings.insert(param.clone(), value);
        }
        let result = self.eval_expr(&function.body, depth + 1);
        self.bindings = saved;
        result
    }
}
