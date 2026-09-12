use std::collections::BTreeMap;

use mech_core::snapshot::{F64Bits, SnapshotValidationContext};
use mech_core::{
    AccessMode, AliasPolicy, ChangeDetectionPolicy, ConstantStore, ConstantStoreBuilder,
    DeliveryMode, ExternalInteraction, FloatWidth, InputPortLayout, InputPortPolicy,
    OperationContractDeclaration, OutputConstruction, OutputPortPolicy, SchemaBody, SchemaDraft,
    SchemaId, SchemaTable, SchemaTableBuilder, ShapeRule, ValueDataDraft, ValueDraft,
};
use mech_syntax::document::{
    AnyCallArgumentSyntax, AstNode, CanonicalOperator, ComprehensionQualifierValueSyntax,
    DocumentId, DocumentSyntax, ExpressionBodySyntax, ExpressionSyntax, FactorSyntax,
    FactorValueSyntax, FormulaSyntax, FsmPipeSyntax, FsmStageSyntax, KindAnnotationSyntax,
    LiteralSyntax, LiteralValueSyntax, MapSyntax, MatrixComprehensionSyntax, MatrixSyntax,
    MultiplicativeExpressionSyntax, NodeFlags, OperatorSyntax, PatternSyntax, PatternValueSyntax,
    RangeExpressionSyntax, RecordSyntax, RecursiveSyntaxNode, Revision, SetComprehensionSyntax,
    SetSyntax, SliceStemSyntax, SliceSyntax, StructureSyntax, StructureValueSyntax,
    SubscriptItemSyntax, SubscriptValueSyntax, SyntaxKind, SyntaxNode, TableSyntax,
    TableValueSyntax, TextRange, TupleStructSyntax, TupleSyntax, VariableDefineSyntax,
    VariableStemSyntax, VariableSyntax,
};

