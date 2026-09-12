use std::collections::BTreeMap;

use mech_core::snapshot::{Complex64Bits, F32Bits, F64Bits, SnapshotValidationContext};
use mech_core::{
    AccessMode, AliasPolicy, CardinalitySpec, ChangeDetectionPolicy, ConstantStore,
    ConstantStoreBuilder, DeliveryMode, DimensionExpr, ExternalInteraction, FloatWidth,
    InputPortLayout, InputPortPolicy, IntegerWidth, NodeId, OperationContractDeclaration,
    OutputConstruction, OutputPortPolicy, SchemaBody, SchemaDraft, SchemaField, SchemaId,
    SchemaTable, SchemaTableBuilder, ShapeRule, ValueDataDraft, ValueDraft,
};
use mech_syntax::document::{
    AnyCallArgumentSyntax, AstNode, CanonicalOperator, ComprehensionQualifierValueSyntax,
    DocumentId, DocumentSyntax, ExpressionBodySyntax, ExpressionSyntax, FactorSyntax,
    FactorValueSyntax, FormulaSyntax, FsmPipeSyntax, FsmStageSyntax, KindAnnotationSyntax,
    LiteralSyntax, LiteralValueSyntax, MapSyntax, MatrixComprehensionSyntax, MatrixSyntax,
    MultiplicativeExpressionSyntax, NodeFlags, OperatorSyntax, PatternSyntax,
    RangeExpressionSyntax, RecordSyntax, RecursiveSyntaxNode, Revision, SetComprehensionSyntax,
    SetSyntax, SliceStemSyntax, SliceSyntax, StructureSyntax, StructureValueSyntax,
    SubscriptItemSyntax, SubscriptValueSyntax, SyntaxKind, SyntaxNode, TableSyntax,
    TableValueSyntax, TextRange, TupleStructSyntax, TupleSyntax, VariableDefineSyntax,
    VariableStemSyntax, VariableSyntax,
};

