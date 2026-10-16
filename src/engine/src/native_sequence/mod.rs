//! Typed canonical-source IR for experimental native sequence compilation.
//! Standard-library signatures are supplied by the distribution; backends
//! consume this graph rather than recognizing benchmark source text.
mod application;
mod captures;
pub mod control;
pub mod grid;
mod matrix;
mod ports;
pub use ports::{
    NativeResourceIntent, NativeResourceRequirement, PARSE_U64_OP, RESOURCE_READ_OP,
    RESOURCE_SEND_OP,
};

use mech_syntax::document::{
    AnyCallArgumentSyntax, AstNode, CanonicalOperator, DocumentId, FunctionCallSyntax,
    IntegerLiteralSyntax, NumberSyntax, OperatorSyntax, ParseConfig, Revision, StringLiteralSyntax,
    SyntaxKind, SyntaxNode, TextSnapshot, VariableDefineSyntax, parse_canonical_document,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Constant {
    Text(String),
    Integer(u64),
    Float(u64),
    Function(Box<Callable>),
    Parameter(usize, SequenceKind),
    Capture(usize, SequenceKind),
}
impl Constant {
    pub fn kind(&self) -> SequenceKind {
        match self {
            Self::Text(_) => SequenceKind::Text,
            Self::Integer(_) => SequenceKind::Integer,
            Self::Float(_) => SequenceKind::Float,
            Self::Function(_) => SequenceKind::Function,
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
/// Operation and value categories admitted by the native collection IR.
/// Core `ResolvedType` and canonical schemas remain semantic type authorities.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SequenceKind {
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
    pub kinds: Vec<SequenceKind>,
    pub windows: BTreeMap<usize, (u8, u8)>,
    pub functions: BTreeMap<String, usize>,
}

/// Typed standard-library operation declarations shared with native backends.
#[derive(Clone, Copy)]
pub enum OutputKind {
    Fixed(SequenceKind),
    Input(usize),
    Numeric,
    Formatted,
}
#[derive(Clone, Copy)]
pub struct OperationSignature {
    pub name: &'static str,
    pub op: u64,
    pub inputs: &'static [&'static [SequenceKind]],
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
    counted_application: bool,
}
impl ParsedSource {
    pub fn parse(source: &str) -> Result<Self, String> {
        let normalized = application::normalize_with_metadata(source)?;
        let source = normalized.source;
        let syntax = Self::parse_syntax(&source)?;
        let parsed = Self {
            source,
            syntax,
            counted_application: normalized.counted,
        };
        if parsed.counted_application {
            let mut functions = Vec::new();
            visit(&parsed.syntax, SyntaxKind::FunctionDefine, &mut functions);
            if functions.iter().any(|function| {
                function.children().any(|child| {
                    child.kind() == SyntaxKind::Identifier
                        && child.text().is_ok_and(|name| name == "sequence/iterate")
                })
            }) {
                return Err("counted applications cannot coexist with a source function named sequence/iterate; rename that function to avoid shadowing the iteration intrinsic".into());
            }
        }
        Ok(parsed)
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
    /// Activated source must use checked sequence admission exclusively. Even
    /// an unbound port attempt must not fall back to a trusted legacy frontend.
    pub fn has_activated_ports(&self) -> bool {
        fn activated(node: &SyntaxNode) -> bool {
            matches!(
                node.kind(),
                SyntaxKind::ModuleImportContextAlias
                    | SyntaxKind::ContextSend
                    | SyntaxKind::PrefixedContextPath
            ) || node.children().any(|child| activated(&child))
        }
        activated(&self.syntax)
    }

    fn matrix_syntax(&self, catalog: Catalog) -> Result<SyntaxNode, String> {
        let imports = ports::SourceBindings::collect(&self.syntax, catalog)?;
        let source = imports.canonical_calls(&self.source, &self.syntax)?;
        let syntax = if source == self.source {
            self.syntax.clone()
        } else {
            Self::parse_syntax(&source)?
        };
        let normalized = matrix::normalize_parsed(&source, &syntax)?;
        if normalized == source {
            Ok(syntax)
        } else {
            Self::parse_syntax(&normalized)
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
            // Keep the existing decimal/scientific Float path. Attached
            // integer kind suffixes use their canonical typed owner below.
            let float_spelling =
                source.contains('.') || source.contains('e') || source.contains('E');
            if !float_spelling
                && let Some(real) =
                    NumberSyntax::cast(node.clone()).and_then(|number| number.real())
            {
                if let Some(typed) = real
                    .value()
                    .and_then(IntegerLiteralSyntax::cast)
                    .and_then(|integer| integer.typed())
                {
                    let digits = text(
                        typed
                            .digits()
                            .ok_or("missing typed integer digits")?
                            .syntax(),
                    )?
                    .replace('_', "");
                    let suffix = text(
                        typed
                            .suffix()
                            .ok_or("missing numeric kind suffix")?
                            .syntax(),
                    )?;
                    return match suffix.as_str() {
                        "u64" if !real.is_negated() => digits
                            .parse::<u64>()
                            .map(Constant::Integer)
                            .map_err(|_| "invalid u64 literal".into()),
                        "f64" => {
                            let mut value =
                                digits.parse::<f64>().map_err(|_| "invalid f64 literal")?;
                            if real.is_negated() {
                                value = -value;
                            }
                            if !value.is_finite() {
                                return Err("non-finite literal".into());
                            }
                            Ok(Constant::Float(value.to_bits()))
                        }
                        "u64" => Err("negative u64 literal".into()),
                        _ => Err("native sequence numeric suffix must be u64 or f64".into()),
                    };
                }
            }
            if float_spelling {
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
    inputs: Vec<(String, SequenceKind)>,
    output: (String, SequenceKind),
    body: SyntaxNode,
}
fn parameter(node: &SyntaxNode) -> Result<(String, SequenceKind), String> {
    let name = node
        .children()
        .find(|n| n.kind() == SyntaxKind::Identifier)
        .ok_or("missing parameter name")?;
    let annotation = node
        .children()
        .find(|n| n.kind() == SyntaxKind::KindAnnotation)
        .ok_or("missing parameter kind")?;
    let kind = match text(&annotation)?.replace(' ', "").as_str() {
        "<string>" => SequenceKind::Text,
        "<u64>" => SequenceKind::Integer,
        "<f64>" => SequenceKind::Float,
        "<{u64:u64}>" => SequenceKind::Histogram,
        "<[u8]:_>" | "<[u8]>" => SequenceKind::Bytes,
        "<[u64]:_>" | "<[u64]>" => SequenceKind::Integers,
        "<[f64]:_>" | "<[f64]>" => SequenceKind::Numbers,
        "<[string]:_>" | "<[string]>" => SequenceKind::Texts,
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
    let syntax = parsed.matrix_syntax(catalog)?;
    let bindings = ports::SourceBindings::collect(&syntax, catalog)?;
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
    bindings.append_constants(&mut program);
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
            program.kinds.push(SequenceKind::Integer);
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
    bindings.validate_function_names(functions.keys())?;
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
        bindings: &ports::SourceBindings,
        active: &mut Vec<String>,
    ) -> Result<usize, String> {
        if let Some(id) = literals.get(&(node.range().start.0, node.range().end.0)) {
            return Ok(*id);
        }
        if node.kind() == SyntaxKind::FunctionCall {
            let call = FunctionCallSyntax::cast(node).ok_or("invalid call")?;
            let name = text(call.function().ok_or("missing function")?.syntax())?;
            let name = bindings.resolve_name(&name).to_string();
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
                        bindings,
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
                        bindings,
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
            if matches!(op, RESOURCE_READ_OP | RESOURCE_SEND_OP) {
                return Err("resource operations require canonical activated ports".into());
            }
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
                    bindings,
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
                    vec![SequenceKind::Text, SequenceKind::Bytes]
                } else if op == 39 {
                    vec![p.kinds[args[0]]]
                } else if op == 73 || input_kinds == [SequenceKind::Integer, SequenceKind::Integer]
                {
                    vec![SequenceKind::Integer, SequenceKind::Integer]
                } else {
                    vec![SequenceKind::Float, SequenceKind::Float]
                };
                let output_kind = if op == 42 {
                    SequenceKind::Text
                } else if op == 39 {
                    p.kinds[args[0]]
                } else {
                    SequenceKind::Float
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
                    p.kinds.push(SequenceKind::CapturedValues);
                    for value in &captured_values[initial..] {
                        let id = p.kinds.len();
                        p.nodes.push(Node {
                            op: 75,
                            args: vec![environment, *value],
                            kernel: None,
                        });
                        p.kinds.push(SequenceKind::CapturedValues);
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
                    if p.kinds[args[0]] == SequenceKind::Integer {
                        SequenceKind::Float
                    } else {
                        SequenceKind::Numbers
                    }
                }
                OutputKind::Formatted => {
                    if matches!(
                        p.kinds[args[0]],
                        SequenceKind::Integer | SequenceKind::Float
                    ) {
                        SequenceKind::Text
                    } else {
                        SequenceKind::Texts
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
            let variable = mech_syntax::document::VariableSyntax::cast(node.clone())
                .ok_or("invalid variable")?;
            if let Some(request) = bindings.read(&variable)? {
                if !active.is_empty() {
                    return Err("native callable cannot read resources".into());
                }
                let args = request.arguments(p)?;
                let id = p.kinds.len();
                p.nodes.push(Node {
                    op: RESOURCE_READ_OP,
                    args,
                    kernel: None,
                });
                p.kinds.push(SequenceKind::Text);
                return Ok(id);
            }
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
                    bindings,
                    active,
                )?;
                for pair in children[1..].chunks(2) {
                    // Canonical operation identity owns aliases such as ×/÷;
                    // source spelling must not select a different native path.
                    let mut op = match OperatorSyntax::cast(pair[0].clone())
                        .and_then(|operator| operator.semantic())
                    {
                        Some(CanonicalOperator::Multiply) => 20,
                        Some(CanonicalOperator::Divide) => 21,
                        Some(CanonicalOperator::Add) => 22,
                        Some(CanonicalOperator::Subtract) => 23,
                        _ => return Err("unsupported numeric operator".into()),
                    };
                    let mut rhs = lower(
                        pair[1].clone(),
                        p,
                        vars,
                        literals,
                        functions,
                        catalog,
                        bindings,
                        active,
                    )?;
                    let concatenate = op == 22
                        && p.kinds[value] == SequenceKind::Text
                        && p.kinds[rhs] == SequenceKind::Text;
                    if concatenate {
                        op = 27;
                    }
                    let integer = p.kinds[value] == SequenceKind::Integer
                        && p.kinds[rhs] == SequenceKind::Integer;
                    if !integer
                        && !concatenate
                        && (!matches!(p.kinds[value], SequenceKind::Float | SequenceKind::Numbers)
                            || !matches!(p.kinds[rhs], SequenceKind::Float | SequenceKind::Numbers))
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
                    let output = if concatenate {
                        SequenceKind::Text
                    } else if integer {
                        SequenceKind::Integer
                    } else if p.kinds[value] == SequenceKind::Float
                        && p.kinds[rhs] == SequenceKind::Float
                    {
                        SequenceKind::Float
                    } else {
                        SequenceKind::Numbers
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
            bindings,
            active,
        )
    }
    // Functions used as arguments are compiled into closed, typed graphs. The
    // runtime never evaluates their nodes; native backends compile their bodies.
    fn references(
        node: &SyntaxNode,
        functions: &BTreeMap<String, SourceFunction>,
        catalog: Catalog,
        bindings: &ports::SourceBindings,
        visited: &mut BTreeSet<String>,
        out: &mut BTreeSet<String>,
    ) -> Result<(), String> {
        if let Some(call) = FunctionCallSyntax::cast(node.clone()) {
            let name = text(call.function().ok_or("missing function")?.syntax())?;
            let name = bindings.resolve_name(&name).to_string();
            if let Some(signature) = catalog.iter().find(|s| s.name == name) {
                let args = call.arguments().ok_or("missing arguments")?.arguments();
                for (arg, kinds) in args.into_iter().zip(signature.inputs) {
                    if kinds.contains(&SequenceKind::Function) {
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
                    references(&function.body, functions, catalog, bindings, visited, out)?;
                }
            }
        }
        for child in node.children() {
            references(&child, functions, catalog, bindings, visited, out)?;
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
        bindings: &ports::SourceBindings,
        active: &mut Vec<String>,
    ) -> Result<(), String> {
        let mut refs = BTreeSet::new();
        references(
            node,
            functions,
            catalog,
            bindings,
            &mut BTreeSet::new(),
            &mut refs,
        )?;
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
                bindings,
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
                    bindings,
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
                    bindings,
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
            if graph.nodes.iter().any(|n| {
                matches!(
                    n.op,
                    1 | 8 | 32 | 45 | 52 | RESOURCE_READ_OP | RESOURCE_SEND_OP
                )
            }) {
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
            p.kinds.push(SequenceKind::Function);
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
        &bindings,
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
        if owner.kind() == SyntaxKind::ModuleImport {
            // Every import was structurally resolved before lowering.
            continue;
        }
        if let Some(send) =
            mech_syntax::document::ContextSendSyntax::cast(owner.clone()).or_else(|| {
                owner
                    .children()
                    .find_map(mech_syntax::document::ContextSendSyntax::cast)
            })
        {
            let request = bindings.send(&send)?;
            let value = lower(
                send.expression()
                    .ok_or("missing resource payload")?
                    .syntax()
                    .clone(),
                &mut program,
                &variables,
                &literal_ids,
                &functions,
                catalog,
                &bindings,
                &mut Vec::new(),
            )?;
            if program.kinds[value] != SequenceKind::Text {
                return Err("resource send requires text payload".into());
            }
            let mut args = request.arguments(&program)?;
            args.push(value);
            program.nodes.push(Node {
                op: RESOURCE_SEND_OP,
                args,
                kernel: None,
            });
            program.kinds.push(SequenceKind::Unit);
        } else if owner.kind() == SyntaxKind::Statement {
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
                &bindings,
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
                &bindings,
                &mut Vec::new(),
            )?;
            if program.kinds[value] != SequenceKind::Unit {
                return Err("bare expressions must return unit".into());
            }
        }
    }
    if program.nodes.is_empty() {
        return Err("source contains no admitted executable operations".into());
    }
    program.resource_requirements()?;
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
                self.kinds[base + index] = SequenceKind::Unit;
            }
        }
        self
    }

    /// Canonicalize constants and handles while preserving source evaluation
    /// order, including failures from otherwise immutable computations.
    /// Kernels can still launch background work; consumers resolve it as needed.
    /// Hoisting across statements requires a separate proof of totality.
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
        let mut previous_node = None;
        for node in &self.nodes {
            let mut parents: BTreeSet<_> = node
                .args
                .iter()
                .filter_map(|id| id.checked_sub(old_base))
                .collect();
            // Runtime domain checks are not described by the coarse output
            // kinds. Preserve every launch and synchronous failure in source
            // order until a selected operation has an explicit totality proof.
            if let Some(parent) = previous_node {
                parents.insert(parent);
            }
            previous_node = Some(dependencies.len());
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
                .find(|i| !done.contains(i) && dependencies[*i].is_subset(&done))
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
