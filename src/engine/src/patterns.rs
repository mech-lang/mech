use crate::*;
#[cfg(feature = "enum")]
use mech_core::snapshot::EnumDraft;
use std::collections::HashMap;

// Patterns
// ----------------------------------------------------------------------------

// Pattern matching is split into two phases. Compilation assigns stable indexes
// to bindings and value expressions and validates any structure made known by
// an expected kind. Matching then stages bindings in private storage and only
// returns them after the complete pattern succeeds.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternBindingSpec {
    pub index: usize,
    pub id: u64,
    pub name: String,
    pub schema: Option<SchemaBody>,
}

#[derive(Clone, Debug)]
pub struct PatternBinding {
    pub index: usize,
    pub id: u64,
    pub name: String,
    pub schema: SchemaBody,
    pub value: ValueCell,
}

#[derive(Clone, Debug)]
pub struct PatternMatch {
    pub matched: bool,
    pub bindings: Vec<PatternBinding>,
}

impl PatternMatch {
    fn no_match() -> Self {
        Self {
            matched: false,
            bindings: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CompiledPatternArraySpread {
    pub kind: PatternArraySpreadKind,
    pub binding: Option<Box<CompiledPattern>>,
}

#[derive(Clone, Debug)]
pub enum CompiledPattern {
    Wildcard,
    Binding {
        binding_index: usize,
        id: u64,
        name: String,
        expected_schema: Option<SchemaBody>,
    },
    ExpressionValue {
        expression_index: usize,
        expression: Expression,
    },
    Tuple {
        elements: Vec<CompiledPattern>,
    },
    Array {
        prefix: Vec<CompiledPattern>,
        spread: Option<CompiledPatternArraySpread>,
        suffix: Vec<CompiledPattern>,
    },
    EnumVariant {
        enum_key: Option<NominalKey>,
        variant_name: String,
        payload: Option<Box<CompiledPattern>>,
    },
    AtomTuple {
        tag_key: NominalKey,
        payload: Vec<CompiledPattern>,
    },
}

impl CompiledPattern {
    /// Unique binding slots in stable compiler-assigned order.
    pub fn binding_specs(&self) -> Vec<PatternBindingSpec> {
        fn collect(pattern: &CompiledPattern, specs: &mut Vec<Option<PatternBindingSpec>>) {
            match pattern {
                CompiledPattern::Binding {
                    binding_index,
                    id,
                    name,
                    expected_schema,
                } => {
                    if specs.len() <= *binding_index {
                        specs.resize(*binding_index + 1, None);
                    }
                    match &mut specs[*binding_index] {
                        Some(spec) if spec.schema.is_none() && expected_schema.is_some() => {
                            spec.schema = expected_schema.clone();
                        }
                        Some(_) => {}
                        slot @ None => {
                            *slot = Some(PatternBindingSpec {
                                index: *binding_index,
                                id: *id,
                                name: name.clone(),
                                schema: expected_schema.clone(),
                            });
                        }
                    }
                }
                CompiledPattern::Tuple { elements } => {
                    for element in elements {
                        collect(element, specs);
                    }
                }
                CompiledPattern::Array {
                    prefix,
                    spread,
                    suffix,
                } => {
                    for element in prefix {
                        collect(element, specs);
                    }
                    if let Some(binding) =
                        spread.as_ref().and_then(|spread| spread.binding.as_deref())
                    {
                        collect(binding, specs);
                    }
                    for element in suffix {
                        collect(element, specs);
                    }
                }
                CompiledPattern::EnumVariant { payload, .. } => {
                    if let Some(payload) = payload {
                        collect(payload, specs);
                    }
                }
                CompiledPattern::AtomTuple { payload, .. } => {
                    for element in payload {
                        collect(element, specs);
                    }
                }
                CompiledPattern::Wildcard | CompiledPattern::ExpressionValue { .. } => {}
            }
        }

        let mut specs = Vec::new();
        collect(self, &mut specs);
        specs.into_iter().flatten().collect()
    }

    /// Non-binding expressions in stable compiler-assigned order.
    pub fn expressions(&self) -> Vec<Expression> {
        fn collect(pattern: &CompiledPattern, expressions: &mut Vec<Option<Expression>>) {
            match pattern {
                CompiledPattern::ExpressionValue {
                    expression_index,
                    expression,
                } => {
                    if expressions.len() <= *expression_index {
                        expressions.resize(*expression_index + 1, None);
                    }
                    expressions[*expression_index] = Some(expression.clone());
                }
                CompiledPattern::Tuple { elements } => {
                    for element in elements {
                        collect(element, expressions);
                    }
                }
                CompiledPattern::Array {
                    prefix,
                    spread,
                    suffix,
                } => {
                    for element in prefix {
                        collect(element, expressions);
                    }
                    if let Some(binding) =
                        spread.as_ref().and_then(|spread| spread.binding.as_deref())
                    {
                        collect(binding, expressions);
                    }
                    for element in suffix {
                        collect(element, expressions);
                    }
                }
                CompiledPattern::EnumVariant { payload, .. } => {
                    if let Some(payload) = payload {
                        collect(payload, expressions);
                    }
                }
                CompiledPattern::AtomTuple { payload, .. } => {
                    for element in payload {
                        collect(element, expressions);
                    }
                }
                CompiledPattern::Wildcard | CompiledPattern::Binding { .. } => {}
            }
        }

        let mut expressions = Vec::new();
        collect(self, &mut expressions);
        expressions.into_iter().flatten().collect()
    }
}

#[derive(Debug, Clone)]
pub struct PatternCompileError {
    pub reason: String,
}

impl MechErrorKind for PatternCompileError {
    fn name(&self) -> &str {
        "PatternCompileError"
    }

    fn message(&self) -> String {
        self.reason.clone()
    }
}

#[derive(Debug, Clone)]
pub struct PatternExpressionValueMissing {
    pub index: usize,
}

impl MechErrorKind for PatternExpressionValueMissing {
    fn name(&self) -> &str {
        "PatternExpressionValueMissing"
    }

    fn message(&self) -> String {
        format!(
            "No sampled value was provided for pattern expression {}.",
            self.index
        )
    }
}

#[derive(Default)]
struct PatternCompiler {
    bindings: Vec<PatternBindingSpec>,
    binding_ids: HashMap<u64, usize>,
    next_expression: usize,
}

fn canonical_atom_key(name: &str) -> MResult<NominalKey> {
    let path = CanonicalNominalPath::new(
        name.split('/')
            .filter(|segment| !segment.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>(),
    )?;
    Ok(NominalKey::from_path(NominalKind::Atom, &path))
}

impl PatternCompiler {
    fn error(&self, pattern: &Pattern, reason: impl Into<String>) -> MechError {
        MechError::new(
            PatternCompileError {
                reason: reason.into(),
            },
            None,
        )
        .with_compiler_loc()
        .with_tokens(pattern.tokens())
    }

    fn compile_binding(
        &mut self,
        pattern: &Pattern,
        id: u64,
        name: String,
        expected_schema: Option<&SchemaBody>,
    ) -> MResult<CompiledPattern> {
        let expected_schema = expected_schema.cloned();
        if let Some(index) = self.binding_ids.get(&id).copied() {
            let existing_schema = self.bindings[index].schema.clone();
            match (&existing_schema, &expected_schema) {
                (Some(existing), Some(expected)) if existing != expected => {
                    return Err(self.error(
                        pattern,
                        format!(
                            "Repeated binding '{}' has incompatible schemas '{:?}' and '{:?}'.",
                            name, existing, expected
                        ),
                    ));
                }
                (None, Some(expected)) => self.bindings[index].schema = Some(expected.clone()),
                _ => {}
            }
            return Ok(CompiledPattern::Binding {
                binding_index: index,
                id,
                name,
                expected_schema,
            });
        }

        let index = self.bindings.len();
        self.binding_ids.insert(id, index);
        self.bindings.push(PatternBindingSpec {
            index,
            id,
            name: name.clone(),
            schema: expected_schema.clone(),
        });
        Ok(CompiledPattern::Binding {
            binding_index: index,
            id,
            name,
            expected_schema,
        })
    }

    fn compile_expression(&mut self, expression: &Expression) -> CompiledPattern {
        let expression_index = self.next_expression;
        self.next_expression += 1;
        CompiledPattern::ExpressionValue {
            expression_index,
            expression: expression.clone(),
        }
    }

    fn compile(
        &mut self,
        pattern: &Pattern,
        expected_schema: Option<&SchemaBody>,
        interpreter: &Interpreter,
    ) -> MResult<CompiledPattern> {
        match pattern {
            Pattern::Wildcard => Ok(CompiledPattern::Wildcard),
            Pattern::Expression(expression) => {
                if let Expression::Literal(Literal::Atom(atom)) = expression {
                    if let Some(SchemaBody::Enum { key, variants }) = expected_schema {
                        let variant = variants
                            .iter()
                            .find(|variant| variant.name == atom.name.to_string())
                            .ok_or_else(|| {
                                self.error(
                                    pattern,
                                    format!(
                                        "'{}' is not a variant of the expected enum.",
                                        atom.name.to_string()
                                    ),
                                )
                            })?;
                        if variant.payload.is_some() {
                            return Err(self.error(
                                pattern,
                                "Enum variant pattern is missing its payload pattern.",
                            ));
                        }
                        return Ok(CompiledPattern::EnumVariant {
                            enum_key: Some(*key),
                            variant_name: atom.name.to_string(),
                            payload: None,
                        });
                    }
                }
                if let Some(var) = extract_pattern_variable(expression) {
                    self.compile_binding(
                        pattern,
                        var.name.hash(),
                        var.name.to_string(),
                        expected_schema,
                    )
                } else {
                    Ok(self.compile_expression(expression))
                }
            }
            Pattern::Tuple(tuple) => {
                let expected_elements = match expected_schema {
                    Some(SchemaBody::Tuple(elements)) => {
                        if elements.len() != tuple.0.len() {
                            return Err(self.error(
                pattern,
                format!(
                  "Tuple pattern has arity {}, but the expected tuple kind has arity {}.",
                  tuple.0.len(),
                  elements.len()
                ),
              ));
                        }
                        Some(elements)
                    }
                    Some(kind) => {
                        return Err(self.error(
                            pattern,
                            format!("Tuple pattern cannot match expected schema '{kind:?}'."),
                        ));
                    }
                    None => None,
                };
                let elements = tuple
                    .0
                    .iter()
                    .enumerate()
                    .map(|(index, element)| {
                        self.compile(
                            element,
                            expected_elements.as_ref().map(|kinds| &kinds[index]),
                            interpreter,
                        )
                    })
                    .collect::<MResult<Vec<_>>>()?;
                Ok(CompiledPattern::Tuple { elements })
            }
            Pattern::Array(array) => {
                let (element_schema, rest_schema) = match expected_schema {
                    Some(SchemaBody::Matrix { element, .. }) => {
                        let element = (**element).clone();
                        (
                            Some(element.clone()),
                            Some(SchemaBody::Matrix {
                                element: Box::new(element),
                                dimensions: vec![DimensionExpr::Hole, DimensionExpr::Hole]
                                    .into_boxed_slice(),
                            }),
                        )
                    }
                    Some(kind) => {
                        return Err(self.error(
                            pattern,
                            format!("Array pattern cannot match expected schema '{kind:?}'."),
                        ));
                    }
                    None => (None, None),
                };
                let prefix = array
                    .prefix
                    .iter()
                    .map(|element| self.compile(element, element_schema.as_ref(), interpreter))
                    .collect::<MResult<Vec<_>>>()?;
                let spread = match &array.spread {
                    Some(spread) => Some(CompiledPatternArraySpread {
                        kind: spread.kind.clone(),
                        binding: match &spread.binding {
                            Some(binding) => Some(Box::new(self.compile(
                                binding,
                                rest_schema.as_ref(),
                                interpreter,
                            )?)),
                            None => None,
                        },
                    }),
                    None => None,
                };
                let suffix = array
                    .suffix
                    .iter()
                    .map(|element| self.compile(element, element_schema.as_ref(), interpreter))
                    .collect::<MResult<Vec<_>>>()?;
                Ok(CompiledPattern::Array {
                    prefix,
                    spread,
                    suffix,
                })
            }
            Pattern::TupleStruct(tuple_struct) => match expected_schema {
                Some(SchemaBody::Enum { key, variants }) => {
                    let variant = variants
                        .iter()
                        .find(|variant| variant.name == tuple_struct.name.to_string())
                        .ok_or_else(|| {
                            self.error(
                                pattern,
                                format!(
                                    "'{}' is not a variant of the expected enum.",
                                    tuple_struct.name.to_string()
                                ),
                            )
                        })?;
                    let payload =
                        match (tuple_struct.patterns.as_slice(), &variant.payload) {
                            ([], None) => None,
                            ([payload_pattern], Some(payload_schema)) => Some(Box::new(
                                self.compile(payload_pattern, Some(payload_schema), interpreter)?,
                            )),
                            _ => {
                                return Err(self.error(
                                pattern,
                                "Enum variant pattern payload arity does not match its definition.",
                            ));
                            }
                        };
                    Ok(CompiledPattern::EnumVariant {
                        enum_key: Some(*key),
                        variant_name: tuple_struct.name.to_string(),
                        payload,
                    })
                }
                Some(SchemaBody::Tuple(schemas)) => {
                    if schemas.len() != tuple_struct.patterns.len() + 1
                        || !matches!(schemas.first(), Some(SchemaBody::Atom(_)))
                    {
                        return Err(self.error(
                pattern,
                "Atom-tagged tuple pattern arity does not match the expected tuple kind.",
              ));
                    }
                    let payload = tuple_struct
                        .patterns
                        .iter()
                        .zip(schemas.iter().skip(1))
                        .map(|(payload, schema)| self.compile(payload, Some(schema), interpreter))
                        .collect::<MResult<Vec<_>>>()?;
                    Ok(CompiledPattern::AtomTuple {
                        tag_key: canonical_atom_key(&tuple_struct.name.to_string())?,
                        payload,
                    })
                }
                Some(kind) => Err(self.error(
                    pattern,
                    format!("Tagged tuple pattern cannot match expected schema '{kind:?}'."),
                )),
                None => self.compile_untyped_tuple_struct(pattern, tuple_struct, interpreter),
            },
        }
    }

    fn compile_untyped_tuple_struct(
        &mut self,
        _pattern: &Pattern,
        tuple_struct: &PatternTupleStruct,
        interpreter: &Interpreter,
    ) -> MResult<CompiledPattern> {
        let payload = tuple_struct
            .patterns
            .iter()
            .map(|payload| self.compile(payload, None, interpreter))
            .collect::<MResult<Vec<_>>>()?;
        Ok(CompiledPattern::AtomTuple {
            tag_key: canonical_atom_key(&tuple_struct.name.to_string())?,
            payload,
        })
    }
}

pub fn compile_pattern(
    pattern: &Pattern,
    expected_schema: Option<&SchemaBody>,
    interpreter: &Interpreter,
) -> MResult<CompiledPattern> {
    PatternCompiler::default().compile(pattern, expected_schema, interpreter)
}

enum PatternExpressionSource<'a, 'execution> {
    Interpreter {
        env: &'a Environment,
        interpreter: &'a InterpreterExecution<'execution>,
    },
    Sampled(&'a [ValueCell]),
}

struct PatternMatchState<'a, 'execution> {
    binding_specs: Vec<PatternBindingSpec>,
    proposed: Vec<Option<ValueCell>>,
    expression_source: PatternExpressionSource<'a, 'execution>,
}

impl PatternMatchState<'_, '_> {
    fn expression_value(
        &self,
        expression_index: usize,
        expression_node: &Expression,
    ) -> MResult<ValueCell> {
        match &self.expression_source {
            PatternExpressionSource::Interpreter { env, interpreter } => {
                // Expression patterns read the arm's outer environment. Proposed
                // captures are intentionally invisible; capture-dependent
                // conditions belong in an explicit arm guard.
                expression_cell(expression_node, Some(env), interpreter)
            }
            PatternExpressionSource::Sampled(values) => {
                values.get(expression_index).cloned().ok_or_else(|| {
                    MechError::new(
                        PatternExpressionValueMissing {
                            index: expression_index,
                        },
                        None,
                    )
                    .with_compiler_loc()
                    .with_tokens(expression_node.tokens())
                })
            }
        }
    }

    fn matches(&mut self, pattern: &CompiledPattern, value: &ValueCell) -> MResult<bool> {
        let value = value.clone();
        match pattern {
            CompiledPattern::Wildcard => Ok(true),
            CompiledPattern::Binding { binding_index, .. } => {
                if let Some(existing) = &self.proposed[*binding_index] {
                    existing.snapshot_eq(&value)
                } else {
                    self.proposed[*binding_index] = Some(value);
                    Ok(true)
                }
            }
            CompiledPattern::ExpressionValue {
                expression_index,
                expression,
            } => {
                let expected = self.expression_value(*expression_index, expression)?;
                values_match(&expected, &value)
            }
            #[cfg(feature = "tuple")]
            CompiledPattern::Tuple { elements } => {
                let Some(values) = value.tuple_elements()? else {
                    return Ok(false);
                };
                if values.len() != elements.len() {
                    return Ok(false);
                }
                for (pattern, value) in elements.iter().zip(&values) {
                    if !self.matches(pattern, value)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            #[cfg(not(feature = "tuple"))]
            CompiledPattern::Tuple { .. } => Ok(false),
            #[cfg(feature = "matrix")]
            CompiledPattern::Array {
                prefix,
                spread,
                suffix,
            } => {
                let values = match matrix_like_values(&value)? {
                    Some(values) => values,
                    None => return Ok(false),
                };
                if values.len() < prefix.len() + suffix.len() {
                    return Ok(false);
                }
                for (pattern, value) in prefix.iter().zip(values.iter()) {
                    if !self.matches(pattern, value)? {
                        return Ok(false);
                    }
                }
                let suffix_start = values.len() - suffix.len();
                for (pattern, value) in suffix.iter().zip(values[suffix_start..].iter()) {
                    if !self.matches(pattern, value)? {
                        return Ok(false);
                    }
                }
                if spread.is_none() && values.len() != prefix.len() + suffix.len() {
                    return Ok(false);
                }
                if let Some(binding) = spread.as_ref().and_then(|spread| spread.binding.as_deref())
                {
                    let middle = capture_middle_matrix(&values, prefix.len(), suffix_start)?;
                    if !self.matches(binding, &middle)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            #[cfg(not(feature = "matrix"))]
            CompiledPattern::Array { .. } => Ok(false),
            #[cfg(feature = "enum")]
            CompiledPattern::EnumVariant {
                enum_key,
                variant_name,
                payload,
            } => {
                let SchemaBody::Enum { key, variants } = value.closed_schema_body()? else {
                    return Ok(false);
                };
                if enum_key.as_ref().is_some_and(|expected| expected != &key) {
                    return Ok(false);
                }
                let draft = value.snapshot()?.canonical_data_draft().map_err(|error| {
                    MechError::new(ValueCellSnapshotFailure { error }, None).with_compiler_loc()
                })?;
                let ValueDataDraft::Enum(actual) = draft else {
                    unreachable!("validated enum schema retains enum data")
                };
                let Some(variant) = variants.get(actual.ordinal as usize) else {
                    return Ok(false);
                };
                if &variant.name != variant_name {
                    return Ok(false);
                }
                match (payload, variant.payload.as_ref(), actual.payload) {
                    (None, None, None) => Ok(true),
                    (Some(pattern), Some(schema), Some(data)) => self.matches(
                        pattern,
                        &ValueCell::from_schema_data(schema.clone(), *data)?,
                    ),
                    _ => Ok(false),
                }
            }
            #[cfg(not(feature = "enum"))]
            CompiledPattern::EnumVariant { .. } => Ok(false),
            #[cfg(any(feature = "enum", all(feature = "tuple", feature = "atom")))]
            CompiledPattern::AtomTuple { tag_key, payload } => {
                #[cfg(feature = "enum")]
                if let SchemaBody::Enum { variants, .. } = value.closed_schema_body()? {
                    let draft = value.snapshot()?.canonical_data_draft().map_err(|error| {
                        MechError::new(ValueCellSnapshotFailure { error }, None).with_compiler_loc()
                    })?;
                    let ValueDataDraft::Enum(actual) = draft else {
                        unreachable!("validated enum schema retains enum data")
                    };
                    let Some(variant) = variants.get(actual.ordinal as usize) else {
                        return Ok(false);
                    };
                    if &canonical_atom_key(&variant.name)? != tag_key {
                        return Ok(false);
                    }
                    return match (payload.as_slice(), variant.payload.as_ref(), actual.payload) {
                        ([], None, None) => Ok(true),
                        ([pattern], Some(schema), Some(data)) => self.matches(
                            pattern,
                            &ValueCell::from_schema_data(schema.clone(), *data)?,
                        ),
                        _ => Ok(false),
                    };
                }
                #[cfg(all(feature = "tuple", feature = "atom"))]
                if let Some(values) = value.tuple_elements()? {
                    if values.len() != payload.len() + 1 {
                        return Ok(false);
                    }
                    let SchemaBody::Atom(actual_tag) = values[0].closed_schema_body()? else {
                        return Ok(false);
                    };
                    if &actual_tag != tag_key {
                        return Ok(false);
                    }
                    for (pattern, value) in payload.iter().zip(values.iter().skip(1)) {
                        if !self.matches(pattern, value)? {
                            return Ok(false);
                        }
                    }
                    return Ok(true);
                }
                Ok(false)
            }
            #[cfg(not(any(feature = "enum", all(feature = "tuple", feature = "atom"))))]
            CompiledPattern::AtomTuple { .. } => Ok(false),
        }
    }

    fn finish(self, matched: bool) -> PatternMatch {
        if !matched {
            return PatternMatch::no_match();
        }
        let bindings = self
            .binding_specs
            .into_iter()
            .zip(self.proposed)
            .filter_map(|(spec, value)| {
                value.map(|value| PatternBinding {
                    index: spec.index,
                    id: spec.id,
                    name: spec.name,
                    schema: value
                        .closed_schema_body()
                        .expect("matched canonical binding retains a valid schema"),
                    value,
                })
            })
            .collect();
        PatternMatch {
            matched: true,
            bindings,
        }
    }
}

pub fn match_compiled_pattern(
    pattern: &CompiledPattern,
    value: &ValueCell,
    env: &Environment,
    interpreter: &InterpreterExecution<'_>,
) -> MResult<PatternMatch> {
    match_compiled_pattern_with_environment(pattern, value, env, interpreter, false)
}

/// Matches a compiled pattern while treating bindings already present in the
/// environment as equality constraints. This is used by comprehension
/// generators, where a later generator may refer to a name introduced by an
/// earlier qualifier.
pub fn match_compiled_pattern_with_environment_constraints(
    pattern: &CompiledPattern,
    value: &ValueCell,
    env: &Environment,
    interpreter: &InterpreterExecution<'_>,
) -> MResult<PatternMatch> {
    match_compiled_pattern_with_environment(pattern, value, env, interpreter, true)
}

fn match_compiled_pattern_with_environment(
    pattern: &CompiledPattern,
    value: &ValueCell,
    env: &Environment,
    interpreter: &InterpreterExecution<'_>,
    seed_existing_bindings: bool,
) -> MResult<PatternMatch> {
    let specs = pattern.binding_specs();
    let proposed = if seed_existing_bindings {
        specs
            .iter()
            .map(|spec| env.get(&spec.id).cloned())
            .collect()
    } else {
        vec![None; specs.len()]
    };
    let mut state = PatternMatchState {
        proposed,
        binding_specs: specs,
        expression_source: PatternExpressionSource::Interpreter { env, interpreter },
    };
    let matched = state.matches(pattern, value)?;
    Ok(state.finish(matched))
}

pub fn match_compiled_pattern_with_values(
    pattern: &CompiledPattern,
    value: &ValueCell,
    expression_values: &[ValueCell],
) -> MResult<PatternMatch> {
    let specs = pattern.binding_specs();
    let mut state = PatternMatchState {
        proposed: vec![None; specs.len()],
        binding_specs: specs,
        expression_source: PatternExpressionSource::Sampled(expression_values),
    };
    let matched = state.matches(pattern, value)?;
    Ok(state.finish(matched))
}

pub trait PatternBindingSink {
    fn commit(&mut self, pattern_match: &PatternMatch) -> MResult<()>;
}

pub struct EnvironmentBindingSink<'a> {
    env: &'a mut Environment,
}

impl<'a> EnvironmentBindingSink<'a> {
    pub fn new(env: &'a mut Environment) -> Self {
        Self { env }
    }
}

impl PatternBindingSink for EnvironmentBindingSink<'_> {
    fn commit(&mut self, pattern_match: &PatternMatch) -> MResult<()> {
        if pattern_match.matched {
            for binding in &pattern_match.bindings {
                self.env.insert(binding.id, binding.value.clone());
            }
        }
        Ok(())
    }
}

pub fn pattern_matches_arguments(
    pattern: &Pattern,
    args: &[ValueCell],
    env: &mut Environment,
    interpreter: &InterpreterExecution<'_>,
) -> MResult<bool> {
    if matches!(pattern, Pattern::Wildcard) {
        return Ok(true);
    }
    if args.len() == 1 {
        return pattern_matches_value(pattern, &args[0], env, interpreter);
    }
    #[cfg(feature = "tuple")]
    {
        let arguments = ValueCell::tuple_from_cells(args)?;
        return pattern_matches_value(pattern, &arguments, env, interpreter);
    }
    #[cfg(not(feature = "tuple"))]
    {
        Ok(args.is_empty() && matches!(pattern, Pattern::Wildcard))
    }
}

pub fn pattern_matches_value(
    pattern: &Pattern,
    value: &ValueCell,
    env: &mut Environment,
    interpreter: &InterpreterExecution<'_>,
) -> MResult<bool> {
    let compiled = compile_pattern(pattern, None, interpreter)?;
    let pattern_match = match_compiled_pattern(&compiled, value, env, interpreter)?;
    EnvironmentBindingSink::new(env).commit(&pattern_match)?;
    Ok(pattern_match.matched)
}

// Collects all variable ids introduced by a pattern (via
// collect_pattern_variable_ids) and removes them from the environment. Used
// to undo bindings when a pattern arm fails or needs to be retried.
pub fn clear_pattern_bindings(pattern: &Pattern, env: &mut Environment) {
    let mut ids = Vec::new();
    collect_pattern_variable_ids(pattern, &mut ids);
    for var_id in ids {
        env.remove(&var_id);
    }
}

// Reconstructs a Value from a pattern using the current environment. This is the inverse of matching. used to extract or re-emit bound values.
pub fn pattern_to_value(
    pattern: &Pattern,
    env: &Environment,
    p: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    match pattern {
        Pattern::Wildcard => Ok(ValueCell::unit()),
        Pattern::Expression(expr) => expression_cell(expr, Some(env), p),
        #[cfg(feature = "tuple")]
        Pattern::Tuple(pattern_tuple) => {
            let mut values = Vec::with_capacity(pattern_tuple.0.len());
            for inner in &pattern_tuple.0 {
                values.push(pattern_to_value(inner, env, p)?);
            }
            return ValueCell::tuple_from_cells(&values);
        }
        #[cfg(feature = "matrix")]
        Pattern::Array(array) => {
            let mut values = Vec::new();
            for inner in &array.prefix {
                let inner_value = pattern_to_value(inner, env, p)?;
                if let Some(inner_values) = matrix_like_values(&inner_value)? {
                    values.extend(inner_values);
                } else {
                    values.push(inner_value);
                }
            }
            if let Some(spread) = &array.spread {
                if let Some(binding) = &spread.binding {
                    let bound = pattern_to_value(binding, env, p)?;
                    if let Some(bound_values) = matrix_like_values(&bound)? {
                        values.extend(bound_values);
                    } else {
                        values.push(bound);
                    }
                }
            }
            for inner in &array.suffix {
                let inner_value = pattern_to_value(inner, env, p)?;
                if let Some(inner_values) = matrix_like_values(&inner_value)? {
                    values.extend(inner_values);
                } else {
                    values.push(inner_value);
                }
            }
            return ValueCell::dynamic_matrix_from_cells(1, values.len(), &values);
        }
        #[cfg(all(feature = "tuple", feature = "atom"))]
        Pattern::TupleStruct(pattern_tuple_struct) => {
            #[cfg(feature = "enum")]
            {
                let variant_id = pattern_tuple_struct.name.hash();
                if let Some((_enum_id, enum_def)) =
                    p.state.borrow().enums.iter().find(|(_, enm)| {
                        enm.variants.iter().any(|variant| variant.id == variant_id)
                    })
                {
                    let payload = if pattern_tuple_struct.patterns.len() == 1 {
                        Some(pattern_to_value(&pattern_tuple_struct.patterns[0], env, p)?)
                    } else if pattern_tuple_struct.patterns.is_empty() {
                        None
                    } else {
                        return Err(MechError::new(FeatureNotEnabledError, Some("Enum tuple-struct patterns currently support zero or one payload value.".to_string())).with_compiler_loc());
                    };
                    let schema = crate::structures::enum_schema(enum_def)?;
                    let ordinal = enum_def
                        .variants
                        .iter()
                        .position(|variant| variant.id == variant_id)
                        .expect("matched enum variant remains present")
                        as u32;
                    let payload = payload
                        .as_ref()
                        .map(canonical_pattern_draft)
                        .transpose()?
                        .map(Box::new);
                    return ValueCell::from_schema_data(
                        schema,
                        ValueDataDraft::Enum(EnumDraft { ordinal, payload }),
                    );
                }
            }
            let mut values = Vec::with_capacity(pattern_tuple_struct.patterns.len() + 1);
            values.push(atom(
                &Atom {
                    name: pattern_tuple_struct.name.clone(),
                },
                p,
            )?);
            for inner in &pattern_tuple_struct.patterns {
                values.push(pattern_to_value(inner, env, p)?);
            }
            return ValueCell::tuple_from_cells(&values);
        }
        #[cfg(not(all(feature = "tuple", feature = "matrix", feature = "atom")))]
        _ => Err(MechError::new(FeatureNotEnabledError, None).with_compiler_loc()),
    }
}

#[cfg(all(feature = "tuple", feature = "atom", feature = "enum"))]
fn canonical_pattern_draft(value: &ValueCell) -> MResult<ValueDataDraft> {
    value.snapshot()?.canonical_data_draft().map_err(|error| {
        MechError::new(ValueCellSnapshotFailure { error }, None).with_compiler_loc()
    })
}

// Variable id harvester. Recursively walks a pattern and pushes the hashed ids
// of all bound variable names into a Vec<u64>. Handles Var expressions, tuples,
// arrays (including spread bindings), and tuple-structs. Used by
// clear_pattern_bindings.
fn collect_pattern_variable_ids(pattern: &Pattern, ids: &mut Vec<u64>) {
    match pattern {
        Pattern::Expression(Expression::Var(var)) => ids.push(var.name.hash()),
        #[cfg(feature = "tuple")]
        Pattern::Tuple(tuple) => {
            for item in &tuple.0 {
                collect_pattern_variable_ids(item, ids);
            }
        }
        #[cfg(feature = "matrix")]
        Pattern::Array(array) => {
            for item in &array.prefix {
                collect_pattern_variable_ids(item, ids);
            }
            if let Some(spread) = &array.spread {
                if let Some(binding) = &spread.binding {
                    collect_pattern_variable_ids(binding, ids);
                }
            }
            for item in &array.suffix {
                collect_pattern_variable_ids(item, ids);
            }
        }
        #[cfg(all(feature = "tuple", feature = "atom"))]
        Pattern::TupleStruct(tuple_struct) => {
            for item in &tuple_struct.patterns {
                collect_pattern_variable_ids(item, ids);
            }
        }
        _ => {}
    }
}

#[cfg(feature = "matrix")]
fn capture_middle_matrix(values: &[ValueCell], start: usize, end: usize) -> MResult<ValueCell> {
    ValueCell::dynamic_matrix_from_cells(1, end.saturating_sub(start), &values[start..end])
}

#[cfg(feature = "matrix")]
pub(crate) fn matrix_like_values(value: &ValueCell) -> MResult<Option<Vec<ValueCell>>> {
    value.matrix_elements()
}

pub(crate) fn pattern_var_is_binding(var: &Var) -> bool {
    var.context.is_none() && !is_internal_pattern_value_identifier(&var.name)
}

fn extract_pattern_variable(expr: &Expression) -> Option<&Var> {
    match expr {
        Expression::Var(var) if pattern_var_is_binding(var) => Some(var),
        Expression::Var(_) => None,
        Expression::Formula(factor) => match factor {
            Factor::Expression(inner_expr) => extract_pattern_variable(inner_expr),
            Factor::Term(term) if term.rhs.is_empty() => {
                extract_pattern_variable_from_term(&term.lhs)
            }
            _ => None,
        },
        _ => None,
    }
}

fn extract_pattern_variable_from_term(factor: &Factor) -> Option<&Var> {
    match factor {
        Factor::Expression(expr) => extract_pattern_variable(expr),
        Factor::Parenthetical(inner) => extract_pattern_variable_from_term(inner),
        _ => None,
    }
}

fn values_match(expected: &ValueCell, actual: &ValueCell) -> MResult<bool> {
    expected.snapshot_eq(actual)
}

#[cfg(all(test, feature = "tuple", feature = "u64"))]
mod tests {
    use super::*;

    fn u64_cell(value: u64) -> ValueCell {
        ValueCell::from_exact(value).unwrap()
    }

    fn tuple(values: &[u64]) -> ValueCell {
        ValueCell::tuple_from_cells(&values.iter().copied().map(u64_cell).collect::<Vec<_>>())
            .unwrap()
    }

    fn assert_u64(cell: &ValueCell, expected: u64) {
        let snapshot = cell.snapshot().unwrap();
        assert!(matches!(snapshot.data(), ValueData::U64(actual) if *actual == expected));
    }

    #[test]
    fn failed_compiled_match_returns_no_bindings_and_cannot_mutate_sink() {
        let id = hash_str("x");
        let binding = CompiledPattern::Binding {
            binding_index: 0,
            id,
            name: "x".to_string(),
            expected_schema: Some(SchemaBody::UnsignedInteger(IntegerWidth::W64)),
        };
        let pattern = CompiledPattern::Tuple {
            elements: vec![binding.clone(), binding],
        };
        let value = tuple(&[1, 2]);

        let pattern_match = match_compiled_pattern_with_values(&pattern, &value, &[]).unwrap();
        assert!(!pattern_match.matched);
        assert!(pattern_match.bindings.is_empty());

        let mut env = Environment::new();
        env.insert(id, u64_cell(9));
        EnvironmentBindingSink::new(&mut env)
            .commit(&pattern_match)
            .unwrap();
        assert_u64(env.get(&id).unwrap(), 9);
    }

    #[test]
    fn existing_environment_bindings_are_constraints_inside_the_matcher() {
        let x_id = hash_str("x");
        let y_id = hash_str("y");
        let pattern = CompiledPattern::Tuple {
            elements: vec![
                CompiledPattern::Binding {
                    binding_index: 0,
                    id: x_id,
                    name: "x".to_string(),
                    expected_schema: Some(SchemaBody::UnsignedInteger(IntegerWidth::W64)),
                },
                CompiledPattern::Binding {
                    binding_index: 1,
                    id: y_id,
                    name: "y".to_string(),
                    expected_schema: Some(SchemaBody::UnsignedInteger(IntegerWidth::W64)),
                },
            ],
        };
        let interpreter = Interpreter::new(0, 10_000);
        let mut services = NoMechExecutionServices;
        let execution = InterpreterExecution::new(&interpreter, &mut services);
        let mut env = Environment::from([(x_id, u64_cell(1))]);

        let mismatch = tuple(&[2, 3]);
        let pattern_match = match_compiled_pattern_with_environment_constraints(
            &pattern, &mismatch, &env, &execution,
        )
        .unwrap();
        assert!(!pattern_match.matched);
        assert!(pattern_match.bindings.is_empty());
        EnvironmentBindingSink::new(&mut env)
            .commit(&pattern_match)
            .unwrap();
        assert_eq!(env.len(), 1);
        assert_u64(env.get(&x_id).unwrap(), 1);

        let match_value = tuple(&[1, 3]);
        let pattern_match = match_compiled_pattern_with_environment_constraints(
            &pattern,
            &match_value,
            &env,
            &execution,
        )
        .unwrap();
        assert!(pattern_match.matched);
        EnvironmentBindingSink::new(&mut env)
            .commit(&pattern_match)
            .unwrap();
        assert_u64(env.get(&x_id).unwrap(), 1);
        assert_u64(env.get(&y_id).unwrap(), 3);
    }

    #[test]
    fn spellable_internal_looking_name_remains_an_ordinary_binding() {
        let source_name = "mech-internal-context-user-value";
        let source_identifier = Identifier {
            name: Token::new(
                TokenKind::Identifier,
                SourceRange::default(),
                source_name.chars().collect(),
            ),
        };
        let source_var = Var {
            name: source_identifier,
            context: None,
            kind: None,
        };
        assert!(pattern_var_is_binding(&source_var));

        let internal_var = Var {
            name: internal_pattern_value_identifier("context-value"),
            context: None,
            kind: None,
        };
        assert!(!pattern_var_is_binding(&internal_var));
    }
}
