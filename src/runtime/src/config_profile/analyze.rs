use std::collections::{BTreeMap, BTreeSet};

use mech_core::MResult;

use super::{
    ConfigEffectfulFunctionNotAllowed, ConfigExpr, ConfigFunction, ConfigItem,
    ConfigProfileOptions, ConfigProfileViolation, ConfigProgram, ConfigRecursionNotAllowed,
    ConfigUnknownFunction,
};

pub struct ConfigAnalyzer {
    #[allow(dead_code)]
    options: ConfigProfileOptions,
}

impl ConfigAnalyzer {
    pub fn new(options: ConfigProfileOptions) -> Self {
        Self { options }
    }

    pub fn analyze(&self, program: &ConfigProgram) -> MResult<()> {
        let mut functions: BTreeMap<String, &ConfigFunction> = BTreeMap::new();
        let mut config_lets = 0usize;

        for item in &program.items {
            match item {
                ConfigItem::Function(function) => {
                    if functions.insert(function.name.clone(), function).is_some() {
                        return Err(ConfigProfileViolation::error(format!(
                            "function `{}` is defined more than once",
                            function.name
                        )));
                    }
                }
                ConfigItem::Let(binding) if binding.name == "config" => config_lets += 1,
                _ => {}
            }
        }

        if config_lets > 1 {
            return Err(ConfigProfileViolation::error(
                "config binding is defined more than once",
            ));
        }

        let mut graph: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for (name, function) in &functions {
            let mut calls = Vec::new();
            collect_calls(&function.body, &mut calls);
            for (call_name, arity) in &calls {
                self.check_call(call_name, *arity, &functions)?;
            }
            graph.insert(
                name.clone(),
                calls
                    .into_iter()
                    .map(|(call, _)| call)
                    .filter(|call| functions.contains_key(call))
                    .collect(),
            );
        }

        for item in &program.items {
            match item {
                ConfigItem::Let(binding) => self.check_expr_calls(&binding.expr, &functions)?,
                ConfigItem::Expr(expr) => self.check_expr_calls(expr, &functions)?,
                ConfigItem::Function(_) => {}
            }
        }

        reject_recursion(&graph)
    }

    fn check_expr_calls(
        &self,
        expr: &ConfigExpr,
        functions: &BTreeMap<String, &ConfigFunction>,
    ) -> MResult<()> {
        let mut calls = Vec::new();
        collect_calls(expr, &mut calls);
        for (name, arity) in calls {
            self.check_call(&name, arity, functions)?;
        }
        Ok(())
    }

    fn check_call(
        &self,
        name: &str,
        arity: usize,
        functions: &BTreeMap<String, &ConfigFunction>,
    ) -> MResult<()> {
        if is_effectful_function_name(name) {
            return Err(ConfigEffectfulFunctionNotAllowed::error(format!(
                "{name} is not allowed in Mech config"
            )));
        }

        if let Some(expected) = builtin_arity(name) {
            if !expected.matches(arity) {
                return Err(ConfigProfileViolation::error(expected.message(name, arity)));
            }
            return Ok(());
        }

        if let Some(function) = functions.get(name) {
            let expected = function.params.len();
            if expected != arity {
                return Err(ConfigProfileViolation::error(format!(
                    "wrong arity for function `{name}`: expected {expected} got {arity}"
                )));
            }
            return Ok(());
        }

        Err(ConfigUnknownFunction::error(format!(
            "{name} is not a known pure Mech config function"
        )))
    }
}

#[derive(Clone, Copy)]
enum BuiltinArity {
    Exact(usize),
    AtLeast(usize),
}

impl BuiltinArity {
    fn matches(self, arity: usize) -> bool {
        match self {
            BuiltinArity::Exact(expected) => arity == expected,
            BuiltinArity::AtLeast(expected) => arity >= expected,
        }
    }

    fn message(self, name: &str, arity: usize) -> String {
        match self {
            BuiltinArity::Exact(expected) => {
                format!("wrong arity for builtin `{name}`: expected {expected} got {arity}")
            }
            BuiltinArity::AtLeast(expected) => format!(
                "wrong arity for builtin `{name}`: expected at least {expected} got {arity}"
            ),
        }
    }
}

fn builtin_arity(name: &str) -> Option<BuiltinArity> {
    match name {
        "str" | "string" => Some(BuiltinArity::Exact(1)),
        "join-path" | "path-join" => Some(BuiltinArity::AtLeast(1)),
        _ => None,
    }
}

fn collect_calls(expr: &ConfigExpr, calls: &mut Vec<(String, usize)>) {
    match expr {
        ConfigExpr::Call { name, args } => {
            calls.push((name.clone(), args.len()));
            for arg in args {
                collect_calls(arg, calls);
            }
        }
        ConfigExpr::List(items) => {
            for item in items {
                collect_calls(item, calls);
            }
        }
        ConfigExpr::Map(entries) => {
            for (_, value) in entries {
                collect_calls(value, calls);
            }
        }
        ConfigExpr::Add(lhs, rhs) | ConfigExpr::Sub(lhs, rhs) => {
            collect_calls(lhs, calls);
            collect_calls(rhs, calls);
        }
        ConfigExpr::Negate(inner) | ConfigExpr::Not(inner) => collect_calls(inner, calls),
        ConfigExpr::Null
        | ConfigExpr::Bool(_)
        | ConfigExpr::Integer(_)
        | ConfigExpr::Float(_)
        | ConfigExpr::String(_)
        | ConfigExpr::Atom(_)
        | ConfigExpr::Var(_) => {}
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
