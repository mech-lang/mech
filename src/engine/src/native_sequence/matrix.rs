//! Lower ordinary shaped comprehensions, transpose, and matrix multiplication
//! to the native sequence graph. This pass follows canonical syntax nodes; it
//! does not select a formula, function name, or benchmark input.
//!
//! A coefficient-producing comprehension is retained as a shape plus a Mech
//! scalar function. Transpose swaps that shape and the coefficient indices.
//! Matrix/vector multiplication consumes the producer directly, so no dense
//! coefficient array is allocated. A native domain check at construction
//! preserves eager checked-arithmetic errors before subsequent effects. Unsupported materialized matrix/range uses
//! are rejected instead of being silently changed into sequence operations.

use super::{text, visit};
use mech_syntax::document::{
    AstNode, ComprehensionQualifierValueSyntax, FactorSyntax, MatrixComprehensionSyntax,
    RangeExpressionSyntax, SyntaxKind, SyntaxNode, VariableDefineSyntax,
};
#[cfg(test)]
use mech_syntax::document::{
    DocumentId, ParseConfig, Revision, TextSnapshot, parse_canonical_document,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone)]
struct Matrix {
    rows: String,
    columns: String,
    coefficient: String,
    transposed: bool,
    origin: usize,
}
#[derive(Clone, Copy, PartialEq)]
enum Orientation {
    Row,
    Column,
    Unknown,
}
#[derive(Clone)]
enum Binding {
    Range(String),
    Matrix(Matrix),
    Vector(Orientation),
    Other,
}
#[derive(Clone)]
enum Expression {
    Matrix(Matrix),
    Value(String, Orientation),
}
impl Expression {
    fn value(self) -> Result<String, String> {
        match self {
            Self::Value(source, _) => Ok(source),
            Self::Matrix(_) => Err("native matrices must be consumed by matrix multiplication; materialization is not yet supported".into()),
        }
    }
}

