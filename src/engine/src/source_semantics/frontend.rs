use std::collections::{BTreeMap, BTreeSet};

use mech_core::snapshot::{
    Complex32Bits, Complex64Bits, F32Bits, F64Bits, OptionDraft, ReifiedKind, ReifiedTypeDraft,
    SnapshotValidationContext,
};
use mech_core::{
    AccessMode, AliasPolicy, BuiltinKindPredicate, BuiltinScalarKind, CanonicalNominalPath,
    CardinalitySpec, ChangeDetectionPolicy, ConstantStore, ConstantStoreBuilder, DeliveryMode,
    DimensionEnvironmentBuilder, DimensionExpr, DimensionLifetime, DimensionParameterDeclaration,
    DimensionParameterId, DimensionParameterOrigin, ExternalInteraction, FloatWidth,
    InputKindScheme, InputPortLayout, InputPortPolicy, IntegerWidth, KindExpr, KindField, KindId,
    NamedKindPathResolver, NodeId, NominalKey, NominalKind, OperationContractDeclaration,
    OutputConstruction, OutputPortPolicy, ResolvedOutputSchemaRule, ResolvedType, SchemaBody,
    SchemaDraft, SchemaField, SchemaId, SchemaTable, SchemaTableBuilder, ShapeContractReference,
    ShapeRule, SourceInputKind, TypeConstraintOrigin, TypeOverloadCandidate, ValueDataDraft,
    ValueDraft, execute_conversion_draft, plan_explicit_cast, plan_numeric_promotion,
};
use mech_syntax::document::{
    AnyCallArgumentSyntax, AstNode, CanonicalOperator, ComprehensionQualifierValueSyntax,
    DocumentId, DocumentSyntax, ExpressionBodySyntax, ExpressionSyntax, FactorSyntax,
    FactorValueSyntax, FormulaSyntax, FsmPipeSyntax, FsmStageSyntax, IntegerLiteralSyntax,
    KindAnnotationSyntax, KindSyntax, KindValueSyntax, LiteralSyntax, LiteralValueSyntax,
    MapSyntax, MatrixComprehensionSyntax, MatrixSyntax, MultiplicativeExpressionSyntax, NodeFlags,
    OperatorSyntax, PatternSyntax, PatternValueSyntax, RangeExpressionSyntax, RecordSyntax,
    RecursiveSyntaxNode, Revision, ScientificLiteralSyntax, SetComprehensionSyntax, SetSyntax,
    SliceStemSyntax, SliceSyntax, StructureSyntax, StructureValueSyntax, SubscriptItemSyntax,
    SubscriptValueSyntax, SyntaxKind, SyntaxNode, TableSyntax, TableValueSyntax, TextRange,
    TupleStructSyntax, TupleSyntax, VariableDefineSyntax, VariableStemSyntax, VariableSyntax,
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
    state_initializers: Box<[SourceStateInitializer]>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceStateInitializer {
    Constant(mech_core::ConstantId),
    Deferred(SourceValue),
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

    pub const fn state_initializers(&self) -> &[SourceStateInitializer] {
        &self.state_initializers
    }

    /// Artifact transport identity for the source input at the same ordinal.
    /// Source names and anchors remain available through `program` and `source_map`.
    pub fn artifact_input_name(&self, ordinal: usize) -> Option<String> {
        self.program
            .inputs
            .get(ordinal)
            .map(|input| crate::encode_source_input_name(&input.name))
    }

    pub fn compile_artifact(&self) -> Result<ProgramArtifact, ArtifactBuildError> {
        if let Some((state, _)) = self
            .state_initializers
            .iter()
            .enumerate()
            .find(|(_, initializer)| matches!(initializer, SourceStateInitializer::Deferred(_)))
        {
            return Err(ArtifactBuildError::DeclaredSourceNodeLoweringUnsupported {
                source_node: self.program.states[state].producer_node,
            });
        }
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
        let mut artifact_program = self.program.clone();
        for input in &mut artifact_program.inputs {
            input.name = crate::encode_source_input_name(&input.name);
        }
        compile_source_program_with_contracts(
            &artifact_program,
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
        builder.declare_input_annotations(expression.syntax(), &BTreeSet::new())?;
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
        builder.declare_definition_input_annotations(definition, &BTreeSet::new())?;
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
        let mut declared_bindings = BTreeSet::new();
        for unit in &units {
            builder.declare_unit_input_annotations(unit, &mut declared_bindings)?;
        }
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
    match node.kind() {
        SyntaxKind::VariableDefine | SyntaxKind::Expression => output.push(node.clone()),
        SyntaxKind::Document
        | SyntaxKind::Body
        | SyntaxKind::Section
        | SyntaxKind::SectionElement
        | SyntaxKind::MechItem => {
            for child in node.children() {
                collect_document_units(&child, output);
            }
        }
        _ => {}
    }
}

fn collect_pattern_bindings(
    pattern: &PatternSyntax,
    output: &mut Vec<PatternBinding>,
) -> Result<(), SourceSemanticError> {
    collect_projected_pattern_bindings(pattern, &mut Vec::new(), output)
}

fn collect_projected_pattern_bindings(
    pattern: &PatternSyntax,
    path: &mut Vec<usize>,
    output: &mut Vec<PatternBinding>,
) -> Result<(), SourceSemanticError> {
    let value = pattern
        .value()
        .ok_or_else(|| missing_kind_child(pattern.syntax(), "pattern body"))?;
    let children = match value {
        PatternValueSyntax::Expression(expression) => {
            if let Some(variable) = standalone_pattern_variable(&expression)
                && let Some(VariableStemSyntax::Identifier(identifier)) = variable.stem()
            {
                output.push(PatternBinding {
                    name: node_text(identifier.syntax())?,
                    schema: variable
                        .annotation()
                        .map(|annotation| annotation_schema_draft(&annotation))
                        .transpose()?,
                    path: path.clone(),
                });
            }
            Vec::new()
        }
        PatternValueSyntax::Array(array) => array
            .elements()
            .iter()
            .map(|element| element.pattern())
            .collect(),
        PatternValueSyntax::Tuple(tuple) => tuple.items().into_iter().map(Some).collect(),
        PatternValueSyntax::AtomStruct(tuple) => tuple.items().into_iter().map(Some).collect(),
        PatternValueSyntax::TupleStruct(tuple) => tuple.items().into_iter().map(Some).collect(),
        PatternValueSyntax::Wildcard(_) => Vec::new(),
    };
    for (index, child) in children.into_iter().enumerate() {
        if let Some(child) = child {
            path.push(index);
            collect_projected_pattern_bindings(&child, path, output)?;
            path.pop();
        }
    }
    Ok(())
}

fn standalone_pattern_variable(expression: &ExpressionSyntax) -> Option<VariableSyntax> {
    fn find(node: &SyntaxNode, range: TextRange) -> Option<VariableSyntax> {
        if node.kind() == SyntaxKind::Variable && node.range() == range {
            return VariableSyntax::cast(node.clone());
        }
        node.children().find_map(|child| find(&child, range))
    }

    find(expression.syntax(), expression.syntax().range())
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
    Index,
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
    C32,
    C64,
    R64,
    OptionDynamic,
    OptionBool,
    OptionString,
    OptionIndex,
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
    OptionC32,
    OptionC64,
    OptionR64,
}

struct BuiltinSchemas {
    table: SchemaTable,
    ids: BTreeMap<BuiltinSchema, SchemaId>,
    input_ids: BTreeMap<usize, SchemaId>,
    node_ids: BTreeMap<usize, SchemaId>,
    constant_ids: BTreeMap<usize, SchemaId>,
    dynamic_payload_ids: BTreeMap<usize, SchemaId>,
}

impl BuiltinSchemas {
    fn build(
        anchor: SourceSemanticAnchor,
        inputs: &[PendingInput],
        nodes: &[PendingNode],
        constants: &[PendingConstant],
    ) -> Result<Self, SourceSemanticError> {
        let mut builder = SchemaTableBuilder::new();
        let mut handles = Vec::new();
        for builtin in [
            BuiltinSchema::Dynamic,
            BuiltinSchema::Bool,
            BuiltinSchema::String,
            BuiltinSchema::Index,
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
            BuiltinSchema::C32,
            BuiltinSchema::C64,
            BuiltinSchema::R64,
            BuiltinSchema::OptionDynamic,
            BuiltinSchema::OptionBool,
            BuiltinSchema::OptionString,
            BuiltinSchema::OptionIndex,
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
            BuiltinSchema::OptionC32,
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
        let input_handles = inputs
            .iter()
            .enumerate()
            .map(|(index, input)| {
                let schema = input.schema.clone().finalize().map_err(|error| {
                    internal(anchor, format!("invalid input schema: {error:?}"))
                })?;
                builder
                    .insert(schema)
                    .map(|handle| (index, handle))
                    .map_err(|error| {
                        internal(anchor, format!("unable to retain input schema: {error:?}"))
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
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
        let dynamic_payload_handles = constants
            .iter()
            .enumerate()
            .filter_map(|(index, constant)| {
                constant
                    .dynamic_payload
                    .as_ref()
                    .and_then(|(_, body, _)| body.as_ref())
                    .map(|body| (index, body))
            })
            .map(|(index, body)| {
                let schema = SchemaDraft {
                    dimension_parameters: Box::new([]),
                    body: body.clone(),
                }
                .finalize()
                .map_err(|error| {
                    internal(anchor, format!("invalid dynamic payload schema: {error:?}"))
                })?;
                builder
                    .insert(schema)
                    .map(|handle| (index, handle))
                    .map_err(|error| {
                        internal(
                            anchor,
                            format!("unable to retain dynamic payload schema: {error:?}"),
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
        let input_ids = input_handles
            .into_iter()
            .map(|(index, handle)| {
                build
                    .resolve(handle)
                    .map(|id| (index, id))
                    .map_err(|error| {
                        internal(anchor, format!("unable to resolve input schema: {error:?}"))
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
        let dynamic_payload_ids = dynamic_payload_handles
            .into_iter()
            .map(|(index, handle)| {
                build
                    .resolve(handle)
                    .map(|id| (index, id))
                    .map_err(|error| {
                        internal(
                            anchor,
                            format!("unable to resolve dynamic payload schema: {error:?}"),
                        )
                    })
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let (table, _) = build.into_parts();
        Ok(Self {
            table,
            ids,
            input_ids,
            node_ids,
            constant_ids,
            dynamic_payload_ids,
        })
    }

    fn id(&self, schema: BuiltinSchema) -> SchemaId {
        self.ids[&schema]
    }

    fn input_id(&self, index: usize) -> SchemaId {
        self.input_ids[&index]
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

    fn dynamic_payload_id(&self, index: usize, fallback: BuiltinSchema) -> SchemaId {
        self.dynamic_payload_ids
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
        BuiltinSchema::Index => SchemaBody::Index,
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
        BuiltinSchema::C32 => SchemaBody::Complex(FloatWidth::W32),
        BuiltinSchema::C64 => SchemaBody::Complex(FloatWidth::W64),
        BuiltinSchema::R64 => SchemaBody::Rational64,
        option => SchemaBody::Option(Box::new(schema_body(
            option_payload_schema(option).expect("closed optional builtin schema"),
        ))),
    }
}

fn dynamic_schema_draft() -> SchemaDraft {
    SchemaDraft {
        dimension_parameters: Box::new([]),
        body: SchemaBody::Dynamic,
    }
}

fn is_dynamic_schema_draft(schema: &SchemaDraft) -> bool {
    schema.dimension_parameters.is_empty() && matches!(schema.body, SchemaBody::Dynamic)
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
        BuiltinSchema::C32 => BuiltinScalarKind::C32,
        BuiltinSchema::C64 => BuiltinScalarKind::C64,
        BuiltinSchema::R64 => BuiltinScalarKind::R64,
        BuiltinSchema::Dynamic
        | BuiltinSchema::Index
        | BuiltinSchema::OptionDynamic
        | BuiltinSchema::OptionBool
        | BuiltinSchema::OptionString
        | BuiltinSchema::OptionIndex
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
        | BuiltinSchema::OptionC32
        | BuiltinSchema::OptionC64
        | BuiltinSchema::OptionR64 => return None,
    })
}

fn option_schema(payload: BuiltinSchema) -> Option<BuiltinSchema> {
    Some(match payload {
        BuiltinSchema::Dynamic => BuiltinSchema::OptionDynamic,
        BuiltinSchema::Bool => BuiltinSchema::OptionBool,
        BuiltinSchema::String => BuiltinSchema::OptionString,
        BuiltinSchema::Index => BuiltinSchema::OptionIndex,
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
        BuiltinSchema::C32 => BuiltinSchema::OptionC32,
        BuiltinSchema::C64 => BuiltinSchema::OptionC64,
        BuiltinSchema::R64 => BuiltinSchema::OptionR64,
        _ => return None,
    })
}

fn option_payload_schema(option: BuiltinSchema) -> Option<BuiltinSchema> {
    Some(match option {
        BuiltinSchema::OptionDynamic => BuiltinSchema::Dynamic,
        BuiltinSchema::OptionBool => BuiltinSchema::Bool,
        BuiltinSchema::OptionString => BuiltinSchema::String,
        BuiltinSchema::OptionIndex => BuiltinSchema::Index,
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
        BuiltinSchema::OptionC32 => BuiltinSchema::C32,
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
        BuiltinScalarKind::C32 => BuiltinSchema::C32,
        BuiltinScalarKind::C64 => BuiltinSchema::C64,
        BuiltinScalarKind::R64 => BuiltinSchema::R64,
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

fn resolved_schema_type(
    schema: BuiltinSchema,
    syntax: &SyntaxNode,
) -> Result<Option<ResolvedType>, SourceSemanticError> {
    let kind = match schema {
        BuiltinSchema::Index => KindExpr::Index,
        _ => {
            let Some(kind) = builtin_kind(schema) else {
                return Ok(None);
            };
            kind.kind_expr()
        }
    };
    ResolvedType::new(kind, Box::new([]))
        .map(Some)
        .map_err(|error| {
            internal(
                SourceSemanticAnchor::for_node(syntax),
                format!("invalid builtin source type: {error}"),
            )
        })
}

fn is_numeric(kind: BuiltinScalarKind) -> bool {
    !matches!(kind, BuiltinScalarKind::Bool | BuiltinScalarKind::String)
}

fn builtin_schema_for_body(body: &SchemaBody) -> Option<BuiltinSchema> {
    Some(match body {
        SchemaBody::Bool => BuiltinSchema::Bool,
        SchemaBody::String => BuiltinSchema::String,
        SchemaBody::Index => BuiltinSchema::Index,
        SchemaBody::UnsignedInteger(IntegerWidth::W8) => BuiltinSchema::U8,
        SchemaBody::UnsignedInteger(IntegerWidth::W16) => BuiltinSchema::U16,
        SchemaBody::UnsignedInteger(IntegerWidth::W32) => BuiltinSchema::U32,
        SchemaBody::UnsignedInteger(IntegerWidth::W64) => BuiltinSchema::U64,
        SchemaBody::UnsignedInteger(IntegerWidth::W128) => BuiltinSchema::U128,
        SchemaBody::SignedInteger(IntegerWidth::W8) => BuiltinSchema::I8,
        SchemaBody::SignedInteger(IntegerWidth::W16) => BuiltinSchema::I16,
        SchemaBody::SignedInteger(IntegerWidth::W32) => BuiltinSchema::I32,
        SchemaBody::SignedInteger(IntegerWidth::W64) => BuiltinSchema::I64,
        SchemaBody::SignedInteger(IntegerWidth::W128) => BuiltinSchema::I128,
        SchemaBody::FloatingPoint(FloatWidth::W32) => BuiltinSchema::F32,
        SchemaBody::FloatingPoint(FloatWidth::W64) => BuiltinSchema::F64,
        SchemaBody::Complex(FloatWidth::W32) => BuiltinSchema::C32,
        SchemaBody::Complex(FloatWidth::W64) => BuiltinSchema::C64,
        SchemaBody::Rational64 => BuiltinSchema::R64,
        _ => return None,
    })
}

fn embed_schema_draft(
    draft: &SchemaDraft,
    output: &mut Vec<DimensionParameterDeclaration>,
    anchor: SourceSemanticAnchor,
) -> Result<SchemaBody, SourceSemanticError> {
    let start = u32::try_from(output.len()).map_err(|_| {
        internal(
            anchor,
            "table schema dimension identity space was exhausted".to_owned(),
        )
    })?;
    let mut replacements = BTreeMap::new();
    for (ordinal, declaration) in draft.dimension_parameters.iter().enumerate() {
        let ordinal = u32::try_from(ordinal).map_err(|_| {
            internal(
                anchor,
                "table schema dimension identity space was exhausted".to_owned(),
            )
        })?;
        let Some(id) = start.checked_add(ordinal).map(DimensionParameterId::new) else {
            return Err(internal(
                anchor,
                "table schema dimension identity space was exhausted".to_owned(),
            ));
        };
        replacements.insert(declaration.id, id);
    }
    for declaration in draft.dimension_parameters.iter() {
        output.push(DimensionParameterDeclaration {
            id: replacements[&declaration.id],
            origin: declaration.origin,
            lifetime: declaration.lifetime,
            lower_bound: remap_dimension_expr(&declaration.lower_bound, &replacements).map_err(
                |_| {
                    internal(
                        anchor,
                        "table schema references an undeclared dimension".to_owned(),
                    )
                },
            )?,
            upper_bound: declaration
                .upper_bound
                .as_ref()
                .map(|bound| remap_dimension_expr(bound, &replacements))
                .transpose()
                .map_err(|_| {
                    internal(
                        anchor,
                        "table schema references an undeclared dimension".to_owned(),
                    )
                })?,
        });
    }
    remap_schema_body(&draft.body, &replacements).map_err(|_| {
        internal(
            anchor,
            "table schema references an undeclared dimension".to_owned(),
        )
    })
}

fn remap_dimension_expr(
    expression: &DimensionExpr,
    replacements: &BTreeMap<DimensionParameterId, DimensionParameterId>,
) -> Result<DimensionExpr, ()> {
    Ok(match expression {
        DimensionExpr::Hole => DimensionExpr::Hole,
        DimensionExpr::Constant(value) => DimensionExpr::Constant(*value),
        DimensionExpr::Parameter(id) => DimensionExpr::Parameter(*replacements.get(id).ok_or(())?),
        DimensionExpr::Add(children) => DimensionExpr::Add(
            children
                .iter()
                .map(|child| remap_dimension_expr(child, replacements))
                .collect::<Result<Vec<_>, _>>()?
                .into_boxed_slice(),
        ),
        DimensionExpr::Multiply(children) => DimensionExpr::Multiply(
            children
                .iter()
                .map(|child| remap_dimension_expr(child, replacements))
                .collect::<Result<Vec<_>, _>>()?
                .into_boxed_slice(),
        ),
        DimensionExpr::Min(children) => DimensionExpr::Min(
            children
                .iter()
                .map(|child| remap_dimension_expr(child, replacements))
                .collect::<Result<Vec<_>, _>>()?
                .into_boxed_slice(),
        ),
        DimensionExpr::Max(children) => DimensionExpr::Max(
            children
                .iter()
                .map(|child| remap_dimension_expr(child, replacements))
                .collect::<Result<Vec<_>, _>>()?
                .into_boxed_slice(),
        ),
    })
}

fn remap_cardinality(
    cardinality: &CardinalitySpec,
    replacements: &BTreeMap<DimensionParameterId, DimensionParameterId>,
) -> Result<CardinalitySpec, ()> {
    Ok(match cardinality {
        CardinalitySpec::Exact(value) => {
            CardinalitySpec::Exact(remap_dimension_expr(value, replacements)?)
        }
        CardinalitySpec::Dynamic { upper_bound } => CardinalitySpec::Dynamic {
            upper_bound: upper_bound
                .as_ref()
                .map(|bound| remap_dimension_expr(bound, replacements))
                .transpose()?,
        },
    })
}

fn remap_schema_body(
    body: &SchemaBody,
    replacements: &BTreeMap<DimensionParameterId, DimensionParameterId>,
) -> Result<SchemaBody, ()> {
    Ok(match body {
        SchemaBody::Option(payload) => {
            SchemaBody::Option(Box::new(remap_schema_body(payload, replacements)?))
        }
        SchemaBody::Tuple(items) => SchemaBody::Tuple(
            items
                .iter()
                .map(|item| remap_schema_body(item, replacements))
                .collect::<Result<Vec<_>, _>>()?
                .into_boxed_slice(),
        ),
        SchemaBody::Record(fields) => SchemaBody::Record(
            fields
                .iter()
                .map(|field| {
                    Ok(SchemaField {
                        name: field.name.clone(),
                        schema: remap_schema_body(&field.schema, replacements)?,
                    })
                })
                .collect::<Result<Vec<_>, ()>>()?
                .into_boxed_slice(),
        ),
        SchemaBody::Matrix {
            element,
            dimensions,
        } => SchemaBody::Matrix {
            element: Box::new(remap_schema_body(element, replacements)?),
            dimensions: dimensions
                .iter()
                .map(|dimension| remap_dimension_expr(dimension, replacements))
                .collect::<Result<Vec<_>, _>>()?
                .into_boxed_slice(),
        },
        SchemaBody::Table { columns, rows } => SchemaBody::Table {
            columns: columns
                .iter()
                .map(|field| {
                    Ok(SchemaField {
                        name: field.name.clone(),
                        schema: remap_schema_body(&field.schema, replacements)?,
                    })
                })
                .collect::<Result<Vec<_>, ()>>()?
                .into_boxed_slice(),
            rows: remap_cardinality(rows, replacements)?,
        },
        SchemaBody::Set {
            element,
            cardinality,
        } => SchemaBody::Set {
            element: Box::new(remap_schema_body(element, replacements)?),
            cardinality: remap_cardinality(cardinality, replacements)?,
        },
        SchemaBody::Map {
            key,
            value,
            cardinality,
        } => SchemaBody::Map {
            key: Box::new(remap_schema_body(key, replacements)?),
            value: Box::new(remap_schema_body(value, replacements)?),
            cardinality: remap_cardinality(cardinality, replacements)?,
        },
        SchemaBody::Enum { key, variants } => SchemaBody::Enum {
            key: *key,
            variants: variants
                .iter()
                .map(|variant| {
                    Ok(mech_core::EnumVariantSchema {
                        name: variant.name.clone(),
                        payload: variant
                            .payload
                            .as_ref()
                            .map(|payload| remap_schema_body(payload, replacements))
                            .transpose()?,
                    })
                })
                .collect::<Result<Vec<_>, ()>>()?
                .into_boxed_slice(),
        },
        scalar => scalar.clone(),
    })
}

fn schema_draft_from_resolved(
    resolved: &ResolvedType,
    anchor: SourceSemanticAnchor,
) -> Result<SchemaDraft, SourceSemanticError> {
    fn body(
        kind: &KindExpr,
        anchor: SourceSemanticAnchor,
    ) -> Result<SchemaBody, SourceSemanticError> {
        Ok(match kind {
            KindExpr::Wildcard => SchemaBody::Dynamic,
            KindExpr::Named(id) => BuiltinScalarKind::from_kind_id(*id)
                .map(BuiltinScalarKind::schema_body)
                .ok_or_else(|| {
                    internal(
                        anchor,
                        format!("resolved named kind {id:?} has no source schema"),
                    )
                })?,
            KindExpr::Id => SchemaBody::Id,
            KindExpr::Index => SchemaBody::Index,
            KindExpr::Atom(key) => SchemaBody::Atom(*key),
            KindExpr::Matrix {
                element,
                dimensions,
            } => SchemaBody::Matrix {
                element: Box::new(body(element, anchor)?),
                dimensions: dimensions.clone(),
            },
            KindExpr::Option(payload) => SchemaBody::Option(Box::new(body(payload, anchor)?)),
            KindExpr::Tuple(items) => SchemaBody::Tuple(
                items
                    .iter()
                    .map(|item| body(item, anchor))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_boxed_slice(),
            ),
            KindExpr::Record(fields) => SchemaBody::Record(
                fields
                    .iter()
                    .map(|field| {
                        Ok(SchemaField {
                            name: field.name.clone(),
                            schema: body(&field.kind, anchor)?,
                        })
                    })
                    .collect::<Result<Vec<_>, SourceSemanticError>>()?
                    .into_boxed_slice(),
            ),
            KindExpr::Table { columns, rows } => SchemaBody::Table {
                columns: columns
                    .iter()
                    .map(|field| {
                        Ok(SchemaField {
                            name: field.name.clone(),
                            schema: body(&field.kind, anchor)?,
                        })
                    })
                    .collect::<Result<Vec<_>, SourceSemanticError>>()?
                    .into_boxed_slice(),
                rows: CardinalitySpec::Exact(rows.clone()),
            },
            KindExpr::Set {
                element,
                cardinality,
            } => SchemaBody::Set {
                element: Box::new(body(element, anchor)?),
                cardinality: CardinalitySpec::Exact(cardinality.clone()),
            },
            KindExpr::Map {
                key,
                value,
                cardinality,
            } => SchemaBody::Map {
                key: Box::new(body(key, anchor)?),
                value: Box::new(body(value, anchor)?),
                cardinality: CardinalitySpec::Exact(cardinality.clone()),
            },
            KindExpr::TypeOf(_) => SchemaBody::ReifiedType,
            KindExpr::Enum(_)
            | KindExpr::Never
            | KindExpr::Hole
            | KindExpr::Parameter(_)
            | KindExpr::Reference(_) => {
                return Err(internal(
                    anchor,
                    format!("resolved output kind cannot become a source schema: {kind:?}"),
                ));
            }
        })
    }

    Ok(SchemaDraft {
        dimension_parameters: resolved.dimension_parameters().to_vec().into_boxed_slice(),
        body: body(resolved.kind(), anchor)?,
    })
}

fn materialize_source_output_draft(
    resolved: &ResolvedType,
    rule: &ResolvedOutputSchemaRule,
    inputs: &[SchemaDraft],
    anchor: SourceSemanticAnchor,
) -> Result<SchemaDraft, SourceSemanticError> {
    match rule {
        ResolvedOutputSchemaRule::FromResolvedType => schema_draft_from_resolved(resolved, anchor),
        ResolvedOutputSchemaRule::FromInput(index) => {
            let input = inputs.get(*index).ok_or_else(|| {
                internal(
                    anchor,
                    format!("output schema input {index} is unavailable"),
                )
            })?;
            let draft = schema_draft_from_resolved(resolved, anchor)?;
            let (
                SchemaBody::Set {
                    element,
                    cardinality,
                },
                SchemaBody::Set {
                    cardinality: template_cardinality,
                    ..
                },
            ) = (&draft.body, &input.body)
            else {
                return Ok(draft);
            };
            let dynamic_parameter = match cardinality {
                CardinalitySpec::Exact(DimensionExpr::Parameter(id))
                    if matches!(template_cardinality, CardinalitySpec::Dynamic { .. })
                        || draft
                            .dimension_parameters
                            .iter()
                            .find(|declaration| declaration.id == *id)
                            .is_some_and(|declaration| {
                                declaration.lifetime == DimensionLifetime::Turn
                            }) =>
                {
                    Some(*id)
                }
                _ => None,
            };
            let Some(dynamic_parameter) = dynamic_parameter else {
                return Ok(draft);
            };
            let declaration = draft
                .dimension_parameters
                .iter()
                .find(|declaration| declaration.id == dynamic_parameter)
                .ok_or_else(|| {
                    internal(
                        anchor,
                        "set output references an unknown dimension".to_owned(),
                    )
                })?;
            let mut replacements = BTreeMap::new();
            for declaration in draft
                .dimension_parameters
                .iter()
                .filter(|declaration| declaration.id != dynamic_parameter)
            {
                replacements.insert(
                    declaration.id,
                    DimensionParameterId::new(replacements.len() as u32),
                );
            }
            let upper_bound = declaration
                .upper_bound
                .as_ref()
                .map(|bound| remap_dimension_expr(bound, &replacements))
                .transpose()
                .map_err(|_| internal(anchor, "set output bound is not closed".to_owned()))?;
            let dimension_parameters = draft
                .dimension_parameters
                .iter()
                .filter(|declaration| declaration.id != dynamic_parameter)
                .map(|declaration| {
                    Ok(DimensionParameterDeclaration {
                        id: replacements[&declaration.id],
                        origin: declaration.origin,
                        lifetime: declaration.lifetime,
                        lower_bound: remap_dimension_expr(&declaration.lower_bound, &replacements)
                            .map_err(|_| {
                                internal(anchor, "set output bound is not closed".to_owned())
                            })?,
                        upper_bound: declaration
                            .upper_bound
                            .as_ref()
                            .map(|bound| remap_dimension_expr(bound, &replacements))
                            .transpose()
                            .map_err(|_| {
                                internal(anchor, "set output bound is not closed".to_owned())
                            })?,
                    })
                })
                .collect::<Result<Vec<_>, SourceSemanticError>>()?;
            let element = remap_schema_body(element, &replacements)
                .map_err(|_| internal(anchor, "set output element is not closed".to_owned()))?;
            Ok(SchemaDraft {
                dimension_parameters: dimension_parameters.into_boxed_slice(),
                body: SchemaBody::Set {
                    element: Box::new(element),
                    cardinality: CardinalitySpec::Dynamic { upper_bound },
                },
            })
        }
        ResolvedOutputSchemaRule::TransposeOfInput(index) => {
            let input = inputs.get(*index).ok_or_else(|| {
                internal(
                    anchor,
                    format!("transpose schema input {index} is unavailable"),
                )
            })?;
            let SchemaBody::Matrix {
                element,
                dimensions,
            } = &input.body
            else {
                return Err(SourceSemanticError {
                    code: "source-semantics/non-matrix-transpose-kind",
                    message: "matrix transpose requires a matrix operand".to_owned(),
                    anchor,
                });
            };
            let [rows, columns] = dimensions.as_ref() else {
                return Err(SourceSemanticError {
                    code: "source-semantics/invalid-transpose-shape",
                    message: "matrix transpose requires exactly two dimensions".to_owned(),
                    anchor,
                });
            };
            Ok(SchemaDraft {
                dimension_parameters: input.dimension_parameters.clone(),
                body: SchemaBody::Matrix {
                    element: element.clone(),
                    dimensions: vec![columns.clone(), rows.clone()].into_boxed_slice(),
                },
            })
        }
        ResolvedOutputSchemaRule::Declared(body) => Ok(SchemaDraft {
            dimension_parameters: resolved.dimension_parameters().to_vec().into_boxed_slice(),
            body: body.clone(),
        }),
        ResolvedOutputSchemaRule::DynamicSetCartesianProduct => {
            let [left, right] = inputs else {
                return Err(internal(
                    anchor,
                    format!(
                        "set Cartesian product requires two inputs, received {}",
                        inputs.len()
                    ),
                ));
            };
            let set_element = |input: &SchemaDraft| {
                let SchemaBody::Set { element, .. } = &input.body else {
                    return Err(internal(
                        anchor,
                        "set operation input is not a set".to_owned(),
                    ));
                };
                Ok(SchemaDraft {
                    dimension_parameters: input.dimension_parameters.clone(),
                    body: element.as_ref().clone(),
                })
            };
            let mut parameters = Vec::new();
            let left = embed_schema_draft(&set_element(left)?, &mut parameters, anchor)?;
            let right = embed_schema_draft(&set_element(right)?, &mut parameters, anchor)?;
            Ok(SchemaDraft {
                dimension_parameters: parameters.into_boxed_slice(),
                body: SchemaBody::Set {
                    element: Box::new(SchemaBody::Tuple(vec![left, right].into_boxed_slice())),
                    cardinality: CardinalitySpec::Dynamic { upper_bound: None },
                },
            })
        }
        ResolvedOutputSchemaRule::DynamicSetPowerset => {
            let input = inputs
                .first()
                .ok_or_else(|| internal(anchor, "set powerset requires one input".to_owned()))?;
            let SchemaBody::Set {
                element,
                cardinality,
            } = &input.body
            else {
                return Err(internal(
                    anchor,
                    "set powerset input is not a set".to_owned(),
                ));
            };
            let upper_bound = match cardinality {
                CardinalitySpec::Exact(cardinality) => Some(cardinality.clone()),
                CardinalitySpec::Dynamic { upper_bound } => upper_bound.clone(),
            };
            Ok(SchemaDraft {
                dimension_parameters: input.dimension_parameters.clone(),
                body: SchemaBody::Set {
                    element: Box::new(SchemaBody::Set {
                        element: element.clone(),
                        cardinality: CardinalitySpec::Dynamic { upper_bound },
                    }),
                    cardinality: CardinalitySpec::Dynamic { upper_bound: None },
                },
            })
        }
    }
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
        "c32" => BuiltinScalarKind::C32,
        "c64" => BuiltinScalarKind::C64,
        "r64" => BuiltinScalarKind::R64,
        _ => return None,
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
    dynamic_payload: Option<(BuiltinSchema, Option<SchemaBody>, ValueDataDraft)>,
}

#[derive(Clone)]
struct PendingInput {
    name: String,
    schema: SchemaDraft,
    anchor: SourceSemanticAnchor,
}

struct PendingNode {
    inferable_projection: bool,
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
    initializer: PendingValue,
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
    bindings: Vec<PatternBinding>,
    dependencies: Vec<PendingValue>,
    syntax: SyntaxNode,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PatternBinding {
    name: String,
    schema: Option<SchemaDraft>,
    path: Vec<usize>,
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
    inputs: Vec<PendingInput>,
    input_by_name: BTreeMap<String, u32>,
    input_declarations: BTreeMap<String, SchemaDraft>,
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
            input_declarations: BTreeMap::new(),
            nodes: Vec::new(),
            states: Vec::new(),
            outputs: Vec::new(),
            bindings: BTreeMap::new(),
            patterns: Vec::new(),
            match_arms: Vec::new(),
            comprehension_qualifiers: Vec::new(),
        }
    }

    fn declare_input_annotations(
        &mut self,
        node: &SyntaxNode,
        bindings: &BTreeSet<String>,
    ) -> Result<(), SourceSemanticError> {
        if node.kind() == SyntaxKind::Variable {
            let variable = VariableSyntax::cast(node.clone()).expect("kind-checked variable cast");
            let stem = self.required(variable.stem(), variable.syntax(), "a variable stem")?;
            let name = node_text(stem.syntax())?;
            if !bindings.contains(&name)
                && let Some(annotation) = variable.annotation()
            {
                let schema = annotation_schema_draft(&annotation)?;
                if let Some(existing) = self.input_declarations.get(&name) {
                    if !is_dynamic_schema_draft(existing)
                        && !is_dynamic_schema_draft(&schema)
                        && *existing != schema
                    {
                        return Err(SourceSemanticError {
                            code: "source-semantics/conflicting-input-kind",
                            message: format!("input {name} has conflicting kind annotations"),
                            anchor: SourceSemanticAnchor::for_node(variable.syntax()),
                        });
                    }
                }
                if !is_dynamic_schema_draft(&schema) || !self.input_declarations.contains_key(&name)
                {
                    self.input_declarations.insert(name, schema);
                }
            }
            return Ok(());
        }
        if node.kind() == SyntaxKind::Pattern {
            let pattern = PatternSyntax::cast(node.clone()).expect("kind-checked pattern cast");
            return self.declare_pattern_input_annotations(&pattern, bindings);
        }
        if node.kind() == SyntaxKind::VariableDefine {
            let definition = VariableDefineSyntax::cast(node.clone())
                .expect("kind-checked variable definition cast");
            return self.declare_definition_input_annotations(&definition, bindings);
        }
        if node.kind() == SyntaxKind::Expression {
            let expression =
                ExpressionSyntax::cast(node.clone()).expect("kind-checked expression cast");
            let body =
                self.required(expression.body(), expression.syntax(), "an expression body")?;
            self.declare_input_annotations(body.syntax(), bindings)?;
            for arm in expression.match_arms() {
                let pattern = self.required(arm.pattern(), arm.syntax(), "a match pattern")?;
                self.declare_pattern_input_annotations(&pattern, bindings)?;
                let mut arm_bindings = bindings.clone();
                let mut pattern_bindings = Vec::new();
                collect_pattern_bindings(&pattern, &mut pattern_bindings)?;
                arm_bindings.extend(pattern_bindings.into_iter().map(|binding| binding.name));
                if let Some(guard) = arm.guard() {
                    self.declare_input_annotations(guard.syntax(), &arm_bindings)?;
                }
                if let Some(value) = arm.value() {
                    self.declare_input_annotations(value.syntax(), &arm_bindings)?;
                }
            }
            return Ok(());
        }
        if node.kind() == SyntaxKind::SetComprehension {
            let value = SetComprehensionSyntax::cast(node.clone())
                .expect("kind-checked set comprehension cast");
            return self.declare_comprehension_input_annotations(
                value.syntax(),
                value.value(),
                value.qualifiers(),
                bindings,
            );
        }
        if node.kind() == SyntaxKind::MatrixComprehension {
            let value = MatrixComprehensionSyntax::cast(node.clone())
                .expect("kind-checked matrix comprehension cast");
            return self.declare_comprehension_input_annotations(
                value.syntax(),
                value.value(),
                value.qualifiers(),
                bindings,
            );
        }
        for child in node.children() {
            self.declare_input_annotations(&child, bindings)?;
        }
        Ok(())
    }

    fn declare_definition_input_annotations(
        &mut self,
        definition: &VariableDefineSyntax,
        bindings: &BTreeSet<String>,
    ) -> Result<(), SourceSemanticError> {
        let value = self.required(
            definition.value(),
            definition.syntax(),
            "a definition value",
        )?;
        self.declare_input_annotations(value.syntax(), bindings)
    }

    fn declare_unit_input_annotations(
        &mut self,
        unit: &SyntaxNode,
        bindings: &mut BTreeSet<String>,
    ) -> Result<(), SourceSemanticError> {
        match unit.kind() {
            SyntaxKind::VariableDefine => {
                let definition = VariableDefineSyntax::cast(unit.clone())
                    .expect("kind-checked variable definition cast");
                self.declare_definition_input_annotations(&definition, bindings)?;
                let variable = self.required(
                    definition.variable(),
                    definition.syntax(),
                    "a defined variable",
                )?;
                let stem = self.required(variable.stem(), variable.syntax(), "a variable stem")?;
                bindings.insert(node_text(stem.syntax())?);
            }
            SyntaxKind::Expression => self.declare_input_annotations(unit, bindings)?,
            _ => unreachable!("document unit collector is closed"),
        }
        Ok(())
    }

    fn declare_comprehension_input_annotations(
        &mut self,
        syntax: &SyntaxNode,
        result: Option<ExpressionSyntax>,
        qualifiers: Vec<mech_syntax::document::ComprehensionQualifierSyntax>,
        bindings: &BTreeSet<String>,
    ) -> Result<(), SourceSemanticError> {
        let mut bindings = bindings.clone();
        for qualifier in qualifiers {
            let qualifier = self.required(
                qualifier.value(),
                qualifier.syntax(),
                "a comprehension qualifier",
            )?;
            match qualifier {
                ComprehensionQualifierValueSyntax::Generator(generator) => {
                    let source = self.required(
                        generator.source(),
                        generator.syntax(),
                        "a generator source",
                    )?;
                    self.declare_input_annotations(source.syntax(), &bindings)?;
                    let pattern = self.required(
                        generator.pattern(),
                        generator.syntax(),
                        "a generator pattern",
                    )?;
                    self.declare_pattern_input_annotations(&pattern, &bindings)?;
                    let mut pattern_bindings = Vec::new();
                    collect_pattern_bindings(&pattern, &mut pattern_bindings)?;
                    bindings.extend(pattern_bindings.into_iter().map(|binding| binding.name));
                }
                ComprehensionQualifierValueSyntax::Definition(definition) => {
                    self.declare_definition_input_annotations(&definition, &bindings)?;
                    let variable = self.required(
                        definition.variable(),
                        definition.syntax(),
                        "a defined variable",
                    )?;
                    let stem =
                        self.required(variable.stem(), variable.syntax(), "a variable stem")?;
                    bindings.insert(node_text(stem.syntax())?);
                }
                ComprehensionQualifierValueSyntax::Filter(filter) => {
                    self.declare_input_annotations(filter.syntax(), &bindings)?;
                }
            }
        }
        let result = self.required(result, syntax, "a comprehension result")?;
        self.declare_input_annotations(result.syntax(), &bindings)
    }

    fn declare_pattern_input_annotations(
        &mut self,
        pattern: &PatternSyntax,
        bindings: &BTreeSet<String>,
    ) -> Result<(), SourceSemanticError> {
        let value = self.required(pattern.value(), pattern.syntax(), "a pattern body")?;
        match value {
            PatternValueSyntax::Expression(expression) => {
                if standalone_pattern_variable(&expression).is_none() {
                    self.declare_input_annotations(expression.syntax(), bindings)?;
                }
            }
            PatternValueSyntax::Array(array) => {
                for element in array.elements() {
                    if let Some(pattern) = element.pattern() {
                        self.declare_pattern_input_annotations(&pattern, bindings)?;
                    }
                }
            }
            PatternValueSyntax::Tuple(tuple) => {
                for pattern in tuple.items() {
                    self.declare_pattern_input_annotations(&pattern, bindings)?;
                }
            }
            PatternValueSyntax::AtomStruct(tuple) => {
                for pattern in tuple.items() {
                    self.declare_pattern_input_annotations(&pattern, bindings)?;
                }
            }
            PatternValueSyntax::TupleStruct(tuple) => {
                for pattern in tuple.items() {
                    self.declare_pattern_input_annotations(&pattern, bindings)?;
                }
            }
            PatternValueSyntax::Wildcard(_) => {}
        }
        Ok(())
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
            let mut result_schema = None;
            for arm in arms {
                let pattern = self.required(arm.pattern(), arm.syntax(), "a match pattern")?;
                let saved = self.bindings.clone();
                let result = (|| {
                    let pattern = self.record_pattern(&pattern)?;
                    inputs.extend(pattern.dependencies.iter().copied());
                    self.bind_pattern(&pattern, value, None)?;
                    let guard_input = if let Some(guard) = arm.guard() {
                        let ordinal = inputs.len() as u32;
                        let guard_value = self.expression(&guard)?.0;
                        inputs.push(self.require_boolean_operand(guard_value, guard.syntax())?);
                        Some(ordinal)
                    } else {
                        None
                    };
                    let result = self.required(arm.value(), arm.syntax(), "a match result")?;
                    let result_input = inputs.len() as u32;
                    let result = self.expression(&result)?.0;
                    let schema = self.schema_draft_of(result);
                    if matches!(schema.body, SchemaBody::Dynamic) {
                        return Err(SourceSemanticError {
                            code: "source-semantics/unresolved-match-result-kind",
                            message: "match arms require a concrete result schema".to_owned(),
                            anchor: SourceSemanticAnchor::for_node(arm.syntax()),
                        });
                    }
                    if result_schema
                        .as_ref()
                        .is_some_and(|expected| expected != &schema)
                    {
                        return Err(SourceSemanticError {
                            code: "source-semantics/incompatible-match-result-kind",
                            message: "match arms require one exact result schema".to_owned(),
                            anchor: SourceSemanticAnchor::for_node(arm.syntax()),
                        });
                    }
                    result_schema.get_or_insert(schema);
                    inputs.push(result);
                    layouts.push((pattern.index, guard_input, result_input));
                    Ok::<_, SourceSemanticError>(())
                })();
                self.bindings = saved;
                result?;
            }
            value = self.emit_with_schema_draft(
                "source/match",
                inputs,
                result_schema.expect("a nonempty match has a result schema"),
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
        mut lhs: PendingValue,
        mut rhs: PendingValue,
        syntax: &SyntaxNode,
    ) -> Result<PendingValue, SourceSemanticError> {
        let (mut name, _) = operator_name(operator);
        if operator == CanonicalOperator::Add
            && (self.schema_of(lhs) == BuiltinSchema::String
                || self.schema_of(rhs) == BuiltinSchema::String)
        {
            name = "string/concat";
        }
        if matches!(
            operator,
            CanonicalOperator::Or | CanonicalOperator::And | CanonicalOperator::Xor
        ) {
            lhs = self.conform_dynamic_operand(lhs, BuiltinSchema::Bool, syntax)?;
            rhs = self.conform_dynamic_operand(rhs, BuiltinSchema::Bool, syntax)?;
        }

        let same_kind_inputs = matches!(
            operator,
            CanonicalOperator::Add
                | CanonicalOperator::Subtract
                | CanonicalOperator::Multiply
                | CanonicalOperator::Divide
                | CanonicalOperator::Modulus
                | CanonicalOperator::Power
                | CanonicalOperator::NotEqual
                | CanonicalOperator::EqualTo
                | CanonicalOperator::StrictNotEqual
                | CanonicalOperator::StrictEqual
                | CanonicalOperator::GreaterThan
                | CanonicalOperator::LessThan
                | CanonicalOperator::GreaterThanEqual
                | CanonicalOperator::LessThanEqual
                | CanonicalOperator::Or
                | CanonicalOperator::And
                | CanonicalOperator::Xor
                | CanonicalOperator::Union
                | CanonicalOperator::Intersection
                | CanonicalOperator::Difference
                | CanonicalOperator::Subset
                | CanonicalOperator::Superset
                | CanonicalOperator::ProperSubset
                | CanonicalOperator::ProperSuperset
                | CanonicalOperator::SymmetricDifference
        );
        if same_kind_inputs {
            match (
                self.is_genuinely_dynamic(lhs),
                self.is_genuinely_dynamic(rhs),
            ) {
                (true, false) => {
                    lhs =
                        self.conform_dynamic_to_schema(lhs, &self.schema_draft_of(rhs), syntax)?;
                }
                (false, true) => {
                    rhs =
                        self.conform_dynamic_to_schema(rhs, &self.schema_draft_of(lhs), syntax)?;
                }
                _ => {}
            }
        }

        if matches!(
            operator,
            CanonicalOperator::Add
                | CanonicalOperator::Subtract
                | CanonicalOperator::Multiply
                | CanonicalOperator::Divide
                | CanonicalOperator::Power
        ) && [lhs, rhs].into_iter().any(|value| {
            let body = self.schema_body_of(value);
            matches!(body, SchemaBody::Complex(FloatWidth::W32))
                || matches!(
                    body,
                    SchemaBody::Matrix { element, .. }
                        if matches!(element.as_ref(), SchemaBody::Complex(FloatWidth::W32))
                )
        }) {
            return Err(SourceSemanticError {
                code: "source-semantics/unsupported-resident-arithmetic-kind",
                message: "resident arithmetic does not provide c32 kernels".to_owned(),
                anchor: SourceSemanticAnchor::for_node(syntax),
            });
        }

        let resolved = self
            .resolve_maintained_call(name, vec![lhs, rhs], syntax)
            .map_err(|mut error| {
                if error.code != "source-semantics/incompatible-call-kind" {
                    return error;
                }
                error.code = match operator {
                    CanonicalOperator::Or
                    | CanonicalOperator::And
                    | CanonicalOperator::Xor => "source-semantics/non-boolean-operator-kind",
                    CanonicalOperator::NotEqual
                    | CanonicalOperator::EqualTo
                    | CanonicalOperator::StrictNotEqual
                    | CanonicalOperator::StrictEqual
                    | CanonicalOperator::GreaterThan
                    | CanonicalOperator::LessThan
                    | CanonicalOperator::GreaterThanEqual
                    | CanonicalOperator::LessThanEqual => {
                        "source-semantics/incompatible-comparison-kinds"
                    }
                    CanonicalOperator::Modulus => "source-semantics/invalid-modulus-kind",
                    CanonicalOperator::Add
                    | CanonicalOperator::Subtract
                    | CanonicalOperator::Multiply
                    | CanonicalOperator::Divide
                    | CanonicalOperator::Power => {
                        let has_c32 = [lhs, rhs].into_iter().any(|value| {
                            let body = self.schema_body_of(value);
                            matches!(body, SchemaBody::Complex(FloatWidth::W32))
                                || matches!(
                                    body,
                                    SchemaBody::Matrix { element, .. }
                                        if matches!(element.as_ref(), SchemaBody::Complex(FloatWidth::W32))
                                )
                        });
                        if has_c32 {
                            "source-semantics/unsupported-resident-arithmetic-kind"
                        } else {
                            "source-semantics/non-numeric-arithmetic-kind"
                        }
                    }
                    _ => error.code,
                };
                error
            })?;
        let Some((inputs, output)) = resolved else {
            return Err(SourceSemanticError {
                code: "source-semantics/unsupported-operator-type-authority",
                message: format!("operator {name} has no maintained type declaration"),
                anchor: SourceSemanticAnchor::for_node(syntax),
            });
        };
        Ok(self.emit_with_schema_draft(name, inputs, output, syntax, "operator", None))
    }

    fn conform_dynamic_to_schema(
        &mut self,
        value: PendingValue,
        target: &SchemaDraft,
        syntax: &SyntaxNode,
    ) -> Result<PendingValue, SourceSemanticError> {
        if !self.is_genuinely_dynamic(value) {
            return Ok(value);
        }
        if matches!(target.body, SchemaBody::Dynamic) {
            return Err(SourceSemanticError {
                code: "source-semantics/unresolved-operator-kind",
                message: "operator operands require at least one concrete kind".to_owned(),
                anchor: SourceSemanticAnchor::for_node(syntax),
            });
        }
        // An undeclared source name is an inference site. Dynamic is a real boxed
        // runtime kind, and Type System v1 never permits casting out of it.
        if let PendingValue::Input(index) = value {
            let input = &mut self.inputs[index as usize];
            if !self.input_declarations.contains_key(&input.name) {
                input.schema = target.clone();
                return Ok(value);
            }
        }
        if let PendingValue::Node(index) = value {
            let node = &mut self.nodes[index as usize];
            if node.inferable_projection {
                node.schema = builtin_schema_for_annotation_body(&target.body)
                    .unwrap_or(BuiltinSchema::Dynamic);
                node.schema_body = Some(target.body.clone());
                node.schema_parameters = target.dimension_parameters.clone();
                node.inferable_projection = false;
                return Ok(value);
            }
        }
        Err(SourceSemanticError {
            code: "source-semantics/unsupported-dynamic-conversion",
            message: "a Dynamic payload cannot satisfy a concrete operand kind; annotate the source input with its concrete kind".to_owned(),
            anchor: SourceSemanticAnchor::for_node(syntax),
        })
    }

    fn require_boolean_operand(
        &mut self,
        operand: PendingValue,
        syntax: &SyntaxNode,
    ) -> Result<PendingValue, SourceSemanticError> {
        match self.schema_of(operand) {
            BuiltinSchema::Bool => Ok(operand),
            BuiltinSchema::Dynamic
                if matches!(self.schema_body_of(operand), SchemaBody::Dynamic) =>
            {
                self.conform_dynamic_to_schema(
                    operand,
                    &SchemaDraft {
                        body: SchemaBody::Bool,
                        dimension_parameters: Box::new([]),
                    },
                    syntax,
                )
            }
            _ => Err(SourceSemanticError {
                code: "source-semantics/non-boolean-operator-kind",
                message: "logical binary operators require boolean operands".to_owned(),
                anchor: SourceSemanticAnchor::for_node(syntax),
            }),
        }
    }

    fn is_genuinely_dynamic(&self, value: PendingValue) -> bool {
        self.schema_of(value) == BuiltinSchema::Dynamic
            && matches!(self.schema_body_of(value), SchemaBody::Dynamic)
    }

    fn conform_dynamic_operand(
        &mut self,
        value: PendingValue,
        target: BuiltinSchema,
        syntax: &SyntaxNode,
    ) -> Result<PendingValue, SourceSemanticError> {
        if self.is_genuinely_dynamic(value) {
            self.conform_value(
                value,
                target,
                syntax,
                "source-semantics/incompatible-operator-kind",
                "operator operands require compatible concrete kinds",
            )
        } else {
            Ok(value)
        }
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
                let schema = self.schema_draft_of(operand);
                let scalar = builtin_schema_for_body(&schema.body);
                let matrix_element = match &schema.body {
                    SchemaBody::Matrix { element, .. } => builtin_schema_for_body(element),
                    _ => None,
                };
                if scalar == Some(BuiltinSchema::C32) || matrix_element == Some(BuiltinSchema::C32)
                {
                    return Err(SourceSemanticError {
                        code: "source-semantics/unsupported-resident-arithmetic-kind",
                        message: "resident arithmetic does not provide c32 negation".to_owned(),
                        anchor: SourceSemanticAnchor::for_node(value.syntax()),
                    });
                }
                let negatable = scalar.or(matrix_element).is_some_and(|schema| {
                    builtin_kind(schema).is_some_and(|kind| {
                        resolved_builtin_type(kind, value.syntax()).is_ok_and(|resolved| {
                            resolved.satisfies(BuiltinKindPredicate::Negatable)
                        })
                    })
                });
                if !negatable {
                    return Err(SourceSemanticError {
                        code: if matches!(schema.body, SchemaBody::Dynamic) {
                            "source-semantics/unresolved-negation-kind"
                        } else {
                            "source-semantics/non-negatable-kind"
                        },
                        message: "unary negation requires a concrete negatable kind".to_owned(),
                        anchor: SourceSemanticAnchor::for_node(value.syntax()),
                    });
                }
                if matches!(schema.body, SchemaBody::Matrix { .. }) {
                    self.emit_with_schema_draft(
                        "math/neg",
                        vec![operand],
                        schema,
                        value.syntax(),
                        "unary",
                        None,
                    )
                } else {
                    self.emit(
                        "math/neg",
                        vec![operand],
                        scalar.expect("validated scalar negation"),
                        value.syntax(),
                        "unary",
                        None,
                    )
                }
            }
            FactorValueSyntax::Not(value) => {
                let operand = self.required(value.operand(), value.syntax(), "a unary operand")?;
                let mut operand = self.factor(&operand)?;
                if self.is_genuinely_dynamic(operand) {
                    operand =
                        self.conform_dynamic_operand(operand, BuiltinSchema::Bool, value.syntax())?;
                }
                let Some((inputs, output)) = self
                    .resolve_maintained_call("logic/not", vec![operand], value.syntax())
                    .map_err(|mut error| {
                        if error.code == "source-semantics/incompatible-call-kind" {
                            error.code = "source-semantics/non-boolean-negation-kind";
                        }
                        error
                    })?
                else {
                    return Err(internal(
                        SourceSemanticAnchor::for_node(value.syntax()),
                        "logical negation has no maintained type declaration".to_owned(),
                    ));
                };
                self.emit_with_schema_draft(
                    "logic/not",
                    inputs,
                    output,
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
                let detail = Some(format!("{function_name}({})", names.join(",")));
                if let Some((inputs, output)) =
                    self.resolve_maintained_call(&function_name, inputs.clone(), value.syntax())?
                {
                    self.emit_with_schema_draft(
                        &function_name,
                        inputs,
                        output,
                        value.syntax(),
                        "call",
                        detail,
                    )
                } else {
                    self.emit(
                        &function_name,
                        inputs,
                        BuiltinSchema::Dynamic,
                        value.syntax(),
                        "call",
                        detail,
                    )
                }
            }
            FactorValueSyntax::MatrixComprehension(value) => self.matrix_comprehension(&value)?,
            FactorValueSyntax::Slice(value) => self.slice(&value)?,
            FactorValueSyntax::Variable(value) => self.variable(&value)?,
        };
        if factor.transpose().is_some() {
            result = if self.is_genuinely_dynamic(result) {
                self.emit(
                    "matrix/transpose",
                    vec![result],
                    BuiltinSchema::Dynamic,
                    factor.syntax(),
                    "postfix",
                    None,
                )
            } else {
                let Some((inputs, schema)) = self.resolve_maintained_call(
                    "matrix/transpose",
                    vec![result],
                    factor.syntax(),
                )?
                else {
                    return Err(internal(
                        SourceSemanticAnchor::for_node(factor.syntax()),
                        "matrix transpose has no maintained type declaration".to_owned(),
                    ));
                };
                self.emit_with_schema_draft(
                    "matrix/transpose",
                    inputs,
                    schema,
                    factor.syntax(),
                    "postfix",
                    None,
                )
            };
        }
        Ok(result)
    }

    fn resolve_maintained_call(
        &mut self,
        name: &str,
        mut inputs: Vec<PendingValue>,
        syntax: &SyntaxNode,
    ) -> Result<Option<(Vec<PendingValue>, SchemaDraft)>, SourceSemanticError> {
        let Ok(mut declaration) = mech_core::maintained_source_type_declaration(name) else {
            return Ok(None);
        };
        let input_types = inputs
            .iter()
            .map(|input| {
                let schema = self.schema_draft_of(*input);
                if matches!(schema.body, SchemaBody::Dynamic) {
                    return Err(SourceSemanticError {
                        code: "source-semantics/unresolved-call-kind",
                        message: format!(
                            "maintained function {name} requires concrete argument kinds"
                        ),
                        anchor: SourceSemanticAnchor::for_node(syntax),
                    });
                }
                ResolvedType::from_schema_body(&schema.body, &schema.dimension_parameters).map_err(
                    |error| {
                        internal(
                            SourceSemanticAnchor::for_node(syntax),
                            format!("invalid maintained call argument schema: {error}"),
                        )
                    },
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        if let Some(template) = declaration.template {
            let schemes = mech_core::instantiate_source_scheme_template(template, &input_types)
                .map_err(|error| {
                    internal(
                        SourceSemanticAnchor::for_node(syntax),
                        format!("unable to instantiate maintained call template: {error:?}"),
                    )
                })?;
            declaration = mech_core::FunctionTypeDeclaration::from_schemes(schemes);
        }
        let overloads = declaration
            .overloads
            .iter()
            .filter(|overload| {
                let accepts_arity = match overload.scheme.inputs() {
                    InputKindScheme::Fixed(expected) => expected.len() == inputs.len(),
                    InputKindScheme::Variadic {
                        prefix,
                        min_repetitions,
                        ..
                    } => inputs.len() >= prefix.len().saturating_add(*min_repetitions as usize),
                };
                accepts_arity
                    && overload
                        .input_layout
                        .iter()
                        .all(|kind| *kind == SourceInputKind::Value)
            })
            .collect::<Vec<_>>();
        if overloads.is_empty() {
            return Err(SourceSemanticError {
                code: "source-semantics/incompatible-call-arity",
                message: format!("maintained function {name} does not accept this argument layout"),
                anchor: SourceSemanticAnchor::for_node(syntax),
            });
        }
        let candidates = overloads
            .iter()
            .map(|overload| TypeOverloadCandidate {
                id: u64::from(overload.id),
                scheme: &overload.scheme,
            })
            .collect::<Vec<_>>();
        let resolved = mech_core::resolve_type_overloads(
            TypeConstraintOrigin::new(name.to_owned(), None),
            &candidates,
            &input_types,
            None,
        )
        .map_err(|error| SourceSemanticError {
            code: "source-semantics/incompatible-call-kind",
            message: error.to_string(),
            anchor: SourceSemanticAnchor::for_node(syntax),
        })?;
        for ((input, actual), conversion) in inputs
            .iter_mut()
            .zip(&input_types)
            .zip(resolved.conversions.iter())
        {
            if actual == &conversion.target {
                continue;
            }
            *input =
                self.apply_resolved_conversion(*input, &conversion.target, conversion, syntax)?;
        }
        let [output] = resolved.outputs.as_ref() else {
            return Err(SourceSemanticError {
                code: "source-semantics/unsupported-call-output-layout",
                message: format!("maintained function {name} must have one value output"),
                anchor: SourceSemanticAnchor::for_node(syntax),
            });
        };
        let output_rule = resolved
            .candidate_ids
            .iter()
            .map(|candidate| {
                overloads
                    .iter()
                    .find(|overload| u64::from(overload.id) == *candidate)
                    .and_then(|overload| overload.output_schema_rules.first())
                    .cloned()
                    .ok_or_else(|| {
                        internal(
                            SourceSemanticAnchor::for_node(syntax),
                            format!(
                                "maintained function {name} selected an unknown output-schema rule"
                            ),
                        )
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let Some(first_rule) = output_rule.first() else {
            return Err(internal(
                SourceSemanticAnchor::for_node(syntax),
                format!("maintained function {name} selected no overload"),
            ));
        };
        if output_rule.iter().any(|rule| rule != first_rule) {
            return Err(internal(
                SourceSemanticAnchor::for_node(syntax),
                format!("maintained function {name} selected conflicting output-schema rules"),
            ));
        }
        let input_schemas = inputs
            .iter()
            .map(|input| self.schema_draft_of(*input))
            .collect::<Vec<_>>();
        Ok(Some((
            inputs,
            materialize_source_output_draft(
                output,
                first_rule,
                &input_schemas,
                SourceSemanticAnchor::for_node(syntax),
            )?,
        )))
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
        let mut values = values;
        let concrete = values
            .iter()
            .copied()
            .filter(|value| !self.is_genuinely_dynamic(*value))
            .collect::<Vec<_>>();
        for value in &concrete {
            if !resolved_schema_type(self.schema_of(*value), range.syntax())?
                .is_some_and(|kind| kind.satisfies(BuiltinKindPredicate::RangeEndpoint))
            {
                return Err(SourceSemanticError {
                    code: "source-semantics/invalid-range-endpoint-kind",
                    message: "range endpoints require a range-endpoint kind".to_owned(),
                    anchor: SourceSemanticAnchor::for_node(range.syntax()),
                });
            }
        }
        let mut peer = *concrete.first().ok_or_else(|| SourceSemanticError {
            code: "source-semantics/unresolved-range-kind",
            message: "range endpoints require a concrete peer kind".to_owned(),
            anchor: SourceSemanticAnchor::for_node(range.syntax()),
        })?;
        for other in concrete.iter().skip(1) {
            peer = self.promote_operands(peer, *other, range.syntax())?.0;
        }
        let element_schema = self.schema_of(peer);
        for value in &mut values {
            *value = self.conform_value(
                *value,
                element_schema,
                range.syntax(),
                "source-semantics/incompatible-range-kind",
                "range endpoints do not share a compatible kind",
            )?;
        }
        let extent = DimensionParameterId::new(0);
        let range_endpoint = resolved_schema_type(element_schema, range.syntax())?
            .is_some_and(|kind| kind.satisfies(BuiltinKindPredicate::RangeEndpoint));
        if !range_endpoint {
            return Err(SourceSemanticError {
                code: "source-semantics/invalid-range-endpoint-kind",
                message: "range endpoints require a range-endpoint kind".to_owned(),
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
        let annotation = variable
            .annotation()
            .map(|annotation| annotation_schema_draft(&annotation))
            .transpose()?;
        if let Some(value) = self.bindings.get(&name).copied() {
            return annotation.map_or(Ok(value), |expected| {
                self.conform_schema_draft(
                    value,
                    &expected,
                    variable.syntax(),
                    "source-semantics/incompatible-local-kind",
                    "local value does not satisfy its occurrence annotation",
                )
            });
        }
        let schema = annotation.unwrap_or_else(dynamic_schema_draft);
        let declared = self
            .input_declarations
            .get(&name)
            .cloned()
            .unwrap_or_else(|| schema.clone());
        if !is_dynamic_schema_draft(&schema)
            && !is_dynamic_schema_draft(&declared)
            && schema != declared
        {
            return Err(SourceSemanticError {
                code: "source-semantics/conflicting-input-kind",
                message: format!("input {name} has conflicting kind annotations"),
                anchor: SourceSemanticAnchor::for_node(variable.syntax()),
            });
        }
        if let Some(index) = self.input_by_name.get(&name) {
            let existing = &self.inputs[*index as usize].schema;
            if !is_dynamic_schema_draft(&schema)
                && !is_dynamic_schema_draft(existing)
                && schema != *existing
            {
                return Err(SourceSemanticError {
                    code: "source-semantics/conflicting-input-kind",
                    message: format!("input {name} has conflicting kind annotations"),
                    anchor: SourceSemanticAnchor::for_node(variable.syntax()),
                });
            }
            return Ok(PendingValue::Input(*index));
        }
        let index = u32::try_from(self.inputs.len()).map_err(|_| SourceSemanticError {
            code: "source-semantics/input-identity-exhausted",
            message: "canonical input count exceeds SourceProgram identity space".to_owned(),
            anchor: SourceSemanticAnchor::for_node(&syntax),
        })?;
        self.input_by_name.insert(name.clone(), index);
        self.inputs.push(PendingInput {
            name,
            schema: declared,
            anchor: SourceSemanticAnchor::for_node(&syntax),
        });
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
            let expected = annotation_schema_draft(&annotation)?;
            value = self.conform_schema_draft(
                value,
                &expected,
                definition.syntax(),
                "source-semantics/incompatible-definition-kind",
                "definition value does not satisfy the declared kind",
            )?;
        }
        let bound = if definition.mutability_marker().is_some() {
            let schema = self.schema_of(value);
            let schema_draft = self.schema_draft_of(value);
            let state = u32::try_from(self.states.len()).map_err(|_| SourceSemanticError {
                code: "source-semantics/state-identity-exhausted",
                message: "canonical state count exceeds SourceProgram identity space".to_owned(),
                anchor: SourceSemanticAnchor::for_node(definition.syntax()),
            })?;
            let node = self.nodes.len() as u32;
            self.states.push(PendingState {
                schema,
                initializer: value,
                producer_node: node,
            });
            self.nodes.push(PendingNode {
                inferable_projection: false,
                operation: operation_reference("core/assign"),
                inputs: vec![PendingValue::State(state)],
                schema,
                schema_body: Some(schema_draft.body),
                schema_parameters: schema_draft.dimension_parameters,
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
                let source = canonical_number_source(&value)?;
                let dynamic_option = annotation == Some(BuiltinSchema::OptionDynamic);
                let selected_suffix = selected_integer_suffix(&value)?;
                if selected_suffix.is_some()
                    && annotation.is_some_and(|schema| schema != BuiltinSchema::Dynamic)
                {
                    let (schema, data) =
                        decode_number(&source, None).ok_or_else(|| SourceSemanticError {
                            code: "source-semantics/invalid-number-literal",
                            message: format!(
                                "canonical number {source:?} could not be represented"
                            ),
                            anchor: SourceSemanticAnchor::for_node(value.syntax()),
                        })?;
                    if dynamic_option {
                        return Ok(self.constant_dynamic_option(schema, None, data));
                    }
                    let literal = self.constant(schema, data);
                    return self.conform_value(
                        literal,
                        annotation.expect("checked numeric annotation"),
                        value.syntax(),
                        "source-semantics/incompatible-literal-kind",
                        "numeric literal does not satisfy its outer kind annotation",
                    );
                }
                let (schema, data) =
                    decode_number(&source, if dynamic_option { None } else { annotation })
                        .ok_or_else(|| SourceSemanticError {
                            code: "source-semantics/invalid-number-literal",
                            message: format!(
                                "canonical number {source:?} could not be represented"
                            ),
                            anchor: SourceSemanticAnchor::for_node(value.syntax()),
                        })?;
                Ok(if dynamic_option {
                    self.constant_dynamic_option(schema, None, data)
                } else {
                    self.constant(schema, data)
                })
            }
            LiteralValueSyntax::Empty(value) => {
                if let Some(option) =
                    annotation.filter(|schema| option_payload_schema(*schema).is_some())
                {
                    return Ok(self.constant(
                        option,
                        ValueDataDraft::Option(OptionDraft {
                            present: false,
                            value: None,
                        }),
                    ));
                }
                if annotation.is_some() {
                    return Err(SourceSemanticError {
                        code: "source-semantics/incompatible-literal-kind",
                        message: "empty literals require an optional kind annotation".to_owned(),
                        anchor: SourceSemanticAnchor::for_node(value.syntax()),
                    });
                }
                Ok(self.emit(
                    "source/empty",
                    Vec::new(),
                    BuiltinSchema::Dynamic,
                    value.syntax(),
                    "empty-literal",
                    None,
                ))
            }
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
                self.exact_literal_constant(
                    annotation,
                    SchemaBody::Atom(key),
                    ValueDataDraft::Atom,
                    value.syntax(),
                )
            }
            LiteralValueSyntax::KindAnnotation(value) => self.kind_value(&value, annotation),
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

    fn exact_literal_constant(
        &mut self,
        annotation: Option<BuiltinSchema>,
        actual: SchemaBody,
        data: ValueDataDraft,
        syntax: &SyntaxNode,
    ) -> Result<PendingValue, SourceSemanticError> {
        require_literal_schema(annotation, BuiltinSchema::Dynamic, syntax)?;
        if annotation == Some(BuiltinSchema::OptionDynamic) {
            return Ok(self.constant_dynamic_option(BuiltinSchema::Dynamic, Some(actual), data));
        }
        Ok(self.constant_exact(actual, data))
    }

    fn kind_value(
        &mut self,
        kind: &KindAnnotationSyntax,
        annotation: Option<BuiltinSchema>,
    ) -> Result<PendingValue, SourceSemanticError> {
        let (kind_expr, dimensions) = annotation_kind_expr(kind)?;
        let paths = BuiltinKindPaths::build(SourceSemanticAnchor::for_node(kind.syntax()))?;
        let reified =
            ReifiedKind::from_closed_kind(&kind_expr, &dimensions, &paths).map_err(|error| {
                internal(
                    SourceSemanticAnchor::for_node(kind.syntax()),
                    format!("unable to canonicalize kind value: {error:?}"),
                )
            })?;
        self.exact_literal_constant(
            annotation,
            SchemaBody::ReifiedType,
            ValueDataDraft::Type(ReifiedTypeDraft::CanonicalKind(
                reified.canonical_bytes().to_vec().into_boxed_slice(),
            )),
            kind.syntax(),
        )
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
            StructureValueSyntax::EmptyMap(value) => Err(SourceSemanticError {
                code: "source-semantics/unresolved-map-entry-kind",
                message: "empty map literals require explicit key and value kinds".to_owned(),
                anchor: SourceSemanticAnchor::for_node(value.syntax()),
            }),
            StructureValueSyntax::EmptySet(value) => Err(SourceSemanticError {
                code: "source-semantics/unresolved-set-element-kind",
                message: "empty set literals require an explicit element kind".to_owned(),
                anchor: SourceSemanticAnchor::for_node(value.syntax()),
            }),
        }
    }

    fn matrix(&mut self, matrix: &MatrixSyntax) -> Result<PendingValue, SourceSemanticError> {
        let rows = matrix.rows();
        let mut values = Vec::new();
        for row in rows {
            let columns = row.columns();
            let mut row_values = Vec::with_capacity(columns.len());
            for column in columns {
                let value = self.required(column.value(), column.syntax(), "a matrix value")?;
                let value = self.expression(&value)?.0;
                row_values.push((!self.take_source_absence(value)).then_some(value));
            }
            values.push(row_values);
        }
        let Some(first) = values.first() else {
            return Err(SourceSemanticError {
                code: "source-semantics/empty-matrix-literal",
                message: "matrix literals require at least one row".to_owned(),
                anchor: SourceSemanticAnchor::for_node(matrix.syntax()),
            });
        };
        if first.is_empty() {
            return Err(SourceSemanticError {
                code: "source-semantics/matrix-row-width",
                message: "matrix literal rows require a nonempty source row".to_owned(),
                anchor: SourceSemanticAnchor::for_node(matrix.syntax()),
            });
        }
        if values.iter().flatten().any(Option::is_none) {
            return self.optional_matrix(values, matrix.syntax());
        }

        let mut rows = Vec::with_capacity(values.len());
        for row in values {
            let row = row
                .into_iter()
                .map(|value| value.expect("absence handled above"))
                .collect::<Vec<_>>();
            let Some((inputs, output)) =
                self.resolve_maintained_call("matrix/horzcat", row, matrix.syntax())?
            else {
                return Err(internal(
                    SourceSemanticAnchor::for_node(matrix.syntax()),
                    "horizontal matrix concatenation has no maintained type declaration".to_owned(),
                ));
            };
            rows.push(self.emit_with_schema_draft(
                "matrix/horzcat",
                inputs,
                output,
                matrix.syntax(),
                "matrix-row",
                None,
            ));
        }
        if rows.len() == 1 {
            return Ok(rows[0]);
        }
        let Some((inputs, output)) =
            self.resolve_maintained_call("matrix/vertcat", rows, matrix.syntax())?
        else {
            return Err(internal(
                SourceSemanticAnchor::for_node(matrix.syntax()),
                "vertical matrix concatenation has no maintained type declaration".to_owned(),
            ));
        };
        Ok(self.emit_with_schema_draft(
            "matrix/vertcat",
            inputs,
            output,
            matrix.syntax(),
            "matrix",
            None,
        ))
    }

    fn optional_matrix(
        &mut self,
        values: Vec<Vec<Option<PendingValue>>>,
        syntax: &SyntaxNode,
    ) -> Result<PendingValue, SourceSemanticError> {
        let element = values
            .iter()
            .flatten()
            .filter_map(|value| *value)
            .map(|value| self.schema_draft_of(value))
            .find(|schema| !matches!(schema.body, SchemaBody::Dynamic))
            .ok_or_else(|| SourceSemanticError {
                code: "source-semantics/unresolved-matrix-element-kind",
                message: "matrix literals require a present value with a concrete element kind"
                    .to_owned(),
                anchor: SourceSemanticAnchor::for_node(syntax),
            })?;
        let Some(payload) = builtin_schema_for_annotation_body(&element.body) else {
            return Err(SourceSemanticError {
                code: "source-semantics/incompatible-matrix-element-kind",
                message: "absent matrix cells require a concrete scalar peer".to_owned(),
                anchor: SourceSemanticAnchor::for_node(syntax),
            });
        };
        let option = if option_payload_schema(payload).is_some() {
            payload
        } else {
            option_schema(payload).expect("builtin scalar schemas have option schemas")
        };
        let mut inputs = Vec::new();
        let mut row_width = None;
        for row in &values {
            let width = row.len();
            row_width.get_or_insert(width);
            if width == 0 || row_width != Some(width) {
                return Err(SourceSemanticError {
                    code: "source-semantics/matrix-row-width",
                    message: "optional matrix rows require one common nonzero scalar width"
                        .to_owned(),
                    anchor: SourceSemanticAnchor::for_node(syntax),
                });
            }
            for value in row {
                inputs.push(match value {
                    Some(value) => self.conform_value(
                        *value,
                        option,
                        syntax,
                        "source-semantics/incompatible-matrix-element-kind",
                        "optional matrix elements require one exact scalar kind",
                    )?,
                    None => self.constant(
                        option,
                        ValueDataDraft::Option(OptionDraft {
                            present: false,
                            value: None,
                        }),
                    ),
                });
            }
        }
        let columns = row_width.expect("nonempty optional matrix has a row width");
        Ok(self.emit_with_schema_draft(
            "matrix/literal",
            inputs,
            SchemaDraft {
                dimension_parameters: Box::new([]),
                body: SchemaBody::Matrix {
                    element: Box::new(schema_body(option)),
                    dimensions: vec![
                        DimensionExpr::Constant(values.len() as u64),
                        DimensionExpr::Constant(columns as u64),
                    ]
                    .into_boxed_slice(),
                },
            },
            syntax,
            "matrix",
            Some(format!("row-width={columns}")),
        ))
    }

    fn take_source_absence(&mut self, value: PendingValue) -> bool {
        let PendingValue::Node(index) = value else {
            return false;
        };
        if index as usize + 1 != self.nodes.len()
            || self.nodes[index as usize].operation.canonical_name() != "source/empty"
        {
            return false;
        }
        self.nodes.pop();
        true
    }

    fn table(&mut self, table: &TableSyntax) -> Result<PendingValue, SourceSemanticError> {
        let value = self.required(table.value(), table.syntax(), "a table presentation")?;
        let (mut headers, rows, syntax): (
            Vec<(String, BuiltinSchema, Option<SchemaDraft>)>,
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
                        Ok((node_text(name.syntax())?, schema, None))
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
                        Ok((
                            node_text(name.syntax())?,
                            annotation_schema(&annotation)?,
                            None,
                        ))
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
                        Ok((
                            node_text(name.syntax())?,
                            annotation_schema(&annotation)?,
                            None,
                        ))
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
            if headers[index].1 == BuiltinSchema::Dynamic && headers[index].2.is_none() {
                let inferred = compiled_rows
                    .iter()
                    .map(|row| self.schema_draft_of(row[index].0))
                    .find(|schema| !matches!(schema.body, SchemaBody::Dynamic))
                    .ok_or_else(|| SourceSemanticError {
                        code: "source-semantics/unresolved-table-column-kind",
                        message: format!(
                            "table field {} has no value from which to infer its kind",
                            headers[index].0
                        ),
                        anchor: SourceSemanticAnchor::for_node(&syntax),
                    })?;
                if let Some(schema) = builtin_schema_for_body(&inferred.body) {
                    headers[index].1 = schema;
                } else {
                    headers[index].2 = Some(inferred);
                }
            }
            let (name, expected, exact) = headers[index].clone();
            for row in &mut compiled_rows {
                row[index].0 = if let Some(expected) = exact.as_ref() {
                    self.conform_exact_table_value(row[index].0, expected, &name, &row[index].1)?
                } else {
                    self.conform_table_value(row[index].0, expected, &name, &row[index].1)?
                };
            }
        }
        let inputs = compiled_rows
            .into_iter()
            .flatten()
            .map(|(value, _)| value)
            .collect::<Vec<_>>();
        let mut schema_parameters = Vec::new();
        let columns = headers
            .iter()
            .map(|(name, schema, exact)| {
                let schema = if let Some(exact) = exact {
                    embed_schema_draft(
                        exact,
                        &mut schema_parameters,
                        SourceSemanticAnchor::for_node(&syntax),
                    )?
                } else {
                    schema_body(*schema)
                };
                Ok(SchemaField {
                    name: name.clone(),
                    schema,
                })
            })
            .collect::<Result<Vec<_>, SourceSemanticError>>()?;
        let schema_body = SchemaBody::Table {
            columns: columns.into_boxed_slice(),
            rows: CardinalitySpec::Exact(DimensionExpr::Constant(widths.len() as u64)),
        };
        let names = headers
            .iter()
            .map(|(name, _, _)| name.as_str())
            .collect::<Vec<_>>();
        Ok(self.emit_with_schema_draft(
            "source/table",
            inputs,
            SchemaDraft {
                dimension_parameters: schema_parameters.into_boxed_slice(),
                body: schema_body,
            },
            &syntax,
            "table",
            Some(format!("headers={names:?};row-widths={widths:?}")),
        ))
    }

    fn map(&mut self, map: &MapSyntax) -> Result<PendingValue, SourceSemanticError> {
        let mut entries = Vec::new();
        for entry in map.entries() {
            let key = self.required(entry.key(), entry.syntax(), "a map key")?;
            let value = self.required(entry.value(), entry.syntax(), "a map value")?;
            entries.push((self.expression(&key)?.0, self.expression(&value)?.0));
        }
        let key_schema = entries
            .iter()
            .map(|(key, _)| self.schema_draft_of(*key))
            .find(|schema| !matches!(schema.body, SchemaBody::Dynamic))
            .ok_or_else(|| SourceSemanticError {
                code: "source-semantics/unresolved-map-key-kind",
                message: "map literals require a concrete key kind".to_owned(),
                anchor: SourceSemanticAnchor::for_node(map.syntax()),
            })?;
        let value_schema = entries
            .iter()
            .map(|(_, value)| self.schema_draft_of(*value))
            .find(|schema| !matches!(schema.body, SchemaBody::Dynamic))
            .ok_or_else(|| SourceSemanticError {
                code: "source-semantics/unresolved-map-value-kind",
                message: "map literals require a concrete value kind".to_owned(),
                anchor: SourceSemanticAnchor::for_node(map.syntax()),
            })?;

        for (key, value) in &mut entries {
            *key = self.conform_dynamic_to_schema(*key, &key_schema, map.syntax())?;
            *value = self.conform_dynamic_to_schema(*value, &value_schema, map.syntax())?;
            if self.schema_draft_of(*key) != key_schema {
                return Err(SourceSemanticError {
                    code: "source-semantics/incompatible-map-key-kind",
                    message: "map literal keys require one exact kind".to_owned(),
                    anchor: SourceSemanticAnchor::for_node(map.syntax()),
                });
            }
            if self.schema_draft_of(*value) != value_schema {
                return Err(SourceSemanticError {
                    code: "source-semantics/incompatible-map-value-kind",
                    message: "map literal values require one exact kind".to_owned(),
                    anchor: SourceSemanticAnchor::for_node(map.syntax()),
                });
            }
        }
        let resolved_key =
            ResolvedType::from_schema_body(&key_schema.body, &key_schema.dimension_parameters)
                .map_err(|error| {
                    internal(
                        SourceSemanticAnchor::for_node(map.syntax()),
                        format!("invalid map key schema: {error}"),
                    )
                })?;
        if !resolved_key.satisfies(BuiltinKindPredicate::Keyable) {
            return Err(SourceSemanticError {
                code: "source-semantics/non-keyable-map-key-kind",
                message: "map literal keys require a keyable kind".to_owned(),
                anchor: SourceSemanticAnchor::for_node(map.syntax()),
            });
        }
        let mut parameters = Vec::new();
        let key = embed_schema_draft(
            &key_schema,
            &mut parameters,
            SourceSemanticAnchor::for_node(map.syntax()),
        )?;
        let value = embed_schema_draft(
            &value_schema,
            &mut parameters,
            SourceSemanticAnchor::for_node(map.syntax()),
        )?;
        let inputs = entries
            .into_iter()
            .flat_map(|(key, value)| [key, value])
            .collect();
        Ok(self.emit_with_schema_draft(
            "source/map",
            inputs,
            SchemaDraft {
                dimension_parameters: parameters.into_boxed_slice(),
                body: SchemaBody::Map {
                    key: Box::new(key),
                    value: Box::new(value),
                    cardinality: CardinalitySpec::Exact(DimensionExpr::Constant(
                        map.entries().len() as u64,
                    )),
                },
            },
            map.syntax(),
            "map",
            None,
        ))
    }

    fn record(&mut self, record: &RecordSyntax) -> Result<PendingValue, SourceSemanticError> {
        let mut inputs = Vec::new();
        let mut fields = Vec::new();
        let mut parameters = Vec::new();
        for binding in record.bindings() {
            let name = self.required(binding.name(), binding.syntax(), "a record field name")?;
            let value = self.required(binding.value(), binding.syntax(), "a record field value")?;
            let name = node_text(name.syntax())?;
            let mut value = self.expression(&value)?.0;
            if let Some(annotation) = binding.annotation() {
                let expected = annotation_schema(&annotation)?;
                value = self.conform_value(
                    value,
                    expected,
                    binding.syntax(),
                    "source-semantics/incompatible-record-field-kind",
                    &format!("record field {name} does not satisfy its kind annotation"),
                )?;
            }
            fields.push(SchemaField {
                name,
                schema: embed_schema_draft(
                    &self.schema_draft_of(value),
                    &mut parameters,
                    SourceSemanticAnchor::for_node(record.syntax()),
                )?,
            });
            inputs.push(value);
        }
        let detail = fields
            .iter()
            .map(|field| field.name.as_str())
            .collect::<Vec<_>>()
            .join(",");
        Ok(self.emit_with_schema_draft(
            "source/record",
            inputs,
            SchemaDraft {
                dimension_parameters: parameters.into_boxed_slice(),
                body: SchemaBody::Record(fields.into_boxed_slice()),
            },
            record.syntax(),
            "record",
            Some(detail),
        ))
    }

    fn set(&mut self, set: &SetSyntax) -> Result<PendingValue, SourceSemanticError> {
        let mut values = Vec::new();
        for item in set.items() {
            let value = self.expression(&item)?.0;
            values.push((!self.take_source_absence(value)).then_some(value));
        }
        let mut element = values
            .iter()
            .filter_map(|value| *value)
            .map(|value| self.schema_draft_of(value))
            .find(|schema| !matches!(schema.body, SchemaBody::Dynamic))
            .ok_or_else(|| SourceSemanticError {
                code: "source-semantics/unresolved-set-element-kind",
                message: "set literals require a concrete present element kind".to_owned(),
                anchor: SourceSemanticAnchor::for_node(set.syntax()),
            })?;
        let absent = values.iter().any(Option::is_none);
        let optional = if absent {
            let payload = builtin_schema_for_annotation_body(&element.body)
                .and_then(|kind| {
                    if option_payload_schema(kind).is_some() {
                        Some(kind)
                    } else {
                        option_schema(kind)
                    }
                })
                .ok_or_else(|| SourceSemanticError {
                    code: "source-semantics/incompatible-set-element-kind",
                    message: "absent set elements require a concrete scalar peer".to_owned(),
                    anchor: SourceSemanticAnchor::for_node(set.syntax()),
                })?;
            element.body = schema_body(payload);
            Some(payload)
        } else {
            None
        };
        let mut inputs = Vec::new();
        for value in values {
            let input = match (value, optional) {
                (Some(value), Some(optional)) => self.conform_value(
                    value,
                    optional,
                    set.syntax(),
                    "source-semantics/incompatible-set-element-kind",
                    "optional set elements require one exact kind",
                )?,
                (None, Some(optional)) => self.constant(
                    optional,
                    ValueDataDraft::Option(OptionDraft {
                        present: false,
                        value: None,
                    }),
                ),
                (Some(value), None) => {
                    self.conform_dynamic_to_schema(value, &element, set.syntax())?
                }
                (None, None) => unreachable!("absence selects an optional element"),
            };
            if self.schema_draft_of(input) != element {
                return Err(SourceSemanticError {
                    code: "source-semantics/incompatible-set-element-kind",
                    message: "set literal elements require one exact kind".to_owned(),
                    anchor: SourceSemanticAnchor::for_node(set.syntax()),
                });
            }
            inputs.push(input);
        }
        let resolved = ResolvedType::from_schema_body(&element.body, &element.dimension_parameters)
            .map_err(|error| {
                internal(
                    SourceSemanticAnchor::for_node(set.syntax()),
                    format!("invalid set element schema: {error}"),
                )
            })?;
        if !resolved.satisfies(BuiltinKindPredicate::Keyable) {
            return Err(SourceSemanticError {
                code: "source-semantics/non-keyable-set-element-kind",
                message: "set literal elements require a keyable kind".to_owned(),
                anchor: SourceSemanticAnchor::for_node(set.syntax()),
            });
        }
        let mut parameters = Vec::new();
        let element = embed_schema_draft(
            &element,
            &mut parameters,
            SourceSemanticAnchor::for_node(set.syntax()),
        )?;
        Ok(self.emit_with_schema_draft(
            "set/define",
            inputs,
            SchemaDraft {
                dimension_parameters: parameters.into_boxed_slice(),
                body: SchemaBody::Set {
                    element: Box::new(element),
                    cardinality: CardinalitySpec::Exact(DimensionExpr::Constant(
                        set.items().len() as u64
                    )),
                },
            },
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
        if let [value] = inputs.as_slice() {
            return Ok(*value);
        }
        let mut parameters = Vec::new();
        let items = inputs
            .iter()
            .map(|value| {
                embed_schema_draft(
                    &self.schema_draft_of(*value),
                    &mut parameters,
                    SourceSemanticAnchor::for_node(tuple.syntax()),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(self.emit_with_schema_draft(
            "source/tuple",
            inputs,
            SchemaDraft {
                dimension_parameters: parameters.into_boxed_slice(),
                body: SchemaBody::Tuple(items.into_boxed_slice()),
            },
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
            value = self.select(value, &item)?;
        }
        Ok(value)
    }

    fn select(
        &mut self,
        source: PendingValue,
        item: &SubscriptItemSyntax,
    ) -> Result<PendingValue, SourceSemanticError> {
        let selectors = match item {
            SubscriptItemSyntax::Bracket(value) => value.values(),
            SubscriptItemSyntax::Brace(value) => value.values(),
            SubscriptItemSyntax::Dot(value) => {
                let field =
                    self.required(value.identifier(), value.syntax(), "a selected field")?;
                let name = node_text(field.syntax())?;
                let mut schema = self.schema_draft_of(source);
                let body = match &schema.body {
                    SchemaBody::Record(fields) => fields
                        .iter()
                        .find(|field| field.name == name)
                        .map(|field| field.schema.clone()),
                    SchemaBody::Table { columns, rows } => {
                        let element = columns
                            .iter()
                            .find(|column| column.name == name)
                            .map(|column| column.schema.clone());
                        let rows = match rows {
                            CardinalitySpec::Exact(rows) => rows.clone(),
                            CardinalitySpec::Dynamic { upper_bound } => {
                                let id = DimensionParameterId::new(
                                    schema.dimension_parameters.len() as u32,
                                );
                                let mut parameters = schema.dimension_parameters.into_vec();
                                parameters.push(DimensionParameterDeclaration {
                                    id,
                                    origin: DimensionParameterOrigin::Inferred,
                                    lifetime: DimensionLifetime::Turn,
                                    lower_bound: DimensionExpr::Constant(0),
                                    upper_bound: upper_bound.clone(),
                                });
                                schema.dimension_parameters = parameters.into_boxed_slice();
                                DimensionExpr::Parameter(id)
                            }
                        };
                        element.map(|element| SchemaBody::Matrix {
                            element: Box::new(element),
                            dimensions: vec![rows, DimensionExpr::Constant(1)].into_boxed_slice(),
                        })
                    }
                    SchemaBody::Dynamic => Some(SchemaBody::Dynamic),
                    _ => None,
                }
                .ok_or_else(|| SourceSemanticError {
                    code: "source-semantics/unknown-selected-field",
                    message: format!("source has no field {name}"),
                    anchor: SourceSemanticAnchor::for_node(item.syntax()),
                })?;
                let selector = self.constant_exact(
                    SchemaBody::Id,
                    ValueDataDraft::Id(mech_core::hash_str(&name)),
                );
                return Ok(self.emit_with_schema_draft(
                    "access/column",
                    vec![source, selector],
                    SchemaDraft { body, ..schema },
                    item.syntax(),
                    "slice",
                    None,
                ));
            }
            SubscriptItemSyntax::DotInteger(value) => {
                let integer =
                    self.required(value.integer(), value.syntax(), "a selected ordinal")?;
                let text = node_text(integer.syntax())?;
                let (schema, data) = decode_number(&text, None).ok_or_else(|| {
                    missing_kind_child(value.syntax(), "a valid selected ordinal")
                })?;
                let selector = self.constant(schema, data);
                return self.select_values(source, vec![Some(selector)], item.syntax());
            }
            SubscriptItemSyntax::Swizzle(value) => {
                let inputs = value
                    .identifiers()
                    .map(|name| node_text(name.syntax()))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .map(|name| self.constant(BuiltinSchema::String, ValueDataDraft::String(name)))
                    .collect::<Vec<_>>();
                let mut all = vec![source];
                all.extend(inputs);
                return Ok(self.emit(
                    "access/swizzle",
                    all,
                    BuiltinSchema::Dynamic,
                    item.syntax(),
                    "slice",
                    None,
                ));
            }
        };
        let selectors = selectors
            .iter()
            .map(|selector| match selector {
                SubscriptValueSyntax::SelectAll(_) => Ok(None),
                _ => self.subscript_value(selector).map(Some),
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.select_values(source, selectors, item.syntax())
    }

    fn select_values(
        &mut self,
        source: PendingValue,
        selectors: Vec<Option<PendingValue>>,
        syntax: &SyntaxNode,
    ) -> Result<PendingValue, SourceSemanticError> {
        if selectors.is_empty() || selectors.len() > 2 {
            return Err(SourceSemanticError {
                code: "source-semantics/invalid-selection-arity",
                message: "selection requires one or two selectors".to_owned(),
                anchor: SourceSemanticAnchor::for_node(syntax),
            });
        }
        if selectors.iter().all(Option::is_none) {
            return Ok(source);
        }
        let mut parameters = Vec::new();
        let body = embed_schema_draft(
            &self.schema_draft_of(source),
            &mut parameters,
            SourceSemanticAnchor::for_node(syntax),
        )?;
        let mut inputs = vec![source];
        let mut counts = Vec::new();
        let mut scalar = Vec::new();
        for selector in &selectors {
            match selector {
                None => {
                    counts.push(None);
                    scalar.push(false);
                }
                Some(value) => {
                    let selector = embed_schema_draft(
                        &self.schema_draft_of(*value),
                        &mut parameters,
                        SourceSemanticAnchor::for_node(syntax),
                    )?;
                    let logical = matches!(&selector, SchemaBody::Bool)
                        || matches!(&selector, SchemaBody::Matrix { element, .. } if element.as_ref() == &SchemaBody::Bool);
                    let (count, is_scalar) = match selector {
                        SchemaBody::Matrix { dimensions, .. } => {
                            (DimensionExpr::Multiply(dimensions), false)
                        }
                        _ => (DimensionExpr::Constant(1), !logical),
                    };
                    // A logical selector's extent bounds its population; it does
                    // not determine how many source elements are selected.
                    let count = if logical {
                        let id = DimensionParameterId::new(parameters.len() as u32);
                        parameters.push(DimensionParameterDeclaration {
                            id,
                            origin: DimensionParameterOrigin::Inferred,
                            lifetime: DimensionLifetime::Turn,
                            lower_bound: DimensionExpr::Constant(0),
                            upper_bound: Some(count),
                        });
                        DimensionExpr::Parameter(id)
                    } else {
                        count
                    };
                    counts.push(Some(count));
                    scalar.push(is_scalar);
                    inputs.push(*value);
                }
            }
        }
        let name = match selectors.as_slice() {
            [Some(_), None] => "access/rows",
            [None, Some(_)] => "access/columns",
            [Some(_), Some(_)] if !scalar.iter().all(|scalar| *scalar) => "access/rectangle",
            _ if scalar.iter().all(|scalar| *scalar) => "access/scalar",
            _ => "access/range",
        };
        let output = match body {
            SchemaBody::Matrix {
                element,
                dimensions,
            } if dimensions.len() == 2 => {
                if scalar.iter().all(|scalar| *scalar) {
                    *element
                } else {
                    let dimensions = if selectors.len() == 1 {
                        vec![
                            counts[0]
                                .clone()
                                .unwrap_or_else(|| DimensionExpr::Multiply(dimensions)),
                            DimensionExpr::Constant(1),
                        ]
                    } else {
                        counts
                            .into_iter()
                            .zip(dimensions.iter())
                            .map(|(count, source)| count.unwrap_or_else(|| source.clone()))
                            .collect()
                    };
                    SchemaBody::Matrix {
                        element,
                        dimensions: dimensions.into_boxed_slice(),
                    }
                }
            }
            SchemaBody::Dynamic => SchemaBody::Dynamic,
            SchemaBody::String => SchemaBody::String,
            SchemaBody::Map { value, .. } => *value,
            SchemaBody::Tuple(items) if selectors.len() == 1 && scalar[0] => {
                let selected = self
                    .constant_selection_ordinal(selectors[0].expect("nonempty scalar selector"))
                    .and_then(|ordinal| ordinal.checked_sub(1))
                    .and_then(|ordinal| usize::try_from(ordinal).ok())
                    .and_then(|ordinal| items.get(ordinal).cloned());
                selected.or_else(|| items.first().filter(|first| items.iter().all(|item| item == *first)).cloned())
                    .ok_or_else(|| SourceSemanticError {
                        code: "source-semantics/unresolved-tuple-selection-kind",
                        message: "a heterogeneous tuple requires a valid constant ordinal to determine its selected kind".to_owned(),
                        anchor: SourceSemanticAnchor::for_node(syntax),
                    })?
            }
            _ => {
                return Err(SourceSemanticError {
                    code: "source-semantics/invalid-selection-source",
                    message: "source kind does not support this selection".to_owned(),
                    anchor: SourceSemanticAnchor::for_node(syntax),
                });
            }
        };
        Ok(self.emit_with_schema_draft(
            name,
            inputs,
            SchemaDraft {
                body: output,
                dimension_parameters: parameters.into_boxed_slice(),
            },
            syntax,
            "slice",
            None,
        ))
    }

    fn constant_selection_ordinal(&self, value: PendingValue) -> Option<u64> {
        let PendingValue::Constant(index) = value else {
            return None;
        };
        let mut schemas = SchemaTableBuilder::new();
        let pending = schemas
            .insert(self.schema_draft_of(value).finalize().ok()?)
            .ok()?;
        let build = schemas.finish().ok()?;
        let schema = build.resolve(pending).ok()?;
        let (schemas, _) = build.into_parts();
        let value = ValueDraft {
            schema,
            shape_values: Box::new([]),
            data: self.constants[index].data.clone(),
        }
        .finalize(&SnapshotValidationContext::new(&schemas))
        .ok()?;
        mech_core::canonical_positional_ordinal(value.data()).ok()
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
                        let item_schema = match self.schema_draft_of(source) {
                            SchemaDraft {
                                dimension_parameters,
                                body:
                                    SchemaBody::Set { element, .. } | SchemaBody::Matrix { element, .. },
                            } => Some(SchemaDraft {
                                dimension_parameters,
                                body: *element,
                            }),
                            _ => None,
                        };
                        self.bind_pattern(&pattern, source, item_schema.as_ref())?;
                        qualifier_layouts.push((
                            inputs.len() as u32,
                            SourceSemanticComprehensionQualifierRole::Generator {
                                pattern: pattern.index,
                            },
                        ));
                        inputs.push(source);
                        inputs.extend(pattern.dependencies.iter().copied());
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
                        let filter_value = self.expression(&filter)?.0;
                        inputs.push(self.require_boolean_operand(filter_value, filter.syntax())?);
                    }
                }
            }
            let result = self.required(result, syntax, "a comprehension result")?;
            let result = self.expression(&result)?.0;
            let value = if self.is_genuinely_dynamic(result) {
                inputs.push(result);
                self.emit(
                    operation,
                    inputs,
                    BuiltinSchema::Dynamic,
                    syntax,
                    "comprehension",
                    None,
                )
            } else {
                let Some((resolved_inputs, schema)) =
                    self.resolve_maintained_call(operation, vec![result], syntax)?
                else {
                    return Err(internal(
                        SourceSemanticAnchor::for_node(syntax),
                        format!("{operation} has no maintained type declaration"),
                    ));
                };
                inputs.push(resolved_inputs[0]);
                self.emit_with_schema_draft(
                    operation,
                    inputs,
                    schema,
                    syntax,
                    "comprehension",
                    None,
                )
            };
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
        collect_pattern_bindings(pattern, &mut bindings)?;
        let mut seen = BTreeSet::new();
        bindings.retain(|binding| seen.insert(binding.name.clone()));
        let dependencies = self.compile_pattern_dependencies(pattern)?;
        let index = self.patterns.len() as u32;
        self.patterns.push(SourceSemanticPattern {
            source: node_text(pattern.syntax())?,
            bindings: bindings
                .iter()
                .map(|binding| binding.name.clone())
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            anchor: SourceSemanticAnchor::for_node(pattern.syntax()),
        });
        Ok(RecordedPattern {
            index,
            bindings,
            dependencies,
            syntax: pattern.syntax().clone(),
        })
    }

    fn compile_pattern_dependencies(
        &mut self,
        pattern: &PatternSyntax,
    ) -> Result<Vec<PendingValue>, SourceSemanticError> {
        let value = self.required(pattern.value(), pattern.syntax(), "a pattern body")?;
        let mut dependencies = Vec::new();
        match value {
            PatternValueSyntax::Expression(expression) => {
                if standalone_pattern_variable(&expression).is_none() {
                    dependencies.push(self.expression(&expression)?.0);
                }
            }
            PatternValueSyntax::Array(array) => {
                for element in array.elements() {
                    if let Some(pattern) = element.pattern() {
                        dependencies.extend(self.compile_pattern_dependencies(&pattern)?);
                    }
                }
            }
            PatternValueSyntax::Tuple(tuple) => {
                for pattern in tuple.items() {
                    dependencies.extend(self.compile_pattern_dependencies(&pattern)?);
                }
            }
            PatternValueSyntax::AtomStruct(tuple) => {
                for pattern in tuple.items() {
                    dependencies.extend(self.compile_pattern_dependencies(&pattern)?);
                }
            }
            PatternValueSyntax::TupleStruct(tuple) => {
                for pattern in tuple.items() {
                    dependencies.extend(self.compile_pattern_dependencies(&pattern)?);
                }
            }
            PatternValueSyntax::Wildcard(_) => {}
        }
        Ok(dependencies)
    }

    fn bind_pattern(
        &mut self,
        pattern: &RecordedPattern,
        source: PendingValue,
        element_schema: Option<&SchemaDraft>,
    ) -> Result<(), SourceSemanticError> {
        let source_schema = element_schema
            .cloned()
            .unwrap_or_else(|| self.schema_draft_of(source));
        let unresolved_source = match source {
            PendingValue::Input(index) => !self
                .input_declarations
                .contains_key(&self.inputs[index as usize].name),
            PendingValue::Node(index) => self.nodes[index as usize].inferable_projection,
            _ => false,
        };
        for (binding_index, binding) in pattern.bindings.iter().enumerate() {
            let mut inferred = source_schema.clone();
            for index in &binding.path {
                inferred.body = match &inferred.body {
                    SchemaBody::Tuple(items) => items.get(*index).cloned(),
                    SchemaBody::Matrix { element, .. } => Some(*element.clone()),
                    SchemaBody::Dynamic => Some(SchemaBody::Dynamic),
                    _ => None,
                }
                .ok_or_else(|| SourceSemanticError {
                    code: "source-semantics/incompatible-pattern-shape",
                    message: format!(
                        "pattern projection {:?} does not exist in its source",
                        binding.path
                    ),
                    anchor: SourceSemanticAnchor::for_node(&pattern.syntax),
                })?;
            }
            if let Some(expected) = &binding.schema {
                if is_dynamic_schema_draft(&inferred) {
                    inferred = expected.clone();
                } else if inferred != *expected && !is_dynamic_schema_draft(expected) {
                    return Err(SourceSemanticError {
                        code: "source-semantics/incompatible-local-kind",
                        message: "pattern projection does not satisfy its kind annotation"
                            .to_owned(),
                        anchor: SourceSemanticAnchor::for_node(&pattern.syntax),
                    });
                }
            }
            let detail = Some(format!(
                "pattern={};binding={binding_index};name={};path={:?}",
                pattern.index, binding.name, binding.path
            ));
            let projection = self.emit_with_schema_draft(
                "source/bind",
                vec![source],
                inferred,
                &pattern.syntax,
                "pattern-binding",
                detail,
            );
            if unresolved_source
                && binding.schema.is_none()
                && self.is_genuinely_dynamic(projection)
            {
                let PendingValue::Node(index) = projection else {
                    unreachable!("emitted projection")
                };
                self.nodes[index as usize].inferable_projection = true;
            }
            self.bindings.insert(binding.name.clone(), projection);
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
            let pattern = self.record_pattern(&pattern)?;
            inputs.extend(pattern.dependencies);
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
        let schema = self
            .input_declarations
            .get(&name)
            .cloned()
            .unwrap_or_else(dynamic_schema_draft);
        self.input_by_name.insert(name.clone(), index);
        self.inputs.push(PendingInput {
            name,
            schema,
            anchor: SourceSemanticAnchor::for_node(node),
        });
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
            PendingValue::Input(index) => {
                builtin_schema_for_annotation_body(&self.inputs[index as usize].schema.body)
                    .unwrap_or(BuiltinSchema::Dynamic)
            }
            PendingValue::State(index) => self.states[index as usize].schema,
            PendingValue::Node(index) => self.nodes[index as usize].schema,
        }
    }

    fn schema_body_of(&self, value: PendingValue) -> SchemaBody {
        match value {
            PendingValue::Constant(index) => self.constants[index]
                .schema_body
                .clone()
                .unwrap_or_else(|| schema_body(self.constants[index].schema)),
            PendingValue::Input(index) => self.inputs[index as usize].schema.body.clone(),
            PendingValue::State(index) => self
                .nodes
                .iter()
                .find(|node| node.state == Some(index))
                .and_then(|node| node.schema_body.clone())
                .unwrap_or_else(|| schema_body(self.states[index as usize].schema)),
            PendingValue::Node(index) => self.nodes[index as usize]
                .schema_body
                .clone()
                .unwrap_or_else(|| schema_body(self.nodes[index as usize].schema)),
        }
    }

    fn schema_draft_of(&self, value: PendingValue) -> SchemaDraft {
        match value {
            PendingValue::Constant(index) => SchemaDraft {
                dimension_parameters: self.constants[index].schema_parameters.clone(),
                body: self.constants[index]
                    .schema_body
                    .clone()
                    .unwrap_or_else(|| schema_body(self.constants[index].schema)),
            },
            PendingValue::Input(index) => self.inputs[index as usize].schema.clone(),
            PendingValue::State(index) => self
                .nodes
                .iter()
                .find(|node| node.state == Some(index))
                .map(|node| SchemaDraft {
                    dimension_parameters: node.schema_parameters.clone(),
                    body: node
                        .schema_body
                        .clone()
                        .unwrap_or_else(|| schema_body(node.schema)),
                })
                .unwrap_or_else(|| SchemaDraft {
                    dimension_parameters: Box::new([]),
                    body: schema_body(self.states[index as usize].schema),
                }),
            PendingValue::Node(index) => SchemaDraft {
                dimension_parameters: self.nodes[index as usize].schema_parameters.clone(),
                body: self.nodes[index as usize]
                    .schema_body
                    .clone()
                    .unwrap_or_else(|| schema_body(self.nodes[index as usize].schema)),
            },
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

    fn apply_resolved_conversion(
        &mut self,
        value: PendingValue,
        target: &ResolvedType,
        plan: &mech_core::ConversionPlan,
        syntax: &SyntaxNode,
    ) -> Result<PendingValue, SourceSemanticError> {
        if let Some(target_schema) = builtin_kind_from_resolved(target).and_then(builtin_schema) {
            return self.apply_conversion(value, target_schema, plan, syntax);
        }
        let target = schema_draft_from_resolved(target, SourceSemanticAnchor::for_node(syntax))?;
        if self.schema_draft_of(value) == target {
            return Ok(value);
        }
        Ok(self.emit_with_schema_draft(
            "convert/kind",
            vec![value],
            target,
            syntax,
            "implicit-conversion",
            Some(format!("target={}", plan.target.semantic_name())),
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
                return Ok(self.emit(
                    "option/some",
                    vec![value],
                    expected,
                    syntax,
                    "present-option",
                    Some("presence=present".to_owned()),
                ));
            };
            let data = self.constants[index].data.clone();
            if expected == BuiltinSchema::OptionDynamic {
                let schema = self.constants[index].schema;
                let schema_body = self.constants[index].schema_body.clone();
                if schema != BuiltinSchema::Dynamic || schema_body.is_some() {
                    return Ok(self.constant_dynamic_option(schema, schema_body, data));
                }
            }
            return Ok(self.constant(
                expected,
                ValueDataDraft::Option(OptionDraft {
                    present: true,
                    value: Some(Box::new(data)),
                }),
            ));
        }
        if actual == BuiltinSchema::Dynamic {
            if !matches!(self.schema_body_of(value), SchemaBody::Dynamic) {
                return Err(SourceSemanticError {
                    code,
                    message: message.to_owned(),
                    anchor: SourceSemanticAnchor::for_node(syntax),
                });
            }
            return self.conform_dynamic_to_schema(
                value,
                &SchemaDraft {
                    body: schema_body(expected),
                    dimension_parameters: Box::new([]),
                },
                syntax,
            );
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

    fn conform_schema_draft(
        &mut self,
        value: PendingValue,
        expected: &SchemaDraft,
        syntax: &SyntaxNode,
        code: &'static str,
        message: &str,
    ) -> Result<PendingValue, SourceSemanticError> {
        if is_dynamic_schema_draft(expected)
            || self.schema_draft_of(value) == *expected
            || schema_annotation_accepts(&self.schema_draft_of(value).body, &expected.body)
        {
            return Ok(value);
        }
        if let Some(expected) = builtin_schema_for_annotation_body(&expected.body) {
            return self.conform_value(value, expected, syntax, code, message);
        }
        let actual = self.schema_draft_of(value);
        let actual_type =
            ResolvedType::from_schema_body(&actual.body, &actual.dimension_parameters).map_err(
                |error| internal(SourceSemanticAnchor::for_node(syntax), error.to_string()),
            )?;
        let expected_type =
            ResolvedType::from_schema_body(&expected.body, &expected.dimension_parameters)
                .map_err(|error| {
                    internal(SourceSemanticAnchor::for_node(syntax), error.to_string())
                })?;
        let plan =
            plan_explicit_cast(&actual_type, &expected_type).map_err(|_| SourceSemanticError {
                code,
                message: message.to_owned(),
                anchor: SourceSemanticAnchor::for_node(syntax),
            })?;
        self.apply_resolved_conversion(value, &expected_type, &plan, syntax)
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

    fn conform_exact_table_value(
        &self,
        value: PendingValue,
        expected: &SchemaDraft,
        field: &str,
        syntax: &SyntaxNode,
    ) -> Result<PendingValue, SourceSemanticError> {
        if self.schema_draft_of(value) == *expected {
            return Ok(value);
        }
        Err(SourceSemanticError {
            code: "source-semantics/incompatible-table-field-kind",
            message: format!("table field {field} does not have one exact inferred kind"),
            anchor: SourceSemanticAnchor::for_node(syntax),
        })
    }

    fn constant(&mut self, schema: BuiltinSchema, data: ValueDataDraft) -> PendingValue {
        let index = self.constants.len();
        self.constants.push(PendingConstant {
            schema,
            schema_body: None,
            schema_parameters: Box::new([]),
            data,
            dynamic_payload: None,
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
            dynamic_payload: None,
        });
        PendingValue::Constant(index)
    }

    fn constant_dynamic_option(
        &mut self,
        payload_schema: BuiltinSchema,
        payload_schema_body: Option<SchemaBody>,
        payload: ValueDataDraft,
    ) -> PendingValue {
        let index = self.constants.len();
        self.constants.push(PendingConstant {
            schema: BuiltinSchema::OptionDynamic,
            schema_body: None,
            schema_parameters: Box::new([]),
            data: ValueDataDraft::Option(OptionDraft {
                present: true,
                value: None,
            }),
            dynamic_payload: Some((payload_schema, payload_schema_body, payload)),
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
            inferable_projection: false,
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

    fn emit_with_schema_draft(
        &mut self,
        operation: &str,
        inputs: Vec<PendingValue>,
        schema: SchemaDraft,
        syntax: &SyntaxNode,
        role: &'static str,
        detail: Option<String>,
    ) -> PendingValue {
        let builtin = builtin_schema_for_body(&schema.body).unwrap_or(BuiltinSchema::Dynamic);
        let value = self.emit(operation, inputs, builtin, syntax, role, detail);
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
        let schemas =
            BuiltinSchemas::build(self.anchor, &self.inputs, &self.nodes, &self.constants)?;
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
            let data = match constant.dynamic_payload {
                Some((payload_schema, _, payload)) => ValueDataDraft::Option(OptionDraft {
                    present: true,
                    value: Some(Box::new(ValueDataDraft::Dynamic(Some(Box::new(
                        ValueDraft {
                            schema: schemas.dynamic_payload_id(index, payload_schema),
                            shape_values: Box::new([]),
                            data: payload,
                        },
                    ))))),
                }),
                None => constant.data,
            };
            let value = ValueDraft {
                schema,
                shape_values: Box::new([]),
                data,
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
            .enumerate()
            .map(|(index, input)| SourceInput {
                name: input.name.clone(),
                schema: schemas.input_id(index),
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
                    matches!(node.schema_body.as_ref(), Some(SchemaBody::Matrix { .. })),
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
        let state_initializers = self
            .states
            .iter()
            .map(|state| match state.initializer {
                PendingValue::Constant(index) => {
                    SourceStateInitializer::Constant(constant_ids[index])
                }
                value => SourceStateInitializer::Deferred(resolve_value(value, &constant_ids)),
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let source_map = SourceSemanticMap {
            inputs: self
                .inputs
                .into_iter()
                .map(|input| input.anchor)
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
                        schema: schemas.node_id(state.producer_node as usize, state.schema),
                        initializer: match state.initializer {
                            PendingValue::Constant(index) => Some(constant_ids[index]),
                            _ => None,
                        },
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
            state_initializers,
        })
    }
}

fn pending_schema(
    value: PendingValue,
    constants: &[SchemaId],
    inputs: &[PendingInput],
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
            .map(|_| schemas.input_id(index as usize))
            .unwrap_or_else(|| schemas.id(BuiltinSchema::Dynamic)),
        PendingValue::State(index) => nodes
            .iter()
            .enumerate()
            .find(|(_, node)| node.state == Some(index))
            .map(|(node_index, node)| schemas.node_id(node_index, node.schema))
            .unwrap_or_else(|| schemas.id(BuiltinSchema::Dynamic)),
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
    matrix_output: bool,
) -> Option<OperationContractDeclaration> {
    let name = operation.canonical_name();
    if let Some(contract) =
        mech_core::maintained_operation_contract(&name, input_count, matrix_output)
    {
        return Some(contract);
    }
    match name.as_str() {
        "range/inclusive" => Some(range_contract(input_count, "inclusive-output")),
        "range/exclusive" => Some(range_contract(input_count, "exclusive-output")),
        "range/inclusive-increment" => {
            Some(range_contract(input_count, "inclusive-increment-output"))
        }
        "range/exclusive-increment" => {
            Some(range_contract(input_count, "exclusive-increment-output"))
        }
        "math/neg" => Some(negation_contract(output_schema)),
        "matrix/transpose" => Some(transpose_contract()),
        "matrix/literal" => Some(crate::matrix_literal_contract(input_count)),
        "string/concat" => Some(operation_contract(input_count, output_schema, state_output)),
        "math/add" | "math/sub" | "math/mul" | "math/div" | "math/mod" | "math/pow"
        | "compare/neq" | "compare/eq" | "compare/sneq" | "compare/seq" | "compare/gt"
        | "compare/lt" | "compare/gte" | "compare/lte" => {
            Some(elementwise_contract(input_count, output_schema))
        }
        "logic/or" | "logic/and" | "logic/not" | "logic/xor" => {
            Some(elementwise_contract(input_count, output_schema))
        }
        _ if name.starts_with("math/")
            && input_count == 1
            && mech_core::maintained_source_type_declaration(&name).is_ok() =>
        {
            Some(negation_contract(output_schema))
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

fn transpose_contract() -> OperationContractDeclaration {
    OperationContractDeclaration {
        inputs: read_inputs(1),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::TransposeOf { input: 0 },
            },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    }
}

fn is_scalar_schema(schema: BuiltinSchema) -> bool {
    matches!(
        schema,
        BuiltinSchema::Bool
            | BuiltinSchema::String
            | BuiltinSchema::Index
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
            | BuiltinSchema::C32
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

fn elementwise_contract(
    input_count: usize,
    output_schema: BuiltinSchema,
) -> OperationContractDeclaration {
    OperationContractDeclaration {
        inputs: read_inputs(input_count),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
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

fn annotation_kind_expr(
    annotation: &KindAnnotationSyntax,
) -> Result<(KindExpr, Box<[DimensionParameterDeclaration]>), SourceSemanticError> {
    let mut dimensions = DimensionEnvironmentBuilder::new();
    let kind = annotation_kind_expr_with(annotation, &mut dimensions)?;
    Ok((kind, dimensions.into_declarations()))
}

fn annotation_kind_expr_with(
    annotation: &KindAnnotationSyntax,
    dimensions: &mut DimensionEnvironmentBuilder,
) -> Result<KindExpr, SourceSemanticError> {
    let kind = annotation
        .kind()
        .ok_or_else(|| missing_kind_child(annotation.syntax(), "kind annotation"))?;
    kind_with_option_expr(&kind, dimensions)
}

fn kind_with_option_expr(
    kind: &mech_syntax::document::KindWithOptionSyntax,
    dimensions: &mut DimensionEnvironmentBuilder,
) -> Result<KindExpr, SourceSemanticError> {
    let inner = kind
        .kind()
        .ok_or_else(|| missing_kind_child(kind.syntax(), "optional kind"))?;
    let inner = kind_expr(&inner, dimensions)?;
    Ok(if kind.question_mark().is_some() {
        KindExpr::Option(Box::new(inner))
    } else {
        inner
    })
}

fn kind_expr(
    kind: &KindSyntax,
    dimensions: &mut DimensionEnvironmentBuilder,
) -> Result<KindExpr, SourceSemanticError> {
    let value = kind
        .value()
        .ok_or_else(|| missing_kind_child(kind.syntax(), "kind"))?;
    let anchor = SourceSemanticAnchor::for_node(value.syntax());
    Ok(match value {
        KindValueSyntax::Any(_) => KindExpr::Wildcard,
        KindValueSyntax::Empty(_) => KindExpr::Never,
        KindValueSyntax::Nested(nested) => KindExpr::TypeOf(Box::new(kind_with_option_expr(
            &nested
                .kind()
                .ok_or_else(|| missing_kind_child(nested.syntax(), "nested kind"))?,
            dimensions,
        )?)),
        KindValueSyntax::Atom(atom) => {
            let name = atom
                .name()
                .ok_or_else(|| missing_kind_child(atom.syntax(), "atom kind name"))?;
            let source = node_text(name.syntax())?;
            let path = CanonicalNominalPath::new(
                source
                    .trim_start_matches(':')
                    .split('/')
                    .filter(|segment| !segment.is_empty())
                    .map(str::to_owned)
                    .collect::<Vec<_>>(),
            )
            .map_err(|error| internal(anchor, format!("invalid atom kind path: {error:?}")))?;
            KindExpr::Atom(NominalKey::from_path(NominalKind::Atom, &path))
        }
        KindValueSyntax::Scalar(scalar) => {
            if scalar.constraint().is_some() {
                return Err(SourceSemanticError {
                    code: "source-semantics/unsupported-kind-constraint",
                    message: "reified scalar range constraints are not representable".to_owned(),
                    anchor,
                });
            }
            let name = scalar
                .name()
                .ok_or_else(|| missing_kind_child(scalar.syntax(), "scalar kind name"))?;
            match node_text(name.syntax())?.as_str() {
                "id" => KindExpr::Id,
                "ix" | "index" => KindExpr::Index,
                name => builtin_kind_named(name)
                    .map(BuiltinScalarKind::kind_expr)
                    .ok_or_else(|| SourceSemanticError {
                        code: "source-semantics/unsupported-kind-value",
                        message: format!("unknown scalar kind {name:?}"),
                        anchor,
                    })?,
            }
        }
        KindValueSyntax::Map(map) => KindExpr::Map {
            key: Box::new(kind_expr(
                &map.key()
                    .ok_or_else(|| missing_kind_child(map.syntax(), "map key kind"))?,
                dimensions,
            )?),
            value: Box::new(kind_expr(
                &map.value()
                    .ok_or_else(|| missing_kind_child(map.syntax(), "map value kind"))?,
                dimensions,
            )?),
            cardinality: inferred_kind_dimension(dimensions, anchor)?,
        },
        KindValueSyntax::Set(set) => KindExpr::Set {
            element: Box::new(kind_expr(
                &set.element()
                    .ok_or_else(|| missing_kind_child(set.syntax(), "set element kind"))?,
                dimensions,
            )?),
            cardinality: set
                .literal_constraint()
                .as_ref()
                .map(kind_dimension)
                .transpose()?
                .map_or_else(
                    || inferred_kind_dimension(dimensions, anchor),
                    |dimension| Ok(dimension),
                )?,
        },
        KindValueSyntax::Matrix(matrix) => {
            let element = matrix
                .element()
                .ok_or_else(|| missing_kind_child(matrix.syntax(), "matrix element kind"))?;
            let extents = matrix
                .dimensions()
                .iter()
                .map(kind_dimension)
                .collect::<Result<Vec<_>, _>>()?;
            if extents.is_empty() {
                return Err(SourceSemanticError {
                    code: "source-semantics/unsupported-kind-value",
                    message: "reified matrix kinds require explicit dimensions".to_owned(),
                    anchor,
                });
            }
            KindExpr::Matrix {
                element: Box::new(kind_with_option_expr(&element, dimensions)?),
                dimensions: extents.into_boxed_slice(),
            }
        }
        KindValueSyntax::Tuple(tuple) => KindExpr::Tuple(
            tuple
                .items()
                .iter()
                .map(|kind| kind_expr(kind, dimensions))
                .collect::<Result<Vec<_>, _>>()?
                .into_boxed_slice(),
        ),
        KindValueSyntax::Record(record) => {
            let names = record.fields();
            let kinds = record.field_kinds();
            if names.len() != kinds.len() {
                return Err(missing_kind_child(record.syntax(), "record field kind"));
            }
            KindExpr::Record(
                names
                    .iter()
                    .zip(&kinds)
                    .map(|(name, kind)| {
                        Ok(KindField {
                            name: node_text(name.syntax())?,
                            kind: annotation_kind_expr_with(kind, dimensions)?,
                        })
                    })
                    .collect::<Result<Vec<_>, SourceSemanticError>>()?
                    .into_boxed_slice(),
            )
        }
        KindValueSyntax::Table(table) => {
            let names = table.field_names();
            let kinds = table.field_kinds();
            if names.len() != kinds.len() {
                return Err(missing_kind_child(table.syntax(), "table field kind"));
            }
            KindExpr::Table {
                columns: names
                    .iter()
                    .zip(&kinds)
                    .map(|(name, kind)| {
                        Ok(KindField {
                            name: node_text(name.syntax())?,
                            kind: annotation_kind_expr_with(kind, dimensions)?,
                        })
                    })
                    .collect::<Result<Vec<_>, SourceSemanticError>>()?
                    .into_boxed_slice(),
                rows: table
                    .constraint()
                    .as_ref()
                    .map(kind_dimension)
                    .transpose()?
                    .map_or_else(
                        || inferred_kind_dimension(dimensions, anchor),
                        |dimension| Ok(dimension),
                    )?,
            }
        }
    })
}

fn inferred_kind_dimension(
    dimensions: &mut DimensionEnvironmentBuilder,
    anchor: SourceSemanticAnchor,
) -> Result<DimensionExpr, SourceSemanticError> {
    dimensions
        .declare(
            DimensionParameterOrigin::Inferred,
            DimensionLifetime::Activation,
            DimensionExpr::Constant(0),
            None,
        )
        .map(DimensionExpr::Parameter)
        .map_err(|error| internal(anchor, format!("unable to declare kind extent: {error:?}")))
}

fn annotation_schema(
    annotation: &KindAnnotationSyntax,
) -> Result<BuiltinSchema, SourceSemanticError> {
    let draft = annotation_schema_draft(annotation)?;
    builtin_schema_for_annotation_body(&draft.body).ok_or_else(|| SourceSemanticError {
        code: "source-semantics/unsupported-kind-annotation",
        message: "this value position requires a builtin scalar kind annotation".to_owned(),
        anchor: SourceSemanticAnchor::for_node(annotation.syntax()),
    })
}

fn builtin_schema_for_annotation_body(body: &SchemaBody) -> Option<BuiltinSchema> {
    match body {
        SchemaBody::Dynamic => Some(BuiltinSchema::Dynamic),
        SchemaBody::Option(payload) => option_schema(builtin_schema_for_annotation_body(payload)?),
        _ => builtin_schema_for_body(body),
    }
}

fn annotation_schema_draft(
    annotation: &KindAnnotationSyntax,
) -> Result<SchemaDraft, SourceSemanticError> {
    let kind = annotation
        .kind()
        .ok_or_else(|| missing_kind_child(annotation.syntax(), "kind annotation"))?;
    let inner = kind
        .kind()
        .ok_or_else(|| missing_kind_child(kind.syntax(), "optional kind"))?;
    let mut body = kind_schema_body(&inner)?;
    if kind.question_mark().is_some() {
        body = SchemaBody::Option(Box::new(body));
    }
    Ok(SchemaDraft {
        dimension_parameters: Box::new([]),
        body,
    })
}

fn kind_schema_body(kind: &KindSyntax) -> Result<SchemaBody, SourceSemanticError> {
    let value = kind
        .value()
        .ok_or_else(|| missing_kind_child(kind.syntax(), "kind"))?;
    let anchor = SourceSemanticAnchor::for_node(value.syntax());
    Ok(match value {
        KindValueSyntax::Any(_) => SchemaBody::Dynamic,
        KindValueSyntax::Empty(empty) => {
            return Err(SourceSemanticError {
                code: "source-semantics/unsupported-empty-kind-schema",
                message: "the empty kind cannot describe a materialized source input".to_owned(),
                anchor: SourceSemanticAnchor::for_node(empty.syntax()),
            });
        }
        KindValueSyntax::Nested(_) => SchemaBody::ReifiedType,
        KindValueSyntax::Atom(atom) => {
            let name = atom
                .name()
                .ok_or_else(|| missing_kind_child(atom.syntax(), "atom kind name"))?;
            let source = node_text(name.syntax())?;
            let path = CanonicalNominalPath::new(
                source
                    .trim_start_matches(':')
                    .split('/')
                    .filter(|segment| !segment.is_empty())
                    .map(str::to_owned)
                    .collect::<Vec<_>>(),
            )
            .map_err(|error| internal(anchor, format!("invalid atom kind path: {error:?}")))?;
            SchemaBody::Atom(NominalKey::from_path(NominalKind::Atom, &path))
        }
        KindValueSyntax::Scalar(scalar) => {
            if scalar.constraint().is_some() {
                return Err(SourceSemanticError {
                    code: "source-semantics/unsupported-kind-constraint",
                    message: "scalar range constraints require a first-class constrained schema"
                        .to_owned(),
                    anchor,
                });
            }
            let name = scalar
                .name()
                .ok_or_else(|| missing_kind_child(scalar.syntax(), "scalar kind name"))?;
            let name = node_text(name.syntax())?;
            match name.as_str() {
                "ix" | "index" => SchemaBody::Index,
                _ => builtin_kind_named(&name)
                    .map(BuiltinScalarKind::schema_body)
                    .ok_or_else(|| SourceSemanticError {
                        code: "source-semantics/unsupported-kind-annotation",
                        message: format!("unknown builtin scalar kind {name:?}"),
                        anchor,
                    })?,
            }
        }
        KindValueSyntax::Map(map) => SchemaBody::Map {
            key: Box::new(kind_schema_body(
                &map.key()
                    .ok_or_else(|| missing_kind_child(map.syntax(), "map key kind"))?,
            )?),
            value: Box::new(kind_schema_body(
                &map.value()
                    .ok_or_else(|| missing_kind_child(map.syntax(), "map value kind"))?,
            )?),
            cardinality: CardinalitySpec::Dynamic { upper_bound: None },
        },
        KindValueSyntax::Set(set) => SchemaBody::Set {
            element: Box::new(kind_schema_body(
                &set.element()
                    .ok_or_else(|| missing_kind_child(set.syntax(), "set element kind"))?,
            )?),
            cardinality: kind_extent(set.literal_constraint().as_ref())?,
        },
        KindValueSyntax::Matrix(matrix) => {
            let element = matrix
                .element()
                .ok_or_else(|| missing_kind_child(matrix.syntax(), "matrix element kind"))?;
            let element_kind = element
                .kind()
                .ok_or_else(|| missing_kind_child(element.syntax(), "matrix element kind"))?;
            let mut element = kind_schema_body(&element_kind)?;
            if matrix
                .element()
                .is_some_and(|element| element.question_mark().is_some())
            {
                element = SchemaBody::Option(Box::new(element));
            }
            let dimensions = matrix
                .dimensions()
                .iter()
                .map(kind_dimension)
                .collect::<Result<Vec<_>, _>>()?;
            if dimensions.is_empty() {
                return Err(SourceSemanticError {
                    code: "source-semantics/unsupported-kind-annotation",
                    message: "matrix kind annotations require explicit dimensions".to_owned(),
                    anchor,
                });
            }
            SchemaBody::Matrix {
                element: Box::new(element),
                dimensions: dimensions.into_boxed_slice(),
            }
        }
        KindValueSyntax::Tuple(tuple) => SchemaBody::Tuple(
            tuple
                .items()
                .iter()
                .map(kind_schema_body)
                .collect::<Result<Vec<_>, _>>()?
                .into_boxed_slice(),
        ),
        KindValueSyntax::Record(record) => {
            let names = record.fields();
            let kinds = record.field_kinds();
            if names.len() != kinds.len() {
                return Err(missing_kind_child(record.syntax(), "record field kind"));
            }
            SchemaBody::Record(
                names
                    .iter()
                    .zip(&kinds)
                    .map(|(name, kind)| {
                        Ok(SchemaField {
                            name: node_text(name.syntax())?,
                            schema: annotation_schema_draft(kind)?.body,
                        })
                    })
                    .collect::<Result<Vec<_>, SourceSemanticError>>()?
                    .into_boxed_slice(),
            )
        }
        KindValueSyntax::Table(table) => {
            let names = table.field_names();
            let kinds = table.field_kinds();
            if names.len() != kinds.len() {
                return Err(missing_kind_child(table.syntax(), "table field kind"));
            }
            SchemaBody::Table {
                columns: names
                    .iter()
                    .zip(&kinds)
                    .map(|(name, kind)| {
                        Ok(SchemaField {
                            name: node_text(name.syntax())?,
                            schema: annotation_schema_draft(kind)?.body,
                        })
                    })
                    .collect::<Result<Vec<_>, SourceSemanticError>>()?
                    .into_boxed_slice(),
                rows: kind_extent(table.constraint().as_ref())?,
            }
        }
    })
}

fn kind_extent(literal: Option<&LiteralSyntax>) -> Result<CardinalitySpec, SourceSemanticError> {
    literal.map_or(
        Ok(CardinalitySpec::Dynamic { upper_bound: None }),
        |literal| Ok(CardinalitySpec::Exact(kind_dimension(literal)?)),
    )
}

fn kind_dimension(literal: &LiteralSyntax) -> Result<DimensionExpr, SourceSemanticError> {
    let Some(LiteralValueSyntax::Number(number)) = literal.value() else {
        return Err(SourceSemanticError {
            code: "source-semantics/unsupported-kind-dimension",
            message: "kind extents require unsigned integer constants".to_owned(),
            anchor: SourceSemanticAnchor::for_node(literal.syntax()),
        });
    };
    let annotation = literal
        .annotation()
        .map(|annotation| annotation_schema(&annotation))
        .transpose()?;
    let suffix = selected_integer_suffix(&number)?;
    if let Some(kind) = annotation.or(suffix)
        && !matches!(
            kind,
            BuiltinSchema::U8
                | BuiltinSchema::U16
                | BuiltinSchema::U32
                | BuiltinSchema::U64
                | BuiltinSchema::U128
                | BuiltinSchema::I8
                | BuiltinSchema::I16
                | BuiltinSchema::I32
                | BuiltinSchema::I64
                | BuiltinSchema::I128
        )
    {
        return Err(SourceSemanticError {
            code: "source-semantics/unsupported-kind-dimension",
            message: "kind extents require an integer literal kind".to_owned(),
            anchor: SourceSemanticAnchor::for_node(literal.syntax()),
        });
    }
    let source = canonical_number_source(&number)?.replace('_', "");
    if (suffix.is_some() && decode_number(&source, None).is_none())
        || (annotation.is_some() && decode_number(&source, annotation).is_none())
    {
        return Err(SourceSemanticError {
            code: "source-semantics/unsupported-kind-dimension",
            message: "kind extent is not representable in its selected literal kind".to_owned(),
            anchor: SourceSemanticAnchor::for_node(literal.syntax()),
        });
    }
    let (source, _) = numeric_suffix(&source);
    let (negative, magnitude) = integer_parts(source).ok_or_else(|| SourceSemanticError {
        code: "source-semantics/unsupported-kind-dimension",
        message: "kind extents require unsigned integer constants".to_owned(),
        anchor: SourceSemanticAnchor::for_node(literal.syntax()),
    })?;
    if negative {
        return Err(SourceSemanticError {
            code: "source-semantics/unsupported-kind-dimension",
            message: "kind extents require unsigned integer constants".to_owned(),
            anchor: SourceSemanticAnchor::for_node(literal.syntax()),
        });
    }
    u64::try_from(magnitude)
        .map(DimensionExpr::Constant)
        .map_err(|_| SourceSemanticError {
            code: "source-semantics/unsupported-kind-dimension",
            message: "kind extent exceeds the canonical u64 dimension range".to_owned(),
            anchor: SourceSemanticAnchor::for_node(literal.syntax()),
        })
}

fn schema_annotation_accepts(actual: &SchemaBody, expected: &SchemaBody) -> bool {
    fn cardinality(actual: &CardinalitySpec, expected: &CardinalitySpec) -> bool {
        actual == expected || matches!(expected, CardinalitySpec::Dynamic { upper_bound: None })
    }
    if actual == expected || matches!(expected, SchemaBody::Dynamic) {
        return true;
    }
    match (actual, expected) {
        (SchemaBody::Option(a), SchemaBody::Option(b)) => schema_annotation_accepts(a, b),
        (SchemaBody::Tuple(a), SchemaBody::Tuple(b)) => {
            a.len() == b.len()
                && a.iter()
                    .zip(b)
                    .all(|(a, b)| schema_annotation_accepts(a, b))
        }
        (SchemaBody::Record(a), SchemaBody::Record(b)) => {
            a.len() == b.len()
                && b.iter().all(|b| {
                    a.iter()
                        .find(|a| a.name == b.name)
                        .is_some_and(|a| schema_annotation_accepts(&a.schema, &b.schema))
                })
        }
        (
            SchemaBody::Matrix {
                element: a,
                dimensions: ad,
            },
            SchemaBody::Matrix {
                element: b,
                dimensions: bd,
            },
        ) => ad == bd && schema_annotation_accepts(a, b),
        (
            SchemaBody::Set {
                element: a,
                cardinality: ac,
            },
            SchemaBody::Set {
                element: b,
                cardinality: bc,
            },
        ) => cardinality(ac, bc) && schema_annotation_accepts(a, b),
        (
            SchemaBody::Map {
                key: ak,
                value: av,
                cardinality: ac,
            },
            SchemaBody::Map {
                key: bk,
                value: bv,
                cardinality: bc,
            },
        ) => {
            cardinality(ac, bc)
                && schema_annotation_accepts(ak, bk)
                && schema_annotation_accepts(av, bv)
        }
        (
            SchemaBody::Table {
                columns: a,
                rows: ar,
            },
            SchemaBody::Table {
                columns: b,
                rows: br,
            },
        ) => {
            cardinality(ar, br)
                && a.len() == b.len()
                && a.iter().zip(b).all(|(a, b)| {
                    a.name == b.name && schema_annotation_accepts(&a.schema, &b.schema)
                })
        }
        _ => false,
    }
}

fn missing_kind_child(syntax: &SyntaxNode, child: &str) -> SourceSemanticError {
    SourceSemanticError {
        code: "source-semantics/missing-typed-child",
        message: format!("canonical {child} is missing"),
        anchor: SourceSemanticAnchor::for_node(syntax),
    }
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
        ("c32", BuiltinSchema::C32),
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

fn selected_integer_suffix(
    number: &mech_syntax::document::NumberSyntax,
) -> Result<Option<BuiltinSchema>, SourceSemanticError> {
    let Some(real) = number.real() else {
        return Ok(None);
    };
    let Some(value) = real.value() else {
        return Err(SourceSemanticError {
            code: "source-semantics/missing-typed-child",
            message: "number requires a selected real-number form".to_owned(),
            anchor: SourceSemanticAnchor::for_node(number.syntax()),
        });
    };
    let Some(integer) = IntegerLiteralSyntax::cast(value) else {
        return Ok(None);
    };
    let Some(typed) = integer.typed() else {
        return Ok(None);
    };
    let suffix = typed.suffix().ok_or_else(|| SourceSemanticError {
        code: "source-semantics/missing-typed-child",
        message: "typed integer requires a suffix".to_owned(),
        anchor: SourceSemanticAnchor::for_node(typed.syntax()),
    })?;
    let suffix = node_text(suffix.syntax())?;
    let schema = builtin_kind_named(&suffix)
        .filter(|kind| is_numeric(*kind))
        .and_then(builtin_schema)
        .ok_or_else(|| SourceSemanticError {
            code: "source-semantics/unsupported-number-kind-suffix",
            message: format!("typed integer suffix {suffix:?} is not a builtin numeric kind"),
            anchor: SourceSemanticAnchor::for_node(typed.syntax()),
        })?;
    Ok(Some(schema))
}

fn canonical_number_source(
    number: &mech_syntax::document::NumberSyntax,
) -> Result<String, SourceSemanticError> {
    let source = node_text(number.syntax())?;
    let Some(real) = number.real().and_then(|real| real.value()) else {
        return Ok(source);
    };
    let Some(scientific) = ScientificLiteralSyntax::cast(real) else {
        return Ok(source);
    };
    let Some(exponent) = scientific
        .exponent()
        .and_then(IntegerLiteralSyntax::cast)
        .and_then(|integer| integer.typed())
        .and_then(|typed| typed.suffix())
    else {
        return Ok(source);
    };
    let suffix = node_text(exponent.syntax())?;
    source
        .strip_suffix(&suffix)
        .map(str::to_owned)
        .ok_or_else(|| SourceSemanticError {
            code: "source-semantics/invalid-number-literal",
            message: "scientific exponent suffix is not a terminal source component".to_owned(),
            anchor: SourceSemanticAnchor::for_node(number.syntax()),
        })
}

fn integer_parts(source: &str) -> Option<(bool, u128)> {
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
    Some((negative, u128::from_str_radix(digits, radix).ok()?))
}

fn signed_integer_value(source: &str) -> Option<i128> {
    let (negative, magnitude) = integer_parts(source)?;
    if !negative {
        return i128::try_from(magnitude).ok();
    }
    if magnitude == i128::MAX as u128 + 1 {
        return Some(i128::MIN);
    }
    i128::try_from(magnitude).ok()?.checked_neg()
}

fn real_value(source: &str) -> Option<f64> {
    integer_parts(source)
        .map(|(negative, magnitude)| {
            if negative {
                -(magnitude as f64)
            } else {
                magnitude as f64
            }
        })
        .or_else(|| source.parse::<f64>().ok())
}

fn scalar_data(schema: BuiltinSchema, source: &str) -> Option<ValueDataDraft> {
    let signed = || signed_integer_value(source);
    let unsigned = || {
        let (negative, magnitude) = integer_parts(source)?;
        (!negative).then_some(magnitude)
    };
    let float = || real_value(source);
    Some(match schema {
        BuiltinSchema::U8 => ValueDataDraft::U8(u8::try_from(unsigned()?).ok()?),
        BuiltinSchema::U16 => ValueDataDraft::U16(u16::try_from(unsigned()?).ok()?),
        BuiltinSchema::U32 => ValueDataDraft::U32(u32::try_from(unsigned()?).ok()?),
        BuiltinSchema::U64 => ValueDataDraft::U64(u64::try_from(unsigned()?).ok()?),
        BuiltinSchema::U128 => ValueDataDraft::U128(unsigned()?),
        BuiltinSchema::I8 => ValueDataDraft::I8(i8::try_from(signed()?).ok()?),
        BuiltinSchema::I16 => ValueDataDraft::I16(i16::try_from(signed()?).ok()?),
        BuiltinSchema::I32 => ValueDataDraft::I32(i32::try_from(signed()?).ok()?),
        BuiltinSchema::I64 => ValueDataDraft::I64(i64::try_from(signed()?).ok()?),
        BuiltinSchema::I128 => ValueDataDraft::I128(signed()?),
        BuiltinSchema::F32 => {
            let value = float()?;
            let narrowed = value as f32;
            if value.is_finite() && !narrowed.is_finite() {
                return None;
            }
            ValueDataDraft::F32(F32Bits::from_f32(narrowed))
        }
        BuiltinSchema::F64 => ValueDataDraft::F64(F64Bits::from_f64(float()?)),
        BuiltinSchema::C32 => {
            let value = float()?;
            let narrowed = value as f32;
            if value.is_finite() && !narrowed.is_finite() {
                return None;
            }
            ValueDataDraft::Complex32(Complex32Bits::new(
                F32Bits::from_f32(narrowed),
                F32Bits::from_f32(0.0),
            ))
        }
        BuiltinSchema::C64 => ValueDataDraft::Complex64(Complex64Bits::new(
            F64Bits::from_f64(float()?),
            F64Bits::from_f64(0.0),
        )),
        BuiltinSchema::R64 => ValueDataDraft::Rational64 {
            numerator: i64::try_from(signed()?).ok()?,
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
        let real = real_value(real)?;
        let imaginary = real_value(imaginary)?;
        let data = match schema {
            BuiltinSchema::C32 => {
                let real = real as f32;
                let imaginary = imaginary as f32;
                if !real.is_finite() || !imaginary.is_finite() {
                    return None;
                }
                ValueDataDraft::Complex32(Complex32Bits::new(
                    F32Bits::from_f32(real),
                    F32Bits::from_f32(imaginary),
                ))
            }
            BuiltinSchema::C64 => ValueDataDraft::Complex64(Complex64Bits::new(
                F64Bits::from_f64(real),
                F64Bits::from_f64(imaginary),
            )),
            _ => return None,
        };
        return wrap_optional_number(option, schema, data);
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
        let numerator = signed_integer_value(numerator)?;
        let (negative_denominator, denominator) = integer_parts(denominator)?;
        if negative_denominator {
            return None;
        }
        if denominator == 0 {
            return None;
        }
        let divisor = gcd_u128(numerator.unsigned_abs(), denominator);
        let numerator = i64::try_from(numerator / i128::try_from(divisor).ok()?).ok()?;
        let denominator = u64::try_from(denominator / divisor).ok()?;
        return wrap_optional_number(
            option,
            schema,
            ValueDataDraft::Rational64 {
                numerator,
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
    let schema = if !explicitly_based && number.contains(['e', 'E']) {
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

fn gcd_u128(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
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

#[cfg(test)]
#[path = "review_tests.rs"]
mod review_tests;

#[cfg(test)]
#[path = "mask_review_tests.rs"]
mod mask_review_tests;
