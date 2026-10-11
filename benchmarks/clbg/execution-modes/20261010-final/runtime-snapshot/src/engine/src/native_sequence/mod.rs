//! Typed canonical-source IR for experimental native sequence compilation.
//! Standard-library signatures are supplied by the distribution; backends
//! consume this graph rather than recognizing benchmark source text.
mod application;
mod captures;
pub mod control;
pub mod grid;
mod matrix;

use mech_syntax::document::{
    AnyCallArgumentSyntax, AstNode, DocumentId, FunctionCallSyntax, ParseConfig, Revision,
    StringLiteralSyntax, SyntaxKind, SyntaxNode, TextSnapshot, VariableDefineSyntax,
    parse_canonical_document,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Constant {
    Text(String),
    Integer(u64),
    Float(u64),
    Function(Box<Callable>),
    Parameter(usize, Kind),
    Capture(usize, Kind),
}
impl Constant {
    pub fn kind(&self) -> Kind {
        match self {
            Self::Text(_) => Kind::Text,
            Self::Integer(_) => Kind::Integer,
            Self::Float(_) => Kind::Float,
            Self::Function(_) => Kind::Function,
            Self::Parameter(_, kind) | Self::Capture(_, kind) => *kind,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Callable {
    pub program: Program,
    pub parameters: Vec<usize>,
    pub output: usize,
    #[serde(default)]
    pub captures: Vec<(String, usize)>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Kind {
    Text,
    Integer,
    Bytes,
    Windows,
    Histogram,
    Unit,
    Float,
    Integers,
    Numbers,
    Texts,
    Indices,
    Function,
    Records,
    TextStream,
    ByteTable,
    CapturedValues,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Node {
    pub op: u64,
    pub args: Vec<usize>,
    pub kernel: Option<(u8, u8)>,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Program {
    pub constants: Vec<Constant>,
    pub nodes: Vec<Node>,
    pub kinds: Vec<Kind>,
    pub windows: BTreeMap<usize, (u8, u8)>,
    pub functions: BTreeMap<String, usize>,
}

/// Typed standard-library operation declarations shared with native backends.
#[derive(Clone, Copy)]
pub enum OutputKind {
    Fixed(Kind),
    Input(usize),
    Numeric,
    Formatted,
}
#[derive(Clone, Copy)]
pub struct OperationSignature {
    pub name: &'static str,
    pub op: u64,
    pub inputs: &'static [&'static [Kind]],
    pub output: OutputKind,
}
pub type Catalog = &'static [OperationSignature];

/// A canonical document shared by structural native admission attempts.
/// Application syntax is normalized once; each frontend still checks its own
/// operations and kinds. A matrix rewrite creates a new document only when it
/// changes the source, rather than reparsing each rejected execution path.
pub struct ParsedSource {
    source: String,
    syntax: SyntaxNode,
}
impl ParsedSource {
    pub fn parse(source: &str) -> Result<Self, String> {
        let source = application::normalize(source)?;
        let syntax = Self::parse_syntax(&source)?;
        Ok(Self { source, syntax })
    }
    fn parse_syntax(source: &str) -> Result<SyntaxNode, String> {
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(1), Revision(0), source).map_err(|e| format!("{e:?}"))?,
            ParseConfig::default(),
        );
        if !parsed.diagnostics.is_empty() {
            return Err(format!(
                "canonical source diagnostics: {:?}",
                parsed.diagnostics
            ));
        }
        Ok(parsed.syntax())
    }
    fn matrix_syntax(&self) -> Result<SyntaxNode, String> {
        let source = matrix::normalize_parsed(&self.source, &self.syntax)?;
        if source == self.source {
            Ok(self.syntax.clone())
        } else {
            Self::parse_syntax(&source)
        }
    }
}

fn text(node: &SyntaxNode) -> Result<String, String> {
    node.text().map_err(|e| format!("{e:?}"))
}
fn visit(node: &SyntaxNode, kind: SyntaxKind, out: &mut Vec<SyntaxNode>) {
    if node.kind() == kind {
        out.push(node.clone());
        return;
    }
    for child in node.children() {
        visit(&child, kind, out);
    }
}
fn literal(node: SyntaxNode) -> Result<Constant, String> {
    match node.kind() {
        SyntaxKind::StringLiteral => Ok(Constant::Text(
            StringLiteralSyntax::cast(node)
                .ok_or("invalid string")?
                .decoded_text()
                .ok_or("invalid string escape")?,
        )),
        SyntaxKind::Number => {
            let source = text(&node)?;
            let source = source.trim();
            if source.contains('.') || source.contains('e') || source.contains('E') {
                let value = source.parse::<f64>().map_err(|_| "invalid f64 literal")?;
                if !value.is_finite() {
                    return Err("non-finite literal".into());
                }
                Ok(Constant::Float(value.to_bits()))
            } else {
                Ok(Constant::Integer(
                    source.parse::<u64>().map_err(|_| "invalid u64 literal")?,
                ))
            }
        }
        _ => Err("unsupported literal".into()),
    }
}

use crate::numeric_optimizations::exact_binary_reciprocal as exact_reciprocal;

#[derive(Clone)]
struct SourceFunction {
    inputs: Vec<(String, Kind)>,
    output: (String, Kind),
    body: SyntaxNode,
}
fn parameter(node: &SyntaxNode) -> Result<(String, Kind), String> {
    let name = node
        .children()
        .find(|n| n.kind() == SyntaxKind::Identifier)
        .ok_or("missing parameter name")?;
    let annotation = node
        .children()
        .find(|n| n.kind() == SyntaxKind::KindAnnotation)
        .ok_or("missing parameter kind")?;
    let kind = match text(&annotation)?.replace(' ', "").as_str() {
        "<string>" => Kind::Text,
        "<u64>" => Kind::Integer,
        "<f64>" => Kind::Float,
        "<{u64:u64}>" => Kind::Histogram,
        "<[u8]:_>" | "<[u8]>" => Kind::Bytes,
        "<[u64]:_>" | "<[u64]>" => Kind::Integers,
        "<[f64]:_>" | "<[f64]>" => Kind::Numbers,
        "<[string]:_>" | "<[string]>" => Kind::Texts,
        _ => return Err("unsupported native sequence parameter kind".into()),
    };
    Ok((text(&name)?, kind))
}
fn collect_functions(root: &SyntaxNode) -> Result<BTreeMap<String, SourceFunction>, String> {
    let mut nodes = Vec::new();
    visit(root, SyntaxKind::FunctionDefine, &mut nodes);
    let mut functions = BTreeMap::new();
    for node in nodes {
        let name = text(
            &node
                .children()
                .find(|n| n.kind() == SyntaxKind::Identifier)
                .ok_or("missing function name")?,
        )?;
        let inputs = node
            .children()
            .filter(|n| n.kind() == SyntaxKind::FunctionArg)
            .map(|n| parameter(&n))
            .collect::<Result<Vec<_>, _>>()?;
        let mut parameter_names = std::collections::BTreeSet::new();
        if inputs.iter().any(|(name, _)| !parameter_names.insert(name)) {
            return Err("duplicate function parameter".into());
        }
        let body = node
            .children()
            .find(|n| n.kind() == SyntaxKind::FunctionDefineStatements)
            .ok_or("native sequence functions require statement bodies")?;
        let output_node = body
            .children()
            .find(|n| {
                matches!(
                    n.kind(),
                    SyntaxKind::FunctionOutArg | SyntaxKind::FunctionOutArgs
                )
            })
            .ok_or("missing function output")?;
        let mut outputs = Vec::new();
        visit(&output_node, SyntaxKind::FunctionArg, &mut outputs);
        if outputs.len() != 1 {
            return Err("native sequence functions require one output".into());
        }
        let output = parameter(&outputs[0])?;
        if functions
            .insert(
                name,
                SourceFunction {
                    inputs,
                    output,
                    body,
                },
            )
            .is_some()
        {
            return Err("duplicate function".into());
        }
    }
    Ok(functions)
}

/// Admits immutable definitions, calls, strings and integers from Mech's
/// canonical parser. Every unknown syntax, operation and kind is rejected.
pub fn compile(source: &str, batch: usize, catalog: Catalog) -> Result<Program, String> {
    compile_parsed(&ParsedSource::parse(source)?, batch, catalog)
}
pub fn compile_parsed(
    parsed: &ParsedSource,
    batch: usize,
    catalog: Catalog,
) -> Result<Program, String> {
    if !(1..=8192).contains(&batch) {
        return Err("batch must be 1..8192".into());
    }
    let syntax = parsed.matrix_syntax()?;
    // Constants are collected first, then native instructions reference their
    // stable value IDs. Lowering uses a temporary interleaved instruction list.
    let mut program = Program {
        constants: Vec::new(),
        nodes: Vec::new(),
        kinds: Vec::new(),
        windows: BTreeMap::new(),
        functions: BTreeMap::new(),
    };
    let mut variables = BTreeMap::new();
    fn items(node: &SyntaxNode, out: &mut Vec<SyntaxNode>) {
        if node.kind() == SyntaxKind::MechCodeAlt {
            out.push(node.clone());
            return;
        }
        for child in node.children() {
            items(&child, out);
        }
    }
    let mut all = Vec::new();
    items(&syntax, &mut all);
    if all.is_empty() {
        return Err("no executable Mech items".into());
    }
    // Precollect literals, so constants occupy the beginning of the handle
    // space and all subsequent node IDs agree with native runtime allocation.
    let mut literals = Vec::new();
    visit(&syntax, SyntaxKind::StringLiteral, &mut literals);
    visit(&syntax, SyntaxKind::Number, &mut literals);
    for literal in &literals {
        let value = self::literal(literal.clone())?;
        program.kinds.push(value.kind());
        program.constants.push(value);
    }
    let lengths: Vec<_> = program
        .constants
        .iter()
        .filter_map(|c| match c {
            Constant::Text(s) => Some(s.len() as u64),
            _ => None,
        })
        .collect();
    for length in lengths {
        let value = Constant::Integer(length);
        if !program.constants.contains(&value) {
            program.constants.push(value);
            program.kinds.push(Kind::Integer);
        }
    }
    let reciprocals: Vec<_> = program
        .constants
        .iter()
        .filter_map(|c| match c {
            Constant::Float(bits) => exact_reciprocal(*bits).map(Constant::Float),
            _ => None,
        })
        .collect();
    for value in reciprocals {
        if !program.constants.contains(&value) {
            program.kinds.push(value.kind());
            program.constants.push(value);
        }
    }
    let functions = collect_functions(&syntax)?;
    let mut globals = BTreeMap::new();
    for item in &all {
        let mut definitions = Vec::new();
        visit(item, SyntaxKind::FunctionDefine, &mut definitions);
        if !definitions.is_empty() {
            continue;
        }
        visit(item, SyntaxKind::VariableDefine, &mut definitions);
        for node in definitions {
            let definition = VariableDefineSyntax::cast(node).ok_or("global definition")?;
            globals.insert(
                text(definition.name().ok_or("global name")?.syntax())?,
                definition.value().ok_or("global value")?.syntax().clone(),
            );
        }
    }

    // Expression lowering reuses precollected literals via syntax ranges.
    let mut literal_ids = BTreeMap::new();
    for (id, literal) in literals.into_iter().enumerate() {
        literal_ids.insert((literal.range().start.0, literal.range().end.0), id);
    }
    fn lower(
        node: SyntaxNode,
        p: &mut Program,
        vars: &BTreeMap<String, usize>,
        literals: &BTreeMap<(u32, u32), usize>,
        functions: &BTreeMap<String, SourceFunction>,
        catalog: Catalog,
        active: &mut Vec<String>,
    ) -> Result<usize, String> {
        if let Some(id) = literals.get(&(node.range().start.0, node.range().end.0)) {
            return Ok(*id);
        }
        if node.kind() == SyntaxKind::FunctionCall {
            let call = FunctionCallSyntax::cast(node).ok_or("invalid call")?;
            let name = text(call.function().ok_or("missing function")?.syntax())?;
            let arguments = call.arguments().ok_or("missing arguments")?.arguments();
            if let Some(function) = functions.get(&name) {
                if active.contains(&name) {
                    return Err("recursive native sequence functions unsupported".into());
                }
                if arguments.len() != function.inputs.len() {
                    return Err(format!("{name}: incorrect arity"));
                }
                let mut locals = captures::lexical_globals(vars);
                for (argument, (parameter, kind)) in arguments.into_iter().zip(&function.inputs) {
                    let AnyCallArgumentSyntax::Positional(argument) = argument else {
                        return Err("bound arguments unsupported".into());
                    };
                    let value = lower(
                        argument.value().ok_or("missing value")?.syntax().clone(),
                        p,
                        vars,
                        literals,
                        functions,
                        catalog,
                        active,
                    )?;
                    if p.kinds[value] != *kind {
                        return Err(format!("{name}: incompatible parameter {parameter}"));
                    }
                    locals.insert(parameter.clone(), value);
                }
                active.push(name.clone());
                for statement in function
                    .body
                    .children()
                    .filter(|n| n.kind() == SyntaxKind::Statement)
                {
                    let definition = statement
                        .children()
                        .find_map(VariableDefineSyntax::cast)
                        .ok_or("function bodies require immutable definitions")?;
                    if definition.mutability_marker().is_some() {
                        return Err("mutable function definitions unsupported".into());
                    }
                    let variable =
                        text(definition.name().ok_or("missing definition name")?.syntax())?;
                    let value = lower(
                        definition
                            .value()
                            .ok_or("missing definition value")?
                            .syntax()
                            .clone(),
                        p,
                        &locals,
                        literals,
                        functions,
                        catalog,
                        active,
                    )?;
                    if locals.insert(variable, value).is_some() {
                        return Err("duplicate local definition".into());
                    }
                }
                active.pop();
                let value = *locals
                    .get(&function.output.0)
                    .ok_or("function output is undefined")?;
                if p.kinds[value] != function.output.1 {
                    return Err("function output kind differs from declaration".into());
                }
                return Ok(value);
            }
            let signature = catalog
                .iter()
                .find(|s| s.name == name)
                .ok_or_else(|| format!("unsupported library operation {name}"))?;
            let (op, inputs) = (signature.op, signature.inputs);
            if arguments.len() != inputs.len() {
                return Err(format!("{name}: incorrect arity"));
            }
            let mut args = Vec::new();
            for (argument, expected) in arguments.into_iter().zip(inputs) {
                let value = match argument {
                    AnyCallArgumentSyntax::Positional(x) => x.value(),
                    _ => return Err("bound arguments unsupported".into()),
                }
                .ok_or("missing value")?;
                let id = lower(
                    value.syntax().clone(),
                    p,
                    vars,
                    literals,
                    functions,
                    catalog,
                    active,
                )?;
                if !expected.contains(&p.kinds[id]) {
                    return Err(format!(
                        "{name} expects {expected:?}, got {:?}",
                        p.kinds[id]
                    ));
                }
                args.push(id);
            }
            if matches!(op, 35 | 36 | 39 | 42 | 73) {
                let reference = args[if op == 42 { 1 } else { 2 }];
                let Some(Constant::Function(function)) = p.constants.get(reference) else {
                    return Err("expected a compiled source function".into());
                };
                let input_kinds: Vec<_> = function
                    .parameters
                    .iter()
                    .map(|&i| function.program.kinds[i])
                    .collect();
                let expected = if op == 42 {
                    vec![Kind::Text, Kind::Bytes]
                } else if op == 39 {
                    vec![p.kinds[args[0]]]
                } else if op == 73 || input_kinds == [Kind::Integer, Kind::Integer] {
                    vec![Kind::Integer, Kind::Integer]
                } else {
                    vec![Kind::Float, Kind::Float]
                };
                let output_kind = if op == 42 {
                    Kind::Text
                } else if op == 39 {
                    p.kinds[args[0]]
                } else {
                    Kind::Float
                };
                if input_kinds != expected || function.program.kinds[function.output] != output_kind
                {
                    return Err(format!("{name}: incompatible source function signature"));
                }
                if matches!(op, 35 | 36 | 73) && !function.captures.is_empty() {
                    return Err(
                        "matrix coefficient captures require scalar capture lowering".into(),
                    );
                }
                let mut captured_values = Vec::new();
                for (capture_name, slot) in &function.captures {
                    let value =
                        *vars
                            .get(&captures::global_key(capture_name))
                            .ok_or_else(|| {
                                format!("captured global {capture_name} is not defined before use")
                            })?;
                    if p.kinds[value] != function.program.kinds[*slot] {
                        return Err("captured global kind mismatch".into());
                    }
                    captured_values.push(value);
                }
                let direct_limit = if op == 42 { 3 } else { 2 };
                if captured_values.len() <= direct_limit {
                    args.extend(captured_values);
                } else {
                    // Pack large immutable capture environments into one handle;
                    // ordinary operations retain the existing five-slot ABI.
                    let initial = captured_values.len().min(4);
                    let mut pack = vec![reference];
                    pack.extend_from_slice(&captured_values[..initial]);
                    let mut environment = p.kinds.len();
                    p.nodes.push(Node {
                        op: 74,
                        args: pack,
                        kernel: None,
                    });
                    p.kinds.push(Kind::CapturedValues);
                    for value in &captured_values[initial..] {
                        let id = p.kinds.len();
                        p.nodes.push(Node {
                            op: 75,
                            args: vec![environment, *value],
                            kernel: None,
                        });
                        p.kinds.push(Kind::CapturedValues);
                        environment = id;
                    }
                    args.push(environment);
                }
                if matches!(op, 35 | 36 | 73)
                    && function
                        .program
                        .nodes
                        .iter()
                        .any(|n| !matches!(n.op, 19..=23 | 38))
                {
                    return Err("matrix coefficient requires pure scalar arithmetic".into());
                }
            }
            let output = match signature.output {
                OutputKind::Fixed(kind) => kind,
                OutputKind::Input(i) => p.kinds[args[i]],
                OutputKind::Numeric => {
                    if p.kinds[args[0]] == Kind::Integer {
                        Kind::Float
                    } else {
                        Kind::Numbers
                    }
                }
                OutputKind::Formatted => {
                    if matches!(p.kinds[args[0]], Kind::Integer | Kind::Float) {
                        Kind::Text
                    } else {
                        Kind::Texts
                    }
                }
            };
            if op == 28 {
                if let Some(Constant::Text(value)) = p.constants.get(args[0]) {
                    let length = Constant::Integer(value.len() as u64);
                    return p
                        .constants
                        .iter()
                        .position(|c| c == &length)
                        .ok_or("missing folded length".into());
                }
            }
            let id = p.kinds.len();
            let kernel = if op == 5 {
                Some(*p.windows.get(&args[0]).ok_or("missing window shape")?)
            } else {
                None
            };
            if op == 4 {
                let int = |id| match p.constants.get(id) {
                    Some(Constant::Integer(x)) => Ok(*x),
                    _ => Err("window shape must be constant"),
                };
                let bits = int(args[1])?;
                let width = int(args[2])?;
                if bits == 0 || bits > 8 || width == 0 || width > 64 || bits * width > 64 {
                    return Err("invalid packed window shape".into());
                }
                p.windows.insert(id, (bits as u8, width as u8));
            }
            p.nodes.push(Node { op, args, kernel });
            p.kinds.push(output);
            return Ok(id);
        }
        if node.kind() == SyntaxKind::Variable {
            let name = text(&node)?.trim().to_string();
            return vars
                .get(&name)
                .copied()
                .or_else(|| vars.get(&captures::global_key(&name)).copied())
                .or_else(|| p.functions.get(&name).copied())
                .ok_or_else(|| format!("undefined variable {name}"));
        }
        if matches!(
            node.kind(),
            SyntaxKind::MultiplicativeExpression | SyntaxKind::AdditiveExpression
        ) {
            let children: Vec<_> = node.children().collect();
            if children.len() >= 3 && children.len() % 2 == 1 {
                let mut value = lower(
                    children[0].clone(),
                    p,
                    vars,
                    literals,
                    functions,
                    catalog,
                    active,
                )?;
                for pair in children[1..].chunks(2) {
                    let mut op = match text(&pair[0])?.trim() {
                        "*" => 20,
                        "/" => 21,
                        "+" => 22,
                        "-" => 23,
                        _ => return Err("unsupported numeric operator".into()),
                    };
                    let mut rhs = lower(
                        pair[1].clone(),
                        p,
                        vars,
                        literals,
                        functions,
                        catalog,
                        active,
                    )?;
                    let integer = p.kinds[value] == Kind::Integer && p.kinds[rhs] == Kind::Integer;
                    if !integer
                        && (!matches!(p.kinds[value], Kind::Float | Kind::Numbers)
                            || !matches!(p.kinds[rhs], Kind::Float | Kind::Numbers))
                    {
                        return Err("numeric formulas require matching u64 or f64 inputs".into());
                    }
                    if op == 21 {
                        if let Some(Constant::Float(bits)) = p.constants.get(rhs) {
                            if let Some(inverse) = exact_reciprocal(*bits) {
                                rhs = p
                                    .constants
                                    .iter()
                                    .position(|c| c == &Constant::Float(inverse))
                                    .ok_or("missing exact reciprocal")?;
                                op = 20;
                            }
                        }
                    }
                    let output = if integer {
                        Kind::Integer
                    } else if p.kinds[value] == Kind::Float && p.kinds[rhs] == Kind::Float {
                        Kind::Float
                    } else {
                        Kind::Numbers
                    };
                    let id = p.kinds.len();
                    p.nodes.push(Node {
                        op,
                        args: vec![value, rhs],
                        kernel: None,
                    });
                    p.kinds.push(output);
                    value = id;
                }
                return Ok(value);
            }
        }
        let children: Vec<_> = node.children().collect();
        if children.len() != 1 {
            return Err(format!("unsupported syntax {:?}", node.kind()));
        }
        lower(
            children[0].clone(),
            p,
            vars,
            literals,
            functions,
            catalog,
            active,
        )
    }
    // Functions used as arguments are compiled into closed, typed graphs. The
    // runtime never evaluates their nodes; native backends compile their bodies.
    fn references(
        node: &SyntaxNode,
        functions: &BTreeMap<String, SourceFunction>,
        catalog: Catalog,
        visited: &mut BTreeSet<String>,
        out: &mut BTreeSet<String>,
    ) -> Result<(), String> {
        if let Some(call) = FunctionCallSyntax::cast(node.clone()) {
            let name = text(call.function().ok_or("missing function")?.syntax())?;
            if let Some(signature) = catalog.iter().find(|s| s.name == name) {
                let args = call.arguments().ok_or("missing arguments")?.arguments();
                for (arg, kinds) in args.into_iter().zip(signature.inputs) {
                    if kinds.contains(&Kind::Function) {
                        let AnyCallArgumentSyntax::Positional(arg) = arg else {
                            return Err("bound function unsupported".into());
                        };
                        let name = text(arg.value().ok_or("missing function argument")?.syntax())?
                            .trim()
                            .to_string();
                        if !functions.contains_key(&name) {
                            return Err(format!("expected source function, got {name}"));
                        }
                        out.insert(name);
                    }
                }
            } else if let Some(function) = functions.get(&name) {
                if visited.insert(name) {
                    references(&function.body, functions, catalog, visited, out)?;
                }
            }
        }
        for child in node.children() {
            references(&child, functions, catalog, visited, out)?;
        }
        Ok(())
    }
    fn prepare(
        node: &SyntaxNode,
        p: &mut Program,
        base: &Program,
        globals: &BTreeMap<String, SyntaxNode>,
        literals: &BTreeMap<(u32, u32), usize>,
        functions: &BTreeMap<String, SourceFunction>,
        catalog: Catalog,
        active: &mut Vec<String>,
    ) -> Result<(), String> {
        let mut refs = BTreeSet::new();
        references(node, functions, catalog, &mut BTreeSet::new(), &mut refs)?;
        for name in refs {
            if p.functions.contains_key(&name) {
                continue;
            }
            if active.contains(&name) {
                return Err("recursive native callable unsupported".into());
            }
            active.push(name.clone());
            let source = &functions[&name];
            let mut graph = base.clone();
            prepare(
                &source.body,
                &mut graph,
                base,
                globals,
                literals,
                functions,
                catalog,
                active,
            )?;
            let mut parameters = Vec::new();
            let mut locals = BTreeMap::new();
            let mut captured = Vec::new();
            for (index, name) in captures::free_globals(source, functions, globals)?
                .into_iter()
                .enumerate()
            {
                let kind = captures::infer_kind(
                    globals[&name].clone(),
                    globals,
                    functions,
                    catalog,
                    &mut BTreeSet::new(),
                )?;
                let id = graph.kinds.len();
                graph.constants.push(Constant::Capture(index, kind));
                graph.kinds.push(kind);
                captured.push((name.clone(), id));
                locals.insert(captures::global_key(&name), id);
            }
            for (index, (name, kind)) in source.inputs.iter().enumerate() {
                let id = graph.kinds.len();
                graph.constants.push(Constant::Parameter(index, *kind));
                graph.kinds.push(*kind);
                parameters.push(id);
                locals.insert(name.clone(), id);
            }
            for statement in source
                .body
                .children()
                .filter(|n| n.kind() == SyntaxKind::Statement)
            {
                let def = statement
                    .children()
                    .find_map(VariableDefineSyntax::cast)
                    .ok_or("callable body requires definitions")?;
                if def.mutability_marker().is_some() {
                    return Err("mutable callable unsupported".into());
                }
                let name = text(def.name().ok_or("missing name")?.syntax())?;
                let id = lower(
                    def.value().ok_or("missing value")?.syntax().clone(),
                    &mut graph,
                    &locals,
                    literals,
                    functions,
                    catalog,
                    &mut active.clone(),
                )?;
                if locals.insert(name, id).is_some() {
                    return Err("duplicate callable local".into());
                }
            }
            let output = *locals
                .get(&source.output.0)
                .ok_or("undefined callable output")?;
            if graph.kinds[output] != source.output.1 {
                return Err("callable output kind mismatch".into());
            }
            if graph
                .nodes
                .iter()
                .any(|n| matches!(n.op, 1 | 8 | 32 | 45 | 52))
            {
                return Err("native callable must be pure and cannot perform IO".into());
            }
            let callable = Callable {
                program: graph,
                parameters,
                output,
                captures: captured,
            };
            let id = p.constants.len();
            p.constants.push(Constant::Function(Box::new(callable)));
            p.kinds.push(Kind::Function);
            p.functions.insert(name, id);
            active.pop();
        }
        Ok(())
    }
    let base = program.clone();
    prepare(
        &syntax,
        &mut program,
        &base,
        &globals,
        &literal_ids,
        &functions,
        catalog,
        &mut Vec::new(),
    )?;
    for item in all {
        if item
            .children()
            .all(|child| child.kind() == SyntaxKind::Comment)
        {
            continue;
        }
        let mut definitions = Vec::new();
        visit(&item, SyntaxKind::FunctionDefine, &mut definitions);
        if !definitions.is_empty() {
            continue;
        }
        let owners: Vec<_> = item.children().collect();
        if owners.len() != 1 {
            return Err("unsupported code item".into());
        }
        let owner = &owners[0];
        if owner.kind() == SyntaxKind::Statement {
            let statements: Vec<_> = owner.children().collect();
            if statements.len() != 1 || statements[0].kind() != SyntaxKind::VariableDefine {
                return Err("only immutable definitions are admitted statements".into());
            }
            let def =
                VariableDefineSyntax::cast(statements[0].clone()).ok_or("invalid definition")?;
            if def.mutability_marker().is_some() {
                return Err(
                    "mutable definitions are not admitted by the collection backend".into(),
                );
            }
            let name = text(def.name().ok_or("missing name")?.syntax())?;
            if variables.contains_key(&name) {
                return Err(format!("duplicate definition {name}"));
            }
            let value = lower(
                def.value().ok_or("missing expression")?.syntax().clone(),
                &mut program,
                &variables,
                &literal_ids,
                &functions,
                catalog,
                &mut Vec::new(),
            )?;
            variables.insert(captures::global_key(&name), value);
            variables.insert(name, value);
        } else {
            if owner.kind() != SyntaxKind::Expression {
                return Err("only call expressions are admitted code items".into());
            }
            let value = lower(
                owner.clone(),
                &mut program,
                &variables,
                &literal_ids,
                &functions,
                catalog,
                &mut Vec::new(),
            )?;
            if program.kinds[value] != Kind::Unit {
                return Err("bare expressions must return unit".into());
            }
        }
    }
    if program.nodes.is_empty() {
        return Err("source contains no admitted executable operations".into());
    }
    Ok(program)
}

impl Program {
    /// Fuse adjacent, single-consumer read/record/encode operations. The
    /// placeholders preserve handle IDs; adjacency preserves IO effect order.
    pub fn fuse_record_encoding(mut self) -> Self {
        let base = self.constants.len();
        let mut uses = BTreeMap::<usize, usize>::new();
        for node in &self.nodes {
            for &arg in &node.args {
                *uses.entry(arg).or_default() += 1;
            }
        }
        for i in 2..self.nodes.len() {
            if self.nodes[i].op != 3 || self.nodes[i - 1].op != 2 || self.nodes[i - 2].op != 1 {
                continue;
            }
            let record_id = base + i - 1;
            let read_id = base + i - 2;
            if self.nodes[i].args[0] != record_id
                || self.nodes[i - 1].args[0] != read_id
                || uses.get(&record_id) != Some(&1)
                || uses.get(&read_id) != Some(&1)
            {
                continue;
            }
            let args = vec![
                self.nodes[i - 1].args[1],
                self.nodes[i].args[1],
                self.nodes[i].args[2],
                self.nodes[i - 1].args[2],
            ];
            self.nodes[i].op = 12;
            self.nodes[i].args = args;
            for index in [i - 2, i - 1] {
                self.nodes[index].op = 11;
                self.nodes[index].args.clear();
                self.kinds[base + index] = Kind::Unit;
            }
        }
        self
    }

    /// Schedule independent immutable computations before blocking consumers.
    /// Reads and writes retain their original effect order. Canonical constants
    /// and handle renumbering make equivalent call graphs emit the same object.
    pub fn scheduled(self) -> Result<Self, String> {
        if self.nodes.iter().any(|node| matches!(node.op, 9..=12)) {
            return Err("schedule must precede host IO fusion".into());
        }
        let constants: Vec<_> = self
            .constants
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let mut renamed = BTreeMap::new();
        for (old, constant) in self.constants.iter().enumerate() {
            renamed.insert(old, constants.binary_search(constant).unwrap());
        }
        let old_base = self.constants.len();
        let mut dependencies = Vec::new();
        let mut previous_effect = None;
        for node in &self.nodes {
            let mut parents: BTreeSet<_> = node
                .args
                .iter()
                .filter_map(|id| id.checked_sub(old_base))
                .collect();
            if matches!(node.op, 1 | 8 | 45 | 52 | 72 | 73) {
                if let Some(parent) = previous_effect {
                    parents.insert(parent);
                }
                previous_effect = Some(dependencies.len());
            }
            dependencies.push(parents);
        }
        let mut output = Program {
            kinds: constants.iter().map(Constant::kind).collect(),
            constants,
            nodes: Vec::new(),
            windows: BTreeMap::new(),
            functions: BTreeMap::new(),
        };
        let mut done = BTreeSet::new();
        while done.len() < self.nodes.len() {
            let selected = (0..self.nodes.len())
                .filter(|i| !done.contains(i) && dependencies[*i].is_subset(&done))
                .min_by_key(|&i| {
                    let node = &self.nodes[i];
                    let width = node
                        .kernel
                        .or_else(|| self.windows.get(&(old_base + i)).copied())
                        .map_or(0, |(_, width)| width);
                    let rank = match node.op {
                        5 | 53 => 0,
                        4 | 54 => 1,
                        1..=3 => 2,
                        _ if node
                            .args
                            .iter()
                            .any(|&id| self.kinds[id] == Kind::Histogram) =>
                        {
                            3
                        }
                        _ => 4,
                    };
                    (rank, std::cmp::Reverse(width), i)
                })
                .ok_or("collection dependency cycle")?;
            let old_id = old_base + selected;
            let new_id = output.kinds.len();
            let mut node = self.nodes[selected].clone();
            for arg in &mut node.args {
                *arg = *renamed.get(arg).ok_or("unscheduled input")?;
            }
            output.nodes.push(node);
            output.kinds.push(self.kinds[old_id]);
            if let Some(shape) = self.windows.get(&old_id) {
                output.windows.insert(new_id, *shape);
            }
            renamed.insert(old_id, new_id);
            done.insert(selected);
        }
        Ok(output)
    }
}