struct Lower {
    prefix: String,
    serial: usize,
    functions: Vec<String>,
    producer_count: usize,
    consumed: BTreeSet<usize>,
    pending: Vec<String>,
    scalar_functions: BTreeMap<String, SyntaxNode>,
}
impl Lower {
    fn temporary(&mut self, purpose: &str) -> String {
        let name = format!("{}{purpose}-{}", self.prefix, self.serial);
        self.serial += 1;
        name
    }
    fn unwrap(node: &SyntaxNode) -> SyntaxNode {
        let mut node = node.clone();
        loop {
            if matches!(
                node.kind(),
                SyntaxKind::Factor
                    | SyntaxKind::Variable
                    | SyntaxKind::MatrixComprehension
                    | SyntaxKind::RangeExpression
            ) {
                return node;
            }
            let children = node.children().collect::<Vec<_>>();
            if children.len() != 1 {
                return node;
            }
            node = children[0].clone();
        }
    }
    fn extent(
        &self,
        node: &SyntaxNode,
        bindings: &BTreeMap<String, Binding>,
    ) -> Result<String, String> {
        let mut node = Self::unwrap(node);
        if let Some(factor) = FactorSyntax::cast(node.clone()) {
            if factor.transpose().is_some() {
                return Err("a generator range cannot be transposed".into());
            }
            node = Self::unwrap(factor.value().ok_or("missing generator range")?.syntax());
        }
        if node.kind() == SyntaxKind::Variable {
            let name = text(&node)?.trim().to_string();
            return match bindings.get(&name) {
                Some(Binding::Range(extent)) => Ok(extent.clone()),
                _ => Err(format!(
                    "native matrix generator {name} must name a zero-based exclusive u64 range"
                )),
            };
        }
        let range = RangeExpressionSyntax::cast(node)
            .ok_or("native matrix generators require ordinary zero-based exclusive u64 ranges")?;
        let bounds = range.bounds();
        let operators = range.operators();
        if bounds.len() != 2
            || operators.len() != 1
            || text(operators[0].syntax())?.trim() != ".."
            || text(bounds[0].syntax())?.trim() != "0"
        {
            return Err(
                "native matrix generators currently require an exclusive 0..length range".into(),
            );
        }
        Ok(text(bounds[1].syntax())?.trim().to_string())
    }
    fn generator(
        &self,
        comprehension: &MatrixComprehensionSyntax,
        bindings: &BTreeMap<String, Binding>,
    ) -> Result<(String, String, SyntaxNode), String> {
        let qualifiers = comprehension.qualifiers();
        if qualifiers.len() != 1 {
            return Err("native matrix producers require one generator per nested comprehension; filters and qualifier bindings are not yet supported".into());
        }
        let ComprehensionQualifierValueSyntax::Generator(generator) =
            qualifiers[0].value().ok_or("missing matrix generator")?
        else {
            return Err("native matrix producer requires an ordinary generator".into());
        };
        let pattern = text(
            generator
                .pattern()
                .ok_or("missing generator variable")?
                .syntax(),
        )?
        .trim()
        .to_string();
        if pattern.is_empty() || !pattern.chars().all(|c| c.is_alphanumeric() || c == '-') {
            return Err("native matrix generators require a plain index identifier".into());
        }
        let extent = self.extent(
            generator
                .source()
                .ok_or("missing generator range")?
                .syntax(),
            bindings,
        )?;
        let value = comprehension
            .value()
            .ok_or("missing comprehension result")?
            .syntax()
            .clone();
        Ok((pattern, extent, value))
    }
    fn comprehension_node(node: &SyntaxNode) -> Option<(MatrixComprehensionSyntax, bool)> {
        let node = Self::unwrap(node);
        if let Some(factor) = FactorSyntax::cast(node.clone()) {
            let transposed = factor.transpose().is_some();
            let inner = Self::unwrap(factor.value()?.syntax());
            MatrixComprehensionSyntax::cast(inner).map(|c| (c, transposed))
        } else {
            MatrixComprehensionSyntax::cast(node).map(|c| (c, false))
        }
    }
    fn producer(
        &mut self,
        comprehension: &MatrixComprehensionSyntax,
        bindings: &BTreeMap<String, Binding>,
    ) -> Result<Expression, String> {
        let (outer_name, outer_extent, body) = self.generator(comprehension, bindings)?;
        if let Some((inner, inner_transposed)) = Self::comprehension_node(&body) {
            // Comprehensions concatenate horizontally. Inner column vectors
            // form a matrix; an outer transpose then gives ordinary row order.
            if !inner_transposed {
                return Err("nested native matrix comprehensions require a transposed inner vector to establish matrix rows".into());
            }
            let (inner_name, inner_extent, entry) = self.generator(&inner, bindings)?;
            if outer_name == inner_name {
                return Err("nested native matrix generators require distinct index names".into());
            }
            pure_scalar_calls(&entry, &self.scalar_functions, &mut BTreeSet::new())?;
            let mut forbidden = Vec::new();
            for kind in [SyntaxKind::MatrixComprehension, SyntaxKind::RangeExpression] {
                visit(&entry, kind, &mut forbidden);
            }
            if !forbidden.is_empty() {
                return Err("native matrix entries must be scalar arithmetic expressions".into());
            }
            let coefficient = self.temporary("coefficient");
            let output = self.temporary("entry");
            // Index parameters follow eager generator order: outer first,
            // inner second. Native domain validation therefore reports the
            // same first checked-arithmetic failure as the comprehension.
            // The untransposed horizontal concatenation is a transpose view
            // of this coefficient graph; a final source transpose cancels it.
            self.functions.push(format!("{coefficient}({outer_name}<u64>, {inner_name}<u64>) = ({output}<f64>) := {output} := {}.", text(&entry)?.trim()));
            let origin = self.producer_count;
            self.producer_count += 1;
            let rows = self.temporary("rows");
            let columns = self.temporary("columns");
            let validation = self.temporary("validation");
            self.pending.push(format!("{rows} := {outer_extent}"));
            self.pending.push(format!("{columns} := {inner_extent}"));
            self.pending.push(format!(
                "{validation} := matrix/require-domain({rows}, {columns}, {coefficient})"
            ));
            return Ok(Expression::Matrix(Matrix {
                rows: columns,
                columns: rows,
                coefficient,
                transposed: true,
                origin,
            }));
        }
        let mut variables = Vec::new();
        visit(&body, SyntaxKind::Variable, &mut variables);
        if variables
            .iter()
            .any(|n| text(n).is_ok_and(|s| s.trim() == outer_name))
        {
            return Err("native vector comprehensions currently require a scalar expression invariant across their generator".into());
        }
        pure_scalar_calls(&body, &self.scalar_functions, &mut BTreeSet::new())?;
        let value = self.expression(&body, bindings)?.value()?;
        Ok(Expression::Value(
            format!("sequence/fill({outer_extent}, {value})"),
            Orientation::Row,
        ))
    }
    fn expression(
        &mut self,
        node: &SyntaxNode,
        bindings: &BTreeMap<String, Binding>,
    ) -> Result<Expression, String> {
        if let Some(factor) = FactorSyntax::cast(node.clone()) {
            let mut value =
                self.expression(factor.value().ok_or("missing factor")?.syntax(), bindings)?;
            if factor.transpose().is_some() {
                match &mut value {
                    Expression::Matrix(matrix) => {
                        std::mem::swap(&mut matrix.rows, &mut matrix.columns);
                        matrix.transposed = !matrix.transposed;
                    }
                    Expression::Value(_, orientation) => {
                        *orientation = match *orientation {
                            Orientation::Row => Orientation::Column,
                            Orientation::Column => Orientation::Row,
                            Orientation::Unknown => {
                                return Err(
                                    "native transpose requires a known matrix or vector shape"
                                        .into(),
                                );
                            }
                        };
                    }
                }
            }
            return Ok(value);
        }
        if let Some(comprehension) = MatrixComprehensionSyntax::cast(node.clone()) {
            return self.producer(&comprehension, bindings);
        }
        if node.kind() == SyntaxKind::Variable {
            let name = text(node)?.trim().to_string();
            return match bindings.get(&name) {
                Some(Binding::Matrix(matrix)) => Ok(Expression::Matrix(matrix.clone())),
                Some(Binding::Range(_)) => Err(format!(
                    "native range {name} is only supported as a comprehension generator; range materialization is not yet supported"
                )),
                Some(Binding::Vector(orientation)) => Ok(Expression::Value(name, *orientation)),
                _ => Ok(Expression::Value(name, Orientation::Unknown)),
            };
        }
        if node.kind() == SyntaxKind::MultiplicativeExpression {
            let children = node.children().collect::<Vec<_>>();
            if children.len() >= 3 && children.len() % 2 == 1 {
                let mut left = self.expression(&children[0], bindings)?;
                for pair in children[1..].chunks(2) {
                    let operator = text(&pair[0])?.trim().to_string();
                    let right = self.expression(&pair[1], bindings)?;
                    left = match operator.as_str() {
                        "**" => {
                            let Expression::Matrix(matrix) = left else {
                                return Err("native matrix multiplication currently requires a coefficient-produced matrix on the left and a column vector on the right".into());
                            };
                            let Expression::Value(vector, orientation) = right else {
                                return Err(
                                    "native matrix/matrix multiplication is not yet supported"
                                        .into(),
                                );
                            };
                            if orientation == Orientation::Row {
                                return Err("matrix multiplication requires a column vector; transpose the row comprehension".into());
                            }
                            self.consumed.insert(matrix.origin);
                            Expression::Value(
                                format!(
                                    "{}(sequence/require-length({vector}, {}), {}, {}, \"multiply\")",
                                    if matrix.transposed {
                                        "matrix/apply-transpose"
                                    } else {
                                        "matrix/apply"
                                    },
                                    matrix.columns,
                                    matrix.rows,
                                    matrix.coefficient
                                ),
                                Orientation::Column,
                            )
                        }
                        "·" => Expression::Value(
                            format!("stats/dot({}, {})", left.value()?, right.value()?),
                            Orientation::Unknown,
                        ),
                        _ => Expression::Value(
                            format!("({} {operator} {})", left.value()?, right.value()?),
                            Orientation::Unknown,
                        ),
                    };
                }
                return Ok(left);
            }
        }
        let children = node.children().collect::<Vec<_>>();
        if children.len() == 1 {
            let child = &children[0];
            return match self.expression(child, bindings)? {
                Expression::Matrix(matrix) => Ok(Expression::Matrix(matrix)),
                Expression::Value(value, orientation) => {
                    let original = text(node)?;
                    let start = node.range().start.0 as usize;
                    let a = child.range().start.0 as usize - start;
                    let b = child.range().end.0 as usize - start;
                    Ok(Expression::Value(
                        format!("{}{}{}", &original[..a], value, &original[b..]),
                        orientation,
                    ))
                }
            };
        }
        // Preserve ordinary syntax and whitespace while replacing only child
        // expression ranges. Calls, annotations, literal text, and operator
        // spelling continue through the canonical parser unchanged.
        let original = text(node)?;
        let start = node.range().start.0 as usize;
        let mut result = String::new();
        let mut cursor = 0;
        for child in children {
            let a = child.range().start.0 as usize - start;
            let b = child.range().end.0 as usize - start;
            result.push_str(&original[cursor..a]);
            result.push_str(&self.expression(&child, bindings)?.value()?);
            cursor = b;
        }
        result.push_str(&original[cursor..]);
        Ok(Expression::Value(result, Orientation::Unknown))
    }
    fn definition(
        &mut self,
        node: &SyntaxNode,
        bindings: &mut BTreeMap<String, Binding>,
        declared: &mut BTreeSet<String>,
    ) -> Result<String, String> {
        let def = VariableDefineSyntax::cast(node.clone()).ok_or("invalid definition")?;
        let name = text(def.name().ok_or("missing definition name")?.syntax())?
            .trim()
            .to_string();
        if !declared.insert(name.clone()) {
            return Err(format!("duplicate native definition {name}"));
        }
        let value = def.value().ok_or("missing definition value")?;
        let unwrapped = Self::unwrap(value.syntax());
        let range = if let Some(factor) = FactorSyntax::cast(unwrapped.clone()) {
            Self::unwrap(factor.value().ok_or("missing factor")?.syntax())
        } else {
            unwrapped
        };
        if range.kind() == SyntaxKind::RangeExpression
            || (range.kind() == SyntaxKind::Variable
                && matches!(bindings.get(text(&range)?.trim()), Some(Binding::Range(_))))
        {
            if def.mutability_marker().is_some() {
                return Err("native matrix shapes require immutable range bindings".into());
            }
            let extent = self.extent(&range, bindings)?;
            let length = self.temporary("length");
            bindings.insert(name, Binding::Range(length.clone()));
            return Ok(format!("{length} := {extent}"));
        }
        let lowered = self.expression(value.syntax(), bindings)?;
        match lowered {
            Expression::Matrix(mut matrix) => {
                if def.mutability_marker().is_some() {
                    return Err("native matrix producers require immutable definitions".into());
                }
                if self.pending.is_empty() {
                    // A matrix alias is a shape-preserving immutable binding;
                    // retain harmless scalar bindings for canonical function
                    // statement lists instead of leaving an empty statement.
                    let rows = self.temporary("rows");
                    let columns = self.temporary("columns");
                    self.pending.push(format!("{rows} := {}", matrix.rows));
                    self.pending
                        .push(format!("{columns} := {}", matrix.columns));
                    matrix.rows = rows;
                    matrix.columns = columns;
                }
                bindings.insert(name, Binding::Matrix(matrix));
                Ok(std::mem::take(&mut self.pending).join("\n"))
            }
            Expression::Value(source, orientation) => {
                bindings.insert(
                    name,
                    if orientation == Orientation::Unknown {
                        Binding::Other
                    } else {
                        Binding::Vector(orientation)
                    },
                );
                let original = text(node)?;
                let start = node.range().start.0 as usize;
                let a = value.syntax().range().start.0 as usize - start;
                let b = value.syntax().range().end.0 as usize - start;
                let value = format!("{}{}{}", &original[..a], source, &original[b..]);
                let mut statements = std::mem::take(&mut self.pending);
                statements.push(value);
                Ok(statements.join("\n"))
            }
        }
    }
    fn function(
        &mut self,
        node: &SyntaxNode,
        bindings: &BTreeMap<String, Binding>,
    ) -> Result<String, String> {
        let mut locals = bindings.clone();
        let mut declared = BTreeSet::new();
        for arg in node
            .children()
            .filter(|n| n.kind() == SyntaxKind::FunctionArg)
        {
            let name = text(
                &arg.children()
                    .find(|n| n.kind() == SyntaxKind::Identifier)
                    .ok_or("missing parameter name")?,
            )?
            .trim()
            .to_string();
            if !declared.insert(name.clone()) {
                return Err(format!("duplicate native parameter {name}"));
            }
            let annotation = text(
                &arg.children()
                    .find(|n| n.kind() == SyntaxKind::KindAnnotation)
                    .ok_or("missing parameter kind")?,
            )?
            .replace(' ', "");
            let binding = if matches!(annotation.as_str(), "<[f64]>" | "<[f64]:_>") {
                Binding::Vector(Orientation::Column)
            } else {
                Binding::Other
            };
            locals.insert(name, binding);
        }
        let mut statements = Vec::new();
        visit(node, SyntaxKind::Statement, &mut statements);
        let mut edits = Vec::new();
        for statement in statements {
            let Some(def) = statement
                .children()
                .find(|n| n.kind() == SyntaxKind::VariableDefine)
            else {
                continue;
            };
            let replacement = self
                .definition(&def, &mut locals, &mut declared)?
                .replace('\n', "; ");
            edits.push((
                def.range().start.0 as usize,
                def.range().end.0 as usize,
                replacement,
            ));
        }
        replace_ranges(&text(node)?, node.range().start.0 as usize, edits)
    }
}
fn pure_scalar_calls(
    node: &SyntaxNode,
    functions: &BTreeMap<String, SyntaxNode>,
    active: &mut BTreeSet<String>,
) -> Result<(), String> {
    if node.kind() == SyntaxKind::FunctionCall {
        let name = node
            .children()
            .find(|n| n.kind() == SyntaxKind::Identifier)
            .ok_or("missing scalar function")?;
        let name = text(&name)?;
        let name = name.trim();
        if !matches!(name, "number/to-f64" | "math/sqrt") {
            let body = functions.get(name).ok_or_else(|| format!(
                "native comprehension entry requires pure scalar arithmetic; unsupported operation {name}"
            ))?;
            if !active.insert(name.to_string()) {
                return Err("recursive native matrix coefficient unsupported".into());
            }
            // Accept source helpers only after checking their transitive call
            // graph. Native callable admission subsequently checks scalar kinds,
            // mutation, captures and every lowered operation before execution.
            pure_scalar_calls(body, functions, active)?;
            active.remove(name);
        }
    }
    for child in node.children() {
        pure_scalar_calls(&child, functions, active)?;
    }
    Ok(())
}
fn replace_ranges(
    source: &str,
    base: usize,
    mut edits: Vec<(usize, usize, String)>,
) -> Result<String, String> {
    edits.sort_by_key(|(a, _, _)| *a);
    let mut result = String::new();
    let mut cursor = 0;
    for (a, b, replacement) in edits {
        let (a, b) = (a - base, b - base);
        if a < cursor || b > source.len() {
            return Err("overlapping native matrix syntax edits".into());
        }
        result.push_str(&source[cursor..a]);
        result.push_str(&replacement);
        cursor = b;
    }
    result.push_str(&source[cursor..]);
    Ok(result)
}