use crate::{
    ArtifactBuildContext, ArtifactBuildError, OperationReference, ProgramArtifact, SourceInput,
    SourceNode, SourceNodeOutput, SourceOutput, SourceProgram, SourceState, SourceValue,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceSemanticPattern {
    pub source: String,
    pub bindings: Box<[String]>,
    pub anchor: SourceSemanticAnchor,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SourceSemanticMap {
    pub inputs: Box<[SourceSemanticAnchor]>,
    pub nodes: Box<[SourceSemanticNode]>,
    pub patterns: Box<[SourceSemanticPattern]>,
    pub outputs: Box<[SourceSemanticAnchor]>,
}

/// The complete engine-owned input to canonical artifact construction.
pub struct CanonicalSourceProgram {
    program: SourceProgram,
    schemas: SchemaTable,
    constants: ConstantStore,
    contracts: Box<[Option<OperationContractDeclaration>]>,
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

    pub const fn contracts(&self) -> &[Option<OperationContractDeclaration>] {
        &self.contracts
    }

    pub const fn source_map(&self) -> &SourceSemanticMap {
        &self.source_map
    }

    pub fn compile_artifact(&self) -> Result<ProgramArtifact, ArtifactBuildError> {
        let contracts = self
            .contracts
            .iter()
            .enumerate()
            .map(|(index, contract)| {
                contract
                    .as_ref()
                    .ok_or_else(|| ArtifactBuildError::MissingOperationContract {
                        node: NodeId(index as u32),
                        operation: self.program.nodes[index].operation.clone(),
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
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

    pub fn compile_definition(
        &self,
        definition: &VariableDefineSyntax,
    ) -> Result<CanonicalSourceProgram, SourceSemanticError> {
        reject_recovered_syntax(definition)?;
        let anchor = SourceSemanticAnchor::for_node(definition.syntax());
        let mut builder = SemanticBuilder::new(anchor);
        let (value, syntax) = builder.definition(definition)?;
        builder.publish("result", None, value, &syntax);
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

fn collect_pattern_bindings(
    node: &SyntaxNode,
    output: &mut Vec<String>,
) -> Result<(), SourceSemanticError> {
    if node.kind() == SyntaxKind::Variable {
        let variable = VariableSyntax::cast(node.clone()).expect("kind-checked variable cast");
        if let Some(VariableStemSyntax::Identifier(identifier)) = variable.stem() {
            output.push(node_text(identifier.syntax())?);
        }
        return Ok(());
    }
    for child in node.children() {
        collect_pattern_bindings(&child, output)?;
    }
    Ok(())
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
        let range = node
            .error_nodes()
            .into_iter()
            .map(|node| node.syntax().range())
            .chain(
                node.missing_nodes()
                    .into_iter()
                    .map(|node| node.syntax().range()),
            )
            .chain(node.missing_tokens().into_iter().map(|token| token.range()))
            .min_by_key(|range| (range.start, range.end))
            .unwrap_or_else(|| node.syntax().range());
        return Err(SourceSemanticError {
            code: "source-semantics/recovered-syntax",
            message: "source semantics require a complete canonical syntax tree".to_owned(),
            anchor: SourceSemanticAnchor {
                document: node.syntax().source().document(),
                revision: node.syntax().source().revision(),
                range,
            },
        });
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum BuiltinSchema {
    Dynamic,
    Bool,
    String,
    U8,
    U16,
    U32,
    U64,
    U128,
    I8,
    I16,
    I32,
    I64,
    I128,
    F32,
    F64,
    C64,
    R64,
}

struct BuiltinSchemas {
    table: SchemaTable,
    ids: BTreeMap<BuiltinSchema, SchemaId>,
    node_ids: BTreeMap<usize, SchemaId>,
}

impl BuiltinSchemas {
    fn build(
        anchor: SourceSemanticAnchor,
        nodes: &[PendingNode],
    ) -> Result<Self, SourceSemanticError> {
        let mut builder = SchemaTableBuilder::new();
        let mut handles = Vec::new();
        for builtin in [
            BuiltinSchema::Dynamic,
            BuiltinSchema::Bool,
            BuiltinSchema::String,
            BuiltinSchema::U8,
            BuiltinSchema::U16,
            BuiltinSchema::U32,
            BuiltinSchema::U64,
            BuiltinSchema::U128,
            BuiltinSchema::I8,
            BuiltinSchema::I16,
            BuiltinSchema::I32,
            BuiltinSchema::I64,
            BuiltinSchema::I128,
            BuiltinSchema::F32,
            BuiltinSchema::F64,
            BuiltinSchema::C64,
            BuiltinSchema::R64,
        ] {
            let schema = SchemaDraft {
                dimension_parameters: Box::new([]),
                body: schema_body(builtin),
            }
            .finalize()
            .map_err(|error| internal(anchor, format!("invalid builtin schema: {error:?}")))?;
            let handle = builder
                .insert(schema)
                .map_err(|error| internal(anchor, format!("unable to retain schema: {error:?}")))?;
            handles.push((builtin, handle));
        }
        let node_handles = nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)| node.schema_body.as_ref().map(|body| (index, body)))
            .map(|(index, body)| {
                let schema = SchemaDraft {
                    dimension_parameters: Box::new([]),
                    body: body.clone(),
                }
                .finalize()
                .map_err(|error| internal(anchor, format!("invalid source schema: {error:?}")))?;
                builder
                    .insert(schema)
                    .map(|handle| (index, handle))
                    .map_err(|error| {
                        internal(anchor, format!("unable to retain source schema: {error:?}"))
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let build = builder
            .finish()
            .map_err(|error| internal(anchor, format!("unable to finalize schemas: {error:?}")))?;
        let ids = handles
            .into_iter()
            .map(|(schema, handle)| {
                build
                    .resolve(handle)
                    .map(|id| (schema, id))
                    .map_err(|error| {
                        internal(
                            anchor,
                            format!("unable to resolve {schema:?} schema: {error:?}"),
                        )
                    })
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let node_ids = node_handles
            .into_iter()
            .map(|(index, handle)| {
                build
                    .resolve(handle)
                    .map(|id| (index, id))
                    .map_err(|error| {
                        internal(
                            anchor,
                            format!("unable to resolve source schema: {error:?}"),
                        )
                    })
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let (table, _) = build.into_parts();
        Ok(Self {
            table,
            ids,
            node_ids,
        })
    }

    fn id(&self, schema: BuiltinSchema) -> SchemaId {
        self.ids[&schema]
    }

    fn node_id(&self, index: usize, fallback: BuiltinSchema) -> SchemaId {
        self.node_ids
            .get(&index)
            .copied()
            .unwrap_or_else(|| self.id(fallback))
    }
}

fn schema_body(schema: BuiltinSchema) -> SchemaBody {
    match schema {
        BuiltinSchema::Dynamic => SchemaBody::Dynamic,
        BuiltinSchema::Bool => SchemaBody::Bool,
        BuiltinSchema::String => SchemaBody::String,
        BuiltinSchema::U8 => SchemaBody::UnsignedInteger(IntegerWidth::W8),
        BuiltinSchema::U16 => SchemaBody::UnsignedInteger(IntegerWidth::W16),
        BuiltinSchema::U32 => SchemaBody::UnsignedInteger(IntegerWidth::W32),
        BuiltinSchema::U64 => SchemaBody::UnsignedInteger(IntegerWidth::W64),
        BuiltinSchema::U128 => SchemaBody::UnsignedInteger(IntegerWidth::W128),
        BuiltinSchema::I8 => SchemaBody::SignedInteger(IntegerWidth::W8),
        BuiltinSchema::I16 => SchemaBody::SignedInteger(IntegerWidth::W16),
        BuiltinSchema::I32 => SchemaBody::SignedInteger(IntegerWidth::W32),
        BuiltinSchema::I64 => SchemaBody::SignedInteger(IntegerWidth::W64),
        BuiltinSchema::I128 => SchemaBody::SignedInteger(IntegerWidth::W128),
        BuiltinSchema::F32 => SchemaBody::FloatingPoint(FloatWidth::W32),
        BuiltinSchema::F64 => SchemaBody::FloatingPoint(FloatWidth::W64),
        BuiltinSchema::C64 => SchemaBody::Complex(FloatWidth::W64),
        BuiltinSchema::R64 => SchemaBody::Rational64,
    }
}

#[derive(Clone, Copy)]
enum PendingValue {
    Constant(usize),
    Input(u32),
    State(u32),
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
    schema_body: Option<SchemaBody>,
    state: Option<u32>,
    semantic: SourceSemanticNode,
}

struct PendingState {
    schema: BuiltinSchema,
    initializer: Option<usize>,
    producer_node: u32,
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
    inputs: Vec<(String, BuiltinSchema, SourceSemanticAnchor)>,
    input_by_name: BTreeMap<String, u32>,
    nodes: Vec<PendingNode>,
    states: Vec<PendingState>,
    outputs: Vec<PendingOutput>,
    bindings: BTreeMap<String, PendingValue>,
    patterns: Vec<SourceSemanticPattern>,
}

impl SemanticBuilder {
    fn new(anchor: SourceSemanticAnchor) -> Self {
        Self {
            anchor,
            constants: Vec::new(),
            inputs: Vec::new(),
            input_by_name: BTreeMap::new(),
            nodes: Vec::new(),
            states: Vec::new(),
            outputs: Vec::new(),
            bindings: BTreeMap::new(),
            patterns: Vec::new(),
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
        let mut value = self.expression_body(&body)?;
        let arms = expression.match_arms();
        if !arms.is_empty() {
            let mut inputs = vec![value];
            for arm in arms {
                let pattern = self.required(arm.pattern(), arm.syntax(), "a match pattern")?;
                let saved = self.bindings.clone();
                let result = (|| {
                    for name in self.record_pattern(&pattern)? {
                        self.bindings.insert(name, value);
                    }
                    if let Some(guard) = arm.guard() {
                        inputs.push(self.expression(&guard)?.0);
                    }
                    let result = self.required(arm.value(), arm.syntax(), "a match result")?;
                    inputs.push(self.expression(&result)?.0);
                    Ok::<_, SourceSemanticError>(())
                })();
                self.bindings = saved;
                result?;
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

    fn expression_body(
        &mut self,
        body: &ExpressionBodySyntax,
    ) -> Result<PendingValue, SourceSemanticError> {
        match body {
            ExpressionBodySyntax::FsmPipe(pipe) => self.fsm_pipe(pipe),
            ExpressionBodySyntax::SetComprehension(value) => self.set_comprehension(value),
            ExpressionBodySyntax::MatrixComprehension(value) => self.matrix_comprehension(value),
            ExpressionBodySyntax::Range(range) => self.range(range),
            ExpressionBodySyntax::Formula(formula) => self.formula(formula),
        }
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
        let (name, fixed_schema) = operator_name(operator);
        let lhs_schema = self.schema_of(lhs);
        let rhs_schema = self.schema_of(rhs);
        let schema = fixed_schema.unwrap_or_else(|| {
            if lhs_schema == rhs_schema {
                lhs_schema
            } else {
                BuiltinSchema::Dynamic
            }
        });
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
                self.expression_body(&expression)?
            }
            FactorValueSyntax::Negate(value) => {
                let operand = self.required(value.operand(), value.syntax(), "a unary operand")?;
                let operand = self.factor(&operand)?;
                self.emit(
                    "math/neg",
                    vec![operand],
                    self.schema_of(operand),
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
                    &function_name,
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
                self.schema_of(result),
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
        let terminal = self.operator(operators.last().expect("validated range operator"))?;
        let name = match (terminal, values.len()) {
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
        let schema = variable
            .annotation()
            .map(|annotation| annotation_schema(&annotation))
            .transpose()?
            .unwrap_or(BuiltinSchema::Dynamic);
        if let Some(index) = self.input_by_name.get(&name) {
            let (_, existing, _) = &self.inputs[*index as usize];
            if schema != BuiltinSchema::Dynamic
                && *existing != BuiltinSchema::Dynamic
                && schema != *existing
            {
                return Err(SourceSemanticError {
                    code: "source-semantics/conflicting-input-kind",
                    message: format!("input {name} has conflicting kind annotations"),
                    anchor: SourceSemanticAnchor::for_node(variable.syntax()),
                });
            }
            if *existing == BuiltinSchema::Dynamic && schema != BuiltinSchema::Dynamic {
                self.inputs[*index as usize].1 = schema;
            }
            return Ok(PendingValue::Input(*index));
        }
        let index = u32::try_from(self.inputs.len()).map_err(|_| SourceSemanticError {
            code: "source-semantics/input-identity-exhausted",
            message: "canonical input count exceeds SourceProgram identity space".to_owned(),
            anchor: SourceSemanticAnchor::for_node(&syntax),
        })?;
        self.input_by_name.insert(name.clone(), index);
        self.inputs
            .push((name, schema, SourceSemanticAnchor::for_node(&syntax)));
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
        let bound = if definition.mutability_marker().is_some() {
            let schema = self.schema_of(value);
            let state = u32::try_from(self.states.len()).map_err(|_| SourceSemanticError {
                code: "source-semantics/state-identity-exhausted",
                message: "canonical state count exceeds SourceProgram identity space".to_owned(),
                anchor: SourceSemanticAnchor::for_node(definition.syntax()),
            })?;
            let node = self.nodes.len() as u32;
            let initializer = match value {
                PendingValue::Constant(index) => Some(index),
                _ => None,
            };
            self.states.push(PendingState {
                schema,
                initializer,
                producer_node: node,
            });
            self.nodes.push(PendingNode {
                operation: operation_reference("core/assign"),
                inputs: vec![value],
                schema,
                schema_body: None,
                state: Some(state),
                semantic: SourceSemanticNode {
                    operation: "core/assign".to_owned(),
                    role: "state-definition",
                    detail: Some(name.clone()),
                    anchor: SourceSemanticAnchor::for_node(definition.syntax()),
                },
            });
            PendingValue::State(state)
        } else {
            value
        };
        self.bindings.insert(name, bound);
        Ok((bound, definition.syntax().clone()))
    }

    fn literal(&mut self, literal: &LiteralSyntax) -> Result<PendingValue, SourceSemanticError> {
        let annotation = literal
            .annotation()
            .map(|annotation| annotation_schema(&annotation))
            .transpose()?;
        if literal.true_token().is_some() {
            require_literal_schema(annotation, BuiltinSchema::Bool, literal.syntax())?;
            return Ok(self.constant(BuiltinSchema::Bool, ValueDataDraft::Bool(true)));
        }
        if literal.false_token().is_some() {
            require_literal_schema(annotation, BuiltinSchema::Bool, literal.syntax())?;
            return Ok(self.constant(BuiltinSchema::Bool, ValueDataDraft::Bool(false)));
        }
        let value = self.required(literal.value(), literal.syntax(), "a literal value")?;
        match value {
            LiteralValueSyntax::String(value) => {
                require_literal_schema(annotation, BuiltinSchema::String, value.syntax())?;
                let source = node_text(value.syntax())?;
                let decoded = decode_string(&source).ok_or_else(|| SourceSemanticError {
                    code: "source-semantics/invalid-string-literal",
                    message: "canonical string could not be decoded".to_owned(),
                    anchor: SourceSemanticAnchor::for_node(value.syntax()),
                })?;
                Ok(self.constant(BuiltinSchema::String, ValueDataDraft::String(decoded)))
            }
            LiteralValueSyntax::Number(value) => {
                let source = node_text(value.syntax())?;
                let (schema, data) =
                    decode_number(&source, annotation).ok_or_else(|| SourceSemanticError {
                        code: "source-semantics/invalid-number-literal",
                        message: format!("canonical number {source:?} could not be represented"),
                        anchor: SourceSemanticAnchor::for_node(value.syntax()),
                    })?;
                Ok(self.constant(schema, data))
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
        let (headers, rows, syntax): (
            Vec<(String, BuiltinSchema)>,
            Vec<Vec<ExpressionSyntax>>,
            SyntaxNode,
        ) = match value {
            TableValueSyntax::Fancy(value) => (
                value
                    .header()
                    .into_iter()
                    .flat_map(|header| header.fields())
                    .map(|field| {
                        let name =
                            self.required(field.name(), field.syntax(), "a table field name")?;
                        let schema = field
                            .annotation()
                            .map(|annotation| annotation_schema(&annotation))
                            .transpose()?
                            .unwrap_or(BuiltinSchema::Dynamic);
                        Ok((node_text(name.syntax())?, schema))
                    })
                    .collect::<Result<Vec<_>, SourceSemanticError>>()?,
                value.rows().into_iter().map(|row| row.cells()).collect(),
                value.syntax().clone(),
            ),
            TableValueSyntax::Inline(value) => (
                value
                    .header()
                    .into_iter()
                    .flat_map(|header| header.fields())
                    .map(|field| {
                        let name =
                            self.required(field.name(), field.syntax(), "a table field name")?;
                        let annotation = self.required(
                            field.annotation(),
                            field.syntax(),
                            "a table field kind",
                        )?;
                        Ok((node_text(name.syntax())?, annotation_schema(&annotation)?))
                    })
                    .collect::<Result<Vec<_>, SourceSemanticError>>()?,
                value.rows().into_iter().map(|row| row.cells()).collect(),
                value.syntax().clone(),
            ),
            TableValueSyntax::Regular(value) => (
                value
                    .header()
                    .into_iter()
                    .flat_map(|header| header.fields())
                    .map(|field| {
                        let name =
                            self.required(field.name(), field.syntax(), "a table field name")?;
                        let annotation = self.required(
                            field.annotation(),
                            field.syntax(),
                            "a table field kind",
                        )?;
                        Ok((node_text(name.syntax())?, annotation_schema(&annotation)?))
                    })
                    .collect::<Result<Vec<_>, SourceSemanticError>>()?,
                value.rows().into_iter().map(|row| row.cells()).collect(),
                value.syntax().clone(),
            ),
        };
        let mut inputs = Vec::new();
        let mut widths = Vec::new();
        for row in rows {
            widths.push(row.len());
            for (index, cell) in row.into_iter().enumerate() {
                let mut value = self.expression(&cell)?.0;
                if let Some((name, expected)) = headers.get(index) {
                    value = self.conform_table_value(value, *expected, name, cell.syntax())?;
                }
                inputs.push(value);
            }
        }
        if widths.iter().any(|width| *width != headers.len()) {
            return Err(SourceSemanticError {
                code: "source-semantics/table-row-width",
                message: "table row width does not match the declared header".to_owned(),
                anchor: SourceSemanticAnchor::for_node(&syntax),
            });
        }
        let schema_body = SchemaBody::Table {
            columns: headers
                .iter()
                .map(|(name, schema)| SchemaField {
                    name: name.clone(),
                    schema: schema_body(*schema),
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            rows: CardinalitySpec::Exact(DimensionExpr::Constant(widths.len() as u64)),
        };
        let names = headers
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>();
        Ok(self.emit_with_schema_body(
            "source/table",
            inputs,
            schema_body,
            &syntax,
            "table",
            Some(format!("headers={names:?};row-widths={widths:?}")),
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
        let mut value = stem;
        for item in subscripts.items() {
            let selector = self.subscript(&item)?;
            value = self.emit(
                "access/index",
                vec![value, selector],
                BuiltinSchema::Dynamic,
                item.syntax(),
                "slice",
                None,
            );
        }
        Ok(value)
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
        let saved = self.bindings.clone();
        let compiled = (|| {
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
                        let source = self.expression(&source)?.0;
                        for name in self.record_pattern(&pattern)? {
                            self.bindings.insert(name, source);
                        }
                        inputs.push(source);
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
        })();
        self.bindings = saved;
        compiled
    }

    fn record_pattern(
        &mut self,
        pattern: &PatternSyntax,
    ) -> Result<Vec<String>, SourceSemanticError> {
        self.required(pattern.value(), pattern.syntax(), "a pattern body")?;
        let mut bindings = Vec::new();
        collect_pattern_bindings(pattern.syntax(), &mut bindings)?;
        bindings.sort();
        bindings.dedup();
        self.patterns.push(SourceSemanticPattern {
            source: node_text(pattern.syntax())?,
            bindings: bindings.clone().into_boxed_slice(),
            anchor: SourceSemanticAnchor::for_node(pattern.syntax()),
        });
        Ok(bindings)
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
            self.record_pattern(&pattern)?;
            inputs.push(self.emit(
                "source/fsm-stage",
                Vec::new(),
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
        self.inputs.push((
            name,
            BuiltinSchema::Dynamic,
            SourceSemanticAnchor::for_node(node),
        ));
        Ok(PendingValue::Input(index))
    }

    fn value_for_name_node(
        &mut self,
        node: &SyntaxNode,
    ) -> Result<PendingValue, SourceSemanticError> {
        self.input_for_node(node)
    }

    fn schema_of(&self, value: PendingValue) -> BuiltinSchema {
        match value {
            PendingValue::Constant(index) => self.constants[index].schema,
            PendingValue::Input(index) => self.inputs[index as usize].1,
            PendingValue::State(index) => self.states[index as usize].schema,
            PendingValue::Node(index) => self.nodes[index as usize].schema,
        }
    }

    fn conform_table_value(
        &mut self,
        value: PendingValue,
        expected: BuiltinSchema,
        field: &str,
        syntax: &SyntaxNode,
    ) -> Result<PendingValue, SourceSemanticError> {
        let actual = self.schema_of(value);
        if expected == BuiltinSchema::Dynamic
            || actual == BuiltinSchema::Dynamic
            || actual == expected
        {
            return Ok(value);
        }
        if let PendingValue::Constant(index) = value
            && let ValueDataDraft::F64(number) = &self.constants[index].data
            && let Some(data) = scalar_data(expected, &number.to_f64().to_string())
        {
            self.constants[index] = PendingConstant {
                schema: expected,
                data,
            };
            return Ok(value);
        }
        Err(SourceSemanticError {
            code: "source-semantics/incompatible-table-field-kind",
            message: format!("table field {field} does not satisfy its kind annotation"),
            anchor: SourceSemanticAnchor::for_node(syntax),
        })
    }

    fn constant(&mut self, schema: BuiltinSchema, data: ValueDataDraft) -> PendingValue {
        let index = self.constants.len();
        self.constants.push(PendingConstant { schema, data });
        PendingValue::Constant(index)
    }

    fn emit(
        &mut self,
        operation: &str,
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
            schema_body: None,
            state: None,
            semantic: SourceSemanticNode {
                operation: operation.to_owned(),
                role,
                detail,
                anchor: SourceSemanticAnchor::for_node(syntax),
            },
        });
        PendingValue::Node(index)
    }

    fn emit_with_schema_body(
        &mut self,
        operation: &str,
        inputs: Vec<PendingValue>,
        schema_body: SchemaBody,
        syntax: &SyntaxNode,
        role: &'static str,
        detail: Option<String>,
    ) -> PendingValue {
        let value = self.emit(
            operation,
            inputs,
            BuiltinSchema::Dynamic,
            syntax,
            role,
            detail,
        );
        let PendingValue::Node(index) = value else {
            unreachable!("emit always returns a node")
        };
        self.nodes[index as usize].schema_body = Some(schema_body);
        value
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
        let schemas = BuiltinSchemas::build(self.anchor, &self.nodes)?;
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
            .map(|(name, schema, _)| SourceInput {
                name: name.clone(),
                schema: schemas.id(*schema),
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let mut contracts = Vec::with_capacity(self.nodes.len());
        let nodes = self
            .nodes
            .iter()
            .enumerate()
            .map(|(index, node)| {
                contracts.push(resolved_operation_contract(
                    &node.operation,
                    node.inputs.len(),
                    node.schema,
                    node.state.is_some(),
                ));
                SourceNode {
                    operation: node.operation.clone(),
                    requirement: None,
                    inputs: node
                        .inputs
                        .iter()
                        .map(|value| resolve_value(*value, &constant_ids))
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                    outputs: vec![match node.state {
                        Some(state) => SourceNodeOutput::State(state),
                        None => SourceNodeOutput::Derived {
                            schema: schemas.node_id(index, node.schema),
                        },
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
                schema: pending_schema(
                    output.source,
                    &constant_schemas,
                    &self.inputs,
                    &self.nodes,
                    &schemas,
                ),
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let source_map = SourceSemanticMap {
            inputs: self
                .inputs
                .into_iter()
                .map(|(_, _, anchor)| anchor)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            nodes: self
                .nodes
                .into_iter()
                .map(|node| node.semantic)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            patterns: self.patterns.into_boxed_slice(),
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
                states: self
                    .states
                    .iter()
                    .map(|state| SourceState {
                        schema: schemas.id(state.schema),
                        initializer: state.initializer.map(|index| constant_ids[index]),
                        producer_node: state.producer_node,
                        producer_output_ordinal: 0,
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
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
    inputs: &[(String, BuiltinSchema, SourceSemanticAnchor)],
    nodes: &[PendingNode],
    schemas: &BuiltinSchemas,
) -> SchemaId {
    match value {
        PendingValue::Constant(index) => constants
            .get(index)
            .map(|schema| schemas.id(*schema))
            .unwrap_or_else(|| schemas.id(BuiltinSchema::Dynamic)),
        PendingValue::Input(index) => inputs
            .get(index as usize)
            .map(|(_, schema, _)| schemas.id(*schema))
            .unwrap_or_else(|| schemas.id(BuiltinSchema::Dynamic)),
        PendingValue::State(index) => schemas.id(nodes
            .iter()
            .find(|node| node.state == Some(index))
            .map(|node| node.schema)
            .unwrap_or(BuiltinSchema::Dynamic)),
        PendingValue::Node(node) => nodes
            .get(node as usize)
            .map(|node_value| schemas.node_id(node as usize, node_value.schema))
            .unwrap_or_else(|| schemas.id(BuiltinSchema::Dynamic)),
    }
}

fn resolve_value(value: PendingValue, constants: &[mech_core::ConstantId]) -> SourceValue {
    match value {
        PendingValue::Constant(index) => SourceValue::Constant(constants[index]),
        PendingValue::Input(index) => SourceValue::Input(index),
        PendingValue::State(index) => SourceValue::State(index),
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

fn resolved_operation_contract(
    operation: &OperationReference,
    input_count: usize,
    output_schema: BuiltinSchema,
    state_output: bool,
) -> Option<OperationContractDeclaration> {
    let name = operation.canonical_name();
    let supported = name.starts_with("math/")
        || name.starts_with("compare/")
        || name.starts_with("logic/")
        || name.starts_with("range/")
        || name == "matrix/transpose"
        || name == "core/assign";
    supported.then(|| operation_contract(input_count, output_schema, state_output))
}

fn operation_contract(
    input_count: usize,
    output_schema: BuiltinSchema,
    state_output: bool,
) -> OperationContractDeclaration {
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
                shape: if state_output {
                    ShapeRule::SameAsInput { input: 0 }
                } else {
                    ShapeRule::Declared
                },
            },
            alias: AliasPolicy::NoAlias,
            change_detection: if state_output {
                ChangeDetectionPolicy::KernelReported
            } else if matches!(
                output_schema,
                BuiltinSchema::Bool
                    | BuiltinSchema::U8
                    | BuiltinSchema::U16
                    | BuiltinSchema::U32
                    | BuiltinSchema::U64
                    | BuiltinSchema::U128
                    | BuiltinSchema::I8
                    | BuiltinSchema::I16
                    | BuiltinSchema::I32
                    | BuiltinSchema::I64
                    | BuiltinSchema::I128
                    | BuiltinSchema::F32
                    | BuiltinSchema::F64
                    | BuiltinSchema::C64
                    | BuiltinSchema::R64
            ) {
                ChangeDetectionPolicy::ExactScalar
            } else {
                ChangeDetectionPolicy::AlwaysChanged
            },
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    }
}

fn operator_name(operator: CanonicalOperator) -> (&'static str, Option<BuiltinSchema>) {
    use CanonicalOperator::*;
    match operator {
        Add => ("math/add", None),
        Subtract => ("math/sub", None),
        Multiply => ("math/mul", None),
        Divide => ("math/div", None),
        Modulus => ("math/mod", None),
        Power => ("math/pow", None),
        MatrixMultiply => ("matrix/matmul", None),
        MatrixSolve => ("matrix/solve", None),
        DotProduct => ("matrix/dot", None),
        CrossProduct => ("matrix/cross", None),
        RangeInclusive => ("range/inclusive", None),
        RangeExclusive => ("range/exclusive", None),
        NotEqual => ("compare/neq", Some(BuiltinSchema::Bool)),
        EqualTo => ("compare/eq", Some(BuiltinSchema::Bool)),
        StrictNotEqual => ("compare/sneq", Some(BuiltinSchema::Bool)),
        StrictEqual => ("compare/seq", Some(BuiltinSchema::Bool)),
        GreaterThan => ("compare/gt", Some(BuiltinSchema::Bool)),
        LessThan => ("compare/lt", Some(BuiltinSchema::Bool)),
        GreaterThanEqual => ("compare/gte", Some(BuiltinSchema::Bool)),
        LessThanEqual => ("compare/lte", Some(BuiltinSchema::Bool)),
        Or => ("logic/or", Some(BuiltinSchema::Bool)),
        And => ("logic/and", Some(BuiltinSchema::Bool)),
        Not => ("logic/not", Some(BuiltinSchema::Bool)),
        Xor => ("logic/xor", Some(BuiltinSchema::Bool)),
        InnerJoin => ("table/join", None),
        LeftOuterJoin => ("table/left-outer-join", None),
        RightOuterJoin => ("table/right-outer-join", None),
        FullOuterJoin => ("table/full-outer-join", None),
        LeftSemiJoin => ("table/left-semi-join", None),
        LeftAntiJoin => ("table/left-anti-join", None),
        Union => ("set/union", None),
        Intersection => ("set/intersection", None),
        Difference => ("set/difference", None),
        Complement => ("set/complement", None),
        Subset => ("set/subset", Some(BuiltinSchema::Bool)),
        Superset => ("set/superset", Some(BuiltinSchema::Bool)),
        ProperSubset => ("set/proper_subset", Some(BuiltinSchema::Bool)),
        ProperSuperset => ("set/proper-superset", Some(BuiltinSchema::Bool)),
        ElementOf => ("set/element-of", Some(BuiltinSchema::Bool)),
        NotElementOf => ("set/not-element-of", Some(BuiltinSchema::Bool)),
        SymmetricDifference => ("set/symmetric-difference", None),
    }
}

fn annotation_schema(
    annotation: &KindAnnotationSyntax,
) -> Result<BuiltinSchema, SourceSemanticError> {
    let source = node_text(annotation.syntax())?;
    let name = source
        .strip_prefix('<')
        .and_then(|source| source.strip_suffix('>'))
        .map(|source| source.trim_end_matches('?'))
        .unwrap_or(source.as_str());
    let schema = match name {
        "bool" => BuiltinSchema::Bool,
        "string" => BuiltinSchema::String,
        "u8" => BuiltinSchema::U8,
        "u16" => BuiltinSchema::U16,
        "u32" => BuiltinSchema::U32,
        "u64" => BuiltinSchema::U64,
        "u128" => BuiltinSchema::U128,
        "i8" => BuiltinSchema::I8,
        "i16" => BuiltinSchema::I16,
        "i32" => BuiltinSchema::I32,
        "i64" => BuiltinSchema::I64,
        "i128" => BuiltinSchema::I128,
        "f32" => BuiltinSchema::F32,
        "f64" => BuiltinSchema::F64,
        "c64" => BuiltinSchema::C64,
        "r64" => BuiltinSchema::R64,
        "*" | "_" => BuiltinSchema::Dynamic,
        _ => BuiltinSchema::Dynamic,
    };
    Ok(schema)
}

fn require_literal_schema(
    annotation: Option<BuiltinSchema>,
    actual: BuiltinSchema,
    syntax: &SyntaxNode,
) -> Result<(), SourceSemanticError> {
    if annotation.is_none_or(|schema| schema == actual || schema == BuiltinSchema::Dynamic) {
        return Ok(());
    }
    Err(SourceSemanticError {
        code: "source-semantics/incompatible-literal-kind",
        message: "literal value does not satisfy its kind annotation".to_owned(),
        anchor: SourceSemanticAnchor::for_node(syntax),
    })
}

fn numeric_suffix(source: &str) -> (&str, Option<BuiltinSchema>) {
    for (suffix, schema) in [
        ("u128", BuiltinSchema::U128),
        ("i128", BuiltinSchema::I128),
        ("u64", BuiltinSchema::U64),
        ("i64", BuiltinSchema::I64),
        ("u32", BuiltinSchema::U32),
        ("i32", BuiltinSchema::I32),
        ("u16", BuiltinSchema::U16),
        ("i16", BuiltinSchema::I16),
        ("f64", BuiltinSchema::F64),
        ("c64", BuiltinSchema::C64),
        ("r64", BuiltinSchema::R64),
        ("u8", BuiltinSchema::U8),
        ("i8", BuiltinSchema::I8),
        ("f32", BuiltinSchema::F32),
    ] {
        if let Some(number) = source.strip_suffix(suffix) {
            return (number, Some(schema));
        }
    }
    (source, None)
}

fn integer_value(source: &str) -> Option<i128> {
    let (negative, magnitude) = source
        .strip_prefix('-')
        .map_or((false, source), |value| (true, value));
    let (radix, digits) = if let Some(value) = magnitude.strip_prefix("0x") {
        (16, value)
    } else if let Some(value) = magnitude.strip_prefix("0o") {
        (8, value)
    } else if let Some(value) = magnitude.strip_prefix("0b") {
        (2, value)
    } else {
        (10, magnitude)
    };
    let magnitude = i128::from_str_radix(digits, radix).ok()?;
    Some(if negative { -magnitude } else { magnitude })
}

fn real_value(source: &str) -> Option<f64> {
    integer_value(source)
        .map(|value| value as f64)
        .or_else(|| source.parse::<f64>().ok())
}

fn scalar_data(schema: BuiltinSchema, source: &str) -> Option<ValueDataDraft> {
    let integer = || integer_value(source);
    let float = || real_value(source);
    Some(match schema {
        BuiltinSchema::U8 => ValueDataDraft::U8(u8::try_from(integer()?).ok()?),
        BuiltinSchema::U16 => ValueDataDraft::U16(u16::try_from(integer()?).ok()?),
        BuiltinSchema::U32 => ValueDataDraft::U32(u32::try_from(integer()?).ok()?),
        BuiltinSchema::U64 => ValueDataDraft::U64(u64::try_from(integer()?).ok()?),
        BuiltinSchema::U128 => ValueDataDraft::U128(u128::try_from(integer()?).ok()?),
        BuiltinSchema::I8 => ValueDataDraft::I8(i8::try_from(integer()?).ok()?),
        BuiltinSchema::I16 => ValueDataDraft::I16(i16::try_from(integer()?).ok()?),
        BuiltinSchema::I32 => ValueDataDraft::I32(i32::try_from(integer()?).ok()?),
        BuiltinSchema::I64 => ValueDataDraft::I64(i64::try_from(integer()?).ok()?),
        BuiltinSchema::I128 => ValueDataDraft::I128(integer()?),
        BuiltinSchema::F32 => ValueDataDraft::F32(F32Bits::from_f32(float()? as f32)),
        BuiltinSchema::F64 => ValueDataDraft::F64(F64Bits::from_f64(float()?)),
        BuiltinSchema::C64 => ValueDataDraft::Complex64(Complex64Bits::new(
            F64Bits::from_f64(float()?),
            F64Bits::from_f64(0.0),
        )),
        BuiltinSchema::R64 => ValueDataDraft::Rational64 {
            numerator: i64::try_from(integer()?).ok()?,
            denominator: 1,
        },
        _ => return None,
    })
}

fn decode_number(
    source: &str,
    annotation: Option<BuiltinSchema>,
) -> Option<(BuiltinSchema, ValueDataDraft)> {
    let source = source.replace('_', "");
    if let Some(complex) = source.strip_suffix(['i', 'j']) {
        let split = complex
            .char_indices()
            .skip(1)
            .filter_map(|(index, character)| {
                matches!(character, '+' | '-')
                    .then_some(index)
                    .filter(|index| !matches!(complex.as_bytes()[index - 1], b'e' | b'E'))
            })
            .last();
        let (real, imaginary) = split.map_or(("0", complex), |index| complex.split_at(index));
        let schema = annotation
            .filter(|schema| *schema != BuiltinSchema::Dynamic)
            .unwrap_or(BuiltinSchema::C64);
        if schema != BuiltinSchema::C64 {
            return None;
        }
        return Some((
            schema,
            ValueDataDraft::Complex64(Complex64Bits::new(
                F64Bits::from_f64(real_value(real)?),
                F64Bits::from_f64(real_value(imaginary)?),
            )),
        ));
    }
    if let Some((numerator, denominator)) = source.split_once('/') {
        let (numerator, _) = numeric_suffix(numerator);
        let (denominator, _) = numeric_suffix(denominator);
        let schema = annotation
            .filter(|schema| *schema != BuiltinSchema::Dynamic)
            .unwrap_or(BuiltinSchema::R64);
        if schema != BuiltinSchema::R64 {
            return None;
        }
        let denominator = u64::try_from(integer_value(denominator)?).ok()?;
        if denominator == 0 {
            return None;
        }
        return Some((
            schema,
            ValueDataDraft::Rational64 {
                numerator: i64::try_from(integer_value(numerator)?).ok()?,
                denominator,
            },
        ));
    }
    let (number, suffix) = numeric_suffix(&source);
    let annotated = annotation.filter(|schema| *schema != BuiltinSchema::Dynamic);
    let schema = if number.contains(['e', 'E']) {
        annotated.unwrap_or(BuiltinSchema::F64)
    } else {
        annotated.or(suffix).unwrap_or(BuiltinSchema::F64)
    };
    scalar_data(schema, number).map(|data| (schema, data))
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
            '0' => '\0',
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            '\\' => '\\',
            '"' => '"',
            'u' => {
                if chars.next()? != '{' {
                    return None;
                }
                let mut digits = String::new();
                loop {
                    let next = chars.next()?;
                    if next == '}' {
                        break;
                    }
                    if !next.is_ascii_hexdigit() || digits.len() == 6 {
                        return None;
                    }
                    digits.push(next);
                }
                let scalar = u32::from_str_radix(&digits, 16).ok()?;
                char::from_u32(scalar)?
            }
            _ => return None,
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
