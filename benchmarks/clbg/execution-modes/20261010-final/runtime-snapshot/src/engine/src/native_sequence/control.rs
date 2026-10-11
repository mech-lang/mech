//! Canonical-source IR for closed native loops and small immutable byte vectors.
//! Loop bodies, predicates and range callbacks are ordinary Mech functions.
pub mod analysis;

use super::{text, visit};
use mech_syntax::document::{
    AnyCallArgumentSyntax, AstNode, ComprehensionQualifierSyntax,
    ComprehensionQualifierValueSyntax, ExpressionSyntax, FunctionCallSyntax, KindAnnotationSyntax,
    KindDefineSyntax, KindValueSyntax, MatrixComprehensionSyntax, PatternSyntax,
    PatternValueSyntax, RangeExpressionSyntax, RecordSyntax, SliceSyntax, StringLiteralSyntax,
    SyntaxKind, SyntaxNode, TupleDestructureSyntax, VariableDefineSyntax, VariableStemSyntax,
    VariableSyntax,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Integer,
    Reference,
    BigInteger,
    Float,
    Boolean,
    Bytes(usize),
    ReadBytes,
    Floats,
    Tuple(Vec<Kind>),
    Record(Vec<(String, Kind)>),
    Type(Box<Kind>),
    Text,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Expr {
    pub kind: Kind,
    pub op: Op,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Op {
    Integer(u64),
    Float(u64),
    Boolean(bool),
    Text(String),
    Type,
    ReadBytes(Vec<u8>),
    Floats(Vec<u64>),
    Parameter(usize),
    Local(usize),
    Tuple(Vec<Expr>),
    Field(Box<Expr>, usize),
    Primitive(String, Vec<Expr>),
    Loop {
        id: usize,
        initial: Box<Expr>,
        predicate: String,
        step: String,
    },
    Branch {
        id: usize,
        condition: Box<Expr>,
        yes: String,
        no: String,
        state: Box<Expr>,
    },
    Match {
        id: usize,
        scrutinee: Box<Expr>,
        arms: Vec<(Expr, Expr)>,
    },
    Reduce {
        id: usize,
        index: usize,
        start: Box<Expr>,
        step: Box<Expr>,
        end: Box<Expr>,
        inclusive: bool,
        body: Box<Expr>,
    },
    Call(String, Vec<Expr>),
    Invoke {
        id: usize,
        name: String,
        arguments: Vec<Expr>,
    },
    Heap {
        id: usize,
        action: u64,
        arguments: Vec<Expr>,
        arity: usize,
        layout: Option<Kind>,
    },
    Arena {
        id: usize,
        capacity: Option<Box<Expr>>,
        arity: usize,
        layout: Kind,
        function: String,
        arguments: Box<Expr>,
    },
    Print {
        id: usize,
        format: String,
        values: Vec<Expr>,
    },
    Sequence {
        effects: Vec<Expr>,
        output: Box<Expr>,
    },
    Parallel {
        id: usize,
        start: Box<Expr>,
        end: Box<Expr>,
        grain: Box<Expr>,
        function: String,
        arguments: Box<Expr>,
        reductions: Vec<String>,
    },
    Write {
        id: usize,
        text: String,
    },
    Scan {
        id: usize,
        count: Box<Expr>,
        state: Box<Expr>,
        function: String,
        width: Box<Expr>,
        separator: String,
        buffer: Box<Expr>,
    },
}
#[derive(Clone, Debug)]
pub struct Function {
    pub inputs: Vec<Kind>,
    pub output: Expr,
}
#[derive(Clone, Debug)]
pub struct Program {
    /// Facts for the current IR. Mutating transforms must call
    /// `refresh_computation` before consuming these facts.
    pub computation: analysis::ComputationAnalysis,
    pub native_functions: bool,
    pub functions: BTreeMap<String, Function>,
    pub output: Expr,
    pub format: String,
    pub argument_count: usize,
    pub effects: Vec<Expr>,
}
impl Program {
    pub fn refresh_computation(&mut self) -> Result<(), String> {
        let computation = analysis::infer(&self.functions, &self.effects, &self.output)?;
        analysis::validate(
            &computation,
            &self.functions,
            &self.effects,
            &self.output,
            self.native_functions,
        )?;
        self.computation = computation;
        Ok(())
    }
}
#[derive(Clone)]
struct SourceFunction {
    inputs: Vec<(String, Kind)>,
    outputs: Vec<(String, Kind)>,
    body: SyntaxNode,
}
fn parameter(
    node: &SyntaxNode,
    aliases: &BTreeMap<String, SyntaxNode>,
) -> Result<(String, Kind), String> {
    let name = node
        .children()
        .find(|n| n.kind() == SyntaxKind::Identifier)
        .ok_or("parameter name")?;
    let annotation = node
        .children()
        .find(|n| n.kind() == SyntaxKind::KindAnnotation)
        .ok_or("parameter kind required")?;
    let kind = native_kind(&annotation, aliases, &mut BTreeSet::new())?;
    Ok((text(&name)?, kind))
}
fn native_kind(
    annotation: &SyntaxNode,
    aliases: &BTreeMap<String, SyntaxNode>,
    active: &mut BTreeSet<String>,
) -> Result<Kind, String> {
    let view = KindAnnotationSyntax::cast(annotation.clone()).ok_or("kind annotation")?;
    let with_option = view.kind().ok_or("kind")?;
    if with_option.question_mark().is_some() {
        return Err("native options require lowering support".into());
    }
    if let Some(KindValueSyntax::Record(record)) = with_option.kind().and_then(|k| k.value()) {
        let names = record.fields();
        let kinds = record.field_kinds();
        if names.len() != kinds.len() {
            return Err("record field annotations required".into());
        }
        let mut seen = BTreeSet::new();
        let fields = names
            .iter()
            .zip(kinds)
            .map(|(name, kind)| {
                let name = text(name.syntax())?;
                if !seen.insert(name.clone()) {
                    return Err("duplicate record field".into());
                }
                Ok((name, native_kind(kind.syntax(), aliases, active)?))
            })
            .collect::<Result<Vec<_>, String>>()?;
        return Ok(Kind::Record(fields));
    }
    let s = text(annotation)?.replace(' ', "");
    if let Some(name) = s.strip_prefix('<').and_then(|s| s.strip_suffix('>')) {
        if let Some(def) = aliases.get(name) {
            if !active.insert(name.to_owned()) {
                return Err("recursive kind aliases require explicit managed links".into());
            }
            let kind = native_kind(def, aliases, active);
            active.remove(name);
            return kind;
        }
    }
    let kind = match s.as_str() {
        "<u64>" => Kind::Integer,
        "<ref>" => Kind::Reference,
        "<bigint>" => Kind::BigInteger,
        "<f64>" => Kind::Float,
        "<bool>" => Kind::Boolean,
        "<[u8]>" => Kind::ReadBytes,
        "<[f64]>" => Kind::Floats,
        _ => {
            let n = s
                .strip_prefix("<[u8]:")
                .and_then(|s| s.strip_suffix(",1>"))
                .and_then(|s| s.parse().ok())
                .ok_or("native control supports u64, bool and fixed byte columns")?;
            if !(1..=64).contains(&n) {
                return Err("fixed byte length must be 1..64".into());
            }
            Kind::Bytes(n)
        }
    };
    Ok(kind)
}
fn args(node: &SyntaxNode) -> Result<(String, Vec<SyntaxNode>), String> {
    let call = FunctionCallSyntax::cast(node.clone()).ok_or("function call")?;
    let name = text(call.function().ok_or("function name")?.syntax())?;
    let arguments = call
        .arguments()
        .ok_or("call arguments")?
        .arguments()
        .into_iter()
        .map(|a| match a {
            AnyCallArgumentSyntax::Positional(x) => {
                Ok(x.value().ok_or("argument")?.syntax().clone())
            }
            _ => Err("positional arguments required".into()),
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok((name, arguments))
}
fn tuple(items: Vec<Expr>) -> Expr {
    Expr {
        kind: Kind::Tuple(items.iter().map(|e| e.kind.clone()).collect()),
        op: Op::Tuple(items),
    }
}
fn flat(kind: &Kind) -> Vec<Kind> {
    match kind {
        Kind::Tuple(k) => k.clone(),
        k => vec![k.clone()],
    }
}
fn scope_scalar(kind: &Kind) -> bool {
    matches!(kind, Kind::Integer | Kind::Float | Kind::Boolean)
}
pub fn scalar_lanes(kind: &Kind) -> Vec<Kind> {
    match kind {
        Kind::Tuple(items) => items.iter().flat_map(scalar_lanes).collect(),
        Kind::Record(fields) => fields.iter().flat_map(|(_, k)| scalar_lanes(k)).collect(),
        k => vec![k.clone()],
    }
}
fn record_lanes(kind: &Kind) -> Result<Vec<Kind>, String> {
    if !matches!(kind, Kind::Record(_)) {
        return Err("ordinary record required".into());
    }
    let lanes = scalar_lanes(kind);
    if lanes.is_empty()
        || lanes.len() > 32
        || lanes
            .iter()
            .any(|k| !scope_scalar(k) && k != &Kind::Reference)
    {
        return Err("managed record requires 1..32 scalar or reference lanes".into());
    }
    Ok(lanes)
}
struct Lower<'a> {
    source: &'a BTreeMap<String, SourceFunction>,
    aliases: &'a BTreeMap<String, SyntaxNode>,
    functions: BTreeMap<String, Function>,
    active: BTreeSet<String>,
    catalog: &'a [&'a str],
    argument_count: usize,
    next_effect: usize,
    native_functions: bool,
}
impl Lower<'_> {
    fn reduced(
        &mut self,
        value: &SyntaxNode,
        qualifiers: &[ComprehensionQualifierSyntax],
        vars: &BTreeMap<String, Expr>,
    ) -> Result<Expr, String> {
        let Some((first, rest)) = qualifiers.split_first() else {
            let body = self.expr(value, vars)?;
            if !matches!(body.kind, Kind::Integer | Kind::Float) {
                return Err("native comprehension sums require u64 or f64 values".into());
            }
            return Ok(body);
        };
        match first.value().ok_or("comprehension qualifier")? {
            ComprehensionQualifierValueSyntax::Generator(generator) => {
                fn range(node: &SyntaxNode) -> Option<RangeExpressionSyntax> {
                    if let Some(range) = RangeExpressionSyntax::cast(node.clone()) {
                        return Some(range);
                    }
                    let children = node.children().collect::<Vec<_>>();
                    if children.len() == 1 {
                        range(&children[0])
                    } else {
                        None
                    }
                }
                let source = generator.source().ok_or("generator source")?;
                let range = range(source.syntax())
                    .ok_or("native sum generators require ordinary ascending u64 ranges")?;
                let bounds = range
                    .bounds()
                    .into_iter()
                    .map(|bound| self.expr(bound.syntax(), vars))
                    .collect::<Result<Vec<_>, _>>()?;
                if !matches!(bounds.len(), 2 | 3) || bounds.iter().any(|e| e.kind != Kind::Integer)
                {
                    return Err("native comprehension range bounds must be u64".into());
                }
                let operators = range.operators();
                let inclusive =
                    match text(operators.last().ok_or("range operator")?.syntax())?.trim() {
                        ".." => false,
                        "..=" => true,
                        _ => return Err("native range operator".into()),
                    };
                let start = bounds[0].clone();
                let end = bounds.last().unwrap().clone();
                let step = if bounds.len() == 3 {
                    bounds[1].clone()
                } else {
                    Expr {
                        kind: Kind::Integer,
                        op: Op::Integer(1),
                    }
                };
                let index = self.effect_id();
                let item = Expr {
                    kind: Kind::Integer,
                    op: Op::Local(index),
                };
                let mut locals = vars.clone();
                let (condition, total) = self.pattern(
                    &generator.pattern().ok_or("generator pattern")?,
                    &item,
                    &mut locals,
                    &mut BTreeSet::new(),
                )?;
                let mut body = self.reduced(value, rest, &locals)?;
                if !total {
                    body = self.filtered(condition, body);
                }
                Ok(Expr {
                    kind: body.kind.clone(),
                    op: Op::Reduce {
                        id: self.effect_id(),
                        index,
                        start: Box::new(start),
                        step: Box::new(step),
                        end: Box::new(end),
                        inclusive,
                        body: Box::new(body),
                    },
                })
            }
            ComprehensionQualifierValueSyntax::Definition(definition) => {
                if definition.mutability_marker().is_some() {
                    return Err("native comprehension bindings are immutable".into());
                }
                let name = text(definition.name().ok_or("qualifier binding name")?.syntax())?;
                let binding = self.expr(
                    definition
                        .value()
                        .ok_or("qualifier binding value")?
                        .syntax(),
                    vars,
                )?;
                if let Some(annotation) = definition.variable().and_then(|v| v.annotation()) {
                    if native_kind(annotation.syntax(), self.aliases, &mut BTreeSet::new())?
                        != binding.kind
                    {
                        return Err("comprehension binding annotation mismatch".into());
                    }
                }
                let mut locals = vars.clone();
                locals.insert(name, binding.clone());
                let output = self.reduced(value, rest, &locals)?;
                Ok(Expr {
                    kind: output.kind.clone(),
                    op: Op::Sequence {
                        effects: vec![binding],
                        output: Box::new(output),
                    },
                })
            }
            ComprehensionQualifierValueSyntax::Filter(filter) => {
                let condition = self.expr(filter.syntax(), vars)?;
                if condition.kind != Kind::Boolean {
                    return Err("native comprehension filter must be boolean".into());
                }
                let body = self.reduced(value, rest, vars)?;
                Ok(self.filtered(condition, body))
            }
        }
    }
    fn filtered(&mut self, condition: Expr, body: Expr) -> Expr {
        let zero = Expr {
            kind: body.kind.clone(),
            op: if body.kind == Kind::Float {
                Op::Float(0)
            } else {
                Op::Integer(0)
            },
        };
        Expr {
            kind: body.kind.clone(),
            op: Op::Match {
                id: self.effect_id(),
                scrutinee: Box::new(condition.clone()),
                arms: vec![
                    (condition, body),
                    (
                        Expr {
                            kind: Kind::Boolean,
                            op: Op::Boolean(true),
                        },
                        zero,
                    ),
                ],
            },
        }
    }
    fn effect_id(&mut self) -> usize {
        let id = self.next_effect;
        self.next_effect += 1;
        id
    }
    fn pattern(
        &mut self,
        pattern: &PatternSyntax,
        value: &Expr,
        vars: &mut BTreeMap<String, Expr>,
        bound: &mut BTreeSet<String>,
    ) -> Result<(Expr, bool), String> {
        let yes = || Expr {
            kind: Kind::Boolean,
            op: Op::Boolean(true),
        };
        match pattern.value().ok_or("match pattern body")? {
            PatternValueSyntax::Wildcard(_) => Ok((yes(), true)),
            PatternValueSyntax::Tuple(pattern) => {
                let Kind::Tuple(kinds) = &value.kind else {
                    return Err("native tuple pattern requires a tuple scrutinee".into());
                };
                let items = pattern.items();
                if items.len() != kinds.len() {
                    return Err("native tuple pattern arity".into());
                }
                let mut condition = yes();
                let mut total = true;
                for (i, (pattern, kind)) in items.iter().zip(kinds).enumerate() {
                    let field = Expr {
                        kind: kind.clone(),
                        op: Op::Field(Box::new(value.clone()), i),
                    };
                    let (part, irrefutable) = self.pattern(pattern, &field, vars, bound)?;
                    condition = Expr {
                        kind: Kind::Boolean,
                        op: Op::Primitive("&&".into(), vec![condition, part]),
                    };
                    total &= irrefutable;
                }
                Ok((condition, total))
            }
            PatternValueSyntax::Expression(expression) => {
                fn variable(node: &SyntaxNode, whole: &SyntaxNode) -> Option<VariableSyntax> {
                    if node.kind() == SyntaxKind::Variable && node.range() == whole.range() {
                        return VariableSyntax::cast(node.clone());
                    }
                    node.children().find_map(|child| variable(&child, whole))
                }
                if let Some(variable) = variable(expression.syntax(), expression.syntax()) {
                    let Some(VariableStemSyntax::Identifier(name)) = variable.stem() else {
                        return Err("match binding requires a lexical identifier".into());
                    };
                    let name = text(name.syntax())?;
                    if !bound.insert(name.clone()) {
                        return Err("duplicate native pattern binding".into());
                    }
                    if let Some(annotation) = variable.annotation() {
                        if native_kind(annotation.syntax(), self.aliases, &mut BTreeSet::new())?
                            != value.kind
                        {
                            return Err(
                                "match binding annotation differs from scrutinee kind".into()
                            );
                        }
                    }
                    vars.insert(name, value.clone());
                    return Ok((yes(), true));
                }
                let literal = self.expr(expression.syntax(), vars)?;
                if literal.kind != value.kind
                    || !matches!(
                        literal.op,
                        Op::Integer(_) | Op::Float(_) | Op::Boolean(_) | Op::Heap { action: 0, .. }
                    )
                {
                    return Err("native match patterns require same-kind scalar constants or lexical bindings".into());
                }
                Ok((
                    Expr {
                        kind: Kind::Boolean,
                        op: Op::Primitive("==".into(), vec![value.clone(), literal]),
                    },
                    false,
                ))
            }
            _ => Err(
                "native matches currently support scalar, binding, wildcard and tuple patterns"
                    .into(),
            ),
        }
    }
    fn matched(
        &mut self,
        scrutinee: Expr,
        syntax: Vec<(PatternSyntax, Option<SyntaxNode>, SyntaxNode)>,
        vars: &BTreeMap<String, Expr>,
        total_required: bool,
    ) -> Result<Expr, String> {
        let mut arms = Vec::new();
        let mut total = false;
        let mut booleans = BTreeSet::new();
        let mut kind = None;
        for (pattern, guard, body) in syntax {
            if total {
                return Err("unreachable native match arm".into());
            }
            let mut locals = vars.clone();
            let (mut condition, irrefutable) =
                self.pattern(&pattern, &scrutinee, &mut locals, &mut BTreeSet::new())?;
            if guard.is_none() {
                total |= irrefutable;
                if let Op::Primitive(name, args) = &condition.op {
                    if name == "==" && scrutinee.kind == Kind::Boolean {
                        if let Op::Boolean(value) = args[1].op {
                            booleans.insert(value);
                        }
                    }
                }
                total |= booleans.len() == 2;
            }
            if let Some(guard) = guard {
                let guard = self.expr(&guard, &locals)?;
                if guard.kind != Kind::Boolean {
                    return Err("native match guard must be boolean".into());
                }
                // A guard executes only after its structural/literal pattern matched.
                condition = Expr {
                    kind: Kind::Boolean,
                    op: Op::Match {
                        id: self.effect_id(),
                        scrutinee: Box::new(condition.clone()),
                        arms: vec![
                            (condition, guard),
                            (
                                Expr {
                                    kind: Kind::Boolean,
                                    op: Op::Boolean(true),
                                },
                                Expr {
                                    kind: Kind::Boolean,
                                    op: Op::Boolean(false),
                                },
                            ),
                        ],
                    },
                };
            }
            let body = self.expr(&body, &locals)?;
            if kind.as_ref().is_some_and(|kind| kind != &body.kind) {
                return Err("native match arm result kinds differ".into());
            }
            kind = Some(body.kind.clone());
            arms.push((condition, body));
        }
        if total_required && !total {
            return Err("native inline match must cover every input (add a wildcard arm)".into());
        }
        Ok(Expr {
            kind: kind.ok_or("empty native match")?,
            op: Op::Match {
                id: self.effect_id(),
                scrutinee: Box::new(scrutinee),
                arms,
            },
        })
    }
    fn function(&mut self, name: &str) -> Result<Function, String> {
        if let Some(f) = self.functions.get(name) {
            return Ok(f.clone());
        }
        if !self.active.insert(name.to_owned()) {
            if !self.native_functions {
                return Err("recursive functions unsupported in closed native loops".into());
            }
            let declaration = self
                .source
                .get(name)
                .ok_or("missing recursive declaration")?;
            let outputs: Vec<_> = declaration.outputs.iter().map(|(_, k)| k.clone()).collect();
            return Ok(Function {
                inputs: declaration.inputs.iter().map(|(_, k)| k.clone()).collect(),
                output: Expr {
                    kind: if outputs.len() == 1 {
                        outputs[0].clone()
                    } else {
                        Kind::Tuple(outputs)
                    },
                    op: Op::Parameter(0),
                },
            });
        }
        let source = self
            .source
            .get(name)
            .ok_or_else(|| format!("undefined source function {name}"))?
            .clone();
        let mut locals = BTreeMap::new();
        for (i, (name, kind)) in source.inputs.iter().enumerate() {
            if locals
                .insert(
                    name.clone(),
                    Expr {
                        kind: kind.clone(),
                        op: Op::Parameter(i),
                    },
                )
                .is_some()
            {
                return Err("duplicate parameter".into());
            }
        }
        let output = if source.body.kind() == SyntaxKind::FunctionDefineMatchArms {
            let inputs = source
                .inputs
                .iter()
                .map(|(name, _)| locals[name].clone())
                .collect::<Vec<_>>();
            let scrutinee = if inputs.len() == 1 {
                inputs[0].clone()
            } else {
                tuple(inputs)
            };
            let arms = source
                .body
                .children()
                .filter(|n| n.kind() == SyntaxKind::FunctionMatchArm)
                .map(|arm| {
                    Ok((
                        arm.children()
                            .find_map(PatternSyntax::cast)
                            .ok_or("function arm pattern")?,
                        None,
                        arm.children()
                            .find_map(ExpressionSyntax::cast)
                            .ok_or("function arm result")?
                            .syntax()
                            .clone(),
                    ))
                })
                .collect::<Result<Vec<_>, String>>()?;
            let result = self.matched(scrutinee, arms, &locals, false)?;
            if result.kind != source.outputs[0].1 {
                return Err("pattern function result differs from declared kind".into());
            }
            result
        } else {
            let mut effects = Vec::new();
            for statement in source
                .body
                .children()
                .filter(|n| n.kind() == SyntaxKind::Statement)
            {
                let previous: BTreeSet<_> = locals.keys().cloned().collect();
                self.statement(&statement, &mut locals)?;
                if self.native_functions {
                    effects.extend(
                        locals
                            .iter()
                            .filter(|(k, _)| !previous.contains(*k))
                            .map(|(_, v)| v.clone()),
                    );
                }
            }
            let mut values = Vec::new();
            for (name, kind) in &source.outputs {
                let value = locals
                    .get(name)
                    .ok_or_else(|| format!("undefined output {name}"))?
                    .clone();
                if &value.kind != kind {
                    return Err(format!(
                        "{name}: declared output differs from body {:?}",
                        value.kind
                    ));
                }
                values.push(value);
            }
            let output = if values.len() == 1 {
                values.remove(0)
            } else {
                tuple(values)
            };
            let output = if self.native_functions {
                Expr {
                    kind: output.kind.clone(),
                    op: Op::Sequence {
                        effects,
                        output: Box::new(output),
                    },
                }
            } else {
                output
            };
            output
        };
        let f = Function {
            inputs: source.inputs.iter().map(|(_, k)| k.clone()).collect(),
            output,
        };
        self.functions.insert(name.to_owned(), f.clone());
        self.active.remove(name);
        Ok(f)
    }
    fn statement(
        &mut self,
        node: &SyntaxNode,
        vars: &mut BTreeMap<String, Expr>,
    ) -> Result<(), String> {
        if let Some(d) = node.children().find_map(VariableDefineSyntax::cast) {
            if d.mutability_marker().is_some() {
                return Err("native control uses immutable loop state".into());
            }
            let name = text(d.name().ok_or("definition name")?.syntax())?;
            let value = self.expr(d.value().ok_or("definition value")?.syntax(), vars)?;
            if let Some(annotation) = d.variable().and_then(|v| v.annotation()) {
                if native_kind(annotation.syntax(), self.aliases, &mut BTreeSet::new())?
                    != value.kind
                {
                    return Err("variable annotation mismatch".into());
                }
            }
            if vars.insert(name, value).is_some() {
                return Err("duplicate definition".into());
            }
            return Ok(());
        }
        if let Some(d) = node.children().find_map(TupleDestructureSyntax::cast) {
            let value = self.expr(d.value().ok_or("destructure value")?.syntax(), vars)?;
            let Kind::Tuple(kinds) = &value.kind else {
                return Err("destructure requires tuple".into());
            };
            let names = d.names();
            if names.len() != kinds.len() {
                return Err("destructure arity".into());
            }
            for (i, name) in names.into_iter().enumerate() {
                if vars
                    .insert(
                        text(name.syntax())?,
                        Expr {
                            kind: kinds[i].clone(),
                            op: Op::Field(Box::new(value.clone()), i),
                        },
                    )
                    .is_some()
                {
                    return Err("duplicate definition".into());
                }
            }
            return Ok(());
        }
        Err("native control statements require definitions or tuple destructuring".into())
    }
    fn expr(&mut self, node: &SyntaxNode, vars: &BTreeMap<String, Expr>) -> Result<Expr, String> {
        let s = text(node)?;
        let s = s.trim();
        if let Some(e) = vars.get(s) {
            return Ok(e.clone());
        }
        if let Some(expression) = ExpressionSyntax::cast(node.clone()) {
            let arms = expression.match_arms();
            if !arms.is_empty() {
                let scrutinee =
                    self.expr(expression.body().ok_or("match input")?.syntax(), vars)?;
                let arms = arms
                    .into_iter()
                    .map(|arm| {
                        Ok((
                            arm.pattern().ok_or("match pattern")?,
                            arm.guard().map(|e| e.syntax().clone()),
                            arm.value().ok_or("match result")?.syntax().clone(),
                        ))
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                return self.matched(scrutinee, arms, vars, true);
            }
        }
        if node.kind() == SyntaxKind::Number {
            if s.contains('.') || s.contains('e') || s.ends_with("f64") {
                let value = s
                    .strip_suffix("f64")
                    .unwrap_or(s)
                    .parse::<f64>()
                    .map_err(|_| "f64 literal required")?;
                return Ok(Expr {
                    kind: Kind::Float,
                    op: Op::Float(value.to_bits()),
                });
            }
            return Ok(Expr {
                kind: Kind::Integer,
                op: Op::Integer(s.parse().map_err(|_| "u64 literal required")?),
            });
        }
        if let Some(l) = StringLiteralSyntax::cast(node.clone()) {
            return Ok(Expr {
                kind: Kind::Text,
                op: Op::Text(l.decoded_text().ok_or("invalid string escape")?),
            });
        }
        if s == "true" || s == "false" {
            return Ok(Expr {
                kind: Kind::Boolean,
                op: Op::Boolean(s == "true"),
            });
        }
        if let Some(annotation) = KindAnnotationSyntax::cast(node.clone()) {
            return Ok(Expr {
                kind: Kind::Type(Box::new(native_kind(
                    annotation.syntax(),
                    self.aliases,
                    &mut BTreeSet::new(),
                )?)),
                op: Op::Type,
            });
        }
        if let Some(record) = RecordSyntax::cast(node.clone()) {
            let mut fields = Vec::new();
            let mut values = Vec::new();
            let mut seen = BTreeSet::new();
            for binding in record.bindings() {
                let name = text(binding.name().ok_or("record field name")?.syntax())?;
                if !seen.insert(name.clone()) {
                    return Err("duplicate record field".into());
                }
                let value =
                    self.expr(binding.value().ok_or("record field value")?.syntax(), vars)?;
                if let Some(annotation) = binding.annotation() {
                    if native_kind(annotation.syntax(), self.aliases, &mut BTreeSet::new())?
                        != value.kind
                    {
                        return Err("record field annotation mismatch".into());
                    }
                }
                fields.push((name, value.kind.clone()));
                values.push(value);
            }
            return Ok(Expr {
                kind: Kind::Record(fields),
                op: Op::Tuple(values),
            });
        }
        if let Some(slice) = SliceSyntax::cast(node.clone()) {
            let mut value = self.expr(slice.stem().ok_or("field stem")?.syntax(), vars)?;
            for item in slice.subscripts().ok_or("field selectors")?.items() {
                let selection = text(item.syntax())?;
                let name = selection
                    .trim()
                    .strip_prefix('.')
                    .ok_or("native record requires dot fields")?;
                let Kind::Record(fields) = &value.kind else {
                    return Err("dot field requires record".into());
                };
                let index = fields
                    .iter()
                    .position(|(field, _)| field == name)
                    .ok_or("unknown record field")?;
                value = Expr {
                    kind: fields[index].1.clone(),
                    op: Op::Field(Box::new(value), index),
                };
            }
            return Ok(value);
        }
        if node.kind() == SyntaxKind::Tuple {
            let mut items = node
                .children()
                .filter(|n| n.kind() == SyntaxKind::Expression)
                .map(|n| self.expr(&n, vars))
                .collect::<Result<Vec<_>, _>>()?;
            if items.is_empty() {
                return Err("empty tuple".into());
            }
            // Canonical Mech singleton tuples are grouping, not an aggregate.
            if items.len() == 1 {
                return Ok(items.remove(0));
            }
            return Ok(tuple(items));
        }
        if node.kind() == SyntaxKind::FunctionCall {
            let (name, a) = args(node)?;
            if self.source.contains_key(&name) {
                let f = self.function(&name)?;
                let values = a
                    .iter()
                    .map(|x| self.expr(x, vars))
                    .collect::<Result<Vec<_>, _>>()?;
                if values.iter().map(|e| e.kind.clone()).collect::<Vec<_>>() != f.inputs {
                    return Err(format!("{name}: incompatible source arguments"));
                }
                return Ok(Expr {
                    kind: f.output.kind,
                    op: if self.native_functions {
                        let id = self.next_effect;
                        self.next_effect += 1;
                        Op::Invoke {
                            id,
                            name,
                            arguments: values,
                        }
                    } else {
                        Op::Call(name, values)
                    },
                });
            }
            if !self.catalog.contains(&name.as_str()) {
                return Err(format!("unsupported native control operation {name}"));
            }
            if name == "stats/sum" && a.len() == 1 {
                fn comprehension(node: &SyntaxNode) -> Option<MatrixComprehensionSyntax> {
                    if let Some(value) = MatrixComprehensionSyntax::cast(node.clone()) {
                        return Some(value);
                    }
                    let children = node.children().collect::<Vec<_>>();
                    if children.len() == 1 {
                        comprehension(&children[0])
                    } else {
                        None
                    }
                }
                let expression = comprehension(&a[0])
                    .ok_or("native stats/sum requires a matrix comprehension")?;
                let value = expression.value().ok_or("comprehension result")?;
                return self.reduced(value.syntax(), &expression.qualifiers(), vars);
            }
            if !self.active.is_empty() && name == "io/argument-u64" {
                return Err("entry arguments cannot be captured by native source functions".into());
            }
            if !self.native_functions
                && !self.active.is_empty()
                && matches!(
                    name.as_str(),
                    "io/write" | "io/write-scan" | "io/argument-u64" | "parallel/reduce-ranges"
                )
            {
                return Err("native source callbacks must be pure; host IO and scheduling belong at the entry".into());
            }
            if matches!(
                name.as_str(),
                "memory/with-ref-arena" | "memory/with-arena" | "memory/with-frame"
            ) {
                let region = name != "memory/with-frame";
                if a.len() != if region { 4 } else { 2 } {
                    return Err("memory scope signature".into());
                }
                let capacity = if region {
                    Some(Box::new(self.expr(&a[0], vars)?))
                } else {
                    None
                };
                let layout = if region {
                    let e = self.expr(&a[1], vars)?;
                    if name == "memory/with-arena" {
                        let Kind::Type(layout) = e.kind else {
                            return Err("arena record kind required".into());
                        };
                        record_lanes(&layout)?;
                        *layout
                    } else {
                        let Op::Integer(n) = e.op else {
                            return Err("arena arity must be literal".into());
                        };
                        if !(1..=32).contains(&n) {
                            return Err("arena arity 1..32".into());
                        }
                        Kind::Tuple(vec![Kind::Reference; n as usize])
                    }
                } else {
                    Kind::Tuple(vec![])
                };
                let arity = scalar_lanes(&layout).len();
                if capacity.as_ref().is_some_and(|e| e.kind != Kind::Integer) {
                    return Err("arena capacity must be u64".into());
                }
                let i = if region { 2 } else { 0 };
                let function = text(&a[i])?.trim().to_owned();
                let arguments = self.expr(&a[i + 1], vars)?;
                let f = self.function(&function)?;
                if f.inputs != flat(&arguments.kind)
                    || f.inputs.iter().flat_map(scalar_lanes).count() > 32
                    || f.inputs
                        .iter()
                        .flat_map(scalar_lanes)
                        .any(|k| !scope_scalar(&k))
                    || scalar_lanes(&f.output.kind).len() > 32
                    || scalar_lanes(&f.output.kind)
                        .iter()
                        .any(|k| !scope_scalar(k))
                {
                    return Err("memory scope callbacks require scalar inputs and outputs; references cannot escape".into());
                }
                let id = self.next_effect;
                self.next_effect += 1;
                return Ok(Expr {
                    kind: f.output.kind,
                    op: Op::Arena {
                        id,
                        capacity,
                        arity,
                        layout,
                        function,
                        arguments: Box::new(arguments),
                    },
                });
            }
            if name == "control/while" {
                if a.len() != 3 {
                    return Err("control/while arity".into());
                }
                let initial = self.expr(&a[0], vars)?;
                let predicate = text(&a[1])?.trim().to_owned();
                let step = text(&a[2])?.trim().to_owned();
                let p = self.function(&predicate)?;
                let f = self.function(&step)?;
                if p.inputs != flat(&initial.kind)
                    || p.output.kind != Kind::Boolean
                    || f.inputs != p.inputs
                    || f.output.kind != initial.kind
                {
                    return Err("loop callback signatures must preserve the state".into());
                }
                return Ok(Expr {
                    kind: initial.kind.clone(),
                    op: Op::Loop {
                        id: if self.native_functions {
                            self.next_effect += 1;
                            self.next_effect - 1
                        } else {
                            0
                        },
                        initial: Box::new(initial),
                        predicate,
                        step,
                    },
                });
            }
            if name == "control/if" {
                if a.len() != 4 {
                    return Err("control/if arity".into());
                }
                let condition = self.expr(&a[0], vars)?;
                let state = self.expr(&a[3], vars)?;
                let yes = text(&a[1])?.trim().to_owned();
                let no = text(&a[2])?.trim().to_owned();
                let y = self.function(&yes)?;
                let n = self.function(&no)?;
                if condition.kind != Kind::Boolean
                    || y.inputs != flat(&state.kind)
                    || n.inputs != y.inputs
                    || n.output.kind != y.output.kind
                {
                    return Err("conditional callback signatures".into());
                }
                return Ok(Expr {
                    kind: y.output.kind,
                    op: Op::Branch {
                        id: if self.native_functions {
                            self.next_effect += 1;
                            self.next_effect - 1
                        } else {
                            0
                        },
                        condition: Box::new(condition),
                        yes,
                        no,
                        state: Box::new(state),
                    },
                });
            }
            if name == "parallel/reduce-ranges" {
                if a.len() != 6 {
                    return Err("parallel/reduce-ranges arity".into());
                }
                let start = self.expr(&a[0], vars)?;
                let end = self.expr(&a[1], vars)?;
                let grain = self.expr(&a[2], vars)?;
                let function = text(&a[3])?.trim().to_owned();
                let arguments = self.expr(&a[4], vars)?;
                let reducers = self.expr(&a[5], vars)?;
                let Op::Text(reducers) = reducers.op else {
                    return Err("constant reducers required".into());
                };
                let reductions: Vec<_> = reducers.split(',').map(str::to_owned).collect();
                let f = self.function(&function)?;
                let mut inputs = vec![Kind::Integer; 2];
                inputs.extend(flat(&arguments.kind));
                if inputs.iter().any(|kind| kind != &Kind::Integer) {
                    return Err("parallel range arguments must be u64 scalars".into());
                }
                if [start.kind.clone(), end.kind.clone(), grain.kind.clone()]
                    != [const { Kind::Integer }; 3]
                    || f.inputs != inputs
                    || flat(&f.output.kind) != vec![Kind::Integer; reductions.len()]
                    || reductions.iter().any(|s| s != "sum" && s != "max")
                {
                    return Err("parallel range signature/reducers".into());
                }
                return Ok(Expr {
                    kind: f.output.kind,
                    op: Op::Parallel {
                        id: if self.native_functions {
                            self.next_effect += 1;
                            self.next_effect - 1
                        } else {
                            0
                        },
                        start: Box::new(start),
                        end: Box::new(end),
                        grain: Box::new(grain),
                        function,
                        arguments: Box::new(arguments),
                        reductions,
                    },
                });
            }
            if name == "io/write-scan" {
                if a.len() != 6 {
                    return Err("io/write-scan expects count, state, source step, width, separator, buffer size".into());
                }
                let count = self.expr(&a[0], vars)?;
                let state = self.expr(&a[1], vars)?;
                let function = text(&a[2])?.trim().to_owned();
                let width = self.expr(&a[3], vars)?;
                let separator = self.expr(&a[4], vars)?;
                let buffer = self.expr(&a[5], vars)?;
                let Op::Text(separator) = separator.op else {
                    return Err("constant scan separator required".into());
                };
                let kinds = flat(&state.kind);
                let callback = self.function(&function)?;
                let mut expected = kinds.clone();
                expected.push(Kind::Integer);
                if [count.kind.clone(), width.kind.clone(), buffer.kind.clone()]
                    != [const { Kind::Integer }; 3]
                    || kinds.len() > 32
                    || kinds
                        .iter()
                        .any(|k| !matches!(k, Kind::Integer | Kind::Float | Kind::Boolean))
                    || callback.inputs != kinds
                    || flat(&callback.output.kind) != expected
                {
                    return Err("scan step must return its scalar state followed by one unsigned byte value".into());
                }
                let id = self.next_effect;
                self.next_effect += 1;
                return Ok(Expr {
                    kind: state.kind.clone(),
                    op: Op::Scan {
                        id,
                        count: Box::new(count),
                        state: Box::new(state),
                        function,
                        width: Box::new(width),
                        separator,
                        buffer: Box::new(buffer),
                    },
                });
            }
            let values = a
                .iter()
                .map(|x| self.expr(x, vars))
                .collect::<Result<Vec<_>, _>>()?;
            self.primitive(name, values)
        } else {
            let children: Vec<_> = node.children().collect();
            if children.len() >= 3 && children.len() % 2 == 1 {
                let mut value = self.expr(&children[0], vars)?;
                for pair in children[1..].chunks(2) {
                    let op = text(&pair[0])?;
                    let rhs = self.expr(&pair[1], vars)?;
                    value = self.primitive(op.trim().to_owned(), vec![value, rhs])?;
                }
                return Ok(value);
            }
            if children.len() == 1 {
                return self.expr(&children[0], vars);
            }
            Err(format!(
                "unsupported native control syntax {:?}: {s}",
                node.kind()
            ))
        }
    }
    fn primitive(&mut self, name: String, values: Vec<Expr>) -> Result<Expr, String> {
        let kinds: Vec<_> = values.iter().map(|e| e.kind.clone()).collect();
        let int = Kind::Integer;
        let boolean = Kind::Boolean;
        if name.starts_with("bigint/") {
            let big = Kind::BigInteger;
            let action = match name.as_str() {
                "bigint/new" if kinds == [int.clone()] => 10,
                "bigint/assign" if kinds == [big.clone(), big.clone()] => 11,
                "bigint/multiply-u64-into" if kinds == [big.clone(), big.clone(), int.clone()] => {
                    12
                }
                "bigint/add-into" if kinds == [big.clone(), big.clone(), big.clone()] => 13,
                "bigint/subtract-into" if kinds == [big.clone(), big.clone(), big.clone()] => 14,
                "bigint/floor-u64" if kinds == [big.clone(), big.clone()] => 15,
                "bigint/add-multiple-u64" if kinds == [big.clone(), big.clone(), int.clone()] => 16,
                "bigint/subtract-multiple-u64"
                    if kinds == [big.clone(), big.clone(), int.clone()] =>
                {
                    17
                }
                "bigint/less-equal" if kinds == [big.clone(), big.clone()] => 18,
                _ => {
                    return Err(format!(
                        "{name}: incompatible arbitrary-precision kinds {kinds:?}"
                    ));
                }
            };
            let id = self.next_effect;
            self.next_effect += 1;
            return Ok(Expr {
                kind: if action == 15 {
                    int
                } else if action == 18 {
                    boolean
                } else {
                    big
                },
                op: Op::Heap {
                    id,
                    action,
                    arguments: values,
                    arity: 0,
                    layout: None,
                },
            });
        }
        if name.starts_with("heap/") || name.starts_with("memory/") || name == "io/print" {
            let id = self.next_effect;
            self.next_effect += 1;
            let (kind, op) = match name.as_str() {
                "heap/null" | "memory/null" if values.is_empty() => (
                    Kind::Reference,
                    Op::Heap {
                        id,
                        action: 0,
                        arguments: values,
                        arity: 0,
                        layout: None,
                    },
                ),
                "memory/allocate" if values.len() == 1 => {
                    let fields = record_lanes(&values[0].kind)?;
                    (
                        Kind::Reference,
                        Op::Heap {
                            id,
                            action: 1,
                            arity: fields.len(),
                            layout: Some(values[0].kind.clone()),
                            arguments: values,
                        },
                    )
                }
                "memory/read" if values.len() == 2 && values[0].kind == Kind::Reference => {
                    let Kind::Type(layout) = &values[1].kind else {
                        return Err("record read kind required".into());
                    };
                    let fields = record_lanes(layout)?;
                    (
                        *layout.clone(),
                        Op::Heap {
                            id,
                            action: 2,
                            arity: fields.len(),
                            layout: Some(*layout.clone()),
                            arguments: vec![values[0].clone()],
                        },
                    )
                }
                "heap/new-refs" if values.len() == 1 => {
                    let fields = flat(&values[0].kind);
                    if fields.is_empty()
                        || fields.len() > 32
                        || fields.iter().any(|k| k != &Kind::Reference)
                    {
                        return Err("heap/new-refs needs 1..32 reference fields".into());
                    }
                    (
                        Kind::Reference,
                        Op::Heap {
                            id,
                            action: 1,
                            arguments: values,
                            arity: fields.len(),
                            layout: Some(Kind::Tuple(fields.clone())),
                        },
                    )
                }
                "heap/read-refs" if kinds == [Kind::Reference, int.clone()] => {
                    let Op::Integer(arity) = values[1].op else {
                        return Err("record arity must be literal".into());
                    };
                    if !(1..=32).contains(&arity) {
                        return Err("record arity 1..32".into());
                    }
                    (
                        Kind::Tuple(vec![Kind::Reference; arity as usize]),
                        Op::Heap {
                            id,
                            action: 2,
                            arguments: vec![values[0].clone()],
                            arity: arity as usize,
                            layout: Some(Kind::Tuple(vec![Kind::Reference; arity as usize])),
                        },
                    )
                }
                "io/print"
                    if !values.is_empty()
                        && kinds[0] == Kind::Text
                        && kinds[1..].iter().all(|k| k == &int) =>
                {
                    let Op::Text(format) = &values[0].op else {
                        return Err("literal print format required".into());
                    };
                    if format.len() > 4096
                        || values.len() > 33
                        || format.matches("{}").count() != values.len() - 1
                        || format.replace("{}", "").contains(['{', '}'])
                    {
                        return Err("print format/field count".into());
                    }
                    (
                        int.clone(),
                        Op::Print {
                            id,
                            format: format.clone(),
                            values: values[1..].to_vec(),
                        },
                    )
                }
                _ => {
                    return Err(format!(
                        "{name}: incompatible native record kinds {kinds:?}"
                    ));
                }
            };
            return Ok(Expr { kind, op });
        }
        let kind = match name.as_str() {
            "+" | "-" | "*" | "/" if kinds == [Kind::Float, Kind::Float] => Kind::Float,
            "==" | "!=" | "<" | "<=" | ">" | ">=" if kinds == [Kind::Float, Kind::Float] => boolean,
            "number/to-f64" if kinds == [int.clone()] => Kind::Float,
            "bytes/from-text" if kinds == [Kind::Text] => {
                let Op::Text(s) = &values[0].op else {
                    return Err("constant byte text required by this native subset".into());
                };
                return Ok(Expr {
                    kind: Kind::ReadBytes,
                    op: Op::ReadBytes(s.as_bytes().to_vec()),
                });
            }
            "sequence/f64-values" if kinds.len() == 1 && matches!(kinds[0], Kind::Tuple(_)) => {
                let Op::Tuple(items) = &values[0].op else {
                    return Err("constant f64 values required".into());
                };
                let bits = items
                    .iter()
                    .map(|e| match e.op {
                        Op::Float(v) => Ok(v),
                        Op::Integer(v) => Ok((v as f64).to_bits()),
                        _ => Err("literal numeric values required"),
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                if bits.is_empty() || bits.len() > 4096 {
                    return Err("constant f64 column length must be 1..4096".into());
                }
                return Ok(Expr {
                    kind: Kind::Floats,
                    op: Op::Floats(bits),
                });
            }
            "sequence/cumulative-sum" if kinds == [Kind::Floats] => {
                let Op::Floats(bits) = &values[0].op else {
                    return Err(
                        "this native subset folds cumulative sums of constant columns".into(),
                    );
                };
                let mut sum = 0.0;
                let out = bits
                    .iter()
                    .map(|&bits| {
                        sum += f64::from_bits(bits);
                        sum.to_bits()
                    })
                    .collect();
                return Ok(Expr {
                    kind: Kind::Floats,
                    op: Op::Floats(out),
                });
            }
            "sequence/length"
                if kinds.len() == 1 && matches!(kinds[0], Kind::ReadBytes | Kind::Floats) =>
            {
                int
            }
            "sequence/get"
                if kinds.len() == 2 && kinds[0] == Kind::ReadBytes && kinds[1] == int =>
            {
                int
            }
            "sequence/get" if kinds.len() == 2 && kinds[0] == Kind::Floats && kinds[1] == int => {
                Kind::Float
            }
            "io/write" if kinds == [Kind::Text] => {
                let Op::Text(text) = &values[0].op else {
                    return Err("constant write text required by this native subset".into());
                };
                let id = self.next_effect;
                self.next_effect += 1;
                return Ok(Expr {
                    kind: Kind::Integer,
                    op: Op::Write {
                        id,
                        text: text.clone(),
                    },
                });
            }
            "+" | "-" | "*" | "/" | "number/mod" | "number/max"
                if kinds == [int.clone(), int.clone()] =>
            {
                int
            }
            "==" | "!=" if kinds == [Kind::Reference, Kind::Reference] => boolean,
            "==" | "!=" if kinds == [boolean.clone(), boolean.clone()] => boolean,
            "==" | "!=" | "<" | "<=" | ">" | ">=" if kinds == [int.clone(), int.clone()] => boolean,
            "&&" | "||" if kinds == [boolean.clone(), boolean.clone()] => boolean,
            "control/select" if kinds.len() == 3 && kinds[0] == boolean && kinds[1] == kinds[2] => {
                kinds[1].clone()
            }
            "number/shift-left" if kinds == [int.clone(), int.clone()] => int,
            "number/factorial" if kinds == [int.clone()] => int,
            "number/bounded" if kinds == vec![int.clone(); 3] => int,
            "io/argument-u64" if kinds == [int.clone()] => {
                let Op::Integer(i) = values[0].op else {
                    return Err("argument index must be constant".into());
                };
                if !(1..=32).contains(&i) {
                    return Err("argument index 1..32".into());
                }
                self.argument_count = self.argument_count.max(i as usize);
                int
            }
            "sequence/iota-bytes" | "sequence/zero-bytes" if kinds == [int.clone()] => {
                let Op::Integer(n) = values[0].op else {
                    return Err("small byte length must be constant".into());
                };
                if !(1..=64).contains(&n) {
                    return Err("byte length 1..64".into());
                }
                Kind::Bytes(n as usize)
            }
            "sequence/get"
                if kinds.len() == 2 && matches!(kinds[0], Kind::Bytes(_)) && kinds[1] == int =>
            {
                int
            }
            "sequence/set" | "sequence/rotate-prefix"
                if kinds.len() == 3
                    && matches!(kinds[0], Kind::Bytes(_))
                    && kinds[1] == int
                    && kinds[2] == int =>
            {
                kinds[0].clone()
            }
            "sequence/reverse-prefix"
                if kinds.len() == 2 && matches!(kinds[0], Kind::Bytes(_)) && kinds[1] == int =>
            {
                kinds[0].clone()
            }
            _ => return Err(format!("{name}: incompatible native kinds {kinds:?}")),
        };
        Ok(Expr {
            kind,
            op: Op::Primitive(name, values),
        })
    }
}

pub fn compile(source: &str, catalog: &[&str]) -> Result<Program, String> {
    compile_parsed(&super::ParsedSource::parse(source)?, catalog)
}
pub fn compile_parsed(parsed: &super::ParsedSource, catalog: &[&str]) -> Result<Program, String> {
    let syntax = &parsed.syntax;
    let mut kind_defs = Vec::new();
    visit(syntax, SyntaxKind::KindDefine, &mut kind_defs);
    let mut aliases = BTreeMap::new();
    for node in kind_defs {
        let definition = KindDefineSyntax::cast(node).ok_or("kind definition")?;
        let name = text(definition.name().ok_or("kind name")?.syntax())?;
        let annotation = definition
            .annotation()
            .ok_or("kind definition annotation")?
            .syntax()
            .clone();
        if aliases.insert(name, annotation).is_some() {
            return Err("duplicate kind definition".into());
        }
    }
    for annotation in aliases.values() {
        native_kind(annotation, &aliases, &mut BTreeSet::new())?;
    }
    let mut defs = Vec::new();
    visit(syntax, SyntaxKind::FunctionDefine, &mut defs);
    let mut functions = BTreeMap::new();
    for node in defs {
        let name = text(
            &node
                .children()
                .find(|n| n.kind() == SyntaxKind::Identifier)
                .ok_or("function name")?,
        )?;
        let inputs = node
            .children()
            .filter(|n| n.kind() == SyntaxKind::FunctionArg)
            .map(|n| parameter(&n, &aliases))
            .collect::<Result<Vec<_>, _>>()?;
        let body = node
            .children()
            .find(|n| {
                matches!(
                    n.kind(),
                    SyntaxKind::FunctionDefineStatements | SyntaxKind::FunctionDefineMatchArms
                )
            })
            .ok_or("native function body required")?;
        let outputs = if body.kind() == SyntaxKind::FunctionDefineMatchArms {
            let annotation = body
                .children()
                .find(|n| n.kind() == SyntaxKind::KindAnnotation)
                .ok_or("pattern function output kind")?;
            vec![(
                "$result".into(),
                native_kind(&annotation, &aliases, &mut BTreeSet::new())?,
            )]
        } else {
            let out = body
                .children()
                .find(|n| {
                    matches!(
                        n.kind(),
                        SyntaxKind::FunctionOutArg | SyntaxKind::FunctionOutArgs
                    )
                })
                .ok_or("function outputs")?;
            let mut outputs = Vec::new();
            visit(&out, SyntaxKind::FunctionArg, &mut outputs);
            let outputs = outputs
                .iter()
                .map(|n| parameter(n, &aliases))
                .collect::<Result<Vec<_>, _>>()?;
            outputs
        };
        if outputs.is_empty() {
            return Err("function output required".into());
        }
        if functions
            .insert(
                name,
                SourceFunction {
                    inputs,
                    outputs,
                    body,
                },
            )
            .is_some()
        {
            return Err("duplicate function".into());
        }
    }
    let native_functions = functions
        .values()
        .any(|f| f.body.kind() == SyntaxKind::FunctionDefineMatchArms)
        || {
            let mut matches = Vec::new();
            visit(syntax, SyntaxKind::MatchArm, &mut matches);
            visit(syntax, SyntaxKind::MatrixComprehension, &mut matches);
            !matches.is_empty()
        }
        || functions.values().any(|f| {
            f.inputs
                .iter()
                .chain(f.outputs.iter())
                .any(|(_, k)| k == &Kind::Reference)
        })
        || {
            let mut calls = Vec::new();
            visit(syntax, SyntaxKind::FunctionCall, &mut calls);
            calls.iter().any(|n| {
                args(n).is_ok_and(|(name, _)| {
                    name.starts_with("heap/")
                        || name.starts_with("memory/")
                        || name.starts_with("bigint/")
                        || name == "io/print"
                })
            })
        };
    let mut lower = Lower {
        source: &functions,
        aliases: &aliases,
        functions: BTreeMap::new(),
        active: BTreeSet::new(),
        catalog,
        argument_count: 0,
        next_effect: 0,
        native_functions,
    };
    let mut vars = BTreeMap::new();
    let mut roots = Vec::new();
    visit(syntax, SyntaxKind::MechCodeAlt, &mut roots);
    let mut output = None;
    let mut effects = Vec::new();
    let mut effect_ids = BTreeSet::new();
    fn collect_effects(e: &Expr, ids: &mut BTreeSet<usize>) -> bool {
        let mut new = false;
        match &e.op {
            Op::Write { id, .. } => new |= ids.insert(*id),
            Op::Scan {
                id,
                count,
                state,
                width,
                buffer,
                ..
            } => {
                for e in [count, state, width, buffer] {
                    new |= collect_effects(e, ids);
                }
                new |= ids.insert(*id);
            }
            Op::Tuple(a) | Op::Call(_, a) | Op::Primitive(_, a) => {
                for e in a {
                    new |= collect_effects(e, ids);
                }
            }
            Op::Field(e, _) => new |= collect_effects(e, ids),
            Op::Reduce {
                id,
                start,
                step,
                end,
                ..
            } => {
                for e in [start, step, end] {
                    new |= collect_effects(e, ids);
                }
                new |= ids.insert(*id);
            }
            Op::Match { id, scrutinee, .. } => {
                new |= collect_effects(scrutinee, ids);
                new |= ids.insert(*id);
            }
            Op::Loop { initial, .. } => new |= collect_effects(initial, ids),
            Op::Branch {
                condition, state, ..
            } => {
                new |= collect_effects(condition, ids);
                new |= collect_effects(state, ids);
            }
            Op::Parallel {
                start,
                end,
                grain,
                arguments,
                ..
            } => {
                for e in [start, end, grain, arguments] {
                    new |= collect_effects(e, ids);
                }
            }
            _ => {}
        }
        new
    }
    for root in roots {
        if root.children().all(|n| n.kind() == SyntaxKind::Comment) {
            continue;
        }
        let mut root_kinds = Vec::new();
        visit(&root, SyntaxKind::KindDefine, &mut root_kinds);
        if !root_kinds.is_empty()
            || root
                .children()
                .any(|n| n.kind() == SyntaxKind::FunctionDefine)
        {
            continue;
        }
        let mut stmt = Vec::new();
        visit(&root, SyntaxKind::Statement, &mut stmt);
        if stmt.iter().any(|s| {
            s.children().any(|c| {
                matches!(
                    c.kind(),
                    SyntaxKind::VariableDefine | SyntaxKind::TupleDestructure
                )
            })
        }) {
            for s in stmt {
                let previous: BTreeSet<_> = vars.keys().cloned().collect();
                lower.statement(&s, &mut vars)?;
                for (name, value) in &vars {
                    if (native_functions && !previous.contains(name))
                        || collect_effects(value, &mut effect_ids)
                    {
                        effects.push(value.clone());
                    }
                }
            }
            continue;
        }
        let mut calls = Vec::new();
        visit(&root, SyntaxKind::FunctionCall, &mut calls);
        let call = calls.first().ok_or("top-level output call required")?;
        let (name, a) = args(call)?;
        let mut nested = Vec::new();
        if name == "io/write" && a.len() == 1 {
            visit(&a[0], SyntaxKind::FunctionCall, &mut nested);
        }
        if name == "io/print"
            || name == "io/write-scan"
            || (name == "io/write" && a.len() == 1 && nested.is_empty())
        {
            let value = lower.expr(call, &vars)?;
            collect_effects(&value, &mut effect_ids);
            effects.push(value);
            continue;
        }
        if name != "io/write" || a.len() != 1 || output.is_some() {
            return Err("one io/write required".into());
        }
        let mut f = Vec::new();
        visit(&a[0], SyntaxKind::FunctionCall, &mut f);
        let (name, a) = args(f.first().ok_or("text/format output required")?)?;
        if name != "text/format" || a.len() < 2 {
            return Err("text/format expects format and scalar fields".into());
        }
        let format = lower.expr(&a[0], &vars)?;
        let Op::Text(format) = format.op else {
            return Err("constant format required".into());
        };
        let values = a[1..]
            .iter()
            .map(|x| lower.expr(x, &vars))
            .collect::<Result<Vec<_>, _>>()?;
        if values.iter().any(|v| v.kind != Kind::Integer)
            || (format.matches("{}").count() + format.matches("{:i64}").count()) != values.len()
            || values.len() > 64
            || format
                .replace("{}", "")
                .replace("{:i64}", "")
                .contains(['{', '}'])
        {
            return Err("format field count/kinds".into());
        }
        output = Some((format, tuple(values)));
    }
    let (format, output) = match output {
        Some(out) => out,
        None if !effects.is_empty() => (String::new(), tuple(Vec::new())),
        None => return Err("missing native control output".into()),
    };
    let computation = analysis::infer(&lower.functions, &effects, &output)?;
    analysis::validate(
        &computation,
        &lower.functions,
        &effects,
        &output,
        native_functions,
    )?;
    Ok(Program {
        computation,
        native_functions,
        functions: lower.functions,
        output,
        format,
        argument_count: lower.argument_count,
        effects,
    })
}

#[cfg(test)]
mod application_tests {
    use super::*;
    const CATALOG: &[&str] = &["io/print", "control/while"];
    const FUNCTIONS: &str = "Touch(x<u64>) => <u64> := io/print(\"touch:{}\\n\", x)\nAdd(a<u64>, b<u64>) => <u64> := a + b\n";
    fn invoke_ids(expression: &Expr, name: &str, ids: &mut BTreeSet<usize>) {
        match &expression.op {
            Op::Invoke {
                id,
                name: called,
                arguments,
            } => {
                if called == name {
                    ids.insert(*id);
                }
                for child in arguments {
                    invoke_ids(child, name, ids);
                }
            }
            Op::Sequence { effects, output } => {
                for child in effects {
                    invoke_ids(child, name, ids);
                }
                invoke_ids(output, name, ids);
            }
            Op::Print { values, .. }
            | Op::Primitive(_, values)
            | Op::Tuple(values)
            | Op::Call(_, values) => {
                for child in values {
                    invoke_ids(child, name, ids);
                }
            }
            Op::Field(child, _) => invoke_ids(child, name, ids),
            _ => {}
        }
    }
    #[test]
    fn repeated_pipeline_holes_share_effectful_call_identity() {
        let source = format!(
            "{FUNCTIONS}result := Touch(7) |> Add(_, _)\nio/print(\"result:{{}}\\n\", result)\n"
        );
        let program = compile(&source, CATALOG).unwrap();
        let mut ids = BTreeSet::new();
        for effect in &program.effects {
            invoke_ids(effect, "Touch", &mut ids);
        }
        invoke_ids(&program.output, "Touch", &mut ids);
        // The native emitter memoizes Expr, including this unique Invoke id.
        assert_eq!(ids.len(), 1);
    }
    #[test]
    fn separately_written_effectful_calls_keep_distinct_identities() {
        let source = format!(
            "{FUNCTIONS}first := Touch(7)\nsecond := Touch(7)\nresult := Add(first, second)\nio/print(\"result:{{}}\\n\", result)\n"
        );
        let program = compile(&source, CATALOG).unwrap();
        let mut ids = BTreeSet::new();
        for effect in &program.effects {
            invoke_ids(effect, "Touch", &mut ids);
        }
        assert_eq!(ids.len(), 2);
    }
    #[test]
    fn callable_pipeline_retains_effects_inside_each_loop_invocation() {
        let source = format!(
            "{FUNCTIONS}More(x<u64>) => <bool> := x < 3\nStep(x<u64>) => <u64> := Touch(x) |> Add(_, _) |> Add(_, x + 1)\nresult := control/while(0, More, Step)\nio/print(\"result:{{}}\\n\", result)\n"
        );
        let program = compile(&source, CATALOG).unwrap();
        let mut ids = BTreeSet::new();
        invoke_ids(&program.functions["Step"].output, "Touch", &mut ids);
        assert_eq!(ids.len(), 1);
        assert!(
            program.computation.functions["Step"]
                .effects
                .contains(&analysis::Effect::Output)
        );
    }
}