/// Called after application syntax has been normalized. Canonical sequence
/// programs without array syntax pass through byte-for-byte.
#[cfg(test)]
pub(super) fn normalize(source: &str) -> Result<String, String> {
    let syntax = super::ParsedSource::parse_syntax(source)?;
    normalize_parsed(source, &syntax)
}
pub(super) fn normalize_parsed(source: &str, syntax: &SyntaxNode) -> Result<String, String> {
    if !source.contains("**") && !source.contains('·') && !source.contains("<-") {
        return Ok(source.to_string());
    }
    let mut roots = Vec::new();
    visit(syntax, SyntaxKind::MechCodeAlt, &mut roots);
    let mut scalar_functions = BTreeMap::new();
    for root in &roots {
        let Some(owner) = root.children().next() else {
            continue;
        };
        if owner.kind() == SyntaxKind::FunctionDefine {
            let name = text(
                &owner
                    .children()
                    .find(|n| n.kind() == SyntaxKind::Identifier)
                    .ok_or("missing scalar function name")?,
            )?;
            let body = owner
                .children()
                .find(|n| n.kind() == SyntaxKind::FunctionDefineStatements)
                .ok_or("missing scalar function body")?;
            if scalar_functions
                .insert(name.trim().to_string(), body)
                .is_some()
            {
                return Err("duplicate native function".into());
            }
        }
    }
    let mut prefix = "matrix-temp-".to_string();
    while source.contains(&prefix) {
        prefix.push_str("local-");
    }
    let mut lower = Lower {
        prefix,
        serial: 0,
        functions: Vec::new(),
        producer_count: 0,
        consumed: BTreeSet::new(),
        pending: Vec::new(),
        scalar_functions,
    };
    let mut bindings = BTreeMap::new();
    let mut declared = BTreeSet::new();
    let mut edits = Vec::new();
    let mut functions = Vec::new();
    for root in roots {
        let Some(owner) = root.children().next() else {
            continue;
        };
        if owner.kind() == SyntaxKind::FunctionDefine {
            functions.push(owner);
        } else if owner.kind() == SyntaxKind::Statement {
            if let Some(def) = owner
                .children()
                .find(|n| n.kind() == SyntaxKind::VariableDefine)
            {
                edits.push((
                    def.range().start.0 as usize,
                    def.range().end.0 as usize,
                    lower.definition(&def, &mut bindings, &mut declared)?,
                ));
            }
        } else if owner.kind() == SyntaxKind::Expression {
            edits.push((
                owner.range().start.0 as usize,
                owner.range().end.0 as usize,
                {
                    let value = lower.expression(&owner, &bindings)?.value()?;
                    let mut statements = std::mem::take(&mut lower.pending);
                    statements.push(value);
                    statements.join("\n")
                },
            ));
        }
    }
    // Named functions have document scope. Resolve their immutable global
    // producers after collecting top-level declarations, while ordinary
    // executable expressions still use definition order.
    for function in functions {
        edits.push((
            function.range().start.0 as usize,
            function.range().end.0 as usize,
            lower.function(&function, &bindings)?,
        ));
    }
    if lower.consumed.len() != lower.producer_count {
        return Err("unused native matrix producer: materialized construction is not yet supported, and eliminating an unused producer could hide coefficient errors".into());
    }
    let normalized = replace_ranges(source, 0, edits)?;
    if lower.functions.is_empty() {
        Ok(normalized)
    } else {
        Ok(format!("{}\n{normalized}", lower.functions.join("\n")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn source(body: &str) -> String {
        super::super::application::normalize(body).unwrap()
    }
    #[test]
    fn ordinary_sequence_source_is_unchanged() {
        let s = "x := sequence/fill(3, 1.0)\n";
        assert_eq!(normalize(s).unwrap(), s);
    }
    #[test]
    fn row_order_and_rectangular_transpose_are_explicit() {
        let s = source(
            "rows := 0..3\ncolumns := 0..2\nM := [[number/to-f64(10 * i + j) | j <- columns]' | i <- rows]'\nx := [1.0 | j <- columns]'\ny := M ** x\nz := M' ** y\nr := y · y\nio/write(number/format(r, 2))\n",
        );
        let n = normalize(&s).unwrap();
        assert!(n.contains("(i<u64>, j<u64>)"), "{n}");
        assert!(n.contains("matrix/apply(sequence/require-length(x,"), "{n}");
        assert!(
            n.contains("matrix/apply-transpose(sequence/require-length(y,"),
            "{n}"
        );
        assert!(n.contains("stats/dot(y, y)"), "{n}");
        assert!(!n.contains("<-") && !n.contains("**"), "{n}");
    }
    #[test]
    fn functions_capture_shape_and_shadow_outer_names() {
        let s = source(
            "indices := 0..3\nA := [[1.0 / number/to-f64(i + j + 1) | j <- indices]' | i <- indices]'\nstep(x<[f64]>) => <[f64]> := A' ** (A ** x)\nseed := [1.0 | i <- indices]'\nv := sequence/iterate(seed, 2, step)\nio/write(number/format(v, 2))\n",
        );
        let n = normalize(&s).unwrap();
        assert!(n.contains("matrix/apply("), "{n}");
        assert!(n.contains("matrix/apply-transpose("), "{n}");
        assert!(n.contains("\"multiply\""), "{n}");
        assert!(n.contains("1.0 / number/to-f64(i + j + 1)"), "{n}");
    }
    #[test]
    fn construction_validation_precedes_later_effects_and_empty_iterations() {
        let s = source(
            "indices := 0..3\nA := [[number/to-f64(10 / (i + j)) | j <- indices]' | i <- indices]'\nio/write(\"after construction\")\nstep(x<[f64]>) => <[f64]> := A ** x\nseed := [1.0 | i <- indices]'\nv := sequence/iterate(seed, 0, step)\n",
        );
        let n = normalize(&s).unwrap();
        let validation = n.find(" := matrix/require-domain(").unwrap();
        assert!(validation < n.find("io/write(").unwrap(), "{n}");
        assert!(validation < n.find("sequence/iterate(").unwrap(), "{n}");
        assert_eq!(n.matches("matrix/require-domain(").count(), 1, "{n}");
    }
    #[test]
    fn inline_producer_construction_is_validated_before_the_consumer() {
        let s = "x := [1.0 | k <- 0..2]'\ny := [[number/to-f64(i + 2 * j) | j <- 0..2]' | i <- 0..3]' ** x\n";
        let n = normalize(s).unwrap();
        assert!(
            n.find("matrix/require-domain(").unwrap() < n.find("y := matrix/apply(").unwrap(),
            "{n}"
        );
        assert!(
            parse_canonical_document(
                TextSnapshot::new(DocumentId(1), Revision(0), n.as_str()).unwrap(),
                ParseConfig::default()
            )
            .diagnostics
            .is_empty(),
            "{n}"
        );
    }
    #[test]
    fn aliases_preserve_shape_and_transpose_cancels() {
        let s = "rows := 0..3\ncolumns := 0..2\nother-columns := columns\nA := [[number/to-f64(i + 2 * j) | j <- other-columns]' | i <- rows]'\nB := A'\nC := B'\nx := [1.0 | k <- columns]'\ny := C ** x\n";
        let n = normalize(s).unwrap();
        assert_eq!(n.matches("matrix/require-domain(").count(), 1, "{n}");
        assert!(n.contains("y := matrix/apply("), "{n}");
        assert!(
            parse_canonical_document(
                TextSnapshot::new(DocumentId(1), Revision(0), n.as_str()).unwrap(),
                ParseConfig::default()
            )
            .diagnostics
            .is_empty(),
            "{n}"
        );
    }
    #[test]
    fn scalar_parentheses_and_quoted_markers_are_retained() {
        let s = "x := 2.0 * (3.0 + 4.0)\nlabel := \"** · <-\"\n";
        let n = normalize(s).unwrap();
        assert!(n.contains("(3.0 + 4.0)"), "{n}");
        assert!(n.contains("label := \"** · <-\""), "{n}");
    }
    #[test]
    fn function_parameter_shadows_the_global_matrix() {
        let s = source(
            "A := [[1.0 | j <- 0..2]' | i <- 0..3]'\nf(A<[f64]>) => <f64> := A · A\nx := [1.0 | k <- 0..2]'\ny := A ** x\n",
        );
        let n = normalize(&s).unwrap();
        assert!(n.contains("stats/dot(A, A)"), "{n}");
    }
    #[test]
    fn named_functions_can_reference_later_immutable_producers() {
        let s = source(
            "step(x<[f64]>) => <[f64]> := A' ** (A ** x)\nindices := 0..3\nA := [[1.0 / number/to-f64(i + j + 1) | j <- indices]' | i <- indices]'\nseed := [1.0 | i <- indices]'\nv := sequence/iterate(seed, 2, step)\n",
        );
        let n = normalize(&s).unwrap();
        assert!(n.contains("matrix/apply("), "{n}");
        assert!(n.contains("matrix/apply-transpose("), "{n}");
    }
    #[test]
    fn coefficient_helpers_resolve_forward_and_transitive_calls() {
        let s = source(
            "A := [[coefficient(i, j) | j <- 0..2]' | i <- 0..3]'\ncoefficient(i<u64>, j<u64>) = (out<f64>) := out := 1.0 / denominator(i, j).\ndenominator(i<u64>, j<u64>) = (out<f64>) := out := number/to-f64(i + j + 1).\nx := [1.0 | j <- 0..2]'\ny := A ** x\n",
        );
        let n = normalize(&s).unwrap();
        assert!(n.contains("coefficient(i, j)"), "{n}");
        assert!(n.contains("matrix/require-domain("), "{n}");
        assert!(n.contains("matrix/apply("), "{n}");
    }
    #[test]
    fn helper_calls_do_not_hide_impurity_or_recursion() {
        for body in [
            "out := number/to-f64(io/argument-u64(1)).",
            "out := coefficient(i, j).",
            "out := second(i, j).\nsecond(i<u64>, j<u64>) = (out<f64>) := out := coefficient(i, j).",
        ] {
            let s = source(&format!(
                "coefficient(i<u64>, j<u64>) = (out<f64>) := {body}\nA := [[coefficient(i, j) | j <- 0..2]' | i <- 0..3]'\nx := [1.0 | j <- 0..2]'\ny := A ** x\n"
            ));
            assert!(normalize(&s).is_err(), "{s}");
        }
    }
    #[test]
    fn duplicate_virtual_bindings_are_rejected_before_erasure() {
        for s in [
            "indices := 0..3\nindices := 0..4\nx := [1.0 | i <- indices]'\n",
            "A := [[1.0 | j <- 0..2]' | i <- 0..3]'\nA := [[2.0 | j <- 0..2]' | i <- 0..3]'\nx := [1.0 | k <- 0..2]'\ny := A ** x\n",
            "x := [1.0 | k <- 0..2]'\nf(i<f64>, i<f64>) = (out<f64>) := out := i + i.\n",
        ] {
            assert!(normalize(s).is_err(), "{s}");
        }
    }
    #[test]
    fn reject_materialization_shape_errors_and_impure_coefficients() {
        for s in [
            "a := [[1.0 | j <- 0..2]' | i <- 0..3]'\nio/write(\"unused\")\n",
            "x := [1.0 | j <- 0..2]\na := [[1.0 | j <- 0..2]' | i <- 0..3]'\ny := a ** x\n",
            "a := [[number/to-f64(io/argument-u64(1)) | j <- 0..2]' | i <- 0..3]'\nx := [1.0 | j <- 0..2]'\ny := a ** x\n",
            "a := [[1.0 | j <- 1..2]' | i <- 0..3]'\nx := [1.0 | j <- 0..2]'\ny := a ** x\n",
        ] {
            assert!(normalize(s).is_err(), "{s}");
        }
    }
}
