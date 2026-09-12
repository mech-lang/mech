use std::collections::{BTreeMap, BTreeSet};

use mech_core::snapshot::{
    Complex64Bits, F32Bits, F64Bits, OptionDraft, ReifiedKind, ReifiedTypeDraft,
    SnapshotValidationContext,
};
use mech_core::{
    AccessMode, AliasPolicy, BuiltinScalarKind, CanonicalNominalPath, CardinalitySpec,
    ChangeDetectionPolicy, ConstantStore, ConstantStoreBuilder, DeliveryMode, DimensionExpr,
    DimensionLifetime, DimensionParameterDeclaration, DimensionParameterId,
    DimensionParameterOrigin, ExternalInteraction, FloatWidth, InputPortLayout, InputPortPolicy,
    IntegerWidth, KindExpr, KindId, NamedKindPathResolver, NodeId, NominalKey, NominalKind,
    OperationContractDeclaration, OutputConstruction, OutputPortPolicy, ResolvedType, SchemaBody,
    SchemaDraft, SchemaField, SchemaId, SchemaTable, SchemaTableBuilder, ShapeContractReference,
    ShapeRule, ValueDataDraft, ValueDraft, execute_conversion_draft, plan_explicit_cast,
    plan_numeric_promotion,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceSemanticMatchArm {
    pub node: u32,
    pub pattern: u32,
    pub guard_input: Option<u32>,
    pub result_input: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceSemanticComprehensionQualifierRole {
    Generator { pattern: u32 },
    Definition,
    Filter,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceSemanticComprehensionQualifier {
    pub node: u32,
    pub input_ordinal: u32,
    pub role: SourceSemanticComprehensionQualifierRole,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SourceSemanticMap {
    pub inputs: Box<[SourceSemanticAnchor]>,
    pub nodes: Box<[SourceSemanticNode]>,
    pub patterns: Box<[SourceSemanticPattern]>,
    pub match_arms: Box<[SourceSemanticMatchArm]>,
    pub comprehension_qualifiers: Box<[SourceSemanticComprehensionQualifier]>,
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
    OptionBool,
    OptionString,
    OptionU8,
    OptionU16,
    OptionU32,
    OptionU64,
    OptionU128,
    OptionI8,
    OptionI16,
    OptionI32,
    OptionI64,
    OptionI128,
    OptionF32,
    OptionF64,
    OptionC64,
    OptionR64,
}

struct BuiltinSchemas {
    table: SchemaTable,
    ids: BTreeMap<BuiltinSchema, SchemaId>,
    node_ids: BTreeMap<usize, SchemaId>,
    constant_ids: BTreeMap<usize, SchemaId>,
}

impl BuiltinSchemas {
    fn build(
        anchor: SourceSemanticAnchor,
        nodes: &[PendingNode],
        constants: &[PendingConstant],
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
            BuiltinSchema::OptionBool,
            BuiltinSchema::OptionString,
            BuiltinSchema::OptionU8,
            BuiltinSchema::OptionU16,
            BuiltinSchema::OptionU32,
            BuiltinSchema::OptionU64,
            BuiltinSchema::OptionU128,
            BuiltinSchema::OptionI8,
            BuiltinSchema::OptionI16,
            BuiltinSchema::OptionI32,
            BuiltinSchema::OptionI64,
            BuiltinSchema::OptionI128,
            BuiltinSchema::OptionF32,
            BuiltinSchema::OptionF64,
            BuiltinSchema::OptionC64,
            BuiltinSchema::OptionR64,
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
            .filter_map(|(index, node)| node.schema_body.as_ref().map(|body| (index, node, body)))
            .map(|(index, node, body)| {
                let schema = SchemaDraft {
                    dimension_parameters: node.schema_parameters.clone(),
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
        let constant_handles = constants
            .iter()
            .enumerate()
            .filter_map(|(index, constant)| {
                constant
                    .schema_body
                    .as_ref()
                    .map(|body| (index, constant, body))
            })
            .map(|(index, constant, body)| {
                let schema = SchemaDraft {
                    dimension_parameters: constant.schema_parameters.clone(),
                    body: body.clone(),
                }
                .finalize()
                .map_err(|error| internal(anchor, format!("invalid constant schema: {error:?}")))?;
                builder
                    .insert(schema)
                    .map(|handle| (index, handle))
                    .map_err(|error| {
                        internal(
                            anchor,
                            format!("unable to retain constant schema: {error:?}"),
                        )
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
        let constant_ids = constant_handles
            .into_iter()
            .map(|(index, handle)| {
                build
                    .resolve(handle)
                    .map(|id| (index, id))
                    .map_err(|error| {
                        internal(
                            anchor,
                            format!("unable to resolve constant schema: {error:?}"),
                        )
                    })
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let (table, _) = build.into_parts();
        Ok(Self {
            table,
            ids,
            node_ids,
            constant_ids,
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

    fn constant_id(&self, index: usize, fallback: BuiltinSchema) -> SchemaId {
        self.constant_ids
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
        option => SchemaBody::Option(Box::new(schema_body(
            option_payload_schema(option).expect("closed optional builtin schema"),
        ))),
    }
}

fn builtin_kind(schema: BuiltinSchema) -> Option<BuiltinScalarKind> {
    Some(match schema {
        BuiltinSchema::Bool => BuiltinScalarKind::Bool,
        BuiltinSchema::String => BuiltinScalarKind::String,
        BuiltinSchema::U8 => BuiltinScalarKind::U8,
        BuiltinSchema::U16 => BuiltinScalarKind::U16,
        BuiltinSchema::U32 => BuiltinScalarKind::U32,
        BuiltinSchema::U64 => BuiltinScalarKind::U64,
        BuiltinSchema::U128 => BuiltinScalarKind::U128,
        BuiltinSchema::I8 => BuiltinScalarKind::I8,
        BuiltinSchema::I16 => BuiltinScalarKind::I16,
        BuiltinSchema::I32 => BuiltinScalarKind::I32,
        BuiltinSchema::I64 => BuiltinScalarKind::I64,
        BuiltinSchema::I128 => BuiltinScalarKind::I128,
        BuiltinSchema::F32 => BuiltinScalarKind::F32,
        BuiltinSchema::F64 => BuiltinScalarKind::F64,
        BuiltinSchema::C64 => BuiltinScalarKind::C64,
        BuiltinSchema::R64 => BuiltinScalarKind::R64,
        BuiltinSchema::Dynamic
        | BuiltinSchema::OptionBool
        | BuiltinSchema::OptionString
        | BuiltinSchema::OptionU8
        | BuiltinSchema::OptionU16
        | BuiltinSchema::OptionU32
        | BuiltinSchema::OptionU64
        | BuiltinSchema::OptionU128
        | BuiltinSchema::OptionI8
        | BuiltinSchema::OptionI16
        | BuiltinSchema::OptionI32
        | BuiltinSchema::OptionI64
        | BuiltinSchema::OptionI128
        | BuiltinSchema::OptionF32
        | BuiltinSchema::OptionF64
        | BuiltinSchema::OptionC64
        | BuiltinSchema::OptionR64 => return None,
    })
}

fn option_schema(payload: BuiltinSchema) -> Option<BuiltinSchema> {
    Some(match payload {
        BuiltinSchema::Bool => BuiltinSchema::OptionBool,
        BuiltinSchema::String => BuiltinSchema::OptionString,
        BuiltinSchema::U8 => BuiltinSchema::OptionU8,
        BuiltinSchema::U16 => BuiltinSchema::OptionU16,
        BuiltinSchema::U32 => BuiltinSchema::OptionU32,
        BuiltinSchema::U64 => BuiltinSchema::OptionU64,
        BuiltinSchema::U128 => BuiltinSchema::OptionU128,
        BuiltinSchema::I8 => BuiltinSchema::OptionI8,
        BuiltinSchema::I16 => BuiltinSchema::OptionI16,
        BuiltinSchema::I32 => BuiltinSchema::OptionI32,
        BuiltinSchema::I64 => BuiltinSchema::OptionI64,
        BuiltinSchema::I128 => BuiltinSchema::OptionI128,
        BuiltinSchema::F32 => BuiltinSchema::OptionF32,
        BuiltinSchema::F64 => BuiltinSchema::OptionF64,
        BuiltinSchema::C64 => BuiltinSchema::OptionC64,
        BuiltinSchema::R64 => BuiltinSchema::OptionR64,
        _ => return None,
    })
}

fn option_payload_schema(option: BuiltinSchema) -> Option<BuiltinSchema> {
    Some(match option {
        BuiltinSchema::OptionBool => BuiltinSchema::Bool,
        BuiltinSchema::OptionString => BuiltinSchema::String,
        BuiltinSchema::OptionU8 => BuiltinSchema::U8,
        BuiltinSchema::OptionU16 => BuiltinSchema::U16,
        BuiltinSchema::OptionU32 => BuiltinSchema::U32,
        BuiltinSchema::OptionU64 => BuiltinSchema::U64,
        BuiltinSchema::OptionU128 => BuiltinSchema::U128,
        BuiltinSchema::OptionI8 => BuiltinSchema::I8,
        BuiltinSchema::OptionI16 => BuiltinSchema::I16,
        BuiltinSchema::OptionI32 => BuiltinSchema::I32,
        BuiltinSchema::OptionI64 => BuiltinSchema::I64,
        BuiltinSchema::OptionI128 => BuiltinSchema::I128,
        BuiltinSchema::OptionF32 => BuiltinSchema::F32,
        BuiltinSchema::OptionF64 => BuiltinSchema::F64,
        BuiltinSchema::OptionC64 => BuiltinSchema::C64,
        BuiltinSchema::OptionR64 => BuiltinSchema::R64,
        _ => return None,
    })
}

fn builtin_schema(kind: BuiltinScalarKind) -> Option<BuiltinSchema> {
    Some(match kind {
        BuiltinScalarKind::Bool => BuiltinSchema::Bool,
        BuiltinScalarKind::String => BuiltinSchema::String,
        BuiltinScalarKind::U8 => BuiltinSchema::U8,
        BuiltinScalarKind::U16 => BuiltinSchema::U16,
        BuiltinScalarKind::U32 => BuiltinSchema::U32,
        BuiltinScalarKind::U64 => BuiltinSchema::U64,
        BuiltinScalarKind::U128 => BuiltinSchema::U128,
        BuiltinScalarKind::I8 => BuiltinSchema::I8,
        BuiltinScalarKind::I16 => BuiltinSchema::I16,
        BuiltinScalarKind::I32 => BuiltinSchema::I32,
        BuiltinScalarKind::I64 => BuiltinSchema::I64,
        BuiltinScalarKind::I128 => BuiltinSchema::I128,
        BuiltinScalarKind::F32 => BuiltinSchema::F32,
        BuiltinScalarKind::F64 => BuiltinSchema::F64,
        BuiltinScalarKind::C64 => BuiltinSchema::C64,
        BuiltinScalarKind::R64 => BuiltinSchema::R64,
        BuiltinScalarKind::C32 => return None,
    })
}

fn builtin_kind_from_resolved(value: &ResolvedType) -> Option<BuiltinScalarKind> {
    let KindExpr::Named(kind) = value.kind() else {
        return None;
    };
    BuiltinScalarKind::from_kind_id(*kind)
}

fn resolved_builtin_type(
    kind: BuiltinScalarKind,
    syntax: &SyntaxNode,
) -> Result<ResolvedType, SourceSemanticError> {
    ResolvedType::new(kind.kind_expr(), Box::new([])).map_err(|error| {
        internal(
            SourceSemanticAnchor::for_node(syntax),
            format!("invalid builtin source type: {error}"),
        )
    })
}

fn is_numeric(kind: BuiltinScalarKind) -> bool {
    !matches!(kind, BuiltinScalarKind::Bool | BuiltinScalarKind::String)
}

fn builtin_kind_named(name: &str) -> Option<BuiltinScalarKind> {
    Some(match name {
        "bool" => BuiltinScalarKind::Bool,
        "string" => BuiltinScalarKind::String,
        "u8" => BuiltinScalarKind::U8,
        "u16" => BuiltinScalarKind::U16,
        "u32" => BuiltinScalarKind::U32,
        "u64" => BuiltinScalarKind::U64,
        "u128" => BuiltinScalarKind::U128,
        "i8" => BuiltinScalarKind::I8,
        "i16" => BuiltinScalarKind::I16,
        "i32" => BuiltinScalarKind::I32,
        "i64" => BuiltinScalarKind::I64,
        "i128" => BuiltinScalarKind::I128,
        "f32" => BuiltinScalarKind::F32,
        "f64" => BuiltinScalarKind::F64,
        "c64" => BuiltinScalarKind::C64,
        "r64" => BuiltinScalarKind::R64,
        _ => return None,
    })
}

fn annotation_kind_expr(source: &str) -> Option<KindExpr> {
    let name = source.strip_prefix('<')?.strip_suffix('>')?;
    let (name, optional) = name
        .strip_suffix('?')
        .map_or((name, false), |name| (name, true));
    let kind = match name {
        "*" => Some(KindExpr::Wildcard),
        "_" => Some(KindExpr::Hole),
        _ => builtin_kind_named(name).map(BuiltinScalarKind::kind_expr),
    }?;
    Some(if optional {
        KindExpr::Option(Box::new(kind))
    } else {
        kind
    })
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
    schema_body: Option<SchemaBody>,
    schema_parameters: Box<[DimensionParameterDeclaration]>,
    data: ValueDataDraft,
}

struct PendingNode {
    operation: OperationReference,
    inputs: Vec<PendingValue>,
    schema: BuiltinSchema,
    schema_body: Option<SchemaBody>,
    schema_parameters: Box<[DimensionParameterDeclaration]>,
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

struct RecordedPattern {
    index: u32,
    bindings: Vec<String>,
    syntax: SyntaxNode,
}

struct BuiltinKindPaths(BTreeMap<KindId, CanonicalNominalPath>);

impl BuiltinKindPaths {
    fn build(anchor: SourceSemanticAnchor) -> Result<Self, SourceSemanticError> {
        BuiltinScalarKind::ALL
            .into_iter()
            .map(|kind| {
                kind.canonical_path()
                    .map(|path| (kind.kind_id(), path))
                    .map_err(|error| {
                        internal(anchor, format!("invalid builtin kind path: {error:?}"))
                    })
            })
            .collect::<Result<BTreeMap<_, _>, _>>()
            .map(Self)
    }
}

impl NamedKindPathResolver for BuiltinKindPaths {
    fn canonical_path(&self, id: KindId) -> Option<&CanonicalNominalPath> {
        self.0.get(&id)
    }
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
    match_arms: Vec<SourceSemanticMatchArm>,
    comprehension_qualifiers: Vec<SourceSemanticComprehensionQualifier>,
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
            match_arms: Vec::new(),
            comprehension_qualifiers: Vec::new(),
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
            let mut layouts = Vec::with_capacity(arms.len());
            for arm in arms {
                let pattern = self.required(arm.pattern(), arm.syntax(), "a match pattern")?;
                let saved = self.bindings.clone();
                let result = (|| {
                    let pattern = self.record_pattern(&pattern)?;
                    self.bind_pattern(&pattern, value)?;
                    let guard_input = if let Some(guard) = arm.guard() {
                        let ordinal = inputs.len() as u32;
                        inputs.push(self.expression(&guard)?.0);
                        Some(ordinal)
                    } else {
                        None
                    };
                    let result = self.required(arm.value(), arm.syntax(), "a match result")?;
                    let result_input = inputs.len() as u32;
                    inputs.push(self.expression(&result)?.0);
                    layouts.push((pattern.index, guard_input, result_input));
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
            let PendingValue::Node(node) = value else {
                unreachable!("emit always returns a node")
            };
            self.match_arms.extend(layouts.into_iter().map(
                |(pattern, guard_input, result_input)| SourceSemanticMatchArm {
                    node,
                    pattern,
                    guard_input,
                    result_input,
                },
            ));
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
            value = self.emit_operator(operator, value, rhs, syntax)?;
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
    ) -> Result<PendingValue, SourceSemanticError> {
        let (name, fixed_schema) = operator_name(operator);
        let lhs_schema = self.schema_of(lhs);
        let rhs_schema = self.schema_of(rhs);
        let (lhs, rhs, promoted) = match operator {
            CanonicalOperator::Add
            | CanonicalOperator::Subtract
            | CanonicalOperator::Multiply
            | CanonicalOperator::Divide
            | CanonicalOperator::Modulus
            | CanonicalOperator::NotEqual
            | CanonicalOperator::EqualTo
            | CanonicalOperator::GreaterThan
            | CanonicalOperator::LessThan
            | CanonicalOperator::GreaterThanEqual
            | CanonicalOperator::LessThanEqual => self.promote_operands(lhs, rhs, syntax)?,
            CanonicalOperator::Power
                if lhs_schema == BuiltinSchema::R64 && rhs_schema == BuiltinSchema::I32 =>
            {
                (lhs, rhs, Some(BuiltinSchema::R64))
            }
            CanonicalOperator::Power => self.promote_operands(lhs, rhs, syntax)?,
            _ => (lhs, rhs, None),
        };
        let schema = fixed_schema.unwrap_or_else(|| {
            promoted.unwrap_or_else(|| {
                if lhs_schema == rhs_schema {
                    lhs_schema
                } else {
                    BuiltinSchema::Dynamic
                }
            })
        });
        Ok(self.emit(name, vec![lhs, rhs], schema, syntax, "operator", None))
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
        if factor.transpose().is_some() {
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
        let (values, element_schema) = match values.as_slice() {
            [first, second] => {
                let (first, second, promoted) =
                    self.promote_operands(*first, *second, range.syntax())?;
                let element = promoted
                    .or_else(|| {
                        (self.schema_of(first) == self.schema_of(second))
                            .then(|| self.schema_of(first))
                    })
                    .filter(|schema| *schema != BuiltinSchema::Dynamic)
                    .ok_or_else(|| SourceSemanticError {
                        code: "source-semantics/unresolved-range-kind",
                        message: "range endpoint kinds must resolve before artifact construction"
                            .to_owned(),
                        anchor: SourceSemanticAnchor::for_node(range.syntax()),
                    })?;
                (vec![first, second], element)
            }
            [first, second, third] => {
                let (first, mut second, first_schema) =
                    self.promote_operands(*first, *second, range.syntax())?;
                let (first, third, final_schema) =
                    self.promote_operands(first, *third, range.syntax())?;
                let element = final_schema
                    .or(first_schema)
                    .or_else(|| {
                        (self.schema_of(first) == self.schema_of(third))
                            .then(|| self.schema_of(first))
                    })
                    .filter(|schema| *schema != BuiltinSchema::Dynamic)
                    .ok_or_else(|| SourceSemanticError {
                        code: "source-semantics/unresolved-range-kind",
                        message: "range endpoint kinds must resolve before artifact construction"
                            .to_owned(),
                        anchor: SourceSemanticAnchor::for_node(range.syntax()),
                    })?;
                second = self.conform_value(
                    second,
                    element,
                    range.syntax(),
                    "source-semantics/incompatible-range-kind",
                    "range endpoints do not share a promotable numeric kind",
                )?;
                (vec![first, second, third], element)
            }
            _ => unreachable!("range arity was validated"),
        };
        let extent = DimensionParameterId::new(0);
        if !builtin_kind(element_schema).is_some_and(is_numeric) {
            return Err(SourceSemanticError {
                code: "source-semantics/non-numeric-range-kind",
                message: "range endpoints require a concrete numeric kind".to_owned(),
                anchor: SourceSemanticAnchor::for_node(range.syntax()),
            });
        }
        Ok(self.emit_with_schema_draft(
            name,
            values,
            SchemaDraft {
                dimension_parameters: vec![DimensionParameterDeclaration {
                    id: extent,
                    origin: DimensionParameterOrigin::Inferred,
                    lifetime: DimensionLifetime::Turn,
                    lower_bound: DimensionExpr::Constant(0),
                    upper_bound: None,
                }]
                .into_boxed_slice(),
                body: SchemaBody::Matrix {
                    element: Box::new(schema_body(element_schema)),
                    dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Parameter(extent)]
                        .into_boxed_slice(),
                },
            },
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
        let mut value = self.expression(&expression)?.0;
        if let Some(annotation) = variable.annotation() {
            let expected = annotation_schema(&annotation)?;
            value = self.conform_value(
                value,
                expected,
                definition.syntax(),
                "source-semantics/incompatible-definition-kind",
                "definition value does not satisfy the declared kind",
            )?;
        }
        let bound = if definition.mutability_marker().is_some() {
            let PendingValue::Constant(initializer) = value else {
                return Err(SourceSemanticError {
                    code: "source-semantics/nonconstant-state-initializer",
                    message: "mutable definitions require a constant initializer".to_owned(),
                    anchor: SourceSemanticAnchor::for_node(expression.syntax()),
                });
            };
            let schema = self.schema_of(value);
            let state = u32::try_from(self.states.len()).map_err(|_| SourceSemanticError {
                code: "source-semantics/state-identity-exhausted",
                message: "canonical state count exceeds SourceProgram identity space".to_owned(),
                anchor: SourceSemanticAnchor::for_node(definition.syntax()),
            })?;
            let node = self.nodes.len() as u32;
            self.states.push(PendingState {
                schema,
                initializer: Some(initializer),
                producer_node: node,
            });
            self.nodes.push(PendingNode {
                operation: operation_reference("core/assign"),
                inputs: vec![PendingValue::State(state)],
                schema,
                schema_body: None,
                schema_parameters: Box::new([]),
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
            return self.literal_constant(
                annotation,
                BuiltinSchema::Bool,
                ValueDataDraft::Bool(true),
                literal.syntax(),
            );
        }
        if literal.false_token().is_some() {
            return self.literal_constant(
                annotation,
                BuiltinSchema::Bool,
                ValueDataDraft::Bool(false),
                literal.syntax(),
            );
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
                self.literal_constant(
                    annotation,
                    BuiltinSchema::String,
                    ValueDataDraft::String(decoded),
                    value.syntax(),
                )
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
            LiteralValueSyntax::Atom(value) => {
                let source = node_text(value.syntax())?;
                let path = CanonicalNominalPath::new(
                    source
                        .trim_start_matches(':')
                        .split('/')
                        .filter(|segment| !segment.is_empty())
                        .map(str::to_owned)
                        .collect::<Vec<_>>(),
                )
                .map_err(|error| {
                    internal(
                        SourceSemanticAnchor::for_node(value.syntax()),
                        format!("invalid atom path: {error:?}"),
                    )
                })?;
                let key = NominalKey::from_path(NominalKind::Atom, &path);
                Ok(self.constant_exact(SchemaBody::Atom(key), ValueDataDraft::Atom))
            }
            LiteralValueSyntax::KindAnnotation(value) => self.kind_value(&value),
        }
    }

    fn literal_constant(
        &mut self,
        annotation: Option<BuiltinSchema>,
        actual: BuiltinSchema,
        data: ValueDataDraft,
        syntax: &SyntaxNode,
    ) -> Result<PendingValue, SourceSemanticError> {
        require_literal_schema(annotation, actual, syntax)?;
        if let Some(option) = annotation.filter(|schema| option_payload_schema(*schema).is_some()) {
            return Ok(self.constant(
                option,
                ValueDataDraft::Option(OptionDraft {
                    present: true,
                    value: Some(Box::new(data)),
                }),
            ));
        }
        Ok(self.constant(actual, data))
    }

    fn kind_value(
        &mut self,
        kind: &KindAnnotationSyntax,
    ) -> Result<PendingValue, SourceSemanticError> {
        let source = node_text(kind.syntax())?;
        let kind_expr = annotation_kind_expr(&source).ok_or_else(|| SourceSemanticError {
            code: "source-semantics/unsupported-kind-value",
            message: "kind value is not yet representable by the canonical type store".to_owned(),
            anchor: SourceSemanticAnchor::for_node(kind.syntax()),
        })?;
        let paths = BuiltinKindPaths::build(SourceSemanticAnchor::for_node(kind.syntax()))?;
        let reified = ReifiedKind::from_closed_kind(&kind_expr, &[], &paths).map_err(|error| {
            internal(
                SourceSemanticAnchor::for_node(kind.syntax()),
                format!("unable to canonicalize kind value: {error:?}"),
            )
        })?;
        Ok(self.constant_exact(
            SchemaBody::ReifiedType,
            ValueDataDraft::Type(ReifiedTypeDraft::CanonicalKind(
                reified.canonical_bytes().to_vec().into_boxed_slice(),
            )),
        ))
    }

    fn structure(
        &mut self,
        structure: &StructureSyntax,
    ) -> Result<PendingValue, SourceSemanticError> {
        let value = self.required(structure.value(), structure.syntax(), "a structure value")?;
        match value {
            StructureValueSyntax::Matrix(value) => self.matrix(&value),
            StructureValueSyntax::MatrixComprehension(value) => self.matrix_comprehension(&value),
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
        let (mut headers, rows, syntax): (
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
        let mut widths = Vec::new();
        let mut compiled_rows = Vec::new();
        for row in rows {
            widths.push(row.len());
            compiled_rows.push(
                row.into_iter()
                    .map(|cell| {
                        self.expression(&cell)
                            .map(|value| (value.0, cell.syntax().clone()))
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            );
        }
        if widths.iter().any(|width| *width != headers.len()) {
            return Err(SourceSemanticError {
                code: "source-semantics/table-row-width",
                message: "table row width does not match the declared header".to_owned(),
                anchor: SourceSemanticAnchor::for_node(&syntax),
            });
        }
        for index in 0..headers.len() {
            if headers[index].1 == BuiltinSchema::Dynamic {
                headers[index].1 = compiled_rows
                    .iter()
                    .map(|row| self.schema_of(row[index].0))
                    .find(|schema| *schema != BuiltinSchema::Dynamic)
                    .ok_or_else(|| SourceSemanticError {
                        code: "source-semantics/unresolved-table-column-kind",
                        message: format!(
                            "table field {} has no value from which to infer its kind",
                            headers[index].0
                        ),
                        anchor: SourceSemanticAnchor::for_node(&syntax),
                    })?;
            }
            let (name, expected) = headers[index].clone();
            for row in &mut compiled_rows {
                row[index].0 =
                    self.conform_table_value(row[index].0, expected, &name, &row[index].1)?;
            }
        }
        let inputs = compiled_rows
            .into_iter()
            .flatten()
            .map(|(value, _)| value)
            .collect::<Vec<_>>();
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
            let mut qualifier_layouts = Vec::new();
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
                        let pattern = self.record_pattern(&pattern)?;
                        self.bind_pattern(&pattern, source)?;
                        qualifier_layouts.push((
                            inputs.len() as u32,
                            SourceSemanticComprehensionQualifierRole::Generator {
                                pattern: pattern.index,
                            },
                        ));
                        inputs.push(source);
                    }
                    ComprehensionQualifierValueSyntax::Definition(definition) => {
                        qualifier_layouts.push((
                            inputs.len() as u32,
                            SourceSemanticComprehensionQualifierRole::Definition,
                        ));
                        inputs.push(self.definition(&definition)?.0);
                    }
                    ComprehensionQualifierValueSyntax::Filter(filter) => {
                        qualifier_layouts.push((
                            inputs.len() as u32,
                            SourceSemanticComprehensionQualifierRole::Filter,
                        ));
                        inputs.push(self.expression(&filter)?.0);
                    }
                }
            }
            let result = self.required(result, syntax, "a comprehension result")?;
            inputs.push(self.expression(&result)?.0);
            let value = self.emit(
                operation,
                inputs,
                BuiltinSchema::Dynamic,
                syntax,
                "comprehension",
                None,
            );
            let PendingValue::Node(node) = value else {
                unreachable!("emit always returns a node")
            };
            self.comprehension_qualifiers
                .extend(qualifier_layouts.into_iter().map(|(input_ordinal, role)| {
                    SourceSemanticComprehensionQualifier {
                        node,
                        input_ordinal,
                        role,
                    }
                }));
            Ok(value)
        })();
        self.bindings = saved;
        compiled
    }

    fn record_pattern(
        &mut self,
        pattern: &PatternSyntax,
    ) -> Result<RecordedPattern, SourceSemanticError> {
        self.required(pattern.value(), pattern.syntax(), "a pattern body")?;
        let mut bindings = Vec::new();
        collect_pattern_bindings(pattern.syntax(), &mut bindings)?;
        let mut seen = BTreeSet::new();
        bindings.retain(|name| seen.insert(name.clone()));
        let index = self.patterns.len() as u32;
        self.patterns.push(SourceSemanticPattern {
            source: node_text(pattern.syntax())?,
            bindings: bindings.clone().into_boxed_slice(),
            anchor: SourceSemanticAnchor::for_node(pattern.syntax()),
        });
        Ok(RecordedPattern {
            index,
            bindings,
            syntax: pattern.syntax().clone(),
        })
    }

    fn bind_pattern(
        &mut self,
        pattern: &RecordedPattern,
        source: PendingValue,
    ) -> Result<(), SourceSemanticError> {
        let pattern_index = pattern.index;
        for (binding_index, name) in pattern.bindings.iter().enumerate() {
            let binding_index = u32::try_from(binding_index).map_err(|_| SourceSemanticError {
                code: "source-semantics/pattern-binding-identity-exhausted",
                message: "pattern binding count exceeds semantic identity space".to_owned(),
                anchor: self.anchor,
            })?;
            let projection = self.emit(
                "source/bind",
                vec![source],
                BuiltinSchema::Dynamic,
                &pattern.syntax,
                "pattern-binding",
                Some(format!(
                    "pattern={pattern_index};binding={binding_index};name={name}"
                )),
            );
            self.bindings.insert(name.clone(), projection);
        }
        Ok(())
    }

    fn fsm_pipe(&mut self, pipe: &FsmPipeSyntax) -> Result<PendingValue, SourceSemanticError> {
        let instance = self.required(pipe.instance(), pipe.syntax(), "an FSM instance")?;
        let name = self.required(instance.name(), instance.syntax(), "an FSM name")?;
        let mut inputs = Vec::new();
        let mut argument_names = Vec::new();
        if let Some(arguments) = instance.arguments() {
            for argument in arguments.arguments() {
                let value = match argument {
                    AnyCallArgumentSyntax::Positional(argument) => {
                        argument_names.push(String::new());
                        self.required(argument.value(), argument.syntax(), "an FSM argument value")?
                    }
                    AnyCallArgumentSyntax::Bound(argument) => {
                        let name = self.required(
                            argument.name(),
                            argument.syntax(),
                            "an FSM argument name",
                        )?;
                        argument_names.push(node_text(name.syntax())?);
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
            Some(format!(
                "{}({})",
                node_text(name.syntax())?,
                argument_names.join(",")
            )),
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

    fn promote_operands(
        &mut self,
        lhs: PendingValue,
        rhs: PendingValue,
        syntax: &SyntaxNode,
    ) -> Result<(PendingValue, PendingValue, Option<BuiltinSchema>), SourceSemanticError> {
        let lhs_schema = self.schema_of(lhs);
        let rhs_schema = self.schema_of(rhs);
        if lhs_schema == BuiltinSchema::Dynamic || rhs_schema == BuiltinSchema::Dynamic {
            return Ok((lhs, rhs, None));
        }
        let (Some(lhs_kind), Some(rhs_kind)) = (builtin_kind(lhs_schema), builtin_kind(rhs_schema))
        else {
            return Ok((lhs, rhs, None));
        };
        let lhs_type = resolved_builtin_type(lhs_kind, syntax)?;
        let rhs_type = resolved_builtin_type(rhs_kind, syntax)?;
        let Some(plan) = plan_numeric_promotion(&lhs_type, &rhs_type)
            .map_err(|error| internal(SourceSemanticAnchor::for_node(syntax), error.to_string()))?
        else {
            if is_numeric(lhs_kind) && is_numeric(rhs_kind) && lhs_kind != rhs_kind {
                return Err(SourceSemanticError {
                    code: "source-semantics/numeric-promotion-failed",
                    message: format!(
                        "{} and {} have no lossless numeric promotion",
                        lhs_kind.canonical_name(),
                        rhs_kind.canonical_name()
                    ),
                    anchor: SourceSemanticAnchor::for_node(syntax),
                });
            }
            return Ok((lhs, rhs, None));
        };
        let result_kind = builtin_kind_from_resolved(&plan.result).ok_or_else(|| {
            internal(
                SourceSemanticAnchor::for_node(syntax),
                "numeric promotion returned a non-scalar result".to_owned(),
            )
        })?;
        let result_schema = builtin_schema(result_kind).ok_or_else(|| {
            internal(
                SourceSemanticAnchor::for_node(syntax),
                "numeric promotion returned an unsupported scalar result".to_owned(),
            )
        })?;
        let lhs = self.apply_conversion(lhs, result_schema, &plan.left, syntax)?;
        let rhs = self.apply_conversion(rhs, result_schema, &plan.right, syntax)?;
        Ok((lhs, rhs, Some(result_schema)))
    }

    fn apply_conversion(
        &mut self,
        value: PendingValue,
        target: BuiltinSchema,
        plan: &mech_core::ConversionPlan,
        syntax: &SyntaxNode,
    ) -> Result<PendingValue, SourceSemanticError> {
        if self.schema_of(value) == target {
            return Ok(value);
        }
        if let PendingValue::Constant(index) = value {
            let data = execute_conversion_draft(self.constants[index].data.clone(), &plan.step)
                .map_err(|error| SourceSemanticError {
                    code: "source-semantics/constant-conversion-failed",
                    message: error.to_string(),
                    anchor: SourceSemanticAnchor::for_node(syntax),
                })?;
            return Ok(self.constant(target, data));
        }
        Ok(self.emit(
            "convert/kind",
            vec![value],
            target,
            syntax,
            "implicit-conversion",
            Some(format!(
                "target={}",
                builtin_kind(target).unwrap().canonical_name()
            )),
        ))
    }

    fn conform_value(
        &mut self,
        value: PendingValue,
        expected: BuiltinSchema,
        syntax: &SyntaxNode,
        code: &'static str,
        message: &str,
    ) -> Result<PendingValue, SourceSemanticError> {
        let actual = self.schema_of(value);
        if expected == BuiltinSchema::Dynamic || actual == expected {
            return Ok(value);
        }
        if let Some(payload) = option_payload_schema(expected) {
            let value = self.conform_value(value, payload, syntax, code, message)?;
            let PendingValue::Constant(index) = value else {
                return Err(SourceSemanticError {
                    code,
                    message: message.to_owned(),
                    anchor: SourceSemanticAnchor::for_node(syntax),
                });
            };
            let data = self.constants[index].data.clone();
            return Ok(self.constant(
                expected,
                ValueDataDraft::Option(OptionDraft {
                    present: true,
                    value: Some(Box::new(data)),
                }),
            ));
        }
        if actual == BuiltinSchema::Dynamic {
            return Ok(self.emit(
                "convert/kind",
                vec![value],
                expected,
                syntax,
                "declared-conversion",
                Some(format!(
                    "target={}",
                    builtin_kind(expected)
                        .map(BuiltinScalarKind::canonical_name)
                        .unwrap_or("dynamic")
                )),
            ));
        }
        let (Some(actual_kind), Some(expected_kind)) =
            (builtin_kind(actual), builtin_kind(expected))
        else {
            return Err(SourceSemanticError {
                code,
                message: message.to_owned(),
                anchor: SourceSemanticAnchor::for_node(syntax),
            });
        };
        let actual_type = resolved_builtin_type(actual_kind, syntax)?;
        let expected_type = resolved_builtin_type(expected_kind, syntax)?;
        let plan =
            plan_explicit_cast(&actual_type, &expected_type).map_err(|_| SourceSemanticError {
                code,
                message: message.to_owned(),
                anchor: SourceSemanticAnchor::for_node(syntax),
            })?;
        self.apply_conversion(value, expected, &plan, syntax)
            .map_err(|_| SourceSemanticError {
                code,
                message: message.to_owned(),
                anchor: SourceSemanticAnchor::for_node(syntax),
            })
    }

    fn conform_table_value(
        &mut self,
        value: PendingValue,
        expected: BuiltinSchema,
        field: &str,
        syntax: &SyntaxNode,
    ) -> Result<PendingValue, SourceSemanticError> {
        self.conform_value(
            value,
            expected,
            syntax,
            "source-semantics/incompatible-table-field-kind",
            &format!("table field {field} does not satisfy its kind annotation"),
        )
    }

    fn constant(&mut self, schema: BuiltinSchema, data: ValueDataDraft) -> PendingValue {
        let index = self.constants.len();
        self.constants.push(PendingConstant {
            schema,
            schema_body: None,
            schema_parameters: Box::new([]),
            data,
        });
        PendingValue::Constant(index)
    }

    fn constant_exact(&mut self, schema_body: SchemaBody, data: ValueDataDraft) -> PendingValue {
        let index = self.constants.len();
        self.constants.push(PendingConstant {
            schema: BuiltinSchema::Dynamic,
            schema_body: Some(schema_body),
            schema_parameters: Box::new([]),
            data,
        });
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
            schema_parameters: Box::new([]),
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
        self.emit_with_schema_draft(
            operation,
            inputs,
            SchemaDraft {
                dimension_parameters: Box::new([]),
                body: schema_body,
            },
            syntax,
            role,
            detail,
        )
    }

    fn emit_with_schema_draft(
        &mut self,
        operation: &str,
        inputs: Vec<PendingValue>,
        schema: SchemaDraft,
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
        self.nodes[index as usize].schema_body = Some(schema.body);
        self.nodes[index as usize].schema_parameters = schema.dimension_parameters;
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
        let schemas = BuiltinSchemas::build(self.anchor, &self.nodes, &self.constants)?;
        let constant_schema_ids = self
            .constants
            .iter()
            .enumerate()
            .map(|(index, constant)| schemas.constant_id(index, constant.schema))
            .collect::<Vec<_>>();
        let mut constants = ConstantStoreBuilder::new(&schemas.table);
        let mut handles = Vec::with_capacity(self.constants.len());
        for (index, constant) in self.constants.into_iter().enumerate() {
            let schema = constant_schema_ids[index];
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
                    &constant_schema_ids,
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
            match_arms: self.match_arms.into_boxed_slice(),
            comprehension_qualifiers: self.comprehension_qualifiers.into_boxed_slice(),
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
    constants: &[SchemaId],
    inputs: &[(String, BuiltinSchema, SourceSemanticAnchor)],
    nodes: &[PendingNode],
    schemas: &BuiltinSchemas,
) -> SchemaId {
    match value {
        PendingValue::Constant(index) => constants
            .get(index)
            .copied()
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
    match name.as_str() {
        "range/inclusive" => Some(range_contract(input_count, "inclusive-output")),
        "range/exclusive" => Some(range_contract(input_count, "exclusive-output")),
        "range/inclusive-increment" => {
            Some(range_contract(input_count, "inclusive-increment-output"))
        }
        "range/exclusive-increment" => {
            Some(range_contract(input_count, "exclusive-increment-output"))
        }
        "convert/kind" => Some(conversion_contract()),
        "math/neg" => Some(negation_contract(output_schema)),
        "math/add" | "math/sub" | "math/mul" | "math/div" | "math/mod" | "math/pow"
        | "compare/neq" | "compare/eq" | "compare/sneq" | "compare/seq" | "compare/gt"
        | "compare/lt" | "compare/gte" | "compare/lte" | "logic/or" | "logic/and" | "logic/not"
        | "logic/xor" | "matrix/transpose" | "core/assign" => {
            Some(operation_contract(input_count, output_schema, state_output))
        }
        _ => None,
    }
}

fn range_contract(input_count: usize, contract_name: &str) -> OperationContractDeclaration {
    OperationContractDeclaration {
        inputs: read_inputs(input_count),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::Build {
                postcondition: ShapeContractReference {
                    module_path: vec!["range".to_owned()].into_boxed_slice(),
                    contract_name: contract_name.to_owned(),
                },
            },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    }
}

fn conversion_contract() -> OperationContractDeclaration {
    OperationContractDeclaration {
        inputs: read_inputs(1),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::SameAsInput { input: 0 },
            },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    }
}

fn negation_contract(output_schema: BuiltinSchema) -> OperationContractDeclaration {
    OperationContractDeclaration {
        inputs: read_inputs(1),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::SameAsInput { input: 0 },
            },
            alias: AliasPolicy::NoAlias,
            change_detection: if is_scalar_schema(output_schema) {
                ChangeDetectionPolicy::ExactScalar
            } else {
                ChangeDetectionPolicy::KernelReported
            },
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    }
}

fn is_scalar_schema(schema: BuiltinSchema) -> bool {
    matches!(
        schema,
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
    )
}

fn read_inputs(input_count: usize) -> InputPortLayout {
    InputPortLayout::Fixed(
        (0..input_count)
            .map(|_| InputPortPolicy {
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            })
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    )
}

fn operation_contract(
    input_count: usize,
    output_schema: BuiltinSchema,
    state_output: bool,
) -> OperationContractDeclaration {
    OperationContractDeclaration {
        inputs: read_inputs(input_count),
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
            } else if is_scalar_schema(output_schema) {
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
        .unwrap_or(source.as_str());
    let (name, optional) = name
        .strip_suffix('?')
        .map_or((name, false), |name| (name, true));
    let mut schema = match name {
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
    if optional {
        schema = option_schema(schema).ok_or_else(|| SourceSemanticError {
            code: "source-semantics/unsupported-option-kind",
            message: "optional annotations require a concrete scalar payload kind".to_owned(),
            anchor: SourceSemanticAnchor::for_node(annotation.syntax()),
        })?;
    }
    Ok(schema)
}

fn require_literal_schema(
    annotation: Option<BuiltinSchema>,
    actual: BuiltinSchema,
    syntax: &SyntaxNode,
) -> Result<(), SourceSemanticError> {
    if annotation.is_none_or(|schema| {
        schema == actual
            || schema == BuiltinSchema::Dynamic
            || option_payload_schema(schema) == Some(actual)
    }) {
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
    } else if let Some(value) = magnitude.strip_prefix("0d") {
        (10, value)
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
        BuiltinSchema::F32 => {
            let value = float()?;
            let narrowed = value as f32;
            if value.is_finite() && !narrowed.is_finite() {
                return None;
            }
            ValueDataDraft::F32(F32Bits::from_f32(narrowed))
        }
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
    let option = annotation.filter(|schema| option_payload_schema(*schema).is_some());
    let annotation = annotation.map(|schema| option_payload_schema(schema).unwrap_or(schema));
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
        return wrap_optional_number(
            option,
            schema,
            ValueDataDraft::Complex64(Complex64Bits::new(
                F64Bits::from_f64(real_value(real)?),
                F64Bits::from_f64(real_value(imaginary)?),
            )),
        );
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
        return wrap_optional_number(
            option,
            schema,
            ValueDataDraft::Rational64 {
                numerator: i64::try_from(integer_value(numerator)?).ok()?,
                denominator,
            },
        );
    }
    let (number, suffix) = numeric_suffix(&source);
    let annotated = annotation.filter(|schema| *schema != BuiltinSchema::Dynamic);
    let magnitude = number.strip_prefix('-').unwrap_or(number);
    let explicitly_based = ["0d", "0x", "0o", "0b"]
        .into_iter()
        .any(|prefix| magnitude.starts_with(prefix));
    let schema = if number.contains(['e', 'E']) {
        annotated.unwrap_or(BuiltinSchema::F64)
    } else {
        annotated.or(suffix).unwrap_or(if explicitly_based {
            BuiltinSchema::I64
        } else {
            BuiltinSchema::F64
        })
    };
    wrap_optional_number(option, schema, scalar_data(schema, number)?)
}

fn wrap_optional_number(
    option: Option<BuiltinSchema>,
    schema: BuiltinSchema,
    data: ValueDataDraft,
) -> Option<(BuiltinSchema, ValueDataDraft)> {
    let Some(option) = option else {
        return Some((schema, data));
    };
    (option_payload_schema(option) == Some(schema)).then(|| {
        (
            option,
            ValueDataDraft::Option(OptionDraft {
                present: true,
                value: Some(Box::new(data)),
            }),
        )
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_conformance_allocates_a_distinct_constant() {
        let anchor = SourceSemanticAnchor {
            document: DocumentId(1),
            revision: Revision(1),
            range: TextRange::empty(mech_syntax::document::TextSize::ZERO),
        };
        let mut builder = SemanticBuilder::new(anchor);
        let original = builder.constant(
            BuiltinSchema::F64,
            ValueDataDraft::F64(F64Bits::from_f64(1.0)),
        );
        let converted = builder
            .conform_table_value(
                original,
                BuiltinSchema::U8,
                "small",
                &SyntaxNode::new_root(
                    std::sync::Arc::new(mech_syntax::document::GreenNode {
                        id: mech_syntax::document::NodeId(1),
                        kind: SyntaxKind::Literal,
                        text_len: mech_syntax::document::TextSize::ZERO,
                        children: std::sync::Arc::from([]),
                        flags: NodeFlags::NONE,
                        structural_hash: 0,
                    }),
                    mech_syntax::document::TextSnapshot::new(DocumentId(1), Revision(1), "")
                        .unwrap(),
                ),
            )
            .unwrap();
        assert!(matches!(original, PendingValue::Constant(0)));
        assert!(matches!(converted, PendingValue::Constant(1)));
        assert_eq!(builder.constants[0].schema, BuiltinSchema::F64);
        assert!(matches!(builder.constants[0].data, ValueDataDraft::F64(_)));
        assert_eq!(builder.constants[1].schema, BuiltinSchema::U8);
        assert!(matches!(builder.constants[1].data, ValueDataDraft::U8(1)));
    }
}