use crate::{
    ArtifactBuildContext, ArtifactBuildError, OperationReference, ProgramArtifact, SourceInput,
    SourceNode, SourceNodeOutput, SourceOutput, SourceProgram, SourceValue,
    compile_source_program_with_contracts,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceSemanticAnchor {
    pub document: DocumentId,
    pub revision: Revision,
    pub range: TextRange,
}

impl SourceSemanticAnchor {
    fn for_node(node: &SyntaxNode) -> Self {
        Self {
            document: node.source().document(),
            revision: node.source().revision(),
            range: node.range(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceSemanticNode {
    pub operation: String,
    pub role: &'static str,
    pub detail: Option<String>,
    pub anchor: SourceSemanticAnchor,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SourceSemanticMap {
    pub inputs: Box<[SourceSemanticAnchor]>,
    pub nodes: Box<[SourceSemanticNode]>,
    pub outputs: Box<[SourceSemanticAnchor]>,
}

/// The complete engine-owned input to canonical artifact construction.
pub struct CanonicalSourceProgram {
    program: SourceProgram,
    schemas: SchemaTable,
    constants: ConstantStore,
    contracts: Box<[OperationContractDeclaration]>,
    source_map: SourceSemanticMap,
}

impl CanonicalSourceProgram {
    pub const fn program(&self) -> &SourceProgram {
        &self.program
    }

    pub const fn schemas(&self) -> &SchemaTable {
        &self.schemas
    }

    pub const fn constants(&self) -> &ConstantStore {
        &self.constants
    }

    pub const fn contracts(&self) -> &[OperationContractDeclaration] {
        &self.contracts
    }

    pub const fn source_map(&self) -> &SourceSemanticMap {
        &self.source_map
    }

    pub fn compile_artifact(&self) -> Result<ProgramArtifact, ArtifactBuildError> {
        let contracts = self.contracts.iter().collect::<Vec<_>>();
        compile_source_program_with_contracts(
            &self.program,
            &mut ArtifactBuildContext::new(&self.schemas, &self.constants),
            &contracts,
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceSemanticError {
    pub code: &'static str,
    pub message: String,
    pub anchor: SourceSemanticAnchor,
}

impl core::fmt::Display for SourceSemanticError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for SourceSemanticError {}

/// Engine entry point for canonical typed source.
#[derive(Clone, Debug, Default)]
pub struct CanonicalSourceFrontend;

impl CanonicalSourceFrontend {
    pub fn compile_expression(
        &self,
        expression: &ExpressionSyntax,
    ) -> Result<CanonicalSourceProgram, SourceSemanticError> {
        reject_recovered_syntax(expression)?;
        let anchor = SourceSemanticAnchor::for_node(expression.syntax());
        let mut builder = SemanticBuilder::new(anchor);
        let output = builder.expression(expression)?;
        builder.publish("result", None, output.0, expression.syntax());
        builder.finish()
    }

    /// Compile every outermost canonical definition or expression in physical
    /// document order. S7 completes the canonical document parser; keeping the
    /// typed document entry point here fixes the engine boundary now.
    pub fn compile_document(
        &self,
        document: &DocumentSyntax,
    ) -> Result<CanonicalSourceProgram, SourceSemanticError> {
        reject_recovered_syntax(document)?;
        let anchor = SourceSemanticAnchor::for_node(document.syntax());
        let mut units = Vec::new();
        collect_document_units(document.syntax(), &mut units);
        let mut builder = SemanticBuilder::new(anchor);
        let mut last = None;
        for unit in units {
            match unit.kind() {
                SyntaxKind::VariableDefine => {
                    let definition = VariableDefineSyntax::cast(unit)
                        .expect("kind-checked variable definition cast");
                    last = Some(builder.definition(&definition)?);
                }
                SyntaxKind::Expression => {
                    let expression =
                        ExpressionSyntax::cast(unit).expect("kind-checked expression cast");
                    last = Some(builder.expression(&expression)?);
                }
                _ => unreachable!("document unit collector is closed"),
            }
        }
        let Some((value, syntax)) = last else {
            return Err(SourceSemanticError {
                code: "source-semantics/empty-document",
                message: "canonical document contains no executable source unit".to_owned(),
                anchor,
            });
        };
        builder.publish("result", None, value, &syntax);
        builder.finish()
    }
}

fn collect_document_units(node: &SyntaxNode, output: &mut Vec<SyntaxNode>) {
    if matches!(
        node.kind(),
        SyntaxKind::VariableDefine | SyntaxKind::Expression
    ) {
        output.push(node.clone());
        return;
    }
    for child in node.children() {
        collect_document_units(&child, output);
    }
}

fn reject_recovered_syntax<N: RecursiveSyntaxNode>(node: &N) -> Result<(), SourceSemanticError> {
    if node.syntax().flags().intersects(
        NodeFlags::ERROR
            | NodeFlags::MISSING
            | NodeFlags::CONTAINS_ERROR
            | NodeFlags::CONTAINS_MISSING,
    ) || !node.error_nodes().is_empty()
        || !node.missing_nodes().is_empty()
        || !node.missing_tokens().is_empty()
    {
        return Err(SourceSemanticError {
            code: "source-semantics/recovered-syntax",
            message: "source semantics require a complete canonical syntax tree".to_owned(),
            anchor: SourceSemanticAnchor::for_node(node.syntax()),
        });
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum BuiltinSchema {
    Dynamic,
    Bool,
    String,
    F64,
}

struct BuiltinSchemas {
    table: SchemaTable,
    dynamic: SchemaId,
    bool_: SchemaId,
    string: SchemaId,
    f64_: SchemaId,
}

impl BuiltinSchemas {
    fn build(anchor: SourceSemanticAnchor) -> Result<Self, SourceSemanticError> {
        let mut builder = SchemaTableBuilder::new();
        let mut insert = |body| {
            let schema = SchemaDraft {
                dimension_parameters: Box::new([]),
                body,
            }
            .finalize()
            .map_err(|error| internal(anchor, format!("invalid builtin schema: {error:?}")))?;
            builder
                .insert(schema)
                .map_err(|error| internal(anchor, format!("unable to retain schema: {error:?}")))
        };
        let dynamic = insert(SchemaBody::Dynamic)?;
        let bool_ = insert(SchemaBody::Bool)?;
        let string = insert(SchemaBody::String)?;
        let f64_ = insert(SchemaBody::FloatingPoint(FloatWidth::W64))?;
        let build = builder
            .finish()
            .map_err(|error| internal(anchor, format!("unable to finalize schemas: {error:?}")))?;
        let dynamic = build.resolve(dynamic).map_err(|error| {
            internal(
                anchor,
                format!("unable to resolve dynamic schema: {error:?}"),
            )
        })?;
        let bool_ = build.resolve(bool_).map_err(|error| {
            internal(anchor, format!("unable to resolve bool schema: {error:?}"))
        })?;
        let string = build.resolve(string).map_err(|error| {
            internal(
                anchor,
                format!("unable to resolve string schema: {error:?}"),
            )
        })?;
        let f64_ = build.resolve(f64_).map_err(|error| {
            internal(anchor, format!("unable to resolve f64 schema: {error:?}"))
        })?;
        let (table, _) = build.into_parts();
        Ok(Self {
            table,
            dynamic,
            bool_,
            string,
            f64_,
        })
    }

    fn id(&self, schema: BuiltinSchema) -> SchemaId {
        match schema {
            BuiltinSchema::Dynamic => self.dynamic,
            BuiltinSchema::Bool => self.bool_,
            BuiltinSchema::String => self.string,
            BuiltinSchema::F64 => self.f64_,
        }
    }
}

#[derive(Clone, Copy)]
enum PendingValue {
    Constant(usize),
    Input(u32),
    Node(u32),
}

struct PendingConstant {
    schema: BuiltinSchema,
    data: ValueDataDraft,
}

struct PendingNode {
    operation: OperationReference,
    inputs: Vec<PendingValue>,
    schema: BuiltinSchema,
    semantic: SourceSemanticNode,
}

struct PendingOutput {
    name: String,
    interactive_symbol: Option<String>,
    source: PendingValue,
    anchor: SourceSemanticAnchor,
}

struct SemanticBuilder {
    anchor: SourceSemanticAnchor,
    constants: Vec<PendingConstant>,
    inputs: Vec<(String, SourceSemanticAnchor)>,
    input_by_name: BTreeMap<String, u32>,
    nodes: Vec<PendingNode>,
    outputs: Vec<PendingOutput>,
    bindings: BTreeMap<String, PendingValue>,
}

impl SemanticBuilder {
    fn new(anchor: SourceSemanticAnchor) -> Self {
        Self {
            anchor,
            constants: Vec::new(),
            inputs: Vec::new(),
            input_by_name: BTreeMap::new(),
            nodes: Vec::new(),
            outputs: Vec::new(),
            bindings: BTreeMap::new(),
        }
    }

    fn required<T>(
        &self,
        value: Option<T>,
        syntax: &SyntaxNode,
        expected: &'static str,
    ) -> Result<T, SourceSemanticError> {
        value.ok_or_else(|| SourceSemanticError {
            code: "source-semantics/missing-typed-child",
            message: format!("{} requires {expected}", syntax_kind_name(syntax.kind())),
            anchor: SourceSemanticAnchor::for_node(syntax),
        })
    }

    fn expression(
        &mut self,
        expression: &ExpressionSyntax,
    ) -> Result<(PendingValue, SyntaxNode), SourceSemanticError> {
        let body = self.required(expression.body(), expression.syntax(), "an expression body")?;
        let mut value = match body {
            ExpressionBodySyntax::FsmPipe(pipe) => self.fsm_pipe(&pipe)?,
            ExpressionBodySyntax::SetComprehension(value) => self.set_comprehension(&value)?,
            ExpressionBodySyntax::MatrixComprehension(value) => {
                self.matrix_comprehension(&value)?
            }
            ExpressionBodySyntax::Range(range) => self.range(&range)?,
            ExpressionBodySyntax::Formula(formula) => self.formula(&formula)?,
        };
        let arms = expression.match_arms();
        if !arms.is_empty() {
            let mut inputs = vec![value];
            for arm in arms {
                let pattern = self.required(arm.pattern(), arm.syntax(), "a match pattern")?;
                inputs.push(self.pattern(&pattern)?);
                if let Some(guard) = arm.guard() {
                    inputs.push(self.expression(&guard)?.0);
                }
                let result = self.required(arm.value(), arm.syntax(), "a match result")?;
                inputs.push(self.expression(&result)?.0);
            }
            value = self.emit(
                "source/match",
                inputs,
                BuiltinSchema::Dynamic,
                expression.syntax(),
                "match",
                None,
            );
        }
        Ok((value, expression.syntax().clone()))
    }

    fn formula(&mut self, formula: &FormulaSyntax) -> Result<PendingValue, SourceSemanticError> {
        match formula {
            FormulaSyntax::Logic(value) => {
                self.chain(value.syntax(), value.operands(), value.operators())
            }
            FormulaSyntax::Comparison(value) => {
                self.chain(value.syntax(), value.operands(), value.operators())
            }
            FormulaSyntax::Additive(value) => {
                self.chain(value.syntax(), value.operands(), value.operators())
            }
            FormulaSyntax::Multiplicative(value) => self.multiplicative(value),
            FormulaSyntax::Power(value) => {
                self.chain(value.syntax(), value.operands(), value.operators())
            }
            FormulaSyntax::Table(value) => {
                self.chain(value.syntax(), value.operands(), value.operators())
            }
            FormulaSyntax::Set(value) => {
                self.chain(value.syntax(), value.operands(), value.operators())
            }
            FormulaSyntax::Factor(value) => self.factor(value),
        }
    }

    fn chain<O: AstNode>(
        &mut self,
        syntax: &SyntaxNode,
        operands: Vec<FormulaSyntax>,
        operators: Vec<O>,
    ) -> Result<PendingValue, SourceSemanticError> {
        if operands.len() != operators.len().saturating_add(1) {
            return Err(SourceSemanticError {
                code: "source-semantics/invalid-operator-chain",
                message: format!(
                    "operator chain has {} operands and {} operators",
                    operands.len(),
                    operators.len()
                ),
                anchor: SourceSemanticAnchor::for_node(syntax),
            });
        }
        let mut operands = operands.into_iter();
        let first = self.required(operands.next(), syntax, "a left operand")?;
        let mut value = self.formula(&first)?;
        for (operator, rhs) in operators.into_iter().zip(operands) {
            let operator = self.operator(&operator)?;
            let rhs = self.formula(&rhs)?;
            value = self.emit_operator(operator, value, rhs, syntax);
        }
        Ok(value)
    }

    fn multiplicative(
        &mut self,
        value: &MultiplicativeExpressionSyntax,
    ) -> Result<PendingValue, SourceSemanticError> {
        self.chain(value.syntax(), value.operands(), value.operators())
    }

    fn operator<O: AstNode>(&self, operator: &O) -> Result<CanonicalOperator, SourceSemanticError> {
        OperatorSyntax::cast(operator.syntax().clone())
            .and_then(|operator| operator.semantic())
            .ok_or_else(|| SourceSemanticError {
                code: "source-semantics/unknown-operator",
                message: "canonical operator has no semantic identity".to_owned(),
                anchor: SourceSemanticAnchor::for_node(operator.syntax()),
            })
    }

    fn emit_operator(
        &mut self,
        operator: CanonicalOperator,
        lhs: PendingValue,
        rhs: PendingValue,
        syntax: &SyntaxNode,
    ) -> PendingValue {
        let (name, schema) = operator_name(operator);
        self.emit(name, vec![lhs, rhs], schema, syntax, "operator", None)
    }

    fn factor(&mut self, factor: &FactorSyntax) -> Result<PendingValue, SourceSemanticError> {
        let value = self.required(factor.value(), factor.syntax(), "a factor value")?;
        let mut result = match value {
            FactorValueSyntax::Parenthetical(value) => {
                let expression = self.required(
                    value.expression(),
                    value.syntax(),
                    "a parenthesized expression",
                )?;
                self.formula(&expression)?
            }
            FactorValueSyntax::Negate(value) => {
                let operand = self.required(value.operand(), value.syntax(), "a unary operand")?;
                let operand = self.factor(&operand)?;
                self.emit(
                    "math/neg",
                    vec![operand],
                    BuiltinSchema::Dynamic,
                    value.syntax(),
                    "unary",
                    None,
                )
            }
            FactorValueSyntax::Not(value) => {
                let operand = self.required(value.operand(), value.syntax(), "a unary operand")?;
                let operand = self.factor(&operand)?;
                self.emit(
                    "logic/not",
                    vec![operand],
                    BuiltinSchema::Bool,
                    value.syntax(),
                    "unary",
                    None,
                )
            }
            FactorValueSyntax::Structure(value) => self.structure(&value)?,
            FactorValueSyntax::Literal(value) => self.literal(&value)?,
            FactorValueSyntax::Call(value) => {
                let function =
                    self.required(value.function(), value.syntax(), "a function name")?;
                let function_name = node_text(function.syntax())?;
                let arguments = self.required(
                    value.arguments(),
                    value.syntax(),
                    "a function argument list",
                )?;
                let mut inputs = Vec::new();
                let mut names = Vec::new();
                for argument in arguments.arguments() {
                    match argument {
                        AnyCallArgumentSyntax::Positional(argument) => {
                            let value = self.required(
                                argument.value(),
                                argument.syntax(),
                                "an argument value",
                            )?;
                            inputs.push(self.expression(&value)?.0);
                            names.push(String::new());
                        }
                        AnyCallArgumentSyntax::Bound(argument) => {
                            let name = self.required(
                                argument.name(),
                                argument.syntax(),
                                "a bound argument name",
                            )?;
                            let value = self.required(
                                argument.value(),
                                argument.syntax(),
                                "a bound argument value",
                            )?;
                            names.push(node_text(name.syntax())?);
                            inputs.push(self.expression(&value)?.0);
                        }
                    }
                }
                self.emit(
                    "source/call",
                    inputs,
                    BuiltinSchema::Dynamic,
                    value.syntax(),
                    "call",
                    Some(format!("{function_name}({})", names.join(","))),
                )
            }
            FactorValueSyntax::MatrixComprehension(value) => self.matrix_comprehension(&value)?,
            FactorValueSyntax::Slice(value) => self.slice(&value)?,
            FactorValueSyntax::Variable(value) => self.variable(&value)?,
        };
        if factor
            .direct_tokens()
            .iter()
            .any(|token| token.kind() == SyntaxKind::Apostrophe)
        {
            result = self.emit(
                "matrix/transpose",
                vec![result],
                BuiltinSchema::Dynamic,
                factor.syntax(),
                "postfix",
                None,
            );
        }
        Ok(result)
    }

    fn range(
        &mut self,
        range: &RangeExpressionSyntax,
    ) -> Result<PendingValue, SourceSemanticError> {
        let bounds = range.bounds();
        let operators = range.operators();
        if !matches!(bounds.len(), 2 | 3) || operators.len() + 1 != bounds.len() {
            return Err(SourceSemanticError {
                code: "source-semantics/invalid-range",
                message: "range requires two or three bounds with matching operators".to_owned(),
                anchor: SourceSemanticAnchor::for_node(range.syntax()),
            });
        }
        let values = bounds
            .iter()
            .map(|bound| self.formula(bound))
            .collect::<Result<Vec<_>, _>>()?;
        let first = self.operator(&operators[0])?;
        let name = match (first, values.len()) {
            (CanonicalOperator::RangeInclusive, 2) => "range/inclusive",
            (CanonicalOperator::RangeExclusive, 2) => "range/exclusive",
            (CanonicalOperator::RangeInclusive, 3) => "range/inclusive-increment",
            (CanonicalOperator::RangeExclusive, 3) => "range/exclusive-increment",
            _ => {
                return Err(SourceSemanticError {
                    code: "source-semantics/invalid-range-operator",
                    message: "range contains a non-range operator".to_owned(),
                    anchor: SourceSemanticAnchor::for_node(range.syntax()),
                });
            }
        };
        Ok(self.emit(
            name,
            values,
            BuiltinSchema::Dynamic,
            range.syntax(),
            "range",
            None,
        ))
    }

    fn variable(&mut self, variable: &VariableSyntax) -> Result<PendingValue, SourceSemanticError> {
        let stem = self.required(variable.stem(), variable.syntax(), "a variable stem")?;
        let (name, syntax) = match stem {
            VariableStemSyntax::Identifier(value) => {
                (node_text(value.syntax())?, value.syntax().clone())
            }
            VariableStemSyntax::Context(value) => {
                (node_text(value.syntax())?, value.syntax().clone())
            }
        };
        if let Some(value) = self.bindings.get(&name) {
            return Ok(*value);
        }
        if let Some(index) = self.input_by_name.get(&name) {
            return Ok(PendingValue::Input(*index));
        }
        let index = u32::try_from(self.inputs.len()).map_err(|_| SourceSemanticError {
            code: "source-semantics/input-identity-exhausted",
            message: "canonical input count exceeds SourceProgram identity space".to_owned(),
            anchor: SourceSemanticAnchor::for_node(&syntax),
        })?;
        self.input_by_name.insert(name.clone(), index);
        self.inputs
            .push((name, SourceSemanticAnchor::for_node(&syntax)));
        Ok(PendingValue::Input(index))
    }

    fn definition(
        &mut self,
        definition: &VariableDefineSyntax,
    ) -> Result<(PendingValue, SyntaxNode), SourceSemanticError> {
        let variable = self.required(
            definition.variable(),
            definition.syntax(),
            "a defined variable",
        )?;
        let stem = self.required(variable.stem(), variable.syntax(), "a variable stem")?;
        let name = node_text(stem.syntax())?;
        let expression = self.required(
            definition.value(),
            definition.syntax(),
            "a definition value",
        )?;
        let value = self.expression(&expression)?.0;
        let bound = self.emit(
            "source/bind",
            vec![value],
            BuiltinSchema::Dynamic,
            definition.syntax(),
            "definition",
            Some(name.clone()),
        );
        self.bindings.insert(name, bound);
        Ok((bound, definition.syntax().clone()))
    }

    fn literal(&mut self, literal: &LiteralSyntax) -> Result<PendingValue, SourceSemanticError> {
        if literal.true_token().is_some() {
            return Ok(self.constant(BuiltinSchema::Bool, ValueDataDraft::Bool(true)));
        }
        if literal.false_token().is_some() {
            return Ok(self.constant(BuiltinSchema::Bool, ValueDataDraft::Bool(false)));
        }
        let value = self.required(literal.value(), literal.syntax(), "a literal value")?;
        match value {
            LiteralValueSyntax::String(value) => {
                let source = node_text(value.syntax())?;
                let decoded = decode_string(&source).ok_or_else(|| SourceSemanticError {
                    code: "source-semantics/invalid-string-literal",
                    message: "canonical string could not be decoded".to_owned(),
                    anchor: SourceSemanticAnchor::for_node(value.syntax()),
                })?;
                Ok(self.constant(BuiltinSchema::String, ValueDataDraft::String(decoded)))
            }
            LiteralValueSyntax::Number(value) => {
                let source = node_text(value.syntax())?.replace('_', "");
                if let Ok(number) = source.parse::<f64>() {
                    Ok(self.constant(
                        BuiltinSchema::F64,
                        ValueDataDraft::F64(F64Bits::from_f64(number)),
                    ))
                } else {
                    Ok(self.emit(
                        "source/literal",
                        Vec::new(),
                        BuiltinSchema::Dynamic,
                        value.syntax(),
                        "number-literal",
                        Some(source),
                    ))
                }
            }
            LiteralValueSyntax::Empty(value) => Ok(self.emit(
                "source/empty",
                Vec::new(),
                BuiltinSchema::Dynamic,
                value.syntax(),
                "empty-literal",
                None,
            )),
            LiteralValueSyntax::Atom(value) => Ok(self.emit(
                "source/atom",
                Vec::new(),
                BuiltinSchema::Dynamic,
                value.syntax(),
                "atom-literal",
                Some(node_text(value.syntax())?),
            )),
            LiteralValueSyntax::KindAnnotation(value) => Ok(self.kind_value(&value)),
        }
    }

    fn kind_value(&mut self, kind: &KindAnnotationSyntax) -> PendingValue {
        self.emit(
            "source/kind",
            Vec::new(),
            BuiltinSchema::Dynamic,
            kind.syntax(),
            "kind-value",
            kind.syntax().text().ok(),
        )
    }

    fn structure(
        &mut self,
        structure: &StructureSyntax,
    ) -> Result<PendingValue, SourceSemanticError> {
        let value = self.required(structure.value(), structure.syntax(), "a structure value")?;
        match value {
            StructureValueSyntax::Matrix(value) => self.matrix(&value),
            StructureValueSyntax::Table(value) => self.table(&value),
            StructureValueSyntax::Map(value) => self.map(&value),
            StructureValueSyntax::Record(value) => self.record(&value),
            StructureValueSyntax::Set(value) => self.set(&value),
            StructureValueSyntax::Tuple(value) => self.tuple(&value),
            StructureValueSyntax::TupleStruct(value) => self.tuple_struct(&value),
            StructureValueSyntax::EmptyMap(value) => Ok(self.emit(
                "source/map",
                Vec::new(),
                BuiltinSchema::Dynamic,
                value.syntax(),
                "map",
                Some("0 entries".to_owned()),
            )),
            StructureValueSyntax::EmptySet(value) => Ok(self.emit(
                "set/define",
                Vec::new(),
                BuiltinSchema::Dynamic,
                value.syntax(),
                "set",
                Some("0 items".to_owned()),
            )),
        }
    }

    fn matrix(&mut self, matrix: &MatrixSyntax) -> Result<PendingValue, SourceSemanticError> {
        let rows = matrix.rows();
        let mut inputs = Vec::new();
        let mut widths = Vec::new();
        for row in rows {
            let columns = row.columns();
            widths.push(columns.len());
            for column in columns {
                let value = self.required(column.value(), column.syntax(), "a matrix value")?;
                inputs.push(self.expression(&value)?.0);
            }
        }
        Ok(self.emit(
            "source/matrix",
            inputs,
            BuiltinSchema::Dynamic,
            matrix.syntax(),
            "matrix",
            Some(format!("row-widths={widths:?}")),
        ))
    }

    fn table(&mut self, table: &TableSyntax) -> Result<PendingValue, SourceSemanticError> {
        let value = self.required(table.value(), table.syntax(), "a table presentation")?;
        let (headers, rows, syntax): (Vec<String>, Vec<Vec<ExpressionSyntax>>, SyntaxNode) =
            match value {
                TableValueSyntax::Fancy(value) => (
                    value
                        .header()
                        .into_iter()
                        .flat_map(|header| header.fields())
                        .map(|field| node_text(field.syntax()))
                        .collect::<Result<Vec<_>, _>>()?,
                    value.rows().into_iter().map(|row| row.cells()).collect(),
                    value.syntax().clone(),
                ),
                TableValueSyntax::Inline(value) => (
                    value
                        .header()
                        .into_iter()
                        .flat_map(|header| header.fields())
                        .map(|field| node_text(field.syntax()))
                        .collect::<Result<Vec<_>, _>>()?,
                    value.rows().into_iter().map(|row| row.cells()).collect(),
                    value.syntax().clone(),
                ),
                TableValueSyntax::Regular(value) => (
                    value
                        .header()
                        .into_iter()
                        .flat_map(|header| header.fields())
                        .map(|field| node_text(field.syntax()))
                        .collect::<Result<Vec<_>, _>>()?,
                    value.rows().into_iter().map(|row| row.cells()).collect(),
                    value.syntax().clone(),
                ),
            };
        let mut inputs = Vec::new();
        let mut widths = Vec::new();
        for row in rows {
            widths.push(row.len());
            for cell in row {
                inputs.push(self.expression(&cell)?.0);
            }
        }
        Ok(self.emit(
            "source/table",
            inputs,
            BuiltinSchema::Dynamic,
            &syntax,
            "table",
            Some(format!("headers={headers:?};row-widths={widths:?}")),
        ))
    }

    fn map(&mut self, map: &MapSyntax) -> Result<PendingValue, SourceSemanticError> {
        let mut inputs = Vec::new();
        for entry in map.entries() {
            let key = self.required(entry.key(), entry.syntax(), "a map key")?;
            let value = self.required(entry.value(), entry.syntax(), "a map value")?;
            inputs.push(self.expression(&key)?.0);
            inputs.push(self.expression(&value)?.0);
        }
        Ok(self.emit(
            "source/map",
            inputs,
            BuiltinSchema::Dynamic,
            map.syntax(),
            "map",
            None,
        ))
    }

    fn record(&mut self, record: &RecordSyntax) -> Result<PendingValue, SourceSemanticError> {
        let mut inputs = Vec::new();
        let mut names = Vec::new();
        for binding in record.bindings() {
            let name = self.required(binding.name(), binding.syntax(), "a record field name")?;
            let value = self.required(binding.value(), binding.syntax(), "a record field value")?;
            names.push(node_text(name.syntax())?);
            inputs.push(self.expression(&value)?.0);
        }
        Ok(self.emit(
            "source/record",
            inputs,
            BuiltinSchema::Dynamic,
            record.syntax(),
            "record",
            Some(names.join(",")),
        ))
    }

    fn set(&mut self, set: &SetSyntax) -> Result<PendingValue, SourceSemanticError> {
        let inputs = set
            .items()
            .iter()
            .map(|item| self.expression(item).map(|value| value.0))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(self.emit(
            "set/define",
            inputs,
            BuiltinSchema::Dynamic,
            set.syntax(),
            "set",
            None,
        ))
    }

    fn tuple(&mut self, tuple: &TupleSyntax) -> Result<PendingValue, SourceSemanticError> {
        let inputs = tuple
            .items()
            .iter()
            .map(|item| self.expression(item).map(|value| value.0))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(self.emit(
            "source/tuple",
            inputs,
            BuiltinSchema::Dynamic,
            tuple.syntax(),
            "tuple",
            None,
        ))
    }

    fn tuple_struct(
        &mut self,
        tuple: &TupleStructSyntax,
    ) -> Result<PendingValue, SourceSemanticError> {
        let name = self.required(tuple.name(), tuple.syntax(), "a tuple-structure name")?;
        let value = self.required(tuple.value(), tuple.syntax(), "a tuple-structure value")?;
        let value = self.expression(&value)?.0;
        Ok(self.emit(
            "source/tuple-struct",
            vec![value],
            BuiltinSchema::Dynamic,
            tuple.syntax(),
            "tuple-struct",
            Some(node_text(name.syntax())?),
        ))
    }

    fn slice(&mut self, slice: &SliceSyntax) -> Result<PendingValue, SourceSemanticError> {
        let stem = self.required(slice.stem(), slice.syntax(), "a slice stem")?;
        let stem = match stem {
            SliceStemSyntax::Identifier(value) => self.value_for_name_node(value.syntax())?,
            SliceStemSyntax::Context(value) => self.input_for_node(value.syntax())?,
        };
        let subscripts = self.required(slice.subscripts(), slice.syntax(), "subscripts")?;
        let mut inputs = vec![stem];
        for item in subscripts.items() {
            inputs.push(self.subscript(&item)?);
        }
        Ok(self.emit(
            "access/index",
            inputs,
            BuiltinSchema::Dynamic,
            slice.syntax(),
            "slice",
            None,
        ))
    }

    fn subscript(
        &mut self,
        item: &SubscriptItemSyntax,
    ) -> Result<PendingValue, SourceSemanticError> {
        match item {
            SubscriptItemSyntax::Bracket(value) => {
                let values = value
                    .values()
                    .iter()
                    .map(|value| self.subscript_value(value))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(self.emit(
                    "source/bracket-subscript",
                    values,
                    BuiltinSchema::Dynamic,
                    value.syntax(),
                    "subscript",
                    None,
                ))
            }
            SubscriptItemSyntax::Brace(value) => {
                let values = value
                    .values()
                    .iter()
                    .map(|value| self.subscript_value(value))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(self.emit(
                    "source/brace-subscript",
                    values,
                    BuiltinSchema::Dynamic,
                    value.syntax(),
                    "subscript",
                    None,
                ))
            }
            value => Ok(self.emit(
                "source/subscript",
                Vec::new(),
                BuiltinSchema::Dynamic,
                value.syntax(),
                "subscript",
                value.syntax().text().ok(),
            )),
        }
    }

    fn subscript_value(
        &mut self,
        value: &SubscriptValueSyntax,
    ) -> Result<PendingValue, SourceSemanticError> {
        match value {
            SubscriptValueSyntax::Formula(value) => {
                let formula =
                    self.required(value.formula(), value.syntax(), "a subscript value")?;
                self.formula(&formula)
            }
            SubscriptValueSyntax::Range(value) => {
                let range = self.required(value.range(), value.syntax(), "a subscript range")?;
                self.range(&range)
            }
            SubscriptValueSyntax::SelectAll(value) => Ok(self.emit(
                "source/select-all",
                Vec::new(),
                BuiltinSchema::Dynamic,
                value.syntax(),
                "subscript",
                None,
            )),
        }
    }

    fn set_comprehension(
        &mut self,
        value: &SetComprehensionSyntax,
    ) -> Result<PendingValue, SourceSemanticError> {
        self.comprehension(
            value.syntax(),
            value.value(),
            value.qualifiers(),
            "set/comprehension",
        )
    }

    fn matrix_comprehension(
        &mut self,
        value: &MatrixComprehensionSyntax,
    ) -> Result<PendingValue, SourceSemanticError> {
        self.comprehension(
            value.syntax(),
            value.value(),
            value.qualifiers(),
            "matrix/comprehension",
        )
    }

    fn comprehension(
        &mut self,
        syntax: &SyntaxNode,
        result: Option<ExpressionSyntax>,
        qualifiers: Vec<mech_syntax::document::ComprehensionQualifierSyntax>,
        operation: &'static str,
    ) -> Result<PendingValue, SourceSemanticError> {
        let mut inputs = Vec::new();
        for qualifier in qualifiers {
            let qualifier = self.required(
                qualifier.value(),
                qualifier.syntax(),
                "a comprehension qualifier",
            )?;
            match qualifier {
                ComprehensionQualifierValueSyntax::Generator(generator) => {
                    let pattern = self.required(
                        generator.pattern(),
                        generator.syntax(),
                        "a generator pattern",
                    )?;
                    let source = self.required(
                        generator.source(),
                        generator.syntax(),
                        "a generator source",
                    )?;
                    inputs.push(self.pattern(&pattern)?);
                    inputs.push(self.expression(&source)?.0);
                }
                ComprehensionQualifierValueSyntax::Definition(definition) => {
                    inputs.push(self.definition(&definition)?.0);
                }
                ComprehensionQualifierValueSyntax::Filter(filter) => {
                    inputs.push(self.expression(&filter)?.0);
                }
            }
        }
        let result = self.required(result, syntax, "a comprehension result")?;
        inputs.push(self.expression(&result)?.0);
        Ok(self.emit(
            operation,
            inputs,
            BuiltinSchema::Dynamic,
            syntax,
            "comprehension",
            None,
        ))
    }

    fn pattern(&mut self, pattern: &PatternSyntax) -> Result<PendingValue, SourceSemanticError> {
        let value = self.required(pattern.value(), pattern.syntax(), "a pattern body")?;
        let (operation, inputs, detail, syntax) = match value {
            PatternValueSyntax::Wildcard(value) => (
                "source/pattern-wildcard",
                Vec::new(),
                None,
                value.syntax().clone(),
            ),
            PatternValueSyntax::Expression(value) => (
                "source/pattern-expression",
                vec![self.expression(&value)?.0],
                None,
                value.syntax().clone(),
            ),
            PatternValueSyntax::Array(value) => {
                let mut inputs = Vec::new();
                let mut modifiers = Vec::new();
                for element in value.elements() {
                    let nested = self.required(
                        element.pattern(),
                        element.syntax(),
                        "an array pattern element",
                    )?;
                    inputs.push(self.pattern(&nested)?);
                    modifiers.push(if element.spread().is_some() {
                        "spread"
                    } else if element.rest().is_some() {
                        "rest"
                    } else {
                        "item"
                    });
                }
                (
                    "source/pattern-array",
                    inputs,
                    Some(modifiers.join(",")),
                    value.syntax().clone(),
                )
            }
            PatternValueSyntax::Tuple(value) => {
                let inputs = value
                    .items()
                    .iter()
                    .map(|item| self.pattern(item))
                    .collect::<Result<Vec<_>, _>>()?;
                ("source/pattern-tuple", inputs, None, value.syntax().clone())
            }
            PatternValueSyntax::AtomStruct(value) => {
                let inputs = value
                    .items()
                    .iter()
                    .map(|item| self.pattern(item))
                    .collect::<Result<Vec<_>, _>>()?;
                (
                    "source/pattern-atom-struct",
                    inputs,
                    value.name().and_then(|name| name.syntax().text().ok()),
                    value.syntax().clone(),
                )
            }
            PatternValueSyntax::TupleStruct(value) => {
                let inputs = value
                    .items()
                    .iter()
                    .map(|item| self.pattern(item))
                    .collect::<Result<Vec<_>, _>>()?;
                (
                    "source/pattern-tuple-struct",
                    inputs,
                    value.name().and_then(|name| name.syntax().text().ok()),
                    value.syntax().clone(),
                )
            }
        };
        Ok(self.emit(
            operation,
            inputs,
            BuiltinSchema::Dynamic,
            &syntax,
            "pattern",
            detail,
        ))
    }

    fn fsm_pipe(&mut self, pipe: &FsmPipeSyntax) -> Result<PendingValue, SourceSemanticError> {
        let instance = self.required(pipe.instance(), pipe.syntax(), "an FSM instance")?;
        let name = self.required(instance.name(), instance.syntax(), "an FSM name")?;
        let mut inputs = Vec::new();
        if let Some(arguments) = instance.arguments() {
            for argument in arguments.arguments() {
                let value = match argument {
                    AnyCallArgumentSyntax::Positional(argument) => {
                        self.required(argument.value(), argument.syntax(), "an FSM argument value")?
                    }
                    AnyCallArgumentSyntax::Bound(argument) => {
                        self.required(argument.value(), argument.syntax(), "an FSM argument value")?
                    }
                };
                inputs.push(self.expression(&value)?.0);
            }
        }
        for stage in pipe.stages() {
            let (value, role, syntax) = match stage {
                FsmStageSyntax::State(value) => (
                    self.required(value.value(), value.syntax(), "an FSM transition value")?,
                    "state",
                    value.syntax().clone(),
                ),
                FsmStageSyntax::Async(value) => (
                    self.required(value.value(), value.syntax(), "an FSM transition value")?,
                    "async",
                    value.syntax().clone(),
                ),
                FsmStageSyntax::Output(value) => (
                    self.required(value.value(), value.syntax(), "an FSM output value")?,
                    "output",
                    value.syntax().clone(),
                ),
            };
            let pattern = self.required(value.pattern(), value.syntax(), "an FSM value pattern")?;
            let pattern = self.pattern(&pattern)?;
            inputs.push(self.emit(
                "source/fsm-stage",
                vec![pattern],
                BuiltinSchema::Dynamic,
                &syntax,
                "fsm-stage",
                Some(role.to_owned()),
            ));
        }
        Ok(self.emit(
            "source/fsm",
            inputs,
            BuiltinSchema::Dynamic,
            pipe.syntax(),
            "fsm",
            Some(node_text(name.syntax())?),
        ))
    }

    fn input_for_node(&mut self, node: &SyntaxNode) -> Result<PendingValue, SourceSemanticError> {
        let name = node_text(node)?;
        if let Some(value) = self.bindings.get(&name) {
            return Ok(*value);
        }
        if let Some(index) = self.input_by_name.get(&name) {
            return Ok(PendingValue::Input(*index));
        }
        let index = u32::try_from(self.inputs.len()).map_err(|_| SourceSemanticError {
            code: "source-semantics/input-identity-exhausted",
            message: "canonical input count exceeds SourceProgram identity space".to_owned(),
            anchor: SourceSemanticAnchor::for_node(node),
        })?;
        self.input_by_name.insert(name.clone(), index);
        self.inputs
            .push((name, SourceSemanticAnchor::for_node(node)));
        Ok(PendingValue::Input(index))
    }

    fn value_for_name_node(
        &mut self,
        node: &SyntaxNode,
    ) -> Result<PendingValue, SourceSemanticError> {
        self.input_for_node(node)
    }

    fn constant(&mut self, schema: BuiltinSchema, data: ValueDataDraft) -> PendingValue {
        let index = self.constants.len();
        self.constants.push(PendingConstant { schema, data });
        PendingValue::Constant(index)
    }

    fn emit(
        &mut self,
        operation: &'static str,
        inputs: Vec<PendingValue>,
        schema: BuiltinSchema,
        syntax: &SyntaxNode,
        role: &'static str,
        detail: Option<String>,
    ) -> PendingValue {
        let index = self.nodes.len() as u32;
        self.nodes.push(PendingNode {
            operation: operation_reference(operation),
            inputs,
            schema,
            semantic: SourceSemanticNode {
                operation: operation.to_owned(),
                role,
                detail,
                anchor: SourceSemanticAnchor::for_node(syntax),
            },
        });
        PendingValue::Node(index)
    }

    fn publish(
        &mut self,
        name: &str,
        interactive_symbol: Option<String>,
        source: PendingValue,
        syntax: &SyntaxNode,
    ) {
        self.outputs.push(PendingOutput {
            name: name.to_owned(),
            interactive_symbol,
            source,
            anchor: SourceSemanticAnchor::for_node(syntax),
        });
    }

    fn finish(self) -> Result<CanonicalSourceProgram, SourceSemanticError> {
        let schemas = BuiltinSchemas::build(self.anchor)?;
        let constant_schemas = self
            .constants
            .iter()
            .map(|constant| constant.schema)
            .collect::<Vec<_>>();
        let mut constants = ConstantStoreBuilder::new(&schemas.table);
        let mut handles = Vec::with_capacity(self.constants.len());
        for constant in self.constants {
            let schema = schemas.id(constant.schema);
            let value = ValueDraft {
                schema,
                shape_values: Box::new([]),
                data: constant.data,
            }
            .finalize(&SnapshotValidationContext::new(&schemas.table))
            .map_err(|error| {
                internal(
                    self.anchor,
                    format!("unable to finalize source constant: {error:?}"),
                )
            })?;
            handles.push(constants.insert(value).map_err(|error| {
                internal(
                    self.anchor,
                    format!("unable to retain source constant: {error:?}"),
                )
            })?);
        }
        let constant_build = constants.finish().map_err(|error| {
            internal(
                self.anchor,
                format!("unable to finalize source constants: {error:?}"),
            )
        })?;
        let constant_ids = handles
            .into_iter()
            .map(|handle| {
                constant_build.resolve(handle).map_err(|error| {
                    internal(
                        self.anchor,
                        format!("unable to resolve source constant: {error:?}"),
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let inputs = self
            .inputs
            .iter()
            .map(|(name, _)| SourceInput {
                name: name.clone(),
                schema: schemas.dynamic,
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let mut contracts = Vec::with_capacity(self.nodes.len());
        let nodes = self
            .nodes
            .iter()
            .map(|node| {
                contracts.push(operation_contract(node.inputs.len()));
                SourceNode {
                    operation: node.operation.clone(),
                    requirement: None,
                    inputs: node
                        .inputs
                        .iter()
                        .map(|value| resolve_value(*value, &constant_ids))
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                    outputs: vec![SourceNodeOutput::Derived {
                        schema: schemas.id(node.schema),
                    }]
                    .into_boxed_slice(),
                }
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let outputs = self
            .outputs
            .iter()
            .map(|output| SourceOutput {
                name: output.name.clone(),
                interactive_symbol: output.interactive_symbol.clone(),
                source: resolve_value(output.source, &constant_ids),
                schema: pending_schema(output.source, &constant_schemas, &self.nodes, &schemas),
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let source_map = SourceSemanticMap {
            inputs: self
                .inputs
                .into_iter()
                .map(|(_, anchor)| anchor)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            nodes: self
                .nodes
                .into_iter()
                .map(|node| node.semantic)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            outputs: self
                .outputs
                .into_iter()
                .map(|output| output.anchor)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        };
        Ok(CanonicalSourceProgram {
            program: SourceProgram {
                requirements: Default::default(),
                inputs,
                states: Box::new([]),
                nodes,
                outputs,
                constraints: Box::new([]),
            },
            schemas: schemas.table,
            constants: constant_build.store,
            contracts: contracts.into_boxed_slice(),
            source_map,
        })
    }
}

fn pending_schema(
    value: PendingValue,
    constants: &[BuiltinSchema],
    nodes: &[PendingNode],
    schemas: &BuiltinSchemas,
) -> SchemaId {
    match value {
        PendingValue::Constant(index) => constants
            .get(index)
            .map(|schema| schemas.id(*schema))
            .unwrap_or(schemas.dynamic),
        PendingValue::Input(_) => schemas.dynamic,
        PendingValue::Node(node) => nodes
            .get(node as usize)
            .map(|node| schemas.id(node.schema))
            .unwrap_or(schemas.dynamic),
    }
}

fn resolve_value(value: PendingValue, constants: &[mech_core::ConstantId]) -> SourceValue {
    match value {
        PendingValue::Constant(index) => SourceValue::Constant(constants[index]),
        PendingValue::Input(index) => SourceValue::Input(index),
        PendingValue::Node(node) => SourceValue::NodeOutput {
            node,
            output_ordinal: 0,
        },
    }
}

fn operation_reference(name: &str) -> OperationReference {
    let mut parts = name.split('/').map(str::to_owned).collect::<Vec<_>>();
    let operation_name = parts.pop().unwrap_or_else(|| "source".to_owned());
    OperationReference {
        module_path: parts.into_boxed_slice(),
        operation_name,
    }
}

fn operation_contract(input_count: usize) -> OperationContractDeclaration {
    OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            (0..input_count)
                .map(|_| InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::AlwaysChanged,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    }
}

fn operator_name(operator: CanonicalOperator) -> (&'static str, BuiltinSchema) {
    use CanonicalOperator::*;
    match operator {
        Add => ("math/add", BuiltinSchema::Dynamic),
        Subtract => ("math/sub", BuiltinSchema::Dynamic),
        Multiply => ("math/mul", BuiltinSchema::Dynamic),
        Divide => ("math/div", BuiltinSchema::Dynamic),
        Modulus => ("math/mod", BuiltinSchema::Dynamic),
        Power => ("math/pow", BuiltinSchema::Dynamic),
        MatrixMultiply => ("matrix/matmul", BuiltinSchema::Dynamic),
        MatrixSolve => ("matrix/solve", BuiltinSchema::Dynamic),
        DotProduct => ("matrix/dot", BuiltinSchema::Dynamic),
        CrossProduct => ("matrix/cross", BuiltinSchema::Dynamic),
        RangeInclusive => ("range/inclusive", BuiltinSchema::Dynamic),
        RangeExclusive => ("range/exclusive", BuiltinSchema::Dynamic),
        NotEqual => ("compare/neq", BuiltinSchema::Bool),
        EqualTo => ("compare/eq", BuiltinSchema::Bool),
        StrictNotEqual => ("compare/sneq", BuiltinSchema::Bool),
        StrictEqual => ("compare/seq", BuiltinSchema::Bool),
        GreaterThan => ("compare/gt", BuiltinSchema::Bool),
        LessThan => ("compare/lt", BuiltinSchema::Bool),
        GreaterThanEqual => ("compare/gte", BuiltinSchema::Bool),
        LessThanEqual => ("compare/lte", BuiltinSchema::Bool),
        Or => ("logic/or", BuiltinSchema::Bool),
        And => ("logic/and", BuiltinSchema::Bool),
        Not => ("logic/not", BuiltinSchema::Bool),
        Xor => ("logic/xor", BuiltinSchema::Bool),
        InnerJoin => ("table/join", BuiltinSchema::Dynamic),
        LeftOuterJoin => ("table/left-outer-join", BuiltinSchema::Dynamic),
        RightOuterJoin => ("table/right-outer-join", BuiltinSchema::Dynamic),
        FullOuterJoin => ("table/full-outer-join", BuiltinSchema::Dynamic),
        LeftSemiJoin => ("table/left-semi-join", BuiltinSchema::Dynamic),
        LeftAntiJoin => ("table/left-anti-join", BuiltinSchema::Dynamic),
        Union => ("set/union", BuiltinSchema::Dynamic),
        Intersection => ("set/intersection", BuiltinSchema::Dynamic),
        Difference => ("set/difference", BuiltinSchema::Dynamic),
        Complement => ("set/complement", BuiltinSchema::Dynamic),
        Subset => ("set/subset", BuiltinSchema::Bool),
        Superset => ("set/superset", BuiltinSchema::Bool),
        ProperSubset => ("set/proper_subset", BuiltinSchema::Bool),
        ProperSuperset => ("set/proper-superset", BuiltinSchema::Bool),
        ElementOf => ("set/element-of", BuiltinSchema::Bool),
        NotElementOf => ("set/not-element-of", BuiltinSchema::Bool),
        SymmetricDifference => ("set/symmetric-difference", BuiltinSchema::Dynamic),
    }
}

fn decode_string(source: &str) -> Option<String> {
    if source.starts_with("\"\"\"") && source.ends_with("\"\"\"") && source.len() >= 6 {
        return Some(source[3..source.len() - 3].to_owned());
    }
    let body = source.strip_prefix('"')?.strip_suffix('"')?;
    let mut output = String::with_capacity(body.len());
    let mut chars = body.chars();
    while let Some(character) = chars.next() {
        if character != '\\' {
            output.push(character);
            continue;
        }
        let escaped = chars.next()?;
        output.push(match escaped {
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            '\\' => '\\',
            '"' => '"',
            other => other,
        });
    }
    Some(output)
}

fn node_text(node: &SyntaxNode) -> Result<String, SourceSemanticError> {
    node.text().map_err(|error| SourceSemanticError {
        code: "source-semantics/source-range",
        message: format!("unable to read canonical source range: {error:?}"),
        anchor: SourceSemanticAnchor::for_node(node),
    })
}

fn internal(anchor: SourceSemanticAnchor, message: String) -> SourceSemanticError {
    SourceSemanticError {
        code: "source-semantics/internal",
        message,
        anchor,
    }
}

fn syntax_kind_name(kind: SyntaxKind) -> String {
    format!("{kind:?}")
}
